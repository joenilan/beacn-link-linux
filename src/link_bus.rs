//! Persistent Link In channel bus (native/link_bus.c).
//!
//! Four LinX sinks stay up for the whole connection and are bridged to the
//! Studio's Link In pairs. Applications are routed by pointing their output
//! stream at a sink, the same way BEACN Link on Windows uses the Studio's
//! "Link N Out" endpoints. Unrelated desktop activity never stops the bus.
use crate::metering::Mailbox;
use std::{
    ffi::{c_char, c_int, c_void},
    sync::Arc,
};

pub const CHANNELS: usize = 4;
const SNAPSHOT_BYTES: usize = 1 << 20;

unsafe extern "C" {
    fn lb_open(mailboxes: *const *const Mailbox) -> *mut c_void;
    fn lb_close(bus: *mut c_void);
    fn lb_set_card(bus: *mut c_void, card: c_int);
    fn lb_maintain(bus: *mut c_void) -> c_int;
    fn lb_snapshot(bus: *mut c_void, out: *mut c_char, size: usize) -> c_int;
    fn lb_route(bus: *mut c_void, node: u32, serial: u64, channel: c_int) -> c_int;
    fn lb_target_name(bus: *mut c_void, node: u32, out: *mut c_char, size: usize) -> c_int;
    fn lb_route_named(bus: *mut c_void, node: u32, serial: u64, name: *const c_char) -> c_int;
    fn lb_set_level(bus: *mut c_void, channel: c_int, volume: f32, mute: c_int) -> c_int;
}

/// A Link In channel's send level as PipeWire reports it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Level {
    /// False until the sink has reported its properties.
    pub known: bool,
    /// Linear gain 0..1 (both channels averaged).
    pub volume: f32,
    pub mute: bool,
}

/// State of the physical Studio transport behind the four sinks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Transport {
    /// No Studio selected.
    #[default]
    Off,
    /// Studio selected; its 96-kHz transport is starting.
    Starting,
    /// Configured and linked, idle.
    Ready,
    Running,
    /// Failed (busy, unplugged, driver error); retried automatically.
    Error,
}

/// The Studio's Link Out pairs, published as four LinX sources.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LinkOut {
    pub sources_ready: usize,
    /// The Studio capture transport feeding them.
    pub transport: Transport,
    pub links: usize,
    /// LinX switched the Studio's session profile off and restores it later.
    pub profile_owned: bool,
    /// The Studio card's current session profile ("off" while LinX owns it).
    pub profile: String,
    pub error: Option<String>,
}
impl LinkOut {
    /// All four Link Out sources carry the Studio's capture pairs.
    pub fn complete(&self) -> bool {
        self.sources_ready == CHANNELS
            && matches!(self.transport, Transport::Ready | Transport::Running)
            && self.links == CHANNELS * 2
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BusStatus {
    pub healthy: bool,
    pub metadata: bool,
    pub sinks_ready: usize,
    pub card: Option<u32>,
    pub transport: Transport,
    pub bridge_links: usize,
    /// Changes whenever the routing metadata or a sink is recreated.
    pub generation: u64,
    pub error: Option<String>,
    pub levels: [Level; CHANNELS],
    pub link_out: LinkOut,
}
impl BusStatus {
    /// All four channels can carry audio to the Studio.
    pub fn complete(&self) -> bool {
        self.healthy
            && self.metadata
            && self.sinks_ready == CHANNELS
            && matches!(self.transport, Transport::Ready | Transport::Running)
            && self.bridge_links == CHANNELS * 2
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stream {
    pub node: u32,
    pub serial: u64,
    pub client: u64,
    pub pid: u64,
    pub running: bool,
    /// Link In 1..4 the stream is currently playing to, 0 otherwise.
    pub channel: u8,
    pub movable: bool,
    /// Link In 1..4 the stream's routing metadata points at, 0 otherwise.
    pub target_channel: u8,
    /// Node information (application identity) has arrived.
    pub info: bool,
    pub app: String,
    pub binary: String,
    pub media: String,
    pub node_name: String,
}
impl Stream {
    /// Per-application identity, matching the session manager's own
    /// per-application stream memory where possible.
    pub fn app_key(&self) -> String {
        [&self.app, &self.binary, &self.node_name]
            .into_iter()
            .find(|s| !s.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| format!("pid {}", self.pid))
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Snapshot {
    pub status: BusStatus,
    pub streams: Vec<Stream>,
}

fn unhex(text: &str) -> Result<String, String> {
    if text.len() % 2 != 0 {
        return Err("Invalid bus text.".into());
    }
    let bytes = (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).map_err(|_| "Invalid bus text."))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
fn int<T: std::str::FromStr>(text: &str) -> Result<T, String> {
    text.parse().map_err(|_| format!("Invalid bus number {text:?}."))
}

fn transport(code: &str) -> Transport {
    match code {
        "1" => Transport::Starting,
        "2" => Transport::Ready,
        "3" => Transport::Running,
        "4" => Transport::Error,
        _ => Transport::Off,
    }
}

pub fn parse(raw: &str) -> Result<Snapshot, String> {
    let mut snapshot = Snapshot::default();
    let mut seen_bus = false;
    for line in raw.lines().filter(|l| !l.is_empty()) {
        let f = line.split('\t').collect::<Vec<_>>();
        match f[0] {
            "bus" if f.len() == 9 => {
                let card: i64 = int(f[4])?;
                let error = unhex(f[8])?;
                snapshot.status = BusStatus {
                    healthy: f[1] == "1",
                    metadata: f[2] == "1",
                    sinks_ready: int(f[3])?,
                    card: u32::try_from(card).ok(),
                    transport: transport(f[5]),
                    bridge_links: int(f[6])?,
                    generation: int(f[7])?,
                    error: (!error.is_empty()).then_some(error),
                    levels: snapshot.status.levels,
                    link_out: snapshot.status.link_out.clone(),
                };
                seen_bus = true;
            }
            "out" if f.len() == 7 => {
                let error = unhex(f[6])?;
                snapshot.status.link_out = LinkOut {
                    sources_ready: int(f[1])?,
                    transport: transport(f[2]),
                    links: int(f[3])?,
                    profile_owned: f[4] == "1",
                    profile: unhex(f[5])?,
                    error: (!error.is_empty()).then_some(error),
                };
            }
            "level" if f.len() == 5 => {
                let channel: usize = int(f[1])?;
                let volume: f32 = int(f[3])?;
                if !(1..=CHANNELS).contains(&channel) || !volume.is_finite() {
                    return Err("Invalid level record.".into());
                }
                snapshot.status.levels[channel - 1] = Level {
                    known: f[2] == "1",
                    volume: volume.clamp(0.0, 1.0),
                    mute: f[4] == "1",
                };
            }
            "stream" if f.len() == 14 => snapshot.streams.push(Stream {
                node: int(f[1])?,
                serial: int(f[2])?,
                client: int(f[3])?,
                pid: int(f[4])?,
                running: f[5] == "3",
                channel: int(f[6])?,
                movable: f[7] != "1",
                target_channel: int(f[8])?,
                info: f[9] == "1",
                app: unhex(f[10])?,
                binary: unhex(f[11])?,
                media: unhex(f[12])?,
                node_name: unhex(f[13])?,
            }),
            _ => return Err(format!("Unexpected bus record {:?}.", f[0])),
        }
    }
    if !seen_bus {
        return Err("Bus status absent.".into());
    }
    snapshot.streams.sort_by_key(|s| (s.node, s.serial));
    Ok(snapshot)
}

/// Owner of the native bus. Dropping it releases the sinks; the session
/// manager then returns their streams to the desktop default output.
pub struct LinkBus {
    raw: *mut c_void,
    meters: [Arc<Mailbox>; CHANNELS],
    buffer: Vec<u8>,
}
// The C owner serializes every entry point on its own thread loop lock.
unsafe impl Send for LinkBus {}

impl LinkBus {
    pub fn open() -> Result<Self, String> {
        let meters: [Arc<Mailbox>; CHANNELS] = Default::default();
        let pointers = meters.each_ref().map(Arc::as_ptr);
        let raw = unsafe { lb_open(pointers.as_ptr()) };
        if raw.is_null() {
            return Err("Could not start the LinX audio bus.".into());
        }
        Ok(Self {
            raw,
            meters,
            buffer: vec![0; SNAPSHOT_BYTES],
        })
    }
    pub fn set_card(&mut self, card: Option<u32>) {
        let card = card.and_then(|c| c_int::try_from(c).ok()).unwrap_or(-1);
        unsafe { lb_set_card(self.raw, card) }
    }
    /// Self-healing pass: recreates any owned sink, transport or link that
    /// disappeared. `Ok(false)` is a transient PipeWire stall (skip this
    /// cycle, keep everything); `Err` means the connection itself is gone.
    pub fn maintain(&mut self) -> Result<bool, String> {
        match unsafe { lb_maintain(self.raw) } {
            0 => Ok(true),
            -2 => Ok(false),
            _ => Err("The PipeWire connection was lost.".into()),
        }
    }
    pub fn snapshot(&mut self) -> Result<Snapshot, String> {
        self.buffer.fill(0);
        let code =
            unsafe { lb_snapshot(self.raw, self.buffer.as_mut_ptr().cast(), self.buffer.len()) };
        if code != 0 {
            return Err("Audio bus snapshot unavailable.".into());
        }
        let len = self
            .buffer
            .iter()
            .position(|b| *b == 0)
            .ok_or("Unterminated bus snapshot.")?;
        parse(std::str::from_utf8(&self.buffer[..len]).map_err(|_| "Invalid bus snapshot.")?)
    }
    /// Point a stream at Link In 1..4, or 0 to clear LinX's target.
    pub fn route(&mut self, stream: &Stream, channel: u8) -> Result<(), String> {
        if channel as usize > CHANNELS {
            return Err("Link In channel must be 1–4.".into());
        }
        if unsafe { lb_route(self.raw, stream.node, stream.serial, channel as c_int) } != 0 {
            return Err(format!("Could not move {} to Link In {channel}.", stream.app_key()));
        }
        Ok(())
    }
    /// The output a stream is explicitly targeted at (node.name), if any.
    pub fn target_name(&mut self, stream: &Stream) -> Option<String> {
        let mut out = [0u8; 256];
        if unsafe { lb_target_name(self.raw, stream.node, out.as_mut_ptr().cast(), out.len()) } != 0
        {
            return None;
        }
        let len = out.iter().position(|b| *b == 0)?;
        String::from_utf8(out[..len].to_vec()).ok()
    }
    /// Return a stream to a named output, or the desktop default if it is gone.
    pub fn route_named(&mut self, stream: &Stream, name: &str) -> Result<(), String> {
        let name = std::ffi::CString::new(name).map_err(|_| "Invalid output name.")?;
        if unsafe { lb_route_named(self.raw, stream.node, stream.serial, name.as_ptr()) } != 0 {
            return Err(format!("Could not return {} to its output.", stream.app_key()));
        }
        Ok(())
    }
    /// Set a channel's send level. `None` leaves that part unchanged.
    pub fn set_level(
        &mut self,
        channel: u8,
        volume: Option<f32>,
        mute: Option<bool>,
    ) -> Result<(), String> {
        if !(1..=CHANNELS as u8).contains(&channel) {
            return Err("Link In channel must be 1–4.".into());
        }
        let volume = match volume {
            Some(v) if v.is_finite() => v.clamp(0.0, 1.0),
            Some(_) => return Err("Invalid level.".into()),
            None => -1.0,
        };
        let mute = mute.map_or(-1, c_int::from);
        if unsafe { lb_set_level(self.raw, channel as c_int, volume, mute) } != 0 {
            return Err(format!("Link In {channel} is not ready."));
        }
        Ok(())
    }
    pub fn meter(&self, channel: u8) -> Option<Arc<Mailbox>> {
        (1..=CHANNELS as u8)
            .contains(&channel)
            .then(|| self.meters[channel as usize - 1].clone())
    }
}
impl Drop for LinkBus {
    fn drop(&mut self) {
        unsafe { lb_close(self.raw) };
        for meter in &self.meters {
            meter.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn hex(s: &str) -> String {
        s.bytes().map(|b| format!("{b:02x}")).collect()
    }
    #[test]
    fn parses_status_and_streams() {
        let raw = format!(
            "bus\t1\t1\t4\t2\t3\t8\t77\t\nlevel\t2\t1\t0.500000\t1\nstream\t104\t3345\t101\t184809\t3\t3\t0\t3\t1\t{}\t{}\t{}\t{}\n",
            hex("Firefox"),
            hex("firefox-esr"),
            hex("AudioStream"),
            hex("Firefox")
        );
        let s = parse(&raw).unwrap();
        assert!(s.status.complete());
        assert_eq!(s.status.card, Some(2));
        assert_eq!(s.status.transport, Transport::Running);
        assert_eq!(s.status.generation, 77);
        assert_eq!(
            s.status.levels[1],
            Level {
                known: true,
                volume: 0.5,
                mute: true
            }
        );
        assert!(!s.status.levels[0].known);
        assert_eq!(s.streams.len(), 1);
        let stream = &s.streams[0];
        assert_eq!((stream.node, stream.serial, stream.channel), (104, 3345, 3));
        assert_eq!(stream.target_channel, 3);
        assert!(stream.running && stream.movable && stream.info);
        assert_eq!(stream.app_key(), "Firefox");
    }
    #[test]
    fn transport_states_and_error_text() {
        let raw = format!("bus\t1\t1\t4\t-1\t0\t0\t0\t{}\n", hex("busy"));
        let s = parse(&raw).unwrap();
        assert_eq!(s.status.transport, Transport::Off);
        assert_eq!(s.status.card, None);
        assert_eq!(s.status.error.as_deref(), Some("busy"));
        assert!(!s.status.complete());
        let s = parse("bus\t1\t1\t4\t2\t4\t0\t0\t\n").unwrap();
        assert_eq!(s.status.transport, Transport::Error);
        assert!(!s.status.complete());
    }
    #[test]
    fn parses_link_out_status() {
        let raw = format!(
            "bus\t1\t1\t4\t2\t3\t8\t0\t\nout\t4\t3\t8\t1\t{}\t\n",
            hex("off")
        );
        let s = parse(&raw).unwrap();
        let out = &s.status.link_out;
        assert!(out.complete());
        assert_eq!(out.transport, Transport::Running);
        assert!(out.profile_owned);
        assert_eq!(out.profile, "off");
        assert_eq!(out.error, None);
        let raw = format!("bus\t1\t1\t4\t2\t3\t8\t0\t\nout\t4\t4\t0\t0\t\t{}\n", hex("busy"));
        let out = parse(&raw).unwrap().status.link_out;
        assert!(!out.complete());
        assert_eq!(out.transport, Transport::Error);
        assert_eq!(out.error.as_deref(), Some("busy"));
    }
    #[test]
    fn app_key_falls_back_to_binary_then_node() {
        let raw = format!(
            "bus\t1\t1\t4\t2\t2\t8\t0\t\nstream\t5\t6\t7\t8\t2\t0\t1\t0\t0\t\t{}\t\t{}\n",
            hex("game.exe"),
            hex("node")
        );
        let s = parse(&raw).unwrap();
        assert_eq!(s.streams[0].app_key(), "game.exe");
        assert!(!s.streams[0].movable);
        assert!(!s.streams[0].running);
        assert!(!s.streams[0].info);
    }
    #[test]
    fn rejects_missing_status_and_malformed_records() {
        assert!(parse("").is_err());
        assert!(parse("bus\t1\n").is_err());
        assert!(parse("bus\t1\t1\t4\t2\t1\t8\t0\tzz\n").is_err());
    }
}

//! LinX Link engine: persistent channel bus, per-application channel memory,
//! and the BEACN Link control channel (app publication / remote assignment).
//!
//! Model (matches BEACN Link 1.0.4 on Windows):
//! - Four Link In channels exist for the whole connection.
//! - An application is "on" a channel when its output streams play to that
//!   channel's sink. Displayed state is read back from the graph.
//! - LinX remembers each application's chosen channel and re-applies it to
//!   every new stream of that application (media pause/resume, new tabs,
//!   restarts) and after the session manager or a sink is recreated.
//! - Only applications that are currently playing are published, every
//!   500 ms. Remote assignment 1..4 routes the app; 0 is ignored, as on
//!   Windows.
//! - The audio bus is independent of the USB control channel: a control
//!   failure is retried in the background while audio keeps flowing.
//! - Pausing LinX returns every application it moved to its previous output.
//! - An application can also be placed on a device of this PC (Spotify on the
//!   speakers) while the rest stay on their channels; a placement overrides
//!   the remembered channel until the application is linked again.
use crate::{
    link_bus::{BusStatus, LinkBus, Output, Stream, CHANNELS},
    metering::Mailbox,
    session::{PollPolicy, PollReply, ReceivedNamePolicy, Session},
    Assignment, Record, MAX_PAYLOAD,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};

/// Longest application name LinX publishes to BEACN App.
pub const MAX_WIRE_NAME: usize = 64;
/// Windows treats the host as gone after 10 publish ticks without a reply.
pub const HOST_TIMEOUT: Duration = Duration::from_secs(5);
/// A local assignment is published as intended for this long, so BEACN App
/// sees it before the graph confirms it.
pub const LOCAL_GRACE: Duration = Duration::from_secs(3);
const ROUTE_ATTEMPTS: u8 = 3;

/// Where an application was explicitly placed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    /// Link In 1..4.
    Channel(u8),
    /// A desktop output, by node.name.
    Device(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    pub key: String,
    pub name: String,
    pub wire_name: Vec<u8>,
    /// Observed channel: 1..4 when every stream plays to that channel, else 0.
    pub channel: u8,
    /// Remembered channel, if the user assigned one.
    pub remembered: Option<u8>,
    /// Placement that overrides the remembered channel, if any.
    pub pinned: Option<Target>,
    pub running: bool,
    pub streams: Vec<Stream>,
}

/// Printable-ASCII publication name, as BEACN App expects.
pub fn wire_name(name: &str) -> Vec<u8> {
    let mut bytes: Vec<u8> = name
        .chars()
        .map(|c| if (' '..='~').contains(&c) { c as u8 } else { b'?' })
        .collect();
    while bytes.last() == Some(&b' ') {
        bytes.pop();
    }
    bytes.truncate(MAX_WIRE_NAME);
    if bytes.is_empty() {
        bytes = b"App".to_vec();
    }
    bytes
}
fn stable_tag(key: &str) -> u16 {
    // FNV-1a: stable across runs, unlike the std hasher.
    let mut hash: u32 = 0x811c9dc5;
    for b in key.bytes() {
        hash = (hash ^ b as u32).wrapping_mul(0x01000193);
    }
    (hash ^ (hash >> 16)) as u16
}

/// Group streams into applications. LinX's own nodes never appear here.
pub fn applications(streams: &[Stream], remembered: &BTreeMap<String, u8>) -> Vec<App> {
    let mut apps: BTreeMap<String, App> = BTreeMap::new();
    for stream in streams {
        let key = stream.app_key();
        let name = key.trim().to_owned();
        let app = apps.entry(key.clone()).or_insert_with(|| App {
            wire_name: wire_name(&name),
            name,
            remembered: remembered.get(&key).copied(),
            pinned: None,
            key,
            channel: 0,
            running: false,
            streams: vec![],
        });
        app.running |= stream.running;
        app.streams.push(stream.clone());
    }
    let mut apps: Vec<App> = apps.into_values().collect();
    for app in &mut apps {
        let first = app.streams[0].channel;
        app.channel = if app.streams.iter().all(|s| s.channel == first) {
            first
        } else {
            0
        };
    }
    // Distinct wire names; the suffix derives from the app key so it never
    // changes when other applications come and go.
    let mut counts: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
    for app in &apps {
        *counts.entry(app.wire_name.clone()).or_default() += 1;
    }
    let mut used = BTreeSet::new();
    for app in &mut apps {
        if counts[&app.wire_name] > 1 {
            let mut name = app.wire_name.clone();
            name.truncate(MAX_WIRE_NAME - 6);
            name.extend_from_slice(format!(" #{:04x}", stable_tag(&app.key)).as_bytes());
            app.wire_name = name;
        }
        while !used.insert(app.wire_name.clone()) {
            app.wire_name.push(b'+');
        }
    }
    apps
}

/// Records to publish, within one frame: like Windows, only applications that
/// are currently playing; those on a channel first, then by name. `intent`
/// overrides an app's channel while a local assignment settles.
pub fn records_for(apps: &[App], intent: &BTreeMap<String, u8>) -> Vec<Record> {
    let channel = |a: &App| intent.get(&a.key).copied().unwrap_or(a.channel);
    let mut ordered: Vec<&App> = apps.iter().filter(|a| a.running).collect();
    ordered.sort_by_key(|a| (channel(a) == 0, a.wire_name.clone()));
    let mut out = Vec::new();
    let mut used = 1; // terminator
    for app in ordered {
        let size = 2 + app.wire_name.len();
        if used + size > MAX_PAYLOAD {
            continue;
        }
        used += size;
        out.push(Record {
            name: app.wire_name.clone(),
            assignment: Assignment::from_byte(channel(app)),
        });
    }
    out
}

/// Remote assignments from BEACN App: (app key, channel) for records whose
/// channel differs from what LinX last published for that name. Channel 0
/// and values above 4 are ignored, matching BEACN Link on Windows.
pub fn remote_commands(
    apps: &[App],
    published: &[Record],
    received: &[Record],
) -> Vec<(String, u8)> {
    let mut out: Vec<(String, u8)> = Vec::new();
    for record in received {
        let name = ReceivedNamePolicy::WindowsCString.for_planning(record).name;
        let channel = record.assignment.byte();
        if channel == 0 || channel as usize > CHANNELS {
            continue;
        }
        let shown = published
            .iter()
            .find(|r| r.name == name)
            .map(|r| r.assignment.byte());
        if shown == Some(channel) {
            continue;
        }
        if let Some(app) = apps.iter().find(|a| a.wire_name == name) {
            if !out.iter().any(|(k, _)| k == &app.key) {
                out.push((app.key.clone(), channel));
            }
        }
    }
    out
}

/// A move of a LinX-placed stream made outside LinX.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopMove {
    /// Retargeted to another Link channel.
    Channel { key: String, id: (u32, u64), channel: u8 },
    /// Retargeted away from LinX (the caller confirms the new output).
    Away { key: String, id: (u32, u64) },
}

/// Moves made outside LinX since the last pass. `wrote` holds the channel
/// LinX placed each stream on (streams found on their application's
/// remembered channel count as placed); `observed` the last seen target. A
/// change counts only after the stream had settled on LinX's placement, so
/// LinX's own pending writes (a reassignment not yet echoed) are never
/// mistaken for the user's.
pub fn desktop_moves(
    streams: &[Stream],
    remembered: &BTreeMap<String, u8>,
    wrote: &mut BTreeMap<(u32, u64), u8>,
    observed: &mut BTreeMap<(u32, u64), u8>,
) -> Vec<DesktopMove> {
    let live: BTreeSet<_> = streams.iter().map(|s| (s.node, s.serial)).collect();
    wrote.retain(|id, _| live.contains(id));
    observed.retain(|id, _| live.contains(id));
    let mut moves = Vec::new();
    for stream in streams.iter().filter(|s| s.info && s.movable) {
        let id = (stream.node, stream.serial);
        let key = stream.app_key();
        let now = stream.target_channel;
        if now != 0 && !wrote.contains_key(&id) && remembered.get(&key) == Some(&now) {
            wrote.insert(id, now);
        }
        let before = observed.insert(id, now);
        let Some(&placed) = wrote.get(&id) else { continue };
        if before != Some(placed) || now == placed {
            continue;
        }
        moves.push(if now != 0 {
            DesktopMove::Channel { key, id, channel: now }
        } else {
            DesktopMove::Away { key, id }
        });
    }
    moves
}

/// Routes needed to honor remembered channels for streams not yet settled.
/// Each stream is settled once (or after repeated failures), so a later manual
/// move from a desktop volume control is respected until the user assigns
/// the app again. Streams whose identity has not arrived yet wait.
pub fn pending_routes(
    streams: &[Stream],
    remembered: &BTreeMap<String, u8>,
    handled: &BTreeSet<(u32, u64)>,
) -> Vec<(Stream, u8)> {
    streams
        .iter()
        .filter(|s| s.info && s.movable && !handled.contains(&(s.node, s.serial)))
        .filter_map(|s| {
            let channel = *remembered.get(&s.app_key())?;
            (s.channel != channel || s.target_channel != channel).then(|| (s.clone(), channel))
        })
        .collect()
}

/// A connected Studio found through sysfs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Studio {
    pub sysfs: PathBuf,
    pub descriptors: Vec<u8>,
    /// USB bus/device numbers; change on every replug.
    pub address: (u16, u16),
    pub card: Option<u32>,
}
impl Studio {
    fn same_device(&self, other: &Studio) -> bool {
        self.sysfs == other.sysfs
            && self.address == other.address
            && self.descriptors == other.descriptors
    }
}
pub fn find_studio() -> Option<Studio> {
    use std::{fs, io::Read};
    let mut found = Vec::new();
    for entry in fs::read_dir("/sys/bus/usb/devices").ok()?.take(512).flatten() {
        let path = entry.path();
        let read = |name: &str| fs::read_to_string(path.join(name)).unwrap_or_default();
        if read("idVendor").trim() == "33ae" && read("idProduct").trim() == "4003" {
            if let Ok(canonical) = path.canonicalize() {
                found.push(canonical);
            }
        }
    }
    if found.len() != 1 {
        return None;
    }
    let sysfs = found.remove(0);
    let number = |name: &str| -> Option<u16> {
        fs::read_to_string(sysfs.join(name)).ok()?.trim().parse().ok()
    };
    let address = (number("busnum")?, number("devnum")?);
    let mut descriptors = Vec::new();
    fs::File::open(sysfs.join("descriptors"))
        .ok()?
        .take((crate::usb_descriptors::MAX_DESCRIPTOR_BYTES + 1) as u64)
        .read_to_end(&mut descriptors)
        .ok()?;
    let card = card_for(&sysfs);
    Some(Studio {
        sysfs,
        descriptors,
        address,
        card,
    })
}
fn card_for(sysfs: &Path) -> Option<u32> {
    let mut cards = Vec::new();
    for entry in std::fs::read_dir("/sys/class/sound").ok()?.flatten() {
        let name = entry.file_name();
        let Some(number) = name
            .to_str()
            .and_then(|n| n.strip_prefix("card"))
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        if entry
            .path()
            .join("device")
            .canonicalize()
            .is_ok_and(|p| p.starts_with(sysfs))
        {
            cards.push(number);
        }
    }
    (cards.len() == 1).then(|| cards[0])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Control {
    /// No Studio connected.
    #[default]
    Absent,
    Connecting,
    /// USB control channel open (BEACN App may or may not be answering).
    Connected,
    /// Failed; retried automatically.
    Retrying,
}

#[derive(Debug, Clone, Default)]
pub struct State {
    pub bus: BusStatus,
    pub apps: Vec<App>,
    pub studio: bool,
    pub control: Control,
    /// BEACN App on the streaming PC answered within the last five seconds.
    pub host: bool,
    pub control_error: Option<String>,
    pub last_reply: Option<&'static str>,
    pub exchanges: u64,
    pub beacn: Beacn,
    /// Desktop audio outputs, LinX's channels included.
    pub outputs: Vec<Output>,
    /// The output the desktop currently uses as its default (node.name).
    pub default_output: Option<String>,
    /// Sound output is a device on this PC: LinX channels are not in use.
    pub local_output: bool,
}

/// What LinX and BEACN App last exchanged, for diagnostics only.
#[derive(Debug, Clone, Default)]
pub struct Beacn {
    /// Name and Link In (0 = unassigned) LinX publishes, in wire order.
    pub published: Vec<(String, u8)>,
    /// Records BEACN App last returned, and when (Unix milliseconds).
    pub received: Vec<(String, u8)>,
    pub received_at_ms: Option<u64>,
    /// When BEACN App last answered an exchange (Unix milliseconds).
    pub host_seen_at_ms: Option<u64>,
    /// Recent remote assignments, oldest first.
    pub remote: Vec<RemoteAssignment>,
}

#[derive(Debug, Clone)]
pub struct RemoteAssignment {
    pub at_ms: u64,
    pub key: String,
    pub channel: u8,
    pub error: Option<String>,
}

const REMOTE_LOG: usize = 8;

fn pairs(records: &[Record]) -> Vec<(String, u8)> {
    records
        .iter()
        .map(|r| {
            // Received names are NUL-terminated inside a fixed-width field.
            let name = r.name.split(|&b| b == 0).next().unwrap_or_default();
            (String::from_utf8_lossy(name).into_owned(), r.assignment.byte())
        })
        .collect()
}

fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

const CONTROL_RETRY: Duration = Duration::from_secs(3);
const CONTROL_CADENCE: Duration = Duration::from_millis(500);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(2);
const STUDIO_SCAN: Duration = Duration::from_secs(2);

enum ControlEvent {
    State(Control, Option<String>),
    /// Exchange completed; true when the reply proves BEACN App is present.
    Reply(&'static str, bool),
    Remote(Vec<Record>, Vec<Record>),
}
/// The BEACN Link control channel runs on its own thread so USB timeouts
/// (BEACN App closed, streaming PC asleep) never delay audio routing.
struct ControlThread {
    studio: Arc<Mutex<Option<Studio>>>,
    records: Arc<Mutex<Vec<Record>>>,
    events: mpsc::Receiver<ControlEvent>,
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}
impl ControlThread {
    fn spawn() -> Self {
        let studio = Arc::new(Mutex::new(None::<Studio>));
        let records = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, events) = mpsc::channel();
        let (s, r, e) = (studio.clone(), records.clone(), stop.clone());
        let handle = std::thread::Builder::new()
            .name("linx-control".into())
            .spawn(move || control_loop(s, r, e, tx))
            .ok();
        Self {
            studio,
            records,
            events,
            stop,
            handle,
        }
    }
    fn signal_stop(&self) {
        self.stop.store(true, Ordering::Release);
    }
}
impl Drop for ControlThread {
    fn drop(&mut self) {
        self.signal_stop();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
fn control_loop(
    studio: Arc<Mutex<Option<Studio>>>,
    records: Arc<Mutex<Vec<Record>>>,
    stop: Arc<AtomicBool>,
    events: mpsc::Sender<ControlEvent>,
) {
    let mut session: Option<(Studio, Session<crate::native_usb::ControlUsb>)> = None;
    let mut retry = Instant::now();
    let mut state = Control::Absent;
    let report = |state: &mut Control, next: Control, error: Option<String>| {
        if *state != next || error.is_some() {
            *state = next;
            let _ = events.send(ControlEvent::State(next, error));
        }
    };
    while !stop.load(Ordering::Acquire) {
        let current = studio.lock().ok().and_then(|s| s.clone());
        let same = match (&session, &current) {
            (Some((open, _)), Some(now)) => open.same_device(now),
            (None, _) => true,
            _ => false,
        };
        if !same {
            session = None;
            retry = Instant::now();
        }
        let Some(device) = current else {
            session = None;
            report(&mut state, Control::Absent, None);
            std::thread::sleep(CONTROL_CADENCE);
            continue;
        };
        if session.is_none() {
            if Instant::now() < retry {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            report(&mut state, Control::Connecting, None);
            let opened = crate::native_usb::open_control(&device.sysfs, &device.descriptors)
                .map_err(|e| e.to_string())
                .and_then(|usb| {
                    let mut s = Session::new_with_poll_policy(
                        usb,
                        crate::usb_handshake::MAX_TIMEOUT,
                        PollPolicy::ReportRejectedMarker,
                    )
                    .map_err(|e| e.to_string())?;
                    s.initialize_with_read_timeout(HANDSHAKE_TIMEOUT)
                        .map_err(|e| e.to_string())?;
                    Ok(s)
                });
            match opened {
                Ok(s) => {
                    eprintln!("LinX control channel connected");
                    session = Some((device, s));
                    report(&mut state, Control::Connected, None);
                }
                Err(error) => {
                    eprintln!("LinX control channel unavailable: {error}");
                    report(&mut state, Control::Retrying, Some(error));
                    retry = Instant::now() + CONTROL_RETRY;
                    continue;
                }
            }
        }
        let started = Instant::now();
        let list = records.lock().map(|r| r.clone()).unwrap_or_default();
        let (_, s) = session.as_mut().expect("opened above");
        let result = (|| {
            s.publish(&list).map_err(|e| e.to_string())?;
            std::thread::sleep(Duration::from_millis(100));
            s.poll().map_err(|e| e.to_string())
        })();
        match result {
            Ok(reply) => {
                let (label, host) = match reply {
                    PollReply::NoPayload { .. } => ("no payload", true),
                    PollReply::RejectedMarker { .. } => ("rejected marker", false),
                    PollReply::Records { records, .. } => {
                        let _ = events.send(ControlEvent::Remote(list.clone(), records));
                        ("assignments", true)
                    }
                };
                let _ = events.send(ControlEvent::Reply(label, host));
            }
            Err(error) => {
                eprintln!("LinX control exchange failed: {error}");
                session = None;
                report(&mut state, Control::Retrying, Some(error));
                retry = Instant::now() + CONTROL_RETRY;
                continue;
            }
        }
        std::thread::sleep(CONTROL_CADENCE.saturating_sub(started.elapsed()));
    }
}

/// The LinX channel the desktop default points at while LinX is the output.
const LINX_DEFAULT_OUTPUT: &str = "linx_link_in_1";

/// One of LinX's own channel outputs (LinX Link Out .. Link 4 Out).
pub fn is_linx_output(name: &str) -> bool {
    name.starts_with("linx_link_in_")
}

/// Owner of one LinX connection. Dropping it returns moved applications to
/// their previous output and releases every owned object.
pub struct LinkService {
    bus: LinkBus,
    remembered: BTreeMap<String, u8>,
    /// Output each app used before LinX first moved it (node.name).
    previous: BTreeMap<String, String>,
    /// Streams settled for the current routing generation.
    handled: BTreeSet<(u32, u64)>,
    attempts: BTreeMap<(u32, u64), u8>,
    generation: Option<u64>,
    /// Local assignments published as intended until the graph confirms them.
    intent: BTreeMap<String, (u8, Instant)>,
    /// Link In channel LinX last wrote as each stream's target.
    wrote: BTreeMap<(u32, u64), u8>,
    /// Target channel each stream was last observed with (0 = not LinX).
    observed: BTreeMap<(u32, u64), u8>,
    studio: Option<Studio>,
    control: ControlThread,
    studio_scan: Instant,
    host_seen: Option<Instant>,
    /// Sound output is a device on this PC (not LinX): assigned applications
    /// play there too and LinX applies no channels until it returns.
    local: bool,
    /// `local` was taken from the desktop default at connection.
    local_known: bool,
    /// Placements that override the remembered channel: a device on this PC,
    /// or a channel that stays in use while sound otherwise plays locally.
    pinned: BTreeMap<String, Target>,
    /// Outputs present at the last observation, to re-apply device
    /// placements when one comes back.
    present: BTreeSet<String>,
    state: State,
}

impl LinkService {
    pub fn connect(
        remembered: BTreeMap<String, u8>,
        pinned: BTreeMap<String, Target>,
    ) -> Result<Self, String> {
        let mut service = Self {
            bus: LinkBus::open()?,
            remembered,
            pinned,
            present: BTreeSet::new(),
            previous: BTreeMap::new(),
            handled: BTreeSet::new(),
            wrote: BTreeMap::new(),
            observed: BTreeMap::new(),
            attempts: BTreeMap::new(),
            generation: None,
            intent: BTreeMap::new(),
            studio: None,
            control: ControlThread::spawn(),
            studio_scan: Instant::now(),
            host_seen: None,
            local: false,
            local_known: false,
            state: State::default(),
        };
        service.tick()?;
        Ok(service)
    }
    pub fn state(&self) -> &State {
        &self.state
    }
    pub fn remembered(&self) -> &BTreeMap<String, u8> {
        &self.remembered
    }
    pub fn pinned(&self) -> &BTreeMap<String, Target> {
        &self.pinned
    }

    /// Where an application should play: its placement, else (while LinX is
    /// the output, or its placed device is unplugged) its remembered channel.
    /// `None` follows the default.
    fn effective(&self, key: &str) -> Option<Target> {
        match self.pinned.get(key) {
            Some(Target::Device(name)) if !self.present.contains(name) => {
                (!self.local).then(|| self.remembered.get(key).map(|c| Target::Channel(*c))).flatten()
            }
            Some(target) => Some(target.clone()),
            None if !self.local => self.remembered.get(key).map(|c| Target::Channel(*c)),
            None => None,
        }
    }
    pub fn meter(&self, channel: u8) -> Option<Arc<Mailbox>> {
        self.bus.meter(channel)
    }
    pub fn out_meter(&self, channel: u8) -> Option<Arc<Mailbox>> {
        self.bus.out_meter(channel)
    }

    fn scan_studio(&mut self) {
        if Instant::now() < self.studio_scan {
            return;
        }
        self.studio_scan = Instant::now() + STUDIO_SCAN;
        let found = find_studio();
        if found != self.studio {
            let card = found.as_ref().and_then(|s| s.card);
            if self.studio.as_ref().and_then(|s| s.card) != card
                || found.as_ref().map(|s| s.address) != self.studio.as_ref().map(|s| s.address)
            {
                eprintln!("LinX Studio card: {card:?}");
                self.bus.set_card(card);
            }
            if let Ok(mut studio) = self.control.studio.lock() {
                *studio = found.clone();
            }
            self.studio = found;
        }
    }

    fn intent_map(&mut self) -> BTreeMap<String, u8> {
        let apps = &self.state.apps;
        self.intent.retain(|key, (channel, at)| {
            at.elapsed() < LOCAL_GRACE
                && apps
                    .iter()
                    .find(|a| &a.key == key)
                    .is_some_and(|a| a.channel != *channel)
        });
        self.intent.iter().map(|(k, (c, _))| (k.clone(), *c)).collect()
    }

    /// Hand the current list to the control thread and collect its results.
    fn exchange(&mut self) -> Vec<(String, u8)> {
        let intent = self.intent_map();
        let list = records_for(&self.state.apps, &intent);
        self.state.beacn.published = pairs(&list);
        if let Ok(mut records) = self.control.records.lock() {
            *records = list;
        }
        let mut commands: Vec<(String, u8)> = Vec::new();
        while let Ok(event) = self.control.events.try_recv() {
            match event {
                ControlEvent::State(control, error) => {
                    self.state.control = control;
                    if control != Control::Connected {
                        self.host_seen = None;
                    }
                    if error.is_some() || control == Control::Connected {
                        self.state.control_error = error;
                    }
                }
                ControlEvent::Reply(label, host) => {
                    self.state.exchanges += 1;
                    self.state.last_reply = Some(label);
                    if host {
                        self.host_seen = Some(Instant::now());
                        self.state.beacn.host_seen_at_ms = Some(unix_ms());
                    }
                }
                ControlEvent::Remote(published, received) => {
                    self.state.beacn.received = pairs(&received);
                    self.state.beacn.received_at_ms = Some(unix_ms());
                    for command in remote_commands(&self.state.apps, &published, &received) {
                        if !commands.iter().any(|(k, _)| k == &command.0) {
                            commands.push(command);
                        }
                    }
                }
            }
        }
        let host = self
            .host_seen
            .is_some_and(|at| at.elapsed() < HOST_TIMEOUT);
        if host != self.state.host {
            eprintln!("LinX BEACN App {}", if host { "connected" } else { "not answering" });
        }
        self.state.host = host;
        commands
    }

    fn observe(&mut self) -> Result<(), String> {
        let snapshot = self.bus.snapshot()?;
        let live: BTreeSet<_> = snapshot.streams.iter().map(|s| (s.node, s.serial)).collect();
        self.handled.retain(|id| live.contains(id));
        self.attempts.retain(|id, _| live.contains(id));
        if self.generation != Some(snapshot.status.generation) {
            // Metadata or a sink was recreated: LinX's earlier targets are
            // gone, so every remembered channel must be applied again.
            self.generation = Some(snapshot.status.generation);
            self.handled.clear();
            self.attempts.clear();
        }
        self.state.bus = snapshot.status;
        self.state.apps = applications(&snapshot.streams, &self.remembered);
        for app in &mut self.state.apps {
            app.pinned = self.pinned.get(&app.key).cloned();
        }
        let present: BTreeSet<String> = snapshot.outputs.iter().map(|o| o.name.clone()).collect();
        if present != self.present {
            // A device came or went: placements on it are applied again.
            for app in &self.state.apps {
                if matches!(self.pinned.get(&app.key), Some(Target::Device(_))) {
                    for s in &app.streams {
                        self.handled.remove(&(s.node, s.serial));
                    }
                }
            }
            self.present = present;
        }
        self.state.outputs = snapshot.outputs;
        self.state.default_output = snapshot.default_output;
        // On connecting, the default the user chose says where sound plays
        // (the effective one falls back elsewhere while LinX's outputs are
        // still being created): a device on this PC keeps LinX's channels
        // unused until LinX is chosen.
        if !self.local_known && self.state.bus.metadata {
            if let Some(name) = snapshot.configured_output.as_ref().or(self.state.default_output.as_ref()) {
                self.local = !is_linx_output(name);
                self.local_known = true;
                self.state.local_output = self.local;
            }
        }
        Ok(())
    }

    /// One maintenance cycle: heal the bus, re-apply remembered channels to
    /// new streams, then apply remote assignments. Ordinary desktop changes
    /// and transient PipeWire stalls never stop audio; fails only if the
    /// PipeWire connection itself is gone.
    pub fn tick(&mut self) -> Result<(), String> {
        self.scan_studio();
        if !self.bus.maintain()? {
            eprintln!("LinX: PipeWire busy; keeping current routes");
            self.exchange();
            return Ok(());
        }
        if let Err(error) = self.observe() {
            eprintln!("LinX observation skipped: {error}");
            return Ok(());
        }
        if self.state.bus.sinks_ready == CHANNELS && self.state.bus.metadata {
            let streams: Vec<Stream> = self
                .state
                .apps
                .iter()
                .flat_map(|a| a.streams.iter().cloned())
                .collect();
            if !self.local {
                self.learn_desktop_moves(&streams);
            }
            let mut channels = BTreeMap::new();
            let mut devices = BTreeMap::new();
            for app in &self.state.apps {
                match self.effective(&app.key) {
                    Some(Target::Channel(c)) => {
                        channels.insert(app.key.clone(), c);
                    }
                    Some(Target::Device(name)) => {
                        devices.insert(app.key.clone(), name);
                    }
                    None => {}
                }
            }
            let routes = pending_routes(&streams, &channels, &self.handled);
            // Device placements: once per stream, while the device exists
            // (a missing device keeps the stream on the default until it
            // returns).
            let mut placed = Vec::new();
            for stream in streams.iter().filter(|s| s.info && s.movable) {
                let id = (stream.node, stream.serial);
                let Some(name) = devices.get(&stream.app_key()) else { continue };
                if self.handled.contains(&id) || !self.present.contains(name) {
                    continue;
                }
                if stream.target_channel == 0 && self.bus.target_name(stream).as_deref() == Some(name) {
                    continue;
                }
                placed.push((stream.clone(), name.clone()));
            }
            for (stream, name) in &placed {
                let id = (stream.node, stream.serial);
                self.wrote.remove(&id);
                match self.bus.route_named(stream, name) {
                    Ok(()) => {
                        self.handled.insert(id);
                    }
                    Err(error) => {
                        let tries = self.attempts.entry(id).or_default();
                        *tries += 1;
                        eprintln!("LinX placement failed ({tries}/{ROUTE_ATTEMPTS}): {error}");
                        if *tries >= ROUTE_ATTEMPTS {
                            self.handled.insert(id);
                        }
                    }
                }
            }
            for (stream, channel) in &routes {
                let id = (stream.node, stream.serial);
                self.remember_previous(stream);
                match self.bus.route(stream, *channel) {
                    Ok(()) => {
                        self.handled.insert(id);
                        self.wrote.insert(id, *channel);
                    }
                    Err(error) => {
                        let tries = self.attempts.entry(id).or_default();
                        *tries += 1;
                        eprintln!("LinX route failed ({tries}/{ROUTE_ATTEMPTS}): {error}");
                        if *tries >= ROUTE_ATTEMPTS {
                            self.handled.insert(id);
                        }
                    }
                }
            }
            // Streams that need nothing are settled once their identity is
            // known; those waiting for a placed device stay open.
            for stream in streams.iter().filter(|s| s.info) {
                let waiting = devices.get(&stream.app_key()).is_some_and(|n| !self.present.contains(n));
                let id = (stream.node, stream.serial);
                if !waiting
                    && !routes.iter().any(|(r, _)| (r.node, r.serial) == id)
                    && !placed.iter().any(|(p, _)| (p.node, p.serial) == id)
                {
                    self.handled.insert((stream.node, stream.serial));
                }
            }
            if !routes.is_empty() || !placed.is_empty() {
                let _ = self.observe();
            }
        }
        self.state.studio = self.studio.is_some();
        for (key, channel) in self.exchange() {
            eprintln!("LinX remote assignment: {key} -> {channel}");
            let error = self.assign(&key, channel).err();
            if let Some(error) = &error {
                eprintln!("LinX remote assignment failed: {error}");
            }
            let log = &mut self.state.beacn.remote;
            log.push(RemoteAssignment { at_ms: unix_ms(), key, channel, error });
            if log.len() > REMOTE_LOG {
                log.remove(0);
            }
        }
        Ok(())
    }

    /// Assign an application (all of its current and future streams) to
    /// Link In 1..4, or 0 to return it to the output it used before LinX.
    pub fn assign(&mut self, key: &str, channel: u8) -> Result<(), String> {
        if channel as usize > CHANNELS {
            return Err("Link In channel must be 1–4.".into());
        }
        if channel == 0 {
            self.remembered.remove(key);
            self.intent.remove(key);
        } else {
            self.remembered.insert(key.to_owned(), channel);
            self.intent.insert(key.to_owned(), (channel, Instant::now()));
        }
        match self.pinned.remove(key) {
            // Off its device: onto the channel, or the default.
            Some(Target::Device(_)) => self.reroute(key, channel == 0 || self.local),
            // Kept on the Studio while sound plays locally: move it there.
            Some(Target::Channel(_)) if self.local && channel != 0 => {
                self.pinned.insert(key.to_owned(), Target::Channel(channel));
                self.reroute(key, false);
            }
            Some(Target::Channel(_)) if self.local => self.reroute(key, true),
            _ => {}
        }
        if self.local {
            // Sound plays on this PC: keep the choice for when LinX is the
            // output again.
            return Ok(());
        }
        let streams: Vec<Stream> = self
            .state
            .apps
            .iter()
            .filter(|a| a.key == key)
            .flat_map(|a| a.streams.iter().cloned())
            .collect();
        let mut result = Ok(());
        for stream in streams.iter().filter(|s| s.movable) {
            if channel != 0 && stream.channel == channel && stream.target_channel == channel {
                continue;
            }
            if channel == 0 && stream.channel == 0 && stream.target_channel == 0 {
                continue;
            }
            if channel != 0 {
                self.remember_previous(stream);
            }
            let routed = match (channel, self.previous.get(key).cloned()) {
                (0, Some(name)) => self.bus.route_named(stream, &name),
                _ => self.bus.route(stream, channel),
            };
            if let Err(error) = routed {
                result = Err(error);
            }
            let id = (stream.node, stream.serial);
            self.handled.insert(id);
            if channel == 0 {
                self.wrote.remove(&id);
            } else {
                self.wrote.insert(id, channel);
            }
        }
        if channel == 0 {
            self.previous.remove(key);
        }
        let _ = self.observe();
        result
    }

    /// Adopt moves made outside LinX (a desktop volume control): a stream
    /// that had settled where LinX put it and is now targeted elsewhere by
    /// someone else changes its application's remembered channel, or makes
    /// LinX forget the application when moved off LinX, so later streams of
    /// that application (new tabs, resumed media) follow the user's choice.
    fn learn_desktop_moves(&mut self, streams: &[Stream]) {
        for change in desktop_moves(streams, &self.remembered, &mut self.wrote, &mut self.observed) {
            match change {
                DesktopMove::Channel { key, id, channel } => {
                    eprintln!("LinX: {key} moved to Link channel {channel} outside LinX; remembering it");
                    self.remembered.insert(key.clone(), channel);
                    self.intent.remove(&key);
                    self.wrote.insert(id, channel);
                }
                DesktopMove::Away { key, id } => {
                    // Only an explicit target on another output counts; a
                    // cleared target is not a user choice.
                    let Some(stream) = streams.iter().find(|s| (s.node, s.serial) == id) else {
                        continue;
                    };
                    let Some(name) = self.bus.target_name(stream) else { continue };
                    eprintln!("LinX: {key} moved to {name} outside LinX; no longer assigned");
                    self.remembered.remove(&key);
                    self.intent.remove(&key);
                    self.previous.remove(&key);
                    self.wrote.remove(&id);
                }
            }
        }
    }

    /// Record the output an app used before LinX first moves one of its
    /// streams, whichever path (local, remote, remembered) moves it.
    fn remember_previous(&mut self, stream: &Stream) {
        let key = stream.app_key();
        if stream.target_channel == 0 && !self.previous.contains_key(&key) {
            if let Some(name) = self.bus.target_name(stream) {
                self.previous.insert(key, name);
            }
        }
    }

    /// Set a Link In channel's send level (volume 0..1, mute). Session-manager
    /// state keeps it for the next connection like any device volume.
    pub fn set_level(
        &mut self,
        channel: u8,
        volume: Option<f32>,
        mute: Option<bool>,
    ) -> Result<(), String> {
        self.bus.set_level(channel, volume, mute)
    }

    /// Choose where sound plays. A LinX channel (any of them) means the whole
    /// LinX setup: the default becomes LinX Link Out and every assigned
    /// application returns to its own channel. A device on this PC takes
    /// everything, assigned applications included; their channels are kept
    /// for when LinX is chosen again.
    pub fn set_default_output(&mut self, name: &str) -> Result<(), String> {
        if !self.state.outputs.iter().any(|o| o.name == name) {
            return Err("That output is no longer available.".into());
        }
        // Choosing a sound output sends everything there: placements end.
        let pinned: Vec<String> = std::mem::take(&mut self.pinned).into_keys().collect();
        for key in &pinned {
            self.reroute(key, true);
        }
        if is_linx_output(name) {
            self.bus.set_default_output(LINX_DEFAULT_OUTPUT)?;
            self.local = false;
            // Re-apply every remembered channel on the next cycle.
            self.handled.clear();
            self.attempts.clear();
        } else {
            self.bus.set_default_output(name)?;
            self.local = true;
            let streams: Vec<Stream> = self
                .state
                .apps
                .iter()
                .flat_map(|a| a.streams.iter().cloned())
                .filter(|s| s.movable && s.target_channel != 0)
                .collect();
            for stream in streams {
                // A cleared target follows the new default.
                let _ = self.bus.route(&stream, 0);
                self.wrote.remove(&(stream.node, stream.serial));
            }
        }
        self.state.local_output = self.local;
        let _ = self.observe();
        Ok(())
    }

    /// Place one application: on Link In 1..4, on a device of this PC, or
    /// (`None`) nowhere, following the default. A channel placement made
    /// while sound plays locally keeps that application on the Studio.
    pub fn place(&mut self, key: &str, target: Option<Target>) -> Result<(), String> {
        match target {
            None => {
                let was = self.pinned.remove(key);
                let result = self.assign(key, 0);
                if was.is_some() {
                    self.reroute(key, true);
                }
                result
            }
            Some(Target::Channel(channel)) => {
                if !(1..=CHANNELS as u8).contains(&channel) {
                    return Err("Link In channel must be 1–4.".into());
                }
                self.pinned.remove(key);
                if self.local {
                    self.remembered.insert(key.to_owned(), channel);
                    self.intent.insert(key.to_owned(), (channel, Instant::now()));
                    self.pinned.insert(key.to_owned(), Target::Channel(channel));
                    self.reroute(key, false);
                    let _ = self.observe();
                    Ok(())
                } else {
                    self.assign(key, channel)
                }
            }
            Some(Target::Device(name)) => {
                if is_linx_output(&name) {
                    return self.place(key, Some(Target::Channel(1)));
                }
                if !self.state.outputs.iter().any(|o| o.name == name) {
                    return Err("That output is no longer available.".into());
                }
                self.intent.remove(key);
                self.pinned.insert(key.to_owned(), Target::Device(name));
                self.reroute(key, false);
                let _ = self.observe();
                Ok(())
            }
        }
    }

    /// Place every application at once. A device becomes the sound output
    /// (everything plays there, new applications too); a channel puts every
    /// current application on it with LinX as the output; `None` unlinks
    /// every application.
    pub fn place_all(&mut self, target: Option<Target>) -> Result<(), String> {
        let keys: Vec<String> = self.state.apps.iter().map(|a| a.key.clone()).collect();
        match target {
            Some(Target::Device(name)) => self.set_default_output(&name),
            Some(Target::Channel(channel)) => {
                if !(1..=CHANNELS as u8).contains(&channel) {
                    return Err("Link In channel must be 1–4.".into());
                }
                for key in &keys {
                    self.remembered.insert(key.clone(), channel);
                    self.intent.insert(key.clone(), (channel, Instant::now()));
                }
                self.set_default_output(LINX_DEFAULT_OUTPUT)
            }
            None => {
                for key in &keys {
                    self.pinned.remove(key);
                    self.remembered.remove(key);
                    self.intent.remove(key);
                    self.reroute(key, true);
                }
                self.previous.clear();
                let _ = self.observe();
                Ok(())
            }
        }
    }

    /// Move an application's streams to where it belongs now. `follow`
    /// clears LinX's target so they play on the default output instead.
    fn reroute(&mut self, key: &str, follow: bool) {
        let target = if follow { None } else { self.effective(key) };
        let streams: Vec<Stream> = self
            .state
            .apps
            .iter()
            .filter(|a| a.key == key)
            .flat_map(|a| a.streams.iter().cloned())
            .filter(|s| s.movable)
            .collect();
        for stream in streams {
            let id = (stream.node, stream.serial);
            let routed = match &target {
                Some(Target::Channel(c)) => {
                    self.remember_previous(&stream);
                    self.wrote.insert(id, *c);
                    self.bus.route(&stream, *c)
                }
                Some(Target::Device(name)) => {
                    self.wrote.remove(&id);
                    self.bus.route_named(&stream, name)
                }
                None => {
                    self.wrote.remove(&id);
                    self.bus.route(&stream, 0)
                }
            };
            if let Err(error) = routed {
                eprintln!("LinX: {error}");
            }
            self.handled.insert(id);
        }
    }

    /// Return every stream LinX routed to its previous output (or the desktop
    /// default). Channel memory is kept for the next connection.
    pub fn release(&mut self) {
        let _ = self.observe();
        let streams: Vec<Stream> = self
            .state
            .apps
            .iter()
            .flat_map(|a| a.streams.iter().cloned())
            .filter(|s| s.movable && (s.target_channel != 0 || s.channel != 0))
            .collect();
        for stream in streams {
            let routed = match self.previous.get(&stream.app_key()).cloned() {
                Some(name) => self.bus.route_named(&stream, &name),
                None => self.bus.route(&stream, 0),
            };
            if let Err(error) = routed {
                eprintln!("LinX release: {error}");
            }
        }
        self.previous.clear();
    }
}
impl Drop for LinkService {
    fn drop(&mut self) {
        // Stop USB first so the bus closes without waiting on the device.
        self.control.signal_stop();
        self.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn stream(node: u32, app: &str, channel: u8, running: bool) -> Stream {
        Stream {
            node,
            serial: node as u64 * 10,
            client: 1,
            pid: 2,
            running,
            channel,
            movable: true,
            target_channel: channel,
            info: true,
            app: app.into(),
            binary: String::new(),
            media: String::new(),
            node_name: String::new(),
        }
    }
    fn rec(name: &[u8], a: u8) -> Record {
        Record {
            name: name.to_vec(),
            assignment: Assignment::from_byte(a),
        }
    }
    #[test]
    fn groups_streams_and_reports_mixed_channels_as_zero() {
        let streams = [
            stream(1, "Firefox", 2, true),
            stream(2, "Firefox", 2, false),
            stream(3, "Discord", 1, false),
            stream(4, "Discord", 0, true),
        ];
        let apps = applications(&streams, &BTreeMap::from([("Firefox".into(), 2)]));
        assert_eq!(apps.len(), 2);
        let discord = &apps[0];
        let firefox = &apps[1];
        assert_eq!((discord.key.as_str(), discord.channel), ("Discord", 0));
        assert_eq!((firefox.channel, firefox.remembered), (2, Some(2)));
        assert!(firefox.running && discord.running);
    }
    #[test]
    fn wire_names_are_printable_bounded_distinct_and_stable() {
        assert_eq!(wire_name("Spotify \u{266b}"), b"Spotify ?".to_vec());
        assert_eq!(wire_name("   "), b"App".to_vec());
        assert_eq!(wire_name(&"x".repeat(100)).len(), MAX_WIRE_NAME);
        let a = stream(1, "Spotify \u{266b}", 0, true);
        let b = stream(2, "Spotify \u{266a}", 0, true);
        let apps = applications(&[a.clone(), b], &BTreeMap::new());
        assert_eq!(apps.len(), 2);
        assert_ne!(apps[0].wire_name, apps[1].wire_name);
        // The suffix does not depend on which other apps exist.
        let c = stream(3, "Spotify \u{2669}", 0, true);
        let again = applications(&[a.clone(), c], &BTreeMap::new());
        let name = |apps: &[App]| apps.iter().find(|x| x.key == a.app).unwrap().wire_name.clone();
        assert_eq!(name(&apps), name(&again));
    }
    #[test]
    fn records_publish_only_playing_apps_assigned_first_within_one_frame() {
        let mut streams = vec![
            stream(1, "Idle", 0, false),
            stream(2, "Playing", 0, true),
            stream(3, "Assigned", 3, true),
            stream(4, "PausedAssigned", 2, false),
        ];
        for i in 0..40 {
            streams.push(stream(10 + i, &format!("Filler application {i:02}"), 0, true));
        }
        let apps = applications(&streams, &BTreeMap::new());
        let out = records_for(&apps, &BTreeMap::new());
        assert_eq!(out[0].name, b"Assigned".to_vec());
        assert_eq!(out[0].assignment.byte(), 3);
        assert!(!out.iter().any(|r| r.name == b"Idle" || r.name == b"PausedAssigned"));
        assert!(crate::encode_records(&out).is_ok());
        assert!(out.len() < apps.len());
        // Local intent is published before the graph confirms it.
        let intent = BTreeMap::from([("Playing".to_string(), 4)]);
        let out = records_for(&apps, &intent);
        assert!(out.iter().any(|r| r.name == b"Playing" && r.assignment.byte() == 4));
    }
    #[test]
    fn diagnostic_names_stop_at_the_field_terminator() {
        // As received from BEACN App: NUL, then stale bytes in the field.
        let received = [rec(b"Brave\0\xff\xfe\x10\x20", 2), rec(b"Spotify", 0)];
        assert_eq!(
            pairs(&received),
            vec![("Brave".to_owned(), 2), ("Spotify".to_owned(), 0)]
        );
    }
    #[test]
    fn desktop_moves_are_learned_only_from_settled_placements() {
        let stream = |target: u8| Stream {
            node: 7,
            serial: 70,
            info: true,
            movable: true,
            app: "Brave".into(),
            target_channel: target,
            channel: target,
            ..Default::default()
        };
        let remembered: BTreeMap<String, u8> = [("Brave".to_owned(), 4)].into();
        let (mut wrote, mut observed) = (BTreeMap::new(), BTreeMap::new());
        // Restored onto its remembered channel: counts as LinX's placement.
        assert!(desktop_moves(&[stream(4)], &remembered, &mut wrote, &mut observed).is_empty());
        // The user moves it to the speakers: LinX must let go.
        assert_eq!(
            desktop_moves(&[stream(0)], &remembered, &mut wrote, &mut observed),
            vec![DesktopMove::Away { key: "Brave".into(), id: (7, 70) }]
        );
        // Or to another Link channel: remembered there.
        let (mut wrote, mut observed) = (BTreeMap::new(), BTreeMap::new());
        desktop_moves(&[stream(4)], &remembered, &mut wrote, &mut observed);
        assert_eq!(
            desktop_moves(&[stream(2)], &remembered, &mut wrote, &mut observed),
            vec![DesktopMove::Channel { key: "Brave".into(), id: (7, 70), channel: 2 }]
        );
    }
    #[test]
    fn linx_reassignment_in_flight_is_not_a_desktop_move() {
        let stream = |target: u8| Stream {
            node: 7,
            serial: 70,
            info: true,
            movable: true,
            app: "Brave".into(),
            target_channel: target,
            ..Default::default()
        };
        let mut remembered: BTreeMap<String, u8> = [("Brave".to_owned(), 4)].into();
        let (mut wrote, mut observed) = (BTreeMap::new(), BTreeMap::new());
        desktop_moves(&[stream(4)], &remembered, &mut wrote, &mut observed);
        // LinX (or BEACN App) reassigns to 1; the snapshot still shows 4.
        remembered.insert("Brave".into(), 1);
        wrote.insert((7, 70), 1);
        assert!(desktop_moves(&[stream(4)], &remembered, &mut wrote, &mut observed).is_empty());
        // The write lands: settled, still nothing to learn.
        assert!(desktop_moves(&[stream(1)], &remembered, &mut wrote, &mut observed).is_empty());
        // Streams that never had a LinX placement are left alone.
        let (mut wrote, mut observed) = (BTreeMap::new(), BTreeMap::new());
        let none = BTreeMap::new();
        desktop_moves(&[stream(0)], &none, &mut wrote, &mut observed);
        assert!(desktop_moves(&[stream(3)], &none, &mut wrote, &mut observed).is_empty());
    }
    #[test]
    fn remote_commands_follow_windows_semantics() {
        let apps = applications(
            &[stream(1, "Firefox", 1, true), stream(2, "Discord", 0, true)],
            &BTreeMap::new(),
        );
        let published = vec![rec(b"Firefox", 1), rec(b"Discord", 0)];
        let out = remote_commands(
            &apps,
            &published,
            &[
                rec(b"Firefox", 1),       // what we published: no-op
                rec(b"Discord\0junk", 4), // C-string name
                rec(b"Unknown", 2),
                rec(b"Firefox", 9), // invalid channel
            ],
        );
        assert_eq!(out, vec![("Discord".to_string(), 4)]);
        // Zero never unassigns (ignored by BEACN Link on Windows).
        assert!(remote_commands(&apps, &published, &[rec(b"Firefox", 0)]).is_empty());
        // A stale echo of our own table does not revert a local change.
        let after_local = vec![rec(b"Firefox", 3)];
        assert_eq!(
            remote_commands(&apps, &after_local, &[rec(b"Firefox", 3)]),
            vec![]
        );
    }
    #[test]
    fn remembered_channel_is_applied_once_to_each_new_identified_stream() {
        let remembered = BTreeMap::from([("Firefox".to_string(), 3)]);
        let mut handled = BTreeSet::new();
        let streams = [stream(1, "Firefox", 0, true), stream(2, "Discord", 0, true)];
        let routes = pending_routes(&streams, &remembered, &handled);
        assert_eq!(routes.len(), 1);
        assert_eq!((routes[0].0.node, routes[0].1), (1, 3));
        handled.insert((1, 10));
        assert!(pending_routes(&streams, &remembered, &handled).is_empty());
        // A new stream (media resume, new tab) gets the remembered channel.
        let resumed = [stream(5, "Firefox", 0, true)];
        assert_eq!(pending_routes(&resumed, &remembered, &handled).len(), 1);
        // Already there, not movable, or identity not known yet: nothing to do.
        let mut fixed = stream(6, "Firefox", 0, true);
        fixed.movable = false;
        let mut unknown = stream(8, "Firefox", 0, true);
        unknown.info = false;
        assert!(pending_routes(
            &[stream(7, "Firefox", 3, true), fixed, unknown],
            &remembered,
            &handled
        )
        .is_empty());
        // Playing there but the metadata target was lost: route again.
        let mut lost = stream(9, "Firefox", 3, true);
        lost.target_channel = 0;
        assert_eq!(pending_routes(&[lost], &remembered, &handled).len(), 1);
    }
}

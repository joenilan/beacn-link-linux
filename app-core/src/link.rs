//! LinX Link worker: owns one `LinkService` on a background thread and
//! publishes a plain view for the UI. Channel choices persist per application
//! so media pauses, new streams and restarts keep their Link In channel.
use beacn_link_protocol::{
    link_bus::{LinkOut, Transport},
    link_engine::{is_linx_output, Beacn, Control, LinkService},
    metering::Mailbox,
};
pub use beacn_link_protocol::link_engine::Target;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender, TrySendError},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub const CHANNELS: u8 = 4;
/// The Sound output entry that stands for LinX's four channels together.
pub const LINX_OUTPUT: &str = "BEACN Studio (LinX)";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkApp {
    pub key: String,
    pub name: String,
    /// Observed Link In channel (1..4), or 0 for the desktop output.
    pub channel: u8,
    /// Channel LinX re-applies to this app's new streams.
    pub remembered: Option<u8>,
    /// Placement overriding the remembered channel (a device on this PC).
    pub pinned: Option<Target>,
    pub running: bool,
    pub streams: usize,
    /// `application.icon-name`, when the application gives one.
    pub icon: String,
    /// Process binary, to find an icon when it gives none.
    pub binary: String,
    /// Its own volume in thousandths (1000 = 100%), when known.
    pub volume: Option<u16>,
    pub mute: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Phase {
    /// Not connected yet (startup or waiting for PipeWire).
    #[default]
    Starting,
    /// Connected: Link In channels exist.
    Running,
    /// Paused by the owner: channels released, apps on desktop output.
    Paused,
    /// PipeWire unavailable; retrying.
    Retrying,
}

#[derive(Clone, Default)]
pub struct LinkView {
    pub phase: Phase,
    pub auto_connect: bool,
    /// A Studio is connected to this PC.
    pub studio: bool,
    pub transport: Transport,
    /// All four channels reach the Studio.
    pub audio_ready: bool,
    pub control: Control,
    /// BEACN App on the streaming PC is answering.
    pub host: bool,
    pub control_error: Option<String>,
    pub audio_error: Option<String>,
    pub message: Option<String>,
    pub apps: Vec<LinkApp>,
    pub meters: [Option<Arc<Mailbox>>; CHANNELS as usize],
    /// What the Studio sends on Link Out 1..4.
    pub out_meters: [Option<Arc<Mailbox>>; CHANNELS as usize],
    pub levels: [beacn_link_protocol::link_bus::Level; CHANNELS as usize],
    pub exchanges: u64,
    pub last_reply: Option<&'static str>,
    /// Diagnostics: what LinX and BEACN App last exchanged.
    pub beacn: Beacn,
    /// The Studio's Link Out pairs as LinX sources.
    pub link_out: LinkOut,
    /// Desktop outputs (LinX's channels included), by description.
    pub outputs: Vec<OutputChoice>,
    /// The desktop's current default output (node.name); LinX's channels
    /// all count as the one LinX entry.
    pub default_output: Option<String>,
    /// Sound plays on a device on this PC; LinX channels are not in use.
    pub local_output: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputChoice {
    /// node.name, what the default refers to.
    pub name: String,
    pub description: String,
}
impl LinkView {
    pub fn apps_on(&self, channel: u8) -> impl Iterator<Item = &LinkApp> {
        self.apps.iter().filter(move |a| a.channel == channel)
    }
    /// One-line status for header, tray and tooltips.
    pub fn status(&self) -> &'static str {
        match self.phase {
            Phase::Starting => "Starting",
            Phase::Paused => "Link paused",
            Phase::Retrying => "Waiting for PipeWire",
            Phase::Running if !self.studio => "Connect BEACN Studio",
            Phase::Running if !self.audio_ready => "Starting Studio audio",
            Phase::Running if self.local_output => "Playing on this PC",
            Phase::Running if !self.host => "Audio linked; BEACN App offline",
            Phase::Running => "Link active",
        }
    }
}

#[derive(Clone, Debug)]
pub enum LinkCommand {
    Connect,
    Pause,
    /// Assign an application key to Link In 1..4, or 0 for desktop output.
    Assign(String, u8),
    /// Set a Link In channel's send level: linear volume 0..1 and/or mute.
    Level {
        channel: u8,
        volume: Option<f32>,
        mute: Option<bool>,
    },
    AutoConnect(bool),
    /// Make a desktop output (node.name) the default.
    DefaultOutput(String),
    /// Place one application (key) on a channel or device, or `None` to
    /// unlink it.
    Place(String, Option<Target>),
    /// Place every application at once (a device becomes the sound output).
    PlaceAll(Option<Target>),
    /// An application's own volume (linear, up to 1.5) and/or mute.
    AppLevel {
        key: String,
        volume: Option<f32>,
        mute: Option<bool>,
    },
    /// Put an application back exactly as it was (undo): its placement and
    /// remembered channel.
    Restore {
        key: String,
        pinned: Option<Target>,
        remembered: Option<u8>,
    },
}

/// Per-application channel memory: `~/.config/linx/channels`.
pub struct ChannelStore {
    path: Option<PathBuf>,
}
impl ChannelStore {
    pub fn user() -> Self {
        Self {
            path: crate::preferences::Preferences::user_path()
                .and_then(|p| p.parent().map(|d| d.join("channels"))),
        }
    }
    pub fn load(&self) -> BTreeMap<String, u8> {
        let Some(path) = &self.path else {
            return BTreeMap::new();
        };
        let Ok(text) = fs::read_to_string(path) else {
            return BTreeMap::new();
        };
        parse_channels(&text)
    }
    pub fn save(&self, channels: &BTreeMap<String, u8>) -> Result<(), String> {
        let path = self.path.as_ref().ok_or("No LinX settings directory.")?;
        let parent = path.parent().ok_or("No LinX settings directory.")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let temp = parent.join(format!(".channels-{}", std::process::id()));
        let mut file = fs::File::create(&temp).map_err(|e| e.to_string())?;
        file.write_all(format_channels(channels).as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        fs::rename(&temp, path).map_err(|e| e.to_string())
    }
}
pub fn format_channels(channels: &BTreeMap<String, u8>) -> String {
    let mut out = String::from("linx-channels=1\n");
    for (key, channel) in channels {
        let hex: String = key.bytes().map(|b| format!("{b:02x}")).collect();
        out.push_str(&format!("{channel}\t{hex}\n"));
    }
    out
}
pub fn parse_channels(text: &str) -> BTreeMap<String, u8> {
    let mut lines = text.lines();
    if lines.next() != Some("linx-channels=1") {
        return BTreeMap::new();
    }
    lines
        .filter_map(|line| {
            let (channel, hex) = line.split_once('\t')?;
            let channel: u8 = channel.parse().ok().filter(|c| (1..=CHANNELS).contains(c))?;
            if hex.len() % 2 != 0 {
                return None;
            }
            let bytes = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
                .collect::<Option<Vec<_>>>()?;
            Some((String::from_utf8(bytes).ok()?, channel))
        })
        .collect()
}

/// Per-application placements on devices of this PC (or a channel kept
/// while sound plays locally): `~/.config/linx/placements`.
pub struct PlacementStore {
    path: Option<PathBuf>,
}
impl PlacementStore {
    pub fn user() -> Self {
        Self {
            path: crate::preferences::Preferences::user_path()
                .and_then(|p| p.parent().map(|d| d.join("placements"))),
        }
    }
    pub fn load(&self) -> (BTreeMap<String, Target>, BTreeSet<String>) {
        self.path
            .as_ref()
            .and_then(|path| fs::read_to_string(path).ok())
            .map(|text| parse_placements(&text))
            .unwrap_or_default()
    }
    pub fn save(&self, placements: &BTreeMap<String, Target>, released: &BTreeSet<String>) -> Result<(), String> {
        let path = self.path.as_ref().ok_or("No LinX settings directory.")?;
        let parent = path.parent().ok_or("No LinX settings directory.")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let temp = parent.join(format!(".placements-{}", std::process::id()));
        let mut file = fs::File::create(&temp).map_err(|e| e.to_string())?;
        file.write_all(format_placements(placements, released).as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        fs::rename(&temp, path).map_err(|e| e.to_string())
    }
}
fn hex(text: &str) -> String {
    text.bytes().map(|b| format!("{b:02x}")).collect()
}
fn unhex(hex: &str) -> Option<String> {
    if hex.is_empty() || hex.len() % 2 != 0 {
        return None;
    }
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
        .collect::<Option<Vec<_>>>()?;
    String::from_utf8(bytes).ok()
}
/// Placements, then applications released from a device while closed
/// (`released\t-\t<key>`).
pub fn format_placements(placements: &BTreeMap<String, Target>, released: &BTreeSet<String>) -> String {
    let mut out = String::from("linx-placements=1\n");
    for key in released {
        out.push_str(&format!("released\t-\t{}\n", hex(key)));
    }
    for (key, target) in placements {
        match target {
            Target::Channel(c) => out.push_str(&format!("channel\t{c}\t{}\n", hex(key))),
            Target::Device(name) => {
                out.push_str(&format!("device\t{}\t{}\n", hex(name), hex(key)))
            }
        }
    }
    out
}
pub fn parse_placements(text: &str) -> (BTreeMap<String, Target>, BTreeSet<String>) {
    let mut lines = text.lines();
    let (mut placements, mut released) = (BTreeMap::new(), BTreeSet::new());
    if lines.next() != Some("linx-placements=1") {
        return (placements, released);
    }
    for line in lines {
        let mut parts = line.split('\t');
        let (Some(kind), Some(value), Some(key)) = (parts.next(), parts.next(), parts.next()) else { continue };
        let Some(key) = unhex(key) else { continue };
        match kind {
            "channel" => {
                if let Some(c) = value.parse().ok().filter(|c| (1..=CHANNELS).contains(c)) {
                    placements.insert(key, Target::Channel(c));
                }
            }
            "device" => {
                if let Some(name) = unhex(value) {
                    placements.insert(key, Target::Device(name));
                }
            }
            "released" => {
                released.insert(key);
            }
            _ => {}
        }
    }
    (placements, released)
}

/// Sound outputs the user hid from LinX's lists (by node.name):
/// `~/.config/linx/hidden-outputs`. Hiding only affects LinX's menus.
pub struct HiddenOutputs {
    path: Option<PathBuf>,
}
impl HiddenOutputs {
    pub fn user() -> Self {
        Self {
            path: crate::preferences::Preferences::user_path()
                .and_then(|p| p.parent().map(|d| d.join("hidden-outputs"))),
        }
    }
    pub fn load(&self) -> std::collections::BTreeSet<String> {
        self.path
            .as_ref()
            .and_then(|path| fs::read_to_string(path).ok())
            .map(|text| parse_hidden(&text))
            .unwrap_or_default()
    }
    pub fn save(&self, hidden: &std::collections::BTreeSet<String>) -> Result<(), String> {
        let path = self.path.as_ref().ok_or("No LinX settings directory.")?;
        let parent = path.parent().ok_or("No LinX settings directory.")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let temp = parent.join(format!(".hidden-outputs-{}", std::process::id()));
        let mut file = fs::File::create(&temp).map_err(|e| e.to_string())?;
        file.write_all(format_hidden(hidden).as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        fs::rename(&temp, path).map_err(|e| e.to_string())
    }
}
pub fn format_hidden(hidden: &std::collections::BTreeSet<String>) -> String {
    let mut out = String::from("linx-hidden-outputs=1\n");
    for name in hidden {
        out.extend(name.bytes().map(|b| format!("{b:02x}")));
        out.push('\n');
    }
    out
}
pub fn parse_hidden(text: &str) -> std::collections::BTreeSet<String> {
    let mut lines = text.lines();
    if lines.next() != Some("linx-hidden-outputs=1") {
        return Default::default();
    }
    lines
        .filter_map(|hex| {
            if hex.is_empty() || hex.len() % 2 != 0 {
                return None;
            }
            let bytes = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
                .collect::<Option<Vec<_>>>()?;
            String::from_utf8(bytes).ok()
        })
        .collect()
}

const TICK: Duration = Duration::from_millis(500);
const RETRY: Duration = Duration::from_secs(2);

pub struct LinkWorker {
    tx: SyncSender<LinkCommand>,
    latest: Arc<Mutex<LinkView>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}
impl LinkWorker {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::sync_channel::<LinkCommand>(16);
        let latest = Arc::new(Mutex::new(LinkView::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let view = latest.clone();
        let ending = stop.clone();
        let thread = thread::spawn(move || run(rx, view, ending));
        Self {
            tx,
            latest,
            stop,
            thread: Some(thread),
        }
    }
    pub fn view(&self) -> LinkView {
        self.latest.lock().map(|v| v.clone()).unwrap_or_default()
    }
    pub fn send(&self, command: LinkCommand) -> Result<(), String> {
        self.tx.try_send(command).map_err(|e| match e {
            TrySendError::Full(_) => "LinX is busy; try again.".into(),
            TrySendError::Disconnected(_) => "LinX stopped. Reopen the app.".into(),
        })
    }
    /// Joins the worker; LinX's channels are released before this returns.
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
impl Drop for LinkWorker {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run(rx: mpsc::Receiver<LinkCommand>, latest: Arc<Mutex<LinkView>>, ending: Arc<AtomicBool>) {
    let mut preferences = crate::preferences::Preferences::user_path()
        .ok_or_else(|| "No user configuration directory.".to_string())
        .and_then(crate::preferences::Preferences::load);
    let mut view = LinkView {
        auto_connect: preferences.as_ref().map(|(_, enabled)| *enabled).unwrap_or(true),
        ..Default::default()
    };
    if let Err(error) = &preferences {
        view.message = Some(error.clone());
    }
    let store = ChannelStore::user();
    let mut remembered = store.load();
    let placements = PlacementStore::user();
    let (mut pinned, mut released) = placements.load();
    let mut service: Option<LinkService> = None;
    // Connect at launch only when automatic connection is enabled.
    let mut want = view.auto_connect;
    if !want {
        view.phase = Phase::Paused;
    }
    let mut retry_at = Instant::now();
    let mut next_tick = Instant::now();
    let publish = |view: &LinkView| {
        if let Ok(mut latest) = latest.lock() {
            *latest = view.clone();
        }
    };
    while !ending.load(Ordering::Acquire) {
        match rx.recv_timeout(next_tick.saturating_duration_since(Instant::now())) {
            Ok(command) => match command {
                LinkCommand::Connect => {
                    want = true;
                    retry_at = Instant::now();
                    view.message = None;
                }
                LinkCommand::Pause => {
                    want = false;
                    if let Some(s) = service.take() {
                        remembered = s.remembered().clone();
                        pinned = s.pinned().clone();
                        released = s.released().clone();
                    }
                    view.phase = Phase::Paused;
                    view.message = Some("Link paused. Applications use your normal output.".into());
                }
                LinkCommand::Assign(key, channel) => {
                    let result = match service.as_mut() {
                        Some(s) => s.assign(&key, channel),
                        None => Err("Connect LinX first.".into()),
                    };
                    view.message = result.err();
                }
                LinkCommand::Level {
                    channel,
                    volume,
                    mute,
                } => {
                    let result = match service.as_mut() {
                        Some(s) => s.set_level(channel, volume, mute),
                        None => Err("Connect LinX first.".into()),
                    };
                    view.message = result.err();
                }
                LinkCommand::DefaultOutput(name) => {
                    let result = match service.as_mut() {
                        Some(s) => s.set_default_output(&name),
                        None => Err("Connect LinX first.".into()),
                    };
                    view.message = result.err();
                }
                LinkCommand::Place(key, target) => {
                    let result = match service.as_mut() {
                        Some(s) => s.place(&key, target),
                        None => Err("Connect LinX first.".into()),
                    };
                    view.message = result.err();
                }
                LinkCommand::PlaceAll(target) => {
                    let result = match service.as_mut() {
                        Some(s) => s.place_all(target),
                        None => Err("Connect LinX first.".into()),
                    };
                    view.message = result.err();
                }
                LinkCommand::AppLevel { key, volume, mute } => {
                    // Scrolling sends many; a stale one is not worth a message.
                    if let Some(s) = service.as_mut() {
                        let _ = s.set_app_level(&key, volume, mute);
                    }
                }
                LinkCommand::Restore { key, pinned, remembered } => {
                    let result = match service.as_mut() {
                        Some(s) => s.restore(&key, pinned, remembered),
                        None => Err("Connect LinX first.".into()),
                    };
                    view.message = result.err();
                }
                LinkCommand::AutoConnect(enabled) => {
                    view.auto_connect = enabled;
                    view.message = match preferences.as_mut() {
                        Ok((store, _)) => store.save(enabled).err(),
                        Err(error) => Some(error.clone()),
                    };
                }
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        if Instant::now() < next_tick && service.is_some() {
            // Commands were handled; refresh the view right away below.
        } else {
            next_tick = Instant::now() + TICK;
            if want && service.is_none() && Instant::now() >= retry_at {
                match LinkService::connect(remembered.clone(), pinned.clone(), released.clone()) {
                    Ok(s) => {
                        eprintln!("LinX connected");
                        service = Some(s);
                    }
                    Err(error) => {
                        eprintln!("LinX connect failed: {error}");
                        view.phase = Phase::Retrying;
                        view.audio_error = Some(error);
                        retry_at = Instant::now() + RETRY;
                    }
                }
            } else if let Some(s) = service.as_mut() {
                if let Err(error) = s.tick() {
                    // PipeWire went away: release and reconnect automatically.
                    eprintln!("LinX lost PipeWire: {error}");
                    remembered = s.remembered().clone();
                    pinned = s.pinned().clone();
                    released = s.released().clone();
                    service = None;
                    view.phase = Phase::Retrying;
                    view.audio_error = Some(error);
                    retry_at = Instant::now() + RETRY;
                }
            }
        }
        if let Some(s) = service.as_ref() {
            if s.remembered() != &remembered {
                remembered = s.remembered().clone();
                if let Err(error) = store.save(&remembered) {
                    view.message = Some(format!("Could not save channel choices: {error}"));
                }
            }
            if s.pinned() != &pinned || s.released() != &released {
                pinned = s.pinned().clone();
                released = s.released().clone();
                if let Err(error) = placements.save(&pinned, &released) {
                    view.message = Some(format!("Could not save placements: {error}"));
                }
            }
            fill(&mut view, s);
        } else {
            view.apps.clear();
            view.outputs.clear();
            view.default_output = None;
            view.local_output = false;
            view.meters = Default::default();
            view.out_meters = Default::default();
            view.levels = Default::default();
            view.audio_ready = false;
            view.studio = false;
            view.transport = Transport::Off;
            view.control = Control::Absent;
            view.host = false;
            view.last_reply = None;
            view.beacn = Beacn::default();
            view.link_out = LinkOut::default();
        }
        publish(&view);
    }
    drop(service); // releases channels; streams return to desktop output
    view.phase = Phase::Paused;
    view.apps.clear();
    view.meters = Default::default();
    view.out_meters = Default::default();
    view.last_reply = None;
    view.beacn = Beacn::default();
    view.link_out = LinkOut::default();
    publish(&view);
}

fn fill(view: &mut LinkView, service: &LinkService) {
    let state = service.state();
    view.phase = Phase::Running;
    view.studio = state.studio;
    view.transport = state.bus.transport;
    view.audio_ready = state.bus.complete();
    view.audio_error = state.bus.error.clone();
    view.control = state.control;
    view.host = state.host;
    view.control_error = state.control_error.clone();
    view.exchanges = state.exchanges;
    view.last_reply = state.last_reply;
    view.beacn = state.beacn.clone();
    view.link_out = state.bus.link_out.clone();
    view.levels = state.bus.levels;
    // The desktop's own outputs, then LinX as one device: choosing it means
    // the whole LinX setup, not one channel.
    view.outputs = state
        .outputs
        .iter()
        .filter(|o| !is_linx_output(&o.name))
        .map(|o| OutputChoice {
            name: o.name.clone(),
            description: o.description.clone(),
        })
        .collect();
    if let Some(linx) = state.outputs.iter().find(|o| is_linx_output(&o.name)) {
        view.outputs.push(OutputChoice {
            name: linx.name.clone(),
            description: LINX_OUTPUT.into(),
        });
    }
    view.default_output = match &state.default_output {
        Some(name) if is_linx_output(name) => view
            .outputs
            .iter()
            .find(|o| is_linx_output(&o.name))
            .map(|o| o.name.clone()),
        other => other.clone(),
    };
    view.local_output = state.local_output;
    view.apps = state
        .apps
        .iter()
        .map(|a| LinkApp {
            key: a.key.clone(),
            name: a.name.clone(),
            channel: a.channel,
            remembered: a.remembered,
            pinned: a.pinned.clone(),
            running: a.running,
            streams: a.streams.len(),
            icon: a.icon.clone(),
            binary: a.binary.clone(),
            volume: a.volume,
            mute: a.mute,
        })
        .collect();
    for channel in 1..=CHANNELS {
        view.meters[channel as usize - 1] = service.meter(channel);
        view.out_meters[channel as usize - 1] = service.out_meter(channel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn placements_round_trip_and_ignore_bad_lines() {
        let placements = BTreeMap::from([
            ("Spotify".to_string(), Target::Device("alsa_output.pci-0000.analog-stereo".into())),
            ("Game\tX".to_string(), Target::Channel(3)),
        ]);
        let released: BTreeSet<String> = ["Discord".to_string()].into();
        assert_eq!(parse_placements(&format_placements(&placements, &released)), (placements, released));
        assert!(parse_placements("other=1\nchannel\t1\t41").0.is_empty());
        let (parsed, released) = parse_placements(
            "linx-placements=1\nchannel\t9\t41\ndevice\tzz\t41\nwhat\t1\t41\nchannel\t2\t42\nreleased\t-\t43\n",
        );
        assert_eq!(parsed, BTreeMap::from([("B".to_string(), Target::Channel(2))]));
        assert_eq!(released, BTreeSet::from(["C".to_string()]));
    }
    #[test]
    fn hidden_outputs_round_trip_and_ignore_bad_lines() {
        let hidden: std::collections::BTreeSet<String> =
            ["alsa_output.hdmi-stereo".to_owned(), "odd\tname".to_owned()].into();
        assert_eq!(parse_hidden(&format_hidden(&hidden)), hidden);
        assert!(parse_hidden("something-else=1\n6869").is_empty());
        assert_eq!(parse_hidden("linx-hidden-outputs=1\nzz\n686\n6869\n").len(), 1);
    }
    #[test]
    fn channel_memory_round_trips_and_ignores_bad_lines() {
        let channels = BTreeMap::from([
            ("Firefox".to_string(), 2u8),
            ("Tab\tName ü".to_string(), 4u8),
        ]);
        let text = format_channels(&channels);
        assert_eq!(parse_channels(&text), channels);
        assert!(parse_channels("other=1\n2\t41\n").is_empty());
        let parsed = parse_channels("linx-channels=1\n9\t41\n2\tzz\n3\t41\nnot a line\n");
        assert_eq!(parsed, BTreeMap::from([("A".to_string(), 3u8)]));
    }
}

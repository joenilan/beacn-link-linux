//! Explicit one-stream desktop target lease. WirePlumber owns its route links.
//! No default/volume writes, foreign link deletion, USB or PCM payload reads.
use crate::{
    catalog::Catalog,
    execution::Observation,
    executor::{Channel, Direction, GraphSnapshot, ObjectRef},
    native_audio::PinnedPlayback,
    native_catalog::PrivateConfig,
    native_pipewire::parse_observation,
    planner::ApplicationId,
};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    ffi::{c_char, c_int, c_void, CString},
    fs,
    marker::PhantomData,
    os::unix::{
        fs::{FileTypeExt, MetadataExt},
        io::IntoRawFd,
        net::UnixStream,
    },
    path::Path,
    path::PathBuf,
    rc::{Rc, Weak},
    time::{Duration, Instant},
};
const DESKTOP_CLIENT_CONFIG: &str = "context.properties = { support.dbus = false context.num-data-loops = 0 }\ncontext.modules = [ { name = libpipewire-module-protocol-native } { name = libpipewire-module-metadata } ]\ncontext.objects = []\ncontext.exec = []\n";

/// Sealed proof from a live desktop lease, distinct from private link receipts.
/// Zero means the original desktop destination; one means verified Studio output.
pub struct DesktopRoutePublication {
    inventory: crate::planner::Inventory,
    records: Vec<crate::Record>,
    lease: Weak<Cell<u64>>,
    nonce: u64,
}
impl DesktopRoutePublication {
    pub fn inventory(&self) -> &crate::planner::Inventory {
        &self.inventory
    }
    pub fn records(&self) -> &[crate::Record] {
        &self.records
    }
    pub(crate) fn valid(&self) -> bool {
        self.nonce != 0 && self.lease.upgrade().is_some_and(|l| l.get() == self.nonce)
    }
    pub(crate) fn lease(&self) -> Weak<Cell<u64>> {
        self.lease.clone()
    }
    pub(crate) fn nonce(&self) -> u64 {
        self.nonce
    }
}
#[cfg(test)]
pub(crate) fn test_publication(
    inventory: crate::planner::Inventory,
    assignment: u8,
    lease: &Rc<Cell<u64>>,
    nonce: u64,
) -> DesktopRoutePublication {
    DesktopRoutePublication {
        records: vec![crate::Record {
            name: inventory.applications[0].advertised_name.clone(),
            assignment: crate::Assignment::from_byte(assignment),
        }],
        inventory,
        lease: Rc::downgrade(lease),
        nonce,
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Receipt {
    id: u32,
    serial: u64,
}
impl From<ObjectRef> for Receipt {
    fn from(r: ObjectRef) -> Self {
        Self {
            id: r.id,
            serial: r.serial,
        }
    }
}
impl From<Receipt> for ObjectRef {
    fn from(r: Receipt) -> Self {
        Self {
            id: r.id,
            serial: r.serial,
        }
    }
}
unsafe extern "C" {
    fn getuid() -> u32;
    fn bcl_open_desktop_target(
        fd: c_int,
        config: *const c_char,
        cookie: u32,
        name: *const c_char,
        server: u32,
    ) -> *mut c_void;
    fn bcl_snapshot(session: *mut c_void, buffer: *mut c_char, size: usize) -> c_int;
    fn bcl_desktop_sink(session: *mut c_void, owner: *const c_char, result: *mut Receipt) -> c_int;
    fn bcl_desktop_move(
        session: *mut c_void,
        source: *const Receipt,
        client_serial: u64,
        pid: u32,
        original: *const Receipt,
    ) -> c_int;
    fn bcl_desktop_check(session: *mut c_void) -> c_int;
    fn bcl_desktop_stop(session: *mut c_void) -> c_int;
    fn bcl_desktop_release_sink(session: *mut c_void) -> c_int;
    fn bcl_close(session: *mut c_void);
    fn bcl_desktop_daemon_pid(session: *mut c_void) -> u32;
    fn bcl_desktop_find_studio(session: *mut c_void, card: u32, result: *mut Receipt) -> c_int;
    fn bcl_desktop_bridge(
        session: *mut c_void,
        hardware: *const Receipt,
        card: u32,
        pair: c_int,
        owner: *const c_char,
        links: *mut Receipt,
        adapter: *mut Receipt,
    ) -> c_int;
    fn bcl_desktop_switch_pair(session: *mut c_void, pair: c_int, links: *mut Receipt) -> c_int;
    fn bcl_desktop_bridge_check(session: *mut c_void) -> c_int;
    fn bcl_desktop_adapter_ready(session: *mut c_void) -> c_int;
    #[cfg(all(test, feature = "native-test-faults"))]
    fn bcl_desktop_test_metadata_guards() -> c_int;
    #[cfg(all(test, feature = "native-test-faults"))]
    fn bcl_desktop_test_core_identity() -> c_int;
    #[cfg(all(test, feature = "native-test-faults"))]
    fn bcl_desktop_test_pair_mapping() -> c_int;
    #[cfg(feature = "native-test-faults")]
    fn bcl_desktop_test_fail_after_target_echo(session: *mut c_void) -> c_int;
}

/// Identify the complete existing stereo destination without claiming its links.
pub fn stereo_target(graph: &GraphSnapshot, source: ObjectRef) -> Result<ObjectRef, String> {
    let outputs = graph
        .ports
        .iter()
        .filter(|p| p.node == source && p.direction == Direction::Output)
        .collect::<Vec<_>>();
    if outputs.len() != 2
        || !outputs.iter().any(|p| p.channel == Channel::Left)
        || !outputs.iter().any(|p| p.channel == Channel::Right)
    {
        return Err("Desktop selection needs one complete stereo stream.".into());
    }
    let ids = outputs.iter().map(|p| p.object).collect::<BTreeSet<_>>();
    let links = graph
        .links
        .iter()
        .filter(|l| ids.contains(&l.output))
        .collect::<Vec<_>>();
    let mut target = None;
    if links.len() != 2 {
        return Err("Desktop stream has missing or multiple destinations.".into());
    }
    for output in outputs {
        let linked = links
            .iter()
            .filter(|l| l.output == output.object)
            .collect::<Vec<_>>();
        if linked.len() != 1 {
            return Err("Desktop stream fanout refused.".into());
        }
        let input = graph
            .ports
            .iter()
            .find(|p| p.object == linked[0].input)
            .ok_or("Desktop input disappeared.")?;
        if input.direction != Direction::Input
            || input.channel != output.channel
            || target.is_some_and(|t| t != input.node)
        {
            return Err("Desktop stereo route is split or mismatched.".into());
        }
        target = Some(input.node);
    }
    let target = target.ok_or("Desktop destination absent.")?;
    if graph
        .ports
        .iter()
        .filter(|p| p.node == target && p.direction == Direction::Input)
        .count()
        != 2
    {
        return Err("Desktop target needs exactly two inputs.".into());
    }
    Ok(target)
}

fn foreign(mut graph: GraphSnapshot, source: ObjectRef, sink: Option<ObjectRef>) -> GraphSnapshot {
    let outputs = graph
        .ports
        .iter()
        .filter(|p| p.node == source && p.direction == Direction::Output)
        .map(|p| p.object)
        .collect::<BTreeSet<_>>();
    graph.generation = 1;
    graph.links.retain(|l| !outputs.contains(&l.output));
    if let Some(sink) = sink {
        graph.nodes.retain(|n| *n != sink);
        graph.ports.retain(|p| p.node != sink);
    }
    graph.nodes.sort();
    graph.ports.sort_by_key(|p| p.object);
    graph.links.sort_by_key(|l| l.object);
    graph.streams.sort_by_key(|s| s.node);
    graph
}

pub struct DesktopTarget {
    session: *mut c_void,
    _config: PrivateConfig,
    bindings: BTreeMap<String, String>,
    source: ObjectRef,
    original: ObjectRef,
    client_serial: u64,
    pid: u32,
    baseline: Option<GraphSnapshot>,
    sink: Option<ObjectRef>,
    generation: u64,
    failed: bool,
    cleanup: Option<Result<(), String>>,
    server: u32,
    bridge_links: BTreeSet<ObjectRef>,
    adapter: Option<ObjectRef>,
    physical: Option<DesktopPlayback>,
    pair: u8,
    managed_playback: bool,
    expected_apps: Vec<crate::planner::Application>,
    proof_epoch: Rc<Cell<u64>>,
    _single_thread: PhantomData<Rc<()>>,
}
struct DesktopPlayback {
    proof: PinnedPlayback,
    sysfs: PathBuf,
    expected: Vec<u8>,
}
impl DesktopPlayback {
    fn binding(&self) -> Result<(), String> {
        if fs::read(self.sysfs.join("descriptors")).map_err(|e| e.to_string())? != self.expected {
            return Err("Desktop Studio binding changed.".into());
        }
        Ok(())
    }
}
fn publication_pair(pair: u8, bridge_verified: bool) -> Result<u8, String> {
    if !bridge_verified {
        return Ok(0);
    }
    if (1..=4).contains(&pair) {
        Ok(pair)
    } else {
        Err("Desktop publication pair is unsupported.".into())
    }
}
impl DesktopTarget {
    pub fn manage_playback_until_stop(&mut self) {
        self.managed_playback = true;
    }
    /// Resolve one current Studio ALSA sink from a selected sysfs device. Read
    /// only; the retained connection and the later bridge revalidate identity.
    pub fn studio_hardware(&self, sysfs: &Path) -> Result<ObjectRef, String> {
        let selected = sysfs.canonicalize().map_err(|e| e.to_string())?;
        if !selected.starts_with("/sys/devices") {
            return Err("Invalid Studio sysfs path.".into());
        }
        let mut cards = Vec::new();
        for entry in fs::read_dir("/sys/class/sound").map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
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
                .is_ok_and(|p| p.starts_with(&selected))
            {
                cards.push(number);
            }
        }
        if cards.len() != 1 {
            return Err("Studio ALSA card absent or ambiguous.".into());
        }
        let mut receipt = Receipt::default();
        if unsafe { bcl_desktop_find_studio(self.session, cards[0], &mut receipt) } != 0 {
            return Err("Studio playback endpoint absent or ambiguous.".into());
        }
        Ok(receipt.into())
    }
    #[cfg(feature = "native-test-faults")]
    pub fn fail_after_target_echo_for_test(&mut self) -> Result<(), String> {
        if unsafe { bcl_desktop_test_fail_after_target_echo(self.session) } != 0 {
            return Err("Desktop failure injection refused.".into());
        }
        Ok(())
    }
    /// Caller explicitly selects a current application. A fresh native graph
    /// must independently resolve that exact client, PID and sole stream.
    pub fn connect(
        catalog: &Catalog,
        application: &ApplicationId,
        authorized: bool,
    ) -> Result<Self, String> {
        Self::connect_named(catalog, application, "linx-desktop-check", authorized)
    }
    pub fn connect_named(
        catalog: &Catalog,
        application: &ApplicationId,
        wire_name: &str,
        authorized: bool,
    ) -> Result<Self, String> {
        if !authorized {
            return Err("Desktop routing needs explicit authority.".into());
        }
        if std::env::vars_os().any(|(k, _)| k.to_str().is_some_and(|k| k.starts_with("PIPEWIRE_")))
        {
            return Err("Desktop routing refuses PipeWire environment overrides.".into());
        }
        let mut service = crate::catalog::ApplicationService::default();
        service.update(catalog.clone())?;
        service.select(application, wire_name)?;
        let expected_apps = service
            .inventory()
            .ok_or("Desktop selection absent.")?
            .applications
            .clone();
        let app = catalog
            .applications
            .get(application)
            .ok_or("Selected desktop application absent.")?;
        if app.streams.len() != 1 {
            return Err("Desktop qualification currently requires one selected stream.".into());
        }
        let client_serial = application
            .0
            .strip_prefix("pw-client:")
            .and_then(|s| s.split_once(":pid:"))
            .ok_or("Invalid selected client.")?
            .0
            .parse::<u64>()
            .map_err(|_| "Invalid client serial.")?;
        let parts = catalog.core.split(':').collect::<Vec<_>>();
        if parts.len() != 5
            || parts[0] != "pw-core"
            || parts[2] != "pid"
            || parts[4] != "pipewire-0"
        {
            return Err("Desktop routing requires the observed user desktop core.".into());
        }
        let cookie = parts[1]
            .parse::<u32>()
            .map_err(|_| "Invalid desktop cookie.")?;
        let server = parts[3]
            .parse::<u32>()
            .map_err(|_| "Invalid desktop server.")?;
        if server == 0 {
            return Err("Invalid desktop server.".into());
        }
        let uid = unsafe { getuid() };
        let runtime = PathBuf::from(format!("/run/user/{uid}"));
        let m = fs::symlink_metadata(&runtime).map_err(|e| e.to_string())?;
        if !m.is_dir()
            || m.file_type().is_symlink()
            || m.uid() != uid
            || m.mode() & 0o777 != 0o700
            || fs::canonicalize(&runtime).map_err(|e| e.to_string())? != runtime
        {
            return Err("Desktop runtime ownership refused.".into());
        }
        let path = runtime.join("pipewire-0");
        let before = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if !before.file_type().is_socket() || before.uid() != uid {
            return Err("Desktop socket ownership refused.".into());
        }
        let socket = UnixStream::connect(&path).map_err(|e| e.to_string())?;
        let after = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if (before.dev(), before.ino(), before.uid()) != (after.dev(), after.ino(), after.uid()) {
            return Err("Desktop socket changed.".into());
        }
        let config = PrivateConfig::with_text(DESKTOP_CLIENT_CONFIG)?;
        let file = CString::new(
            config
                .directory
                .join("client.conf")
                .to_str()
                .ok_or("Invalid config path.")?,
        )
        .map_err(|e| e.to_string())?;
        let name = CString::new("pipewire-0").unwrap();
        let session = unsafe {
            bcl_open_desktop_target(
                socket.into_raw_fd(),
                file.as_ptr(),
                cookie,
                name.as_ptr(),
                server,
            )
        };
        if session.is_null() {
            return Err("Desktop core/metadata connection refused.".into());
        }
        let daemon = unsafe { bcl_desktop_daemon_pid(session) };
        if daemon == 0 {
            unsafe { bcl_close(session) };
            return Err("Actual desktop daemon identity absent.".into());
        }
        let mut owner = Self {
            session,
            _config: config,
            bindings: BTreeMap::from([(application.0.clone(), wire_name.into())]),
            source: ObjectRef { id: 0, serial: 0 },
            original: ObjectRef { id: 0, serial: 0 },
            client_serial,
            pid: app.pid,
            baseline: None,
            sink: None,
            generation: 0,
            failed: false,
            cleanup: None,
            server: daemon,
            bridge_links: Default::default(),
            adapter: None,
            physical: None,
            pair: 1,
            managed_playback: false,
            expected_apps,
            proof_epoch: Rc::new(Cell::new(0)),
            _single_thread: PhantomData,
        };
        let observed = owner.observe()?;
        if observed.inventory.applications != owner.expected_apps {
            return Err("Desktop stream roster changed before connection.".into());
        }
        let streams = observed
            .graph
            .streams
            .iter()
            .filter(|s| s.application_id == *application && s.stream_id == app.streams[0].id)
            .collect::<Vec<_>>();
        if streams.len() != 1 {
            return Err("Selected native stream identity changed.".into());
        }
        owner.source = streams[0].node;
        owner.original = stereo_target(&observed.graph, owner.source)?;
        owner.baseline = Some(foreign(observed.graph, owner.source, None));
        Ok(owner)
    }
    fn observe(&mut self) -> Result<Observation, String> {
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or("Desktop generation exhausted.")?;
        let mut buf = vec![0; crate::graph_snapshot::MAX_GRAPH_TSV_BYTES + 1];
        if unsafe { bcl_snapshot(self.session, buf.as_mut_ptr().cast(), buf.len()) } != 0 {
            return Err("Desktop graph unavailable.".into());
        }
        let len = buf
            .iter()
            .position(|b| *b == 0)
            .ok_or("Unterminated desktop graph.")?;
        let raw = std::str::from_utf8(&buf[..len]).map_err(|_| "Invalid native graph text.")?;
        parse_observation(raw, &self.bindings, "linx-desktop-target", self.generation)
    }
    fn conserved(&self, graph: &GraphSnapshot) -> Result<(), String> {
        let mut observed = graph.clone();
        observed
            .links
            .retain(|l| !self.bridge_links.contains(&l.object));
        if let Some(adapter) = self.adapter {
            observed.nodes.retain(|n| *n != adapter);
            observed.ports.retain(|p| p.node != adapter);
        }
        if self.baseline.as_ref() != Some(&foreign(observed, self.source, self.sink)) {
            return Err("Foreign desktop graph changed; target lease stopped.".into());
        }
        Ok(())
    }
    pub fn start(&mut self) -> Result<ObjectRef, String> {
        self.proof_epoch.set(0);
        if self.failed || self.sink.is_some() || self.cleanup.is_some() {
            return Err("Desktop lease cannot restart.".into());
        }
        self.failed = true; // every partial start remains latched unless fully verified
        let mut sink = Receipt::default();
        let token = CString::new(format!("linx-desktop-{}", std::process::id())).unwrap();
        if unsafe { bcl_desktop_sink(self.session, token.as_ptr(), &mut sink) } != 0 {
            return Err("Owned desktop sink creation failed.".into());
        }
        self.sink = Some(sink.into());
        let observed = self.observe()?;
        self.conserved(&observed.graph)?;
        if unsafe {
            bcl_desktop_move(
                self.session,
                &self.source.into(),
                self.client_serial,
                self.pid,
                &self.original.into(),
            )
        } != 0
        {
            return Err("Desktop target move was not verified.".into());
        }
        let observed = self.observe()?;
        self.conserved(&observed.graph)?;
        if stereo_target(&observed.graph, self.source)? != sink.into() {
            return Err("Desktop target differs after move.".into());
        }
        self.failed = false;
        Ok(sink.into())
    }
    pub fn check(&mut self) -> Result<(), String> {
        self.proof_epoch.set(0);
        if self.failed || self.sink.is_none() || self.cleanup.is_some() {
            return Err("Desktop lease is stopped or failed.".into());
        }
        self.failed = true;
        if unsafe { bcl_desktop_check(self.session) } != 0 {
            return Err("Desktop identity/route/metadata check failed.".into());
        }
        if let Some(physical) = self.physical.as_mut() {
            physical.binding()?;
            if unsafe { bcl_desktop_bridge_check(self.session) } != 0 {
                return Err("Desktop Studio bridge changed.".into());
            }
            physical.proof.validate().map_err(|_| {
                format!(
                    "Desktop Studio PCM proof refused; {}",
                    physical.proof.diagnostic()
                )
            })?;
        }
        let observed = self.observe()?;
        self.conserved(&observed.graph)?;
        self.failed = false;
        Ok(())
    }
    /// First-pair desktop qualification only. Uses the selected existing ALSA
    /// identity, creating only a retained 96-kHz FLOAT_LE/8ch adapter; no raw opens,
    /// USB transfers, rate/default changes or proprietary app execution.
    pub fn attach_studio(
        &mut self,
        sysfs: &Path,
        expected: &[u8],
        hardware: ObjectRef,
        authorized: bool,
    ) -> Result<(), String> {
        self.proof_epoch.set(0);
        if !authorized
            || self.failed
            || self.physical.is_some()
            || self.sink.is_none()
            || self.cleanup.is_some()
        {
            return Err(
                "Desktop Studio attachment requires a current explicit target lease.".into(),
            );
        }
        self.failed = true;
        crate::usb_handshake::check_descriptors(expected).map_err(|e| e.to_string())?;
        let selected = sysfs.canonicalize().map_err(|e| e.to_string())?;
        if !selected.starts_with("/sys/devices")
            || fs::read(selected.join("descriptors")).map_err(|e| e.to_string())? != expected
            || fs::read_to_string(selected.join("idVendor"))
                .map_err(|e| e.to_string())?
                .trim()
                != "33ae"
            || fs::read_to_string(selected.join("idProduct"))
                .map_err(|e| e.to_string())?
                .trim()
                != "4003"
            || fs::read_to_string("/sys/class/dmi/id/product_name")
                .unwrap_or_default()
                .to_lowercase()
                .contains("vmware")
        {
            return Err("Selected desktop Studio identity refused.".into());
        }
        let mut proof = PinnedPlayback::pin(&selected, self.server)
            .map_err(|_| "Selected desktop Studio PCM preflight refused.")?;
        if self.managed_playback {
            proof.manage_until_stop();
        }
        let card = proof.card();
        self.physical = Some(DesktopPlayback {
            proof,
            sysfs: selected,
            expected: expected.to_vec(),
        });
        let token = CString::new(format!("linx-desktop-{}", std::process::id())).unwrap();
        let mut links = [Receipt::default(); 2];
        let mut adapter = Receipt::default();
        if unsafe {
            bcl_desktop_bridge(
                self.session,
                &hardware.into(),
                card,
                self.pair as c_int,
                token.as_ptr(),
                links.as_mut_ptr(),
                &mut adapter,
            )
        } != 0
        {
            return Err("Desktop Studio monitor bridge not acknowledged.".into());
        }
        self.bridge_links = links.into_iter().map(ObjectRef::from).collect();
        self.adapter = Some(adapter.into());
        if self.bridge_links.len() != 2 {
            return Err("Desktop Studio bridge receipts ambiguous.".into());
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let observed = self.observe()?;
            self.conserved(&observed.graph)?;
            let physical = self.physical.as_mut().unwrap();
            physical.binding()?;
            let ready = unsafe { bcl_desktop_adapter_ready(self.session) };
            if ready == 0 && physical.proof.closed().is_ok() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(20));
                continue;
            }
            if ready != 1 {
                return Err(format!(
                    "Desktop owned adapter did not become RUNNING (state={ready}); {}",
                    physical.proof.diagnostic()
                ));
            }
            match physical.proof.startup_probe() {
                Ok(true) => break,
                Ok(false) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(20))
                }
                _ => {
                    return Err(format!(
                        "Desktop Studio startup refused; {}",
                        physical.proof.diagnostic()
                    ))
                }
            }
        }
        self.failed = false;
        Ok(())
    }
    pub fn physical_stats(&self) -> Option<(u32, u64)> {
        self.physical.as_ref().map(|p| p.proof.stats())
    }
    /// Select one of the four verified stereo pairs. Before Studio attachment
    /// this records the requested pair; during an attached lease it replaces
    /// the exact two retained bridge links and verifies their new destinations.
    pub fn set_pair(&mut self, pair: u8) -> Result<(), String> {
        self.proof_epoch.set(0);
        if !(1..=4).contains(&pair) || self.failed || self.cleanup.is_some() {
            return Err("Desktop stereo-pair change needs a current target lease.".into());
        }
        if self.physical.is_none() {
            self.pair = pair;
            return Ok(());
        }
        if self.sink.is_none() {
            return Err("Desktop pair switch needs an active target sink.".into());
        }
        if pair == self.pair {
            return self.check();
        }

        self.failed = true;
        if unsafe { bcl_desktop_check(self.session) } != 0
            || unsafe { bcl_desktop_bridge_check(self.session) } != 0
        {
            return Err("Desktop pair switch refused by current route proof.".into());
        }
        let mut links = [Receipt::default(); 2];
        if unsafe { bcl_desktop_switch_pair(self.session, pair as c_int, links.as_mut_ptr()) } != 0
        {
            return Err("Desktop stereo-pair switch was not verified.".into());
        }

        // Record fresh C-owner serial receipts before any later check can fail,
        // so stop() can still reconcile precisely what the C owner now owns.
        self.bridge_links = links.into_iter().map(ObjectRef::from).collect();
        self.pair = pair;
        let physical = self.physical.as_mut().expect("checked attached playback");
        physical.binding()?;
        physical.proof.validate().map_err(|_| {
            format!(
                "Desktop Studio PCM proof refused; {}",
                physical.proof.diagnostic()
            )
        })?;
        let observed = self.observe()?;
        self.conserved(&observed.graph)?;
        if unsafe { bcl_desktop_bridge_check(self.session) } != 0 {
            return Err("Desktop stereo-pair bridge changed after switch.".into());
        }
        self.failed = false;
        Ok(())
    }
    pub(crate) fn server_pid(&self) -> u32 {
        self.server
    }
    /// Fresh native verification, never a supplied graph or cached UI status.
    pub fn publication(
        &mut self,
        inventory: &crate::planner::Inventory,
    ) -> Result<DesktopRoutePublication, String> {
        self.proof_epoch.set(0);
        let result = (|| {
            if self.failed || self.cleanup.is_some() || inventory.applications != self.expected_apps
            {
                return Err("Desktop publication roster changed or lease failed.".into());
            }
            let bridge_verified = if self.sink.is_some() {
                if self.physical.is_none() || self.adapter.is_none() || self.bridge_links.len() != 2
                {
                    return Err("Desktop publication requires the verified physical bridge.".into());
                }
                self.check()?;
                true
            } else {
                false
            };
            let assignment = publication_pair(self.pair, bridge_verified)?;
            let observed = self.observe()?;
            self.conserved(&observed.graph)?;
            if observed.inventory.applications != self.expected_apps
                || stereo_target(&observed.graph, self.source)?
                    != self.sink.unwrap_or(self.original)
            {
                return Err("Desktop publication source or target changed.".into());
            }
            let mut tracker = crate::lifecycle::InventoryTracker::default();
            tracker.advance(inventory).map_err(|e| e.to_string())?;
            let records = vec![crate::Record {
                name: self.expected_apps[0].advertised_name.clone(),
                assignment: crate::Assignment::from_byte(assignment),
            }];
            crate::encode_records(&records).map_err(|e| e.to_string())?;
            self.proof_epoch.set(self.generation);
            Ok(DesktopRoutePublication {
                inventory: inventory.clone(),
                records,
                lease: Rc::downgrade(&self.proof_epoch),
                nonce: self.generation,
            })
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    pub fn stop(&mut self) -> Result<(), String> {
        self.proof_epoch.set(0);
        if let Some(result) = &self.cleanup {
            return result.clone();
        }
        self.failed = true;
        let result = (|| {
            if unsafe { bcl_desktop_stop(self.session) } != 0 {
                return Err("Desktop original target restoration unverified.".into());
            }
            if let Some(physical) = &self.physical {
                let deadline = Instant::now() + Duration::from_secs(8);
                loop {
                    physical.binding()?;
                    if physical.proof.closed().is_ok() {
                        break;
                    }
                    if Instant::now() >= deadline {
                        return Err("Desktop Studio PCM did not close after owned unlink.".into());
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
            if self.sink.is_some() && unsafe { bcl_desktop_release_sink(self.session) } != 0 {
                return Err("Owned desktop sink release unverified.".into());
            }
            let observed = self.observe()?;
            self.conserved(&observed.graph)?;
            if stereo_target(&observed.graph, self.source)? != self.original {
                return Err("Original stereo route not restored.".into());
            }
            Ok(())
        })();
        self.cleanup = Some(result.clone());
        result
    }
}
impl Drop for DesktopTarget {
    fn drop(&mut self) {
        if self.sink.is_some() && self.cleanup.is_none() {
            let _ = self.stop();
        }
        unsafe { bcl_close(self.session) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::{ExistingLink, Port};
    #[cfg(feature = "native-test-faults")]
    #[test]
    fn native_metadata_conflicts_latch_and_cannot_restore_over_foreign_intent() {
        assert_eq!(unsafe { bcl_desktop_test_metadata_guards() }, 6);
    }
    #[cfg(feature = "native-test-faults")]
    #[test]
    fn native_core_daemon_is_pinned_separately_from_socket_peer() {
        assert_eq!(unsafe { bcl_desktop_test_core_identity() }, 7);
    }
    fn r(id: u32) -> ObjectRef {
        ObjectRef {
            id,
            serial: id as u64 + 100,
        }
    }
    #[cfg(feature = "native-test-faults")]
    #[test]
    fn native_stereo_pair_indices_cover_exactly_four_stereo_outputs() {
        assert_eq!(unsafe { bcl_desktop_test_pair_mapping() }, 11);
    }
    #[test]
    fn publication_reports_only_a_verified_selected_pair() {
        assert_eq!(publication_pair(1, false), Ok(0));
        assert_eq!(publication_pair(4, false), Ok(0));
        for pair in 1..=4 {
            assert_eq!(publication_pair(pair, true), Ok(pair));
        }
        assert!(publication_pair(0, true).is_err());
        assert!(publication_pair(5, true).is_err());
    }
    fn fixture() -> GraphSnapshot {
        GraphSnapshot {
            core_id: "pw-core:7".into(),
            generation: 1,
            nodes: vec![r(1), r(2), r(3)],
            streams: vec![],
            ports: vec![
                Port {
                    object: r(11),
                    node: r(1),
                    direction: Direction::Output,
                    channel: Channel::Left,
                },
                Port {
                    object: r(12),
                    node: r(1),
                    direction: Direction::Output,
                    channel: Channel::Right,
                },
                Port {
                    object: r(21),
                    node: r(2),
                    direction: Direction::Input,
                    channel: Channel::Left,
                },
                Port {
                    object: r(22),
                    node: r(2),
                    direction: Direction::Input,
                    channel: Channel::Right,
                },
                Port {
                    object: r(31),
                    node: r(3),
                    direction: Direction::Input,
                    channel: Channel::Left,
                },
            ],
            links: vec![
                ExistingLink {
                    object: r(41),
                    output: r(11),
                    input: r(21),
                    owner: None,
                },
                ExistingLink {
                    object: r(42),
                    output: r(12),
                    input: r(22),
                    owner: None,
                },
            ],
        }
    }
    #[test]
    fn complete_route_resolves_without_adopting_foreign_links() {
        let graph = fixture();
        assert_eq!(stereo_target(&graph, r(1)), Ok(r(2)));
        assert!(foreign(graph, r(1), None).links.is_empty());
    }
    #[test]
    fn split_fanout_reused_port_and_partial_route_are_refused() {
        for case in 0..5 {
            let mut graph = fixture();
            match case {
                0 => graph.links[1].input = r(31),
                1 => graph.links.push(ExistingLink {
                    object: r(43),
                    output: r(11),
                    input: r(31),
                    owner: None,
                }),
                2 => graph.ports[0].object.serial += 1,
                3 => {
                    graph.links.pop();
                }
                _ => graph.ports[3].channel = Channel::Left,
            }
            assert!(stereo_target(&graph, r(1)).is_err(), "case {case}");
        }
    }
    #[test]
    fn foreign_guard_ignores_only_selected_route_and_owned_sink() {
        let graph = fixture();
        let baseline = foreign(graph.clone(), r(1), None);
        let mut changed = graph.clone();
        changed.generation = 99;
        changed.links[0].object = r(61);
        assert_eq!(baseline, foreign(changed, r(1), None));
        let mut changed = graph.clone();
        changed.nodes[2].serial += 1;
        assert_ne!(baseline, foreign(changed, r(1), None));
        let mut changed = graph;
        changed.links.push(ExistingLink {
            object: r(60),
            output: r(31),
            input: r(22),
            owner: Some("linx-desktop-target".into()),
        });
        assert_ne!(baseline, foreign(changed, r(1), None));
    }
    #[test]
    fn missing_authority_refuses_before_connection() {
        let catalog = Catalog {
            generation: 1,
            core: String::new(),
            applications: Default::default(),
        };
        assert!(DesktopTarget::connect(&catalog, &ApplicationId("unused".into()), false).is_err());
    }
}

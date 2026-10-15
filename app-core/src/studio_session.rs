//! Explicit bounded hardware qualification. No default desktop/USB factory.
//! The existing supervisor bridge remains available for earlier evidence.
//! Explicit native-playback-test owns the adapter in-process with no IPC.
use beacn_link_protocol::{
    catalog::ApplicationService,
    execution::{Backend, Effect, Observation},
    executor::{Operation, Ownership},
    link_session::{LinkSession, OpenFailure, Options, Phase, Status},
    native_routing_lease::NativeRoutingLease,
    native_usb::{self, NativeUsb, OpenError},
    receipts::Acknowledgment,
    routing_control::{CleanupReport, OwnedRoutingBackend, StopReason},
    session::{PollPolicy, ReceivedNamePolicy},
    usb_handshake::{Handshake, Publication},
};
use std::{
    cell::RefCell,
    io::Write,
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct Configuration {
    pub routing: crate::private_session::Configuration,
    pub sysfs: PathBuf,
    pub expected: Vec<u8>,
    pub pair: u8,
    pub physical: bool,
    pub playback: Playback,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Playback {
    #[default]
    Supervisor,
    #[cfg(feature = "native-playback-test")]
    NativeStudio,
    #[cfg(feature = "native-playback-test")]
    NativeSynthetic,
}
#[cfg(feature = "native-playback-test")]
pub(crate) fn native_supervisor_closed() -> bool {
    supervisor_ended(0)
}
#[cfg(feature = "native-playback-test")]
fn supervisor_ended(fd: i32) -> bool {
    #[repr(C)]
    struct PollFd {
        fd: i32,
        events: i16,
        revents: i16,
    }
    unsafe extern "C" {
        fn poll(fds: *mut PollFd, count: usize, timeout: i32) -> i32;
    }
    // Native playback has no stdin command/ack protocol. EOF, input or an FD
    // error requests orderly owner cleanup; this never reads a payload.
    let mut descriptor = PollFd {
        fd,
        events: 1,
        revents: 0,
    };
    (unsafe { poll(&mut descriptor, 1, 0) }) != 0
}
#[cfg(feature = "native-playback-test")]
struct NativeBridge {
    config: beacn_link_protocol::native_playback::Configuration,
    output: Option<beacn_link_protocol::native_playback::Output>,
    owner: Option<beacn_link_protocol::native_playback::NativePlayback>,
}
#[derive(Default)]
struct Bridge {
    pair: u8,
    active: bool,
    attempted: bool,
    cleanup_error: Option<String>,
    #[cfg(feature = "native-playback-test")]
    native: Option<NativeBridge>,
}
impl Bridge {
    fn exchange(&mut self, start: bool) -> Result<(), String> {
        self.exchange_with(start, supervisor_exchange)
    }
    fn exchange_with(
        &mut self,
        start: bool,
        exchange: impl FnOnce(u8, bool) -> Result<(), String>,
    ) -> Result<(), String> {
        exchange(self.pair, start)?;
        self.active = start;
        Ok(())
    }
    fn start(
        &mut self,
        _proof: Option<&beacn_link_protocol::route_publication::VerifiedRoutePublication>,
    ) -> Result<(), String> {
        self.begin()?;
        #[cfg(feature = "native-playback-test")]
        if let Some(native) = &mut self.native {
            let proof = _proof.ok_or("Native playback needs fresh route proof.")?;
            match beacn_link_protocol::native_playback::NativePlayback::start(
                native.config.clone(),
                native
                    .output
                    .take()
                    .ok_or("Native playback output already consumed.")?,
                proof,
            ) {
                Ok(owner) => {
                    let receipt = owner.receipt().ok_or("Native playback receipt absent.")?;
                    println!(
                        "NATIVE PLAYBACK ACTIVE {} node={}:{} links={}",
                        self.pair,
                        receipt.node.id,
                        receipt.node.serial,
                        receipt
                            .ownership
                            .links
                            .iter()
                            .map(|r| format!("{}:{}", r.id, r.serial))
                            .collect::<Vec<_>>()
                            .join(",")
                    );
                    native.owner = Some(owner);
                }
                Err(error) => {
                    self.active = false;
                    if let Err(cleanup) = error.cleanup {
                        self.cleanup_error = Some(cleanup);
                    }
                    return Err(error.message);
                }
            }
            return Ok(());
        }
        self.exchange(true)
    }
    fn begin(&mut self) -> Result<(), String> {
        if self.attempted {
            return Err("Physical bridge cannot be restarted in this bounded session.".into());
        }
        self.attempted = true;
        // An uncertain start must also request owned cleanup.
        self.active = true;
        Ok(())
    }
    fn stop(&mut self) -> Result<(), String> {
        #[cfg(feature = "native-playback-test")]
        if let Some(native) = &mut self.native {
            if let Some(error) = &self.cleanup_error {
                return Err(error.clone());
            }
            if let Some(mut owner) = native.owner.take() {
                if let Err(error) = owner.stop() {
                    self.cleanup_error = Some(error.clone());
                    return Err(error);
                }
                println!("NATIVE PLAYBACK COMPLETE {}", self.pair);
            }
            self.active = false;
            return Ok(());
        }
        self.stop_with(supervisor_exchange)
    }
    fn check(&mut self) -> Result<(), String> {
        #[cfg(feature = "native-playback-test")]
        if let Some(owner) = self.native.as_mut().and_then(|n| n.owner.as_mut()) {
            owner.check()?;
        }
        Ok(())
    }
    fn native(&self) -> bool {
        #[cfg(feature = "native-playback-test")]
        {
            self.native.is_some()
        }
        #[cfg(not(feature = "native-playback-test"))]
        {
            false
        }
    }
    fn refresh(
        &mut self,
        _proof: &beacn_link_protocol::route_publication::VerifiedRoutePublication,
    ) -> Result<(), String> {
        #[cfg(feature = "native-playback-test")]
        if let Some(owner) = self.native.as_mut().and_then(|n| n.owner.as_mut()) {
            owner.refresh(_proof)?;
        }
        Ok(())
    }
    fn stop_with(
        &mut self,
        exchange: impl FnOnce(u8, bool) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(error) = &self.cleanup_error {
            return Err(error.clone());
        }
        if self.active {
            if let Err(error) = self.exchange_with(false, exchange) {
                self.cleanup_error = Some(error.clone());
                return Err(error);
            }
        }
        Ok(())
    }
}
fn supervisor_exchange(pair: u8, start: bool) -> Result<(), String> {
    let command = if start { "START" } else { "STOP" };
    let expected = format!(
        "PHYSICAL {} {}\n",
        if start { "ACTIVE" } else { "COMPLETE" },
        pair
    );
    println!("PHYSICAL {command} {pair}");
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let reply = read_reply(Duration::from_secs(12))?;
    if reply != expected.as_bytes() {
        return Err("Physical supervisor acknowledgment refused.".into());
    }
    Ok(())
}
/// poll bounds the test IPC; EOF, oversized lines and unexpected text fail closed.
fn read_reply(timeout: Duration) -> Result<Vec<u8>, String> {
    #[repr(C)]
    struct PollFd {
        fd: i32,
        events: i16,
        revents: i16,
    }
    unsafe extern "C" {
        fn poll(fds: *mut PollFd, count: usize, timeout: i32) -> i32;
        fn read(fd: i32, buffer: *mut std::ffi::c_void, count: usize) -> isize;
    }
    let deadline = Instant::now() + timeout;
    let mut line = Vec::new();
    while line.len() < 64 {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("Physical supervisor timed out.".into());
        }
        let mut fd = PollFd {
            fd: 0,
            events: 1,
            revents: 0,
        };
        if unsafe {
            poll(
                &mut fd,
                1,
                remaining.as_millis().min(i32::MAX as u128) as i32,
            )
        } <= 0
        {
            return Err("Physical supervisor unavailable.".into());
        }
        let mut byte = [0];
        if unsafe { read(0, byte.as_mut_ptr().cast(), 1) } != 1 {
            return Err("Physical supervisor closed.".into());
        }
        line.push(byte[0]);
        if byte[0] == b'\n' {
            return Ok(line);
        }
    }
    Err("Physical supervisor line exceeded limit.".into())
}

struct Routes {
    native: NativeRoutingLease,
    bridge: Rc<RefCell<Bridge>>,
}
impl Backend for Routes {
    fn observe(&mut self, generation: u64) -> Result<Observation, String> {
        self.native.observe(generation)
    }
    fn perform(&mut self, operation: &Operation) -> Effect {
        self.native.perform(operation)
    }
    fn reconcile(&mut self) -> Result<Option<Acknowledgment>, String> {
        self.native.reconcile()
    }
}
impl OwnedRoutingBackend for Routes {
    fn ownership(&self) -> &Ownership {
        self.native.ownership()
    }
    fn stop_owned(&mut self) -> Result<CleanupReport, String> {
        // Every failure/quit path passes here before USB release. Always release
        // native handles even if bridge cleanup cannot be confirmed.
        let bridge = self.bridge.borrow_mut().stop();
        let native = self.native.stop_owned();
        bridge?;
        native
    }
}
pub struct Session<B: beacn_link_protocol::usb_handshake::Backend = NativeUsb> {
    config: Configuration,
    link: LinkSession<Routes, Handshake<B>>,
    bridge: Rc<RefCell<Bridge>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "native-playback-test")]
    #[test]
    fn native_owner_cancellation_observes_only_its_retained_supervisor_fd() {
        use std::{
            io::Write,
            os::{fd::AsRawFd, unix::net::UnixStream},
        };
        let (input, mut owner) = UnixStream::pair().unwrap();
        assert!(!supervisor_ended(input.as_raw_fd()));
        owner.write_all(b"stop").unwrap();
        assert!(supervisor_ended(input.as_raw_fd()));
        let (input, owner) = UnixStream::pair().unwrap();
        assert!(!supervisor_ended(input.as_raw_fd()));
        drop(owner);
        assert!(supervisor_ended(input.as_raw_fd()));
    }
    #[test]
    fn uncertain_start_still_requires_cleanup_and_never_restarts() {
        let mut bridge = Bridge {
            pair: 4,
            ..Bridge::default()
        };
        bridge.begin().unwrap();
        assert!(bridge
            .exchange_with(true, |pair, start| {
                assert_eq!(pair, 4);
                assert!(start);
                Err("timeout".into())
            })
            .is_err());
        assert!(bridge.active);
        bridge
            .stop_with(|pair, start| {
                assert_eq!(pair, 4);
                assert!(!start);
                Ok(())
            })
            .unwrap();
        assert!(!bridge.active);
        assert!(bridge.begin().is_err());
        bridge
            .stop_with(|_, _| panic!("duplicate stop sent IPC"))
            .unwrap();
    }
    #[test]
    fn cleanup_failure_is_latched_and_cannot_be_mistaken_for_release() {
        let mut bridge = Bridge {
            pair: 1,
            ..Bridge::default()
        };
        bridge.begin().unwrap();
        assert!(bridge.stop_with(|_, _| Err("unverified".into())).is_err());
        assert!(bridge.active);
        assert!(bridge
            .stop_with(|_, _| panic!("unconfirmed cleanup retried"))
            .is_err());
    }
    #[test]
    fn invalid_roster_is_refused_before_native_device_access() {
        let mut session = Session::new(Configuration {
            routing: crate::private_session::Configuration {
                runtime: PathBuf::from("/no-core"),
                core: "beacn-test-unused".into(),
                cookie: 1,
                server: 1,
                owner: "unused".into(),
                destinations: vec![],
                cycles: 40,
            },
            sysfs: PathBuf::from("/no-device"),
            expected: vec![],
            pair: 1,
            physical: true,
            playback: Playback::Supervisor,
        });
        assert!(session.start(&ApplicationService::default()).is_err());
        assert_eq!(session.status().phase, Phase::Idle);
    }
}
impl<B: beacn_link_protocol::usb_handshake::Backend> Session<B> {
    pub fn new(config: Configuration) -> Self {
        Self {
            config,
            link: Default::default(),
            bridge: Rc::new(RefCell::new(Bridge::default())),
        }
    }
    pub fn status(&self) -> &Status {
        self.link.status()
    }
    fn start_with(
        &mut self,
        service: &ApplicationService,
        transport: impl FnOnce(
            &beacn_link_protocol::planner::Inventory,
            Options,
        ) -> Result<Handshake<B>, OpenFailure>,
    ) -> Result<(), String> {
        let c = &self.config;
        if !(1..=4).contains(&c.pair)
            || service.selected().len() != 1
            || !(9..=40).contains(&c.routing.cycles)
            || service.current().is_none_or(|observed| {
                observed.core
                    != format!(
                        "pw-core:{}:pid:{}:{}",
                        c.routing.cookie, c.routing.server, c.routing.core
                    )
            })
        {
            return Err("Studio qualification requires one current app, exact owned core and bounded options.".into());
        }
        if matches!(
            self.link.status().phase,
            Phase::Running | Phase::NeedsReview
        ) {
            return Err("Studio session is running or cleanup needs review.".into());
        }
        let bridge = Bridge {
            pair: c.pair,
            ..Bridge::default()
        };
        #[cfg(feature = "native-playback-test")]
        let bridge = {
            let mut bridge = bridge;
            if c.playback != Playback::Supervisor {
                if !c.physical {
                    return Err("Native playback requires an explicit playback target.".into());
                }
                use beacn_link_protocol::native_playback::{Configuration, Output};
                let output = match c.playback {
                    Playback::NativeStudio => {
                        Output::pin_studio(&c.sysfs, &c.expected, c.routing.server, true)?
                    }
                    Playback::NativeSynthetic => Output::synthetic(),
                    Playback::Supervisor => unreachable!(),
                };
                bridge.native = Some(NativeBridge {
                    config: Configuration {
                        runtime: c.routing.runtime.clone(),
                        core: c.routing.core.clone(),
                        cookie: c.routing.cookie,
                        server: c.routing.server,
                        owner: c.routing.owner.clone(),
                        pair: c.pair,
                    },
                    output: Some(output),
                    owner: None,
                });
            }
            bridge
        };
        *self.bridge.borrow_mut() = bridge;
        let bridge = self.bridge.clone();
        let bindings = service
            .selected()
            .iter()
            .map(|(id, name)| (id.0.clone(), name.clone()))
            .collect();
        self.link.start(
            service,
            c.routing.destinations.clone(),
            Options {
                cycles: c.routing.cycles,
                poll_policy: PollPolicy::ReportRejectedMarker,
                names: ReceivedNamePolicy::WindowsCString,
                initialization_timeout: Duration::from_secs(5),
            },
            || {
                NativeRoutingLease::connect(
                    &c.routing.runtime,
                    &c.routing.core,
                    c.routing.cookie,
                    c.routing.server,
                    bindings,
                    c.routing.owner.clone(),
                )
                .map(|native| Routes { native, bridge })
            },
            transport,
        )
    }
    pub fn check(&mut self, service: &ApplicationService) -> Result<(), String> {
        self.link.check(service)?;
        let checked = self.bridge.borrow_mut().check();
        match checked {
            Ok(()) => Ok(()),
            Err(error) => self.link.abort(error),
        }
    }
    pub fn tick(&mut self, service: &ApplicationService) -> Result<(), String> {
        // Close physical playback before the last two publication cycles and
        // before the session's bounded completion restores the idle USB profile.
        if self.status().completed_cycles + 2 >= self.config.routing.cycles {
            let stopped = self.bridge.borrow_mut().stop();
            if let Err(error) = stopped {
                return self.link.abort(error);
            }
        }
        self.link.tick(service)?;
        let status = self.status();
        println!(
            "STUDIO CYCLE {} assigned={}",
            status.completed_cycles,
            status
                .routes
                .assignments
                .values()
                .next()
                .copied()
                .unwrap_or(0)
        );
        eprintln!("STUDIO reply={}", status.last_reply.unwrap_or("none"));
        if self.config.physical
            && status.phase == Phase::Running
            && status.completed_cycles + 8 < self.config.routing.cycles
            && status.routes.assignments.len() == 1
            && status
                .routes
                .assignments
                .values()
                .all(|pair| *pair == self.config.pair)
            && !self.bridge.borrow().attempted
        {
            let proof = if self.bridge.borrow().native() {
                Some(self.link.verify_routes(service)?)
            } else {
                None
            };
            let started = self.bridge.borrow_mut().start(proof.as_ref());
            if let Err(error) = started {
                return self.link.abort(error);
            }
        } else if self.bridge.borrow().native() && self.bridge.borrow().active {
            let proof = self.link.verify_routes(service)?;
            let refreshed = self.bridge.borrow_mut().refresh(&proof);
            if let Err(error) = refreshed {
                return self.link.abort(error);
            }
        }
        Ok(())
    }
    pub fn stop(&mut self, reason: StopReason) -> Result<(), String> {
        self.link.stop(reason)
    }
}

impl Session<NativeUsb> {
    pub fn start(&mut self, service: &ApplicationService) -> Result<(), String> {
        #[cfg(feature = "native-playback-test")]
        if self.config.playback == Playback::NativeSynthetic {
            return Err("Synthetic playback cannot open native USB.".into());
        }
        let config = self.config.clone();
        let c = &config;
        self.start_with(service, |inventory, options| {
            let publication =
                Publication::bounded_selected(inventory, options.cycles, options.poll_policy)
                    .map_err(|e| OpenFailure {
                        message: e.to_string(),
                        cleanup: Ok(()),
                    })?
                    .with_initialization_diagnostic();
            let opened = if c.physical {
                native_usb::open_publication_with_owned_playback(
                    &c.sysfs,
                    &c.expected,
                    publication,
                    true,
                    c.routing.server,
                )
            } else {
                native_usb::open_publication_with_startup(
                    &c.sysfs,
                    &c.expected,
                    publication,
                    true,
                    beacn_link_protocol::usb_handshake::Startup::ObservedWindows,
                )
            };
            opened.map_err(|error| {
                let cleanup = match &error {
                    OpenError::Setup(failure) if !failure.cleanup.baseline_restored => {
                        Err("Studio startup cleanup unverified.".into())
                    }
                    _ => Ok(()),
                };
                OpenFailure {
                    message: error.to_string(),
                    cleanup,
                }
            })
        })
    }
}
#[cfg(feature = "native-playback-test")]
impl Session<beacn_link_protocol::scripted_peer::ScriptedPeer> {
    pub fn start_synthetic(&mut self, service: &ApplicationService) -> Result<(), String> {
        if self.config.playback != Playback::NativeSynthetic || !self.config.physical {
            return Err("No-USB native playback requires explicit synthetic output.".into());
        }
        let pair = self.config.pair;
        self.start_with(service, |inventory, options| {
            let publication =
                Publication::bounded_selected(inventory, options.cycles, options.poll_policy)
                    .map_err(|e| OpenFailure {
                        message: e.to_string(),
                        cleanup: Ok(()),
                    })?
                    .with_initialization_diagnostic();
            let records = inventory
                .applications
                .iter()
                .map(|a| beacn_link_protocol::Record {
                    name: a.advertised_name.clone(),
                    assignment: beacn_link_protocol::Assignment::from_byte(pair),
                })
                .collect();
            let (peer, _) = beacn_link_protocol::scripted_peer::ScriptedPeer::new(vec![
                beacn_link_protocol::scripted_peer::Reply::Records(records),
            ])
            .map_err(|message| OpenFailure {
                message,
                cleanup: Ok(()),
            })?;
            Handshake::prepare_publication(peer, publication).map_err(|e| OpenFailure {
                message: e.error.to_string(),
                cleanup: if e.cleanup.baseline_restored {
                    Ok(())
                } else {
                    Err("Synthetic transport cleanup unverified.".into())
                },
            })
        })
    }
}

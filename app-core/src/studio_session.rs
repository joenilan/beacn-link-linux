//! Explicit bounded hardware qualification. No default desktop/USB factory.
//! The Rust session owns native routing and USB. The existing external test
//! supervisor owns the physical bridge, requested only after route verification.
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
}
#[derive(Default)]
struct Bridge {
    pair: u8,
    active: bool,
    attempted: bool,
    cleanup_error: Option<String>,
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
    fn start(&mut self) -> Result<(), String> {
        self.begin()?;
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
        self.stop_with(supervisor_exchange)
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
pub struct Session {
    config: Configuration,
    link: LinkSession<Routes, Handshake<NativeUsb>>,
    bridge: Rc<RefCell<Bridge>>,
}

#[cfg(test)]
mod tests {
    use super::*;
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
        });
        assert!(session.start(&ApplicationService::default()).is_err());
        assert_eq!(session.status().phase, Phase::Idle);
    }
}
impl Session {
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
    pub fn start(&mut self, service: &ApplicationService) -> Result<(), String> {
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
        *self.bridge.borrow_mut() = Bridge {
            pair: c.pair,
            ..Bridge::default()
        };
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
            |inventory, options| {
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
            },
        )
    }
    pub fn check(&mut self, service: &ApplicationService) -> Result<(), String> {
        self.link.check(service)
    }
    pub fn tick(&mut self, service: &ApplicationService) -> Result<(), String> {
        // Close physical playback before the last two publication cycles and
        // before the session's bounded completion restores the idle USB profile.
        if self.status().completed_cycles + 2 >= self.status().cycle_limit {
            let stopped = self.bridge.borrow_mut().stop();
            if let Err(error) = stopped {
                let _ = self.link.stop(StopReason::OperationFailed(error.clone()));
                return Err(error);
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
            && status.completed_cycles + 8 < status.cycle_limit
            && status.routes.assignments.len() == 1
            && status
                .routes
                .assignments
                .values()
                .all(|pair| *pair == self.config.pair)
            && !self.bridge.borrow().attempted
        {
            let started = self.bridge.borrow_mut().start();
            if let Err(error) = started {
                let _ = self.link.stop(StopReason::OperationFailed(error.clone()));
                return Err(error);
            }
        }
        Ok(())
    }
    pub fn stop(&mut self, reason: StopReason) -> Result<(), String> {
        self.link.stop(reason)
    }
}

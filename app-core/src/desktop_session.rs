//! Explicit desktop qualification adapter for the existing worker/UI controls.
use beacn_link_protocol::{
    catalog::ApplicationService,
    desktop_link_session::{DesktopLinkSession, StudioDesktopLease},
    executor::ObjectRef,
    link_session::{OpenFailure, Options, Status},
    native_usb::{self, NativeUsb, OpenError},
    routing_control::{self, StopReason},
    session::{PollPolicy, ReceivedNamePolicy},
    usb_handshake::{self, Handshake, Publication},
};
use std::path::PathBuf;
#[derive(Clone)]
pub struct Configuration {
    pub sysfs: PathBuf,
    pub expected: Vec<u8>,
    pub hardware: ObjectRef,
    pub cycles: usize,
}
pub struct Session {
    config: Configuration,
    owner: DesktopLinkSession<StudioDesktopLease, Handshake<NativeUsb>>,
    status: Status,
    pub physical_stats: Option<(u32, u64)>,
}
impl Session {
    pub fn new(config: Configuration) -> Self {
        Self {
            config,
            owner: Default::default(),
            status: Default::default(),
            physical_stats: None,
        }
    }
    pub fn status(&self) -> &Status {
        &self.status
    }
    pub fn cleanup(&self) -> Option<Result<(), String>> {
        self.owner.status().route_cleanup.clone()
    }
    fn update(&mut self, service: Option<&ApplicationService>, reason: Option<StopReason>) {
        if let Some(routes) = self.owner.routes() {
            if let Some(stats) = routes.physical_stats() {
                self.physical_stats = Some(stats);
            }
        }
        let s = self.owner.status();
        let phase = match s.phase {
            beacn_link_protocol::link_session::Phase::Running => routing_control::Phase::Running,
            beacn_link_protocol::link_session::Phase::NeedsReview => {
                routing_control::Phase::NeedsReview
            }
            beacn_link_protocol::link_session::Phase::Stopped => routing_control::Phase::Stopped,
            _ => routing_control::Phase::Idle,
        };
        self.status = Status {
            phase: s.phase,
            completed_cycles: s.completed_cycles,
            cycle_limit: Some(s.cycle_limit),
            last_reply: s.last_reply,
            error: s.error.clone(),
            transport_cleanup: s.transport_cleanup.clone(),
            routes: routing_control::Status {
                phase,
                reason: reason.or_else(|| s.error.clone().map(StopReason::OperationFailed)),
                // Desktop cleanup has a different proof type; never fabricate private receipt counts.
                cleanup: None,
                observed_revision: service.map(|s| s.revision()),
                verified_revision: service
                    .filter(|_| s.assignment.is_some())
                    .map(|s| s.revision()),
                assignments: service
                    .into_iter()
                    .flat_map(|service| service.selected().keys())
                    .filter_map(|id| s.assignment.map(|a| (id.clone(), a)))
                    .collect(),
            },
        };
    }
    pub fn start(&mut self, service: &ApplicationService) -> Result<(), String> {
        let c = self.config.clone();
        let options = Options {
            cycles: c.cycles,
            poll_policy: PollPolicy::ReportRejectedMarker,
            names: ReceivedNamePolicy::WindowsCString,
            initialization_timeout: usb_handshake::DIAGNOSTIC_INITIALIZATION_TIMEOUT,
        };
        let result = self.owner.start(
            service,
            options,
            || StudioDesktopLease::connect(service, &c.sysfs, &c.expected, c.hardware, true),
            |routes, inventory, options| {
                let publication =
                    Publication::bounded_desktop(inventory, options.cycles, options.poll_policy)
                        .map_err(|e| OpenFailure {
                            message: e.to_string(),
                            cleanup: Ok(()),
                        })?
                        .with_initialization_diagnostic();
                native_usb::open_publication_with_owned_playback(
                    &c.sysfs,
                    &c.expected,
                    publication,
                    true,
                    routes.server_pid(),
                )
                .map_err(|e| {
                    let cleanup = match &e {
                        OpenError::Setup(f) if !f.cleanup.baseline_restored => {
                            Err("Desktop USB startup cleanup unverified.".into())
                        }
                        _ => Ok(()),
                    };
                    OpenFailure {
                        message: e.to_string(),
                        cleanup,
                    }
                })
            },
        );
        self.update(Some(service), None);
        result
    }
    pub fn check(&mut self, service: &ApplicationService) -> Result<(), String> {
        let r = self.owner.check(service);
        self.update(Some(service), None);
        r
    }
    pub fn tick(&mut self, service: &ApplicationService) -> Result<(), String> {
        let r = self.owner.tick(service);
        self.update(Some(service), None);
        r
    }
    pub fn stop(&mut self, reason: StopReason) -> Result<(), String> {
        let r = self.owner.stop();
        self.update(None, Some(reason));
        r
    }
}

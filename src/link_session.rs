//! One explicitly bounded or owner-managed session. No implicit native factories.
use crate::{
    catalog::ApplicationService,
    executor::Destination,
    planner::Inventory,
    routing_control::{
        AssignmentResult, OwnedRoutingBackend, RoutingControl, Status as RouteStatus, StopReason,
    },
    session::{PollPolicy, PublicationCycles, ReceivedNamePolicy, Session},
    usb_handshake::{self, Handshake},
};
use std::time::Duration;

/// Explicit, consuming release of the retained transport. No implicit retry.
pub trait OwnedControlTransport: PublicationCycles {
    fn release(self) -> Result<(), String>;
}
impl<B: usb_handshake::Backend> OwnedControlTransport for Handshake<B> {
    fn release(self) -> Result<(), String> {
        let result = self.close();
        if result.release == Some(Ok(())) && result.baseline_restored {
            Ok(())
        } else {
            Err(format!("control transport release unverified: {result:?}"))
        }
    }
}
pub struct OpenFailure {
    pub message: String,
    pub cleanup: Result<(), String>,
}
#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub cycles: usize,
    pub poll_policy: PollPolicy,
    pub names: ReceivedNamePolicy,
    pub initialization_timeout: Duration,
}
impl Options {
    fn validate(&self) -> Result<(), String> {
        if !(1..=usb_handshake::MAX_PUBLICATION_CYCLES).contains(&self.cycles)
            || ![
                usb_handshake::MAX_TIMEOUT,
                usb_handshake::DIAGNOSTIC_INITIALIZATION_TIMEOUT,
            ]
            .contains(&self.initialization_timeout)
        {
            return Err("bounded session options refused".into());
        }
        Ok(())
    }
}
/// Explicit continuous control policy. No count sentinel can opt a bounded
/// caller into this mode; the owner must select `start_managed` separately.
#[derive(Debug, Clone, Copy)]
pub struct ManagedOptions {
    pub poll_policy: PollPolicy,
    pub names: ReceivedNamePolicy,
    pub initialization_timeout: Duration,
}
impl ManagedOptions {
    fn validate(&self) -> Result<(), String> {
        Options {
            cycles: 1,
            poll_policy: self.poll_policy,
            names: self.names,
            initialization_timeout: self.initialization_timeout,
        }
        .validate()
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Phase {
    #[default]
    Idle,
    Running,
    Stopped,
    NeedsReview,
}
#[derive(Debug, Clone, Default)]
pub struct Status {
    pub phase: Phase,
    pub completed_cycles: usize,
    /// Some for bounded qualification; None for an explicit managed session.
    /// Interpret together with phase: idle sessions have no budget either.
    pub cycle_limit: Option<usize>,
    pub last_reply: Option<&'static str>,
    pub error: Option<String>,
    pub transport_cleanup: Option<Result<(), String>>,
    pub routes: RouteStatus,
}
pub struct LinkSession<B: OwnedRoutingBackend, T: OwnedControlTransport> {
    routing: RoutingControl<B>,
    control: Option<Session<T>>,
    options: Option<ManagedOptions>,
    last_revision: Option<u64>,
    status: Status,
}
impl<B: OwnedRoutingBackend, T: OwnedControlTransport> Default for LinkSession<B, T> {
    fn default() -> Self {
        Self {
            routing: RoutingControl::default(),
            control: None,
            options: None,
            last_revision: None,
            status: Status::default(),
        }
    }
}
impl<B: OwnedRoutingBackend, T: OwnedControlTransport> Drop for LinkSession<B, T> {
    fn drop(&mut self) {
        let _ = self.stop(StopReason::Shutdown);
    }
}
impl<B: OwnedRoutingBackend, T: OwnedControlTransport> LinkSession<B, T> {
    pub fn status(&self) -> &Status {
        &self.status
    }
    pub fn start(
        &mut self,
        service: &ApplicationService,
        destinations: Vec<Destination>,
        options: Options,
        routes: impl FnOnce() -> Result<B, String>,
        transport: impl FnOnce(&Inventory, Options) -> Result<T, OpenFailure>,
    ) -> Result<(), String> {
        options.validate()?;
        self.start_with(
            service,
            destinations,
            Some(options.cycles),
            ManagedOptions {
                poll_policy: options.poll_policy,
                names: options.names,
                initialization_timeout: options.initialization_timeout,
            },
            routes,
            |inventory| transport(inventory, options),
        )
    }
    /// Retain the same route and transport owners until explicit stop or any
    /// failed proof/exchange. No automatic reopen or roster replacement.
    pub fn start_managed(
        &mut self,
        service: &ApplicationService,
        destinations: Vec<Destination>,
        options: ManagedOptions,
        routes: impl FnOnce() -> Result<B, String>,
        transport: impl FnOnce(&Inventory, ManagedOptions) -> Result<T, OpenFailure>,
    ) -> Result<(), String> {
        options.validate()?;
        self.start_with(service, destinations, None, options, routes, |inventory| {
            transport(inventory, options)
        })
    }
    fn start_with(
        &mut self,
        service: &ApplicationService,
        destinations: Vec<Destination>,
        cycle_limit: Option<usize>,
        options: ManagedOptions,
        routes: impl FnOnce() -> Result<B, String>,
        transport: impl FnOnce(&Inventory) -> Result<T, OpenFailure>,
    ) -> Result<(), String> {
        if matches!(self.status.phase, Phase::Running | Phase::NeedsReview) {
            return Err("running or unverified link session cannot start".into());
        }
        self.status = Status {
            cycle_limit,
            ..Status::default()
        };
        self.options = Some(options);
        if let Err(error) = self.routing.start(service, destinations, routes) {
            self.status.error = Some(error.clone());
            let _ = self.stop(StopReason::StartupFailed(error.clone()));
            return Err(error);
        }
        let inventory = service.inventory().ok_or("no selected inventory")?;
        let opened = match transport(inventory) {
            Ok(opened) => opened,
            Err(failure) => {
                self.status.transport_cleanup = Some(failure.cleanup);
                self.status.error = Some(failure.message.clone());
                let _ = self.stop(StopReason::StartupFailed(failure.message.clone()));
                return Err(failure.message);
            }
        };
        let session =
            Session::new_with_poll_policy(opened, usb_handshake::MAX_TIMEOUT, options.poll_policy)
                .expect("fixed nonzero timeout");
        self.control = Some(session);
        if let Err(error) = self
            .control
            .as_mut()
            .unwrap()
            .initialize_with_read_timeout(options.initialization_timeout)
        {
            return self.fail(error.to_string());
        }
        self.status.phase = Phase::Running;
        self.status.routes = self.routing.status().clone();
        self.last_revision = Some(service.revision());
        Ok(())
    }
    /// Check selection/lifecycle even between ticks. No protocol exchange here.
    pub fn check(&mut self, service: &ApplicationService) -> Result<(), String> {
        if self.status.phase != Phase::Running {
            return Ok(());
        }
        if let Err(error) = self.routing.check(service) {
            return self.fail(error);
        }
        self.status.routes = self.routing.status().clone();
        Ok(())
    }
    pub fn tick(&mut self, service: &ApplicationService) -> Result<(), String> {
        if self.status.phase != Phase::Running {
            return Err("link session is not running".into());
        }
        self.check(service)?;
        if self
            .last_revision
            .is_some_and(|previous| service.revision() <= previous)
        {
            return self.fail("fresh application observation required before each cycle".into());
        }
        let result = (|| {
            let proof = self.routing.verify(service)?;
            let control = self.control.as_mut().ok_or("control session absent")?;
            control
                .refresh_verified_publication(&proof)
                .map_err(|e| e.to_string())?;
            control
                .publish(proof.records())
                .map_err(|e| e.to_string())?;
            let reply = control.poll().map_err(|e| e.to_string())?;
            let label =
                match self
                    .routing
                    .assignments(service, &reply, self.options.unwrap().names)?
                {
                    AssignmentResult::NoAssignments => "no assignments",
                    AssignmentResult::Blocked(_) => "assignment blocked",
                    AssignmentResult::Verified(_) => "assignment applied and verified",
                };
            Ok::<_, String>(label)
        })();
        match result {
            Ok(label) => {
                self.last_revision = Some(service.revision());
                self.status.completed_cycles = match self.status.completed_cycles.checked_add(1) {
                    Some(cycles) => cycles,
                    None => return self.fail("session cycle counter exhausted".into()),
                };
                self.status.last_reply = Some(label);
                self.status.routes = self.routing.status().clone();
                if Some(self.status.completed_cycles) == self.status.cycle_limit {
                    self.stop(StopReason::Owner)?;
                }
                Ok(())
            }
            Err(error) => self.fail(error),
        }
    }
    fn fail<R>(&mut self, error: String) -> Result<R, String> {
        self.status.error = Some(error.clone());
        let cleanup = self.stop(StopReason::OperationFailed(error.clone()));
        Err(match cleanup {
            Ok(()) => error,
            Err(e) => format!("{error}; {e}"),
        })
    }
    /// Stop routes before restoring transport ownership, so a future physical
    /// owner must include its playback shutdown in its route-session release.
    pub fn stop(&mut self, reason: StopReason) -> Result<(), String> {
        if self.control.is_none() && self.options.is_none() {
            return match self.status.phase {
                Phase::NeedsReview => Err("cleanup remains unverified".into()),
                _ => Ok(()),
            };
        }
        let routes = self.routing.stop(reason);
        if let Some(mut session) = self.control.take() {
            session.close();
            self.status.transport_cleanup = Some(session.into_transport().release());
        }
        self.options = None;
        self.status.routes = self.routing.status().clone();
        let transport_ok = self
            .status
            .transport_cleanup
            .as_ref()
            .is_none_or(|r| r.is_ok());
        self.status.phase = if routes.is_ok() && transport_ok {
            Phase::Stopped
        } else {
            Phase::NeedsReview
        };
        if self.status.phase == Phase::NeedsReview {
            Err("link cleanup remains unverified; restart blocked".into())
        } else {
            Ok(())
        }
    }
}

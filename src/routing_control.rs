//! Rust start/stop/status and lifecycle orchestration. No native access by default.
use crate::{
    catalog::ApplicationService,
    execution::{Backend, Execution},
    executor::{Destination, Ownership},
    planner::{ApplicationId, Outcome, Resolution},
    publication_epoch::PublicationEpoch,
    route_publication::VerifiedRoutePublication,
    session::{PollReply, ReceivedNamePolicy},
};
use std::collections::BTreeMap;

pub use crate::receipts::ReleaseReport as CleanupReport;
/// A retained owned session. Stop must release its actual native handles even
/// when observation fails, then independently verify cleanup or return Err.
/// Tags/numeric IDs alone never authorize deletion. Implementors supply evidence.
pub trait OwnedRoutingBackend: Backend {
    fn ownership(&self) -> &Ownership;
    fn stop_owned(&mut self) -> Result<CleanupReport, String>;
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Phase {
    #[default]
    Idle,
    Running,
    Stopped,
    NeedsReview,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    Owner,
    Shutdown,
    LifecycleChanged(String),
    OperationFailed(String),
    StartupFailed(String),
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Status {
    pub phase: Phase,
    pub reason: Option<StopReason>,
    pub cleanup: Option<Result<CleanupReport, String>>,
    /// Revision of the last accepted caller observation, not a freshness clock.
    pub observed_revision: Option<u64>,
    /// Revision at which assignments were verified; ordinary polling does not
    /// refresh this proof. Cached status is never permission to publish or apply.
    pub verified_revision: Option<u64>,
    /// Last fully verified data while running. Cleared as soon as the session stops.
    pub assignments: BTreeMap<ApplicationId, u8>,
}
pub enum AssignmentResult {
    NoAssignments,
    /// Opaque/conflicting/unknown records cause no mutation, including no stop.
    Blocked(Vec<Resolution>),
    Verified(VerifiedRoutePublication),
}
struct Active<B> {
    backend: B,
    epoch: PublicationEpoch,
    destinations: Vec<Destination>,
    assignments: BTreeMap<ApplicationId, u8>,
}
pub struct RoutingControl<B: OwnedRoutingBackend> {
    active: Option<Active<B>>,
    status: Status,
}
impl<B: OwnedRoutingBackend> Default for RoutingControl<B> {
    fn default() -> Self {
        Self {
            active: None,
            status: Status::default(),
        }
    }
}
impl<B: OwnedRoutingBackend> Drop for RoutingControl<B> {
    fn drop(&mut self) {
        if self.active.is_some() {
            let _ = self.stop(StopReason::Shutdown);
        }
    }
}
impl<B: OwnedRoutingBackend> RoutingControl<B> {
    pub fn status(&self) -> &Status {
        &self.status
    }
    /// Explicit factory: idle construction never opens a connection. NeedsReview
    /// cannot restart, and start never automatically rebinds a changed selection.
    pub fn start(
        &mut self,
        service: &ApplicationService,
        destinations: Vec<Destination>,
        factory: impl FnOnce() -> Result<B, String>,
    ) -> Result<(), String> {
        if matches!(self.status.phase, Phase::Running | Phase::NeedsReview) {
            return Err("running or unverified session cannot start".into());
        }
        let inventory = service.inventory().ok_or("no observed selection")?;
        if inventory.applications.is_empty() {
            return Err("select at least one live application".into());
        }
        let mut backend = factory()?;
        let setup = (|| {
            let baseline = backend.observe(service.revision())?;
            let mut epoch = PublicationEpoch::begin(service, &baseline)?;
            let pending = epoch
                .receive(
                    service,
                    &PollReply::Records {
                        records: vec![],
                        ignored_tail: 0,
                    },
                    ReceivedNamePolicy::ExactBytes,
                )?
                .ok_or("missing validation plan")?;
            epoch.prepare(
                service,
                &pending,
                &baseline,
                &baseline,
                &destinations,
                backend.ownership(),
            )?;
            Ok::<_, String>(epoch)
        })();
        let epoch = match setup {
            Ok(epoch) => epoch,
            Err(error) => {
                let cleanup = backend.stop_owned();
                self.status = Status {
                    phase: if cleanup.is_ok() {
                        Phase::Stopped
                    } else {
                        Phase::NeedsReview
                    },
                    reason: Some(StopReason::StartupFailed(error.clone())),
                    cleanup: Some(cleanup),
                    ..Status::default()
                };
                return Err(error);
            }
        };
        let assignments = inventory
            .applications
            .iter()
            .map(|a| (a.id.clone(), 0))
            .collect();
        self.active = Some(Active {
            backend,
            epoch,
            destinations,
            assignments,
        });
        self.status = Status {
            phase: Phase::Running,
            observed_revision: Some(service.revision()),
            ..Status::default()
        };
        Ok(())
    }
    /// Must follow each catalog/selection update, including operator edits.
    /// The bound backend is closed before returning a lifecycle error.
    pub fn check(&mut self, service: &ApplicationService) -> Result<(), String> {
        let Some(active) = self.active.as_mut() else {
            return Ok(());
        };
        match active.epoch.check_current(service) {
            Ok(()) => {
                self.status.observed_revision = Some(service.revision());
                Ok(())
            }
            Err(error) => {
                let cleanup = self.stop(StopReason::LifecycleChanged(error.clone()));
                Err(match cleanup {
                    Ok(_) => error,
                    Err(e) => format!("{error}; cleanup unverified: {e}"),
                })
            }
        }
    }
    /// Idempotent: an already attempted stop only returns its retained result.
    /// Stop releases all owned session links, not a guessed wire-level unroute.
    pub fn stop(&mut self, reason: StopReason) -> Result<Option<CleanupReport>, String> {
        let Some(mut active) = self.active.take() else {
            return self.status.cleanup.clone().transpose();
        };
        let cleanup = active.backend.stop_owned();
        self.status.phase = if cleanup.is_ok() {
            Phase::Stopped
        } else {
            Phase::NeedsReview
        };
        self.status.reason = Some(reason);
        self.status.assignments.clear();
        self.status.verified_revision = None;
        self.status.cleanup = Some(cleanup.clone());
        cleanup.map(Some)
    }
    pub fn assignments(
        &mut self,
        service: &ApplicationService,
        reply: &PollReply,
        names: ReceivedNamePolicy,
    ) -> Result<AssignmentResult, String> {
        self.check(service)?;
        let mut active = self.active.take().ok_or("routing session is not running")?;
        let result: Result<AssignmentResult, String> = (|| {
            let Some(pending) = active.epoch.receive(service, reply, names)? else {
                return Ok(AssignmentResult::NoAssignments);
            };
            if pending.plan().resolutions.is_empty() {
                return Ok(AssignmentResult::NoAssignments);
            }
            if pending
                .plan()
                .resolutions
                .iter()
                .any(|r| !matches!(r.outcome, Outcome::Proposed(_)))
            {
                return Ok(AssignmentResult::Blocked(
                    pending.plan().resolutions.clone(),
                ));
            }
            let before = active.backend.observe(service.revision())?;
            let after = active.backend.observe(service.revision())?;
            let ownership = active.backend.ownership().clone();
            let prepared = active.epoch.prepare(
                service,
                &pending,
                &before,
                &after,
                &active.destinations,
                &ownership,
            )?;
            let mut execution = Execution::new(prepared, &after, ownership)?;
            execution.run(&mut active.backend)?;
            let mut assignments = active.assignments.clone();
            for route in pending.plan().proposed_routes() {
                assignments.insert(route.application_id.clone(), route.link.assignment());
            }
            let before = active.backend.observe(service.revision())?;
            let after = active.backend.observe(service.revision())?;
            let publication = active.epoch.verify_routes(
                service,
                &before,
                &after,
                &active.destinations,
                execution.journal().ownership(),
                &assignments,
            )?;
            active.assignments = assignments;
            Ok(AssignmentResult::Verified(publication))
        })();
        self.active = Some(active);
        match result {
            Ok(result) => {
                if matches!(result, AssignmentResult::Verified(_)) {
                    self.status.assignments = self.active.as_ref().unwrap().assignments.clone();
                    self.status.verified_revision = Some(service.revision());
                }
                Ok(result)
            }
            Err(error) => {
                let cleanup = self.stop(StopReason::OperationFailed(error.clone()));
                Err(match cleanup {
                    Ok(_) => error,
                    Err(e) => format!("{error}; cleanup unverified: {e}"),
                })
            }
        }
    }
}

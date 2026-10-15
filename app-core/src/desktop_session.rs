//! Explicit desktop qualification adapter for the existing worker/UI controls.
use beacn_link_protocol::{
    catalog::ApplicationService,
    desktop_link_session::{DesktopLinkSession, StudioDesktopLease},
    executor::ObjectRef,
    link_session::{ManagedOptions, OpenFailure, Options, Status},
    native_usb::{self, NativeUsb, OpenError},
    routing_control::{self, StopReason},
    session::{PollPolicy, ReceivedNamePolicy},
    usb_handshake::{self, Handshake, Publication},
};
use beacn_link_protocol::{link_session::Phase, planner::Application};
use std::path::PathBuf;

#[derive(Clone)]
struct ResumeIntent {
    core: String,
    selection_revision: u64,
    applications: Vec<Application>,
    pair: u8,
}
impl ResumeIntent {
    fn verified(
        service: &ApplicationService,
        phase: Phase,
        assignment: Option<u8>,
    ) -> Option<Self> {
        if phase != Phase::Running {
            return None;
        }
        let pair = assignment.filter(|pair| (1..=4).contains(pair))?;
        let core = service.current()?.core.clone();
        let applications = service.inventory()?.applications.clone();
        if applications.len() != 1 || applications[0].streams.len() != 1 {
            return None;
        }
        Some(Self {
            core,
            selection_revision: service.selection_revision(),
            applications,
            pair,
        })
    }

    fn matches(&self, service: &ApplicationService) -> bool {
        service.current().map(|catalog| catalog.core.as_str()) == Some(self.core.as_str())
            && service.selection_revision() == self.selection_revision
            && service
                .inventory()
                .is_some_and(|inventory| inventory.applications == self.applications)
    }
}

#[derive(Clone)]
pub struct Configuration {
    pub sysfs: PathBuf,
    pub expected: Vec<u8>,
    pub hardware: ObjectRef,
    pub cycles: usize,
}
pub struct Session {
    config: Option<Configuration>,
    owner: DesktopLinkSession<StudioDesktopLease, Handshake<NativeUsb>>,
    status: Status,
    pub physical_stats: Option<(u32, u64)>,
    resume_intent: Option<ResumeIntent>,
}
impl Session {
    pub fn new(config: Configuration) -> Self {
        Self {
            config: Some(config),
            owner: Default::default(),
            status: Default::default(),
            physical_stats: None,
            resume_intent: None,
        }
    }
    pub fn managed() -> Self {
        Self {
            config: None,
            owner: Default::default(),
            status: Default::default(),
            physical_stats: None,
            resume_intent: None,
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
            cycle_limit: s.cycle_limit,
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
        if let Some(service) = service {
            if self
                .resume_intent
                .as_ref()
                .is_some_and(|intent| !intent.matches(service))
            {
                self.resume_intent = None;
            }
            if let Some(intent) = ResumeIntent::verified(service, self.status.phase, s.assignment) {
                self.resume_intent = Some(intent);
            }
        }
    }
    pub fn start(&mut self, service: &ApplicationService) -> Result<(), String> {
        self.start_with_resume(service, true)
    }
    pub fn start_for_assignment(&mut self, service: &ApplicationService) -> Result<(), String> {
        // An explicit new assignment supersedes the previous pair. Do not
        // briefly route to the remembered pair before applying the new request.
        self.start_with_resume(service, false)
    }
    fn start_with_resume(
        &mut self,
        service: &ApplicationService,
        resume: bool,
    ) -> Result<(), String> {
        if !resume {
            self.resume_intent = None;
        }
        let Some(c) = self.config.clone() else {
            let resume_pair = match self.resume_intent.as_ref() {
                Some(intent) if intent.matches(service) => Some(intent.pair),
                Some(_) => {
                    self.resume_intent = None;
                    None
                }
                None => None,
            };
            let options = ManagedOptions {
                poll_policy: PollPolicy::ReportRejectedMarker,
                names: ReceivedNamePolicy::WindowsCString,
                initialization_timeout: usb_handshake::DIAGNOSTIC_INITIALIZATION_TIMEOUT,
            };
            let mut result = self.owner.start_managed(
                service,
                options,
                || StudioDesktopLease::discover(service),
                |routes, inventory, options| {
                    let publication = Publication::managed_desktop(inventory, options.poll_policy)
                        .map_err(|e| OpenFailure {
                            message: e.to_string(),
                            cleanup: Ok(()),
                        })?
                        .with_initialization_diagnostic();
                    open_control(routes, publication)
                },
            );
            if result.is_ok() {
                if let Some(pair) = resume_pair {
                    result = self.owner.assign(service, pair);
                }
            }
            self.update(Some(service), None);
            return result;
        };
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
    pub fn assign(&mut self, service: &ApplicationService, pair: u8) -> Result<(), String> {
        let r = self.owner.assign(service, pair);
        self.update(Some(service), None);
        r
    }
    pub fn stop(&mut self, reason: StopReason) -> Result<(), String> {
        let r = self.owner.stop();
        self.update(None, Some(reason));
        r
    }
}

fn open_control(
    routes: &StudioDesktopLease,
    publication: Publication,
) -> Result<Handshake<NativeUsb>, OpenFailure> {
    native_usb::open_publication_with_owned_playback(
        routes.sysfs(),
        routes.descriptors(),
        publication,
        true,
        routes.server_pid(),
    )
    .map_err(|e| OpenFailure {
        cleanup: match &e {
            OpenError::Setup(f) if !f.cleanup.baseline_restored => {
                Err("Desktop USB startup cleanup unverified.".into())
            }
            _ => Ok(()),
        },
        message: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::ResumeIntent;
    use beacn_link_protocol::{
        catalog::{self, ApplicationService},
        link_session::Phase,
        planner::ApplicationId,
    };

    fn service(generation: u64, core: &str, stream: u64) -> ApplicationService {
        let catalog = catalog::parse(
            generation,
            &format!(
                "catalog\t1\t2\t{core}\napp\t3\t4\t{stream}\tlive\t46697265666f78\t46697265666f78\n"
            ),
        )
        .unwrap();
        let mut service = ApplicationService::default();
        service.update(catalog).unwrap();
        service
            .select(&ApplicationId("pw-client:3:pid:4".into()), "Firefox")
            .unwrap();
        service
    }

    fn intent(service: &ApplicationService) -> ResumeIntent {
        ResumeIntent::verified(service, Phase::Running, Some(2)).unwrap()
    }

    #[test]
    fn verified_intent_allows_new_snapshot_version_for_same_route() {
        let original = service(1, "74657374", 5);
        let intent = intent(&original);
        let mut refreshed = service(2, "74657374", 5);
        refreshed
            .update(
                catalog::parse(
                    3,
                    "catalog\t1\t2\t74657374\napp\t3\t4\t5\tlive\t46697265666f78\t46697265666f78\n",
                )
                .unwrap(),
            )
            .unwrap();
        assert_ne!(original.revision(), refreshed.revision());
        assert_eq!(
            original.selection_revision(),
            refreshed.selection_revision()
        );
        assert!(intent.matches(&refreshed));
        assert_eq!(intent.pair, 2);
    }

    #[test]
    fn verified_intent_rejects_stream_core_alias_and_reselection_changes() {
        let original = service(1, "74657374", 5);
        let intent = intent(&original);

        let changed_stream = service(2, "74657374", 6);
        assert!(!intent.matches(&changed_stream));

        let changed_core = service(2, "636f726532", 5);
        assert!(!intent.matches(&changed_core));

        let mut changed_alias = service(2, "74657374", 5);
        let id = ApplicationId("pw-client:3:pid:4".into());
        changed_alias.select(&id, "Firefox Alias").unwrap();
        assert!(!intent.matches(&changed_alias));

        let mut reselected = service(2, "74657374", 5);
        reselected.unselect(&id).unwrap();
        reselected.select(&id, "Firefox").unwrap();
        assert_eq!(
            original.inventory().unwrap().applications,
            reselected.inventory().unwrap().applications
        );
        assert_ne!(
            original.selection_revision(),
            reselected.selection_revision()
        );
        assert!(!intent.matches(&reselected));
    }

    #[test]
    fn only_verified_running_link_pair_can_be_remembered() {
        let service = service(1, "74657374", 5);
        assert!(ResumeIntent::verified(&service, Phase::Stopped, Some(2)).is_none());
        assert!(ResumeIntent::verified(&service, Phase::Running, Some(0)).is_none());
        assert!(ResumeIntent::verified(&service, Phase::Running, Some(5)).is_none());
    }
}

//! Pure app intent for waiting on a replacement audio stream.
//!
//! This intent is not selection or route authority. A resumed stream must be
//! selected from the current catalog and pass the normal native proof path.

use beacn_link_protocol::{
    catalog::ApplicationService,
    link_session::{Phase, Status},
    planner::{Application, ApplicationId},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Waiting {
    pub id: ApplicationId,
    pub name: String,
    pub pair: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    Running,
    Idle,
    Gone,
    Ambiguous,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    None,
    Suspend,
    Resume(Waiting),
    Cancel,
}

#[derive(Clone, Debug)]
struct Intent {
    waiting: Waiting,
    core: String,
    application: Application,
    suspended: bool,
}

#[derive(Default)]
pub struct Continuity {
    intent: Option<Intent>,
}

impl Continuity {
    /// Remember only a fully verified running assignment for one selected
    /// application and one exact live stream.
    pub fn remember(&mut self, service: &ApplicationService, status: &Status) {
        if self.intent.as_ref().is_some_and(|intent| intent.suspended) {
            return;
        }

        let Some((id, name, application)) = verified_application(service, status) else {
            self.clear();
            return;
        };
        let Some(core) = service.current().map(|catalog| catalog.core.clone()) else {
            self.clear();
            return;
        };
        self.intent = Some(Intent {
            waiting: Waiting {
                id,
                name,
                pair: status.routes.assignments.values().copied().next().unwrap(),
            },
            core,
            application,
            suspended: false,
        });
    }

    pub fn id(&self) -> Option<&ApplicationId> {
        self.intent.as_ref().map(|intent| &intent.waiting.id)
    }

    /// Whether the observed service still belongs to this intent's exact core
    /// and has no conflicting user selection or alias edit.
    pub fn idle_matches(&self, service: &ApplicationService) -> bool {
        self.intent.as_ref().is_some_and(|intent| {
            service
                .current()
                .is_some_and(|catalog| catalog.core == intent.core)
                && selection_matches_or_empty(service, &intent.waiting)
        })
    }

    pub fn clear(&mut self) {
        self.intent = None;
    }

    /// The app-level intent is visible to the waiter only after the old owned
    /// session has stopped successfully.
    pub fn waiting(&self) -> Option<Waiting> {
        self.intent
            .as_ref()
            .filter(|intent| intent.suspended)
            .map(|intent| intent.waiting.clone())
    }

    /// Decide whether to retain the exact running route, stop it before a
    /// replacement can be selected, or make one fresh resume attempt.
    pub fn decide(
        &self,
        service: &ApplicationService,
        activity: Activity,
        status: &Status,
    ) -> Decision {
        let Some(intent) = self.intent.as_ref() else {
            return Decision::None;
        };
        if !self.idle_matches(service) || status.phase == Phase::NeedsReview {
            return Decision::Cancel;
        }

        if intent.suspended {
            return match activity {
                Activity::Idle => Decision::None,
                Activity::Running => match observed_replacement(service, &intent.waiting) {
                    // The native activity query is authoritative for the pinned
                    // client generation. A missing row can lag that query while
                    // the client has no current audio node; only an observed
                    // stale or multi-stream row makes the candidate ambiguous.
                    Replacement::Absent => Decision::None,
                    Replacement::Live => Decision::Resume(intent.waiting.clone()),
                    Replacement::Invalid => Decision::Cancel,
                },
                Activity::Gone | Activity::Ambiguous => Decision::Cancel,
            };
        }

        if activity == Activity::Gone || activity == Activity::Ambiguous {
            return Decision::Cancel;
        }

        if status.phase != Phase::Running {
            return Decision::Cancel;
        }

        match activity {
            Activity::Idle => Decision::Suspend,
            Activity::Running
                if exact_live_roster(service, &intent.waiting, &intent.application) =>
            {
                Decision::None
            }
            Activity::Running => Decision::Suspend,
            Activity::Gone | Activity::Ambiguous => Decision::Cancel,
        }
    }

    /// Mark the intent as waiting only after the caller confirms owned cleanup.
    pub fn suspended(&mut self) {
        if let Some(intent) = self.intent.as_mut() {
            intent.suspended = true;
        }
    }

    /// Consume before attempting start so a failure cannot create a retry loop.
    pub fn take_resume(&mut self) -> Option<Waiting> {
        self.intent
            .take()
            .filter(|intent| intent.suspended)
            .map(|intent| intent.waiting)
    }
}

fn verified_application(
    service: &ApplicationService,
    status: &Status,
) -> Option<(ApplicationId, String, Application)> {
    if status.phase != Phase::Running || status.routes.assignments.len() != 1 {
        return None;
    }
    let (id, pair) = status.routes.assignments.iter().next()?;
    if !(1..=4).contains(pair) {
        return None;
    }
    let selected = service.selected();
    if selected.len() != 1 || selected.get(id).is_none() {
        return None;
    }
    let name = selected.get(id)?.clone();
    let inventory = service.inventory()?;
    if inventory.applications.len() != 1 || inventory.applications[0].id != *id {
        return None;
    }
    let application = inventory.applications[0].clone();
    if application.streams.len() != 1
        || application.advertised_name.as_slice() != name.as_bytes()
        || application.streams[0].state != beacn_link_protocol::planner::StreamState::Live
    {
        return None;
    }
    Some((id.clone(), name, application))
}

fn selection_matches_or_empty(service: &ApplicationService, waiting: &Waiting) -> bool {
    service.selected().is_empty()
        || (service.selected().len() == 1
            && service.selected().get(&waiting.id) == Some(&waiting.name))
}

enum Replacement {
    Absent,
    Live,
    Invalid,
}

fn observed_replacement(service: &ApplicationService, waiting: &Waiting) -> Replacement {
    let Some(catalog) = service.current() else {
        return Replacement::Invalid;
    };
    let Some(application) = catalog.applications.get(&waiting.id) else {
        return Replacement::Absent;
    };
    if application.streams.len() == 1
        && application
            .streams
            .iter()
            .all(|stream| stream.state == beacn_link_protocol::planner::StreamState::Live)
    {
        Replacement::Live
    } else {
        Replacement::Invalid
    }
}

fn exact_live_roster(
    service: &ApplicationService,
    waiting: &Waiting,
    original: &Application,
) -> bool {
    service.selected().len() == 1
        && service.selected().get(&waiting.id) == Some(&waiting.name)
        && service.inventory().is_some_and(|inventory| {
            inventory.applications.len() == 1 && inventory.applications.first() == Some(original)
        })
}

#[cfg(test)]
mod tests {
    use super::{Activity, Continuity, Decision, Waiting};
    use beacn_link_protocol::{
        catalog::{self, ApplicationService},
        link_session::{Phase, Status},
        planner::ApplicationId,
    };
    use std::collections::BTreeMap;

    const ID: &str = "pw-client:3:pid:4";
    const CORE: &str = "74657374";

    fn service(generation: u64, core: &str, rows: &str, selected: bool) -> ApplicationService {
        let mut service = ApplicationService::default();
        service
            .update(catalog::parse(generation, &format!("catalog\t1\t2\t{core}\n{rows}")).unwrap())
            .unwrap();
        if selected {
            service
                .select(&ApplicationId(ID.into()), "Firefox")
                .unwrap();
        }
        service
    }

    fn live_service(generation: u64, stream: u64) -> ApplicationService {
        service(
            generation,
            CORE,
            &format!("app\t3\t4\t{stream}\tlive\t46697265666f78\t66697265666f78\n"),
            true,
        )
    }

    fn running(pair: u8) -> Status {
        let mut status = Status {
            phase: Phase::Running,
            ..Status::default()
        };
        status
            .routes
            .assignments
            .insert(ApplicationId(ID.into()), pair);
        status
    }

    fn remembered() -> (Continuity, ApplicationService) {
        let service = live_service(1, 5);
        let mut continuity = Continuity::default();
        continuity.remember(&service, &running(2));
        assert_eq!(continuity.id(), Some(&ApplicationId(ID.into())));
        (continuity, service)
    }

    #[test]
    fn idle_suspends_waits_for_same_client_and_resumes_new_live_stream_once() {
        let (mut continuity, original) = remembered();
        assert_eq!(
            continuity.decide(&original, Activity::Idle, &running(2)),
            Decision::Suspend
        );
        assert!(continuity.waiting().is_none());

        continuity.suspended();
        assert_eq!(
            continuity.waiting(),
            Some(Waiting {
                id: ApplicationId(ID.into()),
                name: "Firefox".into(),
                pair: 2,
            })
        );
        let no_stream = service(2, CORE, "", false);
        let stopped = Status {
            phase: Phase::Stopped,
            ..Status::default()
        };
        assert_eq!(
            continuity.decide(&no_stream, Activity::Idle, &stopped),
            Decision::None
        );

        let replacement = live_service(3, 6);
        let decision = continuity.decide(&replacement, Activity::Running, &stopped);
        let expected = Waiting {
            id: ApplicationId(ID.into()),
            name: "Firefox".into(),
            pair: 2,
        };
        assert_eq!(decision, Decision::Resume(expected.clone()));
        assert_eq!(continuity.take_resume(), Some(expected));
        assert!(continuity.take_resume().is_none());

        // A successful fresh start/assignment records the replacement stream.
        continuity.remember(&replacement, &running(2));
        assert_eq!(
            continuity.decide(&replacement, Activity::Running, &running(2)),
            Decision::None
        );
    }

    #[test]
    fn replacement_client_or_core_change_cancels_wait() {
        let (mut continuity, _) = remembered();
        continuity.suspended();
        let stopped = Status {
            phase: Phase::Stopped,
            ..Status::default()
        };
        let changed_client = service(
            2,
            CORE,
            "app\t9\t4\t6\tlive\t46697265666f78\t66697265666f78\n",
            false,
        );
        assert_eq!(
            continuity.decide(&changed_client, Activity::Gone, &stopped),
            Decision::Cancel
        );

        // Reparse under another core while preserving the same client/node ids.
        let changed_core = service(
            2,
            "636f726532",
            "app\t3\t4\t6\tlive\t46697265666f78\t66697265666f78\n",
            false,
        );
        assert_eq!(
            continuity.decide(&changed_core, Activity::Running, &stopped),
            Decision::Cancel
        );
        assert!(!continuity.idle_matches(&changed_core));
    }

    #[test]
    fn alias_edit_unrelated_selection_and_ambiguous_activity_cancel() {
        let (mut continuity, _) = remembered();
        continuity.suspended();
        let stopped = Status {
            phase: Phase::Stopped,
            ..Status::default()
        };

        let mut renamed = live_service(2, 6);
        renamed
            .select(&ApplicationId(ID.into()), "Firefox renamed")
            .unwrap();
        assert_eq!(
            continuity.decide(&renamed, Activity::Running, &stopped),
            Decision::Cancel
        );

        let mut unrelated = live_service(2, 6);
        unrelated
            .update(
                catalog::parse(
                    3,
                    &format!(
                        "catalog\t1\t2\t{CORE}\napp\t3\t4\t6\tlive\t46697265666f78\t66697265666f78\napp\t9\t10\t11\tlive\t4d75736963\t6d75736963\n"
                    ),
                )
                .unwrap(),
            )
            .unwrap();
        unrelated
            .select(&ApplicationId("pw-client:9:pid:10".into()), "Music")
            .unwrap();
        assert_eq!(
            continuity.decide(&unrelated, Activity::Running, &stopped),
            Decision::Cancel
        );
        assert!(!continuity.idle_matches(&unrelated));
        assert_eq!(
            continuity.decide(&live_service(2, 6), Activity::Ambiguous, &stopped),
            Decision::Cancel
        );
    }

    #[test]
    fn remember_requires_one_verified_assignment_and_does_not_overwrite_waiting_intent() {
        let (mut continuity, _) = remembered();
        continuity.suspended();
        let original_waiting = continuity.waiting();

        let replacement = live_service(2, 6);
        let mut other = running(4);
        other
            .routes
            .assignments
            .insert(ApplicationId("pw-client:9:pid:10".into()), 3);
        continuity.remember(&replacement, &other);
        assert_eq!(continuity.waiting(), original_waiting);

        let mut invalid = running(0);
        invalid.routes.assignments = BTreeMap::from([(ApplicationId(ID.into()), 0)]);
        let mut fresh = Continuity::default();
        fresh.remember(&replacement, &invalid);
        assert!(fresh.id().is_none());
    }

    #[test]
    fn running_process_waits_through_catalog_lag_but_rejects_stale_or_ambiguous_rows() {
        let (mut continuity, _) = remembered();
        continuity.suspended();
        let stopped = Status {
            phase: Phase::Stopped,
            ..Status::default()
        };

        // Native process identity can be observed before the catalog row lands.
        let no_row = service(2, CORE, "", false);
        assert!(continuity.idle_matches(&no_row));
        assert_eq!(
            continuity.decide(&no_row, Activity::Running, &stopped),
            Decision::None
        );

        let stale = service(
            3,
            CORE,
            "app\t3\t4\t6\tstale\t46697265666f78\t66697265666f78\n",
            false,
        );
        assert_eq!(
            continuity.decide(&stale, Activity::Running, &stopped),
            Decision::Cancel
        );

        let multiple = service(
            4,
            CORE,
            "app\t3\t4\t6\tlive\t46697265666f78\t66697265666f78\napp\t3\t4\t7\tlive\t46697265666f78\t66697265666f78\n",
            false,
        );
        assert_eq!(
            continuity.decide(&multiple, Activity::Running, &stopped),
            Decision::Cancel
        );
    }

    #[test]
    fn unchanged_roster_stays_running_and_gone_or_stopped_cancels() {
        let (continuity, service) = remembered();
        assert_eq!(
            continuity.decide(&service, Activity::Running, &running(2)),
            Decision::None
        );
        assert_eq!(
            continuity.decide(
                &service,
                Activity::Gone,
                &Status {
                    phase: Phase::Running,
                    ..Status::default()
                }
            ),
            Decision::Cancel
        );
        assert_eq!(
            continuity.decide(
                &service,
                Activity::Running,
                &Status {
                    phase: Phase::Stopped,
                    ..Status::default()
                }
            ),
            Decision::Cancel
        );
    }

    #[test]
    fn explicit_clear_cancels_a_suspended_intent() {
        let (mut continuity, service) = remembered();
        continuity.suspended();
        continuity.clear();
        assert!(continuity.id().is_none());
        assert!(continuity.waiting().is_none());
        assert_eq!(
            continuity.decide(
                &service,
                Activity::Running,
                &Status {
                    phase: Phase::Running,
                    ..Status::default()
                }
            ),
            Decision::None
        );
    }
}

use beacn_link_protocol::{
    catalog::ApplicationService,
    execution::{Backend, Effect, Execution, Observation},
    executor::*,
    planner::{ApplicationId, Outcome},
    publication_epoch::PublicationEpoch,
    receipts::Acknowledgment,
    route_publication::VerifiedRoutePublication,
    session::{PollReply, ReceivedNamePolicy},
    usb_handshake::Publication,
    Assignment, Record,
};

#[path = "support/routing_fixture.rs"]
mod fixture;
use fixture::*;

struct Fake {
    observation: Observation,
    calls: usize,
    fail_at: Option<usize>,
    next_id: u32,
}
impl Fake {
    fn new(observation: Observation) -> Self {
        Self {
            observation,
            calls: 0,
            fail_at: None,
            next_id: 3000,
        }
    }
}
impl Backend for Fake {
    fn observe(&mut self, _: u64) -> Result<Observation, String> {
        Ok(self.observation.clone())
    }
    fn perform(&mut self, operation: &Operation) -> Effect {
        self.calls += 1;
        if self.fail_at == Some(self.calls) {
            return Effect::Failed("bounded injected no-effect failure".into());
        }
        let ack = match operation {
            Operation::Connect {
                output,
                input,
                owner,
            } => {
                self.next_id += 1;
                let link = ExistingLink {
                    object: ObjectRef {
                        id: self.next_id,
                        serial: 20000 + self.next_id as u64,
                    },
                    output: output.object,
                    input: input.object,
                    owner: Some(owner.clone()),
                };
                self.observation.graph.links.push(link.clone());
                Acknowledgment::Created(link)
            }
            Operation::Disconnect { link, .. } => {
                self.observation
                    .graph
                    .links
                    .retain(|l| l.object != link.object);
                Acknowledgment::Disconnected(link.object)
            }
        };
        Effect::Acknowledged(ack)
    }
    fn reconcile(&mut self) -> Result<Option<Acknowledgment>, String> {
        Ok(None)
    }
}

#[test]
fn two_apps_three_streams_route_publish_move_and_receipt_restoration() {
    let (service, baseline, destinations, ownership) = setup();
    let mut epoch = PublicationEpoch::begin(&service, &baseline).unwrap();
    let zero = epoch
        .verify_routes(
            &service,
            &baseline,
            &baseline,
            &destinations,
            &ownership,
            &assignments(0, 0),
        )
        .unwrap();
    assert_eq!(zero.records().len(), 2);
    // Native USB gate is deliberately still single-app.
    assert!(Publication::from_inventory(zero.inventory()).is_err());
    let pending = epoch
        .receive(&service, &reply(1, 4), ReceivedNamePolicy::WindowsCString)
        .unwrap()
        .unwrap();
    assert_eq!(pending.plan().proposed_routes().count(), 2);
    let prepared = epoch
        .prepare(
            &service,
            &pending,
            &baseline,
            &baseline,
            &destinations,
            &ownership,
        )
        .unwrap();
    assert_eq!(prepared.steps.len(), 6);
    let mut backend = Fake::new(baseline.clone());
    let mut execution = Execution::new(prepared, &baseline, ownership).unwrap();
    execution.run(&mut backend).unwrap();
    let observed = backend.observation.clone();
    let proof = epoch
        .verify_routes(
            &service,
            &observed,
            &observed,
            &destinations,
            execution.journal().ownership(),
            &assignments(1, 4),
        )
        .unwrap();
    let bytes = beacn_link_protocol::encode_records(proof.records()).unwrap();
    assert_eq!(
        beacn_link_protocol::decode_records(&bytes).unwrap(),
        proof.records()
    );
    assert_eq!(
        proof
            .records()
            .iter()
            .map(|r| r.assignment.byte())
            .collect::<Vec<_>>(),
        [1, 4]
    );

    let pending = epoch
        .receive(&service, &reply(2, 3), ReceivedNamePolicy::WindowsCString)
        .unwrap()
        .unwrap();
    let prepared = epoch
        .prepare(
            &service,
            &pending,
            &observed,
            &observed,
            &destinations,
            execution.journal().ownership(),
        )
        .unwrap();
    assert_eq!(prepared.steps.len(), 12);
    let mut moved =
        Execution::new(prepared, &observed, execution.journal().ownership().clone()).unwrap();
    moved.run(&mut backend).unwrap();
    let after = &backend.observation;
    epoch
        .verify_routes(
            &service,
            after,
            after,
            &destinations,
            moved.journal().ownership(),
            &assignments(2, 3),
        )
        .unwrap();
    // Recovery uses acknowledged concrete objects, including new restoration serials.
    moved.rollback(&mut backend).unwrap();
    let after = &backend.observation;
    VerifiedRoutePublication::verify_selected(
        after,
        after,
        &destinations,
        moved.recovery_journal().unwrap().ownership(),
        &assignments(1, 4),
    )
    .unwrap();
}

#[test]
fn partial_multi_app_failure_has_no_publishable_desired_result_and_recovers_owned_prefix() {
    let (service, baseline, destinations, ownership) = setup();
    let mut epoch = PublicationEpoch::begin(&service, &baseline).unwrap();
    let pending = epoch
        .receive(&service, &reply(1, 4), ReceivedNamePolicy::WindowsCString)
        .unwrap()
        .unwrap();
    let prepared = epoch
        .prepare(
            &service,
            &pending,
            &baseline,
            &baseline,
            &destinations,
            &ownership,
        )
        .unwrap();
    let mut backend = Fake::new(baseline.clone());
    backend.fail_at = Some(4);
    let mut execution = Execution::new(prepared, &baseline, ownership).unwrap();
    assert!(execution.run(&mut backend).is_err());
    assert_eq!(execution.journal().prefix(), 3);
    let partial = &backend.observation;
    assert!(epoch
        .verify_routes(
            &service,
            partial,
            partial,
            &destinations,
            execution.journal().ownership(),
            &assignments(1, 4)
        )
        .is_err());
    assert!(epoch.halted());
    execution.rollback(&mut backend).unwrap();
    assert_eq!(backend.observation, baseline);
    // The failed epoch stays stopped even after successful explicit receipt recovery.
    assert!(epoch
        .receive(&service, &reply(1, 4), ReceivedNamePolicy::WindowsCString)
        .is_err());
    assert!(PublicationEpoch::begin(&service, &baseline).is_ok());
}

#[test]
fn lifecycle_edit_even_reversed_core_change_and_stream_change_halt_the_epoch() {
    for kind in 0..6 {
        let (mut service, baseline, _, _) = setup();
        let mut epoch = PublicationEpoch::begin(&service, &baseline).unwrap();
        match kind {
            0 => {
                service.unselect(&id(1)).unwrap();
                service.select(&id(1), "Game").unwrap();
            }
            1 => {
                service
                    .update(catalog(2, &ROWS.replace("\t21\t", "\t23\t")))
                    .unwrap();
            }
            2 => {
                service
                    .update(catalog(2, &ROWS.replace("\tlive\t", "\tstale\t")))
                    .unwrap();
            }
            3 => {
                let mut c = catalog(2, ROWS);
                c.core = "new-core".into();
                service.update(c).unwrap();
            }
            4 => {
                service.update(catalog(2, "")).unwrap();
                service.update(catalog(3, ROWS)).unwrap();
                service.select(&id(0), "Player").unwrap();
                service.select(&id(1), "Game").unwrap();
            }
            _ => {
                service.select(&id(0), "Renamed").unwrap();
                service.select(&id(0), "Player").unwrap();
            }
        }
        assert!(
            epoch
                .receive(&service, &reply(1, 4), ReceivedNamePolicy::WindowsCString)
                .is_err(),
            "case {kind}"
        );
        assert!(epoch.halted());
    }
}

#[test]
fn unchanged_catalog_refresh_invalidates_pending_plan_but_can_accept_a_fresh_reply() {
    let (mut service, baseline, destinations, ownership) = setup();
    let mut epoch = PublicationEpoch::begin(&service, &baseline).unwrap();
    let old = epoch
        .receive(&service, &reply(1, 4), ReceivedNamePolicy::WindowsCString)
        .unwrap()
        .unwrap();
    let selection_revision = service.selection_revision();
    service.update(catalog(2, ROWS)).unwrap();
    assert_eq!(service.selection_revision(), selection_revision);
    let mut fresh = baseline.clone();
    fresh.inventory = service.inventory().unwrap().clone();
    fresh.graph.generation = fresh.inventory.snapshot_id;
    assert!(epoch
        .prepare(&service, &old, &fresh, &fresh, &destinations, &ownership)
        .is_err());
    assert!(epoch.halted());
    let mut epoch = PublicationEpoch::begin(&service, &fresh).unwrap();
    let pending = epoch
        .receive(&service, &reply(1, 4), ReceivedNamePolicy::WindowsCString)
        .unwrap()
        .unwrap();
    assert!(epoch
        .prepare(
            &service,
            &pending,
            &fresh,
            &fresh,
            &destinations,
            &ownership
        )
        .is_ok());
}

#[test]
fn pending_assignments_cannot_cross_epochs_even_with_identical_roster_and_generation() {
    let (service, baseline, destinations, ownership) = setup();
    let mut original = PublicationEpoch::begin(&service, &baseline).unwrap();
    let pending = original
        .receive(&service, &reply(1, 4), ReceivedNamePolicy::WindowsCString)
        .unwrap()
        .unwrap();
    let mut replacement = PublicationEpoch::begin(&service, &baseline).unwrap();
    assert!(replacement
        .prepare(
            &service,
            &pending,
            &baseline,
            &baseline,
            &destinations,
            &ownership
        )
        .is_err());
    assert!(replacement.halted());
    assert!(original
        .prepare(
            &service,
            &pending,
            &baseline,
            &baseline,
            &destinations,
            &ownership
        )
        .is_ok());
}

#[test]
fn opaque_empty_missing_and_conflicting_records_never_infer_unroute() {
    let (service, baseline, destinations, ownership) = setup();
    let mut epoch = PublicationEpoch::begin(&service, &baseline).unwrap();
    for reply in [
        PollReply::NoPayload { ignored_tail: 8 },
        PollReply::RejectedMarker { ignored_tail: 0 },
    ] {
        assert!(epoch
            .receive(&service, &reply, ReceivedNamePolicy::ExactBytes)
            .unwrap()
            .is_none());
    }
    let empty = PollReply::Records {
        records: vec![],
        ignored_tail: 0,
    };
    let pending = epoch
        .receive(&service, &empty, ReceivedNamePolicy::ExactBytes)
        .unwrap()
        .unwrap();
    assert!(epoch
        .prepare(
            &service,
            &pending,
            &baseline,
            &baseline,
            &destinations,
            &ownership
        )
        .unwrap()
        .steps
        .is_empty());
    let one = PollReply::Records {
        records: vec![Record {
            name: b"Game".to_vec(),
            assignment: Assignment::from_byte(4),
        }],
        ignored_tail: 0,
    };
    let pending = epoch
        .receive(&service, &one, ReceivedNamePolicy::ExactBytes)
        .unwrap()
        .unwrap();
    assert_eq!(
        epoch
            .prepare(
                &service,
                &pending,
                &baseline,
                &baseline,
                &destinations,
                &ownership
            )
            .unwrap()
            .steps
            .len(),
        2
    );
    let pending = epoch
        .receive(&service, &reply(0, 255), ReceivedNamePolicy::WindowsCString)
        .unwrap()
        .unwrap();
    assert!(pending
        .plan()
        .resolutions
        .iter()
        .all(|r| matches!(r.outcome, Outcome::OpaqueAssignment { .. })));
    assert!(epoch
        .prepare(
            &service,
            &pending,
            &baseline,
            &baseline,
            &destinations,
            &ownership
        )
        .is_err());
    let mut epoch = PublicationEpoch::begin(&service, &baseline).unwrap();
    let conflict = PollReply::Records {
        records: vec![
            Record {
                name: b"Game".to_vec(),
                assignment: Assignment::from_byte(1),
            },
            Record {
                name: b"Game".to_vec(),
                assignment: Assignment::from_byte(2),
            },
        ],
        ignored_tail: 0,
    };
    let pending = epoch
        .receive(&service, &conflict, ReceivedNamePolicy::ExactBytes)
        .unwrap()
        .unwrap();
    assert!(matches!(
        pending.plan().resolutions[0].outcome,
        Outcome::ConflictingAssignments { .. }
    ));
    assert!(epoch
        .prepare(
            &service,
            &pending,
            &baseline,
            &baseline,
            &destinations,
            &ownership
        )
        .is_err());
}

#[test]
fn proof_requires_every_app_all_streams_complete_receipts_and_current_domains() {
    let (service, baseline, destinations, ownership) = setup();
    let mut epoch = PublicationEpoch::begin(&service, &baseline).unwrap();
    let pending = epoch
        .receive(&service, &reply(1, 4), ReceivedNamePolicy::WindowsCString)
        .unwrap()
        .unwrap();
    let prepared = epoch
        .prepare(
            &service,
            &pending,
            &baseline,
            &baseline,
            &destinations,
            &ownership,
        )
        .unwrap();
    let mut backend = Fake::new(baseline.clone());
    let mut execution = Execution::new(prepared, &baseline, ownership).unwrap();
    execution.run(&mut backend).unwrap();
    for kind in 0..9 {
        let mut o = backend.observation.clone();
        let mut own = execution.journal().ownership().clone();
        let mut map = assignments(1, 4);
        match kind {
            0 => {
                map.remove(&id(1));
            }
            1 => {
                map.insert(ApplicationId("foreign".into()), 2);
            }
            2 => {
                map.insert(id(1), 5);
            }
            3 => {
                o.graph.links.pop();
            }
            4 => {
                own.links.remove(&o.graph.links[5].object);
            }
            5 => {
                o.graph.links[5].owner = Some("foreign".into());
            }
            6 => {
                o.inventory.applications[1].advertised_name = b"Player".to_vec();
            }
            7 => {
                o.graph.streams.pop();
            }
            _ => {
                o.graph.ports[0].channel = Channel::Other;
            }
        }
        assert!(
            VerifiedRoutePublication::verify_selected(&o, &o, &destinations, &own, &map).is_err(),
            "case {kind}"
        );
    }
    let mut changed = backend.observation.clone();
    changed.graph.core_id = "replaced-core".into();
    assert!(epoch
        .verify_routes(
            &service,
            &changed,
            &changed,
            &destinations,
            execution.journal().ownership(),
            &assignments(1, 4)
        )
        .is_err());
    assert!(epoch.halted());
    assert!(VerifiedRoutePublication::verify(
        &backend.observation,
        &backend.observation,
        &destinations,
        execution.journal().ownership(),
        1
    )
    .is_err());
}

#[test]
fn mixed_bound_and_unlinked_apps_need_explicit_zero_and_empty_roster_is_refused() {
    let (service, baseline, destinations, ownership) = setup();
    let mut epoch = PublicationEpoch::begin(&service, &baseline).unwrap();
    let one = PollReply::Records {
        records: vec![Record {
            name: b"Game".to_vec(),
            assignment: Assignment::from_byte(4),
        }],
        ignored_tail: 0,
    };
    let pending = epoch
        .receive(&service, &one, ReceivedNamePolicy::ExactBytes)
        .unwrap()
        .unwrap();
    let prepared = epoch
        .prepare(
            &service,
            &pending,
            &baseline,
            &baseline,
            &destinations,
            &ownership,
        )
        .unwrap();
    let mut backend = Fake::new(baseline);
    let mut execution = Execution::new(prepared, &backend.observation, ownership).unwrap();
    execution.run(&mut backend).unwrap();
    let o = &backend.observation;
    let proof = epoch
        .verify_routes(
            &service,
            o,
            o,
            &destinations,
            execution.journal().ownership(),
            &assignments(0, 4),
        )
        .unwrap();
    assert_eq!(proof.records()[0].assignment.byte(), 0);
    assert!(PublicationEpoch::begin(&service, o).is_err()); // No implicit adoption of existing routes.
    let mut empty_service = ApplicationService::default();
    empty_service.update(catalog(1, ROWS)).unwrap();
    let mut empty = o.clone();
    empty.inventory = empty_service.inventory().unwrap().clone();
    empty.graph.generation = empty.inventory.snapshot_id;
    assert!(PublicationEpoch::begin(&empty_service, &empty).is_err());
}

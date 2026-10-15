use beacn_link_protocol::{
    routing_control::{AssignmentResult, Phase, RoutingControl, StopReason},
    session::{PollReply, ReceivedNamePolicy},
    Assignment, Record,
};
#[path = "support/routing_fixture.rs"]
mod fixture;
use fixture::*;

#[path = "support/owned_backend.rs"]
mod owned;
use owned::*;
fn route(
    control: &mut RoutingControl<Fake>,
    service: &beacn_link_protocol::catalog::ApplicationService,
) {
    assert!(matches!(
        control
            .assignments(service, &reply(1, 4), ReceivedNamePolicy::WindowsCString)
            .unwrap(),
        AssignmentResult::Verified(_)
    ));
}

#[test]
fn explicit_start_stop_status_and_restart_do_not_open_or_repeat_work_implicitly() {
    let (service, baseline, destinations, ownership) = setup();
    let mut control = RoutingControl::default();
    assert_eq!(control.status().phase, Phase::Idle);
    let (fake, state) = backend(baseline.clone(), ownership.clone());
    control
        .start(&service, destinations.clone(), || Ok(fake))
        .unwrap();
    assert_eq!(control.status().phase, Phase::Running);
    assert!(control
        .start(&service, destinations.clone(), || panic!(
            "factory must not run"
        ))
        .is_err());
    route(&mut control, &service);
    assert_eq!(state.borrow().observation.graph.links.len(), 6);
    assert_eq!(control.status().assignments, assignments(1, 4));
    let proof = control.stop(StopReason::Owner).unwrap().unwrap();
    assert_eq!(proof.observed_links, 6);
    assert_eq!(proof.already_absent, 0);
    assert_eq!(state.borrow().observation, baseline);
    assert_eq!(control.status().phase, Phase::Stopped);
    assert!(control.status().assignments.is_empty());
    assert_eq!(control.stop(StopReason::Shutdown).unwrap(), Some(proof));
    assert_eq!(state.borrow().stops, 1);
    assert!(control
        .assignments(&service, &reply(1, 4), ReceivedNamePolicy::WindowsCString)
        .is_err());
    let (fake, next) = backend(baseline.clone(), ownership);
    control.start(&service, destinations, || Ok(fake)).unwrap();
    route(&mut control, &service);
    drop(control);
    assert_eq!(next.borrow().stops, 1);
    assert_eq!(next.borrow().observation, baseline);
}

#[test]
fn lifecycle_loss_stale_alias_core_and_selection_edits_stop_once_before_more_work() {
    for kind in 0..5 {
        let (mut service, baseline, destinations, ownership) = setup();
        let (fake, state) = backend(baseline, ownership);
        let mut control = RoutingControl::default();
        control.start(&service, destinations, || Ok(fake)).unwrap();
        route(&mut control, &service);
        let calls = state.borrow().calls;
        match kind {
            0 => {
                service.update(catalog(2, "")).unwrap();
            }
            1 => {
                service
                    .update(catalog(2, &ROWS.replace("\tlive\t", "\tstale\t")))
                    .unwrap();
            }
            2 => {
                service.select(&id(0), "Changed").unwrap();
            }
            3 => {
                let mut c = catalog(2, ROWS);
                c.core = "replacement".into();
                service.update(c).unwrap();
            }
            _ => {
                service.unselect(&id(1)).unwrap();
            }
        }
        assert!(control.check(&service).is_err());
        assert_eq!(control.status().phase, Phase::Stopped);
        assert!(matches!(
            control.status().reason,
            Some(StopReason::LifecycleChanged(_))
        ));
        assert_eq!(state.borrow().calls, calls);
        assert_eq!(state.borrow().stops, 1);
        assert!(state.borrow().observation.graph.links.is_empty());
        assert!(control
            .assignments(&service, &reply(1, 4), ReceivedNamePolicy::WindowsCString)
            .is_err());
        assert_eq!(state.borrow().stops, 1);
    }
}

#[test]
fn source_already_gone_and_reused_numeric_receipt_id_preserve_foreign_objects() {
    let (mut service, baseline, destinations, ownership) = setup();
    let (fake, state) = backend(baseline, ownership);
    let mut control = RoutingControl::default();
    control.start(&service, destinations, || Ok(fake)).unwrap();
    route(&mut control, &service);
    // Simulate a source being removed by its own owner, including its links.
    {
        let mut state = state.borrow_mut();
        state.observation.graph.links.remove(0);
        // The same numeric ID now denotes another owner's new serial. The native
        // handle model still contains only the old full identity.
        state.observation.graph.links[0].object.serial += 50000;
        state.observation.graph.links[0].owner = Some("ours".into()); // spoofed tag
    }
    let foreign = state.borrow().observation.graph.links[0].clone();
    service.unselect(&id(1)).unwrap();
    assert!(control.check(&service).is_err());
    assert_eq!(state.borrow().observation.graph.links, vec![foreign]);
    let proof = control.status().cleanup.as_ref().unwrap().as_ref().unwrap();
    assert_eq!((proof.observed_links, proof.already_absent), (4, 2));
}

#[test]
fn opaque_no_payload_empty_and_conflicting_assignments_never_infer_stop_or_unroute() {
    let (mut service, baseline, destinations, ownership) = setup();
    let (fake, state) = backend(baseline, ownership);
    let mut control = RoutingControl::default();
    control.start(&service, destinations, || Ok(fake)).unwrap();
    route(&mut control, &service);
    service.update(catalog(2, ROWS)).unwrap();
    control.check(&service).unwrap();
    assert_eq!(control.status().observed_revision, Some(service.revision()));
    assert!(control.status().verified_revision.unwrap() < service.revision());
    for poll in [
        PollReply::NoPayload { ignored_tail: 0 },
        PollReply::RejectedMarker { ignored_tail: 0 },
        PollReply::Records {
            records: vec![],
            ignored_tail: 0,
        },
    ] {
        assert!(matches!(
            control
                .assignments(&service, &poll, ReceivedNamePolicy::ExactBytes)
                .unwrap(),
            AssignmentResult::NoAssignments
        ));
    }
    let before = state.borrow().calls;
    assert!(matches!(
        control
            .assignments(&service, &reply(0, 255), ReceivedNamePolicy::WindowsCString)
            .unwrap(),
        AssignmentResult::Blocked(_)
    ));
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
    assert!(matches!(
        control
            .assignments(&service, &conflict, ReceivedNamePolicy::ExactBytes)
            .unwrap(),
        AssignmentResult::Blocked(_)
    ));
    assert_eq!(control.status().phase, Phase::Running);
    assert_eq!(state.borrow().calls, before);
    assert_eq!(state.borrow().stops, 0);
    assert_eq!(control.status().assignments, assignments(1, 4));
    // Missing Player record retains its previously verified route.
    let one = PollReply::Records {
        records: vec![Record {
            name: b"Game".to_vec(),
            assignment: Assignment::from_byte(2),
        }],
        ignored_tail: 0,
    };
    assert!(matches!(
        control
            .assignments(&service, &one, ReceivedNamePolicy::ExactBytes)
            .unwrap(),
        AssignmentResult::Verified(_)
    ));
    assert_eq!(control.status().assignments, assignments(1, 2));
}

#[test]
fn known_partial_failure_releases_owned_prefix_without_forward_retry() {
    let (service, baseline, destinations, ownership) = setup();
    let (fake, state) = backend(baseline.clone(), ownership);
    state.borrow_mut().fail_at = Some(4);
    let mut control = RoutingControl::default();
    control.start(&service, destinations, || Ok(fake)).unwrap();
    assert!(control
        .assignments(&service, &reply(1, 4), ReceivedNamePolicy::WindowsCString)
        .is_err());
    assert_eq!(control.status().phase, Phase::Stopped);
    assert_eq!(state.borrow().calls, 4);
    assert_eq!(state.borrow().observation, baseline);
    assert_eq!(
        control
            .status()
            .cleanup
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .observed_links,
        3
    );
}

#[test]
fn unknown_effect_and_failed_cleanup_release_handles_but_block_restart_without_retry() {
    for unknown in [false, true] {
        let (service, baseline, destinations, ownership) = setup();
        let (fake, state) = backend(baseline.clone(), ownership);
        if unknown {
            state.borrow_mut().unknown_at = Some(4);
        } else {
            state.borrow_mut().cleanup_fails = true;
        }
        let mut control = RoutingControl::default();
        control
            .start(&service, destinations.clone(), || Ok(fake))
            .unwrap();
        if unknown {
            assert!(control
                .assignments(&service, &reply(1, 4), ReceivedNamePolicy::WindowsCString)
                .is_err());
        } else {
            route(&mut control, &service);
            assert!(control.stop(StopReason::Owner).is_err());
        }
        assert_eq!(control.status().phase, Phase::NeedsReview);
        assert!(control.status().assignments.is_empty());
        assert!(state.borrow().handles.is_empty());
        assert!(state.borrow().observation.graph.links.is_empty());
        assert!(control
            .start(&service, destinations, || panic!(
                "restart factory forbidden"
            ))
            .is_err());
        assert!(control.stop(StopReason::Owner).is_err());
        assert_eq!(state.borrow().stops, 1);
    }
}

#[test]
fn startup_validation_and_observation_failure_still_release_only_the_owned_session() {
    let (mut empty, _, destinations, _) = setup();
    empty.unselect(&id(0)).unwrap();
    empty.unselect(&id(1)).unwrap();
    let mut unopened = RoutingControl::<Fake>::default();
    assert!(unopened
        .start(&empty, destinations, || panic!(
            "empty selection must not open native access"
        ))
        .is_err());
    let (service, baseline, mut destinations, ownership) = setup();
    let (fake, state) = backend(baseline.clone(), ownership.clone());
    destinations[0].node.serial += 1;
    let mut control = RoutingControl::default();
    assert!(control.start(&service, destinations, || Ok(fake)).is_err());
    assert_eq!(state.borrow().calls, 0);
    assert_eq!(state.borrow().stops, 1);
    assert_eq!(control.status().phase, Phase::Stopped);
    let (_, _, destinations, _) = setup();
    let (fake, state) = backend(baseline, ownership);
    control.start(&service, destinations, || Ok(fake)).unwrap();
    route(&mut control, &service);
    state.borrow_mut().observation_fails = true;
    assert!(control
        .assignments(&service, &reply(2, 3), ReceivedNamePolicy::WindowsCString)
        .is_err());
    assert_eq!(state.borrow().stops, 1);
    assert!(state.borrow().handles.is_empty());
    assert_eq!(control.status().phase, Phase::NeedsReview);
}

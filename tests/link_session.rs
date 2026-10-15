use beacn_link_protocol::{
    decode_frame, decode_records, encode_frame, encode_records,
    link_session::{LinkSession, ManagedOptions, OpenFailure, Options, Phase},
    planner::Inventory,
    routing_control::StopReason,
    session::{PollPolicy, ReceivedNamePolicy},
    usb_handshake,
    usb_handshake::{Handshake, Publication},
    Assignment, Frame, Record,
};
use std::time::Duration;
#[allow(dead_code)]
#[path = "support/routing_fixture.rs"]
mod fixture;
#[path = "support/owned_backend.rs"]
mod owned;
#[path = "../src/scripted_peer.rs"]
mod peer;
use fixture::*;
use owned::*;
use peer::*;
fn options(cycles: usize) -> Options {
    Options {
        cycles,
        poll_policy: PollPolicy::ReportRejectedMarker,
        names: ReceivedNamePolicy::WindowsCString,
        initialization_timeout: Duration::from_millis(500),
    }
}
fn reply(a: u8, b: u8) -> Reply {
    Reply::Records(vec![
        Record {
            name: b"Player".to_vec(),
            assignment: Assignment::from_byte(a),
        },
        Record {
            name: b"Game".to_vec(),
            assignment: Assignment::from_byte(b),
        },
    ])
}
fn open(
    peer: ScriptedPeer,
    inv: &Inventory,
    options: Options,
) -> Result<Handshake<ScriptedPeer>, OpenFailure> {
    let publication = Publication::bounded_selected(inv, options.cycles, options.poll_policy)
        .map_err(|e| OpenFailure {
            message: e.to_string(),
            cleanup: Ok(()),
        })?;
    Handshake::prepare_publication(peer, publication).map_err(|e| OpenFailure {
        message: e.error.to_string(),
        cleanup: if e.cleanup.baseline_restored {
            Ok(())
        } else {
            Err("setup cleanup differs".into())
        },
    })
}
#[test]
fn complete_multi_app_bytes_routes_no_payload_moves_and_budget_cleanup_are_one_session() {
    let (mut service, baseline, destinations, ownership) = setup();
    let (backend, state) = backend(baseline.clone(), ownership);
    let (peer, log) = ScriptedPeer::new(vec![
        reply(1, 4),
        Reply::NoPayload,
        Reply::Rejected,
        reply(2, 3),
        reply(2, 3),
    ])
    .unwrap();
    let mut session = LinkSession::default();
    session
        .start(
            &service,
            destinations,
            options(5),
            || Ok(backend),
            |i, o| open(peer, i, o),
        )
        .unwrap();
    for n in 2..=6 {
        service.update(catalog(n, ROWS)).unwrap();
        session.tick(&service).unwrap();
    }
    assert_eq!(session.status().completed_cycles, 5);
    assert_eq!(session.status().phase, Phase::Stopped);
    assert_eq!(state.borrow().observation.graph.links, baseline.graph.links);
    assert_eq!(state.borrow().stops, 1);
    let log = log.borrow();
    assert_eq!(log.releases, 1);
    assert_eq!(log.closes, 1);
    assert_eq!(log.writes.len(), 12);
    let records: Vec<_> = log
        .writes
        .iter()
        .filter_map(|b| match decode_frame(b).unwrap() {
            Frame::Data(p) => Some(decode_records(&p).unwrap()),
            _ => None,
        })
        .collect();
    assert_eq!(records.len(), 5);
    assert_eq!(
        records
            .iter()
            .map(|r| r.iter().map(|r| r.assignment.byte()).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        vec![vec![0, 0], vec![1, 4], vec![1, 4], vec![1, 4], vec![2, 3]]
    );
    assert!(session.tick(&service).is_err());
    assert!(session.stop(StopReason::Owner).is_ok());
}
#[test]
fn stale_observation_and_lifecycle_stop_before_any_next_control_submission() {
    for changed in [false, true] {
        let (mut service, baseline, destinations, ownership) = setup();
        let (backend, state) = backend(baseline, ownership);
        let (peer, log) = ScriptedPeer::new(vec![reply(1, 4)]).unwrap();
        let mut session = LinkSession::default();
        session
            .start(
                &service,
                destinations,
                options(4),
                || Ok(backend),
                |i, o| open(peer, i, o),
            )
            .unwrap();
        service.update(catalog(2, ROWS)).unwrap();
        session.tick(&service).unwrap();
        let submissions = log.borrow().writes.len();
        if changed {
            service.unselect(&id(1)).unwrap();
            assert!(session.check(&service).is_err());
        } else {
            assert!(session.tick(&service).is_err());
        }
        assert_eq!(log.borrow().writes.len(), submissions);
        assert_eq!(log.borrow().releases, 1);
        assert_eq!(state.borrow().stops, 1);
        assert!(state.borrow().observation.graph.links.is_empty());
    }
}
#[test]
fn partial_route_failure_and_control_faults_never_retry_and_release_both_owners() {
    for kind in 0..5 {
        let (mut service, baseline, destinations, ownership) = setup();
        let (backend, state) = backend(baseline, ownership);
        let (peer, log) = ScriptedPeer::new(vec![if kind == 4 {
            Reply::Malformed
        } else {
            reply(1, 4)
        }])
        .unwrap();
        if kind == 0 {
            state.borrow_mut().fail_at = Some(4);
        }
        if kind == 1 {
            log.borrow_mut().fail_write = Some(3);
        }
        if kind == 2 {
            log.borrow_mut().short_write = Some(4);
        }
        if kind == 3 {
            log.borrow_mut().bad_initialization = true;
        }
        let mut session = LinkSession::default();
        let start = session.start(
            &service,
            destinations,
            options(3),
            || Ok(backend),
            |i, o| open(peer, i, o),
        );
        if kind == 3 {
            assert!(start.is_err());
        } else {
            start.unwrap();
            service.update(catalog(2, ROWS)).unwrap();
            assert!(session.tick(&service).is_err());
        }
        let submissions = log.borrow().writes.len();
        assert!(session.tick(&service).is_err());
        assert_eq!(log.borrow().writes.len(), submissions);
        assert_eq!(log.borrow().releases, 1);
        assert_eq!(state.borrow().stops, 1);
        assert!(state.borrow().observation.graph.links.is_empty());
        assert_eq!(session.status().phase, Phase::Stopped);
    }
}
#[test]
fn unverified_cleanup_blocks_reopening_and_known_open_failure_stops_routes() {
    for kind in 0..3 {
        let (service, baseline, destinations, ownership) = setup();
        let (backend, state) = backend(baseline, ownership);
        let (peer, log) = ScriptedPeer::new(vec![reply(1, 4)]).unwrap();
        if kind == 0 {
            log.borrow_mut().release_fails = true;
        }
        if kind == 1 {
            state.borrow_mut().cleanup_fails = true;
        }
        let mut session = LinkSession::default();
        let start = session.start(
            &service,
            destinations.clone(),
            options(2),
            || Ok(backend),
            |i, o| {
                if kind == 2 {
                    Err(OpenFailure {
                        message: "controlled open failure".into(),
                        cleanup: Err("unverified claim restoration".into()),
                    })
                } else {
                    open(peer, i, o)
                }
            },
        );
        if kind == 2 {
            assert!(start.is_err());
        } else {
            start.unwrap();
            assert!(session.stop(StopReason::Owner).is_err());
        }
        assert_eq!(session.status().phase, Phase::NeedsReview);
        assert_eq!(state.borrow().stops, 1);
        assert!(session
            .start(
                &service,
                destinations,
                options(2),
                || panic!("must not open routes"),
                |_, _| panic!("must not open control")
            )
            .is_err());
    }
}
#[test]
fn explicit_selected_gate_keeps_default_single_app_policy_and_frame_bounds() {
    let (service, _, _, _) = setup();
    let inv = service.inventory().unwrap();
    assert!(Publication::bounded(inv, 2).is_err());
    assert!(Publication::from_inventory(inv).is_err());
    assert!(Publication::bounded_selected(inv, 2, PollPolicy::Strict).is_ok());
    for cycles in [0, 41] {
        assert!(Publication::bounded_selected(inv, cycles, PollPolicy::Strict).is_err());
    }
    let mut invalid = inv.clone();
    invalid.applications[1].advertised_name = invalid.applications[0].advertised_name.clone();
    assert!(Publication::bounded_selected(&invalid, 2, PollPolicy::Strict).is_err());
    invalid = inv.clone();
    for (n, a) in invalid.applications.iter_mut().enumerate() {
        a.advertised_name = vec![b'A' + n as u8; 255];
    }
    assert!(Publication::bounded_selected(&invalid, 2, PollPolicy::Strict).is_err());
    invalid = inv.clone();
    invalid.applications.clear();
    assert!(Publication::bounded_selected(&invalid, 2, PollPolicy::Strict).is_err());
}

fn managed_options() -> ManagedOptions {
    ManagedOptions {
        poll_policy: PollPolicy::ReportRejectedMarker,
        names: ReceivedNamePolicy::WindowsCString,
        initialization_timeout: Duration::from_millis(500),
    }
}
fn open_managed(
    peer: ScriptedPeer,
    inventory: &Inventory,
    options: ManagedOptions,
) -> Result<Handshake<ScriptedPeer>, OpenFailure> {
    Handshake::prepare_publication(
        peer,
        Publication::managed_selected(inventory, options.poll_policy).unwrap(),
    )
    .map_err(|e| OpenFailure {
        message: e.error.to_string(),
        cleanup: Err("unverified setup".into()),
    })
}

#[test]
fn managed_session_retains_owners_beyond_test_budget_then_pause_and_fresh_restart() {
    let (mut service, baseline, destinations, ownership) = setup();
    let (backend, state) = backend(baseline.clone(), ownership.clone());
    let (peer, log) = ScriptedPeer::new(vec![reply(1, 4)]).unwrap();
    let mut session = LinkSession::default();
    session
        .start_managed(
            &service,
            destinations.clone(),
            managed_options(),
            || Ok(backend),
            |i, o| open_managed(peer, i, o),
        )
        .unwrap();
    for n in 2..=129 {
        service.update(catalog(n, ROWS)).unwrap();
        session.tick(&service).unwrap();
    }
    assert_eq!(session.status().phase, Phase::Running);
    assert_eq!(session.status().completed_cycles, 128);
    assert_eq!(session.status().cycle_limit, None);
    assert_eq!(log.borrow().releases, 0);
    assert_eq!(state.borrow().stops, 0);
    assert_eq!(session.status().routes.assignments.len(), 2);
    session.stop(StopReason::Owner).unwrap();
    assert_eq!(log.borrow().releases, 1);
    assert_eq!(state.borrow().stops, 1);
    assert_eq!(state.borrow().observation.graph.links, baseline.graph.links);
    // Restart consumes new owners, not a reset/replay of the previous handles.
    let (next_backend, next_state) = owned::backend(baseline, ownership);
    let (next_peer, next_log) = ScriptedPeer::new(vec![reply(4, 1)]).unwrap();
    session
        .start_managed(
            &service,
            destinations,
            managed_options(),
            || Ok(next_backend),
            |i, o| open_managed(next_peer, i, o),
        )
        .unwrap();
    service.update(catalog(130, ROWS)).unwrap();
    session.tick(&service).unwrap();
    assert_eq!(session.status().completed_cycles, 1);
    drop(session);
    assert_eq!(next_state.borrow().stops, 1);
    assert_eq!(next_log.borrow().releases, 1);
    assert!(next_state.borrow().observation.graph.links.is_empty());
}

#[test]
fn managed_failures_after_long_operation_stop_without_more_submissions_and_block_bad_cleanup() {
    for kind in 0..5 {
        let (mut service, baseline, destinations, ownership) = setup();
        let (backend, state) = backend(baseline, ownership);
        let (peer, log) = ScriptedPeer::new(vec![reply(1, 4)]).unwrap();
        let mut session = LinkSession::default();
        session
            .start_managed(
                &service,
                destinations.clone(),
                managed_options(),
                || Ok(backend),
                |i, o| open_managed(peer, i, o),
            )
            .unwrap();
        for n in 2..=65 {
            service.update(catalog(n, ROWS)).unwrap();
            session.tick(&service).unwrap();
        }
        let before = log.borrow().writes.len();
        match kind {
            0 => {} // stale application observation
            1 => {
                service.unselect(&id(1)).unwrap();
            }
            2 => {
                state.borrow_mut().observation_fails = true;
            }
            3 => {
                log.borrow_mut().changed = true;
            }
            _ => {
                log.borrow_mut().fail_write = Some(before + 1);
            }
        }
        if kind != 0 && kind != 1 {
            service.update(catalog(66, ROWS)).unwrap();
        }
        assert!(session.tick(&service).is_err());
        let stopped = log.borrow().writes.len();
        assert!(stopped <= before + usize::from(kind == 4));
        assert!(session.tick(&service).is_err());
        assert_eq!(log.borrow().writes.len(), stopped);
        assert_eq!(log.borrow().releases, 1);
        assert_eq!(state.borrow().stops, 1);
        assert!(state.borrow().observation.graph.links.is_empty());
        if kind == 2 || kind == 3 {
            assert_eq!(session.status().phase, Phase::NeedsReview);
            assert!(session
                .start_managed(
                    &service,
                    destinations,
                    managed_options(),
                    || panic!("unverified cleanup reopened routes"),
                    |_, _| panic!("reopened USB")
                )
                .is_err());
        } else {
            assert_eq!(session.status().phase, Phase::Stopped);
        }
    }
}

#[test]
fn managed_invalid_options_do_not_construct_owners_and_do_not_change_bounded_policy() {
    let (service, _, destinations, _) = setup();
    let mut session: LinkSession<owned::Fake, Handshake<ScriptedPeer>> = LinkSession::default();
    let mut options = managed_options();
    options.initialization_timeout = Duration::from_secs(6);
    assert!(session
        .start_managed(
            &service,
            destinations,
            options,
            || panic!("invalid options opened routes"),
            |_, _| panic!("invalid options opened control")
        )
        .is_err());
    assert_eq!(session.status().phase, Phase::Idle);
    let inventory = service.inventory().unwrap();
    for cycles in [0, 41, usize::MAX] {
        assert!(Publication::bounded_selected(inventory, cycles, PollPolicy::Strict).is_err());
    }
    let mut invalid = inventory.clone();
    invalid.applications[1].advertised_name = invalid.applications[0].advertised_name.clone();
    assert!(Publication::managed_selected(&invalid, PollPolicy::Strict).is_err());
}

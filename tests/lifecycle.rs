use beacn_link_protocol::lifecycle::{InventoryTracker, LifecycleError};
use beacn_link_protocol::planner::*;
use beacn_link_protocol::{Assignment, Record};

fn app(id: &str, ids: &[(&str, StreamState)]) -> Application {
    Application {
        id: ApplicationId(id.into()),
        advertised_name: b"game".to_vec(),
        streams: ids
            .iter()
            .map(|(id, state)| Stream {
                id: StreamId((*id).into()),
                state: *state,
            })
            .collect(),
    }
}
fn inventory(snapshot_id: u64, applications: Vec<Application>) -> Inventory {
    Inventory {
        snapshot_id,
        applications,
    }
}
fn records() -> [Record; 1] {
    [Record {
        name: b"game".to_vec(),
        assignment: Assignment::from_byte(1),
    }]
}
fn assert_stale(tracker: &InventoryTracker, old: &Plan) {
    assert!(matches!(
        tracker.require_current_snapshot(old),
        Err(LifecycleError::PlanSnapshotMismatch { .. })
    ));
}

#[test]
fn close_then_reopen_same_pid_requires_new_identity_and_fresh_plan() {
    let mut tracker = InventoryTracker::default();
    let first = inventory(
        1,
        vec![app(
            "pw-client:70:pid:2001",
            &[("pw-node:900", StreamState::Live)],
        )],
    );
    let old = plan(&first, &records()).unwrap();
    assert!(matches!(
        tracker.require_current_snapshot(&old),
        Err(LifecycleError::NoSnapshot)
    ));
    let added = tracker.advance(&first).unwrap();
    assert_eq!(
        added.appeared_applications,
        vec![ApplicationId("pw-client:70:pid:2001".into())]
    );
    tracker.require_current_snapshot(&old).unwrap();
    let closed = tracker.advance(&inventory(2, vec![])).unwrap();
    assert_eq!(closed.disappeared_applications, added.appeared_applications);
    assert_eq!(closed.streams[0].stream.state, StreamState::Disappeared);
    assert_eq!(closed.streams[0].previous_state, Some(StreamState::Live));
    assert_stale(&tracker, &old);
    assert!(matches!(
        plan(tracker.current().unwrap(), &records())
            .unwrap()
            .resolutions[0]
            .outcome,
        Outcome::UnknownName
    ));
    let reopened = inventory(
        3,
        vec![app(
            "pw-client:71:pid:2001",
            &[("pw-node:901", StreamState::Live)],
        )],
    );
    let changed = tracker.advance(&reopened).unwrap();
    assert_eq!(changed.appeared_applications[0].0, "pw-client:71:pid:2001");
    assert_eq!(changed.streams[0].previous_state, None);
    let fresh = plan(tracker.current().unwrap(), &records()).unwrap();
    tracker.require_current_snapshot(&fresh).unwrap();
    assert_eq!(
        fresh.proposed_routes().next().unwrap().application_id.0,
        "pw-client:71:pid:2001"
    );
    assert_stale(&tracker, &old);
}

#[test]
fn changing_stream_sets_and_state_force_replanning_without_partial_routes() {
    let mut tracker = InventoryTracker::default();
    let first = inventory(
        10,
        vec![app(
            "a",
            &[
                ("pw-node:900", StreamState::Live),
                ("pw-node:901", StreamState::Live),
            ],
        )],
    );
    tracker.advance(&first).unwrap();
    let old = plan(&first, &records()).unwrap();
    let next = inventory(
        11,
        vec![app(
            "a",
            &[
                ("pw-node:902", StreamState::Live),
                ("pw-node:901", StreamState::Live),
            ],
        )],
    );
    let delta = tracker.advance(&next).unwrap();
    assert_eq!(
        delta
            .streams
            .iter()
            .map(|s| (s.stream.id.0.as_str(), s.stream.state))
            .collect::<Vec<_>>(),
        vec![
            ("pw-node:900", StreamState::Disappeared),
            ("pw-node:902", StreamState::Live)
        ]
    );
    assert_stale(&tracker, &old);
    let fresh = plan(tracker.current().unwrap(), &records()).unwrap();
    assert_eq!(
        fresh.proposed_routes().next().unwrap().stream_ids,
        vec![
            StreamId("pw-node:901".into()),
            StreamId("pw-node:902".into())
        ]
    );
    let mut stale = next.clone();
    stale.snapshot_id = 12;
    stale.applications[0].streams[0].state = StreamState::Stale;
    let delta = tracker.advance(&stale).unwrap();
    assert_eq!(delta.streams[0].previous_state, Some(StreamState::Live));
    assert!(plan(tracker.current().unwrap(), &records())
        .unwrap()
        .proposed_routes()
        .next()
        .is_none());
    let mut recovered = stale.clone();
    recovered.snapshot_id = 13;
    recovered.applications[0].streams[0].state = StreamState::Live;
    tracker.advance(&recovered).unwrap();
    assert_eq!(
        plan(tracker.current().unwrap(), &records())
            .unwrap()
            .proposed_routes()
            .next()
            .unwrap()
            .stream_ids
            .len(),
        2
    );
}

#[test]
fn rejected_updates_preserve_inventory_and_old_generation_is_not_fresh() {
    let first = inventory(7, vec![app("a", &[("s", StreamState::Live)])]);
    let mut tracker = InventoryTracker::default();
    tracker.advance(&first).unwrap();
    for id in [7, 6, 0] {
        assert!(matches!(
            tracker.advance(&inventory(id, vec![])),
            Err(LifecycleError::OutOfOrderSnapshot { .. })
        ));
        assert_eq!(tracker.current(), Some(&first));
    }
    let bad = inventory(8, vec![app("b", &[("s", StreamState::Live)])]);
    assert!(matches!(
        tracker.advance(&bad),
        Err(LifecycleError::StreamOwnerChanged { .. })
    ));
    assert_eq!(tracker.current(), Some(&first));
    let duplicate = inventory(
        8,
        vec![app(
            "a",
            &[("s", StreamState::Live), ("s", StreamState::Live)],
        )],
    );
    assert!(matches!(
        tracker.advance(&duplicate),
        Err(LifecycleError::InvalidInventory(_))
    ));
    assert_eq!(tracker.current(), Some(&first));
    let old = plan(&first, &records()).unwrap();
    let mut same = first.clone();
    same.snapshot_id = 8;
    assert!(tracker.advance(&same).unwrap().streams.is_empty());
    assert_stale(&tracker, &old);
}

#[test]
fn deltas_are_order_independent_and_missing_tombstones_are_emitted_once() {
    let mut a = InventoryTracker::default();
    let mut b = InventoryTracker::default();
    let first = inventory(
        1,
        vec![
            app("z", &[("s2", StreamState::Live)]),
            app("a", &[("s1", StreamState::Stale)]),
        ],
    );
    let mut reversed = first.clone();
    reversed.applications.reverse();
    assert_eq!(a.advance(&first).unwrap(), b.advance(&reversed).unwrap());
    let empty = inventory(2, vec![]);
    assert_eq!(a.advance(&empty).unwrap(), b.advance(&empty).unwrap());
    assert!(a.advance(&inventory(3, vec![])).unwrap().streams.is_empty());
}

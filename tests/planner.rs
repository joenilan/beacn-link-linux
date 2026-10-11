use beacn_link_protocol::planner::*;
use beacn_link_protocol::{Assignment, Record};

fn stream(id: &str, state: StreamState) -> Stream {
    Stream {
        id: StreamId(id.into()),
        state,
    }
}
fn app(id: &str, name: &str, streams: Vec<Stream>) -> Application {
    Application {
        id: ApplicationId(id.into()),
        advertised_name: name.as_bytes().to_vec(),
        streams,
    }
}
fn inventory(applications: Vec<Application>) -> Inventory {
    Inventory {
        snapshot_id: 17,
        applications,
    }
}
fn record(name: &str, value: u8) -> Record {
    Record {
        name: name.as_bytes().to_vec(),
        assignment: Assignment::from_byte(value),
    }
}

#[test]
fn proposes_all_four_links_and_multiple_sorted_streams() {
    let apps: Vec<_> = (1..=4)
        .map(|i| {
            app(
                &format!("a{i}"),
                &format!("app{i}"),
                vec![
                    stream(&format!("s{i}-b"), StreamState::Live),
                    stream(&format!("s{i}-a"), StreamState::Live),
                ],
            )
        })
        .collect();
    let records: Vec<_> = (1..=4).map(|i| record(&format!("app{i}"), i)).collect();
    let result = plan(&inventory(apps), &records).unwrap();
    assert_eq!(result.snapshot_id, 17);
    assert_eq!(result.proposed_routes().count(), 4);
    for (i, route) in result.proposed_routes().enumerate() {
        assert_eq!(route.link.assignment(), i as u8 + 1);
        assert_eq!(
            route.stream_ids,
            vec![
                StreamId(format!("s{}-a", i + 1)),
                StreamId(format!("s{}-b", i + 1))
            ]
        );
    }
}
#[test]
fn reports_unknown_and_colliding_names_without_guessing_or_case_folding() {
    let input = inventory(vec![
        app("b", "game", vec![stream("sb", StreamState::Live)]),
        app("a", "game", vec![stream("sa", StreamState::Live)]),
    ]);
    let result = plan(
        &input,
        &[record("game", 1), record("Game", 1), record("missing", 2)],
    )
    .unwrap();
    assert_eq!(result.proposed_routes().count(), 0);
    assert_eq!(result.resolutions[0].outcome, Outcome::UnknownName);
    assert_eq!(
        result.resolutions[1].outcome,
        Outcome::AmbiguousName {
            application_ids: vec![ApplicationId("a".into()), ApplicationId("b".into())],
        }
    );
    assert_eq!(result.resolutions[2].outcome, Outcome::UnknownName);
}
#[test]
fn stale_disappeared_or_absent_streams_block_whole_application() {
    let input = inventory(vec![
        app(
            "a",
            "game",
            vec![
                stream("live", StreamState::Live),
                stream("stale", StreamState::Stale),
                stream("gone", StreamState::Disappeared),
            ],
        ),
        app("b", "voice", vec![]),
    ]);
    let result = plan(&input, &[record("game", 1), record("voice", 2)]).unwrap();
    assert_eq!(result.proposed_routes().count(), 0);
    assert_eq!(
        result.resolutions[0].outcome,
        Outcome::UnavailableStreams {
            application_id: ApplicationId("a".into()),
            streams: vec![
                stream("gone", StreamState::Disappeared),
                stream("stale", StreamState::Stale)
            ],
        }
    );
    assert_eq!(
        result.resolutions[1].outcome,
        Outcome::NoStreams {
            application_id: ApplicationId("b".into())
        }
    );
    assert!(plan(&input, &[]).unwrap().resolutions.is_empty()); // No implicit unroute.
}
#[test]
fn identical_duplicates_coalesce_conflicts_and_opaque_values_do_not_route() {
    let input = inventory(vec![app("a", "game", vec![stream("s", StreamState::Live)])]);
    let same = plan(&input, &[record("game", 1), record("game", 1)]).unwrap();
    assert_eq!(same.proposed_routes().count(), 1);
    assert_eq!(same.resolutions[0].occurrences, 2);
    for values in [[1, 2], [0, 1], [1, 255]] {
        let result = plan(
            &input,
            &[record("game", values[0]), record("game", values[1])],
        )
        .unwrap();
        assert_eq!(result.proposed_routes().count(), 0);
        let mut expected = values.to_vec();
        expected.sort();
        assert_eq!(
            result.resolutions[0].outcome,
            Outcome::ConflictingAssignments { values: expected }
        );
    }
    for value in [0, 5, 255] {
        let result = plan(&input, &[record("game", value)]).unwrap();
        assert_eq!(result.proposed_routes().count(), 0);
        assert_eq!(
            result.resolutions[0].outcome,
            Outcome::OpaqueAssignment { value }
        );
    }
}
#[test]
fn valid_outputs_are_deterministic_under_inventory_and_record_reordering() {
    let mut input = inventory(vec![
        app(
            "a",
            "game",
            vec![
                stream("z", StreamState::Live),
                stream("x", StreamState::Live),
            ],
        ),
        app("b", "voice", vec![stream("v", StreamState::Stale)]),
        app("c", "collision", vec![stream("c", StreamState::Live)]),
        app("d", "collision", vec![stream("d", StreamState::Live)]),
    ]);
    let mut records = vec![
        record("voice", 2),
        record("game", 1),
        record("game", 1),
        record("collision", 4),
    ];
    let before = plan(&input, &records).unwrap();
    input.applications.reverse();
    for app in &mut input.applications {
        app.streams.reverse();
    }
    records.reverse();
    assert_eq!(plan(&input, &records).unwrap(), before);
}
#[test]
fn invalid_local_identity_snapshot_aborts_without_a_partial_plan() {
    let live = stream("s", StreamState::Live);
    assert_eq!(
        plan(&inventory(vec![app("", "game", vec![])]), &[]),
        Err(InventoryError::EmptyApplicationId)
    );
    assert_eq!(
        plan(
            &inventory(vec![app("a", "game", vec![]), app("a", "voice", vec![])]),
            &[]
        ),
        Err(InventoryError::DuplicateApplicationId {
            id: ApplicationId("a".into())
        })
    );
    assert_eq!(
        plan(
            &inventory(vec![app("a", "game", vec![stream("", StreamState::Live)])]),
            &[]
        ),
        Err(InventoryError::EmptyStreamId {
            application_id: ApplicationId("a".into())
        })
    );
    for input in [
        inventory(vec![app("a", "game", vec![live.clone(), live.clone()])]),
        inventory(vec![
            app("a", "game", vec![live.clone()]),
            app("b", "voice", vec![live.clone()]),
        ]),
    ] {
        assert_eq!(
            plan(&input, &[record("game", 1)]),
            Err(InventoryError::DuplicateStreamId {
                id: StreamId("s".into())
            })
        );
    }
}
#[test]
fn unsupported_ascii_names_and_conservative_input_limits_are_explicit() {
    let input = inventory(vec![app("a", "game", vec![stream("s", StreamState::Live)])]);
    let bad = Record {
        assignment: Assignment::from_byte(1),
        name: vec![0xC3, 0xA9],
    };
    let result = plan(&input, &[bad]).unwrap();
    assert!(matches!(
        result.resolutions[0].outcome,
        Outcome::UnsupportedName { .. }
    ));
    let mut bad_inventory = input.clone();
    bad_inventory.applications[0].advertised_name = vec![0];
    assert!(matches!(
        plan(&bad_inventory, &[]),
        Err(InventoryError::UnsupportedAdvertisedName { .. })
    ));
    assert_eq!(
        plan(&input, &vec![record("game", 1); MAX_RECORDS + 1]),
        Err(InventoryError::TooManyRecords {
            count: MAX_RECORDS + 1
        })
    );
    assert_eq!(
        plan(
            &inventory(vec![input.applications[0].clone(); MAX_APPLICATIONS + 1]),
            &[]
        ),
        Err(InventoryError::TooManyApplications {
            count: MAX_APPLICATIONS + 1
        })
    );
    let streams: Vec<_> = (0..=MAX_STREAMS)
        .map(|n| stream(&format!("s{n}"), StreamState::Live))
        .collect();
    assert_eq!(
        plan(&inventory(vec![app("a", "game", streams)]), &[]),
        Err(InventoryError::TooManyStreams {
            count: MAX_STREAMS + 1
        })
    );
}

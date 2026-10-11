use beacn_link_protocol::executor::*;
use beacn_link_protocol::lifecycle::InventoryTracker;
use beacn_link_protocol::planner::*;
use beacn_link_protocol::{Assignment, Record};
use std::collections::BTreeSet;

fn obj(id: u32, serial: u64) -> ObjectRef {
    ObjectRef { id, serial }
}
fn node(id: u32) -> ObjectRef {
    obj(id, u64::from(id) * 10)
}
fn p(id: u32, n: u32, d: Direction, c: Channel) -> Port {
    Port {
        object: node(id),
        node: node(n),
        direction: d,
        channel: c,
    }
}
fn inventory() -> Inventory {
    Inventory {
        snapshot_id: 7,
        applications: vec![Application {
            id: ApplicationId("app".into()),
            advertised_name: b"game".to_vec(),
            streams: vec![Stream {
                id: StreamId("s".into()),
                state: StreamState::Live,
            }],
        }],
    }
}
fn frame(a: u8) -> [Record; 1] {
    [Record {
        assignment: Assignment::from_byte(a),
        name: b"game".to_vec(),
    }]
}
fn graph() -> GraphSnapshot {
    GraphSnapshot {
        generation: 7,
        core_id: "private-session-1".into(),
        nodes: vec![node(10), node(20), node(30)],
        ports: vec![
            p(11, 10, Direction::Output, Channel::Left),
            p(12, 10, Direction::Output, Channel::Right),
            p(21, 20, Direction::Input, Channel::Left),
            p(22, 20, Direction::Input, Channel::Right),
            p(31, 30, Direction::Input, Channel::Left),
            p(32, 30, Direction::Input, Channel::Right),
        ],
        streams: vec![StreamBinding {
            application_id: ApplicationId("app".into()),
            stream_id: StreamId("s".into()),
            node: node(10),
        }],
        links: vec![
            ExistingLink {
                object: node(40),
                output: node(11),
                input: node(21),
                owner: Some("ours".into()),
            },
            ExistingLink {
                object: node(41),
                output: node(12),
                input: node(22),
                owner: Some("ours".into()),
            },
        ],
    }
}
fn tracker(i: &Inventory) -> InventoryTracker {
    let mut t = InventoryTracker::default();
    t.advance(i).unwrap();
    t
}
fn owned() -> Ownership {
    Ownership {
        token: "ours".into(),
        links: BTreeSet::from([node(40), node(41)]),
    }
}
fn dest() -> [Destination; 1] {
    [Destination {
        assignment: 2,
        node: node(30),
    }]
}
fn prep(
    g: &GraphSnapshot,
    fresh: &GraphSnapshot,
    own: &Ownership,
) -> Result<PreparedChange, PrepareError> {
    let i = inventory();
    prepare(
        &tracker(&i),
        &plan(&i, &frame(2)).unwrap(),
        g,
        fresh,
        &dest(),
        own,
    )
}

#[test]
fn move_is_deterministic_and_rollback_covers_only_acknowledged_prefix() {
    let g = graph();
    let result = prep(&g, &g, &owned()).unwrap();
    assert_eq!(result.steps.len(), 4);
    assert!(
        matches!(&result.steps[0].operation,Operation::Disconnect{link,..} if link.object==node(40))
    );
    assert!(
        matches!(&result.steps[1].operation,Operation::Disconnect{link,..} if link.object==node(41))
    );
    assert!(
        matches!(&result.steps[2].operation,Operation::Connect{output,input,..} if output.object==node(11) && input.object==node(31))
    );
    assert!(
        matches!(&result.steps[3].operation,Operation::Connect{output,input,..} if output.object==node(12) && input.object==node(32))
    );
    assert!(result.rollback_for_prefix(0).unwrap().is_empty());
    assert!(matches!(
        &result.rollback_for_prefix(3).unwrap()[0],
        Undo::RemoveCreated {
            forward_step: 2,
            ..
        }
    ));
    assert!(
        matches!(&result.rollback_for_prefix(2).unwrap()[0],Undo::Restore{previous,..} if previous.object==node(41))
    );
    assert_eq!(
        result.rollback_for_prefix(5),
        Err(PrepareError::InvalidPrefix)
    );
    let mut reverse = g.clone();
    reverse.nodes.reverse();
    reverse.ports.reverse();
    reverse.links.reverse();
    assert_eq!(prep(&reverse, &reverse, &owned()).unwrap(), result);
}

#[test]
fn all_four_explicit_mappings_work_and_correct_foreign_links_are_preserved() {
    let i = inventory();
    let t = tracker(&i);
    let mut g = graph();
    g.links.clear();
    for assignment in 1..=4 {
        let prepared = prepare(
            &t,
            &plan(&i, &frame(assignment)).unwrap(),
            &g,
            &g,
            &[Destination {
                assignment,
                node: node(30),
            }],
            &owned(),
        )
        .unwrap();
        assert_eq!(prepared.steps.len(), 2);
    }
    let mut correct = graph();
    correct.links[0].input = node(31);
    correct.links[1].input = node(32);
    for l in &mut correct.links {
        l.owner = None;
    }
    assert!(prep(
        &correct,
        &correct,
        &Ownership {
            token: "ours".into(),
            links: BTreeSet::new()
        }
    )
    .unwrap()
    .steps
    .is_empty());
    for assignment in [0, 5, 255] {
        assert_eq!(
            prepare(
                &t,
                &plan(&i, &frame(2)).unwrap(),
                &g,
                &g,
                &[Destination {
                    assignment,
                    node: node(30)
                }],
                &owned()
            ),
            Err(PrepareError::UnknownAssignment(assignment))
        );
    }
    assert_eq!(
        prepare(&t, &plan(&i, &frame(2)).unwrap(), &g, &g, &[], &owned()),
        Err(PrepareError::MissingDestination(2))
    );
}

#[test]
fn stale_generation_core_change_and_disappeared_or_reused_nodes_block() {
    let i = inventory();
    let mut t = tracker(&i);
    let pl = plan(&i, &frame(2)).unwrap();
    let g = graph();
    let mut next = i.clone();
    next.snapshot_id = 8;
    t.advance(&next).unwrap();
    assert!(matches!(
        prepare(&t, &pl, &g, &g, &dest(), &owned()),
        Err(PrepareError::Lifecycle(_))
    ));
    let mut fresh = g.clone();
    fresh.generation = 8;
    assert_eq!(
        prep(&g, &fresh, &owned()),
        Err(PrepareError::GenerationMismatch)
    );
    fresh = g.clone();
    fresh.core_id = "restarted-server".into();
    assert_eq!(prep(&g, &fresh, &owned()), Err(PrepareError::CoreChanged));
    fresh = g.clone();
    fresh.nodes.retain(|n| n.id != 10);
    fresh.ports.retain(|p| p.node.id != 10);
    fresh.links.clear();
    fresh.streams.clear();
    assert_eq!(
        prep(&g, &fresh, &owned()),
        Err(PrepareError::MissingObject(node(10)))
    );
    fresh = g.clone();
    let reused = obj(10, 999);
    fresh.nodes[0] = reused;
    for p in &mut fresh.ports {
        if p.node.id == 10 {
            p.node = reused;
        }
    }
    fresh.streams[0].node = reused;
    assert_eq!(
        prep(&g, &fresh, &owned()),
        Err(PrepareError::ReusedObject {
            expected: node(10),
            observed: reused
        })
    );
}

#[test]
fn owner_tag_without_receipt_and_unexpected_or_reused_links_block() {
    let g = graph();
    let no_receipts = Ownership {
        token: "ours".into(),
        links: BTreeSet::new(),
    };
    assert_eq!(
        prep(&g, &g, &no_receipts),
        Err(PrepareError::UnownedLink(node(40)))
    );
    let mut foreign = g.clone();
    foreign.links[0].owner = None;
    assert_eq!(
        prep(&foreign, &foreign, &owned()),
        Err(PrepareError::UnownedLink(node(40)))
    );
    let mut fresh = g.clone();
    fresh.links[0].object = obj(40, 900);
    assert_eq!(
        prep(&g, &fresh, &owned()),
        Err(PrepareError::ReusedObject {
            expected: node(40),
            observed: obj(40, 900)
        })
    );
    fresh = g.clone();
    fresh.links[0].owner = Some("another-owner".into());
    assert_eq!(
        prep(&g, &fresh, &owned()),
        Err(PrepareError::RouteChanged(StreamId("s".into())))
    );
    let mut empty = g.clone();
    empty.links.clear();
    assert_eq!(
        prep(&empty, &g, &owned()),
        Err(PrepareError::RouteChanged(StreamId("s".into())))
    );
}

#[test]
fn blocked_outcomes_and_fabricated_plan_contents_cannot_be_partially_applied() {
    let i = inventory();
    let t = tracker(&i);
    let g = graph();
    for assignment in [0, 5] {
        assert!(matches!(
            prepare(
                &t,
                &plan(&i, &frame(assignment)).unwrap(),
                &g,
                &g,
                &dest(),
                &owned()
            ),
            Err(PrepareError::BlockedResolution(_))
        ));
    }
    let mut fabricated = plan(&i, &frame(2)).unwrap();
    if let Outcome::Proposed(route) = &mut fabricated.resolutions[0].outcome {
        route.stream_ids.clear();
    }
    assert_eq!(
        prepare(&t, &fabricated, &g, &g, &dest(), &owned()),
        Err(PrepareError::PlanContents)
    );
    fabricated = plan(&i, &frame(2)).unwrap();
    fabricated.resolutions[0].occurrences = usize::MAX;
    assert_eq!(
        prepare(&t, &fabricated, &g, &g, &dest(), &owned()),
        Err(PrepareError::PlanContents)
    );
    let mut bad_i = i.clone();
    bad_i.applications[0].streams[0].state = StreamState::Disappeared;
    assert!(matches!(
        prepare(
            &tracker(&bad_i),
            &plan(&bad_i, &frame(2)).unwrap(),
            &g,
            &g,
            &dest(),
            &owned()
        ),
        Err(PrepareError::BlockedResolution(_))
    ));
}

#[test]
fn ambiguous_ports_destinations_and_fanout_are_rejected() {
    let g = graph();
    let mut duplicate = g.clone();
    duplicate
        .ports
        .push(p(13, 10, Direction::Output, Channel::Left));
    assert_eq!(
        prep(&duplicate, &duplicate, &owned()),
        Err(PrepareError::AmbiguousStereo(node(10)))
    );
    let i = inventory();
    let pl = plan(&i, &frame(2)).unwrap();
    assert_eq!(
        prepare(
            &tracker(&i),
            &pl,
            &g,
            &g,
            &[dest()[0].clone(), dest()[0].clone()],
            &owned()
        ),
        Err(PrepareError::InvalidMapping)
    );
    assert_eq!(
        prepare(
            &tracker(&i),
            &pl,
            &g,
            &g,
            &[
                Destination {
                    assignment: 1,
                    node: node(30)
                },
                dest()[0].clone()
            ],
            &owned()
        ),
        Err(PrepareError::InvalidMapping)
    );
    let mut fanout = g.clone();
    fanout.links.push(ExistingLink {
        object: node(42),
        output: node(11),
        input: node(31),
        owner: Some("ours".into()),
    });
    assert_eq!(
        prep(&fanout, &fanout, &owned()),
        Err(PrepareError::AmbiguousRoute(node(11)))
    );
    let mut swapped = g.clone();
    swapped.ports[0].channel = Channel::Right;
    swapped.ports[1].channel = Channel::Left;
    assert!(matches!(
        prep(&g, &swapped, &owned()),
        Err(PrepareError::ChangedPort(_))
    ));
}

#[test]
fn each_step_requires_fresh_endpoints_ownership_and_acknowledged_disconnects() {
    let i = inventory();
    let t = tracker(&i);
    let g = graph();
    let prepared = prep(&g, &g, &owned()).unwrap();
    validate_step(&prepared, 0, &t, &g, &owned()).unwrap();
    assert!(matches!(
        validate_step(&prepared, 2, &t, &g, &owned()),
        Err(PrepareError::RouteChanged(_))
    ));
    let mut disconnected = g.clone();
    disconnected.links.clear();
    validate_step(&prepared, 2, &t, &disconnected, &owned()).unwrap();
    let mut reused = disconnected.clone();
    reused.ports[4].object = obj(31, 999);
    assert!(matches!(
        validate_step(&prepared, 2, &t, &reused, &owned()),
        Err(PrepareError::ReusedObject { .. })
    ));
    assert!(matches!(
        validate_step(
            &prepared,
            0,
            &t,
            &g,
            &Ownership {
                token: "ours".into(),
                links: BTreeSet::new()
            }
        ),
        Err(PrepareError::UnownedLink(_))
    ));
    assert_eq!(
        validate_step(&prepared, 99, &t, &g, &owned()),
        Err(PrepareError::InvalidStep)
    );
}

#[test]
fn cli_application_is_explicitly_disabled() {
    let p = std::process::Command::new(env!("CARGO_BIN_EXE_link-protocol"))
        .arg("apply")
        .output()
        .unwrap();
    assert_eq!(p.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&p.stderr).contains("application is disabled"));
}

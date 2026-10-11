use beacn_link_protocol::planner::{ApplicationId, StreamId};
use beacn_link_protocol::{executor::*, receipts::*};
use std::collections::BTreeSet;
fn o(id: u32, serial: u64) -> ObjectRef {
    ObjectRef { id, serial }
}
fn setup() -> (PreparedChange, GraphSnapshot, Ownership) {
    let source = o(10, 100);
    let sink = o(20, 200);
    let output = Port {
        object: o(11, 110),
        node: source,
        direction: Direction::Output,
        channel: Channel::Left,
    };
    let input = Port {
        object: o(21, 210),
        node: sink,
        direction: Direction::Input,
        channel: Channel::Left,
    };
    let stream = StreamBinding {
        application_id: ApplicationId("app".into()),
        stream_id: StreamId("s".into()),
        node: source,
    };
    let link = ExistingLink {
        object: o(30, 300),
        output: output.object,
        input: input.object,
        owner: Some("ours".into()),
    };
    let graph = GraphSnapshot {
        generation: 7,
        core_id: "pw-core:1".into(),
        nodes: vec![source, sink],
        ports: vec![output.clone(), input.clone()],
        streams: vec![stream.clone()],
        links: vec![link.clone()],
    };
    let prepared = PreparedChange {
        generation: 7,
        core_id: graph.core_id.clone(),
        steps: vec![
            GuardedStep {
                stream: stream.clone(),
                operation: Operation::Disconnect {
                    link: link.clone(),
                    output: output.clone(),
                    input: input.clone(),
                },
                undo: Undo::Restore {
                    previous: link.clone(),
                    output: output.clone(),
                    input: input.clone(),
                },
            },
            GuardedStep {
                stream,
                operation: Operation::Connect {
                    output: output.clone(),
                    input: input.clone(),
                    owner: "ours".into(),
                },
                undo: Undo::RemoveCreated {
                    forward_step: 1,
                    output,
                    input,
                    owner: "ours".into(),
                },
            },
        ],
    };
    (
        prepared,
        graph,
        Ownership {
            token: "ours".into(),
            links: BTreeSet::from([link.object]),
        },
    )
}
#[test]
fn receipts_resolve_reverse_rollback_and_allow_reused_id_with_new_serial() {
    let (p, g, own) = setup();
    let mut j = ReceiptJournal::new(p, &g, own).unwrap();
    j.acknowledge(0, Acknowledgment::Disconnected(o(30, 300)))
        .unwrap();
    assert!(j.ownership().links.is_empty());
    let mut created = g.links[0].clone();
    created.object = o(30, 301);
    j.acknowledge(1, Acknowledgment::Created(created.clone()))
        .unwrap();
    assert_eq!(j.prefix(), 2);
    assert_eq!(j.state(), JournalState::Complete);
    assert!(j.ownership().links.contains(&created.object));
    assert!(matches!(&j.rollback()[0],ReceiptUndo::RemoveCreated(l) if *l==created));
    assert!(
        matches!(&j.rollback()[1],ReceiptUndo::Restore{previous,..} if previous.object==o(30,300))
    );
}
#[test]
fn invalid_and_out_of_order_acknowledgments_leave_prefix_and_ledger_unchanged() {
    let (p, g, own) = setup();
    let mut j = ReceiptJournal::new(p, &g, own.clone()).unwrap();
    assert_eq!(
        j.acknowledge(1, Acknowledgment::Disconnected(o(30, 300))),
        Err(ReceiptError::OutOfOrder)
    );
    assert_eq!(
        j.acknowledge(0, Acknowledgment::Disconnected(o(30, 999))),
        Err(ReceiptError::Mismatch)
    );
    assert_eq!(j.prefix(), 0);
    assert_eq!(j.ownership(), &own);
    j.acknowledge(0, Acknowledgment::Disconnected(o(30, 300)))
        .unwrap();
    for object in [o(11, 301), o(30, 300)] {
        let mut l = g.links[0].clone();
        l.object = object;
        assert_eq!(
            j.acknowledge(1, Acknowledgment::Created(l)),
            Err(ReceiptError::IdentityCollision)
        );
        assert_eq!(j.prefix(), 1);
        assert!(j.ownership().links.is_empty());
    }
    let mut l = g.links[0].clone();
    l.object = o(30, 301);
    l.owner = Some("other".into());
    assert_eq!(
        j.acknowledge(1, Acknowledgment::Created(l)),
        Err(ReceiptError::Mismatch)
    );
}
#[test]
fn failure_and_unknown_stop_without_promoting_pending_work() {
    for unknown in [false, true] {
        let (p, g, own) = setup();
        let mut j = ReceiptJournal::new(p, &g, own).unwrap();
        j.acknowledge(0, Acknowledgment::Disconnected(o(30, 300)))
            .unwrap();
        j.stop(1, unknown).unwrap();
        assert_eq!(j.prefix(), 1);
        assert_eq!(
            j.state(),
            if unknown {
                JournalState::Unknown
            } else {
                JournalState::Failed
            }
        );
        assert_eq!(j.rollback().len(), 1);
        assert_eq!(
            j.acknowledge(1, Acknowledgment::Created(g.links[0].clone())),
            Err(ReceiptError::Stopped)
        );
    }
}
#[test]
fn initial_graph_and_ledger_must_match_preparation_domain() {
    let (p, mut g, mut own) = setup();
    g.core_id = "different".into();
    assert!(matches!(
        ReceiptJournal::new(p.clone(), &g, own.clone()),
        Err(ReceiptError::InitialGraph)
    ));
    g.core_id = p.core_id.clone();
    own.links.insert(o(99, 999));
    assert!(matches!(
        ReceiptJournal::new(p, &g, own),
        Err(ReceiptError::InitialGraph)
    ));
}

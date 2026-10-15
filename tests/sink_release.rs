use beacn_link_protocol::{
    executor::*,
    receipts::{verify_sink_release, SinkReceipt},
};
fn object(id: u32) -> ObjectRef {
    ObjectRef {
        id,
        serial: id as u64 + 1000,
    }
}
fn fixture() -> (GraphSnapshot, GraphSnapshot, SinkReceipt) {
    let mut before = GraphSnapshot {
        generation: 1,
        core_id: "pw-core:123".into(),
        nodes: vec![object(1), object(2), object(3)],
        ports: vec![],
        streams: vec![],
        links: vec![],
    };
    for i in 0..8 {
        before.ports.push(Port {
            object: object(10 + i),
            node: object(3),
            direction: Direction::Input,
            channel: Channel::Other,
        });
    }
    for (id, node, direction, channel) in [
        (20, 1, Direction::Output, Channel::Left),
        (21, 1, Direction::Output, Channel::Right),
        (22, 2, Direction::Input, Channel::Left),
        (23, 2, Direction::Input, Channel::Right),
    ] {
        before.ports.push(Port {
            object: object(id),
            node: object(node),
            direction,
            channel,
        });
    }
    for (id, out, input) in [(30, 20, 10), (31, 21, 11), (32, 20, 22)] {
        before.links.push(ExistingLink {
            object: object(id),
            output: object(out),
            input: object(input),
            owner: Some("same-tag".into()),
        });
    }
    let receipt = SinkReceipt {
        node: object(3),
        ports: (10..18).map(object).collect(),
        ownership: Ownership {
            token: "same-tag".into(),
            links: [object(30), object(31)].into_iter().collect(),
        },
    };
    let mut after = before.clone();
    after.nodes.retain(|n| *n != receipt.node);
    after.ports.retain(|p| p.node != receipt.node);
    after
        .links
        .retain(|l| !receipt.ownership.links.contains(&l.object));
    (before, after, receipt)
}
#[test]
fn complete_sink_release_preserves_foreign_same_tag_link_and_accepts_already_absent_receipts() {
    let (before, after, receipt) = fixture();
    assert_eq!(
        verify_sink_release(&before, &after, &receipt)
            .unwrap()
            .acknowledged_links,
        2
    );
    assert_eq!(after.links.len(), 1);
    assert_eq!(
        verify_sink_release(&after, &after, &receipt)
            .unwrap()
            .already_absent,
        2
    );
}
#[test]
fn numeric_reuse_and_foreign_graph_changes_cannot_be_removed_or_reported_restored() {
    let (mut before, mut after, receipt) = fixture();
    // An unrelated object reusing a receipt ID has its own generation.
    let foreign = ObjectRef {
        id: 30,
        serial: 9999,
    };
    before.links.retain(|l| l.object != object(30));
    before.links.push(ExistingLink {
        object: foreign,
        output: object(21),
        input: object(23),
        owner: Some("same-tag".into()),
    });
    after.links.push(before.links.last().unwrap().clone());
    assert!(verify_sink_release(&before, &after, &receipt).is_ok());
    after.links.retain(|l| l.object != foreign);
    assert!(verify_sink_release(&before, &after, &receipt).is_err());
}
#[test]
fn incomplete_ledger_changed_owner_and_foreign_links_attached_to_sink_refuse_exact_cleanup() {
    let (mut before, after, mut receipt) = fixture();
    receipt.ports.remove(&object(10));
    assert!(verify_sink_release(&before, &after, &receipt).is_err());
    receipt.ports.insert(object(10));
    before.links[0].owner = Some("foreign".into());
    assert!(verify_sink_release(&before, &after, &receipt).is_err());
    before.links[0].owner = Some("same-tag".into());
    before.links.push(ExistingLink {
        object: object(33),
        output: object(21),
        input: object(12),
        owner: Some("same-tag".into()),
    });
    assert!(verify_sink_release(&before, &after, &receipt).is_err());
}

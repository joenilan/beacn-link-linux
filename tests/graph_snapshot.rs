use beacn_link_protocol::{
    executor::*, graph_snapshot::*, lifecycle::InventoryTracker, planner, Assignment, Record,
};
fn input() -> String {
    let mut text = "inventory\tpw-client:70:pid:2001\tgame\tpw-node:100\tlive\nowner\tours\ndestination\t1\t20\t200\n".to_owned();
    for side in ["expected", "observed"] {
        for row in [
            "core\tpw-core:123",
            "node\t10\t100",
            "node\t20\t200",
            "port\t11\t110\t10\t100\tout\tFL",
            "port\t12\t120\t10\t100\tout\tFR",
            "port\t21\t210\t20\t200\tin\tFL",
            "port\t22\t220\t20\t200\tin\tFR",
            "stream\tpw-client:70:pid:2001\tpw-node:100\t10\t100",
        ] {
            text.push_str(&format!("{side}\t{row}\n"));
        }
    }
    text
}
#[test]
fn bridge_reuses_inventory_and_graph_types_to_prepare() {
    let d = preparation_from_tsv(7, &input()).unwrap();
    let mut t = InventoryTracker::default();
    t.advance(&d.inventory).unwrap();
    let plan = planner::plan(
        &d.inventory,
        &[Record {
            assignment: Assignment::from_byte(1),
            name: b"game".to_vec(),
        }],
    )
    .unwrap();
    let p = prepare(
        &t,
        &plan,
        &d.expected,
        &d.observed,
        &d.destinations,
        &d.ownership,
    )
    .unwrap();
    assert_eq!(p.steps.len(), 2);
    assert_eq!(p.core_id, "pw-core:123");
    assert!(d.ownership.links.is_empty());
}
#[test]
fn bridge_rejects_partial_duplicate_malformed_and_excessive_metadata() {
    for text in [
        input().replace("observed\tcore\tpw-core:123\n", ""),
        input() + "owner\tsecond\n",
        input() + "observed\tnode\t10\t999\n",
        input() + "receipt\t99\t999\nreceipt\t99\t999\n",
        input() + "destination\t1\t-20\t200\n",
        input() + "unexpected\tfield\n",
        input().replace("\tout\tFL", "\tout\tunknown"),
        "x".repeat(MAX_GRAPH_TSV_BYTES + 1),
    ] {
        assert!(preparation_from_tsv(7, &text).is_err());
    }
}
#[test]
fn native_core_change_is_refused_and_tags_do_not_become_receipts() {
    let mut text = input();
    text.push_str("expected\tlink\t30\t300\t11\t110\t21\t210\tours\nobserved\tlink\t30\t300\t11\t110\t21\t210\tours\n");
    let d = preparation_from_tsv(
        7,
        &text.replace("observed\tcore\tpw-core:123", "observed\tcore\tpw-core:124"),
    )
    .unwrap();
    assert!(d.ownership.links.is_empty());
    let mut t = InventoryTracker::default();
    t.advance(&d.inventory).unwrap();
    let plan = planner::plan(&d.inventory, &[]).unwrap();
    assert_eq!(
        prepare(&t, &plan, &d.expected, &d.observed, &[], &d.ownership),
        Err(PrepareError::CoreChanged)
    );
}

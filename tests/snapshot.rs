use beacn_link_protocol::planner::{plan, Outcome};
use beacn_link_protocol::snapshot::{inventory_from_tsv, MAX_TSV_BYTES};
use beacn_link_protocol::{Assignment, Record};
use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn stdin_bridge_preserves_identity_and_surfaces_alias_collisions() {
    let rows = "pw-client:70:pid:2001\tgame\tpw-node:901\tlive\n\
pw-client:70:pid:2001\tgame\tpw-node:900\tlive\n";
    let inventory = inventory_from_tsv(12, rows).unwrap();
    let records = [Record {
        name: b"game".to_vec(),
        assignment: Assignment::from_byte(1),
    }];
    let proposal = plan(&inventory, &records).unwrap();
    assert_eq!(
        proposal.proposed_routes().next().unwrap().stream_ids[0].0,
        "pw-node:900"
    );
    let collision = format!("{rows}pw-client:71:pid:2001\tgame\tpw-node:902\tlive\n");
    let result = plan(&inventory_from_tsv(12, &collision).unwrap(), &records).unwrap();
    assert!(matches!(
        result.resolutions[0].outcome,
        Outcome::AmbiguousName { .. }
    ));
    let mut child = Command::new(env!("CARGO_BIN_EXE_link-protocol"))
        .args([
            "plan-snapshot",
            "12",
            "0e0000ac040167616d650500766f69636500",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(rows.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{:?}", output.stderr);
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("snapshot_id: 12"));
    assert!(text.contains("Proposed(") && text.contains("pw-client:70:pid:2001"));
    assert!(text.contains("OpaqueAssignment"));
}

#[test]
fn stdin_bridge_rejects_partial_or_conflicting_inventory() {
    for rows in [
        "a\tgame\ts\tlive\nb\tgame\ts\tlive\n", // Duplicate stream ownership.
        "a\tgame\ts\tlive\na\tvoice\tt\tlive\n", // One identity, two wire names.
        "a\tgame\ts\tunknown\n",
        "a\tgame\ts\n",
        "a\tgáme\ts\tlive\n",
        "\n",
    ] {
        assert!(inventory_from_tsv(1, rows).is_err(), "{rows:?}");
    }
    assert!(inventory_from_tsv(1, &"x".repeat(MAX_TSV_BYTES + 1)).is_err());
    assert!(inventory_from_tsv(1, "").unwrap().applications.is_empty());
}

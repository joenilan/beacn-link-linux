use beacn_link_protocol::{
    catalog::{self, ApplicationService},
    planner::{self, ApplicationId},
    Assignment, Record,
};
fn snapshot(generation: u64, rows: &str) -> catalog::Catalog {
    catalog::parse(
        generation,
        &format!("catalog\t42\t99\t70697065776972652d30\n{rows}"),
    )
    .unwrap()
}
const A: &str = "app\t10\t123\t20\tlive\t6d7076\t6d7076\n";
const B: &str = "app\t11\t124\t21\tlive\t67616d65\t67616d65\n";
fn id() -> ApplicationId {
    ApplicationId("pw-client:10:pid:123".into())
}
#[test]
fn real_metadata_grouping_keeps_labels_separate_from_explicit_wire_names() {
    let cat = snapshot(
        1,
        &format!("{A}app\t10\t123\t22\tlive\t436166c3a9096e616d650a\t6d7076\n"),
    );
    let a = &cat.applications[&id()];
    assert_eq!(a.streams.len(), 2);
    assert!(a.names.contains("Café\tname\n"));
    let mut service = ApplicationService::default();
    service.update(cat).unwrap();
    service.select(&id(), "Media player").unwrap();
    assert_eq!(
        service.inventory().unwrap().applications[0].advertised_name,
        b"Media player"
    );
    assert!(service.select(&id(), "Café").is_err());
}
#[test]
fn malformed_or_reused_metadata_never_becomes_a_catalog() {
    let good = format!("catalog\t42\t99\t70697065776972652d30\n{A}");
    for bad in [
        good.replace("\t123\t", "\t0\t"),
        good.replace("\t20\t", "\t0\t"),
        good.replace("\tlive\t", "\tdisappeared\t"),
        good.replace("6d7076", "ff"),
        good.replace("6d7076", "abc"),
        format!("{good}{A}"),
        good.trim_end().into(),
        good.replace("catalog\t42", "catalog\t4294967296"),
    ] {
        assert!(catalog::parse(1, &bad).is_err(), "{bad:?}");
    }
    assert!(catalog::parse(0, &good).is_err());
    assert!(catalog::parse(1, &"x".repeat(catalog::MAX_CATALOG_BYTES + 1)).is_err());
}
#[test]
fn catalog_application_and_label_bounds_are_enforced() {
    let rows = (1..=257)
        .map(|n| format!("app\t{n}\t{n}\t{n}\tlive\t61\t62\n"))
        .collect::<String>();
    assert!(catalog::parse(1, &format!("catalog\t1\t1\t61\n{rows}")).is_err());
    assert!(catalog::parse(
        1,
        &format!(
            "catalog\t1\t1\t61\napp\t1\t1\t1\tlive\t{}\t62\n",
            "61".repeat(513)
        )
    )
    .is_err());
}
#[test]
fn duplicate_names_and_frame_overflow_refuse_selection_atomically() {
    let mut service = ApplicationService::default();
    service.update(snapshot(1, &format!("{A}{B}"))).unwrap();
    service.select(&id(), "x".repeat(255).as_str()).unwrap();
    let revision = service.revision();
    let b = ApplicationId("pw-client:11:pid:124".into());
    assert!(service.select(&b, &"x".repeat(255)).is_err());
    assert!(service.select(&b, &"y".repeat(255)).is_err()); // Two records exceed one frame.
    assert!(service
        .select(&ApplicationId("missing".into()), "new")
        .is_err());
    assert_eq!(service.revision(), revision);
    assert_eq!(service.selected().len(), 1);
    service.select(&b, "game").unwrap();
    assert_eq!(service.selected().len(), 2);
}
#[test]
fn short_duplicate_alias_is_refused_without_changing_selection_or_revision() {
    let mut service = ApplicationService::default();
    service.update(snapshot(1, &format!("{A}{B}"))).unwrap();
    service.select(&id(), "Player").unwrap();
    let revision = service.revision();
    let selection_revision = service.selection_revision();
    assert!(service
        .select(&ApplicationId("pw-client:11:pid:124".into()), "Player")
        .is_err());
    assert_eq!(service.revision(), revision);
    assert_eq!(service.selection_revision(), selection_revision);
    assert_eq!(service.selected().len(), 1);
}
#[test]
fn departure_revokes_once_and_reopened_name_needs_fresh_selection() {
    let mut service = ApplicationService::default();
    service.update(snapshot(1, A)).unwrap();
    service.select(&id(), "mpv").unwrap();
    let update = service.update(snapshot(2, "")).unwrap();
    assert_eq!(update.revoked, vec![id()]);
    assert_eq!(update.lifecycle.disappeared_applications, vec![id()]);
    assert!(service.update(snapshot(3, "")).unwrap().revoked.is_empty());
    let reopened = A.replace("\t123\t", "\t125\t").replace("\t20\t", "\t30\t");
    service.update(snapshot(4, &reopened)).unwrap();
    assert!(service.selected().is_empty());
    service
        .select(&ApplicationId("pw-client:10:pid:125".into()), "mpv")
        .unwrap();
}
#[test]
fn core_change_stale_state_and_old_generation_cannot_keep_a_selection() {
    let mut service = ApplicationService::default();
    service.update(snapshot(1, A)).unwrap();
    service.select(&id(), "mpv").unwrap();
    let old_revision = service.revision();
    assert!(service.update(snapshot(1, "")).is_err());
    assert_eq!(service.revision(), old_revision);
    let stale = A.replace("\tlive\t", "\tstale\t");
    assert_eq!(
        service.update(snapshot(2, &stale)).unwrap().revoked,
        vec![id()]
    );
    assert!(service.select(&id(), "mpv").is_err());
    service.update(snapshot(3, A)).unwrap();
    service.select(&id(), "mpv").unwrap();
    let mut different = snapshot(4, A);
    different.core = "pw-core:43:pid:100:pipewire-0".into();
    let update = service.update(different).unwrap();
    assert!(update.core_changed);
    assert_eq!(update.revoked, vec![id()]);
}
#[test]
fn selection_edits_invalidate_old_plans_through_existing_lifecycle_owner() {
    let mut service = ApplicationService::default();
    service.update(snapshot(1, &format!("{A}{B}"))).unwrap();
    service.select(&id(), "mpv").unwrap();
    let plan = planner::plan(
        service.inventory().unwrap(),
        &[Record {
            assignment: Assignment::from_byte(1),
            name: b"mpv".to_vec(),
        }],
    )
    .unwrap();
    service.require_current_plan(&plan).unwrap();
    service
        .select(&ApplicationId("pw-client:11:pid:124".into()), "game")
        .unwrap();
    assert!(service.require_current_plan(&plan).is_err());
    service.unselect(&id()).unwrap();
    assert!(service.unselect(&id()).is_err());
}
#[test]
fn direct_catalog_mutation_cannot_bypass_identity_validation() {
    let mut invalid = snapshot(1, A);
    invalid.applications.get_mut(&id()).unwrap().pid = 999;
    let mut service = ApplicationService::default();
    assert!(service.update(invalid).is_err());
    assert!(service.current().is_none());
}

use beacn_link_protocol::{
    execution::Observation, executor::*, graph_snapshot::preparation_from_tsv,
    planner::StreamState, route_publication::VerifiedRoutePublication,
};

fn fixture(assignment: u8) -> (Observation, Vec<Destination>, Ownership) {
    let mut text = "inventory\tapp\tgame\ts\tlive\nowner\tours\n".to_owned();
    for side in ["expected", "observed"] {
        for row in [
            "core\tpw-core:123",
            "node\t10\t100",
            "node\t20\t200",
            "node\t30\t300",
            "port\t11\t110\t10\t100\tout\tFL",
            "port\t12\t120\t10\t100\tout\tFR",
            "port\t21\t210\t20\t200\tin\tFL",
            "port\t22\t220\t20\t200\tin\tFR",
            "port\t31\t310\t30\t300\tin\tFL",
            "port\t32\t320\t30\t300\tin\tFR",
            "stream\tapp\ts\t10\t100",
        ] {
            text.push_str(&format!("{side}\t{row}\n"));
        }
        if assignment != 0 {
            let left = if assignment == 1 { 21 } else { 31 };
            let right = left + 1;
            text.push_str(&format!("{side}\tlink\t40\t400\t11\t110\t{left}\t{}\tours\n{side}\tlink\t41\t410\t12\t120\t{right}\t{}\tours\n", left * 10, right * 10));
        }
    }
    let data = preparation_from_tsv(7, &text).unwrap();
    let mut ownership = data.ownership;
    ownership
        .links
        .extend(data.observed.links.iter().map(|l| l.object));
    (
        Observation {
            graph: data.observed,
            inventory: data.inventory,
        },
        vec![
            Destination {
                assignment: 1,
                node: ObjectRef {
                    id: 20,
                    serial: 200,
                },
            },
            Destination {
                assignment: 2,
                node: ObjectRef {
                    id: 30,
                    serial: 300,
                },
            },
        ],
        ownership,
    )
}

#[test]
fn verified_data_reports_only_complete_actual_route_or_unlinked_zero() {
    for assignment in [0, 1, 2] {
        let (o, d, own) = fixture(assignment);
        let p = VerifiedRoutePublication::verify(&o, &o, &d, &own, assignment).unwrap();
        assert_eq!(p.inventory(), &o.inventory);
        assert_eq!(p.records()[0].assignment.byte(), assignment);
        assert_eq!(p.records()[0].name, b"game");
    }
    let (o, d, own) = fixture(0);
    assert!(VerifiedRoutePublication::verify(&o, &o, &d, &own, 1).is_err());
    let (o, d, own) = fixture(1);
    for assignment in [0, 2, 5, 255] {
        assert!(VerifiedRoutePublication::verify(&o, &o, &d, &own, assignment).is_err());
    }
}

#[test]
fn tags_ids_and_incomplete_acknowledgments_cannot_verify_a_route() {
    let (mut o, d, mut own) = fixture(1);
    own.links.clear();
    assert!(VerifiedRoutePublication::verify(&o, &o, &d, &own, 1).is_err());
    own.links.extend(o.graph.links.iter().map(|l| l.object));
    own.links.remove(&o.graph.links[0].object);
    assert!(VerifiedRoutePublication::verify(&o, &o, &d, &own, 1).is_err());
    own.links.extend(o.graph.links.iter().map(|l| l.object));
    o.graph.links[0].owner = Some("foreign".into());
    assert!(VerifiedRoutePublication::verify(&o, &o, &d, &own, 1).is_err());
    let (mut o, d, own) = fixture(1);
    o.graph.links.pop();
    assert!(VerifiedRoutePublication::verify(&o, &o, &d, &own, 1).is_err());
}

#[test]
fn changed_core_generation_identity_and_stale_source_are_refused() {
    let (o, d, own) = fixture(1);
    let mut changed = o.clone();
    changed.graph.core_id = "new-core".into();
    assert!(VerifiedRoutePublication::verify(&o, &changed, &d, &own, 1).is_err());
    changed = o.clone();
    changed.graph.generation += 1;
    assert!(VerifiedRoutePublication::verify(&changed, &changed, &d, &own, 1).is_err());
    changed = o.clone();
    changed.inventory.applications[0].streams[0].state = StreamState::Stale;
    assert!(VerifiedRoutePublication::verify(&changed, &changed, &d, &own, 1).is_err());
    let mut replaced = d.clone();
    replaced[0].node.serial += 1;
    assert!(VerifiedRoutePublication::verify(&o, &o, &replaced, &own, 1).is_err());
}

#[test]
fn unlinked_publication_requires_exact_binding_and_stereo_ports() {
    let (mut o, d, own) = fixture(0);
    o.graph.streams.clear();
    assert!(VerifiedRoutePublication::verify(&o, &o, &d, &own, 0).is_err());
    let (mut o, d, own) = fixture(0);
    o.graph.ports[0].channel = Channel::Other;
    assert!(VerifiedRoutePublication::verify(&o, &o, &d, &own, 0).is_err());
}

#[test]
fn playback_revalidation_keeps_original_core_inventory_destinations_and_receipts() {
    let (observed, destinations, ownership) = fixture(1);
    let proof =
        VerifiedRoutePublication::verify(&observed, &observed, &destinations, &ownership, 1)
            .unwrap();
    assert_eq!(proof.destination(1), Some(&destinations[0]));
    assert!(proof.verify_observation(&observed).is_ok());
    for kind in 0..5 {
        let mut changed = observed.clone();
        match kind {
            0 => changed.graph.core_id = "different-core".into(),
            1 => changed.inventory.snapshot_id += 1,
            2 => {
                changed.graph.links.pop();
            }
            3 => changed.graph.links[0].owner = Some("same-ID-foreign-owner".into()),
            _ => changed.inventory.applications[0].advertised_name = b"replacement".to_vec(),
        }
        assert!(proof.verify_observation(&changed).is_err());
    }
}

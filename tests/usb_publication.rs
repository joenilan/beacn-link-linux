use beacn_link_protocol::{
    encode_frame, encode_records,
    session::{PollPolicy, PollReply, Session, Transport, TransportError},
    snapshot,
    usb_handshake::{Backend, Handshake, Observation, Publication},
    Assignment, Frame, Record,
};
use std::{cell::RefCell, rc::Rc, time::Duration};
const T: Duration = Duration::from_millis(500);
fn inventory() -> beacn_link_protocol::planner::Inventory {
    snapshot::inventory_from_tsv(1, "pw-client:44:pid:55\tLinux test\tpw-node:66\tlive\n").unwrap()
}
#[derive(Default)]
struct Journal {
    writes: Vec<Vec<u8>>,
    reads: usize,
    releases: usize,
    fail_write: Option<usize>,
    short_write: Option<usize>,
    changed: bool,
    reply: Vec<u8>,
    bad_a2: bool,
    write_timeouts: Vec<u32>,
    read_timeouts: Vec<u32>,
}
struct Fake {
    log: Rc<RefCell<Journal>>,
    alt: u8,
}
impl Backend for Fake {
    fn observe(&mut self) -> Result<Observation, i32> {
        Ok(Observation {
            instance: if self.log.borrow().changed { 99 } else { 44 },
            descriptors: include_str!("../fixtures/usb-handshake.synthetic.hex")
                .split_whitespace()
                .map(|s| u8::from_str_radix(s, 16).unwrap())
                .collect(),
            configuration: 1,
            alternate: self.alt,
            kernel_bound: false,
            audio_interfaces: (0..3).map(|n| (n, 0, "snd-usb-audio".into())).collect(),
        })
    }
    fn claim_three(&mut self) -> Result<(), i32> {
        Ok(())
    }
    fn select_one(&mut self) -> Result<(), i32> {
        self.alt = 1;
        Ok(())
    }
    fn write(&mut self, b: &[u8], ms: u32) -> Result<usize, i32> {
        let mut l = self.log.borrow_mut();
        l.writes.push(b.to_vec());
        l.write_timeouts.push(ms);
        let n = l.writes.len();
        if l.fail_write == Some(n) {
            Err(-7)
        } else if l.short_write == Some(n) {
            Ok(b.len() - 1)
        } else {
            Ok(b.len())
        }
    }
    fn read(&mut self, b: &mut [u8], ms: u32) -> Result<usize, i32> {
        let mut l = self.log.borrow_mut();
        l.reads += 1;
        l.read_timeouts.push(ms);
        if l.reads == 1 {
            b.fill(0);
            b[..4].copy_from_slice(
                &(if l.bad_a2 {
                    0xA1000000u32
                } else {
                    0xA2000000u32
                })
                .to_le_bytes(),
            );
            Ok(512)
        } else {
            let n = l.reply.len();
            b[..n].copy_from_slice(&l.reply);
            Ok(n)
        }
    }
    fn release_three(&mut self) -> Result<(), i32> {
        self.log.borrow_mut().releases += 1;
        self.alt = 0;
        Ok(())
    }
    fn close(&mut self) {}
}
fn setup(journal: Journal) -> (Rc<RefCell<Journal>>, Session<Handshake<Fake>>) {
    let log = Rc::new(RefCell::new(journal));
    let owner = Handshake::prepare_publication(
        Fake {
            log: log.clone(),
            alt: 0,
        },
        Publication::from_inventory(&inventory()).unwrap(),
    )
    .unwrap();
    (log, Session::new(owner, T).unwrap())
}
fn records() -> Vec<Record> {
    Publication::from_inventory(&inventory())
        .unwrap()
        .records()
        .to_vec()
}

fn verified_route(
    generation: u64,
    assignment: u8,
) -> beacn_link_protocol::route_publication::VerifiedRoutePublication {
    use beacn_link_protocol::{
        execution::Observation, executor::Destination, graph_snapshot::preparation_from_tsv,
        route_publication::VerifiedRoutePublication,
    };
    let mut text =
        "inventory\tpw-client:44:pid:55\tLinux test\tpw-node:66\tlive\nowner\tours\n".to_owned();
    for side in ["expected", "observed"] {
        for row in [
            "core\tpw-core:123",
            "node\t10\t66",
            "node\t20\t200",
            "port\t11\t110\t10\t66\tout\tFL",
            "port\t12\t120\t10\t66\tout\tFR",
            "port\t21\t210\t20\t200\tin\tFL",
            "port\t22\t220\t20\t200\tin\tFR",
            "stream\tpw-client:44:pid:55\tpw-node:66\t10\t66",
        ] {
            text.push_str(&format!("{side}\t{row}\n"));
        }
        if assignment == 1 {
            for row in [
                "link\t40\t400\t11\t110\t21\t210\tours",
                "link\t41\t410\t12\t120\t22\t220\tours",
            ] {
                text.push_str(&format!("{side}\t{row}\n"));
            }
        }
    }
    let parsed = preparation_from_tsv(generation, &text).unwrap();
    let destination = Destination {
        assignment: 1,
        node: parsed.observed.nodes[1],
    };
    let mut ownership = parsed.ownership;
    ownership
        .links
        .extend(parsed.observed.links.iter().map(|l| l.object));
    let observation = Observation {
        inventory: parsed.inventory,
        graph: parsed.observed,
    };
    VerifiedRoutePublication::verify(
        &observation,
        &observation,
        &[destination],
        &ownership,
        assignment,
    )
    .unwrap()
}

fn verified_mode() -> (Rc<RefCell<Journal>>, Session<Handshake<Fake>>) {
    let log = Rc::new(RefCell::new(Journal {
        reply: 0xAC000000u32.to_le_bytes().to_vec(),
        ..Journal::default()
    }));
    let owner = Handshake::prepare_publication(
        Fake {
            log: log.clone(),
            alt: 0,
        },
        Publication::bounded(&inventory(), 2)
            .unwrap()
            .with_verified_routes(),
    )
    .unwrap();
    (log, Session::new(owner, T).unwrap())
}

#[test]
fn verified_mode_can_publish_actual_assignment_and_exhausts_existing_budget() {
    let (log, mut session) = verified_mode();
    session.initialize().unwrap();
    for (generation, assignment) in [(2, 0), (3, 1)] {
        let proof = verified_route(generation, assignment);
        session.refresh_verified_publication(&proof).unwrap();
        session.publish(proof.records()).unwrap();
        session.poll().unwrap();
    }
    assert_eq!(
        log.borrow().writes[4],
        encode_frame(&Frame::Data(
            encode_records(verified_route(3, 1).records()).unwrap()
        ))
        .unwrap()
    );
    let submissions = log.borrow().writes.len();
    assert!(session
        .refresh_verified_publication(&verified_route(4, 1))
        .is_err());
    assert_eq!(log.borrow().writes.len(), submissions);
    assert!(session.into_transport().close().baseline_restored);
}

#[test]
fn verified_mode_refuses_unverified_refresh_frozen_proof_and_wrong_frame() {
    let (log, mut session) = verified_mode();
    session.initialize().unwrap();
    let mut newer = inventory();
    newer.snapshot_id = 2;
    assert!(session.refresh_publication(&newer).is_err());
    assert_eq!(log.borrow().writes.len(), 2);
    let (log, mut session) = verified_mode();
    session.initialize().unwrap();
    let proof = verified_route(2, 1);
    session.refresh_verified_publication(&proof).unwrap();
    assert!(session.publish(&records()).is_err());
    assert_eq!(log.borrow().writes.len(), 2);
    let (log, mut session) = verified_mode();
    session.initialize().unwrap();
    let proof = verified_route(2, 0);
    session.refresh_verified_publication(&proof).unwrap();
    session.publish(proof.records()).unwrap();
    session.poll().unwrap();
    assert!(session.refresh_verified_publication(&proof).is_err());
    assert_eq!(log.borrow().writes.len(), 4);
}

#[test]
fn default_periodic_gate_cannot_enable_verified_route_updates() {
    let log = Rc::new(RefCell::new(Journal::default()));
    let owner = Handshake::prepare_publication(
        Fake {
            log: log.clone(),
            alt: 0,
        },
        Publication::bounded(&inventory(), 2).unwrap(),
    )
    .unwrap();
    let mut session = Session::new(owner, T).unwrap();
    session.initialize().unwrap();
    assert!(session
        .refresh_verified_publication(&verified_route(2, 1))
        .is_err());
    assert_eq!(log.borrow().writes.len(), 2);
}

#[test]
fn one_shot_verified_policy_also_requires_proof_before_first_publication() {
    let log = Rc::new(RefCell::new(Journal::default()));
    let owner = Handshake::prepare_publication(
        Fake {
            log: log.clone(),
            alt: 0,
        },
        Publication::from_inventory(&inventory())
            .unwrap()
            .with_verified_routes(),
    )
    .unwrap();
    let mut session = Session::new(owner, T).unwrap();
    session.initialize().unwrap();
    assert!(session.publish(&records()).is_err());
    assert_eq!(log.borrow().writes.len(), 2);
}
fn initialization_diagnostic() -> (Rc<RefCell<Journal>>, Session<Handshake<Fake>>) {
    let log = Rc::new(RefCell::new(Journal {
        reply: 0xAC000000u32.to_le_bytes().to_vec(),
        ..Default::default()
    }));
    let owner = Handshake::prepare_publication(
        Fake {
            log: log.clone(),
            alt: 0,
        },
        Publication::bounded(&inventory(), 1)
            .unwrap()
            .with_initialization_diagnostic(),
    )
    .unwrap();
    (log, Session::new(owner, T).unwrap())
}
#[test]
fn selected_gate_extends_only_explicit_initial_a2_read() {
    let (log, mut s) = initialization_diagnostic();
    s.initialize_with_read_timeout(Duration::from_secs(5))
        .unwrap();
    let mut next = inventory();
    next.snapshot_id = 2;
    s.refresh_publication(&next).unwrap();
    s.publish(&records()).unwrap();
    s.poll().unwrap();
    assert_eq!(log.borrow().write_timeouts, [500, 500, 500, 500]);
    assert_eq!(log.borrow().read_timeouts, [5000, 500]);
    assert!(s.into_transport().close().baseline_restored);
}
#[test]
fn diagnostic_deadline_cannot_expand_writes_later_reads_or_default_gate() {
    let (log, mut s) = setup(Journal::default());
    assert!(s
        .initialize_with_read_timeout(Duration::from_secs(5))
        .is_err());
    assert_eq!(log.borrow().writes.len(), 2);
    assert_eq!(log.borrow().reads, 0);
    s.into_transport().close();
    for kind in 0..3 {
        let (log, mut s) = initialization_diagnostic();
        if kind == 0 {
            assert!(s
                .initialize_with_read_timeout(Duration::from_millis(5001))
                .is_err());
            assert_eq!(log.borrow().reads, 0);
            s.into_transport().close();
            continue;
        }
        s.initialize_with_read_timeout(Duration::from_secs(5))
            .unwrap();
        let mut next = inventory();
        next.snapshot_id = 2;
        s.refresh_publication(&next).unwrap();
        let mut owner = s.into_transport();
        let frame = encode_frame(&Frame::Data(encode_records(&records()).unwrap())).unwrap();
        if kind == 1 {
            assert!(owner.write(3, &frame, Duration::from_secs(5)).is_err());
        } else {
            owner.write(3, &frame, T).unwrap();
            owner.write(3, &0xAC010000u32.to_le_bytes(), T).unwrap();
            assert!(owner
                .read(0x83, &mut [0; 512], Duration::from_secs(5))
                .is_err());
        }
        assert_eq!(log.borrow().reads, 1);
        owner.close();
    }
}
#[test]
fn exact_publication_and_poll_use_existing_session_and_exhaust_the_gate() {
    let response = encode_frame(&Frame::Data(
        encode_records(&[Record {
            name: b"Linux test".to_vec(),
            assignment: Assignment::from_byte(2),
        }])
        .unwrap(),
    ))
    .unwrap();
    let (log, mut session) = setup(Journal {
        reply: response,
        ..Default::default()
    });
    session.initialize().unwrap();
    session.publish(&records()).unwrap();
    assert!(
        matches!(session.poll().unwrap(),PollReply::Records{records: r,..} if r[0].assignment.byte()==2)
    );
    assert!(session.publish(&records()).is_err());
    assert_eq!(log.borrow().writes.len(), 4);
    assert_eq!(log.borrow().reads, 2);
    assert_eq!(
        log.borrow().writes[2],
        encode_frame(&Frame::Data(encode_records(&records()).unwrap())).unwrap()
    );
    assert!(session.into_transport().close().baseline_restored);
    assert_eq!(log.borrow().releases, 1);
}
#[test]
fn changed_list_or_wrong_order_is_refused_without_native_submission() {
    for wrong_order in [false, true] {
        let (log, mut session) = setup(Journal::default());
        session.initialize().unwrap();
        if wrong_order {
            assert!(session.poll().is_err());
        } else {
            let mut changed = records();
            changed[0].assignment = Assignment::from_byte(1);
            assert!(session.publish(&changed).is_err());
        }
        assert_eq!(log.borrow().writes.len(), 2);
        assert_eq!(log.borrow().reads, 1);
        session.into_transport().close();
        assert_eq!(log.borrow().releases, 1);
    }
}
#[test]
fn bad_handshake_cannot_enable_publication_even_through_direct_transport() {
    let (log, mut s) = setup(Journal {
        bad_a2: true,
        ..Default::default()
    });
    assert!(s.initialize().is_err());
    let mut owner = s.into_transport();
    let frame = encode_frame(&Frame::Data(encode_records(&records()).unwrap())).unwrap();
    assert_eq!(owner.write(3, &frame, T), Err(TransportError::NoTransfer));
    assert_eq!(log.borrow().writes.len(), 2);
    owner.close();
}
#[test]
fn partial_and_uncertain_post_handshake_writes_stop_without_retry() {
    for at in [3, 4] {
        for short in [false, true] {
            let (log, mut s) = setup(Journal {
                fail_write: (!short).then_some(at),
                short_write: short.then_some(at),
                ..Default::default()
            });
            s.initialize().unwrap();
            if at == 3 {
                assert!(s.publish(&records()).is_err());
            } else {
                s.publish(&records()).unwrap();
                assert!(s.poll().is_err());
            }
            assert!(s.publish(&records()).is_err());
            assert_eq!(log.borrow().writes.len(), at);
            assert_eq!(log.borrow().reads, 1);
            s.into_transport().close();
        }
    }
}
#[test]
fn changed_identity_and_malformed_poll_halt_the_transport() {
    let (log, mut s) = setup(Journal::default());
    s.initialize().unwrap();
    log.borrow_mut().changed = true;
    assert!(s.publish(&records()).is_err());
    assert_eq!(log.borrow().writes.len(), 2);
    assert!(!s.into_transport().close().baseline_restored);
    for reply in [
        vec![0; 3],
        vec![0xff, 0xff, 0xff, 0xac],
        vec![10, 0, 0, 0xac, 0],
    ] {
        let (log, mut s) = setup(Journal {
            reply,
            ..Default::default()
        });
        s.initialize().unwrap();
        s.publish(&records()).unwrap();
        assert!(s.poll().is_err());
        let mut owner = s.into_transport();
        assert_eq!(
            owner.write(3, &[0, 0, 1, 0xac], T),
            Err(TransportError::NoTransfer)
        );
        assert_eq!(log.borrow().writes.len(), 4);
        owner.close();
    }
}
#[test]
fn policy_requires_exactly_one_bound_live_application_and_keeps_zero_opaque() {
    let original = inventory();
    assert_eq!(
        Publication::from_inventory(&original).unwrap().records()[0]
            .assignment
            .byte(),
        0
    );
    let mut variants = Vec::new();
    let mut x = original.clone();
    x.applications.clear();
    variants.push(x);
    let mut x = original.clone();
    x.applications[0].streams.clear();
    variants.push(x);
    let mut x = original.clone();
    x.applications[0].streams[0].state = beacn_link_protocol::planner::StreamState::Stale;
    variants.push(x);
    let mut x = original.clone();
    x.applications.push(x.applications[0].clone());
    variants.push(x);
    let mut x = original.clone();
    x.applications[0].advertised_name = vec![0xff];
    variants.push(x);
    for x in variants {
        assert!(Publication::from_inventory(&x).is_err());
    }
}
#[test]
fn no_payload_is_distinct_from_an_empty_list_and_never_implies_a_route() {
    let (_, mut s) = setup(Journal {
        reply: 0xAC000000u32.to_le_bytes().to_vec(),
        ..Default::default()
    });
    s.initialize().unwrap();
    s.publish(&records()).unwrap();
    assert_eq!(s.poll().unwrap(), PollReply::NoPayload { ignored_tail: 0 });
    assert!(s.into_transport().close().baseline_restored);
}

fn periodic(journal: Journal, cycles: usize) -> (Rc<RefCell<Journal>>, Session<Handshake<Fake>>) {
    periodic_policy(journal, cycles, PollPolicy::Strict, PollPolicy::Strict)
}
fn periodic_policy(
    journal: Journal,
    cycles: usize,
    gate_policy: PollPolicy,
    session_policy: PollPolicy,
) -> (Rc<RefCell<Journal>>, Session<Handshake<Fake>>) {
    let log = Rc::new(RefCell::new(journal));
    let owner = Handshake::prepare_publication(
        Fake {
            log: log.clone(),
            alt: 0,
        },
        Publication::bounded_with_poll_policy(&inventory(), cycles, gate_policy).unwrap(),
    )
    .unwrap();
    (
        log,
        Session::new_with_poll_policy(owner, T, session_policy).unwrap(),
    )
}
#[test]
fn marker_observation_cycles_require_fresh_source_and_exhaust_the_budget() {
    let (log, mut s) = periodic_policy(
        Journal {
            reply: 0xACFFFFFFu32.to_le_bytes().to_vec(),
            ..Default::default()
        },
        2,
        PollPolicy::ReportRejectedMarker,
        PollPolicy::ReportRejectedMarker,
    );
    s.initialize().unwrap();
    for id in [2, 3] {
        let mut next = inventory();
        next.snapshot_id = id;
        s.refresh_publication(&next).unwrap();
        s.publish(&records()).unwrap();
        assert!(matches!(
            s.poll().unwrap(),
            PollReply::RejectedMarker { .. }
        ));
    }
    let mut next = inventory();
    next.snapshot_id = 4;
    assert!(s.refresh_publication(&next).is_err());
    assert_eq!(log.borrow().writes.len(), 6);
    assert_eq!(log.borrow().reads, 3);
    assert!(s.into_transport().close().baseline_restored);
}
#[test]
fn marker_then_bound_assignment_uses_current_inventory_without_applying_routes() {
    let (log, mut s) = periodic_policy(
        Journal {
            reply: 0xACFFFFFFu32.to_le_bytes().to_vec(),
            ..Default::default()
        },
        2,
        PollPolicy::ReportRejectedMarker,
        PollPolicy::ReportRejectedMarker,
    );
    s.initialize().unwrap();
    let mut next = inventory();
    next.snapshot_id = 2;
    s.refresh_publication(&next).unwrap();
    s.publish(&records()).unwrap();
    assert!(matches!(
        s.poll().unwrap(),
        PollReply::RejectedMarker { .. }
    ));
    log.borrow_mut().reply = encode_frame(&Frame::Data(
        encode_records(&[Record {
            name: b"Linux test".to_vec(),
            assignment: Assignment::from_byte(1),
        }])
        .unwrap(),
    ))
    .unwrap();
    next.snapshot_id = 3;
    s.refresh_publication(&next).unwrap();
    s.publish(&records()).unwrap();
    let PollReply::Records {
        records: replies, ..
    } = s.poll().unwrap()
    else {
        panic!("expected records")
    };
    let plan = beacn_link_protocol::planner::plan(&next, &replies).unwrap();
    assert_eq!(plan.snapshot_id, 3);
    assert_eq!(plan.proposed_routes().count(), 1);
    assert!(s.into_transport().close().baseline_restored);
    assert_eq!(log.borrow().writes.len(), 6); // handshake and two pub/poll pairs only
}
#[test]
fn marker_policy_mismatch_and_source_change_cannot_submit_another_cycle() {
    for kind in 0..4 {
        let gate = if kind == 0 {
            PollPolicy::Strict
        } else {
            PollPolicy::ReportRejectedMarker
        };
        let controller = if kind == 1 {
            PollPolicy::Strict
        } else {
            PollPolicy::ReportRejectedMarker
        };
        let (log, mut s) = periodic_policy(
            Journal {
                reply: 0xACFFFFFFu32.to_le_bytes().to_vec(),
                ..Default::default()
            },
            2,
            gate,
            controller,
        );
        s.initialize().unwrap();
        let mut next = inventory();
        next.snapshot_id = 2;
        s.refresh_publication(&next).unwrap();
        s.publish(&records()).unwrap();
        let outcome = s.poll();
        if kind == 1 {
            assert!(outcome.is_err());
        } else {
            assert!(matches!(outcome.unwrap(), PollReply::RejectedMarker { .. }));
        }
        next.snapshot_id = if kind == 2 { 2 } else { 3 };
        if kind == 3 {
            next.applications[0].streams[0].id.0.push('x');
        }
        assert!(s.refresh_publication(&next).is_err());
        assert!(s.publish(&records()).is_err());
        assert_eq!(log.borrow().writes.len(), 4);
        s.into_transport().close();
    }
}
#[test]
fn uncertain_or_partial_native_submission_after_marker_still_halts_permanently() {
    for at in [5, 6] {
        for short in [false, true] {
            let (log, mut s) = periodic_policy(
                Journal {
                    reply: 0xACFFFFFFu32.to_le_bytes().to_vec(),
                    fail_write: (!short).then_some(at),
                    short_write: short.then_some(at),
                    ..Default::default()
                },
                3,
                PollPolicy::ReportRejectedMarker,
                PollPolicy::ReportRejectedMarker,
            );
            s.initialize().unwrap();
            let mut next = inventory();
            next.snapshot_id = 2;
            s.refresh_publication(&next).unwrap();
            s.publish(&records()).unwrap();
            assert!(matches!(
                s.poll().unwrap(),
                PollReply::RejectedMarker { .. }
            ));
            next.snapshot_id = 3;
            s.refresh_publication(&next).unwrap();
            if at == 5 {
                assert!(s.publish(&records()).is_err());
            } else {
                s.publish(&records()).unwrap();
                assert!(s.poll().is_err());
            }
            next.snapshot_id = 4;
            assert!(s.refresh_publication(&next).is_err());
            assert!(s.publish(&records()).is_err());
            assert_eq!(log.borrow().writes.len(), at);
            assert_eq!(log.borrow().reads, 2);
            assert!(s.into_transport().close().baseline_restored);
            assert_eq!(log.borrow().releases, 1);
        }
    }
}
#[test]
fn periodic_cycles_require_new_inventory_and_exhaust_exact_budget() {
    let (log, mut s) = periodic(
        Journal {
            reply: 0xAC000000u32.to_le_bytes().to_vec(),
            ..Default::default()
        },
        2,
    );
    s.initialize().unwrap();
    for id in [2, 3] {
        let mut next = inventory();
        next.snapshot_id = id;
        s.refresh_publication(&next).unwrap();
        s.publish(&records()).unwrap();
        assert!(matches!(s.poll().unwrap(), PollReply::NoPayload { .. }));
    }
    let mut next = inventory();
    next.snapshot_id = 4;
    assert!(s.refresh_publication(&next).is_err());
    assert_eq!(log.borrow().writes.len(), 6);
    assert_eq!(log.borrow().reads, 3);
    assert!(s.into_transport().close().baseline_restored);
    assert_eq!(log.borrow().releases, 1);
    for budget in [0, 41, usize::MAX] {
        assert!(Publication::bounded(&inventory(), budget).is_err());
    }
}
#[test]
fn periodic_cannot_publish_a_frozen_or_rebound_source_or_skip_refresh() {
    for kind in 0..6 {
        let (log, mut s) = periodic(Journal::default(), 2);
        s.initialize().unwrap();
        let mut next = inventory();
        next.snapshot_id = 2;
        match kind {
            0 => next.snapshot_id = 1,
            1 => next.applications[0].id.0.push('x'),
            2 => next.applications[0].streams[0].id.0.push('x'),
            3 => {
                next.applications[0].streams[0].state =
                    beacn_link_protocol::planner::StreamState::Disappeared
            }
            4 => next.applications[0].advertised_name = b"replacement".to_vec(),
            _ => {}
        }
        if kind == 5 {
            assert!(s.publish(&records()).is_err());
        } else {
            assert!(s.refresh_publication(&next).is_err());
        }
        assert!(s.publish(&records()).is_err());
        assert_eq!(log.borrow().writes.len(), 2);
        assert_eq!(log.borrow().reads, 1);
        s.into_transport().close();
    }
}
#[test]
fn periodic_refuses_refresh_mid_exchange_and_never_recovers_failed_cycle() {
    for kind in 0..4 {
        let (log, mut s) = periodic(
            Journal {
                reply: if kind == 3 {
                    0xACFFFFFFu32.to_le_bytes().to_vec()
                } else {
                    0xAC000000u32.to_le_bytes().to_vec()
                },
                short_write: (kind == 1).then_some(3),
                fail_write: (kind == 2).then_some(4),
                ..Default::default()
            },
            2,
        );
        s.initialize().unwrap();
        let mut next = inventory();
        next.snapshot_id = 2;
        s.refresh_publication(&next).unwrap();
        if kind == 0 {
            next.snapshot_id = 3;
            assert!(s.refresh_publication(&next).is_err());
        } else if kind == 1 {
            assert!(s.publish(&records()).is_err());
        } else {
            s.publish(&records()).unwrap();
            assert!(s.poll().is_err());
        }
        let before = log.borrow().writes.len();
        next.snapshot_id = 4;
        assert!(s.refresh_publication(&next).is_err());
        assert!(s.publish(&records()).is_err());
        assert_eq!(log.borrow().writes.len(), before);
        s.into_transport().close();
    }
}

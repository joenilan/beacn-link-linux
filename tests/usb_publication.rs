use beacn_link_protocol::{
    encode_frame, encode_records,
    session::{PollReply, Session, Transport, TransportError},
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
    fn write(&mut self, b: &[u8], _: u32) -> Result<usize, i32> {
        let mut l = self.log.borrow_mut();
        l.writes.push(b.to_vec());
        let n = l.writes.len();
        if l.fail_write == Some(n) {
            Err(-7)
        } else if l.short_write == Some(n) {
            Ok(b.len() - 1)
        } else {
            Ok(b.len())
        }
    }
    fn read(&mut self, b: &mut [u8], _: u32) -> Result<usize, i32> {
        let mut l = self.log.borrow_mut();
        l.reads += 1;
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

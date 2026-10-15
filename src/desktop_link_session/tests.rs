use super::*;
use crate::{
    catalog,
    native_desktop_target::test_publication,
    planner::ApplicationId,
    session::{self, ReceivedNamePolicy},
    usb_handshake::{self, Backend, Handshake, Publication},
};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
};
const ID: &str = "pw-client:11:pid:42";
#[derive(Default)]
struct Shared {
    events: Vec<&'static str>,
    frames: Vec<Vec<u8>>,
    replies: VecDeque<Vec<u8>>,
    assignment: u8,
    fail_apply: bool,
    fail_cleanup: bool,
}
struct Lease {
    shared: Rc<RefCell<Shared>>,
    epoch: Rc<Cell<u64>>,
    nonce: u64,
}
impl DesktopLease for Lease {
    fn publication(&mut self, inventory: &Inventory) -> Result<DesktopRoutePublication, String> {
        self.nonce += 1;
        self.epoch.set(self.nonce);
        Ok(test_publication(
            inventory.clone(),
            self.shared.borrow().assignment,
            &self.epoch,
            self.nonce,
        ))
    }
    fn apply_pair(&mut self, pair: u8) -> Result<(), String> {
        self.epoch.set(0);
        let mut s = self.shared.borrow_mut();
        s.events.push("route-apply");
        if s.fail_apply {
            Err("injected partial route failure".into())
        } else {
            s.assignment = pair;
            Ok(())
        }
    }
    fn stop(&mut self) -> Result<(), String> {
        self.epoch.set(0);
        let mut s = self.shared.borrow_mut();
        s.events.push("route-stop");
        if s.fail_cleanup {
            Err("injected cleanup failure".into())
        } else {
            Ok(())
        }
    }
}
struct Fake {
    shared: Rc<RefCell<Shared>>,
    baseline: usb_handshake::Observation,
    initialized: bool,
}
impl Fake {
    fn new(shared: Rc<RefCell<Shared>>) -> Self {
        Self {
            shared,
            initialized: false,
            baseline: usb_handshake::Observation {
                instance: 44,
                descriptors: include_str!("../../fixtures/usb-handshake.synthetic.hex")
                    .split_whitespace()
                    .map(|s| u8::from_str_radix(s, 16).unwrap())
                    .collect(),
                configuration: 1,
                alternate: 0,
                kernel_bound: false,
                audio_interfaces: (0..3).map(|n| (n, 0, "snd-usb-audio".into())).collect(),
            },
        }
    }
}
impl Backend for Fake {
    fn observe(&mut self) -> Result<usb_handshake::Observation, i32> {
        Ok(self.baseline.clone())
    }
    fn claim_three(&mut self) -> Result<(), i32> {
        Ok(())
    }
    fn select_one(&mut self) -> Result<(), i32> {
        self.baseline.alternate = 1;
        Ok(())
    }
    fn write(&mut self, bytes: &[u8], _: u32) -> Result<usize, i32> {
        if bytes.len() > 4 {
            self.shared.borrow_mut().frames.push(bytes.to_vec());
        }
        Ok(bytes.len())
    }
    fn read(&mut self, bytes: &mut [u8], _: u32) -> Result<usize, i32> {
        let reply = if !self.initialized {
            self.initialized = true;
            let mut v = vec![0; 512];
            v[..4].copy_from_slice(&0xA2000000u32.to_le_bytes());
            v
        } else {
            self.shared
                .borrow_mut()
                .replies
                .pop_front()
                .unwrap_or_else(|| 0xAC000000u32.to_le_bytes().to_vec())
        };
        bytes[..reply.len()].copy_from_slice(&reply);
        Ok(reply.len())
    }
    fn release_three(&mut self) -> Result<(), i32> {
        self.shared.borrow_mut().events.push("usb-release");
        self.baseline.alternate = 0;
        Ok(())
    }
    fn close(&mut self) {}
}
fn service() -> ApplicationService {
    let mut s = ApplicationService::default();
    s.update(catalog::parse(1,"catalog\t7\t9\t70697065776972652d30\napp\t11\t42\t20\tlive\t46697265666f78\t66697265666f78\n").unwrap()).unwrap();
    s.select(&ApplicationId(ID.into()), "Firefox").unwrap();
    s
}
fn advance(s: &mut ApplicationService) {
    let mut c = s.current().unwrap().clone();
    c.generation += 1;
    s.update(c).unwrap();
}
fn options(cycles: usize) -> Options {
    Options {
        cycles,
        poll_policy: session::PollPolicy::ReportRejectedMarker,
        names: ReceivedNamePolicy::WindowsCString,
        initialization_timeout: usb_handshake::MAX_TIMEOUT,
    }
}
fn open(
    shared: &Rc<RefCell<Shared>>,
    inventory: &Inventory,
    cycles: usize,
    desktop: bool,
) -> Handshake<Fake> {
    let p = if desktop {
        Publication::bounded_desktop(inventory, cycles, session::PollPolicy::ReportRejectedMarker)
            .unwrap()
    } else {
        Publication::bounded(inventory, cycles)
            .unwrap()
            .with_verified_routes()
    };
    Handshake::prepare_publication(Fake::new(shared.clone()), p)
        .unwrap_or_else(|_| panic!("fake prepare"))
}
fn start(
    shared: Rc<RefCell<Shared>>,
    s: &ApplicationService,
    cycles: usize,
) -> DesktopLinkSession<Lease, Handshake<Fake>> {
    let mut owner = DesktopLinkSession::default();
    let a = shared.clone();
    let b = shared;
    owner
        .start(
            s,
            options(cycles),
            || {
                Ok(Lease {
                    shared: a,
                    epoch: Rc::new(Cell::new(0)),
                    nonce: 0,
                })
            },
            |_, i, _| Ok(open(&b, i, cycles, true)),
        )
        .unwrap();
    owner
}
fn reply(assignment: u8) -> Vec<u8> {
    // Inbound C-string tail is authored explicitly; outgoing encoding stays strict.
    let mut payload = vec![12, assignment];
    payload.extend_from_slice(b"Firefox\0TAIL");
    payload.push(0);
    crate::encode_frame(&crate::Frame::Data(payload)).unwrap()
}
#[test]
fn original_output_then_applied_publication_and_opaque_zero_preserve_route() {
    let shared = Rc::new(RefCell::new(Shared::default()));
    shared
        .borrow_mut()
        .replies
        .extend([reply(1), 0xAC000000u32.to_le_bytes().to_vec(), reply(0)]);
    let mut s = service();
    let mut owner = start(shared.clone(), &s, 4);
    for _ in 0..3 {
        advance(&mut s);
        owner.tick(&s).unwrap();
    }
    assert_eq!(owner.status.assignment, Some(1));
    let values = shared
        .borrow()
        .frames
        .iter()
        .map(|f| crate::decode_records(&f[4..]).unwrap()[0].assignment.byte())
        .collect::<Vec<_>>();
    assert_eq!(values, [0, 1, 1]);
    assert_eq!(
        shared
            .borrow()
            .events
            .iter()
            .filter(|e| **e == "route-apply")
            .count(),
        1
    );
    owner.stop().unwrap();
    owner.stop().unwrap();
    assert_eq!(&shared.borrow().events[1..], ["route-stop", "usb-release"]);
}
#[test]
fn stale_selection_edit_and_departure_stop_before_publication() {
    for mode in 0..3 {
        let shared = Rc::new(RefCell::new(Shared::default()));
        let mut s = service();
        let mut owner = start(shared.clone(), &s, 2);
        if mode == 1 {
            s.select(&ApplicationId(ID.into()), "Renamed").unwrap();
        }
        if mode == 2 {
            let mut c = s.current().unwrap().clone();
            c.generation += 1;
            c.applications.clear();
            s.update(c).unwrap();
        }
        assert!(owner.tick(&s).is_err());
        assert!(shared.borrow().frames.is_empty());
        assert_eq!(shared.borrow().events, ["route-stop", "usb-release"]);
    }
}
#[test]
fn partial_apply_failure_and_cleanup_failure_latch_without_forward_publication() {
    for bad_cleanup in [false, true] {
        let shared = Rc::new(RefCell::new(Shared {
            fail_apply: true,
            fail_cleanup: bad_cleanup,
            ..Shared::default()
        }));
        shared.borrow_mut().replies.push_back(reply(1));
        let mut s = service();
        let mut owner = start(shared.clone(), &s, 2);
        advance(&mut s);
        assert!(owner.tick(&s).is_err());
        assert_eq!(shared.borrow().frames.len(), 1);
        assert_eq!(
            owner.status.phase,
            if bad_cleanup {
                Phase::NeedsReview
            } else {
                Phase::Stopped
            }
        );
        assert_eq!(
            shared.borrow().events,
            ["route-apply", "route-stop", "usb-release"]
        );
        if bad_cleanup {
            assert!(owner
                .start(
                    &s,
                    options(2),
                    || panic!("blocked factory"),
                    |_, _, _| panic!("blocked transport")
                )
                .is_err());
        }
        let _ = owner.stop();
        assert_eq!(shared.borrow().events.len(), 3);
    }
}
#[test]
fn all_four_pairs_apply_switch_and_publish_only_verified_assignment() {
    let shared = Rc::new(RefCell::new(Shared::default()));
    shared
        .borrow_mut()
        .replies
        .extend([reply(1), reply(2), reply(3), reply(4)]);
    let mut s = service();
    let mut owner = start(shared.clone(), &s, 5);
    for pair in 1..=4 {
        advance(&mut s);
        owner.tick(&s).unwrap();
        assert_eq!(owner.status.assignment, Some(pair));
    }
    let published = shared
        .borrow()
        .frames
        .iter()
        .map(|f| crate::decode_records(&f[4..]).unwrap()[0].assignment.byte())
        .collect::<Vec<_>>();
    assert_eq!(published, [0, 1, 2, 3]);
    owner.stop().unwrap();
    assert_eq!(
        shared.borrow().events,
        [
            "route-apply",
            "route-apply",
            "route-apply",
            "route-apply",
            "route-stop",
            "usb-release"
        ]
    );
}
#[test]
fn local_assignment_requires_live_owner_and_verifies_each_pair_before_display() {
    let shared = Rc::new(RefCell::new(Shared::default()));
    let mut service = service();
    let mut owner = start(shared.clone(), &service, 6);
    assert!(owner.assign(&service, 0).is_err());
    assert!(shared.borrow().events.is_empty());
    for pair in 1..=4 {
        owner.assign(&service, pair).unwrap();
        assert_eq!(owner.status().assignment, Some(pair));
        advance(&mut service);
        owner.tick(&service).unwrap();
        assert_eq!(owner.status().assignment, Some(pair));
    }
    owner.stop().unwrap();
    assert!(owner.assign(&service, 1).is_err());
    assert_eq!(
        shared
            .borrow()
            .frames
            .iter()
            .map(|f| crate::decode_records(&f[4..]).unwrap()[0].assignment.byte())
            .collect::<Vec<_>>(),
        [1, 2, 3, 4]
    );
}
#[test]
fn failed_local_assignment_does_not_publish_or_display_success() {
    let shared = Rc::new(RefCell::new(Shared {
        fail_apply: true,
        ..Shared::default()
    }));
    let service = service();
    let mut owner = start(shared.clone(), &service, 2);
    assert!(owner.assign(&service, 1).is_err());
    assert_eq!(owner.status().assignment, None);
    assert_eq!(owner.status().phase, Phase::Stopped);
    assert!(shared.borrow().frames.is_empty());
    assert_eq!(
        shared.borrow().events,
        ["route-apply", "route-stop", "usb-release"]
    );
}
#[test]
fn revoked_or_expired_seal_cannot_refresh_or_submit() {
    for mode in 0..3 {
        let shared = Rc::new(RefCell::new(Shared::default()));
        let mut s = service();
        let transport = open(&shared, s.inventory().unwrap(), 2, true);
        let mut control = Session::new(transport, usb_handshake::MAX_TIMEOUT).unwrap();
        control.initialize().unwrap();
        advance(&mut s);
        let epoch = Rc::new(Cell::new(1));
        let proof = test_publication(s.inventory().unwrap().clone(), 0, &epoch, 1);
        if mode == 0 {
            epoch.set(0);
            assert!(control.refresh_desktop_publication(&proof).is_err());
        } else if mode == 1 {
            drop(epoch);
            assert!(control.refresh_desktop_publication(&proof).is_err());
        } else {
            control.refresh_desktop_publication(&proof).unwrap();
            epoch.set(0);
            assert!(control.publish(proof.records()).is_err());
        }
        assert!(shared.borrow().frames.is_empty());
    }
}
#[test]
fn desktop_policy_rejects_raw_inventory_and_private_policy_rejects_desktop_seal() {
    for desktop in [false, true] {
        let shared = Rc::new(RefCell::new(Shared::default()));
        let mut s = service();
        let t = open(&shared, s.inventory().unwrap(), 2, desktop);
        let mut control = Session::new(t, usb_handshake::MAX_TIMEOUT).unwrap();
        control.initialize().unwrap();
        advance(&mut s);
        let epoch = Rc::new(Cell::new(1));
        let proof = test_publication(s.inventory().unwrap().clone(), 0, &epoch, 1);
        if desktop {
            assert!(control.refresh_publication(s.inventory().unwrap()).is_err());
        } else {
            assert!(control.refresh_desktop_publication(&proof).is_err());
        }
        assert!(shared.borrow().frames.is_empty());
    }
}
#[test]
fn budget_releases_and_invalid_budget_never_opens_a_factory() {
    let shared = Rc::new(RefCell::new(Shared::default()));
    let mut s = service();
    let mut owner = start(shared.clone(), &s, 1);
    advance(&mut s);
    owner.tick(&s).unwrap();
    assert_eq!(owner.status.phase, Phase::Stopped);
    assert_eq!(owner.status.completed_cycles, 1);
    assert_eq!(shared.borrow().events, ["route-stop", "usb-release"]);
    for cycles in [0, 41] {
        assert!(owner
            .start(
                &s,
                options(cycles),
                || panic!("invalid factory"),
                |_, _, _| panic!("invalid transport")
            )
            .is_err());
    }
}

#[test]
fn route_factory_failure_is_visible_and_never_opens_control() {
    let mut owner: DesktopLinkSession<Lease, Handshake<Fake>> = DesktopLinkSession::default();
    assert!(owner
        .start(
            &service(),
            options(2),
            || Err("desktop disappeared".into()),
            |_, _, _| panic!("no control after factory failure")
        )
        .is_err());
    assert_eq!(owner.status().phase, Phase::Stopped);
    assert_eq!(owner.status().error.as_deref(), Some("desktop disappeared"));
    owner.stop().unwrap();
}
#[test]
fn another_lease_and_replayed_inventory_cannot_continue_a_desktop_epoch() {
    for replace_lease in [false, true] {
        let shared = Rc::new(RefCell::new(Shared::default()));
        let mut s = service();
        let t = open(&shared, s.inventory().unwrap(), 3, true);
        let mut control = Session::new(t, usb_handshake::MAX_TIMEOUT).unwrap();
        control.initialize().unwrap();
        advance(&mut s);
        let epoch = Rc::new(Cell::new(1));
        let proof = test_publication(s.inventory().unwrap().clone(), 0, &epoch, 1);
        control.refresh_desktop_publication(&proof).unwrap();
        control.publish(proof.records()).unwrap();
        control.poll().unwrap();
        let next = if replace_lease {
            advance(&mut s);
            Rc::new(Cell::new(2))
        } else {
            epoch.clone()
        };
        next.set(2);
        let proof = test_publication(s.inventory().unwrap().clone(), 0, &next, 2);
        assert!(control.refresh_desktop_publication(&proof).is_err());
        assert_eq!(shared.borrow().frames.len(), 1);
    }
}

#[test]
fn managed_desktop_outlives_bounded_budget_and_releases_on_selection_loss() {
    let shared = Rc::new(RefCell::new(Shared::default()));
    shared.borrow_mut().replies.push_back(reply(1));
    let mut service = service();
    let mut owner = DesktopLinkSession::default();
    let a = shared.clone();
    let b = shared.clone();
    owner
        .start_managed(
            &service,
            ManagedOptions {
                poll_policy: session::PollPolicy::ReportRejectedMarker,
                names: ReceivedNamePolicy::WindowsCString,
                initialization_timeout: usb_handshake::MAX_TIMEOUT,
            },
            || {
                Ok(Lease {
                    shared: a,
                    epoch: Rc::new(Cell::new(0)),
                    nonce: 0,
                })
            },
            |_, inventory, options| {
                let p = Publication::managed_desktop(inventory, options.poll_policy).unwrap();
                Ok(Handshake::prepare_publication(Fake::new(b), p)
                    .unwrap_or_else(|_| panic!("managed fake prepare")))
            },
        )
        .unwrap();
    for _ in 0..42 {
        advance(&mut service);
        owner.tick(&service).unwrap();
    }
    assert_eq!(owner.status().phase, Phase::Running);
    assert_eq!(owner.status().cycle_limit, None);
    assert_eq!(owner.status().completed_cycles, 42);
    assert_eq!(owner.status().assignment, Some(1));
    assert_eq!(shared.borrow().frames.len(), 42);
    service
        .select(&ApplicationId(ID.into()), "Different name")
        .unwrap();
    assert!(owner.check(&service).is_err());
    assert_eq!(owner.status().phase, Phase::Stopped);
    assert_eq!(&shared.borrow().events[1..], ["route-stop", "usb-release"]);
    owner.stop().unwrap();
    assert_eq!(shared.borrow().events.len(), 3);
}

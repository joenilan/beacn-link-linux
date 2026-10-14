use beacn_link_protocol::{
    session::{self, Session, Transport, TransportError},
    usb_handshake::*,
};
use std::{cell::RefCell, rc::Rc, time::Duration};
const TIMEOUT: Duration = Duration::from_millis(500);
fn baseline() -> Observation {
    Observation {
        instance: 44,
        descriptors: include_str!("../fixtures/usb-handshake.synthetic.hex")
            .split_whitespace()
            .map(|s| u8::from_str_radix(s, 16).unwrap())
            .collect(),
        configuration: 1,
        alternate: 0,
        kernel_bound: false,
        audio_interfaces: (0..3).map(|n| (n, 0, "snd-usb-audio".into())).collect(),
    }
}
#[derive(Default)]
struct State {
    calls: Vec<String>,
    observations: usize,
    observe_failure: Option<usize>,
    change_at: Option<(usize, &'static str)>,
    claim_error: bool,
    alt_error: bool,
    zero_error: bool,
    clear_error: bool,
    release_error: bool,
    write_error: bool,
    read_error: bool,
    write_length: Option<usize>,
    read_length: Option<usize>,
    wrong_reply: bool,
}
struct Fake {
    shared: Rc<RefCell<State>>,
    current: Observation,
}
impl Backend for Fake {
    fn observe(&mut self) -> Result<Observation, i32> {
        let mut state = self.shared.borrow_mut();
        state.observations += 1;
        let n = state.observations;
        if state.observe_failure == Some(n) {
            return Err(-4);
        }
        if let Some((at, field)) = state.change_at {
            if at == n {
                match field {
                    "instance" => self.current.instance += 1,
                    "cache" => self.current.descriptors[12] = 2,
                    "configuration" => self.current.configuration = 2,
                    "alternate" => self.current.alternate = 2,
                    "kernel" => self.current.kernel_bound = true,
                    "audio" => self.current.audio_interfaces[1].1 = 1,
                    "driver" => self.current.audio_interfaces[2].2 = "replacement".into(),
                    _ => panic!(),
                }
            }
        }
        Ok(self.current.clone())
    }
    fn claim_three(&mut self) -> Result<(), i32> {
        let mut state = self.shared.borrow_mut();
        state.calls.push("claim:3".into());
        if state.claim_error {
            Err(-6)
        } else {
            Ok(())
        }
    }
    fn select_one(&mut self) -> Result<(), i32> {
        let mut state = self.shared.borrow_mut();
        state.calls.push("alternate:3:1".into());
        self.current.alternate = 1;
        if state.alt_error {
            Err(-7)
        } else {
            Ok(())
        } // May have changed despite missing completion.
    }
    fn select_zero(&mut self) -> Result<(), i32> {
        let mut state = self.shared.borrow_mut();
        state.calls.push("alternate:3:0".into());
        self.current.alternate = 0;
        if state.zero_error {
            Err(-7)
        } else {
            Ok(())
        }
    }
    fn clear_input_halt(&mut self) -> Result<(), i32> {
        let mut state = self.shared.borrow_mut();
        state.calls.push("clear-input:83".into());
        if state.clear_error {
            Err(-7)
        } else {
            Ok(())
        }
    }
    fn write(&mut self, bytes: &[u8], ms: u32) -> Result<usize, i32> {
        let mut state = self.shared.borrow_mut();
        state.calls.push(format!(
            "write:{:08X}:{ms}",
            u32::from_le_bytes(bytes.try_into().unwrap())
        ));
        if state.write_error {
            Err(-7)
        } else {
            Ok(state.write_length.unwrap_or(bytes.len()))
        }
    }
    fn read(&mut self, bytes: &mut [u8], ms: u32) -> Result<usize, i32> {
        let mut state = self.shared.borrow_mut();
        state.calls.push(format!("read:{}:{ms}", bytes.len()));
        let word = if state.wrong_reply {
            0xA1000000u32
        } else {
            0xA2000000u32
        };
        bytes[..4].copy_from_slice(&word.to_le_bytes());
        if state.read_error {
            Err(-7)
        } else {
            Ok(state.read_length.unwrap_or(bytes.len()))
        }
    }
    fn release_three(&mut self) -> Result<(), i32> {
        let mut state = self.shared.borrow_mut();
        state.calls.push("release:3:restore:0".into());
        if state.release_error {
            Err(-4)
        } else {
            self.current.alternate = 0;
            Ok(())
        }
    }
    fn close(&mut self) {
        self.shared.borrow_mut().calls.push("close".into());
    }
}
fn setup(state: State) -> (Rc<RefCell<State>>, Result<Handshake<Fake>, SetupFailure>) {
    let shared = Rc::new(RefCell::new(state));
    let result = Handshake::prepare(Fake {
        shared: shared.clone(),
        current: baseline(),
    });
    (shared, result)
}
fn observed_setup(state: State) -> (Rc<RefCell<State>>, Result<Handshake<Fake>, SetupFailure>) {
    let shared = Rc::new(RefCell::new(state));
    let result = Handshake::prepare_with_startup(
        Fake {
            shared: shared.clone(),
            current: baseline(),
        },
        beacn_link_protocol::usb_handshake::Startup::ObservedWindows,
    );
    (shared, result)
}
#[test]
fn explicit_observed_startup_prepares_only_vendor_input_before_handshake() {
    let (shared, transport) = observed_setup(State::default());
    let mut s = Session::new(transport.unwrap(), TIMEOUT).unwrap();
    s.initialize().unwrap();
    assert!(s.into_transport().close().baseline_restored);
    assert_eq!(
        shared.borrow().calls,
        [
            "claim:3",
            "alternate:3:0",
            "alternate:3:1",
            "clear-input:83",
            "write:A0000000:500",
            "write:A1000000:500",
            "read:512:500",
            "release:3:restore:0",
            "close",
        ]
    );
}
#[test]
fn observed_startup_failures_never_send_bulk_or_repeat_preparation() {
    for field in ["zero", "one", "clear"] {
        let state = State {
            zero_error: field == "zero",
            alt_error: field == "one",
            clear_error: field == "clear",
            ..Default::default()
        };
        let (shared, result) = observed_setup(state);
        let failure = result.err().unwrap();
        assert!(failure.cleanup.baseline_restored);
        let calls = &shared.borrow().calls;
        assert_eq!(
            calls
                .iter()
                .filter(|s| s.as_str() == "release:3:restore:0")
                .count(),
            1
        );
        assert_eq!(
            calls
                .iter()
                .filter(|s| s.as_str() == "alternate:3:0")
                .count(),
            1
        );
        assert_eq!(
            calls
                .iter()
                .filter(|s| s.as_str() == "clear-input:83")
                .count(),
            usize::from(field == "clear")
        );
        assert!(!calls
            .iter()
            .any(|s| s.starts_with("write:") || s.starts_with("read:")));
    }
}
#[test]
fn fresh_identity_is_required_at_each_observed_startup_boundary() {
    for at in 2..=6 {
        let (shared, result) = observed_setup(State {
            change_at: Some((at, "instance")),
            ..Default::default()
        });
        let failure = result.err().unwrap();
        assert!(!failure.cleanup.baseline_restored);
        let calls = &shared.borrow().calls;
        assert!(!calls
            .iter()
            .any(|s| s.starts_with("write:") || s.starts_with("read:")));
        assert_eq!(
            calls
                .iter()
                .filter(|s| s.as_str() == "release:3:restore:0")
                .count(),
            1
        );
    }
}
#[test]
fn full_session_uses_exact_handshake_and_restores_only_its_claim() {
    let (shared, transport) = setup(State::default());
    let mut session = Session::new(transport.unwrap(), TIMEOUT).unwrap();
    assert!(session.initialize().is_ok());
    let cleanup = session.into_transport().close();
    assert_eq!(
        cleanup,
        Cleanup {
            release: Some(Ok(())),
            baseline_restored: true
        }
    );
    assert_eq!(
        shared.borrow().calls,
        vec![
            "claim:3",
            "alternate:3:1",
            "write:A0000000:500",
            "write:A1000000:500",
            "read:512:500",
            "release:3:restore:0",
            "close"
        ]
    );
}
#[test]
fn handshake_only_diagnostic_read_does_not_authorize_publication_or_retry() {
    let shared = Rc::new(RefCell::new(State::default()));
    let transport = Handshake::prepare_initialization_diagnostic(
        Fake {
            shared: shared.clone(),
            current: baseline(),
        },
        Startup::ObservedWindows,
    )
    .unwrap();
    let mut session = Session::new(transport, TIMEOUT).unwrap();
    session
        .initialize_with_read_timeout(DIAGNOSTIC_INITIALIZATION_TIMEOUT)
        .unwrap();
    let mut transport = session.into_transport();
    // Valid one-record data frame; a successful handshake still cannot send it.
    assert_eq!(
        transport.write(3, &[7, 0, 0, 172, 4, 1, 103, 97, 109, 101, 0], TIMEOUT),
        Err(TransportError::NoTransfer)
    );
    assert_eq!(
        transport.read(0x83, &mut [0; 512], TIMEOUT),
        Err(TransportError::NoTransfer)
    );
    assert!(transport.close().baseline_restored);
    let calls = &shared.borrow().calls;
    assert_eq!(calls.iter().filter(|c| c.starts_with("read:")).count(), 1);
    assert!(calls.iter().any(|c| c == "read:512:5000"));
    assert_eq!(calls.iter().filter(|c| c.starts_with("write:")).count(), 2);
    assert!(calls
        .iter()
        .filter(|c| c.starts_with("write:"))
        .all(|c| c.ends_with(":500")));
}
#[test]
fn handshake_default_rejects_diagnostic_deadline_and_diagnostic_errors_never_retry() {
    let (shared, transport) = setup(State::default());
    let mut session = Session::new(transport.unwrap(), TIMEOUT).unwrap();
    assert!(session
        .initialize_with_read_timeout(DIAGNOSTIC_INITIALIZATION_TIMEOUT)
        .is_err());
    assert!(session.into_transport().close().baseline_restored);
    assert!(!shared.borrow().calls.iter().any(|c| c.starts_with("read:")));
    for state in [
        State {
            read_error: true,
            ..Default::default()
        },
        State {
            wrong_reply: true,
            ..Default::default()
        },
        State {
            read_length: Some(0),
            ..Default::default()
        },
    ] {
        let shared = Rc::new(RefCell::new(state));
        let transport = Handshake::prepare_initialization_diagnostic(
            Fake {
                shared: shared.clone(),
                current: baseline(),
            },
            Startup::ObservedWindows,
        )
        .unwrap();
        let mut session = Session::new(transport, TIMEOUT).unwrap();
        assert!(session
            .initialize_with_read_timeout(DIAGNOSTIC_INITIALIZATION_TIMEOUT)
            .is_err());
        assert!(session.initialize().is_err());
        assert!(session.into_transport().close().baseline_restored);
        assert_eq!(
            shared
                .borrow()
                .calls
                .iter()
                .filter(|c| c.starts_with("read:"))
                .count(),
            1
        );
    }
}
#[test]
fn completion_does_not_authorize_publication_poll_or_a_second_handshake() {
    let (shared, transport) = setup(State::default());
    let mut session = Session::new(transport.unwrap(), TIMEOUT).unwrap();
    session.initialize().unwrap();
    assert!(session.publish(&[]).is_err());
    assert!(session.poll().is_err());
    assert_eq!(session.state(), session::State::Halted);
    assert!(session.into_transport().close().baseline_restored);
    assert_eq!(
        shared
            .borrow()
            .calls
            .iter()
            .filter(|s| s.starts_with("write"))
            .count(),
        2
    );
    let (shared, t) = setup(State::default());
    let mut t = t.unwrap();
    assert_eq!(
        t.write(3, &0xAC010000u32.to_le_bytes(), TIMEOUT),
        Err(TransportError::NoTransfer)
    );
    assert!(t.close().baseline_restored);
    assert!(!shared.borrow().calls.iter().any(|s| s.starts_with("write")));
}
#[test]
fn profile_or_existing_ownership_refusal_closes_without_claim_or_alt_change() {
    for field in [
        "bound",
        "active",
        "configuration",
        "revision",
        "audio_active",
        "missing_audio",
    ] {
        let shared = Rc::new(RefCell::new(State::default()));
        let mut current = baseline();
        match field {
            "bound" => current.kernel_bound = true,
            "active" => current.alternate = 1,
            "configuration" => current.configuration = 2,
            "revision" => current.descriptors[12] = 2,
            "audio_active" => current.audio_interfaces[0].1 = 1,
            "missing_audio" => current.audio_interfaces.pop().map(|_| ()).unwrap(),
            _ => panic!(),
        }
        let failure = match Handshake::prepare(Fake {
            shared: shared.clone(),
            current,
        }) {
            Err(e) => e,
            Ok(_) => panic!("{field}"),
        };
        assert_eq!(failure.error, Error::Profile);
        assert_eq!(shared.borrow().calls, vec!["close"]);
    }
}
#[test]
fn setup_faults_release_only_acknowledged_claim_and_report_cleanup() {
    for state in [
        State {
            claim_error: true,
            ..Default::default()
        },
        State {
            alt_error: true,
            ..Default::default()
        },
        State {
            observe_failure: Some(2),
            ..Default::default()
        },
        State {
            observe_failure: Some(3),
            ..Default::default()
        },
    ] {
        let claimed = !state.claim_error;
        let (shared, result) = setup(state);
        let failure = match result {
            Err(e) => e,
            Ok(_) => panic!(),
        };
        assert_eq!(
            failure.cleanup.release,
            if claimed { Some(Ok(())) } else { None }
        );
        assert!(failure.cleanup.baseline_restored);
        assert_eq!(
            shared
                .borrow()
                .calls
                .iter()
                .filter(|s| s.starts_with("release"))
                .count(),
            usize::from(claimed)
        );
        assert_eq!(shared.borrow().calls.last().unwrap(), "close");
    }
}
#[test]
fn every_identity_profile_and_audio_change_stops_before_bulk_submission() {
    for field in [
        "instance",
        "cache",
        "configuration",
        "alternate",
        "kernel",
        "audio",
        "driver",
    ] {
        for at in [2, 3, 4] {
            let (shared, result) = setup(State {
                change_at: Some((at, field)),
                ..Default::default()
            });
            if at < 4 {
                assert!(result.is_err());
            } else {
                let mut session = Session::new(result.unwrap(), TIMEOUT).unwrap();
                assert!(session.initialize().is_err());
                session.into_transport().close();
            }
            let state = shared.borrow();
            assert!(
                !state.calls.iter().any(|s| s.starts_with("write")),
                "{field} at {at}"
            );
            assert_eq!(
                state
                    .calls
                    .iter()
                    .filter(|s| s.starts_with("release"))
                    .count(),
                1
            );
        }
    }
}
#[test]
fn unknown_partial_and_bad_reply_outcomes_never_retry_but_always_cleanup() {
    for state in [
        State {
            write_error: true,
            ..Default::default()
        },
        State {
            read_error: true,
            ..Default::default()
        },
        State {
            write_length: Some(3),
            ..Default::default()
        },
        State {
            read_length: Some(511),
            ..Default::default()
        },
        State {
            wrong_reply: true,
            ..Default::default()
        },
    ] {
        let (shared, result) = setup(state);
        let mut session = Session::new(result.unwrap(), TIMEOUT).unwrap();
        assert!(session.initialize().is_err());
        assert!(session.initialize().is_err());
        assert!(session.poll().is_err());
        assert!(session.into_transport().close().baseline_restored);
        let state = shared.borrow();
        assert!(
            state
                .calls
                .iter()
                .filter(|s| s.starts_with("write"))
                .count()
                <= 2
        );
        assert!(state.calls.iter().filter(|s| s.starts_with("read")).count() <= 1);
        assert_eq!(
            state
                .calls
                .iter()
                .filter(|s| s.starts_with("release"))
                .count(),
            1
        );
    }
}
#[test]
fn cleanup_failure_is_reported_and_drop_does_not_repeat_a_release() {
    let (shared, result) = setup(State {
        release_error: true,
        ..Default::default()
    });
    let cleanup = result.unwrap().close();
    assert_eq!(
        cleanup,
        Cleanup {
            release: Some(Err(-4)),
            baseline_restored: false
        }
    );
    assert_eq!(
        shared
            .borrow()
            .calls
            .iter()
            .filter(|s| s.starts_with("release"))
            .count(),
        1
    );
    assert_eq!(
        shared
            .borrow()
            .calls
            .iter()
            .filter(|s| s == &&"close".to_string())
            .count(),
        1
    );
    let (shared, result) = setup(State::default());
    drop(result.unwrap());
    assert_eq!(
        shared.borrow().calls,
        vec!["claim:3", "alternate:3:1", "release:3:restore:0", "close"]
    );
}
#[test]
fn finite_timeout_and_wrong_direction_length_are_refused_before_io() {
    assert_eq!(timeout_ms(Duration::ZERO), None);
    assert_eq!(timeout_ms(Duration::from_nanos(1)), Some(1));
    assert_eq!(timeout_ms(Duration::from_millis(500)), Some(500));
    assert_eq!(timeout_ms(Duration::from_millis(501)), None);
    for (pipe, bytes, timeout) in [
        (0x83, 0xA0000000u32.to_le_bytes(), TIMEOUT),
        (3, 0xA0000000u32.to_le_bytes(), Duration::ZERO),
        (3, 0xA0000000u32.to_le_bytes(), Duration::from_secs(1)),
    ] {
        let (shared, result) = setup(State::default());
        let mut t = result.unwrap();
        assert_eq!(
            t.write(pipe, &bytes, timeout),
            Err(TransportError::NoTransfer)
        );
        t.close();
        assert!(!shared.borrow().calls.iter().any(|s| s.starts_with("write")));
    }
    let (shared, result) = setup(State::default());
    let mut t = result.unwrap();
    assert_eq!(
        t.read(0x83, &mut [0; 511], TIMEOUT),
        Err(TransportError::NoTransfer)
    );
    t.close();
    assert!(!shared.borrow().calls.iter().any(|s| s.starts_with("read")));
}

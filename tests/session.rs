use beacn_link_protocol::{
    planner::*,
    session::{
        decode_poll_reply, decode_poll_reply_with_policy, PollPolicy, PollReply, Session, State,
        Step, Transport, TransportError, IN_PIPE, OUT_PIPE,
    },
    *,
};
use std::{collections::VecDeque, time::Duration};

const TIMEOUT: Duration = Duration::from_millis(200);
enum Exchange {
    Write(Vec<u8>, Result<usize, TransportError>),
    Read(Vec<u8>, Result<usize, TransportError>),
}
struct Script {
    exchanges: VecDeque<Exchange>,
    calls: usize,
    read_timeouts: VecDeque<Duration>,
}
impl Transport for Script {
    fn write(
        &mut self,
        pipe: u8,
        bytes: &[u8],
        timeout: Duration,
    ) -> Result<usize, TransportError> {
        assert_eq!((pipe, timeout), (OUT_PIPE, TIMEOUT));
        self.calls += 1;
        let Some(Exchange::Write(expected, result)) = self.exchanges.pop_front() else {
            panic!("unexpected write");
        };
        assert_eq!(bytes, expected);
        result
    }
    fn read(
        &mut self,
        pipe: u8,
        bytes: &mut [u8],
        timeout: Duration,
    ) -> Result<usize, TransportError> {
        let expected_timeout = self.read_timeouts.pop_front().unwrap_or(TIMEOUT);
        assert_eq!(
            (pipe, timeout, bytes.len()),
            (IN_PIPE, expected_timeout, 512)
        );
        self.calls += 1;
        let Some(Exchange::Read(reply, result)) = self.exchanges.pop_front() else {
            panic!("unexpected read");
        };
        bytes[..reply.len()].copy_from_slice(&reply);
        result
    }
}
fn write(bytes: Vec<u8>) -> Exchange {
    let length = bytes.len();
    Exchange::Write(bytes, Ok(length))
}
fn read(bytes: Vec<u8>) -> Exchange {
    let length = bytes.len();
    Exchange::Read(bytes, Ok(length))
}
fn word(value: u32) -> Vec<u8> {
    value.to_le_bytes().to_vec()
}
fn a2() -> Vec<u8> {
    let mut bytes = word(0xA2000000);
    bytes.extend([0x5A; 508]); // Synthetic metadata, never a real device identity.
    bytes
}
fn startup() -> Vec<Exchange> {
    vec![write(word(0xA0000000)), write(word(0xA1000000)), read(a2())]
}
fn session(exchanges: Vec<Exchange>) -> Session<Script> {
    Session::new(
        Script {
            exchanges: exchanges.into(),
            calls: 0,
            read_timeouts: VecDeque::new(),
        },
        TIMEOUT,
    )
    .unwrap()
}
fn record(assignment: u8, name: &[u8]) -> Record {
    Record {
        assignment: Assignment::from_byte(assignment),
        name: name.to_vec(),
    }
}
fn ready(extra: Vec<Exchange>) -> Session<Script> {
    let mut exchanges = startup();
    exchanges.extend(extra);
    let mut session = session(exchanges);
    session.initialize().unwrap();
    session
}
fn ready_policy(extra: Vec<Exchange>, policy: PollPolicy) -> Session<Script> {
    let mut exchanges = startup();
    exchanges.extend(extra);
    let mut session = Session::new_with_poll_policy(
        Script {
            exchanges: exchanges.into(),
            calls: 0,
            read_timeouts: VecDeque::new(),
        },
        TIMEOUT,
        policy,
    )
    .unwrap();
    session.initialize().unwrap();
    session
}
#[test]
fn rejected_marker_observation_is_exact_bounded_and_distinct_from_no_payload() {
    for length in [4, 5, 512] {
        let mut bytes = word(0xACFFFFFF);
        bytes.resize(length, 0x5A);
        assert!(decode_poll_reply(&bytes).is_err());
        assert_eq!(
            decode_poll_reply_with_policy(&bytes, PollPolicy::ReportRejectedMarker).unwrap(),
            PollReply::RejectedMarker {
                ignored_tail: length - 4
            }
        );
    }
    for length in [0, 1, 2, 3, 513] {
        let mut bytes = word(0xACFFFFFF);
        bytes.resize(length, 0);
        assert!(decode_poll_reply_with_policy(&bytes, PollPolicy::ReportRejectedMarker).is_err());
    }
    for value in [0xABFFFFFF, 0xACFFFFFE, 0xAC01FFFF, 0xAC010000] {
        assert!(
            decode_poll_reply_with_policy(&word(value), PollPolicy::ReportRejectedMarker).is_err()
        );
    }
    assert_eq!(
        decode_poll_reply_with_policy(&word(0xAC000000), PollPolicy::ReportRejectedMarker).unwrap(),
        PollReply::NoPayload { ignored_tail: 0 }
    );
}
#[test]
fn diagnostic_initialization_deadline_changes_only_the_first_read() {
    let mut exchanges = startup();
    exchanges.extend([write(word(0xAC010000)), read(word(0xAC000000))]);
    let mut s = Session::new(
        Script {
            exchanges: exchanges.into(),
            calls: 0,
            read_timeouts: [Duration::from_secs(5)].into(),
        },
        TIMEOUT,
    )
    .unwrap();
    s.initialize_with_read_timeout(Duration::from_secs(5))
        .unwrap();
    assert!(matches!(s.poll().unwrap(), PollReply::NoPayload { .. }));
    assert_eq!(s.into_transport().calls, 5);
}
#[test]
fn diagnostic_initialization_zero_or_failed_read_never_retries() {
    let mut exchanges = startup();
    exchanges.pop();
    exchanges.push(Exchange::Read(vec![], Err(TransportError::Unknown)));
    let mut s = Session::new(
        Script {
            exchanges: exchanges.into(),
            calls: 0,
            read_timeouts: [Duration::from_secs(5)].into(),
        },
        TIMEOUT,
    )
    .unwrap();
    assert_eq!(
        s.initialize_with_read_timeout(Duration::ZERO),
        Err(session::Error::ZeroTimeout)
    );
    assert_eq!(s.state(), State::New);
    assert!(s
        .initialize_with_read_timeout(Duration::from_secs(5))
        .is_err());
    assert_eq!(s.state(), State::Halted);
    assert!(s
        .initialize_with_read_timeout(Duration::from_secs(5))
        .is_err());
    assert_eq!(s.into_transport().calls, 3);
}
#[test]
fn explicit_observation_accepts_next_call_after_marker_without_resubmitting_it() {
    let records = vec![record(1, b"Linux test")];
    let response = encode_frame(&Frame::Data(encode_records(&records).unwrap())).unwrap();
    let mut s = ready_policy(
        vec![
            write(word(0xAC010000)),
            read(word(0xACFFFFFF)),
            write(word(0xAC010000)),
            read(response),
        ],
        PollPolicy::ReportRejectedMarker,
    );
    assert!(matches!(
        s.poll().unwrap(),
        PollReply::RejectedMarker { .. }
    ));
    assert_eq!(s.state(), State::Ready);
    assert!(
        matches!(s.poll().unwrap(), PollReply::Records { records: received, .. } if received == records)
    );
    let script = s.into_transport();
    assert_eq!(script.calls, 7);
    assert!(script.exchanges.is_empty());
}
#[test]
fn marker_policy_never_recovers_transport_partial_or_malformed_failures() {
    for kind in 0..3 {
        let mut script = vec![write(word(0xAC010000)), read(word(0xACFFFFFF))];
        if kind == 0 {
            script.push(Exchange::Write(word(0xAC010000), Ok(3)));
        } else {
            script.push(write(word(0xAC010000)));
            script.push(if kind == 1 {
                Exchange::Read(vec![], Err(TransportError::Unknown))
            } else {
                read(vec![5, 0, 0, 0xAC, 1])
            });
        }
        let mut s = ready_policy(script, PollPolicy::ReportRejectedMarker);
        assert!(matches!(
            s.poll().unwrap(),
            PollReply::RejectedMarker { .. }
        ));
        assert!(s.poll().is_err());
        assert_eq!(s.state(), State::Halted);
        assert!(s.poll().is_err());
        assert_eq!(s.into_transport().calls, if kind == 0 { 6 } else { 7 });
    }
}
fn assert_halted(mut session: Session<Script>, calls: usize) {
    assert_eq!(session.state(), State::Halted);
    assert_eq!(
        session.initialize(),
        Err(session::Error::WrongState(State::Halted))
    );
    assert_eq!(
        session.poll(),
        Err(session::Error::WrongState(State::Halted))
    );
    assert_eq!(
        session.publish(&[]),
        Err(session::Error::WrongState(State::Halted))
    );
    assert_eq!(session.into_transport().calls, calls);
}

#[test]
fn full_exchange_publishes_inventory_and_reuses_planner_for_assignments() {
    let advertised = vec![record(0, b"game"), record(0, b"voice")];
    // Independently specified wire bytes, not generated by the session.
    let mut outgoing = word(0xAC00000E);
    outgoing.extend_from_slice(b"\x04\x00game\x05\x00voice\x00");
    let mut incoming = word(0xAC00000E);
    incoming.extend_from_slice(b"\x04\x02game\x05\x00voice\x00");
    incoming.extend([0xFF; 17]);
    let mut session = ready(vec![
        write(outgoing),
        write(word(0xAC010000)),
        read(incoming),
    ]);
    session.publish(&advertised).unwrap();
    let PollReply::Records {
        records,
        ignored_tail,
    } = session.poll().unwrap()
    else {
        panic!()
    };
    assert_eq!(ignored_tail, 17);
    let inventory = Inventory {
        snapshot_id: 42,
        applications: vec![
            Application {
                id: ApplicationId("app/game".into()),
                advertised_name: b"game".to_vec(),
                streams: vec![Stream {
                    id: StreamId("stream/game".into()),
                    state: StreamState::Live,
                }],
            },
            Application {
                id: ApplicationId("app/voice".into()),
                advertised_name: b"voice".to_vec(),
                streams: vec![Stream {
                    id: StreamId("stream/voice".into()),
                    state: StreamState::Live,
                }],
            },
        ],
    };
    let plan = plan(&inventory, &records).unwrap();
    assert_eq!(plan.snapshot_id, 42);
    let routes: Vec<_> = plan.proposed_routes().collect();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].application_id, ApplicationId("app/game".into()));
    assert_eq!(routes[0].link, Link::Two);
    assert!(matches!(
        plan.resolutions[1].outcome,
        Outcome::OpaqueAssignment { value: 0 }
    ));
    assert_eq!(session.state(), State::Ready);
    let transport = session.into_transport();
    assert_eq!(transport.calls, 6);
    assert!(transport.exchanges.is_empty());
}

#[test]
fn handshake_body_is_retained_but_debug_is_redacted() {
    let mut session = session(startup());
    let reply = session.initialize().unwrap();
    assert_eq!(reply.body(), &[0x5A; 508]);
    assert_eq!(
        format!("{reply:?}"),
        "HandshakeReply { body: [redacted; 508] }"
    );
    assert_eq!(
        session.initialize(),
        Err(session::Error::WrongState(State::Ready))
    );
    assert_eq!(session.into_transport().calls, 3);
}

#[test]
fn handshake_failures_halt_at_each_boundary_without_retry() {
    for (index, step) in [Step::A0, Step::A1, Step::A2].into_iter().enumerate() {
        for error in [TransportError::NoTransfer, TransportError::Unknown] {
            let mut exchanges = startup();
            exchanges[index] = if index < 2 {
                Exchange::Write(
                    word(if index == 0 { 0xA0000000 } else { 0xA1000000 }),
                    Err(error),
                )
            } else {
                Exchange::Read(vec![], Err(error))
            };
            let mut session = session(exchanges);
            assert_eq!(
                session.initialize(),
                Err(session::Error::Transport { step, error })
            );
            assert_halted(session, index + 1);
        }
    }
    for length in [0, 4, 511, 513] {
        let mut exchanges = startup();
        exchanges[2] = Exchange::Read(a2(), Ok(length));
        let mut session = session(exchanges);
        assert!(matches!(
            session.initialize(),
            Err(session::Error::TransferLength { step: Step::A2, .. })
        ));
        assert_halted(session, 3);
    }
    let mut wrong = a2();
    wrong[..4].copy_from_slice(&0xAC000000u32.to_le_bytes());
    let mut session = session(vec![
        write(word(0xA0000000)),
        write(word(0xA1000000)),
        read(wrong),
    ]);
    assert_eq!(
        session.initialize(),
        Err(session::Error::UnexpectedReply { word: 0xAC000000 })
    );
    assert_halted(session, 3);
}

#[test]
fn partial_or_impossible_write_lengths_halt_without_read_or_resubmit() {
    for length in [0, 3, 5] {
        let mut session = session(vec![Exchange::Write(word(0xA0000000), Ok(length))]);
        assert!(matches!(
            session.initialize(),
            Err(session::Error::TransferLength { step: Step::A0, .. })
        ));
        assert_halted(session, 1);
    }
    let mut session = ready(vec![Exchange::Write(word(0xAC010000), Ok(3))]);
    assert!(matches!(
        session.poll(),
        Err(session::Error::TransferLength {
            step: Step::PollRequest,
            ..
        })
    ));
    assert_halted(session, 4);
    let mut frame = word(0xAC000001);
    frame.push(0);
    let mut session = ready(vec![Exchange::Write(frame, Ok(4))]);
    assert!(matches!(
        session.publish(&[]),
        Err(session::Error::TransferLength {
            step: Step::Publish,
            ..
        })
    ));
    assert_halted(session, 4);
}

#[test]
fn every_short_declared_reply_is_rejected_even_if_buffer_contains_old_bytes() {
    let mut full = word(0xAC000007);
    full.extend_from_slice(b"\x04\x01game\x00");
    for actual in 0..full.len() {
        let mut session = ready(vec![
            write(word(0xAC010000)),
            Exchange::Read(full.clone(), Ok(actual)),
        ]);
        assert!(matches!(
            session.poll(),
            Err(session::Error::Reply(Error::Truncated { .. }))
        ));
        assert_halted(session, 5);
    }
}

#[test]
fn empty_payload_and_empty_record_list_remain_distinct_from_assignment_zero() {
    let mut empty = word(0xAC000000);
    empty.extend([0xFF; 508]);
    assert_eq!(
        decode_poll_reply(&empty),
        Ok(PollReply::NoPayload { ignored_tail: 508 })
    );
    assert_eq!(
        decode_poll_reply(&[1, 0, 0, 0xAC, 0]),
        Ok(PollReply::Records {
            records: vec![],
            ignored_tail: 0
        })
    );
    assert_eq!(
        decode_poll_reply(&[4, 0, 0, 0xAC, 1, 0, b'x', 0]),
        Ok(PollReply::Records {
            records: vec![record(0, b"x")],
            ignored_tail: 0
        })
    );
    assert!(decode_frame(&empty).is_err()); // Do not relax the exact-frame codec.
}

#[test]
fn unsupported_envelopes_and_malformed_records_fail_closed() {
    for value in [
        0xACFFFFFF, 0xAC0001FD, 0xAC020001, 0xAC010000, 0xA2000000, 0xAB000001,
    ] {
        let mut session = ready(vec![write(word(0xAC010000)), read(word(value))]);
        assert!(session.poll().is_err(), "{value:08X}");
        assert_halted(session, 5);
    }
    for payload in [vec![1], vec![1, 1, b'x'], vec![0, 0], vec![2, 1, b'x']] {
        let mut reply = word(0xAC000000 | payload.len() as u32);
        reply.extend(payload);
        assert!(decode_poll_reply(&reply).is_err());
    }
    assert!(decode_poll_reply(&[0; 513]).is_err());
    let mut raw = word(0xAC000004);
    raw.extend([1, 255, 0xFF, 0]);
    assert_eq!(
        decode_poll_reply(&raw),
        Ok(PollReply::Records {
            records: vec![record(255, &[0xFF])],
            ignored_tail: 0,
        })
    );
}

#[test]
fn oversized_or_unsupported_publication_sends_nothing_and_preserves_ready() {
    let mut session = ready(vec![]);
    for records in [
        vec![record(0, b"\xC3\xA9")],
        vec![record(0, b"")],
        vec![record(0, &[b'x'; 255]), record(0, &[b'y'; 249])],
    ] {
        assert!(matches!(
            session.publish(&records),
            Err(session::Error::Encode(_))
        ));
        assert_eq!(session.state(), State::Ready);
    }
    assert_eq!(session.into_transport().calls, 3);
    let records = vec![record(1, &[b'x'; 255]), record(4, &[b'y'; 248])];
    let frame = encode_frame(&Frame::Data(encode_records(&records).unwrap())).unwrap();
    assert_eq!(frame.len(), 512);
    let mut session = ready(vec![write(frame)]);
    session.publish(&records).unwrap();
    assert_eq!(session.into_transport().calls, 4);
}

#[test]
fn post_handshake_transport_errors_halt_without_repeating_the_exchange() {
    for error in [TransportError::NoTransfer, TransportError::Unknown] {
        let mut session = ready(vec![Exchange::Write(word(0xAC010000), Err(error))]);
        assert_eq!(
            session.poll(),
            Err(session::Error::Transport {
                step: Step::PollRequest,
                error
            })
        );
        assert_halted(session, 4);
        let mut session = ready(vec![
            write(word(0xAC010000)),
            Exchange::Read(vec![], Err(error)),
        ]);
        assert_eq!(
            session.poll(),
            Err(session::Error::Transport {
                step: Step::PollReply,
                error
            })
        );
        assert_halted(session, 5);
        let mut frame = word(0xAC000001);
        frame.push(0);
        let mut session = ready(vec![Exchange::Write(frame, Err(error))]);
        assert_eq!(
            session.publish(&[]),
            Err(session::Error::Transport {
                step: Step::Publish,
                error
            })
        );
        assert_halted(session, 4);
    }
}

#[test]
fn uninitialized_or_closed_session_cannot_access_transport() {
    assert!(matches!(
        Session::new(
            Script {
                exchanges: VecDeque::new(),
                calls: 0,
                read_timeouts: VecDeque::new(),
            },
            Duration::ZERO
        ),
        Err(session::Error::ZeroTimeout)
    ));
    let mut session = session(vec![]);
    assert_eq!(session.poll(), Err(session::Error::WrongState(State::New)));
    assert_eq!(
        session.publish(&[]),
        Err(session::Error::WrongState(State::New))
    );
    session.close();
    assert_eq!(
        session.initialize(),
        Err(session::Error::WrongState(State::Closed))
    );
    assert_eq!(
        session.poll(),
        Err(session::Error::WrongState(State::Closed))
    );
    assert_eq!(session.into_transport().calls, 0);
}

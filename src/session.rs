//! Call-driven, transport-neutral static protocol subset. No USB or routing backend.
//! A successful write is a transport completion, not remote application acceptance.
use crate::{
    decode_records, encode_frame, encode_records, ControlWord, Frame, Header, Record, MAX_FRAME,
};
use std::{fmt, time::Duration};

pub const OUT_PIPE: u8 = 0x03;
pub const IN_PIPE: u8 = 0x83;

/// Implementors must bound each operation by `timeout`, report actual transferred
/// bytes, and never retry internally or fall back to another device/interface.
/// `NoTransfer` is only valid when no bytes were submitted or received.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    NoTransfer,
    Unknown,
}
pub trait Transport {
    fn write(&mut self, pipe: u8, bytes: &[u8], timeout: Duration)
        -> Result<usize, TransportError>;
    fn read(
        &mut self,
        pipe: u8,
        bytes: &mut [u8],
        timeout: Duration,
    ) -> Result<usize, TransportError>;
}

/// An opt-in transport policy for caller-observed inventory between successful
/// exchanges. This does not supply OS freshness or authorize route application.
pub trait PublicationCycles: Transport {
    fn refresh_publication(
        &mut self,
        inventory: &crate::planner::Inventory,
    ) -> Result<(), TransportError>;
    fn refresh_verified_publication(
        &mut self,
        _publication: &crate::route_publication::VerifiedRoutePublication,
    ) -> Result<(), TransportError> {
        Err(TransportError::NoTransfer)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    New,
    Ready,
    Halted,
    Closed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    A0,
    A1,
    A2,
    Publish,
    PollRequest,
    PollReply,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    ZeroTimeout,
    WrongState(State),
    Transport {
        step: Step,
        error: TransportError,
    },
    TransferLength {
        step: Step,
        expected: usize,
        actual: usize,
    },
    Reply(crate::Error),
    UnexpectedReply {
        word: u32,
    },
    Encode(crate::Error),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "session: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Exact A2 body, intentionally opaque. Debug output omits potentially private
/// device metadata. No version, identity or capability semantics are asserted.
#[derive(Clone, PartialEq, Eq)]
pub struct HandshakeReply([u8; MAX_FRAME - 4]);
impl HandshakeReply {
    pub fn body(&self) -> &[u8; MAX_FRAME - 4] {
        &self.0
    }
}
impl fmt::Debug for HandshakeReply {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("HandshakeReply { body: [redacted; 508] }")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PollReply {
    /// Exact ACFFFFFF marker, reported only by an explicit observation policy.
    /// No assignments or peer-state/heartbeat semantics are inferred.
    RejectedMarker { ignored_tail: usize },
    /// Observed reader accepts AC000000 without calling its record parser.
    /// This is not assignment zero, unroute, or an authoritative empty inventory.
    NoPayload { ignored_tail: usize },
    Records {
        records: Vec<Record>,
        ignored_tail: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PollPolicy {
    #[default]
    Strict,
    /// The inspected Windows callback returns without parsing/updating its
    /// counter on this marker, while its scheduler keeps running. This policy
    /// reports that completed observation; it never retries failed native I/O.
    ReportRejectedMarker,
}

/// Explicit interpretation at the planning boundary, never a wire-codec change.
/// The planner still validates printable ASCII and exact current inventory names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReceivedNamePolicy {
    #[default]
    ExactBytes,
    /// Match the inspected Windows string consumers: bytes before the first NUL.
    /// Remaining bytes are opaque, not an alias, PID or routing instruction.
    WindowsCString,
}
impl ReceivedNamePolicy {
    pub fn for_planning(self, record: &Record) -> Record {
        let length = match self {
            Self::ExactBytes => record.name.len(),
            Self::WindowsCString => record
                .name
                .iter()
                .position(|b| *b == 0)
                .unwrap_or(record.name.len()),
        };
        Record {
            assignment: record.assignment,
            name: record.name[..length].to_vec(),
        }
    }
}

/// Parse the declared payload only, as the inspected Windows reader does. Require
/// its entire length to be present (a safety check missing in that reader).
/// Extra transfer bytes are counted, not treated as another message or record.
/// The generic exact-sized frame codec deliberately remains unchanged.
pub fn decode_poll_reply(bytes: &[u8]) -> Result<PollReply, Error> {
    decode_poll_reply_with_policy(bytes, PollPolicy::Strict)
}
pub fn decode_poll_reply_with_policy(bytes: &[u8], policy: PollPolicy) -> Result<PollReply, Error> {
    if bytes.len() > MAX_FRAME {
        return Err(Error::Reply(crate::Error::TrailingData {
            expected: MAX_FRAME,
            actual: bytes.len(),
        }));
    }
    if bytes.len() < 4 {
        return Err(Error::Reply(crate::Error::Truncated {
            needed: 4,
            actual: bytes.len(),
        }));
    }
    let word = u32::from_le_bytes(bytes[..4].try_into().expect("length checked"));
    if word == 0xACFFFFFF && policy == PollPolicy::ReportRejectedMarker {
        return Ok(PollReply::RejectedMarker {
            ignored_tail: bytes.len() - 4,
        });
    }
    if word == 0xAC000000 {
        return Ok(PollReply::NoPayload {
            ignored_tail: bytes.len() - 4,
        });
    }
    let Header::Data { length } = Header::decode(&bytes[..4]).map_err(Error::Reply)? else {
        return Err(Error::UnexpectedReply { word });
    };
    let end = 4 + usize::from(length);
    if bytes.len() < end {
        return Err(Error::Reply(crate::Error::Truncated {
            needed: end,
            actual: bytes.len(),
        }));
    }
    Ok(PollReply::Records {
        records: decode_records(&bytes[4..end]).map_err(Error::Reply)?,
        ignored_tail: bytes.len() - end,
    })
}

/// Owns one selected transport, serializing all exchanges through `&mut self`.
/// Any transport/malformed-reply failure halts the session. Strict default also
/// halts on the rejected marker; an explicit policy reports it separately.
/// No automatic retry, reconnect,
/// publication timer, routing, or hardware access is implemented here.
pub struct Session<T> {
    transport: T,
    timeout: Duration,
    state: State,
    poll_policy: PollPolicy,
}
impl<T: Transport> Session<T> {
    pub fn new(transport: T, timeout: Duration) -> Result<Self, Error> {
        Self::new_with_poll_policy(transport, timeout, PollPolicy::Strict)
    }
    pub fn new_with_poll_policy(
        transport: T,
        timeout: Duration,
        poll_policy: PollPolicy,
    ) -> Result<Self, Error> {
        if timeout.is_zero() {
            return Err(Error::ZeroTimeout);
        }
        Ok(Self {
            transport,
            timeout,
            state: State::New,
            poll_policy,
        })
    }
    pub fn state(&self) -> State {
        self.state
    }
    /// Local stop only; does not cancel OS I/O or send a guessed shutdown word.
    pub fn close(&mut self) {
        self.state = State::Closed;
    }
    /// Consume the controller to let the caller release the selected transport.
    pub fn into_transport(self) -> T {
        self.transport
    }
    fn require(&self, state: State) -> Result<(), Error> {
        if self.state == state {
            Ok(())
        } else {
            Err(Error::WrongState(self.state))
        }
    }
    fn write(&mut self, step: Step, bytes: &[u8]) -> Result<(), Error> {
        let actual = self
            .transport
            .write(OUT_PIPE, bytes, self.timeout)
            .map_err(|error| Error::Transport { step, error })?;
        if actual != bytes.len() {
            return Err(Error::TransferLength {
                step,
                expected: bytes.len(),
                actual,
            });
        }
        Ok(())
    }
    fn read(&mut self, step: Step, buffer: &mut [u8; MAX_FRAME]) -> Result<usize, Error> {
        self.read_with_timeout(step, buffer, self.timeout)
    }
    fn read_with_timeout(
        &mut self,
        step: Step,
        buffer: &mut [u8; MAX_FRAME],
        timeout: Duration,
    ) -> Result<usize, Error> {
        let actual = self
            .transport
            .read(IN_PIPE, buffer, timeout)
            .map_err(|error| Error::Transport { step, error })?;
        if actual > buffer.len() {
            return Err(Error::TransferLength {
                step,
                expected: buffer.len(),
                actual,
            });
        }
        Ok(actual)
    }
    fn finish<R>(&mut self, result: Result<R, Error>) -> Result<R, Error> {
        if result.is_err() {
            self.state = State::Halted;
        }
        result
    }
    pub fn initialize(&mut self) -> Result<HandshakeReply, Error> {
        self.initialize_with_read_timeout(self.timeout)
    }
    /// Explicit one-read diagnostic deadline. Writes and all later operations
    /// retain the controller's ordinary timeout. A failed read still halts.
    pub fn initialize_with_read_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<HandshakeReply, Error> {
        self.require(State::New)?;
        if timeout.is_zero() {
            return Err(Error::ZeroTimeout);
        }
        let result = (|| {
            self.write(Step::A0, &ControlWord::A0.word().to_le_bytes())?;
            self.write(Step::A1, &ControlWord::A1.word().to_le_bytes())?;
            let mut buffer = [0; MAX_FRAME];
            let actual = self.read_with_timeout(Step::A2, &mut buffer, timeout)?;
            if actual != MAX_FRAME {
                return Err(Error::TransferLength {
                    step: Step::A2,
                    expected: MAX_FRAME,
                    actual,
                });
            }
            let word = u32::from_le_bytes(buffer[..4].try_into().expect("fixed buffer"));
            if word != ControlWord::A2.word() {
                return Err(Error::UnexpectedReply { word });
            }
            Ok(HandshakeReply(
                buffer[4..].try_into().expect("fixed buffer"),
            ))
        })();
        let reply = self.finish(result)?;
        self.state = State::Ready;
        Ok(reply)
    }
    /// Explicit caller-owned records; does not invent assignments from desired
    /// routes. Whole-list encoding succeeds before any bytes are sent.
    pub fn publish(&mut self, records: &[Record]) -> Result<(), Error> {
        self.require(State::Ready)?;
        let payload = encode_records(records).map_err(Error::Encode)?;
        let frame = encode_frame(&Frame::Data(payload)).map_err(Error::Encode)?;
        let result = self.write(Step::Publish, &frame);
        self.finish(result)
    }
    pub fn poll(&mut self) -> Result<PollReply, Error> {
        self.require(State::Ready)?;
        let result = (|| {
            self.write(Step::PollRequest, &ControlWord::Ac01.word().to_le_bytes())?;
            let mut buffer = [0; MAX_FRAME];
            let actual = self.read(Step::PollReply, &mut buffer)?;
            decode_poll_reply_with_policy(&buffer[..actual], self.poll_policy)
        })();
        self.finish(result)
    }
}

impl<T: PublicationCycles> Session<T> {
    pub fn refresh_verified_publication(
        &mut self,
        publication: &crate::route_publication::VerifiedRoutePublication,
    ) -> Result<(), Error> {
        self.require(State::Ready)?;
        let result = self
            .transport
            .refresh_verified_publication(publication)
            .map_err(|error| Error::Transport {
                step: Step::Publish,
                error,
            });
        self.finish(result)
    }
    pub fn refresh_publication(
        &mut self,
        inventory: &crate::planner::Inventory,
    ) -> Result<(), Error> {
        self.require(State::Ready)?;
        let result = self
            .transport
            .refresh_publication(inventory)
            .map_err(|error| Error::Transport {
                step: Step::Publish,
                error,
            });
        self.finish(result)
    }
}

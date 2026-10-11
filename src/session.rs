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
    /// Observed reader accepts AC000000 without calling its record parser.
    /// This is not assignment zero, unroute, or an authoritative empty inventory.
    NoPayload { ignored_tail: usize },
    Records {
        records: Vec<Record>,
        ignored_tail: usize,
    },
}

/// Parse the declared payload only, as the inspected Windows reader does. Require
/// its entire length to be present (a safety check missing in that reader).
/// Extra transfer bytes are counted, not treated as another message or record.
/// The generic exact-sized frame codec deliberately remains unchanged.
pub fn decode_poll_reply(bytes: &[u8]) -> Result<PollReply, Error> {
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
/// Any transport/peer failure halts the session. No automatic retry, reconnect,
/// publication timer, routing, or hardware access is implemented here.
pub struct Session<T> {
    transport: T,
    timeout: Duration,
    state: State,
}
impl<T: Transport> Session<T> {
    pub fn new(transport: T, timeout: Duration) -> Result<Self, Error> {
        if timeout.is_zero() {
            return Err(Error::ZeroTimeout);
        }
        Ok(Self {
            transport,
            timeout,
            state: State::New,
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
        let actual = self
            .transport
            .read(IN_PIPE, buffer, self.timeout)
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
        self.require(State::New)?;
        let result = (|| {
            self.write(Step::A0, &ControlWord::A0.word().to_le_bytes())?;
            self.write(Step::A1, &ControlWord::A1.word().to_le_bytes())?;
            let mut buffer = [0; MAX_FRAME];
            let actual = self.read(Step::A2, &mut buffer)?;
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
            decode_poll_reply(&buffer[..actual])
        })();
        self.finish(result)
    }
}

//! Offline codecs for a static-evidence subset. No device access.
//! Strict limits are conservative policy, not claims about device tolerance.
use std::fmt;

pub const MAX_PAYLOAD: usize = 508;
pub const MAX_NAME: usize = 255;
pub const MAX_FRAME: usize = MAX_PAYLOAD + 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Truncated {
        needed: usize,
        actual: usize,
    },
    TrailingData {
        expected: usize,
        actual: usize,
    },
    PayloadLength {
        length: usize,
    },
    InvalidHeader {
        word: u32,
    },
    UnsupportedHeader {
        word: u32,
    },
    RejectedHeader,
    MissingTerminator,
    EmptyName {
        record: usize,
    },
    NameTooLong {
        record: usize,
        length: usize,
    },
    UnsupportedName {
        record: usize,
        index: usize,
        byte: u8,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { needed, actual } => write!(f, "truncated: need {needed} bytes, have {actual}"),
            Self::TrailingData { expected, actual } => write!(f, "trailing data: expected {expected} bytes, have {actual}"),
            Self::PayloadLength { length } => write!(f, "payload length {length} outside 1..={MAX_PAYLOAD}"),
            Self::InvalidHeader { word } => write!(f, "invalid header 0x{word:08X}"),
            Self::UnsupportedHeader { word } => write!(f, "unsupported header 0x{word:08X}"),
            Self::RejectedHeader => f.write_str("0xACFFFFFF is a rejected header"),
            Self::MissingTerminator => f.write_str("missing zero-length record terminator"),
            Self::EmptyName { record } => write!(f, "record {record}: empty name"),
            Self::NameTooLong { record, length } => write!(f, "record {record}: name length {length} exceeds {MAX_NAME}"),
            Self::UnsupportedName { record, index, byte } => write!(f, "record {record}: unsupported name byte 0x{byte:02X} at {index}; encoding requires printable ASCII"),
        }
    }
}
impl std::error::Error for Error {}

/// Observed constants; names deliberately imply no handshake semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlWord {
    A0,
    A1,
    A2,
    Ac01,
}
impl ControlWord {
    pub const fn word(self) -> u32 {
        match self {
            Self::A0 => 0xA0000000,
            Self::A1 => 0xA1000000,
            Self::A2 => 0xA2000000,
            Self::Ac01 => 0xAC010000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Header {
    Data { length: u16 },
    Control(ControlWord),
}
impl Header {
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        exact_length(bytes.len(), 4)?;
        let word = u32::from_le_bytes(bytes.try_into().expect("length checked"));
        let control = match word {
            0xA0000000 => Some(ControlWord::A0),
            0xA1000000 => Some(ControlWord::A1),
            0xA2000000 => Some(ControlWord::A2),
            0xAC010000 => Some(ControlWord::Ac01),
            _ => None,
        };
        if let Some(value) = control {
            return Ok(Self::Control(value));
        }
        if word == 0xACFFFFFF {
            return Err(Error::RejectedHeader);
        }
        if word >> 24 != 0xAC {
            return Err(Error::InvalidHeader { word });
        }
        // Bits 23..16 outside the observed data writer are unsupported.
        if word & 0x00FF0000 != 0 {
            return Err(Error::UnsupportedHeader { word });
        }
        let length = (word & 0xFFFF) as usize;
        validate_payload(length)?;
        Ok(Self::Data {
            length: length as u16,
        })
    }

    pub fn encode(self) -> Result<[u8; 4], Error> {
        let word = match self {
            Self::Control(value) => value.word(),
            Self::Data { length } => {
                validate_payload(length as usize)?;
                0xAC000000 | u32::from(length)
            }
        };
        Ok(word.to_le_bytes())
    }
}

fn validate_payload(length: usize) -> Result<(), Error> {
    if !(1..=MAX_PAYLOAD).contains(&length) {
        Err(Error::PayloadLength { length })
    } else {
        Ok(())
    }
}
fn exact_length(actual: usize, expected: usize) -> Result<(), Error> {
    if actual < expected {
        Err(Error::Truncated {
            needed: expected,
            actual,
        })
    } else if actual > expected {
        Err(Error::TrailingData { expected, actual })
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    Data(Vec<u8>),
    Control(ControlWord),
}

/// Exact-sized frames only. A2's observed 512-byte response is not modeled;
/// use Header::decode on its first four bytes without implying a full parse.
pub fn decode_frame(bytes: &[u8]) -> Result<Frame, Error> {
    if bytes.len() < 4 {
        return Err(Error::Truncated {
            needed: 4,
            actual: bytes.len(),
        });
    }
    match Header::decode(&bytes[..4])? {
        Header::Control(value) => {
            exact_length(bytes.len(), 4)?;
            Ok(Frame::Control(value))
        }
        Header::Data { length } => {
            exact_length(bytes.len(), 4 + usize::from(length))?;
            Ok(Frame::Data(bytes[4..].to_vec()))
        }
    }
}

pub fn encode_frame(frame: &Frame) -> Result<Vec<u8>, Error> {
    let (header, payload): (Header, &[u8]) = match frame {
        Frame::Control(value) => (Header::Control(*value), &[]),
        Frame::Data(bytes) => {
            validate_payload(bytes.len())?;
            (
                Header::Data {
                    length: bytes.len() as u16,
                },
                bytes,
            )
        }
    };
    let mut bytes = header.encode()?.to_vec();
    bytes.extend_from_slice(payload);
    Ok(bytes)
}

/// All byte values are retained. Zero and values above four are opaque.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Assignment(u8);
impl Assignment {
    pub const fn from_byte(value: u8) -> Self {
        Self(value)
    }
    pub const fn byte(self) -> u8 {
        self.0
    }
    pub fn known_link(self) -> Option<u8> {
        if (1..=4).contains(&self.0) {
            Some(self.0)
        } else {
            None
        }
    }
}

/// Decoding preserves name bytes; it does not guess an ANSI codepage or UTF-8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub assignment: Assignment,
    pub name: Vec<u8>,
}

pub fn decode_records(bytes: &[u8]) -> Result<Vec<Record>, Error> {
    if bytes.len() > MAX_PAYLOAD {
        return Err(Error::PayloadLength {
            length: bytes.len(),
        });
    }
    let mut records = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let length = usize::from(bytes[offset]);
        offset += 1;
        if length == 0 {
            exact_length(bytes.len(), offset)?;
            return Ok(records);
        }
        let end = offset + 1 + length;
        if end > bytes.len() {
            return Err(Error::Truncated {
                needed: end,
                actual: bytes.len(),
            });
        }
        records.push(Record {
            assignment: Assignment::from_byte(bytes[offset]),
            name: bytes[offset + 1..end].to_vec(),
        });
        offset = end;
    }
    Err(Error::MissingTerminator)
}

/// Conservative compatibility mode: printable ASCII only, no silent truncation.
/// This does not reproduce the suspected Windows character-count/copy mismatch.
pub fn encode_records(records: &[Record]) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::new();
    for (record, value) in records.iter().enumerate() {
        let length = value.name.len();
        if length == 0 {
            return Err(Error::EmptyName { record });
        }
        if length > MAX_NAME {
            return Err(Error::NameTooLong { record, length });
        }
        for (index, byte) in value.name.iter().copied().enumerate() {
            if !(0x20..=0x7E).contains(&byte) {
                return Err(Error::UnsupportedName {
                    record,
                    index,
                    byte,
                });
            }
        }
        let total = bytes.len() + 2 + length + 1; // Reserve final terminator.
        if total > MAX_PAYLOAD {
            return Err(Error::PayloadLength { length: total });
        }
        bytes.push(length as u8);
        bytes.push(value.assignment.byte());
        bytes.extend_from_slice(&value.name);
    }
    bytes.push(0);
    Ok(bytes)
}

/// Pure offline routing proposals over provider-neutral snapshots.
pub mod planner;

/// Bounded text input for pure routing proposals; no OS access.
pub mod snapshot;

/// Pure inventory lifecycle tracking and plan generation checks.
pub mod lifecycle;

/// Pure guarded route preparation; actual audio application remains disabled.
pub mod executor;

/// Bounded graph metadata bridge into guarded preparation.
pub mod graph_snapshot;

/// Pure operation acknowledgment and concrete receipt bookkeeping.
pub mod receipts;

pub mod execution;

pub mod route_publication;
/// Guarded transport-neutral protocol exchanges; no USB or automatic routing.
pub mod session;

/// Offline cached USB descriptor validation; never opens or claims a device.
pub mod usb_descriptors;

/// Guarded one-shot USB handshake lifecycle; native access is separately gated.
pub mod usb_handshake;

#[cfg(all(feature = "native-usb", target_os = "linux"))]
pub mod native_usb;

#[cfg(all(feature = "native-pipewire", target_os = "linux"))]
pub mod native_pipewire;

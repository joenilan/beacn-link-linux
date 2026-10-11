# Offline protocol codec

The dependency-free Rust library and link-protocol CLI implement a confirmed
static-evidence subset. They do not replace Windows Link, discover applications,
route audio, contact devices, or implement a handshake. All validation below
uses synthetic bytes. See [protocol evidence](protocol.md) for provenance.

## Use

Rust 1.88 or newer is required. No external crates are used.

```text
cargo build --offline -j 1
cargo test --offline -j 1
cargo run --offline --bin link-protocol -- decode 0e0000ac040167616d650500766f69636500
cargo run --offline --bin link-protocol -- encode 1:game 0:voice
cargo run --offline --bin link-protocol -- decode-header 000000a0
```

The decode example reports game on assignment 1 (Link 1) and voice on
assignment 0 (opaque/unknown). Encode outputs that same hex frame. Quote a
hex argument to include spaces, or quote a record argument containing spaces.
An empty encode list produces a data frame with only the zero terminator.
Failures print a specific error to stderr and exit with code 2.

CLI decode expects one exact data frame containing assignment records, or a
four-byte observed control word. It prints escaped raw names and name hex, not
an assumed Unicode string. decode-header accepts exactly four bytes and is
useful for inspecting an observed header without interpreting its body.
All CLI input is command-line text; no device or file reads are implemented.

## Supported subset and conservative policy

- Data envelope: little-endian DWORD 0xAC000000 | payload length, followed by
  exactly that many bytes. Payload length must be 1..508; frame maximum is 512.
- Bits 23..16 in a data header must be zero. Other AC header variants are
  unsupported except observed control 0xAC010000. 0xACFFFFFF is rejected.
- Observed constants 0xA0000000, 0xA1000000, 0xA2000000 and 0xAC010000
  can be encoded/decoded as four-byte headers. Their names are numeric labels.
  No initialization order, timing, success meaning or response body is modeled.
  A full observed 512-byte A2 response is deliberately unsupported by
  decode_frame; Header::decode can inspect its first four bytes.
- Assignment records: length u8, assignment u8, exactly length name bytes,
  followed by a final zero-length terminator. No trailing bytes, padding,
  missing terminator, partial record or silent list truncation is accepted.
- Names in encoded records must be nonempty printable ASCII (bytes 0x20..0x7E)
  and at most 255 bytes; combined records plus terminator must fit 508 bytes.
  These are strict codec limits, not claims about all firmware behavior.
- Decode preserves arbitrary name bytes and every assignment byte. Assignment
  values 1..4 have a known Link index. Zero and 5..255 remain opaque; zero
  is not interpreted as unroute. Encoding also preserves those opaque assignments.
- No process-path normalization is performed. Callers provide already selected
  names. Duplicate names and ordering are retained without deduplication.

The Windows character-count/byte-copy mismatch, ANSI codepage and Unicode
behavior remain unresolved. Encoding rejects non-ASCII and control bytes
instead of silently reproducing suspected Windows bugs. Raw decoding is
therefore intentionally broader than conservative ASCII encoding.

## Library API

Header::decode/encode handles exact four-byte headers. decode_frame and
encode_frame handle the envelope or a header-only control word. decode_records
and encode_records handle assignment lists independently. Assignment exposes
byte() and known_link(); Record stores the name as Vec<u8>.

Error is a typed enum distinguishing truncation, trailing data, payload/name
limits, empty or unsupported outgoing names, missing terminator, invalid or
unsupported headers, and the rejected header sentinel. Error fields identify
required/actual byte counts or the failing record/name byte. No recovery,
padding acceptance or stream resynchronization is assumed.

## Verification and limits

On 2026-10-11, installed Windows MSVC Rust/Cargo 1.88.0 compiled this crate.
cargo build --offline -j 1 and formatting checks completed;
cargo test --offline -j 1 passed all eight focused
integration tests, including synthetic fixtures and CLI encode/decode/error
checks. Build and tests used no external dependencies or installations.

The fixtures directory contains handwritten hex, not device captures.
Tests cover frame and record roundtrips, all shorter prefixes of the ASCII
fixture, missing terminators, malformed/unsupported headers, conservative
boundaries, trailing data and unsupported outgoing names.

Linux compilation/runtime, real USB messages, handshake bodies, discovery,
audio routing and interoperability with BEACN hardware/App remain untested.
No USB, PipeWire, audio, daemon, startup, network or target-binary code is present.

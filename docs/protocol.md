# BEACN Link protocol evidence (static)

Partial static reconstruction of BEACNLink.exe 1.0.4, 5255096 bytes.
SHA256: 74760CE85BEE8620C5BF81F5951FECFE20A04EDF163129361C6890D5481DB64C.
Observed on 2026-10-11 using pefile 2024.8.26 and Capstone 5.0.7.
Addresses are preferred virtual addresses, image base 0x140000000, not file offsets.
No target execution, device access, capture or Linux runtime validation occurred.
Confirmed means instructions/imports/RTTI were inspected; purpose labels below
are inferences. This is not a hardware-validated interoperability specification.

## Discovery anchor correction

0x14013296D constructs interface GUID {BB9A2EE6-D5EF-4AB7-AA23-85B2F6DDF006}.
Nearby code loads VID/PID 33AE:4003. The call at 0x1401329E1 to 0x1400075F0
formats the UTF-16 string VID_%04X&PID_%04X; it is not USB I/O.
0x1401329F0 instead calls 0x140136450, which stores the GUID-containing record.

## Transport and envelopes

- 0x140130610 and 0x140130B70 build DWORD (N | 0xAC000000), copy payload
  immediately after it, and write N+4 bytes. Normal copy capacity is 512 bytes.
- WinUsb_WritePipe calls 0x1401306D4/0x140130C2F use pipe ID 0x03.
- Reads 0x1401307F3/0x140130D11 use pipe ID 0x83.
- DWORD stores imply little-endian transmission; pipe descriptor type is unverified.
- 0x140130710 writes four-byte DWORDs 0xA0000000 then 0xA1000000, checks
  successful four-byte writes, reads exactly 512 bytes, and requires first
  DWORD 0xA2000000. Initialization-handshake purpose is an inference.
- 0x140130C60 writes 0xAC010000, reads up to 512 bytes, requires at least four,
  checks high byte 0xAC, rejects 0xACFFFFFF, extracts low-16-bit payload length,
  and accepts lengths 1..508. Complete length consistency validation is unverified.
- Setup calls 0x14013121C/0x14013123F set policy ID 3 with four-byte
  value 150 on pipes 0x83/0x03. Calls 0x140131264/0x140131285 set
  policy ID 2 with one-byte value 1 on both pipes; meanings are not inferred here.
- 0x140130DD3 passes the copied payload to 0x1401313F0.
  The request-word semantics and remaining message families are unknown.

## Received names and assignments

0x1401313F0 parses repeated records:
[length:u8][assignment:u8][name:length bytes], terminated by a zero length.
It stores records; assignments 1..4 call 0x14012FB10 at 0x140131573.
Other values are retained without that immediate routing call.
0x1401E1FA0 copies the specified bytes and appends a local NUL; string consumers
use UTF-8 character handling. No Windows PID field occurs in this record layout.

0x14012FB10 selects Link Out, Link 2 Out, Link 3 Out or Link 4 Out using
assignment-1 and also searches for Studio in endpoint names.
0x14012FA40 uses WindowsCreateString and RoGetActivationFactory for
Windows.Media.Internal.AudioPolicyConfig. Interface constants are
{AB3D4648-E242-459F-B02F-541C70306324} and
{2A59116D-6C4F-45E0-A74F-707E3FEF9258}.
Calls 0x14012FD07/0x14012FD21, and the alternate-factory path, use vtable
offset 0xC8 with each matching integer ID, flow 0, roles 1 then 0,
and an endpoint HSTRING. Per-process persisted endpoint assignment is the
semantic inference; the precise undocumented interface ABI is not verified.

## Process names and local PID map

0x14012EA70 refreshes the routing map. The session interface queried before
0x14012ED9A is {BFB7FF88-7239-4FC9-8FA2-07C950BE9C6D}.
Its vtable-0x70 result feeds helper 0x14012E990, which calls OpenProcess
(0x14012E9E0), QueryFullProcessImageNameA (0x14012EA05), and CloseHandle.
The same PID is appended to the name's integer vector at 0x14012F1F8..0x14012F20A.
Multiple session PIDs can therefore share one local name key.

Outgoing enumeration 0x14012D1B0 likewise obtains a session PID at 0x14012D3DE,
then calls 0x14012C750. That helper uses OpenProcess at 0x14012C7A0 and
QueryFullProcessImageNameA at 0x14012C7C5 (1024-byte supplied buffer).

Static reconstruction of the naming path at 0x14012D54D..0x14012D748 and the
analogous map path: search for .exe, otherwise .dll; cut at the matching
extension position; retain text after the last backslash. Empty/unmatched
paths can be skipped. The source is an ANSI process image path, not a
window title or a marketing/product display name. Source basename case is
preserved; no lowercase rewrite was observed.

Suffix-presence checks call 0x140053680, whose comparator 0x1400568D0 applies
locale character mapping when decoded characters differ. Extraction-index
checks call 0x140053570 with exact comparator 0x1400567B0 (subtraction at
0x140056899). Consequently differently cased suffixes can pass presence
but fail exact extraction. Last-separator helper 0x1401C7EB0 searches backward.
Case and locale edge cases remain runtime untested.

## Outgoing serialization and scheduling

0x1401315E0 calls fresh enumeration 0x14012D1B0 at 0x14013163A. For each
name it obtains an assignment via 0x14012FEB0, stores length and assignment
at 0x140131868/0x14013186C, and copies the name at 0x140131881.
It starts a remaining-space counter at 507, subtracts selected length+2,
writes a zero terminator at 0x140131934 and returns bytes written+1.
Exact full-list truncation and boundary behavior remain unverified.

Important mismatch: the loop 0x140131825..0x140131857 counts UTF-8 characters,
0x14013185C narrows that count to u8, and the copy uses that many bytes.
The receiver consumes a byte length. Non-ASCII and names beyond 255 characters
therefore need explicit validation; do not describe this as correct UTF-8
byte-length serialization. ANSI-source conversion is also unresolved.

0x14012E54B calls the serializer; 0x14012E55F immediately invokes the writer
callback with its buffer and returned size.
RTTI identifies StudioCompanionUsbCallbacks, including juce::Timer as a base.
Its timer callback is 0x14012E510 (vtable slot at 0x1404051A8).
Constructor 0x14012E670 configures interval 0x1F4 (500 ms) at
0x14012E6F1/0x14012E7B9. Each callback freshly enumerates and sends the list.
Actual callback jitter, delivery cadence and disconnected-state behavior are
unverified. No event-driven change-notification contract was established.

The local assignment reporter 0x14012FEB0 starts with zero, compares the four
Link Out labels plus Studio, returns index+1 on match, and returns zero after
no match (0x14012FFE8, 0x140130066). This explains a reported zero locally;
the incoming zero bypasses routing at 0x14013155B..0x140131578.
Neither observation establishes an unroute command.

Routing-map refreshes are visible at constructor calls 0x140130271 and
0x140131AD1. Incoming-name lookup miss triggers 0x14012FB57, then retries.
A separate periodic map-refresh or notification policy remains unknown.

## Unknowns and validation boundary

The complete protocol is not decoded. Request semantics beyond these paths,
other message families, full buffer/record validation, non-ASCII/codepage
behavior, long-name behavior, duplicate naming, exact undocumented routing ABI,
map-notification policy and reconnection/error behavior remain unresolved.
The first-message timing and whether these envelopes suffice for discovery
on Windows App have not been tested. Assignment zero is not proven unroute.

An [offline codec](offline-codec.md) implements the strict static subset with
synthetic tests. Native Linux runtime and hardware interoperability are untested;
no discovery or routing implementation is claimed. Controlled captures or
device experiments need separate approval.
Keep executables, captures and raw disassembly outside this repository.

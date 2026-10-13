# Protocol session: static evidence and offline implementation

Updated 2026-10-11. This batch adds a working transport-neutral controller over
scripted transfers. It does not add a USB backend, hardware discovery, a daemon,
or remote Windows visibility. Local native routing remains at the separately
validated [private-core checkpoint](native-recovery.md); ordinary apply is disabled.

## Evidence provenance

Focused read-only tracing of BEACNLink.exe 1.0.4 (uninstall version 1.0.4.0),
5,255,096 bytes, SHA256
`74760CE85BEE8620C5BF81F5951FECFE20A04EDF163129361C6890D5481DB64C`.
Tools: pefile 2024.8.26 and Capstone 5.0.7. Preferred image base is
`0x140000000`; VAs below are preferred virtual addresses, not runtime addresses.
File offsets refer to this exact PE. No proprietary binary was executed and no
hardware, capture, USB passthrough or audio mutation occurred in this batch.
Raw traces and source/test archives remain private and outside Git.

| Inspected path | Preferred VA range (end exclusive) | Starting file offset |
| --- | --- | --- |
| Initialization helper | 0x140130710–0x140130B62 | 0x12FB10 |
| Poll callback | 0x140130C60–0x140130E15 | 0x130060 |
| Device constructor | 0x140131020–0x140131332 | 0x130420 |
| Outgoing serializer | 0x1401315E0–0x140131962 | 0x1309E0 |
| Data writer | 0x140130610–0x140130700 | 0x12FA10 |
| Writer callback | 0x140130B70–0x140130C53 | 0x12FF70 |
| Timer callback | 0x14012E510–0x14012E5C9 | 0x12D910 |
| Device destructor | 0x140130ED0–0x14013100B | 0x1302D0 |

Instruction anchors within these ranges map to file offsets by adding their
displacement from the starting VA to the table's starting file offset.
This trace follows already identified paths, rather than repeating the initial audit.

## Observations

### Initialization deadline continuation (2026-10-13)

Focused tracing of the same pinned 1.0.4.0 PE with pefile 2024.8.26 and
Capstone 5.0.7 followed the base USB constructor at preferred VAs
0x1401368D0–0x140136A2B, starting file offset 0x135CD0. CreateFileW occurs
at 0x1401369B1 and WinUsb_Initialize at 0x1401369C8. No pipe timeout-policy
call appears in this inspected path before derived initialization at
0x1401311F5. The 150 ms policies at 0x14013121C/0x14013123F follow it.
Microsoft documents the default bulk PIPE_TRANSFER_TIMEOUT as zero (unlimited)
in its [WinUSB policy reference](https://learn.microsoft.com/en-us/windows-hardware/drivers/usbcon/winusb-functions-for-pipe-policy-modification).
An unlimited initial bulk deadline on this newly initialized Windows handle is
therefore an inference from these paths and the API default, not a measured
firmware requirement or justification for unbounded Linux I/O.

`Session::initialize_with_read_timeout` allows a positive caller-selected A2
deadline only in New state. A0/A1 and later exchanges retain the ordinary
deadline; initialization failure still halts without another exchange.
The selected native publication gate exposes only the fixed five-second
`DIAGNOSTIC_INITIALIZATION_TIMEOUT` through explicit diagnostic preparation.
Its native owner consumes the initial-read allowance before making that call;
later reads remain limited to 500 ms even after a refused or failed first read.
Default preparation and default `initialize()` remain unchanged.

In the VM, one 500 ms A2 read returned libusb -7 TIMEOUT with zero transferred
bytes. Three separate diagnostic attempts subsequently received valid complete
A2 replies in 669, 439 and 584 ms. Each was a fresh authorized attempt with its
own snapshots and confirmed cleanup, not a retry within a failed session.
This establishes responses beyond 500 ms in two runs; it does not establish
the cause of every earlier failure. The 508-byte A2 body remains opaque/redacted.

Initialization writes little-endian A0 then A1 to OUT 03. Calls at
0x140130783/0x1401307BE check success and exactly four transferred bytes.
The IN 83 read at 0x1401307F3 requires exactly 512 bytes and A2 as its first
DWORD (checks at 0x140130801/0x14013080E).

The helper copies response bytes 4–31. From the DWORD at response offset 4,
it stores the low 16 bits and formats the two upper nibbles and the byte at
bits 23–16 through string helpers (0x140130842–0x1401308BC). It examines the
20-byte field at offsets 8–27 until NUL or a non-positive signed byte
(0x140130AAA–0x140130AED); high-bit bytes select a fallback string. Byte 28
is returned to the caller as a nonzero boolean (0x140130B1D–0x140130B24).
The field labels, their authority and the meaning of the boolean are unknown.
The Linux controller retains the entire 508-byte body opaquely and redacts Debug.

### Embedded-NUL name investigation (2026-10-13)

A bounded diagnostic hardware run received three records with assignment 1,
25 name bytes, one embedded NUL and a failed exact match to the 19-byte owned
test name. The owner again reported that Link In was unavailable. The record
names were redacted; their identity, NUL position and suffix meaning were not
observed in this run. Do not call these resolved assignments for the test app.

Focused tracing of the same pinned binary/tools joins the string-copy helper's
unwind fragments at preferred VAs 0x1401E1FA0–0x1401E2031, starting file offset
0x1E13A0. It copies the supplied name length at 0x1401E1FF0 and appends a local
NUL at 0x1401E1FF5. The existing exact UTF-8 comparator at
0x1400567B0–0x1400568C2, starting file offset 0x55BB0, returns equality when
both decoded characters are zero at 0x14005689E–0x1400568AC. These observations
support investigating first-NUL comparison, not automatically adding aliases or
reinterpreting suffix bytes as a PID, source kind or routing instruction.

The periodic example now reports first-NUL position, opaque tail byte count and
whether the prefix equals the exact owned advertised name. All names remain
redacted; codec bytes and planner bindings remain unchanged. Its first passive
40-cycle run received no records, so the prefix hypothesis is still untested.
No routing or matching-policy change has been made from the observed shape alone.

The subsequent owner-ready prefix run confirmed eight records with first NUL
at byte 19, five opaque tail bytes and an exact prefix match to the owned name;
all carried assignment 1. The compatibility implementation therefore exposes
`ReceivedNamePolicy::{ExactBytes, WindowsCString}`. `for_planning` produces a
separate record preserving the assignment byte and using the selected name
interpretation. It neither modifies wire decoding/raw records nor case-folds,
guesses suffix meanings or finds applications by loose prefix. Empty/non-ASCII
prefixes still fail planner name validation; unknown names, duplicate conflicts,
opaque assignment values and stale streams retain their existing outcomes.
Normalized collisions are passed together to the existing planner, which
coalesces identical values or blocks conflicting values. Default policy stays
ExactBytes; this is an explicit interoperability choice at the planning boundary.

Four authored-fixture tests prove raw/default preservation, hardware-shaped
resolution with current-generation guards, collision handling, unknown/opaque
values and unsupported/stale refusal. The fixture tail is invented, not captured.
The native hardware mode subsequently resolved a real assignment-1 record to
one owned-stream proposal; it applied no route. See publication docs for scope.

The constructor calls alternate setting 0, then 1 (0x140131190/0x1401311D8),
resets IN 83 (0x1401311E4), and invokes initialization (0x1401311F5).
It does not test the helper's return before setting policies and starting its
callbacks/worker. Policy 3 receives DWORD 150 on IN/OUT, and policy 2 receives
one-byte 1 on OUT/IN (0x14013121C–0x140131285); these returns are also ignored.
Microsoft documents policy 3 as PIPE_TRANSFER_TIMEOUT in milliseconds and
policy 2 as AUTO_CLEAR_STALL for bulk/interrupt IN pipes. The observed OUT call
does not establish validity or effectiveness. These calls do not prove actual
descriptor types, Linux setup requirements, or the initialization timeout; the
policy calls occur after initialization.
See [WinUSB policy definitions](https://learn.microsoft.com/en-us/windows-hardware/drivers/usbcon/winusb-functions-for-pipe-policy-modification)
and [alternate-setting API](https://learn.microsoft.com/en-us/windows/win32/api/winusb/nf-winusb-winusb_setcurrentalternatesetting).

Polling writes AC010000 to OUT 03, requires success/count 4, then reads at most
512 bytes from IN 83. It checks actual count >=4, high byte AC and the rejected
sentinel ACFFFFFF (0x140130CD3–0x140130D41). The low 16 bits supply N.
For N=1..508 it copies exactly N bytes from response+4 at 0x140130D7F and
calls the existing assignment parser at 0x140130DD3. **It does not compare
actual count against 4+N before this copy.** Zero and out-of-range lengths skip
the parser; the path still writes 10 to a callback byte at 0x140130DDC.
Extra bytes beyond declared N are not passed to the parser. Bits 23–16 are not
checked by this reader; their semantics are unknown.

Initialization, polling and outgoing writes use the same critical section
at the main object's +0x3E0. The writer callbacks build AC000000|N, but discard
the BOOL/count returned by WinUsb_WritePipe at 0x1401306D4/0x140130C2F.
This is weaker completion handling than initialization and polling.

The serializer starts its remaining counter at 507. At
0x14013185F–0x140131862 it compares the selected u8 name length alone with
remaining space. It writes the length/assignment/name, then subtracts length+2
at 0x14013188A–0x140131891. A name exceeding remaining space ends the list
through 0x1401318EC–0x140131934, rather than skipping that name and continuing.
This augments the already recorded character-count/byte-copy mismatch.

The 500 ms publication timer is established in [protocol.md](protocol.md).
Its callback examines/decrements a counter and chooses state bytes 1/2;
polling sets a counter to 10. The worker's scheduling path was inspected only
in split unwind fragments; the exact polling cadence is not established.
The destructor aborts IN 83 at 0x140130F1E, then calls the worker stop helper
with argument 1000 at 0x140130F24–0x140130F2D.

## Derivations, hypotheses and unknowns

### Focused startup/worker continuation (2026-10-12)

The same pinned binary and pefile/Capstone versions above were traced read-only;
no proprietary executable, REA, capture or device I/O was run for this analysis.
The constructor's alternate-zero call at 0x140131190 is followed by timeGetTime
at 0x140131196. It passes that tick plus one to 0x1401E2500 at 0x1401311CA–1CD,
before alternate one (0x1401311D8) and IN 83 reset (0x1401311E4).
The wait helper at 0x1401E2500–0x1401E25A5, file offset 0x1E1900, loops until
timeGetTime reaches the supplied tick. This establishes a next-tick wait rather
than a guessed long device-settle timeout. Scheduler granularity/jitter is unknown.

Microsoft documents [WinUsb_ResetPipe](https://learn.microsoft.com/en-us/windows/win32/api/winusb/nf-winusb-winusb_resetpipe)
as clearing pipe stall state and resetting the data toggle. The tested libusb
1.0.29 [Linux adapter](https://github.com/libusb/libusb/blob/v1.0.29/libusb/os/linux_usbfs.c#L1470)
implements clear-halt through USBFS; current Linux
[endpoint handling](https://github.com/torvalds/linux/blob/master/drivers/usb/core/message.c)
sends endpoint CLEAR_FEATURE and resets endpoint state after success. This supports
a narrowly scoped Linux preparation experiment; it does not prove VM/firmware
equivalence or that omitted preparation caused the earlier unknown A2 failure.

The thread vtable slot at 0x140405408 points to 0x140130E20. Contiguous tracing
joins unwind fragments 0x140130E20–E39, E39–EB6 and EB6–EBC; the joined range
starts at file offset 0x130220. The loop checks its stop byte at +0x1ED,
atomically sets the callback's pending flag at 0x140130E69, invokes scheduling
helper 0x1401EC690 at 0x140130E77, then invokes wait helper 0x1401E1420 at
0x140130E96. Its constant at VA 0x140426C58 / file offset 0x425658 is double
500.0. The scheduling path is asynchronous; this is not measured poll cadence.
Initialization precedes thread startup at 0x1401312DA. The inspected loop contains
no new A0/A1/A2 call or reconnect handling; discovery/reconstruction elsewhere
remains untraced. The worker cannot explain a read failure before its startup.

Successful versus failed native journals use the same descriptor-cache hash,
selected vendor endpoints and bulk deadlines. Both restored their device/profile
and owned private processes. The failed run stopped at A2 before list/periodic
policy activation; its native error code was not retained. Do not infer timeout,
stall, device disconnect, or a proven fix from those journals.

An explicit `Startup::ObservedWindows` option now performs alternate zero, a
conservative 1 ms sleep, alternate one and vendor IN 83 clear-halt before any
handshake bulk transfer. It revalidates the retained identity/audio profile at
each boundary, performs no post-error recovery, and leaves default setup unchanged.
See [native publication](native-usb-publication.md) for checks and test scope.

- A short successful poll transfer can make the Windows copy consume bytes
  beyond the received portion of its reused buffer. This is a static derivation,
  not a reproduced device fault or exploit.
- Accepting a name when less than length+2 space remains can underflow the
  unsigned remaining counter. This is a boundary derivation; the precise
  resulting runtime behavior has not been tested.
- A version-like string and identity-like text are hypotheses for the A2 fields.
  The implementation does not label them firmware version, serial or capability.
- The timer/poll counter relationship suggests liveness tracking. A five-second
  disconnect rule would be a hypothesis, so the controller does not implement it.
- Hardware descriptors, first-message timing, alternate-setting/audio coexistence,
  remote list acceptance, Unicode, assignment-zero actions and reconnect behavior
  still need reproducible hardware evidence.

## Implementation and deliberate policies

### Exact rejected-marker observation policy (2026-10-12)

Focused read-only tracing of the same binary confirms `cmp ecx, ACFFFFFF` at
preferred VA 0x140130D35 / file offset 0x130135, followed by a branch at
0x140130D3B / file offset 0x13013B to critical-section release/return at
0x140130DE3 / file offset 0x1301E3. The marker bypasses assignment parsing at
0x140130DD3 and the counter write at 0x140130DDC. The callback returns void and
does not stop the independent worker/timer, close/reinitialize the device or
resubmit the failed callback. This is observed control flow, not a decoded
firmware meaning or proof of a healthy connection. The read must have succeeded
with at least four bytes before that branch. Native errors are a different case.

The exact codec and `decode_poll_reply` retain strict rejection by default.
`PollPolicy::ReportRejectedMarker` is explicit and returns
`PollReply::RejectedMarker { ignored_tail }` only for the exact marker in a
successful bounded 4..512-byte reply. It carries no assignments, inferred empty
inventory, assignment-zero action, counter update or heartbeat semantics.
Oversize/short/nearby unsupported words and malformed records still fail. Session
and bounded selected-USB gate must both opt in; a policy mismatch blocks the next
submission. Every marker consumes one scheduled cycle and another publication
still requires a fresh pinned inventory. There is no retry of uncertain I/O or
peer recovery. The default handshake, one-shot and bounded-strict paths stay intact.

`src/session.rs` owns one `Transport` and serializes exchanges through exclusive
mutable access. `Transport` implementors must report actual byte counts, honor
the caller's nonzero finite per-operation timeout and perform no hidden retry or
device fallback. This interface contract has only been exercised with scripted
transfers; it is not proof of OS timeout or cancellation behavior.

`initialize()` performs A0 write -> A1 write -> exact 512-byte A2 read.
Only successful initialization permits `publish()` or `poll()`. Requiring this
gate is a deliberate policy, unlike the inspected constructor's ignored result.
`HandshakeReply::body()` exposes opaque bytes to an explicit caller; avoid
logging them because real metadata could contain private identity information.

`publish()` reuses `encode_records` and `encode_frame`. It validates the entire
printable-ASCII list before writing, with 508-byte payload/512-byte transfer
limits. It never silently truncates, chunks, guesses assignments or substitutes
desired routes for observed local assignments. A full write is only transport
completion: the traced exchange has no established remote publication acknowledgment.

`poll()` performs AC010000 write -> bounded read. `decode_poll_reply()` handles
AC000000 as `NoPayload`, distinct from a terminated empty list or assignment
zero. It requires 4+N received bytes and parses only that declared payload through
the existing record codec. It reports the ignored tail length and preserves
raw names/all assignment bytes. Unsupported header bits, N>508 and malformed
records fail closed. The generic exact-sized `decode_frame()` is unchanged.

States are New -> Ready -> Halted/Closed. Any transfer, count or peer reply error
halts further exchanges, even `NoTransfer`: earlier writes may already have
changed peer state. No automatic retry, reconnect or reuse of the same controller
is permitted. A local encoding refusal leaves Ready and sends nothing.
`close()` only prevents future calls; it invents no shutdown packet and cannot
cancel OS I/O. Transport disposal is the caller's responsibility.

Returned records remain input to the existing planner/lifecycle owners.
`NoPayload` is not an instruction to replace the inventory with an empty one or
remove routes. The controller never calls PipeWire or bypasses guarded preparation,
receipts or explicit execution authority. No automatic publisher/scheduler exists.

## Validation and next milestone

Identical canonical code tree built offline on Windows and Ubuntu 26.04.1 with
Rust/Cargo 1.88.0. All 25 focused integration tests passed on both: 10 session,
8 existing codec/CLI and 7 planner. The synthetic full exchange advertises two
explicit application names, ingests a padded reply, resolves one assignment to
Link 2 using the existing inventory/planner and leaves assignment zero opaque.
It applies no audio routes.

Boundary tests cover all short prefixes of a declared reply even when a scripted
transport fills bytes beyond its reported count, startup failures at each step,
partial/impossible write counts, rejected/unsupported headers, malformed records,
raw non-ASCII/opaque input, exact-capacity publication, oversize refusal before
I/O, metadata redaction and no exchanges after failure/close. Windows formatting
and diff checks passed; Linux rustfmt was not run. Previous native routing and
recovery tests were preserved, not rerun for this protocol-only change.

Tested-code SHA256:
`60da7424fffafc6cfeb9149de1da6d35346ffd2c1e6856994675cd7c4c994847`.
Scope: src/native/tests/tests_python/tests_native/tools/examples/fixtures plus
Cargo manifests/build.rs; canonical LF, sorted path + NUL + per-file SHA256 + LF,
then aggregate SHA256. Source archive SHA256:
`6b10c44286eae11f62017d30e3172d721b9a1b67fdea89c1c05d66d089391a18`.
Private Linux validation journal SHA256:
`5e99034e48a446210ad96852bcc248a1431f056250fb41d2d5a3a2506e050611`.
The archive precedes documentation updates; its executable/test code is identical.
Guest free disk after this lightweight batch was 23,367,229,440 bytes; the owner
was notified it remains below 25 GB. No packages/toolchains were installed.

The subsequent [descriptor preparation batch](usb-descriptor-validation.md) adds
a bounded offline parser/cache probe and a VM-first hardware procedure. Actual
descriptors are still unverified and no USB transport exists. The first interoperability acceptance is one real Linux
application appearing in Windows App and receiving a routing assignment with
verified audio. USB attachment, captures and physical routing require separately
scoped authorization; isolated-core approval does not cover them. Offline tests
cannot establish that the candidate exchanges are sufficient for discovery.

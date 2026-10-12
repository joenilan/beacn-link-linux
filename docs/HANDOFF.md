# BEACN Link Linux — continuation handoff
Updated 2026-10-12. Resume development; do not restart the investigation.

## Objective
Keep the streaming PC on Windows with BEACN App / Mix Create / Studio.
Replace the gaming PC's Windows Link application with a Linux implementation,
possibly alongside Windows dual boot for unsupported games. The eventual result
must expose Linux applications to the streaming PC's Windows App and accept its
routing assignments. A generic local mixer or USB audio driver is not enough.

## Incoming verified checkpoint (historical)
Repository: https://github.com/joenilan/beacn-link-linux (private), main.
Windows checkout: E:\git\beacn-link-linux.
Latest implementation: 39d36a35377df257fd0cdbd95a7577a8bc61e80b,
“Add PipeWire graph snapshots and link receipts [skip ci]”.
Parent: 43c9453b4d02d62bf471af7ba473a79b4c3d0a0e.
The owner pushed it; GitHub main and all 13 intended files were independently
verified. Local working-tree cleanliness after the owner's push was not rechecked.
This handoff's later documentation commit does not change that tested code.

Implemented:
- Dependency-free Rust protocol framing/name-assignment codec and CLI.
- Pure provider-neutral routing planner.
- Read-only PipeWire application inventory and explicit wire-name bindings.
- Complete-inventory lifecycle comparison and generation guards.
- Guarded route preparation and rollback descriptions.
- Read-only node/port/link projection, preserving foreign links.
- Pure ordered acknowledgment/receipt journal with actual link identity contracts.

Relevant owners: src/executor.rs, src/graph_snapshot.rs, src/receipts.rs,
tools/pipewire_inventory.py, tools/pipewire_graph.py, existing planner/lifecycle.
Do not build parallel identity or ownership systems.

## What was actually validated
At the implementation checkpoint:
- Identical source: Windows and Ubuntu offline build; 36 Rust integration tests
  and 15 Python tests on each. Python tests invoked the actual Rust CLI.
- Windows formatting check passed. Linux rustfmt was unavailable; do not claim it ran.
- Real read-only graph: two audio nodes, six ports, two foreign links, one live
  stream. Explicit preparation produced zero changes for an already-correct route.
  A wrong destination serial was refused with ReusedObject.
- Earlier real Firefox playback discovery matched native metadata.
- Earlier isolated private PipeWire core: one owned synthetic stereo stream moved
  A -> disconnected -> B, matching planner identities. Owned objects/processes and
  temporary data were removed; the desktop's immediate filtered baseline matched.

Important failed experiment:
Creating temporary sinks on the desktop/shared PipeWire core caused fallback
auto_null/default behavior to change. The run aborted before routing links.
Cleanup restored logical routes/defaults, with new object identities and a
suspended-format difference documented. Do not repeat this design.
The successful routing test used an explicitly selected private core/socket/runtime,
without desktop device discovery, fallback handling, Pulse or a session manager.
It retained standard access controls.

These results DO NOT prove audible output, hardware transport, remote Windows
visibility, native production executor operation or end-to-end interoperability.

## Next bounded implementation batch

**Completed continuation:** the optional `native-pipewire` feature now provides
the native executor described below. Read [native-executor.md](native-executor.md)
for source owners, build dependencies, exact runtime outcomes and limits.
The owner explicitly authorized the disposable-core VM test during this
continuation. That scope did not include desktop or hardware routing.
The actual Rust/C backend moved one owned stereo stream A -> B, stopped after
three acknowledged steps of a controlled B -> A failure, then restored B with
new native receipts. Owned cleanup and the desktop settings/state comparison
passed. Ordinary `link-protocol apply` remains disabled.

**Completed fault/recovery continuation:** explicit rollback continuation now
retains the acknowledged recovery prefix and revalidates before resuming.
Actual private-core tests passed for native factory rejection, deliberately
withheld acknowledgment delivery, interrupted recovery and rejected restoration.
Request counters proved no automatic retries or repeated acknowledged operations.
A 32-iteration transient-client smoke test preserved the current receipts and
issued no mutation requests. See [native-recovery.md](native-recovery.md).
The owner continued the existing isolated-VM authorization; physical/desktop
routing stayed outside scope.

**Completed protocol continuation:** focused static tracing established A2 body
handling, ignored constructor/writer results, the short-poll length-check gap and
serializer prefix/capacity behavior. `src/session.rs` now sequences A0/A1/A2,
publishes validated complete lists and decodes bounded assignment replies over
a caller-provided transport. Failures halt without retry; metadata remains opaque.
All 25 focused session/codec/planner tests pass on identical Windows/Linux code.
No USB backend or hardware test exists. Read [protocol-session.md](protocol-session.md)
for exact provenance, deliberate policies and validation hashes.

**Completed descriptor preparation:** `src/usb_descriptors.rs` and the offline
`inspect-usb-descriptors` CLI now validate a bounded cached descriptor stream.
`tools/usb_descriptor_probe.py` reads one explicitly selected sysfs cache and
reports descriptor candidates separately from active alternates/driver ownership.
All 25 focused Rust tests pass on identical Windows/Linux code; 5 new Python
probe tests pass on Linux with synthetic files and the actual CLI. There is still
no hardware descriptor evidence or USB transport. Read
[usb-descriptor-validation.md](usb-descriptor-validation.md) for exact limits/hashes
and a reviewable descriptor-only physical test. The owner wants the existing VM
first, then native Linux on the gaming PC; that preference does not authorize
passthrough or device I/O.

**Authorized hardware preflight:** the owner explicitly authorized the descriptor-only
VM procedure. Fresh private host/guest baselines were saved; repeated host defaults,
endpoint formats/volumes/mutes/session settings and persisted audio-policy registry
hashes matched. One physical target is present on Windows. The VM has no USB
controller/root hubs (all three VM controller flags FALSE); no handoff or hardware
probe has run. The owner must add/enable a VMware USB controller and manage any
needed guest restart. The owner subsequently enabled USB 1.1 and restarted the
guest. Read-only checks confirm one UHCI controller (class 0x0c0300, uhci_hcd)
and one 12 Mb/s root hub; still no BEACN target is present in the guest. Fresh
post-controller guest and Windows baselines are saved. The next action is owner-managed
Studio connection through VMware's Removable Devices menu, followed by the one
bounded cache probe and owner-managed return/compare. USB 1.1 is only a candidate
for this descriptor observation; device fallback is unverified and it does not
qualify later audio operation. The owner then attempted connection and received
VMware's ideal-controller/fallback warning. A read-only guest query timed out;
Windows still reported the physical target and three USB functions. No cached
descriptors were read. Host defaults/formats/volumes/mutes and per-user audio/policy
trees match the refreshed baseline, but a Chatterino session appeared and 17
MMDevices registry values changed (16 binary fields plus one endpoint-reference
string). Their cause/semantics are unresolved; do not call this an exact unchanged
baseline or blindly rewrite registry values. Return/keep Studio on Windows and
resolve the USB compatibility setting before another handoff. Continue the
existing descriptor-only authorization. Do not ask for that
same test permission again. No protocol writes, capture or routing is authorized.
See the preflight section in [usb-descriptor-validation.md](usb-descriptor-validation.md).

**Completed physical descriptor observation:** one attached Studio validated at
configuration 1, vendor interface 3, alternate 1, bulk OUT 03 / IN 83 with 512-byte
max packets. Current alternate was 0, no driver bound on interface 3; audio interfaces
0–2 were bound to snd-usb-audio. Cached speed reported 480 Mb/s. The owner returned
the device; guest target count is zero and Windows functions are OK. Defaults,
formats, volumes/mutes and logical guest settings match, but object identities and
some Windows/guest persistent state differ. Do not claim an exact unchanged baseline.
Read [usb-descriptor-observation.md](usb-descriptor-observation.md) for hashes,
the complete comparison and limits. No protocol/control writes or alternate change
were performed. Descriptor-only permission is completed; it does not authorize
a handshake. Preserve full state-file contents in future baselines, not only hashes.

**Completed native USB implementation:** `native-usb` now builds a selected-fd
libusb adapter with discovery disabled. `usb_handshake` permits only A0/A1/A2,
revalidates exact profile/ownership/audio state, never retries and reports claim
release/restoration. Publication/polling remain refused. Windows default build and
25 focused tests passed; Linux default/optional builds, 33 focused tests, 1 ABI unit
test and 7 runner tests passed. Final runner tests also pass on Windows. Saved
physical descriptors pass the strict profile with no new device access.
Read [native-usb-handshake.md](native-usb-handshake.md) for exact hashes, limitations
and a prepared one-shot hardware test with full persistent-file backups. The normal
CLI remains disabled. No hardware handshake or new passthrough occurred.

**Current owner authorization:** after reviewing the completed USB implementation,
the owner explicitly authorized the hardware and VM operations needed to get the
replacement working, including temporary audio interruptions. Continue necessary
implementation and bounded hardware tests without repeated routine approval
requests. Preserve settings snapshots and unrelated work. Ask before database
edits or deleting applications/system files. This authorization comes from the
current owner conversation, not from the earlier descriptor-only permission.

**First authorized handshake attempt:** the owner connected Studio after fresh
full-content baselines. Setup stopped on revalidation before any A0/A1/A2 bulk
exchange. Claim release and device baseline verification succeeded; attached versus
post-child guest graph/state files match exactly. The adapter's likely mismatch
was Linux's sysfs `usbfs` userspace-claim driver versus libusb's inactive-kernel
report. It now accepts that pair only after its retained handle acknowledges the
claim. The corrected native build, two unit tests and eight handshake tests passed
before the corrected hardware attempt below. Read [native-usb-handshake.md](native-usb-handshake.md)
for provenance and distinction between observed failure and inferred cause.

**Completed first return comparison:** guest target is absent and Windows USB
functions report OK. Windows defaults/formats/volumes/mutes match; guest state-file
contents/modes match exactly. Guest graph properties/parameters and metadata match
after accounting for one recreated node and four ports. Windows sessions retain
the same instances with state differences, and MMDevices has 17 changed values
(16 binary, one string); user audio/policy registry trees match. Preserve all full
contents/differences; no blind registry rewrite or exact unchanged-baseline claim.

**Completed corrected hardware handshake:** the owner connected Studio under the
standing hardware/VM authorization. With a 482-second-old fresh baseline, the
corrected native executable completed exact four-byte A0/A1 writes and a 512-byte
A2 reply, released interface 3 and verified the original device baseline. No
timeout/retry occurred; the child exited 0. Attached-versus-post-child graph and
state files match exactly. Read-only sysfs confirmed all alternates zero, audio
drivers preserved and no vendor-interface driver. After owner return, guest target
is absent and all Windows USB functions report OK. Windows defaults, endpoint
state/format/volume/mute and sessions match; guest state files match exactly. Guest
graph settings match after one recreated node/four ports; MMDevices has the same
kind of 17 recorded value differences, with user audio/policy trees matching.
No persistence was rewritten. See [native-usb-handshake.md](native-usb-handshake.md)
for exact hashes and limits. App publication/polling/audio remain untested.

**Next work:** implement a separately enabled, bounded app-list publication and
assignment-poll mode over the existing session/codec and native USB owner. Retain
the default handshake-only gate, exact device/claim guards, no automatic retry and
explicit cleanup. Bind an explicit wire name through the existing inventory and
lifecycle owners; keep assignment zero opaque and avoid route application during
this first exchange. Build/test the exact tree, then use fresh full-content
baselines for a controlled VM test under existing owner authorization. Check
Windows App visibility with the owner; native transfer success alone does not
establish it. Preserve both earlier evidence directories.
A receiving agent should use this authorization context alongside the current
owner instructions and applicable execution environment.
The first acceptance remains one real Linux app visible/routable in Windows App
with verified audio. Local gaps also remain across native core
disconnect/restart, long-lived registry cleanup and capacity. Withheld acknowledgment
delivery is not proof of packet-loss handling. Keep desktop apply disabled.
The original batch below records the first native implementation.

Implement native execution and acknowledgment capture for the existing guarded
plan and receipt journal, first restricted to an explicitly selected disposable
private PipeWire core. Keep normal desktop CLI apply disabled by default.

1. Inspect current source/contracts and private-core evidence. Choose the smallest
   maintainable native PipeWire interface; document any needed dependency.
2. Subscribe to relevant registry/core events and establish the intended server
   instance. Revalidate node/port/link IDs plus serials before each operation.
   A core cookie and two snapshots do not eliminate races.
3. Execute only planned changes. Record success only after observed native
   acknowledgment and exact endpoint/ownership verification.
4. Maintain actual created-link receipts. Owner tags alone never authorize deletion.
   Preserve foreign links, desktop fallback/defaults and unrelated applications.
5. On failure or unknown outcome, stop progression. Reconcile the pending effect
   before retry or rollback. Roll back only the known successful prefix, using fresh
   identities; never delete an ambiguous/reused object.
6. Under the owner's permitted isolated test scope, snapshot first, exercise
   forward A-to-B and controlled partial failure/recovery with synthetic audio,
   then verify cleanup and the untouched desktop baseline.
7. Update canonical docs with exact observed outcomes, limits and next step.
   Build the final code tree; run focused checks and publish only as authorized.

Do not spend another batch merely rearranging docs or duplicating tests. Deliver
the native integration slice while retaining the safety properties above.

## Longer roadmap / unresolved protocol work
After native local routing is proven:
- USB/interface discovery and coexistence with Linux audio devices.
- Full handshake, disconnect/reconnect and transport behavior.
- Remote application-list publication and assignment ingestion.
- Explicit assignment-zero semantics and Unicode/length behavior.
- One Linux application visible in Windows App, routed there, with verified audio.
- Multi-app/four-stereo-pair lifecycle, reconnects, rate changes and error recovery.

Hardware captures, USB passthrough and driver/firmware changes need separately
scoped authorization. Physical cached descriptors are validated; hardware protocol
execution and audio interoperability remain unverified.

## Static findings worth preserving
Installed Windows Link studied: 1.0.4.0, native AMD64 PE, JUCE 7.0.6.
Protocol docs contain binary SHA256 and address provenance.
USB VID 33AE / PID 4003; vendor WinUSB MI03 alongside the audio interface.
Candidate bulk OUT 03 / IN 83, little-endian AC envelope and bounded records.
Assignments 1–4 map to Link outputs; zero remains opaque.
The Windows program uses per-process audio-policy routing, so restarting it is
not evidence that prior routing is undone.
The app-list refresh timer was statically reconstructed as 500 ms.
Preserve the observed character-count/byte-copy mismatch rather than “fixing”
wire encoding without runtime evidence. Read docs/protocol.md for actual offsets,
confidence and unknowns.

## Environment and private evidence
Windows has the checkout and existing dedicated SSH setup for the Ubuntu guest.
VMware VM is under D:\VMs\BEACN-Link-Linux.
Guest was upgraded by the owner from Ubuntu 24.04.5 to 26.04.1; current evidence
uses Ubuntu 26.04.1, PipeWire 1.6.2, Rust/Cargo 1.88.0 and Python 3.14.4.
VM configuration at the original graph checkpoint: 2 vCPU, 4 GiB RAM,
40 GiB growable virtual disk, NAT networking; no USB, physical sound passthrough
or shared folders. The owner subsequently enabled USB 1.1 for the completed
descriptor test; current device ownership must be checked before new execution.
Use SSH for commands. Screen lock does not stop SSH; VM suspension/host sleep does.

No GitHub credentials were installed in the guest. Sources were archived, transferred
using the existing verified SSH connection, hash-checked, and built in isolated dirs.
The original graph checkpoint source is under the user's home in
beacn-link-graph-20261011T204629Z/source.
The latest tested USB source is in the existing guest user's home under
beacn-link-usb-claim-20261012T134712Z/source. Its corrected optional handshake example
is built and hardware initialization has passed. The first runtime attempt used
the preserved original source under
beacn-link-usb-final-20261012T132115Z/source. Library-version/cache-check modes remain
offline. Latest free disk is about 22.7 GB; continue lightweight builds only.
Locate the existing isolated Rust homes from local artifacts rather than installing
another toolchain. Resolve current IP/user/known_hosts/key references locally;
keep those details and raw evidence out of Git and logs.
Use StrictHostKeyChecking=yes, BatchMode=yes and the dedicated identity.
Do not read private key contents or weaken permissions.

Private Windows evidence beside the existing VM host-SSH setup includes:
- controlled-virtual-routing-result.json
- virtual-routing-baseline.json
- virtual-routing-settled-check.json
- virtual-routing-settled-comparison.json
- private-core-routing-result.json
- private-core-routing-verification.json
plus source manifests/build logs. Verify actual filenames before using them.
The earlier graph checkpoint free-space result was roughly 24 GB decimal, below the owner's
25 GB heavy-build threshold. These were lightweight single-job builds, not a
permission to launch large builds. Measure current space/resource use first.

REA checkout: E:\git\rea, historical main
b2da21abe11ac43a4cc24b233bd52ce10995e4a1. It was cloned but not installed/run.
No need to run it to continue the current native routing slice.

## Lightweight validation commands
Run from the repository with the existing toolchain selected:
- cargo build --offline --locked -j 1
- cargo test --offline --locked -j 1
- cargo fmt --check (where installed)
- Set BEACN_TEST_PLANNER_BIN to the absolute built link-protocol executable.
- python -m unittest discover -s tests_python
On Windows the binary ends in .exe; on Linux it does not.
Use the repository's exact scripts/contracts where they supersede this summary.
No GitHub CI/workflow is requested.

## Why work is being handed over
The prior assistant's Windows task-start service repeatedly returned
AppServerBackendRequestError, including for read-only verification. The PC still
reported connected. This does not establish a Deadlink, SSH, VM or project bug.
Earlier publication checks also failed to recognize relayed approvals; the owner
then committed/pushed successfully. Publication is complete, not pending.
Use the new agent's supported direct environment and current owner instructions.
Do not repair/reinstall Deadlink, reset credentials or repeat approval loops
because of that historical orchestration failure.

## First actions for the receiving agent
1. Read AGENTS.md and this handoff, inspect git status, then fetch main.
   Fast-forward only if safe; preserve any local changes.
2. Reconcile against implementation commit 39d36a3 and read canonical docs.
3. Verify existing Windows/VM access read-only, toolchain and current disk space.
4. Follow the current next-work note: the one-shot USB implementation is complete,
   and the owner authorized the necessary hardware/VM continuation. The original
   native fault/recovery batch is implemented. Keep progress factual and report
   implemented / built / runtime-tested / pushed separately.
5. Leave Jawjack, Haiwire, AI benchmarks and other agents' processes alone.

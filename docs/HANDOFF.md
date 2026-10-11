# BEACN Link Linux — continuation handoff
Updated 2026-10-11. Resume development; do not restart the investigation.

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

**Next work:** move toward descriptor evidence and a bounded hardware-validation
procedure before implementing a Linux transport. The first acceptance remains
one real Linux app visible/routable in Windows App with verified audio. Hardware
attachment/captures/routing need separately scoped approval; current isolated-VM
authorization does not cover them. Local gaps also remain across native core
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
scoped authorization; there has been no hardware runtime validation.

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
VM configuration at last inspection: 2 vCPU, 4 GiB RAM, 40 GiB growable virtual disk,
NAT networking; no USB, physical sound passthrough or shared folders.
Use SSH for commands. Screen lock does not stop SSH; VM suspension/host sleep does.

No GitHub credentials were installed in the guest. Sources were archived, transferred
using the existing verified SSH connection, hash-checked, and built in isolated dirs.
Latest guest source is under the user's home in
beacn-link-graph-20261011T204629Z/source.
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
The last guest free-space result was roughly 24 GB decimal, below the owner's
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
4. Continue the native fault/recovery work in the completed-continuation note.
   The original native batch is implemented. Keep progress factual and report
   implemented / built / runtime-tested / pushed separately.
5. Leave Jawjack, Haiwire, AI benchmarks and other agents' processes alone.

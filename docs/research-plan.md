# Phased interoperability plan

Goal: keep BEACN App/Mix Create on Windows and replace Link on the Linux gaming PC.
Existing beacn-on-linux work reportedly exposes four stereo Link pairs; that
prior research is context, not a locally verified replacement for discovery.

1. Static tracing: follow the GUID/VID/PID anchor, WinUSB calls and audio-session
   discovery RTTI. [Static protocol evidence](protocol.md) now covers envelopes,
   name/assignment records, PID provenance and the 500 ms list timer. Resolve
   the remaining ABI, Unicode/codepage, bounds and reconnect uncertainties.
   Exit: defensible candidate messages/transport and explicit unknowns.
2. Separately approved controlled captures: agree scope, devices, routing actions,
   consent and local artifact handling first. No capture is authorized now.
   Exit: sanitized protocol specification corroborated by repeatable evidence.
3. Minimal Linux interoperability: candidate PipeWire application metadata and
   audio routing must be tested, not assumed. Exit: one real Linux app appears
   in Windows App and can be routed from Windows, with expected audible output.
4. Robustness: test multiple apps/pairs, naming, app lifecycle, reconnects,
   restarts, rate changes and errors. Preserve normal streaming-PC behavior.

A generic USB/audio Linux port alone does not prove remote app visibility.
Some pipe IDs and framing/record fields are statically reconstructed; complete
handshake semantics, assignment-zero actions and runtime compatibility remain unknown.
Preserve the sender character-count/byte-copy mismatch until separately validated.
An [offline codec](offline-codec.md) now validates a strict static subset using
synthetic tests. A [pure routing planner](offline-planner.md) adds deterministic
proposals and explicit failure outcomes over synthetic provider-neutral inventory.
The offline crate now has [verified Linux build, tests and synthetic runtime](linux-validation.md).
A bounded [read-only PipeWire adapter](pipewire-inventory.md) now projects
explicit application identities/wire bindings into the pure planner.
One real Firefox playback node was subsequently observed and resolved from a
single populated snapshot. This proves local metadata discovery for that case;
remote Windows visibility and hardware interoperability remain unverified.
Isolated synthetic-stream routing was subsequently verified as described below.
An [offline lifecycle tracker](offline-lifecycle.md) now compares successive
complete inventories and guards saved plan generations. Closure/reopen, changed
stream sets, node-ID reuse and stale snapshots are checked synthetically;
no OS lifecycle or routing behavior is inferred from those tests.
The [controlled virtual routing evidence](controlled-virtual-routing.md) distinguishes
two outcomes: a shared-core attempt aborted before linking and recreated the
fallback with new identities; a private-core attempt moved the same owned stereo
stream A -> disconnected -> B, matched planner identities, and passed cleanup
with an exact match of the desktop's immediate filtered baseline.

An [offline routing preparation contract](offline-executor.md) now connects pure
proposals to explicitly configured stereo destinations and produces guarded
link-change and rollback descriptions. It rejects stale generations/identities,
ambiguous stereo mappings and removal of unowned links; all checks remain pure.
The [read-only graph provider and pure receipt journal](pipewire-graph.md)
now reuse existing identities and executor types. Identical-source Windows/Linux
tests passed, and one real already-correct stereo route passed read-only
preparation with foreign links preserved and a wrong destination serial refused.
The next local gap is the native application backend: actual create receipts and
acknowledgments, fresh per-operation checks and guarded rollback that preserve
desktop fallback and foreign routes. Actual CLI route application remains disabled. The private
harness is separate evidence and was not reused as a production executor. USB discovery,
handshake/reconnect semantics, remote assignment handling and Windows App
visibility still need separately scoped evidence and implementation. No further
audio mutation or hardware test was performed in the preparation-contract slice.
Keep raw evidence private.

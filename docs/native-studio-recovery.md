# Native Studio Pause and explicit reconnect qualification

Updated 2026-10-15 UTC. The prior first-pair native playback acceptance remains
in [native-playback-startup.md](native-playback-startup.md). This next check is
built and passes virtual and owner-attended physical checks. The owner heard
both runs and reported needing to reassign mpv after the first success. Assignment
persistence remains unfinished. This does not enable ordinary desktop routing,
managed USB or durable hotplug.

## Bounded sequence

The explicit native Studio example accepts
`--authorize-bounded-native-studio-recovery-gui`, restricted to pair 1 and two
sessions, each retaining the existing 40-cycle budget and 4096-check PCM fuse.
The GUI sequence has an 80-second deadline inside the 90-second supervisor bound.
It selects one real mpv application through the existing widgets, enables
automatic connection, waits for a verified first-pair assignment and holds
playback for four seconds. It then clicks Pause, verifies the existing route and
transport cleanup reports, and holds the paused state with automatic still on.

An independent metadata-only observer must verify closed physical PCMs, no
remaining private links, an exact full desktop/state snapshot and the released
connected USB binding before acknowledging Pause. The GUI will not click Connect
until this acknowledgment exists in its exclusive test evidence directory.
The observer cannot reset a failed first owner or reuse its clock progress.
Explicit resume creates a fresh observer with new node/link/port receipts and a
new PCM progress history. After the second assigned playback holds for four
seconds, the GUI quits while running and verifies release through the worker.
Both physical owners must independently prove at least three seconds of clock
progress; combined progress cannot conceal a failed second owner.

These clicks operate only on the isolated qualification window. No owner's
mouse, Windows app automation, USB handoff, driver change, payload capture or
external Python audio bridge is involved. A returned assignment is still needed
for each physical session, so the owner may need to place mpv in Link In again.
The ordinary installed-app factory, UI and physical limits remain unchanged.

## Software and virtual evidence

Exact source: `beacn-native-source-20261015T123729Z`.
Frozen native Studio example SHA256:
`834e7cddc60524c8aab9234d0029ecceb71ba2ae8a66cf37afdd4f310828bd94`.
The native Studio and synthetic GUI examples built offline using cached targets.
Six native library checks and 44 focused Linux Python checks pass. New negative
cases reject missing pause proof, foreign routes, an open PCM, settings/binding
drift, a departed GUI owner, duplicate/premature resume, insufficient progress in
either owner and recovery options outside the explicit native first-pair scope.

The synthetic GUI uses the same recovery widget sequence with the existing
ScriptedPeer and virtual sink. It has no physical pause-acknowledgment target.
Run `beacn-native-playback-gui-20261015T123811Z` completed two nine-cycle sessions
with actual native ACTIVE/COMPLETE markers, Pause held, explicit resume and Quit.
Service journal SHA256:
`d5b4425c0c07ba1faafe77b0304635e8e24c62ef5498eb4994d2f90f885e75b4`.
GUI process journal SHA256:
`63c4e78e9ed3878eb136f19cc9a4e1909793e9699272806461b62f56f1cb66c6`.

The outer private-core baseline, full laptop desktop/settings/state, temporary
configurations, owned process exits and connected USB binding matched. The inner
service graph comparison was deferred while the outer owned mpv remained alive;
its `private_graph_restored=false` is preserved. Only the completed outer teardown
establishes the private-core restoration claim. No physical PCM was opened and no
USB I/O occurred during this rehearsal.

Laptop free disk after cached builds/preflight was about 8 GB; no more heavy
builds are needed for this check. No earlier source, executable or evidence was
deleted. Keep hardware results separate from the virtual evidence above.

## Owner-attended physical result

The owner explicitly replied **Start now** after the build/preflight, then
reported **heard both, didnt stay assigned, had to drag it after the first
success**. Run `beacn-native-studio-recovery-20261015T124049Z` completed with two
actual native physical owners in the same GUI process. Assignment 1 arrived at
cycle 15 in the first session and cycle 6 after explicit resume. The GUI paused
at cycle 23 and quit during the second playback at cycle 14. The owner had to
place mpv in Link In again; do not claim assignment continuity across Pause or
independently verified remote visibility after the second assignment.

Both native owners recorded 32 PCM checks, with pointer progress 340302 and
340304 frames. Independent metadata observations recorded 17/19 clock checks and
381967/430253 frames of progress, respectively. The paused-state witness verified
closed PCMs, absent private links, full desktop/state equality, released USB
binding and a still-live GUI owner before admitting explicit reconnect.

Final immediate and enclosing launcher comparisons matched the original full
laptop desktop/settings/routes/defaults/volumes/mutes/state and USB binding.
All owned processes stopped, the private core restored and both PCMs closed.
No USB toggle or external Python playback owner was used. Streaming-PC saved
settings were not independently inspected. Screenshots show the teal running and
gray paused states; the early transition captures still display unassigned
channel labels and do not establish rendered assignment labels.

Complete result SHA256:
`ed1e1b131782fe3e8fca4b0ec9507c69291ec9499568b9188eac65689b90b5b4`.
Pause witness SHA256:
`429732241fdd5c4dbb695ad05ccb7c29c3a892fdec9963077af7599269a9b7b9`.
Source, raw results, comparisons, screenshots and owner report remain private.

### Later comparison differs

A later audit did **not** match the original desktop/state baseline. The saved
Firefox output-stream channel volumes changed from `[0.562351, 0.562351]` to
`[1.0, 1.0]`; the stream node/port/link generations changed and the corresponding
`stream-properties` file hash changed. Other saved stream entries matched, and
the connected Studio binding remained exact. Cause was not established. No
foreign volume, state file or graph object was restored or normalized.

The first late-audit helper stopped on an assertion before saving its differing
dimensions. That failed audit is recorded; the subsequent full late snapshot,
difference paths and analysis were preserved. Immediate cleanup evidence remains
valid for its observation time, but there is no claim of exact late restoration.

Next: integrate ordinary desktop sources and the approved Console interface,
address assignment continuity with fresh routing proof, and separately qualify
failure/hotplug, multi-app physical routing and sustained operation. No test is
running; Studio can remain attached.

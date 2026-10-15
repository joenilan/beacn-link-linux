# LinX continuation handoff
Updated 2026-10-15 UTC. Resume development; do not restart the investigation.

## Latest checkpoint: real native stereo meters

The owner requested live meters and confirmed the repository is now public.
Actions remains disabled. A bounded read-only review of published `ab5e82e`
found no credentials, private host/key paths, device serials or capture artifacts;
the only non-text blobs were the two intended UI screenshots. This was not a
full security audit or a paid scan. Keep private evidence outside Git.

The actual Rust/C/GPUI app now measures its exact owned sink monitor through a
retained native `pw_filter`. It reduces stereo peak/mean-square power in roughly
40-ms windows, sends only aggregates through a lock-free mailbox, and displays
RMS bars plus peak/hold markers on the verified assigned pair. Other pairs are
unlit. UI polling is about 30 Hz while active; unchanged sequences expire after
250 ms. Missing/invalid/paused input clears; digital silence decays to an unlit
-60 dBFS floor. Fader/mute controls remain disabled; no volume/mute writes or
remote Windows mix metering were added. No samples are recorded or exported.

A native mpv-to-pw-cat calibrated source passed actual GUI assignments 1-4:
left/right peaks approximately -33.984/-39.992 dBFS, RMS about -37/-43 dBFS.
Digital silence, resumption, Pause/Connect on pair 4 and source exit passed.
Desktop, saved-state files and binding matched exactly. The subsequent real
Firefox run passed live metering through all four pairs and Pause, with actual
dark/light window captures now used in README. Its native owners cleaned up;
saved values and binding match. Firefox's retained idle stream/original route,
two recreated selected link identities, a cleared local 48-kHz Format cache and
saved JSON field ordering prevent claiming a byte-exact pre-Play graph/state.
See [meter details and evidence](native-meters.md). Firefox's original Paused
status was restored. Sampled LinX CPU was 2.6-2.7% of one core, RSS about 182 MiB.

Four mailbox/freshness, one C reduction and six ballistics checks passed on
Debian/PipeWire 1.4.2. The first live attempt refused a missing client-node module;
the desktop-only client config now includes it and one data loop. Failed-start
cleanup succeeded. Its proven new empty Test-source state entry was removed
with the app closed and WirePlumber stopped; every other saved value was kept.
The earlier target/PCM checks still pass. Luna reviewed RT safety and exact
ownership; listener removal synchronizes callback teardown before mailbox release.

The final binary completed a 600-second calibrated meter hold, repeated pair
switches, silence and resumption: 90 aggregate samples, sampled CPU 2.96% of one
core, RSS steady at 181.96 MiB. Pause released both owners; Connect retained the
final Link 1 assignment. The collector wrongly expected Link 4 and timed out;
the failed overall result remains in the receipt. Its final desktop, state-file
bytes and binding comparison was exact. Do not report that collector run as an
overall pass. The helper now checks the actual pre-Pause pair.
Its focused follow-up on the same binary passed all four pairs, silence and
resumption, Pause/Connect on pair 4, source exit and exact desktop/state/binding
restoration. Both native cleanup owners succeeded. Only the collector changed.
Final native binary SHA256:
`7595abcda6a3e73486b9d72f53bff8a545d8ee9736fcd87477dc474477fbf6dd`.
README captures came from the preceding Firefox run; the final change clears
telemetry on non-Running model observations and does not alter the visible layout.
The app is left open on the normal desktop in clean Paused mode for owner review.
Next useful controls are local Linux send gain/mute on the owned sink monitor;
the PipeWire 1.4.2 property path was researched, but no gain/mute writes or UI
controls are implemented. They need native property echo/ownership validation
and a separate runtime check before enabling the existing faders/buttons.

Private receipts: `meter-checks.json`, `meter-unit-checks.json/.log`,
`meter-cleanup.json`, plus compact capture provenance/history. Scripts drive the
actual native controls and collect evidence; they do not implement the audio tap,
routing or app session. Existing source/cache/executable locations are reused.

## Latest checkpoint: native recovery, saved-target protection and release prep

The owner approved **GPLv3**. LinX now declares `GPL-3.0-only` in all three
Cargo packages and includes the full license. The public-facing README describes
**LinX - unofficial BEACN Link app for Linux**, with real native dark/light
Console screenshots, build/use instructions and explicit feature limits.
Screenshot capture uses the opt-in native control socket; framing is standalone,
with no review server or Python application runtime. Contribution and release
preparation guides are included. No GitHub release or visibility change was made.

Actual native work fixed same-stream Pause/Connect intent, idle-source route
restoration, closed-source cleanup and a false PCM descriptor-limit failure.
See [native desktop recovery](native-desktop-recovery.md) for the exact identity
and proof boundaries. Recreated-stream resume remains explicit reassignment;
do not claim general automatic restart/hotplug recovery.

A first Firefox sustained run stopped after about 200 seconds on source departure.
Native cleanup succeeded, but WirePlumber had cached the temporary LinX sink as
Firefox's saved target. This was caught by persistent-state comparison and was
not counted as a successful soak. A separate volume change was preserved.

The current native backend therefore requires the scoped
[WirePlumber 0.5.8 target-cache guard](wireplumber-integration.md) before opening
USB. Its source reverses to the pinned installed upstream digest; other versions
require review/rebase. A missing guard was refused in the actual GUI at zero
cycles with exact state preservation. New sinks use random per-lease names.
The test laptop's old cached Firefox target alone was repaired with WirePlumber
stopped and exact prior-run evidence; other saved fields/volumes were retained.
That machine-specific repair is not part of the integration installer.

All five integration fixtures executed on Debian. Focused checks passed for
three core resume cases, ten native target cases and three PCM cases. Luna's
bounded source review found no concrete issue in these recovery paths. A native
GUI silent-source exit run then verified unchanged saved state while assigned,
clean closure of both native owners, and exact pre-launch desktop/state/binding
restoration. The source itself did not disable target persistence.

The dependency-notice candidate uses locked offline Linux Cargo metadata, with
566 components. Exact upstream commit/text hashes fill 26 missing package texts;
six packages and embedded-asset inventory remain release review items. Generated
notices and provenance remain private. No paid security scan ran.

The existing source tree, cache and normal desktop app were reused. Keep Actions
disabled, use `[skip ci]`, and preserve the fixed private receipts and snapshots.
The physical output checks below remain distinct from offline and silent tests.

The subsequent actual-GUI silent-source soak completed **1,200 seconds** across
all four pairs: 1,195 observed completed cycles, 4,784 native owned PCM checks,
and hardware pointer progress 115,806,762. The final source-exit cleanup succeeded;
desktop snapshot, persistent-state bytes/hashes and connected binding matched
the pre-launch baseline exactly. The selected source did not disable target
persistence. RSS ranged from 185.8 to 190.4 MiB; sampled CPU was about 3.4% of one
core. This is a silent physical-path stability check, not owner listening proof
or a multi-hour soak. Binary SHA256:
`62fdb91006f3a3a59cd953d93af62d4b8b5c75939e816e1ecf6d395990f11803`.
Private receipt: `owned-stability.json`, mode `soak`.

After Rust formatting, the exact final native tree built in the existing cache
in 2.5 seconds. Final executable SHA256:
`062a54772f160b343188cdad5b99aa59c780c2e66a89d0cd6f0f190e2d3db6cf`.
Its actual GUI lifecycle check resumed the same stream on pair 4 after Pause,
explicitly switched to pair 3, then joined Quit while active. The process and
owned control socket exited; device binding was unchanged. The strict raw
comparison did not pass: stock WirePlumber created an empty `media.role:Test`
entry for the disposable source, and the local sink retained its negotiated
48-kHz Format cache. No volume/mute or saved-target value changed. With the app
closed, only that proven new empty entry was removed while WirePlumber was
stopped. After restart all saved values match the lifecycle baseline; the only
remaining persistent byte difference is JSON serialization in `default-routes`.
Do not describe this lifecycle run as byte-exact desktop/state restoration.
Full receipts: `owned-stability.json` mode `lifecycle`, and `owned-cleanup.json`.
LinX is reopened on the normal desktop and left **Paused**, without selections
or an active Studio session. Next owner check: real Firefox playback, local
assignment/switching and heard output, then ordinary-use lifecycle acceptance.

## Latest checkpoint: actual local assignment and live switching passed

The owner remotely observed the announced four-pair run and confirmed that Firefox
**moved through all four Link In boxes in BEACN App, with visible audio activity**.
This confirms remote assignment reflection and incoming signal for the actual
native app, beyond its local route proof. The run held each pair for 15 seconds,
reached 95 cycles without errors, then Paused with both cleanup owners Ok, saved
state exact and connected binding unchanged. Private receipt: `remote-watch.json`.
The owner observed meters remotely; direct listening on this build is still pending.

The owner explicitly corrected the paused-only stopping point and authorized
driving the actual app's start/assignment path, with heard-output confirmation
later. Do not reimpose the older unattended restriction below on this authorized
app/device path. Preserve snapshots, ownership checks and cleanup.

A live Connect run held Firefox selected for 23 cycles with assignment zero;
the owner saw it in BEACN App's Applications list. Source review found the well's
chosen `channel` was never used: it only opened the drawer. The UI now has **Assign
to Link In / Link 2 In / Link 3 In / Link 4 In** for the chosen well. Native
`Command::Assign` binds current app/stream/alias/roster, selects only if absent,
starts if needed, uses the existing planner and applies the native route. The
well fills only after a fresh sealed route/PCM proof; no USB reply is injected.

Real execution exposed two switch bugs missed offline: ordinary-port discovery
rejected the sink's monitor ports, then removing all old links before creating
new links paused ALSA into PREPARED and reset its clock. Explicit monitor mode
keeps application ports strict. The new pair is now retained/acknowledged before
old exact proxies are destroyed, preserving continuous playback. Cleanup owns
both pending and current arrays. Luna reviewed partial-failure ownership.

Final main SHA256 `2eb730013389121f8de54f159aa4f7db6c2e458c91a8f49eb759c404f1f7001a`
passed real native UI assignments 1→2→3→4 in one physical session. Every actual
window capture shows Firefox in that well; native phase remains Running and the
verified assignment matches. There were 15 cycles, 60 USB-owned PCM checks and
hardware pointer progress 1,407,003. No audio payload was captured. Pause restored
Firefox's original output; both PCMs closed and both cleanup owners returned Ok.
Persistent-state bytes/hashes and connected binding match after WirePlumber's
delayed save settles. Raw graph equality is false only for the two recreated
Firefox route link serials; excluding only those identities, the graph matches.

Five focused checks pass: two native local-assignment/failure checks, two
request/stale-identity checks, and the C monitor/ordinary-port/ledger fixture.
The final UI wording fix makes unused wells say Choose application after the
selected app is already assigned. Current build and repeated live receipt are
in the fixed private workspace. No new trees, caches, branches or servers.
Next: longer use/reconnect and owner listening confirmation when home.
One stereo app is still the desktop limit. The actual app is left paused.

## Latest checkpoint: native component control and four-pair desktop routing

The owner has left and authorized agent control of LinX for further development.
Do not start unattended physical listening/assignment checks. Use the new
[native control interface](native-control.md) to drive the actual app without
moving their mouse. It is opt-in, same-UID, private Linux Unix socket control,
not an extra review server or Python session owner. State includes component
bounds, actual stream identities, selections, assignments, phase/error/cleanup.
Named click/text/scroll, app-only screenshot, resize, Pause and joined Quit use
the actual GPUI/native paths. The default build/launch has no control endpoint.

The owner tried selecting Firefox and assigning it again. Diagnostic main
`751f691ae9f70629cec5e5d4d38765b80a343806076d2cd4f433183a52dd18b9`
stopped at cycle 10 because its selected roster/core changed; selected became
empty before a received assignment, PCM checks remained zero and release was
clean. The log cannot distinguish the original revocation from an explicit
Remove; do not invent the cause. New logs distinguish selection commands and
revocation old/new stream metadata, and errors are no longer obscured by routine
"Selection updated" feedback. Empty wells distinguish selected/awaiting/stopped
from truly unselected. The actual underlying owner-reported failure is not yet
reproduced or claimed fixed.

Desktop routing now accepts one live stereo application assigned to any of the
four Link In pairs and switches the retained bridge pair on a changed received
assignment. It publishes the new pair only after native bridge/PCM/graph checks.
Only retained link proxies are destroyed; disappearance is verified by serial
before replacement. Confirmed detached ledger records are retired by exact
pointer to prevent accumulation through repeated switches. Pending/foreign
objects cannot authorize retirement. Zero/unresolved records preserve the
existing documented semantics. Multi-application desktop routing is still absent.

The USB transport's independent PCM proof now receives the managed desktop
lifetime too; previously only the target-side proof lifted its 4096-check count
fuse. Bounded and other constructors keep their limits. Identity, format, held-FD,
clock/stall and ownership checks remain. This is preventive, not the cause of
the cycle-10 owner failure.

The fixed source/build workspace and target cache were reused, without new
source trees, branches, servers or binary copies. The real main is compiled with
`studio,ui-qualification,native-control`; final build/test receipts and owner
runtime state are retained privately in the fixed LinX workspace. Main SHA256:
`9b15818ff782e8ebef59743a491142433a33fccee8296b7672557f46f7ae566c`.
The final exact native tree passes 10 desktop-session, eight desktop-target,
three PCM-guard and two USB policy/guard checks; the independent control transport
passes four checks, including an idle client yielding the listener. The reviewed
UI dispatcher pins its actual app window and partial screenshot writes remove
only their own inode; previous completed captures remain protected. Through the
actual desktop app, 37 native requests exercise
drawers, search/name editing, Firefox Select/Remove, both themes and five accents
while paused. Desktop and tracked persistent state match exactly. Additional
negative requests refuse disabled/missing controls, invalid resize/input/ops and
overwriting the existing screenshot. Native `quit` joins the worker, exits the
process and removes its owned socket; desktop/state match exactly afterward.
The same binary is reopened on the normal desktop and left paused, no selections.
The app-only control screenshot was captured and visually inspected. About
31 GiB remains free; no audio/USB session was started by these control checks.
Keep the app paused while selecting during unattended UI work: saved automatic
connection is enabled, so selection without Pause can open the device.

Next: reproduce the owner failure with the new invalidation diagnostics, then
confirm output and live reassignment through all four pairs while the owner listens.
All-four physical playback/switching and durable stream restart/reconnect remain
unaccepted in this newest main. Actions stay disabled; use [skip ci].

## Latest checkpoint: corrected Console on the actual desktop; owner review pending

The owner rejected the prior visual port and observed Link 4 wrapped below the
other strips in the real desktop app. Earlier isolated images did not establish
that actual window's appearance. Do not claim 1:1 approval. Two explicitly selected
Luna subagents provided a bounded visual review and the strip component; the root
agent integrated, compiled, captured and inspected the result. AGENTS now requires
Luna for suitable bounded delegated work rather than inheriting a costly parent.

Console now uses native equal-flex, non-wrapping rows, with explicit two-row narrow
layout below 700 pixels. It no longer derives fixed strip widths from the outer
viewport. The actual desktop frame reports the approved four strips exactly:
x=14/204/394/584, y=104, each 190x420, at viewport 788x568. Header/subheader/bank/
footer geometry follows the original reference, including 174px faders, all six
scale labels, source/category titles, thin separators, bevelled caps, source chips,
the original SVG tool/speaker/arrows and one footer. Settings holds automatic
connection/appearance; clicking an application well opens real selection. Drawers
expand a compact window within the display bounds and restore its original size
when closed, unless the owner has resized it. Gain/mute/meters and Mini remain
unwired; meters are unlit and dB is a dash. No false gains or routing records.

The reference uses Segoe UI/Consolas. Owner-provided system font files are installed
privately on the test laptop and hashed outside Git, not committed or included in
a distributable. This establishes the local review font environment, not portable
font packaging. Light palette support remains, but this batch's inspected frame
is Dark/Mint; do not claim a new full appearance/widget matrix.

The existing fixed source/build workspace was reused. Actual main with `studio`
and developer capture support built in 2.87 seconds; SHA256:
`68db20e4101ea40d24b9edecf2eb06d8b07b6ea30ef3b2a35c0408261c72de1e`.
`--capture-frame` is developer-feature-only and captures only this app's window.
Linux GPU scene readback is unsupported, so the diagnostic uses its exact retained
X11 handle, not another window or whole desktop. A normal desktop Xwayland display
was available; no VNC/display/review server was installed. Native SIGTERM/SIGINT now
requests the same worker join/release as Quit instead of abrupt termination.

The actual main passed read-only native render/normal-signal-close: full desktop/
settings/state exact, owned catalog configs restored, no USB/audio. The owner then
closed the old Wayland app normally. The new main was atomically installed and
reopened on their normal desktop with `--studio`, no selection or automated audio,
and its own desktop window captured/inspected. Current app stays open for review.
No new hardware playback acceptance or design approval was reported. Existing
scripted widget sequences predate the moved controls; update their interactions
when next needed rather than claiming they passed on this tree.

No review servers are running. Fixed current source/app/logs are retained, about
32 GiB free remains, and previous render attempts are compactly archived instead
of making new trees or binary copies. Next: owner review/direct native app use;
fix any remaining parity or runtime issue in this same app. Do not restart audio
checks without fresh readiness or silently substitute a test fixture for the app.

## Latest checkpoint: actual Studio-enabled LinX executable open for owner testing

The owner prioritizes a usable native app over further scripted test cycles.
`app-gpui` now has an explicit `studio` build feature and `--studio` entry into
the retained Rust/C desktop owner. The normal no-argument build remains read-only.
This is the actual GUI main binary, not a qualification example or Python session
controller. It discovers one unique supported Studio and its current eight-channel
ALSA sink on the authenticated native desktop connection when Connect is pressed.
No host address, serial, cached descriptor file or numeric endpoint is needed in
the launcher. Descriptor/device/PCM identity and all later bridge guards remain.

`DesktopLinkSession::start_managed` and `Publication::managed_desktop` retain the
existing sealed desktop proof policy through Pause/Quit/failure instead of a
40-cycle countdown. Bounded constructors and PCM qualification fuses remain
bounded; only the explicit managed desktop owner lifts its observation-count fuse.
Clock/owner/format/FD/capture/identity checks remain enforced. One existing stereo
application and first Link In only; other apps and pairs remain unavailable in
this desktop path. Selection loss or changed foreign graph stops safely; automatic
hotplug/rebind, persisted selections and durable assignment continuity are unfinished.

Compiled source `beacn-native-source-20261015T143148Z` passes 23 focused native
library checks, including continued sealed publication past 40 cycles followed by
selection-loss cleanup, unchanged bounded limits and managed PCM metadata checks.
The offline actual GPUI main build with `studio` completed in 7.18 seconds. Native
app SHA256 `145cf13e3709ffc63c5fd46dcd4e2252732a06d9b99f997f02f058e37f8c613b`.
It was launched on the existing normal Debian GNOME session with no selection,
no timed fixture and no automated USB/audio start. A user LinX launcher was added;
no autostart entry or new remote desktop/review server was installed. Appearance
still differs from the approved mockup and is **not** owner-approved or 1:1.
Applications starts collapsed; select one stream, Connect, assign to first Link In
in streaming-PC BEACN App, then Pause to restore its original output. Gain/mute/
meters remain disabled. Physical acceptance of this managed main binary is pending
the owner's direct interaction; do not equate unit/build/process liveness with it.

The owner explicitly rejected excessive test directories, binary copies and stale
servers. Follow the new AGENTS hygiene section. Use a fixed current Linux workspace
and one app binary, retain small source manifests/evidence privately, and reuse a
single review server only when needed, closing it after two hours. Do not invoke
the old timestamp-copy staging helpers for another routine iteration. The current
app executable is hard-linked instead of copied again. Inactive GPUI example and
retired egui generated artifacts were removed after path/process checks, recovering
about 31 GiB; approximately 33 GiB remains free. Source, lockfiles, snapshots and
evidence were preserved. A stale exited-GUI wrapper/private bus/Xvfb was stopped;
the expired original review server was closed. Do not close the owner-facing app
while they are testing it. Do not resume unrelated Windows cleanup.

Next: obtain direct app-window/audio/Pause feedback, fix any native failure in this
same runtime, then match the approved Console's measured geometry and styling.
No more repeated automatic physical test runs or claims of approved visual parity.

## Latest checkpoint: approved Console ported to native GPUI

Read [gpui-interface.md](gpui-interface.md#native-console-qualification-2026-10-15).
The owner correctly rejected a link to old utility-layout screenshots. The approved
01 Console is now the actual native interface: 788x568, four contiguous strips,
live application wells, expanding selection/Appearance drawers, Light/Dark and five
palettes. Below 600 pixels the strips reflow to two columns. Appearance is currently
in memory. Faders/Mute remain disabled, dB reads a dash and meter rails are unlit;
no gain/mute/meter backend was invented. Shared status colors remain unchanged.
Matrix/Mini remain concepts. The ignored review landing page now links versioned
native captures and explicitly marks the earlier cards/list images historical.

Exact compiled source `beacn-native-source-20261015T141718Z` passed offline cached
root and GPUI example builds. Console private GUI run `141816Z` used two actual mpv
processes, aliases, automatic connection, actual assignments 1/4, theme switching
and Pause. Outer private-core teardown and full desktop/settings/state/binding
matched exactly. Catalog journal SHA256:
`1a4735a509a837a6cb7a628502ccf2e2bb790fe2725dbcb82191ac29445013ca`.
No USB protocol I/O or physical playback was performed; no fresh attended test was
requested. Normal Desktop startup remains gated.

Read-only visual run `141751Z` passed all actual theme/palette/drawer/selection and
narrow-layout widgets, but its outer desktop/state comparison differed. One stream
changed identity/params/props, ports/links and saved stream-properties; cause is
unknown. Preserve raw evidence; do not overwrite foreign settings to match. The
preceding shorter visual run `141401Z` matched exactly. Later private-run equality
does not erase the earlier mismatch. Complete evidence and image provenance are
copied/hashed privately outside Git.
The final read-only audit matches the post-visual-run desktop/state snapshot,
finds both physical PCMs closed and no owned test process. Receipt SHA256:
`65c1888dc26370a42cb1b979c051ea4f08f7d065f99406dcc875fefc424df296`.

Before the cached builds, the laptop had about 1.77 GB free. With active-process,
resolved path/inode/owner guards, only this project's inactive incremental compiler
objects were removed, recovering 5.56 GB. Source, lockfiles, frozen binaries and
evidence were preserved; recovery snapshots matched exactly. This did not resume
Windows C: cleanup. Latest free space is about 3.63 GB; avoid redundant heavy builds.

Next: owner review of the actual Console captures, then fresh-readiness-gated desktop
GUI Resume/failure and longer-session hardware qualification. No installed app,
managed physical session, durable assignments, hotplug or new hardware support was
added. Keep Actions disabled and checkpoints local with [skip ci].

## Latest checkpoint: desktop publication, GUI assignment and Pause heard

Read [native-desktop-session.md](native-desktop-session.md). An opt-in sealed
DesktopRoutePublication and bounded DesktopLinkSession now join the real desktop
catalog, planner, guarded USB session and retained desktop/Studio owner. Unassigned
apps keep their original output; only a received first-Link-In assignment applies
audio. Private receipt/zero rules stay unchanged. Expired/revoked/replaced/replayed
proof and raw/private policy bypasses are refused; failed cleanup blocks restart.

CLI run `linx-desktop-session-20261015T134927Z` (source `134548Z`) passed 40 cycles,
assignment one at cycle four and actual native PCM progress. Owner: “ok it worked.”
Result SHA256 `fd47c77f39f9a0be733a3a7954915285c44bd7b56c2f8d7d944c4fc7ea3aaed8`.
GUI run `linx-desktop-gui-20261015T135607Z` (source `135050Z`) passed actual selection,
automatic start, assignment/playback, Pause/held pause and Quit. Owner confirmed
**Yes—video played, then paused.** Transport proof: 36 checks / 822,967 frames.
Result SHA256 `1d3283cea3174c0a59193962cd2e922ccba36f326ec74d2c6b83ef125a4747cb`.
Assigned/paused screenshots visibly show Firefox in first Link In/teal icon, then
unassigned/gray icon and paused status. The automatic toggle remains on.
Both runs restored their foreign graph/settings, saved state and USB binding;
selected links were recreated, both PCMs closed. Older owned cache difference below
remains recorded. Evidence copied and hashed outside Git.

The earlier GUI run `135233Z` was started without fresh owner readiness. It got
no assignment, no PCM playback and timed out in the widget probe; its full baseline
matched. It is not playback acceptance. Wait for a fresh ready response for every
attended physical run, then announce and start immediately. Do useful offline work
while waiting; do not ask retrospectively whether the owner watched.

Final compiled source `beacn-native-source-20261015T135926Z` passes 101 focused root
checks and cached native root/GPUI builds. Eight app-core startup/preferences
checks passed on the accepted GUI source. The final source adds startup failure
status and exact initial proof/inventory matching; it has not had a new physical
run. The existing worker has an explicit DesktopStudioSession test connection;
normal Desktop/default GUI still gated. The approved Console layout is not yet
ported. No test is left running. Only the local screenshot review server remains.

Next: desktop GUI Resume/failure and longer-session qualification, then expand
generic app/stream/pair coverage and port approved Console design. No app-specific
backend path, managed physical session, hotplug, durable assignment continuity or
installed autostart was added. Laptop has about 1.7 GB free after cached builds;
no packages installed, source/lockfiles/evidence deleted, or workflows enabled.

## Latest checkpoint: ordinary Firefox desktop video heard

Read [native-desktop-target.md](native-desktop-target.md). The new opt-in desktop
lease selects one already-running stereo app through the native catalog, moves
it using an exact echoed WirePlumber target request, and restores its prior
metadata/destination. It uses the shared graph/inventory bridge and retained native
adapter owner; no application-specific executable path exists in the Rust backend.
The owner confirmed **Heard the Firefox video** through first Link In in
`linx-desktop-studio-20261015T132948Z`, exact source
`beacn-native-source-20261015T132933Z`: 98 PCM checks, 476,138 hardware frames,
FLOAT_LE/8ch/96k, capture closed. Result SHA256
`73b71ab5ed156bb01cba5a88b995979e347577854f7013f487bd03c95b3f62a9`.
Foreign graph/settings, saved state and USB binding matched that run's baseline;
selected links were recreated, both PCMs closed. No vendor USB I/O was used.

The existing desktop 48-kHz node stalled. The working path uses a retained owned
96-kHz adapter excluded from WirePlumber port/pause policy, leaving the ordinary
ALSA node/global audio configuration untouched. The actual daemon PID is pinned
from Core metadata separately from a socket-activated listener's credentials.
The first failed metadata write left an owned saved-property cache entry; its
owned target override was removed. A whole-file restoration guard later refused
because foreign settings changed. Preserve that difference; subsequent exact
per-run baselines include it. No foreign setting was overwritten.

Later exact source `beacn-native-source-20261015T133151Z` passes 25 focused checks,
all four native virtual mappings/fault cleanup and a real desktop failure after
an echoed target write. Failed start/restart remain latched and cleanup caches.
`linx-desktop-failure-20261015T133314Z` result SHA256
`52050995ee428b5239d483a55cadec02ff70592406e7164b8e6fdd10f7eca0c2`.
No new physical playback was claimed for that later failure-injection source.

This is still a bounded qualification example, not a normal installed application.
Next integrate a separate sealed desktop route proof into protocol publication
and the existing GUI session owner; never weaken private link-receipt/zero rules.
One app/stream and physical first pair only; multi-app, hotplug and durable
assignment continuity remain open. Laptop cached builds use about 6 GB free;
no packages installed or old artifacts removed. Keep snapshots and evidence private.

## Latest checkpoint: native Studio Pause and explicit reconnect heard

Read [native-studio-recovery.md](native-studio-recovery.md). The next native GUI
check passes its real virtual-endpoint/widget rehearsal: two fresh
nine-cycle owners, Pause held while automatic stays on, explicit resume and Quit
while playing. Six native library and 44 focused Linux Python checks pass.
The physical mode adds an independent released USB/PCM/full settings-state barrier
before reconnect and requires separate clock progress in both owners. It remains
two bounded sessions, not managed USB, hotplug or ordinary desktop integration.
Source `beacn-native-source-20261015T123729Z`; frozen native Studio image SHA256
`834e7cddc60524c8aab9234d0029ecceb71ba2ae8a66cf37afdd4f310828bd94`.
Physical run `beacn-native-studio-recovery-20261015T124049Z` then passed: two native
owners in the same GUI process, cycle counts 23/14, independent paused-state
barrier, explicit resume and quit while playing. The owner heard both runs but
had to reassign mpv after the first success; assignment persistence is unfinished.
Result SHA256 `ed1e1b131782fe3e8fca4b0ec9507c69291ec9499568b9188eac65689b90b5b4`.
Immediate full settings/state/USB and private-core cleanup comparisons matched;
both PCMs closed and all owned children exited. **Later** audit differs in Firefox
stream volume/identities and its state-file hash, with cause unknown. Preserve
that evidence; do not claim exact late restoration or restore foreign changes.
Studio is attached and released; no test is running. Next ordinary desktop/native
integration, assignment continuity and approved Console UI; failure/hotplug,
multi-app physical routing and long runs remain open. Laptop has about 8 GB free;
cached builds finished. Keep evidence and snapshots private.

The owner handed C: cleanup to another agent. Return to LinX development; do not
continue disk cleanup or disturb the owner's other projects/builds.

## Latest checkpoint: native first-pair playback heard and released cleanly

Read [native-playback-startup.md](native-playback-startup.md). Owner returned,
explicitly started attended runs and confirmed **Heard tones; mpv stayed in Link In**.
First assigned native attempt failed before sound; metadata diagnostic established
PREPARED with exact owner/FD/FLOAT_LE/8ch/96k identity. Initial link acknowledgment
was earlier than RUNNING. Startup now waits up to two seconds only for verified
PREPARED, with fresh route/graph proof; ordinary RUNNING/clock checks stay strict.
No invalid/unknown state is retried or adopted. Failure/cleanup still latch.

Exact source `beacn-native-source-20261015T120640Z`; native Studio image SHA256
`eac7adf632f6e771332a6928083ec00e6dfa3d2f5af74ed59b0680a37fc690a7`.
Accepted run `beacn-native-studio-gui-20261015T120932Z`, result SHA256
`33c287df7e5899086716a0224d1f36b05d1d99a7a8d90f920e3d0d4743b31156`:
40 cycles, assignment 1 at cycle 5, actual native ACTIVE/COMPLETE, 132 PCM checks,
independent 69 metadata clock observations and owner-heard sound. Adapter/routes/
USB/processes released, both PCMs closed, immediate and late laptop settings/state
and connected binding exactly equal. No Python playback bridge or payload capture.
The first failure had an unequal desktop/state comparison; preserve that evidence
and its limits rather than claiming every run restored exactly. Later baselines
matched. Six native-playback and five native-usb-only library checks pass; exact
native private GUI Pause/resume/Quit and full outer comparisons also pass.

Next: physical Pause/reconnect/failure, ordinary desktop integration and approved
Console UI. Keep features minimal and add only realistic, separately proven
capabilities. Owner also requested a future Windows edition; share UI/protocol
but implement/qualify Windows audio/USB adapters after Linux round one is stable.
No new Windows backend or remote mix controls exist. Studio stays on the laptop;
no test is running. Laptop free disk is about 10 GB; preserve all source/artifacts.

## Design checkpoint: actual visual references and three new directions

Owner rejected study 01 for retaining the existing cards/list layout. Its source
review was primarily textual; it was not a full visual comparison. Study 02 now
inspects actual published BEACN 1.4, Wave Link 3 Windows/macOS, Sonar and UAD images.
Open [the comparison](../design/linx-concept/directions/index.html) and
[visual findings/provenance](../design/linx-concept/directions/README.md).
Console uses vertical strips/fader caps/meters/app wells; Signal map uses a matrix
and source inspector; Mini is a compact utility. Owner liked all three mockups and
chose **01 Console as the default**, with Light/Dark and accent choices. All three
now share those themes and Mint/Blue/Violet/Amber/Rose palettes. Icon/status colors
remain independent. This is approval of visual direction, not hardware acceptance.
All gains, assignments, levels and session states are demo fixtures. Linux send
gain remains distinct from unverified remote personal/audience mix controls.
72 browser layout/state combinations, thirty theme/palette/layout combinations,
narrow Light/Dark and interaction checks pass. Screenshots,
reference image caches and manifests stay private outside Git. No Rust/gate,
device/audio I/O, laptop settings or test-artifact changes occurred.
Owner wants direct clickable localhost review links, not only file paths.
The latest physical first-pair acceptance is above. Keep Rust/GPUI Kit and existing
snapshot/owner logic. The earlier study remains linked for provenance.

## Earlier checkpoint: native playback owner and GUI pass unattended

Owner asked to finish everything possible while asleep. Optional Rust/C playback
now owns an eight-channel sink/two monitor links in-process, with fresh verified
routes, pinned generations, native PCM metadata proof, latched failure and exact
release comparison. Native Studio remains bounded to 40 cycles/4096 PCM checks;
normal physical/desktop apply stays disabled. No USB I/O, physical PCM open,
Windows change, capture or owner mouse input occurred in this batch.

Read [native-playback-owner.md](native-playback-owner.md). Exact native-qualified
source `beacn-native-source-20261015T035526Z`; native run
`beacn-native-playback-20261015T035622Z`, result SHA256
`20912f49c3b71213a946f678b7cc3d57651c1435390892a452b8ad04e0bfa4af`;
GUI run `beacn-native-playback-gui-20261015T035801Z`, result SHA256
`e3cdf1376a28cdfa093929e5090c61d4fbc3139c69fd551b68f0f384c0ece923`.
All four virtual mappings, 128 refreshes/64.417 seconds, four native failure
boundaries, Drop/source-exit release and actual GUI Pause/held pause/reconnect/
Quit pass. Original private core, full laptop settings/state, configs/processes
and connected USB binding match; blue/gray screenshots inspected. 71 Linux root,
17 changed Windows, 39 Linux Python, twelve core and eight reference checks pass.
Native and production builds pass; source/evidence limits are in the report.

First physical native-owner check is **built but unrun**. Native Studio artifact
SHA256 `c95827ee62245e545c8745745a7f6ce9bc95b72aff8468bb72602618375c0a1d`.
Use existing private native-playback ready/checkpoint state for its exact artifact
and pending launcher. Python is a metadata-only witness in that explicit mode.
Do not substitute earlier accepted supervisor evidence for native acceptance.
When owner is awake, finish preflight, explain Applications/all Link In boxes
and first-pair listening, wait for fresh **start now**, then run immediately.
Next qualify physical pause/failure/reconnect and ordinary desktop integration.
No test is running. Keep Actions disabled and local checkpoints [skip ci].

Build disk is below 25 GB (about 14 GB after preserving compiled artifacts).
Owner notified; avoid repeated heavy full builds/staging and never delete
evidence/source/running artifacts. Cached native targets and immutable executables
were used. No dependencies/toolchains/drivers installed in this batch.

## Current checkpoint: managed session lifetime passes native private GUI

`LinkSession::start_managed` and `Publication::managed_selected` now explicitly
retain the existing route/transport owners until Pause, Quit or failure. Every
cycle still needs a newer catalog observation and complete fresh route proof.
No count sentinel enables this mode; bounded constructors still reject budgets
outside 1..40. Counter exhaustion stops safely. Selection/core/stream changes,
stale proof, route/control faults and failed cleanup retain the existing stop,
no-retry and NeedsReview behavior. No ordinary desktop or hardware gate widened.

The explicit private GUI mode ran two real mpv apps on native private routes 1/4
for 96 cycles, paused with automatic connection still enabled, held the pause,
resumed explicitly, stopped on selection change, restarted with one app, and quit
with verified release. Whole private core, laptop desktop/settings/routes/defaults/
volumes/mutes/state, owned configs/processes and connected USB binding matched.
No USB I/O or physical output occurred. Source `beacn-native-source-20261015T030813Z`;
run `beacn-managed-gui-20261015T031153Z`, complete-result SHA256
`93fdfd7b8d908a5f449cc9bef83f273751cf1f9a21ea5a2d8000a65f344256a5`.

57 focused root checks pass on Windows and Linux, plus 31 Python runner, eleven
core and eight reference-GUI checks. Exact native binaries, production GPUI and
explicit Studio example build. Production mapped window/read-only catalog/gray
icon/normal close and exact desktop/state/config/USB comparison pass in
`beacn-gui-desktop-20261015T031452Z` (journal SHA256
`affe340fc444669b858c3a3b80844fa7ce692e0b75f9ed6f05344f2301f46c82`).
The unchanged bounded GUI widget/session sequence also passes; its outer strict
desktop/state comparison failed amid a pre-existing desktop stream's recreation.
Its raw mismatch and all earlier failed setup attempts remain private; no foreign
state was restored. See [rust-link-session.md](rust-link-session.md#managed-lifetime)
for exact evidence boundaries and the build-interference correction.

Persistent physical playback is still unfinished: Studio GUI remains explicitly
40-cycle qualification with the external Python bridge, its 45-second bound and
native PCM proof's 4096-check fuse. Do not select managed mode for that adapter or
raise those limits to claim standalone acceptance. Next implement retained native
Rust/C physical playback ownership, then qualify physical pause/failure/reconnect
and ordinary desktop routing. Nothing needs watching or toggling now. Finish
all builds first, then obtain fresh **start now** for any watched hardware run.
Do not build over helper executables while a qualification is using them.

## Current checkpoint: native GUI physical Link In passed

The owner explicitly replied **start now**, then **Heard tones; mpv stayed in
Link In**. The real native GUI session received assignment 1 at cycle 9, verified
its native stereo route, started physical playback through the existing bridge,
continued control/publication, stopped playback before cycles 39/40, and verified
complete route/transport release after all 40 cycles.

Qualified source `beacn-native-source-20261015T024631Z`; hardware evidence
`beacn-studio-gui-20261015T024727Z`, complete-result SHA256
`932cea339ecd87a578d4b96b3639123ea059628594da4adc52b2164a3cd5677a`.
116 native active-PCM checks measured 1,356,890 frames of progress; 64 independent
clock observations measured 1,436,472. Source identity was preserved, all four
run-owned processes stopped, original private core restored, both PCMs closed,
and complete laptop desktop/settings/routes/defaults/volumes/mutes/state and
connected USB binding matched. Streaming-PC saved settings were not compared.

The explicit `studio-session-test` feature is bounded one-app qualification, not
normal installed-app acceptance. Session, selection, routing and USB owners are
Rust; physical playback still uses the existing external Python test bridge.
Default production USB/apply gates and 40-cycle budget remain unchanged. Read
[native-studio-gui.md](native-studio-gui.md) for boundaries and earlier failures.

Exact production and immutable Studio example builds pass. Five Linux Python
ownership/protocol and all 31 runner checks pass; eleven core and eight reference
GUI checks passed on identical core source. Native private GUI regression
`beacn-gui-20261015T023740Z` passes. Production `--software-renderer` also passes
real isolated window/read-only catalog/gray X11 icon/normal close and exact laptop
settings/config/USB comparison (`beacn-gui-desktop-20261015T025113Z`). No driver,
firmware, ACL, global renderer or Windows setting was changed.

Earlier no-assignment runs lacked coordinated owner participation; do not infer
peer/audio failure from them. Graphics-startup stalls preceded device access;
per-process installed Mesa CPU Vulkan selection lets the test window open, with
the exact hybrid GPU stall untraced. An SSH outage was recovered before success.
No further hardware test is running. Finish builds/checks, give exact viewing/
dragging instructions, then wait for fresh **start now** before another watched
run. Leave Studio connected; no toggle is needed. Next implement standalone Rust
physical playback/ordinary desktop routing, then qualify physical pause/failure/
reconnect and longer operation. Keep Actions disabled and checkpoints [skip ci].

## Current checkpoint: automatic waiting and pause

The owner requested Windows-style launch-and-wait behavior before physical GUI
testing. The GPUI window now has **Connect automatically**, plus Connect/Pause.
Normal launch defaults automatic on; its toggle is the only saved LinX setting
and is written only on change. The current production preview still has no
physical/desktop apply factory: it waits honestly, with Connect disabled. Private
test mode remains manual by default and never uses user preferences. No login
autostart entry, installer or background close-to-tray was added.

`app-core::startup::Supervisor` owns intent, not mutation authority. Existing
session/receipt owners still validate and release every route/transport. Pause
holds across reconnection; explicit Connect or automatic off/on re-arms it.
Failed starts, bounded completion and cleanup requiring review never cause an
automatic restart loop. Turning automatic off leaves a running session active.
Lost read-only catalog discovery now retries every two seconds instead of killing
the worker. It revokes old selections and re-observes; no same-name rebinding.
Unsupported/changed/symlinked preference files are preserved, with visible errors.

Eight new preference/lifecycle checks and the eight prior shared-model/egui checks
pass. Native GPUI automatic start, verified assignment, pause held over ticks,
re-arm and quit while automatic all pass on the disposable core. A separate real
worker waits with no server, observes its startup, detects its exit and reconnects
to a new core without reopening the app. Production read-only GUI close and actual
gray icon pass; desktop/settings/state/config comparisons match and owned processes
exit. The first GUI sequence failed because two expanded editors no longer fit
its setup viewport; the final probe uses a larger setup view, then still checks
720x520, narrow and short-window scroll. Raw failed evidence remains private.

Full Windows hotplug/reconnect policy remains untraced; see protocol-session.md.
No more reverse engineering is needed to implement this independent UI policy.
Next: finish the Rust physical session/playback integration, then validate Studio
attach/detach and continuous operation after the owner finishes streaming. Do not
confuse native catalog recovery with hardware hotplug acceptance. No physical test,
Windows change, USB handoff, REA or owner mouse automation was performed here.
See [gpui-interface.md](gpui-interface.md#startup-and-waiting) for exact semantics.

Startup-qualified source `beacn-native-source-20261015T015044Z`, native result
`beacn-gui-20261015T015125Z` SHA256
`24843f723497d8d4d57367386de44fb1e3eb694bd345410238ac9124fff48ff4`.
Discovery recovery journal `beacn-startup-20261015T014842Z` is retained privately;
its worker/core source matches the final startup-qualified tree. Production
read-only journal `beacn-gui-desktop-20261015T014949Z` SHA256
`405212fe747a82aa08de21607277ecb66ebf85cc799a90ebe503728d570a1699`.

## Current checkpoint: LinX GPUI Kit frontend works

The owner chose GPUI Kit (used in their other apps), compact/expanding layout and
the independent working name **LinX**. No trademark clearance is claimed. Keep
historical evidence/repository identifiers stable. See
[gpui-interface.md](gpui-interface.md) for build/run commands and full acceptance.

`app-gpui/` is now the primary UI, pinned GPUI Kit 0.7.3 with Rust 1.99.0. The
original 1.88 backend toolchain/default remains intact. Discovery, model, worker,
private session, icon geometry/state and Linux tray moved to `app-core/`; the old
egui reference under `app/` reexports those owners and its eight checks still pass.
Root dependencies/default gates remain unchanged. No JS shell/webview/speech.

Default window 720x520; four channel cards wrap to two rows at narrow widths.
Per-app details and info expand on demand, and short views scroll instead of
clipping controls. Search includes aliases. Kit inputs/buttons use actual keyboard
and pointer dispatch. A saved unchanged alias is disabled/no-op, avoiding an
unnecessary fixed-roster stop. The lynx status/tray rules remain truthful; normal
discovery is gray, scripted private sessions blue, and default Start stays disabled.

Two real mpv apps pass native GPUI selection/naming/search, 720x520/440x520/
900x640/440x380 rendering, small-window scroll/reachable controls, pairs 1/4,
stop/restart, unchanged-save continuity, selection-change stop and quit while
running. The app-quit hook verifies both route and transport release, plus config
removal. Complete private-core teardown and relevant desktop/settings/state hashes
match. The production build omits qualification features and independently passes
read-only desktop discovery, actual gray X11 icon and normal WM_DELETE_WINDOW.
Shared tray private-bus five-state/menu/fallback/release checks pass. No real
desktop-panel visibility, Wayland dynamic taskbar icon or release performance claim.

Idle redraws now depend on visible changes, not catalog revision increments.
Discovery still observes every 500 ms and before commands. An isolated debug/Xvfb
sample fell from .350 to .070 CPU seconds per 2.001 seconds; about 205 MiB RSS.
No speculative build flags. 74 focused root, 31 runner and eight shared-model/egui
checks pass. GPUI's 1.92/1.94 documentation mismatch and first screenshot helper's
32-bit-depth refusal remain recorded privately. 38 new development packages,
no upgrades/removals; setup/settings comparison exact. No driver/service changes.

Qualified source `beacn-native-source-20261015T012848Z`; complete native result
SHA256 `d95f9c3d259fceecbf344320f9e9909b413784d36b1ef80d0e0408e078e05398`.
Production journal SHA256
`6f449886d7d74c7ea4b1548044015ea28b6f4461663da9455d22746c688988b0`.
Build/runtime logs, source hashes and inspected screenshots are preserved privately.

The owner remains live streaming: no USB, physical audio, Windows changes, desktop
apply, OS mouse/computer-use or REA. Next supply the physical transport/playback
owner for the shared session and qualify actual multi-app GUI/hardware after the
stream. Shared-desktop apply, unaffected-app continuity, durable reconnect and
packaging remain open. Keep Actions disabled and local [skip ci] checkpoints.

## Previous checkpoint: GUI sessions and status-aware lynx icon

The owner remains live streaming: software/private-core qualification only. No
USB, physical audio, Windows changes, desktop route apply or mouse automation.
See [rust-link-session.md](rust-link-session.md) for the exact ownership contract.

LinkSession joins existing selection, epoch, routing and guarded transport owners.
Every bounded publication needs fresh catalog plus complete observed route proof;
assignments apply through existing acknowledgment/receipt owners. Stop releases
routes before transport. Lifecycle/alias/core changes or failures stop both,
without retry. Unverified cleanup blocks restart. Legacy single-app gates remain;
only the explicit bounded_selected gate allows a fixed multi-app roster.

The GUI now supports Start/Stop and verified channel cards only through explicit
private-session-test configuration: real native private routes, actual Handshake/
Session bytes, scripted peer, no USB. Two real mpv apps pass select/name/start,
pairs 1/4, stop/release, restart and selection-change lifecycle release using real
widgets on isolated Xvfb. The normal binary still has no route/device factory and
Start remains disabled. This is not physical GUI or shared-desktop acceptance.

One lynx silhouette and status mapping serve the interface/window and optional
Linux tray: gray disconnected, amber connecting, teal successful running Link,
red error/review, blue private scripted test. Discovery alone never turns it teal.
The tray offers Open/Stop/Quit on its own bus thread and explains missing-host
fallback. No close-to-tray, autostart or persistence yet. The private D-Bus test
checks all icon pixels/states and menu callbacks, plus bus-name release. Actual
GNOME/KDE panel visibility remains open; no desktop extension was installed.

Eight GUI/model tests, five new session tests, 74 focused Linux root tests and 31
Python runner tests pass. The production GUI separately opens read-only discovery
and closes normally on isolated Xvfb/private D-Bus, preserving settings/state.
All enclosing private-core, desktop, hashes, config and process comparisons pass.
Intermediate negotiated disposable-node Format/vector differences remain false
in evidence; final original whole-core equality is required and passes.
Qualified source `beacn-native-source-20261015T004833Z`; GUI/session qualification
SHA256 `f1becbf4d96b8e9f46016685a8f6e0ffbfc971cebbc154b8363ad5f7241f81ef`.
Tray and production window-icon/normal-close evidence hashes are in the session
report. 61 evidence/provenance files copied and hashed privately. The latter test
also verifies the actual X11 gray icon property; absent-host fallback passes.

Next: native physical transport/playback ownership for this session, then real
multi-app GUI/hardware qualification after the stream. Shared-desktop apply,
unaffected-app continuity, durable reconnect and installation remain open. Keep
Actions disabled and checkpoints local with [skip ci]. Do not repeat initial audit.

## Previous checkpoint: Rust graphical discovery/selection preview works

The owner remains live streaming. No USB, hardware/audio, Windows app changes or
desktop route application. The first Rust GUI uses egui/eframe 0.33.3 in the separate
`app/` package; root core dependencies/default features are unchanged. See
[rust-interface.md](rust-interface.md), including normal Linux run instructions.

Actual native application discovery runs off the render thread. Search,
Select/Remove and published-name editing use the existing ApplicationService.
An eight-command queue is nonblocking; fresh native observations precede commands.
Queued actions pin the displayed domain/application/streams, so replacements are
refused. Core/stale/departure changes revoke selections and failure disables the
last list with an explanation. Selections are in-memory; no automatic reconnect.

The window clearly shows routing off, disabled Start link and inactive channel
cards. It has no routing backend factory/native-usb feature. RoutingControl is
still separate verified core API; the GUI does not yet make apps visible on Windows.
Five egui/model checks pass plus the focused root checks and 31 Linux Python checks.
On an isolated Xvfb display, actual window widgets select two real mpv processes,
edit distinct names, rename, remove and close with owned configs removed. Rendered
screenshot inspected privately; complete graph/desktop/settings/state cleanup exact.
Production no-test-feature GUI also opens the normal read-only desktop catalog and
closes normally on its own isolated WM_DELETE_WINDOW, with exact settings/state.
No owner's mouse/desktop interaction. Only Xvfb was installed, no upgrades/removals.
Qualified source `beacn-native-source-20261015T002307Z`; complete qualification
SHA256 `77910484eccb37256e9ec35a7f6e991a632fd4665b0b258f4d983c1d4f804036`.
205 private evidence/provenance files were copied/hashed before final provenance.

Next: connect the graphical controls to RoutingControl/session state and guarded
transport, keeping desktop apply disabled until qualified. Multi-app hardware,
unaffected-app continuity, durable reconnect and installation remain open. Keep
Actions disabled, hardware closed while live, and local [skip ci] commits only.

## Previous checkpoint: Rust start/stop/status and native lifecycle cleanup pass

The owner remains live streaming. Continue software/private-core tests only: no
USB/physical audio, Windows app/routing changes or desktop apply. See
[rust-routing-control.md](rust-routing-control.md).

Rust RoutingControl now provides explicit start/stop/status, fixed-roster lifecycle
checks and verified assignment handling. Idle opens nothing; empty selection and
duplicate start cannot invoke a factory. Selection/core/source changes stop the
whole owned test session. Opaque/empty protocol replies do not stop or unroute.
Cached status separates observed and verified revisions. Unverified cleanup clears
assignments, enters NeedsReview and blocks restart. Stop is idempotent.

NativeRoutingLease retains the original private executor plus an independent
source-unbound inspector. It releases its own native handles even after source
departure or observation failure. Receipt-owner comparison verifies removal of
known surviving links and exact preservation of every remaining graph object;
it never selects deletion targets by ID/tag. No C/default USB gates changed.

Two actual mpv processes passed start, pairs 1/4, zero/no-mutation, stop/restart,
and pairs 2/3. Killing one source stopped the session: two links already absent,
two remaining owned links released, and a foreign same-tag link preserved by full
identity. No automatic restart occurred. Full private-core/desktop/state/config/
process cleanup passed after owned-node teardown. Source
`beacn-native-source-20261014T230633Z`; control journal SHA256
`a97d576faf3c6fd470999c44a45e654013a6f8fe28a71927b1e1082ada4c947c`.
55 Windows Rust checks, 54 Linux Rust checks and 30 Linux Python checks pass.

Remaining apps currently stop together when a selected app exits; unaffected-app
continuity is not implemented. Default foreground service remains read-only; these
controls are an API plus bounded native qualification example, not a GUI or daemon.
Next: Rust interface around discovery/control/status. UI framework not selected.
Shared-desktop apply, multi-app USB/physical interoperability and durable reconnect
remain open. Keep Actions disabled, hardware closed and local [skip ci] commits.

## Previous checkpoint: native two-app private routing and publication data pass

The owner is live streaming. Continue software work only; no USB/physical audio,
Windows app/routing changes or desktop apply. See
[multi-app-publication.md](multi-app-publication.md) for the implementation and limits.

A fixed-roster Rust publication epoch now connects explicit application selections
to session reply interpretation, guarded preparation and complete observed-route
publication data. Selected lifecycle/alias/core changes halt it, even edits that
are reversed before use. Routine polling invalidates pending plans; private epoch
identity prevents cross-epoch replay. Short duplicate aliases are refused atomically.
NoPayload/missing records/zero still do not imply unroute. Single-app USB gates and
all existing native backend/receipt restrictions remain unchanged.

Two actual mpv processes on one owned private core simultaneously routed to pairs
1/4, moved to 2/3, verified both complete routes and restored their prior routes
using concrete receipt recovery. A second retained executor stopped after two
successful creates, refused publication and rolled back to an exact graph baseline.
No USB or physical audio occurred. 43 Windows Rust tests, 42 Linux Rust tests and
29 Linux Python checks pass. Source `beacn-native-source-20261014T224851Z`; native
result SHA256 `966b77ff771bf577cb3ab768c985015865b48acbcd4345713c852bbbd8138994`.

Intermediate adapter Format/channel-vector negotiation differs and stays recorded
as false in the nested journal. After complete owned-node teardown, the original
whole private core, full laptop settings/state and new config/process cleanup pass
exactly. Earlier failed attempts/artifacts are retained. Multi-app hardware control
and physical playback are untested; the foreground service remains read-only.
Next: Rust app controls/status using the same owners, then selected lifecycle stop
and receipt cleanup integration. Keep hardware testing paused while the owner is
live; default desktop apply and Actions stay disabled. Local [skip ci] commits only.

## Previous checkpoint: native Rust read-only application/service works

`beacn-linx inventory` and `beacn-linx service SECONDS` now use a distinct native
read-only PipeWire catalog. No Python discovery helper or Cargo dependency is
required by the app. It discovers actual application metadata, accepts explicit
multi-app selections/ASCII wire aliases, revokes departed/stale/core-changed
identities and invalidates old plans on selection edits. Selections are in memory;
USB/publication, desktop apply, GUI/autostart and automatic reconnect are absent.
See [rust-application-service.md](rust-application-service.md).

The immutable C catalog mode binds only Node/Client queries and refuses every
mutation call before endpoint access. Existing private-executor restrictions were
preserved. The actual test-only C refusal check, eight Rust model tests (Windows
and Linux), 28 focused Python runner checks and exact native offline builds pass.
The live desktop query returned zero apps and matched settings/state exactly;
populated discovery was qualified with real mpv processes on a private core.

One retained native Rust service/core selected two apps, revoked a departed app,
observed its reopen as a new unselected identity, accepted explicit reselection,
unselected the first app and quit normally. EOF/deadline/input bounds and complete
graph/desktop/state/config/process cleanup passed. The enclosing first mpv then
routed all four private destinations. Existing actual native fault/recovery and
32 repeated already-correct routes also passed. No USB or physical audio occurred.
Live source `beacn-native-source-20261014T222104Z`; service journal
`7c88fd7476200ed150983ca74a5ac34a2a21ac88a87018f87cd3548e1a4aa224`.
102 private evidence/provenance files were copied/hashed before final provenance.

The owner said they would switch Studio back for streaming. The later no-USB
baseline found no Studio on the laptop; Windows Link/peer visibility was not
rechecked. Do not attach, start hardware tests or assume that return grants new
execution authority. Continue offline. Next: guarded multi-app publication/lifecycle
integration, then Rust UI controls. Keep default apply and Actions disabled,
preserve all artifacts and use local [skip ci] commits unless publication is asked.

## Previous checkpoint: real Linux application heard on all four pairs

The owner named the app **Beacn LinX** and selected Rust for the application.
Rust already owns protocol/USB/planning/lifecycle/execution; Python remains
qualification tooling with a small C PipeWire FFI. No UI framework selected yet.
Full implementation, provenance, hashes and limits:
[native-application-routing.md](native-application-routing.md).

Unmodified Debian mpv 0.40.0 was discovered by actual application name/native peer
PID and executable generation, then routed by Windows BEACN App assignments 1–4.
Four separate 40-cycle runs passed concurrent physical playback/publication and
owner-heard tones. Pairs 2–4 confirmations also covered the expected meter and
continued mpv assignment. Two actual stereo receipts preceded each physical start.
Each run stopped all four owned processes, closed PCMs and exactly restored the
private graph, USB ACL/binding and full laptop settings/routes/state. Streaming-PC
saved settings are outside the comparisons. No USB toggle was needed.

Live source: first pair `beacn-native-source-20261014T215427Z`; pairs 2–4
`beacn-native-source-20261014T215833Z`. A later no-USB source-exit probe detected
ordinary mpv closure at request 4 after two routes, delivered no next inventory,
terminated the waiting native test child and cleaned up exactly. Its expected
runner exit 2/native -9 is negative-test teardown, not graceful native recovery.
A fresh isolated core/session then routed a new mpv identity through all four
destinations and cleaned up. Lifecycle source `beacn-native-source-20261014T220341Z`.
34 focused Python checks pass and exact native programs build offline. 1,193
private evidence/provenance/report files were copied and hashed outside Git.

Hardware tests are finished for the owner's pre-stream window. Studio is released
and may return to the gaming PC; Windows Link can be reopened by the owner after
physical return. No more USB execution is assumed after that handoff. Private local
runner enforces the window cutoff; do not extend it without new owner direction.
Next software work: whole-desktop application discovery/selection, multiple apps,
Rust service/UI controls and durable lifecycle/reconnect. Ordinary desktop apply
remains disabled. This is bounded real-app/four-pair acceptance, not a shipped
daemon or full replacement. Actions stay disabled; local [skip ci] commits only.

## Previous checkpoint: concurrent native audio/control and Link In retention pass

The separate Debian laptop now passes bounded continuous first-pair audio while
native USB publication/polling continue. The owner confirmed silent-run Link In
retention, then reported "had to drag it in there first, then heard tones, stayed
in link in" for the audible run. No USB toggle was needed. Full implementation,
evidence hashes, guard limits and provenance are in
[native-continuous-studio.md](native-continuous-studio.md).

Live source `beacn-native-source-20261014T214415Z`: silent assignment at cycle 5,
136 native active-PCM checks and 1,744,716 frames of concurrent progress; audible
assignment at cycle 9, 120 checks and 1,522,199 frames. Both completed all 40 cycles,
82 fresh inventories, final post-audio verified publication and exact laptop
settings/routes/state/USB/ACL cleanup. All four owned processes stopped and PCMs
closed. Independent physical worker observations also progressed. Streaming-PC
saved settings are outside these comparisons; UI/listening evidence is the owner.

The explicit continuous mode retains a supervised physical worker and authenticates
its PCM owner before each active-audio vendor exchange. Default idle USB gates and
native route/receipt/removal guards stay enforced. An initial ESTALE metadata-bind
failure cleaned up safely. A narrow fix requires unfinished read-only query scope
and confirmed removal before the barrier; six synthetic port-change rounds exercised
78 removed bindings while preserving routing, recovery and exact cleanup.

64 focused checks pass and exact native programs build offline. 614 evidence and
provenance files plus owner reports are hashed outside Git. A final diagnostic-only
harness change preserves actual applied-route reporting even if continuous proof
fails. No REA, proprietary execution, capture, driver changes or desktop apply.

Next: ordinary application inventory/routing, other three pairs, then longer
lifecycle/reconnect behavior. Keep Studio attached to the laptop and Windows Link
closed. Do not repeat the successful first-pair test without a new reason or claim
long-lived daemon/full replacement acceptance. Actions remain disabled; no push
was requested. Prior VMware recovery limitations remain separate.

## Previous checkpoint: bounded assignment-driven PipeWire audio was heard

The separate Debian laptop now passes the bounded combined path: Windows BEACN
App assignment 1 -> native receipt-verified private FL/FR route -> Studio's
physical AUX0/AUX1 pair -> owner-heard tones -> physical cleanup -> freshly
verified assignment-1 USB republication through cycle 40. The owner reported
"i heard it, its in link in" during the announced audible run.
This establishes first-pair feasibility with the owned PipeWire test source;
ordinary desktop application apply remains disabled.

`--authorize-studio-first-pair-silence` selects the combined test in
`tests_native/usb_publication.py`; audible mode instead requires both
`--authorize-studio-first-pair-tone` and `--audible-first-pair`. Both require the
explicit private hardware routing mode and at least two cycles. The unchanged
USB gate requires idle audio during vendor bulk exchanges. The new bounded IPC
therefore pauses vendor exchanges only after assignment 1 has actual native
stereo receipts. The supervisor requires one exact, authorized physical request
after prior fresh inventories. Wrong pair, duplicate/early request, incomplete
cleanup or changed inventory stops the attempt without further exchanges.

`tests_native/studio_pipewire_bridge.py` reuses the existing ALSA/USB snapshot,
held-process identity, format and clock guards. It requires native execution,
the pinned released device/card/character-device inode, closed PCMs, exact
destination serial/controller PID, a running source, two active native-owned
FL/FR links and consistent port/node identities. A separate retained controller
creates only the raw eight-channel 96-kHz adapter and first-pair monitor bridge.
Each clock observation rechecks the route, bridge, selected device, initialized
vendor owner, PCM format/owner and complete foreign desktop snapshot.

The bridge controller is disposed before the native owner receives completion.
Exact private graph, closed PCMs, held binding and desktop snapshots must match.
The native owner then obtains a fresh inventory and verifies the route again.
Success additionally requires an actual verified USB publication after physical
completion; an assignment arriving at the final cycle cannot produce a false pass.
No driver changes, detach/reset, capture, desktop default/volume changes or relaxed
USB/receipt/removal guards were introduced. Existing default modes stay synthetic.

Silent combined run: assignment 1 at cycle 9, two acknowledged native stereo
creates, 18 physical clock observations and 369,199 frames of progress. Cycles
10–40 republished verified assignment 1. The owner reported that the name entered
Link In then disappeared. Its timing/cause is unconfirmed; do not claim continuous
UI retention. Hardware journal SHA256:
`0b8474bd68ea3305fc7598c23bb5cc12bac01b06ad7afc7907407f25cb3b236b`.
Physical-stage journal SHA256:
`1ad254a4a85e08e9bef908f140020561bf42d55266ab6715d3bc855bbc786fe0`.

Audible combined run: assignment 1 at cycle 3, two acknowledged native stereo
creates, 18 observations and 369,392 frames of physical progress. The authored
stereo S16LE pattern uses 440-Hz bursts capped at -24 dBFS, with a silent lead
and repeated quiet pattern so bridge setup cannot consume the whole tone.
Only AUX0/AUX1 receive it; physical output lasts about four seconds. The owner
heard it and reported Link In presence. Cycles 4–40 republished verified assignment
1; the complete run had one record, eight NoPayload and 31 rejected-marker replies
across 82 fresh inventories. Marker semantics remain unknown. Hardware journal:
`b81bfe535e1f7ec804b21a3cafaa80d75fee8ebc3587c40212db5114c1711d13`.
Physical-stage journal:
`8c85619a1daf024706ea03ce1beec6fd3c83b57fbc2f7bdb419ec471897a2ecc`.

Both runs exited 0 without timeout/retry, preserved source identity, stopped all
four owned processes, restored the private core and selected USB ACL exactly,
and matched complete laptop desktop/settings/routes/state/binding baselines with
closed PCMs. Streaming-PC saved settings are outside these comparisons. The last
owner screenshot identifies BEACN App v1.4.0; its live version was not requeried.
448 private artifacts plus owner reports were copied/hashed outside Git.
Live source: `beacn-native-source-20261014T211501Z`. Subsequent focused validation
adds malformed-metadata diagnostics and explicitly labels paused vendor transfers;
it changes no packet, routing or audio behavior. Twenty runner and three bridge
checks pass, covering authorization/IPC sequencing, native owner/endpoints,
quiet-tone bounds and failure journaling. Fourteen ALSA checks and the native
assignment-1/2 probe also pass. Final source `beacn-native-source-20261014T212301Z`
built all exact native programs offline and passed the three bridge checks.

Next: implement guarded concurrent vendor publication and continuous physical
playback, establish stable UI retention, then integrate ordinary desktop streams
and verify the other three pairs. The current pause-and-resume diagnostic is
bounded acceptance, not continuous daemon or full replacement acceptance. Keep
Studio attached to the laptop, Windows Link closed, Actions and ordinary apply
disabled. No new physical handoff is needed for the next software batch.

## Current owner plan: separate Debian laptop with the Windows agent retained

The gaming PC has **no native Linux installation**; establishing feasibility
before migration remains the goal. After the VMware transport failures below,
the owner volunteered a separate Linux laptop for a direct Studio comparison.
The agent remains active on Windows and now has authenticated SSH access to
Debian 13, kernel 6.12.111, PipeWire 1.4.2 and WirePlumber 0.5.8. This supersedes
the earlier VM-only next step without requiring a gaming-PC installation/reboot
or another agent session. The owner explicitly accepted the laptop's first
observed SSH host key; it is pinned privately for subsequent connections.

The owner also requested remote desktop. Existing GNOME Remote Desktop 48.1
remote login was configured after a private baseline, with credentials delivered
through stdin and TLS key/certificate held by the service account. The service
is active; Windows-to-laptop RDP negotiation and the pinned TLS certificate were
verified, and the owner confirmed an interactive login. An unused desktop-sharing
setup stalled on keyring access; its settings were restored to the saved baseline.
After setup, the saved graph/settings and persistent-state hashes compare equal.
No Chrome Remote Desktop, mouse automation or additional desktop was installed.

The owner corrected an incorrectly connected cable. Studio now enumerates as
33AE:4003 at 480 Mbps under snd-usb-audio. Its eight-channel FLOAT_LE ALSA PCM
supports 48000/96000 Hz and implicit feedback. The first native 96-kHz silent
PCM run and a Debian private-core executor check now pass, as recorded below.
Full pre/post graph/settings, persistent files and USB binding are preserved.
The owner reaffirmed authority for necessary login/relogin and reconnect/recovery
steps. The owner has heard both direct-ALSA and bounded assignment-driven
PipeWire first-pair tests. The current checkpoint above records the combined
control/audio evidence and its remaining continuous-streaming limits.
Keep ordinary apply and Actions disabled; preserve all VM evidence. Do not repeat
failed controller/rate tests or the initial audit, or install/run REA.

## Earlier separate native audio, PipeWire and publication checkpoint

The owner-ready 96-kHz direct-ALSA first-pair tone was audible: the owner reported
"i heard it". The original journal retains its terminal-state refusal despite
18 advancing clock observations and child exit 0; do not relabel that journal a
clean harness pass. Its SHA256 is
`44aa2dfb6ec8849168b2d2e7bb77f496f785d28096f297dfc764eaf4d849151f`.
Full graph/settings/routes, persistent-state hashes and released USB binding
matched afterward. This is audible raw first-pair acceptance, not audible
assignment-driven PipeWire acceptance.

The completion guard now recognizes post-drain SETUP only with the retained
child PID and all authored frames consumed, after sufficient prior progress.
Natural exit and cleanup signaling are recorded separately. A fresh silent
96-kHz run naturally exited 0 with no cleanup signal, 18 observations and an
observed DRAINING tail. Its journal SHA256 is
`c9014c044d0f5cc4d6592046b635106b6e8e9da0fbcd494da00887ac2d925f82`.
The SETUP branch has focused owner/progress refusal tests; SETUP was not observed
in that live validation. Original tone evidence remains unchanged.

`tests_native/private_core.py --authorize-studio-first-pair-silence` additionally
requires explicit isolated routing and the selected sysfs device. It refuses
virtual hosts, ambiguous cards, occupied Studio PCMs and unreleased binding.
The owned private core uses 96000 Hz and a retained raw eight-channel ALSA
adapter, with process-verified controller ownership. Only destination A's FL/FR
monitor ports bridge to AUX0/AUX1. Exact observed port pairs and retained proxies
govern the bridge; no numeric-ID/owner-tag deletion authority is added.

The native executor acknowledged the source-to-A stereo route. The selected
FLOAT_LE/8-channel/96000 PCM was owned by the retained private server, with
four seconds of validated queue progress. Actual A-to-B and partial-A/rollback-B
also passed. All owned objects/processes/runtime were removed, Studio PCMs
closed, and full desktop/state/binding comparisons passed. Journal SHA256:
`8a58df68eb2f6a27e214f1e48bc30c0a3676bec7ef4c06ee362b90ed9b1ee376`.
This is physical silent routing of an owned synthetic source. Ordinary desktop
application apply remains disabled; no audible PipeWire acceptance is claimed.

The final guarded harness rerun passed with 19 observations and 376,879 frames
of progress, active A/B routes, partial failure and restored B with new serials.
Its journal SHA256 is
`f8e2fa8e90f8e9372d8e895d2477c29ead113186e3ceaaf3ec911e081bee6fcb`.
The default synthetic mode also passed, journal SHA256
`fea186591f22dfca960dfdcdeedb2f50325bf05289a092581a0bb0731703a56c`.
An intervening progress-journal collision stopped a test cleanly; its evidence
is retained. Individual observations now use exclusive sequence filenames,
with one final aggregate. Missing physical authorization and combined physical/
extended-fault options refuse before device/core execution. Final code source:
`beacn-native-source-20261014T205710Z`; exact native programs built offline and
focused checks passed. All private evidence is copied and hashed outside Git.

Native laptop A0/A1/A2 initialization passed and restored the selected-device
ACL/binding exactly. Initial 40-cycle publication and five-cycle diagnostics
returned only rejected markers; the owner saw no name. The packet/list/polling
source matches the successful VM source; later changes add handshake-only
diagnostic access, not publication encoding changes. A limited diagnostic now
reports only byte 28's nonzero boolean, matching the one-shot example. It was
false both here and in earlier VM-visible evidence; its semantics remain unknown.

An explicit `--single-publication-hold` runner mode requires one C-string hardware
cycle without routing, then holds for 30 seconds without further USB requests.
It preserves all existing device/inventory/cleanup guards and the 50-second
supervisor bound. The first hold still showed no name. After the owner fully
reopened BEACN App on the streaming PC, another hold received valid NoPayload
and the owner saw the name briefly in Applications. Peer state changed alongside
the result; the cause and continuous visibility are not established.

The Debian USB/private-routing runner now explicitly configures DSP FL/FR ports
after verifying each retained controller's client PID and virtual destination.
The no-USB assignment probe passed actual native routes 1 then 2 and verified
publication 0 -> 1 -> 2. Fourteen ALSA, fifteen playback and seventeen publication
runner checks passed on the exact staged source. A native multi-cycle hold
invocation refused before inventory/device access. Private artifacts and owner
observations are preserved outside Git.

The owner-ready native laptop assignment run then received assignment 1 at cycle
3 for the exact owned name before its NUL. The native executor acknowledged two
stereo creates (receipt prefix 2) and verified the actual route. Cycles 4–40
published freshly verified assignment 1. One record reply, seven NoPayload replies
and 32 rejected-marker observations completed across 81 fresh inventories; no
marker semantics are inferred. The owner reported "its in link in". Exit 0,
owned source preservation, private baseline restoration, all three processes
stopped, selected USB ACL restoration and exact connected desktop/state/binding
comparisons passed. Streaming-PC saved settings are outside these local comparisons.
Journal SHA256:
`ddc5cd50b191b73223d8d67b6555ff7228c9b60d37091074fc286c17553541fc`.
The fresh-peer visibility hold journal SHA256 is
`fb0ef38ee01b752428a0d4e44bab453767769b911cdec53b6486153b940c044b`.
This establishes native laptop USB assignment -> private native routing -> verified
USB publication, separate from the physical silent bridge and audible raw ALSA
checks. The subsequent bounded combined result is recorded above; ordinary
application integration remains a later milestone.

## Native Debian PCM and private executor passed (2026-10-14)

Native-only `--native-direct-card` pins the actual selected ALSA card/device
identity without assuming UCM split nodes. It verifies nonvirtual execution,
closed selected playback/capture PCMs, the same retained device inode, complete
foreign graph/settings equality and released USB binding. It does not force a
graph rate or change other devices; the original VM UCM/isolation mode remains.
Raw-card comparison allows no negotiated-format exceptions.

The 48-kHz run stalled at pointer zero but exited and restored the complete
baseline. At 96 kHz the pointer advanced; the first harness refused the normal
DRAINING transition. That observation is retained. The corrected harness accepts
DRAINING only after sufficient validated progress, still checks exact child PID,
and waits for natural exit within the existing bounds. A confirmed closed PCM
can also settle while the same child exits; no exit-code-only acceptance.

The corrected four-second 96-kHz silent run exited 0, made 18 clock observations
and advanced 1,152 -> 380,336 frames. All desktop/settings/routes/state-file/USB
comparisons passed, including complete persistent-state hashes. Journal SHA256:
`6fb895b7eb9366b2aa32d02ee22bff6b5ecdec5c48b56ed766265ab49ac70b1c`.
This establishes native direct-ALSA queue progress and clean completion; it does
not establish audible output or independently observed USB delivery. The clock
descriptor is read-only source 41; one bounded current-rate metadata request
was refused EBUSY with no claim/detach/reset/write. The 96-kHz diagnostic uses
the earlier VM clock observation; a current native clock value is unverified.

Explicit audible authorization now prepares three 350-ms 440-Hz tones capped
at -24 dBFS on channels 0/1 only, with a one-second silent lead and quiet tail.
All other channels stay silent. Tone mode requires its separate authorization
flag and the owner ready to listen. At that earlier checkpoint no audible run had started.

The laptop has isolated Rust 1.88 and Debian build dependencies; the working VM
was not reinstalled. Exact native programs build against libusb 1.0.28 and
PipeWire 1.4.2. Cache-only descriptor validation passes without device access.
Focused playback checks passed 27 cases with one initially skipped built-planner
check, which then passed separately against the new CLI.

Debian's private null sinks initially exposed one unlabelled convert-mode port;
the strict executor refused this layout before routing. The harness now validates
its retained controller's client PID and each owned destination, configures
explicit stereo DSP PortConfig at 48000 Hz, and requires actual FL/FR ports.
Prepared raw private metadata is retained for refusal diagnosis. Native snapshot
refusals now identify missing client/process/port/direction metadata; no guard
was weakened. The exact native A-to-B, partial-failure and rollback check passes,
with actual active links, new rollback serials, private baseline restoration,
owned cleanup and unchanged desktop/state files. Private journal SHA256:
`cc15c535b568024b333ddc74d92f111f1ded5ae5a1584ad580a4e9d07a464d97`.
143 private artifacts were copied/hashed outside Git. This is private-core
routing, not Studio routing. The later VM recovery check could not connect over
SSH; no VM recovery or current process-state claim is made.

## USB return evidence and initialized direct-ALSA coexistence (2026-10-14)

Owner launcher 172808 loaded all 17 probes, passed review and exited 0. At
96000 Hz and 416-byte endpoint sizes, OUT had three entries and three zero returns
at USB-submit, HCD-submit and EHCI-enqueue, but no givebacks/completions. IN had
15 entries/zero returns at each stage and three completions: 24 successful packets,
six empty and 18 nonempty, 7,232 bytes. The three OUT buffers total 9,536 bytes,
matching the ALSA queue pointer 298 -> 298. Successful enqueue is software
acceptance, not hardware delivery. Cancellation returned zero for three OUT and
12 IN URBs; the later OUT dequeue returned -16 (already unlinked), not a new
playback submission rejection. The child remained D / usb_kill_urb. Thirty result
files were retained/hash-verified. Summary SHA256:
`eec7e1b28d9a3fdd63f41ea916e623155b2f9ecac14f60cd50d8e32b17bbaf15`.

Owner reconnect released the child and closed all PCMs; graph/settings/state-file
comparisons passed. Recovery SHA256:
`1b4b998dbcb70b93b96770dcfd1d5b71f07c1f3aea143f21e955b723baa0ac53`.
Windows audio matched; 16 machine-audio registry differences remain preserved and
unclassified. No blanket restore was performed.

The direct ALSA runner now supports caller-owned initialized vendor coexistence,
sharing the existing exact process-start and retained selected-USB-FD checks.
PID/start arguments must be a complete positive pair; identity/FD are rechecked
during activation. These checks never grant authority to terminate the helper.
All 62 focused Linux cases passed on exact source
`beacn-link-physical-playback-20261014T173834Z/source`; 39 unchanged Rust inputs
reuse the validated CLI without a heavy rebuild.

One separately snapshotted initialized 96-kHz direct-ALSA attempt still stalled
at pointer 298. The vendor helper released its claim successfully, but reported
baseline_restored false while audio remained active; aplay 15077 remained in
D / usb_kill_urb. No whole-device recovery is claimed for this latest attempt.
Twenty-seven artifacts were copied/hashed; outer journal SHA256:
`8203690395fb4e6a96d321c4f29cf907efcda0c2f56451c9963a68ef72482be3`;
ALSA journal `3b1ace6bd9299b77c94836e7b784fe2e92296e05da14c51f310b6a3f5bd0eec9`.
Windows endpoint/default settings and registry matched; one unrelated session-list
change was preserved. Three read-only samples found active device/controller power
and a shared IRQ counter unchanged; this is not EHCI-specific causal evidence.
No additional root probe/audio run occurred. An optional bounded root-readonly
VM diagnostic was staged but is superseded by the owner's laptop plan. Verify
VM recovery after the physical move; do not start further VM playback.

## Live USB handoff entries, recovered default and return-code preparation (2026-10-14)

The owner ran launcher 170813. Twelve probes loaded without warnings/stderr,
exited 0 and passed aggregate review. At matching 96000-Hz rates/416-byte packet
sizes, three IN completions again contained six empty and 18 nonempty successful
packets (7,232 bytes). OUT had three usb_submit_urb, usb_hcd_submit_urb and
ehci_urb_enqueue entries, but zero givebacks/completions. IN had 15 entries at
each submission/enqueue stage and three givebacks/completions. One OUT kill entry
occurred at relative second four. These are function entries, not return/acceptance
evidence. OUT URBs each contained eight packets, buffers 3,072/3,136/3,328 bytes,
interval one. Their total 9,536 bytes / 32 bytes per frame = 298 frames matches
the ALSA queue pointer 298 -> 298. This does not prove hardware delivery.

The owned aplay process 13181 remained in D / usb_kill_urb. Twenty-nine result
files were copied/hashed before recovery. Summary SHA256:
`7e20fb44aa455262ddc4a2b2b6d21c011f0a76e838609729945fe2ae9dc08d93`;
journal `1eb87c89d4fcf257b42615b88e519b9843d4e82dd7aff52cbf265222714a7b4e`.
Windows audio and registry match the immediate attempt baseline exactly.

Owner reconnect released all owned processes/PCMs and preserved persistent files,
nodes/ports/links/device settings, but changed live default.audio.sink from Link 1
to Link 4. This was classified before proceeding. Fresh complete guest snapshots,
bindings and endpoint pins were saved, the single old default was restored via
the normal-user metadata API after revalidating its exact current value and
unique original Link-1 target, then all logical metadata/settings/routes/file
comparisons passed. No persistent file was changed by restoration. Thirty-five
recovered artifacts were copied/hashed. Recovery SHA256:
`b49346de6fdb23ec13f0515112ad80290aa04d441861a8cd81fa48f14e5f4001`;
default restoration SHA256:
`77e74075099f1fb49dd793b8a620da0d92d862b715c63aef5484917e1e5efbfe`.
Windows audio matches exactly after reconnect; 18 machine-audio registry
differences remain unclassified/preserved, with no blanket restore.

New optional `--observe-usb-returns` requires the handoff mode and adds an EHCI
dequeue entry plus typed fexit probes for USB submit, HCD submit, EHCI enqueue and
dequeue. Actual guest vmlinux BTF confirms the urb arguments. Typed return arguments
avoid PID/CPU/pointer correlation maps. Bounded negative errno/zero return counts
and successful enqueue packet/buffer/interval/start-frame aggregates distinguish
rejection, a missing return and queued software work from eventual giveback.
Zero enqueue return remains neither delivery nor audible acceptance. The decoder
requires 17-probe coverage, validates matching per-endpoint entry/return counts
and geometry totals, and retains legacy 5/7/12-probe meanings. New kernel fexit
loading remains pending the matched audio-free preflight; no fallback audio retry.
The ALSA journal now explicitly calls its progress an ALSA queue pointer, not
independent delivery evidence. Ordinary physical apply and Actions stay disabled.

All 60 focused Linux cases pass on exact source
`beacn-link-physical-playback-20261014T172651Z/source`; unchanged Rust inputs reuse
the validated CLI. An inert fexit-only code-generation attempt refused on
administrator-only tracefs discovery, before compilation/attachment/audio; it is
not syntax/load proof. The existing SSH context still cannot perform this check.
Fresh guest/Windows snapshots and the byte-verified launcher
`beacn-feedback-ready-20261014T172808Z/run.py` are staged. Launcher SHA256:
`e921f8b4e832a498d11fe473e103a4c235175457522961b989d43f36926666b4`.
Leave Studio attached; source/settings/generation drift and ten-minute expiry
refuse execution. No root/audio run has started. Next owner-terminal execution
first checks all 17 probes without audio, then one bounded 96-kHz child if that
gate succeeds. Collect return/accepted-enqueue/giveback evidence and compare.
Another blocked child requires selected-device recovery before further playback.

## Prelaunch reconnect refused; fresh handoff launcher staged (2026-10-14)

The owner reconnected Studio again after launcher 170306 was prepared. Both
invocations reported only RuntimeError. Read-only verification finds unchanged
source, a nonexpired baseline, changed USB generation, no evidence directory and
no aplay/bpftrace process. No new probe/audio run occurred. Fresh logical node/
port/link/metadata/device settings and persistent-state bytes match the old
baseline despite regenerated identities. Host differences are retained privately.

The private launcher now prints controlled RuntimeError reasons and distinguishes
USB generation drift, settings drift, expiry, changed source and reused evidence.
It retains all guards. Fresh guest/Windows snapshots and a byte-verified replacement
are staged at `beacn-feedback-ready-20261014T170813Z/run.py`, using unchanged
57-case validated source `beacn-link-physical-playback-20261014T170230Z/source`.
Launcher SHA256: `073567620e1fea40818755ca7087dae9ca61bed1feab9555ba743d7c4ecfa983`.
Leave Studio attached when running it; another reconnect requires new preparation.
The ten-minute baseline expiry remains. Twelve-probe kernel loading/USB handoff
observation is still pending; no further playback outcome is claimed.

## Matched-clock input progressed; USB handoff observation is next (2026-10-14)

The owner ran `beacn-feedback-ready-20261014T164723Z/run.py`. Seven probes loaded
without warnings/stderr and exited 0. Device clock and both endpoint configurations
were **96000 Hz**, current/maximum packet sizes **416 bytes**. Three IN completions
contained 24 successful packets: six empty and 18 nonempty, totaling **7,232 bytes**.
All requested lengths were 416 bytes; no packet errors were observed. Three sync
callbacks occurred, all in the first relative second, and no OUT completion was
observed. Unlike the 48-kHz run, this startup received usable implicit-feedback
lengths; the pipeline then stopped. Do not claim sustained transport or audio.

ALSA pointer stayed 298 -> 298 across ten samples. The owned aplay process 11432
remained in D / usb_kill_urb after termination. Twenty-nine result files were
copied/hashed. Summary SHA256:
`637baf5ed39d8cbd6864d94f0fbdbba5b05229f378fbd1f19146edb241f8fb21`;
playback journal:
`82c6231978195faf0f1eccfce426199e3108e877c7a141add0a9e11ba9df6ee7`.
Windows endpoint settings and registry match the immediate attempt baseline;
one session list differs and is retained privately.

The owner disconnected/reconnected Studio. Fresh recovery verifies no aplay/
bpftrace process, both PCMs closed, released interfaces, matching descriptor/
attributes and responsive desktop services. Logical nodes/ports/links/metadata/
device settings and all persistent-state file bytes match. USB generation changed;
identity equality is not claimed. Thirty recovered result files were copied/hashed.
Recovery SHA256:
`d1d2a74b28fc5a8b411dffba1b55a479af66b96895cb6f523d1bdfea615eee0b`.
Windows endpoint defaults/volumes/mutes/formats/states match after reconnect.
Full host audio differs and 17 machine-audio registry differences remain
unclassified and preserved; no blanket restoration occurred.

The next optional `--observe-usb-handoff` mode adds five selected-device ISO
entry probes: usb_submit_urb, usb_hcd_submit_urb, ehci_urb_enqueue,
usb_hcd_giveback_urb and usb_kill_urb. It distinguishes reaching submission,
controller enqueue, giveback and cancellation from ALSA completion. **Entry is
not a successful return or acceptance receipt.** The requested submission geometry
is a bounded aggregate of packet count, buffer byte count and interval; no buffer
contents, per-event rows or addresses are read/exported. usb_submit_urb has not
initialized urb->ep at entry, so endpoint/type filtering uses its initialized pipe.
The mode requires the selected EHCI controller. All five actual guest symbols are
present in vmlinux. Kernel compile/load remains pending the matched preflight gate.

The decoder keeps legacy five/seven-probe evidence separate from twelve-probe
handoff evidence, checks bounded dimensions and per-endpoint geometry accounting,
and reports invalid/missing observations. Full raw baseline objects are now saved
before execution to support logical comparison after device generation changes.
The 57 focused Linux checks pass on exact source
`beacn-link-physical-playback-20261014T170230Z/source`; unchanged Rust inputs reuse
the validated CLI. Fresh guest/Windows baselines and the byte-verified launcher
`beacn-feedback-ready-20261014T170306Z/run.py` are staged. Launcher SHA256:
`bc06f5bff9073c8beaab0ed21dd3b8776a7bbf3dcadaf019fe013a88dc5296f2`.
Its baseline expires after ten minutes; source/settings/generation drift refuses
execution. No root/audio execution has started. Next run one 96-kHz handoff
observer through the owner's terminal sudo context, then collect and compare. Keep
ordinary physical apply off and Actions disabled. Earlier staged/pending notes
below are historical; the matched-rate run and its recovery are complete.

## Matched-rate recovery verified; packet check staged (2026-10-14)

After the owner's VMware disconnect/reconnect, read-only recovery confirms no
aplay/bpftrace process, closed playback/capture PCMs, released interfaces, matching
descriptors/attributes and responsive existing desktop services. Logical nodes,
ports, links and defaults match the saved pre-attempt graph; all persistent-state
files match exactly. USB generation changed as expected, so do not claim numeric
identity equality. Nineteen recovered artifacts were copied/hashed. Recovery SHA256:
`2d71482cc7bdb498dff7f0d06beb99c76a4dbdf2728b18358d45bbe887e9ac30`.

Windows endpoint defaults/volumes/mutes/formats/states match. Full Windows audio
has one session-list difference; registry has 16 changed machine-audio values,
each a 16-byte binary value. They remain unclassified and preserved privately;
no blanket restore or full host byte-equality claim. A fresh host baseline is saved
for the next diagnostic.

The explicit matched-clock observer is staged at
`beacn-feedback-ready-20261014T164723Z/run.py`, using the 54-case validated source
`beacn-link-physical-playback-20261014T164016Z/source`. Launcher SHA256:
`c7921131760f98800c3b33858465cf3614871d53ccc04b78dc4f8be78c8f2b56`.
It pins the prior measured-clock evidence and fresh released binding/settings;
the baseline expires after ten minutes. Source/binding/settings drift refuses
execution. No root or audio execution has started. Next owner-terminal sudo
execution observes actual rates, requested packet geometry and errors at 96 kHz.
Then collect/hash and compare; if owned cleanup blocks, stop playback and recover
the selected device before more tests. Actions remain disabled, ordinary physical
apply remains off. The pending-recovery statements below describe the earlier
failed attempt and are superseded by this recovery checkpoint.

## Measured clock mismatch and matched-rate failure (2026-10-14)

The owner ran `beacn-feedback-ready-20261014T153914Z/run.py`. The corrected
observer loaded seven probes, exited 0 without warnings/stderr and validated
complete metadata coverage. It observed the driver's existing clock-41 getter
return **96000 Hz**, while IN/OUT configuration remained **48000 Hz**, current
packet size **288 bytes**, maximum **416 bytes**. All 11,472 requested IN packets
were 288 bytes: 11,308 EOVERFLOW, 158 EXDEV and six successful empty packets.
There were 1,434 noncancelled IN completions, 12 cleanup cancellations, 1,434 sync
callbacks, zero successful bytes and no OUT completions. ALSA still stalled at
pointer 0 -> 0; collector validation is not audio acceptance.

Immediate/fresh guest settings/routes/files/binding comparisons pass exactly;
no owned process remains from that run. Windows registry/defaults/endpoint
volumes/mutes/formats/states match. Full Windows audio differs because three
sessions disappeared across two endpoints, with no changes to surviving sessions.
Those private differences are preserved; do not claim full byte equality or
restore unrelated sessions. Twenty-nine result files were copied/hashed.
Summary SHA256: `8c390897aed8710e1a649e458c5bd1bc549f63b9394c0d407e18e0a8e2d23c81`;
playback journal: `52a3be1279c3df43c05377487c0265141c0fa7b6f38962d0cc3fa43c0e596c9d`.

This measured mismatch justified one new direct-ALSA diagnostic at 96000 Hz,
not another blind PipeWire force-rate trial. The runner now has an explicit
`--matched-device-clock-96000` opt-in, retaining 48000 by default, finite
eight-channel FLOAT_LE authored silence, conversion refusals and existing
identity/foreign-graph/cleanup guards. The private wrapper hash-checked the
prior clock report and required its selected binding to match before execution.
Exact source: `beacn-link-physical-playback-20261014T163645Z/source`.

That four-second silence attempt also stalled: ten samples at pointer 298 -> 298.
The owned aplay process 10595 remained in D / usb_kill_urb after termination;
existing PipeWire/WirePlumber processes remain sleeping/responsive. Windows audio
and registry match the fresh attempt baseline exactly. Guest comparison refuses
because the target is not released. **Recovery is pending; do not start another
playback or claim owned cleanup/settings restoration.** Eighteen attempt files
were copied/hashed before recovery. Journal SHA256:
`0e54af353e5d89032204d069004830b19378d9bce1af3413c159c20c0daabead`;
read-only recovery SHA256:
`42b6a86fa60f5f5c437615562f8a5cea9cee9fed2e815aca14c66288010940d5`.

The owner was asked to disconnect/reconnect Studio once through VMware, keeping
Windows Link closed. Next verify that the owned process exited and compare the
saved guest baseline using fresh identities. Only after recovery may the matched
96-kHz observer check actual device/configured rates, requested packet sizes and
completion errors. Matching the reported clock alone did not solve playback;
failed-packet received lengths remain unknown. No driver/firmware change, payload
capture, ordinary physical executor apply or audible acceptance occurred.

The aggregate runner now exposes nested playback error/child-exit fields in its
summary and propagates the explicit 96-kHz diagnostic to its non-root child.
Actions remain disabled; local checkpoints use [skip ci]. Earlier pending notes
below are historical and superseded by this checkpoint.

## Clock/geometry collector load failure and corrected preflight (2026-10-14)

The owner ran `beacn-feedback-ready-20261014T152029Z/run.py`. Its audio-free
attachment check exited 0, but the actual collector exited 255 before any audio
marker or playback journal. Retained stderr reports BPF program-load error -12
(ENOMEM) for the snd_complete_urb probe. This is an observer-load failure, not a
new physical playback outcome. Immediate/fresh guest comparisons and Windows
audio/registry comparisons pass exactly; no aplay/bpftrace process remains.
Ten private artifacts were copied/hashed. Summary SHA256:
`8ce2438793f3d185da558a9954049acf6dec8a97e86895ad89a2339daae604b9`.

The preflight omitted the actual run's `-k` helper-warning instrumentation and
`-c` child context. It therefore did not validate the same load configuration.
The precise allocation/verifier/JIT cause remains unknown without verbose logs;
do not assert that the VM exhausted all RAM or that the device failed. Both
phases now use one command builder, differing only by `--dry-run`. The versioned
bpftrace implementation returns from dry-run before running a non-USDT child;
these probes contain no USDT. One failed preflight may collect a verbose,
audio-free load diagnostic, then stops; it never retries audio or disables warnings.

The packet guard/unrolled body is narrowed from 64 to eight, matching every
observed callback in the first live run and the upstream capture/implicit-feedback
one-millisecond cap at this endpoint's 125-us interval. Larger counts become
explicitly unsupported, rather than silently truncated. This reduces static
unrolled work; successful kernel loading/memory benefit remains unverified.
The decoder checks the eight-packet bound for new runs while preserving legacy
64-bound evidence. Nonzero collector exits now populate `error`; a launch marker
without a final journal leaves the audio outcome unknown, not false success.

All 51 focused Linux cases pass on exact code source
`beacn-link-physical-playback-20261014T153912Z/source`; new regression cases cover
matched preflight/runtime arguments, failed-exit reporting and packet bounds.
Unchanged Rust inputs reuse the verified CLI. No heavy build or installation.
Private next launcher: `beacn-feedback-ready-20261014T153914Z/run.py`, byte-verified,
with a fresh Windows baseline. The same bounded clock/request-geometry check is
pending owner-terminal execution; keep Studio attached. Collect/hash outputs and
compare guest/host baselines afterward. Do not claim the revised observer loads
or that physical audio works before that evidence exists.

## Live feedback error checkpoint and clock/geometry preparation (2026-10-14)

The owner ran the prepared Ubuntu terminal launcher. Compilation/load/attachment
passed; five probes attached, the collector exited 0 and the normal-user ALSA
child exited after its two-second pointer-stall guard. Observer `error: null`
means collection completed, not that audio succeeded. Nine playback samples
remain pointer 0 -> 0. Retain the original `live_probe_validated: false`: it was
an intentionally conservative default awaiting aggregate review.

Aggregate IN `0x82` observations: 1,437 noncancelled URB completions, all URB status
0, plus 12 cleanup cancellations. Their 11,496 packets comprise 11,490 EOVERFLOW
(-75) packets and six successful empty packets. Successful bytes total zero;
sync-handler callbacks total 1,437. No OUT `0x02` completion was observed.
This supports the feedback queue-wait explanation, while device/driver/VMware
root cause remains unresolved. Two printed signedness warnings concern the loop
comparison; observed totals consistently account for eight packets per callback.
Keep these warnings and original output. The revised loop casts packet count to
uint64 after retaining its 1..64 guard; negative counts become out-of-bound.

All immediate and fresh recovery guest comparisons pass exactly. No aplay or
bpftrace process remains; existing PipeWire/WirePlumber processes are responsive.
Windows audio and registry match the fresh pre-run baseline exactly. Twenty-seven
private result artifacts were copied and hashed. Summary SHA256:
`ff043e7ed1ea811ded5332f445c5a6ed87cdee671a275d9e20f4e51f5c6dae70`;
playback journal SHA256:
`88a369d5b8795eee57933e05ed1f92794f91c1946d1a7c5dc1b14764eadde57c`.
Old restart-wide unclassified state differences remain preserved separately.

Upstream EHCI maps its babble bit to EOVERFLOW and does not update actual_length
for that error; never interpret failed-packet zero lengths as received sizes.
Cached UAC2 descriptors show one clock, ID 41, attributes 3, controls 0x05, shared
by all four terminals. Frequency/validity controls are advertised read-only.
Upstream clock setup can continue after a requested/device-rate mismatch on a
read-only clock. These are reference-code/descriptor observations; the device's
actual rate and equivalence to Ubuntu downstream behavior remain unmeasured.

Next observer version adds bounded requested-length histograms, configured IN/
OUT rates/packet sizes and aggregate return values from the driver's existing
get_sample_rate_v2v3 clock-41 read. It issues no extra USB requests, reads no
payload buffers and makes no rate/driver/firmware change. The actual kernel BTF
contains the required endpoint fields and clock-reader function. Seven probe
attachments are now required. `tools/usb_feedback_review.py` validates bounded
JSON/maps, dimensions, counts, coverage flags and explicit unknowns; warnings in
stdout are retained even when stderr is empty. No exit code alone proves audio.

All 48 focused Linux cases pass on exact code source
`beacn-link-physical-playback-20261014T152027Z/source`, including parser refusals and
actual cancellation signals. Unchanged Rust inputs reuse the verified CLI.
New kernel compilation/attachment and geometry/clock measurements are pending.
Private next launcher: `beacn-feedback-ready-20261014T152029Z/run.py`, byte-verified;
it first validates source hashes and the audio-free attachment gate, then runs
the bounded silence diagnostic as the normal desktop owner if the gate passes.
A fresh private Windows baseline is saved. The SSH sudo context is still denied
after the owner's terminal authentication; use that terminal launcher again,
without changing sudoers/SSH permissions. Keep Studio attached and collect/hash
the new evidence plus fresh guest/host comparisons after execution.

## Aggregate completion observer preparation (superseded by live checkpoint above)

`tests_native/studio_feedback_stats.py` generates selected-device-only aggregate
kernel observations for Studio OUT `0x02` / implicit-feedback IN `0x82`. It reads
completion status, bounded per-packet status/length fields and counts sync-handler
calls. It never accesses transfer/setup buffers or emits per-event rows. Counts
are grouped by relative second and endpoint; packets beyond the 64-entry bound,
unexpected lengths and other errors remain explicitly unclassified. Cancelled
completions are counted but their stale packet fields are excluded. These are
transport/driver metadata, not audio/USB payload capture or delivery acceptance.

Normal preparation is metadata only. Authorized execution first requires a
successful `bpftrace --dry-run` compile/load/attach check with no audio child.
Then bpftrace starts the existing guarded finite ALSA silence runner only after
its probes attach, using `setpriv` to return audio execution to the desktop owner.
The collector has a 30-second ceiling and retained-process time/output guards.
ALSA now handles SIGTERM as a cancellation request, leaving owned child cleanup
reachable; a Linux test verifies two actual signals including one during cleanup.
Root-child outcome/aggregate coverage never become verified merely from exit 0.

The VM has bpftrace 0.25.0, bpftool, kernel/module BTF and both target symbols.
The existing SSH account has no effective capabilities or passwordless sudo;
unprivileged BPF is disabled and tracefs is administrator-only. Unprivileged
compilation stops at function/event discovery, before bytecode validation.
The noninteractive attachment gate was exercised: it refused before any audio
child, and full guest settings/routes/file/binding plus Windows audio/registry
comparisons passed exactly. Gate summary SHA256:
`49eb91e43e0ed54ac41371575ec6d58c88c92e01eb7749c0afba4f355479b6a5`.
No new physical playback, live probe attachment or kernel compiler success is
claimed for this batch. No sudoers/capability/SSH/driver changes were made.

All 43 focused Linux checks pass on exact code source
`beacn-link-physical-playback-20261014T144527Z/source`; unchanged Rust inputs reuse
the prior verified CLI with provenance. Guest free disk remains about 18.1 GB;
no installation or heavy build occurred. A single normal-user Ubuntu terminal
launcher has been staged and byte-verified, with a fresh private Windows baseline.
Private launcher label: `beacn-feedback-ready-20261014T144550Z/run.py`;
SHA256 `19f3ba63c2132710019d70f0282b5d2cc4d0c44dcc5070bc8d9c0cccf677e9ed`.
It checks the exact prepared source hashes, prompts for sudo in Ubuntu only,
performs the attachment gate and runs the bounded counter/silence diagnostic if
that gate passes. Next obtain administrator execution via that owner-terminal
launcher or an existing authorized administrator path. The access method is
missing; owner authorization for the bounded test already exists. Never request
a password in chat or broaden permissions. After execution, use strict SSH to
collect/hash private results and compare the retained fresh host baseline.

## Latest VM-only submission/clock diagnostic (2026-10-14)

The existing physical playback activation owner now records the bounded cached
sysfs `urbnum` statistic with its identity-pinned alternate/endpoint observations.
Both PipeWire and direct ALSA runners already use that owner, so each clock
record includes the counter. It is device-wide cumulative submission metadata,
not an outstanding-request count, completion count, feedback payload or proof of
audio delivery. Missing counters remain unknown; malformed/wrapped values fail.
Retained ALSA stream metadata explicitly reports sync endpoint `0x82`, interface
2 alternate 1 and `Implicit Feedback Mode: Yes`; do not enable a generic driver
override merely because the module's override parameter is false.

All 39 focused playback/graph/ALSA/handshake cases pass on Linux. Exact validated
code source: `beacn-link-physical-playback-20261014T140703Z/source`; unchanged Rust
inputs reuse the prior verified CLI with provenance. Guest free disk is about
18.2 GB; no heavy build or installation occurred.

A new authorized, guarded direct ALSA EHCI 48 kHz authored-silence diagnostic
collected nine clock samples. Both audio alternates were 1, submissions rose
146 to 1,564 over 2.053 seconds, but hardware pointer stayed zero. The two-second
stall guard stopped the owned child, which exited. Full guest settings/routes,
saved files and USB binding compare exactly; Windows audio and registry compare
exactly within this attempt. Seventeen private artifacts were downloaded and
hashed. Journal SHA256:
`bf07a5f30ecb73e90ebff48b81958471b0dbe14a2bbd6c51a1a13592aa235f84`;
comparison SHA256:
`c0d6438294a753d50922b419aaf036eb2f2f42a42dba81a1b32d07433122dcd3`.
Read-only ten-sample observers before/after remain fully unchanged, with idle
counters fixed at 115 and 1,569 respectively. No matching timestamp window was
present in the existing VMware logs; that is a visibility gap, not error-free
transport evidence. No packet/audio capture, driver change or audible test ran.

Next inspect the implicit-feedback queue/clock path and available existing
transport/status metadata to distinguish continued submissions from useful
feedback/completions. Do not repeat controller/rate variants or claim the rising
counter proves delivery. No owner watching, dragging or manual USB toggle is
needed for this investigation; Studio remains attached and recovered.

Follow-up upstream v7.0 source review identifies a plausible implicit-feedback
queue wait: empty/error-only incoming completions can produce no output layout
while input requests continue to be resubmitted. This is a hypothesis, not a
measured completion status or verified match to Ubuntu's downstream patches.
Retained status confirms the app wrote 24,000 frames (`appl_ptr`), with pointer
zero and delay 24,000. The upstream PCM pointer advances while preparing output
requests; call it driver queue progress, not independent hardware delivery proof.
Existing kernel journal has no timestamped records for the attempt window.
Assess aggregate feedback/completion-status observability before more playback.
See the linked playback document for exact reference sources and limitations.

The optional prepared `tests_native/studio_native_preflight.py` is read-only: existing durable snapshot
and cached USB/endpoint owners, one selected card, four actual pair mappings,
normal device access and explicit native-versus-VM detection. It starts no audio
and changes no routing. Its readiness flag is for a separate guarded test, not
audio or full-product acceptance; other physical devices can block the existing
isolation guard. Unknown/virtualized contexts never claim native readiness.
Two focused cases pass on Windows/Linux. The eight-source-file portable ZIP
imports and runs on Linux, saves a stable baseline, verifies four endpoints and
correctly rejects the VM as native. Its manifest/allowlist/source hashes and ZIP
integrity are verified. `tools/package_native_preflight.py` rebuilds it outside
the repository with no Rust/proprietary binaries, credentials or private evidence.
Prepared private ZIP label: beacn-native-preflight-20261014T1330Z.zip;
SHA256 `710d97e1deaeda047655ec67717e603beef2130d1d4bb1c88498ce4f240e3790`.
The VM remains on recovered EHCI. A fresh host PnP query finds no present
33AE:4003 instances, so there is no evidence of native Windows interfaces still
owning this selected device; this does not prove isochronous delivery works.

## Current controller recovery and durable evidence checkpoint (2026-10-14)

**EHCI restored; no further physical playback.** The xHCI comparison lost SSH
and the guest-tools heartbeat during the same direct ALSA silence diagnostic.
Its completion, clock progress and initial cleanup are unconfirmed. A graceful
stop timed out; the dedicated test VM required a recorded forced power-off.
The owned xHCI/PCIe trial fields were removed and original EHCI slot 34 restored.
The owner logged into Ubuntu again. Fresh read-only comparison passes selected
`ehci-pci` at 480 Mb/s, released USB profile, cached descriptor hash, all four
endpoints, logical nodes/ports/links/defaults and saved-file values/permissions.
`stream-properties` differs only in JSON member order; retain its raw diff.
Windows endpoint defaults/volumes/mutes/formats/states match, while two session
lists, sixteen existing binary registry values and one added policy entry differ
across the controller/recovery restarts. Preserve those unclassified differences;
do not claim full byte-identical restoration or overwrite unrelated state.

The owner-requested power-on connection automation is now verified: Studio
reattached without a manual USB handoff on both controller starts. The retained
rule matches VID/PID plus the exact cached host port; its machine-specific value
and VMX backups remain private. This is power-on autoconnect, not a live USB reset
API. Windows Link stays closed and Studio remains attached to the recovered VM.
No mouse automation, driver/firmware change, incoming capture or REA was used.

The forced stop exposed an evidence durability defect: nine newest guest attempt
files survived as empty files, including its baseline/PCM and four clock records.
Preserve them and the transport-loss marker; never derive sample progress from
their filenames. Complete pre-controller backups had already been copied to
Windows and remain intact. The existing shared evidence writer now exclusively
creates and flushes file data and POSIX directory entries before execution.
Pinned descriptor files and authored PCM reuse it; a failed flush prevents native
child launch. All 37 focused handshake/playback/graph/ALSA Python cases pass on
Linux; Windows passes 35 with two existing skips. This is offline validation,
not a new forced-crash or successful hardware run. Exact source:
beacn-link-physical-playback-20261014T131759Z/source. Unchanged Rust inputs reuse
the prior exact built CLI with recorded provenance. Free guest disk: about 18.2 GB.

Fourteen final recovery artifacts were copied and hash-verified on Windows;
final recovery SHA256:
`cdbc18f84d0f8056b84f9a1a1fadec981315cd227a566b2bb9676a25ffe8e7b1`.
Read [native-studio-playback.md](native-studio-playback.md) for details. Continue
the VM investigation under the owner constraint above; the prepared native
preflight is optional deferred work, not a VM test gate. Do not repeat failed
controller variants or visibility/drag tests. This supersedes
earlier next-step/recovery instructions below. Ordinary physical native route
application remains disabled.

## Current recovery checkpoint (2026-10-14)

**Recovery verified; physical clock remains unresolved.** Read
[native-studio-playback.md](native-studio-playback.md) first. New cached USB
observations prove both playback and implicit-feedback audio alternates active
during the 48 kHz stall. A maintainer-supported, temporary owned-stream 96 kHz
experiment selected the requested ALSA rate but advanced only 298 samples before
freezing. Its playback child exited; PipeWire subsequently stopped answering
queries and is in kernel state D at `usb_kill_urb`, with audio alternates still 1.
The post-test graph query failed; preserve that failed comparison. On October 14
the owner disconnected and reconnected Studio. The same PipeWire process left
kernel state D and resumed answering without a service restart. Fresh comparison
passes logical nodes, ports, links, defaults, all four saved files byte for byte,
cached descriptor hash, original rate overrides and released USB interfaces.
The owned playback node is absent. Windows audio state matches exactly, while
the USB cycle left seventeen registry-value differences (sixteen binary values
in machine audio and one added string in user policy); retain these unclassified
differences and full backups, not a blanket restoration claim. Studio stays on
the VM and Windows Link stays closed. No mouse automation, capture, driver change
or further rate-workaround playback was performed. A subsequent direct ALSA
silence diagnostic (below) narrows the clock stall beyond PipeWire playback.
Do not repeat the failed rate test.

Ignored `local/physical_rate_recovery.py` has compared the recovered
desktop against the pre-rate-test full backups, including logical foreign
resources/defaults and absence of the owned force-rate node. Existing private
evidence is preserved. The runner now records USB activation, supports explicit
48/96 kHz PCM, restricts temporary forced rates to the selected idle test graph,
and fails comparison promptly if the desktop is unavailable. Twelve playback and
nine graph Python cases pass on Windows/Linux; unchanged Rust inputs reuse the
previous exact built CLI. New offline source:
beacn-link-physical-playback-20261014T040623Z/source. Latest free disk: about 18.3 GB.
This recovery checkpoint supersedes the earlier exact-cleanup status below.

**Direct ALSA diagnostic completed:** new `tests_native/studio_alsa_playback.py`
uses one retained `aplay` process, explicit raw hardware selection, eight-channel
FLOAT_LE 48 kHz authored silence, four-second input and bounded clock supervision.
It refuses an occupied card or competing app/device/rate override, verifies that
the unchanged PipeWire graph remains foreign, and checks actual PCM owner/format
and cached USB activation. Source
beacn-link-physical-playback-20261014T124203Z/source passes 24 focused Python checks
on Linux; Windows passes 22 with two existing Linux-only skips. Unchanged Rust
inputs still reuse the exact prior built CLI. The hardware run verified ten
observations but stayed at hardware pointer zero and stopped after two stalled
seconds. It created no PipeWire playback stream or native routing receipt.
Owned child exit, desktop/routes/defaults, saved files, released USB binding and
per-attempt Windows audio/registry comparisons all pass exactly; the same
PipeWire server remains responsive. No additional reconnect is needed. Raw
evidence and hashes are preserved privately. This narrows the failure to the
ALSA/USB/device path but does not identify Linux versus VMware versus the device.
Next isolate VM USB controller/passthrough behavior; do not rerun visibility/drag
tests or claim physical/audible output.

## Current hardware checkpoint (2026-10-13)

**Physical playback probe implemented; audio clock blocked:** read
[native-studio-playback.md](native-studio-playback.md) for the current test owners,
flags, exact evidence and unresolved clock path. The complete physical split graph
now projects through existing owners; all four stereo pairs and Link 1's actual
links verify. The selected VM now has backed-up USB 2.0 EHCI/PCI-bridge configuration
and Studio remains attached through `ehci-pci`. No mouse/UI automation was used.
Native initialization succeeds with the explicit handshake-only initial-read
diagnostic, including a thirty-second held claim. Neither immediate-release nor
held initialization cures the physical PCM stall. The final silent run stopped
after a three-second zero-progress hardware clock; owned PCM/native children
exited, vendor cleanup passed, and final connected guest/host comparisons matched
exactly. No audible run occurred. Reboot-wide logical settings match, but JSON
ordering, one host session activity state and sixteen registry properties differ
and remain preserved. Read the new doc before interpreting those comparisons.
Exact source: beacn-link-physical-playback-20261014T032358Z/source; fifteen focused
Python and thirty-five Rust gate/session checks pass on Windows and Linux. Latest
guest free disk is about 18.4 GB. Do not redo visibility/drag tests; investigate
USB audio clock/implicit feedback and VM isochronous delivery next. Ordinary
desktop/native physical apply remains disabled; private executor guards are intact.

**Native assignment integration implemented:** read
[native-assignment-publication.md](native-assignment-publication.md) for current
owners, explicit flags, checks and hashes. The new path applies normalized received
assignments through the existing native executor on the selected owned private
core, then derives each outgoing record from fresh verified actual routes and
native receipts. It continues all 40 cycles instead of ending at the first
assignment. Default publication/ordinary desktop apply remain unchanged.

An isolated native probe passed real routing 0 → 1 → 2, receipt prefixes 2 and 4,
source preservation, owned cleanup and all connected local comparisons, without
USB transfers. Final source adds one-shot proof enforcement and a diagnostic-line
clarification and builds at beacn-link-usb-publication-routed-20261013T142858Z/source.
Windows/Linux pass 24 route/publication cases; Linux passes all 27 Python runner
cases (Windows passes 25 with two Linux-only skips). Existing native routing code
was reused, not replaced. Free guest disk is about 19.4 GB; lightweight builds only.

**Integrated hardware test passed:** the owner reported that the Linux test
entry turned yellow and entered the first Link In channel. Native assignment 1
arrived at cycle 9, the existing executor completed two acknowledged stereo-link
creates, and fresh route verification permitted assignment-1 publication at
cycles 10–40. Two repeated assignment-1 replies completed with receipt prefix
zero, issuing no additional route operations. All 40 cycles and 81 fresh
supervised inventories completed. A2 took 42 ms. Source preservation, vendor
claim release, three owned-process stops, private-core restoration and connected
guest binding/desktop/state plus host audio/registry comparisons all passed.
This proves remote UI assignment acceptance and native private-core execution/
verified publication together; physical Studio audio and prolonged UI lifecycle
remain unverified. See the integration doc for the private journal hash.

The ignored
`local/routed_connected_attempt.py` pins the final executable/runner and uses
fresh connected host/guest snapshots, bounded execution and connected comparisons
through strict SSH. It can run `--probe run-attached` without USB I/O; the completed
probe is preserved under its earlier source/evidence state. Announce start/end and
leave Studio connected. No computer-use/mouse automation. Routes in this batch
target owned virtual sinks, not physical Studio audio endpoints.

**Next: physical audio integration.** Read-only metadata identifies one Studio
audio device (33AE:4003), four stereo Audio/Sink nodes described as BEACN Studio
Link Link 1/2/3/4, four stereo sources and internal eight-channel sink/source
nodes. No PCM/USB I/O was requested by this readiness inspection. Names are
descriptive context, not deletion/route authority; verify exact fresh device,
node/port serials, pair mapping and current foreign links before any physical
test. The current native executor still intentionally accepts only its owned
private core/synthetic nodes. Do not bypass that guard or claim the virtual
destinations already feed Studio. Prepare the next explicitly selected physical
audio path and short audible test while preserving settings/unrelated work.
Studio remains connected to the VM; no per-test handoff or mouse automation.

The explicit initialization diagnostic now permits a single A2 read up to five
seconds, while all writes and later reads retain their 500 ms limits. Default
initialization remains 500 ms. No timeout retry or invalid-A2 bypass exists.
The earlier connected attempt stopped at A2 with libusb TIMEOUT (-7), zero of
512 bytes received. A fresh diagnostic attempt received a valid A2 in 669 ms
and completed 40 publication/poll cycles: seven NoPayload replies, 33 rejected
markers, 81 fresh inventory requests, no assignment records. This proves that
a valid response can exceed 500 ms in this VM, not a firmware timeout requirement.

Two subsequent owner-observation attempts completed all 40 cycles. A2 took
439 ms and 584 ms respectively. The first received two one-record replies,
neither matching the advertised test name; the second received no records.
Names were redacted, so these journals cannot identify the unmatched records.
The owner saw BEACN Linux VM Test but could only assign it to Aux 2, Music,
System or Aux 1, not Link In. The owner did not try Aux 2 because that was not
the intended goal. Do not report successful assignment or change the acceptance
target to an arbitrary local mixer channel. Resolve source classification and
the remote assignment mapping before another owner drag test.

Every attempt saved fresh connected guest/host baselines. Owned claims released,
both private processes stopped, the private graph restored, and connected guest
binding/desktop graph/persistent files plus host audio/registry matched exactly.
These comparisons cover the VM and gaming/host PC, not the streaming PC's UI or
profile state. Studio remains connected to the VM. No physical return comparison
has run this session; prior pre-handoff evidence is preserved separately.

Before every observation test, give the owner the exact place to watch and the
purpose of the requested action, wait for readiness, announce the start and end.
Do not ask retrospectively whether they watched an unannounced run. No computer
use or mouse automation is permitted. Windows Link stays closed; streaming-PC
BEACN App stays open. Necessary VM/hardware continuation is already authorized.

Exact source: beacn-link-usb-publication-initialization-20261013T131603Z/source
in the existing guest home. Windows/Linux passed 42 focused Rust integration
cases; Linux also passed two native unit checks and all 24 Python runner cases
(Windows passed 22 and skipped two Linux-only cases). Both native examples and
the default build passed. Free guest disk: 20,186,005,504 bytes; use lightweight
single-job builds. See publication/session docs for provenance and hashes.

The reply diagnostic now reports assignment bytes, name lengths, exact bound-name
matches, embedded-NUL counts and printable-ASCII status for at most four records
per reply. All names remain redacted; extra records are counted without expanding
the log. It does not change decoding, name bindings, planning or USB scheduling.
The exact native example built in the guest; this diagnostic has not run on hardware.
Source: beacn-link-usb-publication-reply-20261013T133631Z/source. See publication
docs for hashes. No unchanged test matrix or hardware attempt was repeated for it.

**Resolved name interpretation (2026-10-13):** the owner-ready prefix diagnostic
received eight assignment-1 records whose first 19 bytes exactly matched the
owned test name, followed by a NUL and five opaque bytes. An explicit
`ReceivedNamePolicy::WindowsCString` now adapts received names at the planning
boundary; wire decoding retains all raw bytes and default matching stays exact.
The existing planner still guards exact inventory names, stream liveness,
duplicate/conflicting assignments, opaque zero/unsupported values and generation.
Windows/Linux passed 19 focused session tests including four new compatibility
cases. Linux passed all 25 runner cases; Windows passed 23 with two Linux-only
skips. Default and exact native builds passed.

A separate owner-ready compatibility hardware run recognized assignment 1 at
cycle 5 and produced one existing-planner proposal for the owned stream. It
stopped publication, held five seconds, rechecked inventory, released its claim
and cleaned up; all connected local comparisons passed. The owner reports the
entry disappeared while being dragged, so UI retention/drop acceptance remains
unverified. No route was applied. Source:
beacn-link-usb-publication-c-string-20261013T135803Z/source. Free guest disk:
19,808,038,912 bytes; continue lightweight builds. Read publication/session docs
for hashes, explicit flags and limits.

Completed native integration above supersedes this prior next-work note: keep the bounded test source alive during interaction,
apply a resolved assignment only on the selected private core through existing
native observations/preparation/receipts, and advertise an assignment only after
actual local-route verification. Do not keep sending unapplied zero over a
received assignment or claim physical audio from a virtual target. Physical
BEACN endpoints and ordinary desktop apply remain outside this private-core slice.

Historical preceding diagnostic: owner again reported Link In unavailable. Three
replies carried assignment 1 and 25-byte names with one NUL, none matching the
19-byte test name exactly. A new diagnostic reports first-NUL position, opaque
tail length and whether the preceding bytes match the owned name. Its native
build passed; a passive run received no records, leaving that comparison untested.
All connected local comparisons and owned cleanup passed in both runs. See
publication/session docs for hashes and focused static string-copy/comparator
evidence. Latest source: beacn-link-usb-publication-reply-nul-20261013T134722Z/source;
free guest disk is about 20.0 GB. Codec and planner still preserve exact raw names.

The prefix diagnostic and name-resolution hardware milestones above supersede
the prior next-work note. Resolve why the visible source is
not assignable to Link In, then
test a correctly identified remote assignment. Receiving and resolving that
assignment precedes physical audio routing. Ordinary desktop apply stays disabled.
The end goal remains Linux applications controlled from Windows BEACN App with
verified audio through Studio Link; visibility alone does not meet it.

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

**Completed publication implementation:** an explicit native mode permits one
pinned single-app list and one assignment poll through the existing owners.
Default handshake-only access stays restricted; zero stays opaque and no routes
are applied. Windows/Linux focused tests/builds pass. An actual owned `pw-cat`
stream in a private core was discovered/bound to `BEACN Linux VM Test`; the source
probe and cleanup restored the private graph and desktop/state files exactly,
without USB access. Read [native-usb-publication.md](native-usb-publication.md)
for source owners, final hashes, supervision and remaining limits.

**Current testing constraint:** Windows BEACN Link on the gaming/host PC must be
closed; the owner closed it and the process check confirmed none. The replacement
runs in the Linux VM, while BEACN App remains on the Windows streaming PC. Verify
Link remains closed before each publication attempt; do not launch it to make
Linux interoperability appear to work.

**Completed hardware publication:** Windows Link stayed closed and the Linux
replacement completed an app-list write. The first immediate poll returned
ACFFFFFF; cleanup passed, with visibility unobserved because the owner did not
know where to look. The owner supplied a BEACN App v1.4.0 screenshot; identify the
bottom ASSIGNMENTS / Applications box containing `obs64 System`, immediately
right of Outgoing Studio Link Assignments, before the next observation.
A separately built diagnostic example waits 500 ms before its one poll and
reports only the unknown boolean at A2 byte 28. The controlled next attempt
returned a valid no-payload poll (512 bytes, 508 ignored tail) and exited 0 with
verified device/private-process cleanup. A2 byte 28 was zero; its meaning is
unknown. The owner saw `BEACN Linux VM Test` briefly in that remote App box.
No assignment records or audio interoperability are established.

That second attempt used a fresh connected-state protocol baseline because the
owner had already reattached. Preserve the distinction from the earlier full
pre-attachment/returned references. The connected guest had one BEACN ALSA device,
22 nodes/76 ports/16 existing links; native protocol execution preserved its
entire graph and state files exactly. After owner return, Windows audio settings
and guest returned graph semantics match the prior returned reference; guest
state files match the fresh baseline. Cross-handoff exact comparisons differ by
device presence and registry values. All evidence is retained, with no rewrites.
Read [native-usb-publication.md](native-usb-publication.md) for exact outcomes,
comparison scopes and hashes. Studio is back on Windows.

**Periodic implementation completed, hardware pending:** `native_usb_periodic`
opts into 1..40 publication/poll cycles using the existing selected-device owner.
Fresh, strictly increasing inventories are requested after initialization, before
each publication, and after each reply. The runner checks its retained private
core/playback processes, exact application/stream identities, live state and no
private links. Frozen/rebound/disappeared sources or any failed exchange halt;
there is no retry or reconnect. Defaults remain handshake-only/one-shot.
Known bound assignments 1..4 stop further zero publication and are planned against
the fresh inventory without applying routes. The child is externally supervised
for 50 seconds with bounded IPC and retained-process cleanup.

Windows and Linux focused checks passed: 28 Rust integration cases, Linux's two
native USB unit checks, and 16 Python runner cases on Linux (two Linux-only cases
skip on Windows). The final source, differing only by two documentation comments,
was rebuilt in Linux; its three-cycle private-source probe served seven fresh
inventories and restored the desktop graph/state files exactly. No USB I/O occurred.
Final source is `beacn-link-usb-publication-periodic-final-20261012T153055Z/source`
under the existing guest home. See [native-usb-publication.md](native-usb-publication.md)
for hashes and precise validation scope. Guest free disk is about 21.3 GB.

**Hardware attempt stopped before publication:** the owner connected Studio and
was ready at the exact Applications box. A0/A1 completed, but the A2 native read
returned an unknown transport outcome. No list, periodic cycle, assignment or
route was submitted afterward; no retry occurred. Device release/baseline,
private graph restoration and both owned process exits passed. Connected desktop
and all tracked state files matched exactly after cleanup. Studio is back on
Windows; Windows defaults/endpoint states/formats/volumes/mutes/sessions and guest
state files match the pre-test baseline. Guest graph exact comparison differs
across handoff; 16 machine-audio registry values differ and remain preserved.
See [native-usb-publication.md](native-usb-publication.md) for hashes and limits.

The native read path had discarded libusb's error code/transferred count. These
are now logged on failed bulk I/O without payloads, retry or treating partial
bytes as success. The new exact tree was built in Linux; both native unit checks
passed. Latest build source is
`beacn-link-usb-publication-periodic-errors-20261012T153706Z/source` under the guest
home. This diagnostic change has not been tested on hardware. Free disk: 21.1 GB.

**Focused startup continuation:** the pinned Windows constructor explicitly
selects alternate zero, waits until the next millisecond tick, selects alternate
one, then resets vendor IN 83 before A0/A1/A2. The previously omitted wait and
pipe preparation are now an explicit `Startup::ObservedWindows` experiment;
defaults are unchanged. Retained identity/profile checks bracket each operation;
any error releases only the owned claim and stops before bulk. There is no device
reset, audio-interface mutation, automatic stall recovery or increased timeout.
The worker starts after initialization, so its scheduling cannot explain this
A2 failure. Full reconnect discovery remains untraced. Read
[protocol-session.md](protocol-session.md) for anchors, provenance and limits.

Windows/Linux focused validation passed 31 Rust integration cases; Linux also
passed two native unit checks and 17 Python runner cases. Exact Linux examples
built under `beacn-link-usb-publication-startup-20261012T154756Z/source` in the
existing guest home. Free disk is about 20.9 GB. This startup option is not yet
hardware validated; passing a test would not establish the earlier failure cause.

**Observed-startup hardware result:** initialization completed and cycle 1
published/polled successfully (512-byte no-payload response, 508 ignored tail).
Cycle 2's poll returned ACFFFFFF; the current policy halted without retry. No
assignment records or routes were applied. Claim/baseline, private graph and both
owned process exits were verified. Studio returned to Windows; host checked audio
settings and guest state files match baseline, with handoff graph/registry
differences retained. See publication docs for hashes and comparison limits.
This is one successful startup/cycle, not sustained visibility or a proven fix.

**Rejected-marker continuation implemented:** focused tracing confirms ACFFFFFF
returns from the stock poll callback without assignments/counter update or worker/
timer stop. An explicit `PollPolicy::ReportRejectedMarker` now reports the exact
marker separately and permits the next scheduled bounded cycle after fresh
inventory. Strict defaults and transport/partial/malformed failure halts remain.
It does not infer no-payload, peer health or a decoded firmware meaning. Default
and native examples built; Linux passed 38 focused Rust cases, two native unit
checks and 23 Python runner cases. Hardware validation is pending. Latest source:
`beacn-link-usb-publication-marker-20261012T162455Z/source` in the guest home.
Free guest disk is about 20.5 GB. Read publication/session docs for hashes/anchors.

The owner now requests direct USB handoff automation and
explicitly forbids computer-use/mouse automation. Native computer-use inspection
failed before any UI input and was stopped. Direct live VMware USB control remains
unverified. Fresh `prepare-connected` / `compare-connected` are now implemented
and use the existing snapshot/cache owners, so Studio can remain in the VM across
tests; return only at the end of the testing session. Keep one attempt
per evidence directory and no automatic USB retries. Sustained publication,
assignment receipt and audio remain pending; ordinary apply stays disabled.

Connected-workflow source built at
`beacn-link-usb-publication-connected-final-20261012T160535Z/source` in the existing
guest home. Linux passed 22 runner tests; Windows passed 20 with two Linux-only
cases skipped. Native examples rebuilt; Rust behavior is unchanged from the
31-case startup checks. Actual connected-stage hardware validation is pending.
The ignored `local/connected_usb_attempt.py` composes fresh connected prepare,
one authorized run, connected compare and host snapshots through direct SSH,
without switching USB or invoking any UI. Its read-only `check` verified source
pins and guest target count zero. It has not run a protocol attempt. Record a full
pre-handoff baseline at session start separately; a connected baseline is not a
pre-attachment backup. Free guest disk is about 20.7 GB. Read publication docs for
exact hashes and scope. No UI automation is permitted for future continuation.

**Historical next-work note, superseded by the checkpoint above:** a fresh-snapshot 40-cycle observed-marker hardware test. The ignored
`local/marker_connected_attempt.py` pins this build and composes connected prepare,
one run and connected comparison through direct SSH. Full host/guest pre-handoff
backups are separately saved by `local/publication_marker_prepare.py`. Its read-only
check confirmed target count zero and source pins. The owner should attach once
and leave Studio in the VM for this testing session; compare after each owned
claim release, and perform the Windows return comparison at session end. Keep
Windows Link closed and identify the same exact streaming-PC Applications box.
Sustained visibility, assignment receipt and audio remain unproven.
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
The latest built USB source is in the existing guest user's home under
beacn-link-usb-publication-periodic-final-20261012T153055Z/source. Its periodic
source/cleanup probe passed without USB. The prior diagnostic source under
beacn-link-usb-publication-diagnostic-20261012T144842Z/source passed hardware
list/no-payload poll plus brief owner-observed visibility. The earlier source/cleanup
probe used the preserved original publication
source under beacn-link-usb-publication-final-20261012T143405Z/source.
The corrected handshake source under
beacn-link-usb-claim-20261012T134712Z/source passed hardware initialization.
The first runtime attempt used
the preserved original source under
beacn-link-usb-final-20261012T132115Z/source. Library-version/cache-check modes remain
offline. Latest free disk is about 21.6 GB; continue lightweight builds only.
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

Final build/source `beacn-native-source-20261015T133608Z` retains strict 96-kHz
PCM validation and passes the same 25 focused checks. The final read-only audit
matches the failure-cleanup desktop/state snapshot exactly; both PCMs are closed
and no owned native test process remains. Private evidence was copied and hashed
outside Git. No additional physical acceptance is claimed for this source.

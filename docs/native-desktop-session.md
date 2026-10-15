# Native desktop publication and GUI Pause

Updated 2026-10-15 UTC. The owner confirmed **“Yes—video played, then paused.”**
An opt-in native desktop session now joins the already-playing Firefox stream,
guarded vendor publication/polling, first-pair Studio playback and the existing
GPUI worker/widgets. This is bounded hardware qualification, not an enabled
normal application, installed daemon, hotplug policy or final Console layout.

## Ownership and behavior

`DesktopLinkSession` uses the existing ApplicationService, inventory tracker,
planner, received-name policy, Session, guarded Handshake and native desktop lease.
The roster is pinned to one selected live stereo stream, catalog core and selection
revision. Every exchange needs a fresh complete inventory and fresh native route
proof. A changed core, app/stream identity or selection stops the retained owners;
the same displayed application name cannot automatically rebind a new process.

Desktop proof is deliberately a separate sealed type from VerifiedRoutePublication.
WirePlumber owns the application's stream links; LinX owns only its retained sink,
96-kHz adapter and physical bridge. Zero publication means verified playback at
the original desktop output. One means the echoed owned metadata request, complete
source/target graph, retained first-pair bridge and live native PCM proof all pass.
Private receipt checks and the private zero-means-unlinked rule are unchanged.

Proof carries a weak reference to its actual lease and a changing nonce. A new
observation, mutation, stop or owner destruction invalidates an earlier proof.
The desktop Handshake policy refuses raw inventory, private proof refreshes,
expired/revoked proof, a replacement lease and replayed inventory. It rechecks the
seal before publication and polling submissions. No numeric object ID, owner tag,
cached GUI state or independently supplied graph can mint desktop proof.

The session leaves Firefox's original output alone while publishing zero. Only a
received first-Link-In assignment resolved by the existing planner starts the
native bridge. Applied state is verified before publishing assignment one on the
next cycle. NoPayload, rejected markers and opaque zero do not infer unroute.
Unknown/conflicting records are blocked. Other physical pairs stop this explicit
first-pair check without applying them. No USB transfer is retried after failure.
The existing 1..40-cycle budget, 500-ms ordinary transfer limit and separately
selected five-second initial-read diagnostic stay in place. Tick uses the earlier
500-ms diagnostic cadence on the service worker, not the UI thread.

Pause releases native audio and restores original target/metadata before releasing
USB. Both cleanup outcomes remain visible and cached. Unverified cleanup blocks
another factory. Native backend factory failures now remain visible in session
error/status without opening control transport. Restart needs a fresh explicit
owner; this batch does not retain assignments across Pause or implement managed
physical sessions.

## GUI adapter

The explicit `desktop-session-test` features in app-core/app-gpui add a desktop
Studio configuration to the existing worker and Start/Pause/automatic-connect
supervisor. Ordinary `Connection::Desktop` and default features remain gated.
The real native desktop catalog supplies the app row; the normal selection/alias
model supplies its published name. The backend has no Firefox executable path.

Desktop cleanup is shown through a separate model field. It is never converted
into fabricated private link receipt counts. Existing session phase and observed
assignments feed the UI: Firefox appears in first Link In, the shared lynx icon
turns teal while connected, and Pause clears active assignments and turns it gray.
The automatic toggle stays enabled while Pause is held. Preferences for normal
startup are unchanged; this explicit fixture does not install login startup or
write the ordinary application's settings.

Qualification uses GPUI Kit events inside an owned Xvfb display with the already
installed software Vulkan implementation. It never uses the owner's desktop
mouse/focus or executes BEACN binaries. There is no PCM payload read/audio capture,
external playback subprocess, driver detach or firmware change.

## Live evidence

1. `linx-desktop-session-20261015T134927Z`, exact source
   `beacn-native-source-20261015T134548Z`: 40 desktop/protocol cycles, assignment one
   at cycle four and no protocol error. Last retained desktop sample had 72 PCM
   checks / 1,703,439 hardware frames; the separate transport PCM proof reported
   144 checks / 1,751,437 frames. The owner reported “ok it worked.” Result SHA256:
   `fd47c77f39f9a0be733a3a7954915285c44bd7b56c2f8d7d944c4fc7ea3aaed8`.
2. `linx-desktop-gui-20261015T135607Z`, exact source
   `beacn-native-source-20261015T135050Z`: actual GUI selection, automatic start,
   live first-pair assignment/playback, Pause, pause held and clean Quit.
   The transport PCM proof reported 36 checks / 822,967 frames. The owner confirmed
   the video played and stopped reaching the streaming PC at automatic Pause.
   Assigned/paused framebuffer captures were inspected: Firefox is visibly in
   first Link In with teal icon, then unassigned with gray icon and “Link paused.”
   Result SHA256:
   `1d3283cea3174c0a59193962cd2e922ccba36f326ec74d2c6b83ef125a4747cb`.

Both successful runs closed playback/capture and restored their baseline's foreign
settings/graph, persistent state bytes/hashes and connected USB binding. The two
selected application links were recreated, so the raw desktop object snapshot is
not exact. Their source/destination identities and remaining graph were checked
separately. The earlier desktop batch's retained owned cache entry is still
recorded in [native-desktop-target.md](native-desktop-target.md); these per-run
matches do not erase that older mismatch or authorize overwriting foreign changes.

Preserve `linx-desktop-gui-20261015T135233Z` as a non-acceptance: the owner was not
ready to watch. The GUI connected but got no assignment, performed no native PCM
playback and completed its bounded session. Its widget probe eventually timed out
waiting for assignment. Desktop/state/USB comparisons were exact. The agent must
wait for a fresh ready response before each owner-attended run, then announce and
start immediately. Do not infer GUI playback from that first run's zero process
exit or connection capture. Failed-result SHA256:
`5990907ec8c7f6178159b2e8dd42d697a6cea7a1cd0c1b438fa958719c105f84`.

## Validation and next work

Final compiled source `beacn-native-source-20261015T135926Z` passes 101 focused root
checks: native/desktop owner, catalog/graph, existing link session, route publication,
protocol Session, USB handshake/publication and the new desktop fault/lifecycle
cases. Nine new desktop tests cover original-output then applied publication,
opaque-zero retention, stale/edit/departure stop, partial route/cleanup failures,
unsupported pair refusal, revoked/expired/replaced/replayed seals, distinct policy
gates, exact budget and visible factory failures. Eight app-core startup/preferences
checks pass on the earlier GUI source. Offline root and GPUI example/main builds
pass with existing toolchain/dependencies; no package or workflow was added.

That final source adds startup error reporting and initial proof/inventory equality
checks after the accepted GUI run. No additional physical acceptance is claimed
for it. Native GUI qualification is the entry in `app-gpui/examples/desktop_probe.rs`;
CLI qualification is `examples/desktop_session_probe.rs`. Both require explicit
selected device/cache and current app/hardware identities; neither is a production
startup path. Source/executable hashes, tool logs, raw comparisons and screenshots
remain private outside Git.

Next: qualify desktop GUI Resume/failure handling and long-lived sessions, then
expand stream/application and physical-pair support through the same generic
backend. Port the approved Console visual direction onto these working controls.
Device discovery/hotplug, durable assignment continuity, installed startup,
performance/long runs, local gains/mutes/meters and remote mix controls remain
separate unfinished capabilities. Laptop free space is about 1.7 GB after cached
builds; preserve evidence and check headroom before another large build.

The accepted CLI image SHA256 is
`0c1b7e5dc44dbdaf6d07292b54b07f6a91369b2c7ef0f1cc210d01294f401470`;
the accepted GUI image SHA256 is
`4105d4681124bfa939ab55d33f50398a5babcc74a283a5965d3bcec33252ac37`.
The later compiled GUI image SHA256 is
`1a6977de46d23b7b99e3f7ac818576b1d024db46987715c952b72e9da1e178f3`.
A final read-only audit after the builds matches the accepted GUI run's released
desktop and state snapshot exactly. Playback/capture are closed and no owned test
process remains. Only the loopback screenshot review server is left running.

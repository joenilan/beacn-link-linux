# Rust Link session and lynx status

Updated 2026-10-14 (qualification continued into 2026-10-15 UTC).

## Managed lifetime

`LinkSession::start_managed` takes explicit `ManagedOptions` and the same retained
route/transport factories. `Publication::managed_selected` requires a fixed,
nonempty roster and verified route publication before every exchange. This is an
explicit alternative to the bounded APIs, which retain their 1..40 policy. Status
uses `cycle_limit: Some(n)` for a bounded run and `None` for a managed run; phase
must also be read because Idle has no budget. The completed counter uses checked
addition. Stop/Drop still close routes before consuming the transport.

Managed operation has no automatic reopen, new-process rebinding, transfer retry
or permission to bypass stale observations. Every tick reuses the existing
ApplicationService, route verification, epoch and acknowledgment owners. Failed
cleanup blocks both bounded and managed starts. The owner must keep observing and
call check/tick/stop; this API is not an independent watchdog or hotplug policy.

The private worker exposes this only through explicit
`Connection::ManagedPrivateSession` in qualification builds. The ordinary app
continues read-only discovery. The existing Studio adapter remains bounded and
uses neither new managed constructor. Its external physical bridge and active-PCM
check budget remain unchanged; this software milestone is not continuous physical
playback acceptance.

To exercise the managed GUI sequence, build the explicit `ui-qualification`
`interface_probe` example and use the existing authorized no-USB
`routing-probe --application-mpv --catalog-service-check ... --gui-check ...`
harness with `LINX_MANAGED_PRIVATE_GUI_TEST=1`. This requires its owned core,
explicit routing authorization and private display, just like the bounded GUI
probe. Normal app startup does not read this variable. The harness allows 85
seconds for this sequence, independently of the session's lifetime. No physical
budget is raised.

Qualified source `beacn-native-source-20261015T030813Z`; native managed run
`beacn-managed-gui-20261015T031153Z` passes two real mpv apps, routes 1/4 and at
least 96 cycles before Pause, pause held while automatic remains enabled,
explicit resume, selection-change stop, restart with one app and Quit while
running. Both cleanup owners verify release. The complete original private core,
desktop/settings/routes/defaults/volumes/mutes/state, owned configs/processes and
connected USB binding match afterward. Complete-result SHA256
`93fdfd7b8d908a5f449cc9bef83f273751cf1f9a21ea5a2d8000a65f344256a5`;
service journal SHA256
`74e7b36e14d563d47c40820687334e2df8e658d208cfeb12c75ac92a4a6d0a29`.
Native GUI running/paused screenshots were inspected. No USB or physical audio
occurred in these checks.

Four new root checks exercise 128-cycle retention/pause/fresh-owner restart/Drop,
faults after 64 cycles, invalid managed policy and unchanged bounded limits, plus
96-cycle transport operation followed by stale-proof or raw-inventory refusal.
The focused root suite has 57 passing checks on Windows and Linux; eleven core,
eight reference GUI and 31 Python runner checks also pass. Native production and
Studio qualification builds pass. Production window/catalog/gray-icon/normal-close
and exact desktop/settings/config/USB comparison pass separately.

Earlier evidence is retained without converting failures to passes. The first
probe used two identical mpv wire names and was correctly refused before session
start. A concurrent default-feature root test build then replaced the native
catalog helper during a later run: its 96-cycle GUI sequence passed but subsequent
CLI checks correctly reported native discovery unavailable. Rebuilding the helper
and running without concurrent builds produced the complete passing run above.
Never overwrite qualification executables while their owners are running.

The bounded GUI regression `beacn-gui-20261015T031457Z` passes its full widget/
route/session sequence and verified release, with all owned processes stopped
and original private core restored. Its outer desktop/state comparison is **not
exact**: a stream already on the desktop before the probe was recreated with new
node/port/link identities, and persistent state differed at the run boundary.
The first failed setup run also has desktop/state drift. Later comparisons are
not substituted for those mismatches. No foreign settings/routes were restored
and the strict comparator remains unchanged. The managed and production runs
above each have their own exact matching comparisons.

## Session ownership

`LinkSession` joins the existing ApplicationService, PublicationEpoch,
RoutingControl, Session and guarded Handshake owners. It takes explicit route
and transport factories; constructing it opens nothing. Start validates the fixed
selected roster, obtains an owned route lease, then initializes the transport.
Every cycle requires a newer catalog observation and fresh complete route
proof before publication. Received assignments pass through the existing planner,
executor, acknowledgment and route-verification path. Only verified data is
published on the next cycle. NoPayload/rejected/blocked records do not infer unroute.

The new `Publication::bounded_selected` is an explicit 1-40-cycle multi-app gate
that always requires verified publication. Legacy constructors still require one
application. Empty/stale/duplicate-name/oversized rosters are refused. Ordinary
transfer deadlines remain 500 ms; the existing optional initial-read diagnostic
does not widen later exchanges. No C executor or native USB selection gate changed.

Stop releases the route lease before consuming/releasing the retained transport.
Lifecycle/selection changes and protocol/executor errors stop both owners, without
forward retries. Cleanup outcomes remain visible. Unverified cleanup enters
NeedsReview and blocks another factory call. Stop and Drop do not repeat release.
An eventual physical backend must also own playback shutdown before its transport
is released; the current private implementation does not supply physical playback.

## GUI integration and limits

The GUI worker handles Start/Stop, timed exchanges, observations and native route
barriers; the render thread only queues commands and reads snapshots. Queued Start
pins the displayed core, aliases and every selected application's exact stream
binding. Stop remains available after discovery loss. The first cycle waits for a
new observation instead of reusing Start's revision. Channel cards display the
latest verified assignments, cleared immediately on stop.

The explicit Linux-only `private-session-test` feature supplies an injected
`Connection::PrivateSession` configuration. It pins the guarded disposable core,
server cookie/PID, endpoint serials and owner token. Its ScriptedPeer implements
the existing USB backend trait and sends synthetic handshake/poll bytes through
the actual Handshake and Session gates. It performs no device I/O. This feature
cannot enable desktop apply. The normal executable has no CLI to create this
configuration and continues read-only application discovery with Start disabled.

## Lynx icon and tray

One Rust lynx-face silhouette supplies the application indicator, dynamic native
window icon and optional Linux tray icon. Pointed ears, long ear tufts and a cheek
ruff remain recognizable at small sizes. Status has a text label as well as color:

| Color | Meaning |
| --- | --- |
| Gray | Link disconnected/stopped; discovery alone never means connected |
| Amber | Link initialized, waiting for its first successful control cycle |
| Teal | Running Link session with a successful control cycle |
| Red | Discovery/session error or cleanup requiring review |
| Blue | Explicit private-core session with a scripted peer |

These colors are LinX's design, not a claim of exact BEACN palette parity. The
state behavior follows the requested connection-status rule. No normal GUI device
connection is implemented yet; its discovery success cannot produce a teal icon.

The optional `status-tray` feature uses pinned ksni 0.3.6's Linux
[StatusNotifierItem API](https://docs.rs/ksni/0.3.6/ksni/trait.Tray.html).
Bus registration/updates run on a separate thread. The tray uses ARGB pixmaps at
16/22/32/64 pixels, text tooltips, error attention icons, Open, Stop and Quit.
Callbacks set bounded action flags; route changes stay with the existing worker.
Window close still exits normally; close-to-tray/autostart are not implemented.
If there is no compatible tray host, the window remains usable and explains that
the tray is unavailable. Desktop-specific panel visibility remains to be checked
on the user's chosen Linux desktop; no extension or host configuration was changed.

## Evidence

Five new root session tests cover actual framed multi-app exchanges, opaque
replies, verified route changes, cycle exhaustion, stale/lifecycle refusal,
partial route/control faults and failed cleanup. Eight GUI/model tests cover
widget actions and the new status/backend/start-binding rules. The focused Linux
root suite passes 74 checks; all 31 Python runner checks pass.

On the isolated Xvfb display, two actual mpv processes were selected/renamed via
real widgets, started and assigned to private pairs 1/4, stopped, restarted, then
stopped by a selected-roster change. Both route and transport cleanup were verified.
The rendered screenshot was inspected. Complete owned-core teardown restored the
original graph; desktop/settings/state hashes and owned configs/processes match.
Intermediate negotiated Format/channel vectors remain recorded as unequal while
disposable nodes exist; the final whole-core comparison is mandatory.

A separate disposable D-Bus session with an owned fake tray host reads all five
actual icon/property states, checks ARGB pixels, invokes Open/Stop/Quit through the
bus, and verifies bus-name release. This qualifies protocol behavior, not visibility
in GNOME/KDE's real panel. No owner's desktop bus or mouse is used.

No USB, physical audio or Windows app changes occurred. Multi-app hardware,
shared-desktop apply, unaffected-app continuity, durable reconnect, persistence
and installation remain open. Keep hardware closed while the owner streams,
Actions disabled and checkpoints local with [skip ci]. A narrow dependency/security
review is recommended before release for the optional bus/GUI dependency trees;
no paid scan was requested or run.

Qualified source: `beacn-native-source-20261015T004833Z`. Private evidence:

- GUI/session qualification `beacn-gui-20261015T005109Z`, SHA256
  `f1becbf4d96b8e9f46016685a8f6e0ffbfc971cebbc154b8363ad5f7241f81ef`.
- Tray bus journal `beacn-tray-20261015T004958Z`, SHA256
  `a79276d50fb2394825fb858e007cd21c217cb0b80d8e9b8f3f0c676c82a621f0`.
- Production window/icon/close journal `beacn-gui-desktop-20261015T005052Z`, SHA256
  `5dfc3b222e64d2f385148465eac1ccf791df343b4c316beb3f3a4175ac6d756e`.

The production check reads the actual X11 window icon property and verifies the
gray lynx, then sends normal close only to its owned window. Missing tray-host
fallback also exits normally on its own private bus. Earlier failed helper runs
remain private: asynchronous property reads needed to await all updated fields,
and Xlib's 64-bit sign extension required masking its 32-bit icon values. One
forced probe exit left its generated config; it was preserved in that run's
evidence and removed from /tmp after confirming the exact process had exited.
An unrelated older config was left untouched. 61 final evidence/provenance files
were copied and hashed outside Git before derived PNG previews.

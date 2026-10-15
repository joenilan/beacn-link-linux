# LinX GPUI Kit interface

Updated 2026-10-14; native qualification continued into 2026-10-15 UTC.
The owner selected **LinX** as the independent working product name: Link with
an X, also evoking Linux. Repository/crate names and historical evidence identifiers
remain stable. This naming decision is not a trademark-clearance result.

## Application structure

The primary interface is now `app-gpui/`, using pinned GPUI Kit 0.7.3 and its
styled components/assets. No JavaScript shell, webview or speech feature is
enabled. The previous egui preview is retained under `app/` for comparison and
regression checks. Both use the same `app-core/` model, worker, private-session
adapter, status mapping, icon geometry and optional Linux tray. There is no second
implementation of routing, application selection or protocol handling.

The core still builds with Rust 1.88. The new frontend is qualified with Rust
1.99.0, pinned in its own toolchain file. The existing default toolchain remains
unchanged. GPUI Kit's installation guide suggested Rust 1.92, but actual pinned
dependencies failed on slice::as_array with 1.92 and std::hint::cold_path with
1.94. Those failed builds remain in private provenance; no registry source or
compiler safety gate was patched. The successful build uses 1.99.0.

The initial dependency setup added 38 development packages, about 250 MB, without
upgrading or removing packages. No VM reinstallation, device driver, firmware,
audio-service restart or desktop extension was involved. Relevant settings/state
matched the saved baseline after installation.

## Compact, expanding layout

Default client size is 720 by 520 pixels. The header has the lynx session icon,
LinX name, actual session text, Connect/Pause and an expandable information panel.
Four channel cards share a row when there is space and become two rows in narrow
windows. Selecting a card reveals its assignment explanation. Application details
expand individually for process/stream identity and published-name editing.
Selected aliases appear in the primary row, with the original app name retained
in its status line. Search matches app names, executable names and selected aliases.

The application list fills remaining space and scrolls. At short window heights,
the whole view also scrolls so expanded panels cannot hide the remaining controls. Expanding sections and
resizing use the current layout; normal controls do not forcibly resize the user's
window. Future controls can be added to these sections without reserving large
empty panels. There are no simulated audio meters or enabled volume controls.

Kit buttons/inputs supply focus, keyboard editing and interaction behavior. All
name/selection commands still pass through the existing conservative wire-name
validation. Queued commands pin their displayed core/streams; a core change also
clears UI drafts. Selections remain in memory and are revoked on discovery loss;
they are never rebound to a same-named replacement. See startup behavior below.

The normal executable remains read-only discovery with Connect disabled. It cannot
construct a device or desktop routing backend. The explicit private-session test
feature uses only guarded disposable cores and the synthetic peer, just as before.
See [rust-link-session.md](rust-link-session.md) for route/transport ownership.

## Startup and waiting

The main interface has a GPUI Kit **Connect automatically** switch. Normal launch
defaults it on and waits for an available, enabled backend and live selections;
an absent device is not a reason to close/reopen the app. The current production
preview has no physical backend, so remains waiting with routing disabled. It
does not claim that catalog discovery means Studio is connected. The private
qualification mode defaults to manual and never reads/writes user preferences.

`app-core::startup::Supervisor` owns automatic/manual intent only. Available
backend factories and the existing session/receipt owners still validate every
start. Pause releases the current session and holds automatic starts, including
across disappearance/reappearance. Connect or turning automatic off/on re-arms
it. Turning automatic off alone leaves an already running session active. Failed
starts and bounded test completion never loop automatically; cleanup needing
review blocks all starts. The existing fixed-roster session rules remain intact.

Normal mode saves only the toggle in `$XDG_CONFIG_HOME/linx/preferences`, or
`$HOME/.config/linx/preferences`. Merely launching does not create a file. Writes
use a private temporary file and atomic replacement; unexpected formats,
symlinks, oversized files or detected external edits are preserved/refused.
Read failure defaults automatic connection off and is shown in the UI. No BEACN,
PipeWire, Windows or routing settings are persisted by this preference owner.
Launch at login, installation, background close-to-tray and cross-launch app
selection persistence are separate future work; no login entry is installed.

The worker retries lost/unavailable read-only catalog connections every two
seconds, drops old handles/configs, and re-observes before accepting a command.
Generation remains monotonic across reconnects. Stop and preference changes work
without a catalog; queued app/start intents still pin their exact observed core
and binding. It never restores selections by app name after a reconnect.

Existing static research establishes initialization and the asynchronous 500 ms
Windows loop, but does not fully trace its discovery/reconnect policy. These
startup semantics are an independent product choice, not an assertion of binary
equivalence. See [protocol-session.md](protocol-session.md#focused-startupworker-continuation-2026-10-12).

Startup qualification adds eight focused preference/lifecycle checks, native
private-core automatic routing/pause/re-arm/quit checks, and an actual read-only
worker probe which waits with no server, observes the server, detects its exit,
then observes a new core without restarting the worker. Original desktop state,
owned configs and processes compare cleanly. Physical Studio hotplug/session
recovery remains unqualified in this frontend and is deferred while streaming.

Startup-qualified source `beacn-native-source-20261015T015044Z`; complete native
GUI/session result `beacn-gui-20261015T015125Z` SHA256
`24843f723497d8d4d57367386de44fb1e3eb694bd345410238ac9124fff48ff4`.
It verifies the automatically started session's actual pair-1 assignment, not just
its Running phase. Separate actual catalog recovery is retained as
`beacn-startup-20261015T014842Z`; normal read-only production close/icon journal
`beacn-gui-desktop-20261015T014949Z` SHA256
`405212fe747a82aa08de21607277ecb66ebf85cc799a90ebe503728d570a1699`.
The final production build uses the same qualified code; later changes are docs.

## Status and responsiveness

The same gray/amber/teal/red/blue status rules serve the header, tray and icon.
Catalog discovery never means device connected. Private tests stay blue. SVG
assets are generated from the same geometry as tray/window pixmaps. The X11
property update uses the retained GPUI window's native handle; it does not search
for or modify other windows. Dynamic Wayland taskbar icons remain compositor/app
identity behavior and are not qualified here. The Linux tray retains Open, Stop
and Quit, plus an unavailable-host indication in the window.

Native observations remain off the render thread and commands remain bounded.
The UI takes fresh snapshots but requests redraw only when visible catalog,
selection, session, assignments, feedback or tray availability changes. Catalog
generation increments alone do not redraw an idle window. The worker still
observes every 500 ms and before commands; this is not a frozen inventory cache.
Normal application quit explicitly stops/joins the worker and releases its lease.

An initial debug/Xvfb idle sample used 0.350 CPU seconds over 2.001 seconds. After
removing generation-only redraws, the same bounded check used 0.070 CPU seconds
over 2.001 seconds, with about 205 MiB RSS. This is a short isolated diagnostic,
not release, real-desktop, GPU, gaming or long-duration performance acceptance.
No speculative build-optimization flags were introduced.

Saving an unchanged selected name is disabled and is also a no-op at the command
boundary. Editing the input refreshes its Save state. This prevents an unchanged
save from advancing the fixed-roster selection epoch and stopping a live session.

## Native qualification

The GPUI feature-only probe runs on an owned Xvfb display/private D-Bus session.
It uses GPUI Kit's window-level pointer/key event API, never the owner's mouse or
desktop. Two actual mpv applications are discovered through the native worker.
Real widgets expand rows, edit/select distinct aliases, search one alias and clear
search. Native windows render at 720x520, 440x520, 900x640 and 440x380; compact, narrow,
expanded, small expanded/scrolled and running screenshots are saved privately and inspected.

The same run starts native private routes to pairs 1/4, verifies both assignments,
stops, restarts, removes one selection to trigger lifecycle release, restarts the
remaining app, then quits while running. The actual app-quit hook verifies route
and transport cleanup plus catalog-config removal before writing the success
marker. Complete original-core teardown, relevant desktop/settings/state hashes,
config and owned-process comparisons pass. Intermediate disposable-node Format/
channel-vector differences remain recorded as unequal until full node teardown.
The small-window case scrolls the actual window and reaches its Save control;
an unchanged-save attempt also leaves the running session active.

The separately built production binary omits all qualification features, opens
normal read-only desktop discovery on isolated Xvfb/private D-Bus, exposes the
actual gray X11 icon and handles normal WM_DELETE_WINDOW with exact cleanup.
Private portal/FUSE/desktop-service warnings remain in the raw logs; no unrelated
permissions or desktop services were repaired to suppress them.

The existing eight egui/model checks pass against the extracted shared core.
The 74 focused root checks and 31 Python runner checks pass. No USB or physical
audio occurred. Hardware tests remain closed while the owner streams. Multi-app
physical GUI integration, shared-desktop apply, unaffected-app continuity, durable
reconnect, release packaging and actual desktop-panel visibility remain open.

## Sources and next work

Run the read-only preview on Linux with its normal desktop account:

```sh
cargo +1.99.0 run --manifest-path app-gpui/Cargo.toml --locked
```

The nested toolchain file also selects 1.99.0 when running Cargo from `app-gpui/`.
Optional `--private-runtime PATH --core NAME` only changes discovery's target; it
does not enable routing. Do not run this GUI as root or attach Studio while the
owner's stream is active. Hardware/backend integration remains a separate step.

Qualified source `beacn-native-source-20261015T012848Z`. Private GUI/session
qualification `beacn-gui-20261015T012900Z` has complete-result SHA256
`d95f9c3d259fceecbf344320f9e9909b413784d36b1ef80d0e0408e078e05398`.
Production window/icon/normal-close journal `beacn-gui-desktop-20261015T012925Z`
has SHA256 `6f449886d7d74c7ea4b1548044015ea28b6f4461663da9455d22746c688988b0`.
The shared tray's five real private-bus states, callbacks, missing-host fallback
and bus-name release also pass after extraction. Raw evidence remains outside Git.

GPUI Kit API/requirements were checked against the
[official installation guide](https://gpui-kit.com/docs/installation/),
[getting-started guide](https://gpui-kit.com/docs/getting-started/),
[component docs](https://gpui-kit.com/component/) and exact downloaded 0.7.3 sources.
Before release, a focused dependency/security review is recommended for the new
GUI/platform/asset dependency tree. No paid scan was requested or run.

Next connect a physical transport/playback owner after the owner's stream and
qualify actual multi-app hardware behavior through this frontend. Keep default
apply disabled, Actions disabled, private evidence outside Git and [skip ci]
checkpoints local unless the owner asks to publish.

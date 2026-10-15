# Beacn LinX Rust interface

Updated 2026-10-14 (native qualification continued into 2026-10-15 UTC). The first
graphical application is implemented in Rust using egui/eframe 0.33.3, pinned with
a separate application lockfile. This version supports the existing Rust 1.88
toolchain; see the [versioned eframe API](https://docs.rs/eframe/0.33.3/eframe/).
The root protocol/executor package retains its dependency-free manifest and default
features. This is an application discovery/selection preview, not desktop apply.

## Run on Linux

```sh
cargo run --manifest-path app/Cargo.toml --locked --features native-catalog
```

Run as the normal desktop user with its normal PipeWire session. Optional
`--private-runtime PATH --core NAME` arguments select only the existing guarded
disposable core. They do not enable routing. The default build without
native-catalog shows an explicit unavailable discovery state. No USB/native-usb
feature is enabled by this application. No installer/autostart or system service.

The window shows real discovered apps, case-insensitive search, selected apps first,
explicit Select/Remove controls and editable published names. A selected alias
uses Save name; unsupported/duplicate names are refused by the existing selection
owner and explained in the window. Process ID/stream count are available in a
tooltip to distinguish identically named applications. Display labels are separate
from conservative printable-ASCII wire names. No silent aliasing or truncation.

Routing is visibly off. Start link is disabled, and all four channel cards say
there is no active session. Selected apps are not claimed to appear on Windows.
No device online state is invented. RoutingControl remains the verified API for
later integration; this GUI has no backend factory or route-application control.

## Ownership and responsiveness

The native read-only catalog and ApplicationService live on one dedicated Rust
worker thread. The GUI only sends bounded selection commands and reads the latest
snapshot. An eight-command bounded queue uses nonblocking submission; results
replace one shared snapshot rather than accumulating an unbounded event queue.
The worker polls every 500 ms and obtains a fresh native observation before every
command. Native barriers never run in the egui render loop.

Each queued action retains the displayed core and exact application/stream
binding. If either changes before the fresh worker observation, the old action is
refused. The existing owner revokes stale/departed/core-changed selections; same-name
reopens stay unselected. UI alias drafts also reset on a core change. A discovery
failure revokes selections, leaves the last list disabled, explains the error and
stops the worker. It does not automatically reconnect or retry a native failure.

Normal window close stops/joins the worker and releases its owned catalog config.
An in-flight native observation may add its existing barrier deadlines to close
latency. Forced termination/signals are not graceful-cleanup acceptance cases.
Selections, window geometry and egui state are not persisted. There is no telemetry,
web content, background daemon or route/audio capture path in the interface.

## Verification

Five focused egui/model tests pass on Linux: actual pointer/key events select,
rename and remove through the existing owner; search and stale/offline controls;
duplicate/unsupported names and atomic refusal; departure/reopen; and queued actions
against replaced domains or stream bindings. The root focused checks and all 31
Linux runner checks also pass. These tests do not claim Windows GUI qualification.

The test-feature-only `app/examples/interface_probe.rs` opens the real window on
an isolated Xvfb display with software rendering. It injects egui input events
inside that window, never OS mouse input or the owner's desktop. Two actual mpv
processes are discovered through the native catalog. The test selects the first,
edits the second's duplicate default name, selects it, renames the first, captures
the rendered window, removes the second and normally stops the worker. Selected
state is checked after every asynchronous command and owned config cleanup is
verified. The screenshot is inspected outside Git. The enclosing private graph,
desktop settings/state and process cleanup comparisons all match exactly.

The production binary is separately built without the qualification feature. A
second isolated Xvfb test opens its default read-only desktop catalog and sends
WM_DELETE_WINDOW only to its own window. Normal exit is zero, the owned catalog
config is removed, and full laptop desktop/settings/state matches afterward.
This is an actual production window/normal-close check, not an owner-visible
desktop launch. No Studio is attached; no USB or physical audio occurred.

Xvfb was the only added Debian package (no upgrades/removals). Existing toolchain,
VM, drivers and audio packages were preserved. egui dependencies were fetched once;
subsequent exact builds/tests use the application lockfile offline. Glow is the
renderer; no wgpu renderer is enabled. All private evidence includes source and
binary hashes. Earlier failed probes remain preserved (one correctly refused two
identical mpv aliases before the script was corrected to edit the second name).

Qualified source: `beacn-native-source-20261015T002307Z`. Private evidence:

- Complete qualification `beacn-gui-20261015T002321Z`, SHA256
  `77910484eccb37256e9ec35a7f6e991a632fd4665b0b258f4d983c1d4f804036`.
- Rendered raw screenshot SHA256
  `02f64bdc8593761e74551ca57ebdb92691e50c28fc2801198dec3f0c09fb7083`.
- Production normal-close journal `beacn-gui-desktop-20261015T002326Z`, SHA256
  `315652e6ee9cbcca4d4002dec7c35bf8141d8325f71745cac58b5b4d67fb6557`.

205 evidence/provenance files were copied and hashed outside Git before the
derived PNG and final provenance. Source code hashes are checked against the
native-qualified tree before publication.

## Next

Connect this interface to the guarded routing controller and native transport
through explicit session state, then qualify the real multi-app hardware path
when the owner is available. Desktop apply remains disabled. Unaffected-app
continuity, durable reconnect, persistence/installation and graceful signals
remain open. Keep hardware tests closed during the owner's stream, Actions
disabled and checkpoints local with [skip ci]. Before release, a focused dependency
security review is recommended because this batch adds a GUI dependency tree;
no paid security scan was requested or run.

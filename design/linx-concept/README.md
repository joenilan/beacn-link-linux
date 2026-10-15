# LinX interface study 01

Open [index.html](index.html) directly in a browser. There is no build step,
server, dependency download or installation. This is a reviewable design proposal
for the existing Rust/GPUI Kit app, not a change of application framework.

The default **Overview** is a compact 680-pixel window: status-aware lynx,
connection controls, four Studio Link channels and shared apps. **Mixer** expands
to 900 pixels and introduces illustrative Linux app volume/mute controls. Both
reflow to two channel columns on narrow screens. Settings expand in place.
Selecting a channel filters its applications; select again or use Show all to
clear the filter. Native sliders support keyboard adjustment. The preview-state
selector exercises waiting, connecting, connected, paused, attention and private
test presentation. All examples and meter values are fictional and labeled.

No device/audio API, process execution, network request, storage or IPC is used.
Controls only change this page's in-memory demonstration. Reload resets it.
The state picker is a visual scenario selector, not a replacement for the Rust
startup supervisor or evidence of physical connection/reconnect behavior.

## Design direction

- Dark neutral surfaces, restrained teal, a single thin frame and short rows.
  Status is always text as well as color; the lynx follows the current app's
  gray/amber/teal/red/blue mapping and exact existing silhouette geometry.
- Keep useful controls beside the application they affect. Route badges are
  observations: the current protocol receives assignment decisions from BEACN
  App. This study does not invent a working local assignment command.
- Connection and Pause remain prominent. Automatic connection is distinct from
  launch at login; the latter is labeled planned and disabled.
- The remote mixer is an optional collapsed section with disabled controls.
  Avoid an always-visible wall of unavailable faders or a full DAW interface.
- No looping meters, fabricated latency/performance statistics, copied BEACN
  artwork, external fonts or marketing dependencies. Meter drawings are static
  demo marks that react only to the preview controls.

## Research and capability limits

Reviewed official sources on 2026-10-15:

| Source | What informs this proposal |
| --- | --- |
| [BEACN application assignment guide](https://support.beacn.com/en-US/assigning-apps-in-beacn-software-281983) | Familiar application names and channel destinations; preserve the streaming-PC assignment workflow. |
| [BEACN Mix guide](https://support.beacn.com/en-US/getting-started-with-beacn-mix-282019) | Channel grouping, source lists and adjacent mute controls. This is design inspiration, not proof of the Link protocol. |
| [Elgato Wave Link](https://www.elgato.com/us/en/s/wave-link-app) | A readable relationship between sources, destinations and levels; complexity can grow from a simple view. |
| [GPUI Kit Slider](https://gpui-kit.com/component/slider/) | The chosen native toolkit has slider primitives; no framework migration is needed for this direction. Latest website APIs must still be checked against our pinned 0.7.3 before implementation. |
| [WirePlumber volume/mute control](https://pipewire.pages.freedesktop.org/wireplumber/man/wpctl.html) | Linux offers node volume/mute operations. This establishes platform capability, not an implemented or tested LinX volume owner. |

Our [current protocol evidence](../../docs/protocol.md), subsequent native
session work and [playback owner](../../docs/native-playback-owner.md) concern
application publication, assignments, routing and playback. They do **not**
establish commands to read or move BEACN App's monitor/audience faders. Supporting
Linux volume does not establish remote BEACN mixer control.

| Control | Practical scope | LinX status |
| --- | --- | --- |
| Linux app volume/mute | Change the source on this PC before sending it; every downstream mix receiving that audio is affected. | Platform capability known; native owner/UI integration and physical qualification still needed. |
| Local channel send level | Scale an owned channel's outgoing audio without changing the remote fader. | Proposed; grouping, multiple sources and gain ownership require design and validation. Not represented as a working channel fader here. |
| Independent monitor/audience faders, remote mic or device controls | Change a distinct mix or device state on the Windows streaming PC. | Unknown protocol/API capability; disabled future exploration. |
| Metering | Display measured signal activity with a stated measurement point. | Preview values only; real telemetry and its audio-data boundary need separate implementation/qualification. |

Remote mixer support has two possible research routes: establish whether the
device's control protocol exposes the needed state/commands, or investigate an
optional Windows companion and a supported control interface. Neither is
implemented or known to control BEACN faders. Do not substitute Windows app
volume or Linux send gain for the distinct monitor/audience mixes.

## Implementation sequence

1. Finish the already prepared attended native playback check, then physical
   Pause/failure/reconnect. Ordinary desktop routing remains gated.
2. Translate the approved visual direction into `app-gpui`, reusing `app-core`
   snapshots, startup supervisor, icon and worker. Keep route badges observational
   and represent unknown/unavailable values explicitly. No fake meters or active
   future sliders in the production view.
3. Add local app volume/mute as a separate native control owner, starting with
   offline semantics and a disposable-core rehearsal. Reobserve exact core/node
   generations, stream set and current gain before mutation; never target a
   same-named replacement, a default device or all nodes of an unvalidated PID.
   Observe current volume/mute before the first change; do not reset an existing
   level when showing the UI. Start with 0–100% and no implicit gain boost.
4. Define whether a local control changes the app's shared OS stream or an
   independently owned send. The former can affect local listening too; the
   latter needs a new proven signal/gain path. Make that scope visible before
   enabling the control. Keep independent stream/channel balance unless the
   user explicitly changes it.
5. Coalesce rapid slider intent off the render thread, check fresh identity
   immediately before each write, and display confirmed read-back state. On
   disconnect or conflict, disable the control rather than replay stale intent.
   Restore only owned temporary changes whose current state still matches our
   last write. Preserve external adjustments; report uncertainty without
   overwriting another app's settings. Persistent user volume preferences need
   an explicit product policy before automatic restore is added.
6. Qualify audible volume/mute, app exit, external adjustment, Pause and cleanup
   while attended. Remote monitor/audience control is a separate later milestone.

## Review evidence

Isolated headless Chrome on Windows rendered both views at 320, 440, 740 and
1,060 pixels across all six states (48 layouts), without horizontal overflow,
script errors or external requests. Volume, keyboard adjustment, mute, filtering,
empty channel, settings, Pause-held auto intent and reconnect presentation were
exercised. Desktop and narrow screenshots were visually inspected. Raw screenshots
and the browser report are private under `local/design-review-*`, outside Git.

These checks validate this browser mockup only. They do not establish GPUI pixel
parity, Linux/Wayland rendering, accessibility certification or hardware behavior.
No Rust source, routing gates, dependency lockfiles or saved settings were changed.

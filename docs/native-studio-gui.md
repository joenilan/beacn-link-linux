# Bounded native Studio GUI integration

Updated 2026-10-15 UTC. The owner explicitly reopened hardware testing after
connecting Studio to the native Debian laptop. Keep the earlier streaming cutoff
as historical context; do not treat that old authorization window as current.
The successful run followed the owner's **start now** reply. Future watched tests
need another clear handoff after all builds/checks are finished. No further
hardware test is running or automatically scheduled.

## Implementation

The explicit `studio-session-test` feature supplies a native GUI session through
`app-core::studio_session` and `app-gpui/examples/studio_probe.rs`. The default
production frontend still cannot construct a USB or shared-desktop routing owner.
This is one fixed selected ordinary application, on a retained disposable core,
with 40 publication/poll cycles and an explicit physical pair. It is not a daemon,
shared-desktop application takeover or installed product acceptance.

The shared worker now has one session adapter for disabled, scripted-private and
real Studio sessions. Backend availability and private-test identity are separate:
native Studio handshake/publication can show the real connected icon; scripted
sessions remain blue. The native adapter reuses LinkSession, ApplicationService,
publication epochs, NativeRoutingLease, NativeUsb and the existing codec/planner/
receipt owners. No root default gate or cycle budget was widened.

The bridge is still the existing Python qualification supervisor, not a new Rust
physical audio implementation. Only actual verified assignment of the selected
app to the explicit pair can request physical START. The supervisor independently
checks the retained source, native GUI PID/start, destination/port/link identities,
selected ALSA device, owned private server and PCM progress. Its exact ACTIVE and
COMPLETE acknowledgments have bounded IPC. Wrong-pair, early, late or duplicate
requests fail closed. The native route backend's consuming cleanup always stops
physical playback before releasing routes and USB; unconfirmed cleanup blocks
restart. Original scope remains owned objects, never tag/ID-selected deletion.

The GUI's native test events stay inside its own Xvfb window, leaving the owner's
mouse untouched. The wrapper-reported GUI PID must have the exact executable,
user and descendant relationship, then is retained with a Linux pidfd. Shutdown
checks the actual process rather than assuming wrapper exit means GUI exit.
Evidence and settings snapshots remain outside Git. USB access already existed;
no ACL, driver, firmware or desktop routing changes were needed in this batch.

## Successful physical run

Physical-stage journal SHA256:
`1a61ade9efcf467eebf73df524d3bf1ecdc6822839b5d5f567154327d299c6f9`.
Owner-report SHA256:
`c21c375c7ecceafd88e311f42471c2556df3ac181ee50204397379a529bf82fa`.

The owner moved mpv into first Link In and reported **Heard tones; mpv stayed in
Link In**. Source `beacn-native-source-20261015T024631Z`; hardware run
`beacn-studio-gui-20261015T024727Z`, result SHA256
`932cea339ecd87a578d4b96b3639123ea059628594da4adc52b2164a3cd5677a`.
Assignment 1 arrived at cycle 9; native stereo verification preceded physical
START. Control/publication continued while the selected PCM ran. Physical STOP
preceded cycle 39, followed by cycle 40 and verified route/USB release.

116 native active-PCM checks measured 1,356,890 frames of progress; 64 independent
physical observations measured 1,436,472. There was one physical start, no retry,
no timeout and no cleanup error. The real mpv source was preserved; all four
run-owned processes stopped, original private core restored, both PCMs closed,
and full laptop desktop/settings/routes/defaults/volumes/mutes/state and USB
binding comparisons matched. The Windows observation is the owner's report,
not captured audio/UI or a crosstalk measurement; streaming-PC saved settings
were not read or compared.

The GUI's own widgets select real observed mpv and enable actual auto-connect.
This proves bounded GUI-initiated first-pair interoperability on an owned private
core. It does not enable the normal executable or qualify arbitrary shared-desktop
apps, concurrent multi-pair physical routing, physical GUI pause/reconnect,
installation or persistent operation. The physical bridge remains external test
code rather than a completed standalone Rust playback implementation.

Five Linux Python checks cover IPC authority/sequencing and actual pidfd/ancestry
ownership; all 31 runner checks also pass. Eleven focused core and eight reference
GUI checks pass on identical core source. Native private GUI regression passes.
The production build omits qualification features, and explicit software rendering
passes an actual isolated window, read-only catalog, gray icon, normal close and
exact laptop settings/config/USB comparisons.

## Earlier control-only attempts

After graphics initialization was isolated to the installed Mesa CPU Vulkan
implementation, the real GUI completed 40 native USB initialization/publication/
poll cycles and verified route/transport release. The selected app remained at
assignment zero. No physical bridge was started and no tones were played. The
complete hardware test therefore remained **unaccepted** at that checkpoint, though its control
path passed. Laptop desktop/settings/routes/defaults/volumes/mutes/state and
connected USB binding matched afterward; both PCMs were closed and private core
and owned processes released.

Control-only run `beacn-studio-gui-20261015T023024Z` result SHA256
`fa320db79854e7184d453c845244b735e4e287648a7198789b9ffc73040d24c0`.
The intended audible run `beacn-studio-gui-20261015T023135Z` also received only
assignment zero, so it played nothing; result SHA256
`25741832e8955898973664fa0bde347d0bf2cca7721634fa22b6d04e7c447389`.
The owner subsequently said they had been waiting for a clear instruction to
participate. Do not infer visibility, a rejected drag, peer failure or audio
failure from these runs. Finish software checks/builds first, then ask them to
watch Applications **and all four Link In boxes**, move mpv to first Link In,
and reply **start now**. Start immediately after that reply, with no intervening
build or investigation. Quiet output starts only after received assignment 1.

Earlier graphics-startup timeouts occurred before device claim or publication.
Their raw journals are preserved. Pinning the test process to the installed
`lvp_icd.json` made its real window open and native control proceed. The precise
hybrid-driver stall is untraced, not a proven defect in a named GPU driver. No
driver was installed/reconfigured. `--software-renderer` is now an explicit
Linux production-preview fallback, scoped only to that process. Its environment
selection follows the [Vulkan loader contract](https://github.com/KhronosGroup/Vulkan-Loader/blob/main/docs/LoaderDriverInterface.md#overriding-the-default-driver-discovery).
Hardware-renderer and real-desktop performance acceptance remain open.

The native runner now records actual nonzero verified routing separately from
selected-pair physical activation, and validates the source before aggregate PCM
proof. Missing physical progress cannot erase earlier control/routing evidence,
but cannot turn an incomplete physical test into a passing run either.

Build the explicit test example with:

```sh
cargo +1.99.0 build --manifest-path app-gpui/Cargo.toml \
  --features studio-session-test --example studio_probe --locked --offline
```

Do not run the example without the owner-provisioned supervisor/core/baseline and
the corresponding hardware authorization. It has no fallback to desktop routing.
Next qualify pause/failure cleanup during physical output and implement the
standalone Rust playback owner, ordinary desktop routing and durable sessions.

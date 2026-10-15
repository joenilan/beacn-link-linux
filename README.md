# LinX

Independent, unofficial Linux compatibility research; not affiliated with BEACN.

A [visual design comparison](design/linx-concept/directions/index.html) now uses
the owner-selected Console default, with a routing matrix and compact utility
alternative. All three support Light/Dark and five accent palettes.
Its [image-based research](design/linx-concept/directions/README.md) compares
published BEACN, Wave Link, Sonar and UAD interfaces. The proposals target the
existing Rust/GPUI Kit app; all controls/data are fictional and change no audio
or production routing. Future Linux gain and remote BEACN mix controls remain
separate capabilities.

The primary [LinX interface](docs/gpui-interface.md) now uses GPUI Kit: a compact
window, responsive channel cards, expandable application details and a shared
lynx status icon/tray. Real widgets and private-session routing/cleanup pass native
Linux checks. The normal GUI keeps USB/device connection and desktop apply disabled.
The [explicit Studio GUI check](docs/native-studio-gui.md) now passes owner-heard
first-pair output and clean release on an owned test core; standalone playback
and ordinary desktop routing remain unfinished.
An optional [native Rust/C playback owner](docs/native-playback-owner.md) now
passes all four virtual mappings, native failure/release checks and GUI
Pause/reconnect/Quit. Its [first physical Studio pair](docs/native-playback-startup.md)
also passes with owner-heard mpv output and exact immediate/late laptop settings
comparisons. Physical Pause/reconnect, ordinary desktop integration and an
installed standalone app remain unfinished.
The earlier egui preview remains available for comparison. The primary UI also has
a saved Connect automatically toggle, waiting/paused states and read-only discovery
recovery. Automatic session tests use owned private cores; physical GUI connection
and launch-at-login installation remain future work.

Status: offline Rust codec/CLI, planner, lifecycle and graph/inventory owners,
an optional native PipeWire executor restricted to disposable private cores,
and a guarded Linux libusb transport. Native private-core routing, acknowledgments,
partial-failure recovery and owned cleanup are verified. Ordinary desktop apply
remains disabled.

The [Rust LinX application/service](docs/rust-application-service.md) now
provides native read-only application discovery, explicit multi-app selections,
identity/lifecycle tracking and a bounded foreground command loop. Same-core real
application exit/reopen, selection revocation and exact cleanup pass. USB publication
and desktop routing are not yet connected to this service.

The [Rust graphical preview](docs/rust-interface.md) now provides native discovery,
search, explicit selection and published-name editing. Its real window/widgets
pass isolated Linux tests with two actual apps and exact settings/state cleanup.
A status-aware lynx icon is shared by the window and optional Linux tray.
[Start/stop/session integration](docs/rust-link-session.md) passes actual private-core
GUI tests with a scripted protocol peer; normal desktop routing stays disabled.

The [multi-app integration](docs/multi-app-publication.md) now connects selection,
reply planning, native private routing and verified publication data. Two real mpv
processes simultaneously routed separate pairs, moved and recovered safely on a
disposable core. Multi-app USB/physical interoperability remains untested.

[Rust routing controls](docs/rust-routing-control.md) now provide start/stop/status
and verified lifecycle cleanup. On the private core, closing one of two real apps
released the owned routes while preserving a foreign link with the same owner tag.
The foreground application remains read-only; the graphical interface is implemented
and private-session GUI integration is qualified. Physical/desktop integration is next.

The owner placed the Linux test application in the streaming PC's first Link In
channel. Linux received assignment 1, applied its private stereo route and
published freshly verified assignment 1 through forty hardware cycles. This
establishes assignment/publication integration with virtual destinations.

The physical playback probe now verifies all four Studio stereo split pairs and
the complete internal hardware graph. The VM uses USB 2.0 EHCI. Native initialization
passes, but silent physical PCM stalls even with the vendor claim held open.
Owned cleanup and connected comparisons pass; reboot-wide state differences are
recorded separately. A separate Debian 13 laptop now completes four seconds of
96-kHz direct-ALSA silence with queue progress, natural exit and unchanged settings.
The native programs build there, and private A-to-B routing/partial-failure recovery
passes after explicitly configuring the test's stereo DSP ports. The owner heard
the quiet first-pair direct-ALSA tone. An explicitly selected private PipeWire
bridge also drove Studio's first pair with validated 96-kHz silent queue progress,
then passed A-to-B routing and partial-failure recovery with settings preserved.
Native laptop USB initialization passes. After reopening the streaming PC's
BEACN App, the owner assigned the Linux test into Link In; Linux applied its
private stereo route and published verified assignment 1 through cycle 40.
A combined native run now receives assignment 1, verifies that route, drives
Studio's physical first pair through PipeWire, and resumes verified USB publication
after owned audio cleanup. The owner heard the quiet tones and saw the test in
Link In. This is bounded first-pair acceptance with an owned test source.
A subsequent [continuous native test](docs/native-continuous-studio.md) kept
first-pair audio active while publication and polling continued through 40 cycles.
The owner heard repeating tones and confirmed the app stayed in Link In after
assignment. Exact laptop settings and owned cleanup comparisons passed.
A subsequent [real-application test](docs/native-application-routing.md) routed
unmodified mpv through each of Studio's four pairs. The owner heard every pair;
concurrent control and exact laptop cleanup passed. A no-USB source-exit check
stopped safely, and a fresh isolated session routed all four destinations.
The shared-desktop routing, multi-app hardware control, physical GUI integration and
long-lived recovery still need integration.
Read the current handoff and playback report before another test.
Windows Link stays closed during hardware tests.

Goal: a Linux gaming PC connected through BEACN Studio while the Windows
streaming PC retains BEACN App and Mix Create. First prove that one Linux
application can appear in Windows App and be routed from it.

- [Verified inventory](docs/inventory.md)
- [Static findings](docs/static-analysis.md)
- [Protocol evidence](docs/protocol.md)
- [Protocol session, focused static findings and validation](docs/protocol-session.md)
- [USB descriptor validator and VM-first hardware plan](docs/usb-descriptor-validation.md)
- [First physical descriptor observation and return comparison](docs/usb-descriptor-observation.md)
- [Native USB handshake implementation and proposed test](docs/native-usb-handshake.md)
- [Native USB publication, assignment polling and visibility test](docs/native-usb-publication.md)
- [Verified native assignment/publication integration](docs/native-assignment-publication.md)
- [Selected physical playback probe and remaining clock failure](docs/native-studio-playback.md)
- [Offline codec and usage](docs/offline-codec.md)
- [Offline routing planner](docs/offline-planner.md)
- [Offline routing preparation](docs/offline-executor.md)
- [Native private-core executor and validation](docs/native-executor.md)
- [Offline inventory lifecycle](docs/offline-lifecycle.md)
- [Read-only PipeWire inventory](docs/pipewire-inventory.md)
- [Read-only graph projection and receipts](docs/pipewire-graph.md)
- [Controlled virtual routing evidence](docs/controlled-virtual-routing.md)
- [Linux offline validation](docs/linux-validation.md)
- [Phased research plan](docs/research-plan.md)
- [REA prerequisites](docs/toolchain.md)

Keep proprietary binaries, raw captures, secrets, device serial numbers and
machine-specific endpoint GUIDs outside Git. Protocol identifiers and the
inspected executable hash are documented for reproducibility.
GitHub Actions must remain disabled; no workflow files are included.

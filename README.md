# beacn-link-linux

Independent, unofficial Linux compatibility research; not affiliated with BEACN.

Status: offline Rust codec/CLI, planner, lifecycle and graph/inventory owners,
an optional native PipeWire executor restricted to disposable private cores,
and a guarded Linux libusb transport. Native private-core routing, acknowledgments,
partial-failure recovery and owned cleanup are verified. Ordinary desktop apply
remains disabled.

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
Real desktop application
routing, audible PipeWire integration and a usable Linux replacement remain
unverified. Read the current handoff and playback report before another test.
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

# beacn-link-linux

Independent, unofficial Linux compatibility research; not affiliated with BEACN.

Status: partial static evidence plus a dependency-free offline Rust protocol
codec, CLI, provider-neutral routing planner, graph/inventory adapters and an
optional native PipeWire executor restricted to disposable private cores, plus a
transport-neutral protocol session controller validated with scripted transfers.
An optional Linux libusb transport completed a real one-shot A0/A1/A2 handshake
with Studio in the VM, released its interface claim and verified device cleanup.
The owner returned Studio to Windows; defaults/formats/volumes/mutes/sessions and
guest persistent files match the backups. Guest graph settings match after
accounting for recreated objects; Windows device-registry differences are recorded.
The first attempt had stopped before protocol exchange; its claim-observation fix
was built and tested before the successful separate attempt. The transport is
separate from the ordinary CLI. An explicit one-list/one-poll mode now builds and
passes focused checks; its real owned Linux source/cleanup probe preserved the
desktop exactly. Hardware publication and streaming-PC BEACN App visibility
remain unverified. Windows Link on the gaming PC stays closed during those tests.
The native executor verified A-to-B routing, server rejection, interrupted
acknowledgments and explicit rollback continuation, with unchanged desktop settings
and persistent-state hashes after owned cleanup. The suite has 76 Rust integration
tests and 33 Python tests, plus two native USB ABI/ownership unit tests. The latest
publication batch validates 25 focused Rust tests and 13 runner tests on both
platforms, native Linux builds/unit tests and the actual isolated source probe.
The probe subsequently validated one owner-authorized physical Studio descriptor
cache in the VM; the device was returned to Windows with logical settings matching
and persistent-state/identity differences recorded. Hardware initialization is now
verified; app-list publication, assignment exchange and audio interoperability
remain untested. The preceding native batch validated
12 focused recovery tests and actual private-core faults. Linux builds the C adapter against PipeWire
1.6.2. Normal desktop CLI apply remains disabled.
An earlier read-only graph preparation preserved two
already-correct foreign links and refused an incorrect destination serial.
The Linux adapter
also verified a real Firefox playback node in one populated snapshot, after an
initial empty inventory. A private disposable PipeWire core subsequently verified
one owned synthetic stereo stream moving from A to B, matching planner identities,
and complete cleanup with an unchanged desktop baseline. The earlier shared-core
attempt aborted before linking; both outcomes are recorded separately.
Remote discovery, desktop routing policy and hardware interoperability remain
unverified.
No Link replacement is working yet.

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

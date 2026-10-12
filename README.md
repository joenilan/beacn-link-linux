# beacn-link-linux

Independent, unofficial Linux compatibility research; not affiliated with BEACN.

Status: partial static evidence plus a dependency-free offline Rust protocol
codec, CLI, provider-neutral routing planner, graph/inventory adapters and an
optional native PipeWire executor restricted to disposable private cores, plus a
transport-neutral protocol session controller validated with scripted transfers.
The native executor verified A-to-B routing, server rejection, interrupted
acknowledgments and explicit rollback continuation, with unchanged desktop settings
and persistent-state hashes after owned cleanup. The suite has 61 Rust integration
tests and 20 Python tests; the latest descriptor continuation validates 25 focused
descriptor/session/codec tests on Windows/Linux and 5 new probe tests on Linux.
The probe operates on an explicitly named cached sysfs device; hardware use remains
unauthorized and untested. The preceding native batch validated
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

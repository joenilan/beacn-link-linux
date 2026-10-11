# beacn-link-linux

Independent, unofficial Linux compatibility research; not affiliated with BEACN.

Status: partial static evidence plus a dependency-free offline Rust protocol
codec, CLI and provider-neutral routing planner with synthetic tests.
Native Linux runtime and hardware
interoperability are untested; no Link replacement is working yet.

Goal: a Linux gaming PC connected through BEACN Studio while the Windows
streaming PC retains BEACN App and Mix Create. First prove that one Linux
application can appear in Windows App and be routed from it.

- [Verified inventory](docs/inventory.md)
- [Static findings](docs/static-analysis.md)
- [Protocol evidence](docs/protocol.md)
- [Offline codec and usage](docs/offline-codec.md)
- [Offline routing planner](docs/offline-planner.md)
- [Phased research plan](docs/research-plan.md)
- [REA prerequisites](docs/toolchain.md)

Keep proprietary binaries, raw captures, secrets, device serial numbers and
machine-specific endpoint GUIDs outside Git. Protocol identifiers and the
inspected executable hash are documented for reproducibility.
GitHub Actions must remain disabled; no workflow files are included.

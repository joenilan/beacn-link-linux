# Contributing to LinX

LinX is an unofficial BEACN Link app for Linux, licensed under GPL-3.0-only.
Contributions are welcome under that license. The current native application is
experimental; start with the current status and limits in [README.md](README.md).

## Where changes belong

- `app-gpui/`: the Rust GPUI Kit desktop application.
- `app-core/`: shared application state, selection, startup and session adapters.
- `src/` and `native/`: protocol, identity/receipt owners, PipeWire and USB support.
- `docs/`: protocol findings, implementation boundaries and reproducible evidence.

Use the existing native owners for behavior changes. A script may collect
diagnostics or drive the developer control interface, but must not become a
second routing or device/session implementation. Application names are labels;
they are not authority to adopt a replacement stream or delete another client's
PipeWire objects.

## Propose and validate a change

Describe the concrete problem, resulting behavior, and validation performed.
Keep changes focused. Run the checks relevant to the behavior you changed; the
[handoff](docs/HANDOFF.md) identifies the current build and evidence paths.
Reuse your checkout and build target directory. Check free space before a large
build and clean up obsolete generated output after checking that it is unused.

Separate unit checks, synthetic/private-core routing, physical interoperability
and human listening results. A passing build does not establish any of the last
three. Test hardware only when its owner has authorized the specific use; snapshot
settings/routes/defaults/volumes/mutes and persistent-state hashes first, then
verify cleanup. Do not restart the whole audio service as a substitute for
releasing owned resources.

GitHub Actions are currently disabled. Do not add workflows as an unrelated
change; use `[skip ci]` in commits while that policy applies.

## Reports and screenshots

Include the LinX commit, Linux distribution, PipeWire/WirePlumber versions,
observed behavior and relevant sanitized error text. Say whether the problem
occurred on physical Linux or through VM USB passthrough. Remove device serials,
host addresses, credentials and machine-specific endpoint identifiers.

Do not attach proprietary executables, raw traffic/audio captures or private
settings dumps. Keep a report small enough to review. For documentation images,
use [the native screenshot workflow](docs/screenshots.md) and review the actual
captured window before committing it.

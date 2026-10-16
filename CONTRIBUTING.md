# Contributing to LinX

LinX is an unofficial BEACN Link app for Linux, licensed under GPL-3.0-only.
Contributions are welcome under that license. Start with the current status
and limits in [README.md](README.md) and the design in
[docs/link-console.md](docs/link-console.md).

## Where changes belong

- `app-gpui/`: the GPUI desktop application (Console, tray, control socket).
- `app-core/`: the Link worker, preferences and shared app state.
- `src/` and `native/`: the PipeWire channel bus, Link engine, USB control
  channel and protocol codecs.
- `docs/`: how LinX works today. Keep research notes and experiment logs out
  of the repository.

Behavior belongs in the native Rust/C owners. Scripts may build, capture
screenshots or drive the developer control socket, but must not become a
second routing or device implementation. LinX only destroys PipeWire objects
it created, and routes applications only through session-manager metadata.

## Propose and validate a change

Describe the problem, the resulting behavior and how you validated it. Keep
changes focused and run the tests for what you changed:

```sh
cargo test --lib --features link-bus
cargo test --manifest-path app-core/Cargo.toml --features link --lib
```

Say which results come from unit tests, from a desktop without the Studio,
and from real Studio hardware. When testing on a desktop, note any routing or
volume settings the test changed and restore them.

GitHub Actions are disabled. Do not add workflows; use `[skip ci]` in commits.

## Reports and screenshots

Include the LinX commit, Linux distribution, PipeWire and WirePlumber
versions, what happened and any relevant error text. Remove device serials,
host addresses and credentials. Do not attach proprietary executables or raw
audio/USB captures. For documentation images use
[the native screenshot workflow](docs/screenshots.md) and review the actual
window capture before committing it.

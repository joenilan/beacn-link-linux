# Beacn LinX application

Rust/egui application preview. Real Linux application discovery, search, explicit
selection and published-name editing. Routing and USB are disabled.

```sh
cargo run --manifest-path app/Cargo.toml --locked --features native-catalog
```

Run as the normal Linux desktop user. Native catalog access remains read-only.
Selections last only until the window closes. See
[interface contract and qualification](../docs/rust-interface.md).

This separate manifest/lockfile keeps GUI dependencies out of default core builds.
The `ui-qualification` feature is for the isolated native-window test example only.

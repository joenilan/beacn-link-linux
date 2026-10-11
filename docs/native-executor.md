# Native executor on a disposable PipeWire core

Implemented over handoff `e5251db`, 2026-10-11. The default crate/CLI remains
offline; `link-protocol apply` and `--apply` still refuse application. The optional
`native-pipewire` feature adds a Linux backend and a separate `native_private`
example. It is not a production desktop router.

## Implementation contract

`src/execution.rs` reuses existing preparation/step validation, lifecycle tracker,
graph/ownership types and receipt journal. It observes before every forward and
recovery operation, requires the complete bound inventory to match the prepared
generation, and stops on any refusal. `src/native_pipewire.rs` owns one
single-threaded session and accepts no imported receipts. Explicit wire bindings
reuse the inventory adapter's client-serial/PID and node-serial identity patterns
and liveness policy through the existing bounded Rust metadata parsers.

`native/pipewire.c` uses public headers, core/registry listeners, node/port/client/
link info events and ordered core sync barriers. The connected Unix socket FD is
passed directly to PipeWire; `SO_PEERCRED` checks UID and expected server PID.
Native core name and cookie must match. There is no default-socket fallback.

The connection requires a direct owned mode-0700 `/tmp/beacn-pw-*` directory,
`beacn-test-*` socket and exact private `client.conf` without drop-ins.
Configuration redirection variables are refused. The client loads only
protocol-native. The remote module allowlist is protocol-native, metadata,
spa-node-factory, client-node, adapter, link-factory and access; any device or
other module stops the session. Both endpoint nodes require the session's
synthetic-test token and no `device.id`. Tags alone never authorize deletion.

Creates use exact selected port IDs, `link-factory` and `object.linger=false`.
Success requires creation-proxy bound/link-info events, synced registry identity,
exact endpoints/owner, paused or active native link state, and a fresh Rust graph
confirming exact node/port/link serials with no extra fanout. Deletion requires
the session's actual receipt plus fresh exact identities/owner and destroys the
retained **creation resource proxy**. It never calls `pw_registry_destroy(ID)`.
Success requires proxy removal, sync completion and fresh absence of that exact
link identity. Query-proxy cleanup releases bindings without deleting globals.

The factory uses numeric endpoint IDs without server-side compare-and-swap.
Events, barriers and post-verification detect races; they do not make graph
operations, stereo pairs or batches atomic. Disposable scope remains essential.
The core cookie identifies an instance; it is not authentication.

## Stopped work and recovery

Only observed native acknowledgments advance the ordered receipt prefix.
Pre-request refusals stop as Failed; uncertain issued requests stop as Unknown.
Unknown work cannot be retried or rolled back. Explicit reconciliation checks the
pending native proxy and fresh graph before settling a receipt or establishing
no surviving effect. Rejected journal evidence preserves Unknown. Settlement
never resumes forward work.

Explicit rollback reverses only the known successful prefix. Created-link removal
uses actual receipts; restoration creates links with new receipts/serials. Each
recovery operation uses the same fresh validation and native acknowledgment path.
Interrupted recovery retains its own journal and refuses silent restart. The later
[recovery continuation](native-recovery.md) adds explicit, freshly guarded
`resume_rollback`; forward work never resumes automatically. Session disconnection releases
non-lingering links; this is private-core teardown, not production route restoration.
There is no cross-process ledger persistence/import.

Registry history is capped at 40,960 records, native metadata at 8 MiB, and the
existing graph/planner bounds remain enforced. Departed observation records stay
until session close, so long-lived churn eventually fails closed. Every sync
barrier has a five-second deadline. Acknowledgment polling also has a five-second
loop deadline; its final pair of barriers can extend wall time beyond that.

## Build and explicit test commands

No Cargo dependency was added. Native builds require Linux, a C compiler, `ar`,
PipeWire/SPA headers >= 1.0 and installed `libpipewire-0.3.so.0`. Normally
`pkg-config` supplies include flags. Alternatively,
`BEACN_PIPEWIRE_SYSROOT=/absolute/extracted-root` selects isolated extracted
headers without package installation. Cross compilation is refused. PipeWire
1.6.2 was observed; other versions are untested.

```sh
cargo build --offline --locked -j 1 --features native-pipewire
cargo build --offline --locked -j 1 --features native-pipewire --example native_private
cargo test --offline --locked -j 1 --features native-pipewire
target/debug/examples/native_private --help
```

The example requires explicit runtime, core, cookie, PID, token, wire binding and
destination identities. Stdin commands are `route GENERATION FRAME-HEX`,
`reconcile`, `rollback`, `resume-rollback`, `quit`, and the test-only
`route-fail-before GENERATION FRAME-HEX STEP`. Generations must increase.
Failure injection stops **before** the selected native request; it does not
simulate server rejection or loss of a native acknowledgment.

`tests_native/private_core.py` is a standalone reproducible harness outside
ordinary unittest discovery. Run only with specific owner authorization:

```sh
python3 tests_native/private_core.py --authorize-isolated-routing \
  --executor /absolute/path/native_private --codec /absolute/path/link-protocol \
  --evidence-dir /absolute/new/private/evidence-directory
```

It snapshots filtered desktop settings, channels, routes, defaults, volumes/mutes
(Props), formats/port configuration, target/clock/settings metadata and relevant
WirePlumber file hashes before mutation. It starts only owned private server,
controller, synthetic playback and executor processes; then compares private and
desktop baselines after cleanup. Evidence is kept outside the repository.

## Observed validation

The owner authorized only the isolated VM test. Ubuntu 26.04.1, PipeWire 1.6.2,
GCC 15.2.0, Rust/Cargo 1.88.0 and Python 3.14.4 were used. SSH host-key checks
stayed enabled. Windows SSH agent discovery hung; `IdentityAgent=none` with the
existing dedicated key fixed that connection. No key, SSH configuration, VM
configuration or system package was changed.

Matching Ubuntu header packages 1.6.2-1ubuntu1.2 were downloaded into the isolated
build directory, checked against the existing APT index SHA256 and extracted
there without installation:

| Package | Bytes | SHA256 |
| --- | --- | --- |
| libpipewire-0.3-dev | 60502 | `acb939694a484a55fd1a8a266284ee2bb117c80f95def5102c58692c9524618c` |
| libspa-0.2-dev | 127432 | `855c99e14cd51eacb9ac2536a8a03efd0889e150ce4fdee57789774dd963ceb9` |

Offline locked single-job builds passed on Windows (default) and Linux (native).
**42 Rust integration tests and 15 Python tests** passed; Python invoked the built
CLI. Windows Rust formatting passed; C used clang-format 22.1.3. No Linux rustfmt
run is claimed. New behavior checks cover fresh inventory refusal, reused IDs,
receipt-based restoration with new serials, foreign link preservation, partial
failure, unknown outcomes, rejected reconciliation and interrupted recovery.
At this original checkpoint native fault/lost-ack outcomes were synthetic only;
the [later native fault tests](native-recovery.md) establish the bounded outcomes
listed there.

The final actual native test observed these active links. The same generated
private source ports were 24/25 throughout; these are not hardware identities.

| Stage | FL link ID/serial -> input | FR link ID/serial -> input | Journal |
| --- | --- | --- | --- |
| A | 27/32 -> 18 | 28/33 -> 19 | Complete, prefix 2 |
| B | 28/35 -> 20 | 27/36 -> 21 | Complete, prefix 4 |
| Controlled partial A | 27/38 -> 18 | absent | Failed, prefix 3 |
| Restored B | 28/41 -> 20 | 27/40 -> 21 | Recovery Complete, prefix 3 |

All six final checks passed: native routing, private filtered baseline restoration,
unchanged desktop filtered graph/settings, unchanged persistent-state hashes,
no desktop test objects, and removal of private runtime/configuration. Owned
processes were reaped and generated PCM removed. Earlier settings snapshots,
failed-run journals and source archives were preserved. Server, playback and
executor stderr were empty. The existing controller still printed
`unsupported type PipeWire:Interface:SecurityContext`; the whole harness was
not warning-free. No access policy was weakened.

The desktop filtered baseline contained 6 nodes, 10 ports, 2 links, 0 devices
and 4 metadata objects; it matched exactly without identity normalization.
Playback recorded exit 1 after controlled termination; executor and controller
recorded exit 0. The owned server was waited/reaped before runtime removal.
Guest free space was 23,712,223,232 / 23,691,841,536 bytes before/after final
validation. The owner was informed of the below-25-GB heavy-build threshold;
only small single-job builds ran and no data was deleted to make space.

The final native build used canonical staged Git blobs, hash-checked after
transfer. The following archive/source hashes exclude the later verification
prose update; the tested **code/fixtures hash** includes `src/`, `native/`,
`tests/`, `tests_python/`, `tests_native/`, `tools/`, `examples/`, `fixtures/`,
Cargo manifests and `build.rs`, and matches the final staged code. It uses sorted
relative path, NUL, per-file SHA256, LF concatenation, then SHA256.

| Artifact | SHA256 |
| --- | --- |
| Canonical tested code/fixtures | `aaeaed0d6213a4a116645ee45b51304467ced976ec223c0707a6a5ab17811315` |
| Transferred source archive before verification prose | `1b293f6103fdc01b7105e01ec0578bec36d35cb54ffba7d068d009bdd8b33940` |
| Extracted source before verification prose | `ea0975874918ed6b0bbd0690b9456564c4ac369052854b5ae47759f28aedbf4d` |
| Native executor executable | `f491d6af9cfa2c1d7ecfbe7f379e829d4446863a2913449ff39706de157a6bd8` |
| Runtime harness | `363e8637eea66c047af6db85912665c306b18fa6d6dfb0fa263e117c13d44a67` |
| Private final runtime journal | `147d4a72feea9d3324a83f985d73aaf848bdb5e0bd1720f7b983fef080e036be` |
| Private final build/validation journal | `10852cbb28e4fb71561392c24d8ce9be24d8ccc6415649b3736afce0035a58c7` |

GitHub Actions `enabled` was independently read as false. No workflow was added.

Two earlier diagnostic runs stopped after A, before B mutation, because a
registry bind raced a transient metadata-query client's departure. Both cleaned
up with unchanged desktop baselines. ENOENT on the exact read-only observation
proxy now records disappearance; surviving playback references to that missing
client still refuse projection. Creation-proxy errors remain pending/unknown;
other core errors stop the session. A later passing run exposed local-proxy
teardown warnings, fixed before final validation.

## Limits and next work

Native link state and receipt recovery do not prove audible output, measured PCM
delivery, BEACN USB transport, remote discovery or Windows interoperability.
No capture, hardware routing, WSL, driver/firmware or proprietary binary operation
ran. Desktop apply stays disabled.

Native rejection, acknowledgment interruption and explicit recovery continuation
are now covered by [the following batch](native-recovery.md). Remaining work is
core disconnect/restart and long-lived registry/resource bounds.
This slice adds FFI, socket/file/process and routing boundaries; a narrow Codex
Security review is recommended before expanding beyond disposable scope.
No paid scan was requested or run.

Sources: PipeWire 1.6.2 [core API and ordered sync](https://raw.githubusercontent.com/PipeWire/pipewire/1.6.2/src/pipewire/core.h),
[proxy events](https://raw.githubusercontent.com/PipeWire/pipewire/1.6.2/src/pipewire/proxy.h),
[non-lingering link resources](https://raw.githubusercontent.com/PipeWire/pipewire/1.6.2/src/modules/module-link-factory.c),
and [configuration resolution](https://raw.githubusercontent.com/PipeWire/pipewire/1.6.2/src/pipewire/conf.c).

# Native fault handling and explicit recovery continuation

Continuation of `a788787` on 2026-10-11. Ordinary desktop apply remains disabled.
The owner's isolated-VM-only authorization was continued; no physical device,
Windows routing, capture, WSL, driver/firmware or proprietary binary operation ran.

## Behavior

`Execution::resume_rollback` explicitly continues an existing stopped recovery.
Unknown pending work must first be reconciled. It requires the original bound
inventory, generation/core, valid graph, pending step and actual receipt to match
fresh observations **before** changing journal state. It preserves the existing
prefix, ownership ledger and serial history. Earlier acknowledged operations are
not repeated. Rejected continuation preserves the failed journal. Completing an
already reconciled last step requires no new operation. Ordinary `rollback`
still refuses restarting an existing recovery; forward work never resumes.

The native backend now keeps the creation proxy's local resource ID alongside
its actual global link receipt. Pending native errors are correlated with that
resource ID; numeric global IDs or owner tags never authorize deletion. Pure
failure/reconciliation descriptions now distinguish no surviving effect from
claiming an operation never transiently occurred.

## Opt-in native fault instrumentation

`native-test-faults` implies `native-pipewire`. Without that explicitly selected
feature, native fault controls and counters are absent from the C adapter/Rust
API. The existing private-runtime, core/module, node-token, peer and receipt guards
still apply. Test faults are consumed once, including on local refusal, so they
cannot carry over into recovery.

- `RejectCreate` sends a real native create request for a deliberately nonexistent
  factory. The server's error and resource-removal events establish rejection;
  fresh graphs show no surviving link. No server permission was changed.
- `WithholdAcknowledgment` submits the real operation through the native loop,
  deliberately returns Unknown before acknowledgment delivery to Rust, and keeps
  its pending proxy/identity. Explicit reconciliation processes native events and
  verifies fresh graph evidence. This tests the native request/receipt boundary;
  it does **not** drop network packets or disconnect the server.
- Native counters record request submissions, withheld deliveries and observed
  rejections. They are test evidence, not deletion authority.

The separate example supports `route-native-reject GENERATION FRAME STEP`,
`route-native-unknown GENERATION FRAME STEP`, `rollback-native-reject STEP`,
`rollback-native-unknown STEP`, `resume-rollback`, and `fault-stats` with the test
feature. `rollback-fail-before STEP` remains a pre-request orchestration fault.

```sh
cargo build --offline --locked -j 1 --features native-test-faults --example native_private
python3 tests_native/private_core.py --authorize-isolated-routing --fault-tests \
  --executor /absolute/path/native_private --codec /absolute/path/link-protocol \
  --evidence-dir /absolute/new/private/evidence-directory
```

The harness snapshots settings/channels/routes/defaults/volumes/mutes and relevant
persistent-state hashes first, then mutates only owned synthetic objects on its
private core. It retains every result and cleans up only owned processes/files.
All evidence remains outside Git. Existing header extraction and toolchain were
reused without package installation or changing SSH host-key checks.

## Actual private-core results

The backend passed these cases on Ubuntu 26.04.1 / PipeWire 1.6.2, GCC 15.2.0,
Rust/Cargo 1.88.0 and Python 3.14.4:

| Case | Observed stop | Explicit completion |
| --- | --- | --- |
| Rejected B-to-A create after two disconnects | Forward Failed, prefix 2; no links | Rollback restored B with two new receipts |
| Withheld B-to-A create acknowledgment | Forward Unknown, prefix 2; one native A link exists | Reconcile acknowledged prefix 3 without resubmission; rollback restored B |
| Withheld acknowledgment during rollback | Recovery Unknown, prefix 1; two removals actually completed | Reconcile settled prefix 2; resume restored B with exactly two requests |
| Rejected restoration during rollback | Recovery Failed, prefix 2; no links | Explicit resume completed two restorations |

Blocked routing, rollback and continuation commands submitted zero requests.
Reconciliation submitted zero requests. Final native counters were **40 request
submissions, two deliberately withheld deliveries and two observed rejections**.
Each successful restoration used fresh serials rather than treating old IDs as
recreated identities. Server/playback/executor stderr were empty. The controller
still reported unsupported SecurityContext and transient stale-resource errors;
the overall harness was not warning-free and permissions were not weakened.

A bounded client-churn smoke check ran **32** increasing-generation already-correct
routes, interleaving fresh metadata-query clients. It preserved the final link
IDs/serials and submitted no additional mutations. This is not an hours-long
soak, a test of the 40,960-record cap, or proof of bounded lifetime resource use.

All seven requested checks passed: fault cases, ordinary routing, private filtered
baseline restoration, unchanged desktop filtered graph/settings, unchanged
persistent-state hashes, no desktop test objects and runtime/config removal.
Earlier snapshots and archives were preserved. The desktop comparison includes
the existing unrelated stream and its links. Only owned test processes were
terminated/reaped; generated silent PCM was removed.

Windows and Linux passed the **12 focused execution/receipt tests**, including
two new checks for continuation without repeating a prefix and refusal of a
reused receipt while preserving stopped state. Linux built both normal native
and fault-enabled configurations. Windows formatting and C formatting checks
passed. No redundant full test matrix or Linux rustfmt run is claimed; the suite
now contains 44 Rust integration tests and the previous 15 Python tests.

The final desktop baseline contained 6 nodes, 10 ports, 2 links, 0 devices and
4 metadata objects, matching after cleanup without identity normalization.
Guest free space was 23,497,945,088 / 23,497,826,304 bytes before/after the final
lightweight validation. The owner was informed of the below-25-GB heavy-build
threshold; no source, existing evidence or user data was removed to make space.

Source files were transferred with canonical LF bytes and verified per file
before both native configurations were built. The code hash below selects
`src/`, `native/`, `tests/`, `tests_python/`, `tests_native/`, `tools/`, `examples/`,
`fixtures/`, Cargo manifests and `build.rs`; it uses the previous sorted
path/NUL/file-SHA256/LF convention and matches the final staged code.
The archive predates this verification prose; journals and archives remain private.

| Artifact | SHA256 |
| --- | --- |
| Tested code/fixtures | `db7d707c32865153cdba1219f181f00a8c49d6af0934dfa3dd01867d2da59964` |
| Transferred source archive | `eeaf2c0d0f822c0a15ac2960fca2f3bfd7a808932ad284a9e14262111333a6f7` |
| Fault-enabled native executor | `01403aa533c6bfa114470dce3d22d271639c15c02ec441f9ea011209e16fe5c0` |
| Runtime harness | `4aaa3c37ff408aa4ae71cbc728a83ca24d0ec6149727d652d25739d8262908b2` |
| Final private runtime journal | `c5cacd734ec0d8f040eea05a779f8334d5ff3f7546f7d3b4c84bd7134b419373` |
| Final private build/validation journal | `85e4e914a15254e71935c012abab995e277443d544fd379b3879997b9319ee0b` |

GitHub Actions was independently rechecked: `enabled=false`. No workflow was added.

## Remaining work

Test core disconnect/restart while a request is pending, deliberate permission
rejection and bounded registry/resource retirement under long-lived churn.
Capacity exhaustion currently fails closed; departed proxies/observation history
are retained until session close. No cross-process ledger recovery exists.
Hardware transport, measured/audible PCM delivery and Windows interoperability
remain unverified; normal desktop apply remains disabled. The earlier narrow
security-review recommendation still applies; no paid scan was requested/run.

Native event/lifetime references: PipeWire 1.6.2
[core error and resource-removal dispatch](https://raw.githubusercontent.com/PipeWire/pipewire/1.6.2/src/pipewire/core.c) and
[proxy removal/destruction](https://raw.githubusercontent.com/PipeWire/pipewire/1.6.2/src/pipewire/proxy.c).

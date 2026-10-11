# Read-only PipeWire application snapshot

`tools/pipewire_inventory.py` reads one default-user PipeWire graph snapshot
and can feed explicit application/wire bindings to the existing pure Rust
planner. It reads metadata only: no stream creation, routing, device changes,
daemon, startup behavior, audio samples or USB traffic.

Python 3's standard JSON parser handles the external format without adding Rust
crates or pip packages. Live mode additionally requires installed `pw-dump`
(Ubuntu package `pipewire-bin`) and an accessible existing user session.
The adapter never starts audio services or requests sudo.

## Run

From a built source directory:

```sh
python3 tools/pipewire_inventory.py --snapshot-id 7
python3 tools/pipewire_inventory.py --dump fixtures/pipewire.synthetic.json \
  --snapshot-id 7 --bind 'pw-client:70:pid:2001=game' \
  --plan 0e0000ac040167616d650500766f69636500 \
  --planner-bin ./target/debug/link-protocol
```

The fixture is handwritten and represents two playback nodes from one client.
Its metadata name `Native Game` is separate from the explicit wire alias `game`.
Without `--bind`, no observed application is advertised automatically.
Bindings to absent identities, duplicate bindings and unsupported wire names
are rejected. Two different identities may deliberately share a wire name;
the existing planner then reports `AmbiguousName` rather than choosing one.
`--plan` accepts only a supplied data-frame hex string and invokes only the
built pure planner; it does not receive or send BEACN messages.

Default output is filtered JSON metadata. It excludes raw device properties,
process arguments, user/host properties, media titles and paths read from the OS.
Keep actual session output private; do not commit raw snapshots.

## Identity contract

The adapter selects only `PipeWire:Interface:Node` objects whose
`media.class` is exactly `Stream/Output/Audio`. Capture streams, hardware
sinks/sources, ports and links are ignored.

| Field | Meaning and use |
| --- | --- |
| Top-level node `id` | PipeWire global node ID; diagnostic only, never a PID or Link number |
| Node `object.serial` | Creates snapshot-local stream ID `pw-node:SERIAL` |
| Node `client.id` | Resolves the Client object in the same dump |
| Client `object.serial` | Identifies that client instance in this graph |
| `application.process.id` | Advertised process PID; node property preferred, client property fallback, provenance recorded |
| Application ID | Adapter label `pw-client:CLIENT-SERIAL:pid:ADVERTISED-PID`; groups streams within this snapshot |
| `application.name`, `application.id`, `application.process.binary` | Display metadata only; each field uses node property then client fallback |
| Explicit `--bind ID=NAME` | Supplies the exact printable-ASCII wire identity, 1..255 bytes |

PID and application properties are metadata claims, not authenticated OS
identity. No executable lookup, process-path normalization, name derivation or
`/proc` inspection occurs. Display values are capped at 512 characters.

Serials and PIDs are not persistent application IDs. Different clients with
the same PID stay separate. Snapshot IDs are caller-supplied version labels
(default: current time in nanoseconds), not a freshness guarantee.
Bindings must be re-established against newly observed identities after
client/server lifecycle changes. This adapter stores no history or tombstones.

Missing client/serial/PID metadata is reported as unresolved playback streams;
any unresolved stream blocks the entire planner projection. Duplicate graph
IDs or relevant serials and oversized/malformed input reject the snapshot.
This prevents presenting missing identity metadata as a valid empty graph.

Node states `running`, `idle` and `suspended` map to `Live`, meaning present
with a recognized state in this dump. Other/missing states map to `Stale`,
which makes the core block that application's proposals. `Live` does not
promise current audio, a usable device, routability, or continued existence.
No omitted object is inferred to mean unroute.

## Session and input bounds

Live mode requires the current user's `XDG_RUNTIME_DIR/pipewire-0` socket.
It explicitly queries `pw-dump -N -r pipewire-0` in that runtime directory,
without monitor mode. The query has a five-second budget and limits stdout
to 8 MiB and stderr to 16 KiB. Only the adapter's own dump process can be
terminated on budget failure; audio services are untouched.

An unavailable session/tool/query produces `status: unavailable`, null planner
inventory and exit 3. A successful dump containing zero playback streams
produces `status: available`, a valid empty inventory and exit 0.
Invalid inputs or bindings exit 2. Planning returns the pure CLI's exit code.
At most 16,384 graph objects, 256 applications and 4,096 playback streams are
accepted.

The Rust bridge `link-protocol plan-snapshot SNAPSHOT-ID DATA-FRAME-HEX`
reads at most 2 MiB of UTF-8 TSV on stdin. Each row is:

```text
application-id<TAB>explicit-wire-name<TAB>stream-id<TAB>live|stale|disappeared
```

IDs are nonempty printable ASCII, at most 128 bytes; wire names use the
existing 255-byte ASCII policy. Rows for the same application must retain one
wire name, and core validation rejects duplicate stream ownership.
The bridge creates pure proposals and performs no OS/device enumeration.

## Verification

Verified on 2026-10-11 from an uncommitted slice over base commit
`5d90439aca8caad758ff568237b89542e15df0a6`. Only this slice's 13 intended edits
were overlaid on a Git archive of that base; other working-tree content was not
included. The 34,188-byte archive was transferred to a new isolated guest
directory. Both archive and extracted file-tree SHA-256 checks passed before
building. Verification documentation was finalized afterward. A byte comparison
proved the tested code/fixtures match Git content after CRLF-to-LF normalization;
no non-line-ending content differences were present. Existing files were not
rewritten solely to change line endings.

| Artifact | SHA-256 |
| --- | --- |
| Transferred archive | `493edc879715d4e2c6f280fef0bb17f8592e10d309f3f29ba04b98570c5bad70` |
| Extracted source tree, before this verification text | `acafaabf9c8e0849d70b2c13593dceeb6394487e82f3561100f27bec2d583bc4` |
| Tested raw code/test/tool/example/fixture tree and Cargo manifests | `de8c1b4d985661d3e0a0dc6234f3962465bdf9e1fa05eee0a81289f319825bf0` |
| Git-normalized code/fixture tree, CRLF converted to LF | `8872cdf6fc3edbd02d450257eb91105530adfd11a74a6e3f4bc6ec779999f6b2` |

Tree hashes are SHA-256 of UTF-8 lines `relative-path + NUL + file-SHA256 + LF`,
sorted by relative POSIX path. Directories are omitted. The code/fixture hash
selects `src/`, `tests/`, `tests_python/`, `tools/`, `examples/`, `fixtures/`,
`Cargo.toml` and `Cargo.lock`. The Windows Git archive exported nine existing code/fixture files with CRLF,
while their stored Git blobs use LF; the comparison confirmed those are the
only differences. The normalized hash applies CRLF-to-LF before hashing each
selected file and is verified again against committed blobs. Exact per-file
manifests and full validation logs are retained privately outside Git.

### Builds and synthetic checks

Windows used the existing Rust/Cargo 1.88.0 and Python 3.12 toolchain.
Formatting, `cargo build --offline --locked -j 1`,
`cargo test --offline --locked -j 1`, Python unittest discovery and
`git diff --check` all exited 0. All 17 Rust integration tests (8 codec,
7 planner, 2 snapshot bridge) and all six Python tests passed. Python checks
set `BEACN_TEST_PLANNER_BIN` to the built CLI, so identity projection,
multi-stream proposals and name collisions crossed the actual stdin bridge.

Linux was freshly verified as Ubuntu 26.04.1 LTS, x86_64, kernel
7.0.0-38-generic, using existing isolated Rust 1.88.0
(6b00bc388 2025-06-23), Cargo 1.88.0 (873a06493 2025-05-10),
GCC 15.2.0 (Ubuntu 15.2.0-16ubuntu1), Python 3.14.4 and PipeWire 1.6.2.
Strict verification of the existing SSH host key succeeded after the upgrade.
No new toolchain, crate, pip package or OS package was installed.

The Linux commands below all exited 0:

```sh
cargo +1.88.0 build --offline --locked -j 1
cargo +1.88.0 test --offline --locked -j 1
BEACN_TEST_PLANNER_BIN="$PWD/target/debug/link-protocol" \
  PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests_python -v
# Synthetic adapter invocation and live invocation: examples above and below.
```

All 17 Rust integration tests and six Python tests passed on Linux. The
handwritten two-stream fixture proposed both stream IDs for Link One under
the explicit `game` binding and reported `voice` assignment zero as opaque.
Malformed/oversized identities, duplicate metadata/ownership, unresolved
metadata, stale state, collisions and unavailable-versus-empty behavior are
covered synthetically.

### Actual read-only session outcome

The earlier Ubuntu 24.04.5 preflight read 52 PipeWire objects with zero playback
streams. After the owner upgraded the guest, the 26.04.1 preflight read 55
objects with zero playback streams; the existing three user audio services
were already active. Prior offline Linux evidence remains in
[Linux validation](linux-validation.md).

The actual new adapter was then run against the existing 26.04.1 session:

```sh
python3 tools/pipewire_inventory.py --snapshot-id 8 \
  --plan 0e0000ac040167616d650500766f69636500 \
  --planner-bin ./target/debug/link-protocol
```

It exited 0 with `status: available`, zero playback streams, zero applications,
zero unresolved streams, `planner_ready: true`, and a valid empty planner
inventory. The pure planner returned `UnknownName` for `game` and
`OpaqueAssignment { value: 0 }` for `voice`; it proposed no routes.
This verifies access, parsing and empty-snapshot behavior. It does not verify
identity selection on a populated real application graph, audible output,
remote discovery, protocol transport or BEACN interoperability.

The archive was 34 KB and the dependency-free build was run with one job.
Free space before/after this build was 24,738,742,272 / 24,642,236,416 bytes;
target-file sizes summed to 116,082,577 bytes. The owner was notified that free
space was below the 25 GB threshold for heavy builds; this small existing-toolchain
build proceeded under the clarified rule. No cleanup or heavy download occurred.

### Invocation review and remaining limits

Both subprocess calls use argument arrays with no shell; supplied aliases,
hex and paths are never concatenated into a shell command. Dump time/output
budgets and JSON/identity bounds are enforced before projection. The optional
pure CLI call has a five-second timeout; its output is bounded by the Rust
frame/inventory limits. Launch/session failures return unavailable with no
inventory, and malformed snapshots cannot become a valid empty projection.

No audio samples, raw USB traffic, BEACN devices, route writes, service
start/stop, stream creation or firmware/driver changes were involved.
Snapshot state is a momentary observation, and an eventual executor would still
need to revalidate identity and liveness before applying any proposal.

## References

- [pw-dump: JSON snapshot and remote selection](https://docs.pipewire.org/page_man_pw-dump_1.html)
- [PipeWire key definitions: IDs, serials, client and application metadata](https://docs.pipewire.org/group__pw__keys.html)
- [Pure routing planner](offline-planner.md)

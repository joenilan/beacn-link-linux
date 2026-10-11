# Offline inventory lifecycle

The pure `lifecycle` module fills the gap between independent inventory
snapshots. It reuses `Inventory`, `ApplicationId`, `StreamId`, `Stream`,
`StreamState` and `Plan` from the existing planner. It reads no OS state,
polls nothing and applies no routes. The PipeWire adapter remains a one-shot
reader; no daemon, watch loop, automatic wire binding or executor was added.

## Use and ordering contract

`InventoryTracker::default()` starts without a snapshot.
`advance(&inventory)` validates the complete supplied inventory using existing
planner checks, computes an `InventoryDelta`, and stores an owned copy.
`current()` returns the latest accepted inventory for a fresh call to `plan`.
All rejected updates leave that inventory unchanged. Callers stop a failed update
iteration; retained data is the last accepted snapshot, not a new observation.

The first snapshot may use any u64 version. Later versions must strictly
increase within one tracker. This is the tracker's caller policy, not a
PipeWire sequence contract or change to the core planner. Prefer an ordered
local generation counter. A timestamp that goes backward will be rejected;
a greater timestamp does not prove physical freshness.

Only complete, successfully resolved inventories belong in `advance`.
An unavailable query, unresolved graph or partial result must not be represented
as an empty inventory. A genuinely empty complete snapshot can report app
closure. Start a new tracker for a new server/session identity domain; matching
serials/PIDs across a server restart are not evidence of the same object.

```rust
use beacn_link_protocol::{lifecycle::InventoryTracker, planner::plan};
// first and next are complete caller-supplied Inventory values;
// records are already decoded assignment records.
let mut history = InventoryTracker::default();
let first_delta = history.advance(&first)?;
let old_plan = plan(history.current().unwrap(), &records)?;
history.require_current_snapshot(&old_plan)?;
let next_delta = history.advance(&next)?;
assert!(history.require_current_snapshot(&old_plan).is_err());
let fresh_plan = plan(history.current().unwrap(), &records)?;
history.require_current_snapshot(&fresh_plan)?;
```

`require_current_snapshot(&plan)` checks only that the plan refers to the
latest accepted generation. It reports no-snapshot or generation-mismatch
errors; it does not authenticate plan contents or authorize audio changes.
An eventual executor still needs fresh OS identity/liveness checks.

## Transition policy

| Change | Result |
| --- | --- |
| App identity appears/disappears | Sorted application-ID delta |
| Stream appears | Its supplied state, with no previous state |
| Stream is omitted from the next complete inventory | Disappeared delta carrying its prior owner/state |
| Existing stream changes state | Delta with the new Stream and previous state |
| Same stream ID moves between application IDs | Rejected update; previous inventory retained |
| Equal or older snapshot version | Rejected update; previous inventory retained |
| Invalid ownership, names or planner limits | Existing inventory error; previous inventory retained |

Stream changes sort by StreamId; application changes sort by ApplicationId.
Missing streams are emitted once as delta tombstones and are not inserted
into the new current inventory. Thus a closed app does not leave an old
advertised name that collides with an explicitly bound reopened app.
The tracker does not derive or transfer aliases; every next inventory supplies
its own explicit wire bindings. No omission implies an unroute action.

PipeWire global node IDs are diagnostic only. The adapter already uses node
object serials for StreamId and client serial plus advertised PID for
ApplicationId. A reused node ID with a new serial is a new stream; a reopened
client with the same PID and new client serial needs a new explicit binding.
The new synthetic adapter check verifies both cases.

The existing planner continues to propose all live streams for a uniquely
bound app and block the whole app when any supplied stream is stale or
disappeared. After a stream is added/removed or recovers, callers build a new
plan from `current()`; saved plans remain tied to their original generation.

## Verification

Prepared over base commit `d045ac624e336895d58cdbff6d389ab4b2786727`.
Windows build, formatting, `git diff --check`, 21 Rust integration tests and
seven Python tests passed using existing toolchains and offline, locked,
single-job Cargo. Four new Rust checks cover close/reopen, changing multi-stream
sets and state recovery, rejection/rollback and deterministic deltas.
One new Python check uses handwritten successive graphs to distinguish node ID
reuse from serial reuse and reject an old client binding after reopen.

On 2026-10-11 the same slice passed on freshly verified Ubuntu 26.04.1 LTS,
using existing isolated Rust/Cargo 1.88.0, GCC 15.2.0 and Python 3.14.4.
Strict SSH host-key verification succeeded. Offline, locked, single-job Cargo
build and tests, and Python unittest discovery with the built planner path,
all exited 0: 21 Rust integration tests and seven Python tests passed.
No packages, dependencies or toolchains were installed.

The 42107-byte archive contained the committed base plus only this slice's
8 intended source/documentation edits. Git archive used a temporary
`core.autocrlf=false` setting to export stored LF bytes without rewriting the
checkout. Archive and extracted-file hashes were checked before building;
verification text was finalized afterward.

| Artifact | SHA-256 |
| --- | --- |
| Transferred archive | `95b37403d1814fbccb0e9e50892406d62213035c212cf812fe8ea01dc8e803e7` |
| Extracted tree before finalized verification text | `3b40f6cc744c5f4da82d1a78949cfba879bb50e970f7bf959613fd9bfb17ba4e` |
| Tested code/fixtures and Cargo manifests | `3aebc658263f0ee945739b1062073b63a042327803991f88322b79e5189f0449` |

Tree hashes use the same sorted relative-path/NUL/file-hash/LF algorithm as
[PipeWire validation](pipewire-inventory.md). The code hash selects `src/`,
`tests/`, `tests_python/`, `tools/`, `examples/`, `fixtures/`, `Cargo.toml`
and `Cargo.lock`; committed blobs are checked against that hash.
Full command logs and exact manifests remain private outside Git.

Before/after build free space was 24,364,109,824 /
24,241,795,072 bytes; target file sizes summed to
145,713,446 bytes. This was a small build using the existing toolchain;
no cleanup or heavy download occurred. The owner was kept informed that available
space remains below 25 GB for heavy builds.

No live browser snapshot, actual OS lifecycle transition, audio routing,
USB transport or hardware interoperability was tested. The only PipeWire
command in this validation was a version query; synthetic Python tests use
handwritten graphs and mock the unavailable-session path.

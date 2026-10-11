# Read-only graph projection and acknowledgment receipts

This implemented slice adds `tools/pipewire_graph.py`, the pure Rust
`graph_snapshot` bridge and `receipts` journal. It reuses the existing planner,
inventory adapter, lifecycle tracker and executor types. Actual route application
remains disabled; no native mutation or rollback backend exists.

## Observation and identity contract

The Python reader uses the existing bounded `collect_live()` query against the
current user's default `pipewire-0` socket. Inspect mode takes one snapshot;
live preparation takes an anchored snapshot and a second observation. File-based
preparation requires both `--expected-dump` and `--dump`; a supplied file is
never represented as a fresh live observation.

One native Core object and its unsigned 32-bit `info.cookie` are required.
The existing graph core label becomes `pw-core:COOKIE`; user/host names are
not exported. PipeWire documents this cookie as identifying the server instance.
It is a finite random identifier, not authentication or persistent identity.
Equal cookies and a caller generation do not establish atomicity or freshness.
A changed cookie makes existing preparation fail.

Application and stream IDs remain exactly the inventory adapter's
`pw-client:CLIENT-SERIAL:pid:ADVERTISED-PID` and `pw-node:NODE-SERIAL`.
No new application owner scheme, process lookup or automatic alias is introduced.
Unresolved playback metadata blocks projection. Recognized stream states retain
the adapter's existing live/stale policy. Preparation refuses a change in the
explicitly bound inventory between observations.

Projection includes nodes with `Audio/*`, `Stream/Output/Audio` and
`Stream/Input/Audio` media classes, their ports and audio links. Capture-node
metadata is included only to resolve graph endpoints; no audio is captured.
Objects carry numeric IDs plus serials. Ports carry exact parent-node references,
direction and FL/FR/Other channels. Monitor ports are retained; audio control ports
are rejected. Missing channel/direction metadata, inconsistent directions or link
properties, unresolved audio endpoints and duplicate represented IDs/serials
are refused. Foreign audio links are retained rather than silently omitted.
Other channels remain represented and make selected non-stereo mappings fail
under the existing executor policy.

Output contains filtered graph and explicit inventory metadata. Browser/media
titles, user/host names, command arguments and device properties are not exported.
Bounds reuse the 8 MiB dump/16,384-object query and existing graph limits:
8192 nodes, 16,384 ports, 16,384 links and 4096 bindings. Query budget remains five
seconds, with 16 KiB stderr. Only an owned query process may be terminated.

## Explicit dry-run preparation

`--prepare DATA-FRAME-HEX` requires `--executor-bin`, an explicit owner token,
explicit application `--bind ID=WIRE-NAME` values for requested applications,
and `--destination ASSIGNMENT:NODE-ID:SERIAL` mappings. Assignments 1 through 4
are supported. Duplicate assignments or sharing a destination identity are refused.
No sink is selected by its name, claimed automatically or assumed to be BEACN.

An optional graph property `beacn.link.owner` supplies a reported owner tag.
This is a metadata convention for a future backend, not an observed BEACN
protocol field. It never creates a trusted receipt. Caller-supplied
`--receipt LINK-ID:SERIAL` values populate the separate ledger; the executor
still requires both tag and matching receipt to describe removing an old link.
A fabricated caller ledger cannot be authenticated by these pure functions.

The Python tool invokes only the built pure CLI:
`link-protocol prepare-graph GENERATION DATA-FRAME-HEX`.
Its bounded UTF-8 TSV stdin contains the following tagged rows:

| Row | Fields after the tag |
| --- | --- |
| `inventory` | application ID, explicit wire name, stream ID, state |
| `expected` / `observed`, then `core` | core label |
| `expected` / `observed`, then `node` | ID, serial |
| `expected` / `observed`, then `port` | ID, serial, node ID, node serial, in/out, FL/FR/other |
| `expected` / `observed`, then `stream` | application ID, stream ID, node ID, node serial |
| `expected` / `observed`, then `link` | ID, serial, output ID/serial, input ID/serial, optional owner |
| `destination` | assignment, node ID, node serial |
| `owner` | token |
| `receipt` | link ID, serial |

Fields use tabs and rows use newlines. Empty link-owner fields mean unowned.
Unknown/malformed rows, duplicate core/owner records and missing metadata fail.
The 8 MiB bridge reuses the existing strict inventory parser and graph validator,
then feeds the tracker, planner and `executor::prepare`. Output begins with
`DRY RUN` and lists descriptions only. `apply` and `--apply` still fail.
Two snapshots and pure checks do not remove races after observation.

## Pure acknowledgment bookkeeping

`ReceiptJournal` consumes an unmodified prepared change, its pre-operation
graph and trusted ownership ledger. It accepts only the next acknowledgment:
either the exact disconnected link reference or an actual created link receipt
whose endpoints and owner match the operation. Wrong identities, mismatches,
out-of-order events and occupied-ID/previously-seen-serial collisions fail without
advancing the prefix or changing the ledger. A removed numeric ID may be reused
with a new serial. Successful disconnects remove receipts; successful creates
add their actual ID/serial.

Failure and unknown outcomes stop progression without acknowledging pending
work. Rollback descriptions cover only the known successful prefix in reverse
order and resolve created-link removals to the actual receipt. An unknown
pending effect is excluded and must be reconciled before any real recovery.
No automatic retry, native acknowledgment subscription, rollback validation,
route application or recovery operation is implemented. Caller acknowledgment
events remain trusted data, not independent proof of a completed operation.

## Validation state

Prepared over main commit `43c9453`. On 2026-10-11, Windows formatting and
formatting-check, offline locked single-job Cargo build and tests, and Python
unittest discovery all passed: **36 Rust integration tests and 15 Python tests**.
`BEACN_TEST_PLANNER_BIN` selected the actual built CLI, so Python bridge
checks executed Rust preparation and existing planner checks rather than skipping
them. New tests cover graph identity reuse, filtered/deterministic metadata,
missing/conflicting metadata, foreign-link retention, explicit destinations,
changed liveness, core changes, partial/malformed input, receipt collisions,
ordered acknowledgments and failed/unknown outcomes.

The identical source passed on the existing Ubuntu 26.04.1 guest with
Rust/Cargo 1.88.0 and Python 3.14.4: offline locked single-job build,
all **36 Rust integration tests** and **15 Python tests**, including the actual
Python-to-Rust bridge. No package or toolchain was installed. The archive and
every extracted file hash were checked before building.

| Artifact | SHA-256 |
| --- | --- |
| Transferred 66067-byte archive | `55000fbf5a9b35939f212fc78516aa1881e84059436e3aa513e5215257d25340` |
| Extracted source before final documentation | `fe845ee089025a6caa91949ec82b9f6d1c6e41e658818fde1b3f13c34888163c` |
| Tested code/fixtures and Cargo manifests | `16f72f97780d2f33d9b886ad721c20980a1a56b5c0f5bb003825e665c91c1c1e` |

Tree hashing follows the sorted relative-path/NUL/file-hash/LF convention in
[offline lifecycle validation](offline-lifecycle.md). The archive contained
committed base `43c9453` plus only nine intended code/test edits. Final
documentation is outside the tested code hash; staged code blobs are checked
against that hash before commit. Full manifests and logs remain private.

The successful small Linux build had 24,006,537,216 /
23,775,522,816 bytes free before/after; target files totaled
281,175,151 bytes. The owner was informed that free space
was below 25 GB for heavy builds. No cleanup, dependency download or heavy build
occurred. Existing Linux rustfmt unavailability is already documented in
[offline preparation](offline-executor.md); Windows formatting-check passed.

### Actual read-only preparation

The tested reader's functions collected two bounded metadata snapshots from
the existing user session. They reused the previously observed application
identity under the manually supplied wire alias `observed-app`; assignment 1
was explicitly mapped to a previously observed destination node ID/serial.
No destination was selected by name or inferred by the program.

Each projected graph contained **two audio nodes, six ports and two links**.
The bound inventory contained **one live playback stream**. Two existing
foreign links already matched the explicit stereo destination. The actual Rust
`prepare-graph` command exited 0 and printed:

```text
DRY RUN: 0 guarded operations; actual application disabled
```

A second pure invocation reused those same observations and changed only the
configured destination serial. It exited 2 with `ReusedObject`, distinguishing
the configured serial from the observed serial instead of accepting numeric
ID alone. It performed no additional live query. Filtered snapshots, stdin
and result logs remain private outside Git; no raw dump or browser/media titles
were saved.

This validates one real already-correct route through graph projection and
pure preparation, including foreign-link preservation and serial refusal.
It does not validate a proposed move on a live server, native mutation,
acknowledgment receipt capture, acoustic output or BEACN interoperability.
The receipt journal was tested synthetically only. No route, audio-service,
USB or hardware operation ran, and earlier private baselines were preserved.

## Remaining work and references

Native operations/acknowledgments, fresh rollback checks, robust session handling,
USB discovery and Windows interoperability remain unimplemented. The next native
backend must preserve foreign routes and desktop fallback, revalidate current
identities before operations and reconcile unknown outcomes before recovery.

- [Existing executor contract](offline-executor.md)
- [Existing inventory identity/query contract](pipewire-inventory.md)
- [PipeWire core info cookie definition, version 1.6.2](https://gitlab.freedesktop.org/pipewire/pipewire/-/blob/1.6.2/src/pipewire/core.h)

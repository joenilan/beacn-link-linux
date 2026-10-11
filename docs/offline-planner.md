# Offline routing planner

The planner module is pure Rust logic over supplied data. It creates proposed
routes; it cannot apply them, enumerate real applications or read the OS.
There is no PipeWire/USB adapter, device access, daemon or startup behavior.
See [offline codec](offline-codec.md) for wire parsing and ASCII constraints.

## Synthetic example

```text
cargo run --offline --locked --example offline_plan -j 1
cargo test --offline --locked --test planner -j 1
```

offline_plan decodes the existing handwritten ASCII frame fixture. Its
synthetic inventory has game with stream-a and stream-b. The result proposes
both streams for Link One and reports voice assignment zero as opaque.
No output is an applied route or evidence of hardware interoperability.

## Provider-neutral input and output

Inventory contains a caller-supplied snapshot_id and Applications. Each
Application has an ApplicationId, a stable advertised_name, and Streams.
Each Stream has a StreamId and a supplied Live, Stale or Disappeared state.
The two ID wrappers deliberately describe no Linux or Windows metadata field.
The caller owns name stability, ID lifetimes and availability assessment;
snapshot_id is a version label, not a freshness guarantee.

plan(&inventory, &decoded_records) returns Result<Plan, InventoryError>.
Plan carries that snapshot ID and a Resolution for each distinct record name.
Each Resolution contains the raw name, occurrence count and a typed Outcome.
proposed_routes() yields only ProposedRoute values: application ID, Link
One/Two/Three/Four and sorted local stream IDs. It performs no side effects.

Names match exact bytes, including case. No trimming, case folding, path
normalization, title inference or alias generation occurs. Names must follow
the existing nonempty printable-ASCII, 255-byte codec policy. Unsupported
decoded names produce explicit outcomes; invalid advertised inventory names
invalidate the whole snapshot. The planner does not silently choose a
Unicode/codepage strategy or reproduce the suspected Windows serialization bug.

Valid output is deterministic: resolutions sort by name bytes, colliding
application IDs and stream IDs sort by their wrapper values, and repeated
records are grouped independently of input order.

## Decisions and conservative policy

| Input condition | Typed result |
| --- | --- |
| Unique name, assignment 1..4, all streams Live | Proposed route for all that application's streams |
| Unknown exact name | UnknownName |
| Multiple application IDs advertise the same name | AmbiguousName with sorted candidate IDs |
| Same name has different assignment bytes | ConflictingAssignments with sorted values |
| Assignment 0 or 5..255 | OpaqueAssignment, never an inferred unroute |
| Unsupported record name bytes or length | UnsupportedName with codec error |
| Application has no streams | NoStreams |
| Any stream marked Stale or Disappeared | UnavailableStreams with those stream IDs and states |

Failures block that name's proposal while other independently valid names can
still produce proposals. A mixed live/stale application is entirely blocked,
rather than proposing a partial route. Identical duplicate assignments coalesce
into one decision and preserve their occurrence count. Missing records produce
no decisions; omissions never imply removal or unroute.

Per-name precedence is unsupported name, conflicting assignment values, opaque
assignment, then name lookup and stream availability. Thus 0 plus 1 conflicts;
a single opaque value is reported even if its name is unknown or colliding.

InventoryError aborts the entire plan for empty or duplicate application IDs,
empty or globally duplicate stream IDs, unsupported advertised names or input
limit violations. Duplicate stream ownership is never guessed. Conservative
in-memory caps are 256 applications, 4096 total streams and 169 records (the
maximum count of smallest valid records in a 508-byte terminated list).
These are implementation policy, not an asserted BEACN or PipeWire contract.

A disappeared application omitted from the snapshot cannot be distinguished
from an unknown name. A provider must retain explicit stream tombstones when
it wants Disappeared reported. The core stores no history and polls nothing.

## Future adapter boundary

A future provider would construct snapshots, maintain stable ASCII advertised
identities, detect collisions and stream lifetime changes, and supply decoded
assignment records. Which PipeWire properties support these responsibilities
remains undecided and untested. OS-native IDs must not be assumed persistent
or safe from reuse solely because their strings match.

An eventual executor would need to revalidate snapshot version, identity
ownership and every stream's liveness immediately before applying any proposal.
This module does not authorize or implement that executor. Live OS or hardware
integration needs separate approval; the current task stops at this boundary.

## Verified scope

On 2026-10-11, Windows Rust/Cargo 1.88.0 built this dependency-free crate using
offline, single-job Cargo. All seven focused planner tests and all eight codec
tests passed. The offline_plan example ran successfully. Synthetic tests cover
all four Links, multiple streams, unknown/colliding names, case preservation,
stale/disappeared/no-stream outcomes, duplicate conflicts, opaque assignments,
deterministic ordering, invalid identity ownership, ASCII rejection and caps.

On 2026-10-11, source commit cd33276a29a0b7f09655f4a57e9d7f473f28d327
also built and ran on Ubuntu 24.04.5 LTS with Rust/Cargo 1.88.0. All 15
integration tests passed under offline, locked, single-job Cargo, and the
synthetic offline_plan example succeeded. A minimal official Rustup toolchain
was installed in isolated directories without sudo or profile edits. See
[Linux validation](linux-validation.md) for exact commands, exits and provenance.

Real provider metadata, live routing and hardware interoperability remain
untested. Neither validation captured traffic or accessed BEACN devices.

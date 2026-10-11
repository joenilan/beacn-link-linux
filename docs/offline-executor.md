# Offline routing preparation contract

The pure `executor` module converts existing planner proposals into guarded
link-change descriptions. It reads no OS state and performs no routing.
The CLI remains offline; `apply` and `--apply` explicitly fail with exit 2.
No desktop/hardware backend, daemon, service or installation framework exists.

## Supported explicit mapping

A `Destination { assignment, node }` maps assignment 1, 2, 3 or 4 to one
caller-configured node identity. Only mappings used by a proposal are required,
but every supplied mapping is validated. Duplicate assignments and sharing one
node between assignments are rejected. Zero and 5..255 remain unsupported,
including zero as an unroute action. No destination is discovered by name,
claimed automatically, or inferred to be a BEACN endpoint.

Sources require exactly two output ports, one Left/FL and one Right/FR.
Configured destinations require exactly two input ports with those channels.
The model pairs Left to Left and Right to Right. Missing/extra/duplicate/Other
channels, multi-link fanout from a selected output port and ambiguous destinations
are rejected. Format/rate negotiation and physical device capability are outside
this contract.

## Input and identity boundary

| Input | Role |
| --- | --- |
| Existing `InventoryTracker` and `Plan` | Current accepted generation, application/stream identities and proposals |
| Expected `GraphSnapshot` | Anchors for the objects on which planning was based |
| Observed `GraphSnapshot` | Fresh, complete caller observation for validation |
| Explicit destinations | Assignment byte to node ID plus object serial |
| `Ownership` | Caller token plus trusted link receipt ledger, separate from reported graph tags |

Graph snapshots carry a caller-verified server/session `core_id`, generation,
nodes, ports, stream/application bindings and links. `ObjectRef` contains both
numeric ID and object serial. Labels are nonempty printable ASCII up to 128
bytes. Represented node/port/link IDs and serials must be nonzero and globally
unique within the supplied graph. Node/port/link references and directions
must resolve exactly. These are conservative input policies.

Bounds are 8192 nodes, 16384 ports, 16384 links, 4096 stream bindings and at most
four destination mappings, plus the existing planner's bounds. These limits
are implementation policy, not asserted device limits.

A generation label does not establish freshness. A core/session key must change
when the identity domain restarts; a server name alone cannot prove continuity.
Callers must supply complete observations and refresh the existing inventory's
liveness from the provider. The module cannot detect an omitted foreign link,
a fabricated ownership ledger or a falsely labeled observation.

## Preparation checks and ordering

`prepare(tracker, plan, expected, observed, destinations, ownership)` is
all-or-nothing. It reuses the lifecycle generation guard, requires matching
graph generations and core identity, and validates both graph structures.
Any non-Proposed resolution blocks the batch, including unknown names,
ambiguity, stale/disappeared streams, conflicts and opaque assignments.

To reject altered or incomplete proposals, preparation reconstructs the
supported records from the plan and replans against the current inventory.
The complete result, including application ownership, stream set and duplicate
counts, must equal the supplied plan. Generation checking alone is insufficient.

For each selected stream, its application binding, node ID/serial and stereo
ports must match the anchored observation. Missing objects, numeric-ID reuse
with a different serial, changed port metadata and changed outgoing link sets
are rejected. Previously linked input endpoints must also remain valid for
rollback information.

An already-correct existing link is preserved, including a foreign link.
A link requiring removal must have both the matching graph owner token and
its exact ID/serial in the trusted receipt ledger. A tag alone is insufficient.
No removal is proposed for unowned links.

Streams sort by existing StreamId and channels by Left then Right. All required
disconnect descriptions precede all connect descriptions. Equivalent valid
input orderings produce the same operations. A fully correct route produces
zero operations. No omitted planner record implies an unroute.

Each `GuardedStep` carries its stream binding, operation and undo description.
Disconnect operations retain the exact old link, ports and ownership; connect
operations retain exact ports and the caller's token. `validate_step` checks
the current generation/core, binding/liveness, referenced node/port identities
and port metadata, and the selected output port's link state before a future
backend acts. A connect cannot validate while an old or unexpected link remains
on its output port. Disconnects recheck the exact link and receipt ownership.

These functions validate supplied data; they do not authenticate it or authorize
audio changes. Callers must use unmodified output from preparation and make
fresh provider checks around every actual operation. Neither a whole batch nor
a stereo pair is atomic, and validation does not remove races after a snapshot.

## Rollback data, not execution

`PreparedChange::rollback_for_prefix(applied)` returns undo descriptions for
only the acknowledged successful prefix, in reverse order. An excessive prefix
count is rejected; zero produces no undo steps.

- Undoing a disconnect describes restoring the old endpoints and ownership.
  It preserves the old link identity as evidence, not as an identity that can
  be recreated. A restored link will have a new ID/serial.
- Undoing a connect names its forward-step index and endpoints/token. A future
  backend must capture the actual successful-create receipt, then revalidate
  that new link's ID, serial, endpoints and ownership before removal.
  Endpoints alone do not authorize deleting a link.

The caller must distinguish acknowledged success from failure and unknown
outcome. There is no receipt capture, mutation journal, rollback validator,
automatic retry or actual rollback backend in this slice. Rollback descriptions
are not a guarantee that vanished endpoints or interrupted operations can be
recovered.

## Synthetic dry-run example

```text
cargo run --offline --locked --example prepare_route -j 1
cargo test --offline --locked --test executor -j 1
link-protocol apply
```

The example uses handwritten nodes/ports/links and an explicit assignment 2
mapping. It prints two guarded disconnects, two connects and undo descriptions;
its first line is `DRY RUN: 4 guarded operations; actual application disabled`.
The last command is expected to fail with exit 2. No command applies audio routes.

## Verification and provenance

Prepared over source commit `033ea05248df2c57ac9d696b35544c00420b86de`.
On 2026-10-11, Windows formatting and formatting-check, offline locked single-job
build, all **29 Rust integration tests**, seven Python tests and the dry-run
example passed. Eight new Rust checks cover deterministic movement and rollback
prefixes, all four mapping values, preserved correct foreign links, stale
generation/core changes, disappeared/reused objects, tag-versus-receipt ownership,
ambiguous ports/mappings/fanout, blocked/fabricated plans, per-step guards and
explicit CLI application rejection.

The same source bytes were verified and built on Ubuntu 26.04.1 LTS,
kernel 7.0.0-38-generic, with existing Rust/Cargo 1.88.0, GCC 15.2.0 and Python
3.14.4. Offline locked single-job build, the same 29 Rust tests, seven Python
tests and the dry-run example passed. Windows/Linux example stdout matched
exactly. No package or toolchain was installed.

Linux formatting-check was unavailable: the existing minimal toolchain returned
`error: 'cargo-fmt' is not installed for the toolchain '1.88.0-x86_64-unknown-linux-gnu'.`
No component was installed; Windows formatting-check passed. This limitation
is separate from the successful Linux compilation and tests.

| Artifact | SHA-256 |
| --- | --- |
| Transferred 53226-byte archive | `c9de438de72d212daa14d405ae3267f7847c912f562606984352b00a0d3c9982` |
| Extracted source before final documentation | `00725b4046cf1d53a598bcf6b92de718bdd3fb2ffc263f0409f08e68a1bfd99c` |
| Tested code/fixtures and Cargo manifests | `ec4af5b022de16497c163b4f08c38063e7c7917b97562917ee9819eed4d5ff3a` |

The archive contained the committed base plus five intended code/example/test
edits. Sorted relative-path/NUL/file-hash/LF tree hashing follows the existing
[validation convention](offline-lifecycle.md). Committed code blobs are checked
against the tested code hash; final documentation changes are outside that hash.
Full manifests and command logs remain private.

Before/after the successful small Linux build, free space was
24,209,858,560 / 24,027,222,016 bytes. Target files totaled 227,058,418 bytes.
The owner was informed that available space remained below 25 GB for heavy
builds. No cleanup, dependency download or heavy build occurred.

## Remaining integration gap

The read-only adapter still projects application inventories; it does not yet
populate this node/port/link graph contract or consume guarded steps. A future
native integration needs a verified session key, complete fresh graph/binding
construction, explicit destination configuration, owned create receipts,
acknowledgment handling and guarded forward/rollback application. Its session
policy must preserve desktop fallback and foreign routes.

This slice performs only synthetic validation. The earlier
[private-core A-to-B test](controlled-virtual-routing.md) remains separate
evidence from an external harness, not a validation of this module against a
live server. No audio mutation or capture, BEACN/USB access, remote discovery,
Windows assignment exchange or hardware interoperability was attempted here.

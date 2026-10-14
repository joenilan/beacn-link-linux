# Beacn LinX Rust application service

Implemented 2026-10-14 after the owner began returning Studio for streaming.
This batch uses no USB, physical audio, capture or shared-desktop route changes.

`beacn-linx` is now a real Rust command-line application with native read-only
PipeWire discovery and a foreground service loop. It discovers ordinary playback
applications, accepts explicit selections/ASCII wire aliases, tracks multiple
applications and revokes a selection when its exact application disappears,
becomes stale or the core domain changes. Reopening the same display name does
not inherit a prior identity/selection. Selections remain in memory for this
process; no settings or autostart files are written.

This is the read-only application/service foundation. It does not yet connect
these selections to USB publication, apply desktop routes, reconnect automatically,
provide a GUI or install a background daemon. The previous
[real-application/four-pair hardware checkpoint](native-application-routing.md)
remains separately accepted. Multiple selected applications are not evidence of
multiple applications published/routed through Studio.

## Native catalog and capability separation

`src/native_catalog.rs` has its own non-Send session type and C entry point.
It accepts only the owned mode-0700 `/run/user/UID/pipewire-0` socket, or an explicit
existing owned `/tmp/beacn-pw-*` / `beacn-test-*` core for qualification. It checks
canonical paths, socket type/UID and unchanged socket inode around connection.
The C adapter authenticates the Unix peer UID and positive PID, pins core name/
cookie on its first info event, and rejects changes. The cookie identifies an
instance; it is not cryptographic authentication. Environment overrides are refused.

The catalog binds only Node and Client query proxies. Devices, ports, links and
module globals are not bound by this mode. It neither creates audio streams nor
queries PCM/audio payloads. A private exact minimal client config loads only the
native protocol module; it is removed after the native session closes. Cleanup
checks the owned directory identity and exact file contents before removal.

The C session carries an immutable catalog-only flag. `bcl_perform` refuses it
before inspecting endpoints or submitting a mutation, even through raw FFI;
the mutable graph-snapshot API also refuses it. Rust exposes only observation,
not the execution Backend trait or receipts. Existing private-executor module/
device/token/serial/receipt restrictions stay enforced in the other session mode.
The test-only mutation probe actually calls the C mutation entry with null
endpoints and requires refusal before dereference; it does not create a link.
No default routing policy or USB gate was expanded.

Catalog rows use bounded hex-encoded UTF-8 display metadata, preserving Unicode,
tabs/newlines and application/binary names separately from ASCII wire aliases.
Client serial/PID and stream serial identities reuse the existing inventory owner
scheme. Names do not grant routing/removal authority. Invalid, ambiguous, missing,
oversized or reused metadata fails closed. Bounds are 8 MiB, 256 apps, 4,096
streams, 512 bytes per label and the existing 40,960 observation-history cap.
Two ordered native barriers collect metadata, each bounded to five seconds.

`src/catalog.rs` validates even directly constructed catalogs, builds selected
inventories through the existing planner/codec bounds, and delegates transitions
and stale-plan checks to InventoryTracker. Selection edits increment a separate
revision, so a plan made before a UI selection edit cannot remain current merely
because the underlying PipeWire generation was unchanged. Duplicate wire aliases,
dead identities and a list exceeding one protocol frame are refused atomically.
Opaque assignment zero is used only to check encoding size; nothing is published.

## Commands

```sh
cargo build --locked --offline --features native-pipewire --bin beacn-linx
target/debug/beacn-linx inventory
target/debug/beacn-linx service 60
```

The service polls every 500 ms and accepts newline-terminated stdin commands:

```text
select APP-ID=Media Player
unselect APP-ID
status
quit
```

Use an identity from the current APP output. Every command gets a fresh native
catalog before it can change selection. Display labels are escaped in terminal
output. Input is capped at 1,024 bytes and an eight-message channel. Service
duration is explicitly 1–600 seconds; quit, EOF or deadline exits normally.
A pending native observation can add its barrier deadlines to wall time.
Connection/metadata failure stops the process; there is no retry/fallback.
SIGTERM/forced process termination is not yet a graceful-shutdown acceptance case.
Linux/native-pipewire is required for discovery; the default cross-platform binary
provides help and refuses discovery without that feature. No Cargo dependency added.

## Verification

Eight focused Rust tests pass on Windows and Linux: bounded UTF-8 metadata,
malformed/reused identities, direct-construction validation, multi-app selection,
duplicate aliases/frame limits, atomic refusal, stale/core/closure revocation,
reopen identity and selection-revision plan invalidation. The runner's 28 focused
Python checks pass, including refusal to enter hardware mode for service qualification.

Native source `beacn-native-source-20261014T222104Z` built the exact programs offline.
An actual read-only desktop query succeeded with zero playback applications and
an exact before/after settings/state comparison. Populated discovery was then
tested with real mpv processes on a disposable private core, not asserted from
the empty desktop result.

In one retained Rust service/core, the supervisor selected the first mpv process,
started and selected a second, observed its exit and selection revocation, reopened
mpv under a new identity, required fresh selection, unselected the first, and quit
with exit 0. Separate native invocations refused oversized input and exited
normally on EOF/deadline. All ran with the test-only C mutation-refusal check.
Private graph, complete laptop desktop/state, owned config cleanup and all retained
process-exit checks passed. The enclosing existing native executor then routed the
first mpv through all four private destinations and restored its baseline.

Service journal SHA256:
`7c88fd7476200ed150983ca74a5ac34a2a21ac88a87018f87cd3548e1a4aa224`.
The earlier successful check retained an FFI declaration warning; a dedicated
test-only C wrapper removed that warning before the current qualification.
The pre-existing unused `verified_routes` warning remains in builds without USB.

The existing actual native fault/recovery qualification also passes: rejected
creation, withheld acknowledgment, explicit reconciliation, interrupted/rejected
rollback continuation and repeated already-correct routing. Its private graph and
desktop/state cleanup matched exactly. Journal SHA256:
`dbf79fff6a707189c1545d8d13949cc48bd46a88b8ddc3bb260c0c19cc4b2c76`.
102 evidence/provenance files were copied and hashed outside Git before final build
provenance. No proprietary data, host address, key path or live machine identities
are included here.

## Next

Connect explicit selection/lifecycle changes to guarded multi-application USB
publication and verified routes without broadening default apply authority. Empty
lists, departed applications and reconnect must follow observed protocol semantics;
do not guess that assignment zero means unroute. Then add the Rust UI around the
same controls, explicit device/session state and meaningful error recovery.
Hardware remains reserved for the owner; another live test needs fresh
availability and baselines. This batch does not establish Windows Link visibility
or a new physical return/settings comparison.

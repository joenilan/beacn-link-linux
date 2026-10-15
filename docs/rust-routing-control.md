# Rust routing controls and lifecycle release

Updated 2026-10-14. Native private-core qualification only; no USB or physical
audio. The foreground `beacn-linx service` remains read-only while the owner streams.

## Controls

`routing_control::RoutingControl` exposes explicit start, stop, status and assignment
handling for a Rust frontend. Construction opens nothing. Start calls an explicit
backend factory only after checking a nonempty current selection. It uses the
existing fixed-roster publication epoch, executor and acknowledgment journals.
Initial sources must be unlinked; all configured destinations are validated before
the controller enters Running. There is no automatic rebind or reconnect.

| Phase | Meaning |
| --- | --- |
| Idle | No owned routing session has started. |
| Running | One retained session; assignments require fresh guarded observations. |
| Stopped | Owned session released and remaining graph verified. Explicit start is allowed. |
| NeedsReview | Owned handles released, but cleanup could not be verified. Restart is blocked. |

Status includes a stop reason, retained cleanup result, the last caller-observation
revision, and a separate revision for the last verified assignment data. Ordinary
catalog polling does not refresh that proof. Cached status never authorizes a
native operation or USB publication. Stopping clears assignment data. Repeated
stop returns the retained result without another release. Normal Rust drop also
attempts shutdown; callers should stop explicitly to inspect the result.

The caller updates the existing ApplicationService and checks the controller after
every native catalog observation or selection edit. Selected departure, stale
streams, changed streams/alias/selection or core replacement stops the entire
owned fixed-roster session before more work. **Remaining apps also stop in this
bounded policy.** Continuous routing of unaffected apps is future work.

Empty/no-payload/rejected-marker replies produce no route operation. Opaque zero,
unknown/conflicting names or assignments block the batch without changing routes
or stopping the session. Thus a wire zero has not become an indirect unroute.
Missing app records retain the last verified assignment for that selected app;
every subsequent publication proof still verifies the complete actual roster.
Known operation failure stops owned routing without forwarding/retrying more work.

## Native cleanup

`native_routing_lease::NativeRoutingLease` wraps the original private-only executor
and a second pinned graph observation connection with no source bindings. Both
retain the original exact runtime/config/socket/core-cookie/server-PID gates.
No C or native endpoint restrictions changed.

Ordinary bound-inventory rollback cannot proceed after a source disappears. This
lease instead observes the current graph, releases only its retained native
session/proxies, then observes again independently of the departed inventory.
Release happens even if the first observation fails. No IDs or tags are used to
select deletion targets. The existing receipt owner now has a pure `verify_release`
comparison: every acknowledged surviving link must disappear, missing old links
are recorded as already absent, and every other graph object must remain exact.
Reused numeric IDs with different serials and same-tag foreign links are preserved.

Unknown native effects, missing observations or graph changes that prevent exact
verification produce NeedsReview. There is no blind cleanup retry, forward retry,
or reconnect. The simulated unknown-effect branch is tested; a native server-crash
or native unknown-acknowledgment fault through this new controller is not accepted
by the current hardware/private-core evidence.

## Qualification

`examples/native_session_control.rs` exposes bounded private-test stdin controls
only, separate from the production read-only CLI. The existing no-USB routing
probe accepts `--session-control-check BIN` only with the Rust catalog/service
check. The parent enforces exchange deadlines and the child caps input/commands.
These test commands are not a desktop apply mode or a USB transport.

Native source `beacn-native-source-20261014T230633Z` built offline and passed:

- Start, two actual mpv applications on pairs 1/4, and opaque-zero no-mutation.
- Explicit verified stop of four links, repeated stop, and explicit fresh restart.
- Simultaneous routes on pairs 2/3.
- A separate retained creator adds an extra link carrying the same owner tag.
- The second mpv exits. Catalog revocation stops the controller; two disappeared
  links are already absent and the two remaining owned links are released.
- The separate creator's exact link ID/serial remains. Its own creator later
  releases it; the controller does not claim deletion authority from its tag.
- Status stays Stopped without automatic restart; normal quit exits zero.

The enclosing multi-app movement/partial-recovery and read-only service
departure/reopen tests also pass. As previously documented, intermediate private
adapter negotiation caches remain different. After full owned-node teardown the
original private-core graph is exact, all retained children exit, new catalog
configs are removed, and complete laptop settings/routes/defaults/volumes/mutes
and persistent-state hashes match. No Studio was attached to the laptop.

55 focused Rust checks pass on Windows and 54 on Linux (CLI feature difference).
All 30 Python runner checks pass on Linux; Windows skips six Linux-only checks.
Seven controller tests cover lifecycle edits, idempotent stop, startup guards,
stale status revisions, known/unknown partial failures and unverified cleanup.
The existing eight epoch tests share the same fixture and still pass.

Private evidence `beacn-session-control-20261014T230639Z`:

- Control journal SHA256:
  `a97d576faf3c6fd470999c44a45e654013a6f8fe28a71927b1e1082ada4c947c`.
- Enclosing service journal SHA256:
  `84ddb1365aa37e6237ef9af4633ecc07ec20e46f35cda4c0a0db8973c3b2bf57`.
- Complete qualification SHA256:
  `51e1afc426b966664f9bb651e4a7ee9ff9f0718913228f22c25d69c5385ee109`.

153 evidence/provenance files were copied and hashed outside Git before final
provenance. Final compiled code hashes match the native-qualified source.

An earlier qualification helper failed to create its foreign probe link because
it used the native executor's minimal client config for pw-cli. Its cleanup passed;
artifacts remain private. Giving only that CLI the existing richer private-test
config fixed the helper without changing the native executor/catalog gates.

## Next

The [Rust egui interface](rust-interface.md) now provides read-only discovery and
selection. Next connect session state/controls to the interface. Foreground service integration, unaffected-app
continuity, multi-app USB/hardware interoperability, shared-desktop apply,
persistence/autostart, graceful signal handling and durable reconnect remain open.
Keep hardware testing closed while the owner is live. Actions remain disabled;
use local [skip ci] checkpoints and preserve private evidence.

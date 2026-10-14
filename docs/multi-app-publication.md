# Guarded multi-application integration

Updated 2026-10-14. This is native private-core qualification without USB or
physical audio. Ordinary desktop apply and the native single-app USB gate remain
unchanged. The owner is streaming; hardware testing is closed.

## Implemented path

`publication_epoch::PublicationEpoch` connects the existing Rust application
selection model to decoded session replies, the planner, guarded preparation,
and verified publication data. It performs no I/O itself. The existing native
executor and receipt journals remain the only operation/recovery owners.

An epoch starts with a nonempty explicit selection and a matching observation
whose selected sources are all exactly unlinked stereo streams. It pins the
catalog domain, graph domain, selected process/stream identities, aliases and
selection revision. The caller must establish both observations came from the
same retained core; these differently formatted domain strings are not silently
converted or treated as equivalent.

Every reply/prepare/proof operation checks the current selection. Departure,
stale streams, changed stream membership, core replacement or a selection edit
permanently halts the epoch. A distinct selection revision records intervening
edits even if an alias/selection is changed back before the next operation.
Routine catalog polling advances the observation revision without changing that
selection revision. It still invalidates pending plans. Pending assignments also
carry a private epoch identity and cannot cross even otherwise identical epochs.

Received names use the existing explicit ExactBytes/WindowsCString policy before
planning. NoPayload and RejectedMarker produce no plan. Missing records produce
no operation for the missing app; zero/unsupported assignments and conflicting
records remain explicit blocked planner outcomes. No departure/zero/empty-list
unroute semantics have been invented. Short duplicate wire aliases are now
refused atomically by the application selection model.

`VerifiedRoutePublication::verify_selected` requires an exact, complete assignment
map for every selected app. Nonzero assignments must already be fully applied
through the existing guarded executor contract, with concrete trusted receipts
for every stream's two stereo links. Explicit zero requires an observed unlinked
source. Missing receipts, partial application, wrong destinations, ambiguous wire
names and incomplete stereo fail the whole proof. Output is canonical and subject
to the existing one-frame byte budget. Desired state is never cached as proof.
The original `verify` and `usb_handshake::Publication` still require one app.

## Native verification

`examples/native_multi_app.rs` is a separate, explicitly authorized disposable-core
qualification program, not a desktop apply mode. It uses native Rust catalog
discovery, explicit aliases and NativePipeWire's original runtime/socket/core/PID,
endpoint/token and receipt restrictions. Catalog cookie/server PID/core name must
match the pinned executor connection. No C/backend restrictions were relaxed.

The existing no-USB routing harness can opt into it with `--multi-app-check BIN`,
only together with `--catalog-service-check BIN` in ordinary-mpv `routing-probe`.
It launches two actual mpv processes with distinct native identities on one
retained private core. The native example:

- Creates four acknowledged links for simultaneous assignments 1 and 4.
- Verifies both applications before producing canonical publication records.
- Moves both to assignments 2 and 3 through eight guarded operations.
- Recovers both original routes using newly acknowledged restoration identities.
- Opens a fresh executor session only after the previous owned links are gone.
- Stops a second batch before its third native request, refuses desired-state
  publication, and rolls its two acknowledged links back to an exact graph baseline.

The enclosing Rust service also repeats departure/reopen/reselection and bounded
input/EOF/deadline checks. Its failure cleanup now requests normal service exit
before bounded termination, allowing the owned read-only config to be removed.

Native source `beacn-native-source-20261014T224851Z` builds offline. Focused checks:
43 Rust tests pass on Windows, 42 on Linux (one CLI feature difference), and all
29 Python runner tests pass on Linux. Windows skips six Linux-only runner checks.
The eight epoch tests include a two-app/three-stream simulated execution, stale
and cross-epoch plans, complete/mixed proofs, lifecycle edits and partial recovery.

Private evidence `beacn-multi-app-20261014T224856Z`:

- Native multi-app result SHA256:
  `966b77ff771bf577cb3ab768c985015865b48acbcd4345713c852bbbd8138994`.
- Catalog/service journal SHA256:
  `c63543cd2b11ee4acb3c36bbf9baf09ac362dfc72dd4e7b268bd648928412540`.
- Enclosing complete qualification SHA256:
  `a200bf430eb0df98a148436cdc61032b6de90dd7d5d3668847bc22a7f3875bd0`.

160 evidence/provenance files were copied and hashed outside Git, including the
failed attempts. Final compiled code hashes match the native-qualified source.

The intermediate full private graph comparison remains **false** in the journal:
routing initializes Format/channel vectors on the retained disposable source and
destination adapters. The before/after dumps are retained without normalization.
All test links are released at that checkpoint. Complete cleanup is evaluated by
the enclosing owner after every owned app/destination is stopped: the original
whole private-core baseline matches exactly, all retained processes exit, and
the laptop desktop settings/defaults/routes/volumes/mutes and persistent hashes
match exactly. New owned catalog configs are removed. Earlier failed qualification
attempts and their private artifacts are retained; they are not acceptance runs.

## Remaining work

This proves simultaneous native private-core application routing and multi-app
publication **data**, not multi-app USB interoperability or physical audio. The
foreground application service remains read-only. Next integrate explicit Rust
app controls/status with these owners and qualify lifecycle-triggered stopping
and receipt cleanup before enabling a native transport mode. UI, shared-desktop
apply, durable selections/autostart and automatic reconnect remain separate work.
No hardware test or streaming-PC observation is implied by this checkpoint.

# Native desktop stream routing

Updated 2026-10-15 UTC. The owner supplied a playing Firefox video and confirmed
**Heard the Firefox video** through the streaming PC after the native desktop
check. This is the first qualification of an already-running ordinary desktop
stream, separate from the earlier applications launched on a private core.

## Implemented path

The opt-in `native-desktop-target` feature opens the current user's observed
`pipewire-0` core. A selected catalog application must have one current stereo
stream, with exact client serial/PID, source node/port generations and one complete
original stereo destination. The application name is a selection label; no Firefox
executable path or application-specific audio backend exists in the Rust owner.

LinX creates a non-lingering stereo sink, then requests the selected stream's
`target.object` through the existing default metadata owner. WirePlumber creates
and owns those stream links. LinX never adopts their IDs/tags as deletion receipts
and cannot invoke the private executor's mutation API on this connection.
Metadata writes are bounded, submitted once, and must receive their exact echo;
a core sync alone does not prove that a remote metadata owner processed a write.
The prior property and original destination are retained for explicit restoration.

The optional physical qualification uses the selected Studio's existing desktop
ALSA identity only to verify the device is unique and unused. It creates its own
retained eight-channel FLOAT_LE/96-kHz adapter with the earlier native adapter
owner, linking the stereo sink monitors only to AUX0/1. Desktop input can remain
at 48 kHz; PipeWire handles conversion. No default, global clock configuration,
volume, mute, profile, firmware, driver or USB vendor transfer is written.

The owned physical adapter is excluded from WirePlumber item creation through
`wireplumber.is-virtual`, matching the installed 0.5.8 create-item.lua exclusion.
Without that explicit ownership boundary, policy reconfigured its ports and
suspended it. The ordinary desktop ALSA node stays untouched. New temporary sinks
and adapters disable saved-property/target restoration and have zero session
priority. The existing private factory and proof requirements remain separate.

A socket-activated desktop listener may identify the user service manager through
SO_PEERCRED. LinX authenticates that socket peer and separately pins the actual
Core `application.process.id`; it never infers the expected PCM owner from a live
status file. The existing native proof requires the pinned process generation,
held matching character-device FD, RUNNING, eight FLOAT_LE channels at 96 kHz,
closed capture and monotonic hardware progress. Startup can wait at most two
seconds for the retained adapter's observed idle-to-running transition; closed
PCM is never accepted as active playback. Normal PCM checks still stop on reversal,
a two-second stall, changed identity/format or the existing observation limit.

On stop, only retained bridge/adapter proxies are destroyed. LinX restores the
original stereo target, restores the exact prior metadata value, verifies routing,
releases its retained sink and compares the foreign graph. Cleanup results cache;
failed owners cannot restart. Foreign metadata changes latch a conflict and cannot
be overwritten as though they were LinX's earlier target request.

## Live evidence and limits

- Generic silent move/restore: `linx-desktop-route-20261015T131223Z`, exact source
  `beacn-native-source-20261015T131151Z`, 98 checks. Result SHA256
  `ec614c6c235064846fa3148d4b1fa555f92c055e5301b3b77f94671a33afac17`.
- Owner-heard Firefox: `linx-desktop-studio-20261015T132948Z`, exact source
  `beacn-native-source-20261015T132933Z`, 98 PCM checks and hardware pointer progress
  476,138 frames. Result SHA256
  `73b71ab5ed156bb01cba5a88b995979e347577854f7013f487bd03c95b3f62a9`.
  No application publication/dragging or vendor USB initialization was needed for
  this fixed first-pair transport check. That does not qualify remote assignment.

Each successful run restored its baseline's foreign graph/settings, persistent
state bytes/hashes and connected USB binding, and closed playback and capture.
The selected stream's two links were recreated by WirePlumber, so the raw desktop
object snapshot is **not** byte-for-byte equal; native source/destination identities
and the remaining graph were verified separately. Owner hearing is not an audio
capture, level measurement, crosstalk result or a long-run stability result.

Preserve the failed runs too. The first desktop client lacked the metadata marshal
module and stopped without mutation. Two early metadata-echo checks returned too
soon; one left an owned target override and a temporary sink's saved-property
cache entry. The exact override was removed with generation/value checks. A later
whole-file restoration attempt refused because foreign settings had changed; it
wrote nothing. That original owned cache entry remains recorded, and later exact
per-run baselines must not be described as restoration of the entire turn's first
snapshot. Subsequent owned sinks disable that persistence. Do not restart the
whole audio service or overwrite changed Firefox settings to conceal the difference.

The existing desktop 48-kHz ALSA node opened RUNNING but stalled at pointer zero.
The failed checks stopped after the two-second limit and released all owned audio,
restoring their per-run state. Another early stop correctly rejected the socket
listener PID as the PCM owner. The retained 96-kHz adapter initially exposed the
WirePlumber configuration race described above. These failures were not accepted
as playback or retried in the same owner.

## Focused checks and real failure recovery

Later source `beacn-native-source-20261015T133151Z` passes thirteen native library,
nine catalog and three graph checks. These include exact daemon generation,
metadata conflicts, incomplete/fanned-out/reused routes, native PCM identity and
clock rejection, and default-denied authority. No physical acceptance is inferred
from those checks.

`linx-desktop-private-regression-20261015T133202Z` passes all four existing native
virtual playback mappings, partial creation faults, source departure/revocation,
two-app selection and bounded service lifecycle. Its outer runner verifies private
core teardown, owned child exit and the exact laptop desktop/state/USB baseline.
The inner catalog journal explicitly defers final private-graph teardown to that
outer runner. Catalog journal SHA256:
`0ecc98d86a032edc72a3d1b77f13c9e9d4f1b6f1fc4c3dfa14d94aeb2afc4feb`.

`linx-desktop-failure-20261015T133314Z` injects failure **after** a real echoed
`target.object` write and before move acknowledgment. Start/check/restart refuse;
stop restores the original destination and metadata, releases the owned sink,
and repeated stop returns the cached verified cleanup. Foreign graph/settings,
saved state and USB binding match that attempt's baseline except selected-link
recreation. No Studio audio or vendor USB I/O is requested. Result SHA256:
`52050995ee428b5239d483a55cadec02ff70592406e7164b8e6fdd10f7eca0c2`.

## Remaining integration

This is an explicit bounded example, **not the normal GUI backend**. The default
feature set, application service and ordinary GUI routing remain gated. The
existing VerifiedRoutePublication requires private native link receipts and zero
means an unlinked source; do not weaken it to pretend WirePlumber's links are owned.
Desktop publication needs a distinct sealed proof of the retained metadata lease,
fresh selected catalog/graph and exact owned physical bridge. Unassigned desktop
apps retain their original output. Connect that proof to the existing codec,
planner/session lifecycle and GUI owner, then qualify selection, pause/restoration,
remote assignment changes and departure before enabling the normal desktop flow.

Multiple streams/apps, desktop pairs 2-4, hotplug, assignment persistence, installed
autostart, long runs, gains/mutes/meters and remote mixer controls remain open.
The private helper uses an explicit one-stream application selection; it is not a
promise that every application already works. Raw journals, settings snapshots,
descriptor caches, live identities and executable/source hashes remain outside Git.

Final build/source `beacn-native-source-20261015T133608Z` retains strict 96-kHz
PCM validation and passes the same 25 focused checks. The final read-only audit
matches the failure-cleanup desktop/state snapshot exactly; both PCMs are closed
and no owned native test process remains. Private evidence was copied and hashed
outside Git. No additional physical acceptance is claimed for this source.

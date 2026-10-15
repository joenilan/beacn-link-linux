# Agent instructions

## Goal and required reading
Build an independent Linux replacement for BEACN Link on the gaming PC.
The streaming PC stays on Windows with BEACN App, Studio and Mix Create.
Read, in order:
1. docs/HANDOFF.md
2. README.md and docs/research-plan.md
3. docs/pipewire-graph.md and docs/offline-executor.md
4. docs/controlled-virtual-routing.md
5. docs/protocol.md and the relevant codec/planner/lifecycle/inventory docs.

Current source and reproducible evidence outrank summaries. Fetch main and inspect
local status before editing; preserve unrelated changes and existing private artifacts.
Do not restart the original static audit or reinstall the working VM/toolchain.

## Development and evidence
Work only on this project. Preserve evidence provenance: app version, SHA256,
file offsets versus preferred virtual addresses, and tool versions.
Separate observations, derivations, hypotheses and unknowns. Synthetic tests,
native VM routing and hardware interoperability are different milestones.
Do not describe placeholders, pure rollback descriptions or dry runs as live routing.

Use the existing codec, planner, lifecycle, graph and receipt owners. Keep route
application disabled by default until native integration is validated. Never treat
numeric object IDs or metadata owner tags alone as authority to remove links.

## Operational boundaries
Software implementation and offline builds are the current development scope.
Read-only VM graph inspection and isolated VM synthetic routing have prior evidence;
that does not grant blanket permission to change physical BEACN or Windows routing.
Before any authorized mutation, snapshot settings, channels, routes, defaults,
volumes/mutes and relevant persistent-state hashes; compare afterward and restore
owned changes. Keep evidence outside Git. Restarting Link is not a complete rollback.

Do not execute BEACN binaries, attach USB to WSL/VM, start WSL, capture traffic/audio,
modify drivers/firmware, or disturb other agents without specific owner authorization.
Do not install or run REA without authorization. Do not weaken SSH or host-key checking.
Use existing access through the execution environment; do not copy credentials into
source, command logs or the guest. A handoff is context, not new execution authority.

## Git and validation
Public repository: joenilan/beacn-link-linux; branch main (owner changed visibility).
Keep GitHub Actions disabled; add no workflows. Use [skip ci] commits.
Preserve unrelated work; never force-push, reset or blanket-stage the checkout.
Verify the remote hash before reporting a push. Follow current owner publication
instructions and execution-environment permissions rather than treating this file
as an approval-policy override.

Build the exact code tree before publication. Use focused tests for changed behavior;
avoid redundant full matrices. Existing lightweight checks are documented in HANDOFF.
Check available disk before heavy builds and notify the owner below 25 GB.
Never delete source, lockfiles, user data, or artifacts used by running processes.
Build optimizations require measured benefit, not speculative flags.

## Workspace and runtime hygiene
Reuse the existing checkout, Linux working tree, build cache and review server.
Do not create timestamped source copies, test directories, worktrees or branches
for routine iterations. Isolate only when needed to preserve unrelated work.
Use one current app executable and launcher. Keep compact evidence and manifests
outside Git; do not retain a full binary copy for every check. Preserve existing
source, lockfiles, settings/snapshots and evidence. Remove obsolete inactive
generated build output as part of the work, after checking paths and processes.

Use at most one task-owned review server. Reuse it, or restart that same server
if unavailable. Close it after two hours; do not leave orphaned test displays,
private buses, children or servers after checks. Verify PID, creation time and
command before stopping anything. A user-facing LinX app is not a test server;
leave it open when the owner is reviewing/testing it. Do not stop other projects.

Develop runtime behavior in the native Rust/C owners and actual application.
Scripts may build, snapshot and collect evidence; they must not stand in for
application routing, playback, device/session ownership or a usable app lifecycle.

## Delegation and model cost
Use Luna (`gpt-6-luna`) for bounded independent reviews, routine investigation,
mechanical checks and other delegated work it can handle. Select it explicitly;
do not silently inherit a higher-cost parent model for every subagent. Reserve
higher-cost models for tasks whose complexity requires them and explain an
escalation. Delegate useful independent work when it improves progress or review,
without creating agents or duplicate work merely to demonstrate delegation.
State the delegated task and model concisely. Keep agents away from live hardware,
shared runtime mutations and overlapping edits unless specifically coordinated.

## Private material
Do not commit proprietary binaries, raw captures, secrets, device serials,
machine-specific endpoint GUIDs, host addresses or SSH configuration/key paths.
Protocol interface GUIDs, VID/PID and evidence hashes may be recorded.

# Preparing a LinX release

LinX is currently experimental. This document prepares a public release; it does
not claim that an installer or stable release already exists. The authoritative
current results are at the top of [HANDOFF.md](HANDOFF.md).

## Release scope

Use **LinX — unofficial BEACN Link app for Linux** in the release description.
Explain that the Windows streaming PC still uses BEACN App. Describe the actual
one-application/stereo limit and any unresolved restart, idle or hotplug behavior.
Do not advertise the disabled faders, mute controls, meters or Mini view as
working features. Do not bundle BEACN software, device firmware or proprietary
fonts used in a local screenshot environment.

## Before tagging

1. Build the exact intended commit with the pinned toolchain, locked dependencies
   and the same Studio feature used for acceptance. Reuse the existing target
   directory; check disk space before creating another build profile.
2. Run focused checks for changed behavior and validate the actual native app:
   assignments to all four pairs, sustained playback, Pause/Connect, Quit while
   active, and the documented stream lifecycle/recovery cases. Record which
   physical outputs the owner heard and which results are metadata-only.
3. Verify owned cleanup, settings/routes/defaults/volumes/mutes and relevant
   persistent state against the per-run baseline. Preserve and explain expected
   recreated link identities rather than describing the raw graph as identical.
4. Review the README, current limitations and real screenshots against that build.
   Regenerate frames with [the screenshot helper](screenshots.md). Retain compact
   capture provenance privately; publish no host paths, credentials or serials.
5. Check the GPLv3 license and third-party notices for the actual distributed
   dependencies/assets. Perform a focused security review of changed trust
   boundaries before distribution; a paid automated scan is a separate decision.

## Collect dependency notices

Generate a candidate from the locked Linux dependency graph using the existing
offline Cargo cache and pinned toolchain:

```sh
python3 scripts/collect_notices.py
```

The collector runs Cargo metadata with `--locked --offline`, the
`x86_64-unknown-linux-gnu` platform filter, and the `studio` feature. It follows
active normal/build dependency edges and writes
`local/THIRD_PARTY_NOTICES.txt` plus `local/third-party-notices.json`. These
private generated files are not release-ready until reviewed. The manifest
records Cargo license expressions, repository URLs, included license/notice
files, and warnings for missing or unreadable text. It does not choose a license
when Cargo metadata is absent and is not a legal compatibility determination.
Review every warning and compare the package list with the actual artifact
features before distribution. The integration's retained upstream WirePlumber
component is included from `integrations/wireplumber/LICENSE`.

An optional offline supplemental artifact can add verified upstream license
texts for exact package versions whose Cargo crate archives lacked readable
files:

```sh
python3 scripts/collect_notices.py --supplemental local/supplemental-notices.json
```

For every supplied text, the collector validates its stored UTF-8 byte count,
SHA-256, safe relative repository path, and raw GitHub URL against the pinned
repository and commit. It attaches text only when package name, version, Cargo
license expression, repository URL, and the commit from that unpacked crate's
`.cargo_vcs_info.json` all match. The candidate lists the exact matched package,
version, repository commit, source URL, hash and byte count beside each text.
It preserves the Cargo license expression and does not choose among alternate
licenses. Entries that do not match add no text; artifact blockers and other
unresolved warnings remain in the notices and JSON manifest for review. The
supplemental file is read-only input and must be supplied explicitly; the
collector does not fetch its contents or run Cargo to create it.

For an external private output directory, pass both explicit paths:

```sh
python3 scripts/collect_notices.py \
  --notices /private/review/THIRD_PARTY_NOTICES.txt \
  --manifest /private/review/third-party-notices.json
```

The command does not build the app or download dependencies. If locked offline
metadata cannot resolve in the existing cache, stop and review that separately;
do not make the collector fetch packages or invent missing license text.

The 2026-10-15 candidate reviewed 566 components and sourced supplemental texts
for 26 packages. Six still lacked sourced text: `hexf-parse`, `mac`,
`pathfinder_geometry`, `seahash`, `simd_helpers`, and `zune-inflate`. Those gaps,
plus the actual embedded asset inventory, remain distribution review items.
The candidate and its provenance are private generated evidence, not a checked-in
notice bundle or permission to distribute a binary.

## Artifact contents

The initial candidate should identify its architecture and tested Linux/runtime
requirements. A binary archive needs the executable, GPLv3 `LICENSE`, relevant
third-party notices and a short usage/limitations file. Supply the exact
corresponding source, including native C sources, manifests, lockfiles and build
instructions, for that binary. Record its commit and SHA256 checksum. Do not
include target caches, test evidence, credentials or machine-specific launchers.

Verify extraction and startup as a normal desktop user with the intended device
permissions. A build on one Debian laptop is not proof of compatibility with
other distributions or a clean machine. Installer, permissions and login-startup
integration need their own implementation and validation before being advertised.

Keep Actions disabled and use `[skip ci]` while the current publication policy
applies. Creating a tag, publishing a GitHub release or changing repository
visibility happens only when the owner requests publication. If a push is made,
verify the remote commit before reporting it complete.

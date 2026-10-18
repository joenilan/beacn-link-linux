# Preparing a LinX release

LinX is experimental. Releases are built locally (see below) and published on GitHub.

## Release scope

Describe it as **LinX — unofficial BEACN Link app for Linux**, and say that
the Windows streaming PC still uses BEACN App. List only features verified on
the release build, plus the limits in the README. Do not bundle BEACN
software, device firmware or proprietary fonts.

## Before tagging

1. Build the exact commit with the pinned toolchain and locked dependencies,
   using the `studio` feature.
2. With the Studio attached and BEACN App running on the streaming PC, check:
   audio on all four Link In channels, assignment from LinX and from BEACN
   App, faders and mute, media pause/resume, Pause/Connect, unplug and replug,
   and a long session. Confirm applications return to their previous outputs
   after Pause and Quit.
3. Update the README and real screenshots from that build
   ([screenshot guide](screenshots.md)).
4. Check the GPLv3 license and third-party notices for the distributed
   dependencies, and review changed trust boundaries (USB access, control
   socket) before distribution.

## Build the packages

```sh
packaging/release.sh
```

From a clean checkout of the release commit this builds, in
`local/release/v<version>/`:

- `linx-<version>-1.fc<N>.x86_64.rpm`, built natively on Fedora;
- `linx_<version>-1_amd64.deb` and `linx-<version>-x86_64-linux.tar.gz`, built
  in a Debian 13 container (podman) so the binary runs on glibc 2.41 and
  newer;
- `THIRD_PARTY_NOTICES.txt` (shipped in every package) and `SHA256SUMS`.

All packages install `/usr/bin/linx` (tarball: `~/.local` or `/usr/local`), the
desktop entry and icon, and the udev rule. GitHub Actions stay disabled;
publish the reviewed files with `gh release create v<version> ... --notes-file`.
Every package also includes the README and the optional WirePlumber 96 kHz
rule with its guide under `usr/share/doc/linx/wireplumber`. The rule remains
documentation only: installation does not enable it or restart WirePlumber.
For dependencies whose crate archive has no license file, the notices list the
declared license expression and append the standard SPDX text
(`packaging/licenses`, license-list-data v3.27.0), marked as such. Packages are
built with `studio,native-control`: the control socket stays off unless LinX
is started with `--control-socket`.

## Collect dependency notices

```sh
python3 scripts/collect_notices.py
```

The collector runs Cargo metadata with `--locked --offline` for
`x86_64-unknown-linux-gnu` and the `studio` feature, and writes
`local/THIRD_PARTY_NOTICES.txt` and `local/third-party-notices.json`. Review
every warning and compare the package list with the actual artifact before
distribution; the output is a candidate, not a legal determination.

An optional offline file can add verified upstream license texts for exact
package versions whose crate archives lack them:

```sh
python3 scripts/collect_notices.py --supplemental local/supplemental-notices.json
```

Each supplied text must match its package name, version, license expression,
repository and pinned commit, byte count and SHA-256; entries that do not
match add nothing.

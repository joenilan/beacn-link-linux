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
- `linx-<version>-1-x86_64.pkg.tar.zst`, that same tree packaged with makepkg
  in an Arch Linux container;
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

## Publish the package repositories

After the GitHub release is published:

```sh
packaging/publish-repo.sh
```

`release.sh` signs the RPM with the LinX package key (fingerprint
`E72D6EE9C417012ECE280124E14B87807621D25D`; `--unsigned` skips it for builds
without the key). `publish-repo.sh` adds the release's RPM and Debian package
to the dnf and apt repositories, keeps the newest three versions, signs the
repository metadata (`repomd.xml.asc`, `InRelease`, `Release.gpg`), commits
to the `gh-pages` branch from a checkout in `local/pages`, and pushes it to
GitHub Pages (https://joenilan.github.io/beacn-link-linux/). Also add the
release's `.rpm`, `.deb` and `.pkg.tar.zst` to the combined "joenilan apps"
repository with `scripts/publish-packages.sh` in the `joenilan.github.io`
repository, then `scripts/mirror-packages.py` there. Both scripts ask
for the key's passphrase through gpg-agent. It needs `createrepo_c` and
`rpm-sign`; `--no-push` builds and commits locally only. Keep an offline
backup of the secret key and its revocation certificate.

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

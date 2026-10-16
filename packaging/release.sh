#!/usr/bin/env bash
# Build LinX release packages from the current commit:
#   - Fedora RPM, built natively;
#   - Debian package and portable tarball, built in a Debian 13 container so
#     the binary runs on older glibc (Debian 13, Fedora 4x, Arch ...).
# Output: local/release/v<version>/ (ignored by Git), with SHA256SUMS.
#
# Usage: packaging/release.sh [--allow-dirty]
#   --allow-dirty  build uncommitted changes (never for a published release).
# Crates whose source archive ships no license file are listed in the notices
# with the standard text of their declared license (packaging/licenses, SPDX
# license-list-data v3.27.0), clearly marked as such.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/app-gpui/Cargo.toml" | head -1)
OUT="$ROOT/local/release/v$VERSION"
DIRTY=0
for arg in "$@"; do
  case "$arg" in
    --allow-dirty) DIRTY=1 ;;
    *) echo "Unknown argument: $arg" >&2; exit 2 ;;
  esac
done

die() { echo "release: $*" >&2; exit 1; }
cd "$ROOT"
if [ "$DIRTY" = 0 ] && ! git diff --quiet HEAD --; then
  die "uncommitted changes; commit first (a release builds the exact commit)"
fi
COMMIT=$(git rev-parse --short HEAD)
rm -rf "$OUT"
mkdir -p "$OUT"
echo "LinX $VERSION ($COMMIT) -> $OUT"

# ---- third-party notices -------------------------------------------------
supplemental=()
[ -f local/supplemental-notices.json ] && supplemental=(--supplemental local/supplemental-notices.json)
python3 scripts/collect_notices.py "${supplemental[@]}"
python3 - "$OUT/THIRD_PARTY_NOTICES.txt" <<'EOF'
import json, re, sys
out = sys.argv[1]
data = json.load(open("local/third-party-notices.json"))
text = open("local/THIRD_PARTY_NOTICES.txt").read()
missing = []
for w in data["warnings"]:
    m = re.match(r"No readable license/notice text for (\S+) (\S+)$", w)
    if m:
        missing.append(m.groups())
licenses = {p["name"] + " " + p["version"]: p.get("license") or "" for p in data["packages"]}
if missing:
    text += ("\n\n" + "=" * 78 + "\nPackages whose source archive contains no license file\n" + "=" * 78 +
             "\nFor the packages below, the published crate archive ships no license or notice\n"
             "file. Each is listed with the license expression declared in its Cargo\n"
             "metadata, followed by the standard text of that license from SPDX\n"
             "license-list-data v3.27.0. No copyright holder line was available upstream.\n")
    used = set()
    for name, version in missing:
        expr = licenses.get(f"{name} {version}", "")
        ids = sorted(set(re.findall(r"[A-Za-z0-9.\-]+", expr)) - {"OR", "AND", "WITH"})
        text += f"\n- {name} {version}: {expr}\n"
        used.update(ids)
    for spdx in sorted(used):
        try:
            body = open(f"packaging/licenses/{spdx}.txt").read()
        except FileNotFoundError:
            sys.exit(f"release: no standard text for {spdx} in packaging/licenses")
        text += "\n" + "-" * 78 + f"\n{spdx}\n" + "-" * 78 + "\n" + body
open(out, "w").write(text)
print(f"Notices: {len(data['packages'])} packages, {len(missing)} with declared-license texts")
EOF

# ---- staging (shared layout for every package) -----------------------------
stage() { # stage <binary> <destdir>
  local bin=$1 dest=$2
  install -Dm755 "$bin" "$dest/usr/bin/linx"
  install -Dm644 integrations/desktop/io.github.linx.desktop "$dest/usr/share/applications/io.github.linx.desktop"
  install -Dm644 integrations/desktop/io.github.linx.svg "$dest/usr/share/icons/hicolor/scalable/apps/io.github.linx.svg"
  install -Dm644 integrations/udev/70-linx-beacn-studio.rules "$dest/usr/lib/udev/rules.d/70-linx-beacn-studio.rules"
  install -Dm644 LICENSE "$dest/usr/share/licenses/linx/LICENSE"
  install -Dm644 "$OUT/THIRD_PARTY_NOTICES.txt" "$dest/usr/share/licenses/linx/THIRD_PARTY_NOTICES.txt"
  install -Dm644 README.md "$dest/usr/share/doc/linx/README.md"
}

# ---- Fedora RPM (native) ---------------------------------------------------
source "$HOME/.cargo/env" 2>/dev/null || true
FEDORA_TARGET="$ROOT/local/release-target/fedora"
CARGO_TARGET_DIR="$FEDORA_TARGET" cargo build --release --locked --features studio,native-control \
  --manifest-path app-gpui/Cargo.toml --bin beacn-linx-gpui
rm -rf "$OUT/stage-rpm"
stage "$FEDORA_TARGET/release/beacn-linx-gpui" "$OUT/stage-rpm"
rpmbuild -bb packaging/linx.spec \
  --define "_topdir $OUT/rpmbuild" \
  --define "linx_version $VERSION" \
  --define "linx_stage $OUT/stage-rpm" >/dev/null
find "$OUT/rpmbuild/RPMS" -name '*.rpm' -exec cp {} "$OUT/" \;
rm -rf "$OUT/rpmbuild" "$OUT/stage-rpm"

# ---- Debian package and portable tarball (Debian 13 container) -------------
podman run --rm --security-opt label=disable \
  -v "$ROOT":/src:ro \
  -v "$OUT":/out \
  -v linx-debian-cargo:/root/.cargo \
  -v linx-debian-rustup:/root/.rustup \
  -v linx-debian-target:/target \
  docker.io/library/debian:13 \
  bash /src/packaging/build-debian.sh "$VERSION"

(cd "$OUT" && sha256sum -- *.rpm *.deb *.tar.gz > SHA256SUMS)
echo
cat "$OUT/SHA256SUMS"

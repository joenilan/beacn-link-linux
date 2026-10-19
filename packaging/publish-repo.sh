#!/usr/bin/env bash
# Publish the signed dnf and apt package repositories for the release built by
# packaging/release.sh to the gh-pages branch, served by GitHub Pages at
# https://joenilan.github.io/beacn-link-linux/.
#
#   rpm/   RPM repository: signed packages, repodata with repomd.xml.asc
#   deb/   apt repository: pool/ and dists/stable (InRelease, Release.gpg)
#   linx.asc, linx.gpg, linx.repo, linx.sources: key and repository files
#
# The packages are the release's own files (the RPM is signed by release.sh);
# the newest KEEP versions stay in the repositories. Signing asks for the
# key's passphrase through gpg-agent. GitHub Actions stay disabled: this
# publishes from the local checkout.
#
# Usage: packaging/publish-repo.sh [--no-push]
#   --no-push  update the local gh-pages checkout (local/pages) only.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/app-gpui/Cargo.toml" | head -1)
OUT="$ROOT/local/release/v$VERSION"
PAGES="$ROOT/local/pages"
KEY=${LINX_SIGNING_KEY:-E72D6EE9C417012ECE280124E14B87807621D25D}
BASE_URL=https://joenilan.github.io/beacn-link-linux
KEEP=3
PUSH=1
for arg in "$@"; do
  case "$arg" in
    --no-push) PUSH=0 ;;
    *) echo "Unknown argument: $arg" >&2; exit 2 ;;
  esac
done

die() { echo "publish-repo: $*" >&2; exit 1; }
command -v createrepo_c >/dev/null || die "createrepo_c is not installed (sudo dnf install createrepo_c)"
gpg --list-secret-keys "$KEY" >/dev/null 2>&1 || die "signing key $KEY is not available"
RPM=$(ls "$OUT"/linx-"$VERSION"-*.x86_64.rpm 2>/dev/null | head -1)
DEB=$(ls "$OUT"/linx_"$VERSION"-*_amd64.deb 2>/dev/null | head -1)
[ -n "$RPM" ] && [ -n "$DEB" ] || die "no $VERSION packages in $OUT; run packaging/release.sh"
# The repository only takes the RPM that release.sh signed with this key.
rpm -qp --qf '%{SIGPGP:pgpsig}%{RSAHEADER:pgpsig}%{OPENPGP:pgpsig}\n' "$RPM" 2>/dev/null \
  | grep -qi "${KEY: -16}" || die "$RPM is not signed with $KEY"

# ---- gh-pages checkout -------------------------------------------------------
REMOTE=$(git -C "$ROOT" remote get-url origin)
if [ ! -d "$PAGES/.git" ]; then
  if git ls-remote --exit-code --heads "$REMOTE" gh-pages >/dev/null; then
    git clone -q --branch gh-pages --single-branch "$REMOTE" "$PAGES"
  else
    git init -q -b gh-pages "$PAGES"
    git -C "$PAGES" remote add origin "$REMOTE"
  fi
fi
cd "$PAGES"
# Commit as the main checkout's author.
for setting in user.name user.email; do
  value=$(git -C "$ROOT" config "$setting" || true)
  [ -n "$value" ] && git config "$setting" "$value"
done
if git ls-remote --exit-code --heads origin gh-pages >/dev/null; then
  git pull -q --ff-only origin gh-pages
fi

# ---- keys and repository definitions ----------------------------------------
touch .nojekyll
gpg --armor --export "$KEY" > linx.asc
gpg --export "$KEY" > linx.gpg
cat > linx.repo <<EOF
[linx]
name=LinX (unofficial BEACN Link app for Linux)
baseurl=$BASE_URL/rpm
enabled=1
gpgcheck=1
repo_gpgcheck=1
gpgkey=$BASE_URL/linx.asc
EOF
cat > linx.sources <<EOF
Types: deb
URIs: $BASE_URL/deb
Suites: stable
Components: main
Architectures: amd64
Signed-By: /usr/share/keyrings/linx-archive-keyring.gpg
EOF

# ---- packages: add this version, keep the newest KEEP ------------------------
prune() { # prune <dir> <glob>: keep the newest KEEP versions
  ls "$1"/$2 2>/dev/null | sort -V | head -n -"$KEEP" | xargs -r rm -f --
}
mkdir -p rpm deb/pool/main/l/linx
cp "$RPM" rpm/
cp "$DEB" deb/pool/main/l/linx/
prune rpm 'linx-*.rpm'
prune deb/pool/main/l/linx 'linx_*.deb'

# ---- RPM repository ----------------------------------------------------------
createrepo_c -q --update rpm
rm -f rpm/repodata/repomd.xml.asc
gpg --batch --yes -u "$KEY" --detach-sign --armor -o rpm/repodata/repomd.xml.asc rpm/repodata/repomd.xml

# ---- apt repository ----------------------------------------------------------
python3 -I - deb <<'EOF'
import gzip, hashlib, io, lzma, os, sys, tarfile, time

root = sys.argv[1]
dist = os.path.join(root, "dists", "stable")
binary = os.path.join(dist, "main", "binary-amd64")
os.makedirs(binary, exist_ok=True)

def ar_members(path):
    data = open(path, "rb").read()
    assert data[:8] == b"!<arch>\n", path
    pos = 8
    while pos + 60 <= len(data):
        header = data[pos:pos + 60]
        name = header[:16].decode().strip().rstrip("/")
        size = int(header[48:58])
        yield name, data[pos + 60:pos + 60 + size]
        pos += 60 + size + (size & 1)

def control(path):
    for name, body in ar_members(path):
        if name.startswith("control.tar"):
            if name.endswith(".xz"):
                body = lzma.decompress(body)
            elif name.endswith(".gz"):
                body = gzip.decompress(body)
            elif name.endswith(".zst"):
                from compression import zstd
                body = zstd.decompress(body)
            with tarfile.open(fileobj=io.BytesIO(body)) as tar:
                member = next(m for m in tar.getmembers() if m.name.lstrip("./") == "control")
                return tar.extractfile(member).read().decode().strip()
    raise SystemExit(f"no control file in {path}")

stanzas = []
pool = os.path.join(root, "pool")
for dirpath, _, files in sorted(os.walk(pool)):
    for f in sorted(files):
        if not f.endswith(".deb"):
            continue
        path = os.path.join(dirpath, f)
        blob = open(path, "rb").read()
        rel = os.path.relpath(path, root)
        stanzas.append(control(path) + "\n" + "\n".join([
            f"Filename: {rel}",
            f"Size: {len(blob)}",
            f"MD5sum: {hashlib.md5(blob).hexdigest()}",
            f"SHA1: {hashlib.sha1(blob).hexdigest()}",
            f"SHA256: {hashlib.sha256(blob).hexdigest()}",
        ]))
packages = ("\n\n".join(stanzas) + "\n").encode()
open(os.path.join(binary, "Packages"), "wb").write(packages)
with gzip.GzipFile(os.path.join(binary, "Packages.gz"), "wb", mtime=0) as g:
    g.write(packages)

files = []
for name in ("main/binary-amd64/Packages", "main/binary-amd64/Packages.gz"):
    blob = open(os.path.join(dist, name), "rb").read()
    files.append((name, len(blob), hashlib.md5(blob).hexdigest(), hashlib.sha256(blob).hexdigest()))
release = [
    "Origin: LinX",
    "Label: LinX",
    "Suite: stable",
    "Codename: stable",
    "Architectures: amd64",
    "Components: main",
    "Description: LinX, unofficial BEACN Link app for Linux",
    "Date: " + time.strftime("%a, %d %b %Y %H:%M:%S UTC", time.gmtime()),
    "MD5Sum:",
    *[f" {md5} {size} {name}" for name, size, md5, _ in files],
    "SHA256:",
    *[f" {sha} {size} {name}" for name, size, _, sha in files],
]
open(os.path.join(dist, "Release"), "w").write("\n".join(release) + "\n")
print(f"apt: {len(stanzas)} package(s)")
EOF
rm -f deb/dists/stable/InRelease deb/dists/stable/Release.gpg
gpg --batch --yes -u "$KEY" --clearsign -o deb/dists/stable/InRelease deb/dists/stable/Release
gpg --batch --yes -u "$KEY" --detach-sign --armor -o deb/dists/stable/Release.gpg deb/dists/stable/Release

# ---- landing page --------------------------------------------------------------
cat > index.html <<EOF
<!doctype html>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>LinX packages</title>
<style>body{font:16px/1.5 system-ui,sans-serif;max-width:46rem;margin:2rem auto;padding:0 16px}pre{background:#f2f2f2;padding:.75rem;overflow-x:auto}@media(prefers-color-scheme:dark){body{background:#16181b;color:#e4e6e9}pre{background:#24272b}a{color:#8ab4f8}}</style>
<h1>LinX package repositories</h1>
<p>Signed dnf and apt repositories for <a href="https://github.com/joenilan/beacn-link-linux">LinX</a>,
an unofficial BEACN Link app for Linux (not affiliated with BEACN). Current version: $VERSION.</p>
<p>Signing key fingerprint: <code>$(gpg --with-colons --fingerprint "$KEY" | awk -F: '/^fpr/{print $10; exit}')</code></p>
<h2>Fedora</h2>
<pre>sudo dnf config-manager addrepo --from-repofile=$BASE_URL/linx.repo
sudo dnf install linx</pre>
<h2>Debian 13 / Ubuntu</h2>
<pre>sudo curl -fsSLo /usr/share/keyrings/linx-archive-keyring.gpg $BASE_URL/linx.gpg
sudo curl -fsSLo /etc/apt/sources.list.d/linx.sources $BASE_URL/linx.sources
sudo apt update
sudo apt install linx</pre>
EOF

git add -A
if git diff --cached --quiet; then
  echo "publish-repo: nothing changed"
else
  git commit -q -m "LinX $VERSION packages [skip ci]

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
  echo "publish-repo: committed $(git rev-parse --short HEAD) in $PAGES"
fi
[ "$PUSH" = 1 ] || exit 0
git push -q origin gh-pages
gh api "repos/joenilan/beacn-link-linux/pages" >/dev/null 2>&1 \
  || gh api --silent -X POST "repos/joenilan/beacn-link-linux/pages" -f 'source[branch]=gh-pages' -f 'source[path]=/' >/dev/null
echo "publish-repo: pushed; $BASE_URL/"

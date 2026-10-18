#!/usr/bin/env bash
# Runs inside the Debian 13 container started by packaging/release.sh:
# /src = repository (read-only), /out = release directory, /target = Cargo
# target cache. Builds the Debian package and the portable tarball.
set -euo pipefail
VERSION=$1
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq --no-install-recommends \
  ca-certificates curl build-essential pkg-config clang cmake dpkg-dev \
  libpipewire-0.3-dev libusb-1.0-0-dev libxkbcommon-dev libxkbcommon-x11-dev \
  libwayland-dev libvulkan-dev libfontconfig-dev libfreetype-dev libxcb1-dev \
  libx11-dev libx11-xcb-dev libssl-dev libdbus-1-dev libglib2.0-dev libzstd-dev >/dev/null

TOOLCHAIN=$(sed -n 's/^channel = "\(.*\)"/\1/p' /src/app-gpui/rust-toolchain.toml)
if ! [ -x /root/.cargo/bin/rustup ]; then
  curl -sSf https://sh.rustup.rs | sh -s -- -y -q --profile minimal --default-toolchain "$TOOLCHAIN"
fi
source /root/.cargo/env
rustup toolchain install --profile minimal "$TOOLCHAIN" 2>&1 | grep -v -E "^(info|  )" || true

cd /src
CARGO_TARGET_DIR=/target cargo build --release --locked --features studio,native-control \
  --manifest-path app-gpui/Cargo.toml --bin beacn-linx-gpui
BIN=/target/release/beacn-linx-gpui

stage() {
  local dest=$1
  install -Dm755 "$BIN" "$dest/usr/bin/linx"
  install -Dm644 integrations/desktop/io.github.linx.desktop "$dest/usr/share/applications/io.github.linx.desktop"
  install -Dm644 integrations/desktop/io.github.linx.svg "$dest/usr/share/icons/hicolor/scalable/apps/io.github.linx.svg"
  install -Dm644 integrations/udev/70-linx-beacn-studio.rules "$dest/usr/lib/udev/rules.d/70-linx-beacn-studio.rules"
  install -Dm644 LICENSE "$dest/usr/share/doc/linx/copyright"
  install -Dm644 /out/THIRD_PARTY_NOTICES.txt "$dest/usr/share/doc/linx/THIRD_PARTY_NOTICES.txt"
  install -Dm644 README.md "$dest/usr/share/doc/linx/README.md"
  install -Dm644 integrations/wireplumber/README.md "$dest/usr/share/doc/linx/wireplumber/README.md"
  install -Dm644 integrations/wireplumber/51-beacn-studio-link-rate.conf "$dest/usr/share/doc/linx/wireplumber/51-beacn-studio-link-rate.conf"
}

# ---- Debian package ----
WORK=$(mktemp -d)
PKG=$WORK/linx_${VERSION}-1_amd64
stage "$PKG"
mkdir -p "$WORK/debian" "$PKG/DEBIAN"
printf 'Source: linx\n\nPackage: linx\nArchitecture: amd64\n' > "$WORK/debian/control"
DEPENDS=$(cd "$WORK" && dpkg-shlibdeps -O -e "$PKG/usr/bin/linx" 2>/dev/null | sed -n 's/^shlibs:Depends=//p')
cat > "$PKG/DEBIAN/control" <<EOF
Package: linx
Version: ${VERSION}-1
Architecture: amd64
Maintainer: DreadedZombie <joenilan+dev@gmail.com>
Depends: ${DEPENDS}, libvulkan1, libwayland-client0, libwayland-egl1, pipewire (>= 1.0), wireplumber
Recommends: mesa-vulkan-drivers
Section: sound
Priority: optional
Homepage: https://github.com/joenilan/beacn-link-linux
Description: Unofficial BEACN Link app for Linux
 LinX gives a Linux gaming PC BEACN Studio's four Link channels:
 applications play to "LinX Link Out" .. "LinX Link 4 Out", the Studio's
 return pairs arrive as "LinX Link In" .. "LinX Link 4 In", and BEACN App on
 the Windows streaming PC sees each application and can move it.
 Unofficial; not affiliated with BEACN.
EOF
cat > "$PKG/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
udevadm control --reload-rules >/dev/null 2>&1 || true
udevadm trigger --subsystem-match=usb --attr-match=idVendor=33ae >/dev/null 2>&1 || true
EOF
chmod 755 "$PKG/DEBIAN/postinst"
dpkg-deb --root-owner-group --build "$PKG" /out/ >/dev/null

# ---- portable tarball ----
TAR=linx-${VERSION}-x86_64-linux
mkdir -p "$WORK/$TAR"
stage "$WORK/$TAR/root"
mv "$WORK/$TAR/root"/* "$WORK/$TAR/" && rmdir "$WORK/$TAR/root"
install -m755 packaging/install.sh packaging/uninstall.sh "$WORK/$TAR/"
tar -C "$WORK" -czf "/out/$TAR.tar.gz" "$TAR"
chown "$(stat -c %u:%g /out)" /out/*.deb /out/*.tar.gz
echo "Debian 13 build: glibc $(ldd --version | head -1 | awk '{print $NF}'), Depends: $DEPENDS"

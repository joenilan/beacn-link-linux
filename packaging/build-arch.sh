#!/usr/bin/env bash
# Runs inside the Arch Linux container started by packaging/release.sh:
# /out = release directory holding the portable tarball (built on Debian 13,
# so its binary runs on Arch's newer glibc). Packages that tree with makepkg.
set -euo pipefail
VERSION=$1
pacman -Sy --noconfirm --needed base-devel >/dev/null 2>&1
useradd -m builder
WORK=/home/builder/linx
mkdir -p "$WORK/src-tree"
tar -C "$WORK/src-tree" -xzf "/out/linx-${VERSION}-x86_64-linux.tar.gz" --strip-components=1 "linx-${VERSION}-x86_64-linux/usr"
# Arch keeps licenses under /usr/share/licenses.
install -Dm644 /src/LICENSE "$WORK/src-tree/usr/share/licenses/linx/LICENSE"
cat > "$WORK/PKGBUILD" <<EOF
pkgname=linx
pkgver=${VERSION//-/.}
pkgrel=1
pkgdesc='Unofficial BEACN Link app for Linux'
arch=(x86_64)
url='https://github.com/joenilan/beacn-link-linux'
license=(GPL-3.0-only)
depends=(glibc gcc-libs libx11 libxcb libxkbcommon libxkbcommon-x11 'libusb>=1.0.27' 'pipewire>=1.0' wireplumber vulkan-icd-loader wayland)
optdepends=('vulkan-driver: GPU rendering (for example vulkan-radeon, vulkan-intel or nvidia-utils)')
install=linx.install
options=(!strip !debug)
package() {
  cp -a "\$startdir/src-tree/usr" "\$pkgdir/"
}
EOF
cat > "$WORK/linx.install" <<'EOF'
post_install() {
  udevadm control --reload-rules >/dev/null 2>&1 || true
  udevadm trigger --subsystem-match=usb --attr-match=idVendor=33ae >/dev/null 2>&1 || true
}
post_upgrade() { post_install; }
post_remove() { udevadm control --reload-rules >/dev/null 2>&1 || true; }
EOF
chown -R builder "$WORK"
su builder -c "cd $WORK && PKGDEST=$WORK makepkg --nodeps --noconfirm >/dev/null"
cp "$WORK"/linx-*.pkg.tar.zst /out/
chown "$(stat -c %u:%g /out)" /out/*.pkg.tar.zst
echo "Arch package: $(basename "$WORK"/linx-*.pkg.tar.zst)"

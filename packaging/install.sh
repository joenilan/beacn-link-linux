#!/bin/sh
# Install LinX from the portable tarball.
#   ./install.sh            for this user (~/.local), USB rule via sudo
#   ./install.sh --system   for all users (/usr/local), needs sudo
set -eu
cd "$(dirname "$0")"
if [ "${1:-}" = "--system" ]; then
  PREFIX=/usr/local
  SUDO=sudo
else
  PREFIX=$HOME/.local
  SUDO=
fi
$SUDO install -Dm755 usr/bin/linx "$PREFIX/bin/linx"
$SUDO install -Dm644 usr/share/applications/io.github.linx.desktop "$PREFIX/share/applications/io.github.linx.desktop"
$SUDO install -Dm644 usr/share/icons/hicolor/scalable/apps/io.github.linx.svg \
  "$PREFIX/share/icons/hicolor/scalable/apps/io.github.linx.svg"
# Lets the desktop user open BEACN Studio's Link control interface.
sudo install -Dm644 usr/lib/udev/rules.d/70-linx-beacn-studio.rules /etc/udev/rules.d/70-linx-beacn-studio.rules
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=usb --attr-match=idVendor=33ae || true
update-desktop-database "$PREFIX/share/applications" >/dev/null 2>&1 || true
echo "LinX installed to $PREFIX/bin/linx. Start it from your application menu."
case ":$PATH:" in *":$PREFIX/bin:"*) ;; *) echo "Note: $PREFIX/bin is not on your PATH." ;; esac

#!/bin/sh
# Remove a tarball installation of LinX (same option as install.sh).
set -eu
if [ "${1:-}" = "--system" ]; then
  PREFIX=/usr/local
  SUDO=sudo
else
  PREFIX=$HOME/.local
  SUDO=
fi
$SUDO rm -f "$PREFIX/bin/linx" \
  "$PREFIX/share/applications/io.github.linx.desktop" \
  "$PREFIX/share/icons/hicolor/scalable/apps/io.github.linx.svg" \
  "$PREFIX/share/doc/linx/README.md" \
  "$PREFIX/share/doc/linx/wireplumber/README.md" \
  "$PREFIX/share/doc/linx/wireplumber/51-beacn-studio-link-rate.conf"
$SUDO rmdir "$PREFIX/share/doc/linx/wireplumber" "$PREFIX/share/doc/linx" 2>/dev/null || true
rm -f "${XDG_CONFIG_HOME:-$HOME/.config}/autostart/io.github.linx.desktop"
sudo rm -f /etc/udev/rules.d/70-linx-beacn-studio.rules
sudo udevadm control --reload-rules
echo "LinX removed. Channel choices in ~/.config/linx were kept."

#!/usr/bin/env bash
# Capture the running LinX Console for the README on a Wayland desktop.
#
# GPUI captures only its own window, and only through X11, so this helper
# restarts the installed LinX under Xwayland with the native-control socket,
# captures and frames the window with scripts/capture_screenshot.py, then
# restarts LinX normally. Audio pauses for a few seconds at each restart;
# remembered channels are restored automatically. Run it when nothing
# important is playing.
#
# Usage: scripts/capture_x11.sh [--frame] [extra capture_screenshot.py args]
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
SOCK="${XDG_RUNTIME_DIR:?}/linx-control.sock"
LINX=$(command -v linx) || { echo "linx is not installed on PATH" >&2; exit 1; }

stop_linx() {
  local pid
  for pid in $(pgrep -x linx || true); do
    python3 - "$SOCK" <<'EOF' 2>/dev/null || true
import socket, sys
s = socket.socket(socket.AF_UNIX); s.settimeout(3); s.connect(sys.argv[1])
s.sendall(b'{"id":1,"op":"quit"}\n'); s.recv(4096)
EOF
    for _ in $(seq 40); do kill -0 "$pid" 2>/dev/null || break; sleep 0.5; done
    if kill -0 "$pid" 2>/dev/null; then
      kill -TERM "$pid"
      for _ in $(seq 40); do kill -0 "$pid" 2>/dev/null || break; sleep 0.5; done
    fi
    if kill -0 "$pid" 2>/dev/null; then
      echo "LinX $pid did not exit" >&2
      exit 1
    fi
  done
  return 0
}

start_linx() { # start_linx x11|wayland
  if [ "$1" = x11 ]; then
    env -u WAYLAND_DISPLAY setsid "$LINX" --studio --control-socket "$SOCK" >/dev/null 2>&1 </dev/null &
  else
    setsid "$LINX" --studio --control-socket "$SOCK" >/dev/null 2>&1 </dev/null &
  fi
  for _ in $(seq 60); do [ -S "$SOCK" ] && return 0; sleep 0.5; done
  echo "LinX control socket did not appear" >&2; exit 1
}

stop_linx
start_linx x11
trap 'stop_linx; start_linx wayland' EXIT
# The window manager may restore an older X11 window size: set the designed
# console size (788x568 inside GPUI's 20 px client-side shadow margin).
python3 - "$SOCK" <<'PY'
import socket, sys
s = socket.socket(socket.AF_UNIX); s.settimeout(3); s.connect(sys.argv[1])
s.sendall(b'{"id":1,"op":"resize","width":828,"height":608}\n'); s.recv(4096)
PY
sleep 3 # let channels, meters and BEACN App state settle
python3 "$ROOT/scripts/capture_screenshot.py" --socket "$SOCK" "$@"

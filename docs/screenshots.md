# LinX screenshots

The helpers require Python 3 and Pillow (`python3-pil` on Debian 13). They are
documentation tools; the native LinX app does not use Python.

The README screenshot is framed from a real, app-only LinX GPUI capture. The
capture tool talks only to an already-running app's explicitly enabled local
native-control socket. It requests `state` and `screenshot`; it does not launch
or stop LinX, assign routes, change controls, use the desktop, or capture other
windows. Native control is opt-in; start LinX separately with
`--control-socket /absolute/private/path/linx-control.sock` when preparing a
capture. Keep the socket in a canonical directory owned by your user with mode
0700. The tool checks the socket's owner/mode and Linux peer credentials before
accepting state or image data.

The running executable must include the `native-control` feature. For a
read-only capture session, from `app-gpui/` run
`cargo run --features native-control -- --control-socket "$XDG_RUNTIME_DIR/linx-control.sock"`.
To capture a Studio session, build with `studio,native-control` and add `--studio`
when launching. Reuse the already-running app when it has this endpoint enabled.

With the app running and the output directories present:

```sh
mkdir -p local
python3 scripts/capture_screenshot.py \
  --socket "$XDG_RUNTIME_DIR/linx-control.sock"
```

This writes the raw PNG to `local/linx-console-raw.png` and a private manifest
to `local/linx-console-capture.json`. The manifest records app phase, currently
verified Link pairs, UTC capture time, binary and image SHA-256 hashes, and
dimensions. Both local artifacts are mode 0600. Review the raw capture and
manifest for private information before sharing them.

To also regenerate the README image, add `--frame`. It imports the existing
framing function directly and writes `assets/screenshots/linx-console.png`:

```sh
python3 scripts/capture_screenshot.py \
  --socket "$XDG_RUNTIME_DIR/linx-control.sock" \
  --frame
```

The framing script adds a dark teal and mint background, fine grid, rounded
image mask, and soft shadow. It preserves the screenshot content and never
enlarges it. The default canvas is 1200 × 860. To frame an already reviewed raw
image separately, use `python3 scripts/frame_screenshot.py --input
local/linx-console-raw.png`.

The app writes a temporary PPM beside its socket using exclusive creation. The
collector refuses to proceed if that exact path already exists; collect or
move an earlier capture before retrying. After it has validated the new file,
saved the PNG and manifest, and verified the capture file's inode and contents,
it removes only that newly captured PPM. Output files are never followed
through symlinks. Do not substitute a full-desktop screenshot or a designed
mockup for the native app capture in the README.

For manual app-window qualification, the developer-only `--capture-frame`
option can save the GPUI-rendered window as PPM. It needs the existing Studio
feature and a fresh output path:

```sh
mkdir -p local
cd app-gpui
cargo run --features studio,ui-qualification -- \
  --capture-frame ../local/linx-console-capture.ppm
```

The app waits three seconds before capture. Convert a manually captured PPM
with Python 3 and Pillow, then review it before framing:

```sh
cd ..
python3 -c 'from PIL import Image; Image.open("local/linx-console-capture.ppm").save("local/linx-console-raw.png")'
python3 scripts/frame_screenshot.py --input local/linx-console-raw.png
```

# Developer control socket

The optional `native-control` feature exposes the running LinX window through
a local Unix socket for automated UI checks and screenshots. It dispatches
through the app's own GPUI handlers; there is no second routing path, network
listener, cursor movement or keyboard injection.

```sh
linx --studio --control-socket "$XDG_RUNTIME_DIR/linx-control.sock"
```

Builds and launches without `--control-socket` have no endpoint. The parent
directory must be private and owned by the user; the socket is mode 0600 and
only peers with the same UID are served. Existing paths are refused, and
shutdown removes only the socket it created.

## Protocol

One JSON object per line, each with an `id` and `op`. Requests are capped at
16 KiB, responses at 2 MiB, the queue at eight entries and request lifetime at
five seconds. One client is served at a time. A successful response to an
event means it was dispatched; read `state` to see the result.

```json
{"id":1,"op":"state"}
{"id":2,"op":"click","target":"applications-2"}
{"id":3,"op":"scroll","target":"fader-2","delta":-40}
```

Responses are `{"id":1,"result":{"ok":true,"value":...}}` or
`{"result":{"ok":false,"error":"..."}}`. Unknown, hidden or disabled
components are refused.

| Operation | Behavior |
| --- | --- |
| `state`, `components` | Phase, status, Studio/transport/BEACN App state, applications with their observed and remembered channels, per-channel level and meter, visible components with bounds |
| `click` with `target` | Click the named component |
| `text` with `target`, `text` | Replace the application search text (up to 1024 bytes) |
| `scroll` with `target`, `delta` | Wheel event inside the app (faders move 1 dB per step) |
| `resize` with `width`, `height` | Resize within 360–1600 by 400–1200 |
| `screenshot` | Capture only this window to a private PPM next to the socket |
| `pause`, `connect` | Same as the Pause/Connect button |
| `close` | Same as the window's close button (tray or minimize) |
| `quit` | Stop routing and quit |

Component IDs: `session`, `settings`, `applications`, `applications-2` to
`applications-4`, `fader-1` to `fader-4`, `mute-1` to `mute-4`,
`route-<0..4>-<app id>` (0 is Off; the app id is in `state`), `search-input`,
`application-list`, `auto-connect`, `theme-dark`, `theme-light`,
`accent-mint/blue/violet/amber/rose`, `launch-at-login` and `quit-app`.

Meter values in `state` are aggregates (RMS, peak, hold in dBFS); no audio
samples are exposed. The controller must remove the previous screenshot file
before requesting another; the app never overwrites it or accepts a
caller-chosen path.

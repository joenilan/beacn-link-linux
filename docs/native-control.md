# Native LinX component control

The optional `native-control` feature exposes the actual GPUI application through
an explicitly enabled Linux Unix socket. It uses the normal native worker and
real GPUI click/text/scroll handlers. There is no separate routing implementation,
browser server, OS cursor movement or keyboard injection into another app.

Build with `--features studio,native-control` to include the physical backend and
control interface. Normal builds and launches without `--control-socket` have no
endpoint. Example on the normal Linux desktop account:

```sh
linx --studio --control-socket "$XDG_RUNTIME_DIR/linx-control.sock"
```

The absolute parent directory must already be private, canonical and owned by
that account. The socket is mode 0600; Linux peer credentials must have the same
effective UID. Existing paths are refused. Shutdown removes only the retained
socket inode. No network listener, stored credentials or SSH configuration is
created. This is a local development control interface, not an MCP server.

## Protocol

Send one JSON object per line. Every request needs a string/number `id` and `op`.
Requests are capped at 16 KiB, responses at 2 MiB, the queue at eight entries and
the request lifetime at five seconds. The UI rejects expired requests before
dispatch. One controller is served at a time; an idle or incomplete client is
disconnected after five seconds so it cannot monopolize the listener. The target
is the retained app window handle rather than the first window in a list.
A response to an event means it was
dispatched; read `state` to observe asynchronous worker completion or failure.

```json
{"id":1,"op":"state"}
{"id":2,"op":"pause"}
{"id":3,"op":"click","target":"applications"}
{"id":4,"op":"text","target":"search-input","text":"Firefox"}
```

Successful UI responses use `{"id":1,"result":{"ok":true,"value":...}}`;
UI refusal uses `result.ok=false` and `result.error`. Framing/transport errors
have a top-level `error`. Unknown, hidden, missing and disabled components are
refused. The component list reports current bounds, visibility and enabled state;
it is a curated inventory of controls rather than arbitrary memory inspection.

| Operation | Behavior |
| --- | --- |
| `state`, `components` | Actual catalog/stream identities, selections, verified assignments, pending waiting identity/name/pair, phase, startup state, errors, cleanup, completed cycles and visible component bounds |
| `click` with `target` | Dispatch through the named native component's GPUI handler |
| `text` with `target`, `text` | Replace the search or an expanded published-name input, up to 1024 bytes |
| `scroll` with `target`, `delta` | Send a bounded vertical wheel event inside the app |
| `resize` with `width`, `height` | Resize this window within 360–1600 by 400–1200 |
| `screenshot` | Capture only this app's retained window; returns the private PPM path |
| `pause` | Queue the same native Stop command as Pause, including while idle |
| `quit` | Join/release the native worker, then close the application |

Typical IDs are `session`, `settings`, `applications`, `applications-2` through
`applications-4`, `applications-settings`, `auto-connect`, `theme-light`,
`theme-dark`, `accent-mint/blue/violet/amber/rose`, `search-input` and
`application-list`. Rows expose `details-<id>` and expanded `name-<id>` /
`save-<id>`. Read-only and non-Studio rows expose `select-<id>` for Select/Remove;
in Studio mode, selected rows expose it for Remove, while unselected rows omit it.
Discover IDs from `state`; do not bind a future process or stream by an old
numeric ID or display name. Studio rows expose `route-<pair>-<id>` for the
currently chosen well. This **Assign** action also selects the app when needed,
starts the actual desktop owner if needed, validates current identity and planner
intent, and reports a nonzero assignment only after a fresh physical route proof.
It is independent of received USB assignment records. Expanded name editing uses
Assign with name in Studio mode; it requires removing an existing selection before
changing its published name. Read-only and non-Studio views keep Select with name /
Save name.
While Studio waits for source audio, `state.waiting` reports the remembered
application/name/pair separately from verified `applications[].assignment`. The
matching Console well shows that name with Waiting for audio. The session button
becomes Pause and cancels the pending intent through Stop; meter state is null.
Mini, gain and mute remain unwired. Live stereo meters now follow the verified
pair. `state.meter` is null when unavailable, paused, invalid or stale; otherwise
it includes `pair`, `rms_dbfs`, `peak_dbfs`, `hold_dbfs`, native `sequence`/`frames`
and linear peak/RMS aggregates. It never includes audio samples. See
[native meters](native-meters.md) for measurement and freshness boundaries.

The controller must collect/remove the preceding screenshot before requesting
another. The app never overwrites that path or accepts a caller-selected output
path. GPU readback is attempted first, then the app's own X11/Xwayland handle;
unsupported Wayland capture returns an error. It never captures the whole desktop.

Automatic connection can be enabled in the user's preferences. For UI-only work,
send `pause` and observe `startup=Paused` **before selecting an application**.
Selecting while automatic connection is armed can start the native hardware
session. App control does not grant access to BEACN App on another computer or
manufacture received assignment records.

## Current evidence and remaining boundary

On 2026-10-15 the actual desktop main was driven through this endpoint to open
Applications, edit/search/select/remove Firefox, expand details, change both
themes and all five accents, and capture its own window. It stayed paused with no
USB/audio start; desktop and tracked persistent-state comparison matched exactly.
Transport tests cover local round-trip, private socket mode, disabled-by-default
startup, preserved occupied paths and oversized requests. Focused native checks
cover all four planned assignments and cleanup. A subsequent actual native-app
run also assigned Firefox through all four wells in one physical session, with
continuous PCM progress and original-route restoration on Pause. Saved state
matched after its delayed write settled. Only the recreated selected-route link
serials differ in the raw graph. Heard output remains an owner observation.
In the subsequent announced run, the owner remotely confirmed that BEACN App
reflected all four local assignments and showed incoming audio activity. Each
pair held for 15 seconds; the native session reached 95 cycles before clean
Pause, with saved-state hashes matching. Direct listening remains pending.

An earlier owner attempt stopped after its selected roster changed, before any
assignment was accepted. Logs then lacked the invalidating selection event;
the precise cause remains unknown. Revocation now records old/new stream identity
metadata, and explicit selection commands are logged separately. The UI retains
the session error above routine selection feedback. Do not claim that this has
already fixed or reproduced the owner's physical failure.

Reference architecture: Haiwire's separated native control transport/UI dispatch
(`experiments/gpui-window/src/control_stdio.rs`, `control_window.rs`) and explicit
opt-in control documented in its `docs/INTEGRATIONS/native-control-toggle.md`.
LinX uses a Linux same-user socket and its pinned GPUI Kit APIs rather than
copying Windows named-pipe or unrelated application code.

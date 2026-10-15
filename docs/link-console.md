# Link Console

The default `--studio` mode. LinX gives the Linux PC four Link In channels on
BEACN Studio and routes applications to them, the way BEACN Link does on
Windows. The earlier single-application session remains as `--studio-legacy`.

## How it works

| Owner | Role |
|---|---|
| `native/link_bus.c` | PipeWire client on a `pw_thread_loop`. Owns four sinks `linx_link_in_1..4` ("LinX Link In N"), one 96-kHz 8-channel ALSA transport on the Studio (`hw:N,0`, AUX0–7), links from each sink's monitor to its Link In pair, and an 8-port level meter. Every 500 ms it recreates anything owned that disappeared or failed, with backoff. Routes applications only by writing `target.object` in the session manager's `default` metadata, like a desktop volume control. |
| `src/link_bus.rs` | FFI and snapshot parsing. A short PipeWire stall is distinguished from a lost connection. |
| `src/link_engine.rs` | `LinkService`: groups streams into applications, keeps per-application channel memory, restores each app's previous output on Off and Pause, publishes playing applications to BEACN App and applies its assignments on a separate USB thread. |
| `src/native_usb.rs` (`ControlUsb`) | BEACN Link control interface (interface 3 only; audio stays with the kernel driver). |
| `app-core/src/link.rs` | `LinkWorker`: connection lifecycle, automatic reconnection after PipeWire restarts, channel memory in `~/.config/linx/channels`. |
| `app-gpui/src/link_ui.rs`, `link_control.rs` | Console view, tray lifecycle and the optional developer control socket. |

Behavior:

- Any number of applications on any channel; moving one is a single redirect.
- Channel memory re-applies to new streams (media pause/resume, new tabs,
  restarts) and after PipeWire or WirePlumber recreate their objects.
- The channel shown for an application is read back from PipeWire. Only links
  into audio sinks count, so capture tools such as OBS application audio do
  not hide a route.
- Faders and mute set each LinX sink's volume and mute. The Studio transport
  and meters read the post-fader signal, and the same level is the
  "LinX Link In N" volume in desktop sound settings, in both directions. The
  fader spans 0 to −60 dB plus a silent stop; the wheel moves 1 dB.
- Playing applications are published to BEACN App every 500 ms; assignments
  1–4 made there move the application. "BEACN App connected" means it
  answered within the last five seconds.
- A USB control failure is retried every few seconds and never interrupts
  audio.
- Pause and Quit return every moved application to the output it used before.
  No WirePlumber changes are required.
- Closing the window keeps LinX in the system tray, or minimized where the
  desktop has no tray.

## Verified

Software checks on Debian 13 (PipeWire 1.4.2, WirePlumber 0.5.8) with the
actual GUI: assignment, a capture tap, media pause/resume, Pause and
reconnect, channel moves, Off, faders and mute against a test tone, and
close to the dock. WirePlumber's saved application settings matched their
starting values afterwards.

Pending with the Studio attached: audio on each Link In, names and
assignments in BEACN App, unplug and replug, a long session, desktops with a
tray host, and distributions whose BEACN UCM profile splits the Studio into
separate Link outputs.

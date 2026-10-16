# Link Console

The default `--studio` mode. LinX gives the Linux PC four Link Out channels on
BEACN Studio and routes applications to them, the way BEACN Link does on
Windows. The earlier single-application session remains as `--studio-legacy`.

## How it works

| Owner | Role |
|---|---|
| `native/link_bus.c` | PipeWire client on a `pw_thread_loop`. Owns four sinks `linx_link_in_1..4` ("LinX Link Out", "LinX Link N Out", the Windows driver's gaming-PC names; BEACN App shows them as Link In), one 96-kHz 8-channel ALSA transport on the Studio (`hw:N,0`, AUX0–7), links from each sink's monitor to its Studio pair, and an 8-port level meter. Every 500 ms it recreates anything owned that disappeared or failed, with backoff. Also owns the Studio capture side: a second 96-kHz adapter in the same node group feeding four return sources `linx_link_out_1..4` ("LinX Link In", "LinX Link N In", as on Windows; node names are kept for compatibility). It binds the session manager's device for the Studio card (matched by ALSA card id in `object.path`), switches its profile to "off" without saving while LinX runs, and restores the previous profile on Pause/Quit only if it is still off. Routes applications only by writing `target.object` in the session manager's `default` metadata, like a desktop volume control. |
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
  "LinX Link Out" / "LinX Link N Out" volume in desktop sound settings, in both directions. The
  fader spans 0 to −60 dB plus a silent stop; the wheel moves 1 dB.
- Playing applications are published to BEACN App every 500 ms; assignments
  1–4 made there move the application. "BEACN App connected" means it
  answered within the last five seconds.
- A USB control failure is retried every few seconds and never interrupts
  audio.
- Pause and Quit return every moved application to the output it used before.
  No WirePlumber changes are required.
- Closing the window keeps LinX in the system tray, or minimized where the
  desktop has no tray. "Start LinX when you log in" writes a standard entry in
  `~/.config/autostart`; at login LinX starts with `--start-hidden`.

## Verified

With BEACN Studio on a Debian 13 laptop (PipeWire 1.4.2, WirePlumber 0.5.8)
and BEACN App on the Windows streaming PC, listened to on the streaming PC:

- Firefox moved through Link In 1–4 from LinX: heard on each channel, moves
  within about a second, the 96-kHz transport running continuously, and
  BEACN App showing the same channel.
- Assignments made in BEACN App moved Firefox within a second.
- Short and 45-second media pauses kept Link 1; when Firefox recreated its
  stream, LinX routed the new stream to Link 1 before it played.
- Fader levels and mute applied to the channel.
- Unplugging and replugging the Studio recovered on its own within about
  1.5 s, with the assignment kept.
- Closing and reopening BEACN App did not interrupt audio.

Software checks with the actual GUI also cover capture taps, Pause and
reconnect, Off, close to the dock, launch at login and a 15-minute churn soak
with stable memory; saved application settings matched afterwards.

Not yet covered: desktops with a tray host, multi-hour sessions, and
distributions whose BEACN UCM profile splits the Studio into separate Link
outputs.

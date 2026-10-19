# Future ideas

Ideas that are wanted but not scheduled. Nothing here is implemented.

## Output device switcher

Switch the desktop's default output (for example speakers and a BEACN/LinX
channel) from LinX, for any user and any outputs.

- Where: a *Sound output* submenu in the tray (quickest for frequent
  switching), and optionally a dropdown in the Console header.
- How: the native PipeWire layer (`native/link_bus.c`) already binds the
  session manager's `default` metadata and tracks every `Audio/Sink`. Add
  reading `default.audio.sink` and writing `default.configured.audio.sink`
  (what `wpctl set-default` does), expose the sink list and current default to
  the UI, and add a switch command.
- Behaviour: applications assigned to a LinX channel stay on it; everything
  else follows the new default, as LinX already does for desktop output changes.
  LinX stores nothing itself, so the desktop's own volume applet stays in sync.
- Later: a global shortcut to toggle between two outputs (on Wayland through
  the desktop's shortcut portal).
- Open question: which output a user means by "BEACN" while LinX owns the
  Studio (the Studio's own desktop devices are off then; LinX's channels
  replace them).

## Windows version

A Windows build of LinX as an alternative to the official Link app on PC2.
The GPUI interface already supports Windows; the work is elsewhere:

- Per-application routing to the Link Out endpoints through Windows audio
  policy (the official app uses the undocumented `AudioPolicyConfig` interface).
- Access to the Studio's Link control interface, which BEACN's Windows driver
  owns.
- Research item: needs a feasibility check before any implementation.

## Done

- **Linux on both PCs** (2026-10-18): LinX on PC2 tested against
  [beacn-utility](https://github.com/beacn-on-linux/beacn-utility) and
  Pipeweaver on a Debian 13 PC1. Audio on all four channels, application
  names, assignment from either side and Mix Create work; the one protocol
  difference found (a final record without a terminator) is handled since
  0.1.6.

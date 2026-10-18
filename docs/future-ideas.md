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

## Linux on both PCs

Test LinX (PC2) against [beacn-utility](https://github.com/beacn-on-linux/beacn-utility)
on a Linux streaming PC (PC1), whose Studio Link page lists applications on the
other PC and sets their channels. Compare protocol behaviour with the utility's
implementation and record any differences.

# Future ideas

Ideas that are wanted but not scheduled. Nothing here is implemented.

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
- **Output device switcher**: a speaker button in the Console header and a
  *Sound output* tray submenu choose between BEACN Studio (LinX, every
  assigned application on its channel) and the desktop's own devices
  (everything plays there). Available while LinX is connected. Still open: a
  global shortcut to toggle, and keeping a device choice across a LinX
  restart.

# LinX

### Unofficial BEACN Link app for Linux

LinX is an independent Linux desktop app that does what BEACN Link does on
Windows: it gives the gaming PC four Link Out channels on BEACN Studio, and
sends applications to them. BEACN App on the Windows streaming PC sees each
application and its channel, and can move it.

LinX is **unofficial and not affiliated with BEACN**. It is experimental
compatibility software, not a BEACN product or a replacement for BEACN App on
Windows.

![LinX native Console with Spotify on Link 2 Out](assets/screenshots/linx-console.png)

## What works today

- Four persistent channels, **LinX Link Out** and **LinX Link 2–4 Out** (the
  names BEACN's Windows driver uses on the gaming PC; BEACN App on the
  streaming PC shows them as Link In … Link 4 In), bridged to the Studio's Link
  In pairs at 96 kHz. They also appear in your desktop sound settings.
- Four return inputs, **LinX Link In** and **LinX Link 2–4 In**, carrying
  what BEACN App sends to this PC (for example your microphone), for Discord
  and other applications.
- LinX owns the whole Studio while it runs: it switches the desktop's own
  Studio devices off (without saving that choice) and hands the Studio back
  on Pause or Quit.
- Any number of applications, each on any channel. Moving an application
  between channels is a single redirect; nothing is torn down.
- Applications keep their channel through media pause/resume, new browser
  tabs, application restarts and PipeWire/WirePlumber restarts.
- Other applications, notifications, default-device changes and capture tools
  (such as OBS application audio) do not disturb the links.
- Application names are published to BEACN App, and assignments made there
  move the application.
- A slim IN level bar per strip for what the Studio sends back on that pair.
- Live stereo levels for every channel, plus working faders and mute that set
  what this PC sends on each Link Out. They stay in sync with the channel's
  volume in your desktop sound settings.
- One LinX per desktop session: launching it again (application menu,
  terminal, login autostart) shows the running window instead of starting a
  second copy that would compete for the Studio.
- Closing the window keeps LinX running in the system tray (or minimized,
  on desktops without a tray). Optional launch at login starts it hidden and
  connected. Pause or Quit returns every application to the
  output it used before.
- A sound output switcher (speaker button in the header, *Sound output* in
  the tray): *BEACN Studio (LinX)* plays each assigned application on its
  Link channel and the rest on Link Out; a device on this PC (Speakers,
  HDMI ...) plays everything, assigned applications included, and LinX keeps
  their channels for when you switch back.
- Light and dark themes, with five accent palettes.

Tested with BEACN Studio and BEACN App on Debian 13 (PipeWire 1.4.2):
audio on all four channels, assignment from either side, media pause,
faders and mute, and Studio unplug/replug. See
[how LinX works](docs/link-console.md) for details and what is not yet
covered.

## Current limits

Faders, mute and meters act on what this PC sends. LinX does not control
the streaming PC's mix.
Installable Linux packages are available from
[Releases](https://github.com/joenilan/beacn-link-linux/releases).
There is no Windows version.
Tested on Debian 13 (PipeWire 1.4.2) and Fedora 44 (PipeWire 1.6.2,
WirePlumber 0.5.13, split BEACN UCM profile). If LinX ends without Pause or
Quit (a crash), the Studio's desktop profile stays off until the next login
or replug. After a WirePlumber restart while LinX runs, the desktop's own
Studio devices return only after a replug or new login.

## Install

The easiest way is LinX's package repository: your system's normal updates
(`dnf upgrade`, `apt upgrade`, Discover) then keep LinX current. Packages and
repositories are signed with the LinX key, fingerprint
`E72D 6EE9 C417 012E CE28  0124 E14B 8780 7621 D25D`.

Fedora:

```sh
sudo dnf config-manager addrepo --from-repofile=https://joenilan.github.io/beacn-link-linux/linx.repo
sudo dnf install linx
```

Debian 13 / Ubuntu:

```sh
sudo curl -fsSLo /usr/share/keyrings/linx-archive-keyring.gpg https://joenilan.github.io/beacn-link-linux/linx.gpg
sudo curl -fsSLo /etc/apt/sources.list.d/linx.sources https://joenilan.github.io/beacn-link-linux/linx.sources
sudo apt update
sudo apt install linx
```

Arch Linux (from the combined [joenilan apps](https://joenilan.github.io/)
repository, signed with the same key):

```sh
curl -fsSL https://joenilan.github.io/packages/joenilan-apps.asc | sudo pacman-key --add -
sudo pacman-key --lsign-key E72D6EE9C417012ECE280124E14B87807621D25D
curl -fsSL https://joenilan.github.io/packages/joenilan-apps.pacman.conf | sudo tee -a /etc/pacman.conf
sudo pacman -Sy linx
```

Or download a package from [Releases](https://github.com/joenilan/beacn-link-linux/releases)
(x86_64). Each one installs `linx`, its application-menu entry and icon, and
the USB rule that lets your desktop account reach the Studio's Link control
interface; replug the Studio once after installing. On systemd 257.7 or newer
that rule is redundant (systemd's own hwdb already grants access to
`33ae:4003`); it is kept for older systems.

- Fedora: `sudo dnf install ./linx-<version>-1.fc44.x86_64.rpm`
- Debian 13 / Ubuntu: `sudo apt install ./linx_<version>-1_amd64.deb`
- Arch Linux: `sudo pacman -U ./linx-<version>-1-x86_64.pkg.tar.zst`
- Other distributions: extract `linx-<version>-x86_64-linux.tar.gz` and run
  `./install.sh` (for your user) or `./install.sh --system`; `./uninstall.sh`
  removes it.

Requirements: PipeWire 1.0 or newer with WirePlumber, BEACN Studio connected
to this PC, and BEACN App on the Windows streaming PC. Start LinX from the
application menu; Settings → *Launch at login* starts it in the tray.

The optional [96 kHz WirePlumber rule](integrations/wireplumber/README.md)
and its guide are included under `/usr/share/doc/linx/wireplumber` in RPM and
Debian packages, and under `share/doc/linx/wireplumber` in a tarball install.
The rule is not enabled automatically. Follow the guide to install it manually
and restart WirePlumber when you choose to use it.

Updates: from 0.1.4, LinX checks these GitHub releases shortly after it starts
and every six hours (Settings → *Check for updates automatically*; turn it off
to check only with *Check now*). A newer version shows an **Update** button in
the header. Nothing downloads or installs without a click: *Download* fetches
the package for your install (RPM, Deb, or the per-user tarball) and checks it
against the release's `SHA256SUMS`; *Install* runs `dnf`/`apt-get` through
your system's password prompt (the per-user tarball needs none); *Restart LinX*
starts the new version. `/usr/local` installs and builds from source only
check and link to the release page.

## Build from source

Use a Linux desktop with
PipeWire and WirePlumber, connect BEACN Studio to the Linux PC, and run
BEACN App on the Windows streaming PC. The GPUI app uses the Rust 1.99.0
toolchain pinned in `app-gpui/rust-toolchain.toml`. The native build needs a
C compiler, `pkg-config`, PipeWire/SPA development headers 1.0 or newer
(`libpipewire-0.3-dev` on Debian/Ubuntu, `pipewire-devel` on Fedora) and
libusb 1.0.27 or newer (`libusb-1.0-0-dev` / `libusb1-devel`).

```sh
cd app-gpui
cargo run --features studio -- --studio
```

Click a channel's application box, then pick 1–4 next to an application.
**Off** returns it to its previous output. You can also choose
"LinX Link Out" or "LinX Link N Out" as an application's output in your desktop sound settings.
Audio works as is. For the BEACN App connection (names and remote
assignment), your desktop account needs access to the Studio's USB device; if
LinX reports the BEACN App link as unavailable, install
[`integrations/udev/70-linx-beacn-studio.rules`](integrations/udev/70-linx-beacn-studio.rules)
to `/etc/udev/rules.d/` and replug the Studio.

To add LinX to your application menu, copy
[`integrations/desktop/io.github.linx.desktop`](integrations/desktop/io.github.linx.desktop)
to `~/.local/share/applications/` (adjust `Exec` to where you put the `linx`
binary) and `io.github.linx.svg` to `~/.local/share/icons/hicolor/scalable/apps/`.

LinX shows its status in the system tray. KDE Plasma, Cinnamon, Xfce and
Ubuntu's GNOME have one; on stock GNOME (Debian, Fedora Workstation) install
the AppIndicator extension (`gnome-shell-extension-appindicator`), then log
out and back in. Without a tray, closing the window minimizes LinX instead.

## Screenshots

The screenshot is taken from LinX's own native window and framed with the
standalone [screenshot helper](scripts/frame_screenshot.py). See
[the screenshot guide](docs/screenshots.md) for capture and review steps.

## Project status and license

Copyright © 2026 LinX contributors.

LinX is licensed under [GNU GPL version 3 only](LICENSE). Third-party
components retain their respective copyright and license notices.

LinX is an independent working name, not a trademark-clearance result. BEACN
and BEACN Link are referenced only to identify the compatibility target.

## Further reading

- [How LinX works](docs/link-console.md)
- [96 kHz fix for the normal Studio Link profile](integrations/wireplumber/README.md)
- [Contributing](CONTRIBUTING.md)
- [Developer control socket](docs/native-control.md)
- [Screenshots](docs/screenshots.md)
- [Release preparation](docs/releasing.md)

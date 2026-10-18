# BEACN Studio USB2 Link rate rule

This optional WirePlumber rule pins the BEACN Studio USB2 Link card's hidden
split PCM parent nodes to 96 kHz. It does not change PipeWire's graph clock or
other audio devices, and it does not require LinX. It targets the normal
`BEACN Studio Link` UCM profile, which exposes four playback sinks and four
capture sources.

LinX currently uses its own Studio transports while running; this rule is for
ordinary Studio routing through the desktop profile and is not required by
that LinX backend.

## Why the hidden parent matters

WirePlumber creates a shared hardware node beneath the four stereo endpoints
in each direction. The rule matches that node using `api.alsa.split.parent`,
so the physical device opens at 96 kHz. Matching only the visible stereo
endpoints does not reliably constrain the hardware node's rate. PipeWire can
resample the desktop streams to the hardware rate.

On the tested system, without the rule, a 48-kHz stream opened the normal
profile's playback hardware at 48 kHz, but its hardware pointer remained at
zero and a finite stream did not complete. With the rule, the same finite
stream completed and the pointer advanced at the 96-kHz hardware rate.

## Install

The rule is optional. It is shipped as documentation and is not enabled by
installing a LinX package. To enable it, copy it into the per-user
WirePlumber configuration directory. Review any existing file with the same
name before replacing it.

From the repository checkout root:

```sh
mkdir -p ~/.config/wireplumber/wireplumber.conf.d
cp -i integrations/wireplumber/51-beacn-studio-link-rate.conf \
  ~/.config/wireplumber/wireplumber.conf.d/
```

For an RPM or Debian installation, use the packaged copy:

```sh
mkdir -p ~/.config/wireplumber/wireplumber.conf.d
cp -i /usr/share/doc/linx/wireplumber/51-beacn-studio-link-rate.conf \
  ~/.config/wireplumber/wireplumber.conf.d/
```

For a per-user tarball installation, use
`~/.local/share/doc/linx/wireplumber/51-beacn-studio-link-rate.conf`; for a
system tarball installation, use
`/usr/local/share/doc/linx/wireplumber/51-beacn-studio-link-rate.conf`.

Pause or quit LinX before restarting WirePlumber. Restarting WirePlumber
interrupts audio and recreates the Studio nodes:

```sh
systemctl --user restart wireplumber
```

## Remove

Remove the installed rule and restart WirePlumber again:

```sh
rm ~/.config/wireplumber/wireplumber.conf.d/51-beacn-studio-link-rate.conf
systemctl --user restart wireplumber
```

## Verification and limits

Verified on Fedora 44 with ALSA lib 1.2.16.1, PipeWire 1.6.9 and WirePlumber
0.5.18. With the normal Link profile active, the card retained its four sinks
and four sources. The two hidden split parents reported 96 kHz with only 96 kHz
allowed. PipeWire's configured default and allowed graph rates remained
48 kHz, with `clock.force-rate` at zero; the Studio hardware ran at 96 kHz.

Four finite 3-second, 48-kHz stereo playback streams completed successfully.
A separate attended test sent one, two, three and four quiet beeps to the
four sinks; the owner heard the corresponding counts on Windows Link In
channels 1–4. Four native capture streams each
delivered 96,000 frames to a 48-kHz client. Return channels 1 and 2 showed
about −30 dBFS RMS; channels 3 and 4 remained silent, consistent with the
existing Windows routing. Return audio was assessed using level statistics;
no microphone sample files were saved.

After unplugging and reconnecting the Linux-side USB cable, the normal Link
profile returned automatically with all four sinks and sources. Both hidden
parents retained 96 kHz, and all four finite playback tests passed again
without restarting WirePlumber. LinX was paused during these native-profile
tests and restored afterward.

Other distributions and older PipeWire/WirePlumber releases have not been
verified. The rule relies on
PipeWire's native ALSA split PCM parent support and WirePlumber's handling of
those parent nodes. The relevant upstream support is present in PipeWire 1.4.2
and WirePlumber 0.5.8 sources, but this rule has only been tested with the newer
versions listed above. The card name match is exact; check the device's
`api.alsa.card.name` if your system uses a different name.

The rate setting addresses the observed mismatch between the Studio hardware
transport and PipeWire's default device rate. It does not establish the cause
of the device's advertised rate or prove that every BEACN Studio setup needs
this override. It leaves the system-wide PipeWire clock at its existing rate.

References: [WirePlumber ALSA configuration](https://pipewire.pages.freedesktop.org/wireplumber/daemon/configuration/alsa.html),
[PipeWire audio properties](https://pipewire.pages.freedesktop.org/pipewire/page_man_pipewire-props_7.html).

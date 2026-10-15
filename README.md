# LinX

### Unofficial BEACN Link app for Linux

LinX is an independent Linux desktop app for routing a live stereo application
through BEACN Studio to one of four Link In channels. BEACN App remains on the
Windows streaming PC, where you can see the incoming application and its
assignment.

LinX is **unofficial and not affiliated with BEACN**. It is experimental
compatibility software, not a BEACN product or a replacement for BEACN App on
Windows.

![LinX native Console with a live application assigned to Link In 4](assets/screenshots/linx-console.png)

Actual native app capture. The selected application's stereo levels appear on
its assigned pair; faders and mute buttons remain disabled.

<details>
<summary>Light theme</summary>

![LinX native Console with Firefox levels in the light theme](assets/screenshots/linx-console-light.png)

</details>

## What works today

- Native Rust and GPUI desktop application with PipeWire application discovery.
- One selected live stereo application at a time.
- Local assignment to Link In 1, 2, 3, or 4, verified by the native route owner.
- Live stereo RMS bars with peak and hold markers for the selected application.
- Pause and Quit release LinX-owned audio and device resources.
- Media Pause remembers the channel and waits for the same application's audio.
- Light and dark themes, with five accent palettes.

The owner confirmed that all four assignments appeared in BEACN App with
incoming meter activity during a native run. Direct listening acceptance for
the current app build is still pending. See the [current handoff](docs/HANDOFF.md)
for exact runtime evidence and its limits.
An earlier twenty-minute silent physical-path run also passed repeated pair switching
and exact desktop/saved-state cleanup.

## Current limits

LinX currently routes one stereo application at a time. Multi-application
desktop routing, Linux gain and mute controls, Mini mode, remote volume
controls, installers, and launch-at-login support are not implemented. Faders
and mute buttons are disabled. Meters show local digital levels for the one
selected application and are not calibrated analog VU meters or remote Windows
mix levels; see [native meter behavior and evidence](docs/native-meters.md).
Assignment continuity after application restart and broader hotplug recovery
still need work.
Pausing media shows **Waiting for audio** and releases the active route. Playback
can resume automatically on the remembered channel while the same PipeWire client
remains. Restarted clients, ambiguous streams and unverified cleanup require
attention; this is not general application-restart recovery.
The current native backend requires a scoped WirePlumber 0.5.8 integration to
keep temporary LinX routes out of saved application targets. Other WirePlumber
versions are not supported yet. This is not yet ready for regular use.

## Try the native Studio mode

This is a development build path, not an installer. Use a Linux desktop with
PipeWire/WirePlumber, connect BEACN Studio to the Linux PC, and run BEACN App on
the Windows streaming PC. The GPUI app uses the Rust 1.99.0 toolchain pinned in
`app-gpui/rust-toolchain.toml`. The native Linux build needs PipeWire/SPA
development headers 1.0 or newer and libusb 1.0.27 or newer; see the
[PipeWire build notes](docs/native-executor.md) and
[USB requirements](docs/native-usb-handshake.md).

Before enabling Studio mode, install and verify the
[WirePlumber target-cache guard](docs/wireplumber-integration.md). Follow its
stock-source hash check and per-user installation steps at a safe point with
LinX paused. LinX refuses desktop routing when that guard is missing.

From the repository root:

```sh
cd app-gpui
cargo run --features studio -- --studio
```

Start audio in an application, click a channel's **Choose application** box, and
use **Assign to Link In** (or Link 2/3/4 In) beside that application. LinX connects
and fills the well after verifying the route. Pause or quit to release the session.
Launching without `--studio` stays in the
read-only discovery mode. See [native desktop session details](docs/native-desktop-session.md),
the [optional developer control interface](docs/native-control.md), and the
[GPUI interface notes](docs/gpui-interface.md).

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

- [Current handoff and runtime evidence](docs/HANDOFF.md)
- [Contributing](CONTRIBUTING.md)
- [Release preparation](docs/releasing.md)
- [Native GPUI interface](docs/gpui-interface.md)
- [Native desktop session](docs/native-desktop-session.md)
- [Native meter behavior and evidence](docs/native-meters.md)
- [Protocol and compatibility research](docs/protocol.md)
- [Phased research plan](docs/research-plan.md)

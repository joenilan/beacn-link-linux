# LinX visual study 02

Open [the comparison](index.html) in the existing localhost preview at
`/directions/`. Direct starting views are `?view=console`, `?view=matrix` and
`?view=mini`; the in-page buttons preserve the same demo gain/mute settings.
`#research` jumps to the illustrated reference board. This remains a browser
design proposal for the existing Rust/GPUI Kit app, not a framework change.

The owner approved the three mockups and chose **01 Console as the default**.
All layouts now support Light/Dark plus Mint, Blue, Violet, Amber and Rose accent
palettes. Direct links can specify `?view=console&theme=light&accent=violet`.
Appearance changes retain the current demo gains/mutes; no preferences are saved.
Status colors remain independent of the palette. Light themes use darker accent
colors for contrast, and the dark source inspector uses its matching light accent.

The owner rejected study 01 because it largely repeated the existing cards/list
layout. That first pass read official text sources and produced a layout
refinement; describing it as a visual design study overstated its coverage.
This revision inspects actual published interface images and makes the resulting
layout choices visible and reviewable. Study 01 remains for provenance.

## What was actually inspected

Reviewed 2026-10-15. These are public official promotional/support images, not
installed applications, live usability tests or proof of protocol capabilities.
Full-resolution images, fetched HTML, source URLs, SHA256 manifests and browser
review evidence remain in ignored private `local/` directories. No proprietary
screenshots or captured binaries are committed.

| Interface / primary source | Visual observations | Decision for LinX |
| --- | --- | --- |
| [BEACN App, published 1.4 visuals](https://www.beacn.com/en-us/pages/beacn-app-mix-control-your-beacn-ecosystem) | Side-by-side personal/audience faders, narrow channel color bars, meter/fader grouping and applications below each strip. Teal outlines and a dense routing table. | Keep each channel's apps and local send controls together. Use four lean strips; do not mimic supported dual-mix controls before they exist. |
| [Wave Link 3, Windows and macOS images](https://www.elgato.com/uk/en/explorer/products/wave/wave-link-30-redefining-creator-audio-control/) | Source rows, mix columns and small send controls at intersections. Windows is neutral dark; the macOS image has a dark translucent shell. | Make a distinct source/destination matrix alternative. LinX cells inspect observed assignments rather than pretend a local routing command works. The chalk/ink palette is our own choice, not a claim that Wave Link is light-themed. |
| [SteelSeries Sonar mixer promotional image](https://steelseries.com/lp/sonar-for-streamers) | Tall category faders, separate category colors, apps beneath and a prominent suite sidebar. The source image includes a video play overlay. | Keep glanceable category identity and adjacent mute. Omit the suite sidebar, effects pages and marketing navigation. |
| [Universal Audio Console promotional image](https://www.uaudio.com/products/uad-console-app) | Repeated console strips, fader caps, calibrated scales, narrow meters, aligned labels and extensive insert/monitor controls. | Borrow instrument-like fader geometry and numerical precision. Omit plugin racks and dense recording-console controls. |

The source images are individually enlarged from the research board. The cached
copies are private evidence for this workspace and are not a distributed asset
pack. A fresh clone needs its own reference images or can follow the official
source links; the three proposed interfaces themselves need no downloaded asset.
The existing LinX SVG mark is reused from `app-core/src/icon.rs` geometry.

### Exact image provenance

| Inspected image | SHA256 |
| --- | --- |
| [BEACN mixing suite](https://www.beacn.com/cdn/shop/files/beacn-app-mixing-suite-1-4.jpg?v=1781644284&width=1800) | `6cf3b47c8a5c0b2d07ce279992e4e7f18d32f6b3bfc1d5a04a90288a4df5349b` |
| [Wave Link Windows](https://d32bbxx72p2025.cloudfront.net/images/Elgato.WaveLink_EcvK5AjFqM.width-702.format-webp.webp) | `8b46e1ac73d791f20bd8d7faba2d2a4f93eb80ac914389715baef52404dea908` |
| [Wave Link macOS](https://d32bbxx72p2025.cloudfront.net/images/Wave_Link_3.0_Mac_Liquid_Glass.width-702.format-webp.webp) | `3bdfe818f3d99c287f243f4da128eaaf91bbea32a40fa3133a06b0a70a6b5b81` |
| [Sonar mixer](https://media.steelseriescdn.com/filer_public/16/22/1622a887-4b16-4bf2-839f-746308cf2145/sonarforstreamers_mixer_2023.png) | `b1af2fcde91c0750fb477af6a12791f3e42ad8acb5dc841eee11b44a54c2d144` |
| [UAD Console](https://www.uaudio.com/cdn/shop/files/uad_console_gallery_1.png?v=1742249572&width=1600) | `8487c2855fa18f758437a0620dca151ab65bc87658748a9009dbda05cb5323b7` |

## Three materially different proposals

**01 Console — default.** Four contiguous vertical strips replace the channel
cards plus separate application list. Each strip owns a calibrated fader, stereo
meter, dB readout, mute and application well. The neutral instrument surface uses
small category accents, restrained fader bevels and a compact connection header.
At full size the proposed frame is about 788 by 568 pixels with four apps.

**02 Signal map.** A nearly flat table replaces the
console. Sources form rows, four Link channels form columns and one dark source
inspector holds the selected app's gain/mute. This emphasizes routing clarity
without duplicating four sets of faders. Cells are observations, not route edits.
At full size the frame is about 890 by 572 pixels.

**03 Mini.** A small 405 by 334-pixel utility uses one short source
row per app, horizontal gain, inline mute and an Open console action. It has no
channel-card row, settings sidebar or duplicate application catalog. This is a
compact companion presentation, not another feature screen users must visit.

Console is the chosen default; Mini provides the compact presentation. The matrix
is an alternative information architecture, not a mandatory extra screen. Theme
and accent choices are shared, so layout does not force a different appearance.
Responsive layouts reflow naturally; no automatic resize policy is specified for
the native window. Settings only expand when requested. This visual approval does
not establish or enable any new backend capability.

## Capability and interaction boundary

The examples use one fictional app per channel so controls have a clear scope.
The dB faders and mute modify the same local presentation model in all views.
Multiple applications per channel need the separate gain/grouping ownership
design already described in [study 01's implementation plan](../README.md).
Meters are static illustrations which respond to demo gain/mute. They are not
calibrated signal measurements and are never animated as if live audio existed.

All gains are proposed **Linux sends before Studio**. They are not Windows-side
personal/audience mix faders. The UI does not invent those commands or expose
working remote controls. The matrix allows source inspection only. App and route
examples are not discovered hardware state. The six connection scenarios are
presentation fixtures; the current Rust startup/session owners remain authority.

No audio/USB access, processes, network requests, timers or persistence occur in
the mockup script. Reference images load from the existing localhost server;
official source links open only when clicked. No production Rust/UI/backends,
dependencies, lockfiles, laptop settings or hardware-test artifacts changed.

## Verification

Isolated headless Chrome rendered all three layouts across six states at widths
320, 480, 900 and 1152 pixels without horizontal overflow (72 combinations).
Vertical/horizontal native slider keyboard adjustment, gain readouts, mute,
held Pause intent, private-mode resume, source inspection, unchanged assignment
cells and Mini-to-Console expansion passed. All four research images loaded;
no console errors, external requests or local-storage entries were observed.
Thirty theme/palette/layout combinations also rendered, plus narrow views in both
themes. Theme changes preserve gains; status icon color remains unchanged across
all five palettes. Direct theme links load the requested appearance. Desktop and
light/dark screenshots were visually inspected.
Desktop, narrow and research-board screenshots are retained privately under
`local/directions-review-*`. This proves browser presentation only, not GPUI
rendering/performance, accessibility certification or physical audio behavior.

Next implementation gate remains the prepared attended native playback check.
Visual migration should then reuse existing snapshots, worker, icon and lifecycle
owners without enabling simulated production controls or widening routing gates.

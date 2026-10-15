# Native playback owner

Updated 2026-10-15 UTC. Virtual endpoints pass the unattended checks below. A
subsequent attended [startup correction and physical first-pair check](native-playback-startup.md)
now pass with owner-heard mpv output and exact immediate/late laptop comparisons.
Physical pairs 2–4, Pause/reconnect and ordinary desktop integration remain pending.
The earlier unattended evidence below involved no USB I/O or Studio PCM open.

## Ownership and integration

Root feature `native-playback` adds `NativePlayback`, reusing the authenticated
private-core connection, native creation ledger, route publication proof and
receipt owners. It launches no playback subprocess/pw-cli, claims no USB,
captures no audio and changes no desktop defaults or drivers.

The C adapter creates one non-lingering eight-channel sink, configures DSP input
ports and creates two monitor links. Requested physical format is FLOAT_LE,
eight channels, 96000 Hz. Pair 1 maps FL/FR to AUX0/1, pair 2 to AUX2/3, pair 3
to AUX4/5 and pair 4 to AUX6/7. Qualification uses the same implementation with
a null sink factory; virtual success is not physical negotiation or audibility.

Start requires one selected app already routed and verified at the requested
pair. `VerifiedRoutePublication` retains its core, destinations and trusted route
receipt ledger for independent native revalidation. NativePlayback pins the
sink/link/monitor/input generations and active link states. Refresh needs a newer
fixed-roster inventory. Every failed check/refresh latches the failure, preventing
silent recovery. The owner must keep observing; this is not a separate watchdog.

The sealed physical target also requires explicit authority, selected 33AE:4003
descriptor cache, native sysfs device, one closed playback/capture PCM, exact
character-device identity and retained private-server generation. It refuses
VMware. Active proof reuses the existing metadata-only PCM owner/format/FD/clock
checks. The 4096-check fuse and Studio session's 40-cycle budget remain unchanged.

Creation proxies are retained before acknowledgment, including partial/uncertain
creates. Only those proxies are released; there is no registry destroy by ID,
name or tag. `SinkReceipt` is comparison data, never deletion authority.
Independent observations check exact foreign graph conservation, including
same-tag links and reused numeric IDs. A bounded metadata-only wait checks PCM
closure. Unverified cleanup blocks session restart. The ABI was checked against
installed headers and upstream [core creation API](https://docs.pipewire.org/group__pw__core.html),
[link factory](https://docs.pipewire.org/page_module_link_factory.html) and
[SPA PortConfig](https://docs.pipewire.org/group__spa__param.html); native runtime
evidence below uses installed PipeWire 1.4.2.

Core/GPUI feature `native-playback-test` adds native Studio and native synthetic
modes to the existing session adapter. Synthetic mode uses the actual native
sink, executor and worker with ScriptedPeer; it cannot open USB and stays blue.
Native Studio uses the existing selected NativeUsb owner and in-process playback.
The historical supervisor bridge remains separately available for reproducing
accepted earlier evidence. Normal startup still has no physical/desktop factory.

Pause, lifecycle/operation failure, budget completion and Quit release playback
before routes and USB. `LinkSession::abort` preserves playback errors/attention.
In native Studio qualification, supervisor EOF/input/error requests orderly worker
cleanup through a retained input FD; no playback command/ack protocol is needed.
Its socket/pipe behavior has a focused unit check.

The physical entry is `studio_probe --authorize-bounded-native-studio-gui ...`,
with the explicit feature and existing supervisor/core/device/cache arguments.
Harness flag `--native-studio-playback` selects it. Python then only observes
process identity, exact native node/link/port receipts, channel mapping, PCM
identity/format/clock and full pre/post settings/state/binding comparisons. It
never starts the old physical bridge in this mode. Witness parser/graph/sequence
tests pass, but its physical path has not run. This is not an installed app.

## Evidence

Exact native-qualified source `beacn-native-source-20261015T035526Z`.

- Native run `beacn-native-playback-20261015T035622Z`: four virtual pairs, 128
  refreshes over 64.417 seconds on pair 1, eight refreshes on each other pair,
  stale-proof rejection followed by latched failure, four real native creation
  failure boundaries, Drop release and source exit. Complete-result SHA256
  `20912f49c3b71213a946f678b7cc3d57651c1435390892a452b8ad04e0bfa4af`.
- GUI run `beacn-native-playback-gui-20261015T035801Z`: real mpv selected through
  actual widgets, scripted assignment 1, native sink, Pause held with automatic
  enabled, explicit reconnect and Quit while running. Complete-result SHA256
  `e3cdf1376a28cdfa093929e5090c61d4fbc3139c69fd551b68f0f384c0ece923`.
- Both restore original private core, full laptop desktop/settings/routes/defaults/
  volumes/mutes/state, owned configs/processes and connected USB binding. Running
  and paused screenshots were inspected: private blue and paused gray.
- 71 Linux root, 17 changed-behavior Windows, 39 Linux Python, twelve core and
  eight reference-GUI checks pass. The latter two use identical core source;
  the subsequent final change is confined to native-owner latching/probe assertion.
- Native root/GPUI/Studio builds pass. Default production preview was built from
  `beacn-native-source-20261015T035131Z`: only its feature-excluded native module
  and root qualification example changed afterward. Its mapped window, read-only
  catalog, gray icon, normal close and exact desktop/state/config/USB comparison
  pass in `beacn-gui-desktop-20261015T035900Z`, journal SHA256
  `cf943335680de11c70e6ea96ccfa3a9b2d69a17cf147ae7fb72cd0060f562123`.

Failed setups remain private: inherited prohibited PipeWire overrides, absent
virtual monitor ports (refused before creation), and a synthetic peer lacking the
existing initial-read diagnostic policy. Clean explicit environment, owned virtual
monitor ports and matching publication policy corrected them. No gate was relaxed
and no failed evidence was rewritten. Qualification executables are frozen before
launch so builds cannot replace a running artifact.

CLI reproduction needs root features `native-playback,native-test-faults,qualification-peer`
and catalog-service `--playback-owner-check --virtual-monitors` in the existing
authorized no-USB routing-probe scope. The GUI example requires
`native-playback-test`, owned private display/core and both qualification markers.
All mutations still require fresh pre/post snapshots and owned cleanup.

## Next acceptance boundary

The first native-owner physical check now passes as recorded in
[native-playback-startup.md](native-playback-startup.md). Further watched checks
still need preflight, exact Applications/all Link In instructions and a fresh
**start now**, followed by immediate execution. Physical pause/failure/reconnect,
extended operation, ordinary desktop routing, multi-app hardware and installation
remain unaccepted. Streaming-PC saved settings have not been compared. Do not
leave a test running overnight.

The laptop fell below 25 GB free during builds; owner notified. Cached targets were
reused and no source, lockfile, user data, running artifact or earlier evidence was
deleted. Check disk before further heavy builds. A narrow security/dependency
review is recommended before release/default physical enablement; no paid scan
was requested or run.

# Selected Studio playback probe

Updated 2026-10-13 (private evidence timestamps extend into 2026-10-14 UTC).
This is an explicitly authorized physical feasibility test, separate from the
[native private-core executor](native-executor.md) and
[accepted assignment/publication checkpoint](native-assignment-publication.md).
Ordinary desktop apply remains disabled. No audible acceptance is claimed.

## Implementation and boundaries

`tests_native/studio_playback.py` uses one finite, retained `pw-cat` process on
the desktop core. It creates no virtual sink, changes no defaults or volumes,
and never destroys a link or releases another process's USB claim. Cleanup
terminates only its own playback process; the session manager removes that
process's links. This is tool-mediated native PCM playback, not physical
application by the Rust/C private-core executor or a native creation receipt.

The existing inventory/graph owners project the complete graph, including
`Stream/Output/Audio/Internal` and `Stream/Input/Audio/Internal` channel splitters.
These internal nodes are foreign graph resources, not published applications.
The existing codec/planner must separately recognize the observed stereo route
as zero further operations. That read-only check does not apply a route.

Before playback, select exactly one 33AE:4003 device, correlate its ALSA card's
sysfs ancestor to that device, and pin the current device/node/port serials.
Selection uses the actual UCM split positions, not descriptions or profile order:
Link 1/2/3/4 use AUX0/1, AUX2/3, AUX4/5, AUX6/7 respectively. Verify each FL/FR
input, its paired internal output, and the existing links to the eight-channel
hardware sink. Four fresh pairs passed this read-only check in the VM.

The stream targets the selected object serial, forbids fallback, reconnect,
movement and remixing, disables its saved volume/target state, and uses a private
minimal client configuration. Each observation verifies retained process identity,
unique stream identity, unchanged physical references/core, exactly two correct
FL/FR links, and preservation of every baseline graph resource/link. The ALSA
hardware pointer must continue advancing; a three-second stall stops the test.

PCM is authored locally: 48 kHz stereo s16le, twelve seconds total. Silence is
the default. Explicit `--audible` adds three half-second 440 Hz beeps at -24 dBFS,
after eight silent seconds, with short amplitude ramps. Only start audible mode
after silent preflight succeeds and the owner is ready to watch the first Link In
meter and listen on their usual monitor. No application dragging is needed.
No incoming audio or USB traffic is captured.

Full connected settings/binding/state-file backups precede each test; afterward
compare routes/defaults/formats/volumes/mutes and all four WirePlumber files.
Exact differences remain recorded. One narrowly classified exception reports
an initially empty negotiated `Format` cached on the pinned internal splitter;
it does not ignore any other node, format, identity, property or saved-state change.
Do not reset that foreign node merely to erase its runtime format cache.

## Handshake-only diagnostics and coexistence

`Handshake::prepare_initialization_diagnostic` and
`native_usb::open_handshake_initialization_diagnostic` reuse existing owners and
allow one initial A2 read up to five seconds. Default handshake limits remain
500 ms. Writes and all other reads retain 500 ms limits; invalid/partial/unknown
I/O halts without retry. A valid A2 remains mandatory. These APIs grant no
publication or polling capability.

The native handshake example explicitly supports:

- `--authorize-handshake`: existing default preparation and deadline.
- `--authorize-handshake-initialization-diagnostic`: existing observed startup,
  five-second initial read, then immediate explicit cleanup.
- `--authorize-handshake-held-diagnostic`: the same valid initialization, an
  announced thirty-second retained vendor claim, then explicit cleanup. No list
  publication or polling occurs during the hold.

For the held diagnostic, the physical runner accepts an explicit caller-owned
helper PID and start time. It checks the actual opened selected USB-device
identity, fresh cached descriptors and expected interface profile. It stops only
its own PCM process and compares against that held baseline; it never stops or
releases the helper. The outer owner must subsequently confirm the helper's exit,
native cleanup and the complete released baseline. Numeric IDs/PIDs or tags alone
are not cleanup authority.

## Observed results

Windows and Linux pass 15 focused Python cases, 13 handshake-gate cases, 20
publication-gate cases and two session initialization cases. Default and Linux
native-USB handshake builds pass. Exact tested source is
`beacn-link-physical-playback-20261014T032358Z/source` in the existing guest home.
No packages or toolchains were installed. Latest free guest disk is about 18.4 GB;
continue lightweight single-job builds only.

The first UHCI preflight linked correctly but exceeded the playback deadline.
A verbose second probe showed only about three seconds of hardware progress over
seventeen seconds, then a frozen hardware pointer. No EOF/drain event occurred.
The original USB descriptor reported 480 Mb/s despite attachment through UHCI.

The selected VM was shut down gracefully through `vmrun`; configurations and
settings were backed up before changing it. Enabling EHCI initially exposed the
missing PCI bridge and occupied primary slots. The final selected-VM configuration
adds EHCI and PCI bridge 0, moves only the SATA CD-ROM controller to the bridge,
and leaves the main SCSI disk's slot unchanged. Linux now observes `ehci-pci` for
Studio at 480 Mb/s. The owner reconnected Studio once after this restart.
[VMware's troubleshooting guidance](https://knowledge.broadcom.com/external/article/307966)
requires USB 2.0 emulation for USB 2.0 devices; this change did not establish a cure.

After restart/reconnect, logical guest node settings, channel ports, routes and
defaults match the pre-restart baseline. Saved values match; `stream-properties`
has different JSON member order, so its byte hash differs. Across the full host
restart interval, one existing session changed inactive to active and sixteen
16-byte registry properties changed. Host audio settings otherwise match.
These differences are preserved, not overwritten or reported as an exact match.
Each subsequent bounded attempt's own connected host comparison passes exactly.

EHCI playback still stalls. A plain 500 ms handshake stopped at A2 with zero
bytes and released cleanly, before any PCM. The new explicit startup/read
diagnostic succeeded, but subsequent PCM still stalled. Keeping the initialized
vendor claim open also did not cure it: A2 completed with a valid 512-byte reply
in 931 ms, then ten verified route observations saw the hardware pointer remain
zero. ALSA reports eight FLOAT_LE channels at 48 kHz, playback Running, and capture
PCM closed. Those status observations do not prove a cause or justify starting
a capture stream.

The final held helper exited zero with confirmed claim release/baseline
restoration. The owned PCM process exited after the clock-stall stop. Final guest
desktop/state/binding and connected host audio/registry comparisons matched
exactly. No audible run was started. Studio remains attached to the VM.

Private evidence hashes:

- Tested code manifest (52 code/build-input files):
  `f1369e75c82a205514f3f6099becb2296908a000e52f01a176cb7b90602b6305`.
- Transferred source archive:
  `dcd74f9f535ce93fb33b8e7d3f1e3355db2511322443c20bf0135360ebdef8c5`.
- PCM journal: `0e231739808e70d75bade1e3ff42ee9817dc81d43be92741409387fed02a09cf`.
- PCM comparison: `7af20af4d57f5c199c2fc845b9f76c52509a3125290e356a1195ec10cda9a7d1`.
- Held native journal: `766af2d084652f9324c7280935696d613f55fc1744cbae332d44041de3929f06`.
- PCM runner: `feba85796804fccc9dd201d6495a251758ed148448251576f6c60e871ebabcf0`.

## Next

Investigate the USB audio clock/implicit-feedback path and VMware isochronous
delivery using these saved observations. USB 2.0 controller selection, a correct
graph, initialization alone and an initialized held vendor claim are insufficient
in this VM. Do not repeat owner drag/visibility tests or add speculative driver,
firmware, profile, rate or persistent configuration changes. Kernel logs are
unavailable through current unprivileged access; do not infer an absence of errors
from empty `journalctl` output or failed `dmesg` access. No REA, input-audio capture,
traffic capture, driver/firmware change or USB 3.0 migration was performed.
The first physical milestone remains a complete stable silent PCM run, followed
by an announced, owner-ready meter/listening test. Full assignment-to-physical
native integration and real-application audio remain subsequent work.

# Selected Studio playback probe

Updated 2026-10-14.
This is an explicitly authorized physical feasibility test, separate from the
[native private-core executor](native-executor.md) and
[accepted assignment/publication checkpoint](native-assignment-publication.md).
Ordinary desktop apply remains disabled. No audible acceptance is claimed.

## Native feasibility without installing Linux

The gaming PC currently has no Linux installation. The owner needs the
replacement demonstrated before migrating. VM-based software work continues;
the next proposed hardware comparison is a temporary desktop live session,
not a prerequisite to install Linux or abandon Windows. Ubuntu's official
[Try Ubuntu guide](https://ubuntu.com/desktop/docs/en/latest/tutorial/try-ubuntu-desktop/)
describes this temporary session; the matching
[26.04.1 desktop image](https://releases.ubuntu.com/26.04.1/) is available.
Creating media and temporarily rebooting the gaming PC are separate owner actions
that have not occurred during this work.

`tests_native/studio_native_preflight.py` saves a complete private baseline using
the durable shared writer, reads cached USB identity and existing graph data,
verifies one selected card and all four actual pair mappings, and reports normal
device access plus PCM-idle/isolation checks. It detects virtualization and will
not report native readiness for a VM or unknown context. No audio child, vendor
transfer, route/default/volume/mute change or dependency installation occurs.
Its readiness flag prepares a separate authorized silence check; it cannot
establish an advancing hardware clock or a finished replacement app. The existing
isolation guard can refuse a native machine containing other audio devices;
preserve that report instead of bypassing the guard or changing those devices.

`tools/package_native_preflight.py` creates an exclusive portable ZIP from eight
explicit source paths, an instruction file and a per-file SHA256 manifest. It
includes the existing guarded silence runner but no Rust/proprietary binaries,
private data or access credentials. The read-only command in the bundle is:

```sh
python3 tests_native/studio_native_preflight.py --evidence-dir "$HOME/beacn-native-preflight-$(date -u +%Y%m%dT%H%M%SZ)"
```

Private prepared ZIP label: `beacn-native-preflight-20261014T1330Z.zip`.
SHA256: `710d97e1deaeda047655ec67717e603beef2130d1d4bb1c88498ce4f240e3790`.
Windows verifies its allowlist, integrity and exact current source hashes.
Linux imports the unpacked bundle, saves a stable metadata baseline, verifies
four endpoints and returns native-readiness false for VMware as intended.
Two focused readiness/detection cases pass on both platforms. No native hardware
or new physical playback was tested. Live-session evidence must be retained
before shutting down because its home directory is temporary.

## Latest controller comparison and recovery (2026-10-14)

Workstation 26.0.1 build 25688693 ran a backed-up, selected-VM-only xHCI comparison
through CLI. The first power-on failed before Linux boot because the minimal VM
lacked a PCIe slot for xHCI. After retaining that error and configuration, one
PCIe root-port bridge was added while powered off. Boot then succeeded. Linux
confirmed the selected Studio beneath `xhci_hcd` at 480 Mb/s, unchanged cached
descriptors, released interfaces, all four actual endpoint mappings, logical
routes/defaults and exact saved-file contents. Login to the normal Ubuntu desktop
was necessary for its existing uaccess permissions; no ACL/group/driver bypass
was added and no desktop automation was used.

The existing direct ALSA silence diagnostic then lost its SSH transport with
exit 255; cached VMware logs also recorded a guest-tools heartbeat timeout.
The Windows audio/registry comparison for that attempt matches exactly, but
guest child exit, hardware-clock progress and cleanup are unconfirmed. No retry
was performed. The graceful selected-VM stop timed out. A recorded recovery
power-off was required, after which only owned controller fields were restored:
EHCI at its original slot 34; xHCI and the trial PCIe root port removed.
Unrelated current VMX fields were preserved instead of replacing the entire VMX.

After the owner's desktop login, fresh read-only recovery passes `ehci-pci`,
480 Mb/s, released USB interfaces, descriptor hash, four actual endpoints, logical
nodes/ports/links/defaults and parsed saved-file values including permissions.
The 416-byte `stream-properties` file has only reordered JSON members; the other
tracked saved files match exactly. Windows endpoint defaults, volumes, mutes,
formats and states match, while two session lists, sixteen existing 16-byte binary
registry values and one added policy entry differ across the restarts. Their
full snapshots/diffs remain private and unclassified; no blanket registry or
foreign-settings restoration was attempted.

The owner-requested direct power-on USB connection automation works on both
starts. It uses VMware's documented
[autoconnect rule](https://knowledge.broadcom.com/external/article?legacyId=1648),
constrained to VID/PID and the selected cached host port. The machine-specific
rule stays in the private VMX/backups, and is intentionally retained after the
controller trial to avoid repeated handoffs. Its scope is VM power-on/restart;
it is not a verified live detach/reconnect API. The recovered VM keeps Studio
attached and Windows Link closed. No further hardware playback followed recovery.

## Evidence durability repair

The recovery power-off exposed a harness defect: the nine newest guest attempt
artifacts were empty after restart, including baseline metadata, authored PCM and
four clock-record files. There is no final journal or comparison. Preserve the
empty files, hashes and transport-loss marker; a filename alone is not a clock
observation. Complete pre-controller settings/file backups were already copied
to Windows. Fourteen final recovery artifacts were downloaded and hash-verified
separately; recovery SHA256:
`cdbc18f84d0f8056b84f9a1a1fadec981315cd227a566b2bb9676a25ffe8e7b1`.

`tests_native/usb_handshake.py` now owns a shared exclusive binary evidence
writer. It serializes JSON before file creation, flushes and fsyncs file contents,
and on POSIX fsyncs the containing directory and its private parent so a newly
created evidence root is persisted before device execution. The handshake's
pinned descriptor cache and both playback runners' authored PCM reuse this
writer. A flush failure stops before launching the native child. Tests verify
JSON/byte round trips, refusal to overwrite existing evidence, directory flushes
and blocked execution on a flush failure. No user persistent file is rewritten.

Exact new source: `beacn-link-physical-playback-20261014T131759Z/source`.
All 37 focused Python cases pass on Linux; Windows passes 35 with two existing
Linux-only skips. The unchanged Rust input map and exact reused CLI remain
recorded with build provenance. No redundant Rust matrix or toolchain install
was run. These checks validate the writer's behavior, not crash-recovery
durability under a new forced power-off or successful audio. Free guest disk is
about 18.2 GB. Next prepare a native Linux comparison outside VMware, with fresh
native preflight/backups and intact device/route guards. The VM results do not
distinguish a Linux driver problem from virtualization or device behavior.

## Current recovery state (2026-10-14)

**Recovery now passes.** The owner completed a Studio disconnect/reconnect after
the failed rate experiment below. Before reconnect, cached enumeration confirmed
Studio absent from Linux and present on Windows; the same PipeWire process was
responsive in state S rather than D. After reconnect, the read-only comparison
passes logical nodes, ports, links and defaults, exact saved-file bytes, unchanged
cached descriptor hash and rate overrides, absence of the owned playback node,
and the released USB interface profile. No audio-service restart was required.
Windows audio state is exact, but registry state differs in seventeen values:
sixteen existing binary machine-audio values and one added user-policy string.
Those changes remain unclassified and preserved privately; do not rewrite them.
Fresh recovery observations and hashes were copied to exclusive host evidence
directories, separately from the failed journal/comparison. Studio stays attached
to Linux; no further playback or incoming capture occurred during recovery.

Existing VMware logs contain `NOT_BOUND` failures while aborting audio endpoints
02/82, including at the earlier 48 kHz stop and the recovery disconnect. These
are abort diagnostics, not proof of missing packets or a VMware root cause. No
isochronous-delivery detail appears in those logs. The subsequent direct ALSA
test below isolates selected-device playback from PipeWire rather than rerunning
the same rate workaround.

## Direct ALSA diagnostic (2026-10-14)

`tests_native/studio_alsa_playback.py` is an explicit finite silence diagnostic,
not a native route executor or audio-input capture. It correlates the selected
cached USB device to one ALSA card, pins its playback device node identity,
requires both PCM directions closed, and uses the existing graph/endpoint,
snapshot, binding, activation and comparison owners. It refuses other application
streams, physical devices and rate overrides. Baseline settings, full saved-file
backups, graph and released interfaces must remain stable before launch.

The retained `aplay` child opens only the selected raw `hw:card,0` endpoint at
48 kHz/eight FLOAT_LE channels, with ALSA resampling/channel/format/soft-volume
conversion disabled and a system config path. Four seconds of all-zero PCM is
authored privately. No graph stream, default, volume, mute or vendor command is
created or changed by this runner. Each bounded observation verifies the foreign
PipeWire graph, selected node/endpoint, ALSA owner PID and exact hardware format,
USB activation and monotonically advancing hardware pointer. A two-second stall,
format/owner change, graph change, log bound or deadline stops only the retained
child. Cleanup never kills the desktop server or deletes its links; a child that
fails to exit leaves cleanup explicitly unverified.

The real run at source
`beacn-link-physical-playback-20261014T124203Z/source` verified ten observations
with actual `aplay` PCM ownership and matching hardware format, but both first
and last hardware pointer were zero. It stopped for the two-second stall; the
owned process exited and the desktop, routes, saved files, released USB binding
and per-attempt Windows audio/registry comparisons pass exactly. The same
PipeWire server remains responsive after this run. Studio stays attached and no
reconnect is needed. This is evidence of a stall on the direct ALSA/USB/device
path, not proof of which component caused it or of audible output.

Three new direct-probe cases cover finite silent PCM, rejected format/rate/channel
conversion, foreign/stopped/missing PCM ownership and oversized metadata. Linux
passes all 24 direct/playback/graph Python cases. Windows passes 22 with two
existing Linux-only skips. No unchanged Rust build matrix was repeated; exact
Rust inputs and the reused CLI hash remain recorded with the isolated source.
Latest guest free disk is about 18.3 GB; no packages or toolchains were installed.

Harness SHA256: `3af5cf40bdd045f3f1c1f38bcfd247904e87bcef4fb110b8fd9218213c9376ed`.
`aplay` SHA256: `4436d05a3f9ce9d92432ea73dd5b6233f69d6243a6cfb3b6b1b8852622cb26c5`.
Silence SHA256: `b15b79e1f79b625b7be660d31da74943bff98b94eee89544d233aabfbb91d51a`.
Journal SHA256: `f1ac1e8a866ef2f2f0536a48e3a830485a7cfabcf1eb5b13b6c643e445e28cf6`.
Comparison SHA256: `09a3abe120d3f11bf3ecb97741aa923ca1c7a570ef8563df4af875fb8cead606`.
Eighteen private guest artifacts were downloaded and hashed; tool versions and
host snapshots are retained separately (`aplay` 1.2.15.2, Python 3.14.4,
guest kernel 7.0.0-38-generic). Next isolate VM USB controller/delivery
behavior before another physical playback attempt. Keep the failed rate journal
and the USB-cycle registry differences separate from this clean per-attempt
comparison.

## Previous activation and rate experiments

The new cached activation diagnostic verified audio interfaces 1 and 2 at
alternate 1 throughout nine stalled 48 kHz observations. OUT 02 has attributes
05; IN 82 has attributes 25 (implicit-feedback data), both with 416-byte maximum
packets and interval 1. Vendor interface 3 remained at its released baseline.
Thus a closed capture PCM does not imply that the feedback interface is inactive.
Upstream Linux starts the sync endpoint from its
[playback setup](https://github.com/torvalds/linux/blob/v7.0/sound/usb/pcm.c#L266)
and can wait for incoming feedback before submitting playback transfers in
[endpoint.c](https://github.com/torvalds/linux/blob/v7.0/sound/usb/endpoint.c#L1487).
These source observations guide a hypothesis, not proof of the installed kernel's
exact path or of missing USB packets; no traffic was captured.

The UCM maintainer's
[BEACN-specific 96 kHz workaround](https://github.com/beacn-on-linux/beacn-ucm-profiles/issues/1)
provided a concrete rate experiment. The runner now accepts explicit
`--rate 96000 --force-graph-rate`, applying
[PipeWire's active-node rate property](https://raw.githubusercontent.com/PipeWire/pipewire/1.6.2/doc/dox/config/pipewire-props.7.md)
only to its owned stream. Default rate remains 48 kHz; no persistent rate file or
global metadata key is edited. PCM duration/frequency/ramp/peak bounds are retained
at both supported rates, and actual ALSA rate must match an explicit forced rate.
The rate mode refuses other application streams, other physical audio devices or
existing rate overrides, and stops if they appear during the run.

The 96 kHz hardware experiment did **not** pass: ALSA selected eight FLOAT_LE
channels at 96 kHz, both audio interfaces activated, and the hardware pointer
advanced to 298 then froze for three seconds. The owned playback process exited,
but the post-test PipeWire query timed out. Subsequent read-only inspection found
the PipeWire server in kernel state D, waiting in `usb_kill_urb`; audio interfaces
1/2 remained at alternate 1, with ALSA PCM in SETUP. All four saved WirePlumber
files still match exactly, and the connected Windows audio/registry comparison
passes. Guest graph restoration was **unverified** at that point; the later
recovery result above is separate evidence and does not turn this attempt into
successful playback.

The owner was asked to disconnect/reconnect only Studio through VMware to clear
this USB operation, with Windows Link closed, and subsequently completed recovery.
No audio-service restart or termination, driver change or further physical attempt
was performed. The read-only recovery comparison confirms released interfaces,
absence of the owned rate-forcing node, reachable desktop, logical settings/
channels/routes/defaults and saved-file equality. Recreated object IDs and format
caches are retained in raw observations, rather than deleting foreign resources
or rewriting current saved files.

The comparison now stops after the first unavailable desktop query and explicitly
reports cleanup unverified, rather than repeatedly querying a blocked server.
New source at `beacn-link-physical-playback-20261014T040623Z/source` passes twelve
playback and nine graph Python checks on Windows/Linux. Rust inputs match the
previous built source byte for byte; its exact CLI binary is reused with recorded
build provenance. No unchanged Rust matrix was repeated. Latest free guest disk
is about 18.3 GB. The new guards/fail-fast comparison are offline-validated;
no further hardware run has tested them while the server is blocked.

Private failed rate-run source: beacn-link-physical-playback-20261014T035359Z/source.
Journal: `582c8d20129d7d24af57bc23de7a5215ea672c38bd9199ffb7b6afa96ce7c146`.
Failed comparison: `22ae30deacd6d4a6966136140ea9e520dfb6dc4fd6198140fd042fa641be90dd`.
Recovery evidence must be appended separately; do not replace this failed result.

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

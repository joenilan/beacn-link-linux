# USB descriptor preparation and VM-first hardware plan

Updated 2026-10-11. The owner chose the existing VM for first hardware testing,
then native Linux on the gaming PC. This is a test-target preference, not new
authority for USB passthrough, device I/O, captures or physical routing.
No BEACN device was attached or accessed in this implementation batch.

## What is known and what is policy

Windows inventory established VID/PID 33AE:4003 and WinUSB MI_03.
[Static tracing](protocol-session.md) established OUT 03, IN 83 and calls to
alternate settings 0 then 1. None of these establishes the actual USB endpoint
types, interface class, configuration value, packet sizes or Linux driver binding.
There is no verified hardware descriptor artifact in this repository.

The new candidate policy requires the static VID/PID, interface **number** 3,
alternate **value** 1, vendor class FF and exactly two ordinary bulk endpoints
03/83 with nonzero packet sizes <=1024 and no shared addresses on other interfaces.
Vendor class, bulk type and this exact endpoint count are conservative assumptions
to validate, not observations about BEACN hardware. A refusal means investigate
the descriptors; do not automatically choose audio interfaces or change the policy.
Packet-size bounds alone do not validate speed-specific USB rules or companions.

`src/usb_descriptors.rs` parses Linux's cached device/configuration stream.
It bounds input at 256 KiB and 4096 descriptors, walks each bLength, validates
known header lengths, counts, duplicates, alternate zero and endpoint placement,
and rejects inconsistent configuration totals. Linux documents that cached
wTotalLength may overstate available bytes, so it is never used to skip unchecked
memory or find the next configuration. Strict refusal of an inconsistency is
deliberate policy; this is not a general USB compatibility parser.
Class/IAD/companion bodies are bounded and skipped, not semantically validated.
Parsed reports retain numeric fields, not strings or raw extension bodies.

Primary references: [Linux cached-descriptor ABI](https://github.com/torvalds/linux/blob/master/Documentation/ABI/stable/sysfs-bus-usb),
[USB Chapter 9 structures](https://github.com/torvalds/linux/blob/master/include/uapi/linux/usb/ch9.h)
and [sysfs attribute formats](https://github.com/torvalds/linux/blob/master/drivers/usb/core/sysfs.c).
The formats distinguish decimal bConfigurationValue/bAlternateSetting from
hexadecimal bInterfaceNumber and class fields. Configuration values and interface
numbers must not be treated as array indexes.

## Working entry points

The ordinary CLI accepts bounded binary stdin, opens no device and prints
numeric descriptors followed by either a static candidate report or an error:

```sh
cargo build --offline --locked -j 1
target/debug/link-protocol inspect-usb-descriptors 7 < private-cached-descriptors.bin
```

Here 7 is an example observed **active configuration value**, not a BEACN claim.
CLI success is not permission or evidence that a transport can claim the device.
The fixture `fixtures/usb-composite.synthetic.hex` was generated for tests and
has a synthetic audio-class interface plus vendor alternate settings; it is not
a sanitized hardware capture. It intentionally uses configuration 7 and different
packet sizes to catch assumptions about indexes and 512-byte USB packets.

`tools/usb_descriptor_probe.py` reads one explicitly named Linux sysfs USB device,
never enumerates a fallback target and never opens `/dev/bus/usb`. It reads cached
numeric attributes, descriptors and interface driver links; it does not request
strings, read serial/manufacturer/product attributes, issue libusb calls, detach
drivers, select configurations/alternates, reset pipes or write sysfs.
It invokes the trusted built CLI with a five-second limit and samples the cache
again. A changed directory identity, descriptors, configuration, alternate or
driver list aborts the report. Equal snapshots are not an atomic freshness proof,
nor evidence that an unbound interface has no userspace owner.

```sh
# Only after scoped hardware permission and owner-managed attachment:
python3 tools/usb_descriptor_probe.py \
  --sysfs-device /sys/bus/usb/devices/OWNER_SELECTED_DEVICE \
  --codec target/debug/link-protocol > /PRIVATE_OUTSIDE_GIT/descriptor-report.json
```

The report contains a descriptor SHA256, numeric metadata, current driver names
and an explicit gate: kernel-driver-bound, alternate-change-required or candidate
only without authority. Current alternate state and advertised alternate 1 are
separate. The tool cannot qualify hardware via `--fixture-directory`; that mode
is explicitly labeled synthetic and exists for POSIX file/symlink tests.
Raw descriptors and reports should stay outside Git until inspected/sanitized.

## Concrete first physical test: descriptors only in the existing VM

This procedure is prepared for review; it has not run and is not authorized by
the prior disposable-PipeWire-core permission.

1. Agree on a short interruption window and the exact Studio device. Snapshot
   Windows host and guest settings, endpoint channels/formats, routes/defaults,
   volumes/mutes and relevant persistent-state hashes into private evidence.
   Do not stop applications, change VM configuration or detach hardware implicitly.
2. With specific owner authorization, the owner connects the selected physical
   Studio to the existing guest through VMware. Do not add autoconnect rules or
   change drivers/firmware. Treat the entire composite device as moving: host USB
   access and host audio may be interrupted. VMware documents that host and guest
   cannot simultaneously access an ordinary USB device; see
   [Broadcom USB guidance](https://knowledge.broadcom.com/external/article/307966).
   The streaming PC's Windows App/Studio/Mix Create setup remains the eventual
   remote side; this step does not establish that it remains unaffected.
3. Select the guest's exact sysfs device privately and run the one cache probe
   above, with no publication, polling, control transfers or audio streams.
   Limit the descriptor observation to 30 seconds total. The guest's normal kernel
   enumeration/driver binding during attachment is distinct from probe reads;
   this is not a guarantee of a completely unchanged guest audio graph.
4. Stop on mismatch, driver ownership, changed cache, timeout or missing device.
   Do not detach a kernel driver, change alternate setting or retry another target.
   A successful report only establishes a descriptor candidate and current state.
5. The owner returns the same device to the host. Compare both baselines and
   restore only explicitly owned changes under the agreed scope. Re-enumeration
   may replace endpoint identities; compare settings/routes semantically as well
   as recording identity changes. Do not equate restarting Link with restoration.

Only after descriptor review: implement a selected-device Linux transport with
explicit configuration/interface/driver checks, finite operations and no auto
detach/retry/fallback, then seek a separate bounded A0/A1/A2 handshake test.
Publication, incoming assignments, real routing/audio and native-Linux acceptance
follow as separate milestones. Descriptor-only approval would not cover them.

## Exact validation and limitations

Identical canonical code built offline on Windows and Ubuntu 26.04.1, using the
existing Rust/Cargo 1.88.0 toolchain. All 25 focused Rust integration tests pass
on both: 7 descriptor/CLI, 10 session and 8 codec/CLI. All 5 new Python probe
tests pass on Linux using the actual built Rust CLI and synthetic POSIX files.
The probe's sysfs directory/symlink tests are Linux-oriented; they are skipped
on Windows. Windows formatting and diff checks passed; Linux rustfmt was not run.
No native routing tests were repeated for this metadata-only change.

Tests cover every truncated fixture prefix; zero, inconsistent and oversized
totals; missing/duplicate descriptors/counts; two configurations; wrong device,
class, alternate or transfer type; shared audio-interface addresses; exact packet
metadata; input/count limits; actual CLI errors; cache changes during validation;
and an unchanged bound driver link. The serial sentinel is never read or reported.
These tests do not prove real sysfs behavior, USB ownership or interoperability.

Tested-code SHA256:
`4ef14186053efc7f79599aecac2cc314be52de6bb8045d8113aa7907d44bb64b`.
The canonical hash scope/algorithm is the same as [protocol-session.md](protocol-session.md).
Source archive SHA256:
`77a2c25822a7316a40bd68199aa85a034868ffb0f580d6e53b9a9f9aecd6fcde`.
Private Linux validation journal SHA256:
`57ef583847ee7e35508d29511a30e304727387748e4d716f353ee80573c4059a`.
The archive precedes these documentation updates; tested code is identical.
Guest free disk after the batch: 23,212,453,888 bytes, still below 25 GB. Builds
used one job; no packages were installed and existing evidence was preserved.

No paid security scan was run. Descriptor parsing is an untrusted-input boundary;
a narrow security review is a useful later check before enabling a hardware backend.

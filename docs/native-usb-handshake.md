# Native USB handshake implementation and test plan

Updated 2026-10-12. The optional Linux transport now compiles against the existing
libusb runtime and implements one guarded A0/A1/A2 exchange through the existing
session controller. The first device attempt stopped during setup, before the
protocol exchange, and released its claim; details and correction appear below.
Ordinary CLI device
access and route application remain disabled. The earlier authorization covered
the completed [descriptor observation](usb-descriptor-observation.md) only;
the owner subsequently authorized the necessary hardware/VM continuation,
including this handshake and temporary audio interruptions.

## Source owners and supported profile

`src/usb_handshake.rs` owns the one-shot lifecycle and implements `session::Transport`.
`src/native_usb.rs` supplies the Linux backend behind `native-usb`.
`examples/native_usb_handshake.rs` is a separate explicit execution entry point.
`tests_native/usb_handshake.py` provides prepare/run/compare stages with private
evidence and full backups of the four tracked WirePlumber state files.
No parallel inventory, routing, receipt or protocol codec owner was introduced.

The supported profile is the inspected device: VID/PID 33AE:4003, bcdUSB 0200,
bcdDevice 0100, one configuration with value 1, audio interfaces 0–2, vendor
interface 3 alternate 0 with no endpoints, alternate 1 with bulk OUT 03 / IN 83,
512-byte maximum packets and subclass/protocol 00/00. Initial alternate settings
must be zero, interface 3 must have no kernel driver, and the retained device must
report libusb high-speed (480 Mb/s). Other revisions/configurations are refused.
Audio alternate settings and driver names must remain identical across checks.
This policy does not establish audio format/channel compatibility.

The caller supplies the exact private descriptor cache and sysfs device directory.
The backend validates the profile/cache before opening its derived
`/dev/bus/usb/BBB/DDD` node. It checks character-device type, opens with O_NOFOLLOW,
and compares path/fd filesystem identities before wrapping the retained fd.
It rechecks sysfs identity, cached bytes, configuration, kernel ownership, and
audio interface state before each transfer. The wrapped descriptor, bus/address
and speed must match. These checks reduce replacement races; they do not create
an atomic snapshot of sysfs and the device. The retained fd is the I/O authority,
so no replacement/fallback device is opened if the original disappears.

## Native calls and completion

The backend initializes one libusb context with NO_DEVICE_DISCOVERY, wraps the
explicit fd, claims only interface 3 and selects alternate 1. It uses no device
enumeration/open-by-VID/PID, configuration setter, kernel-driver detach, auto-detach,
reset or clear-halt API. Configuration/descriptor/ownership observations may
involve native queries; this is not a cached-descriptor-only execution path.
Driver ownership or a userspace busy claim stops setup rather than taking ownership.

The handshake gate permits only four-byte A0 then A1 writes on 03 and one
512-byte read on 83. The existing session requires exact transferred counts and
an A2 first DWORD. Every native I/O error is treated as an unknown outcome;
there is no retry or prefix resend. Short counts stop progression. The transport
refuses publication, polling, arbitrary writes and a second handshake even after
successful initialization. A2 metadata is opaque and redacted in output.

Positive bulk deadlines are rounded upward to milliseconds and capped at 500 ms;
zero is never passed because libusb treats it as unlimited. Claim, alternate,
query and cleanup calls do not all expose that deadline. The runner supervises
the child for 12 seconds, kills only that child on timeout, then waits at most
two more seconds. A kernel-blocked child can remain unconfirmed; that outcome is
recorded and comparison is refused pending manual review. A kill is not proof
of interface or persistent-state restoration.

Explicit close releases only the acknowledged claim, checks the original baseline,
closes libusb before its caller-owned fd, then exits the context. Release is
documented to send SET_INTERFACE and reset the first alternate, which is zero
in the accepted profile. Failed release or changed observation yields
`baseline_restored: false`; Drop attempts cleanup only as a safety net and does
not repeat an already attempted release. This flag covers the selected device's
observed profile/interface state, not the entire host/guest audio baseline or
peer state after A0/A1. Peer recovery remains hardware-unverified.

Primary ABI references: the [public libusb 1.0.29 header](https://github.com/libusb/libusb/blob/v1.0.29/libusb/libusb.h),
[selected-fd wrapping and interface lifecycle](https://libusb.sourceforge.io/api-1.0/group__libusb__dev.html),
and [bulk completion/timeout semantics](https://libusb.sourceforge.io/api-1.0/group__libusb__syncio.html).
The direct Rust C declarations use documented opaque handles and repr(C) layouts;
Linux tests check descriptor/option/version sizes and offsets. The feature is
limited to 64-bit native Linux builds, with libusb >=1.0.27. No external Rust
crate or development-header installation is required. The default crate remains
dependency-free; `native-usb` links `libusb-1.0.so.0`.

## Build and verified checks

```sh
cargo build --offline --locked -j 1 --features native-usb --example native_usb_handshake
cargo test --offline --locked -j 1 --features native-usb --lib --test usb_handshake
target/debug/examples/native_usb_handshake --library-version
target/debug/examples/native_usb_handshake --check-cache /PRIVATE/device-cached-descriptors.bin
```

Library-version and cache-check modes initialize no libusb context and open no
USB device. The saved physical cache from the completed descriptor test passes
the strict handshake profile. The synthetic handshake fixture is authored test
data, not copied from proprietary/raw hardware evidence.

Windows default build and 25 focused integration tests passed: 8 new handshake,
10 session, 7 descriptor/CLI. Linux default build and 33 focused integration tests
passed, including the 8 codec/CLI regressions. The final native feature/example
built, its 1 ABI/library unit test and 8 handshake integration tests passed, and
7 Python runner tests passed on both platforms. These cover successful exchange,
profile/ownership changes, uncertain setup, partial transfers, reply rejection,
cleanup failure, exact claim ownership, disabled post-handshake operations,
state-content backups, pinned cache, stale authority inputs, timeout, unconfirmed
exit, no retry and preserving comparison differences. Runner execution is mocked
in Python tests; native device functions were not called in validation.
Windows formatting and diff checks passed; Linux rustfmt was not run.

Tested final code SHA256:
`cb9e610d21d650f76657a349d557da303708a2a6e2d03a79c7b909ac1dd5c358`.
Hash scope/algorithm is the canonical procedure in [protocol-session.md](protocol-session.md).
Source archive SHA256:
`a1c8f2e1c03631265fa09dd7b77132fd993b34f15d99d0a277c7693dbfea66eb`.
Final Linux validation journal SHA256:
`eec3b3386ac73af22cada27afb3b6462cc92f8f64d4292b24a499fa19fbee741`.
Native handshake executable SHA256:
`b5dfae328a910ad899ae65f6098964f984bd229fb4b41ab748eb285a8b33e3e0`.
Runner SHA256:
`ba744deb3a867aff35dc8ef5da8412e0c8a0746f755fd9aba114a6e92cf434e9`.
Only runner/tests changed after the first complete matrix; final Linux checks
cover those changes without repeating unrelated codec tests. Documentation follows
the archive; executable/test source matches the recorded final code hash.

Guest runtime: Ubuntu 26.04.1, Rust/Cargo 1.88.0, libusb 1.0.29
(existing package 2:1.0.29-2build1). Final free disk was 22,814,289,920 bytes,
below 25 GB; validation used lightweight single-job builds and installed nothing.
Private sources/archives/journals and previous settings snapshots were preserved.
No paid security scan ran; a narrow review of the FFI/device/persistence boundaries
is useful before enabling broader hardware operation.

## Proposed first handshake in the existing VM

The owner authorized this plan and the necessary hardware/VM continuation after
reviewing the implementation. It was used for the first setup attempt described
below. No A0/A1/A2 exchange has completed. Refresh baselines older than ten minutes
before execution.

1. Keep Studio on Windows while saving fresh host endpoint/default/volume/mute,
   session and registry baselines with full private contents. Save guest filtered
   graph plus full four-file WirePlumber backups through the runner's prepare
   stage, within ten minutes of the attempt. Preserve old evidence separately.
   Inspect current VMware controller and device ownership; do not silently change
   controllers, permissions, drivers or stop applications to make the test pass.
2. The owner connects the same Studio Link to the existing VM. Privately select
   the one sysfs target and verified expected cache. Stop on ambiguity, cache or
   speed mismatch, active audio alternates, driver ownership or insufficient access.
   No elevation, permission/udev edits or fallback is implicit in test authorization.
3. Run one child using the following staged runner with the built, hash-pinned
   executable. It claims interface 3, selects alternate 1, sends A0 then A1,
   reads/validates A2, releases the owned claim and checks alternate-zero/profile
   restoration. The child and runner save bounded output; no raw A2 identity
   metadata is logged. There is no publication, polling, audio or route application.
4. Success, refusal, failure or timeout all end the attempt. The owner returns
   Studio to Windows. Compare host and guest settings/state, record identity
   changes, and inspect differences against complete backups. Restore only
   attributable owned changes under the test scope, never overwrite concurrent
   work or blindly rewrite live registry/state. A discrepancy is a test limit,
   not permission for another handshake or broader cleanup.

```sh
python3 tests_native/usb_handshake.py prepare --evidence-dir /PRIVATE/new-handshake \
  --executor target/debug/examples/native_usb_handshake
# Only after explicit owner authorization and owner-managed attachment:
python3 tests_native/usb_handshake.py run --authorize-handshake \
  --evidence-dir /PRIVATE/new-handshake --executor target/debug/examples/native_usb_handshake \
  --sysfs-device /sys/bus/usb/devices/OWNER_SELECTED --expected-cache /PRIVATE/verified-cache.bin
# After owner return to Windows:
python3 tests_native/usb_handshake.py compare --evidence-dir /PRIVATE/new-handshake \
  --executor target/debug/examples/native_usb_handshake
```

Passing this test would establish one initialization exchange and observed cleanup
for this device/VM. It would not prove remote Windows visibility, publication,
assignment handling, audible audio, reconnects or native-Linux acceptance.

## First authorized attempt and claim-observation correction

The owner authorized necessary hardware/VM continuation and connected Studio.
Fresh host and guest baselines included full registry/state-file contents. The
selected guest device reported cached speed 480 Mb/s and was accessible to the
existing guest user without permission changes. The original executable ran once
with a 119-second-old baseline. It exited 2 during setup revalidation, before
creating the session or submitting A0/A1/A2. Its claim release succeeded and the
selected-device observation matched the original baseline. No timeout occurred;
the child exited. The attached-versus-post-child guest graph and all four tracked
state files matched exactly. A subsequent cached sysfs observation confirmed
interface 3 alternate zero with no driver, and audio interfaces 0–2 still at
alternate zero with snd-usb-audio. The owner then returned Studio to Windows:
guest target count is zero and all three Windows USB functions report OK.
Windows defaults and complete endpoint state/format/volume/mute observations
match. All four tracked guest state-file contents and modes match exactly.
The filtered guest graph differs by one recreated node and four ports; after
removing object/client identities and remapping parent node IDs to unique node
names, all graph properties/parameters and metadata match. No links or devices
were present in either filtered graph. The exact graph comparison remains false.

Windows session count stays 30, with the same instances and only session-state
differences. User audio and policy registry trees match. MMDevices differs in
17 values: 16 binary values of 16 bytes and one string. Their cause/meaning is
unresolved. Full before/after contents and private differences are preserved;
no registry or guest state was overwritten. This is observed logical restoration
with recorded Windows persistence and object-identity differences, not an exact
unchanged host/guest baseline.

The setup error returned generic revalidation code -9001 and did not identify
which native observation failed. The likely cause is the incorrect comparison
between sysfs driver presence and `libusb_kernel_driver_active`: the
[Linux libusb implementation](https://github.com/libusb/libusb/blob/v1.0.29/libusb/os/linux_usbfs.c)
reports `usbfs` userspace claims as inactive kernel drivers. The original adapter
rejected a nonempty sysfs driver whenever that query returned zero. This is a
source-derived explanation; the failed attempt did not log the claimed driver.

The adapter now tracks only its retained handle's acknowledged claim and accepts
the `usbfs`/inactive pair only while that claim is held. It refuses `usbfs` before
its own claim, inconsistent observations, and missing ownership after claim.
Successful release clears that state; the baseline must again have no driver.
This does not authorize detachment or ownership of another handle's claim.

The corrected native example builds on Linux; two unit tests (ABI/version and
ownership combinations) and eight handshake integration tests pass. Metadata and
saved physical-cache checks pass without opening a device. Windows formatting and
diff checks pass. No corrected hardware attempt has run. Keep the old test journal
and backups. Return comparison is complete; fresh backups for a separate corrected
cycle are now saved, awaiting owner-managed connection.

Original guest baseline SHA256:
`1c056fae4ed18503d010e948c7af39d8c3f554bec79737aaeaa83958fdbea96a`.
Original host baseline SHA256:
`d0c31e4aa3c39306b90683750cad8100d691339435799f636fab3c2ee4f7a4fa`.
First handshake journal SHA256:
`143f1c6bfe8173599a1252f55b73027982e819fee42347eb88c868f48d814242`.
First guest return comparison SHA256:
`ed20756fefc3edbf0bd881fcc5716500daa47851401a34c13fd585327f60a189`.
First guest semantic comparison SHA256:
`c6ceff9c0f0495f9634ff6851b6d0e3c1789b744cacb31104392b7d772160c99`.
First Windows return snapshot SHA256:
`428b31872c1dad453ad007f1582f98a72f73ea8b65e598954e8ce973fc58a157`.
Corrected tested code SHA256:
`94f396e153fe322735fd71e1f49bc5e60e2f26c7ba9428ee2333ec3c684e9ec5`.
Corrected source archive SHA256:
`a1af43cc33e774b1aa83a81976f3c4a573f0f1b99a4fbe79b850379448c917d6`.
Corrected Linux validation journal SHA256:
`a619d1628287b48844a64f418d16ec8da78342442ff1ea53ac42a9147ea4045d`.
Corrected native handshake executable SHA256:
`e89ea6a274902d938a24c8a6c31c8bd50b307c0246c79ccc43932facd5064ed5`.
Final free disk was 22,678,618,112 bytes; no packages were installed.

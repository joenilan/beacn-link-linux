# Native USB app-list publication and assignment poll

Updated 2026-10-12. This batch adds an explicitly enabled one-list/one-poll mode
to the existing Linux USB owner. Builds, focused tests and a real private-core
source probe pass. Hardware publication and visibility in the streaming PC's
BEACN App have not yet run. The earlier real Studio handshake is documented in
[native-usb-handshake.md](native-usb-handshake.md).

## Execution model and source owners

The replacement runs in the Linux VM. Windows BEACN Link on the gaming/host PC
must be closed during testing; the owner closed it and the process check found
none. BEACN App on the separate Windows streaming PC stays running. The owner
authorized necessary hardware/VM continuation; do not repeat routine approval
requests. Physical VMware connection/return and the remote UI observation still
require the owner's actions. Preserve settings and unrelated work.

`usb_handshake::Publication` validates exactly one explicit inventory application,
with nonempty live streams, through the existing lifecycle/planner/codec owners.
It constructs a canonical initial record with assignment zero, retained opaquely.
Zero is a test input, not an established unroute/default-channel meaning.
`Handshake::prepare_publication` enables the additional gate; the native opener
is `native_usb::open_publication`. Default `open_handshake` remains handshake-only.

After a full valid A0/A1/A2 exchange, the mode permits only the exact pinned list
on OUT 03, then AC010000 on OUT 03, then one bounded IN 83 reply. Exact write
counts are required. The existing bounded poll decoder validates actual bytes
against declared payload length before permitting completion. Changed lists,
assignment changes, wrong order/direction/size, malformed responses, device/audio
changes and unknown/partial I/O halt progression. There is no resend, second list,
second poll, automatic retry, guessed shutdown or withdrawal command.
Returned zero/out-of-range assignment values remain opaque. NoPayload is distinct
from an empty authoritative list. The default handshake gate still refuses
publication/polling; a bad A2 cannot enable publication even via direct transport.

`examples/native_usb_publication.rs` reads a bounded caller-supplied TSV snapshot,
initializes, publishes and polls through `Session`. It uses the existing planner
and lifecycle version guard for decoded records and reports counts/assignments
for the bound name, redacting other names and A2 metadata. It applies no routes.
After a successful exchange it retains the source/claim for ten seconds so the
owner can observe the remote UI, then explicitly releases/checks the original
device baseline. This is not a heartbeat or ongoing publication service.

`tests_native/usb_publication.py` reuses the handshake runner's full-content
desktop/state snapshots and owned-child supervisor. Native publication has a
30-second child limit plus two seconds for confirmed kill/exit; ordinary
handshake and source-only modes retain their 12-second bound. An unconfirmed
native exit retains the owned private processes and refuses comparison. A failed
private-process stop also retains PID evidence and refuses comparison.
Cleanup does not prove remote app-entry removal or remote persistence rollback.

## Real Linux application source

The runner starts one PipeWire core in an exclusively owned short runtime
directory, with no hardware discovery, fallback, Pulse/session manager or startup
objects. Configuration/evidence remain in the private test directory. A short
socket path is necessary because PipeWire also creates a `-manager` socket;
an initial long-path source probe failed and its owned server exited, with no
USB I/O. The corrected source probe passed.

It starts one retained `pw-cat` process with authored silent PCM, target zero
(unlinked), autoconnect/reconnect and state restoration disabled. No sink or
audio route is created. Actual read-only private-core dumps must identify exactly
one application matching that process PID and the unique owned node name, with
live streams and no links. The existing inventory adapter explicitly binds it
to `BEACN Linux VM Test`. This is a real Linux process with a synthetic silent
stream; it does not prove game/Firefox acceptance or audible USB audio.

The runner observes the same source immediately before and after the native
child. Snapshot versions and process checks reduce stale-input mistakes; they
do not make inventory and USB publication atomic. Standalone TSV input does not
establish OS freshness. Afterward it stops only retained owned processes and
checks the private graph against its baseline before stopping the server.
Private logs, PCM, configuration and snapshots are retained outside Git.

## Verified checks

Windows default build, 25 focused integration tests (session 10, handshake 8,
publication 7), 13 Python runner tests (existing 7, new 6), formatting and diff
checks pass. Linux default/native-feature example builds, the same 25 integration
tests, two ABI/ownership unit tests and 13 Python runner tests pass. The saved
physical cache still validates offline. Final example/runner changes added only
the UI observation hold and its supervision limit; the final Linux build and
runner/source checks cover them without repeating unchanged Rust integration tests.
Linux rustfmt was not run. No packages were installed.

The actual final source-only probe launched/discovered the owned Linux stream,
ran the native example's offline inventory check, preserved the source, restored
the private graph, and stopped both owned processes. No USB access was requested.
Desktop graph and all four tracked state-file contents/modes matched exactly.
This is native source/cleanup evidence, not hardware publication or Windows UI
visibility. Python runner unit tests use mocks; the source probe is separate.

Final tested code SHA256:
`6a79289c2100ac4a3ebed67a680884ac910a21fe8ef150bbb53db2dc48b41a3d`.
Canonical hash scope follows [protocol-session.md](protocol-session.md).
Final source archive SHA256:
`c73ddcec0d05298ff154b8fdf4cd39ff5101e16bb68884e57b0491ed2a837f16`.
Final Linux validation journal SHA256:
`0e2223a55f201f1badd6fb147442783c98753cfb21714988bc626d16c9a15a0b`.
Native publication executable SHA256:
`4a55fe7e4a07f9100f6099bf75c86457a9274fdb8470f46a967ac599d12591a4`.
Publication runner SHA256:
`3fc4c7437413a1a492af115253575620eba0d802d2935c3676fb9d524ac401ed`.
Shared handshake runner SHA256:
`ed84dcec98fcb906069615b31787f802c51eac88ac761d55b254b16708e31583`.
Final guest free disk: 21,722,476,544 bytes, below the 25 GB threshold; builds
remained lightweight and single-job. Failed/earlier source archives are preserved.
No paid security scan ran; a narrow process/FFI/file-input review remains useful
before expanding operation. No workflows or external Rust dependencies were added.

## First hardware publication test

1. Confirm Windows Link is closed. Save fresh full Windows settings/registry and
   guest graph/four-file backups while Studio remains on Windows. Pin the built
   executable and both runner hashes. Keep the streaming PC's BEACN App visible.
2. The owner connects Studio to the VM. Select one exact target and the verified
   private descriptor cache. Stop on stale baseline, ambiguity or ownership/profile
   mismatch. Start the owned private-core source, publish its one explicit name,
   poll once and perform explicit cleanup. No routing, capture or device fallback.
3. The owner reports whether `BEACN Linux VM Test` appeared in the streaming PC's
   BEACN App. Record this as owner-observed visibility; complete native transfers
   alone do not establish it. Do not assume a short one-shot list persists after
   cleanup; refresh cadence, peer retention and disconnect semantics remain unknown.
4. The owner returns Studio to Windows. Compare full baselines and preserve
   differences. Inspect before restoring attributable owned changes; never
   overwrite concurrent settings or blindly rewrite registry/state files.

```sh
cargo build --offline --locked -j 1 --features native-usb --example native_usb_publication
python3 tests_native/usb_publication.py prepare --evidence-dir /PRIVATE/new-test \
  --executor target/debug/examples/native_usb_publication
# Isolated source-only probe, with Studio still on Windows:
python3 tests_native/usb_publication.py source-probe --authorize-isolated-source \
  --evidence-dir /PRIVATE/new-test --executor target/debug/examples/native_usb_publication
# Use a separate fresh prepare/evidence directory for the actual hardware attempt:
python3 tests_native/usb_publication.py run --authorize-publication \
  --evidence-dir /PRIVATE/new-hardware-test --executor target/debug/examples/native_usb_publication \
  --sysfs-device /sys/bus/usb/devices/OWNER_SELECTED --expected-cache /PRIVATE/verified-cache.bin
python3 tests_native/usb_publication.py compare --evidence-dir /PRIVATE/new-hardware-test \
  --executor target/debug/examples/native_usb_publication
```

# Native USB app-list publication and assignment poll

Current physical interoperability evidence is in
[bounded concurrent Studio audio/control](native-continuous-studio.md).
The sections below preserve earlier USB-only and VM milestones and their limits.

Updated 2026-10-13. The latest checkpoint is recorded under Initialization deadline
diagnostic below; earlier sections retain their historical validation scope.
The subsequent [native assignment integration](native-assignment-publication.md)
implements bounded private-core routing and continued route-verified publication;
its isolated routing probe and integrated hardware assignment/drop test passed.
Physical Studio audio remains unverified; see that document for exact results.
This batch adds an explicitly enabled one-list/one-poll mode
to the existing Linux USB owner. Builds, focused tests and a real private-core
source probe pass. A native app-list write and valid no-payload poll now pass;
the owner saw `BEACN Linux VM Test` briefly in the streaming PC's BEACN App.
No routing assignment or audio interoperability has been verified.
The earlier real Studio handshake is documented in
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

## First hardware attempt: transferred list, rejected poll

Windows Link was absent from the host process check. Fresh full pre-attachment
backups pinned the executable/runners/cache. At baseline age 81.181 seconds,
the native child completed A0/A1/A2 and the entire canonical app-list write.
The poll returned ACFFFFFF and the existing decoder rejected it. The child
exited 2, without timeout/retry, and successfully released its claim/verified
the device baseline. Both owned processes stopped, the private graph restored,
and attached-versus-after-cleanup desktop/state snapshots matched exactly.
The success-only ten-second hold was not entered because of the poll error.

Visibility was unobserved: the owner did not know where to look. Do not label
this as a confirmed absent app. The owner returned Studio; all three Windows
functions were OK, defaults/formats/volumes/mutes matched, guest state-file
contents/modes matched, and returned graph semantics matched after one recreated
node/four ports. Windows gained one session (31 to 32, shared fields unchanged),
16 MMDevices binary values changed, and one user-policy string changed. User
audio registry matched. Complete differences/backups remain private; no rewrites.

Initial host baseline SHA256:
`e527aeb149130e994cc4ad4e77d245fe85127c842ace2491dd01e46f617c43b1`.
Initial guest baseline SHA256:
`cb61c96962827077bd007434da6b172e7512e2bba95fe213e49bdafe7da801a8`.
Rejected-poll native journal SHA256:
`2328f63600de604679314afd60de039507aae2f7d9decad407cc6e034a89e251`.
Initial guest return comparison SHA256:
`78936294bb2b55480e8612123f31c4fd23f449449dcb7f2d48751845138f7af1`.
Initial returned graph semantic comparison SHA256:
`8bd6fd847118ccd26c160515ddca62c9912252f3f5b5bd63185352f6c5965742`.
Initial Windows return SHA256:
`61396aa96d171a9e66023a98280b9e4fbb3b5c5eec600aa5ba3ffa14d1759d56`.

## Controlled timed attempt: native success and brief owner-observed visibility

The owner's streaming-PC screenshot identifies BEACN App v1.4.0. Its selected
ASSIGNMENTS tab has an Applications column showing `obs64 System`, immediately
right of Outgoing Studio Link Assignments. This is the precise box identified
for the observation; top Link In cards are audio-channel controls. The screenshot
was taken after the first return and is preserved privately, not evidence of
presence/absence during the earlier test. BEACN's
[Mix Create guide](https://support.beacn.com/en-US/getting-started-with-beacn-mix-create-282018)
also documents the Applications pool below the knobs in ASSIGNMENTS.

The next native example adds a 500 ms interval between the list write and the
one poll, based on the inspected original application's 500 ms publication timer.
This is a timing hypothesis, not proof of a firmware requirement or sole cause
of the changed result. It also exposes only the boolean/nonzero value of response
byte 28 (body index 24), which the inspected Windows helper returns; its meaning
remains unknown. Other A2 metadata remains redacted. An explicit rejected header
now permits a ten-second observation hold with no further USB requests, followed
by failure/cleanup. Unknown I/O and other errors still clean up immediately.
Success retains the original ten-second hold. No retry or packet-format change.

The diagnostic executable built on Linux and its offline inventory check passed.
Core/library/runner code stayed identical to the focused-tested implementation;
only the example changed, so unrelated matrices were not repeated. The owner had
already reattached Studio before baseline preparation. Default prepare refused
that order without device I/O. A fresh **connected-state protocol baseline** was
then explicitly saved before commands, retaining the previous full pre-attachment
and returned references. This scoped baseline is not a pre-handoff snapshot and
must not be presented as one. The host's connected snapshot had ten active
endpoints; the guest had one BEACN ALSA device, 22 nodes, 76 ports and 16 existing
links. The protocol test preserved this entire connected desktop graph exactly.

At protocol-baseline age 9.782 seconds, the native child completed handshake/list
and returned a valid 512-byte AC000000 no-payload poll (508 ignored tail bytes).
Response byte 28 was zero; this does not mean readiness was false or that an
assignment-zero action occurred. The owner reported `i saw it`, then `briefly`,
for the advertised Linux name in the identified streaming-PC Applications box.
Record this as **brief owner-observed visibility**, not independently captured
video, stable long-lived visibility, a game/Firefox test or audio interoperability.
No assignment records were received and no routes were applied.

The child exited 0 without timeout/retry, released/verified the original device,
preserved the owned source, restored the private graph, and stopped both owned
processes. Attached-before versus after-owned-cleanup desktop and all four state
files matched exactly, including the existing foreign desktop links.
The owner returned Studio; guest target count is zero and Windows functions are
OK. Guest state-file contents/modes match the connected protocol baseline. The
connected-vs-returned graph/default/endpoint comparisons differ as the hardware
disappeared/returned: guest graph is now five nodes/eight ports/no links/devices,
and Windows has 18 active endpoints. Common ten host endpoints retain settings.
Windows defaults and all endpoint state/format/volume/mute observations match
the earlier full returned reference; returned guest semantics also match that
earlier reference after identity normalization. These reference comparisons are
separate from the fresh connected baseline. MMDevices differs from the connected
snapshot in 40 values (eight DWORD, sixteen binary, sixteen strings); user
audio/policy trees match. Preserve all differences, with no blind state/registry
rewrite or exact cross-handoff baseline claim. Twenty-six private files were
hashed; earlier evidence and settings screenshots remain preserved outside Git.

Diagnostic tested code SHA256:
`9a610b271182dadeb4449e9c5e59c7f6d577cb0ed5cba1aca3f9c8e495a9d7ca`.
Diagnostic source archive SHA256:
`e2905f9e74674e75fde16773bfab74503d335929a150482facda3924b241c474`.
Diagnostic Linux build/check journal SHA256:
`3fa97f9b0f01b7519b915e574648dcae6dbf7385355adc2db9297adf2a316d26`.
Diagnostic native executable SHA256:
`c92dc28c491d745867739189aecaec5b3dd5df1bcec31df3cd9c54727b8a8ccb`.
Connected protocol baseline SHA256:
`8ba5141c525b66991d4f722a9dedd21edaaeb418dc64bd81d79efa3278154267`.
Successful publication/poll journal SHA256:
`3fdf65818c1bff9c9eab914aea1688e17df73094cd80843eedd2c81535bce936`.
Diagnostic guest return comparison SHA256:
`afee3fe0905ae688379b991742567897f8444f017df3a27a6ad0d5d9bda0dd20`.
Returned guest versus prior reference comparison SHA256:
`7c436dc3cb29a5fa8de9ba3a75b8a1ed51b8a85c11620291b05e2ba4dda21159`.
Diagnostic Windows return SHA256:
`9fc82c6e11c55d0da8e2a0596b5e4912d32f956fb41f6eaebe05cfe70bfda332`.
Latest measured guest free disk: 21,639,749,632 bytes; no packages installed.

## Bounded periodic VM diagnostic (2026-10-12)

`examples/native_usb_periodic.rs` adds explicit `--authorize-periodic <sysfs-device>
<expected-cache> <cycles>` with a 1..40 budget. `--check-periodic <cycles>` performs
only the fresh-inventory exchange, with no USB access. The ordinary opener remains
handshake-only and `Publication::from_inventory` remains one-shot. The new
`Publication::bounded` policy uses the same transport gate, retained device,
codec, session and lifecycle tracker; there is no second routing/receipt owner.

Initialization leads to an inventory gate. Every publication needs a strictly
newer complete inventory with exactly the pinned application, wire name, streams
and live states. After a fully validated poll the gate either exhausts its budget
or waits for another inventory. Missing refresh, rebinding, stale generation,
wrong order/frame, partial/unknown transfer or rejected/malformed reply halts it.
No failed exchange is retried. Device/profile checks still precede every I/O.

The Python runner opts in with `--cycles`. Its private core and unlinked synthetic
source remain exclusively owned. The child requests snapshots over stdin/stdout;
the runner checks the retained process handles, obtains a fresh bounded pw-dump,
checks exact identities/no links, saves each private dump/projected inventory and
answers with a monotonically increasing generation. IPC has explicit request,
line, body and output caps; nonblocking writes and a 50-second child deadline
prevent indefinite waits. Unknown child exit retains private processes for review.
All snapshot/cleanup owners and used-attempt markers remain in force.

The example requests another inventory after each poll before planning any reply.
It waits 500 ms between list publication and poll as a controlled diagnostic
timing hypothesis. A received assignment 1..4 for the bound name stops publication,
holds for five seconds and checks the source again; it never reports the desired
assignment as applied or repeatedly overwrites it with outgoing local zero.
There is no routing, audio capture, guessed shutdown/empty list, heartbeat recovery
or daemon. Atomic stream-liveness across USB I/O is not established by snapshots.

Windows focused checks: 28 Rust integration cases and seven Python runner cases,
with two further Linux-only pipe-supervisor cases skipped. Linux passed all 28
integration cases, two native USB ABI/owned-claim unit cases and 16 Python runner
cases. Failures covered budget/order, frozen/rebound/disappeared source, malformed
poll, partial/unknown transfer, invalid IPC sequence, timeout and inventory loss.
After two comment-only documentation corrections, the exact final native periodic
and one-shot examples were rebuilt. The final three-cycle private-source probe
served seven fresh inventories, preserved its source, restored the private graph,
stopped both owned processes and matched the desktop graph plus all four tracked
persistent-state files exactly. Studio remained on Windows; no USB I/O occurred.
This proves the fresh-inventory/supervision path, not periodic hardware acceptance.

Final code SHA256:
`81213364ed745647da025d8cde3c9d858c74d29ee400e4c73a81f2e21e879a44`.
Final source archive SHA256:
`d9f667f0e5d8db8b83b51b6cb37947dc6fe32d64eedca00b1282e19636acb338`.
Focused Linux matrix journal SHA256:
`f6b7ca86316f323dec0f1a2b995cc72f224f3f6356517eb24a4bebda0d799a62`.
Final Linux build/private-source probe journal SHA256:
`023a855918acb0a1b9ae785fb99c3edecaa9a4d0371a14fdc7d506932e5592f9`.
Final periodic executable SHA256:
`4e5958ff52ebfc2d39cfec444f4c596f5abdc75fd8545654737feab3a488a359`.
Runner SHA256:
`4e0ec147596827fd88379b59531d7ad60dbfac069265e6c417bde97cb0397a57`.
Shared runner SHA256:
`ed84dcec98fcb906069615b31787f802c51eac88ac761d55b254b16708e31583`.
Final free guest disk: 21,263,077,376 bytes. No packages installed or artifacts deleted.

Next: fresh host/guest snapshots, then owner-managed USB handoff for 40 cycles.
Keep Windows Link closed. In streaming-PC BEACN App v1.4.0, the target is the
bottom ASSIGNMENTS / Applications box containing `obs64 System`, immediately
right of Outgoing Studio Link Assignments. While `BEACN Linux VM Test` is visible,
drag that test entry into the first `Link In` channel. Verify an actual bound
assignment record and compare settings after returning Studio to Windows.
Sustained visibility, assignment receipt and audio remain unverified in this batch.

## First periodic hardware attempt: A2 failure, no publication

Fresh full-content host/guest snapshots preceded the owner-managed USB handoff;
Windows Link remained closed. The 40-cycle attempt used the final executable
above. The initial live-inventory request succeeded, A0/A1 completed, and A2 read
returned `TransportError::Unknown`. The native layer did not retain the libusb
error code or transferred-byte count, so timeout/stall/disconnect cannot be
distinguished from this evidence. Do not label it a timeout or infer a periodic
loop defect: the failure occurred before that loop or any list/poll submission.

There was no retry. Native cleanup reported release success and baseline restored;
the child exited 2 without supervisor timeout. The owned source remained valid,
the private graph was restored, and both owned processes stopped. The connected
desktop graph and all four tracked persistent-state files matched exactly before
versus after owned cleanup. The owner returned Studio without another test.

After return, Windows defaults and endpoint states/formats/volumes/mutes/sessions
match the pre-handoff snapshot; Windows snapshot errors are empty. Guest tracked
state files match exactly, but its desktop graph exact comparison differs across
handoff. Machine-audio registry values differ in 16 places; user audio/policy trees
match. Preserve the complete differences outside Git with no blind rewrite or
whole-system exact rollback claim. Private guest evidence was downloaded.

Failed hardware journal SHA256:
`92bc2eee5caf79fbb7750d678e5c5624ca8c95c5fba8d53254c1489029007e13`.
Guest return comparison SHA256:
`766956fc39bc9c0f9c8ba4a5d15da23c0658f1ef540145af08f187616b36c6c6`.
Windows return snapshot SHA256:
`0bd956e1d74b7848d8bb6de73ae312c8b4770b173cdd917ebdfa5b7162888be2`.

The diagnostic follow-up preserves native failed-bulk endpoint, error code,
requested length and transferred count in stderr; it never logs payload/device
metadata, retries, or interprets partial error transfers as successful completion.
Protocol/gate behavior and deadlines are unchanged. Its exact Linux periodic and
one-shot examples built and both native unit checks passed. Earlier focused matrix
and private-source probe remain the evidence for unchanged periodic/runner behavior;
no extra hardware attempt or repeated routing probe occurred.

Diagnostic code SHA256:
`306398e4ba3051884acef4be1e874f822a311daea2fe283d92d7e77668c00842`.
Diagnostic archive SHA256:
`13d2cde66d4c298697168f7127466dd3c69945d33c3abbe57463111ec697f46c`.
Diagnostic Linux build/unit journal SHA256:
`b0857cd0bf61ff544eff913d49924d492fc8bb5820c6e116680d0ae069dfe51a`.
Diagnostic periodic executable SHA256:
`0a9a1021131b76d953ea0240bc4ea808dedebbd3093efc0be2ef00920e7235d9`.
Latest guest free disk: 21,142,679,552 bytes. Earlier attempts/artifacts remain intact.

Next work is focused startup/reconnect analysis plus meaningful native failure
diagnostics, before another owner-coordinated hardware attempt. Reconnection is
not presently requested. The A2 failure does not negate the earlier successful
handshake/list/no-payload poll and brief owner-observed app visibility, but the
periodic hardware and assignment milestones remain unproven.

## Explicit observed-startup option (2026-10-12)

Focused tracing of the pinned Windows binary establishes alternate zero, a wait
until timeGetTime reaches the prior tick plus one, alternate one, then IN 83 pipe
reset before initialization. The worker is started after initialization and has
no handshake/reconnect call in its inspected loop. See
[protocol-session.md](protocol-session.md) for exact VAs/file offsets/tool versions.
The successful and failed native journals used the same descriptor cache and
deadlines; the failed read's native code was absent. The startup difference is
an experiment target, not a demonstrated failure cause.

`Startup::ObservedWindows` is explicit. The existing default startup and one-shot
constructors remain unchanged. The gate requires a successful interface-3 claim
and fresh exact identity/profile at every preparation boundary; it selects vendor
alternate zero, waits 1 ms, selects one, clears only fixed vendor IN 83, and checks
the profile again before A0. The sleep approximates the observed next-tick wait;
it does not reproduce Windows scheduler granularity. Native clear-halt uses the
retained claimed handle; arbitrary endpoints, audio-interface claims, whole-device
reset, retry, disconnect recovery and longer bulk deadlines are not added.

The explicit native CLI is `--authorize-periodic-startup <sysfs-device>
<expected-cache> <cycles>`. The supervising runner opts in with `--cycles 40
--observed-startup`, records that policy in private journals, and refuses it for
one-shot/default, source-only and metadata stages. Any optional operation that
fails, has an unknown outcome, or observes identity change stops preparation;
only the acknowledged claim is released. No bulk is then submitted. Blocking
libusb setup/clear calls remain subject to the existing 50-second supervisor,
rather than being misrepresented as bounded by the 500 ms bulk timeout.

Windows passed 31 focused Rust integration cases and eight Python runner cases
(two Linux-only supervisor cases skipped). Linux passed all 31 integration cases,
two native USB unit checks and 17 Python runner cases. Exact native periodic and
one-shot examples built. Added tests cover optional preparation order, no bulk or
retry after each preparation failure, fresh identities at each boundary, default
compatibility, and invalid startup-mode refusal before process/device execution.
The previous three-cycle private-source probe remains evidence for the unchanged
IPC/source/cleanup path; it was not repeated in this startup-only batch.

Tested code SHA256:
`2f0e2a019f5ebc619c23d0e48a4f5894926d1f2c7ab80f6ba84c61804b51bfba`.
Source archive SHA256:
`75252db7bfce77cffde132f2c62f8a1a6aba8e65944ae3a148ed5e5e1afcd8fb`.
Linux build/focused-check journal SHA256:
`a9aa32586e7a949496c61db8f6d30bcbfcde666625b52a851a2395d577a54fca`.
Periodic executable SHA256:
`26fbe3a6922683f8d46b65f9b15586a014a18030f636ebbad33b08150b218008`.
Runner SHA256:
`c3926192600d835e2b2165edda40cc58e4d3ffd636fdcdf70af7b1926129224c`.
Shared runner remains
`ed84dcec98fcb906069615b31787f802c51eac88ac761d55b254b16708e31583`.
Latest free guest disk: 20,946,313,216 bytes. No packages installed or artifacts deleted.

Next is a fresh-snapshot, owner-coordinated 40-cycle hardware test of this explicit
preparation policy, with full native failure diagnostics and no automatic retry.
Keep Windows Link closed; Studio remains on Windows until that handoff. Use the
same exact remote Applications box and synthetic entry/Link In assignment as
above. Endpoint preparation equivalence, sustained publication, assignment receipt
and audio remain hardware-unverified for this new option.

### Observed-startup hardware attempt

Fresh full host/guest backups preceded the physical handoff. Windows Link was
absent. Explicit vendor preparation and A0/A1/A2 completed; cycle 1 published and
received a valid 512-byte AC000000 response with 508 ignored tail. Cycle 2 then
published and received ACFFFFFF from its poll. The existing strict rejection
policy halted the session. Four fresh inventory requests were served. No retry,
assignment application or audio routing occurred. Visibility was not reported
by the owner for this attempt. This is a successful initialization/first cycle;
it does not establish sustained publication or the cause of the prior A2 failure.

The child exited 2 without supervisor timeout. Claim release/profile restoration,
owned source preservation, private graph restoration and both process exits
passed. Studio returned to Windows. Host defaults, endpoint states/formats/
volumes/mutes and sessions match the pre-handoff baseline, with no snapshot errors.
Guest persistent-state files match exactly; its graph exact comparison differs
across the physical handoff. Sixteen machine-audio registry values differ; user
audio/policy trees match. Preserve the full private differences with no rewrite.

Hardware journal SHA256:
`850c6ffc9dcf9fee4f7fcd049ca13522d9c142ae87dbdb71f4c6853dc089917a`.
Guest return comparison SHA256:
`54133a497ac163262f0c1a5466de924b81f72e4b6deeb3808d16824e7a995e8c`.
Windows return snapshot SHA256:
`4a7f5e4a65a74d5ef86084fb4fbdbbf3c288a632d0506a799ff6c5c26da9a041`.

Before another protocol test, inspect the stock reader's behavior for ACFFFFFF;
do not silently classify it as no-payload or weaken unknown-I/O handling.
The owner requested direct handoff automation, then explicitly prohibited the
computer-use skill and mouse/UI automation. No UI input was sent. VMware vmrun
1.17.0.25688693 exposes named-device commands, but live USB support has not been
verified. The native computer-use pipe was unavailable and that path was stopped.
Use direct controls only. A connected-state test baseline can remove per-attempt
physical handoffs while retaining full snapshots, claim cleanup and exact local
comparisons; perform the host-return comparison once at session end.

## Connected-state test workflow

The owner explicitly prohibits computer-use/mouse automation. Native UI
inspection sent no input and stopped; no foreground/mouse automation is used.
Direct VMware USB switching remains unverified. Instead, leave Studio in the VM
throughout a testing session, release only the owned vendor-interface claim after
each attempt, and perform the physical return/host comparison once at session end.
This avoids per-test switching without claiming an automatic VMware USB API.

The shared runner now implements `prepare-connected --sysfs-device <selected>`
and `compare-connected`. The publication runner delegates those stages to the
existing owner. Preparation still saves full desktop graph and tracked persistent
contents/modes/hashes. It reuses `usb_descriptor_probe.sample` for bounded cached
device/interface observations before and after the snapshot: exactly one selected
target, configuration 1, alternates zero and vendor interface unclaimed. Its
comparison stamp is not claim/deletion authority; native transfers retain their
separate selected-handle/descriptor/identity gate. A later connected run refuses
a changed stamp. Comparison requires the same released profile/stamp across its
snapshot, confirmed child completion, and confirmed private-process cleanup.
Missing completion journals after a started attempt now block both comparison
modes rather than allowing a premature cleanup claim.

Every attempt still uses a fresh exclusive evidence directory, pinned binary,
ten-minute baseline limit, one used-attempt marker, bounded child supervision and
no automatic retry. Connected comparison writes `connected-comparison.json`, so
the final host-return comparison can be preserved separately. Graph or persistent
differences are saved without rewriting concurrent work. Default pre-handoff
prepare/returned comparison behavior stays intact. Keep the full pre-handoff
baseline at session start: an attached baseline does not cover the physical
handoff or prove whole-system rollback after it.

The ignored private `local/connected_usb_attempt.py` composes fresh connected
prepare, a single authorized observed-startup publication run, connected compare
and host connected snapshots via strict SSH. It never switches USB, executes
Windows Link, invokes UI automation or retries a failed native attempt. Unknown
outer execution remains unconfirmed and is recorded for review. Its read-only
`check` verified the final executable/runner pins and no guest target; no connected
protocol attempt was run. Hardware qualification of the new prepare/compare stages
remains pending until the next useful protocol test.

Linux passed 22 focused Python runner tests; Windows passed 20 and skipped the
two Linux-only pipe-supervisor cases. Added cases cover exact connected snapshots,
missing/ambiguous/claimed/replaced targets, invalid baseline scope, preserved
concurrent graph changes and missing completion journals. Exact native periodic
and one-shot examples were rebuilt. Rust behavior is unchanged from the previous
31-case startup matrix; no redundant routing or USB experiment was run.

Tested code SHA256:
`0037b100ac54795ebfed90f4bc5fb26030e2f728ae1bf736618ee548bbee0c89`.
Source archive SHA256:
`0e99606183c7a62f74f37fa70a4995ffba7a1d5b53238fbac0bfc9d364b6d115`.
Linux build/runner-check journal SHA256:
`d14654c9e4d7cffa7189c21f0c7b329698a0d5288e56228b8ed3cd5eece14ea9`.
Periodic executable SHA256:
`760b847e9bbd290a88d996e2e052df2ad4e9dfedb431e40bbb8a4d1f29d7cfbe`.
Publication runner SHA256:
`ca3dede64a7ab9f4a48c35bbc0dc59122ef5fd89535b1bb03bc20a355b6c0b5b`.
Shared runner SHA256:
`bc30dce277b0c272b1b27b37c4ab05b7a788aacaa2dc9720db0b173fc52aafec`.
Latest guest free disk: 20,746,448,896 bytes. Prior attempts and settings evidence
remain preserved; no VM/toolchain/configuration changes or new software installs.

## Bounded rejected-marker observation

The stock reader's ACFFFFFF branch releases its critical section and returns
without parsing records, updating the callback counter or stopping the worker/
publication timer. See [protocol-session.md](protocol-session.md) for exact binary
provenance and preferred-VA/file-offset anchors. This supports investigating
scheduled continuation; it does not establish the marker's firmware meaning,
peer health or authoritative empty assignments.

The explicit `PollPolicy::ReportRejectedMarker` exists in both the existing
session and bounded selected-device owner. Default decoders/controllers, one-shot
publication and bounded-strict policies remain unchanged. An exact marker from a
successful 4..512-byte transfer becomes a separate typed observation with only
the ignored tail length. It is never routed or treated as NoPayload/assignment
zero. Each observation consumes one cycle; subsequent publication still needs
fresh, strictly increasing inventory with the same owned live application/streams.
Policy mismatch, identity change, malformed/unsupported frames, short writes and
uncertain native outcomes stop further submissions. No failed native I/O is
retried, no recovery command is sent and no bulk deadline is increased.

`native_usb_periodic --authorize-periodic-observed-polls <sysfs-device>
<expected-cache> <cycles>` explicitly selects observed startup and marker policy.
The runner requires `--cycles 40 --observed-startup --observe-rejected-polls` on
an authorized hardware run; it refuses marker observation for source-only,
metadata and one-shot/default invocations. The example reports rejected-marker
observations separately, keeps known assignments tied to fresh inventory and
stops publication on a bound Link 1..4 request without claiming routes applied.
Its cycle spacing is at least the diagnostic 500 ms wait plus inventory/I/O work;
this is not a measured reproduction of Windows callback cadence.

Windows passed 38 focused Rust integration cases and 21 Python runner cases (two
Linux-only cases skipped). Linux passed all 38 integration cases, two native unit
checks and 23 Python runner cases. Default build and exact native periodic and
one-shot examples built. New checks cover exact/short/oversize/nearby markers,
marker then assignment, budget exhaustion, stale/rebound source, controller/gate
policy mismatch, and partial/unknown/malformed failure after a marker without
another submission. No redundant routing probe, capture, device or UI action
occurred during implementation/validation.

Tested code SHA256:
`a94df6031f9bf4c1799802f3a52da05816320b7ca08a253dbfb66162433a0a75`.
Source archive SHA256:
`bb9722c6db69432b345f105f66ce025738e49e042abcd7f373d63f19ef95e095`.
Linux build/focused-check journal SHA256:
`74f43ea70bf3466b47bf8a1cae2a58a1ef3491d5e07a8ee72a7b100b8903aa75`.
Periodic executable SHA256:
`f3bf1439c95299eaea640b02914a1f56731d6d20af63caf596c667ff844f88e7`.
Publication runner SHA256:
`fca1e7044c33e3f9fa90c1749857f266b0b2ace6586b0d202900a88249e6f140`.
Shared runner remains
`bc30dce277b0c272b1b27b37c4ab05b7a788aacaa2dc9720db0b173fc52aafec`.
Latest free guest disk: 20,468,834,304 bytes. All prior artifacts remain intact.

Full pre-handoff host/guest backups are saved privately. The ignored
`local/marker_connected_attempt.py` pins this source, serves fresh connected
prepare/run/compare and host snapshots via direct SSH, and opts into marker
observation. Its read-only check verified the pins and target count zero. Once
the owner connects Studio, leave it in the VM across tests; owned claim cleanup
and exact connected comparisons replace per-test physical switching. Save the
session-end host return comparison separately. No mouse/UI automation is allowed.
Actual connected-stage/marker hardware validation remains pending; sustained
visibility, assignment ingestion and audio are not established by these tests.

## Initialization deadline diagnostic

The first connected marker-policy hardware attempt on 2026-10-13 completed A0/A1
but timed out on A2: libusb -7, transferred zero, requested 512. It published
nothing. Claim release, private-process cleanup and all connected local comparisons
passed. Journal SHA256:
`4fb565a84a5e5e87921062da633858453c259c81066900a5639cc0dc2d48d6bc`.

The explicit `--authorize-periodic-initialization-diagnostic` native mode selects
observed startup, typed rejected-marker observation and a fixed five-second A2
read. Runner `--initialization-diagnostic` requires `--cycles`, `--observed-startup`,
`--observe-rejected-polls` and the existing explicit hardware authorization. It
refuses source-only/one-shot/metadata combinations. A0/A1, publication writes,
poll writes and subsequent reads retain 500 ms limits. The initial native read
allowance is consumed before the call; no failed transfer is repeated. Positive
timeout, phase, count, header, device identity and inventory gates remain intact.
The only added native diagnostic contains elapsed time, deadline, error code and
transfer counts; A2 metadata and names remain redacted. See
[protocol-session.md](protocol-session.md) for the focused Windows timeout evidence.

Windows and Linux passed 42 focused Rust integration cases: session 15,
handshake 11, publication 16. Linux also passed two native ABI/ownership unit
checks and 24 Python runner cases; Windows passed 22 with two Linux-only skips.
Default build and both exact native examples passed. New cases prove that the
longer allowance applies only to opted-in A2, never writes or later reads; default
and over-limit calls refuse before backend I/O, zero timeouts submit nothing,
and failed initialization never retries. Formatting and diff checks passed.

The first diagnostic hardware attempt received valid A2 in 669 ms and completed
40 cycles: seven NoPayload replies with 508 ignored bytes, 33 exact rejected
markers with zero ignored bytes, 81 fresh inventory requests, no records. This
proves that a valid initial reply can exceed the default 500 ms in this VM.
It does not establish a firmware five-second requirement or marker meaning.
Its journal SHA256:
`cc902e54a418a7920e5f3a5b253767447773bd0e5cb66f044bb0c34fd052d5d9`.

Two subsequent observation attempts received A2 in 439 ms and 584 ms and completed
all 40 cycles. The first had two one-record replies, neither matching the bound
test name; names were redacted and their identity is unknown. The second had no
records. The owner confirms seeing BEACN Linux VM Test, with offered destinations
Aux 2, Music, System and Aux 1; Link In was unavailable. The owner did not attempt
the suggested Aux 2 drag. Remote source classification and correct assignment
mapping are unresolved; no assignment ingestion or audio acceptance is claimed.
Observation-attempt journal SHA256 values, respectively:
`2de8408cc613fbf5f8e3f0275b5ad542899024192540c079792fb37c351ed953` and
`5d3c6241055c4eba7c0be6c97b1b1fd2fbc31b9147c445eccab85fc831e1ea47`.

All four attempts used fresh connected guest/host snapshots and separate exclusive
evidence directories. Owned claims released, source identities remained valid,
both private processes stopped, and private-core cleanup plus guest binding,
desktop graph/state and host audio/registry comparisons passed exactly. Connected
comparison SHA256 (identical content across these attempts):
`b3a65323bf71a29726810720d99a614578885fa6d62a1d6f41b21ccf4d56641f`.
These are local comparisons, not streaming-PC settings comparisons or proof of
remote entry withdrawal. Studio remains in the VM; no physical handoff, mouse/UI
automation, capture or audio routing occurred. Session-end Windows return
comparison remains separate. An attached baseline does not cover the handoff.

Tested code SHA256:
`4feac3d93745977de2855347ed883700cc09cea2608991e320273edc61e6c867`.
Source archive SHA256:
`29ff14882653b68548e3687262fdf1f5cd3a602e460cb0713f3c1c998b1e785b`.
Linux build/focused-check journal SHA256:
`b5637edfef361b421da1b2b3eacaa68d2c6f1314d61c842d1dbd212d335db60d`.
Native periodic executable SHA256:
`ea8f2bebdcec5e3dc7986bdaff18899df070f0ce47a180adccf8aedada1d972c`.
Publication runner SHA256:
`5e2037970b8fa22a379e2940f7ad7719537ff06fd29edad8a84ca94243c7d70d`.
Shared runner SHA256:
`bc30dce277b0c272b1b27b37c4ab05b7a788aacaa2dc9720db0b173fc52aafec`.
Guest source: beacn-link-usb-publication-initialization-20261013T131603Z/source.
Latest free guest disk: 20,186,005,504 bytes. Prior archives and private settings
evidence remain intact; no toolchain install, VM configuration change or workflow.

For another UI experiment, first state the exact target and reason, wait for owner
readiness, announce the run start/end, and obtain the observation while it is
fresh. Do not prescribe arbitrary mixer channels as a substitute for the Link
acceptance path. Add redacted per-record assignment/name-match diagnostics before
testing another changed hypothesis; current count-only logs cannot resolve it.

## Redacted reply diagnostic preparation

The periodic example now reports each of the first four decoded records' assignment
byte, name byte count, exact bound-name match, embedded-NUL count and printable-ASCII
status. It redacts names and counts omitted additional diagnostics. This bounded
output can distinguish unmatched names from matching opaque assignment values
without introducing aliases, interpreting zero or exposing other app names.
Decoding, planning, fresh inventory checks, cycle budget and transfers are unchanged.
The startup line now accurately distinguishes the selected initial-read deadline
from the unchanged 500 ms writes/later reads.

The exact native example compiled offline on Linux; formatting and diff checks
passed. This output-only change did not justify rerunning unchanged test matrices.
It has not been exercised on hardware. Its guest source is
beacn-link-usb-publication-reply-20261013T133631Z/source. Code SHA256:
`70f986d3a5ee39e58331cc294834bc852406c828b4e7adea98c2d455fdaf2dc0`.
Archive SHA256:
`67e4f16fcba453f5753e84b2583e5d898e3097b3bccbca38192730b56dd39078`.
Linux build journal SHA256:
`24b565e9325e9dd714a044354b6045712e037cef26f5740cb60d579b469f3c09`.
Native periodic executable SHA256:
`13fee714f2bd03c076f118bf01833892c95532a334b66be686205bfe5efeb923`.
Guest free disk after the lightweight build: 20,088,913,920 bytes. Earlier code,
private evidence and settings snapshots remain preserved. Keep the Link routing
acceptance target; the offered Aux/Music/System choices are not yet mapped to it.

## Embedded-NUL reply investigation

The owner-ready diagnostic run again showed the test app with Link In unavailable.
Native A2 completed in 859 ms, followed by all 40 cycles and 81 fresh inventories.
Three one-record replies each reported assignment 1, 25 name bytes, one NUL,
non-printable-ASCII status and no exact bound-name match. These are raw assignment
records, not resolved assignments for the test source. Journal SHA256:
`f0c39fd2ee4ed1209e3874db74a8237e72da75e78b228a34dc3ecd6c8e86dc62`.

The example now additionally reports first-NUL position, opaque bytes after it
and an exact comparison of the preceding bytes to the owned advertised name.
It preserves raw decoding and strict planner matching; the extra comparison is
diagnostic only. A separate passive run completed A2 in 534 ms, all 40 cycles
and 81 inventories but received no records, so it could not test the new
comparison. Journal SHA256:
`5ee09430d6818b674865fcbbfdd7c90ec68c69d8002279bee801e85df26cd295`.
Both attempts released the claim, preserved the source, stopped both owned
processes, restored the private core and passed every connected local comparison.
No streaming-PC profile comparison or Link assignment/audio acceptance is claimed.

The exact new native example built offline on Linux. Formatting/diff checks
passed; no unchanged matrix rerun. Code SHA256:
`b63f3abe733562b295e696e42f892580c42b29bded4e2f4d3b98a079672759a0`.
Archive SHA256:
`f33d05458fb5d25e0670e2b8f64c46af15efc9f489287c1e53fd10491014a64d`.
Linux build journal SHA256:
`33d13865137e44adbcc119564b353911e7d831fc2fd1e20f5cd9d9e3782aa9bd`.
Native periodic executable SHA256:
`acfced433b5566b05ec4b06916b70a3d4ac807ba4b63f733f9d8a03fb16f77b6`.
Guest source: beacn-link-usb-publication-reply-nul-20261013T134722Z/source.
Free guest disk after build: 20,000,448,512 bytes. All prior evidence is preserved.

Next: one announced owner-ready interaction to elicit a record with the new
comparison, then assess whether explicit Windows string compatibility is justified.
Do not bind a record based on name length, NUL count or assignment byte alone.

## Explicit received-name compatibility and hardware assignment resolution

The next owner-ready prefix run received eight assignment-1 records, each with
NUL at index 19, five opaque bytes after it and an exact prefix match to the owned
19-byte name. It completed all 40 cycles and 81 fresh inventories; A2 took 484 ms.
No matching rule was changed during this diagnostic. Journal SHA256:
`9cbdbd98780d4544c1ca9e9ddc13920b6797183a035f8673256174a0ef6eee2b`.

`session::ReceivedNamePolicy` now provides default ExactBytes and explicit
WindowsCString planning adaptation. The raw wire record remains unchanged;
existing ASCII validation, exact inventory name binding, lifecycle guard and
planner resolution rules still apply. The opaque suffix is not parsed as device
identity, PID or a routing command. The explicit native mode
`--authorize-periodic-c-string-names` selects this policy along with observed
startup, marker observations and bounded initialization. Runner flag
`--windows-c-string-names` requires the existing initialization-diagnostic periodic
hardware options and is refused in default/source-only/nonhardware modes. The
chosen policy is recorded privately. All default modes still match raw bytes.

Windows/Linux passed all 19 session cases, including four new tests of default
and raw preservation, normalized-name collisions, opaque/unknown values,
unsupported prefixes, stale streams and stale plans. Linux passed all 25 Python
runner cases; Windows passed 23 with two Linux-only skips. Default and exact
native periodic builds passed, along with formatting and diff checks. No
unchanged full routing/USB matrix was repeated.

The separate owner-ready compatibility run received a bound assignment 1 at
cycle 5. The existing planner produced one proposal for the owned application
and its live private-core stream. Publication stopped rather than resending
unapplied zero; the child held five seconds, required one more fresh inventory
and cleaned up. It made 12 fresh inventory requests. A2 took 535 ms. The owner
reports the entry disappeared while dragging; successful UI drop/retention is
not established. This proves hardware assignment ingestion and planner
resolution, not actual native route application or audible interoperability.
Journal SHA256:
`c7df87df0d415bd55a4dd101c1b89d249fd3aa1e000b2b68e20cca434b93e9f6`.
Both runs preserved the source, released the claim, stopped owned processes,
restored the private core and passed every connected local comparison. The
streaming PC's state remains outside those comparisons. Studio stays in the VM.

Tested code SHA256:
`957d840c004b09e6a0783eb4c8e9215925e3012b052fd4729336c82beb2acc3a`.
Archive SHA256:
`1819546ff2eaf3dcaac6fd554575bcfe5fcb0ac479a08a4509285effb90ed3a6`.
Linux build/focused-check journal SHA256:
`97dfac66424deb1f068a7e89aea12b1e4d132222d261702886ca7d16d834ffd6`.
Native periodic executable SHA256:
`297fdee5cc3de0c68650331837f9b01261ffcda342986f0add8e5924afa975f9`.
Runner SHA256:
`411ef1cab9939c9035ee606a5fc9c6a024eef6382093e7579c1d8c33fd53b6b8`.
Guest source: beacn-link-usb-publication-c-string-20261013T135803Z/source.
Free guest disk: 19,808,038,912 bytes. All earlier evidence remains intact.

Next: extend test lifetime and integrate the existing native executor on owned
private-core destinations. Continued publication must report only a freshly
verified actual assignment, using existing identity and receipt owners. It must
not silently acknowledge a desired route or overwrite it with unapplied zero.
Virtual private-core links remain distinct from physical Studio audio acceptance.

# Selected Studio playback probe

Updated 2026-10-14.
This is an explicitly authorized physical feasibility test, separate from the
[native private-core executor](native-executor.md) and
[accepted assignment/publication checkpoint](native-assignment-publication.md).
Ordinary desktop apply remains disabled. No audible acceptance is claimed.

## Current owner plan: direct Debian laptop comparison over SSH

The owner volunteered a separate Debian 13 laptop after the VMware failures.
The active agent remains on Windows with authenticated, privately pinned SSH
access to the laptop. No gaming-PC migration/reboot or replacement agent session
is required. Initial inventory reports kernel 6.12.111, PipeWire 1.4.2 and
WirePlumber 0.5.8. After the owner corrected the cable connection, Studio is
visible as 33AE:4003 at 480 Mbps with snd-usb-audio and an eight-channel FLOAT_LE
PCM. The full graph/settings and persistent-state copies/hashes are preserved
privately. Native direct ALSA uses its actual card without importing VM UCM
assumptions or forcing desktop rates.
GNOME remote login is configured for owner access and its RDP/TLS connection
and interactive owner login have been verified. Keep ordinary physical apply
disabled and all VM evidence retained.

## Native 96-kHz silent PCM completed cleanly

The native-only direct-card diagnostic pins the selected card/device identity,
requires closed selected PCMs and an unchanged foreign graph, and preserves
other audio devices. The original VM UCM/isolation gate remains the default.
The 48-kHz native run stalled at pointer zero but cleaned up without blocked
processes or settings changes. The first 96-kHz observation advanced but exposed
a harness refusal of normal DRAINING. DRAINING is now permitted only after
sufficient validated queue progress, with exact child ownership and bounded
natural completion; a closed PCM may settle while that child exits.

The corrected four-second 96-kHz run exited 0 after 18 observations, with pointer
1,152 -> 380,336 and exact desktop/settings/routes/saved-state/USB comparisons.
Complete persistent-state hashes also match. Journal SHA256:
`6fb895b7eb9366b2aa32d02ee22bff6b5ecdec5c48b56ed766265ab49ac70b1c`.
No independently observed USB delivery or audible acceptance is claimed. The
rate diagnostic uses the earlier physical clock observation; a current-rate
read was refused EBUSY with no interface claim/detach/reset/write.

An explicit audible first-pair mode is prepared: three 350-ms 440-Hz tones at
no more than -24 dBFS on channels 0/1, one-second silent lead and quiet tail,
all remaining channels silent. It needs separate tone authorization and owner
listening readiness. The readiness question is pending; no audible run started.
The exact Debian native build and private routing/recovery check also pass;
see [HANDOFF](HANDOFF.md) for the separate private-core evidence and next steps.

## Latest VM result: accepted enqueue without OUT completion

The 17-probe owner run passed loading and aggregate validation. USB submit, HCD
submit and EHCI enqueue each returned zero for all three OUT and 15 IN entries.
OUT still produced no givebacks/completions; IN produced three completions,
24 successful packets and 7,232 bytes at 96000 Hz/416-byte geometry. Three OUT
buffers total 298 queued frames, matching the stalled ALSA pointer. Zero return
proves software enqueue acceptance only. Cancellation returned zero for the
outstanding three OUT/12 IN URBs; a later OUT -16 return means already unlinked.
Owner reconnect then restored all guest logical settings/routes/state-file checks.

One initialized vendor-helper coexistence attempt at 96 kHz also stalled at
pointer 298. The helper's claim release succeeded, but full baseline restoration
failed because audio remained active and the owned ALSA child stayed in
D / usb_kill_urb. No further VM playback is justified. Check read-only recovery
after the owner's physical move. Exact evidence hashes and the preserved host
differences are recorded in [HANDOFF](HANDOFF.md).

The ALSA runner now accepts a caller-owned held helper only with a complete
positive PID/start pair and exact current process identity/selected-device FD.
It shares the existing binding checks and rechecks retention during activation;
it never stops/releases the caller's helper. All 62 focused Linux cases passed
on exact source `beacn-link-physical-playback-20261014T173834Z/source`.
Unchanged native inputs reuse the previously validated CLI. The laptop's actual
ABI and endpoint compatibility remain to be checked independently.

## Handoff reached EHCI entry; acceptance return still unknown

The twelve-probe run loaded/exited cleanly and passed aggregate review. OUT had
three USB-submit/HCD-submit/EHCI-enqueue entries, zero givebacks/completions and
one kill entry at relative second four. IN had 15 entries at each submission/
enqueue stage, three givebacks/completions, 24 successful packets (six empty,
18 nonempty), 7,232 successful bytes. Rates were all 96000 Hz, current/max sizes
416 bytes, with no packet errors. The three eight-packet OUT buffers were
3,072/3,136/3,328 bytes, interval one. Summing them gives 9,536 bytes or 298
eight-channel FLOAT_LE frames, matching the stuck ALSA pointer. Submission entry
and this queued-frame count are not accepted transfers or independent delivery.
Summary SHA256 `7e20fb44aa455262ddc4a2b2b6d21c011f0a76e838609729945fe2ae9dc08d93`;
journal `1eb87c89d4fcf257b42615b88e519b9843d4e82dd7aff52cbf265222714a7b4e`.

Owner reconnect released the blocked aplay process. Linux persistent files and
node/port/link/device settings matched, but live default sink moved Link 1 -> Link
4. After a fresh full snapshot and exact current-value/unique-target revalidation,
only that default was restored through normal-user metadata. All logical metadata/
settings/routes and persistent-state comparisons then pass. Thirty-five recovered
files are retained/hash-verified; recovery SHA256
`b49346de6fdb23ec13f0515112ad80290aa04d441861a8cd81fa48f14e5f4001`.
Windows audio matches before/after the attempt and recovery; registry matched the
attempt, but 18 machine-audio differences remain preserved/unclassified after
reconnect. No unrelated host state was restored.

The [EHCI implementation](https://raw.githubusercontent.com/torvalds/linux/v7.0/drivers/usb/host/ehci-hcd.c)
describes a non-error enqueue return as a promise of later giveback, and ISO
dequeue waits for ordinary completion/time-slot expiry. These upstream contracts
do not establish what the guest/VMware actually did. Optional
`--observe-usb-returns` now adds four typed function-return probes plus an EHCI
dequeue entry. [bpftrace fexit](https://bpftrace.org/docs/release_025/language#fentry-and-fexit)
exposes typed function arguments at return, avoiding cross-thread/CPU pairing.
Guest BTF confirms all urb arguments. Only bounded return codes and successful
enqueue packet/buffer/interval/start-frame metadata are exported; no payloads or
addresses. Zero return does not prove delivery. The decoder validates 17-probe
coverage, entry/return/geometry counts, errors and scheduling bounds. Older probe
formats keep their meanings. New fexit kernel loading remains unverified and must
pass the matched audio-free preflight before a bounded child. No automatic audio
fallback. The ALSA journal now labels queue progress explicitly; ordinary apply
remains off.

All 60 focused Linux cases pass on exact source
`beacn-link-physical-playback-20261014T172651Z/source`, reusing unchanged validated
Rust inputs. An inert fexit-only code-generation check stopped at tracefs permission
discovery, before compilation; no attachment/audio occurred. The full 17-probe
load remains pending owner-terminal sudo. Fresh snapshots and a byte-verified
launcher `beacn-feedback-ready-20261014T172808Z/run.py` are staged, with ten-minute
expiry and source/settings/generation guards. Keep Studio attached when running.
No root/audio execution or accepted-enqueue/delivery result is claimed yet.

## Handoff launcher refused an additional reconnect

The owner reconnected Studio after launcher 170306 preparation, changing its
pinned USB generation. Both commands refused before creating test evidence.
Read-only verification confirms unchanged source, nonexpired baseline, released
interfaces, no owned audio/observer process and matching logical settings/routes/
metadata/device settings/persistent files. The opaque RuntimeError output was a
private launcher reporting defect, not a probe-load or playback outcome.

The replacement prints specific controlled refusal reasons while retaining
generation/settings/source/expiry/one-attempt guards. Fresh snapshots and a
byte-verified launcher are staged at `beacn-feedback-ready-20261014T170813Z/run.py`
on unchanged validated source 170230. Leave Studio attached for execution;
do not reconnect between preparation and launch. The twelve-probe load and USB
handoff measurement remain pending. Earlier launcher 170306 is superseded.

## Matched-clock input and remaining controller-handoff gap

The seven-probe 96-kHz run measured matching device/source/sink rates and
416-byte current/maximum packet sizes. Three IN completions contained 24 successful
packets: six empty, 18 nonempty, 7,232 successful bytes. Requested lengths were
416 bytes; no packet errors were observed. All three sync callbacks occurred in
the first relative second. No OUT completion was observed. This startup differs
from the 48-kHz overflow case, but it did not sustain playback: pointer 298 -> 298,
then the owned aplay process blocked in D / usb_kill_urb after termination.
Summary SHA256 `637baf5ed39d8cbd6864d94f0fbdbba5b05229f378fbd1f19146edb241f8fb21`;
journal `82c6231978195faf0f1eccfce426199e3108e877c7a141add0a9e11ba9df6ee7`.

The [upstream endpoint implementation](https://raw.githubusercontent.com/torvalds/linux/v7.0/sound/usb/endpoint.c)
queues output lengths from successful nonempty implicit-feedback input. The
[USB core](https://raw.githubusercontent.com/torvalds/linux/v7.0/drivers/usb/core/urb.c)
waits in usb_kill_urb until the URB use count reaches zero. These references
motivate inspecting submission/giveback, but do not establish the Ubuntu/VMware
root cause. Output submission, enqueue return and controller progress remain
unmeasured; ALSA's queue pointer is not independent delivery evidence.

Owner reconnect recovered the selected device. No owned audio/collector process
remains, both PCMs are closed, all logical nodes/ports/links/metadata/device
settings match and persistent files match byte-for-byte. Thirty recovered files
were copied/hashed; recovery SHA256
`d1d2a74b28fc5a8b411dffba1b55a479af66b96895cb6f523d1bdfea615eee0b`.
Windows endpoint settings match. Full audio and 17 machine-audio registry
differences remain preserved/unclassified after reconnect; no blanket restore.

The optional `--observe-usb-handoff` adds bounded selected-device ISO entry counts
for usb_submit_urb, usb_hcd_submit_urb, ehci_urb_enqueue, usb_hcd_giveback_urb and
usb_kill_urb, plus requested packet-count/buffer-length/interval aggregates.
Endpoint/type come from urb->pipe, since usb_submit_urb initializes urb->ep after
entry. No payload or addresses are exported, and entry never means accepted
submission. The selected controller must be EHCI. All five guest symbols exist;
the new twelve-probe kernel load remains unverified. The same matched-flag,
audio-free preflight gate must pass before one authorized 96-kHz silence child.
Full baseline objects are saved before audio for post-reconnect comparison.
The decoder validates coverage, bounds and per-endpoint geometry accounting;
legacy five/seven-probe observations retain their original meaning. Fifty-seven
focused Linux checks pass on exact source
`beacn-link-physical-playback-20261014T170230Z/source`. A byte-verified launcher is
staged at `beacn-feedback-ready-20261014T170306Z/run.py` with fresh host/guest
baselines and ten-minute expiry. No root/audio execution has started. Ordinary
physical apply remains off.

## Matched-rate recovery and next packet check

The owner's single VMware reconnect released the blocked aplay process. Both
PCMs are closed; released interfaces/descriptors/attributes match. Fresh logical
nodes/ports/links/defaults and persistent-state bytes match the pre-attempt baseline.
USB generation changed, so old IDs are not reused as authority. Nineteen recovered
artifacts were copied/hashed; recovery SHA256
`2d71482cc7bdb498dff7f0d06beb99c76a4dbdf2728b18358d45bbe887e9ac30`.
Windows endpoint settings/defaults match, while one session list and 16 machine-
audio registry binary values differ. Preserve these unclassified differences;
do not restore unrelated host state or claim complete host byte equality.

The 54-case validated source has a staged, byte-verified matched-clock launcher
`beacn-feedback-ready-20261014T164723Z/run.py`. It verifies the old clock report,
fresh released binding/settings and source hashes; baseline expires at ten minutes.
It has not started root/audio execution. Next observe configured/device rates,
requested sizes and packet errors at 96 kHz, then compare settings/owned cleanup.
The earlier pending-recovery statements below are historical. Ordinary physical
apply remains disabled; no audible acceptance.

## Measured 96-kHz device clock and matched-rate failure

The corrected seven-probe owner-terminal run completed without warnings or
stderr. The existing clock-41 getter returned **96000 Hz** while both driver
endpoints were configured at **48000 Hz**, with 288-byte current packets and
416-byte maxima. Requested lengths account for all 11,472 IN packets: 11,308
EOVERFLOW, 158 EXDEV, six successful empty packets. There were 1,434 noncancelled
IN completions/sync callbacks, zero successful bytes and no OUT completions.
ALSA pointer stayed zero. This establishes a device/configuration mismatch,
without establishing its causal role or the received sizes of failed packets.

The run's immediate/fresh guest comparisons pass exactly. Windows registry and
endpoint settings/defaults pass; full audio differs only by three disappearing
sessions across two endpoints, with surviving session values unchanged. Preserve
the private differences. Twenty-nine result files were copied/hashed, summary
`8c390897aed8710e1a649e458c5bd1bc549f63b9394c0d407e18e0a8e2d23c81`,
playback journal
`52a3be1279c3df43c05377487c0265141c0fa7b6f38962d0cc3fa43c0e596c9d`.

`studio_alsa_playback.py --matched-device-clock-96000` is an explicit diagnostic
opt-in; default remains 48000. It authors four seconds of eight-channel FLOAT_LE
silence, disables all ALSA conversion and verifies the requested actual hardware
parameters. Unsupported/noninteger rates are refused before allocation. Existing
device/PCM ownership, graph, duration and cleanup guards remain. A private wrapper
pinned the previous measured-clock report and selected binding before one test.

Matching 96000 did **not** solve playback: pointer 298 -> 298 across ten samples,
followed by an owned aplay process blocked in D / usb_kill_urb after termination.
Windows audio/registry match the new baseline exactly. Guest restoration remains
unverified because its binding is not released. No further playback may run until
the owner reconnects Studio and fresh recovery verifies owned exit and baseline
preservation. Eighteen private files were copied before recovery; journal SHA256
`0e54af353e5d89032204d069004830b19378d9bce1af3413c159c20c0daabead`.

Next, after recovery, use the same aggregate observer's explicit matched-clock
mode to measure actual device/configured rates and requested sizes at 96000.
Its summary now exposes playback_error/playback_child_exited separately from
observer success. No new driver/firmware changes, payload capture or ordinary
native physical apply. The earlier pending-observer notes below are historical.

## Failed clock/geometry load and corrected checker

The owner-terminal clock/geometry run reached an attachment-check exit 0, then
the actual collector exited 255 before audio. No launch marker or playback
journal exists. Its stderr reports -12 (ENOMEM) while loading the
snd_complete_urb BPF program. All immediate/fresh guest settings/file/binding
comparisons and Windows audio/registry comparisons pass exactly. No owned audio
or collector process remains. Ten private artifacts were copied/hash-verified;
summary SHA256:
`8ce2438793f3d185da558a9954049acf6dec8a97e86895ad89a2339daae604b9`.
No clock or requested-size observations were obtained in this run.

The checker had omitted `-k` and the `-c` child context used by the actual run.
The [bpftrace CLI](https://bpftrace.org/docs/release_025/cli) describes `-k` as
runtime probe-read/map-helper warning instrumentation. One shared command builder
now supplies both phases' options; only `--dry-run` differs. The
[versioned implementation](https://raw.githubusercontent.com/bpftrace/bpftrace/v0.25.0/src/bpftrace.cpp)
returns on dry-run before executing a non-USDT child. These scripts use kernel
probes only. A failed checker may collect one verbose, audio-free load diagnostic
before stopping; it does not retry audio, suppress warnings or change limits.
The exact allocation/verifier/JIT cause of ENOMEM remains unknown.

The observer now handles at most eight packets per callback. All 1,437 measured
noncancelled callbacks in the earlier run account for eight packets each. The
[upstream endpoint setup](https://raw.githubusercontent.com/torvalds/linux/v7.0/sound/usb/endpoint.c)
also caps capture/implicit-feedback URBs to one millisecond: eight packets at the
selected endpoint's 125-us interval. Larger counts remain unsupported. Narrowing
the 64-copy unrolled body reduces static work, but runtime load/memory improvement
has not yet been demonstrated. The decoder enforces eight for new runs and
retains the historical 64-bound input contract for old evidence.

Nonzero collector exit now reports a non-null error. A partial launch marker
without a journal stays unknown. Collector completion remains independent of
playback success. All 51 focused Linux cases pass on exact source
`beacn-link-physical-playback-20261014T153912Z/source`, including the flag/context,
exit-status and packet-limit regressions. The unchanged Rust CLI is reused with
provenance. The corrected owner-terminal launcher is staged/byte-verified at
`beacn-feedback-ready-20261014T153914Z/run.py`; a fresh Windows baseline is retained.
Kernel load and the clock/geometry measurements remain pending. Studio stays
attached, with ordinary native physical apply disabled.

## Live aggregate feedback evidence and clock/geometry preparation

The owner-terminal run compiled and attached five probes. Collection exited 0,
but the normal-user silent ALSA child still stalled at pointer zero for two
seconds, then exited through owned cleanup. Nine samples retain pointer 0 -> 0.
The collector's `error: null` reports collection completion; the nested playback
journal explicitly records failure. No audible or physical executor acceptance.

| Observation | IN `0x82` | OUT `0x02` |
| --- | ---: | ---: |
| Noncancelled URB completions (URB status 0) | 1,437 | 0 |
| Cleanup cancellations | 12 | 0 |
| Packets with EOVERFLOW (-75) | 11,490 | 0 |
| Successful packets, all empty | 6 | 0 |
| Successful bytes | 0 | 0 |
| Sync-handler calls | 1,437 | — |

URB status 0 does not mean each isochronous packet succeeded. The
[kernel error-code contract](https://docs.kernel.org/driver-api/usb/error-codes.html)
describes per-packet errors and EOVERFLOW as data exceeding a packet/buffer limit.
The [v7.0 EHCI implementation](https://github.com/torvalds/linux/blob/v7.0/drivers/usb/host/ehci-sched.c)
maps its babble bit to this error and deliberately leaves actual_length unchanged
on babble. Thus failed-packet zero lengths cannot reveal received packet size.
The upstream implicit-feedback handler counts only successful nonempty packet
lengths before preparing output. These observations support its queue remaining
empty; the layer causing the reported babble remains unknown.

Two signedness warnings were printed to stdout, although stderr is empty. The
positive eight-packet counts consistently account for all observed callbacks;
retain the warning rather than claiming clean compilation. The loop now casts
packet count to uint64, with the same 1..64 guard. No unsupported-packet, invalid-
length or window-limit counter was observed in the retained run.

Immediate/fresh guest comparisons and Windows audio/registry comparisons all
pass exactly within this run. Read-only recovery finds no aplay/bpftrace process
and responsive existing desktop services. Twenty-seven result artifacts are
retained/hash-verified outside Git. Summary SHA256:
`ff043e7ed1ea811ded5332f445c5a6ed87cdee671a275d9e20f4e51f5c6dae70`;
playback journal SHA256:
`88a369d5b8795eee57933e05ed1f92794f91c1946d1a7c5dc1b14764eadde57c`.

Cached descriptor review finds clock 41, attributes 3, controls 0x05, shared by
all four terminals. The [UAC2 field definitions](https://raw.githubusercontent.com/torvalds/linux/v7.0/include/linux/usb/audio-v2.h)
identify the frequency/validity controls as read-only. Upstream
[clock setup](https://raw.githubusercontent.com/torvalds/linux/v7.0/sound/usb/clock.c)
skips programming such a clock and can continue after a frequency mismatch. This
makes actual device-reported frequency useful evidence; it has not yet been
measured, and Ubuntu downstream equivalence has not been established.

The next observer records aggregate requested lengths and bounded source/sink
configuration (rate, current packet size, maximum packet size). It also observes
the parsed return value of the driver's existing clock-41 read via entry/return
probes. No extra control request, payload access or rate change is introduced.
Read-only BTF/symbol discovery confirms those types/fields/functions exist.
`tools/usb_feedback_review.py` checks bounded NDJSON/maps, duplicate keys, valid
dimensions/counts, packet/callback accounting, metadata coverage and warnings;
clock-read failure/missing observations stay unknown. The revised collector needs
seven attached probes and complete geometry/clock observations before validating
its live observation, independently of whether playback succeeds.

All 48 focused Linux cases pass on exact source
`beacn-link-physical-playback-20261014T152027Z/source`; unchanged Rust inputs reuse
the verified CLI. Revised kernel compilation/attachment remains pending the
staged owner-terminal launcher `beacn-feedback-ready-20261014T152029Z/run.py`.
It validates source hashes and an audio-free attachment check before silence.
A fresh private Windows baseline is saved; SSH sudo remains denied even after
terminal authentication. This needs terminal execution, not another authorization
or sudoers/SSH change. Afterward collect/hash results and compare all baselines.

## Aggregate completion observer preparation (historical access checkpoint)

`tests_native/studio_feedback_stats.py` prepares identity-pinned graph/USB/settings
baselines and generates aggregate bpftrace probes for `snd_complete_urb` and
`snd_usb_handle_sync_urb`. The filter matches the selected bus/device, 33AE:4003,
and audio endpoints OUT `0x02` / IN `0x82`. It reads only completion and bounded
packet status/length metadata; no transfer/setup buffer is read, no raw pointer
or per-event record is exported. Cancelled URBs contribute only to completion
counts, since their old packet fields are not useful feedback observations.
Maps group relative second, endpoint and these status buckets:
0 success, 1 EXDEV, 2 EPROTO, 3 EOVERFLOW, 4 ETIMEDOUT, 5 cancellation/removal,
6 other status. Unsupported packet counts above 64 and lengths above 416 stay
explicitly unclassified. Successful lengths are summed, not payloads copied.

Preparation is inert by default; execution requires an explicit status/silence
flag. The first administrator operation is an audio-free `--dry-run` attachment
check. bpftrace 0.25's
[CLI contract](https://bpftrace.org/docs/release_025/cli) covers this check;
its [versioned implementation](https://raw.githubusercontent.com/bpftrace/bpftrace/v0.25.0/src/bpftrace.cpp)
starts a `-c` child after probe attachment. The subsequent audio child uses
`setpriv` and a clean normal-user environment to run the existing finite guarded
ALSA diagnostic. `BEGIN` is used only for relative time, never as readiness proof.
The existing collector child is retained, time/output are bounded, and SIGTERM
now marks ALSA cancellation so its owned-child cleanup remains reachable.
No ordinary native executor apply is enabled.

The actual VM has the tools, both function symbols and module/kernel BTF.
Its normal SSH identity has zero capabilities, no passwordless sudo, disabled
unprivileged BPF and administrator-only tracefs. Unprivileged compilation fails
at event/function discovery, including a metadata-only symbol-list attempt.
The actual noninteractive attachment gate refuses before audio, with exact guest
and Windows baseline comparisons. This is failure-path validation, not compiler,
verifier, attachment, complete aggregate coverage or new audio acceptance.
All 43 focused Python checks pass on Linux, including real SIGTERM cleanup
signals; exact code source is `beacn-link-physical-playback-20261014T144527Z/source`.
Gate summary SHA256:
`49eb91e43e0ed54ac41371575ec6d58c88c92e01eb7749c0afba4f355479b6a5`.
Private staged launcher `beacn-feedback-ready-20261014T144550Z/run.py` is byte-verified
and validates its source manifest before execution. Run it as the normal Ubuntu
user; its sudo prompt stays in that terminal. An administrator access method is
needed, rather than new test authorization. Preserve all output and compare the
fresh private host baseline after execution. Payload recording, driver/policy
changes, another controller/rate trial or UI watching are not part of this test.

## Latest VM-only request-submission diagnostic

The shared `studio_playback.audio_activation` owner now records bounded cached
`urbnum` metadata beside its pinned interfaces and endpoints. Existing PipeWire
and direct ALSA runners include it in their clock observations. Linux
[v7.0 sysfs](https://raw.githubusercontent.com/torvalds/linux/v7.0/drivers/usb/core/sysfs.c)
exposes the device statistic; the
[HCD submission/giveback implementation](https://raw.githubusercontent.com/torvalds/linux/v7.0/drivers/usb/core/hcd.c)
increments it on submission and reverses immediate enqueue failures, without a
normal completion decrement. Treat it as device-wide cumulative submissions,
never outstanding requests, completed packets, feedback payloads or clock proof.
Missing metadata stays unknown; malformed/wrapped values fail the observation.

The retained failed-run ALSA stream metadata explicitly selected sync endpoint
`0x82`, interface 2 alternate 1, and implicit feedback mode. This is configured
driver state, not successful feedback delivery. No generic implicit-feedback
override was enabled.

Exact code source `beacn-link-physical-playback-20261014T140703Z/source` passes
39 focused Python cases on Linux, reusing unchanged verified Rust inputs and CLI.
Before a new authorized EHCI direct ALSA 48 kHz authored-silence diagnostic, ten
idle samples held the counter at 115 with both PCMs closed and all four endpoints
verified. Nine playback samples then recorded 146 to 1,564 submissions over
2.053 seconds with both audio alternates 1; hardware pointer stayed zero.
The two-second stall guard terminated only the owned child, which exited.
Guest desktop/settings/routes, saved-file bytes and USB binding compare exactly;
Windows audio and registry compare exactly within this new attempt. A final
ten-sample idle observer holds at 1,569, both PCMs closed, four endpoints verified
and exact unchanged guest baseline. Ordinary physical executor apply remains off.

Seventeen attempt artifacts were downloaded and hashed outside Git. Journal:
`bf07a5f30ecb73e90ebff48b81958471b0dbe14a2bbd6c51a1a13592aa235f84`;
comparison:
`c0d6438294a753d50922b419aaf036eb2f2f42a42dba81a1b32d07433122dcd3`.
Final idle summary:
`64e1ba0687a589de65c0263090d2f2ae25911be9b49219b0d95c29d2636f78ac`.
Existing VMware logs contain no timestamp window matching this attempt, so they
cannot establish transport status. Root cause and useful feedback/completion
delivery remain unknown. Next inspect the implicit-feedback queue/clock code and
available existing status metadata, without repeating controller/rate variants,
capturing packets/audio or requiring owner UI interaction.

The follow-up upstream v7.0
[endpoint reference](https://raw.githubusercontent.com/torvalds/linux/v7.0/sound/usb/endpoint.c)
shows that implicit-feedback output waits for packet-layout entries. Incoming
completions with zero aggregate successful bytes supply no entry, while the
incoming callback can resubmit its request. This explains how repeated requests
could coexist with no playback queue progress; it is a hypothesis about this
device, not an observed completion status. The
[PCM reference](https://raw.githubusercontent.com/torvalds/linux/v7.0/sound/usb/pcm.c)
advances `hwptr_done` when copying playback data into requests, and its pointer
callback returns that value. The ALSA pointer is consequently a driver queue
progress signal here, not independent hardware-clock or delivery proof.
Retained attempt status has `appl_ptr=24000`, `hw_ptr=0`, `delay=24000` and
`avail=0`, so the application did supply data. Exact upstream sources and hashes
are retained privately; equivalence to Ubuntu's installed downstream patches
has not been verified. The existing kernel journal also has no timestamped
records for the attempt window. Next assess aggregate feedback/completion-status
observability without collecting payloads; another unchanged silence test would
not settle these alternatives.

## Optional native preflight prepared before the owner clarification

The gaming PC currently has no Linux installation. The owner needs the
replacement demonstrated before migrating. Development and current testing stay
in the VM. A temporary desktop live session was proposed before the owner
reaffirmed the VM requirement; it is now deferred. Ubuntu's official
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
Windows verified its allowlist, integrity and the source hashes at preparation.
Later playback diagnostics changed; rebuild the optional bundle before reuse.
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
attached and Windows Link closed. The later submission diagnostic is recorded above.

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
about 18.2 GB. Continue the VM transport/driver investigation under the owner
constraint above, with intact device/route guards. The VM results do not
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

# BEACN Link Linux — continuation handoff
Updated 2026-10-14. Resume development; do not restart the investigation.

## Current owner plan: separate Debian laptop with the Windows agent retained

The gaming PC has **no native Linux installation**; establishing feasibility
before migration remains the goal. After the VMware transport failures below,
the owner volunteered a separate Linux laptop for a direct Studio comparison.
The agent remains active on Windows and now has authenticated SSH access to
Debian 13, kernel 6.12.111, PipeWire 1.4.2 and WirePlumber 0.5.8. This supersedes
the earlier VM-only next step without requiring a gaming-PC installation/reboot
or another agent session. The owner explicitly accepted the laptop's first
observed SSH host key; it is pinned privately for subsequent connections.

The owner also requested remote desktop. Existing GNOME Remote Desktop 48.1
remote login was configured after a private baseline, with credentials delivered
through stdin and TLS key/certificate held by the service account. The service
is active; Windows-to-laptop RDP negotiation and the pinned TLS certificate were
verified, and the owner confirmed an interactive login. An unused desktop-sharing
setup stalled on keyring access; its settings were restored to the saved baseline.
After setup, the saved graph/settings and persistent-state hashes compare equal.
No Chrome Remote Desktop, mouse automation or additional desktop was installed.

The owner corrected an incorrectly connected cable. Studio now enumerates as
33AE:4003 at 480 Mbps under snd-usb-audio. Its eight-channel FLOAT_LE ALSA PCM
supports 48000/96000 Hz and implicit feedback. The first native 96-kHz silent
PCM run and a Debian private-core executor check now pass, as recorded below.
Full pre/post graph/settings, persistent files and USB binding are preserved.
The owner reaffirmed authority for necessary login/relogin and reconnect/recovery
steps. Audible testing still needs the owner ready to listen; a quiet first-pair
test is prepared, with the readiness question pending. Native laptop vendor
initialization/publication and physical PipeWire application remain unverified.
Keep ordinary apply and Actions disabled; preserve all VM evidence. Do not repeat
failed controller/rate tests or the initial audit, or install/run REA.

## Native Debian PCM and private executor passed (2026-10-14)

Native-only `--native-direct-card` pins the actual selected ALSA card/device
identity without assuming UCM split nodes. It verifies nonvirtual execution,
closed selected playback/capture PCMs, the same retained device inode, complete
foreign graph/settings equality and released USB binding. It does not force a
graph rate or change other devices; the original VM UCM/isolation mode remains.
Raw-card comparison allows no negotiated-format exceptions.

The 48-kHz run stalled at pointer zero but exited and restored the complete
baseline. At 96 kHz the pointer advanced; the first harness refused the normal
DRAINING transition. That observation is retained. The corrected harness accepts
DRAINING only after sufficient validated progress, still checks exact child PID,
and waits for natural exit within the existing bounds. A confirmed closed PCM
can also settle while the same child exits; no exit-code-only acceptance.

The corrected four-second 96-kHz silent run exited 0, made 18 clock observations
and advanced 1,152 -> 380,336 frames. All desktop/settings/routes/state-file/USB
comparisons passed, including complete persistent-state hashes. Journal SHA256:
`6fb895b7eb9366b2aa32d02ee22bff6b5ecdec5c48b56ed766265ab49ac70b1c`.
This establishes native direct-ALSA queue progress and clean completion; it does
not establish audible output or independently observed USB delivery. The clock
descriptor is read-only source 41; one bounded current-rate metadata request
was refused EBUSY with no claim/detach/reset/write. The 96-kHz diagnostic uses
the earlier VM clock observation; a current native clock value is unverified.

Explicit audible authorization now prepares three 350-ms 440-Hz tones capped
at -24 dBFS on channels 0/1 only, with a one-second silent lead and quiet tail.
All other channels stay silent. Tone mode requires its separate authorization
flag and the owner ready to listen. No audible run has started.

The laptop has isolated Rust 1.88 and Debian build dependencies; the working VM
was not reinstalled. Exact native programs build against libusb 1.0.28 and
PipeWire 1.4.2. Cache-only descriptor validation passes without device access.
Focused playback checks passed 27 cases with one initially skipped built-planner
check, which then passed separately against the new CLI.

Debian's private null sinks initially exposed one unlabelled convert-mode port;
the strict executor refused this layout before routing. The harness now validates
its retained controller's client PID and each owned destination, configures
explicit stereo DSP PortConfig at 48000 Hz, and requires actual FL/FR ports.
Prepared raw private metadata is retained for refusal diagnosis. Native snapshot
refusals now identify missing client/process/port/direction metadata; no guard
was weakened. The exact native A-to-B, partial-failure and rollback check passes,
with actual active links, new rollback serials, private baseline restoration,
owned cleanup and unchanged desktop/state files. Private journal SHA256:
`cc15c535b568024b333ddc74d92f111f1ded5ae5a1584ad580a4e9d07a464d97`.
143 private artifacts were copied/hashed outside Git. This is private-core
routing, not Studio routing. The later VM recovery check could not connect over
SSH; no VM recovery or current process-state claim is made.

## USB return evidence and initialized direct-ALSA coexistence (2026-10-14)

Owner launcher 172808 loaded all 17 probes, passed review and exited 0. At
96000 Hz and 416-byte endpoint sizes, OUT had three entries and three zero returns
at USB-submit, HCD-submit and EHCI-enqueue, but no givebacks/completions. IN had
15 entries/zero returns at each stage and three completions: 24 successful packets,
six empty and 18 nonempty, 7,232 bytes. The three OUT buffers total 9,536 bytes,
matching the ALSA queue pointer 298 -> 298. Successful enqueue is software
acceptance, not hardware delivery. Cancellation returned zero for three OUT and
12 IN URBs; the later OUT dequeue returned -16 (already unlinked), not a new
playback submission rejection. The child remained D / usb_kill_urb. Thirty result
files were retained/hash-verified. Summary SHA256:
`eec7e1b28d9a3fdd63f41ea916e623155b2f9ecac14f60cd50d8e32b17bbaf15`.

Owner reconnect released the child and closed all PCMs; graph/settings/state-file
comparisons passed. Recovery SHA256:
`1b4b998dbcb70b93b96770dcfd1d5b71f07c1f3aea143f21e955b723baa0ac53`.
Windows audio matched; 16 machine-audio registry differences remain preserved and
unclassified. No blanket restore was performed.

The direct ALSA runner now supports caller-owned initialized vendor coexistence,
sharing the existing exact process-start and retained selected-USB-FD checks.
PID/start arguments must be a complete positive pair; identity/FD are rechecked
during activation. These checks never grant authority to terminate the helper.
All 62 focused Linux cases passed on exact source
`beacn-link-physical-playback-20261014T173834Z/source`; 39 unchanged Rust inputs
reuse the validated CLI without a heavy rebuild.

One separately snapshotted initialized 96-kHz direct-ALSA attempt still stalled
at pointer 298. The vendor helper released its claim successfully, but reported
baseline_restored false while audio remained active; aplay 15077 remained in
D / usb_kill_urb. No whole-device recovery is claimed for this latest attempt.
Twenty-seven artifacts were copied/hashed; outer journal SHA256:
`8203690395fb4e6a96d321c4f29cf907efcda0c2f56451c9963a68ef72482be3`;
ALSA journal `3b1ace6bd9299b77c94836e7b784fe2e92296e05da14c51f310b6a3f5bd0eec9`.
Windows endpoint/default settings and registry matched; one unrelated session-list
change was preserved. Three read-only samples found active device/controller power
and a shared IRQ counter unchanged; this is not EHCI-specific causal evidence.
No additional root probe/audio run occurred. An optional bounded root-readonly
VM diagnostic was staged but is superseded by the owner's laptop plan. Verify
VM recovery after the physical move; do not start further VM playback.

## Live USB handoff entries, recovered default and return-code preparation (2026-10-14)

The owner ran launcher 170813. Twelve probes loaded without warnings/stderr,
exited 0 and passed aggregate review. At matching 96000-Hz rates/416-byte packet
sizes, three IN completions again contained six empty and 18 nonempty successful
packets (7,232 bytes). OUT had three usb_submit_urb, usb_hcd_submit_urb and
ehci_urb_enqueue entries, but zero givebacks/completions. IN had 15 entries at
each submission/enqueue stage and three givebacks/completions. One OUT kill entry
occurred at relative second four. These are function entries, not return/acceptance
evidence. OUT URBs each contained eight packets, buffers 3,072/3,136/3,328 bytes,
interval one. Their total 9,536 bytes / 32 bytes per frame = 298 frames matches
the ALSA queue pointer 298 -> 298. This does not prove hardware delivery.

The owned aplay process 13181 remained in D / usb_kill_urb. Twenty-nine result
files were copied/hashed before recovery. Summary SHA256:
`7e20fb44aa455262ddc4a2b2b6d21c011f0a76e838609729945fe2ae9dc08d93`;
journal `1eb87c89d4fcf257b42615b88e519b9843d4e82dd7aff52cbf265222714a7b4e`.
Windows audio and registry match the immediate attempt baseline exactly.

Owner reconnect released all owned processes/PCMs and preserved persistent files,
nodes/ports/links/device settings, but changed live default.audio.sink from Link 1
to Link 4. This was classified before proceeding. Fresh complete guest snapshots,
bindings and endpoint pins were saved, the single old default was restored via
the normal-user metadata API after revalidating its exact current value and
unique original Link-1 target, then all logical metadata/settings/routes/file
comparisons passed. No persistent file was changed by restoration. Thirty-five
recovered artifacts were copied/hashed. Recovery SHA256:
`b49346de6fdb23ec13f0515112ad80290aa04d441861a8cd81fa48f14e5f4001`;
default restoration SHA256:
`77e74075099f1fb49dd793b8a620da0d92d862b715c63aef5484917e1e5efbfe`.
Windows audio matches exactly after reconnect; 18 machine-audio registry
differences remain unclassified/preserved, with no blanket restore.

New optional `--observe-usb-returns` requires the handoff mode and adds an EHCI
dequeue entry plus typed fexit probes for USB submit, HCD submit, EHCI enqueue and
dequeue. Actual guest vmlinux BTF confirms the urb arguments. Typed return arguments
avoid PID/CPU/pointer correlation maps. Bounded negative errno/zero return counts
and successful enqueue packet/buffer/interval/start-frame aggregates distinguish
rejection, a missing return and queued software work from eventual giveback.
Zero enqueue return remains neither delivery nor audible acceptance. The decoder
requires 17-probe coverage, validates matching per-endpoint entry/return counts
and geometry totals, and retains legacy 5/7/12-probe meanings. New kernel fexit
loading remains pending the matched audio-free preflight; no fallback audio retry.
The ALSA journal now explicitly calls its progress an ALSA queue pointer, not
independent delivery evidence. Ordinary physical apply and Actions stay disabled.

All 60 focused Linux cases pass on exact source
`beacn-link-physical-playback-20261014T172651Z/source`; unchanged Rust inputs reuse
the validated CLI. An inert fexit-only code-generation attempt refused on
administrator-only tracefs discovery, before compilation/attachment/audio; it is
not syntax/load proof. The existing SSH context still cannot perform this check.
Fresh guest/Windows snapshots and the byte-verified launcher
`beacn-feedback-ready-20261014T172808Z/run.py` are staged. Launcher SHA256:
`e921f8b4e832a498d11fe473e103a4c235175457522961b989d43f36926666b4`.
Leave Studio attached; source/settings/generation drift and ten-minute expiry
refuse execution. No root/audio run has started. Next owner-terminal execution
first checks all 17 probes without audio, then one bounded 96-kHz child if that
gate succeeds. Collect return/accepted-enqueue/giveback evidence and compare.
Another blocked child requires selected-device recovery before further playback.

## Prelaunch reconnect refused; fresh handoff launcher staged (2026-10-14)

The owner reconnected Studio again after launcher 170306 was prepared. Both
invocations reported only RuntimeError. Read-only verification finds unchanged
source, a nonexpired baseline, changed USB generation, no evidence directory and
no aplay/bpftrace process. No new probe/audio run occurred. Fresh logical node/
port/link/metadata/device settings and persistent-state bytes match the old
baseline despite regenerated identities. Host differences are retained privately.

The private launcher now prints controlled RuntimeError reasons and distinguishes
USB generation drift, settings drift, expiry, changed source and reused evidence.
It retains all guards. Fresh guest/Windows snapshots and a byte-verified replacement
are staged at `beacn-feedback-ready-20261014T170813Z/run.py`, using unchanged
57-case validated source `beacn-link-physical-playback-20261014T170230Z/source`.
Launcher SHA256: `073567620e1fea40818755ca7087dae9ca61bed1feab9555ba743d7c4ecfa983`.
Leave Studio attached when running it; another reconnect requires new preparation.
The ten-minute baseline expiry remains. Twelve-probe kernel loading/USB handoff
observation is still pending; no further playback outcome is claimed.

## Matched-clock input progressed; USB handoff observation is next (2026-10-14)

The owner ran `beacn-feedback-ready-20261014T164723Z/run.py`. Seven probes loaded
without warnings/stderr and exited 0. Device clock and both endpoint configurations
were **96000 Hz**, current/maximum packet sizes **416 bytes**. Three IN completions
contained 24 successful packets: six empty and 18 nonempty, totaling **7,232 bytes**.
All requested lengths were 416 bytes; no packet errors were observed. Three sync
callbacks occurred, all in the first relative second, and no OUT completion was
observed. Unlike the 48-kHz run, this startup received usable implicit-feedback
lengths; the pipeline then stopped. Do not claim sustained transport or audio.

ALSA pointer stayed 298 -> 298 across ten samples. The owned aplay process 11432
remained in D / usb_kill_urb after termination. Twenty-nine result files were
copied/hashed. Summary SHA256:
`637baf5ed39d8cbd6864d94f0fbdbba5b05229f378fbd1f19146edb241f8fb21`;
playback journal:
`82c6231978195faf0f1eccfce426199e3108e877c7a141add0a9e11ba9df6ee7`.
Windows endpoint settings and registry match the immediate attempt baseline;
one session list differs and is retained privately.

The owner disconnected/reconnected Studio. Fresh recovery verifies no aplay/
bpftrace process, both PCMs closed, released interfaces, matching descriptor/
attributes and responsive desktop services. Logical nodes/ports/links/metadata/
device settings and all persistent-state file bytes match. USB generation changed;
identity equality is not claimed. Thirty recovered result files were copied/hashed.
Recovery SHA256:
`d1d2a74b28fc5a8b411dffba1b55a479af66b96895cb6f523d1bdfea615eee0b`.
Windows endpoint defaults/volumes/mutes/formats/states match after reconnect.
Full host audio differs and 17 machine-audio registry differences remain
unclassified and preserved; no blanket restoration occurred.

The next optional `--observe-usb-handoff` mode adds five selected-device ISO
entry probes: usb_submit_urb, usb_hcd_submit_urb, ehci_urb_enqueue,
usb_hcd_giveback_urb and usb_kill_urb. It distinguishes reaching submission,
controller enqueue, giveback and cancellation from ALSA completion. **Entry is
not a successful return or acceptance receipt.** The requested submission geometry
is a bounded aggregate of packet count, buffer byte count and interval; no buffer
contents, per-event rows or addresses are read/exported. usb_submit_urb has not
initialized urb->ep at entry, so endpoint/type filtering uses its initialized pipe.
The mode requires the selected EHCI controller. All five actual guest symbols are
present in vmlinux. Kernel compile/load remains pending the matched preflight gate.

The decoder keeps legacy five/seven-probe evidence separate from twelve-probe
handoff evidence, checks bounded dimensions and per-endpoint geometry accounting,
and reports invalid/missing observations. Full raw baseline objects are now saved
before execution to support logical comparison after device generation changes.
The 57 focused Linux checks pass on exact source
`beacn-link-physical-playback-20261014T170230Z/source`; unchanged Rust inputs reuse
the validated CLI. Fresh guest/Windows baselines and the byte-verified launcher
`beacn-feedback-ready-20261014T170306Z/run.py` are staged. Launcher SHA256:
`bc06f5bff9073c8beaab0ed21dd3b8776a7bbf3dcadaf019fe013a88dc5296f2`.
Its baseline expires after ten minutes; source/settings/generation drift refuses
execution. No root/audio execution has started. Next run one 96-kHz handoff
observer through the owner's terminal sudo context, then collect and compare. Keep
ordinary physical apply off and Actions disabled. Earlier staged/pending notes
below are historical; the matched-rate run and its recovery are complete.

## Matched-rate recovery verified; packet check staged (2026-10-14)

After the owner's VMware disconnect/reconnect, read-only recovery confirms no
aplay/bpftrace process, closed playback/capture PCMs, released interfaces, matching
descriptors/attributes and responsive existing desktop services. Logical nodes,
ports, links and defaults match the saved pre-attempt graph; all persistent-state
files match exactly. USB generation changed as expected, so do not claim numeric
identity equality. Nineteen recovered artifacts were copied/hashed. Recovery SHA256:
`2d71482cc7bdb498dff7f0d06beb99c76a4dbdf2728b18358d45bbe887e9ac30`.

Windows endpoint defaults/volumes/mutes/formats/states match. Full Windows audio
has one session-list difference; registry has 16 changed machine-audio values,
each a 16-byte binary value. They remain unclassified and preserved privately;
no blanket restore or full host byte-equality claim. A fresh host baseline is saved
for the next diagnostic.

The explicit matched-clock observer is staged at
`beacn-feedback-ready-20261014T164723Z/run.py`, using the 54-case validated source
`beacn-link-physical-playback-20261014T164016Z/source`. Launcher SHA256:
`c7921131760f98800c3b33858465cf3614871d53ccc04b78dc4f8be78c8f2b56`.
It pins the prior measured-clock evidence and fresh released binding/settings;
the baseline expires after ten minutes. Source/binding/settings drift refuses
execution. No root or audio execution has started. Next owner-terminal sudo
execution observes actual rates, requested packet geometry and errors at 96 kHz.
Then collect/hash and compare; if owned cleanup blocks, stop playback and recover
the selected device before more tests. Actions remain disabled, ordinary physical
apply remains off. The pending-recovery statements below describe the earlier
failed attempt and are superseded by this recovery checkpoint.

## Measured clock mismatch and matched-rate failure (2026-10-14)

The owner ran `beacn-feedback-ready-20261014T153914Z/run.py`. The corrected
observer loaded seven probes, exited 0 without warnings/stderr and validated
complete metadata coverage. It observed the driver's existing clock-41 getter
return **96000 Hz**, while IN/OUT configuration remained **48000 Hz**, current
packet size **288 bytes**, maximum **416 bytes**. All 11,472 requested IN packets
were 288 bytes: 11,308 EOVERFLOW, 158 EXDEV and six successful empty packets.
There were 1,434 noncancelled IN completions, 12 cleanup cancellations, 1,434 sync
callbacks, zero successful bytes and no OUT completions. ALSA still stalled at
pointer 0 -> 0; collector validation is not audio acceptance.

Immediate/fresh guest settings/routes/files/binding comparisons pass exactly;
no owned process remains from that run. Windows registry/defaults/endpoint
volumes/mutes/formats/states match. Full Windows audio differs because three
sessions disappeared across two endpoints, with no changes to surviving sessions.
Those private differences are preserved; do not claim full byte equality or
restore unrelated sessions. Twenty-nine result files were copied/hashed.
Summary SHA256: `8c390897aed8710e1a649e458c5bd1bc549f63b9394c0d407e18e0a8e2d23c81`;
playback journal: `52a3be1279c3df43c05377487c0265141c0fa7b6f38962d0cc3fa43c0e596c9d`.

This measured mismatch justified one new direct-ALSA diagnostic at 96000 Hz,
not another blind PipeWire force-rate trial. The runner now has an explicit
`--matched-device-clock-96000` opt-in, retaining 48000 by default, finite
eight-channel FLOAT_LE authored silence, conversion refusals and existing
identity/foreign-graph/cleanup guards. The private wrapper hash-checked the
prior clock report and required its selected binding to match before execution.
Exact source: `beacn-link-physical-playback-20261014T163645Z/source`.

That four-second silence attempt also stalled: ten samples at pointer 298 -> 298.
The owned aplay process 10595 remained in D / usb_kill_urb after termination;
existing PipeWire/WirePlumber processes remain sleeping/responsive. Windows audio
and registry match the fresh attempt baseline exactly. Guest comparison refuses
because the target is not released. **Recovery is pending; do not start another
playback or claim owned cleanup/settings restoration.** Eighteen attempt files
were copied/hashed before recovery. Journal SHA256:
`0e54af353e5d89032204d069004830b19378d9bce1af3413c159c20c0daabead`;
read-only recovery SHA256:
`42b6a86fa60f5f5c437615562f8a5cea9cee9fed2e815aca14c66288010940d5`.

The owner was asked to disconnect/reconnect Studio once through VMware, keeping
Windows Link closed. Next verify that the owned process exited and compare the
saved guest baseline using fresh identities. Only after recovery may the matched
96-kHz observer check actual device/configured rates, requested packet sizes and
completion errors. Matching the reported clock alone did not solve playback;
failed-packet received lengths remain unknown. No driver/firmware change, payload
capture, ordinary physical executor apply or audible acceptance occurred.

The aggregate runner now exposes nested playback error/child-exit fields in its
summary and propagates the explicit 96-kHz diagnostic to its non-root child.
Actions remain disabled; local checkpoints use [skip ci]. Earlier pending notes
below are historical and superseded by this checkpoint.

## Clock/geometry collector load failure and corrected preflight (2026-10-14)

The owner ran `beacn-feedback-ready-20261014T152029Z/run.py`. Its audio-free
attachment check exited 0, but the actual collector exited 255 before any audio
marker or playback journal. Retained stderr reports BPF program-load error -12
(ENOMEM) for the snd_complete_urb probe. This is an observer-load failure, not a
new physical playback outcome. Immediate/fresh guest comparisons and Windows
audio/registry comparisons pass exactly; no aplay/bpftrace process remains.
Ten private artifacts were copied/hashed. Summary SHA256:
`8ce2438793f3d185da558a9954049acf6dec8a97e86895ad89a2339daae604b9`.

The preflight omitted the actual run's `-k` helper-warning instrumentation and
`-c` child context. It therefore did not validate the same load configuration.
The precise allocation/verifier/JIT cause remains unknown without verbose logs;
do not assert that the VM exhausted all RAM or that the device failed. Both
phases now use one command builder, differing only by `--dry-run`. The versioned
bpftrace implementation returns from dry-run before running a non-USDT child;
these probes contain no USDT. One failed preflight may collect a verbose,
audio-free load diagnostic, then stops; it never retries audio or disables warnings.

The packet guard/unrolled body is narrowed from 64 to eight, matching every
observed callback in the first live run and the upstream capture/implicit-feedback
one-millisecond cap at this endpoint's 125-us interval. Larger counts become
explicitly unsupported, rather than silently truncated. This reduces static
unrolled work; successful kernel loading/memory benefit remains unverified.
The decoder checks the eight-packet bound for new runs while preserving legacy
64-bound evidence. Nonzero collector exits now populate `error`; a launch marker
without a final journal leaves the audio outcome unknown, not false success.

All 51 focused Linux cases pass on exact code source
`beacn-link-physical-playback-20261014T153912Z/source`; new regression cases cover
matched preflight/runtime arguments, failed-exit reporting and packet bounds.
Unchanged Rust inputs reuse the verified CLI. No heavy build or installation.
Private next launcher: `beacn-feedback-ready-20261014T153914Z/run.py`, byte-verified,
with a fresh Windows baseline. The same bounded clock/request-geometry check is
pending owner-terminal execution; keep Studio attached. Collect/hash outputs and
compare guest/host baselines afterward. Do not claim the revised observer loads
or that physical audio works before that evidence exists.

## Live feedback error checkpoint and clock/geometry preparation (2026-10-14)

The owner ran the prepared Ubuntu terminal launcher. Compilation/load/attachment
passed; five probes attached, the collector exited 0 and the normal-user ALSA
child exited after its two-second pointer-stall guard. Observer `error: null`
means collection completed, not that audio succeeded. Nine playback samples
remain pointer 0 -> 0. Retain the original `live_probe_validated: false`: it was
an intentionally conservative default awaiting aggregate review.

Aggregate IN `0x82` observations: 1,437 noncancelled URB completions, all URB status
0, plus 12 cleanup cancellations. Their 11,496 packets comprise 11,490 EOVERFLOW
(-75) packets and six successful empty packets. Successful bytes total zero;
sync-handler callbacks total 1,437. No OUT `0x02` completion was observed.
This supports the feedback queue-wait explanation, while device/driver/VMware
root cause remains unresolved. Two printed signedness warnings concern the loop
comparison; observed totals consistently account for eight packets per callback.
Keep these warnings and original output. The revised loop casts packet count to
uint64 after retaining its 1..64 guard; negative counts become out-of-bound.

All immediate and fresh recovery guest comparisons pass exactly. No aplay or
bpftrace process remains; existing PipeWire/WirePlumber processes are responsive.
Windows audio and registry match the fresh pre-run baseline exactly. Twenty-seven
private result artifacts were copied and hashed. Summary SHA256:
`ff043e7ed1ea811ded5332f445c5a6ed87cdee671a275d9e20f4e51f5c6dae70`;
playback journal SHA256:
`88a369d5b8795eee57933e05ed1f92794f91c1946d1a7c5dc1b14764eadde57c`.
Old restart-wide unclassified state differences remain preserved separately.

Upstream EHCI maps its babble bit to EOVERFLOW and does not update actual_length
for that error; never interpret failed-packet zero lengths as received sizes.
Cached UAC2 descriptors show one clock, ID 41, attributes 3, controls 0x05, shared
by all four terminals. Frequency/validity controls are advertised read-only.
Upstream clock setup can continue after a requested/device-rate mismatch on a
read-only clock. These are reference-code/descriptor observations; the device's
actual rate and equivalence to Ubuntu downstream behavior remain unmeasured.

Next observer version adds bounded requested-length histograms, configured IN/
OUT rates/packet sizes and aggregate return values from the driver's existing
get_sample_rate_v2v3 clock-41 read. It issues no extra USB requests, reads no
payload buffers and makes no rate/driver/firmware change. The actual kernel BTF
contains the required endpoint fields and clock-reader function. Seven probe
attachments are now required. `tools/usb_feedback_review.py` validates bounded
JSON/maps, dimensions, counts, coverage flags and explicit unknowns; warnings in
stdout are retained even when stderr is empty. No exit code alone proves audio.

All 48 focused Linux cases pass on exact code source
`beacn-link-physical-playback-20261014T152027Z/source`, including parser refusals and
actual cancellation signals. Unchanged Rust inputs reuse the verified CLI.
New kernel compilation/attachment and geometry/clock measurements are pending.
Private next launcher: `beacn-feedback-ready-20261014T152029Z/run.py`, byte-verified;
it first validates source hashes and the audio-free attachment gate, then runs
the bounded silence diagnostic as the normal desktop owner if the gate passes.
A fresh private Windows baseline is saved. The SSH sudo context is still denied
after the owner's terminal authentication; use that terminal launcher again,
without changing sudoers/SSH permissions. Keep Studio attached and collect/hash
the new evidence plus fresh guest/host comparisons after execution.

## Aggregate completion observer preparation (superseded by live checkpoint above)

`tests_native/studio_feedback_stats.py` generates selected-device-only aggregate
kernel observations for Studio OUT `0x02` / implicit-feedback IN `0x82`. It reads
completion status, bounded per-packet status/length fields and counts sync-handler
calls. It never accesses transfer/setup buffers or emits per-event rows. Counts
are grouped by relative second and endpoint; packets beyond the 64-entry bound,
unexpected lengths and other errors remain explicitly unclassified. Cancelled
completions are counted but their stale packet fields are excluded. These are
transport/driver metadata, not audio/USB payload capture or delivery acceptance.

Normal preparation is metadata only. Authorized execution first requires a
successful `bpftrace --dry-run` compile/load/attach check with no audio child.
Then bpftrace starts the existing guarded finite ALSA silence runner only after
its probes attach, using `setpriv` to return audio execution to the desktop owner.
The collector has a 30-second ceiling and retained-process time/output guards.
ALSA now handles SIGTERM as a cancellation request, leaving owned child cleanup
reachable; a Linux test verifies two actual signals including one during cleanup.
Root-child outcome/aggregate coverage never become verified merely from exit 0.

The VM has bpftrace 0.25.0, bpftool, kernel/module BTF and both target symbols.
The existing SSH account has no effective capabilities or passwordless sudo;
unprivileged BPF is disabled and tracefs is administrator-only. Unprivileged
compilation stops at function/event discovery, before bytecode validation.
The noninteractive attachment gate was exercised: it refused before any audio
child, and full guest settings/routes/file/binding plus Windows audio/registry
comparisons passed exactly. Gate summary SHA256:
`49eb91e43e0ed54ac41371575ec6d58c88c92e01eb7749c0afba4f355479b6a5`.
No new physical playback, live probe attachment or kernel compiler success is
claimed for this batch. No sudoers/capability/SSH/driver changes were made.

All 43 focused Linux checks pass on exact code source
`beacn-link-physical-playback-20261014T144527Z/source`; unchanged Rust inputs reuse
the prior verified CLI with provenance. Guest free disk remains about 18.1 GB;
no installation or heavy build occurred. A single normal-user Ubuntu terminal
launcher has been staged and byte-verified, with a fresh private Windows baseline.
Private launcher label: `beacn-feedback-ready-20261014T144550Z/run.py`;
SHA256 `19f3ba63c2132710019d70f0282b5d2cc4d0c44dcc5070bc8d9c0cccf677e9ed`.
It checks the exact prepared source hashes, prompts for sudo in Ubuntu only,
performs the attachment gate and runs the bounded counter/silence diagnostic if
that gate passes. Next obtain administrator execution via that owner-terminal
launcher or an existing authorized administrator path. The access method is
missing; owner authorization for the bounded test already exists. Never request
a password in chat or broaden permissions. After execution, use strict SSH to
collect/hash private results and compare the retained fresh host baseline.

## Latest VM-only submission/clock diagnostic (2026-10-14)

The existing physical playback activation owner now records the bounded cached
sysfs `urbnum` statistic with its identity-pinned alternate/endpoint observations.
Both PipeWire and direct ALSA runners already use that owner, so each clock
record includes the counter. It is device-wide cumulative submission metadata,
not an outstanding-request count, completion count, feedback payload or proof of
audio delivery. Missing counters remain unknown; malformed/wrapped values fail.
Retained ALSA stream metadata explicitly reports sync endpoint `0x82`, interface
2 alternate 1 and `Implicit Feedback Mode: Yes`; do not enable a generic driver
override merely because the module's override parameter is false.

All 39 focused playback/graph/ALSA/handshake cases pass on Linux. Exact validated
code source: `beacn-link-physical-playback-20261014T140703Z/source`; unchanged Rust
inputs reuse the prior verified CLI with provenance. Guest free disk is about
18.2 GB; no heavy build or installation occurred.

A new authorized, guarded direct ALSA EHCI 48 kHz authored-silence diagnostic
collected nine clock samples. Both audio alternates were 1, submissions rose
146 to 1,564 over 2.053 seconds, but hardware pointer stayed zero. The two-second
stall guard stopped the owned child, which exited. Full guest settings/routes,
saved files and USB binding compare exactly; Windows audio and registry compare
exactly within this attempt. Seventeen private artifacts were downloaded and
hashed. Journal SHA256:
`bf07a5f30ecb73e90ebff48b81958471b0dbe14a2bbd6c51a1a13592aa235f84`;
comparison SHA256:
`c0d6438294a753d50922b419aaf036eb2f2f42a42dba81a1b32d07433122dcd3`.
Read-only ten-sample observers before/after remain fully unchanged, with idle
counters fixed at 115 and 1,569 respectively. No matching timestamp window was
present in the existing VMware logs; that is a visibility gap, not error-free
transport evidence. No packet/audio capture, driver change or audible test ran.

Next inspect the implicit-feedback queue/clock path and available existing
transport/status metadata to distinguish continued submissions from useful
feedback/completions. Do not repeat controller/rate variants or claim the rising
counter proves delivery. No owner watching, dragging or manual USB toggle is
needed for this investigation; Studio remains attached and recovered.

Follow-up upstream v7.0 source review identifies a plausible implicit-feedback
queue wait: empty/error-only incoming completions can produce no output layout
while input requests continue to be resubmitted. This is a hypothesis, not a
measured completion status or verified match to Ubuntu's downstream patches.
Retained status confirms the app wrote 24,000 frames (`appl_ptr`), with pointer
zero and delay 24,000. The upstream PCM pointer advances while preparing output
requests; call it driver queue progress, not independent hardware delivery proof.
Existing kernel journal has no timestamped records for the attempt window.
Assess aggregate feedback/completion-status observability before more playback.
See the linked playback document for exact reference sources and limitations.

The optional prepared `tests_native/studio_native_preflight.py` is read-only: existing durable snapshot
and cached USB/endpoint owners, one selected card, four actual pair mappings,
normal device access and explicit native-versus-VM detection. It starts no audio
and changes no routing. Its readiness flag is for a separate guarded test, not
audio or full-product acceptance; other physical devices can block the existing
isolation guard. Unknown/virtualized contexts never claim native readiness.
Two focused cases pass on Windows/Linux. The eight-source-file portable ZIP
imports and runs on Linux, saves a stable baseline, verifies four endpoints and
correctly rejects the VM as native. Its manifest/allowlist/source hashes and ZIP
integrity are verified. `tools/package_native_preflight.py` rebuilds it outside
the repository with no Rust/proprietary binaries, credentials or private evidence.
Prepared private ZIP label: beacn-native-preflight-20261014T1330Z.zip;
SHA256 `710d97e1deaeda047655ec67717e603beef2130d1d4bb1c88498ce4f240e3790`.
The VM remains on recovered EHCI. A fresh host PnP query finds no present
33AE:4003 instances, so there is no evidence of native Windows interfaces still
owning this selected device; this does not prove isochronous delivery works.

## Current controller recovery and durable evidence checkpoint (2026-10-14)

**EHCI restored; no further physical playback.** The xHCI comparison lost SSH
and the guest-tools heartbeat during the same direct ALSA silence diagnostic.
Its completion, clock progress and initial cleanup are unconfirmed. A graceful
stop timed out; the dedicated test VM required a recorded forced power-off.
The owned xHCI/PCIe trial fields were removed and original EHCI slot 34 restored.
The owner logged into Ubuntu again. Fresh read-only comparison passes selected
`ehci-pci` at 480 Mb/s, released USB profile, cached descriptor hash, all four
endpoints, logical nodes/ports/links/defaults and saved-file values/permissions.
`stream-properties` differs only in JSON member order; retain its raw diff.
Windows endpoint defaults/volumes/mutes/formats/states match, while two session
lists, sixteen existing binary registry values and one added policy entry differ
across the controller/recovery restarts. Preserve those unclassified differences;
do not claim full byte-identical restoration or overwrite unrelated state.

The owner-requested power-on connection automation is now verified: Studio
reattached without a manual USB handoff on both controller starts. The retained
rule matches VID/PID plus the exact cached host port; its machine-specific value
and VMX backups remain private. This is power-on autoconnect, not a live USB reset
API. Windows Link stays closed and Studio remains attached to the recovered VM.
No mouse automation, driver/firmware change, incoming capture or REA was used.

The forced stop exposed an evidence durability defect: nine newest guest attempt
files survived as empty files, including its baseline/PCM and four clock records.
Preserve them and the transport-loss marker; never derive sample progress from
their filenames. Complete pre-controller backups had already been copied to
Windows and remain intact. The existing shared evidence writer now exclusively
creates and flushes file data and POSIX directory entries before execution.
Pinned descriptor files and authored PCM reuse it; a failed flush prevents native
child launch. All 37 focused handshake/playback/graph/ALSA Python cases pass on
Linux; Windows passes 35 with two existing skips. This is offline validation,
not a new forced-crash or successful hardware run. Exact source:
beacn-link-physical-playback-20261014T131759Z/source. Unchanged Rust inputs reuse
the prior exact built CLI with recorded provenance. Free guest disk: about 18.2 GB.

Fourteen final recovery artifacts were copied and hash-verified on Windows;
final recovery SHA256:
`cdbc18f84d0f8056b84f9a1a1fadec981315cd227a566b2bb9676a25ffe8e7b1`.
Read [native-studio-playback.md](native-studio-playback.md) for details. Continue
the VM investigation under the owner constraint above; the prepared native
preflight is optional deferred work, not a VM test gate. Do not repeat failed
controller variants or visibility/drag tests. This supersedes
earlier next-step/recovery instructions below. Ordinary physical native route
application remains disabled.

## Current recovery checkpoint (2026-10-14)

**Recovery verified; physical clock remains unresolved.** Read
[native-studio-playback.md](native-studio-playback.md) first. New cached USB
observations prove both playback and implicit-feedback audio alternates active
during the 48 kHz stall. A maintainer-supported, temporary owned-stream 96 kHz
experiment selected the requested ALSA rate but advanced only 298 samples before
freezing. Its playback child exited; PipeWire subsequently stopped answering
queries and is in kernel state D at `usb_kill_urb`, with audio alternates still 1.
The post-test graph query failed; preserve that failed comparison. On October 14
the owner disconnected and reconnected Studio. The same PipeWire process left
kernel state D and resumed answering without a service restart. Fresh comparison
passes logical nodes, ports, links, defaults, all four saved files byte for byte,
cached descriptor hash, original rate overrides and released USB interfaces.
The owned playback node is absent. Windows audio state matches exactly, while
the USB cycle left seventeen registry-value differences (sixteen binary values
in machine audio and one added string in user policy); retain these unclassified
differences and full backups, not a blanket restoration claim. Studio stays on
the VM and Windows Link stays closed. No mouse automation, capture, driver change
or further rate-workaround playback was performed. A subsequent direct ALSA
silence diagnostic (below) narrows the clock stall beyond PipeWire playback.
Do not repeat the failed rate test.

Ignored `local/physical_rate_recovery.py` has compared the recovered
desktop against the pre-rate-test full backups, including logical foreign
resources/defaults and absence of the owned force-rate node. Existing private
evidence is preserved. The runner now records USB activation, supports explicit
48/96 kHz PCM, restricts temporary forced rates to the selected idle test graph,
and fails comparison promptly if the desktop is unavailable. Twelve playback and
nine graph Python cases pass on Windows/Linux; unchanged Rust inputs reuse the
previous exact built CLI. New offline source:
beacn-link-physical-playback-20261014T040623Z/source. Latest free disk: about 18.3 GB.
This recovery checkpoint supersedes the earlier exact-cleanup status below.

**Direct ALSA diagnostic completed:** new `tests_native/studio_alsa_playback.py`
uses one retained `aplay` process, explicit raw hardware selection, eight-channel
FLOAT_LE 48 kHz authored silence, four-second input and bounded clock supervision.
It refuses an occupied card or competing app/device/rate override, verifies that
the unchanged PipeWire graph remains foreign, and checks actual PCM owner/format
and cached USB activation. Source
beacn-link-physical-playback-20261014T124203Z/source passes 24 focused Python checks
on Linux; Windows passes 22 with two existing Linux-only skips. Unchanged Rust
inputs still reuse the exact prior built CLI. The hardware run verified ten
observations but stayed at hardware pointer zero and stopped after two stalled
seconds. It created no PipeWire playback stream or native routing receipt.
Owned child exit, desktop/routes/defaults, saved files, released USB binding and
per-attempt Windows audio/registry comparisons all pass exactly; the same
PipeWire server remains responsive. No additional reconnect is needed. Raw
evidence and hashes are preserved privately. This narrows the failure to the
ALSA/USB/device path but does not identify Linux versus VMware versus the device.
Next isolate VM USB controller/passthrough behavior; do not rerun visibility/drag
tests or claim physical/audible output.

## Current hardware checkpoint (2026-10-13)

**Physical playback probe implemented; audio clock blocked:** read
[native-studio-playback.md](native-studio-playback.md) for the current test owners,
flags, exact evidence and unresolved clock path. The complete physical split graph
now projects through existing owners; all four stereo pairs and Link 1's actual
links verify. The selected VM now has backed-up USB 2.0 EHCI/PCI-bridge configuration
and Studio remains attached through `ehci-pci`. No mouse/UI automation was used.
Native initialization succeeds with the explicit handshake-only initial-read
diagnostic, including a thirty-second held claim. Neither immediate-release nor
held initialization cures the physical PCM stall. The final silent run stopped
after a three-second zero-progress hardware clock; owned PCM/native children
exited, vendor cleanup passed, and final connected guest/host comparisons matched
exactly. No audible run occurred. Reboot-wide logical settings match, but JSON
ordering, one host session activity state and sixteen registry properties differ
and remain preserved. Read the new doc before interpreting those comparisons.
Exact source: beacn-link-physical-playback-20261014T032358Z/source; fifteen focused
Python and thirty-five Rust gate/session checks pass on Windows and Linux. Latest
guest free disk is about 18.4 GB. Do not redo visibility/drag tests; investigate
USB audio clock/implicit feedback and VM isochronous delivery next. Ordinary
desktop/native physical apply remains disabled; private executor guards are intact.

**Native assignment integration implemented:** read
[native-assignment-publication.md](native-assignment-publication.md) for current
owners, explicit flags, checks and hashes. The new path applies normalized received
assignments through the existing native executor on the selected owned private
core, then derives each outgoing record from fresh verified actual routes and
native receipts. It continues all 40 cycles instead of ending at the first
assignment. Default publication/ordinary desktop apply remain unchanged.

An isolated native probe passed real routing 0 → 1 → 2, receipt prefixes 2 and 4,
source preservation, owned cleanup and all connected local comparisons, without
USB transfers. Final source adds one-shot proof enforcement and a diagnostic-line
clarification and builds at beacn-link-usb-publication-routed-20261013T142858Z/source.
Windows/Linux pass 24 route/publication cases; Linux passes all 27 Python runner
cases (Windows passes 25 with two Linux-only skips). Existing native routing code
was reused, not replaced. Free guest disk is about 19.4 GB; lightweight builds only.

**Integrated hardware test passed:** the owner reported that the Linux test
entry turned yellow and entered the first Link In channel. Native assignment 1
arrived at cycle 9, the existing executor completed two acknowledged stereo-link
creates, and fresh route verification permitted assignment-1 publication at
cycles 10–40. Two repeated assignment-1 replies completed with receipt prefix
zero, issuing no additional route operations. All 40 cycles and 81 fresh
supervised inventories completed. A2 took 42 ms. Source preservation, vendor
claim release, three owned-process stops, private-core restoration and connected
guest binding/desktop/state plus host audio/registry comparisons all passed.
This proves remote UI assignment acceptance and native private-core execution/
verified publication together; physical Studio audio and prolonged UI lifecycle
remain unverified. See the integration doc for the private journal hash.

The ignored
`local/routed_connected_attempt.py` pins the final executable/runner and uses
fresh connected host/guest snapshots, bounded execution and connected comparisons
through strict SSH. It can run `--probe run-attached` without USB I/O; the completed
probe is preserved under its earlier source/evidence state. Announce start/end and
leave Studio connected. No computer-use/mouse automation. Routes in this batch
target owned virtual sinks, not physical Studio audio endpoints.

**Next: physical audio integration.** Read-only metadata identifies one Studio
audio device (33AE:4003), four stereo Audio/Sink nodes described as BEACN Studio
Link Link 1/2/3/4, four stereo sources and internal eight-channel sink/source
nodes. No PCM/USB I/O was requested by this readiness inspection. Names are
descriptive context, not deletion/route authority; verify exact fresh device,
node/port serials, pair mapping and current foreign links before any physical
test. The current native executor still intentionally accepts only its owned
private core/synthetic nodes. Do not bypass that guard or claim the virtual
destinations already feed Studio. Prepare the next explicitly selected physical
audio path and short audible test while preserving settings/unrelated work.
Studio remains connected to the VM; no per-test handoff or mouse automation.

The explicit initialization diagnostic now permits a single A2 read up to five
seconds, while all writes and later reads retain their 500 ms limits. Default
initialization remains 500 ms. No timeout retry or invalid-A2 bypass exists.
The earlier connected attempt stopped at A2 with libusb TIMEOUT (-7), zero of
512 bytes received. A fresh diagnostic attempt received a valid A2 in 669 ms
and completed 40 publication/poll cycles: seven NoPayload replies, 33 rejected
markers, 81 fresh inventory requests, no assignment records. This proves that
a valid response can exceed 500 ms in this VM, not a firmware timeout requirement.

Two subsequent owner-observation attempts completed all 40 cycles. A2 took
439 ms and 584 ms respectively. The first received two one-record replies,
neither matching the advertised test name; the second received no records.
Names were redacted, so these journals cannot identify the unmatched records.
The owner saw BEACN Linux VM Test but could only assign it to Aux 2, Music,
System or Aux 1, not Link In. The owner did not try Aux 2 because that was not
the intended goal. Do not report successful assignment or change the acceptance
target to an arbitrary local mixer channel. Resolve source classification and
the remote assignment mapping before another owner drag test.

Every attempt saved fresh connected guest/host baselines. Owned claims released,
both private processes stopped, the private graph restored, and connected guest
binding/desktop graph/persistent files plus host audio/registry matched exactly.
These comparisons cover the VM and gaming/host PC, not the streaming PC's UI or
profile state. Studio remains connected to the VM. No physical return comparison
has run this session; prior pre-handoff evidence is preserved separately.

Before every observation test, give the owner the exact place to watch and the
purpose of the requested action, wait for readiness, announce the start and end.
Do not ask retrospectively whether they watched an unannounced run. No computer
use or mouse automation is permitted. Windows Link stays closed; streaming-PC
BEACN App stays open. Necessary VM/hardware continuation is already authorized.

Exact source: beacn-link-usb-publication-initialization-20261013T131603Z/source
in the existing guest home. Windows/Linux passed 42 focused Rust integration
cases; Linux also passed two native unit checks and all 24 Python runner cases
(Windows passed 22 and skipped two Linux-only cases). Both native examples and
the default build passed. Free guest disk: 20,186,005,504 bytes; use lightweight
single-job builds. See publication/session docs for provenance and hashes.

The reply diagnostic now reports assignment bytes, name lengths, exact bound-name
matches, embedded-NUL counts and printable-ASCII status for at most four records
per reply. All names remain redacted; extra records are counted without expanding
the log. It does not change decoding, name bindings, planning or USB scheduling.
The exact native example built in the guest; this diagnostic has not run on hardware.
Source: beacn-link-usb-publication-reply-20261013T133631Z/source. See publication
docs for hashes. No unchanged test matrix or hardware attempt was repeated for it.

**Resolved name interpretation (2026-10-13):** the owner-ready prefix diagnostic
received eight assignment-1 records whose first 19 bytes exactly matched the
owned test name, followed by a NUL and five opaque bytes. An explicit
`ReceivedNamePolicy::WindowsCString` now adapts received names at the planning
boundary; wire decoding retains all raw bytes and default matching stays exact.
The existing planner still guards exact inventory names, stream liveness,
duplicate/conflicting assignments, opaque zero/unsupported values and generation.
Windows/Linux passed 19 focused session tests including four new compatibility
cases. Linux passed all 25 runner cases; Windows passed 23 with two Linux-only
skips. Default and exact native builds passed.

A separate owner-ready compatibility hardware run recognized assignment 1 at
cycle 5 and produced one existing-planner proposal for the owned stream. It
stopped publication, held five seconds, rechecked inventory, released its claim
and cleaned up; all connected local comparisons passed. The owner reports the
entry disappeared while being dragged, so UI retention/drop acceptance remains
unverified. No route was applied. Source:
beacn-link-usb-publication-c-string-20261013T135803Z/source. Free guest disk:
19,808,038,912 bytes; continue lightweight builds. Read publication/session docs
for hashes, explicit flags and limits.

Completed native integration above supersedes this prior next-work note: keep the bounded test source alive during interaction,
apply a resolved assignment only on the selected private core through existing
native observations/preparation/receipts, and advertise an assignment only after
actual local-route verification. Do not keep sending unapplied zero over a
received assignment or claim physical audio from a virtual target. Physical
BEACN endpoints and ordinary desktop apply remain outside this private-core slice.

Historical preceding diagnostic: owner again reported Link In unavailable. Three
replies carried assignment 1 and 25-byte names with one NUL, none matching the
19-byte test name exactly. A new diagnostic reports first-NUL position, opaque
tail length and whether the preceding bytes match the owned name. Its native
build passed; a passive run received no records, leaving that comparison untested.
All connected local comparisons and owned cleanup passed in both runs. See
publication/session docs for hashes and focused static string-copy/comparator
evidence. Latest source: beacn-link-usb-publication-reply-nul-20261013T134722Z/source;
free guest disk is about 20.0 GB. Codec and planner still preserve exact raw names.

The prefix diagnostic and name-resolution hardware milestones above supersede
the prior next-work note. Resolve why the visible source is
not assignable to Link In, then
test a correctly identified remote assignment. Receiving and resolving that
assignment precedes physical audio routing. Ordinary desktop apply stays disabled.
The end goal remains Linux applications controlled from Windows BEACN App with
verified audio through Studio Link; visibility alone does not meet it.

## Objective
Keep the streaming PC on Windows with BEACN App / Mix Create / Studio.
Replace the gaming PC's Windows Link application with a Linux implementation,
possibly alongside Windows dual boot for unsupported games. The eventual result
must expose Linux applications to the streaming PC's Windows App and accept its
routing assignments. A generic local mixer or USB audio driver is not enough.

## Incoming verified checkpoint (historical)
Repository: https://github.com/joenilan/beacn-link-linux (private), main.
Windows checkout: E:\git\beacn-link-linux.
Latest implementation: 39d36a35377df257fd0cdbd95a7577a8bc61e80b,
“Add PipeWire graph snapshots and link receipts [skip ci]”.
Parent: 43c9453b4d02d62bf471af7ba473a79b4c3d0a0e.
The owner pushed it; GitHub main and all 13 intended files were independently
verified. Local working-tree cleanliness after the owner's push was not rechecked.
This handoff's later documentation commit does not change that tested code.

Implemented:
- Dependency-free Rust protocol framing/name-assignment codec and CLI.
- Pure provider-neutral routing planner.
- Read-only PipeWire application inventory and explicit wire-name bindings.
- Complete-inventory lifecycle comparison and generation guards.
- Guarded route preparation and rollback descriptions.
- Read-only node/port/link projection, preserving foreign links.
- Pure ordered acknowledgment/receipt journal with actual link identity contracts.

Relevant owners: src/executor.rs, src/graph_snapshot.rs, src/receipts.rs,
tools/pipewire_inventory.py, tools/pipewire_graph.py, existing planner/lifecycle.
Do not build parallel identity or ownership systems.

## What was actually validated
At the implementation checkpoint:
- Identical source: Windows and Ubuntu offline build; 36 Rust integration tests
  and 15 Python tests on each. Python tests invoked the actual Rust CLI.
- Windows formatting check passed. Linux rustfmt was unavailable; do not claim it ran.
- Real read-only graph: two audio nodes, six ports, two foreign links, one live
  stream. Explicit preparation produced zero changes for an already-correct route.
  A wrong destination serial was refused with ReusedObject.
- Earlier real Firefox playback discovery matched native metadata.
- Earlier isolated private PipeWire core: one owned synthetic stereo stream moved
  A -> disconnected -> B, matching planner identities. Owned objects/processes and
  temporary data were removed; the desktop's immediate filtered baseline matched.

Important failed experiment:
Creating temporary sinks on the desktop/shared PipeWire core caused fallback
auto_null/default behavior to change. The run aborted before routing links.
Cleanup restored logical routes/defaults, with new object identities and a
suspended-format difference documented. Do not repeat this design.
The successful routing test used an explicitly selected private core/socket/runtime,
without desktop device discovery, fallback handling, Pulse or a session manager.
It retained standard access controls.

These results DO NOT prove audible output, hardware transport, remote Windows
visibility, native production executor operation or end-to-end interoperability.

## Next bounded implementation batch

**Completed continuation:** the optional `native-pipewire` feature now provides
the native executor described below. Read [native-executor.md](native-executor.md)
for source owners, build dependencies, exact runtime outcomes and limits.
The owner explicitly authorized the disposable-core VM test during this
continuation. That scope did not include desktop or hardware routing.
The actual Rust/C backend moved one owned stereo stream A -> B, stopped after
three acknowledged steps of a controlled B -> A failure, then restored B with
new native receipts. Owned cleanup and the desktop settings/state comparison
passed. Ordinary `link-protocol apply` remains disabled.

**Completed fault/recovery continuation:** explicit rollback continuation now
retains the acknowledged recovery prefix and revalidates before resuming.
Actual private-core tests passed for native factory rejection, deliberately
withheld acknowledgment delivery, interrupted recovery and rejected restoration.
Request counters proved no automatic retries or repeated acknowledged operations.
A 32-iteration transient-client smoke test preserved the current receipts and
issued no mutation requests. See [native-recovery.md](native-recovery.md).
The owner continued the existing isolated-VM authorization; physical/desktop
routing stayed outside scope.

**Completed protocol continuation:** focused static tracing established A2 body
handling, ignored constructor/writer results, the short-poll length-check gap and
serializer prefix/capacity behavior. `src/session.rs` now sequences A0/A1/A2,
publishes validated complete lists and decodes bounded assignment replies over
a caller-provided transport. Failures halt without retry; metadata remains opaque.
All 25 focused session/codec/planner tests pass on identical Windows/Linux code.
No USB backend or hardware test exists. Read [protocol-session.md](protocol-session.md)
for exact provenance, deliberate policies and validation hashes.

**Completed descriptor preparation:** `src/usb_descriptors.rs` and the offline
`inspect-usb-descriptors` CLI now validate a bounded cached descriptor stream.
`tools/usb_descriptor_probe.py` reads one explicitly selected sysfs cache and
reports descriptor candidates separately from active alternates/driver ownership.
All 25 focused Rust tests pass on identical Windows/Linux code; 5 new Python
probe tests pass on Linux with synthetic files and the actual CLI. There is still
no hardware descriptor evidence or USB transport. Read
[usb-descriptor-validation.md](usb-descriptor-validation.md) for exact limits/hashes
and a reviewable descriptor-only physical test. The owner wants the existing VM
first, then native Linux on the gaming PC; that preference does not authorize
passthrough or device I/O.

**Authorized hardware preflight:** the owner explicitly authorized the descriptor-only
VM procedure. Fresh private host/guest baselines were saved; repeated host defaults,
endpoint formats/volumes/mutes/session settings and persisted audio-policy registry
hashes matched. One physical target is present on Windows. The VM has no USB
controller/root hubs (all three VM controller flags FALSE); no handoff or hardware
probe has run. The owner must add/enable a VMware USB controller and manage any
needed guest restart. The owner subsequently enabled USB 1.1 and restarted the
guest. Read-only checks confirm one UHCI controller (class 0x0c0300, uhci_hcd)
and one 12 Mb/s root hub; still no BEACN target is present in the guest. Fresh
post-controller guest and Windows baselines are saved. The next action is owner-managed
Studio connection through VMware's Removable Devices menu, followed by the one
bounded cache probe and owner-managed return/compare. USB 1.1 is only a candidate
for this descriptor observation; device fallback is unverified and it does not
qualify later audio operation. The owner then attempted connection and received
VMware's ideal-controller/fallback warning. A read-only guest query timed out;
Windows still reported the physical target and three USB functions. No cached
descriptors were read. Host defaults/formats/volumes/mutes and per-user audio/policy
trees match the refreshed baseline, but a Chatterino session appeared and 17
MMDevices registry values changed (16 binary fields plus one endpoint-reference
string). Their cause/semantics are unresolved; do not call this an exact unchanged
baseline or blindly rewrite registry values. Return/keep Studio on Windows and
resolve the USB compatibility setting before another handoff. Continue the
existing descriptor-only authorization. Do not ask for that
same test permission again. No protocol writes, capture or routing is authorized.
See the preflight section in [usb-descriptor-validation.md](usb-descriptor-validation.md).

**Completed physical descriptor observation:** one attached Studio validated at
configuration 1, vendor interface 3, alternate 1, bulk OUT 03 / IN 83 with 512-byte
max packets. Current alternate was 0, no driver bound on interface 3; audio interfaces
0–2 were bound to snd-usb-audio. Cached speed reported 480 Mb/s. The owner returned
the device; guest target count is zero and Windows functions are OK. Defaults,
formats, volumes/mutes and logical guest settings match, but object identities and
some Windows/guest persistent state differ. Do not claim an exact unchanged baseline.
Read [usb-descriptor-observation.md](usb-descriptor-observation.md) for hashes,
the complete comparison and limits. No protocol/control writes or alternate change
were performed. Descriptor-only permission is completed; it does not authorize
a handshake. Preserve full state-file contents in future baselines, not only hashes.

**Completed native USB implementation:** `native-usb` now builds a selected-fd
libusb adapter with discovery disabled. `usb_handshake` permits only A0/A1/A2,
revalidates exact profile/ownership/audio state, never retries and reports claim
release/restoration. Publication/polling remain refused. Windows default build and
25 focused tests passed; Linux default/optional builds, 33 focused tests, 1 ABI unit
test and 7 runner tests passed. Final runner tests also pass on Windows. Saved
physical descriptors pass the strict profile with no new device access.
Read [native-usb-handshake.md](native-usb-handshake.md) for exact hashes, limitations
and a prepared one-shot hardware test with full persistent-file backups. The normal
CLI remains disabled. No hardware handshake or new passthrough occurred.

**Current owner authorization:** after reviewing the completed USB implementation,
the owner explicitly authorized the hardware and VM operations needed to get the
replacement working, including temporary audio interruptions. Continue necessary
implementation and bounded hardware tests without repeated routine approval
requests. Preserve settings snapshots and unrelated work. Ask before database
edits or deleting applications/system files. This authorization comes from the
current owner conversation, not from the earlier descriptor-only permission.

**First authorized handshake attempt:** the owner connected Studio after fresh
full-content baselines. Setup stopped on revalidation before any A0/A1/A2 bulk
exchange. Claim release and device baseline verification succeeded; attached versus
post-child guest graph/state files match exactly. The adapter's likely mismatch
was Linux's sysfs `usbfs` userspace-claim driver versus libusb's inactive-kernel
report. It now accepts that pair only after its retained handle acknowledges the
claim. The corrected native build, two unit tests and eight handshake tests passed
before the corrected hardware attempt below. Read [native-usb-handshake.md](native-usb-handshake.md)
for provenance and distinction between observed failure and inferred cause.

**Completed first return comparison:** guest target is absent and Windows USB
functions report OK. Windows defaults/formats/volumes/mutes match; guest state-file
contents/modes match exactly. Guest graph properties/parameters and metadata match
after accounting for one recreated node and four ports. Windows sessions retain
the same instances with state differences, and MMDevices has 17 changed values
(16 binary, one string); user audio/policy registry trees match. Preserve all full
contents/differences; no blind registry rewrite or exact unchanged-baseline claim.

**Completed corrected hardware handshake:** the owner connected Studio under the
standing hardware/VM authorization. With a 482-second-old fresh baseline, the
corrected native executable completed exact four-byte A0/A1 writes and a 512-byte
A2 reply, released interface 3 and verified the original device baseline. No
timeout/retry occurred; the child exited 0. Attached-versus-post-child graph and
state files match exactly. Read-only sysfs confirmed all alternates zero, audio
drivers preserved and no vendor-interface driver. After owner return, guest target
is absent and all Windows USB functions report OK. Windows defaults, endpoint
state/format/volume/mute and sessions match; guest state files match exactly. Guest
graph settings match after one recreated node/four ports; MMDevices has the same
kind of 17 recorded value differences, with user audio/policy trees matching.
No persistence was rewritten. See [native-usb-handshake.md](native-usb-handshake.md)
for exact hashes and limits. App publication/polling/audio remain untested.

**Completed publication implementation:** an explicit native mode permits one
pinned single-app list and one assignment poll through the existing owners.
Default handshake-only access stays restricted; zero stays opaque and no routes
are applied. Windows/Linux focused tests/builds pass. An actual owned `pw-cat`
stream in a private core was discovered/bound to `BEACN Linux VM Test`; the source
probe and cleanup restored the private graph and desktop/state files exactly,
without USB access. Read [native-usb-publication.md](native-usb-publication.md)
for source owners, final hashes, supervision and remaining limits.

**Current testing constraint:** Windows BEACN Link on the gaming/host PC must be
closed; the owner closed it and the process check confirmed none. The replacement
runs in the Linux VM, while BEACN App remains on the Windows streaming PC. Verify
Link remains closed before each publication attempt; do not launch it to make
Linux interoperability appear to work.

**Completed hardware publication:** Windows Link stayed closed and the Linux
replacement completed an app-list write. The first immediate poll returned
ACFFFFFF; cleanup passed, with visibility unobserved because the owner did not
know where to look. The owner supplied a BEACN App v1.4.0 screenshot; identify the
bottom ASSIGNMENTS / Applications box containing `obs64 System`, immediately
right of Outgoing Studio Link Assignments, before the next observation.
A separately built diagnostic example waits 500 ms before its one poll and
reports only the unknown boolean at A2 byte 28. The controlled next attempt
returned a valid no-payload poll (512 bytes, 508 ignored tail) and exited 0 with
verified device/private-process cleanup. A2 byte 28 was zero; its meaning is
unknown. The owner saw `BEACN Linux VM Test` briefly in that remote App box.
No assignment records or audio interoperability are established.

That second attempt used a fresh connected-state protocol baseline because the
owner had already reattached. Preserve the distinction from the earlier full
pre-attachment/returned references. The connected guest had one BEACN ALSA device,
22 nodes/76 ports/16 existing links; native protocol execution preserved its
entire graph and state files exactly. After owner return, Windows audio settings
and guest returned graph semantics match the prior returned reference; guest
state files match the fresh baseline. Cross-handoff exact comparisons differ by
device presence and registry values. All evidence is retained, with no rewrites.
Read [native-usb-publication.md](native-usb-publication.md) for exact outcomes,
comparison scopes and hashes. Studio is back on Windows.

**Periodic implementation completed, hardware pending:** `native_usb_periodic`
opts into 1..40 publication/poll cycles using the existing selected-device owner.
Fresh, strictly increasing inventories are requested after initialization, before
each publication, and after each reply. The runner checks its retained private
core/playback processes, exact application/stream identities, live state and no
private links. Frozen/rebound/disappeared sources or any failed exchange halt;
there is no retry or reconnect. Defaults remain handshake-only/one-shot.
Known bound assignments 1..4 stop further zero publication and are planned against
the fresh inventory without applying routes. The child is externally supervised
for 50 seconds with bounded IPC and retained-process cleanup.

Windows and Linux focused checks passed: 28 Rust integration cases, Linux's two
native USB unit checks, and 16 Python runner cases on Linux (two Linux-only cases
skip on Windows). The final source, differing only by two documentation comments,
was rebuilt in Linux; its three-cycle private-source probe served seven fresh
inventories and restored the desktop graph/state files exactly. No USB I/O occurred.
Final source is `beacn-link-usb-publication-periodic-final-20261012T153055Z/source`
under the existing guest home. See [native-usb-publication.md](native-usb-publication.md)
for hashes and precise validation scope. Guest free disk is about 21.3 GB.

**Hardware attempt stopped before publication:** the owner connected Studio and
was ready at the exact Applications box. A0/A1 completed, but the A2 native read
returned an unknown transport outcome. No list, periodic cycle, assignment or
route was submitted afterward; no retry occurred. Device release/baseline,
private graph restoration and both owned process exits passed. Connected desktop
and all tracked state files matched exactly after cleanup. Studio is back on
Windows; Windows defaults/endpoint states/formats/volumes/mutes/sessions and guest
state files match the pre-test baseline. Guest graph exact comparison differs
across handoff; 16 machine-audio registry values differ and remain preserved.
See [native-usb-publication.md](native-usb-publication.md) for hashes and limits.

The native read path had discarded libusb's error code/transferred count. These
are now logged on failed bulk I/O without payloads, retry or treating partial
bytes as success. The new exact tree was built in Linux; both native unit checks
passed. Latest build source is
`beacn-link-usb-publication-periodic-errors-20261012T153706Z/source` under the guest
home. This diagnostic change has not been tested on hardware. Free disk: 21.1 GB.

**Focused startup continuation:** the pinned Windows constructor explicitly
selects alternate zero, waits until the next millisecond tick, selects alternate
one, then resets vendor IN 83 before A0/A1/A2. The previously omitted wait and
pipe preparation are now an explicit `Startup::ObservedWindows` experiment;
defaults are unchanged. Retained identity/profile checks bracket each operation;
any error releases only the owned claim and stops before bulk. There is no device
reset, audio-interface mutation, automatic stall recovery or increased timeout.
The worker starts after initialization, so its scheduling cannot explain this
A2 failure. Full reconnect discovery remains untraced. Read
[protocol-session.md](protocol-session.md) for anchors, provenance and limits.

Windows/Linux focused validation passed 31 Rust integration cases; Linux also
passed two native unit checks and 17 Python runner cases. Exact Linux examples
built under `beacn-link-usb-publication-startup-20261012T154756Z/source` in the
existing guest home. Free disk is about 20.9 GB. This startup option is not yet
hardware validated; passing a test would not establish the earlier failure cause.

**Observed-startup hardware result:** initialization completed and cycle 1
published/polled successfully (512-byte no-payload response, 508 ignored tail).
Cycle 2's poll returned ACFFFFFF; the current policy halted without retry. No
assignment records or routes were applied. Claim/baseline, private graph and both
owned process exits were verified. Studio returned to Windows; host checked audio
settings and guest state files match baseline, with handoff graph/registry
differences retained. See publication docs for hashes and comparison limits.
This is one successful startup/cycle, not sustained visibility or a proven fix.

**Rejected-marker continuation implemented:** focused tracing confirms ACFFFFFF
returns from the stock poll callback without assignments/counter update or worker/
timer stop. An explicit `PollPolicy::ReportRejectedMarker` now reports the exact
marker separately and permits the next scheduled bounded cycle after fresh
inventory. Strict defaults and transport/partial/malformed failure halts remain.
It does not infer no-payload, peer health or a decoded firmware meaning. Default
and native examples built; Linux passed 38 focused Rust cases, two native unit
checks and 23 Python runner cases. Hardware validation is pending. Latest source:
`beacn-link-usb-publication-marker-20261012T162455Z/source` in the guest home.
Free guest disk is about 20.5 GB. Read publication/session docs for hashes/anchors.

The owner now requests direct USB handoff automation and
explicitly forbids computer-use/mouse automation. Native computer-use inspection
failed before any UI input and was stopped. Direct live VMware USB control remains
unverified. Fresh `prepare-connected` / `compare-connected` are now implemented
and use the existing snapshot/cache owners, so Studio can remain in the VM across
tests; return only at the end of the testing session. Keep one attempt
per evidence directory and no automatic USB retries. Sustained publication,
assignment receipt and audio remain pending; ordinary apply stays disabled.

Connected-workflow source built at
`beacn-link-usb-publication-connected-final-20261012T160535Z/source` in the existing
guest home. Linux passed 22 runner tests; Windows passed 20 with two Linux-only
cases skipped. Native examples rebuilt; Rust behavior is unchanged from the
31-case startup checks. Actual connected-stage hardware validation is pending.
The ignored `local/connected_usb_attempt.py` composes fresh connected prepare,
one authorized run, connected compare and host snapshots through direct SSH,
without switching USB or invoking any UI. Its read-only `check` verified source
pins and guest target count zero. It has not run a protocol attempt. Record a full
pre-handoff baseline at session start separately; a connected baseline is not a
pre-attachment backup. Free guest disk is about 20.7 GB. Read publication docs for
exact hashes and scope. No UI automation is permitted for future continuation.

**Historical next-work note, superseded by the checkpoint above:** a fresh-snapshot 40-cycle observed-marker hardware test. The ignored
`local/marker_connected_attempt.py` pins this build and composes connected prepare,
one run and connected comparison through direct SSH. Full host/guest pre-handoff
backups are separately saved by `local/publication_marker_prepare.py`. Its read-only
check confirmed target count zero and source pins. The owner should attach once
and leave Studio in the VM for this testing session; compare after each owned
claim release, and perform the Windows return comparison at session end. Keep
Windows Link closed and identify the same exact streaming-PC Applications box.
Sustained visibility, assignment receipt and audio remain unproven.
A receiving agent should use this authorization context alongside the current
owner instructions and applicable execution environment.
The first acceptance remains one real Linux app visible/routable in Windows App
with verified audio. Local gaps also remain across native core
disconnect/restart, long-lived registry cleanup and capacity. Withheld acknowledgment
delivery is not proof of packet-loss handling. Keep desktop apply disabled.
The original batch below records the first native implementation.

Implement native execution and acknowledgment capture for the existing guarded
plan and receipt journal, first restricted to an explicitly selected disposable
private PipeWire core. Keep normal desktop CLI apply disabled by default.

1. Inspect current source/contracts and private-core evidence. Choose the smallest
   maintainable native PipeWire interface; document any needed dependency.
2. Subscribe to relevant registry/core events and establish the intended server
   instance. Revalidate node/port/link IDs plus serials before each operation.
   A core cookie and two snapshots do not eliminate races.
3. Execute only planned changes. Record success only after observed native
   acknowledgment and exact endpoint/ownership verification.
4. Maintain actual created-link receipts. Owner tags alone never authorize deletion.
   Preserve foreign links, desktop fallback/defaults and unrelated applications.
5. On failure or unknown outcome, stop progression. Reconcile the pending effect
   before retry or rollback. Roll back only the known successful prefix, using fresh
   identities; never delete an ambiguous/reused object.
6. Under the owner's permitted isolated test scope, snapshot first, exercise
   forward A-to-B and controlled partial failure/recovery with synthetic audio,
   then verify cleanup and the untouched desktop baseline.
7. Update canonical docs with exact observed outcomes, limits and next step.
   Build the final code tree; run focused checks and publish only as authorized.

Do not spend another batch merely rearranging docs or duplicating tests. Deliver
the native integration slice while retaining the safety properties above.

## Longer roadmap / unresolved protocol work
After native local routing is proven:
- USB/interface discovery and coexistence with Linux audio devices.
- Full handshake, disconnect/reconnect and transport behavior.
- Remote application-list publication and assignment ingestion.
- Explicit assignment-zero semantics and Unicode/length behavior.
- One Linux application visible in Windows App, routed there, with verified audio.
- Multi-app/four-stereo-pair lifecycle, reconnects, rate changes and error recovery.

Hardware captures, USB passthrough and driver/firmware changes need separately
scoped authorization. Physical cached descriptors are validated; hardware protocol
execution and audio interoperability remain unverified.

## Static findings worth preserving
Installed Windows Link studied: 1.0.4.0, native AMD64 PE, JUCE 7.0.6.
Protocol docs contain binary SHA256 and address provenance.
USB VID 33AE / PID 4003; vendor WinUSB MI03 alongside the audio interface.
Candidate bulk OUT 03 / IN 83, little-endian AC envelope and bounded records.
Assignments 1–4 map to Link outputs; zero remains opaque.
The Windows program uses per-process audio-policy routing, so restarting it is
not evidence that prior routing is undone.
The app-list refresh timer was statically reconstructed as 500 ms.
Preserve the observed character-count/byte-copy mismatch rather than “fixing”
wire encoding without runtime evidence. Read docs/protocol.md for actual offsets,
confidence and unknowns.

## Environment and private evidence
Windows has the checkout and existing dedicated SSH setup for the Ubuntu guest.
VMware VM is under D:\VMs\BEACN-Link-Linux.
Guest was upgraded by the owner from Ubuntu 24.04.5 to 26.04.1; current evidence
uses Ubuntu 26.04.1, PipeWire 1.6.2, Rust/Cargo 1.88.0 and Python 3.14.4.
VM configuration at the original graph checkpoint: 2 vCPU, 4 GiB RAM,
40 GiB growable virtual disk, NAT networking; no USB, physical sound passthrough
or shared folders. The owner subsequently enabled USB 1.1 for the completed
descriptor test; current device ownership must be checked before new execution.
Use SSH for commands. Screen lock does not stop SSH; VM suspension/host sleep does.

No GitHub credentials were installed in the guest. Sources were archived, transferred
using the existing verified SSH connection, hash-checked, and built in isolated dirs.
The original graph checkpoint source is under the user's home in
beacn-link-graph-20261011T204629Z/source.
The latest built USB source is in the existing guest user's home under
beacn-link-usb-publication-periodic-final-20261012T153055Z/source. Its periodic
source/cleanup probe passed without USB. The prior diagnostic source under
beacn-link-usb-publication-diagnostic-20261012T144842Z/source passed hardware
list/no-payload poll plus brief owner-observed visibility. The earlier source/cleanup
probe used the preserved original publication
source under beacn-link-usb-publication-final-20261012T143405Z/source.
The corrected handshake source under
beacn-link-usb-claim-20261012T134712Z/source passed hardware initialization.
The first runtime attempt used
the preserved original source under
beacn-link-usb-final-20261012T132115Z/source. Library-version/cache-check modes remain
offline. Latest free disk is about 21.6 GB; continue lightweight builds only.
Locate the existing isolated Rust homes from local artifacts rather than installing
another toolchain. Resolve current IP/user/known_hosts/key references locally;
keep those details and raw evidence out of Git and logs.
Use StrictHostKeyChecking=yes, BatchMode=yes and the dedicated identity.
Do not read private key contents or weaken permissions.

Private Windows evidence beside the existing VM host-SSH setup includes:
- controlled-virtual-routing-result.json
- virtual-routing-baseline.json
- virtual-routing-settled-check.json
- virtual-routing-settled-comparison.json
- private-core-routing-result.json
- private-core-routing-verification.json
plus source manifests/build logs. Verify actual filenames before using them.
The earlier graph checkpoint free-space result was roughly 24 GB decimal, below the owner's
25 GB heavy-build threshold. These were lightweight single-job builds, not a
permission to launch large builds. Measure current space/resource use first.

REA checkout: E:\git\rea, historical main
b2da21abe11ac43a4cc24b233bd52ce10995e4a1. It was cloned but not installed/run.
No need to run it to continue the current native routing slice.

## Lightweight validation commands
Run from the repository with the existing toolchain selected:
- cargo build --offline --locked -j 1
- cargo test --offline --locked -j 1
- cargo fmt --check (where installed)
- Set BEACN_TEST_PLANNER_BIN to the absolute built link-protocol executable.
- python -m unittest discover -s tests_python
On Windows the binary ends in .exe; on Linux it does not.
Use the repository's exact scripts/contracts where they supersede this summary.
No GitHub CI/workflow is requested.

## Why work is being handed over
The prior assistant's Windows task-start service repeatedly returned
AppServerBackendRequestError, including for read-only verification. The PC still
reported connected. This does not establish a Deadlink, SSH, VM or project bug.
Earlier publication checks also failed to recognize relayed approvals; the owner
then committed/pushed successfully. Publication is complete, not pending.
Use the new agent's supported direct environment and current owner instructions.
Do not repair/reinstall Deadlink, reset credentials or repeat approval loops
because of that historical orchestration failure.

## First actions for the receiving agent
1. Read AGENTS.md and this handoff, inspect git status, then fetch main.
   Fast-forward only if safe; preserve any local changes.
2. Reconcile against implementation commit 39d36a3 and read canonical docs.
3. Verify existing Windows/VM access read-only, toolchain and current disk space.
4. Follow the current next-work note: the one-shot USB implementation is complete,
   and the owner authorized the necessary hardware/VM continuation. The original
   native fault/recovery batch is implemented. Keep progress factual and report
   implemented / built / runtime-tested / pushed separately.
5. Leave Jawjack, Haiwire, AI benchmarks and other agents' processes alone.

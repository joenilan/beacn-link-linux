# Beacn LinX: ordinary application interoperability

Updated 2026-10-14. The owner named the replacement **Beacn LinX**. Existing Rust
library/package and offline CLI identifiers remain stable.
The owner also selected Rust for the application. The existing Rust core remains
the production foundation; Python is qualification tooling and the small C adapter
supplies the native PipeWire FFI. A UI framework has not been selected.

The current checkpoint is owner-heard real-application output on all four physical
pairs, with concurrent native control and exact laptop cleanup. See the completed
pair table below. Shared-desktop automation and persistent daemon acceptance remain
separate work.

## Real application result

A retained, unmodified Debian `mpv` 0.40.0 process now supplies the audio stream.
The native inventory observes its actual application name, client serial, PID and
node serial; no application-name or PID override supplies those identities.
The supervisor publishes the observed name `mpv` through the existing explicit
wire binding, codec and USB session. Windows BEACN App returned assignment 1;
the existing native planner/executor acknowledged two private FL/FR links.
The concurrent physical bridge then delivered the authored quiet media through
Studio's first pair while native publication and polling continued for 40 cycles.
The owner reported "i hear it". UI retention beyond that report was not separately
confirmed for this application run.

This is an ordinary application's real PipeWire backend on a controlled private
core. It does not discover or move existing shared-desktop applications, establish
PulseAudio/browser/game compatibility, or provide an installed daemon/GUI.
Default desktop apply remains disabled.

## Application owner and isolation

`tests_native/application_source.py` launches only mpv with its native PipeWire
backend and explicit private remote, configuration loading disabled, stereo/rate
selected, no video/terminal and no fallback backend. Its input is an authored WAV
wrapping the existing bounded quiet-tone pattern; no capture or third-party media.
The process-scoped `PIPEWIRE_PROPS` supplies only private node identity/token,
no autoconnect/reconnect, state-restoration refusal and DSP port configuration.
It does not override `application.name` or `application.process.id`.
The [mpv PipeWire implementation](https://github.com/mpv-player/mpv/blob/v0.40.0/audio/out/ao_pipewire.c)
and [PipeWire stream implementation](https://github.com/PipeWire/pipewire/blob/1.4.2/src/pipewire/stream.c)
describe the backend and process-scoped properties used here.

Every fresh inventory verifies the retained process is alive with the same start
time and executable path/inode/device/size/mtime, exactly one observed `mpv`
application/stream, the native client's authenticated peer PID, private node/token,
no device ownership and no autoconnect. Existing native core authentication,
serials, receipts, channel selection, route verification and USB/PCM guards remain
unchanged. Tags alone never authorize deleting a resource. Replacing the process,
binary, name, native peer or scope stops the run.

The disposable laptop lacked mpv. Debian package simulation identified eight new
packages with no removal/upgrade; only mpv and its required dependencies were
installed, without recommends. Package logs/version/binary hash remain private.
The application binary SHA256 is
`5457be986de69e315e78300ea08b99b2716127ce52b654e9fe18aff057f78eac`.
No working toolchain, audio service or existing application was reinstalled.

## First-pair evidence

Live source: `beacn-native-source-20261014T215427Z`.
Hardware run: `beacn-native-publication-20261014T215522Z`.
132 native active-PCM owner checks measured 1,664,177 frames of concurrent hardware
progress; 77 independent physical observations measured 1,655,057 frames.
The run exited 0 without timeout, preserved the mpv source and stopped all four
owned processes. Private graph, USB binding/ACL, closed PCMs and complete laptop
settings/routes/defaults/volumes/mutes/persistent-state comparisons were exact.
Streaming-PC saved settings are outside these local comparisons.

The initial no-USB probe moved mpv through assignments 1 and 2. A subsequent
five-cycle native probe moved the same real source through assignments 1, 2, 3
and 4, verifying each publication from actual native observations, then restored
the complete private graph and desktop/state baselines. Both remained synthetic
destination routing, with no USB exchanges or physical output.

## Explicit additional-pair implementation

Additional physical runs require `--studio-pair 2|3|4`,
`--authorize-studio-pair-tone`, `--audible-studio-pair`, continuous mode and the
existing isolated routing/publication authority. Legacy first-pair authority cannot
authorize a different pair. The native child selects the corresponding explicit
continuous flag, accepts only the selected nonzero assignment before route apply,
and exchanges exact pair-specific START/ACTIVE/STOP/COMPLETE messages. Wrong-pair
or duplicate IPC stops the supervisor. Other-pair replies never apply a route.

Only the selected destination has monitor ports enabled. The retained bridge links
its FL/FR monitors to AUX0/1, AUX2/3, AUX4/5 or AUX6/7 for pairs 1–4 respectively.
This mapping derives from the previously observed UCM split topology. The laptop's
current desktop does not expose those split endpoints; it supplies the selected
raw eight-channel ALSA device. Physical checks must separately establish delivery
on the expected streaming-PC channel. No foreign desktop endpoints are substituted
into the native private executor.

Thirty-three focused Python checks pass, covering ordinary process/executable/name/
native-peer scope, authored WAV format, selected-pair authority, exact IPC ordering,
wrong-pair rejection, distinct stereo slots and the prior continuous bridge guards.
Exact native programs build offline. The five-cycle mpv probe passed all four
native routes. The subsequent source-exit guard adds one focused case (34 total).

## All four physical pairs passed

Pair 1 used the source above. Pairs 2–4 used exact live source
`beacn-native-source-20261014T215833Z`. Each was a separate owner-ready 40-cycle run
with the same unmodified mpv binary, one returned selected assignment, two native
stereo receipts, concurrent physical playback/control and final post-audio verified
publication. Each completed 82 fresh inventories, seven NoPayload and 32 rejected
markers. Marker semantics remain unknown. Publication initially carried opaque
assignment zero; only actual native route verification authorized its nonzero value.

| Pair / expected channel | Received at cycle | Native active-PCM checks | Concurrent pointer progress | Independent observations / progress |
| --- | --- | --- | --- | --- |
| 1 / Link In / AUX0–1 | 6 | 132 | 1,664,177 frames | 77 / 1,655,057 frames |
| 2 / Link 2 In / AUX2–3 | 4 | 140 | 1,749,420 frames | 81 / 1,744,045 frames |
| 3 / Link 3 In / AUX4–5 | 9 | 120 | 1,501,272 frames | 69 / 1,497,913 frames |
| 4 / Link 4 In / AUX6–7 | 17 | 88 | 1,102,221 frames | 50 / 1,092,525 frames |

The owner heard all four. Pairs 2–4 confirmation questions explicitly covered
audible tones, the expected channel meter and mpv staying assigned; the owner
confirmed each worked. These are owner observations, not captured audio/UI or an
independent level/crosstalk measurement. No simultaneous multi-app/four-pair output
was tested. Physical mapping is established for these selected-pair runs, without
claiming desktop UCM split nodes exist on the laptop.

All four runs exited 0 without retry/timeout, preserved source identity, stopped
all four owned processes, restored the private core and selected USB ACL exactly,
closed both PCMs, and matched full laptop settings/routes/defaults/volumes/mutes,
state-file and connected-binding baselines. Streaming-PC saved settings were not
read or compared. Studio was fully released after each test; no USB toggles or
desktop audio-service restarts were needed.

Hardware journals, pairs 1–4, SHA256:

```
1ae84fbac815f738064fcffd76d80eec1a9ce9e34a0b51fb7deab081e23f6d06
ac7ef6e5608900edd3977fa69c9b9c9ef3b932f6a4b5b7969bf94230eee4eec7
f5c127bfbb19552fb2058de7df8a620ae7c10d842aa2a8c754840c31d41e448d
4c8bdf569faa989b1d46b18a512a22c13fa95cadd60aa3b3359f638cc08568fa
```

Physical-stage journals, pairs 1–4:

```
f3e091e3abafea439e59fe13227532a0aff092938554c73c820de963523aaef2
49f91d899c9606d0ba5a07b64907498f1f48d4965fa8115b6c6ac298e807a15c
25cf85a965e1396ebca3feab9b52230005f29bcca809b020c4ec02f3ffe2da92
c3ac10125be2076af058f2a9195100c82eb97e2b88db3d41d9018bb34d96e1da
```

1,193 evidence/provenance/owner-report files were copied and hashed outside Git.
Owner-report SHA256:
`c188b990e6cb4f71c999967d6f8360bc5cc35af53adbc066b3f9ecee5102adac`.
Native environment remains Debian 13.7 / kernel 6.12.111+deb13-amd64, PipeWire 1.4.2,
WirePlumber 0.5.8, libusb 1.0.28, Python 3.13 and isolated Rust 1.88. The last owner
streaming-PC screenshot showed BEACN App v1.4.0; live version was not requeried.

## No-USB source exit and fresh-session check

`--probe-source-exit` is restricted to the five-cycle ordinary-application
`routing-probe`, never hardware. After two successful native routes it terminates
only retained mpv, saves the departed graph and runs the actual process-owner guard.
At inventory request 4 the supervisor detected the exited process and delivered
no further inventory. Assignments 3/4 were not applied. The waiting native test
child was terminated (exit -9), releasing its non-lingering resources; this is
expected negative-test teardown, not a successful graceful native exit or recovery.
The runner returned 2 as expected. All three owned processes stopped and private
graph/laptop desktop/state baselines matched exactly.

A separate fresh private core/session started mpv with a new PID/application identity
and successfully applied all four routes with 11 fresh inventories, then cleaned
up exactly. No old receipts or bindings were imported. This proves fail-closed
source departure and explicit fresh-session startup; it does not prove automatic
recovery, same-core app reopen, USB reconnect or long-lived daemon operation.
Both checks used exact source `beacn-native-source-20261014T220341Z`, with no USB
transfers, physical audio, capture or changes to the default apply policy.

Exit/fresh-session journal hashes respectively:
`37885ed542525019b608f1adbd5748d91b72f46d699665e3215023e5affe0fa1`,
`8def273bbbe53ac7558220d94c202db5231fb3155a8f0035c255ade9be18724b`.

## Next

Build the Rust application/service around these owners: controlled discovery and
selection of ordinary shared-desktop streams, multiple applications, a durable
session lifecycle and explicit error/reconnect behavior. Define the user controls
before selecting the Rust UI framework. Keep desktop apply disabled until its
integration is separately validated. The owner's streaming window takes priority:
hardware is released; remaining work can proceed offline after Studio returns to
the gaming PC and Windows Link. No additional hardware execution is assumed after
that return.

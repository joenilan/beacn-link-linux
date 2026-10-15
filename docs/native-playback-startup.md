# Native playback startup and first physical acceptance

Updated 2026-10-15 UTC. The owner was awake and explicitly started the attended
checks, then reported **“Heard tones; mpv stayed in Link In.”** This qualifies one
real application on the first Studio pair using the in-process Rust/C owner.
It does not enable the normal desktop backend or qualify the other physical pairs.

## Observed failure and correction

The first native-owner run accepted assignment 1, created its owned sink/links,
then failed its strict PCM proof. Initial errors did not identify which metadata
condition failed. Read-only failure diagnostics were added without changing the
failed decision. An assigned diagnostic run showed:

- PCM state PREPARED, expected retained server owner and open matching device FD.
- FLOAT_LE, eight channels, 96000 Hz; pinned process generation/device identity
  unchanged and capture closed.
- No native playback ACTIVE marker or verified clock progress before cleanup.

The original code required RUNNING immediately after link acknowledgment.
That acknowledgment did not establish completion of ALSA startup. The corrected
successful run's independent metadata sampler observed PREPARED followed by
RUNNING about 4.512 ms later. That interval is a sampled observation, not a
guaranteed device startup time or a timing/performance benchmark.

`PinnedPlayback::startup_probe` now recognizes PREPARED only at initial creation
and only with the same complete owner/generation/device/FD/format/capture proof.
It never records PREPARED as playback progress. The native owner waits at most
two seconds, revalidating route publication and native proxy/graph receipts on
each attempt. A failed observation exits immediately; only a positively verified
pending state can wait. The ordinary `validate`, refresh and clock guards still
require RUNNING, and startup cannot be reentered after accepting RUNNING.
Failure, deadline and cleanup semantics remain owned by the existing session.

No UNKNOWN/OPEN/SETUP/XRUN/DRAINING state, wrong owner, wrong format, missing FD,
changed generation or open capture is accepted as pending. No USB retry, reset,
driver detach, new backend factory or relaxed normal playback check was added.
The 40-cycle and 4096-PCM-check limits are unchanged. Diagnostic text is a second
best-effort metadata observation and cannot authorize accepting a failed proof.

## Exact physical evidence

Qualified source: `beacn-native-source-20261015T120640Z`.
Frozen native Studio GUI SHA256:
`eac7adf632f6e771332a6928083ec00e6dfa3d2f5af74ed59b0680a37fc690a7`.

Successful run: `beacn-native-studio-gui-20261015T120932Z`.
Complete result SHA256:
`33c287df7e5899086716a0224d1f36b05d1d99a7a8d90f920e3d0d4743b31156`.

The real mpv application received assignment 1 at cycle 5. Native playback ACTIVE
was followed by normal physical progress and COMPLETE before the bounded end:
40 protocol cycles, 132 owned PCM checks and pointer progress 1,555,988. The
independent metadata-only observer recorded 69 clock checks and queue progress
1,646,226. Its journal identifies the physical owner as native Rust/C, with no
audio payload reads. Python did not launch the historical external playback bridge.

The owner confirmed audible repeating tones and mpv remaining in first Link In
until the run ended. The adapter, native routes, transport and all owned processes
were released; both physical PCMs were closed. The private core, complete immediate
laptop desktop/settings/routes/defaults/volumes/mutes/state-file snapshot and
connected USB binding matched. A later read-only audit matched the same baseline.
Streaming-PC saved settings were not independently inspected; hearing and visible
assignment are owner observations, not captured audio or crosstalk measurements.

## Preserved failures and comparison limits

- `beacn-native-studio-gui-20261015T115444Z`: assignment 1 accepted, generic PCM
  proof failure and no owner-heard sound. Owned resources/PCMs released and USB
  binding matched. The immediate desktop/state comparison was **unequal**: an
  unrelated desktop stream's node/port/link identities, one negotiated Format
  cache and stream-properties hash changed. Those differences were preserved,
  not overwritten or retrospectively normalized into an exact match. Cause was
  not established by that comparison.
- `beacn-native-studio-gui-20261015T115748Z`: a silent diagnostic received no
  assignments and never opened native playback. It restored its baseline, but is
  not a successful PCM diagnostic or hardware playback test.
- `beacn-native-studio-gui-20261015T120351Z`: the assigned diagnostic proved the
  PREPARED metadata condition above, then stopped. Complete-result SHA256
  `1455f397e1faaf4bdef2f6cf361b73f9e98619412f4946bee252b84dfe289a36`.
  Its full desktop/state and USB binding comparisons matched.

Raw snapshots, failed/successful journals, source/build hashes and the owner's
confirmation remain private outside Git. No failed result was replaced.

## Focused verification and remaining work

Six native-playback library checks pass, including verified pending-to-running,
no retry on error, deadline, strict ordinary state validation, changed owner/
format/generation, open capture and missing-FD cases. Five native-usb-only library
checks also pass, confirming the historical supervisor proof stays strict.

The exact source also passes native private-core GUI playback/Pause/held pause/
reconnect/Quit, with actual native ACTIVE/COMPLETE events and full outer baseline
restoration in `beacn-native-playback-gui-20261015T120826Z`. Service-journal SHA256
`4d2c09e6df3ee124dda71cf71f66e01d27928ea4a85274e9d3dfcd9140647940`.
Both native examples build offline from the same source. No new dependencies,
toolchains, drivers or firmware were installed. The current production default
excludes these physical qualification features and remains gated.

Next: physical Pause/reconnect and failure cleanup, then ordinary desktop/native
integration and the approved Console design. Native-owner physical pairs 2–4,
multi-app hardware, durable hotplug, long runs and installation remain unqualified.
Retain the minimal feature scope; volume/mute, meters and remote mix controls
must each earn their own implementation and runtime evidence.

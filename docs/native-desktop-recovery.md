# Native desktop recovery checkpoint

This describes the actual Rust/C desktop owner, not a script-owned routing path.
Private run receipts retain the full settings/graph comparisons; no audio payload
was recorded for these checks.

## Media Pause and waiting intent

Studio application rows use one **Assign to Link In** action to select the app,
start its session and verify the requested pair. A separate Select step is no
longer required. Remove remains available for clearing a selection.

The managed app now keeps a separate in-memory app/channel intent after a
verified assignment. A read-only native observation pins the exact PipeWire
client serial and reported PID on the same authenticated core, including clients
with zero audio nodes. It distinguishes one running source, idle/no source, a
departed client and ambiguous streams. It never matches by app name or PID alone.

Media Pause releases both native owners and shows **Waiting for audio**, an amber
icon, the remembered application well and unlit meters. Pending intent is not a
live selection, a verified assignment, or a playback/publication proof. When the
same client has exactly one current live stream again, LinX consumes that intent
before a single fresh select/start/assign attempt through the existing planner
and native proof owners. A changed stream generation does not reuse old handles.
Explicit Pause, Remove, reassignment or discovery loss cancels the waiter.

A Pause inside the serial cycle can recover only the named native idle-route
result: exact source/client/port identity, cache guard, metadata echo and idle
stereo endpoints must match. USB, protocol, physical and generic route failures
remain errors. Successful route and transport cleanup are required before waiting;
unverified cleanup blocks restart. A node disappearing inside that cycle before
the idle proof is obtained can still require explicit reassignment. Client/process
restart, cross-core matching and multi-stream recovery are not supported.

Focused checks cover seven intent cases, eight startup/request cases, a native
client-activity identity fixture and idle-stop cleanup refusal. A native Firefox
run verified single-action assignment, four ten-second media pauses and automatic
resumption on the original pair, including a recreated source generation. LinX
Pause during waiting cancelled resumption, and later media Play did not override
it. Explicit assignment afterward passed. Saved values and device binding
matched; Firefox's saved JSON property order changed. The graph matched except for the selected Firefox source and its
two port/link generations; all other fields and objects were retained. Native
window captures verified the waiting state. This is not multi-hour stability or
fresh owner listening acceptance of the changed build.

## Same-stream Pause and Connect

The managed desktop session remembers a pair only after a Running route has
verified assignment 1–4. Its in-memory intent pins the PipeWire core, selection
revision and complete selected application/stream roster. A newer observation of
the same roster may resume; core, stream, alias or selection changes invalidate
the intent. Connect applies it through the existing native assignment owner and
fresh proof. Explicit Assign supersedes the remembered pair without first
routing to the old pair. Restarting LinX does not persist this intent.

A live native Firefox Pause/Connect check resumed pair 2. The later media-waiting
path above adds bounded continuity for a surviving client and a fresh stream.

## Idle and closed sources

Forward routing still requires ACTIVE stereo links. Restoration alone may accept
INIT or PAUSED links when the exact pinned source is IDLE or SUSPENDED, with the
same endpoint, port-generation and stereo-count checks. That verifies idle route
intent, not playback. Earlier Firefox media Pause exercised this cleanup path;
that earlier build required an explicit Assign after a recreated stream.

When a source exits, native cleanup confirms disappearance of its exact node and
both pinned output-port generations after fresh graph barriers. It destroys only
retained owned bridges, avoids writes to a reused metadata subject, and requires
the original target generation to remain. Rust excludes only that closed source
generation from expected graph conservation. Replacement and unrelated graph
changes remain visible. Physical PCM closure and owned sink release still apply.

## Saved-target protection

An earlier Firefox soak stopped after about 200 seconds when its source exited.
Both native cleanup owners succeeded, but a persistent-state comparison found
WirePlumber had saved the temporary LinX sink name as Firefox's target. That run
was not a successful sustained-use acceptance. A separate volume change was also
observed and was not treated as LinX-owned.

The current backend requires the scoped
[WirePlumber 0.5.8 target-cache guard](wireplumber-integration.md) before opening
USB. Each desktop sink has a fresh random lease name. Guard loss stops forward
checks while allowing owned cleanup. The pinned upstream script differs only by
the predicate, its save exclusion and the readiness marker; stock persistence
for other targets and properties remains enabled.

On the test laptop, a stopped-WirePlumber migration repaired only the previously
recorded LinX-owned Firefox target field. The old file and full comparison were
preserved privately; current volumes and other fields were retained. Installing
the integration does not perform that migration or erase old entries elsewhere.

An actual native GUI run with an owned `pw-cat` silent source verified unchanged
saved-state hashes while assigned, then clean source-exit handling. After the
owned source exited, desktop state, persistent files and connected-device binding
matched the pre-launch baseline exactly. The source did not disable target
persistence. Silence came directly from `/dev/zero` through native `pw-cat`;
Python only drove UI requests and collected metadata.

The subsequent 1,200-second silent physical-path soak switched repeatedly across
all four pairs. It reached 1,195 observed cycles, 4,784 owned PCM checks and
hardware pointer progress 115,806,762 without a route error. Source exit released
both native owners, with desktop/state/binding exactly matching the pre-launch
baseline. RSS ranged from 185.8 to 190.4 MiB; sampled CPU was about 3.4% of one
core. It is neither heard-output acceptance nor proof of multi-hour stability.

The final build's native GUI also resumed the same source on pair 4, switched to
pair 3 and completed joined Quit while active. Its strict raw comparison found
an empty stock WirePlumber cache entry for the disposable Test source and a local
48-kHz negotiated-format cache. No saved target, volume or mute value changed.
The exact new empty entry was removed with the state owner stopped; after reload,
saved values matched, with only `default-routes` JSON serialization differing in
bytes. That lifecycle run is not counted as byte-exact graph/state restoration.

## Focused checks and remaining limits

The focused checks cover three resume-intent cases, ten native desktop-target
cases, three PCM guard cases, and five WirePlumber fixtures. All five integration
fixtures executed on Debian, including existing/future metadata readiness and
seven predicate cases. Missing readiness was also refused through the actual
GUI at zero playback cycles with exact state preservation.

The PCM file-descriptor check now stops when it finds the held exact descriptor;
it still bounds an unsuccessful search at 256 descriptors. Its real-process
fixture includes more than 300 unrelated descriptors without weakening a missing
match or format/clock/ownership checks.

One stereo application remains the limit. Client/process-restart reassignment,
broader hotplug recovery, installation on other WirePlumber versions, and owner
listening confirmation of the latest native build remain separate acceptance
items. Faders and mute controls remain disabled UI affordances. The subsequent
[native meters](native-meters.md) checkpoint adds aggregate-only stereo levels.

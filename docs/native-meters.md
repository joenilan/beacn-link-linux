# Native stereo level meters

LinX displays local digital audio levels for the one verified stereo application
assigned to a Link In pair. The meter tap reads the monitor of LinX's exact owned
sink through a native PipeWire `pw_filter` (`native/meter.inc`). It reduces each
channel to peak and RMS aggregates over roughly 40 ms windows and publishes those
values through a lock-free atomic mailbox (`src/metering.rs`). Audio samples are
not retained, exported, or recorded, and the meter does not write volume or mute
controls.

The UI accepts fresh mailbox sequences for up to 250 ms and refreshes at about
30 Hz only while the meter is active. It converts linear amplitudes to dBFS on a
-60 to 0 dBFS display range, with fast attack, slower decay, and a one-second
peak hold. Missing, invalid, expired, or paused readings clear the display.
Digital silence is at the -60 dBFS floor and leaves the rails unlit. Only the
selected verified pair lights; the other pairs remain unlit. The disabled faders
and mute buttons do not change audio levels.

These meters report local digital signal levels. They are not measurements of
the Windows BEACN mix, a calibrated analog 0 VU reference, or remote volume and
mute controls.

## Runtime evidence

A known-signal native run exercised all four Link In pairs. The observed stereo
raw peak amplitudes were 0.019989 and 0.0100098, approximately -33.984 and
-39.992 dBFS; RMS readings were approximately -37 and -43 dBFS. The run also
checked digital silence at the floor, signal resumption, Pause followed by
Connect on pair 4, and source-exit cleanup. Desktop state, saved state, and the
connected device binding matched the captured baseline at completion.

The generic signal source was native mpv raw PCM piped directly to native
`pw-cat`. Scripts drove the UI and collected aggregate readings; they did not
provide the audio tap or route. Focused validation passed four mailbox/freshness
checks, one C DSP check, and six Rust ballistics checks. The existing eleven
target and three PCM checks also passed.

The actual native GUI subsequently measured Firefox while it switched through
all four pairs, including a ten-second hold per pair and dark/light window
captures. Readings varied with the video; the sequence/frame counts advanced.
Pause cleared the meters and both native cleanup owners returned Ok. Firefox was
returned to its original Paused playback status. Its idle stream remained in the
graph with the restored original destination; the original destination's cached
48-kHz negotiated Format cleared when idle. Excluding only the two recreated
selected-link identities and that specific negotiated-format cache, the original
route graph matched. All saved setting values and the connected binding matched;
Firefox's saved JSON field ordering changed, so this is not byte-exact state
restoration. Private receipts retain both the strict and normalized comparisons.

During the four ten-second Firefox holds, LinX sampled 2.6–2.7% of one CPU core
and about 182 MiB RSS on the test laptop. These samples are not a controlled
performance benchmark or multi-hour stability proof. Direct listening confirmation
of the latest meter build remains separate from these aggregate measurements.

The final native binary also completed a 600-second calibrated meter hold with
repeated switches through all four pairs. Ninety aggregate samples stayed within
the expected stereo amplitudes; sampled CPU was 2.96% of one core and RSS stayed
at 181.96 MiB. Silence cleared the display and resumption restored the signal.
Pause cleaned both native owners, and Connect correctly retained the final pair,
Link 1. The collector incorrectly expected Link 4 at that last step and timed out;
its failed overall result is preserved. Final desktop, persistent-state bytes and
binding nevertheless matched exactly. This is evidence of the completed active
hold and cleanup, not an overall passing collector run or a multi-hour soak.
The collector now checks the actual pre-Pause pair.
The focused follow-up on the same final binary passed all four pairs, digital
silence/resumption, Pause/Connect on pair 4 and source exit, with exact desktop,
persistent-state bytes and binding restoration. Both native cleanup owners
succeeded. No application code changed to resolve the collector assertion.

Final native executable SHA256:
`7595abcda6a3e73486b9d72f53bff8a545d8ee9736fcd87477dc474477fbf6dd`.
The README images are from the earlier Firefox capture build, before the final
model change that also clears telemetry on non-Running session observations.

The initial live attempt correctly refused filter creation because the restricted
client configuration lacked `libpipewire-module-client-node`. The current
desktop-only configuration includes that module and one data loop for native
real-time processing. All cleanup owners succeeded on the refused attempt. Its
new, empty stock WirePlumber Test-source cache entry was removed with the app
closed and WirePlumber stopped; every other saved value was preserved.

## Ownership and lifetime

The meter uses the existing authenticated desktop core. Its filter node, two
FL/FR input ports and two links are retained and checked by exact generation and
endpoint receipts. The sink may fan out only to the retained physical bridge and
this tap. Rust conservation excludes those proven objects; unrelated changes are
still visible. Pair switches retain the same tap. Stop unlinks the retained tap,
removes its listener and destroys the filter before releasing the bridge/sink;
barriers verify disappearance. Partial creation retains cleanup ownership.

The real-time callback has a bounded stereo sample loop and atomic publication;
it allocates nothing, waits on no locks and performs no file or UI operations.
Only aggregate peak, mean-square power and frame counters cross into Rust.
The mailbox Arc outlives callback teardown. PipeWire 1.4.2 synchronously removes
the real-time callback when its listener is removed, before filter destruction.
See the [official filter API](https://docs.pipewire.org/group__pw__filter.html)
and the matching [PipeWire 1.4.2 implementation](https://github.com/PipeWire/pipewire/blob/1.4.2/src/pipewire/filter.c).

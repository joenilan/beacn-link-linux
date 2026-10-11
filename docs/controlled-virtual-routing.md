# Aborted controlled VM routing test

## Observed result (2026-10-11)

The first controlled test did not prove stream routing. It created two owned
temporary null outputs and an owned silent synthetic playback stream, then stopped
at the first pre-link guard with:

`AssertionError: defaults/clock metadata changed`

No `pw-link` operation ran, no test links were created, and the A-to-B move and
planner comparison stages were never reached. This was an aborted test, not a
successful routing demonstration.

The guest was Ubuntu 26.04.1 with PipeWire 1.6.2. Each temporary output was created
using `pw-cli -r pipewire-0` and `create-node adapter`, with
`factory.name=support.null-audio-sink`, `media.class=Audio/Sink/Virtual`,
`object.linger=false`, `node.autoconnect=false`, stereo FL/FR at 48000 Hz,
explicit DSP port configuration, and a unique ownership tag. An owned
`pw-cat` raw playback process used `--target 0` and disabled stream state
restoration on its own node. These properties did not isolate the outputs from
the shared server's fallback policy.

The original baseline and an immediate pre-mutation baseline remain private.
After terminating only the owned controller and playback processes, all tagged
objects were absent and the synthetic PCM file was deleted. Persistent
WirePlumber state hashes were unchanged. A later read-only snapshot showed both
defaults restored to `auto_null`, and the existing unrelated stream's original
logical FL/FR route restored by the session manager. No manual modification of
that stream was made.

Cleanup did not preserve exact graph identity: `auto_null` changed from node
35 / object serial 35 to node 64 / serial 101; associated ports and links also
changed. The existing stream remained node 59 / serial 72 relative to the
immediate pre-test baseline. After normalizing only the recreated fallback
object's identities, the remaining difference was its negotiated `Format`:
populated before and empty while suspended afterward. Volume, mute, stereo
configuration, defaults, clock metadata, and logical routes matched. Do not
describe this as an exact baseline restoration or a no-impact test.

## Source-backed explanation

PipeWire 1.6.2's
[fallback-sink registry handler and check](https://raw.githubusercontent.com/PipeWire/pipewire/1.6.2/src/modules/module-fallback-sink.c)
explicitly count both `Audio/Sink` and `Audio/Sink/Virtual`. When another sink
exists, the module destroys its fallback; when none remain, it creates one again.
The Pulse
[always-sink module](https://raw.githubusercontent.com/PipeWire/pipewire/1.6.2/src/modules/module-protocol-pulse/modules/module-always-sink.c)
loads that fallback module.

The installed WirePlumber default-node policy checked earlier selects exact
`Audio/Sink` / `Audio/Duplex` sink classes, excluding `Audio/Sink/Virtual`.
Its relevant file hash was
`49060517629cfbe8e45a2551606ed6efaf91e8443f2c0132d63ae1e6fb806373`.
This is a policy mismatch: a class can count as a replacement for the fallback
without being selectable as a default. The versioned code strongly explains the
observed fallback removal and recreation. No event trace or module log was
captured, so the precise event sequence is inferred rather than directly logged.
The earlier Pulse manager sink-class check was insufficient: fallback-sink uses
its own registry handler.

## Recommended next test (proposed, not executed)

Use a disposable private PipeWire core, not the desktop's `pipewire-0`.
The [daemon manual](https://docs.pipewire.org/page_man_pipewire_1.html)
documents a separate runtime directory, core/socket name, remote name, and
explicit configuration. The
[1.6.2 native protocol implementation](https://raw.githubusercontent.com/PipeWire/pipewire/1.6.2/src/modules/module-protocol-native.c)
resolves native sockets through `PIPEWIRE_RUNTIME_DIR`.

1. Save a fresh bounded desktop baseline while preserving previous evidence.
2. Create a short, owned mode-0700 temporary runtime directory, unique socket
   name, and private configuration. Resolve every test client to this directory
   and socket; fail closed if unavailable. Never fall back to `pipewire-0`.
3. Start only an owned foreground `pipewire` process with that explicit
   configuration. Load native protocol, metadata, node/client-node factories,
   adapter, link factory, and the standard access module. Include only support
   and audio conversion SPA mappings and an owned driver if needed. No hardware
   discovery, Pulse/fallback module, portal, bridge, session manager, or startup
   commands. Leave desktop services and permissions untouched.
4. Create two private null outputs and one private synthetic playback stream;
   verify server identity plus each owned node, port, PID, and object serial.
5. Use the existing read-only adapter and pure planner against that core.
   Explicitly link the owned stream's FL/FR ports to A, verify active links,
   disconnect those owned links after fresh identity checks, link to B, and
   verify the same stream/application identities and new active endpoints.
6. Terminate only owned processes, verify private objects/socket removal,
   delete only owned temporary files, and compare the desktop baseline again.

Do not launch the vendor `minimal.conf` unchanged: the
[1.6.2 example](https://raw.githubusercontent.com/PipeWire/pipewire/1.6.2/src/daemon/minimal.conf.in)
enables optional udev, Pulse, and JACK paths.
Use an explicit private configuration with controlled drop-ins;
[configuration loading](https://raw.githubusercontent.com/PipeWire/pipewire/1.6.2/src/pipewire/conf.c)
supports a private directory. Inspect client configuration too so no environment
or drop-in redirects clients. No global configuration edits are needed.

This proposed test would establish native graph routing and the inventory/planner
identity mapping on PipeWire. It would not establish desktop WirePlumber policy,
routing of existing applications, audible playback, BEACN hardware/USB routing,
Windows interoperability, or the Link discovery/remote-routing service. No
private core has been started or tested yet. A read-only check subsequently verified /usr/bin/pipewire, compiled and linked
against 1.6.2, and its installed client configuration. Private-core execution
remains untested.

## Evidence handling

Filtered baseline/result/comparison journals and the private harness remain
outside Git. No raw captures, browser content, keys, device serials, or machine
endpoint identifiers belong in this document. No additional audio mutation was
performed during the follow-up investigation.

# Controlled VM routing tests

This document preserves the earlier native-tool experiments. The later
[private-core executor validation](native-executor.md) exercises the actual
Rust/C backend, acknowledgment receipts and partial-failure recovery.

## Aborted shared-core attempt (2026-10-11)

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

## Private-core isolation method

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
5. Take an explicitly targeted, bounded private-core `pw-dump` snapshot and
   reuse the existing adapter decoder/projection and pure planner. The adapter's
   live collector itself remains hardcoded to the desktop `pipewire-0`; it was
   not redirected or modified.
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

## Successful private-core test (2026-10-11)

After the shared-core failure, a separate disposable private-core test passed
with overall exit 0. The guest remained Ubuntu 26.04.1, kernel 7.0.0-38-generic,
with the installed daemon compiled and linked against PipeWire 1.6.2.
No new tools were installed.

The private daemon used a unique socket in an owned mode-0700 runtime directory,
an explicit private server configuration, and a separate private client
configuration. Every test client targeted that socket and runtime; each private
snapshot checked the core name and owned server PID. No device discovery,
Pulse/fallback module, portal, bridge, session manager, startup commands, or
desktop service/configuration change was used. The standard server access
module remained loaded with its default policy.

Server modules were protocol-native, metadata, spa-node-factory, client-node,
adapter, link-factory, and access. Client modules were protocol-native,
client-node, adapter, and metadata. SPA mappings covered only support and audio
conversion/adapter factories. Server rate was 48000 Hz, quantum 1024, minimum
256, maximum 1024. DBus support was disabled only within the disposable
configurations. The private daemon's startup objects and commands were empty.

Two owned stereo null outputs used `Audio/Sink`,
`support.null-audio-sink`, `object.linger=false`, `node.autoconnect=false`,
and automatic DSP port configuration. An owned `pw-cat` process consumed
generated silent raw S16 PCM at 48000 Hz, FL/FR, with `--target 0` and
automatic DSP port configuration. An owned `pw-cli` controller retained the
non-lingering outputs. Ownership tags, node/port serials, and client PIDs were
checked before route operations; only the owned stream's links were disconnected.

### Same-stream route evidence

The playback node remained **23 / object serial 27** throughout. Output A was
**16 / serial 18**; output B was **19 / serial 21**.

| Stage | Observed links from stream 23 | Verified state |
| --- | --- | --- |
| Initially unlinked | None | Stream suspended |
| Linked A, FL | Link 27 / serial 34; port 24 to node 16 port 17 | Active; stream and A running |
| Linked A, FR | Link 28 / serial 37; port 25 to node 16 port 18 | Active |
| Disconnected A | None | Stream and A idle |
| Linked B, FL | Link 28 / serial 47; port 24 to node 19 port 20 | Active; stream and B running |
| Linked B, FR | Link 27 / serial 50; port 25 to node 19 port 21 | Active |

Link IDs 27 and 28 were reused with different object serials. Numeric IDs alone
would have confused the old and new links. The source node identity did not
change, and each channel was verified at its new destination.

The unchanged adapter decoder/projection resolved client 22 / serial 26 and
the owned playback PID into `pw-client:26:pid:10505`, with stream identity
`pw-node:27`. The PID came from client metadata; node ID 23 was not treated
as a PID. An explicit ASCII binding `controlled-test` was supplied.

Pure planner generations 1 and 2 proposed Link One and Link Two respectively,
with the same application and stream identities. Synthetic assignment frames
were `120000ac0f01636f6e74726f6c6c65642d7465737400` and
`120000ac0f02636f6e74726f6c6c65642d7465737400`. The harness explicitly mapped
One to private A and Two to private B and used native `pw-link` operations.
No BEACN message was transmitted. The repository planner remains pure; no
production routing executor was added.

### Cleanup and baseline comparison

All eleven recorded cleanup checks passed. After stopping the owned playback
and output-controller processes, no tagged nodes or links remained in the
private graph, and its filtered graph matched its pre-output baseline.
The owned server was then stopped; the temporary PCM, private configurations,
socket/lock files and runtime directory were removed. All three owned
processes were reaped.

The desktop's immediate pre-test snapshot contained 6 nodes, 10 ports, 2 links,
0 devices, and 4 metadata objects. The post-cleanup **filtered snapshot matched
exactly without identity normalization**: node/port/link IDs and serials,
relevant properties, audio Props/EnumFormat/Format/PortConfig parameters,
defaults, target entries and clock metadata were unchanged. Volatile runtime
node/link state and unrelated metadata were outside this comparison. Relevant
persistent WirePlumber state-file hashes also matched, and the desktop contained
no test-tagged objects. This is the private-core attempt's immediate baseline;
it does not erase the earlier shared-core attempt's fallback identity changes.
Earlier baselines and evidence were preserved.

Playback recorded exit 1 after controlled termination; controller and server
recorded exit 0. Server and playback stderr were empty. Controller stderr
included these diagnostics:

```text
Error: "unsupported type PipeWire:Interface:SecurityContext"
remote 0: error id:30 seq:88 res:-2 (No such file or directory): no global 26
remote 0: error id:0 seq:90 res:-2 (No such file or directory): unknown resource 30 op:7
```

Their precise cause/timing was not separately traced. They did not invalidate
the observed active-link or cleanup checks. This is not a warning-free
validation; no access policy was weakened to suppress diagnostics.

An initial private-harness invocation had failed during baseline decoding with
`AttributeError: 'str' object has no attribute 'decode'`, before creating any
owned processes or audio objects. Passing bytes to the unchanged adapter decoder
corrected that call. The successful routing evidence above belongs to the later,
completed invocation.

### Provenance and limits

- Codec/planner source: `a9eb0d2f3d52c94b0785bf6d09ee2a59d77f61ac`;
  previously verified code/fixture tree SHA256:
  `3aebc658263f0ee945739b1062073b63a042327803991f88322b79e5189f0449`.
- Unchanged adapter SHA256:
  `bfe64336f96ad078149fa31b8471eed886dd6a8959dd3e3aa34f49e9f127b89a`.
- Private final harness SHA256:
  `ffabaaa6ead4e33ccdb6d1c9605929c38b6ac1cae47e8b3e0e90c5985a9f8eda`.
- Private saved result-wrapper SHA256:
  `cc6d86dff0ff78c3d09a56039964f241c075500ae3f1b5cb38d8b612eb469db1`.

This establishes native graph routing of one owned synthetic stereo stream and
its inventory/planner identity mapping on an isolated PipeWire core. Active
links and running nodes do not establish audible output or measured PCM delivery.
Desktop WirePlumber routing policy, existing application moves, cross-session
identity, production lifecycle/rollback behavior, BEACN hardware/USB routing,
Windows interoperability, and the Link discovery/remote-routing service remain
unverified. The harness and detailed journals remain private rather than becoming
a runtime framework in the repository.

## Evidence handling

Filtered baseline/result/comparison journals and the private harness remain
outside Git. No raw captures, browser content, keys, device serials, or machine
endpoint identifiers belong in this document. This documentation update made no
additional audio mutations or test runs.

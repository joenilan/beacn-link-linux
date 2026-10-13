# Native assignment execution and verified publication

Implemented 2026-10-13. This explicitly enabled integration connects received
assignments to the existing native executor on an owned disposable PipeWire core.
It does not enable ordinary desktop apply or route physical BEACN audio.

## Owners and checks

`session::ReceivedNamePolicy::WindowsCString` adapts received names for the
existing planner while preserving raw reply bytes. Assignment zero and values
outside 1..4 remain opaque and never disconnect or reset the current route.

`examples/support/private_routes.rs` uses the existing `NativePipeWire`,
`InventoryTracker`, `planner::plan`, `executor::prepare` and `Execution` owners.
Before application it obtains native observations and requires their complete
bound inventory to equal the supervisor's current inventory. Native execution
uses the existing receipt journal and retained creation-resource proxies; a failed
or uncertain outcome stops the run without retry or continued publication.

`route_publication::VerifiedRoutePublication` is a pure verification result over
caller-trusted observations and receipts, not independent OS freshness or
authorization. Two observations must agree, including generation/core/object
identities and complete inventory. Known assignment publication requires that
existing preparation would make no changes, exact stereo source bindings and
actual receipt-ledger membership for both current links. Reported owner tags or
numeric IDs alone cannot satisfy it. The controlled initial zero publication
requires an actually unlinked live source with exact stereo outputs; this does
not establish zero as a firmware unroute/default command.

The native coordinator supplies fresh native observations/its own receipt ledger
before every publication. It retains the current assignment only after native
execution and route verification succeed. Read/publish are not atomic with graph
observation; this private core has no desktop device discovery/session manager.
Do not expand this observation contract into production routing without addressing
external races, disconnects and persistence separately.

`Publication::with_verified_routes` explicitly selects the new gate. Every cycle,
including a one-cycle invocation's first publication, requires a verified result
with strictly newer inventory and identical bound application/stream identities.
The gate adopts only that result's canonical record/frame. Ordinary inventory-only
refresh, wrong frames, frozen/rebound source, wrong phase and budget exhaustion
stop before another transfer. Default fixed-record modes remain unchanged.

## Bounded lifetime and runner

The combined native example requires Linux and both `native-usb,native-pipewire`.
`--authorize-periodic-private-routing` takes the usual selected device/cache/cycle
arguments, then explicit private runtime, core name, cookie, server PID, token and
four assignment-to-node-ID/serial mappings. The existing native owner enforces
the private runtime/client configuration, peer credentials, cookie, module allowlist,
synthetic endpoint token and no-device rules. No default-core fallback is added.

The runner's `--private-routing --authorize-isolated-routing` requires the explicit
C-string/initialization/observed-marker/startup periodic options. It creates four
owned virtual destinations through one retained controller and keeps the same
owned playback process through all 40 cycles. Authored silence covers 120 seconds;
the private-routing child limit is 90 seconds, while ordinary periodic mode stays
at 50 seconds. Output and inventory IPC limits remain unchanged. It does not stop
at the first assignment. Each next publication verifies the actual route, so it
does not keep advertising unapplied zero over the desired assignment.

`routing-probe` additionally requires isolated-source authorization and performs
no USB operations, even with Studio connected. It drives authored assignments
1 then 2 through the same native coordinator and verifies canonical publication
data. Default source-only behavior remains unchanged. The connected baseline
and compare owners still save/compare full local settings/state. Every attempt
uses a fresh exclusive evidence root and no retry. Unknown child completion
retains private processes and blocks comparison. Cleanup stops only retained owned
processes and closes native creation proxies; the private baseline is compared
after owned source/destinations depart and before stopping the owned server.

## Validation

Windows/Linux pass four new route-verification tests and 20 publication-gate tests.
New cases cover unlinked versus applied state, missing receipts, incomplete stereo
acknowledgments, changed core/generation/destination, stale source, inventory-only
refresh refusal, stale proof, canonical frame enforcement, default refusal and
one-shot proof enforcement. Windows passed 25 Python runner cases with two
Linux-only skips; Linux passed all 27. These include explicit-mode refusal and
private destination identity checks. Default and combined native builds pass;
formatting/diff checks pass. Existing fault/recovery matrices were preserved.

The isolated native probe on the initial integration tree applied assignment 1
with Complete receipt prefix 2, then assignment 2 with Complete prefix 4. Native
observations verified publication data 0 → 1 → 2. It used seven fresh supervised
inventories, preserved the owned source, stopped three owned processes and restored
the private graph. Connected device binding, guest desktop/state and host
audio/registry comparisons matched exactly. No USB transfers or physical audio
routes occurred. Probe journal SHA256:
`e97e277bdda2786e4365a8ec40fe699ce97209e4253ce1e335a655db0b41b6a1`.
Its source is beacn-link-usb-publication-routed-20261013T142454Z/source in the
existing guest home, code SHA256
`ae1fbd4bf02d4e8eff7665ae4bec9827923a5feb08cfc07746fee398af0ba11d`.

Final code additionally enforces proof for one-shot verified mode and clarifies
the pre-application diagnostic line; coordinator/runner behavior is unchanged.
Exact final build and focused-check code SHA256:
`1293d324b0b1316039711268123c719010d34f9c1c9549194e6f2fef9732dc34`.
Source archive SHA256:
`6483942ce8b44c4ebe217b046ee926f20547b64779b7432db01145591ea44ff3`.
Linux validation journal SHA256:
`859d11cf03b8152900ae7e745be4e2f11ccb4998a806376404f2cedbd25847e9`.
Combined native executable SHA256:
`5b03bef53e3630326647134b01b37cd23c0f181ef349af1ba40bf9199509c811`.
Runner SHA256:
`055cbb8a5146b7ae24b18e3fd3db77b6319be9538aa86b2b4cda1d43a432a108`.
Final guest source: beacn-link-usb-publication-routed-20261013T142858Z/source.
Free guest disk: 19,353,255,936 bytes. Existing extracted headers/toolchain were
reused. A failed build missing the explicit existing header sysroot is preserved;
no packages/toolchains were installed. All prior evidence remains outside Git.

## Integrated hardware result

The owner-ready hardware run used the exact final executable/source above. The
owner reported that BEACN Linux VM Test turned yellow and entered the first Link
In channel. At cycle 9, native USB received an assignment-1 record for the exact
owned name before its NUL; the opaque five-byte tail stayed uninterpreted. The
existing native executor completed two acknowledged stereo link creates (receipt
prefix 2), and fresh native observations verified the actual private-core route.

Cycles 10–40 each published verified assignment 1. Additional assignment-1
records at cycles 12 and 15 verified the already-correct route with Complete
prefix 0; no extra mutations were issued. All 40 publication/poll cycles and 81
fresh supervised inventories completed, with three record replies, six NoPayload
replies and 31 rejected-marker observations. No marker meaning is inferred.
Initial A2 completed in 42 ms with the opted-in five-second bound; later deadlines
remained 500 ms. Native/runner exit was zero without timeout or retry.

Owned source identity remained valid, the vendor claim released with device
baseline restored, all three owned private processes stopped and the private
baseline matched. Connected guest binding/desktop/state and host audio/registry
comparisons were exact with no errors. Studio stays in the VM; no UI automation
or physical return comparison occurred. Streaming-PC state is outside these
local comparisons. The owner's UI report establishes drop acceptance during the
run; continuous visual retention beyond that report is not independently measured.

Private hardware journal SHA256:
`277d714102356d39eea8b6be02825b42823049573d7ba3069998017f851c8e70`.
Connected-comparison SHA256:
`b3a65323bf71a29726810720d99a614578885fa6d62a1d6f41b21ccf4d56641f`.
The host copy preserves 181 evidence files/hashes, including private route
snapshots and authored silent PCM. No raw/proprietary/private material enters Git.
This is real USB assignment → native private-core route → verified USB publication
evidence. It is not physical Studio audio or audible interoperability.

A subsequent read-only metadata check found one selected 33AE:4003 audio device,
four stereo playback nodes described as BEACN Studio Link Link 1/2/3/4, four stereo
source nodes and internal eight-channel sink/source nodes. Each stereo sink has
FL/FR input ports and FL/FR monitor outputs. Machine identities/raw metadata stay
private. These labels do not authorize link removal or prove audible pair mapping.
No PCM/USB transfers, capture or routing changes were requested by the check.
Latest free guest disk after preserved evidence: 19,236,716,544 bytes.

Next: verify physical endpoint/pair identities and implement an explicitly selected
physical-audio path with fresh snapshots and owned cleanup. The existing native
owner's private-core/synthetic-node restrictions remain intact; physical endpoints
must not be substituted into it by weakening those checks. Peer assignment changes,
longer lifecycle/reconnects and audible Studio output remain separate acceptance.

The final USB-only build also passed without modifying the pinned combined binary.
Private build-journal SHA256:
`e861e98223eb7d38e7236214bd3209eed26777bfd6f4460e7c4501cefea0087f`.

# Bounded concurrent Studio audio and native control

Verified 2026-10-14 on the separate Debian 13 laptop. Assignment 1 from Windows
BEACN App now drives the native receipt-verified private stereo route and Studio's
first physical pair while native publication and assignment polling continue.
The owner heard repeating tones and reported that the test stayed in Link In
after manually dragging it there. This is bounded first-pair interoperability;
ordinary desktop application routing remains disabled.

## Implementation and authority

`native_usb_periodic --authorize-periodic-studio-continuous` is a separate,
explicit mode requiring private routing, verified-route publication and at least
nine cycles. Assignment 1 must leave at least eight cycles before physical start.
Assignments 2–4 cannot start this first-pair bridge. Existing codec, lifecycle,
planner, graph, execution and native receipt owners remain authoritative.

After actual native FL/FR receipts, the child requests `PHYSICAL START 1`.
The supervisor retains a bounded physical worker, its controller and exact hardware
node/link identities. Only those two active, serial- and PID-verified bridge links
are excluded from its virtual-source inventory check. Native graph observations
still see every link. Each subsequent publication requires a fresh inventory and
native route verification. The final cycle follows `PHYSICAL STOP 1`, complete
owned bridge cleanup, closed PCMs and an exact held-binding/graph comparison.

The default USB gate still requires idle audio. Its sole new exception requires
verified-route mode, a publication/poll phase, audio interfaces 1/2 at alternate 1,
and every other USB observation field unchanged. `src/native_audio.rs` pins the
unique selected ALSA card while idle, character-device inode/device numbers and
the already-authenticated private server's PID/start time. Before each permitted
active-audio exchange it rechecks that process generation, retained character-device
FD, RUNNING playback owner, FLOAT_LE/eight-channel/96-kHz format, closed capture PCM,
and a nonregressing hardware pointer that advances within two seconds. It opens
no ALSA PCM and reads no audio payload. Checks and FD enumeration are bounded.
Initial handshake and released-baseline cleanup retain their strict idle rules.

The supervisor refuses duplicate, wrong-pair, early or unauthorized physical IPC.
Worker failures abort publication. Cancellation stops the physical worker before
giving the native owner a bounded chance to release its USB claim; only retained
owned processes are terminated. Neither owner tags nor numeric IDs authorize
foreign link deletion. No driver, firmware, defaults, volume, desktop routes,
capture, retry or USB handoff changes were introduced.

## Removed metadata bindings during port replacement

The first continuous attempt applied assignment 1, then stopped when native
metadata binds raced adapter-port replacement (`-ESTALE`). All owned resources,
settings, state and binding were restored; no continuous success was claimed.
Failure journal SHA256:
`bb1c9123be229eb1babba453662f9e87fb9813ae2f6e8835f56074494a81be94`.

PipeWire's [registry-generation contract](https://pipewire.pages.freedesktop.org/pipewire/page_native_protocol.html)
and [1.4.2 server implementation](https://github.com/PipeWire/pipewire/blob/1.4.2/src/pipewire/impl-core.c)
identify this error as a removed global generation during lookup. The C adapter
now defers only an unfinished read-only query proxy's ESTALE, and requires an
actual `global_remove` for that same observation before the sync barrier ends.
If the object remains present or receives initialized info, the barrier fails.
Creation proxies, initialized objects and other errors retain their failure path.
The existing receipt and endpoint freshness checks are unchanged.

The opt-in synthetic `private_core.py --port-churn` test creates retained virtual
adapters and replaces their eight/two/eight ports while the native observer is
connected. Six rounds exercised 78 removed bindings while preserving the same
active stereo route, then passed A-to-B, partial-failure recovery and exact private
graph/desktop/state cleanup. No USB exchanges or physical output occurred.
Journal SHA256:
`e049da5b146f09652e2d1b187eb511204f8fecda9598cab62899fad5aba94fad`.

## Physical results

Both runs used exact live source `beacn-native-source-20261014T214415Z`, 40 cycles,
96-kHz first-pair output, 82 fresh supervised inventories, one assignment record,
seven NoPayload and 32 rejected-marker replies. Marker semantics remain unknown.

| Evidence | Continuous silence | Continuous quiet tones |
| --- | --- | --- |
| First assignment-1 cycle | 5 | 9 |
| Subsequent verified publications | 35 | 31 |
| Native active-PCM checks | 136 | 120 |
| Hardware-pointer progress during native checks | 1,744,716 frames | 1,522,199 frames |
| Independent physical clock observations | 77 | 70 |
| Independent physical progress | 1,727,437 frames | 1,508,760 frames |
| Owner observation | Stayed in Link In | Dragged into Link In; heard tones; stayed there |

Each run exited 0 without timeout, preserved its source, stopped all four owned
processes, restored the private graph and USB ACL exactly, and matched the complete
laptop settings/routes/defaults/volume/mute/state-file and connected USB baselines.
Playback/capture PCMs closed. Streaming-PC saved settings are outside those local
comparisons. Visibility is the owner's report during the bounded run, not an
independent UI capture. No payload or audio was captured.

Silent hardware journal SHA256:
`173bc2fd920fc9b2c9dc8cd6927563e818f6a35768931385626f5169cd9bf48b`.
Silent physical-stage journal:
`5d9126f5029dd9ff5672bd0e9e5489b7509dc083ec29876b4702731028eb7087`.
Audible hardware journal:
`18b55004b67066ff249026ba0346f66b08b479b4cae576d8d108d46db2cdeb2c`.
Audible physical-stage journal:
`e4650c18a383e3059c8746717519610139c17a27a4fdec05aba155190be29e1e`.
Owner-report file:
`8d533b36eae137dc4e7f6cd68dee176917e24d394878b51411cbcad8fc730b5e`.
614 private evidence/provenance/report files were copied and hashed outside Git.

Native environment: Debian 13.7, kernel 6.12.111+deb13-amd64, PipeWire 1.4.2,
WirePlumber 0.5.8, libusb 1.0.28, Python 3.13, isolated Rust 1.88. The last owner
BEACN App screenshot showed v1.4.0; live peer version was not requeried.
The exact native programs build offline; 64 focused Rust/Python checks pass,
including ownership/format/clock refusals, USB phase/default policy, publication,
continuous IPC ordering/cancellation and exact bridge-reference validation.

The final harness additionally records applied-route evidence before checking
aggregate continuous proof, so a failed proof cannot misreport an earlier actual
route as unapplied. This diagnostic-order change and formatting do not change
packets, route mutations or the successful live path.

## Next acceptance

Integrate ordinary Linux application inventory and controlled routing, then verify
the other three physical pairs. Longer daemon operation, app start/exit, assignment
changes, reconnect/recovery and installed product behavior remain unverified.
The VMware PCM-stall/recovery boundary remains separate; native laptop success
does not establish VM recovery. Keep Studio attached to the laptop, Windows Link
closed, GitHub Actions disabled and ordinary apply disabled. No REA was run.

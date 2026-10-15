# WirePlumber LinX target-cache guard

The current native Studio backend requires this per-user integration before it
opens a desktop route. It prevents WirePlumber from replacing an
application's previously saved stream target when LinX temporarily directs that
stream to its owned desktop sink. It leaves stream volume/property persistence,
target restoration, non-LinX target saves, and existing saved target values under
the stock `node/state-stream.lua` policy.

## Supported source and guard

The checked-in replacement retains the WirePlumber 0.5.8
`node/state-stream.lua` source extracted from the installed Debian package used
for the original verification. The unmodified stock file is not shipped in this
repository. Its SHA-256 is:

```
97761d3098d0bf5aa825e1cd3d096a859d4c1d5fbe5fa7a56fbf794116e95e97
```

This integration supports WirePlumber 0.5.8 only. Before installing, compare the
installed stock `node/state-stream.lua` bytes with this pinned digest. Refuse the
install if the file is absent or differs; review and rebase the narrow patch
against that exact release before supporting another version. The repository's
offline test reconstructs the source by reversing only the three checked-in
script additions and verifies the resulting bytes against this digest.

The retained upstream script keeps its MIT notice, with the full MIT text in
`integrations/wireplumber/LICENSE`. LinX additions to that script are
GPL-3.0-only under the repository's top-level `LICENSE`.

The fragment matches only the stock `node/state-stream.lua` component and
overrides its script path. It retains the stock `hooks.stream.state` feature,
which is wanted by `policy.node`; it does not disable the feature or register a
second handler with the same hook name.
The replacement path in the SPA-JSON fragment is deliberately unquoted:
WirePlumber 0.5.8's component override loader otherwise retains literal quote
characters when resolving this property.

The derived handler first resolves `target.object` by the metadata's exact
PipeWire object serial. It skips saving only when the resolved node is an
`Audio/Sink`, has `linx.temporary-target = "1"`, and its `beacn.link.owner`
exactly matches its `node.name` in the `linx-desktop-<pid>-<32-hex-nonce>` form.
All other metadata targets follow the stock handler. The skip returns before
loading or writing the stream's saved values, so it preserves rather than clears
the existing target entry.

When the replacement script loads, it activates a retained ObjectManager for the
`default` metadata object. Its `object-added` handler publishes
`linx.target-cache-guard = "1"` as `Spa:String` on subject `0`. WirePlumber 0.5.8
emits `object-added` for existing matches during activation as well as future
matches, so startup order does not depend on catching a metadata-added event.
LinX must require this marker before opening a desktop route and refuse the
route if it is absent or changes. The marker only proves that the replacement
policy component announced readiness; it does not grant authority to remove
graph objects.

The native app must set both `beacn.link.owner` and `node.name` to the same
per-route owner token and set `linx.temporary-target = "1"` only on its
temporary sink. The 32-hex nonce makes that token distinct from older
PID-only LinX sink names. The marker properties are policy filters, not
cryptographic credentials or cleanup authority.

## Per-user installation

Install these two files into the active user's WirePlumber paths, preserving
the relative paths shown here:

```
$XDG_DATA_HOME/wireplumber/scripts/linx/node-state-stream.lua
$XDG_CONFIG_HOME/wireplumber/wireplumber.conf.d/90-linx-state-stream.conf
```

When the XDG variables are unset, those locations are normally
`~/.local/share/wireplumber/scripts/linx/node-state-stream.lua` and
`~/.config/wireplumber/wireplumber.conf.d/90-linx-state-stream.conf`. Copy the
repository's `integrations/wireplumber/scripts/linx/` directory and the
configuration fragment into those destinations. Do not copy or overwrite the
user's main `wireplumber.conf`.

Before copying, check for existing destination files and preserve them rather
than overwriting another installation. This is an explicit desktop policy
integration, not an automatic side effect of starting LinX.

The configuration is read at daemon startup. After the owner has confirmed a
safe point with LinX paused and no owned desktop route active, restart the
user's WirePlumber service using the system's normal service manager. Do not
restart PipeWire as part of this integration. Verify the marker is present and
stable before enabling desktop routing. Installation and service restart are
not performed by this repository test.

For example, inspect `pw-metadata -n default` for subject `0`, key
`linx.target-cache-guard`, type `Spa:String`, value `1`. LinX checks that same
marker through fresh native graph barriers before opening USB and during route
checks. Missing readiness refuses the route; lost readiness stops forward work
while retaining the normal owned cleanup path.

## Rollback

Pause LinX and confirm no owned desktop route is active. Remove only the two
files listed above, then restart the user's WirePlumber service so the stock
`node/state-stream.lua` component is selected again. Do not edit or delete the
WirePlumber `stream-properties` state file: this integration does not modify or
clear existing entries. Confirm the default metadata object no longer advertises
`linx.target-cache-guard` before allowing LinX routing again.
Without the integration, the current native Studio backend will refuse a new
desktop route. Existing saved targets from older experimental builds require
separate evidence-backed review; installing this guard does not repair them.

## Offline verification

Run from the repository root:

```
python tests_native/wireplumber_guard.py
```

The test checks the pinned stock-source digest, evaluates the standalone Lua
guard predicate through the system Lua 5.4 shared library when available, and
checks that the handler and component override remain scoped. It does not start
WirePlumber, connect to PipeWire, install files, or create routes.

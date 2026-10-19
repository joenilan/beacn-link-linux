# Vendored crates patched for LinX

Applied through `[patch.crates-io]` in `app-gpui/Cargo.toml`. Each directory is
the published crate source with the changes below; licenses are unchanged.

- `ksni` 0.3.6 (Unlicense): implements KDE's
  `org.kde.StatusNotifierItem.ProvideXdgActivationToken` and passes the token
  to the new `Tray::provide_xdg_activation_token` callback.
- `gpui-fast-linux` 0.1.8 (Apache-2.0, Longbridge; modified): adds
  `gpui_linux::set_pending_activation_token`. On Wayland the next new toplevel
  window, or the next `activate` of an existing one, uses that xdg-activation
  token so the compositor raises and focuses the window. Also adds
  `gpui_linux::request_launch_activation_token`: the next `activate` asks the
  compositor for a token from the latest click instead, for a process LinX
  starts (the updater's *Restart LinX*), so the new window opens in front.

Both let a tray click bring LinX's window to the front on Wayland (KWin's focus
stealing prevention otherwise only marks it as wanting attention).

- `gpui-component` 0.7.3 (Apache-2.0, Longbridge; modified): the Linux
  client-side window frame (`src/window_border.rs`) rounds all four corners at
  10px instead of drawing a square frame, border and shadow. LinX opens its
  window with a transparent root surface and paints its own rounded console,
  so the desktop shows through the corners as on Windows.

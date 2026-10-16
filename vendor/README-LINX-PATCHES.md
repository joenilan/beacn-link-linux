# Vendored crates patched for LinX

Applied through `[patch.crates-io]` in `app-gpui/Cargo.toml`. Each directory is
the published crate source with the changes below; licenses are unchanged.

- `ksni` 0.3.6 (Unlicense): implements KDE's
  `org.kde.StatusNotifierItem.ProvideXdgActivationToken` and passes the token
  to the new `Tray::provide_xdg_activation_token` callback.
- `gpui-fast-linux` 0.1.8 (Apache-2.0, Longbridge; modified): adds
  `gpui_linux::set_pending_activation_token`. On Wayland the next new toplevel
  window, or the next `activate` of an existing one, uses that xdg-activation
  token so the compositor raises and focuses the window.

Both let a tray click bring LinX's window to the front on Wayland (KWin's focus
stealing prevention otherwise only marks it as wanting attention).

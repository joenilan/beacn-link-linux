# Synthetic fixtures

These text-only hex examples were constructed by hand, not captured from USB.

- ascii.frame.hex: data frame; game assigned 1, voice assigned 0 (opaque).
- opaque.frame.hex: assignment 255, raw name bytes C3 A9, then terminator.
  Decode preserves these bytes; conservative ASCII encoding rejects them.
- a0.header.hex: the observed A0 DWORD in little-endian order. No handshake
  semantics or full-response format is asserted.

Fixture names are fictional. See docs/offline-codec.md for use and limits.

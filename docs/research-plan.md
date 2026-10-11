# Phased interoperability plan

Goal: keep BEACN App/Mix Create on Windows and replace Link on the Linux gaming PC.
Existing beacn-on-linux work reportedly exposes four stereo Link pairs; that
prior research is context, not a locally verified replacement for discovery.

1. Static tracing: follow the GUID/VID/PID anchor, WinUSB calls and audio-session
   discovery RTTI. Record references, framing candidates and uncertainties.
   Exit: defensible candidate messages/transport and explicit unknowns.
2. Separately approved controlled captures: agree scope, devices, routing actions,
   consent and local artifact handling first. No capture is authorized now.
   Exit: sanitized protocol specification corroborated by repeatable evidence.
3. Minimal Linux interoperability: candidate PipeWire application metadata and
   audio routing must be tested, not assumed. Exit: one real Linux app appears
   in Windows App and can be routed from Windows, with expected audible output.
4. Robustness: test multiple apps/pairs, naming, app lifecycle, reconnects,
   restarts, rate changes and errors. Preserve normal streaming-PC behavior.

A generic USB/audio Linux port alone does not prove remote app visibility.
Packet fields, endpoint numbers, handshakes and routing semantics are unknown.
No implementation or Linux runtime has been validated. Keep raw evidence private.

# Research boundaries

Work only in this repository. Preserve evidence provenance: app version,
SHA256, file offsets versus preferred virtual addresses, and tool versions.
Separate observations, derivations, hypotheses and unknowns.

Do not commit proprietary binaries, raw captures, secrets, device serials or
machine-specific endpoint GUIDs. Protocol interface GUIDs, VID/PID and hashes
may be recorded. Keep private artifacts outside Git.

Do not execute BEACN binaries, change routing, attach USB to WSL, start WSL,
capture traffic/audio, modify drivers/firmware, or disturb other agents.
Controlled captures require separate approval. Do not install or run REA
without authorization.

Keep GitHub Actions disabled. Add no workflow files. Use [skip ci] commits.
Do not describe placeholders as working. Protocol decoding and Linux runtime
remain unverified until reproducible evidence establishes them.

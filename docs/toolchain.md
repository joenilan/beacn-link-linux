# REA prerequisites

REA clone: https://github.com/joenilan/rea, main,
b2da21abe11ac43a4cc24b233bd52ce10995e4a1. AGENTS.md and linked README guides read.

Observed: Node 24.14.1 and npm 11.11.0. REA 6.4.0 accepts Node 24 >=24.11,
but the source pins Node 24.18.0 and npm 11.16.0. Clone dependencies are absent.
No REA setup, build, dependency install or REA command was executed.

Windows Ghidra P0 requires Windows 10+ x64, fixed local NTFS target/temp volumes,
Ghidra 12.1.x with native decompiler, and a full 64-bit JDK in its Java range
(currently 21+). PATH Java is 8; Android Studio provides JDK 21.0.10.
Ghidra/IDA/Hopper were not located in checked commands and app records;
this was not an exhaustive disk search. C: and E: are fixed NTFS volumes; temp-volume suitability remains unverified.

Available static tools: GNU objdump/strings/nm/c++filt, Python 3.12,
pefile 2024.8.26, capstone 5.0.7. Native REA readiness is not established.

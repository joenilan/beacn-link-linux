# Linux offline validation

On 2026-10-11, source commit `cd33276a29a0b7f09655f4a57e9d7f473f28d327`
built and ran successfully in an Ubuntu VM. All 15 integration tests passed.
This verifies the offline codec, CLI and pure planner on Linux; it does not
verify a BEACN Link replacement, live provider integration or hardware behavior.

## Environment and source provenance

- Ubuntu 24.04.5 LTS (Noble Numbat), x86_64; kernel 7.0.0-38-generic.
- rustc 1.88.0 (6b00bc388 2025-06-23), LLVM 20.1.5.
- Cargo 1.88.0 (873a06493 2025-05-10).
- GCC 13.3.0 (Ubuntu 13.3.0-6ubuntu2~24.04.1).
- Committed source only: `git archive --format=tar.gz --prefix=beacn-link-linux/`
  of the exact commit above, transferred and hash-checked before extraction.
  The 21,388-byte archive contained 22 committed files and no `.git` directory.
- Archive SHA256: `7b2999676985b4c81d1f00dc7b4177f80b6ebf795e697d7e3937aa73848ca654`.
- No external crates; build and test commands used `--offline --locked -j 1`.

The minimal toolchain was installed without sudo in isolated Cargo/Rustup
directories, with no profile or PATH edits. The installer URL was checked
against the [official Rust installation guide](https://rust-lang.org/learn/get-started/).
Bootstrap script SHA256 for this run:
`7d0ea0f8eba7fa1ebfe998091cd7ec4501e33ec5ca6b884eb4d894d7da5170af`.
This hash records the downloaded script; it is not an independent signature.

## Commands and exit codes

`validation_root` below represents the isolated staging directory; private
machine paths and connection details are deliberately omitted. The source is
extracted under `$validation_root/beacn-link-linux`.

```sh
export CARGO_HOME="$validation_root/.cargo"
export RUSTUP_HOME="$validation_root/.rustup"
curl --proto '=https' --tlsv1.2 --fail --silent --show-error --max-time 60 \
  https://sh.rustup.rs -o "$validation_root/rustup-init.sh"
sha256sum "$validation_root/rustup-init.sh"
sh "$validation_root/rustup-init.sh" -y --profile minimal \
  --default-toolchain 1.88.0 --no-modify-path
cd "$validation_root/beacn-link-linux"
"$CARGO_HOME/bin/rustc" +1.88.0 --version --verbose
"$CARGO_HOME/bin/cargo" +1.88.0 --version
"$CARGO_HOME/bin/cargo" +1.88.0 build --offline --locked -j 1
"$CARGO_HOME/bin/cargo" +1.88.0 test --offline --locked -j 1
./target/debug/link-protocol decode 0e0000ac040167616d650500766f69636500
./target/debug/link-protocol encode 1:game 0:voice
./target/debug/link-protocol decode-header 000000a0
"$CARGO_HOME/bin/cargo" +1.88.0 run --offline --locked --example offline_plan -j 1
```

Every command above exited 0. The overall remote script and SSH process also
exited 0. Setup downloaded the official toolchain; compilation, tests and
synthetic examples subsequently ran offline with one Cargo job.

| Check | Observed result | Exit |
| --- | --- | --- |
| Build | Successful native Linux build | 0 |
| Planner integration tests | 7 passed, 0 failed | 0 |
| Codec/CLI integration tests | 8 passed, 0 failed | 0 |
| Unit and doc-test targets | 0 tests, no failures | 0 |
| Synthetic decode, encode and control-header decode | Expected output below | 0 each |
| Synthetic planner example | Expected proposal and opaque assignment below | 0 |

## Synthetic output

The decode command printed:

```text
data: 14 payload bytes, 2 records
assignment=1 (Link 1) name="game" name_hex=67616d65
assignment=0 (opaque/unknown) name="voice" name_hex=766f696365
```

Encode reproduced `0e0000ac040167616d650500766f69636500` exactly.
Header decode printed `Control(A0) (static subset; control semantics unknown)`.

The planner example reported snapshot 7. Its `game` record proposed Link One
for application `synthetic-game`, with sorted streams `stream-a` and `stream-b`.
The `voice` record reported `OpaqueAssignment { value: 0 }`. These were pure
proposals over handwritten input and synthetic inventory; no routes were applied.

## Resource use and limits

Available filesystem space was 26,859,900,928 bytes before setup,
26,261,012,480 bytes before build and 26,183,262,208 bytes afterward.
The 25 GB free-space threshold was satisfied before setup and compilation.

Real USB messages, handshake behavior, BEACN application visibility, PipeWire
provider metadata, live audio routing and interoperability remain untested.
No BEACN binaries or USB/audio operations were executed, and no VM restart was
needed. Private host/user/key/IP details and raw logs remain outside Git.

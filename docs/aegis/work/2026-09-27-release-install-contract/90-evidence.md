# Release and installer repair evidence

## Source and regression checks

- Final code commit: `b010a2631ded68cfeb85cca02972c7a7f55f4e43`, pushed to `main` and `refactor/kiss-solid-yagni-baseline`.
- Local verification passed: `cargo fmt --all --check`; `cargo test --workspace --all-targets --all-features --locked` (518 library, 6 CLI, 1 daemon); strict all-target/all-feature Clippy; all-target/all-feature `cargo check`; both binary `--help` smoke checks; shell syntax; `bash tests/installer.sh`; `bash tests/release_test.sh`; and `git diff --check`.
- CI run `36293320983` passed canonical quality, installer/package invariants, cross-distro artifact execution, Windows check/Clippy/tests, Windows release binary build, and Named Pipe IPC smoke.
- Windows failures from run `36292857773` were closed by target-scoping Linux DDC process tests, asserting the correct Windows IPC endpoint, and using the shared cross-platform timezone resolver. The explicit `Europe/Istanbul` weather timestamp regression and the embedded-tzdb fallback test passed on Windows CI.

## Release qualification

- Non-publishing workflow-dispatch run `36293573564` qualified all four targets: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`, and `aarch64-unknown-linux-musl`.
- Each target checked tag/Cargo version agreement (`0.12.1`), archive checksum, ABI metadata, exact archive members (`LICENSE`, `README.md`, both binaries, and `sunreactord.service`), binary versions, and `--help` execution on its target runner.
- Downloaded qualification artifacts independently. All four per-target SHA256 manifests verified. GNU ABI metadata reported baseline `2.35`; musl metadata reported `static`. Both musl binaries per target had no ELF interpreter and no `NEEDED` entries. ELF machine types matched x86_64/aarch64.

## Published release

- Annotated tag `v0.12.1` points to code commit `b010a2631ded68cfeb85cca02972c7a7f55f4e43`.
- Tag CI run `36293807566` passed. Tag-triggered release run `36293807564` passed every target build and the publish job, including the build provenance attestation.
- Public release `v0.12.1` is published, not draft, and not prerelease. It contains exactly four target archives, `SHA256SUMS`, and `ABI-METADATA`.
- Downloaded public assets independently. The combined manifest verified all four archives and ABI metadata; every public archive SHA256 exactly matched its qualification artifact. All archive member sets and ELF architectures matched. x86_64 GNU/musl binaries reported `0.12.1` and passed help smoke tests; both musl variants were static. ARM64 binaries passed target-runner smoke tests in qualification and tag release.
- Ran the checked-in `install.sh` against the public `latest` release with `DESTDIR` and isolated XDG paths under a fresh `/tmp` directory. It selected `0.12.1`, verified the public archive and ABI checksums, staged both x86_64 GNU binaries and `sunreactord.service`, then the staged binaries reported `0.12.1` and passed `--help`.
- Existing `v0.12.0` tag/release was not modified.

## Coverage boundary

- The release producer, published artifacts, an end-to-end staged installer run, installer/packaging tests, Linux distro execution, and Windows CI path were verified.
- The staged install did not start the service on the user's specific host or exercise display hardware. Host-specific service-manager/configuration/hardware behavior remains outside this release evidence.
- The local untracked `releases/` directory was preserved and excluded from Git staging.

## Workspace tooling

- No `aegis-workspace.py` helper was available in `PATH` or under the configured `.agents`/`.codex` skill directories, so the work record was updated manually. No helper-generated bundle/check result is claimed.

# Release and installer repair checkpoint

## Current todo

Get green default-branch CI after the final Windows Clippy boundary correction, then qualify and publish `v0.12.1`.

## Completed

- Captured the resumed checkout state: branch `refactor/kiss-solid-yagni-baseline`, HEAD `7e027ca60f3a5405d1b387f52080289894993c76`, a no-commit merge with `origin/main` at `ee78c046a9d226c39edbabec5753fe1903574084`, existing staged merge changes, and preserved untracked `releases/`.
- Corrected an earlier checkpoint claim: the downloaded public `v0.12.0` executable hashes do **not** match the locally downloaded `v0.1.0` archive. The public binaries themselves report `0.1.0`; the exact actor that attached them is unknown.
- Confirmed the published `v0.12.0` archive omits `sunreactord.service`, and its workflow failed the x86_64 musl smoke step so the publish job was skipped.
- Unified version, archive, checksum, ABI, service-template, stale-binary, and installed-binary checks across `scripts/release.sh`, `.github/workflows/release.yml`, and `install.sh`.
- Added installer transaction rollback, atomic file replacement, restart of an already active service, and a post-start IPC readiness check. Failed rollback keeps its recovery files and prints their path.
- Routed production monitor discovery through the block-aware DDC parser; a malformed-record regression fixture failed before that change and passes after it.
- Retained active TUI refresh-rate persistence; the current form, runtime, example config, and README still expose/use it. Replaced the conflicting merge test with a serialization round-trip regression.
- Replaced the inactive legacy `tests/installer.sh` suite—which used per-asset `.sha256` files and attempted a network release—with a compatibility entry point to `tests/installer_test.sh`.
- Focused TUI config and capability-refresh regressions both pass.
- `bash -n install.sh tests/installer_test.sh tests/installer_test_support.sh tests/installer.sh` passes.
- `bash tests/installer_test.sh` passes, including successful start/readiness, active-service restart, setup-failure rollback, readiness-failure rollback, and retained recovery files after rollback failure.
- `bash tests/release_test.sh` passes, including stale-version, dynamic-musl, unsafe-tag, version-mismatch, and checksum-negative cases.
- `cargo fmt --all --check` passes.
- `cargo test --workspace --all-targets --all-features --locked`: 517 library, 6 CLI, and 1 daemon tests passed.
- Strict workspace Clippy and `cargo check --workspace --all-targets --all-features --locked` pass; both binary `--help` smoke commands pass.
- In an Ubuntu 22.04 container using Rust 1.97.1, the GNU release build passed the GLIBC 2.35 gate (daemon 2.34, CLI 2.35), packaged as `sunreactor-0.12.1-linux-x86_64-gnu.tar.gz`, and passed version, help, member-list, and checksum smoke checks.
- `git diff --check`, cached diff check, and release workflow YAML parsing pass. ShellCheck is not installed in this environment; CI does not invoke it.
- Fresh final rerun after the rollback-log formatting correction: `cargo test --workspace --all-targets --all-features --locked` passed (517 library + 6 CLI + 1 daemon); strict workspace Clippy, workspace `cargo check`, `cargo fmt --all --check`, shell syntax, `bash tests/installer.sh`, and `bash tests/release_test.sh` all passed.
- Refetched `origin`; `origin/main` still equals `MERGE_HEAD` (`ee78c046a9d226c39edbabec5753fe1903574084`). Remote `v0.12.1` tag does not exist. GitHub CLI is authenticated with `repo` and `workflow` scopes.
- Reviewed the complete staged path list: 48 intended paths, no `releases/` entry, no unstaged tracked diff, no unresolved index entries, and all cached whitespace checks pass.
- Confirmed `workflow_dispatch` builds and uploads qualification artifacts only; the publish job is limited to a `v*` tag push and requires exactly four archives.
- Default-branch CI run `36291621537` on `727092a` exposed two integration failures. Installer/packaging and cross-distro compatibility passed; Windows `cargo check` and Linux tests failed.
- Root causes: the process tests invoked `sh` but used Bash-only `$BASHPID`; the local host's `/bin/sh` resolves to Bash, while Ubuntu CI uses a POSIX shell. The merge also Linux-gated `ddcutil`, Linux-gated shared path imports, kept a path-shaped TUI test override against Windows' named-pipe `ControlSocket`, and compiled a Linux sysfs symlink test on Windows.
- Repaired those owners: restored cross-platform `ddcutil` visibility and `Path` imports, made the TUI worker override carry `ControlSocket` with a unique Windows test pipe, gated the sysfs symlink test to Linux, switched child PID capture to POSIX `$!`, and gated the Linux-only `Instant` test import.
- Fresh local post-repair checks pass: 517 library + 6 CLI + 1 daemon tests; strict Clippy; workspace `cargo check`; format and shell syntax; installer and release regression suites.
- Local Windows MSVC cross-check was attempted but this Linux host lacks `lib.exe`, required by `ring`; the pushed Windows CI job remains the authoritative platform gate.
- Follow-up CI run `36292139369` passed Linux format/Clippy/tests, installer/packaging, cross-distro compatibility, and Windows `cargo check`; Windows Clippy then exposed target-specific dead-code/import and style lints.
- Scoped Linux-only readback/runtime/test helpers to Linux or Unix test builds, kept the common probe `Result` contract, and fixed the Windows-only Clippy findings without adding broad lint suppressions.
- Fresh local rerun after the Clippy-boundary repair passes: workspace tests, strict Clippy, workspace `cargo check`, format/shell syntax, installer tests, and release tests.
- CI run `36292580103` reproduced exactly four Windows Clippy findings: three Linux-only test imports in `src/apply/engine.rs` and one Linux-only helper in `src/backends/ddc.rs`; all other CI jobs passed, including Linux tests/Clippy, installer/packaging, Windows `cargo check`, and the cross-distro artifact matrix.
- Applied the narrow platform boundary correction: gate the `ProcessRunner`/atomic/`Arc` test imports and the verified DDC bus helper to Linux. This leaves Linux runtime behavior unchanged and removes Windows-only unused/dead-code findings.
- Fresh verification on the corrected worktree passes: `cargo fmt --all --check`; 517 library, 6 CLI, and 1 daemon tests; strict all-target/all-feature Clippy; all-target/all-feature `cargo check`; both binary `--help` smoke checks; shell syntax; `bash tests/installer.sh`; `bash tests/release_test.sh`; and `git diff --check`.
- Preserved the pre-existing untracked `releases/` directory unchanged; it is excluded from the commit and release upload.

## Active slice

Commit the locally verified CI lint correction and this checkpoint, push to the task branch and `main`, then require fresh green CI before qualifying all four release targets and publishing `v0.12.1`.

## Patch-shape and diagnosis

- PatchShape: release producer/consumer contract mismatch, installer rollback gap, and cross-platform compile/test boundary mismatches.
- Canonical owner: `scripts/release.sh` produces the shared archive contract; `install.sh` validates, installs, and rolls back; platform adapters own OS-specific IPC; process tests use the shell contract they invoke; the release workflow gates qualification and publication.
- Upward drill: tag/source versions, archive members, ABI metadata, installer expectations, the failed musl job, and prior/current test seams.
- Causal status: stale binary version and missing service/asset-contract mismatches are confirmed; the exact public upload actor remains unknown.
- TDD posture: resumed task baseline says TDD mode is off; focused regressions are used without requiring a strict RED/GREEN route.

## Baseline and drift

- Compatibility boundary: retain the package version line, four-target matrix, XDG paths, safe uninstall behavior, current TUI settings, and compatible `main` installation safeguards.
- Retirement boundary: use one archive/checksum contract; the old test-only per-asset checksum suite is retired behind a wrapper to the canonical installer suite. No fallback for stale release binaries is planned.
- Non-goals held: no automatic monitor configuration and no replacement/deletion of the existing `v0.12.0` release.
- Drift decision: continue; changes remain bounded to the release/install contract, installer failure recovery, and regressions exposed by the merged baseline.

## Anti-entropy declaration

- Deletion class: internal test-code retirement.
- Old path: `tests/installer.sh` contained an inactive per-asset checksum and legacy installer-flow suite that attempted network access when run against the current installer.
- New canonical owner: `tests/installer_test.sh` owns install, service, rollback, and readiness coverage; `tests/release_test.sh` owns artifact-contract coverage. `tests/installer.sh` remains a compatibility entry point.
- Preserved behavior: archive/version/ABI checks, XDG and service-path behavior, safe uninstall, rollback, active-daemon restart, and IPC readiness remain covered by the active suites.
- Retired behavior: the obsolete checksum contract and the former test-only automatic discovery/doctor-flow assertions.
- External boundary touched: no. Source-of-truth data risk: none. User confirmation required: no; the user explicitly requested necessary refactoring.
- Lingering-reference check: CI runs `tests/installer_test.sh`; no CI path invokes the old suite directly.

## Evidence still required

- Commit/push of the cross-platform Clippy correction, fresh green default-branch CI, workflow-dispatch qualification for all four release targets, tag-triggered publish, and public asset verification.
- Publish `v0.12.1`, then verify its tag, four archives, combined checksum manifest, ABI metadata, archive members, executable versions, and static musl ELF properties.

## Next step

Fetch and confirm remote heads, review/stage only the two target-specific source files and this checkpoint, commit, then push the correction to the task branch and `main`.

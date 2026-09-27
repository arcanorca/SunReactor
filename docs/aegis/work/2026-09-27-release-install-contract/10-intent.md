# Release and installer repair intent

## Requested outcome

Publish a corrected SunReactor release that installs the source version named by its tag on the supported Linux targets, without requiring users to build from source.

## Scope

- Align `scripts/release.sh`, `.github/workflows/release.yml`, and `install.sh` on version naming, archive members, checksums, ABI metadata, and platform selection.
- Reject stale or incompatible binaries before they can become release assets.
- Fix the musl release build and make the workflow create a missing GitHub Release.
- Preserve reviewed `main` changes while integrating the current 0.12 code line.
- Update version and user-facing release/install documentation, then verify and publish `v0.12.1`.

## Non-goals and preservation boundaries

- No unrelated runtime, UI, or hardware behavior changes.
- Do not replace or delete the existing `v0.12.0` release or its tag.
- Preserve the untracked local `releases/` directory and existing worktrees.
- Do not claim that the installer verifies GitHub attestations unless it actually does.

## Baseline read set

- Current source branch: `refactor/kiss-solid-yagni-baseline` at `7e027ca60f3a5405d1b387f52080289894993c76`.
- Public `main`: `origin/main` at `ee78c046a9d226c39edbabec5753fe1903574084`.
- Shared base: `5bfc08d3f403b5c9b1b0ca60fb8eb6654fc2c55f`.
- Published `v0.12.0` tag resolves to source commit `77e29f8bf337859f510aacae459948bae0540efe`.
- Read current and `main` installers, release workflow/helper, test suites, service template, README, changelog, and distribution compatibility document.
- Inspected the exact published archive and `v0.12.0` workflow run `36137178750`.

## Baseline usage

- Required refs acknowledged: source branch, public `main`, release tag/assets, CI workflow result.
- Required refs cited in plan: yes.
- Missing external evidence: the manual command or actor that attached the stale `v0.1.0` binaries to `v0.12.0` is not available from the repository or Actions run.

## Impact statement

The installer consumes binaries, service configuration, checksums, and ABI metadata produced by a separate release pipeline. A mismatch can install stale code or stop installation before service setup. A failed target build must prevent publication, and artifact identity must be checked against the Cargo/tag version before upload.

## Success evidence

- Every packaged executable reports the requested Cargo version; stale fixtures fail packaging.
- Archives contain the binaries, license, README, and systemd unit expected by the installer.
- GNU ABI checks and static musl checks match the selected target; both x86_64 and aarch64 assets build and smoke-test in CI.
- Installer tests cover tag normalization, checksum/ABI validation, archive members, and staged service installation.
- Workspace tests and relevant checks pass after integration with `main`.
- GitHub Actions publishes `v0.12.1`; downloaded assets match their manifest and pass platform-appropriate smoke checks.

## Stop condition

Finish when the corrected tag is on the public release path, its CI publish workflow succeeds, and downloaded assets satisfy the installer contract. If remote protection or CI blocks publication, keep the verified local changes and report the exact external blocker without calling the release complete.

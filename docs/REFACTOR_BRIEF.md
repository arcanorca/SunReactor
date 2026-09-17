# Refactor brief

Short guidance for a full, behaviour-preserving refactor of SunReactor.
Surveyed on 2026-09-17 from `/home/arcanorca/Projects/SunReactor` (`main`,
HEAD `942afaf`).

## 1. Where to work

- **Use only `/home/arcanorca/Projects/SunReactor`.** It is the only checkout
  with the current work (TUI redesign, weather details and air quality, EDID bus
  addressing, wake watch, policy memoisation, new themes).
- **Ignore every other copy.** They are older commits or snapshots and must not
  be edited or used as a reference:
  - `.worktrees/*` (9 git worktrees on old `wt/*` branches, some dirty),
    `.hermes/`, `.backups/` inside the repo.
  - `~/.cache/sunreactor-*` (detached worktrees and qualification copies),
    `~/Backups/SunReactor`, `~/sunreactor-ph2.6`, `~/Desktop/SunReactor-*`,
    `/tmp/sunreactor-r1-red` (prunable).
- `.worktrees/`, `.hermes/` and `.backups/` are not in `.gitignore`. Add them,
  or tell tools to exclude them, so searches do not return duplicate, outdated
  code.

## 2. Before touching code

1. **Nothing from recent work is committed.** 122 paths differ from HEAD
   (+12.9k / −7.8k lines, 34 untracked). Create a branch and commit this as the
   refactor baseline first.
2. **The index is inconsistent.** Some files are staged as deleted but exist
   again as untracked files: `rust-toolchain.toml` and `docs/phase{8,9,10}-*.md`.
   Decide deliberately (keep `rust-toolchain.toml`; the phase records are
   historical) instead of `git add -A`.
3. **Record the baseline** so every step can be compared:
   - `cargo fmt --all -- --check`
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::dbg_macro -D clippy::todo`
   - `cargo test --workspace --all-targets --all-features` → 488 + 6 + 1 passing
   - `cargo test --workspace --no-default-features` → 247 + 6 + 1 passing
   - `git diff --check`, `cargo build --release`
4. **Environment:**
   - `/tmp` is a full tmpfs (other projects' build trees). Set `TMPDIR` and
     `CLAUDE_CODE_TMPDIR` to a directory under `$HOME`.
   - `target/` is 42 GB; `cargo clean` is safe.
   - The pinned toolchain is Rust 1.97.1.

## 3. Rules to keep

- **Behaviour-preserving steps only.** Each step should be small, pass the full
  matrix, and change structure or behaviour, never both.
- **Do not change names or paths.** `AGENTS.md` (gitignored, read it) fixes
  binary, unit, config, state, cache and socket names and the module boundaries
  (`config`, `paths`, `discovery`, `policy`, `solar`, `state`, `ipc`, …).
- **Keep compatibility.**
  - Config and state serde compatibility: new fields use `serde(default)`, and
    older state files must still load (see the state schema version).
  - IPC between `sunreactorctl` and `sunreactord` must keep working across a
    one-version gap.
- **Keep hardware safety intact.**
  - Monitor identity is never inferred from names, ordering or connector alone.
  - DDC writes go to an EDID-verified bus or through ddcutil identity selectors
    (`backends/ddc.rs`, `runtime/topology.rs`).
  - Unit tests never touch real hardware, HTTP, the real config or the socket.
    Use `ModelEnvironment::isolated()`, `FakeRunner`, and the test `drm_root()`.
- **Keep product semantics.**
  - Applied brightness is `Option` and shows `Unknown`, never 0 %.
  - Ranges use the absolute 0–100 % scale.
  - `transition_gamma` is presented as "Solar curvature → Gamma".
  - Stale weather is shown as stale.
  - City selection is atomic (city, coordinates and timezone change together).
- **Keep TUI rules.**
  - Help and footer metadata (`tui/command.rs`) stay separate from key dispatch
    (`tui/update.rs`).
  - Arrows follow layout direction.
  - Colours come from theme palettes only; the weather pixel art is the
    documented exception, tinted 30 % toward the theme.
- **Keep performance fixes.**
  - Policy previews run on a background thread.
  - The adaptive zenith is memoised (`policy/milestones.rs`); a regression
    brings back a 12 s TUI startup.
  - Wake probes use the fast bus read. `ddcutil detect` takes ~5 s on this
    machine; never call it on a hot path.
- **Keep secrets out.** The OpenWeather key must never appear in logs, errors
  or the TUI.
- **Windows.** `src/platform/windows/*` is not built by Linux CI. After moving
  shared code, at least `cargo check --target x86_64-pc-windows-gnu` (or keep
  those modules untouched).

## 4. Where the effort pays off

| Area | Size | Suggestion |
|------|------|------------|
| `src/tui/tests.rs` | 8.1k lines | Move tests next to their modules; share fixtures (`dummy_status`, `two_monitor_model`, `find_in_buffer`, `buffer_text`) from `tui/test_support.rs`. |
| `src/runtime/orchestrator.rs` | 3.7k lines | Split the run loop, IPC request handling, status assembly, weather refresh, and capability refresh into separate files; the loop body should read as a list of steps. |
| `src/tui/app.rs` | 1.8k lines | Separate the preview job, milestone editing, config save, and input helpers. |
| `src/tui/ui/{weather,automation,light_cycle,weather_art}.rs` | 0.9–1.4k each | 18 `#[allow(clippy::too_many_lines)]` in `src/tui`; split render functions into layout vs. drawing. Move `light_cycle::mix` to `theme.rs`, and the two number fonts (`kit::big_text_rows`, `weather::dot_matrix_rows`) to one `fonts` module. |
| `src/apply/engine.rs` | 1.4k lines | Remove the now-unused `lifecycle_recovery` parameter (the wake watch replaced its only caller). |
| `src/runtime/wake.rs` | small | `WakeReassertReason::{Startup, ManualWake, TopologyRecovery}` are never constructed; rename the enum to `WakeReason`. |

`docs/CLEANUP_NOTES.md` has the full list of unreferenced functions,
`#[allow(dead_code)]` to re-check, and follow-ups.

## 5. Repository clutter (not code)

The root holds gitignored scratch files: `fix_*.py`, `split_*.py`, `patch.py`,
`runtime_*.py`, `get_lines.py`, `check_errors.txt`, `test_tz.rs`,
`cities15000.{txt,zip}`, and release and backup tarballs. They are not built,
but they confuse searches and agents. Move them out or delete them after
confirming nothing references them.

## 6. After the refactor

1. Run the full matrix and compare test counts with the baseline.
2. Only then install to `~/.local/bin` and restart `sunreactord`.
3. Check that the service is active, `sunreactorctl ping` answers,
   `sunreactorctl status` shows both monitors `present`, and
   `~/.config/sunreactor/config.toml` is byte-identical.
4. Open the TUI and check each tab once.

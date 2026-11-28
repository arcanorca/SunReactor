# SunReactor Phase 9 — Owner Brief

## 1. What were we solving?

Runtime behavior when configured displays disappear, return, move to another DDC bus, or are represented by both DDC and ddcci backlight entries.

## 2. Why was it important?

Phase 8 made generated configuration safe, but an existing/manual configuration could still place two logical targets on one physical display. Without runtime reconciliation, the daemon could issue duplicate writes, retain stale assumptions after unplug, or let one failing display interfere with healthy displays.

## 3. Approach taken

- Added an ephemeral `CapabilitySnapshot` in `src/runtime/topology.rs`; it is not persistent configuration or generalized hardware inventory.
- Added pure `reconcile(configured, observed)` classification.
- Reused the existing bounded discovery/`ProcessRunner` path; no libudev, libddcutil, systemd, logind, kernel-module reload, or new permanent event thread.
- Refreshes the snapshot periodically before scheduled daemon ticks.
- Added an apply-plan gate. Only `Present` targets dispatch; unavailable, ambiguous, and proven aliases produce `SkippedTopology` diagnostics.
- DDC-primary suppression uses only the Phase 8 authoritative `ddcci_connector` evidence. Names, ordering, `type=raw`, and serial similarity do not establish cross-backend identity.
- Existing configuration is never rewritten and observed-but-unconfigured devices are never adopted.

## 4. Evidence that it works

Deterministic topology tests: **9 passed**. They cover internal+external presence, disappearance with healthy internal continuation, return, DDC bus change with an authoritative serial selector, unknown selector fail-closed behavior, manual DDC+ddcci suppression, ddcci fallback, no name/type inference, and no auto-adoption.

The existing apply-engine tests remain green: **4 passed**. Full all-features test run: **197 library + 6 binary tests passed**.

These are deterministic/software evidence only; no real DDC, ddcci-driver, hotplug, suspend/resume, or hardware write was exercised.

## 5. What we still do not know

- The current implementation uses periodic observation; it does not yet consume DRM/uevent notifications. Detection latency is therefore bounded by the daemon cadence and discovery duration.
- ddcci-driver-linux monitor hotplug behavior was not hardware-qualified. A stale ddcci node is not treated as sufficient proof; current DDC discovery must qualify the external monitor for DDC-primary suppression.
- A race between observation and unplug can still cause one bounded backend failure. The snapshot cannot make a physical unplug atomic with an apply.
- Real hardware behavior, changed bus assignment, suspend/resume recovery, and multi-monitor identity require manual qualification.

## 6. What changed for the user?

A manual configuration containing a proven DDC monitor and its proven ddcci backlight alias no longer sends both through the effective runtime apply set. A missing configured monitor is skipped without deleting its logical identity or configuration, while healthy configured displays continue. When authoritative discovery sees the configured target again, it becomes eligible on a later tick. New observed hardware is not controlled automatically.

## 7. OWNER DECISION REQUIRED

None for the implemented narrow scope. Event-triggered hotplug monitoring remains intentionally deferred. Adding mandatory libudev/libddcutil, adopting unconfigured hardware, fuzzy identity, persistent config mutation, or systemd/logind coupling still requires an explicit owner decision.

## 8. Recommended next step

Run the manual hardware qualification checklist in `docs/phase9-engineering-record.md`. If lower unplug-to-reconciliation latency is required after that evidence, make a separate owner decision about a narrow event source; do not silently add a dependency or runtime thread.

## 9. Phase boundary

Phase 9 only. Phase 10 was not started.

## Exact Phase 9 files changed

- `src/runtime/mod.rs`
- `src/runtime/topology.rs`
- `src/runtime/orchestrator.rs`
- `src/apply/engine.rs`
- `src/apply/types.rs`
- `src/discovery/mod.rs`
- `docs/phase9-owner-brief.md`
- `docs/phase9-engineering-record.md`

No persistent config schema was changed.

## Git status / diff stat

To be recorded from the final verification command; the working tree intentionally contains pre-Phase-9 dirty changes.

## Evidence level

Inspected: current runtime control flow and dependencies. Implemented: pure reconciliation, runtime refresh hook, apply gate. Compiled: `cargo +stable check --workspace --all-features`. Tested: deterministic topology and existing apply tests plus full all-features suite. Hardware verified: **no**.

## Note on runtime event semantics

Linux DRM/HOTPLUG events are state-change hints, not complete topology snapshots. This implementation does not consume event payloads as state. It re-observes through discovery before scheduled ticks. A future event fast path must debounce/coalesce and then perform the same fresh observation and reconciliation; it must not directly mutate monitor state.

## Note on resume

Existing resume/resync behavior is preserved. Its current contract clears/reasserts runtime apply state; hardware observation is now refreshed on the daemon's next scheduled reconciliation. A dedicated resume topology path was not added.

## Failure isolation

Discovery and backend process calls retain existing bounded `ProcessRunner` timeouts. The apply engine continues to fan out independent targets, and a skipped/unavailable external target does not prevent an eligible internal target from entering the apply set.

## Stale snapshot safety

The snapshot is ephemeral and used only to gate the current effective apply set. It does not rewrite config, delete state identity, or redirect a selector. A device may still disappear after observation; the existing backend failure/backoff path handles that bounded race and the next scheduled observation can classify it again.

## Limitations

The current patch does not provide uevent-triggered fast-path reconciliation, status fields for per-monitor topology classifications, or a `doctor` command. These are deliberately not expanded into this phase without stronger product/owner decisions or additional evidence.

## Phase 9 status

Implementation and deterministic software verification complete within the narrow periodic-reconciliation scope. Hardware qualification remains pending. Phase 10 not started.

## Verification commands

The final record must report fresh output for the commands requested by the owner, including fmt/check/clippy/test/no-default builds/help smokes/installer tests/release tests, rather than reusing Phase 8 results.

## Hardware qualification checklist

For each case, capture the observation command, expected status, whether a brightness write is required, and rollback/safety result:

1. **Laptop internal panel** — `sunreactorctl discover --json`; status shows configured backlight present; use `--dry-run` first, then one bounded brightness write; restore the recorded original level.
2. **Laptop + HDMI/DP monitor** — `sunreactorctl discover --json` and `sunreactorctl status --json`; both configured targets present; one controlled write per physical display; restore both levels.
3. **Laptop + USB-C/dock** — same commands plus record connector/bus; verify selector remains authoritative; write only after identity is unique; restore levels.
4. **Two external monitors** — discover JSON and status JSON; each configured selector maps to one display; write each once; restore independently.
5. **Identical-model monitors** — discover JSON with serial/connector evidence; ambiguous selectors must remain unavailable/unsafe; do not write until selectors are explicit and unique.
6. **Unplug/replug** — record `status --json` before, during, and after unplug; external becomes unavailable, internal remains eligible, return restores eligibility; writes are optional and should start in dry-run.
7. **Suspend/resume** — record status before suspend and after resume; existing logical identities remain; perform a write only after fresh status confirms the target; restore levels.
8. **ddcci-driver installed** — record `discover --json` and relevant `/sys/class/drm`/`/sys/class/backlight` links; proven DDC+ddcci is one effective target, stale ddcci alone is not proof; no writes until DDC qualification is current; restore levels.

Suggested observation commands are read-only unless noted: `sunreactorctl discover --json`, `sunreactorctl status --json`, `readlink -f /sys/class/drm/*/ddcci_backlight`, and `readlink -f /sys/class/backlight/*/device`. Never reload kernel modules automatically.

## No Phase 10

Do not begin runtime hotplug event architecture, generalized identity storage, or UX/doctor work under this closure.

## Git status / diff stat

Populate with the final live `git status --short` and `git diff --stat` output.

## Exact changed files

See the Owner Brief list above; verify against final `git diff --name-only` before closing.

## Final evidence classification

- Static source inspection: complete.
- Deterministic unit tests: complete.
- Fresh full requested verification matrix: pending until final close.
- Linux hardware qualification: pending.
- Release/musl/aarch64 hardware qualification: not claimed.
- Phase 10: not started.

## Phase 9.3 Executive Brief: Identity Restoration & Product-Language Cleanup

1. **Dual-Tone ASCII Logo Restored:**
   - Middle stripe row restored to `palette.secondary_accent` (`chrome_title_secondary`).
   - Adapts naturally across all 24 themes without hardcoded Amber RGBs.
   - Power-on sweep transitions cleanly: dual-tone initial -> sweep band -> dual-tone settled.

2. **Plain Unix Copy & GNOME HIG Alignment:**
   - Stripped all theatrical marketing phrases (`OPERATOR REFERENCE` -> `HELP`, `INSTRUMENT LEGEND` -> `SYMBOLS`, `LOCATION IDENTITY` removed, `DRIVES AUTOMATION` removed).
   - Stripped weather empty state prose paragraph slop -> clean `WEATHER` / `○ Off` / `Enable in Settings > Weather.`.
   - Unified terminology (`ATMOSPHERIC SUBSYSTEM` / `ATMOSPHERIC INPUT` -> `WEATHER` / `WEATHER INPUT`).

3. **Settings Layout & Spacing:**
   - Balanced two-column structure: Left (`INTERFACE` + `SERVICE`), Right (`POWER` + `WEATHER`).
   - Fixed cramping with minimum 1-2 blank rows between semantic groups.
   - Concise labels: `Dim after idle`, `API key`, `SERVICE`.
   - Progressive disclosure: hints visible only on active/focused rows.

4. **Interaction Salience:**
   - Distinct 3-channel active selection: Focus rail `▌`, label typography, and high-contrast value capsule `[ <value> ]`.
   - Pure luminance contrast function ensures safe readable text across every theme.
   - Clean separation of focused vs editing state. Non-interactive rows never imitate focus.


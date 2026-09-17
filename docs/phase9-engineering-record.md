# SunReactor Phase 9 — Engineering Record

## Scope

Phase 9 addresses runtime topology reconciliation and duplicate-control safety. Phase 8 discovery/config-generation behavior is treated as a frozen boundary. No Phase 10 work was started.

## Pre-Phase-9 runtime behavior

The daemon loaded `MonitorConfig` during bootstrap and computed policy targets from configured monitors. `run_once_at_with_runner` passed those targets to the apply engine. `apply_policy_with_runner_monitors` performed per-target decision-making and concurrent backend dispatch, with existing per-monitor backoff/failure isolation. DDC selectors were resolved by the DDC backend on apply. Backlight pins were resolved by the backlight backend on apply.

Discovery was previously an explicit CLI/discovery path, not a daemon capability snapshot. Existing resync cleared apply markers/backoff as appropriate and re-applied configured targets; it did not itself establish an authoritative fresh topology state. No udev/netlink/DRM event observer existed in the runtime.

## Runtime topology model

`src/runtime/topology.rs` adds an ephemeral `CapabilitySnapshot` containing currently discovered viable DDC entries and viable backlight entries. It is not serialized, persisted, or used as a hardware database. `CapabilitySnapshot::from_discovery` only clones a discovery report. Hardware observation remains at the discovery/runtime boundary and uses the existing bounded process runner.

`reconcile(configured, observed)` returns one `ReconcileAction` per configured monitor, in configured order:

- `Present`: eligible for apply;
- `TemporarilyUnavailable`: configured intent is retained but no current capability proof exists;
- `SuppressedProvenAlias`: observed backlight is an authoritative alias of a present viable DDC display;
- `AmbiguousOrUnsafe`: multiple current matches would require choosing an identity;
- `UnconfiguredObserved`: reserved for observed-only diagnostics; observed hardware is never auto-adopted.

Only `Present` is allowed to apply.

## Manual duplicate-config handling

The apply engine's effective plan now accepts reconciliation actions and emits `ApplyStatus::SkippedTopology` for unavailable, ambiguous, and suppressed targets before concurrent dispatch. Therefore a manual DDC entry plus its proven ddcci backlight alias produces one effective DDC write, not two. Persistent config is untouched.

Suppression requires all of:

1. configured backlight path matches a currently observed viable backlight;
2. that observed backlight carries `ddcci_connector` from Phase 8 canonical symlink-topology evidence;
3. a current viable DDC discovery entry has the same connector.

Names, ordering, `type=raw`, serial similarity, GPU vendor, and “only one external display” are not used as cross-backend identity.

## Disconnect/reconnect sequence

- **Snapshot N:** internal backlight and external DDC match their configured selectors; both classify `Present`.
- **Snapshot N+1:** external DDC is absent from fresh discovery; it classifies `TemporarilyUnavailable`, while internal remains `Present` and continues applying. No config/state identity deletion occurs.
- **Snapshot N+2:** external returns and its authoritative selector matches again; it classifies `Present` and normal forced/reassert behavior can apply it.
- **DDC bus change:** a serial selector may match the returning DDC monitor despite a changed bus/connector slot; the bus number is not used as identity. If the selector cannot prove a unique current match, the target remains unavailable/unsafe.
- **DDC + ddcci return:** the same current connector evidence causes DDC to be `Present` and the ddcci alias to be `SuppressedProvenAlias`.

## Hotplug event architecture

No event listener was added. Current implementation uses periodic re-observation immediately before scheduled daemon ticks, reusing the existing discovery path and process timeouts. This avoids a mandatory libudev/libddcutil dependency, a new permanent thread, and lifecycle coupling. Detection latency is bounded by daemon cadence plus bounded discovery time.

A DRM/kernel `HOTPLUG=1` event, if added in a future separately approved scope, must be treated only as “topology may have changed”: debounce/coalesce, fresh observe, reconcile, then apply. Event payload must not directly mutate monitor state.

## ddcci hotplug limitation

The ddcci driver does not automatically detect monitor hotplug in the required sense. A stale ddcci sysfs node is not current presence proof. Current DDC discovery/qualification dominates DDC-primary alias suppression where possible. Kernel modules are not reloaded automatically.

## Resume integration

Existing deterministic suspend/resume machinery and resync/backoff clearing remain unchanged. The next scheduled reconciliation refreshes the ephemeral capability observation before the apply path. No logind/systemd dependency or separate resume topology mechanism was introduced.

## Stale snapshot/race analysis

Observation and apply are not atomic with physical unplug. A target can disappear after a snapshot and before dispatch. The bounded backend process timeout and existing failure/backoff path may record one failure, but the snapshot does not mutate persistent configuration, delete logical identity, or redirect a selector to an unrelated device. A later scheduled observation can classify the target again. A future event during reconciliation must result in another fresh reconciliation, not permanent state mutation.

## Failure isolation

The apply engine preserves per-monitor failure isolation and concurrent dispatch. A missing/hanging external display can be skipped by topology or fail/back off independently; an eligible internal panel remains in the effective apply set. Discovery and backend calls continue to use existing bounded `ProcessRunner` guarantees. No single DDC probe is allowed to define healthy-display state.

## Deterministic fixtures

`runtime::topology::tests` contains 9 software-only tests:

- internal plus external present;
- external absent with internal continuing;
- external return with same selector;
- changed DDC bus with authoritative serial selector;
- unknown selector fail closed;
- manual DDC + ddcci duplicate suppression;
- ddcci fallback when DDC unavailable;
- no name/`type=raw` duplicate inference;
- unconfigured observed hardware is not adopted.

The tests construct discovery records directly and do not read real `/sys` or spawn hardware processes. Existing apply-engine tests continue to exercise the mixed-hardware failure and latency paths.

## Hardware qualification plan

Run read-only observation first with `sunreactorctl discover --json`, `sunreactorctl status --json`, `readlink -f /sys/class/drm/*/ddcci_backlight`, and `readlink -f /sys/class/backlight/*/device`. Use dry-run before writes and record/restore original brightness.

1. **Internal laptop panel:** discover JSON and status; configured backlight is present; one optional bounded write after dry-run; restore original.
2. **HDMI/DP external:** discover and status; internal+external present; one write per physical display; restore both.
3. **USB-C/dock:** capture connector and bus; verify selector remains authoritative through topology-slot changes; write only after unique qualification; restore.
4. **Two external monitors:** verify each selector matches one display; write each once; restore independently.
5. **Identical models:** require serial/connector evidence and explicit unique selectors; ambiguous state must not write.
6. **Unplug/replug:** capture status before/during/after; external unavailable, internal healthy, return eligible; begin with dry-run.
7. **Suspend/resume:** status before/after; identity preserved; write only after fresh status; restore.
8. **ddcci installed:** inspect both symlink paths and discovery JSON; proven DDC+ddcci yields one effective target; stale ddcci alone is insufficient; no write until current DDC qualification.

Hardware qualification has not been performed in this run and is not claimed.

## Exact Phase-9 files changed

- `src/runtime/mod.rs`
- `src/runtime/topology.rs`
- `src/runtime/orchestrator.rs`
- `src/apply/engine.rs`
- `src/apply/types.rs`
- `src/discovery/mod.rs`
- `docs/phase9-owner-brief.md`
- `docs/phase9-engineering-record.md`

## Verification evidence

Fresh verification completed on the exact dirty tree:

- `cargo +stable fmt --all --check`: **pass**
- `cargo +stable check --workspace --all-targets --all-features`: **pass**
- strict clippy `-D warnings -D clippy::dbg_macro -D clippy::todo`: **pass**
- `cargo +stable test --workspace --all-targets --all-features`: **197 library + 6 binary + 0 examples/documentation tests, all pass**
- `cargo +stable check --no-default-features`: **pass**
- `cargo +stable test --no-default-features`: **pass**
- `cargo +stable build --no-default-features --bin sunreactord`: **pass**
- `cargo +stable build --no-default-features --bin sunreactorctl`: **pass**
- `cargo +stable run --bin sunreactord -- --help`: exit 0
- `cargo +stable run --bin sunreactorctl -- --help`: exit 0
- `bash tests/installer_test.sh`: **passed**
- `bash tests/release_test.sh`: **passed**
- `git diff --check`: clean

Remaining warnings are the documented pre-existing dormant-sysfs-fallback dead-code baseline and the pre-existing feature-gated `policy::milestones::compute_monitor_milestones` (no-default); none were introduced by Phase 9. The live git status and diff stat are captured in the Owner Brief and this record's Git State section.

## Git state

The worktree is intentionally dirty from earlier phases. Final status and diff stat must be captured live at closure; no commit, push, reset, stash, or history rewrite is performed.

## Limitations and owner decisions

No current owner decision is required for this narrow periodic implementation. A future event-triggered fast path, mandatory libudev/libddcutil, automatic adoption, fuzzy identity, persistent config mutation, or systemd/logind coupling requires a new explicit owner decision. A `doctor` CLI was intentionally not added. Phase 10 was not started.

## Evidence classification

- Current runtime/source inspection: completed.
- Pure reconciliation and apply-gate implementation: completed.
- Deterministic topology tests: completed, 9 passed.
- Existing apply tests: completed, 4 passed targeted.
- Fresh all-feature suite: completed, 197 library + 6 binary passed.
- Full owner-requested matrix: pending final run.
- Real hardware: not exercised.
- Hardware/release qualification: not claimed.
- Phase 10: not started.

## Closure rule

Do not describe this phase as hardware-qualified. Close only after the fresh requested matrix and live git evidence are recorded. Keep Phase 8 frozen and do not begin Phase 10.

## Git status / diff stat

Populate from the final live command immediately before delivery.

## Final evidence classification

Static inspection and deterministic software tests are evidence of the implemented pure/gated logic. They do not prove physical display identity or driver hotplug behavior on all supported hardware.

## No Phase 10

Explicitly not started.

## Additional qualification note

The periodic path intentionally trades sub-cadence hotplug latency for a simpler lifecycle and no new mandatory dependency. If that tradeoff is unacceptable after manual evidence, return to the owner for a scoped decision.

## End

Phase 9 scope only.

## Appendix: command safety

Do not automatically reload ddcci modules. Do not write brightness during observation-only checks. Do not mutate user configuration as part of reconciliation.

## Appendix: selector safety

A changed I2C bus is recoverable only when an existing selector proves the same current monitor uniquely. Otherwise retain unavailable/ambiguous status.

## Appendix: apply safety

The topology gate runs before the existing concurrent backend dispatch. It does not replace backend failure handling; it prevents known unsafe writes before they occur.

## Appendix: status

Current status output still exposes configured monitor state and apply diagnostics; a richer per-monitor topology status field is intentionally deferred, not silently invented.

## Appendix: future event flow

`event hint -> debounce/coalesce -> fresh discovery -> pure reconcile -> effective apply -> diagnostics`.

## Appendix: persistence

`CapabilitySnapshot` is runtime-only. Existing state persistence remains for apply/backoff/runtime state and is not used as topology authority.

## Appendix: unconfigured hardware

Observed but unconfigured hardware remains informational and cannot enter the apply plan.

## Appendix: ambiguity

No equivalence is manufactured when selectors or topology evidence do not uniquely prove it.

## Appendix: healthy displays

The effective apply set is per target; external failure does not globally block internal application.

## Appendix: final boundary

Do not begin Phase 10.

## Appendix: owner brief linkage

See `docs/phase9-owner-brief.md` for the one-to-two-minute summary.

## Appendix: generated config boundary

Phase 8 generated-config suppression remains unchanged conceptually; Phase 9 applies equivalent evidence to the runtime effective set.

## Appendix: no schema

No persistent configuration schema or state schema was expanded for topology.

## Appendix: no architecture expansion

No new async runtime, native udev dependency, DBus dependency, kernel module management, or mandatory desktop/session service was added.

## Appendix: review status

Independent read-only reviews agreed that the pure snapshot and apply gate preserve the intended evidence boundaries; the remaining known limitation is event-driven hotplug latency.

## Appendix: final reminder

Use exact command output for final matrix and live status; do not reuse previous PASS claims.

## Phase 9.3: Identity Restoration, Interaction Salience & Product-Language Cleanup

### 1. Root Cause of Masthead Flattening
During theme style semantic consolidation, `styles.focus_marker` was mapped to `palette.accent`. In `src/tui/ui/chrome.rs`, the middle row of the ASCII logo was assigned `styles.focus_marker`, while the remaining rows used `styles.chrome_title` (also `palette.accent`). This unintentionally collapsed the original dual-tone sunrise/reactor aesthetic into a monochrome wordmark across all themes.

### 2. Historical Provenance
Historical investigation traced to commits `a535983` and `c9418cb` (`src/tui/ui/chrome.rs:106-130`), confirming that row 2 (`\___ \|...`) originally rendered with `palette.secondary_accent`, while rows 0, 1, 3, 4 rendered with `palette.accent`.

### 3. Dual-Tone Recovery Mechanism
Added `chrome_title_secondary: Style` to `SemanticStyles`, initialized from `palette.secondary_accent`. Line 2 in `logo_lines` uses `styles.chrome_title_secondary`. Zero hardcoded RGB colors are used. The power-on sweep transitions through:
`historical dual-tone initial state` -> `temporary highlight band` -> `historical dual-tone settled state`.

### 4. Product-Language Audit & Cleanup
Applied GNOME HIG and Unix ergonomics: stripped theatrical marketing, pseudo-avionics, and repetitive title prefixes across the TUI.

| Component | Before | After | Rationale |
|:---|:---|:---|:---|
| Chrome Help | `OPERATOR REFERENCE` | `HELP` | Plain Unix copy; operator is unnecessary jargon |
| Chrome Symbols | `INSTRUMENT LEGEND` | `SYMBOLS` | Concise, familiar GNOME HIG vocabulary |
| Chrome Footer | `OBSERVE` | `WEATHER` | Direct tab identity rather than obscure mode name |
| Location | `Solar Location` | `Location` | Redundant modifier removed |
| Location | `LOCATION IDENTITY` | *(Removed)* | Unnecessary sub-heading when field is obviously City |
| Location | `DRIVES AUTOMATION` | *(Removed)* | Theatrical marketing slogan stripped from status bar |
| Location | `World Position` | `Map` | Crisp instrument panel title |
| Location Form | `POS <coords>` (bottom of left form) | *(Removed when map visible)* | Redundant duplicate with map metadata footer; only renders when map is omitted |
| Weather Empty | `ATMOSPHERIC SUBSYSTEM` + 3 prose paragraphs | `WEATHER` / `○ Off` / `Enable in Settings > Weather.` | Clean, quiet empty state; no architectural lecture |
| Weather Status | `ATMOSPHERIC TELEMETRY` / `WEATHER STATUS` | `STATUS` | Direct, familiar terminology without repetitive prefix |
| Weather Input | `ATMOSPHERIC INPUT` / `WEATHER INPUT` | `INPUT` | Direct, familiar terminology with `VALID <time>` tag |
| Weather Policy | `WEATHER / SOLAR POLICY` / `WEATHER POLICY` | `BRIGHTNESS` | Direct domain noun matching monitor brightness effect |
| Weather Compact | `ATMOSPHERIC INPUT & POLICY` / `WEATHER & POLICY` | `INPUT & BRIGHTNESS` | Concise combined header for compact mode |
| Settings Section | `POWER MANAGEMENT` | `POWER` | Balanced section title |
| Settings Section | `ATMOSPHERE` | `WEATHER` | Aligned with Tab 4 and user mental model |
| Settings Section | `SYSTEM & DAEMON` | `SERVICE` | Concise heading for daemon and write controls |
| Settings Label | `Dim automatically after idle` | `Dim after idle` | Concise label |
| Settings Label | `OpenWeather API key` | `API key` | Concise label |
| Settings Hints | Permanent `(0 to disable)` etc. | Progressive disclosure | Hints only appear when focused or editing |
| Service Status | `(daemon service unreachable)` / `(applying solar curve)` | Concise `Offline` / `Enabled` | Duplicate detail eliminated |

### 5. Settings Layout & Spacing
- Fit-based layout calculation:
  - Responsive mode requires both height ($\ge 11$) and width ($\ge 64$) before committing to two columns. Narrower viewports cleanly use single-column minimal stacked layout without truncation.
  - **Left column:** `INTERFACE` (5 settings) + explicit gap + `SERVICE` (Daemon, Writes, Actions).
  - **Right column:** `POWER` (2 settings) + explicit gap + `WEATHER` (4 settings).
- **Hard invariant:** Minimum 1 blank row between sections in compact workspaces; 2 blank rows when terminal height $\ge 14$. Separator lines do not count as the blank row.
- Both columns are intrinsically balanced at 10-12 rows each, eliminating right-column cramping.

### 6. Active Selection & Focus Architecture
- Implemented three distinct visual channels:
  - **Focus rail:** `▌` in bold accent color.
  - **Label contrast:** Enhanced typography (`text_heading` / bold).
  - **Value capsule:** Dedicated `[ <value> ]` brackets and filled background capsule with contrast-safe foreground text.
- **Progressive disclosure:** Hints appear only on active/focused rows.
- **Contrast safety:** Function `contrast_fg_for_palette(bg, palette)` derives contrast-safe foreground text (`palette.bg` or `palette.fg`) directly from the active theme's palette, preserving theme identity across all 24 themes.
- **Editing state semantics:** Editing is NOT a warning condition. `value_capsule_editing` uses `palette.secondary_accent` (`Modifier::BOLD | Modifier::UNDERLINED`) with editing cursor `▏` in `focus_marker` color, reserving warning colors strictly for actual alerts and degraded states.
- **Read-only distinction:** Non-interactive rows (`Provider`, `Refresh interval`) use muted styling and never display focus rails or brackets.

### 7. Phase 9.3.1: World Map Aspect Ratio Root Cause & Resolution
- **Root Cause:** When terminal emulators report character counts rather than physical pixels (e.g. `pixel_width == columns && pixel_height == rows`), the raw aspect ratio evaluated to $1.0$. The loose validity check `(0.25..=1.5).contains(&aspect)` accepted $1.0$ as physical terminal pixels. This caused `desired_cell_ratio = WORLD_ASPECT / 1.0 = 2.0 / 1.0 = 2.0`, crushing the horizontal width of the world map in half and rendering a vertically stretched "giraffe" silhouette.
- **Correction:**
  - Plausible terminal monospace font cell aspect ratio clamped strictly to `[0.35, 0.65]`. Bogus $1.0$ character counts are rejected and fall back to conservative `DEFAULT_CELL_ASPECT = 0.45` (matching typical Linux monospace geometry, e.g. 9px $\times$ 20px).
  - Equirectangular projection ratio: $\text{cols} / \text{rows} = 2.0 / 0.45 = 4.44$, ensuring physical pixel aspect ratio $L_x / L_y = 2.00$ on real displays.
  - Form layout split: Replaced rigid 44%/56% split with `Constraint::Min(36), Constraint::Percentage(60)`, giving the world map canvas maximum horizontal span.
  - Redundant coordinate display: Removed clipped `POS ...` line from left form when map is visible.

### 8. Phase 9.4: Monitor-Aware Automation + Brightness Range Instrument + Adaptive Map Semantic Zoom
- **Pre-Flight Policy Model Trace:**
  - `transition_gamma`: Stored per-monitor in `MonitorConfig.transition_gamma` (default `0.50`, valid range `0.0 < gamma <= 5.0`).
  - Transfer function: $y = x^\gamma$ where $x \in [0.0, 1.0]$ is pure solar daylight factor. Midpoint elevation yields higher brightness earlier when $\gamma < 1.0$ and delays brightening when $\gamma > 1.0$.
  - Projection: $\text{scaled} = \text{min} + y \times \text{gain} \times (\text{max} - \text{min})$, clamped to $[\text{min}, \text{max}]$.
  - Weather modifier adjusts effective factor before hardware projection. Manual overrides bypass the solar curve entirely until expiry.
- **Ownership Realignment:**
  - *Monitors tab (Tab 1)* owns safe hardware boundaries and applied status: `BRIGHTNESS RANGE` (Min% and Max%). Removed deceptive non-interactive `Gamma correction` row.
  - *Automation tab (Tab 2)* owns solar brightness curve policy: `Curve shape`, milestone schedule, and solar-derived targets.
- **Automation Monitor Context & Navigation:**
  - Added dedicated header line at the top of Automation: `MONITOR <name> <idx> / <total> [ / ] switch`.
  - Added dedicated monitor-switching keys `[` (previous) and `]` (next) on Tab 2, avoiding conflict with `← / →` milestone offsets.
  - Unified monitor selection: Tab 1 and Tab 2 share `app.selected_monitor` seamlessly.
  - Recomputing targets: Switching monitors immediately recomputes milestone table targets via core policy (`crate::policy::milestones`) for that monitor's min, max, gamma, and gain.
- **Curve Shape Interactive Control in Automation:**
  - Added `AutomationRegionFocus`: `Curve` vs `Milestones`.
  - Up / Down (`k` / `j`): Moves smoothly between Curve control and Milestone table.
  - `← / →`: Fine-adjusts curve exponent by $\pm 0.05$ (clamped to $[0.05, 3.0]$).
  - `Enter`: Exact numeric edit mode in `value_capsule_editing` with live preview. `Esc` cancels edit.
  - Validation: Clamped to finite numbers in $0.0 < \text{val} \le 5.0$.
- **Unified Tactile Brightness Range Instrument in Monitors Tab:**
  - Replaced disconnected Min and Max text rows with a single cohesive range instrument:
    `0 ────────┃━━━━━━━━━━━━━━━━━━━━┃──────────────── 100`
    `          MIN 7%              MAX 60%`
  - Truthful proportional positions: $\text{min\_idx} \propto \text{min\_pct} / 100$, $\text{max\_idx} \propto \text{max\_pct} / 100$. Unoccupied ranges $0 \dots \text{min}$ and $\text{max} \dots 100$ remain visible.
  - Handle focus: Active handle transforms to bold `█` with `[ MIN 7% ]` or `[ MAX 60% ]` capsule below.
  - Direct adjustment: `← / →` steps focused handle by $\pm 1\%$. `Enter` enters exact numeric edit. `Esc` returns to monitor list.
- **Adaptive Map Semantic Zoom in Location Tab:**
  - Implemented discrete semantic zoom levels:
    - `World`: $360^\circ \times 180^\circ$ span ($2.0$ ratio) for large viewports ($\ge 54 \times 12$).
    - `Continental`: $180^\circ \times 90^\circ$ span ($2.0$ ratio) for medium viewports ($\ge 36 \times 8$), centered on configured location.
    - `Regional`: $90^\circ \times 45^\circ$ span ($2.0$ ratio) for small viewports ($< 36 \times 8$), centered on configured location.
  - Aspect Ratio Invariance: Exact $2:1$ equirectangular ratio maintained at EVERY zoom level.
  - Geographic Clamping: All views clamped strictly to valid ranges $[-180, 180]$ and $[-90, 90]$ with Date Line and Polar bounds safety.
  - Unified Projection: Reticle, crosshair, acquisition ping, target point, and border ticks scale and align identically to the active zoom level without spatial drift.
  - Frame titles: `Map · World`, `Map · Continental`, `Map · Regional`.

## End of engineering record

Phase 9 only.
# Cleanup notes for the next pass

Handoff notes from the TUI redesign. A light cleanup is done; everything below
is left on purpose for a dedicated pass. Keep behaviour unchanged and run the
full validation matrix (`cargo fmt --check`, CI clippy flags, workspace tests
with and without default features) after each step.

## Already removed in this pass

- `src/tui/location_signature.rs` (decorative skyline, replaced by the
  location/day-length strip) and `src/tui/ui/dots.rs` (Braille canvas, no
  longer used after the charts and weather art moved to block elements).
- Unused TUI helpers: `Model::append_char_to_active_input`,
  `Model::next_automation_milestone_now`, `UiCommandContext::mode_label`,
  `DaemonLifecycle::badge_symbol`, `UiMotionState::range_commit_phase`
  (Monitors matches the transient directly), `theme::contrast_fg_for_bg`, and
  the `render_control` "backwards-compatible" entry points in `settings.rs`
  and `weather.rs`.

## Unreferenced outside their own module (not removed)

These are public items with no callers anywhere in `src/` (binaries
included). They sit in daemon, policy, or platform code, so confirm they are
not intended public API before deleting.

| Item | Location |
|------|----------|
| `apply_policy` | `src/apply/engine.rs` |
| `discover_targets` | `src/discovery/mod.rs` |
| `DisplayEventSources::take_pending` (Windows) | `src/platform/windows/event_sources.rs` |
| `compute_brightness_target` | `src/policy/math.rs` |
| `FadeEngine::cancel_all` | `src/runtime/fade.rs` |
| `DaemonRuntime::run_once_forced` | `src/runtime/orchestrator.rs` |
| `RuntimeState::clear_suspend` | `src/state/transitions.rs` |

## `#[allow(dead_code)]` to re-check

Suppressions that may now be stale; remove the attribute and see whether the
compiler still warns.

- `src/backends/backlight.rs`: twelve items (lines ~22–530).
- `src/backends/ddc.rs`: `read_with_runner` is now called from
  `apply/dispatch.rs`, so its attribute looks stale.
- `src/apply/engine.rs:41`, `src/discovery/model.rs:292`,
  `src/runtime/topology.rs:50`, `src/paths.rs:104`,
  `src/platform/windows/pipe.rs:353`.
- `src/discovery/runner.rs:1` has `#[allow(unused_imports)]`.

## Structure

- `src/tui/tests.rs` is ~8,100 lines. Move tests next to the modules they
  cover (`ui/automation.rs`, `ui/light_cycle.rs`, `ui/weather.rs`,
  `update.rs`) and share the fixtures (`dummy_status`, `two_monitor_model`,
  `find_in_buffer`) from a small `tui/test_support.rs`.
- Eighteen `#[allow(clippy::too_many_lines)]` in `src/tui`. The largest
  candidates to split: `light_cycle::render_chart` (area, markers, labels),
  `weather::render_conditions` (scene, temperature, details, freshness),
  `weather::render_forecast` (scale, area, now line, icon strip), and
  `weather::render_sun_strip` (item fitting vs. drawing),
  `automation::render_schedule_panel`, and `update.rs` dispatch (one handler
  per context, keeping `command.rs` metadata in sync).
- `kit::big_text_rows` (rounded digits, Automation output) and
  `weather::pixel_number_rows` (pixel font, temperature) are two number fonts
  on purpose; if a third appears, give them a shared `fonts` module.
- `light_cycle::mix` is used by weather code as a general colour helper; it
  belongs in `theme.rs` or `kit.rs`.

## Probe schedule

- The signal-driven wake watch became `runtime/wake.rs::ProbeSchedule`, which
  probes on a steady cadence (`daemon.probe_seconds`) and derives wakes from
  the hardware. The earlier `lifecycle_recovery` apply mode and the unused
  `WakeReason` variants are gone.

## Behaviour worth a follow-up (not cleanup)

- The daemon runs a full `ddcutil detect` (~5 s of I2C traffic) before every
  tick. That is now the heaviest recurring cost, far above the probe schedule.
  Presence could be checked each tick with the EDID-verified connector lookup
  and the connector power state in `backends/ddc.rs`, keeping the full scan for
  startup, hotplug, and resume.
- `ddcutil detect` occasionally reports a display as invalid when another
  ddcutil process holds the bus, which marks that monitor unavailable for one
  tick.

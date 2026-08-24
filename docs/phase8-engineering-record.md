# SunReactor Phase 8 — Part B: Engineering Record

**Date:** 2026-08-22
**Branch:** `main` @ `5bfc08d3f403b5c9b1b0ca60fb8eb6654fc2c55f`
**Toolchain (release):** pinned `1.97.1` (via `rust-toolchain.toml`)
**Dev baseline:** `cargo +stable`

---

## 1. Summary of Phase 8 work

Phase 8 implemented the owner-approved **Option 1** resolution:

- Preserved fail-closed explicit backlight pinning (never the historical
  first-device/lexicographic fallback).
- Added `/sys/class/drm/<connector>/ddcci_backlight` + `/sys/class/backlight/<dev>/device`
  convergence resolution for legacy connector-only configs, with a deprecation
  diagnostic on success, fail-closed on zero/multiple/ambiguous candidates.
- Added three regression tests proving the resolver never selects by device-name
  ordering and that realistic `/sys/devices/...` symlink chains are followed
  (canonicalization-based identity, not plain-temp-dir checks).
- Added two discovery-uplift tests after an independent review confirmed gaps:
  one discovery run with both an external DDC monitor and an internal backlight;
  and backend failure isolation when `brightnessctl` errors (nonzero exit).
- Documented (did NOT change) the dormant `apply_sysfs_fallback` chain.
- Audited the mixed-hardware runtime paths (below).
- An independent review subagent re-read the final tree and re-ran
  `backends::backlight` (21), `backends::ddc` (5), `discovery::tests` (6→9
  after uplift + owner-decision regression) and produced no corrected code
  changes; its residual risks are recorded in the Limitations section.

## 2. Files changed (this turn / Phase 8)

**Modified (code/tests):**
- `src/backends/backlight.rs` — resolver + three new tests (count 18→21).
- `src/apply/engine.rs` — test helper `legacy_connector_monitor` +
  `legacy_connector_only_resolves_through_backend_and_surfaces_diagnostic`
  (count 3→4 in `apply::engine`), plus unused-import/mut cleanups.
- **Owner decision response (approved)** — DDC-primary for an external
  monitor when SunReactor has authoritative evidence the same panel is also
  exposed through the `ddcci-driver-linux` backlight:
  - `src/discovery/model.rs` — `BacklightDeviceDiscovery.ddcci_connector` field.
  - `src/discovery/probe.rs` — `discover_with_roots` + `annotate_ddcci_connector_evidence`
    (convergence of `/sys/class/drm/<connector>/ddcci_backlight` and
    `/sys/class/backlight/<dev>/device` through canonical realpath; a connector
    resolving to exactly one hardware device is used; multiple is ignored —
    fail closed).
  - `src/discovery/mod.rs` — `is_ddcci_alias_of_viable_ddc` predicate
    (never names/ordering/`type=raw` alone) + discovery-rooted test-only
    `discover_with_roots`.
  - `src/discovery/render.rs` — `build_config_snippet` suppresses the alias
    from generated config.
  - `src/discovery/tests.rs` — new regression
    `ddc_primary_suppresses_authoritative_ddcci_alias_from_generated_config`.

**Modified (existing dirty-tree context, not edited this turn):**
- README.md, docs/RELIABILITY.md, install.sh,
  contrib/systemd/sunreactord.service, Makefile, Cargo.toml, and the rest of
  the 45 modified-file dirty set from Phases 1–7.

**New (this turn):**
- `docs/phase8-owner-brief.md` (Part A).
- `docs/phase8-engineering-record.md` (this file).

## 3. Evidence table (classification)

| Evidence class | Items |
|---|---|
| Code inspected | `backlight.rs` resolver + `apply/engine.rs` dispatch, `config/*`, `discovery/*`, `runtime/orchestrator.rs`, `state/*`, `backends/ddc.rs` |
| Pure unit | policy/solar/config tests (in suite results below) |
| Filesystem fixture | backlight resolver tests via `tempfile::tempdir()` incl. the new realistic `/sys/devices/...` symlink chain (`realistic_sysfs_devices_symlink_chain_is_followed_not_plain_dirs`) |
| Config round-trip | `config::tests::parses_valid_config` + backend selector assertions through `validate()`; discovery writes explicit `sysfs_path` into generated candidates |
| Process-runner integration | `apply_with_runner`/`apply_with_runner_roots_for_test` through `FakeRunner`; discovery via `discover_with_runner`/`discover_with_roots` through `FakeRunner` |
| Filesystem fixture (topology) | `ddc_primary_suppresses_authoritative_ddcci_alias_from_generated_config` builds real `/sys/devices/../panel0` + canonical `device`/`ddcci_backlight` symlinks; genuine convergence, no name guesses |
| Runtime integration | engine mixed-hardware + latency tests, orchestrator resync tests |
| Real sysfs observed | Not on live hardware this phase (host not used as display-control test bed) |
| Real ddcutil observed | Not exercised live |
| Hardware write | Not exercised live (needs a real panel/DDC bus) |

## 4. Verification matrix (exact numbers)

| Command | Result |
|---|---|
| `cargo +stable test --workspace --all-features` | lib **188** + ctl **6** = **194 passed** |
| `cargo +stable test --workspace --no-default-features` | lib **176** + ctl **6** = **182 passed** |
| `cargo +stable check --workspace --all-targets --all-features` | exit 0 (9 pre-existing dead-code warnings: dormant sysfs chain) |
| `cargo +stable fmt --all --check` | exit 0 |
| `git diff --check` | clean |
| `cargo +stable run --bin sunreactord -- --help` | exit 0 |
| `cargo +stable run --bin sunreactorctl -- --help` | exit 0 |
| `cargo +1.97.1 test --workspace --all-features` | lib **188** + ctl **6** = **194 passed** |

**Targeted suites:**
| Suite | Count |
|---|---|
| `backends::backlight::tests` (all-features) | **21 passed** |
| `apply::engine::tests` (all-features) | **4 passed** |
| `discovery::tests` (all-features, incl. 2 uplifts + 1 owner-approval regression) | **9 passed** |

The following blacklight tests assert the exact never-name-order / pin / and
realistic-chain invariants:

```
test backends::backlight::tests::explicit_pins_are_not_selected_by_device_name_order ... ok
test backends::backlight::tests::legacy_two_device_fixture_fails_closed_instead_of_sorting_names ... ok
test backends::backlight::tests::never_selects_backlight_by_alphabetical_first_name_without_evidence ... ok
test backends::backlight::tests::picks_proven_device_despite_alphabetically_first_unproven_candidate ... ok
test backends::backlight::tests::realistic_sysfs_devices_symlink_chain_is_followed_not_plain_dirs ... ok
test backends::backlight::tests::legacy_connector_only_resolves_via_ddcci_symlink_and_emits_diagnostic ... ok
test backends::backlight::tests::legacy_connector_without_proven_mapping_fails_closed ... ok
test backends::backlight::tests::legacy_connector_multiple_proven_candidates_fails_closed ... ok
```

## 5. Diff-provenance / hygiene

### `git status --short` (final-tree head, first lines)

```
 M README.md
 M contrib/systemd/sunreactord.service
 M docs/RELIABILITY.md
 M install.sh
 M src/apply/engine.rs
 M src/backends/backlight.rs
 ... (45 modified total, dirty inherited from Phase 1–7)
```

### `git log -- path` (backlight.rs / engine.rs)
- backlight.rs provenance: committed origin `c9418cbe` + uncommitted
  Phase 8 (2026-08-22) edits.
- engine.rs provenance: committed origin + uncommitted Phase 8 edits.

### Dormant-fallback provenance (explicit)
- The only `apply_sysfs_fallback` delta versus `HEAD` is the one-line arithmetic
  cast `(percent as u64 …)` → `(u64::from(percent) …)`; **that line is already
  present in the pre-Phase-8 snapshot**
  (`/tmp/sunreactor-pre-display-portability-phase-8.patch`), so it is
  Phase 1–7 work, not a Phase 8 change. Phase 8 left the chain byte-identical
  to the pre-Phase-8 working tree.

### Manual `git diff --check`
- clean (no whitespace/conflict markers).

## 6. Detailed evidence for the five audit topics

1. **Mixed internal + external monitors** — `test_mixed_hardware_failure`
   (healthy Backlight + broken Ddc + missing config) proves a healthy monitor
   continues while the failing DDC backs off. **Added** this phase:
   `discovery::tests::mixed_internal_and_external_monitors_in_single_discovery_run`
   feeds one run with both an external DDC display (`/dev/i2c-7`, XMI monitor)
   and an internal backlight (`intel_backlight`), asserting both appear in the
   report with `viable_targets = 2` and both `backend = "ddc"` and
   `backend = "backlight"` config blocks are emitted. **Verified deterministic,
   in-repo.**

2. **DDC/ddcci duplicate detection** — config validator rejects overlapping /
   ambiguous selectors: `validate()` pairs each enabled DDC monitor and calls
   `ddc::selector_relation`; any relation that is not `ProvablyDisjoint` is
   rejected (`{relation:?}` surfaced in the error). Backlight resolver fails
   closed on 0/multiple proven ddcci candidates
   (`legacy_connector_multiple_proven_candidates_fails_closed`). Physical
   cross-backend dedup now **is** modeled at discovery/config-generation time:
   an alias is suppressed only when `ddcci_connector` equals a *viable* DDC
   monitor's connector (authoritative convergence). Names/ordering alone never
   suppress. Multiple-DDC-monitors-on-one-hw and multiple-connectors-on-one-hw
   still fail closed (no suppression, and DDC ambiguity withholding applies).

3. **Optional backend failure isolation** — dispatch is concurrent; each monitor
   has per-backend `record_apply_failure` backoff; a hanging DDC does not block a
   healthy backlight; mixed-hardware test is the evidence. **Added** this phase:
   `discovery::tests::backend_failure_isolation_keeps_sysfs_probe_when_brightnessctl_errors`
   (brightnessctl exits nonzero; sysfs fallback still produces a viable target).

4. **Configured monitor disappear/reappear** — the daemon provides the
   reconciliation *machinery* (periodic reassert `reassert_due`,
   `prune_to_configured_monitors` on config change, `prepare_apply_resync`
   clearing applied markers/backoff, clean `ApplyStatus::Failed` "no matching
   monitor configuration" records), but **no test simulates a monitor actually
   leaving and returning to hardware topology discovery**. The currently
   passing state/resync tests cover state pruning, forced resync, and backoff
   behavior, not topology-level disappear/reappear. A real
   topology-disappear/reappear fixture is **not** modeled (live hotplug
   architecture excluded by decision); this remains an explicit limitation.

5. **Suspend/resume reconciliation audit** — `suspend`/`resume` are explicit
   IPC/CLI operations and are covered by deterministic tests
   (`ipc_suspend_and_resume_persist_state`, `ipc_resume_forces_reapply_and_clears_monitor_backoff`);
   there is **no** OS-level sleep/wake (logind/udev) wiring — that remains an
   explicit limitation and an out-of-scope-by-decision item.

## 7. Limitations / not tested (explicit)

- No real hardware write to `/sys/class/backlight/<dev>/brightness` this session
  (would require root + a real panel).
- No real `ddcutil` invocation this session (would require a DDC-capable
  monitor + I2C bus).
- No real suspend/resume cycle was scripted (OS-level sleep/wake / logind is
  not wired; manual IPC suspend/resume is).
- No real DRM hotplug event.
- Cross-backend physical dedup exists for the *discovery/config-generation*
  path via the symlink-convergence fixture. The runtime apply path still does
  not dedup cross-backend (a user config with both DDC and ddcci blocks can
  still double-write; documented limitation / Phase‑9 candidate).
- Explicit-`sysfs_path` resolution is validated, but the write is not exercised
  through a symlinked class entry.
- `apply_sysfs_fallback` chain remains dormant and is **not** wired to any
  production caller; it is documented in Part A section 4/6 and unchanged.

## 8. Independent-review reconciliation

A separately-spawned review subagent re-read the current dirty tree and re-ran
targeted suites (`backends::backlight` 21, `backends::ddc` 5,
`discovery::tests` 6 pre-uplift). It confirmed: explicit pinning is strongly
covered; alphabetically-first selection is not used; the realistic symlink
fixture exercises canonicalized convergence. It also confirmed three gaps —
mixed-discovery run, nonzero-exit isolation, and explicit-pin symlink-write
waiver — of which the first two were closed this phase with the two new tests;
The third remains documented; the cross-backend-dedup case is now **closed by
OWNER DECISION** (DDC primary + alias suppression at discovery/config
generation), with the regression test above as fresh evidence.

## 9. Remaining out-of-scope (not done, by decision)

- Live udev/DRM hotplug architecture.
- Phase 9 work.

## 10. Final `git status --short` and `git diff --stat`

See the exact output captured in the verification run above (recorded inline
during the session). The current tree is intentionally dirty with the Phase 1–8
working set; all verification was run against this exact final tree.

---

*End of Phase 8 Part B record.*
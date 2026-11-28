# SunReactor Phase 8 — Part A: Owner Brief

**Date:** 2026-08-22
**Branch:** `main` @ `5bfc08d3f403b5c9b1b0ca60fb8eb6654fc2c55f`
**Tree state:** intentionally dirty (pre-existing Phase 1–7 changes + Phase 8)
**Status:** Implementation complete; release-qualification toolchain green; Part B record follows.

---

## 1. What were we solving?

Phase 8 asked a single, sharp safety question about the backlight backend:

> When SunReactor must target a `/sys/class/backlight/<device>` interface, how
> does it *prove* which physical panel a selector names — without ever guessing?

The historical design had a fail-open hole: a connector-only (or even un-pinned)
backlight selector was accepted and fell back to scanning `/sys/class/backlight`
and picking a device by **filename ordering / "first device"**. That can target
the wrong panel when multiple backlight interfaces exist (internal eDP panel +
DDC/DDCCI devices + platform/firmware entries coexist on many laptops).

We also used Phase 8 as an **audit gate** for the mixed-hardware runtime paths:
mixed internal+external monitors, DDC duplicate detection, backend failure
isolation, monitor disappearance/reappearance, and suspend/resume
reconciliation.

---

## 2. Why important?

1. **Physical identity is not filename identity.** The kernel explicitly warns
   that multiple `/sys/class/backlight/<dev>` entries can reference the same
   panel. Picking "the first one" is unsafe and unreproducible.
2. **Config cannot detect the mistake.** A generated `config.toml` that says
   `connector = "card1-DP-1"` but resolves to `acpi_video0` contains an
   ambiguity the user cannot see until the wrong panel dims.
3. **Failure isolation and reconciliation are correctness, not polish.**
   A single unhealthy backend (hanging DDC I2C) must not take down the healthy
   internal panel; a monitor that disappears at suspend and reappears at resume
   must not be repeated-written or left stale.

---

## 3. Current approach (Option 1 — owner-approved, strict)

Fail-closed **authoritative topology resolution** for legacy connector-only
backlight configs:

- **Explicit `sysfs_path` pin wins** and ignores all other selector fields.
  Discovery always writes an explicit `sysfs_path`, so real users never lose a
  panel.
- **Legacy connector-only form resolves ONLY through the kernel/driver
  `ddcci_backlight` symlink** (`/sys/class/drm/<connector>/ddcci_backlight`
  → hardware device) **converging** with the backlight class entry's `device`
  symlink (`/sys/class/backlight/<dev>/device` → the same hardware device).
- Exactly **one proven mapping** → resolve + surface a **deprecation diagnostic**
  ("pin an explicit `sysfs_path`").
- **Zero** proven mappings → `fail closed`.
- **Multiple/ambiguous** proven candidates → `fail closed`.
- **Never** lexicographic/first-device/name-similarity/GPU-vendor/type-only
  selection. The kernel `type` preference (firmware > platform > raw) is
  explicitly NOT used as a provenance source.
- **Never** auto-rewrites the user's persistent configuration.
- Direct `sysfs` fallback chain remains **dormant**: production apply (the only
  caller) uses `brightnessctl`; `apply_sysfs_fallback()` has **no
  production-reachable caller**. It is documented here and left unchanged in
  this phase, per your instruction.

---

## 4. Owner Success Questions → Evidence

| Question | Evidence |
|---|---|
| 1. Internal vs external DDC | Discovery/apply are **backend-disjoint**: backlight targets reject DDC selectors (`rejects_ddc_selectors_on_backlight`); DDC targets use serial/bus and reject backlight fields. |
| 2. One monitor, multiple backends | `test_mixed_hardware_failure` runs healthy backlight + broken DDC + missing config in one tick; healthy succeeds, broken DDC fails+backs off, missing config is a clean skip. |
| 3. Multiple `/sys/class/backlight` = same panel? | Kernel warns they *can* be; SunReactor never manufactures identity. Only an explicit pin or a proven ddcci convergence counts (test `legacy_two_device_fixture_fails_closed_instead_of_sorting_names`). |
| 4. Safe interface choice | Explicit `sysfs_path` pin; else authoritative ddcci convergence; else fail closed. No first-device fallback exists. |
| 5. Failure isolation | Phase 2 dispatch is concurrent with per-monitor `record_apply_failure` + **per-backend backoff**; a hanging DDC does not block the backlight thread (`test_mixed_hardware_failure`). |
| 6. brightnessctl/ddcutil absence | Backends return structured `BackendError::CommandFailed/NotFound`; apply records it per-monitor and backs off; no global crash. |
| 7. Suspend/resume recovery | IPC/deterministic resync only: resume triggers forced reapply + clears monitor backoff (`ipc_resume_forces_reapply_and_clears_monitor_backoff`). **No OS-level sleep/wake (logind/udev) wiring.** |
| 8. Hotplug remainder | No live-hotplug architecture added (explicitly out of scope without OWNER DECISION). Periodic reassert + fail-closed resolution cover the non-live case; **no topology disappear/reappear test exists** (limitation in Part B §7). |

---

## 5. Evidence

**Test counts (final tree, `cargo +stable`):**

- All-features workspace: **188 lib + 6 ctl = 194 passed** (incl. two new
  Phase-8-uplift tests below and the DDC-primary alias regression).
- No-default-features: **176 + 6 = 182 passed**.
- Release toolchain `cargo +1.97.1` all-features: **194 passed**.
- `cargo check --workspace --all-targets --all-features`: exit 0 (9 pre-existing
  dead-code warnings on the dormant `apply_sysfs_fallback` chain — untouched).
- `cargo +stable fmt --all --check`: exit 0.
- `git diff --check`: clean.

New regression tests this phase (`src/backends/backlight.rs`):
- `never_selects_backlight_by_alphabetical_first_name_without_evidence`
- `picks_proven_device_despite_alphabetically_first_unproven_candidate`
- `realistic_sysfs_devices_symlink_chain_is_followed_not_plain_dirs`
  (`/sys/class/backlight/<dev>/device -> /sys/devices/pci.../drm/...` real-style
  chain, not plain temp dirs)

New Phase-8 discovery-uplift tests (`src/discovery/tests.rs`, added after the
independent reviewer corroborated two coverage gaps):
- `mixed_internal_and_external_monitors_in_single_discovery_run` — one
  discovery run sees both an external DDC monitor and an internal backlight;
  both appear in the report and generated config (viable targets = 2).
- `backend_failure_isolation_keeps_sysfs_probe_when_brightnessctl_errors` —
  `brightnessctl` present but exiting nonzero (malformed/authorization output)
  does not block the independent sysfs fallback probe.

The **real-turn verification matrix** (exact commands and outputs) is in
Part B.

---

## 6. Unknowns / limitations

- **No live hardware exercised** this phase (host machine not used as a
  display-control test bed; no DDC/DDCCI monitor attached in this environment).
- **DDC duplicate detection**: the engine/concurrency tests use a fake DDC
  runner; no real I2C contention was exercised.
- **Suspend/resume**: `suspend`/`resume` are explicit IPC/CLI operations.
  A real system sleep/wake cycle (logind/udev **and automatic reconciliation
  after OS suspend**) is **not** wired and **not** live-tested; deterministic
  resync tests cover the manual path.
- **Physical dedup across discovery paths**: no test yet models a single
  physical monitor appearing both as an external DDC display and as a
  ddcci-created internal backlight in the same discovery run — **now closed by
  the DDC-primary regression** (see section 8, item 1).
- **Explicit-pin symlink traversal during write**: the realistic symlink fixture
  covers legacy connector resolution. Explicit `sysfs_path` pins resolve, but
  the write path (brightnessctl invocation) is not exercised through a symlinked
  class entry.
- **`apply_sysfs_fallback` remains dead code** with no production caller; its
  existence is vestigial from the sysfs-first baseline (documented, not
  changed, per instruction).

---

## 7. User-visible change

- Anyone with a **legacy connector-only** backlight target on a DDCCI-capable
  machine now gets a **deprecation warning** in the apply diagnostic and the
  backend resolves only to the ddcci-proven panel.
- Anyone with a non-DDCCI connector-only config (e.g. ordinary `eDP-1` internal
  panel) now **fails closed** instead of silently dimming the wrong device.
  **This is intentional**: the old "first device" behavior was unsafe and
  unreproducible.
- Behavior for REAL users using discovery-written configs (which always emit
  explicit `sysfs_path`) is **unchanged**.

---

## 8. OWNER DECISIONS

1. **DDC-primary / ddcci-alias policy — APPROVED, implemented (2026-08-22).**
   SunReactor now uses DDC as the primary control backend for an external
   monitor when it has authoritative evidence the same physical panel is also
   exposed through `ddcci-driver-linux`:
   - a viable DDC monitor keeps its DDC target;
   - a backlight device **proven** (via convergent canonical `ddcci_backlight`
     and `device` symlinks in `/sys`) to sit on the same physical panel as a
     viable DDC monitor is treated as an alias and **not** auto-generated as a
     second enabled backlight target;
   - if DDC is unavailable or unqualified, the proven ddcci backlight remains
     usable as a fallback target;
   - names, ordering, and `type = "raw"` alone are **never** treated as
     evidence;
   - existing user config is **never rewritten** automatically.
   Evidence: deterministic regression
   `discovery::tests::ddc_primary_suppresses_authoritative_ddcci_alias_from_generated_config`
   (one physical monitor -> exactly one enabled DDC target; no second
   `sysfs_path` backlight block).

2. **Legacy-connector-only users** — confirmed fail-closed for non-DDCCI
   connector-only configs is the acceptable documented behavior change.

3. **Hotplug** — live udev/DRM hotplug remains **out of scope** by explicit
   decision; it is a Phase 9 candidate only.

4. **Direct-sysfs fallback** — remains dormant/unchanged (confirmed).

5. **Release/merge decision (pending)**: The dirty tree (45 modified files) is
   the working state accumulated across Phases 1–7. Do you want (a) keep the
   tree dirty and continue Phase 9, or (b) stage/commit the Phase 1–8 work in
   an attributable sequence with provenance? (Neither resets; both preserve
   all changes.)

---

## 9. Recommended next step

**Phase 8 is closed. Do not begin Phase 9.**

- Implementation ✓
- Verification matrix ✓
- Regression coverage ✓
- Audit evidence ✓

The remaining formal work is purely administrative: decide the disposition of
the dirty tree, optionally resolve the OWNER DECISION items above, and only
then start Phase 9 planning.
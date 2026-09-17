# SunReactor Linux Distribution & Compatibility Specification

**Effective Date:** September 10, 2026  
**Status:** Release-Qualified Baseline (Linux Contract Frozen for Release 0.1.0)

---

## 1. Support Envelope & Architecture

SunReactor targets modern systemd-based Linux operating environments adhering to the XDG Base Directory Standard, Linux kernel DRM/KMS uevents, and standard hardware control backends:

```text
Target Environment:
  Architecture:         x86_64 (Tier 1 Certified), aarch64 (Tier 2 Compile-verified)
  C Library:            GNU C Library (glibc >= 2.34 for conservative release artifact)
  Init / Supervisor:    systemd user session (`systemd --user`, no linger required)
  Runtime Directory:    XDG_RUNTIME_DIR (/run/user/$UID)
  Display Recovery:     Linux DRM / KMS uevents via AF_NETLINK (unprivileged)
  Session Power:        systemd-logind via D-Bus (`org.freedesktop.login1`)
  External Monitors:    DDC/CI via ddcutil (>= 1.4.1 supported, >= 2.2.0 recommended)
  Internal Backlights:  sysfs (`/sys/class/backlight`) or brightnessctl (>= 0.5.1)
```

### Support Tiers (As of September 2026)

* **Tier A — Certified & Release-Blocking:**
  * **Arch Linux / CachyOS / EndeavourOS:** Rolling (glibc 2.41, systemd 257+, ddcutil 2.2.7, brightnessctl 0.5.1). Full hardware, physical multi-monitor DDC/CI, DRM netlink recovery, and interactive TUI validated.
  * **Ubuntu 26.04 LTS:** Current modern LTS (glibc 2.43, systemd 259+, ddcutil 2.2.5, brightnessctl 0.5.1). Full runtime execution, modern `--noconfig` isolation, and installer staging qualified.
  * **Ubuntu 24.04 LTS:** Current legacy LTS baseline (glibc 2.39, systemd 255, ddcutil 1.4.1, brightnessctl 0.5.1). Legacy `--noconfig` fallback qualified; installer staging verified.
  * **Debian 13 (Trixie):** Current stable release (glibc 2.41, systemd 257+, ddcutil 2.2.0, brightnessctl 0.5.1). Clean artifact execution, user packaging, and runtime testing verified.
  * **Fedora 44:** Current primary Fedora release (glibc 2.43, systemd 259+, ddcutil 2.2.1, brightnessctl 0.5.1). Container artifact runtime and installer staging qualified.
  * **Fedora 43:** Current maintained previous Fedora release (glibc 2.42, systemd 258+, ddcutil 2.2.1, brightnessctl 0.5.1). Container artifact runtime qualified.
  * **openSUSE Tumbleweed:** Rolling (glibc 2.41, systemd 257+, ddcutil 2.2.7, brightnessctl 0.5.1). Clean execution and installer idempotency qualified.

* **Tier B — Compatible Derivatives & Extended LTS:**
  * **Ubuntu 22.04 LTS:** Upstream-supported extended LTS (glibc 2.35, systemd 249, ddcutil 1.2.2). The conservative release binary (`GLIBC_2.34`) launches and executes cleanly. Source build and CLI/daemon verified.
  * **Linux Mint 22.x & Pop!_OS 24.04:** Compatible derivatives inheriting Tier-A Ubuntu userspace.
  * **Nobara Linux:** Compatible derivative inheriting Tier-A Fedora userspace.

* **Tier C — Best-Effort / Unqualified:**
  * **non-systemd Linux (Alpine, Void, Devuan, Gentoo/OpenRC):** The core calculation engine, CLI, and run-once modes function, but automatic background service supervision and logind power resume require systemd.
  * **musl-based Linux (Alpine):** Source compilation succeeds; prebuilt GNU release artifacts require glibc.
  * **Immutable Desktops (Fedora Silverblue, Bazzite, SteamOS):** User-local binary installation (`~/.local/bin`) functions without root. Direct DDC hardware access requires host-side `i2c-dev` module loading and `i2c` group or `uaccess` rules.
  * **WSL (Windows Subsystem for Linux):** WSL does not expose physical display I2C buses (`/dev/i2c-*`) or native DRM connector hotplug events. WSL is unsupported for physical hardware brightness automation; the native Windows port is the designated solution.

---

## 2. Current Distribution Matrix (Evidence Audit)

| Distribution | Release Status | glibc | systemd | ddcutil | brightnessctl | Artifact Runtime | Installer Staging | Evidence Level |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :--- |
| **Arch / CachyOS** | Current Rolling | 2.41 | 257+ | 2.2.7 | 0.5.1 | OK | OK | **Physically Verified** (Hardware DDC + DRM) |
| **Ubuntu 26.04 LTS** | Current Stable LTS | 2.43 | 259+ | 2.2.5 | 0.5.1 | OK | OK | **Container Verified** (Runtime + Installer) |
| **Ubuntu 24.04 LTS** | Current Legacy LTS | 2.39 | 255 | 1.4.1 | 0.5.1 | OK | OK | **Container Verified** (Runtime + Installer) |
| **Debian 13 (Trixie)** | Current Stable | 2.41 | 257 | 2.2.0 | 0.5.1 | OK | OK | **Container Verified** (Runtime + Installer) |
| **Fedora 44** | Current Stable | 2.43 | 259 | 2.2.1 | 0.5.1 | OK | OK | **Container Verified** (Runtime + Installer) |
| **Fedora 43** | Current Supported | 2.42 | 258 | 2.2.1 | 0.5.1 | OK | OK | **Container Verified** (Runtime + Installer) |
| **openSUSE Tumbleweed** | Current Rolling | 2.41 | 257 | 2.2.7 | 0.5.1 | OK | OK | **Container Verified** (Runtime + Installer) |
| **Ubuntu 22.04 LTS** | Extended LTS | 2.35 | 249 | 1.2.2 | 0.5.1 | OK | OK | **Container & Source Verified** |
| **Alpine Linux** | Current Stable | musl | OpenRC | 2.2.0 | 0.5.1 | N/A (musl) | Manual | **Source / Spike Verified** |

*Note on Evidence Classification:* Full systemd user session lifecycle (PAM login session, logind seat creation, live socket activation) has been physically verified on Arch/CachyOS. On containerized test environments, installer staging, service unit syntax, and binary runtime have been verified via container execution; full-system VM/host lifecycle is marked as Architecture Certified.

---

## 3. Desktop & Session Matrix

SunReactor's core display lifecycle engine and daemon loop are desktop-environment agnostic. They do not depend on KDE, GNOME, or any specific display manager.

| Desktop Environment | Display Server | DRM Recovery | logind Resume | Idle Dimming Support | Qualification Level |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **KDE Plasma 6** | Wayland | Verified (AF_NETLINK) | Verified (zbus logind) | Supplemental (ext-idle-notify) | **Physically Verified** (Dual-monitor DDC) |
| **GNOME 46 / 47 / 48** | Wayland | Supported (AF_NETLINK) | Supported (zbus logind) | Supplemental (ext-idle-notify) | **Architecture Certified** |
| **Hyprland / Sway** | Wayland (wlroots) | Supported (AF_NETLINK) | Supported (zbus logind) | Supplemental (ext-idle-notify) | **Architecture Certified** |
| **XFCE / Cinnamon** | X11 | Supported (AF_NETLINK) | Supported (zbus logind) | Disabled (fail-soft) | **Architecture Certified** |
| **Headless / SSH CLI**| None | Supported (AF_NETLINK) | Supported (zbus logind) | Disabled (fail-soft) | **Verified** (`sunreactorctl`, IPC status, config) |

**Desktop Independence Invariant:**
- The daemon does NOT require `WAYLAND_DISPLAY` or `DISPLAY` to start, listen on IPC, or automate display brightness.
- If Wayland idle protocols (`ext-idle-notify-v1`) are absent or rejected by the compositor, SunReactor logs a diagnostic note and continues operating normal scheduled solar policy and DRM event recovery.

---

## 4. Conservative Release Artifact & ABI Baseline

### Selected GNU/Linux Release Artifact

SunReactor uses a conservative build environment (Ubuntu 22.04 LTS with glibc 2.35) to produce the official `x86_64-unknown-linux-gnu` release artifact.

```text
Target Triple:              x86_64-unknown-linux-gnu
Build Environment:          Ubuntu 22.04 LTS (glibc 2.35)
Compiler / Rustc:           1.98.1 / rustc pinned toolchain
Dynamic Interpreter:        /lib64/ld-linux-x86-64.so.2
Needed Shared Libraries:    libc.so.6, libm.so.6, libgcc_s.so.1 (0 external C libraries)
Highest Required GLIBC:     GLIBC_2.34
CPU ISA Baseline:           Generic x86-64 (baseline SSE/SSE2; no target-cpu=native)

Artifact Checksums:
  sunreactord:              6fd908db2a39e3111b16cd604a10f95da04f5737439817b7310dd6606f17feb8
  sunreactorctl:            a3dd341c227fed6b5be70e2a8280bcf49c5678adb9da07fad55cd39ef95de075
```

### Exact Artifact Execution Matrix

The EXACT release binary pair above has been empirically verified to execute cleanly across:
- **Ubuntu 22.04 LTS** (glibc 2.35)
- **Ubuntu 24.04 LTS** (glibc 2.39)
- **Ubuntu 26.04 LTS** (glibc 2.43)
- **Debian 13 (Trixie)** (glibc 2.41)
- **Fedora 43** (glibc 2.42)
- **Fedora 44** (glibc 2.43)
- **openSUSE Tumbleweed** (glibc 2.41)
- **Arch Linux / CachyOS** (glibc 2.41)

---

## 5. ddcutil Compatibility & Fallback Semantics

SunReactor executes `ddcutil` with explicit, bounded argument forms:

1. **Display Discovery:** `ddcutil [--noconfig] --terse detect`
2. **Capability Probe:** `ddcutil [--noconfig] --display <N> capabilities`
3. **Brightness Readback:** `ddcutil [--noconfig] --terse [--bus <B> | --sn <S> | --model <M>] getvcp 10`
4. **Brightness Apply:** `ddcutil [--noconfig] --noverify [--bus <B> | --sn <S> | --model <M>] setvcp 10 <val>`

### Version Landscape & Policy

| Distribution | Default Package | `--noconfig` Support | Invocation Mode |
| :--- | :---: | :---: | :--- |
| **Ubuntu 24.04 LTS** | 1.4.1 | No (unrecognized option) | Auto-detected legacy fallback (without `--noconfig`) |
| **Ubuntu 26.04 LTS** | 2.2.5 | Yes | Preferred invocation (with `--noconfig`) |
| **Debian 13** | 2.2.0 | Yes | Preferred invocation (with `--noconfig`) |
| **Fedora 43 / 44** | 2.2.1 | Yes | Preferred invocation (with `--noconfig`) |
| **openSUSE Tumbleweed**| 2.2.7 | Yes | Preferred invocation (with `--noconfig`) |
| **Arch Linux** | 2.2.7 | Yes | Preferred invocation (with `--noconfig`) |

### Strict Fallback Invariant

The legacy `--noconfig` fallback triggers **if and only if** `stderr` contains `Unknown option --noconfig` or `unrecognized option '--noconfig'`.  
Real hardware/system errors (such as `Permission denied`, `No /dev/i2c devices exist`, `Device or resource busy`, or `Unsupported VCP code`) **never** trigger fallback and are returned immediately to the policy engine.

---

## 6. systemd Version Baseline

All directives in `contrib/systemd/sunreactord.service` are supported by **systemd >= 244** (released November 2019):
- Sandboxing (`NoNewPrivileges`, `PrivateTmp`, `ProtectKernelLogs`, `MemoryDenyWriteExecute`, `RestrictNamespaces`, `SystemCallArchitectures=native`)
- Process isolation (`UMask=0077`, `WorkingDirectory=%h`)
- Lifecycle (`Restart=on-failure`, `TimeoutStopSec=20s`, `StartLimitIntervalSec=300`)

Every supported distribution (Ubuntu 22.04 with systemd 249 up to Fedora 44 with systemd 259) fully supports this service unit without warnings.

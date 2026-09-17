# SunReactor Windows Architecture & Port Documentation (Milestones W1, W2 & W2.1)

## Executive Overview

SunReactor Milestone W1 established the **native Windows platform foundation**, cross-platform compilation targets, Windows Known Folders, atomic persistence (`ReplaceFileW`), secure per-user Named Pipe IPC, single-instance execution ownership, console shutdown integration, and Linux regression isolation.

Milestone W2 and W2.1 established the **native Windows display topology, physical devnode correlation, and stable identity layer**. This enables SunReactor to accurately determine connected displays, map desktop sources to physical hardware, distinguish physical from virtual/internal/indirect displays, derive deterministic 128-bit collision-resistant identifiers, and expose discovery information without performing any brightness mutations.

---

## Windows Display Architecture & Five-Layer Correlation

SunReactor strictly separates display layers rather than conflating desktop views with physical monitors:

```text
Layer 1: Windows Desktop SOURCE (GDI Surface / Virtual Desktop)
  - Driven by GPU display engine.
  - Ephemeral coordinates: (x, y, w, h), primary flag.
  - GDI Device Name: \\.\DISPLAY1, \\.\DISPLAY2.
  - Transient Invariant: Monitor numbers and primary status change dynamically. Never persist as physical ID.

Layer 2: CCD PATH (Connecting and Configuring Displays)
  - Active display pipeline linking a desktop source to an output target.
  - Queried via QueryDisplayConfig (QDC_ONLY_ACTIVE_PATHS | QDC_VIRTUAL_MODE_AWARE).
  - Contains: Adapter LUID, Source ID, Target ID, active status, targetAvailable flag.
  - Invariant: (adapter LUID, target ID) are adapter-scoped coordinates, not physical monitor serials.

Layer 3: Display TARGET
  - Output connector on the graphics adapter.
  - Output technology: HDMI, DisplayPort External, Embedded DisplayPort, DVI, VGA, Indirect Wired, etc.
  - EDID Metadata: 3-letter manufacturer ID, product code ID, edidIdsValid flag.
  - Device Interface Path: monitorDevicePath (e.g. \\?\DISPLAY#DEL4198#...#{GUID_DEVINTERFACE_MONITOR}).
  - Invariant: monitorDevicePath is an opaque token for SetupAPI; applications must never parse its format.

Layer 4: PnP Monitor DEVICE (Device Node & Properties)
  - Correlated via SetupAPI: monitorDevicePath -> SetupDiOpenDeviceInterfaceW -> SP_DEVINFO_DATA.
  - Container ID (DEVPKEY_Device_ContainerId): Physical device container assigned by Windows PnP.
  - Device Instance ID (DEVPKEY_Device_InstanceId): PnP devnode instance path.
  - Hardware IDs (DEVPKEY_Device_HardwareIds): Model/class identifiers (e.g. MONITOR\DEL4198).
  - Invariant: SetupAPI is the sole supported route. Production code never parses device path strings.

Layer 5: PHYSICAL_MONITOR Handles (DXVA2 DDC/CI Bridge)
  - Enumerated from HMONITOR via GetPhysicalMonitorsFromHMONITOR.
  - Invariant: Ephemeral OS resources owned by PhysicalMonitorGuard and destroyed immediately via DestroyPhysicalMonitors. Never cached or persisted.
```

---

## Native API Inventory

| API | Subsystem | Purpose | Ownership & Lifetime |
| :--- | :--- | :--- | :--- |
| `GetDisplayConfigBufferSizes` | User32 / CCD | Query required buffer sizes for active paths and mode tables | Synchronous query |
| `QueryDisplayConfig` | User32 / CCD | Retrieve active CCD path and mode arrays | Synchronous; bounded retry on `ERROR_INSUFFICIENT_BUFFER` |
| `DisplayConfigGetDeviceInfo` | User32 / CCD | Query source GDI name (`DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME`) & target device name (`DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME`) | Synchronous; caller manages struct size and packet header |
| `EnumDisplayMonitors` | User32 / GDI | Enumerate logical desktop displays (`HMONITOR`) | Callback loop; stack-allocated vector |
| `GetMonitorInfoW` | User32 / GDI | Query display bounds, work area, and primary flag (`MONITORINFOEXW`) | Synchronous |
| `GetNumberOfPhysicalMonitorsFromHMONITOR` | Dxva2 | Query count of physical handles for an HMONITOR | Read-only observation |
| `GetPhysicalMonitorsFromHMONITOR` | Dxva2 | Acquire array of physical monitor handles for description inspection | **Transient**: Owned by `PhysicalMonitorGuard` |
| `DestroyPhysicalMonitors` | Dxva2 | Release array of physical monitor handles | Invoked in `Drop` of `PhysicalMonitorGuard` |
| `SetupDiCreateDeviceInfoList` | SetupAPI | Create device information set for monitor devnode | Owned by `DeviceInfoSetGuard`; destroyed on drop |
| `SetupDiOpenDeviceInterfaceW` | SetupAPI | Open monitor device interface from `monitorDevicePath` | Scoped to `DeviceInfoSetGuard` |
| `SetupDiGetDeviceInterfaceDetailW` | SetupAPI | Retrieve `SP_DEVINFO_DATA` for opened interface | Stack-allocated struct |
| `SetupDiGetDevicePropertyW` | SetupAPI | Retrieve `ContainerId`, `InstanceId`, and `HardwareIds` | Stack-allocated buffers; properly aligned |
| `SetupDiDestroyDeviceInfoList` | SetupAPI | Free device information set | Invoked in `Drop` of `DeviceInfoSetGuard` |

---

## Stable Identity Contract (Milestone W2.1)

### Precedence & Representation

SunReactor derives candidate monitor identifiers using explicit precedence:

1. **`IdentityQuality::Strong`** (`W3MutationEligibility::AutomaticEligible`):
   * **Condition**: Hardware Container ID is present, valid, and not a nil or generic/system container (`{00000000-0000-0000-ffff-ffffffffffff}`).
   * **Canonical Identity**: Preserves the **FULL 128-bit normalized GUID** (never truncated to 32 bits):
     `win-{mfg}-{product_code}-{full_normalized_guid}`
     (e.g. `win-del-4198-8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c`).
   * **Display Label**: Abbreviated presentation string for human UI (e.g. `win-del-4198-8c7ed206`).
   * **Semantic Caveat**: Strong Windows PnP device-container identity; expected to be stable when the driver/device reports or Windows derives a stable unique container identity; physical port-move behavior remains hardware/driver dependent until validated.
2. **`IdentityQuality::StableDevice`** (`W3MutationEligibility::ExplicitOnly`):
   * **Condition**: Container ID absent or generic, but PnP `DeviceInstanceId` obtained through SetupAPI and EDID are present.
   * **Canonical Identity**: Uses a **128-bit deterministic FNV-1a composite hash** (32 hex characters):
     `win-dev-{mfg}-{product_code}-{full_128bit_hash}`
     (e.g. `win-dev-del-4198-5fe949b345b594dfc6a843ca7a11a5e7`).
   * **Display Label**: Abbreviated presentation string (e.g. `win-dev-del-5fe949b3`).
   * **Mutation Policy**: Port independence is NOT assumed; eligible only under explicit user configuration.
3. **`IdentityQuality::PortBound`** (`W3MutationEligibility::Ineligible`):
   * **Condition**: Hardware and device instance identifiers absent; bound to adapter LUID and target ID.
   * **Format**: `win-port-{adapter_luid_high}_{adapter_luid_low}-{target_id}`.
   * **Mutation Policy**: Ineligible for automatic persistent mutation (fail-closed).
4. **`IdentityQuality::Virtual`** (`W3MutationEligibility::Ineligible`):
   * **Condition**: Display output technology is Indirect Virtual or Miracast.
   * **Format**: `win-vdisp-{target_id}`.
5. **`IdentityQuality::Ambiguous`** (`W3MutationEligibility::Ineligible`):
   * **Condition**: **Collision Detected**. Two or more active monitors produce identical candidate identifiers. Both are downgraded to `Ambiguous`.
   * **Mutation Policy**: Never automatically mutated.
6. **`IdentityQuality::Unknown`** (`W3MutationEligibility::Ineligible`):
   * **Condition**: Insufficient data to classify or identify display.

### Identity Quality & W3 Mutation Eligibility Table

| Quality Tier | Persistent? | W3 Automatic Mutation Eligible? | Reason |
| :--- | :--- | :--- | :--- |
| **Strong** | Yes | **Yes** (Subject to unique match) | Full 128-bit non-system Container ID anchored by PnP device container. |
| **StableDevice** | Yes (Qualified) | **No** (Explicit/manual only) | Supported SetupAPI devnode ID; port independence is not assumed. |
| **PortBound** | No (Transient) | **No** (Fail-closed) | Transient adapter/target coordinate; changes on GPU/cable moves. |
| **Ambiguous** | No | **No** (Fail-closed) | Collision detected; multiple physical targets share evidence. |
| **Virtual** | No | **No** (Fail-closed) | Remote/virtual display; DDC mutation forbidden. |
| **Unknown** | No | **No** (Fail-closed) | Incomplete or unverified topology evidence. |

---

## Display Classification (Milestone W2.1)

| Output Technology | Classification | W3 Mutation Tier | Notes |
| :--- | :--- | :--- | :--- |
| `INTERNAL`, `DISPLAYPORT_EMBEDDED`, `LVDS`, `UDI_EMBEDDED` | **InternalPanel** | Deferred to W3 | Laptop screen; routed to WMI/WinRT backend in W3 |
| `DISPLAYPORT_EXTERNAL`, `DISPLAYPORT_USB_TUNNEL`, `HDMI`, `DVI`, `HD15`, `UDI_EXTERNAL` | **ExternalMonitor** | Strong / ExplicitOnly | External desktop display; routed to DXVA2 in W3 |
| `INDIRECT_WIRED` | **IndirectDisplay** | Ineligible (Pending Probe) | USB docks/dongles (DisplayLink); not assumed virtual, W3 probes physical handles |
| `INDIRECT_VIRTUAL`, `MIRACAST` | **VirtualDisplay** | Ineligible | RDP sessions, virtual display drivers, wireless screens |
| `OTHER`, unrecognized codes | **Unknown** | Ineligible | Fail-safe classification |

---

## Candidate Config Generation Policy

`sunreactorctl discover` generates candidate `config.toml` snippets with strict fail-closed safety:
- **Strong identities**: Emitted as ready monitor configuration blocks using the **full canonical ID**.
- **StableDevice identities**: Emitted commented with an explicit warning: `# Qualified device-bound candidate (port independence unverified)`.
- **PortBound, Ambiguous, Virtual, IndirectDisplay, Unknown**: **NEVER** emitted as candidate monitor configs.

---

## W3 Input Contract

Milestone W3 can rely on the following guarantees:

```text
Configured Canonical ID (e.g. "win-del-4198-8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c")
        │
        ▼ capture_topology_snapshot()
Fresh W2.1 Topology Snapshot
        │
        ▼ Match unique monitor observation by canonical_id
        ├── If IdentityQuality == Ambiguous: FAIL-CLOSED (skip mutation, log warning)
        ├── If W3MutationEligibility == Ineligible: FAIL-CLOSED (skip mutation)
        └── If unique match & eligible:
                │
                ▼ Correlate GDI Device Name (szDevice <=> viewGdiDeviceName)
             HMONITOR
                │
                ▼ GetPhysicalMonitorsFromHMONITOR
             PHYSICAL_MONITOR Handle(s)
                │
                ▼ Execute Read / Write Brightness (Milestone W3)
                │
                ▼ Drop PhysicalMonitorGuard
             DestroyPhysicalMonitors (Immediate RAII Release)
```

W3 will never retain `HMONITOR` or `PHYSICAL_MONITOR` handles across topology changes or tick cycles.

---

# Milestone W3A — Native External Monitor Brightness Backend & Physical Qualification

W3A implements the first real hardware mutation capability on Windows: **native external monitor brightness control** using Microsoft's high-level Monitor Configuration APIs (`dxva2.dll`).

## High-Level API First Architecture

```text
canonical persistent identity
        ↓
fresh topology snapshot (capture_topology_snapshot)
        ↓
unique current monitor observation (IdentityQuality::Strong required)
        ↓
fresh HMONITOR (GDI szDevice correlation)
        ↓
fresh PHYSICAL_MONITOR resource (acquire_physical_monitors, RAII)
        ↓
GetMonitorCapabilities (verify MC_CAPS_BRIGHTNESS = 0x02)
        ↓
GetMonitorBrightness (read & validate native min < max, min <= cur <= max)
        ↓
percent_to_native (deterministic integer-safe nearest-half mapping)
        ↓
redundant write check (current_native == desired_native => NO WRITE)
        ↓
SetMonitorBrightness (native write, pre-validated range)
        ↓
optional readback / restoration
        ↓
DestroyPhysicalMonitors (immediate drop cleanup)
```

## Native API Contract

| API | Usage | Mutation? | Result Semantics |
| :--- | :--- | :--- | :--- |
| `GetMonitorCapabilities` | Query capabilities bitmask | **No** | Checks for `MC_CAPS_BRIGHTNESS` (0x02). Fails closed if missing. |
| `GetMonitorBrightness` | Read native brightness tuple | **No** | Reports `min`, `current`, `max`. Fails closed on nonsensical ranges. |
| `SetMonitorBrightness` | Apply new native brightness | **Yes** | Primary mutation call. Only called when `current_native != desired_native`. |
| `GetNumberOfPhysicalMonitorsFromHMONITOR` | Query physical monitor count | **No** | Validates 1-to-1 mapping. Fails closed if count == 0 or >1. |
| `GetPhysicalMonitorsFromHMONITOR` | Acquire transient physical handles | **No** | Wrapped in RAII `PhysicalMonitorGuard`. |
| `DestroyPhysicalMonitors` | Free physical handles | **No** | Executed automatically upon guard drop. Never leaks handles. |

## Mutation Safety Gate Matrix

| Condition | Automatic Mutation Allowed? | Reason |
| :--- | :--- | :--- |
| `Strong` + Unique Match + Count == 1 | **Yes** | Fully qualified hardware identity with unique physical handle mapping. |
| `StableDevice` | **No** (Fail closed) | Device-instance bound; port independence unverified. Manual/dev probe only. |
| `PortBound` | **No** (Fail closed) | Transient coordinate; volatile across cable moves. |
| `Ambiguous` | **No** (Fail closed) | Multiple active monitors share candidate identifiers. |
| `Virtual` | **No** (Fail closed) | Remote Desktop or driver loopback without physical DDC/CI controls. |
| `Unknown` | **No** (Fail closed) | Unresolved topology observation. |
| `IndirectDisplay` | **No** (Fail closed) | USB-C / DisplayLink docks; pending physical qualification. |
| `target_available == false` | **No** (Fail closed) | Transient disconnected / power-save state. |
| `active == false` | **No** (Fail closed) | Display disabled in Windows Settings. |
| Physical handles count == 0 | **No** (Fail closed) | No physical monitor attached to GDI endpoint. |
| Physical handles count > 1 | **No** (Fail closed) | Ambiguous mapping; cannot prove which handle belongs to which target. |
| Lacks `MC_CAPS_BRIGHTNESS` | **No** (Fail closed) | Driver / monitor does not support high-level brightness. |
| `GetMonitorBrightness` fails | **No** (Fail closed) | Native API error; fail safe. |
| Invalid native range (`min >= max`) | **No** (Fail closed) | Inverted or corrupt driver bounds. |

## Brightness Range & Conversion Model

Native brightness values are continuous integers defined by the monitor firmware (e.g. `0..100`, `0..10`, `10..110`, `20..80`, `0..255`), NOT percentages.

Conversion helpers in `src/platform/windows/brightness.rs`:
- `native_to_percent(min, current, max) -> u8`:
  $$pct = \min\left(100, \left\lfloor \frac{(current - min) \times 100 + (max - min) / 2}{max - min} \right\rfloor\right)$$
- `percent_to_native(min, max, percent) -> u32`:
  $$native = min + \left\lfloor \frac{percent \times (max - min) + 50}{100} \right\rfloor$$

Guarantees:
- 0% maps strictly to `min`
- 100% maps strictly to `max`
- Strictly monotonic across the entire range
- Zero risk of arithmetic overflow (64-bit intermediate calculations)
- Zero division by zero (guarded by `min < max` invariant)

## Multi-Physical-HMONITOR Handling

- **Count == 0**: Fails closed with `PhysicalMonitorUnavailable`.
- **Count == 1**: Safe to proceed with `guard.handles()[0].hPhysicalMonitor`.
- **Count > 1**: Fails closed with `PhysicalMonitorMappingAmbiguous`. Windows does not provide a proven, documented mechanism to correlate individual physical handles inside an `HMONITOR` to PnP devnodes. Writing to all handles is strictly forbidden to prevent misdirected writes.

## Controlled Hardware Test Protocol

Hardware mutation tests require explicit opt-in:
`SUNREACTOR_ALLOW_HARDWARE_TESTS=1 sunreactorctl test-brightness --monitor-id <CANONICAL_ID>`

Protocol:
1. Capture fresh topology & resolve unique `Strong` identity.
2. Acquire transient physical handle & read original native value.
3. Compute small reversible delta ($+5\%$ if current $\le 90\%$, else $-5\%$).
4. Apply test target native value.
5. Readback verify test target was applied.
6. Restore original native value.
7. Readback verify original value was restored.
8. Release physical handles immediately via RAII.

## Internal Panel Status

Internal laptop panel control (via WMI `WmiMonitorBrightness` or WinRT) is **deferred to Milestone W3B**. External backend strictly rejects internal panels.

## Milestone W4 Input

W4 event-driven recovery (handling `WM_DISPLAYCHANGE`, session reconnect, and power-resume events) can rely on:
- All physical handles are ephemeral and never cached.
- Recapturing topology via `capture_topology_snapshot()` always discovers current hardware status.
- External monitor brightness backend fails closed cleanly on transient disconnects (`target_available = false`).


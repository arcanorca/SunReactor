pub mod brightness;
pub mod console;
pub mod display;
pub mod event_sources;
pub mod gdi;
pub mod paths;
pub mod persistence;
pub mod physical_monitors;
pub mod pipe;
pub mod pnp;
pub mod security;
pub mod topology;

pub use brightness::{
    apply_external_brightness_with_api, native_to_percent, percent_to_native,
    probe_external_capability_with_api, read_external_brightness_with_api,
    resolve_and_acquire_external_monitor, test_physical_monitor_brightness, BrightnessRangeError,
    ExternalBrightnessCapability, MonitorConfigApi, NativeBrightnessRange,
    PhysicalQualificationReport, Win32MonitorConfigApi, WindowsExternalApplyResult,
    WindowsExternalBrightnessError,
};
pub use console::register_console_shutdown_handler;
pub use display::{
    classify_output_technology, decode_edid_manufacturer, query_active_ccd_paths, CcdPathInfo,
    CcdSourceInfo, CcdTargetInfo, WindowsDisplayError, WindowsDisplayKind,
};
pub use event_sources::{DisplayEventSources, DisplayRecoveryReason, DisplayRecoveryRequest};
pub use gdi::{enumerate_gdi_monitors, GdiMonitorInfo};
pub use paths as win_paths;
pub use persistence::atomic_write_file;
pub use physical_monitors::{
    query_physical_monitors, PhysicalMonitorGuard, PhysicalMonitorObservation,
};
pub use pipe::{default_windows_pipe_name, BoundControlSocket, ControlSocket, NamedPipeStream};
pub use pnp::{
    format_guid, is_generic_or_system_container, query_pnp_device_info, DeviceInfoSetGuard,
    PnpDeviceInfo,
};
pub use topology::{
    capture_topology_snapshot, derive_stable_identity, IdentityQuality, W3MutationEligibility,
    WindowsMonitorObservation, WindowsTopologySnapshot,
};

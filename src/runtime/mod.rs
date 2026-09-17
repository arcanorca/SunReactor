pub(super) mod capabilities;
#[cfg(target_os = "linux")]
pub(crate) mod events;
pub mod fade;
pub mod idle;
pub(super) mod ipc;
pub(super) mod r#loop;
pub mod orchestrator;
pub(super) mod runtime_config;
pub(super) mod status;
pub(super) mod tick;
pub(crate) mod topology;
pub mod wake;
pub(super) mod weather_refresh;

pub use orchestrator::{DaemonRuntime, IpcOutcome, RuntimeError, TickReport};
pub use weather_refresh::WeatherRefreshState;

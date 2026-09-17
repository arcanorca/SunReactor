//! Platform abstraction boundary.
//!
//! Provides platform-specific integrations (paths, IPC transport, display and session events)
//! behind a clean boundary so core policy and daemon orchestration remain portable.

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(target_os = "windows")]
pub use windows::*;

pub use crate::paths::{AppPaths, IpcEndpoint};

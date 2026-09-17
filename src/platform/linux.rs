//! Linux platform implementation.
//!
//! Consolidates Linux-specific runtime resources:
//! - XDG / UID runtime paths
//! - Unix Domain Socket IPC transport
//! - DRM / udev / logind event sources

pub use crate::ipc::socket::{BoundControlSocket, ControlSocket};
pub use crate::paths;
pub use crate::runtime::events::{
    DisplayEventSources, DisplayRecoveryReason, DisplayRecoveryRequest,
};

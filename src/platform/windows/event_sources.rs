//! Windows display & power event source seam for W1.
//!
//! Windows display, session, and power recovery mechanisms (WM_DISPLAYCHANGE,
//! RegisterPowerSettingNotification, WTSRegisterSessionNotification) are
//! deliberately deferred to Phase W4.
//!
//! In W1, this stub provides the portable event boundary required by the daemon
//! runtime, honestly reporting that hardware display lifecycle events are unavailable.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayRecoveryReason {
    DrmHotplug,
    DrmConnectorChange,
    Resume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayRecoveryRequest {
    pub reason: DisplayRecoveryReason,
}

#[derive(Debug)]
pub struct DisplayEventSources;

impl DisplayEventSources {
    #[must_use]
    pub fn spawn() -> Self {
        tracing::info!(
            platform = "windows",
            display_recovery = "unavailable_in_w1",
            "display_lifecycle_events_deferred_to_w4"
        );
        Self
    }

    #[must_use]
    pub fn drain(&self) -> Option<DisplayRecoveryRequest> {
        None
    }

    #[must_use]
    pub fn take_pending(&self) -> Option<DisplayRecoveryRequest> {
        None
    }
}

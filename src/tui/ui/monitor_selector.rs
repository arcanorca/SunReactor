//! One friendly-name policy for monitors, shared by every workspace.

use crate::tui::Model;

/// Resolves one human-readable monitor identity from configured metadata.
/// Logical IDs stay authoritative for state reconciliation, but are only a
/// fallback when a configured selector has no useful display metadata.
#[must_use]
pub(crate) fn monitor_display_name(app: &Model, logical_id: &str) -> String {
    let configured = app
        .config
        .monitors
        .iter()
        .find(|monitor| monitor.logical_id == logical_id);

    configured
        .and_then(|monitor| {
            monitor
                .selector
                .model
                .as_deref()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .or_else(|| {
                    monitor
                        .selector
                        .connector
                        .as_deref()
                        .map(str::trim)
                        .filter(|name| !name.is_empty())
                })
        })
        .map(normalize_monitor_model_name)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| {
            if logical_id.trim().is_empty() {
                String::from("No configured monitor")
            } else {
                logical_id.to_owned()
            }
        })
}

fn normalize_monitor_model_name(model: &str) -> String {
    let model = model.trim();
    // DDC often exposes Lenovo displays as `LEN <model>`, which is a vendor
    // code rather than the product name users recognize.
    if let Some(rest) = model.strip_prefix("LEN ") {
        return format!("Lenovo {rest}");
    }
    model.to_owned()
}

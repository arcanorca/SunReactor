mod detail;
mod layout;
mod state;

pub(super) use detail::{monitor_limits, valid_applied_percent};
pub(super) use layout::{display_name, monitor_ids, render_monitors};
pub(super) use state::render_monitor_workspace_state;

#[cfg(test)]
mod tests {
    include!("tests.rs");
}

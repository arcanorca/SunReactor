#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_arguments,
    clippy::struct_excessive_bools,
    clippy::module_name_repetitions,
    clippy::match_same_arms,
    clippy::ref_option,
    clippy::return_self_not_must_use,
    clippy::must_use_candidate,
    clippy::doc_markdown
)]

pub mod apply;
pub mod backends;
pub mod config;
pub mod ddcutil;
pub mod discovery;
pub mod doctor;
pub mod ipc;
pub mod paths;
pub mod policy;
mod process;
pub mod runtime;
pub mod solar;
pub mod state;
#[cfg(feature = "tui")]
pub mod tui;
pub mod weather;

pub const PRODUCT_NAME: &str = "sunreactor";

pub const DAEMON_BINARY: &str = "sunreactord";
pub const CLI_BINARY: &str = "sunreactorctl";

pub const SYSTEMD_USER_UNIT: &str = "sunreactord.service";

#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[must_use]
pub fn daemon_help() -> String {
    format!(
        "\
{name} daemon

Usage:
  {bin} [OPTIONS]

Options:
  -h, --help       Show this help text
  -V, --version    Show version information
      --once       Run a single tick and exit

Runtime:
  Config file: {config}
  State file:  {state}
  Socket:      {socket}
  Systemd:     {unit}

Control:
  Listens on a local Unix socket only
  Removes a stale socket when it is safe to do so
",
        name = PRODUCT_NAME,
        bin = DAEMON_BINARY,
        config = paths::CONFIG_FILE,
        state = paths::STATE_FILE,
        socket = paths::SOCKET_PATH_TEMPLATE,
        unit = SYSTEMD_USER_UNIT,
    )
}

#[must_use]
pub fn cli_help() -> String {
    format!(
        "\
{name} CLI

Usage:
  {bin} [OPTIONS] [COMMAND]

Commands:
  status                    Show daemon status over the local control socket
  suspend [--minutes <N>]   Suspend device writes until resume or for N minutes
  resume                    Return to automatic mode and clear overrides
  set <id> <pct>            Set a per-monitor manual override
  set --global <pct>        Set a global manual override
  clear-override            Clear all manual overrides
  clear-override --monitor-id <id>
                            Clear one per-monitor manual override
  clear-override --global   Clear the global manual override only
  reload-config             Reload config inside the running daemon
  ping                      Check whether the daemon socket is alive
  run-once [--force]        Trigger one immediate daemon tick
  discover                  Probe brightness-capable devices locally (read-only)
  discover --apply          Add viable devices transactionally and reload
  config init               Write the default config template
  config validate           Parse and validate the config file
  tui                       Launch the interactive terminal interface

Options:
  -h, --help       Show this help text
  -V, --version    Show version information

IPC:
  Socket: {socket}
  Protocol: one JSON request and one JSON response per connection

Discovery:
  {bin} discover             Human-readable device table
  {bin} discover --json      Machine-readable JSON
  {bin} discover --apply     Deduplicated, atomic config update with rollback
",
        name = PRODUCT_NAME,
        bin = CLI_BINARY,
        socket = paths::SOCKET_PATH_TEMPLATE,
    )
}

//! `sunreactorctl doctor --report`: a self-contained, privacy-redacted
//! Markdown bundle suitable for pasting into a GitHub issue.
//!
//! Everything here is read-only: it never writes hardware, config, or state.
//! Redaction is applied to every piece of collected text before it is placed
//! in the report, so no section can bypass it.

use std::fmt::Write as _;
use std::fs;

use crate::discovery::DiscoveryReport;
use crate::doctor::{CheckStatus, DoctorReport};
use crate::ipc::{ControlSocket, Request, RequestEnvelope, Response};

const REDACTED: &str = "<redacted>";

/// Substrings (lowercase) of keys whose values are always secret.
const SECRET_KEY_PARTS: &[&str] = &[
    "api_key",
    "apikey",
    "token",
    "secret",
    "password",
    "passwd",
    "credential",
    "authorization",
    "serial",
];
const COORDINATE_KEYS: &[&str] = &["latitude", "longitude", "lat", "lon", "lng"];
/// Keys whose value reveals where the user lives or who they are.
const IDENTIFYING_KEYS: &[&str] = &["city", "location_city", "user", "username"];
/// URL query parameter names whose value is dropped.
const SECRET_QUERY_KEYS: &[&str] = &[
    "key",
    "apikey",
    "api_key",
    "appid",
    "token",
    "access_token",
    "secret",
    "password",
    "auth",
    "signature",
    "sig",
];

/// Who the report is being generated for, so their identity can be scrubbed.
#[derive(Debug, Clone, Default)]
pub struct RedactionContext {
    pub home: Option<String>,
    pub username: Option<String>,
}

impl RedactionContext {
    #[must_use]
    pub fn from_environment() -> Self {
        let home = std::env::var("HOME").ok().filter(|home| home.len() > 1);
        let username = std::env::var("USER")
            .or_else(|_| std::env::var("LOGNAME"))
            .ok()
            .filter(|user| !user.is_empty());
        Self { home, username }
    }
}

/// Redact free-form text: home paths, user name, URL credentials and secret
/// query parameters. Idempotent.
#[must_use]
pub fn redact_text(text: &str, ctx: &RedactionContext) -> String {
    let mut out = text.to_owned();
    if let Some(home) = ctx.home.as_deref().filter(|home| home.len() > 1) {
        out = replace_path_prefix(&out, home.trim_end_matches('/'), "~");
    }
    out = redact_home_dirs(&out);
    out = redact_url_credentials(&out);
    out = redact_secret_query_params(&out);
    if let Some(user) = ctx.username.as_deref().filter(|user| user.len() >= 3) {
        out = replace_word(&out, user, "<user>");
    }
    out
}

/// Redact a config file (TOML) or JSON document line by line, then apply
/// [`redact_text`] to the result.
#[must_use]
pub fn redact_config(text: &str, ctx: &RedactionContext) -> String {
    let normalized = text
        .trim_start()
        .starts_with('{')
        .then(|| serde_json::from_str::<serde_json::Value>(text).ok())
        .flatten()
        .and_then(|val| serde_json::to_string_pretty(&val).ok());
    let source = normalized.as_deref().unwrap_or(text);
    let lines: Vec<String> = source.lines().map(redact_key_value_line).collect();
    redact_text(&lines.join("\n"), ctx)
}

fn redact_key_value_line(line: &str) -> String {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') || trimmed.starts_with("//") {
        return line.to_owned();
    }
    let indent_len = line.len() - trimmed.len();
    let Some(sep) = trimmed.find(['=', ':']) else {
        return line.to_owned();
    };
    let key_raw = &trimmed[..sep];
    let key = key_raw
        .trim()
        .trim_matches(['"', '\''])
        .to_ascii_lowercase();
    if key.is_empty()
        || !key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return line.to_owned();
    }
    let value = &trimmed[sep + 1..];
    let value_trim = value.trim();
    if value_trim.is_empty() || value_trim == "{" || value_trim == "[" {
        return line.to_owned();
    }
    let (body, trailing) = split_trailing_comma(value_trim);
    let quote_json = trimmed.as_bytes()[sep] == b':';
    let key_part = &trimmed[..=sep];

    let replacement = if COORDINATE_KEYS.contains(&key.as_str()) {
        Some(
            body.parse::<f64>()
                .ok()
                .filter(|number| number.is_finite())
                .map_or_else(
                    || format!("\"{REDACTED}\""),
                    |number| format!("{number:.0}"),
                ),
        )
    } else if (SECRET_KEY_PARTS.iter().any(|part| key.contains(part)) && !key.ends_with("_env"))
        || IDENTIFYING_KEYS.contains(&key.as_str())
    {
        (body != "\"\"" && body != "''" && body != "null").then(|| format!("\"{REDACTED}\""))
    } else {
        None
    };
    let _ = quote_json;
    match replacement {
        Some(value) => format!("{}{key_part} {value}{trailing}", &line[..indent_len]),
        None => line.to_owned(),
    }
}

fn split_trailing_comma(value: &str) -> (&str, &str) {
    value
        .strip_suffix(',')
        .map_or((value, ""), |body| (body.trim_end(), ","))
}

fn replace_path_prefix(text: &str, prefix: &str, with: &str) -> String {
    if prefix.is_empty() {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(index) = rest.find(prefix) {
        let after = &rest[index + prefix.len()..];
        let boundary_ok = after
            .chars()
            .next()
            .is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '-' || c == '.'));
        out.push_str(&rest[..index]);
        out.push_str(if boundary_ok { with } else { prefix });
        rest = after;
    }
    out.push_str(rest);
    out
}

/// `/home/<name>`, `/Users/<name>` and `/root` -> `~`.
fn redact_home_dirs(text: &str) -> String {
    let mut out = text.to_owned();
    for base in ["/home/", "/Users/"] {
        let mut result = String::with_capacity(out.len());
        let mut rest = out.as_str();
        while let Some(index) = rest.find(base) {
            result.push_str(&rest[..index]);
            let after = &rest[index + base.len()..];
            let name_len = after
                .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-' || c == '.'))
                .unwrap_or(after.len());
            if name_len == 0 {
                result.push_str(base);
            } else {
                result.push('~');
            }
            rest = &after[name_len..];
        }
        result.push_str(rest);
        out = result;
    }
    replace_path_prefix(&out, "/root", "~")
}

fn replace_word(text: &str, word: &str, with: &str) -> String {
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    let mut prev: Option<char> = None;
    while let Some(index) = rest.find(word) {
        let before = rest[..index].chars().next_back().or(prev);
        let after = rest[index + word.len()..].chars().next();
        out.push_str(&rest[..index]);
        if before.is_none_or(|c| !is_word(c)) && after.is_none_or(|c| !is_word(c)) {
            out.push_str(with);
        } else {
            out.push_str(word);
        }
        prev = word.chars().next_back();
        rest = &rest[index + word.len()..];
    }
    out.push_str(rest);
    out
}

fn url_authority_end(s: &str) -> usize {
    s.find(|c: char| matches!(c, '/' | '?' | '#') || c.is_whitespace() || c == '"' || c == '\'')
        .unwrap_or(s.len())
}

/// `scheme://user:pass@host/` -> `scheme://<redacted>@host/`.
fn redact_url_credentials(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(index) = rest.find("://") {
        let (head, tail) = rest.split_at(index + 3);
        out.push_str(head);
        let end = url_authority_end(tail);
        let authority = &tail[..end];
        match authority.rfind('@') {
            Some(at) => {
                out.push_str(REDACTED);
                out.push_str(&authority[at..]);
            }
            None => out.push_str(authority),
        }
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

fn redact_secret_query_params(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.char_indices().peekable();
    let mut copied = 0;
    while let Some((index, c)) = chars.next() {
        if !matches!(c, '?' | '&' | ';') {
            continue;
        }
        let name_start = index + 1;
        let name_len = text[name_start..]
            .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'))
            .unwrap_or(text.len() - name_start);
        let name_end = name_start + name_len;
        if text[name_end..].starts_with('=')
            && SECRET_QUERY_KEYS.contains(&text[name_start..name_end].to_ascii_lowercase().as_str())
        {
            let value_start = name_end + 1;
            let value_len = text[value_start..]
                .find(|ch: char| matches!(ch, '&' | '#' | ';' | '"' | '\'') || ch.is_whitespace())
                .unwrap_or(text.len() - value_start);
            out.push_str(&text[copied..value_start]);
            out.push_str(REDACTED);
            copied = value_start + value_len;
            while chars.peek().is_some_and(|(next, _)| *next < copied) {
                chars.next();
            }
        }
    }
    out.push_str(&text[copied..]);
    out
}

/// Everything the report needs, gathered up front so rendering is pure.
#[derive(Debug, Clone)]
pub struct ReportInputs {
    pub version: String,
    pub git_hash: Option<String>,
    pub os_release: Option<String>,
    pub kernel: Option<String>,
    pub session_type: Option<String>,
    pub desktop: Option<String>,
    pub wayland_display: bool,
    pub doctor_lines: Vec<String>,
    pub doctor_healthy: Option<bool>,
    pub discovery_json: Option<String>,
    pub discovery_summary: Option<String>,
    pub daemon: DaemonProbe,
    pub config_path: Option<String>,
    pub config_text: Result<String, String>,
}

#[derive(Debug, Clone, Default)]
pub enum DaemonProbe {
    #[default]
    NotChecked,
    Unreachable(String),
    Status(String),
    Unexpected(String),
}

/// Extract `NAME` and `VERSION_ID` (falling back to `PRETTY_NAME`) from
/// `/etc/os-release` content.
#[must_use]
pub fn parse_os_release(content: &str) -> String {
    let field = |wanted: &str| {
        content.lines().find_map(|line| {
            let (key, value) = line.split_once('=')?;
            (key.trim() == wanted)
                .then(|| value.trim().trim_matches(['"', '\'']).to_owned())
                .filter(|value| !value.is_empty())
        })
    };
    let name = field("NAME").or_else(|| field("PRETTY_NAME"));
    match (name, field("VERSION_ID")) {
        (Some(name), Some(version)) => format!("{name} {version}"),
        (Some(name), None) => name,
        (None, _) => String::from("unknown"),
    }
}

fn fence(out: &mut String, lang: &str, body: &str) {
    let ticks = if body.contains("```") { "~~~~" } else { "```" };
    let _ = writeln!(out, "{ticks}{lang}");
    out.push_str(body.trim_end());
    let _ = writeln!(out, "\n{ticks}\n");
}

/// Render the Markdown report. Pure; redacts every collected string.
#[must_use]
pub fn render_report(inputs: &ReportInputs, ctx: &RedactionContext) -> String {
    let red = |text: &str| redact_text(text, ctx);
    let mut out = String::new();
    let _ = writeln!(out, "# SunReactor bug report\n");
    let _ = writeln!(
        out,
        "_Generated by `sunreactorctl doctor --report`. Home paths, user name, coordinates, \
         API keys/tokens, URL credentials, city and device serials are redacted; please still \
         skim before posting._\n"
    );

    let _ = writeln!(out, "## Environment\n");
    let _ = writeln!(out, "- SunReactor: {}", red(&inputs.version));
    let _ = writeln!(
        out,
        "- Git commit: {}",
        inputs.git_hash.as_deref().unwrap_or("unavailable")
    );
    let _ = writeln!(
        out,
        "- OS: {}",
        red(inputs.os_release.as_deref().unwrap_or("unknown"))
    );
    let _ = writeln!(
        out,
        "- Kernel: {}",
        red(inputs.kernel.as_deref().unwrap_or("unknown"))
    );
    let _ = writeln!(
        out,
        "- Session type: {}",
        red(inputs.session_type.as_deref().unwrap_or("unset"))
    );
    let _ = writeln!(
        out,
        "- Desktop: {}",
        red(inputs.desktop.as_deref().unwrap_or("unset"))
    );
    let _ = writeln!(
        out,
        "- WAYLAND_DISPLAY set: {}\n",
        if inputs.wayland_display { "yes" } else { "no" }
    );

    let _ = writeln!(out, "## Doctor diagnostics\n");
    match inputs.doctor_healthy {
        Some(true) => {
            let _ = writeln!(out, "Overall health: OK\n");
        }
        Some(false) => {
            let _ = writeln!(out, "Overall health: ISSUES FOUND\n");
        }
        None => {}
    }
    fence(&mut out, "text", &red(&inputs.doctor_lines.join("\n")));

    let _ = writeln!(out, "## Discovery\n");
    if let Some(summary) = &inputs.discovery_summary {
        let _ = writeln!(out, "{}\n", red(summary));
    }
    if let Some(json) = &inputs.discovery_json {
        fence(&mut out, "json", &redact_config(json, ctx));
    }

    let _ = writeln!(out, "## Daemon\n");
    match &inputs.daemon {
        DaemonProbe::NotChecked => {
            let _ = writeln!(out, "Not checked.\n");
        }
        DaemonProbe::Unreachable(reason) => {
            let _ = writeln!(
                out,
                "Not reachable (this is fine if it is not running): {}\n",
                red(reason)
            );
        }
        DaemonProbe::Unexpected(reason) => {
            let _ = writeln!(
                out,
                "Reachable but returned an unexpected reply: {}\n",
                red(reason)
            );
        }
        DaemonProbe::Status(json) => {
            let _ = writeln!(out, "Reachable. Status:\n");
            fence(&mut out, "json", &redact_config(json, ctx));
        }
    }

    let _ = writeln!(out, "## Config\n");
    if let Some(path) = &inputs.config_path {
        let _ = writeln!(out, "Path: `{}`\n", red(path));
    }
    match &inputs.config_text {
        Ok(text) => fence(&mut out, "toml", &redact_config(text, ctx)),
        Err(reason) => {
            let _ = writeln!(out, "Could not read config: {}\n", red(reason));
        }
    }
    out
}

/// Gather inputs from the live system. Read-only; the daemon is optional.
#[must_use]
pub fn collect_inputs() -> ReportInputs {
    let doctor = crate::doctor::run_diagnostics();
    let discovery = crate::discovery::discover();
    let config_path = crate::paths::config_file().ok();
    ReportInputs {
        version: crate::version().to_owned(),
        git_hash: option_env!("SUNREACTOR_GIT_HASH")
            .filter(|hash| !hash.is_empty())
            .map(str::to_owned),
        os_release: fs::read_to_string("/etc/os-release")
            .or_else(|_| fs::read_to_string("/usr/lib/os-release"))
            .ok()
            .map(|content| parse_os_release(&content)),
        kernel: fs::read_to_string("/proc/sys/kernel/osrelease")
            .ok()
            .map(|kernel| kernel.trim().to_owned()),
        session_type: std::env::var("XDG_SESSION_TYPE").ok(),
        desktop: std::env::var("XDG_CURRENT_DESKTOP")
            .or_else(|_| std::env::var("DESKTOP_SESSION"))
            .ok(),
        wayland_display: std::env::var_os("WAYLAND_DISPLAY").is_some(),
        doctor_lines: doctor_lines(&doctor),
        doctor_healthy: Some(doctor.overall_healthy),
        discovery_summary: Some(discovery_summary(&discovery)),
        discovery_json: serde_json::to_string_pretty(&discovery_view(&discovery)).ok(),
        daemon: probe_daemon(),
        config_text: match &config_path {
            Some(path) => fs::read_to_string(path).map_err(|error| error.to_string()),
            None => Err(String::from("config path could not be determined")),
        },
        config_path: config_path.map(|path| path.display().to_string()),
    }
}

fn doctor_lines(report: &DoctorReport) -> Vec<String> {
    report
        .checks
        .iter()
        .map(|check| {
            let status = match check.status {
                CheckStatus::Pass => "[PASS]",
                CheckStatus::Warn => "[WARN]",
                CheckStatus::Error => "[FAIL]",
            };
            format!("{status} {}: {}", check.code, check.message)
        })
        .collect()
}

fn discovery_summary(report: &DiscoveryReport) -> String {
    format!(
        "{} DDC/CI monitor(s), {} backlight device(s), {} viable target(s).",
        report.summary.ddc_monitors,
        report.summary.backlight_devices,
        report.summary.viable_targets
    )
}

/// Discovery data without stable ids / serials, which can embed hardware
/// serial numbers.
fn discovery_view(report: &DiscoveryReport) -> serde_json::Value {
    serde_json::json!({
        "summary": report.summary,
        "backends": report.backends,
        "ddc_observation_complete": report.ddc_observation_complete,
        "ddc_monitors": report.ddc_monitors.iter().map(|m| serde_json::json!({
            "manufacturer": m.manufacturer,
            "model": m.model,
            "display_number": m.display_number,
            "connector": m.connector,
            "brightness_vcp_supported": m.brightness_vcp_supported,
            "backend_viable": m.backend_viable,
            "note": m.note,
        })).collect::<Vec<_>>(),
        "backlight_devices": report.backlight_devices.iter().map(|d| serde_json::json!({
            "class": d.class,
            "max_brightness": d.max_brightness,
            "backlight_type": d.backlight_type,
            "probe_source": d.probe_source,
            "ddcci_connector": d.ddcci_connector,
            "backend_viable": d.backend_viable,
            "note": d.note,
        })).collect::<Vec<_>>(),
        "notes": report.notes,
    })
}

fn probe_daemon() -> DaemonProbe {
    let socket = match ControlSocket::from_runtime() {
        Ok(socket) => socket,
        Err(error) => return DaemonProbe::Unreachable(error.to_string()),
    };
    match socket.send_request(&RequestEnvelope::new(Request::Status)) {
        Ok(response) => match response.response {
            Response::Status { status } => serde_json::to_string_pretty(&status).map_or_else(
                |e| DaemonProbe::Unexpected(e.to_string()),
                DaemonProbe::Status,
            ),
            other => DaemonProbe::Unexpected(format!("{other:?}")),
        },
        Err(error) => DaemonProbe::Unreachable(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> RedactionContext {
        RedactionContext {
            home: Some(String::from("/home/alice")),
            username: Some(String::from("alice")),
        }
    }

    #[test]
    fn coordinates_are_rounded_to_whole_degrees() {
        let out = redact_config("latitude = 41.0082\nlongitude = -28.9784\n", &ctx());
        assert_eq!(out, "latitude = 41\nlongitude = -29");
    }

    #[test]
    fn unparseable_coordinates_are_redacted() {
        let out = redact_config("latitude = \"41.0082N\"\nlon = nan", &ctx());
        assert!(!out.contains("41.0082"));
        assert_eq!(out.matches(REDACTED).count(), 2);
    }

    #[test]
    fn json_coordinates_keep_trailing_comma() {
        let out = redact_config(
            "  \"latitude\": 40.7128,\n  \"longitude\": -74.006\n",
            &ctx(),
        );
        assert_eq!(out, "  \"latitude\": 41,\n  \"longitude\": -74");
    }

    #[test]
    fn api_keys_and_tokens_are_redacted_but_env_names_kept() {
        let out = redact_config(
            "api_key = \"sk-live-123\"\napi_key_env = \"OWM_KEY\"\nauth_token = 'abc'\nclient_secret=\"x\"\nPassword = \"p\"",
            &ctx(),
        );
        assert!(!out.contains("sk-live-123"));
        assert!(!out.contains("abc"));
        assert!(!out.contains("\"x\""));
        assert!(out.contains("api_key_env = \"OWM_KEY\""));
        assert_eq!(out.matches(REDACTED).count(), 4);
    }

    #[test]
    fn empty_secret_values_are_left_as_is() {
        assert_eq!(redact_config("api_key = \"\"", &ctx()), "api_key = \"\"");
    }

    #[test]
    fn city_and_serial_are_redacted() {
        let out = redact_config("city = \"Istanbul\"\nserial = \"ABC123\"\n", &ctx());
        assert!(!out.contains("Istanbul") && !out.contains("ABC123"));
    }

    #[test]
    fn non_sensitive_keys_are_untouched() {
        let input =
            "tick_seconds = 30\ntimezone = \"Europe/Istanbul\"\n# latitude = 12.5\n[location]";
        // The comment line is only subject to text-level redaction.
        assert_eq!(redact_config(input, &ctx()), input);
    }

    #[test]
    fn url_credentials_are_stripped() {
        let out = redact_text(
            "fetch https://bob:hunter2@api.example.com/v1?x=1 ok",
            &ctx(),
        );
        assert_eq!(out, "fetch https://<redacted>@api.example.com/v1?x=1 ok");
        let out = redact_text("http://token@host", &ctx());
        assert_eq!(out, "http://<redacted>@host");
        assert_eq!(
            redact_text("https://example.com/a@b", &ctx()),
            "https://example.com/a@b"
        );
    }

    #[test]
    fn secret_query_parameters_are_stripped() {
        let out = redact_text(
            "GET https://api.x.com/w?lat=1&appid=SECRET1&units=metric&API_KEY=S2#frag",
            &ctx(),
        );
        assert!(!out.contains("SECRET1") && !out.contains("S2"));
        assert!(out.contains("units=metric"));
        assert!(out.contains("appid=<redacted>&units"));
    }

    #[test]
    fn home_paths_and_usernames_become_tilde() {
        let out = redact_text(
            "/home/alice/.config/sunreactor/config.toml and /home/bob/x and /root/y user alice, alicea",
            &ctx(),
        );
        assert_eq!(
            out,
            "~/.config/sunreactor/config.toml and ~/x and ~/y user <user>, alicea"
        );
    }

    #[test]
    fn home_prefix_does_not_eat_longer_names() {
        let out = redact_text("/home/alice2/file", &ctx());
        assert!(
            !out.contains("alice2"),
            "generic /home/<name> rule applies: {out}"
        );
        assert_eq!(out, "~/file");
    }

    #[test]
    fn custom_home_outside_home_dir_is_redacted() {
        let ctx = RedactionContext {
            home: Some(String::from("/var/lib/carol")),
            username: Some(String::from("carol")),
        };
        assert_eq!(redact_text("/var/lib/carol/.cache", &ctx), "~/.cache");
    }

    #[test]
    fn short_usernames_are_not_replaced() {
        let ctx = RedactionContext {
            home: None,
            username: Some(String::from("al")),
        };
        assert_eq!(redact_text("al and also", &ctx), "al and also");
    }

    #[test]
    fn redaction_is_idempotent() {
        let input = "latitude = 41.5\napi_key = \"k\"\npath=/home/alice/x https://u:p@h/?token=t";
        let once = redact_config(input, &ctx());
        assert_eq!(redact_config(&once, &ctx()), once);
    }

    #[test]
    fn redaction_handles_hostile_input_without_panicking() {
        for input in [
            "=",
            ":",
            "==::",
            "key=",
            "?=&=;",
            "://",
            "://@",
            "@://",
            "?token=",
            "&key=\"",
            "日本語 = \"x\"",
            "latitude = ",
            "api_key = {",
            "/home/",
            "/home/\u{e9}",
        ] {
            let _ = redact_config(input, &ctx());
            let _ = redact_text(input, &ctx());
        }
    }

    #[test]
    fn os_release_is_parsed() {
        let content = "PRETTY_NAME=\"CachyOS\"\nNAME=\"CachyOS Linux\"\nVERSION_ID=\"rolling\"\n";
        assert_eq!(parse_os_release(content), "CachyOS Linux rolling");
        assert_eq!(parse_os_release("NAME=Arch\n"), "Arch");
        assert_eq!(parse_os_release("PRETTY_NAME=Foo\n"), "Foo");
        assert_eq!(parse_os_release(""), "unknown");
    }

    #[test]
    fn report_contains_sections_and_no_secrets() {
        let inputs = ReportInputs {
            version: String::from("0.13.0"),
            git_hash: None,
            os_release: Some(String::from("Arch Linux rolling")),
            kernel: Some(String::from("6.9.0")),
            session_type: Some(String::from("wayland")),
            desktop: Some(String::from("KDE")),
            wayland_display: true,
            doctor_lines: vec![String::from(
                "[PASS] SR-CONFIG-OK: valid at /home/alice/.config/x",
            )],
            doctor_healthy: Some(true),
            discovery_summary: Some(String::from("0 DDC/CI monitor(s)")),
            discovery_json: Some(String::from("{\"a\": 1}")),
            daemon: DaemonProbe::Unreachable(String::from("no socket at /run/user/1000/x")),
            config_path: Some(String::from("/home/alice/.config/sunreactor/config.toml")),
            config_text: Ok(String::from(
                "[location]\nlatitude = 41.9\nlongitude = 12.5\n[weather]\napi_key = \"hunter2\"\n",
            )),
        };
        let report = render_report(&inputs, &ctx());
        for heading in [
            "## Environment",
            "## Doctor diagnostics",
            "## Discovery",
            "## Daemon",
            "## Config",
        ] {
            assert!(report.contains(heading), "missing {heading}");
        }
        assert!(report.contains("Git commit: unavailable"));
        assert!(report.contains("Arch Linux rolling"));
        assert!(!report.contains("hunter2"));
        assert!(!report.contains("alice"));
        assert!(!report.contains("41.9") && !report.contains("12.5"));
        assert!(report.contains("latitude = 42"));
    }

    #[test]
    fn report_renders_unreadable_config_and_status() {
        let inputs = ReportInputs {
            version: String::from("x"),
            git_hash: None,
            os_release: None,
            kernel: None,
            session_type: None,
            desktop: None,
            wayland_display: false,
            doctor_lines: Vec::new(),
            doctor_healthy: None,
            discovery_summary: None,
            discovery_json: None,
            daemon: DaemonProbe::Status(String::from("{\"config_path\": \"/home/alice/c\"}")),
            config_path: None,
            config_text: Err(String::from("No such file: /home/alice/c")),
        };
        let report = render_report(&inputs, &ctx());
        assert!(report.contains("Could not read config"));
        assert!(report.contains("Reachable. Status"));
        assert!(!report.contains("alice"));
    }

    #[test]
    fn daemon_status_location_city_is_redacted() {
        let inputs = ReportInputs {
            version: String::from("0.14.2"),
            git_hash: None,
            os_release: None,
            kernel: None,
            session_type: None,
            desktop: None,
            wayland_display: false,
            doctor_lines: Vec::new(),
            doctor_healthy: None,
            discovery_summary: None,
            discovery_json: None,
            daemon: DaemonProbe::Status(String::from(
                "{\"daemon_alive\": true, \"location_city\": \"Istanbul\"}",
            )),
            config_path: None,
            config_text: Err(String::from("none")),
        };
        let report = render_report(&inputs, &ctx());
        assert!(!report.contains("Istanbul"));
        assert!(report.contains("location_city"));
        assert!(report.contains(REDACTED));
    }
}

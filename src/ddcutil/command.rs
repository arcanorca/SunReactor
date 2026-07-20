use super::DdcutilCapabilities;
use crate::backends::ProcessRunner;
use std::time::Duration;

pub(crate) fn probe_capabilities<R: ProcessRunner>(runner: &R) -> DdcutilCapabilities {
    let mut caps = DdcutilCapabilities {
        version_string: String::new(),
        supports_noconfig: false,
        supports_noverify: false,
        supports_terse: false,
        supports_brief: false,
    };

    if let Ok(version_output) = runner.run(
        "ddcutil",
        &["--version".to_string()],
        Duration::from_secs(2),
    ) {
        if version_output.success() {
            let combined = format!("{}\n{}", version_output.stdout, version_output.stderr);
            caps.version_string = combined
                .lines()
                .find(|l| l.contains("ddcutil"))
                .unwrap_or("")
                .trim()
                .to_string();
        }
    }

    if let Ok(help_output) = runner.run("ddcutil", &["--help".to_string()], Duration::from_secs(2))
    {
        if help_output.success() {
            let help_text = format!("{}\n{}", help_output.stdout, help_output.stderr);
            caps.supports_noconfig = help_text.contains("--noconfig");
            caps.supports_noverify = help_text.contains("--noverify");
            caps.supports_terse = help_text.contains("--terse");
            caps.supports_brief = help_text.contains("--brief");
        }
    }

    caps
}

pub(crate) fn build_base_args(caps: &DdcutilCapabilities) -> Vec<String> {
    let mut args = Vec::new();
    if caps.supports_noconfig {
        args.push("--noconfig".to_string());
    }
    args
}

pub(crate) fn build_detect_args(caps: &DdcutilCapabilities) -> Vec<String> {
    let mut args = build_base_args(caps);
    if caps.supports_terse {
        args.push("--terse".to_string());
    } else if caps.supports_brief {
        args.push("--brief".to_string());
    }
    args.push("detect".to_string());
    args
}

pub(crate) fn build_capabilities_args(caps: &DdcutilCapabilities, display: u32) -> Vec<String> {
    let mut args = build_base_args(caps);
    args.push("--display".to_string());
    args.push(display.to_string());
    args.push("capabilities".to_string());
    args
}

pub(crate) fn build_getvcp_args(
    caps: &DdcutilCapabilities,
    display: u32,
    vcp: &str,
) -> Vec<String> {
    let mut args = build_base_args(caps);
    if caps.supports_terse {
        args.push("--terse".to_string());
    } else if caps.supports_brief {
        args.push("--brief".to_string());
    }
    args.push("--display".to_string());
    args.push(display.to_string());
    args.push("getvcp".to_string());
    args.push(vcp.to_string());
    args
}

pub(crate) fn build_setvcp_args(
    caps: &DdcutilCapabilities,
    selection_args: &[String],
    vcp: &str,
    value: &str,
) -> Vec<String> {
    let mut args = build_base_args(caps);
    if caps.supports_noverify {
        args.push("--noverify".to_string());
    }
    args.extend_from_slice(selection_args);
    args.push("setvcp".to_string());
    args.push(vcp.to_string());
    args.push(value.to_string());
    args
}

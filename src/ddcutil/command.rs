use super::DdcutilCapabilities;
use crate::backends::{CommandOutput, ProcessRunner};
use std::time::Duration;

pub(crate) fn probe_capabilities<R: ProcessRunner>(runner: &R) -> DdcutilCapabilities {
    let mut caps = DdcutilCapabilities {
        supports_noconfig: false,
        supports_noverify: false,
        supports_terse: false,
        supports_brief: false,
    };

    if let Ok(help_output) =
        runner.run("ddcutil", &[String::from("--help")], Duration::from_secs(2))
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

pub(crate) fn build_capabilities_args_for_selection(
    caps: &DdcutilCapabilities,
    selection_args: &[String],
) -> Vec<String> {
    let mut args = build_base_args(caps);
    args.extend_from_slice(selection_args);
    args.push("capabilities".to_string());
    args
}

pub(crate) fn build_getvcp_args_for_selection(
    caps: &DdcutilCapabilities,
    selection_args: &[String],
    vcp: &str,
) -> Vec<String> {
    let mut args = build_base_args(caps);
    if caps.supports_terse {
        args.push(String::from("--terse"));
    } else if caps.supports_brief {
        args.push(String::from("--brief"));
    }
    args.extend_from_slice(selection_args);
    args.push(String::from("getvcp"));
    args.push(vcp.to_owned());
    args
}

pub(crate) fn adapt_args_for_capabilities(
    args: &[String],
    caps: &DdcutilCapabilities,
) -> Vec<String> {
    let mut adapted = Vec::with_capacity(args.len());
    for arg in args {
        match arg.as_str() {
            "--noconfig" if !caps.supports_noconfig => {}
            "--noverify" if !caps.supports_noverify => {}
            "--terse" if !caps.supports_terse => {
                if caps.supports_brief {
                    adapted.push(String::from("--brief"));
                }
            }
            "--brief" if !caps.supports_brief => {
                if caps.supports_terse {
                    adapted.push(String::from("--terse"));
                }
            }
            _ => adapted.push(arg.clone()),
        }
    }
    adapted
}

pub(crate) fn reports_unsupported_option(output: &CommandOutput) -> bool {
    const OPTIONS: [&str; 4] = ["--noconfig", "--noverify", "--terse", "--brief"];
    let diagnostic = format!("{}\n{}", output.stderr, output.stdout).to_ascii_lowercase();

    OPTIONS.iter().any(|option| {
        [
            format!("unknown option {option}"),
            format!("unknown option '{option}'"),
            format!("unknown option \"{option}\""),
            format!("unrecognized option {option}"),
            format!("unrecognized option '{option}'"),
            format!("unrecognized option \"{option}\""),
        ]
        .iter()
        .any(|message| diagnostic.contains(message))
    })
}

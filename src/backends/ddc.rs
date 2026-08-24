use std::fmt::Write as _;
use std::time::Duration;

use crate::config::{MonitorConfig, MonitorSelector};

use super::{
    clamp_percent, command_failure, map_command_error, BackendError, BackendKind, BackendWrite,
    ProcessRunner, RealProcessRunner,
};

const DDC_BRIGHTNESS_VCP_CODE: &str = "10";
const DEFAULT_DDC_ADDRESS: u16 = 0x37;

pub fn apply(monitor: &MonitorConfig, percent: u8) -> Result<BackendWrite, BackendError> {
    apply_with_runner(&RealProcessRunner, monitor, percent, Duration::from_secs(4))
}

pub(crate) fn apply_with_runner<R: ProcessRunner>(
    runner: &R,
    monitor: &MonitorConfig,
    percent: u8,
    timeout: Duration,
) -> Result<BackendWrite, BackendError> {
    let percent = clamp_percent(percent);
    let selection = build_selector(&monitor.selector)?;

    let mut attempts = 0u8;

    loop {
        attempts += 1;
        let args = build_setvcp_args(&selection, percent);

        #[cfg(unix)]
        let _lock = {
            let uid = unsafe { libc::geteuid() };
            let lock_path = format!("/run/user/{uid}/sunreactor_i2c.lock");
            let lock_file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(false)
                .open(&lock_path)
                .ok();

            if let Some(ref f) = lock_file {
                use rustix::fs::{flock, FlockOperation};
                let _ = flock(f, FlockOperation::LockExclusive);
            }
            lock_file // keep open until block ends
        };

        match runner.run("ddcutil", &args, timeout) {
            Ok(output) if output.success() => {
                return Ok(BackendWrite {
                    backend: BackendKind::Ddc,
                    applied_percent: percent,
                    attempts,
                    detail: format!("applied via ddcutil using {}", selection.description),
                });
            }
            Ok(output) => {
                let error = command_failure(BackendKind::Ddc, "ddcutil", &output);
                if attempts == 1 && should_retry(&error) {
                    continue;
                }
                return Err(error.with_attempts(attempts));
            }
            Err(error) => {
                let error = map_command_error(BackendKind::Ddc, error);
                if attempts == 1 && should_retry(&error) {
                    continue;
                }
                return Err(error.with_attempts(attempts));
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DdcSelectorPlan {
    args: Vec<String>,
    pub(crate) description: String,
    kind: DdcSelectorKind,
}

impl DdcSelectorPlan {
    #[must_use]
    pub(crate) fn is_bus_only(&self) -> bool {
        matches!(self.kind, DdcSelectorKind::Bus(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DdcSelectorRelation {
    Equal,
    ProvablyDisjoint,
    MayOverlap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DdcSelectorKind {
    Bus(u8),
    Serial(String),
    SerialModel { serial: String, model: String },
    Model(String),
    Edid(String),
}

fn normalize_compare(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn build_setvcp_args(selection: &DdcSelectorPlan, percent: u8) -> Vec<String> {
    let mut args = vec![String::from("--noconfig"), String::from("--noverify")];
    args.extend(selection.args.iter().cloned());
    args.push(String::from("setvcp"));
    args.push(String::from(DDC_BRIGHTNESS_VCP_CODE));
    args.push(percent.to_string());
    args
}

fn build_selector(selector: &MonitorSelector) -> Result<DdcSelectorPlan, BackendError> {
    let serial = normalized(&selector.serial);
    let model = normalized(&selector.model);
    let edid = normalized(&selector.edid);
    let connector = normalized(&selector.connector);
    let sysfs_path = normalized(&selector.sysfs_path);

    if sysfs_path.is_some() {
        return Err(BackendError::InvalidSelector {
            backend: BackendKind::Ddc,
            field: "sysfs_path",
            message: String::from("sysfs_path only applies to backlight devices"),
        });
    }

    if let Some(address) = selector.ddc_address {
        if address != DEFAULT_DDC_ADDRESS {
            return Err(BackendError::InvalidSelector {
                backend: BackendKind::Ddc,
                field: "ddc_address",
                message: format!(
                    "only the standard DDC/CI slave address {DEFAULT_DDC_ADDRESS} is supported"
                ),
            });
        }
    }

    if let Some(edid) = edid {
        validate_edid(&edid)?;
        return Ok(DdcSelectorPlan {
            args: vec![String::from("--edid"), edid.clone()],
            description: String::from("EDID"),
            kind: DdcSelectorKind::Edid(normalize_compare(&edid)),
        });
    }

    if let Some(serial) = serial {
        let mut args = vec![String::from("--sn"), serial.clone()];
        let mut description = format!("serial `{serial}`");
        let kind = if let Some(model) = model {
            args.push(String::from("--model"));
            args.push(model.clone());
            let _ = write!(description, " and model `{model}`");
            DdcSelectorKind::SerialModel {
                serial: normalize_compare(&serial),
                model: normalize_compare(&model),
            }
        } else {
            DdcSelectorKind::Serial(normalize_compare(&serial))
        };

        return Ok(DdcSelectorPlan {
            args,
            description,
            kind,
        });
    }

    if let Some(bus) = selector.ddc_bus {
        return Ok(DdcSelectorPlan {
            args: vec![String::from("--bus"), bus.to_string()],
            description: format!("bus {bus}"),
            kind: DdcSelectorKind::Bus(bus),
        });
    }

    if let Some(model) = model {
        let args = vec![String::from("--model"), model.clone()];
        let description = format!("model `{model}`");

        return Ok(DdcSelectorPlan {
            args,
            description,
            kind: DdcSelectorKind::Model(normalize_compare(&model)),
        });
    }

    if connector.is_some() {
        return Err(BackendError::MissingSelector {
            backend: BackendKind::Ddc,
            expected:
                "serial, model, edid, or ddc_bus; connector alone is not stable enough for apply",
        });
    }

    Err(BackendError::MissingSelector {
        backend: BackendKind::Ddc,
        expected: "serial, model, edid, or ddc_bus",
    })
}

pub(crate) fn selector_plan(selector: &MonitorSelector) -> Result<DdcSelectorPlan, BackendError> {
    build_selector(selector)
}

pub(crate) fn effective_selector_args(
    selector: &MonitorSelector,
) -> Result<Vec<String>, BackendError> {
    Ok(build_selector(selector)?.args)
}

pub(crate) fn selector_relation(
    left: &MonitorSelector,
    right: &MonitorSelector,
) -> Result<DdcSelectorRelation, BackendError> {
    let left = selector_plan(left)?.kind;
    let right = selector_plan(right)?.kind;
    Ok(match (&left, &right) {
        (DdcSelectorKind::Bus(a), DdcSelectorKind::Bus(b)) => {
            if a == b {
                DdcSelectorRelation::Equal
            } else {
                DdcSelectorRelation::ProvablyDisjoint
            }
        }
        (DdcSelectorKind::Serial(a), DdcSelectorKind::Serial(b)) => {
            if a == b {
                DdcSelectorRelation::Equal
            } else {
                DdcSelectorRelation::ProvablyDisjoint
            }
        }
        (DdcSelectorKind::Edid(a), DdcSelectorKind::Edid(b)) => {
            if a == b {
                DdcSelectorRelation::Equal
            } else {
                DdcSelectorRelation::ProvablyDisjoint
            }
        }
        (
            DdcSelectorKind::SerialModel {
                serial: a,
                model: am,
            },
            DdcSelectorKind::SerialModel {
                serial: b,
                model: bm,
            },
        ) => {
            if a != b || am != bm {
                DdcSelectorRelation::ProvablyDisjoint
            } else {
                DdcSelectorRelation::Equal
            }
        }
        (DdcSelectorKind::Model(a), DdcSelectorKind::Model(b)) => {
            if a == b {
                DdcSelectorRelation::Equal
            } else {
                DdcSelectorRelation::ProvablyDisjoint
            }
        }
        (DdcSelectorKind::Model(a), DdcSelectorKind::SerialModel { model: b, .. })
        | (DdcSelectorKind::SerialModel { model: b, .. }, DdcSelectorKind::Model(a)) => {
            if a == b {
                DdcSelectorRelation::MayOverlap
            } else {
                DdcSelectorRelation::ProvablyDisjoint
            }
        }
        (DdcSelectorKind::Serial(a), DdcSelectorKind::SerialModel { serial: b, .. })
        | (DdcSelectorKind::SerialModel { serial: b, .. }, DdcSelectorKind::Serial(a)) => {
            if a == b {
                DdcSelectorRelation::MayOverlap
            } else {
                DdcSelectorRelation::ProvablyDisjoint
            }
        }
        _ => DdcSelectorRelation::MayOverlap,
    })
}

pub(crate) fn selector_matches_discovery(
    selector: &MonitorSelector,
    monitor: &crate::discovery::DdcMonitorDiscovery,
) -> Result<bool, BackendError> {
    let plan = selector_plan(selector)?;
    let matches = match &plan.kind {
        DdcSelectorKind::Bus(bus) => monitor.bus_number == Some(u32::from(*bus)),
        DdcSelectorKind::Serial(serial) => {
            normalized_compare(&monitor.serial).is_some_and(|value| value == *serial)
        }
        DdcSelectorKind::SerialModel { serial, model } => {
            normalized_compare(&monitor.serial).is_some_and(|value| value == *serial)
                && normalized_compare(&monitor.model).is_some_and(|value| value == *model)
        }
        DdcSelectorKind::Model(model) => {
            normalized_compare(&monitor.model).is_some_and(|value| value == *model)
        }
        DdcSelectorKind::Edid(_) => false,
    };
    Ok(matches)
}

pub(crate) fn selector_match_count(
    selector: &MonitorSelector,
    monitors: &[crate::discovery::DdcMonitorDiscovery],
) -> Result<usize, BackendError> {
    let mut count = 0;
    for monitor in monitors {
        if selector_matches_discovery(selector, monitor)? {
            count += 1;
        }
    }
    Ok(count)
}

fn normalized(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn normalized_compare(value: &Option<String>) -> Option<String> {
    value.as_deref().map(normalize_compare)
}

fn validate_edid(edid: &str) -> Result<(), BackendError> {
    if edid.len() != 256 || !edid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(BackendError::InvalidSelector {
            backend: BackendKind::Ddc,
            field: "edid",
            message: String::from("expected exactly 256 hexadecimal characters"),
        });
    }

    Ok(())
}

fn should_retry(error: &BackendError) -> bool {
    matches!(
        error,
        BackendError::CommandTimeout { .. }
            | BackendError::CommandFailed {
                transient: true,
                ..
            }
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::backends::testutil::FakeRunner;
    use crate::config::{MonitorConfig, MonitorSelector};

    use super::{apply_with_runner, selector_relation, DdcSelectorRelation};

    #[test]
    fn selector_semantics_distinguish_equality_from_overlap() {
        let model = MonitorSelector {
            model: Some(String::from("U2720Q")),
            ..MonitorSelector::default()
        };
        let serial_model = MonitorSelector {
            serial: Some(String::from("ABC123")),
            model: Some(String::from("U2720Q")),
            ..MonitorSelector::default()
        };
        let other_serial_model = MonitorSelector {
            serial: Some(String::from("BBB")),
            model: Some(String::from("U2720Q")),
            ..MonitorSelector::default()
        };
        let other_model = MonitorSelector {
            model: Some(String::from("OTHER")),
            ..MonitorSelector::default()
        };

        assert_eq!(
            selector_relation(&model, &serial_model).unwrap(),
            DdcSelectorRelation::MayOverlap
        );
        assert_eq!(
            selector_relation(&serial_model, &other_serial_model).unwrap(),
            DdcSelectorRelation::ProvablyDisjoint
        );
        assert_eq!(
            selector_relation(&model, &other_model).unwrap(),
            DdcSelectorRelation::ProvablyDisjoint
        );
        assert_eq!(
            selector_relation(
                &MonitorSelector {
                    serial: Some(String::from("ABC123")),
                    ..MonitorSelector::default()
                },
                &MonitorSelector {
                    serial: Some(String::from("abc123")),
                    ..MonitorSelector::default()
                },
            )
            .unwrap(),
            DdcSelectorRelation::Equal
        );
        assert_eq!(
            selector_relation(
                &MonitorSelector {
                    ddc_bus: Some(7),
                    ..MonitorSelector::default()
                },
                &MonitorSelector {
                    ddc_bus: Some(8),
                    ..MonitorSelector::default()
                },
            )
            .unwrap(),
            DdcSelectorRelation::ProvablyDisjoint
        );
        assert_eq!(
            selector_relation(
                &MonitorSelector {
                    ddc_bus: Some(7),
                    ..MonitorSelector::default()
                },
                &MonitorSelector {
                    serial: Some(String::from("ABC123")),
                    ..MonitorSelector::default()
                },
            )
            .unwrap(),
            DdcSelectorRelation::MayOverlap
        );

        let mi_model = MonitorSelector {
            model: Some(String::from("Mi Monitor")),
            connector: Some(String::from("card1-DP-1")),
            ..MonitorSelector::default()
        };
        let len_serial_model = MonitorSelector {
            serial: Some(String::from("V305PTDA")),
            model: Some(String::from("LEN P24h-20")),
            connector: Some(String::from("card1-DP-3")),
            ddc_bus: Some(9),
            ..MonitorSelector::default()
        };
        assert_eq!(
            selector_relation(&mi_model, &len_serial_model).unwrap(),
            DdcSelectorRelation::ProvablyDisjoint
        );

        let model_with_bus = MonitorSelector {
            model: Some(String::from("Mi Monitor")),
            ddc_bus: Some(7),
            ..MonitorSelector::default()
        };
        let plan = super::selector_plan(&model_with_bus).expect("model selector should be valid");
        assert_eq!(plan.args, ["--bus", "7"]);
    }

    #[test]
    fn retries_once_for_transient_ddc_failures_and_uses_stable_selectors() {
        let monitor = ddc_monitor(MonitorSelector {
            connector: Some(String::from("card1-DP-1")),
            serial: Some(String::from("ABC123")),
            model: Some(String::from("U2720Q")),
            edid: None,
            sysfs_path: None,
            ddc_bus: Some(7),
            ddc_address: Some(0x37),
        });
        let runner = FakeRunner::new()
            .with_output(
                "ddcutil",
                &[
                    "--noconfig",
                    "--noverify",
                    "--sn",
                    "ABC123",
                    "--model",
                    "U2720Q",
                    "setvcp",
                    "10",
                    "64",
                ],
                Some(1),
                "",
                "Device or resource busy",
            )
            .with_success(
                "ddcutil",
                &[
                    "--noconfig",
                    "--noverify",
                    "--sn",
                    "ABC123",
                    "--model",
                    "U2720Q",
                    "setvcp",
                    "10",
                    "64",
                ],
                "",
            );

        let result = apply_with_runner(&runner, &monitor, 64, Duration::from_secs(4))
            .expect("ddc write should succeed");

        assert_eq!(result.applied_percent, 64);
        assert_eq!(result.attempts, 2);

        let calls = runner.calls();
        assert_eq!(calls.len(), 2);
        assert!(calls[0].contains("|--sn|ABC123|--model|U2720Q|setvcp|10|64"));
        assert!(!calls[0].contains("|--bus|7|"));
        assert!(!calls.iter().any(|call| call.contains("getvcp")));
    }

    #[test]
    fn falls_back_to_bus_when_no_stable_ddc_selector_exists() {
        let monitor = ddc_monitor(MonitorSelector {
            connector: Some(String::from("card1-DP-3")),
            serial: None,
            model: None,
            edid: None,
            sysfs_path: None,
            ddc_bus: Some(9),
            ddc_address: None,
        });
        let runner = FakeRunner::new().with_success(
            "ddcutil",
            &[
                "--noconfig",
                "--noverify",
                "--bus",
                "9",
                "setvcp",
                "10",
                "42",
            ],
            "",
        );

        let result = apply_with_runner(&runner, &monitor, 42, Duration::from_secs(4))
            .expect("ddc write should succeed");

        assert_eq!(result.attempts, 1);
        assert!(result.detail.contains("bus 9"));
    }

    #[test]
    fn rejects_connector_only_selector() {
        let monitor = ddc_monitor(MonitorSelector {
            connector: Some(String::from("card1-DP-1")),
            serial: None,
            model: None,
            edid: None,
            sysfs_path: None,
            ddc_bus: None,
            ddc_address: None,
        });
        let runner = FakeRunner::new();

        let error = apply_with_runner(&runner, &monitor, 50, Duration::from_secs(4))
            .expect_err("connector-only selector must fail");

        assert!(error
            .to_string()
            .contains("connector alone is not stable enough"));
    }

    #[test]
    fn retries_timeouts_only_once() {
        let monitor = ddc_monitor(MonitorSelector {
            connector: None,
            serial: None,
            model: None,
            edid: None,
            sysfs_path: None,
            ddc_bus: Some(6),
            ddc_address: None,
        });
        let runner = FakeRunner::new()
            .with_timeout(
                "ddcutil",
                &[
                    "--noconfig",
                    "--noverify",
                    "--bus",
                    "6",
                    "setvcp",
                    "10",
                    "55",
                ],
                Duration::from_secs(4),
                "timed out",
            )
            .with_timeout(
                "ddcutil",
                &[
                    "--noconfig",
                    "--noverify",
                    "--bus",
                    "6",
                    "setvcp",
                    "10",
                    "55",
                ],
                Duration::from_secs(4),
                "timed out again",
            );

        let error = apply_with_runner(&runner, &monitor, 55, Duration::from_secs(4))
            .expect_err("second timeout should fail");

        assert!(error.to_string().contains("timed out"));
        assert_eq!(runner.calls().len(), 2);
    }

    fn ddc_monitor(selector: MonitorSelector) -> MonitorConfig {
        MonitorConfig {
            logical_id: String::from("desk"),
            backend: crate::backends::BackendKind::Ddc,
            enabled: true,
            allow_topology_retargeting: false,
            min_pct: 0,
            max_pct: 100,
            gain: 1.0,
            transition_gamma: 1.4,
            milestone_adjustments: Vec::new(),
            selector,
        }
    }
}

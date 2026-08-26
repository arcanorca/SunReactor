//! Runtime topology reconciliation for Phase 9.
//!
//! SunReactor is a brightness daemon: it must not write the same physical
//! panel twice, and it must keep controlling healthy displays when another
//! configured display disappears or hangs. Phase 8 established the
//! authoritative DDC<->ddcci identity evidence at discovery/config-generation
//! time. Phase 9 needs that same evidence at *runtime*: a manual or generated
//! config can legally contain both a DDC monitor and a backlight block for the
//! same physical display, and a monitor can physically disappear or return on
//! a changed DDC bus.
//!
//! Design constraints (Owner Phase 9):
//! - Do NOT mutate persistent config, do NOT auto-adopt new hardware, do NOT
//!   invent fuzzy identity from names/ordering/`type=raw`, do NOT add
//!   libudev/libddcutil/systemd.
//! - Observation must be separated from config.
//!
//! This module is pure with respect to hardware: everything it consumes is
//! supplied by the caller. The runtime boundary owns the only
//! command/hardware-touching entry points (`observe`).

use crate::backends::BackendKind;
use crate::config::MonitorConfig;
use crate::discovery::{BacklightDeviceDiscovery, DdcMonitorDiscovery};

/// How a configured monitor should be treated after reconciling against a
/// fresh capability observation.
///
/// This is the classification contract from the owner brief:
/// - `Present` should normally enter hardware apply.
/// - `TemporarilyUnavailable` may not write this tick.
/// - `SuppressedProvenAlias` must NOT produce a second hardware write
///   (the alias of an already-controlled panel is suppressed).
/// - `AmbiguousOrUnsafe` must never write (fail closed).
/// - `UnconfiguredObserved` names a physical device the daemon saw but did
///   not configure; it must never be auto-controlled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconcileAction {
    Present,
    TemporarilyUnavailable,
    SuppressedProvenAlias,
    AmbiguousOrUnsafe,
    /// A bus-only DDC selector is unsafe without explicit opt-in because
    /// adapter numbers can be reused by a different physical display.
    TopologyRetargetingDisabled,
    /// Reserved for future status diagnostics that surface observed but
    /// unconfigured hardware. Never emitted by the configured-target apply
    /// path (the apply gate only classifies configured monitors), so it may
    /// appear unused by dead-code analysis until those diagnostics exist.
    #[allow(dead_code)]
    UnconfiguredObserved,
}

impl ReconcileAction {
    /// Returns true when the configured target should be allowed through the
    /// apply path this tick.
    #[must_use]
    pub fn allowed_to_apply(self) -> bool {
        matches!(self, Self::Present)
    }

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::TemporarilyUnavailable => "temporarily_unavailable",
            Self::SuppressedProvenAlias => "suppressed_proven_alias",
            Self::AmbiguousOrUnsafe => "ambiguous_or_unsafe",
            Self::TopologyRetargetingDisabled => "topology_retargeting_disabled",
            Self::UnconfiguredObserved => "unconfigured_observed",
        }
    }
}

/// A fresh runtime capability observation, kept separate from user config.
///
/// Deliberately NOT a persistent schema; exists only for the current
/// observation/apply cycle.
#[derive(Debug, Clone, Default)]
pub struct CapabilitySnapshot {
    /// DDC targets currently qualified (present + brightness-capable).
    pub ddc_present: Vec<DdcMonitorDiscovery>,
    /// Backlights currently present with viable sysfs entries / driver.
    pub backlights_present: Vec<BacklightDeviceDiscovery>,
}

impl CapabilitySnapshot {
    #[must_use]
    pub(crate) fn from_discovery(report: &crate::discovery::DiscoveryReport) -> Self {
        Self {
            ddc_present: report.ddc_monitors.clone(),
            backlights_present: report.backlight_devices.clone(),
        }
    }

    /// Returns true when a configured backlight monitor's sysfs pin matches a
    /// currently-present discovery backlight device.
    #[must_use]
    pub fn backlight_present_for(&self, monitor: &MonitorConfig) -> bool {
        let Some(configured_path) = configured_backlight_path(monitor) else {
            return false;
        };
        self.backlights_present
            .iter()
            .filter(|observed| observed.backend_viable)
            .any(|observed| resolved_backlight_device_path(observed) == configured_path)
        // Note: a discovered backlight is considered "present" only when it is
        // viable in the snapshot. An entry exists but is not brightness-capable
        // is not a usable target.
    }
}

/// The single pure reconcile operation.
///
/// `configured` is the user's enabled `MonitorConfig` set (unchanged).
/// `observed` is the fresh `CapabilitySnapshot` (runtime observation).
///
/// Returns one action per configured monitor, in the same order as
/// `configured`. This makes the effective runtime target set printable and
/// testable without hardware.
#[must_use]
pub fn reconcile(
    configured: &[MonitorConfig],
    observed: &CapabilitySnapshot,
) -> Vec<ReconcileAction> {
    configured
        .iter()
        .map(|monitor| classify_monitor(monitor, observed))
        .collect()
}

/// Classifies one configured monitor against the snapshot.
///
/// Evidence rules (do NOT infer sameness from names/ordering/serial
/// similarity/connector similarity/GPU vendor/"only one monitor"):
/// - DDC presence is proven by `selector_matches_discovery`.
/// - Backlight presence is proven by an explicit sysfs pin matching a device
///   actually reported by discovery.
/// - A configured backlight proven (via the kernel/driver `ddcci_backlight`
///   symlink convergence annotated on the *observed* device) to alias a
///   *present and viable* discovered DDC monitor is suppressed per DDC-primary.
fn classify_monitor(monitor: &MonitorConfig, observed: &CapabilitySnapshot) -> ReconcileAction {
    match monitor.backend {
        BackendKind::Ddc => {
            let bus_only = crate::backends::ddc::selector_plan(&monitor.selector)
                .is_ok_and(|plan| plan.is_bus_only());
            if bus_only && !monitor.allow_topology_retargeting {
                return ReconcileAction::TopologyRetargetingDisabled;
            }

            // A selector that matches multiple distinct discovered displays is
            // ambiguous: choosing one would manufacture identity. Fail closed.
            let matches = observed
                .ddc_present
                .iter()
                .filter(|observed| {
                    observed.backend_viable
                        && crate::backends::ddc::selector_matches_discovery(
                            &monitor.selector,
                            observed,
                        )
                        .unwrap_or(false)
                })
                .count();
            match matches {
                0 => ReconcileAction::TemporarilyUnavailable,
                1 => ReconcileAction::Present,
                _ => ReconcileAction::AmbiguousOrUnsafe,
            }
        }
        BackendKind::Backlight => {
            if !observed.backlight_present_for(monitor) {
                return ReconcileAction::TemporarilyUnavailable;
            }
            if is_ddcci_alias_of_present_ddc(monitor, observed) {
                ReconcileAction::SuppressedProvenAlias
            } else {
                ReconcileAction::Present
            }
        }
    }
}

/// Returns true when a configured backlight monitor is provably an alias of a
/// present, viable, discovered DDC monitor.
fn is_ddcci_alias_of_present_ddc(monitor: &MonitorConfig, observed: &CapabilitySnapshot) -> bool {
    let Some(configured_path) = configured_backlight_path(monitor) else {
        return false;
    };
    let Some(observed_backlight) = observed
        .backlights_present
        .iter()
        .find(|device| resolved_backlight_device_path(device) == configured_path)
    else {
        return false;
    };
    let Some(alias_connector) = observed_backlight.ddcci_connector.as_deref() else {
        return false;
    };
    observed.ddc_present.iter().any(|monitor| {
        monitor.backend_viable && monitor.connector.as_deref() == Some(alias_connector)
    })
}

/// Returns the canonical absolute form of an observed backlight device path.
fn resolved_backlight_device_path(observed: &BacklightDeviceDiscovery) -> String {
    observed.sysfs_path.trim().to_owned()
}

/// Returns the canonical absolute form of a configured backlight sysfs pin.
fn configured_backlight_path(monitor: &MonitorConfig) -> Option<String> {
    monitor
        .selector
        .sysfs_path
        .as_deref()
        .and_then(crate::backends::backlight::canonical_configured_sysfs_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::PhysicalIdentityStatus;

    fn ddc_observed(serial: &str, connector: Option<&str>, viable: bool) -> DdcMonitorDiscovery {
        DdcMonitorDiscovery {
            occurrence_id: format!("occ-{serial}"),
            stable_id: format!("ddc:{serial}"),
            identity_status: PhysicalIdentityStatus::Unique,
            manufacturer: Some(String::from("DEL")),
            model: Some(String::from("U2720Q")),
            serial: Some(serial.to_owned()),
            display_number: 1,
            bus_number: Some(7),
            connector: connector.map(str::to_owned),
            brightness_vcp_supported: Some(viable),
            backend_viable: viable,
            note: None,
        }
    }

    fn ddc_observed_with_fields(
        model: &str,
        serial: Option<&str>,
        connector: &str,
        bus: u32,
        viable: bool,
    ) -> DdcMonitorDiscovery {
        DdcMonitorDiscovery {
            occurrence_id: format!("occ-{model}-{bus}"),
            stable_id: format!("ddc:{model}:{bus}"),
            identity_status: PhysicalIdentityStatus::Unique,
            manufacturer: Some(String::from("TEST")),
            model: Some(model.to_owned()),
            serial: serial.map(str::to_owned),
            display_number: bus,
            bus_number: Some(bus),
            connector: Some(connector.to_owned()),
            brightness_vcp_supported: Some(viable),
            backend_viable: viable,
            note: None,
        }
    }

    fn backlight_observed(
        device_name: &str,
        ddcci_connector: Option<&str>,
    ) -> BacklightDeviceDiscovery {
        BacklightDeviceDiscovery {
            stable_id: format!("backlight:{device_name}"),
            device_name: device_name.to_owned(),
            class: String::from("backlight"),
            max_brightness: Some(100),
            backlight_type: Some(String::from("raw")),
            probe_source: String::from("brightnessctl"),
            sysfs_path: format!("/sys/class/backlight/{device_name}"),
            ddcci_connector: ddcci_connector.map(str::to_owned),
            backend_viable: true,
            note: None,
        }
    }

    fn ddc_monitor(logical_id: &str, serial: &str) -> MonitorConfig {
        MonitorConfig {
            logical_id: logical_id.to_owned(),
            backend: BackendKind::Ddc,
            enabled: true,
            selector: crate::config::MonitorSelector {
                serial: Some(serial.to_owned()),
                ..crate::config::MonitorSelector::default()
            },
            ..MonitorConfig::default()
        }
    }

    fn backlight_monitor(logical_id: &str, device_name: &str) -> MonitorConfig {
        MonitorConfig {
            logical_id: logical_id.to_owned(),
            backend: BackendKind::Backlight,
            enabled: true,
            selector: crate::config::MonitorSelector {
                sysfs_path: Some(format!("/sys/class/backlight/{device_name}")),
                ..crate::config::MonitorSelector::default()
            },
            ..MonitorConfig::default()
        }
    }

    // -----------------------------------------------------------------
    // BASIC / AMBIGUITY
    // -----------------------------------------------------------------

    #[test]
    fn internal_plus_external_present_both_apply() {
        let configured = vec![
            backlight_monitor("internal", "intel_backlight"),
            ddc_monitor("external", "SN_ABC"),
        ];
        let observed = CapabilitySnapshot {
            backlights_present: vec![backlight_observed("intel_backlight", None)],
            ddc_present: vec![ddc_observed("SN_ABC", Some("card1-DP-1"), true)],
        };
        let actions = reconcile(&configured, &observed);
        assert_eq!(
            actions,
            vec![ReconcileAction::Present, ReconcileAction::Present]
        );
    }

    #[test]
    fn relative_backlight_pin_matches_observed_absolute_sysfs_identity() {
        // R1a characterization: the backend accepts a relative device name
        // and resolves it to the observed absolute sysfs representation, but
        // the current topology gate compares the raw configured string.
        let configured = vec![MonitorConfig {
            logical_id: String::from("internal"),
            backend: BackendKind::Backlight,
            enabled: true,
            selector: crate::config::MonitorSelector {
                sysfs_path: Some(String::from("intel_backlight")),
                ..crate::config::MonitorSelector::default()
            },
            ..MonitorConfig::default()
        }];
        let observed = CapabilitySnapshot {
            backlights_present: vec![backlight_observed("intel_backlight", None)],
            ddc_present: Vec::new(),
        };

        assert_eq!(
            crate::backends::backlight::resolve_backlight_device_name(&configured[0].selector)
                .expect("relative backlight pin should resolve"),
            "intel_backlight"
        );
        assert_eq!(
            observed.backlights_present[0].sysfs_path,
            "/sys/class/backlight/intel_backlight"
        );
        assert_eq!(
            reconcile(&configured, &observed),
            vec![ReconcileAction::Present],
            "backend-resolvable relative pin must not be rejected by topology"
        );
    }

    #[test]
    fn real_two_monitor_metadata_selectors_are_disjoint_and_present() {
        let configured = vec![
            MonitorConfig {
                logical_id: String::from("mi-monitor"),
                backend: BackendKind::Ddc,
                enabled: true,
                selector: crate::config::MonitorSelector {
                    model: Some(String::from("Mi Monitor")),
                    connector: Some(String::from("card1-DP-1")),
                    ..crate::config::MonitorSelector::default()
                },
                ..MonitorConfig::default()
            },
            MonitorConfig {
                logical_id: String::from("len-p24h-20-v305ptda"),
                backend: BackendKind::Ddc,
                enabled: true,
                selector: crate::config::MonitorSelector {
                    serial: Some(String::from("V305PTDA")),
                    model: Some(String::from("LEN P24h-20")),
                    connector: Some(String::from("card1-DP-3")),
                    ddc_bus: Some(9),
                    ..crate::config::MonitorSelector::default()
                },
                ..MonitorConfig::default()
            },
        ];
        let observed = CapabilitySnapshot {
            backlights_present: vec![],
            ddc_present: vec![
                ddc_observed_with_fields("Mi Monitor", None, "card1-DP-1", 7, true),
                ddc_observed_with_fields("LEN P24h-20", Some("V305PTDA"), "card1-DP-3", 9, true),
            ],
        };
        let relation = crate::backends::ddc::selector_relation(
            &configured[0].selector,
            &configured[1].selector,
        )
        .expect("selectors should be valid");
        assert_eq!(
            relation,
            crate::backends::ddc::DdcSelectorRelation::ProvablyDisjoint
        );
        assert_eq!(
            crate::backends::ddc::selector_match_count(
                &configured[0].selector,
                &observed.ddc_present,
            )
            .unwrap(),
            1
        );
        assert_eq!(
            crate::backends::ddc::selector_match_count(
                &configured[1].selector,
                &observed.ddc_present,
            )
            .unwrap(),
            1
        );
        assert_eq!(
            reconcile(&configured, &observed),
            vec![ReconcileAction::Present, ReconcileAction::Present]
        );
    }

    #[test]
    fn external_absent_marks_unavailable_but_internal_keeps_applying() {
        let configured = vec![
            backlight_monitor("internal", "intel_backlight"),
            ddc_monitor("ddc", "SN_ABC"),
        ];
        // external monitor physically unplugged -> ddcutil no longer reports it
        let observed = CapabilitySnapshot {
            backlights_present: vec![backlight_observed("intel_backlight", None)],
            ddc_present: vec![],
        };
        let actions = reconcile(&configured, &observed);
        assert_eq!(
            actions,
            vec![
                ReconcileAction::Present,
                ReconcileAction::TemporarilyUnavailable
            ]
        );
        // healthy internal continues applying; unavailable external skipped
        assert!(actions[0].allowed_to_apply());
        assert!(!actions[1].allowed_to_apply());
    }

    #[test]
    fn external_returns_same_selector_regains_present() {
        let configured = vec![ddc_monitor("ddc", "SN_ABC")];
        let observed = CapabilitySnapshot {
            backlights_present: vec![],
            ddc_present: vec![ddc_observed("SN_ABC", Some("card1-DP-1"), true)],
        };
        let actions = reconcile(&configured, &observed);
        assert_eq!(actions, vec![ReconcileAction::Present]);
        assert!(actions[0].allowed_to_apply());
    }

    #[test]
    fn sequential_snapshots_disconnect_and_reconnect_without_changing_configured_identity() {
        let configured = vec![
            ddc_monitor("left", "SN_LEFT"),
            ddc_monitor("right", "SN_RIGHT"),
        ];
        let original = configured.clone();

        let snapshot_a = CapabilitySnapshot {
            backlights_present: vec![],
            ddc_present: vec![
                ddc_observed("SN_LEFT", Some("card1-DP-1"), true),
                ddc_observed("SN_RIGHT", Some("card1-DP-2"), true),
            ],
        };
        assert_eq!(
            reconcile(&configured, &snapshot_a),
            vec![ReconcileAction::Present, ReconcileAction::Present]
        );

        let snapshot_b = CapabilitySnapshot {
            backlights_present: vec![],
            ddc_present: vec![ddc_observed("SN_LEFT", Some("card1-DP-1"), true)],
        };
        assert_eq!(
            reconcile(&configured, &snapshot_b),
            vec![
                ReconcileAction::Present,
                ReconcileAction::TemporarilyUnavailable
            ]
        );

        let snapshot_c = CapabilitySnapshot {
            backlights_present: vec![],
            ddc_present: vec![
                ddc_observed("SN_LEFT", Some("card1-DP-1"), true),
                ddc_observed("SN_RIGHT", Some("card1-DP-2"), true),
            ],
        };
        assert_eq!(
            reconcile(&configured, &snapshot_c),
            vec![ReconcileAction::Present, ReconcileAction::Present]
        );
        let identity = |monitor: &MonitorConfig| {
            (
                monitor.logical_id.clone(),
                monitor.backend,
                monitor.selector.serial.clone(),
                monitor.selector.model.clone(),
                monitor.selector.connector.clone(),
                monitor.selector.sysfs_path.clone(),
                monitor.selector.ddc_bus,
            )
        };
        assert_eq!(
            configured.iter().map(identity).collect::<Vec<_>>(),
            original.iter().map(identity).collect::<Vec<_>>()
        );
    }

    #[test]
    fn reconnect_with_non_unique_selector_stays_fail_closed() {
        let configured = vec![ddc_monitor("right", "SN_RIGHT")];
        let ambiguous_return = CapabilitySnapshot {
            backlights_present: vec![],
            ddc_present: vec![
                ddc_observed("SN_RIGHT", Some("card1-DP-2"), true),
                ddc_observed("SN_RIGHT", Some("card2-HDMI-1"), true),
            ],
        };

        assert_eq!(
            reconcile(&configured, &ambiguous_return),
            vec![ReconcileAction::AmbiguousOrUnsafe]
        );
        assert!(!ReconcileAction::AmbiguousOrUnsafe.allowed_to_apply());
    }

    #[test]
    fn bus_reuse_classifies_different_occupant_as_present() {
        // A bus-only selector identifies the current topology slot, not the
        // physical monitor that occupied it in an earlier snapshot.
        let configured = vec![MonitorConfig {
            logical_id: String::from("bus-target"),
            backend: BackendKind::Ddc,
            enabled: true,
            allow_topology_retargeting: true,
            selector: crate::config::MonitorSelector {
                ddc_bus: Some(7),
                ..crate::config::MonitorSelector::default()
            },
            ..MonitorConfig::default()
        }];

        let snapshot_a = CapabilitySnapshot {
            backlights_present: vec![],
            ddc_present: vec![ddc_observed("A", Some("card1-DP-1"), true)],
        };
        assert_eq!(
            reconcile(&configured, &snapshot_a),
            vec![ReconcileAction::Present]
        );

        let snapshot_b = CapabilitySnapshot::default();
        assert_eq!(
            reconcile(&configured, &snapshot_b),
            vec![ReconcileAction::TemporarilyUnavailable]
        );

        let snapshot_c = CapabilitySnapshot {
            backlights_present: vec![],
            ddc_present: vec![ddc_observed("B", Some("card1-HDMI-A-1"), true)],
        };
        assert_eq!(
            reconcile(&configured, &snapshot_c),
            vec![ReconcileAction::Present]
        );
        assert!(reconcile(&configured, &snapshot_c)[0].allowed_to_apply());
    }

    #[test]
    fn bus_only_without_opt_in_is_skipped_even_when_bus_is_present() {
        let configured = vec![MonitorConfig {
            logical_id: String::from("bus-target"),
            backend: BackendKind::Ddc,
            enabled: true,
            allow_topology_retargeting: false,
            selector: crate::config::MonitorSelector {
                ddc_bus: Some(7),
                ..crate::config::MonitorSelector::default()
            },
            ..MonitorConfig::default()
        }];
        let observed = CapabilitySnapshot {
            backlights_present: vec![],
            ddc_present: vec![ddc_observed("B", Some("card1-HDMI-A-1"), true)],
        };

        assert_eq!(
            reconcile(&configured, &observed),
            vec![ReconcileAction::TopologyRetargetingDisabled]
        );
        assert!(!reconcile(&configured, &observed)[0].allowed_to_apply());
    }

    #[test]
    fn stronger_ddc_selector_is_not_gated_as_bus_only() {
        let configured = vec![MonitorConfig {
            logical_id: String::from("serial-target"),
            backend: BackendKind::Ddc,
            enabled: true,
            allow_topology_retargeting: false,
            selector: crate::config::MonitorSelector {
                serial: Some(String::from("SN_RIGHT")),
                ddc_bus: Some(7),
                ..crate::config::MonitorSelector::default()
            },
            ..crate::config::MonitorConfig::default()
        }];
        let observed = CapabilitySnapshot {
            backlights_present: vec![],
            ddc_present: vec![ddc_observed("SN_RIGHT", Some("card1-DP-1"), true)],
        };

        assert_eq!(
            reconcile(&configured, &observed),
            vec![ReconcileAction::Present]
        );
    }

    #[test]
    fn ddc_primary_remains_single_effective_target_across_snapshot() {
        let configured = vec![
            ddc_monitor("external-ddc", "SN_RIGHT"),
            backlight_monitor("external-ddcci", "ddcci_backlight_0"),
        ];
        let snapshot = CapabilitySnapshot {
            ddc_present: vec![ddc_observed("SN_RIGHT", Some("card1-DP-2"), true)],
            backlights_present: vec![backlight_observed("ddcci_backlight_0", Some("card1-DP-2"))],
        };

        assert_eq!(
            reconcile(&configured, &snapshot),
            vec![
                ReconcileAction::Present,
                ReconcileAction::SuppressedProvenAlias
            ]
        );
        assert_eq!(
            reconcile(&configured, &snapshot)
                .into_iter()
                .filter(|action| action.allowed_to_apply())
                .count(),
            1
        );
    }

    #[test]
    fn external_bus_changed_but_serial_proves_same_monitor_returns_present() {
        // The monitor comes back on a different I2C bus (bus 9 instead of 7),
        // but the serial-selector is authoritative and matches.
        let configured = vec![ddc_monitor("ddc", "SN_ABC")];
        let mut observed_monitor = ddc_observed("SN_ABC", Some("card1-DP-2"), true);
        observed_monitor.bus_number = Some(9);
        observed_monitor.connector = Some(String::from("card1-DP-2"));
        let observed = CapabilitySnapshot {
            backlights_present: vec![],
            ddc_present: vec![observed_monitor],
        };
        let actions = reconcile(&configured, &observed);
        assert_eq!(actions, vec![ReconcileAction::Present]);
    }

    #[test]
    fn ambiguous_ddc_unknown_selector_fails_closed() {
        // A configured DDC monitor whose serial matches nothing observed is
        // never promoted to Present; the runtime has no topology to prove it.
        let configured = vec![ddc_monitor("ddc", "UNKNOWN_SN")];
        let observed = CapabilitySnapshot {
            backlights_present: vec![],
            ddc_present: vec![ddc_observed("SN_DIFFERENT", Some("card1-DP-1"), true)],
        };
        let actions = reconcile(&configured, &observed);
        assert!(!actions[0].allowed_to_apply());
    }

    // -----------------------------------------------------------------
    // DDC-primary / ddcci-alias duplicate suppression
    // -----------------------------------------------------------------

    #[test]
    fn manual_ddc_plus_ddcci_duplicate_suppresses_second_write() {
        // Manual config of the same physical panel via DDC and ddcci backlight.
        let configured = vec![
            ddc_monitor("external-ddc", "SN_ABC"),
            backlight_monitor("external-backlight", "ddcci_backlight_0"),
        ];
        let observed = CapabilitySnapshot {
            ddc_present: vec![ddc_observed("SN_ABC", Some("card1-DP-1"), true)],
            backlights_present: vec![backlight_observed("ddcci_backlight_0", Some("card1-DP-1"))],
        };
        let actions = reconcile(&configured, &observed);
        assert_eq!(
            actions,
            vec![
                ReconcileAction::Present,
                ReconcileAction::SuppressedProvenAlias
            ]
        );
        assert!(actions[0].allowed_to_apply());
        assert!(!actions[1].allowed_to_apply(), "alias must not write");
    }

    #[test]
    fn ddcci_fallback_when_ddc_unavailable() {
        // DDC is not present/viable, but the ddcci backlight is. The backlight
        // must remain eligible (fallback target), not become unavailable.
        let configured = vec![
            ddc_monitor("external-ddc", "SN_ABC"),
            backlight_monitor("external-backlight", "ddcci_backlight_0"),
        ];
        let observed = CapabilitySnapshot {
            ddc_present: vec![],
            backlights_present: vec![backlight_observed("ddcci_backlight_0", Some("card1-DP-1"))],
        };
        let actions = reconcile(&configured, &observed);
        assert_eq!(
            actions,
            vec![
                ReconcileAction::TemporarilyUnavailable,
                ReconcileAction::Present
            ]
        );
        assert!(!actions[0].allowed_to_apply());
        assert!(actions[1].allowed_to_apply());
    }

    #[test]
    fn no_duplicate_inference_from_names_or_type_raw() {
        // Even though the backlight is named ddcci_backlight_0 (a name), and
        // has type=raw, without authoritative connector evidence it is NOT an
        // alias and must remain Present alongside the DDC monitor.
        let configured = vec![
            ddc_monitor("external-ddc", "SN_ABC"),
            backlight_monitor("external-backlight", "ddcci_backlight_0"),
        ];
        let mut observed_backlight = backlight_observed("ddcci_backlight_0", None);
        observed_backlight.backlight_type = Some(String::from("raw"));
        let observed = CapabilitySnapshot {
            ddc_present: vec![ddc_observed("SN_ABC", Some("card1-DP-1"), true)],
            backlights_present: vec![observed_backlight],
        };
        let actions = reconcile(&configured, &observed);
        assert_eq!(
            actions,
            vec![ReconcileAction::Present, ReconcileAction::Present]
        );
    }

    #[test]
    fn unconfigured_observed_device_is_not_auto_controlled() {
        // The snapshot contains valid hardware (internal panel) but the config
        // has no entry for it: the effective set must not adopt it.
        let configured = vec![ddc_monitor("external-ddc", "SN_ABC")];
        let observed = CapabilitySnapshot {
            ddc_present: vec![ddc_observed("SN_ABC", Some("card1-DP-1"), true)],
            backlights_present: vec![backlight_observed("unexpected_panel", None)],
        };
        let actions = reconcile(&configured, &observed);
        // The only configured monitor is present; nothing is auto-adopted.
        assert_eq!(actions, vec![ReconcileAction::Present]);
    }
}

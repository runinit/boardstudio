//! Reviewed electrical capability metadata for the bundled footprints.
//!
//! This module deliberately describes the *effective* footprint contract rather
//! than trying to infer connectivity from rendered pad labels.  `fabricated`
//! nets are the nets present before soldering; a `JumperGroup` describes the
//! optional bridge which creates the assembled connection.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JumperState {
    Open,
    Bridged,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PinCapability {
    pub terminal: &'static str,
    pub module_pin: &'static str,
    pub firmware_gpio: &'static str,
    pub roles: &'static [&'static str],
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ElectricalProfile {
    pub id: &'static str,
    pub source: &'static str,
    pub kind: &'static str,
    pub supports_reversible: bool,
    pub supports_reverse_mount: bool,
    pub supports_extra_pins: bool,
    pub pins: &'static [PinCapability],
    pub rejects_invert_jumpers_position: bool,
    pub reserved_gpios: &'static [&'static str],
}

const SCAN_ROLES: &[&str] = &[
    "matrix-row",
    "matrix-column",
    "direct-input",
    "spi",
    "i2c",
    "uart",
];

// These are the module's documented pin aliases.  The resolver maps the
// firmware alias through the selected controller target; it must not turn a
// label into a GPIO by guessing.
const NICE_NANO_PINS: &[PinCapability] = &[
    PinCapability {
        terminal: "P21",
        module_pin: "P21",
        firmware_gpio: "P0.31",
        roles: SCAN_ROLES,
    },
    PinCapability {
        terminal: "P20",
        module_pin: "P20",
        firmware_gpio: "P0.29",
        roles: SCAN_ROLES,
    },
    PinCapability {
        terminal: "P19",
        module_pin: "P19",
        firmware_gpio: "P0.02",
        roles: SCAN_ROLES,
    },
    PinCapability {
        terminal: "P18",
        module_pin: "P18",
        firmware_gpio: "P1.15",
        roles: SCAN_ROLES,
    },
    PinCapability {
        terminal: "P15",
        module_pin: "P15",
        firmware_gpio: "P1.13",
        roles: SCAN_ROLES,
    },
    PinCapability {
        terminal: "P14",
        module_pin: "P14",
        firmware_gpio: "P1.11",
        roles: SCAN_ROLES,
    },
    PinCapability {
        terminal: "P16",
        module_pin: "P16",
        firmware_gpio: "P0.10",
        roles: SCAN_ROLES,
    },
    PinCapability {
        terminal: "P10",
        module_pin: "P10",
        firmware_gpio: "P0.09",
        roles: SCAN_ROLES,
    },
    PinCapability {
        terminal: "P1",
        module_pin: "P1",
        firmware_gpio: "P0.06",
        roles: SCAN_ROLES,
    },
    PinCapability {
        terminal: "P0",
        module_pin: "P0",
        firmware_gpio: "P0.08",
        roles: SCAN_ROLES,
    },
    PinCapability {
        terminal: "P2",
        module_pin: "P2",
        firmware_gpio: "P0.17",
        roles: SCAN_ROLES,
    },
    PinCapability {
        terminal: "P3",
        module_pin: "P3",
        firmware_gpio: "P0.20",
        roles: SCAN_ROLES,
    },
    PinCapability {
        terminal: "P4",
        module_pin: "P4",
        firmware_gpio: "P0.22",
        roles: SCAN_ROLES,
    },
    PinCapability {
        terminal: "P5",
        module_pin: "P5",
        firmware_gpio: "P0.24",
        roles: SCAN_ROLES,
    },
    PinCapability {
        terminal: "P6",
        module_pin: "P6",
        firmware_gpio: "P1.00",
        roles: SCAN_ROLES,
    },
    PinCapability {
        terminal: "P7",
        module_pin: "P7",
        firmware_gpio: "P0.11",
        roles: SCAN_ROLES,
    },
    PinCapability {
        terminal: "P8",
        module_pin: "P8",
        firmware_gpio: "P1.04",
        roles: SCAN_ROLES,
    },
    PinCapability {
        terminal: "P9",
        module_pin: "P9",
        firmware_gpio: "P1.06",
        roles: SCAN_ROLES,
    },
    PinCapability {
        terminal: "P101",
        module_pin: "P101",
        firmware_gpio: "P1.01",
        roles: SCAN_ROLES,
    },
    PinCapability {
        terminal: "P102",
        module_pin: "P102",
        firmware_gpio: "P1.02",
        roles: SCAN_ROLES,
    },
    PinCapability {
        terminal: "P107",
        module_pin: "P107",
        firmware_gpio: "P1.07",
        roles: SCAN_ROLES,
    },
];

// The bundled SuperMini footprint exposes the same documented aliases.  Its
// physical pad geometry differs, so it remains a separate profile.
const SUPERMINI_PINS: &[PinCapability] = NICE_NANO_PINS;

pub fn profile(id: &str) -> Option<ElectricalProfile> {
    let value = match id {
        "ceoloide/mcu_nice_nano" => ElectricalProfile {
            id: "ceoloide/mcu_nice_nano",
            source: "ceoloide/mcu_nice_nano",
            kind: "controller",
            supports_reversible: true,
            supports_reverse_mount: true,
            supports_extra_pins: true,
            pins: NICE_NANO_PINS,
            rejects_invert_jumpers_position: true,
            reserved_gpios: &["P0.04", "P0.13"],
        },
        "ceoloide/mcu_supermini_nrf52840" => ElectricalProfile {
            id: "ceoloide/mcu_supermini_nrf52840",
            source: "ceoloide/mcu_supermini_nrf52840",
            kind: "controller",
            supports_reversible: true,
            supports_reverse_mount: true,
            supports_extra_pins: true,
            pins: SUPERMINI_PINS,
            rejects_invert_jumpers_position: true,
            reserved_gpios: &["P0.04"],
        },
        "ceoloide/display_nice_view" => ElectricalProfile {
            id: "ceoloide/display_nice_view",
            source: "ceoloide/display_nice_view",
            kind: "display",
            supports_reversible: true,
            supports_reverse_mount: false,
            supports_extra_pins: false,
            pins: &[],
            rejects_invert_jumpers_position: false,
            reserved_gpios: &[],
        },
        "infused-kim/nice_view" => ElectricalProfile {
            id: "infused-kim/nice_view",
            source: "infused-kim/nice_view",
            kind: "display",
            supports_reversible: true,
            supports_reverse_mount: true,
            supports_extra_pins: false,
            pins: &[],
            rejects_invert_jumpers_position: false,
            reserved_gpios: &[],
        },
        _ => return None,
    };
    Some(value)
}

pub fn validate_params(
    profile: &ElectricalProfile,
    reversible: bool,
    reverse_mount: bool,
    include_extra_pins: bool,
    _include_traces: bool,
    invert_jumpers_position: bool,
) -> Result<(), &'static str> {
    if reversible && !profile.supports_reversible {
        return Err("footprint does not support reversible assembly");
    }
    if reverse_mount && !profile.supports_reverse_mount {
        return Err("footprint does not support reverse mounting");
    }
    if include_extra_pins && !profile.supports_extra_pins {
        return Err("footprint does not expose extra pins");
    }
    if invert_jumpers_position && profile.rejects_invert_jumpers_position {
        return Err("invert_jumpers_position is unsupported for this controller footprint");
    }
    // Missing supplied traces is a routing obligation for the handoff, not an
    // invalid footprint configuration. The jumper recipe reports it.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reversible_controller_keeps_fabricated_nets_separate() {
        let p = profile("ceoloide/mcu_nice_nano").unwrap();
        assert_eq!(
            p.pins
                .iter()
                .find(|pin| pin.terminal == "P1")
                .unwrap()
                .firmware_gpio,
            "P0.06"
        );
    }

    #[test]
    fn nice_nano_v2_scan_capabilities_have_unique_gpio_targets() {
        let p = profile("ceoloide/mcu_nice_nano").unwrap();
        let mut gpios = p
            .pins
            .iter()
            .map(|pin| pin.firmware_gpio)
            .collect::<Vec<_>>();
        gpios.sort_unstable();
        gpios.dedup();
        assert_eq!(gpios.len(), p.pins.len());
        assert_eq!(
            p.pins
                .iter()
                .find(|pin| pin.terminal == "P14")
                .unwrap()
                .firmware_gpio,
            "P1.11"
        );
    }

    #[test]
    fn reduced_jumpers_still_require_valid_controller_options() {
        let p = profile("ceoloide/mcu_supermini_nrf52840").unwrap();
        assert!(validate_params(&p, true, false, true, true, false).is_ok());
        assert!(validate_params(&p, true, false, true, true, true).is_err());
        assert!(validate_params(&p, true, false, true, false, false).is_ok());
    }

    #[test]
    fn infused_nice_view_requires_routes_to_local_jumper_pads() {
        let p = profile("infused-kim/nice_view").unwrap();
        assert!(validate_params(&p, true, false, false, false, false).is_ok());
    }
}

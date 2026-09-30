//! Deterministic ZMK handoff generation.
//!
//! The generator accepts resolved electrical assignments only. It never invents
//! GPIO names from labels: every scan pin must resolve through an electrical
//! profile before files are emitted.

use crate::electrical_profiles::profile;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum SplitTransport {
    Wireless,
    WiredUart,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
pub struct ScanPin {
    pub terminal: String,
    pub gpio: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
pub struct FirmwareKey {
    pub id: String,
    pub row: usize,
    pub column: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
pub struct FirmwareRequest {
    pub controller_profile: String,
    pub board_name: String,
    pub rows: Vec<ScanPin>,
    pub columns: Vec<ScanPin>,
    pub keys: Vec<FirmwareKey>,
    pub diode_direction: String,
    #[serde(default)]
    pub mode: FirmwareScanMode,
    #[serde(default)]
    pub direct_pins: Vec<ScanPin>,
    #[serde(default)]
    pub auxiliary_pins: Vec<ScanPin>,
    #[serde(default)]
    pub key_bindings: Vec<String>,
    #[serde(default)]
    pub peripheral_config: Vec<String>,
    pub transport: Option<SplitTransport>,
    #[serde(default)]
    pub uart_tx: Option<ScanPin>,
    #[serde(default)]
    pub uart_rx: Option<ScanPin>,
    #[serde(default)]
    pub peripheral_overlays: Vec<String>,
    #[serde(default)]
    pub peripheral: Option<Box<FirmwareRequest>>,
    #[serde(default)]
    pub matrix_row_offset: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum FirmwareScanMode {
    #[default]
    Matrix,
    Direct,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
pub struct FirmwarePackage {
    pub files: BTreeMap<String, String>,
    pub warnings: Vec<String>,
}

fn validate_half(request: &FirmwareRequest) -> Result<(), String> {
    let controller = profile(&request.controller_profile).ok_or_else(|| {
        format!(
            "unsupported controller profile: {}",
            request.controller_profile
        )
    })?;
    let known = |pin: &ScanPin| {
        controller.pins.iter().any(|candidate| {
            candidate.terminal == pin.terminal && candidate.firmware_gpio == pin.gpio
        })
    };
    if request
        .rows
        .iter()
        .chain(request.columns.iter())
        .chain(request.direct_pins.iter())
        .chain(request.auxiliary_pins.iter())
        .any(|pin| !known(pin))
    {
        return Err("every scan pin must resolve through the selected controller profile".into());
    }
    if request
        .rows
        .iter()
        .chain(request.columns.iter())
        .chain(request.direct_pins.iter())
        .chain(request.auxiliary_pins.iter())
        .any(|pin| controller.reserved_gpios.contains(&pin.gpio.as_str()))
    {
        return Err(
            "scan assignment uses a controller GPIO reserved for power or battery sensing".into(),
        );
    }
    if request.mode == FirmwareScanMode::Matrix
        && (request.rows.is_empty() || request.columns.is_empty())
    {
        return Err("matrix firmware requires at least one row and column".into());
    }
    if request.mode == FirmwareScanMode::Direct && request.direct_pins.is_empty() {
        return Err("direct firmware requires at least one GPIO input".into());
    }
    if request.mode == FirmwareScanMode::Matrix
        && request.keys.iter().any(|key| {
            if key.row < request.rows.len() {
                key.column >= request.columns.len()
            } else {
                key.row != request.rows.len() || key.column >= request.auxiliary_pins.len()
            }
        })
    {
        return Err("key scan position is outside the resolved matrix".into());
    }
    if !request.key_bindings.is_empty() && request.key_bindings.len() != request.keys.len() {
        return Err("Key bindings must match the current layout positions".into());
    }
    if request.key_bindings.iter().any(|binding| {
        binding != "&none"
            && binding != "&trans"
            && !binding.strip_prefix("&kp ").is_some_and(|key| {
                !key.is_empty()
                    && key
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            })
    }) {
        return Err("Unsupported key binding".into());
    }
    let mut seen_gpio = std::collections::BTreeSet::new();
    if request
        .rows
        .iter()
        .chain(request.columns.iter())
        .chain(request.direct_pins.iter())
        .chain(request.auxiliary_pins.iter())
        .any(|pin| !seen_gpio.insert(pin.gpio.as_str()))
    {
        return Err("a GPIO is assigned to more than one firmware function".into());
    }
    let mut seen_rc = std::collections::BTreeSet::new();
    if request.mode == FirmwareScanMode::Matrix
        && request
            .keys
            .iter()
            .any(|key| !seen_rc.insert((key.row, key.column)))
    {
        return Err("two keys share the same matrix row/column address".into());
    }
    if request.diode_direction != "col2row" && request.diode_direction != "row2col" {
        return Err("diode direction must be col2row or row2col".into());
    }
    if matches!(request.transport, Some(SplitTransport::WiredUart)) {
        let (Some(tx), Some(rx)) = (&request.uart_tx, &request.uart_rx) else {
            return Err("wired split requires explicit UART TX and RX pins".into());
        };
        if !known(tx)
            || !known(rx)
            || tx.gpio == rx.gpio
            || seen_gpio.contains(tx.gpio.as_str())
            || seen_gpio.contains(rx.gpio.as_str())
        {
            return Err(
                "wired split UART pins must be distinct pins from the controller profile".into(),
            );
        }
    }
    Ok(())
}

pub fn generate(request: &FirmwareRequest) -> Result<FirmwarePackage, String> {
    validate_half(request)?;
    let transport = request.transport.as_ref().map(|mode| match mode {
        SplitTransport::Wireless => "wireless",
        SplitTransport::WiredUart => "wired-uart",
    });
    if transport.is_some() && request.peripheral.is_none() {
        return Err("split firmware requires a distinct peripheral plan".into());
    }
    if let Some(peripheral) = request.peripheral.as_deref() {
        if peripheral.peripheral.is_some() || peripheral.transport != request.transport {
            return Err("Both halves must use the same split transport".into());
        }
        validate_half(peripheral)?;
    }
    let local_rows = |half: &FirmwareRequest| {
        (if half.mode == FirmwareScanMode::Direct {
            1
        } else {
            half.rows.len()
        }) + usize::from(!half.auxiliary_pins.is_empty())
    };
    let local_columns = |half: &FirmwareRequest| {
        (if half.mode == FirmwareScanMode::Direct {
            half.direct_pins.len()
        } else {
            half.columns.len()
        })
        .max(half.auxiliary_pins.len())
    };
    let right_offset = local_rows(request);
    let global_rows = right_offset + request.peripheral.as_deref().map(local_rows).unwrap_or(0);
    let global_columns = local_columns(request).max(
        request
            .peripheral
            .as_deref()
            .map(local_columns)
            .unwrap_or(0),
    );
    let mut global_keys = request.keys.clone();
    if let Some(peripheral) = request.peripheral.as_deref() {
        global_keys.extend(peripheral.keys.iter().map(|key| FirmwareKey {
            id: format!("right/{}", key.id),
            row: key.row + right_offset,
            column: key.column,
        }));
    }
    let mut files = BTreeMap::new();
    files.insert("config/boards/shields/boardstudio/Kconfig.shield".into(), "config SHIELD_BOARDSTUDIO\n    def_bool $(shields_list_contains,boardstudio)\n\nconfig SHIELD_BOARDSTUDIO_LEFT\n    def_bool $(shields_list_contains,boardstudio_left)\n\nconfig SHIELD_BOARDSTUDIO_RIGHT\n    def_bool $(shields_list_contains,boardstudio_right)\n".into());
    let mut all_keys = request.keys.clone();
    if let Some(peripheral) = request.peripheral.as_deref() {
        all_keys.extend(peripheral.keys.clone());
    }
    files.insert(
        "config/boards/shields/boardstudio/boardstudio.keymap".into(),
        keymap(&all_keys, request),
    );
    let overlay_text = overlay(
        request,
        transport,
        &global_keys,
        global_rows,
        global_columns,
        0,
    );
    files.insert(
        "config/boards/shields/boardstudio/boardstudio.overlay".into(),
        overlay_text.clone(),
    );
    if transport.is_some() {
        files.insert(
            "config/boards/shields/boardstudio/boardstudio_left.overlay".into(),
            overlay_text,
        );
        if let Some(peripheral) = request.peripheral.as_deref() {
            files.insert(
                "config/boards/shields/boardstudio/boardstudio_right.overlay".into(),
                overlay(
                    peripheral,
                    transport,
                    &global_keys,
                    global_rows,
                    global_columns,
                    right_offset,
                ),
            );
        }
    }
    files.insert(
        "config/boards/shields/boardstudio/README.md".into(),
        instructions(request, transport),
    );
    files.insert("config/west.yml".into(), "manifest:\n  remotes:\n    - name: zmkfirmware\n      url-base: https://github.com/zmkfirmware\n  projects:\n    - name: zmk\n      remote: zmkfirmware\n      revision: v0.3.0\n      import: app/west.yml\n".into());
    files.insert("build.yaml".into(), build_yaml(request));
    files.insert("build-local.sh".into(), local_build_script(request));
    files.insert("README.md".into(), "# Build ZMK locally\n\nInstall the ZMK v0.3.0 local toolchain (west, CMake, Ninja, the Zephyr SDK, and Python dependencies). Extract this package into an empty directory, then run `sh build-local.sh`. The script initializes an isolated west workspace, fetches the pinned manifest, and builds each shield into a separate directory. Firmware outputs are under `build/<shield>/zephyr/zmk.uf2`. Existing workspaces with a different manifest are rejected. See `config/boards/shields/boardstudio/README.md` for hardware and wiring details.\n".into());
    files.insert(".github/workflows/build.yml".into(), "name: Build firmware\non: [push, pull_request, workflow_dispatch]\njobs:\n  build:\n    uses: zmkfirmware/zmk/.github/workflows/build-user-config.yml@v0.3.0\n".into());
    if let Some(transport) = transport {
        files.insert(
            "config/boards/shields/boardstudio/Kconfig.defconfig".into(),
            format!(
                "if SHIELD_BOARDSTUDIO_LEFT\n{}\nendif\nif SHIELD_BOARDSTUDIO_RIGHT\n{}\nendif\n",
                role_kconfig(true, transport),
                role_kconfig(false, transport)
            ),
        );
        files.insert(
            "config/boards/shields/boardstudio/boardstudio_left.conf".into(),
            format!(
                "{}\n{}",
                role_conf(transport),
                request.peripheral_config.join("\n")
            ),
        );
        files.insert(
            "config/boards/shields/boardstudio/boardstudio_right.conf".into(),
            format!(
                "{}\n{}",
                role_conf(transport),
                request
                    .peripheral
                    .as_ref()
                    .map(|half| half.peripheral_config.join("\n"))
                    .unwrap_or_default()
            ),
        );
    }
    if transport.is_none() {
        files.insert(
            "config/boards/shields/boardstudio/boardstudio.conf".into(),
            request.peripheral_config.join("\n"),
        );
    }
    Ok(FirmwarePackage {
        files,
        warnings: Vec::new(),
    })
}

fn overlay(
    request: &FirmwareRequest,
    transport: Option<&str>,
    keys: &[FirmwareKey],
    rows_count: usize,
    columns_count: usize,
    row_offset: usize,
) -> String {
    let map = keys
        .iter()
        .map(|key| format!("RC({}, {})", key.row, key.column))
        .collect::<Vec<_>>()
        .join(" ");
    let transform = format!(
        "matrix_transform0: matrix_transform0 {{ compatible = \"zmk,matrix-transform\"; rows = <{rows_count}>; columns = <{columns_count}>; row-offset = <{row_offset}>; map = <{map}>; }};"
    );
    let split_node = match transport {
        Some("wired-uart") => {
            let pins = match (&request.uart_tx, &request.uart_rx) {
                (Some(tx), Some(rx)) => format!(
                    "\n&pinctrl {{ uart0_default: uart0_default {{ group1 {{ psels = <NRF_PSEL(UART_TX, {}, {})>, <NRF_PSEL(UART_RX, {}, {})>; }}; }}; }};\n&uart0 {{ pinctrl-0 = <&uart0_default>; pinctrl-names = \"default\"; status = \"okay\"; }};",
                    nrf_port(&tx.gpio),
                    nrf_pin(&tx.gpio),
                    nrf_port(&rx.gpio),
                    nrf_pin(&rx.gpio)
                ),
                _ => "".into(),
            };
            format!(
                "\n&pro_micro_serial {{ status = \"okay\"; current-speed = <115200>; }};\n/ {{ wired_split {{ compatible = \"zmk,wired-split\"; device = <&pro_micro_serial>; }}; }};{}\n",
                pins
            )
        }
        _ => String::new(),
    };
    let scanner = if request.mode == FirmwareScanMode::Direct {
        let inputs = request
            .direct_pins
            .iter()
            .map(|pin| gpio_spec(&pin.gpio))
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "kscan0: kscan0 {{ compatible = \"zmk,kscan-gpio-direct\"; wakeup-source; input-gpios = <{inputs}>; }};"
        )
    } else {
        let (row_flags, column_flags) = if request.diode_direction == "col2row" {
            ("GPIO_PULL_DOWN", "0")
        } else {
            ("0", "GPIO_PULL_DOWN")
        };
        let rows = request
            .rows
            .iter()
            .map(|pin| gpio_spec_flags(&pin.gpio, row_flags))
            .collect::<Vec<_>>()
            .join(" ");
        let columns = request
            .columns
            .iter()
            .map(|pin| gpio_spec_flags(&pin.gpio, column_flags))
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "kscan0: kscan0 {{ compatible = \"zmk,kscan-gpio-matrix\"; wakeup-source; diode-direction = \"{}\"; row-gpios = <{rows}>; col-gpios = <{columns}>; }};",
            request.diode_direction
        )
    };
    let extra_scan = if request.auxiliary_pins.is_empty() {
        String::new()
    } else {
        let pins = request
            .auxiliary_pins
            .iter()
            .map(|pin| gpio_spec(&pin.gpio))
            .collect::<Vec<_>>()
            .join(" ");
        let offset = if request.mode == FirmwareScanMode::Direct {
            1
        } else {
            request.rows.len()
        };
        format!(
            "kscan_extra: kscan_extra {{ compatible = \"zmk,kscan-gpio-direct\"; input-gpios = <{pins}>; }}; kscan_combo: kscan_combo {{ compatible = \"zmk,kscan-composite\"; rows = <{}>; columns = <{columns_count}>; main {{ kscan = <&kscan0>; }}; buttons {{ kscan = <&kscan_extra>; row-offset = <{offset}>; }}; }};",
            offset + 1
        )
    };
    let chosen = if request.auxiliary_pins.is_empty() {
        "kscan0"
    } else {
        "kscan_combo"
    };
    format!(
        "#include <zephyr/dt-bindings/gpio/gpio.h>\n#include <dt-bindings/zmk/matrix_transform.h>\n#include <zephyr/dt-bindings/pinctrl/nrf-pinctrl.h>\n/ {{ chosen {{ zmk,kscan = &{chosen}; zmk,matrix-transform = &matrix_transform0; }}; {transform} {scanner} {extra_scan} }};\n{split_node}\n{}",
        request.peripheral_overlays.join("\n")
    )
}

fn nrf_port(gpio: &str) -> String {
    gpio.split('.')
        .next()
        .and_then(|p| p.strip_prefix('P'))
        .unwrap_or("0")
        .to_string()
}
fn nrf_pin(gpio: &str) -> String {
    gpio.split('.')
        .nth(1)
        .and_then(|pin| pin.parse::<u32>().ok())
        .unwrap_or(0)
        .to_string()
}

fn gpio_spec(gpio: &str) -> String {
    let mut parts = gpio.split('.');
    match (parts.next(), parts.next(), parts.next()) {
        // Direct keys are wired to the scan return/common rail. Keep the
        // input biased high and report a pressed key as active-low, matching
        // the matrix pull-up convention and avoiding a floating input.
        (Some(port), Some(pin), None) if port.starts_with('P') => format!(
            "&gpio{} {} (GPIO_ACTIVE_LOW | GPIO_PULL_UP)",
            &port[1..],
            pin.parse::<u32>().expect("reviewed GPIO pin")
        ),
        _ => format!("/* unresolved GPIO {} */", gpio),
    }
}

fn gpio_spec_flags(gpio: &str, flags: &str) -> String {
    let mut parts = gpio.split('.');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(port), Some(pin), None) if port.starts_with('P') => format!(
            "&gpio{} {} (GPIO_ACTIVE_HIGH | {})",
            &port[1..],
            pin.parse::<u32>().expect("reviewed GPIO pin"),
            flags
        ),
        _ => format!("/* unresolved GPIO {} */", gpio),
    }
}

fn keymap(keys: &[FirmwareKey], request: &FirmwareRequest) -> String {
    let mut values = if request.key_bindings.is_empty() {
        vec!["&none".to_string(); request.keys.len()]
    } else {
        request.key_bindings.clone()
    };
    if let Some(half) = &request.peripheral {
        values.extend(if half.key_bindings.is_empty() {
            vec!["&none".to_string(); half.keys.len()]
        } else {
            half.key_bindings.clone()
        });
    }
    debug_assert_eq!(values.len(), keys.len());
    let bindings = values.join(" ");
    format!(
        "#include <behaviors.dtsi>\n#include <dt-bindings/zmk/keys.h>\n\n/ {{\n    keymap {{ compatible = \"zmk,keymap\"; default_layer {{ bindings = <{}>; }}; }};\n}};\n",
        bindings
    )
}

fn local_build_script(request: &FirmwareRequest) -> String {
    let shields = if request.transport.is_some() {
        vec!["boardstudio_left", "boardstudio_right"]
    } else {
        vec!["boardstudio"]
    };
    let mut script = r#"#!/bin/sh
set -eu
package_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$package_dir"
command -v west >/dev/null 2>&1 || { echo 'Install the ZMK v0.3.0 local toolchain and west first.' >&2; exit 1; }
if [ ! -d .west ]; then
  if west topdir >/dev/null 2>&1; then
    echo 'Extract this package outside an existing west workspace.' >&2; exit 1
  fi
  west init -l config
else
  manifest_dir=$(west config manifest.path)
  [ "$manifest_dir" = config ] || { echo 'This workspace uses a different manifest.' >&2; exit 1; }
fi
west update
west zephyr-export
"#.to_string();
    for shield in shields {
        script.push_str(&format!("west build -s zmk/app -d build/{shield} -b nice_nano_v2 -- -DSHIELD={shield} -DZMK_CONFIG=\"$package_dir/config\"\n"));
    }
    script
}

fn instructions(request: &FirmwareRequest, transport: Option<&str>) -> String {
    format!(
        "# BoardStudio ZMK handoff\n\n- Target: ZMK v0.3.0\n- Controller: `{}`\n- Matrix: {} rows x {} columns\n- Diodes: `{}`\n- Split transport: `{}`\n\nUnassigned positions use `&none`; edit bindings in BoardStudio or the generated keymap before building. The included workflow uses ZMK v0.3.0. For a local build, install the ZMK v0.3.0 toolchain and run `sh build-local.sh` from the package root. Builds use isolated directories for each shield. Verify the PCB jumper recipe and local routing obligations before assembly.\n",
        request.controller_profile,
        request.rows.len(),
        request.columns.len(),
        request.diode_direction,
        transport.unwrap_or("unibody")
    )
}

fn build_yaml(request: &FirmwareRequest) -> String {
    if request.transport.is_some() {
        "include:\n  - board: nice_nano_v2\n    shield: boardstudio_left\n    artifact-name: boardstudio_left\n  - board: nice_nano_v2\n    shield: boardstudio_right\n    artifact-name: boardstudio_right\n".into()
    } else {
        "include:\n  - board: nice_nano_v2\n    shield: boardstudio\n    artifact-name: boardstudio\n".into()
    }
}

fn role_kconfig(central: bool, transport: &str) -> String {
    let role = if central { "y" } else { "n" };
    let peripherals = if transport == "wireless" && central {
        "\nconfig ZMK_SPLIT_BLE_CENTRAL_PERIPHERALS\n    default 1"
    } else {
        ""
    };
    format!(
        "config ZMK_SPLIT\n    default y\nconfig ZMK_SPLIT_ROLE_CENTRAL\n    default {}{}\n",
        role, peripherals
    )
}

fn role_conf(transport: &str) -> String {
    if transport == "wireless" {
        "CONFIG_ZMK_SPLIT=y\nCONFIG_ZMK_SPLIT_BLE=y\n".into()
    } else {
        "CONFIG_ZMK_SPLIT=y\nCONFIG_ZMK_SPLIT_WIRED=y\nCONFIG_ZMK_SPLIT_WIRED_UART_MODE_INTERRUPT=y\n".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> FirmwareRequest {
        FirmwareRequest {
            controller_profile: "ceoloide/mcu_nice_nano".into(),
            board_name: "test".into(),
            rows: vec![ScanPin {
                terminal: "P21".into(),
                gpio: "P0.31".into(),
            }],
            columns: vec![ScanPin {
                terminal: "P20".into(),
                gpio: "P0.29".into(),
            }],
            keys: vec![FirmwareKey {
                id: "k1".into(),
                row: 0,
                column: 0,
            }],
            diode_direction: "col2row".into(),
            mode: FirmwareScanMode::Matrix,
            direct_pins: Vec::new(),
            auxiliary_pins: vec![],
            key_bindings: vec![],
            peripheral_config: vec![],
            transport: None,
            uart_tx: None,
            uart_rx: None,
            peripheral_overlays: Vec::new(),
            peripheral: None,
            matrix_row_offset: 0,
        }
    }
    #[test]
    fn exports_reproducible_local_build_commands_for_each_half() {
        let single = generate(&request()).unwrap();
        let script = single
            .files
            .get("build-local.sh")
            .expect("local build entry point");
        assert!(script.contains("west init -l config"));
        assert!(script.contains("west update"));
        assert!(script.contains("-s zmk/app"));
        assert!(script.contains("-DSHIELD=boardstudio"));
        assert!(script.contains("ZMK_CONFIG"));
        let mut split = request();
        split.transport = Some(SplitTransport::Wireless);
        let mut peripheral = request();
        peripheral.transport = split.transport.clone();
        split.peripheral = Some(Box::new(peripheral));
        let package = generate(&split).unwrap();
        let script = &package.files["build-local.sh"];
        assert!(script.contains("build/boardstudio_left"));
        assert!(script.contains("build/boardstudio_right"));
    }

    #[test]
    fn emits_pinned_manifest_and_compact_keymap() {
        let p = generate(&request()).unwrap();
        assert!(p.files["config/west.yml"].contains("v0.3.0"));
        assert!(p.files["config/boards/shields/boardstudio/boardstudio.keymap"].contains("&none"));
    }
    #[test]
    fn rejects_unresolved_gpio() {
        let mut r = request();
        r.rows[0].gpio = "P99.99".into();
        assert!(generate(&r).is_err());
    }
    #[test]
    fn rejects_invalid_scan_shape() {
        let mut r = request();
        r.keys[0].row = 2;
        assert!(generate(&r).is_err());
    }
    #[test]
    fn rejects_duplicate_gpio_and_matrix_address() {
        let mut r = request();
        r.columns[0].gpio = r.rows[0].gpio.clone();
        assert!(generate(&r).is_err());
        let mut r = request();
        r.keys.push(FirmwareKey {
            id: "k2".into(),
            row: 0,
            column: 0,
        });
        assert!(generate(&r).is_err());
    }
    #[test]
    fn emits_direction_specific_pull_flags() {
        let p = generate(&request()).unwrap();
        let overlay = &p.files["config/boards/shields/boardstudio/boardstudio.overlay"];
        assert!(overlay.contains("GPIO_PULL_DOWN"));
        assert!(overlay.contains("GPIO_ACTIVE_HIGH | 0"));
    }
    #[test]
    fn direct_mode_does_not_require_matrix_dimensions() {
        let mut r = request();
        r.mode = FirmwareScanMode::Direct;
        r.rows.clear();
        r.columns.clear();
        r.keys.clear();
        r.direct_pins = vec![ScanPin {
            terminal: "P21".into(),
            gpio: "P0.31".into(),
        }];
        let p = generate(&r).unwrap();
        let overlay = &p.files["config/boards/shields/boardstudio/boardstudio.overlay"];
        assert!(overlay.contains("zmk,kscan-gpio-direct"));
        assert!(!overlay.contains(", &gpio"));
    }
    #[test]
    fn direct_inputs_are_active_low_pullups() {
        let mut r = request();
        r.mode = FirmwareScanMode::Direct;
        r.rows.clear();
        r.columns.clear();
        r.keys.clear();
        r.direct_pins = vec![ScanPin {
            terminal: "P21".into(),
            gpio: "P0.31".into(),
        }];
        let p = generate(&r).unwrap();
        let overlay = &p.files["config/boards/shields/boardstudio/boardstudio.overlay"];
        assert!(overlay.contains("GPIO_ACTIVE_LOW | GPIO_PULL_UP"));
    }
    #[test]
    fn split_package_contains_real_role_configuration() {
        let mut r = request();
        r.transport = Some(SplitTransport::WiredUart);
        r.uart_tx = Some(ScanPin {
            terminal: "P1".into(),
            gpio: "P0.06".into(),
        });
        r.uart_rx = Some(ScanPin {
            terminal: "P0".into(),
            gpio: "P0.08".into(),
        });
        let mut pside = request();
        pside.transport = r.transport.clone();
        pside.uart_tx = r.uart_rx.clone();
        pside.uart_rx = r.uart_tx.clone();
        r.peripheral = Some(Box::new(pside));
        let p = generate(&r).unwrap();
        assert!(p.files["build.yaml"].contains("boardstudio_left"));
        assert!(
            p.files["config/boards/shields/boardstudio/boardstudio_left.conf"]
                .contains("CONFIG_ZMK_SPLIT_WIRED=y")
        );
        assert!(
            p.files
                .contains_key("config/boards/shields/boardstudio/boardstudio_right.overlay")
        );
    }
    #[test]
    fn asymmetric_split_uses_one_global_map_and_offsets_only_local_events() {
        let mut left = request();
        left.columns.push(ScanPin {
            terminal: "P19".into(),
            gpio: "P0.02".into(),
        });
        left.keys.push(FirmwareKey {
            id: "k2".into(),
            row: 0,
            column: 1,
        });
        left.transport = Some(SplitTransport::Wireless);
        let mut right = request();
        right.transport = left.transport.clone();
        left.peripheral = Some(Box::new(right));
        let package = generate(&left).unwrap();
        let front = &package.files["config/boards/shields/boardstudio/boardstudio_left.overlay"];
        let back = &package.files["config/boards/shields/boardstudio/boardstudio_right.overlay"];
        for overlay in [front, back] {
            assert!(overlay.contains("map = <RC(0, 0) RC(0, 1) RC(1, 0)>"));
            assert!(overlay.contains("columns = <2>;"));
        }
        assert!(front.contains("row-offset = <0>"));
        assert!(back.contains("row-offset = <1>"));
        assert_eq!(
            package.files["config/boards/shields/boardstudio/boardstudio.keymap"]
                .matches("&none")
                .count(),
            3
        );
    }
}

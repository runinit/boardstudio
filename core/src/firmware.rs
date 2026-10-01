//! Deterministic ZMK handoff generation.
//!
//! The generator accepts resolved electrical assignments only. It never invents
//! GPIO names from labels: every scan pin must resolve through an electrical
//! profile before files are emitted.

use crate::electrical_profiles::profile;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
mod encoders;
pub use encoders::FirmwareEncoder;

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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FirmwareModuleQualification {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub host_instance_id: Option<String>,
    pub protocol: crate::model::ModuleProtocol,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub catalogue_row: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub source: Option<crate::model::HardwareSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub rotary_profile: Option<crate::model::RotaryProfile>,
    #[serde(default)]
    pub gates: Vec<crate::model::HardwareGate>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FirmwarePhysicalInstance {
    pub id: String,
    pub board_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FirmwareModuleFinding {
    pub id: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FirmwareHardware {
    pub board_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub physical_instance_id: Option<String>,
    pub modules: Vec<FirmwareModuleQualification>,
    pub physical_instances: Vec<FirmwarePhysicalInstance>,
    pub module_findings: Vec<FirmwareModuleFinding>,
    pub embedded_circuit_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FirmwarePartQualification {
    pub part_id: String,
    pub name: String,
    pub source: crate::model::HardwareSource,
    pub gates: Vec<crate::model::HardwareGate>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FirmwareQualification {
    pub board_id: String,
    pub parts: Vec<FirmwarePartQualification>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
pub struct FirmwareRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub qualification: Option<FirmwareQualification>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub hardware: Option<FirmwareHardware>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "export-types",
        ts(as = "Option<Vec<FirmwareEncoder>>", optional)
    )]
    pub encoders: Vec<FirmwareEncoder>,
    #[serde(default)]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub keymap: Option<crate::model::KeymapConfiguration>,
    #[serde(default)]
    #[cfg_attr(feature = "export-types", ts(as = "Option<Vec<String>>", optional))]
    pub encoder_ids: Vec<String>,
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
    if let Some(qualification) = &request.qualification {
        for part in &qualification.parts {
            if let Some(gate) = part
                .gates
                .iter()
                .find(|gate| gate.output == crate::model::HardwareOutput::Firmware)
            {
                return Err(format!("{}: {}", part.name, gate.message));
            }
        }
    }

    if let Some(hardware) = &request.hardware {
        if hardware.board_id.trim().is_empty() {
            return Err("Firmware host board is missing".into());
        }
        if let Some(id) = &hardware.physical_instance_id {
            if !hardware
                .physical_instances
                .iter()
                .any(|instance| &instance.id == id && instance.board_id == hardware.board_id)
            {
                return Err(
                    "The selected physical host instance is missing or belongs to another board"
                        .into(),
                );
            }
        } else if hardware
            .modules
            .iter()
            .any(|module| module.host_instance_id.is_some())
        {
            return Err("Select a physical host instance before exporting modules attached to a specific keyboard half".into());
        }
        if let Some(finding) = hardware.module_findings.first() {
            return Err(format!("{}: {}", finding.id, finding.message));
        }
        let mut module_ids = std::collections::BTreeSet::new();
        for module in hardware.modules.iter().filter(|module| {
            module
                .host_instance_id
                .as_ref()
                .is_none_or(|host| hardware.physical_instance_id.as_ref() == Some(host))
        }) {
            if module.id.trim().is_empty() || !module_ids.insert(&module.id) {
                return Err("Firmware module snapshot has a missing or duplicate identity".into());
            }
            let source_rotary = module.catalogue_row.as_deref() == Some("ec11-evqwgd001")
                && module.source.as_ref().is_some_and(|source| {
                    source.repository == "https://github.com/sadekbaroudi/vik"
                        && source.revision == "cd5d16e4cd9137a229fc673412a89d75f4e64553"
                        && source.path == "pcb/ec11-evqwgd001/ec11-evqwgd001.kicad_pcb"
                        && source.sha256.as_deref()
                            == Some(
                                "2b22c0f0b2b99206ac25029b8acd05c75146d9246cb8e3d9ceec5deacae4e033",
                            )
                })
                && module.protocol == crate::model::ModuleProtocol::Gpio
                && module.rotary_profile.as_ref().is_some_and(|profile| {
                    profile.driver == Some(crate::model::EncoderDriver::Ec11)
                        && profile.steps.is_some_and(|steps| steps > 0)
                        && profile
                            .triggers_per_rotation
                            .is_some_and(|triggers| triggers > 0)
                        && profile.a == "gpio1"
                        && profile.b == "gpio2"
                        && profile.common == "gnd"
                });
            let matching_rotary_encoders = module.rotary_profile.as_ref().map_or(0, |profile| {
                request
                    .encoders
                    .iter()
                    .filter(|encoder| encoder.id == module.id && &encoder.profile == profile)
                    .count()
            });
            if source_rotary
                && (!request.encoder_ids.contains(&module.id) || matching_rotary_encoders != 1)
            {
                return Err(format!(
                    "{} requires one matching configured rotary encoder in the sensor order",
                    module.name
                ));
            }
            let rotary_only = source_rotary
                && request.encoder_ids.contains(&module.id)
                && matching_rotary_encoders == 1;
            if let Some(gate) = module.gates.iter().find(|gate| {
                gate.output == crate::model::HardwareOutput::Firmware
                    && !(rotary_only && gate.code == "module-driver")
            }) {
                return Err(format!("{}: {}", module.name, gate.message));
            }
            if module.protocol != crate::model::ModuleProtocol::PassThrough && !rotary_only {
                return Err(format!(
                    "{}: local ZMK driver support is not qualified for this module; source export cannot claim it is operational.",
                    module.name
                ));
            }
        }
        if !hardware.embedded_circuit_ids.is_empty() {
            return Err("Embedded module circuits require a qualified local firmware driver before functional ZMK export".into());
        }
    }
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
    if request
        .key_bindings
        .iter()
        .any(|b| !crate::keymap::legacy_binding(b))
    {
        return Err("Unsupported key binding".into());
    }
    if let Some(map) = &request.keymap {
        crate::keymap::validate(map)?;
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

/// Firmware qualification can be satisfied by explicit, supported encoder
/// configuration. This only clears the three configuration gates for the
/// matching encoder; electrical contact mapping and unrelated hardware gates
/// remain untouched.
fn clear_configured_thq_encoder_gates(request: &mut FirmwareRequest) -> bool {
    let Some(qualification) = request.qualification.as_mut() else {
        return false;
    };
    let mut cleared = false;
    for part in &mut qualification.parts {
        let is_pinned_thq_source = part.source.repository
            == "https://github.com/Taro-Hayashi/THQWGD001"
            && part.source.revision == "78e1c42dbebca1a9e28cf057d7d84f7eb786aa15"
            && part
                .source
                .path
                .starts_with("KiCad/footprints/THQWGD001.pretty/");
        let configured = is_pinned_thq_source
            && request.encoders.iter().any(|encoder| {
                let profile = &encoder.profile;
                encoder.id == part.part_id
                    && profile.driver == Some(crate::model::EncoderDriver::Ec11)
                    && profile.steps.is_some_and(|value| value > 0)
                    && profile.triggers_per_rotation.is_some_and(|value| value > 0)
                    && !profile.a.trim().is_empty()
                    && !profile.b.trim().is_empty()
                    && !profile.common.trim().is_empty()
                    && profile.a != profile.b
                    && profile.a != profile.common
                    && profile.b != profile.common
                    && !encoder.a_gpio.trim().is_empty()
                    && !encoder.b_gpio.trim().is_empty()
                    && encoder.a_gpio != encoder.b_gpio
            });
        if configured {
            let before = part.gates.len();
            part.gates.retain(|gate| {
                gate.output != crate::model::HardwareOutput::Firmware
                    || !matches!(
                        gate.code.as_str(),
                        "encoder-pulses-per-rotation"
                            | "detents-and-quadrature-direction"
                            | "driver-and-config-hardware-validation"
                    )
            });
            cleared |= part.gates.len() != before;
        }
    }
    cleared
}

pub fn generate(request: &FirmwareRequest) -> Result<FirmwarePackage, String> {
    // An encoder-only board uses its push inputs as the primary direct scanner.
    fn normalize_push_only(half: &mut FirmwareRequest) {
        if half.mode == FirmwareScanMode::Direct
            && half.direct_pins.is_empty()
            && !half.auxiliary_pins.is_empty()
        {
            half.direct_pins = std::mem::take(&mut half.auxiliary_pins);
            for key in &mut half.keys {
                key.row = 0;
            }
        }
    }
    let mut normalized = request.clone();
    encoders::prepare(&mut normalized)?;
    let used_designer_encoder_configuration = clear_configured_thq_encoder_gates(&mut normalized);
    if let Some(map) = &normalized.keymap {
        let known = normalized
            .encoder_ids
            .iter()
            .chain(
                normalized
                    .peripheral
                    .iter()
                    .flat_map(|half| half.encoder_ids.iter()),
            )
            .collect::<std::collections::BTreeSet<_>>();
        if let Some(id) = map
            .layers
            .iter()
            .flat_map(|layer| layer.sensors.keys())
            .find(|id| !known.contains(id))
        {
            return Err(format!("Keymap references unknown rotary sensor {id}"));
        }
    }
    normalize_push_only(&mut normalized);
    if let Some(half) = &mut normalized.peripheral {
        normalize_push_only(half);
    }
    let request = &normalized;
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
    let mut files: BTreeMap<String, String> = BTreeMap::new();
    files.insert("config/boards/shields/boardstudio/Kconfig.shield".into(), "config SHIELD_BOARDSTUDIO\n    def_bool $(shields_list_contains,boardstudio)\n\nconfig SHIELD_BOARDSTUDIO_LEFT\n    def_bool $(shields_list_contains,boardstudio_left)\n\nconfig SHIELD_BOARDSTUDIO_RIGHT\n    def_bool $(shields_list_contains,boardstudio_right)\n".into());
    let mut all_keys = request.keys.clone();
    if let Some(peripheral) = request.peripheral.as_deref() {
        all_keys.extend(peripheral.keys.clone());
    }
    files.insert(
        "config/boards/shields/boardstudio/boardstudio.keymap".into(),
        keymap(&all_keys, request)?,
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
    if request
        .keymap
        .as_ref()
        .is_some_and(|map| !map.macros.is_empty())
    {
        // 128 tap steps expand to 256 queued events; leave capacity for other inputs.
        for (path, text) in &mut files {
            if path.ends_with(".conf") {
                text.push_str("\nCONFIG_ZMK_BEHAVIORS_QUEUE_SIZE=512\n");
            }
        }
    }
    let mut warnings = Vec::new();
    if used_designer_encoder_configuration {
        warnings.push("THQ rotary timing and the EC11 driver are designer-supplied digital configuration. Physical detents, quadrature direction, continuity, and operation remain unverified.".into());
    }
    if request.hardware.as_ref().is_some_and(|hardware| {
        hardware.modules.iter().any(|module| {
            module
                .host_instance_id
                .as_ref()
                .is_none_or(|host| hardware.physical_instance_id.as_ref() == Some(host))
                && module.catalogue_row.as_deref() == Some("ec11-evqwgd001")
                && module.source.as_ref().is_some_and(|source| {
                    source.repository == "https://github.com/sadekbaroudi/vik"
                        && source.revision == "cd5d16e4cd9137a229fc673412a89d75f4e64553"
                        && source.path == "pcb/ec11-evqwgd001/ec11-evqwgd001.kicad_pcb"
                        && source.sha256.as_deref()
                            == Some(
                                "2b22c0f0b2b99206ac25029b8acd05c75146d9246cb8e3d9ceec5deacae4e033",
                            )
                })
                && module.protocol == crate::model::ModuleProtocol::Gpio
                && module.rotary_profile.as_ref().is_some_and(|profile| {
                    profile.driver == Some(crate::model::EncoderDriver::Ec11)
                        && profile.steps.is_some_and(|steps| steps > 0)
                        && profile
                            .triggers_per_rotation
                            .is_some_and(|triggers| triggers > 0)
                        && profile.a == "gpio1"
                        && profile.b == "gpio2"
                        && profile.common == "gnd"
                })
        })
    }) {
        warnings.push("VIK EC11 carrier firmware support exports rotation only. Its unconnected click contact and onboard SK6812 LEDs are not configured.".into());
    }
    Ok(FirmwarePackage { files, warnings })
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

fn keymap(keys: &[FirmwareKey], request: &FirmwareRequest) -> Result<String, String> {
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
    if let Some(map) = &request.keymap {
        let ids = keys.iter().map(|key| key.id.clone()).collect::<Vec<_>>();
        let mut encoders = request.encoder_ids.clone();
        if let Some(half) = &request.peripheral {
            encoders.extend(half.encoder_ids.clone());
        }
        return crate::keymap::source(map, &ids, &values, &encoders);
    }
    let bindings = values.join(" ");
    Ok(format!(
        "#include <behaviors.dtsi>\n#include <dt-bindings/zmk/keys.h>\n\n/ {{\n    keymap {{ compatible = \"zmk,keymap\"; default_layer {{ bindings = <{}>; }}; }};\n}};\n",
        bindings
    ))
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
            qualification: None,
            hardware: None,
            encoders: vec![],
            keymap: None,
            encoder_ids: vec![],
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

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SplitTransport {
    Wireless,
    WiredUart,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScanPin {
    pub terminal: String,
    pub gpio: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FirmwareKey {
    pub id: String,
    pub row: usize,
    pub column: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FirmwareModuleQualification {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_instance_id: Option<String>,
    pub protocol: crate::model::ModuleProtocol,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalogue_row: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<crate::model::HardwareSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotary_profile: Option<crate::model::RotaryProfile>,
    #[serde(default)]
    pub gates: Vec<crate::model::HardwareGate>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FirmwarePhysicalInstance {
    pub id: String,
    pub board_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FirmwareModuleFinding {
    pub id: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FirmwareHardware {
    pub board_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub physical_instance_id: Option<String>,
    pub modules: Vec<FirmwareModuleQualification>,
    pub physical_instances: Vec<FirmwarePhysicalInstance>,
    pub module_findings: Vec<FirmwareModuleFinding>,
    pub embedded_circuit_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FirmwarePartQualification {
    pub part_id: String,
    pub name: String,
    pub source: crate::model::HardwareSource,
    pub gates: Vec<crate::model::HardwareGate>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FirmwareQualification {
    pub board_id: String,
    pub parts: Vec<FirmwarePartQualification>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct FirmwareRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qualification: Option<FirmwareQualification>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hardware: Option<FirmwareHardware>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub encoders: Vec<FirmwareEncoder>,
    #[serde(default)]
    pub keymap: Option<crate::model::KeymapConfiguration>,
    #[serde(default)]
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
#[serde(rename_all = "kebab-case")]
pub enum FirmwareScanMode {
    #[default]
    Matrix,
    Direct,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FirmwarePackage {
    pub files: BTreeMap<String, String>,
    pub warnings: Vec<String>,
}

use crate::model::RotaryProfile;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FirmwareEncoder {
    pub id: String,
    pub profile: RotaryProfile,
    pub a_gpio: String,
    pub b_gpio: String,
}

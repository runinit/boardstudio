use crate::model::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElectricalPlanRequest {
    pub document: ProjectDoc,
    #[serde(default)]
    pub instance_id: Option<String>,
    #[serde(default)]
    pub mode: ElectricalMode,
    #[serde(default)]
    pub locks: BTreeMap<String, String>,
    #[serde(default)]
    pub controller_profile: Option<String>,
    #[serde(default)]
    pub board_id: Option<String>,
    #[serde(default)]
    pub controller_part_id: Option<String>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ElectricalMode {
    #[default]
    Matrix,
    Direct,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElectricalAssignment {
    pub key_id: String,
    pub matrix_id: String,
    pub row: u32,
    pub column: u32,
    pub row_pin: String,
    pub column_pin: String,
    pub locked: bool,
    pub row_firmware_gpio: Option<String>,
    pub column_firmware_gpio: Option<String>,
    pub direct_gpio: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElectricalDiagnostic {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub key_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElectricalPlan {
    pub instance_id: Option<String>,
    pub jumpers: Vec<crate::electrical_jumpers::JumperRecipe>,
    pub module_aliases: BTreeMap<String, String>,
    pub mode: ElectricalMode,
    pub assignments: Vec<ElectricalAssignment>,
    pub row_pins: Vec<String>,
    pub column_pins: Vec<String>,
    pub diagnostics: Vec<ElectricalDiagnostic>,
    pub fingerprint: String,
    pub board_id: Option<String>,
    pub controller_part_id: Option<String>,
    pub revision: u64,
    pub controller_profile: Option<String>,
    pub free_pins: Vec<String>,
    pub nets: Vec<Net>,
    pub diode_direction: String,
    pub peripherals: Vec<crate::electrical_peripherals::PeripheralRequirement>,
    pub peripheral_pins: BTreeMap<String, String>,
    pub peripheral_terminals: BTreeMap<String, String>,
}

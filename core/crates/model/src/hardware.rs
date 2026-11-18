use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HardwareSource {
    pub repository: String,
    pub revision: String,
    pub path: String,
    pub license: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_status: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HardwareOutput {
    Footprint,
    Electrical,
    Mechanical,
    Model,
    Firmware,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HardwareGate {
    pub output: HardwareOutput,
    pub code: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VikRole {
    Host,
    Module,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HardwareProfile {
    pub source: HardwareSource,
    #[serde(default)]
    pub gates: Vec<HardwareGate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vik_role: Option<VikRole>,
    #[serde(default)]
    pub footprint_surface_volumes: bool,
}

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PressContacts {
    pub row: String,
    pub column: String,
    pub independent: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EncoderDriver {
    Ec11,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RotaryProfile {
    pub a: String,
    pub b: String,
    pub common: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steps: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triggers_per_rotation: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver: Option<EncoderDriver>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub press: Option<PressContacts>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotary: Option<RotaryProfile>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PressScanMode {
    Matrix,
    Direct,
    Unassigned,
}

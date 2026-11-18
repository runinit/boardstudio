use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeripheralRequirement {
    pub part_id: String,
    pub source: String,
    pub kind: String,
    pub gpio_terminals: Vec<(String, String)>,
    pub fixed_terminals: Vec<(String, String)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotary: Option<crate::inputs::RotaryProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub press_key_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeripheralRequirementError {
    pub part_id: String,
    pub code: String,
    pub message: String,
}

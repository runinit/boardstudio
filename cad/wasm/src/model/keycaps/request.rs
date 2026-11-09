use boardstudio_contracts::KeycapSpec;
use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct Request {
    pub(super) revision: u64,
    pub(super) specs: Vec<KeycapSpec>,
    #[serde(default)]
    pub(super) printable: bool,
    /// Accept the old field only to reject stale `{ export: true }` callers.
    #[serde(default, rename = "export")]
    legacy_step_export: Option<bool>,
}

pub(super) fn validate_legacy_step_export(value: Option<bool>) -> Result<(), String> {
    if value == Some(true) {
        Err("Keycap STEP export is unsupported; build_keycaps returns preview meshes only".into())
    } else {
        Ok(())
    }
}

impl Request {
    pub(super) fn validate(&self) -> Result<(), String> {
        validate_legacy_step_export(self.legacy_step_export)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn stale_true_export_request_is_rejected_explicitly() {
        let request: Request = serde_json::from_value(json!({
            "revision": 4,
            "specs": [],
            "export": true
        }))
        .expect("old request payload still parses for a clear rejection");
        assert_eq!(
            request.validate().unwrap_err(),
            "Keycap STEP export is unsupported; build_keycaps returns preview meshes only"
        );
    }

    #[test]
    fn preview_request_omits_export_field_and_legacy_false_is_accepted() {
        let without_field: Request = serde_json::from_value(json!({
            "revision": 5,
            "specs": []
        }))
        .expect("new preview payload has no export field");
        without_field.validate().unwrap();

        let legacy_preview: Request = serde_json::from_value(json!({
            "revision": 6,
            "specs": [],
            "export": false
        }))
        .expect("legacy preview request parses");
        legacy_preview.validate().unwrap();
    }
}

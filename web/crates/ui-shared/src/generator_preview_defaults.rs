#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GeneratorPreviewDefaults {
    pub keycap_width: Option<f64>,
    pub keycap_height: Option<f64>,
    pub include_keycap: Option<bool>,
}
pub(crate) fn generator_preview_defaults_from_parameters(
    parameters: &serde_json::Value,
) -> GeneratorPreviewDefaults {
    let value = |name: &str| parameters.get(name).and_then(|entry| entry.get("value"));
    GeneratorPreviewDefaults {
        keycap_width: value("keycap_width").and_then(serde_json::Value::as_f64),
        keycap_height: value("keycap_height").and_then(serde_json::Value::as_f64),
        include_keycap: value("include_keycap").and_then(serde_json::Value::as_bool),
    }
}

#[cfg(test)]
mod preview_default_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn projects_values_from_the_retained_generator_parameter_envelope() {
        assert_eq!(
            generator_preview_defaults_from_parameters(&json!({
                "keycap_width": { "type": "number", "value": 18 },
                "keycap_height": { "type": "number", "value": 17.5 },
                "include_keycap": { "type": "boolean", "value": false }
            })),
            GeneratorPreviewDefaults {
                keycap_width: Some(18.0),
                keycap_height: Some(17.5),
                include_keycap: Some(false),
            }
        );
    }
}

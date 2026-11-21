//! Pure generator input coercion shared by the Parts presentation and native tests.
//!
//! The browser `generator_settings` editor owns drafts, previews and admission;
//! this module keeps the exact form-to-parameter coercion (`effective_value` and
//! `apply_input`) independent of Dioxus and the canvas DOM so its unit tests run
//! under `cargo test`.
use boardstudio_model::model::PartDefinition;
use serde_json::Value;
use std::collections::BTreeMap;

/// The value the form, preview and render agree on: a non-null draft wins, a null draft
/// or a null/missing saved value selects the declared default, and no default stays absent.
pub(crate) fn effective_value<'a>(
    draft: Option<&'a Value>,
    saved: Option<&'a Value>,
    spec: &'a Value,
) -> Option<&'a Value> {
    let default = spec.get("value").filter(|value| !value.is_null());
    match draft {
        Some(value) if !value.is_null() => Some(value),
        Some(_) => default,
        None => saved.filter(|value| !value.is_null()).or(default),
    }
}

pub(crate) fn apply_input(
    definition: &PartDefinition,
    schema: &BTreeMap<String, Value>,
    edits: &BTreeMap<String, Value>,
) -> Result<PartDefinition, String> {
    let Some(base) = definition.generator.as_ref() else {
        return Err("The selected part is no longer generator-backed.".into());
    };
    let mut generator = base.clone();
    for (key, spec) in schema {
        let Some(kind) = spec.get("type").and_then(Value::as_str) else {
            continue;
        };
        let raw = effective_value(edits.get(key), generator.parameters.get(key), spec);
        let Some(raw) = raw else {
            continue;
        };
        let value = match kind {
            "number" => {
                let number = match raw {
                    Value::Number(number) => number.as_f64(),
                    Value::String(text) if !text.trim().is_empty() => text.trim().parse().ok(),
                    _ => None,
                }
                .filter(|number: &f64| number.is_finite())
                .ok_or_else(|| format!("{} must be a finite number.", key.replace('_', " ")))?;
                serde_json::Number::from_f64(number)
                    .map(Value::Number)
                    .ok_or_else(|| format!("{} must be a finite number.", key.replace('_', " ")))?
            }
            "boolean" => raw
                .as_bool()
                .map(Value::Bool)
                .ok_or_else(|| format!("{} must be true or false.", key.replace('_', " ")))?,
            "string" | "net" => raw
                .as_str()
                .map(|value| Value::String(value.to_owned()))
                .ok_or_else(|| format!("{} must be text.", key.replace('_', " ")))?,
            "array" | "object" | "anchor" => {
                let value = if let Some(text) = raw.as_str() {
                    serde_json::from_str::<Value>(text).map_err(|_| {
                        format!("{} must contain valid JSON.", key.replace('_', " "))
                    })?
                } else {
                    raw.clone()
                };
                let valid = if kind == "array" {
                    value.is_array()
                } else {
                    value.is_object()
                };
                if !valid {
                    return Err(format!(
                        "{} must be {}.",
                        key.replace('_', " "),
                        if kind == "array" {
                            "a JSON array"
                        } else {
                            "a JSON object"
                        }
                    ));
                }
                value
            }
            _ => continue,
        };
        generator.parameters.insert(key.clone(), value);
    }
    let mut candidate = definition.clone();
    candidate.generator = Some(generator);
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core_kernel::generators;

    fn definition(source: &str) -> PartDefinition {
        generators::catalogue()
            .expect("catalogue")
            .into_iter()
            .find(|entry| {
                entry
                    .generator
                    .as_ref()
                    .is_some_and(|generator| generator.source == source)
            })
            .expect("bundled generator")
    }

    fn submit(source: &str, edits: Value) -> Result<PartDefinition, String> {
        let base = definition(source);
        let schema = generators::parameter_schema(source).expect("schema");
        let edits: BTreeMap<String, Value> =
            serde_json::from_value(edits).expect("edits are an object");
        apply_input(&base, &schema, &edits)
    }

    #[test]
    fn submitted_numbers_and_lists_pass_the_core_check() {
        let hole = submit(
            "ceoloide/mounting_hole_npth",
            serde_json::json!({ "hole_size": " 2.5 ", "hole_drill": "2.2" }),
        )
        .expect("numeric text is coerced");
        assert_eq!(
            generators::definition_error(&hole),
            Ok(()),
            "the coerced mounting hole passes Core"
        );
        assert!(hole.generator.as_ref().unwrap().parameters["hole_size"].is_number());

        let mcu = submit(
            "ceoloide/mcu_nice_nano",
            serde_json::json!({ "mcu_3dmodel_xyz_offset": "[1, 2, 3]", "designator": "U9" }),
        )
        .expect("list text that parses as JSON is coerced");
        assert_eq!(generators::definition_error(&mcu), Ok(()));
        assert!(mcu.generator.as_ref().unwrap().parameters["mcu_3dmodel_xyz_offset"].is_array());
    }

    #[test]
    fn text_that_does_not_parse_is_refused_before_it_reaches_core() {
        assert!(
            submit(
                "ceoloide/mounting_hole_npth",
                serde_json::json!({ "hole_size": "two" })
            )
            .is_err()
        );
        assert!(
            submit(
                "ceoloide/mcu_nice_nano",
                serde_json::json!({ "mcu_3dmodel_xyz_offset": "[1, 2" })
            )
            .is_err()
        );
    }

    #[test]
    fn the_check_would_catch_text_the_form_did_not_coerce() {
        // Negative control: a value written past the form is the wrong type for Core.
        let mut written = definition("ceoloide/mounting_hole_npth");
        written
            .generator
            .as_mut()
            .unwrap()
            .parameters
            .insert("hole_size".into(), Value::from("2"));
        let error = generators::definition_error(&written).expect_err("text for a number");
        assert!(error.contains("hole_size"), "{error}");
        assert!(error.contains("ceoloide/mounting_hole_npth"), "{error}");
    }

    #[test]
    fn effective_value_prefers_draft_then_saved_then_default() {
        let spec = serde_json::json!({"type": "string", "value": "default"});
        assert_eq!(
            effective_value(
                Some(&Value::String("draft".into())),
                Some(&Value::String("saved".into())),
                &spec
            ),
            Some(&Value::String("draft".into()))
        );
        assert_eq!(
            effective_value(
                Some(&Value::Null),
                Some(&Value::String("saved".into())),
                &spec
            ),
            Some(&Value::String("default".into()))
        );
        assert_eq!(
            effective_value(None, Some(&Value::String("saved".into())), &spec),
            Some(&Value::String("saved".into()))
        );
        assert_eq!(
            effective_value(None, None, &spec),
            Some(&Value::String("default".into()))
        );
        let no_default = serde_json::json!({"type": "string"});
        assert_eq!(effective_value(None, None, &no_default), None);
    }
}

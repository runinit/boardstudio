//! Ordered migrations of persisted project documents.
//!
//! A document carries `formatVersion`; one without it is version 1. Migrations run
//! at Core's load boundary (browser storage, archive import, bundled examples) so
//! nothing past that boundary handles an older shape. Each step takes a document
//! of version N to N+1 and is applied in order.
use serde_json::{Map, Value};

use crate::generators;

/// The version this build reads and writes.
pub const CURRENT_VERSION: u32 = 2;
/// The version of a document that does not state one.
pub const LEGACY_VERSION: u32 = 1;

/// Upgrade a persisted document to [`CURRENT_VERSION`]. Returns the upgraded
/// document and whether anything changed. Unknown fields are preserved.
pub fn migrate_value(mut document: Value) -> Result<(Value, bool), String> {
    let Some(object) = document.as_object_mut() else {
        return Err("Project document must be an object".into());
    };
    let mut version = match object.get("formatVersion") {
        None => LEGACY_VERSION,
        Some(value) => value
            .as_u64()
            .and_then(|number| u32::try_from(number).ok())
            .filter(|number| *number >= LEGACY_VERSION)
            .ok_or_else(|| "Project document has an invalid formatVersion".to_owned())?,
    };
    if version > CURRENT_VERSION {
        return Err(format!(
            "Project document version {version} is newer than this app supports (version {CURRENT_VERSION})"
        ));
    }
    let changed = version < CURRENT_VERSION;
    while version < CURRENT_VERSION {
        match version {
            1 => version_1_to_2(object),
            _ => unreachable!("every version below the current one has a migration"),
        }
        version += 1;
    }
    object.insert("formatVersion".into(), Value::from(CURRENT_VERSION));
    Ok((document, changed))
}

/// [`migrate_value`] for JSON text. Text already at the current version is returned unchanged.
pub fn migrate_json(text: &str) -> Result<(String, bool), String> {
    let value: Value =
        serde_json::from_str(text).map_err(|error| format!("Invalid project JSON: {error}"))?;
    let (value, changed) = migrate_value(value)?;
    if !changed {
        return Ok((text.to_owned(), false));
    }
    let text = serde_json::to_string(&value).map_err(|error| error.to_string())?;
    Ok((text, true))
}

/// The prefix of an identifier that used to name the generator library.
const LEGACY_PREFIX: &str = "ergogen:";
const LEGACY_MODEL_PREFIX: &str = "ergogen:model:";
const LEGACY_LOGO_SOURCE: &str = "utility_ergogen_logo";
const LEGACY_LOGO_NAME: &str = "utility ergogen logo";
/// The `source` of an asset embedded from the bundled model library.
const LEGACY_LIBRARY_SOURCE: &str = "bundled Ergogen library";

fn version_1_to_2(document: &mut Map<String, Value>) {
    normalize_generator_values(document);
    let mut value = Value::Object(std::mem::take(document));
    rename_in_place(&mut value);
    if let Value::Object(renamed) = value {
        *document = renamed;
    }
}

/// The generator source after the logo rename, when it is a built-in generator.
fn built_in_source(source: &str) -> Option<String> {
    let source = source.replace(LEGACY_LOGO_SOURCE, "utility_logo");
    generators::is_generator(&source).then_some(source)
}

/// Definition id → built-in generator source and whether the definition saves a `side`.
fn generator_definitions(document: &Map<String, Value>) -> Vec<(String, String, bool)> {
    let Some(Value::Array(definitions)) = document.get("definitions") else {
        return Vec::new();
    };
    definitions
        .iter()
        .filter_map(|definition| {
            let id = definition.get("id")?.as_str()?;
            let generator = definition.get("generator")?;
            let source = built_in_source(generator.get("source")?.as_str()?)?;
            let saves_side = generator
                .get("parameters")
                .and_then(|parameters| parameters.get("side"))
                .is_some();
            Some((id.to_owned(), source, saves_side))
        })
        .collect()
}

/// Convert saved numeric strings to numbers, drop empty text saved for list parameters and,
/// on back-side parts, state the layer the old renderer derived from the part side.
fn normalize_generator_values(document: &mut Map<String, Value>) {
    let known = generator_definitions(document);

    if let Some(Value::Array(definitions)) = document.get_mut("definitions") {
        for definition in definitions {
            let Some(generator) = definition
                .get_mut("generator")
                .and_then(Value::as_object_mut)
            else {
                continue;
            };
            let Some(source) = generator
                .get("source")
                .and_then(Value::as_str)
                .and_then(built_in_source)
            else {
                continue;
            };
            if let Some(parameters) = generator
                .get_mut("parameters")
                .and_then(Value::as_object_mut)
            {
                convert_parameters(&source, parameters);
            }
        }
    }

    // Parts and assembly members name their parameters differently.
    if let Some(Value::Array(parts)) = document.get_mut("parts") {
        for part in parts {
            normalize_instance(part, "generatorParameters", &known);
        }
    }
    if let Some(Value::Array(assemblies)) = document.get_mut("assemblies") {
        for assembly in assemblies {
            if let Some(Value::Array(members)) = assembly.get_mut("members") {
                for member in members {
                    normalize_instance(member, "parameters", &known);
                }
            }
        }
    }
}

fn normalize_instance(
    instance: &mut Value,
    parameters_key: &str,
    known: &[(String, String, bool)],
) {
    let Some(object) = instance.as_object_mut() else {
        return;
    };
    let Some((_, source, definition_saves_side)) = object
        .get("definitionId")
        .and_then(Value::as_str)
        .and_then(|id| known.iter().find(|(known_id, _, _)| known_id == id))
    else {
        return;
    };
    let back = object.get("side").and_then(Value::as_str) == Some("back");
    if let Some(parameters) = object
        .get_mut(parameters_key)
        .and_then(Value::as_object_mut)
    {
        convert_parameters(source, parameters);
    }
    let saves_side = *definition_saves_side
        || object
            .get(parameters_key)
            .and_then(|parameters| parameters.get("side"))
            .is_some();
    if back && !saves_side {
        let entry = object
            .entry(parameters_key)
            .or_insert_with(|| Value::Object(Map::new()));
        if entry.is_null() {
            *entry = Value::Object(Map::new());
        }
        if let Some(parameters) = entry.as_object_mut() {
            parameters.insert("side".into(), Value::from("B"));
        }
    }
}

fn convert_parameters(source: &str, parameters: &mut Map<String, Value>) {
    let Ok(schema) = generators::parameter_schema(source) else {
        return;
    };
    for (name, declared) in schema {
        let Some(Value::String(text)) = parameters.get(&name) else {
            continue;
        };
        let text = text.trim();
        match declared.get("type").and_then(Value::as_str) {
            Some("number") => {
                if text.is_empty() {
                    parameters.remove(&name);
                } else if let Some(number) = parse_number(text) {
                    parameters.insert(name, number);
                }
            }
            Some("array") if text.is_empty() => {
                parameters.remove(&name);
            }
            _ => {}
        }
    }
}

fn parse_number(text: &str) -> Option<Value> {
    let number: f64 = text
        .parse()
        .ok()
        .filter(|number: &f64| number.is_finite())?;
    if number.fract() == 0.0 && number.abs() < 9_007_199_254_740_992.0 {
        Some(Value::from(number as i64))
    } else {
        serde_json::Number::from_f64(number).map(Value::Number)
    }
}

fn rename_in_place(value: &mut Value) {
    match value {
        Value::String(text) => {
            if let Some(renamed) = renamed(text) {
                *text = renamed;
            }
        }
        Value::Array(items) => items.iter_mut().for_each(rename_in_place),
        Value::Object(object) => {
            if object.keys().any(|key| renamed(key).is_some()) {
                let entries = std::mem::take(object);
                for (key, mut item) in entries {
                    rename_in_place(&mut item);
                    object.insert(renamed(&key).unwrap_or(key), item);
                }
            } else {
                object.values_mut().for_each(rename_in_place);
            }
        }
        _ => {}
    }
}

fn renamed(text: &str) -> Option<String> {
    let mut result = if let Some(rest) = text.strip_prefix(LEGACY_MODEL_PREFIX) {
        format!("{}{rest}", generators::BUNDLED_MODEL_PREFIX)
    } else if let Some(rest) = text.strip_prefix(LEGACY_PREFIX) {
        format!("{}{rest}", generators::DEFINITION_ID_PREFIX)
    } else {
        text.to_owned()
    };
    if result.contains(LEGACY_LOGO_SOURCE) {
        result = result.replace(LEGACY_LOGO_SOURCE, "utility_logo");
    }
    if result == LEGACY_LOGO_NAME {
        result = "utility logo".into();
    }
    if result == LEGACY_LIBRARY_SOURCE {
        result = "bundled footprint library".into();
    }
    (result != text).then_some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn version_1() -> Value {
        json!({
            "format": "boardstudio/v2", "id": "p", "name": "P", "revision": 3,
            "future": { "keep": true },
            "definitions": [
                { "id": "ergogen:ceoloide/switch_mx", "name": "MX switch",
                  "generator": { "source": "ceoloide/switch_mx", "version": "bundled-1",
                    "parameters": { "switch_3dmodel_xyz_offset": "", "hotswap": true } },
                  "models": [{ "assetId": "ergogen:model:kiswitch/SW_Cherry_MX_PCB.stp" }] },
                { "id": "ergogen:ceoloide/mounting_hole_npth", "name": "hole",
                  "generator": { "source": "ceoloide/mounting_hole_npth", "version": "bundled-1",
                    "parameters": { "hole_size": "2.2", "hole_drill": "3" } } },
                { "id": "ergogen:ceoloide/utility_ergogen_logo", "name": "utility ergogen logo",
                  "generator": { "source": "ceoloide/utility_ergogen_logo", "version": "bundled-1",
                    "parameters": {} } },
                { "id": "ergogen:ceoloide/diode_tht_sod123", "name": "diode",
                  "generator": { "source": "ceoloide/diode_tht_sod123", "version": "bundled-1",
                    "parameters": { "side": "B" } } },
                { "id": "custom", "name": "custom", "pads": [] }
            ],
            "parts": [
                { "id": "front", "definitionId": "ergogen:ceoloide/switch_mx", "side": "front" },
                { "id": "back", "definitionId": "ergogen:ceoloide/switch_mx", "side": "back",
                  "generatorParameters": { "hotswap": false } },
                { "id": "back-saved", "definitionId": "ergogen:ceoloide/switch_mx", "side": "back",
                  "generatorParameters": { "side": "F" } },
                { "id": "back-definition-side", "definitionId": "ergogen:ceoloide/diode_tht_sod123",
                  "side": "back" },
                { "id": "hole", "definitionId": "ergogen:ceoloide/mounting_hole_npth", "side": "front",
                  "generatorParameters": { "hole_size": " 3.5 " } },
                { "id": "plain", "definitionId": "custom", "side": "back" }
            ],
            "assemblies": [{ "id": "a", "name": "A", "members": [
                { "id": "m", "definitionId": "ergogen:ceoloide/switch_mx", "side": "back",
                  "parameters": { "switch_3dmodel_xyz_scale": "" } }
            ]}],
            "assets": [{ "id": "ergogen:model:vendor/x.step", "source": "bundled Ergogen library" }],
        })
    }

    #[test]
    fn migrates_a_version_1_document() {
        let (document, changed) = migrate_value(version_1()).unwrap();
        assert!(changed);
        assert_eq!(document["formatVersion"], 2);
        assert_eq!(document["future"], json!({ "keep": true }));

        let definitions = &document["definitions"];
        assert_eq!(definitions[0]["id"], "generator:ceoloide/switch_mx");
        assert_eq!(
            definitions[0]["models"][0]["assetId"],
            "bundled-model:kiswitch/SW_Cherry_MX_PCB.stp"
        );
        assert!(
            definitions[0]["generator"]["parameters"]
                .get("switch_3dmodel_xyz_offset")
                .is_none()
        );
        assert_eq!(definitions[0]["generator"]["parameters"]["hotswap"], true);
        assert_eq!(definitions[1]["generator"]["parameters"]["hole_size"], 2.2);
        assert_eq!(definitions[1]["generator"]["parameters"]["hole_drill"], 3);
        assert_eq!(definitions[2]["id"], "generator:ceoloide/utility_logo");
        assert_eq!(definitions[2]["name"], "utility logo");
        assert_eq!(
            definitions[2]["generator"]["source"],
            "ceoloide/utility_logo"
        );
        assert_eq!(definitions[4]["id"], "custom");

        let parts = &document["parts"];
        assert_eq!(parts[0]["definitionId"], "generator:ceoloide/switch_mx");
        assert!(
            parts[0].get("generatorParameters").is_none(),
            "front parts gain nothing"
        );
        assert_eq!(
            parts[1]["generatorParameters"],
            json!({ "hotswap": false, "side": "B" })
        );
        assert_eq!(
            parts[2]["generatorParameters"],
            json!({ "side": "F" }),
            "a saved side wins"
        );
        assert!(
            parts[3].get("generatorParameters").is_none(),
            "the definition already saves a side"
        );
        assert_eq!(parts[4]["generatorParameters"]["hole_size"], 3.5);
        assert!(
            parts[5].get("generatorParameters").is_none(),
            "no generator, no side"
        );

        let member = &document["assemblies"][0]["members"][0];
        assert_eq!(member["definitionId"], "generator:ceoloide/switch_mx");
        assert_eq!(member["parameters"], json!({ "side": "B" }));
        assert_eq!(document["assets"][0]["id"], "bundled-model:vendor/x.step");
        assert_eq!(document["assets"][0]["source"], "bundled footprint library");
        assert!(!document.to_string().contains("ergogen"));
    }

    #[test]
    fn migration_is_idempotent_and_leaves_current_documents_alone() {
        let (once, _) = migrate_value(version_1()).unwrap();
        let (twice, changed) = migrate_value(once.clone()).unwrap();
        assert!(!changed);
        assert_eq!(once, twice);
    }

    #[test]
    fn current_documents_keep_their_text() {
        let text = r#" { "format":"boardstudio/v2", "formatVersion":2, "x": 1 } "#;
        assert_eq!(migrate_json(text).unwrap(), (text.to_owned(), false));
    }

    #[test]
    fn rejects_newer_or_malformed_versions() {
        let newer = migrate_value(json!({ "formatVersion": 3 })).unwrap_err();
        assert!(newer.contains("newer than this app supports"), "{newer}");
        assert!(migrate_value(json!({ "formatVersion": "2" })).is_err());
        assert!(migrate_value(json!({ "formatVersion": null })).is_err());
        assert!(migrate_value(json!({ "formatVersion": 0 })).is_err());
        assert!(migrate_value(json!([])).is_err());
    }

    #[test]
    fn migrated_documents_deserialize_and_render() {
        let mut definition = generators::catalogue()
            .unwrap()
            .into_iter()
            .find(|item| item.id == "generator:ceoloide/mounting_hole_npth")
            .unwrap();
        definition
            .generator
            .as_mut()
            .unwrap()
            .parameters
            .insert("hole_size".into(), json!("2.2"));
        let mut value =
            serde_json::to_value(crate::model::ProjectDoc::empty("legacy", "Legacy")).unwrap();
        value["definitions"] = json!([definition]);
        value["definitions"][0]["id"] = json!("ergogen:ceoloide/mounting_hole_npth");
        value.as_object_mut().unwrap().remove("formatVersion");

        let (value, changed) = migrate_value(value).unwrap();
        assert!(changed);
        let document: crate::model::ProjectDoc = serde_json::from_value(value).unwrap();
        assert_eq!(document.format_version, CURRENT_VERSION);
        let definition = &document.definitions[0];
        assert_eq!(definition.id, "generator:ceoloide/mounting_hole_npth");
        let size = &definition.generator.as_ref().unwrap().parameters["hole_size"];
        assert_eq!(size, &json!(2.2));
        generators::render_forms(definition, None).expect("a migrated definition renders");
    }
}

//! Strict document admission. Format 7 has no conversion from older schemas.
#[cfg(test)]
use serde_json::Map;
use serde_json::Value;

#[cfg(test)]
use crate::generators;

/// The version this build reads and writes.
pub const CURRENT_VERSION: u32 = 7;
/// The version of a document that does not state one.
pub const LEGACY_VERSION: u32 = 1;

/// Typed Core admission never migrates or modifies the candidate document.
pub(crate) fn require_current_version(version: u32) -> Result<(), String> {
    if version == 0 {
        return Err("Project data failed validation: invalid formatVersion 0".into());
    }
    if version < CURRENT_VERSION {
        return Err(format!(
            "This project uses format {version}. This build requires format {CURRENT_VERSION}. No conversion is available. Your saved data has not been changed."
        ));
    }
    if version > CURRENT_VERSION {
        return Err(format!(
            "This project requires format {version}. This build supports format {CURRENT_VERSION}. Open it with a compatible newer build."
        ));
    }
    Ok(())
}

/// Admit a current document without modifying any saved data. The bool remains
/// false: callers must never write a normalized version back during list/load.
pub fn migrate_value(document: Value) -> Result<(Value, bool), String> {
    require_current_value(&document)?;
    Ok((document, false))
}

/// Inspect the version before schema-dependent deserialization or replacement effects.
pub fn require_current_value(document: &Value) -> Result<(), String> {
    let object = document
        .as_object()
        .ok_or("Project data failed validation: document must be an object")?;
    if object.get("format").and_then(Value::as_str) != Some("boardstudio/v2") {
        return Err("Unsupported document format".into());
    }
    let value = object.get("formatVersion").ok_or_else(|| format!("This legacy project has no formatVersion. This build requires format {CURRENT_VERSION}. No conversion is available. Your saved data has not been changed."))?;
    let version = value
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or("Project data failed validation: invalid formatVersion")?;
    require_current_version(version)
}

/// Historical migration algorithms are retained only to test old fixture provenance.
#[cfg(test)]
fn historical_migrate_value(mut document: Value) -> Result<(Value, bool), String> {
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
    if version > 6 {
        return Err(format!(
            "Project document version {version} is newer than this app supports (version {CURRENT_VERSION})"
        ));
    }
    let changed = version < 6;
    while version < 6 {
        match version {
            1 => version_1_to_2(object),
            2 => version_2_to_3(object),
            3 => version_3_to_4(object),
            4 => version_4_to_5(object),
            5 => version_5_to_6(object),
            _ => unreachable!("every version below the current one has a migration"),
        }
        version += 1;
    }
    object.insert("formatVersion".into(), Value::from(6));
    Ok((document, changed))
}

/// [`migrate_value`] for JSON text. Text already at the current version is returned unchanged.
pub fn migrate_json(text: &str) -> Result<(String, bool), String> {
    let value: Value =
        serde_json::from_str(text).map_err(|error| format!("Invalid project JSON: {error}"))?;
    require_current_value(&value)?;
    Ok((text.to_owned(), false))
}

/// The prefix of an identifier that used to name the generator library.
#[cfg(test)]
const LEGACY_PREFIX: &str = "ergogen:";
#[cfg(test)]
const LEGACY_MODEL_PREFIX: &str = "ergogen:model:";
#[cfg(test)]
const LEGACY_LOGO_SOURCE: &str = "utility_ergogen_logo";
#[cfg(test)]
const LEGACY_LOGO_NAME: &str = "utility ergogen logo";
/// The `source` of an asset embedded from the bundled model library.
#[cfg(test)]
const LEGACY_LIBRARY_SOURCE: &str = "bundled Ergogen library";

#[cfg(test)]
fn version_1_to_2(document: &mut Map<String, Value>) {
    state_back_sides(document);
    let mut value = Value::Object(std::mem::take(document));
    rename_in_place(&mut value);
    if let Value::Object(renamed) = value {
        *document = renamed;
    }
}

/// Give authored openings and their vertices stable identities and record explicit
/// construction ownership.
///
/// Existing physical instances become `legacy`, which preserves today's effective
/// shared overlay and first-instance fallback; the old `constructionLinked` flag never
/// affected that overlay and is retired. Geometry is preserved and no mechanical pair
/// or feature correspondence is inferred.
#[cfg(test)]
fn version_2_to_3(document: &mut Map<String, Value>) {
    identify_mechanical_openings(document.get_mut("mechanical"));
    let Some(Value::Object(hardware)) = document.get_mut("hardware") else {
        return;
    };
    identify_mechanical_openings(hardware.get_mut("sharedConstruction"));
    let Some(Value::Array(instances)) = hardware.get_mut("instances") else {
        return;
    };
    for instance in instances {
        let Some(object) = instance.as_object_mut() else {
            continue;
        };
        identify_mechanical_openings(object.get_mut("mechanical"));
        object.insert("constructionOwnership".into(), Value::from("legacy"));
        object.remove("constructionLinked");
    }
}

/// Rewrite a mechanical configuration's `openings` from point arrays to identified
/// openings and vertices. Geometry, order, Z and height are unchanged.
#[cfg(test)]
fn identify_mechanical_openings(configuration: Option<&mut Value>) {
    let Some(Value::Object(configuration)) = configuration else {
        return;
    };
    let Some(Value::Array(openings)) = configuration.get_mut("openings") else {
        return;
    };
    for (index, opening) in openings.iter_mut().enumerate() {
        let Some(object) = opening.as_object_mut() else {
            continue;
        };
        object
            .entry("id")
            .or_insert_with(|| Value::from(format!("opening-{}", index + 1)));
        if let Some(Value::Array(points)) = object.remove("points") {
            let vertices = points
                .into_iter()
                .enumerate()
                .map(|(vertex_index, point)| {
                    let mut vertex = Map::new();
                    vertex.insert(
                        "id".into(),
                        Value::from(format!("vertex-{}", vertex_index + 1)),
                    );
                    vertex.insert("point".into(), point);
                    Value::Object(vertex)
                })
                .collect();
            object.insert("vertices".into(), Value::Array(vertices));
        } else if let Some(Value::Array(vertices)) = object.get_mut("vertices") {
            for (vertex_index, vertex) in vertices.iter_mut().enumerate() {
                if let Some(vertex) = vertex.as_object_mut() {
                    vertex
                        .entry("id")
                        .or_insert_with(|| Value::from(format!("vertex-{}", vertex_index + 1)));
                }
            }
        }
    }
}

/// The generator source after the logo rename, when it is a built-in generator.
#[cfg(test)]
fn built_in_source(source: &str) -> Option<String> {
    let source = source.replace(LEGACY_LOGO_SOURCE, "utility_logo");
    generators::is_generator(&source).then_some(source)
}

/// Definition id → built-in generator source and whether the definition saves a `side`.
#[cfg(test)]
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

/// Version 3 saved generator parameters as text. Converts the numeric text that the typed
/// generators declare to numbers and removes empty or whitespace-only number and list text.
/// Non-empty list text (for example `"[1,2,3]"`) stays text, as in the old 1 to 2 step. Names
/// that no generator declares and values that cannot be converted are kept, so Core's
/// validation reports them.
#[cfg(test)]
fn version_3_to_4(document: &mut Map<String, Value>) {
    convert_generator_values(document);
}

/// Version 5 adds optional fields to mechanical pairs (`constructionSource`, `halves`), and
/// absence means what version 4 meant. Only the version changes, so an older build refuses a
/// version 5 document instead of dropping the record (ADR-0004).
#[cfg(test)]
fn version_4_to_5(_document: &mut Map<String, Value>) {}

/// Framed constraints are introduced in version 6. Existing records keep their literal data;
/// this compatibility step only marks the format a reader that understands the new kind.
#[cfg(test)]
fn version_5_to_6(_document: &mut Map<String, Value>) {}

/// On back-side parts, state the layer the old renderer derived from the part side.
#[cfg(test)]
fn state_back_sides(document: &mut Map<String, Value>) {
    let known = generator_definitions(document);
    if let Some(Value::Array(parts)) = document.get_mut("parts") {
        for part in parts {
            state_back_side(part, "generatorParameters", &known);
        }
    }
    if let Some(Value::Array(assemblies)) = document.get_mut("assemblies") {
        for assembly in assemblies {
            if let Some(Value::Array(members)) = assembly.get_mut("members") {
                for member in members {
                    state_back_side(member, "parameters", &known);
                }
            }
        }
    }
}

#[cfg(test)]
fn state_back_side(instance: &mut Value, parameters_key: &str, known: &[(String, String, bool)]) {
    let Some(object) = instance.as_object_mut() else {
        return;
    };
    let Some((_, _, definition_saves_side)) = object
        .get("definitionId")
        .and_then(Value::as_str)
        .and_then(|id| known.iter().find(|(known_id, _, _)| known_id == id))
    else {
        return;
    };
    let back = object.get("side").and_then(Value::as_str) == Some("back");
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

/// Convert saved numeric strings to numbers and remove empty or whitespace-only number and list
/// text, on definitions, parts and assembly members. Non-empty list text is not converted.
#[cfg(test)]
fn convert_generator_values(document: &mut Map<String, Value>) {
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
            convert_instance(part, "generatorParameters", &known);
        }
    }
    if let Some(Value::Array(assemblies)) = document.get_mut("assemblies") {
        for assembly in assemblies {
            if let Some(Value::Array(members)) = assembly.get_mut("members") {
                for member in members {
                    convert_instance(member, "parameters", &known);
                }
            }
        }
    }
}

#[cfg(test)]
fn convert_instance(instance: &mut Value, parameters_key: &str, known: &[(String, String, bool)]) {
    let Some(object) = instance.as_object_mut() else {
        return;
    };
    let Some((_, source, _)) = object
        .get("definitionId")
        .and_then(Value::as_str)
        .and_then(|id| known.iter().find(|(known_id, _, _)| known_id == id))
    else {
        return;
    };
    if let Some(parameters) = object
        .get_mut(parameters_key)
        .and_then(Value::as_object_mut)
    {
        convert_parameters(source, parameters);
    }
}

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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
    use super::historical_migrate_value as migrate_value;
    use super::*;
    const CURRENT_VERSION: u32 = 6;
    fn migrate_json(text: &str) -> Result<(String, bool), String> {
        let (value, changed) =
            migrate_value(serde_json::from_str(text).map_err(|e| e.to_string())?)?;
        Ok((
            if changed {
                serde_json::to_string(&value).map_err(|e| e.to_string())?
            } else {
                text.to_owned()
            },
            changed,
        ))
    }
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
        assert_eq!(document["formatVersion"], CURRENT_VERSION);
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
    fn gives_authored_openings_identity_without_moving_geometry() {
        let mut value =
            serde_json::to_value(crate::model::ProjectDoc::empty("doc", "Doc")).unwrap();
        value["formatVersion"] = json!(2);
        value["mechanical"] = json!({ "boardId": "board", "method": "printed", "mount": "tray",
                "plateThickness": 1.5, "plateFoamThickness": 0.0, "pcbThickness": 1.6,
                "bottomFoamThickness": 2.0, "batteryHeight": 0.0, "bottomThickness": 3.0,
                "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3, "profiles": [],
                "openings": [
                    { "points": [ { "x": -2.5, "y": -2.5 }, { "x": 2.5, "y": 2.5 } ], "z": 0.0, "height": 10.0 }
                ] });
        value["hardware"] = json!({
                "instances": [
                    { "id": "left", "name": "Left", "boardId": "board", "half": "left", "role": "primary", "flipped": false, "controllerPartId": null, "constructionLinked": true,
                      "mechanical": { "boardId": "board", "method": "printed", "mount": "tray",
                        "plateThickness": 1.5, "plateFoamThickness": 0.0, "pcbThickness": 1.6,
                        "bottomFoamThickness": 2.0, "batteryHeight": 0.0, "bottomThickness": 3.0,
                        "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3, "profiles": [],
                        "openings": [
                            { "points": [ { "x": 1.0, "y": 1.0 }, { "x": 2.0, "y": 2.0 }, { "x": 1.0, "y": 2.0 } ], "z": 1.0, "height": 5.0 }
                        ] } }
                ],
                "sharedConstruction": { "boardId": "board", "method": "printed", "mount": "tray",
                    "plateThickness": 1.5, "plateFoamThickness": 0.0, "pcbThickness": 1.6,
                    "bottomFoamThickness": 2.0, "batteryHeight": 0.0, "bottomThickness": 3.0,
                    "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3, "profiles": [],
                    "openings": [
                        { "points": [ { "x": 0.0, "y": 0.0 }, { "x": 4.0, "y": 0.0 }, { "x": 4.0, "y": 4.0 } ], "z": -1.0, "height": 8.0 }
                    ] }
        });
        let (document, changed) = migrate_value(value).unwrap();
        assert!(changed);
        assert_eq!(document["formatVersion"], CURRENT_VERSION);

        let canonical = &document["mechanical"]["openings"][0];
        assert_eq!(canonical["id"], "opening-1");
        assert_eq!(canonical["vertices"][0]["id"], "vertex-1");
        assert_eq!(
            canonical["vertices"][0]["point"],
            json!({ "x": -2.5, "y": -2.5 })
        );
        assert_eq!(
            canonical["vertices"][1]["point"],
            json!({ "x": 2.5, "y": 2.5 })
        );
        assert_eq!(canonical["z"], 0.0);
        assert_eq!(canonical["height"], 10.0);
        assert!(canonical.get("points").is_none());

        let shared = &document["hardware"]["sharedConstruction"]["openings"][0];
        assert_eq!(shared["id"], "opening-1");
        assert_eq!(
            shared["vertices"][2]["point"],
            json!({ "x": 4.0, "y": 4.0 })
        );

        let instance = &document["hardware"]["instances"][0];
        assert_eq!(instance["constructionOwnership"], "legacy");
        assert!(instance.get("constructionLinked").is_none());
        assert_eq!(instance["mechanical"]["openings"][0]["id"], "opening-1");
        assert_eq!(
            instance["mechanical"]["openings"][0]["vertices"][1]["point"],
            json!({ "x": 2.0, "y": 2.0 })
        );

        // No mechanical pairs or feature correspondences are invented.
        assert!(document["hardware"].get("pairs").is_none());

        // The migrated document deserializes and round-trips its geometry.
        let parsed: crate::model::ProjectDoc = serde_json::from_value(document.clone()).unwrap();
        let openings = parsed
            .mechanical
            .as_ref()
            .unwrap()
            .openings
            .as_ref()
            .unwrap();
        assert_eq!(openings[0].id, "opening-1");
        assert_eq!(openings[0].vertices.len(), 2);
        assert_eq!(openings[0].vertices[0].point.x, -2.5);
        assert_eq!(
            parsed.hardware.as_ref().unwrap().instances[0].construction_ownership,
            crate::model::MechanicalConstructionOwnership::Legacy
        );
    }

    #[test]
    fn migration_is_idempotent_and_leaves_current_documents_alone() {
        let (once, _) = migrate_value(version_1()).unwrap();
        let (twice, changed) = migrate_value(once.clone()).unwrap();
        assert!(!changed);
        assert_eq!(once, twice);
    }

    /// Version 5 adds optional mirrored-half fields; version 6 adds framed constraints. Neither
    /// step invents ownership or rewrites existing geometry.
    #[test]
    fn version_4_document_migrates_without_changing_its_payload() {
        let value = json!({
            "format": "boardstudio/v2", "formatVersion": 4, "id": "p", "name": "P", "revision": 2,
            "future": { "keep": true },
            "hardware": { "pairs": [{
                "id": "pair", "members": ["left", "right"],
                "mapping": { "at": { "x": 200.0, "y": 0.0 }, "rotation": 0.0 },
                "construction": { "method": "printed", "mount": "tray", "integratedPlateFrame": false,
                    "plateThickness": 1.3, "plateFoamThickness": 2.0, "bottomFoamThickness": 2.0,
                    "bottomThickness": 3.0, "plateToPcb": 2.2, "wallThickness": 2.0, "clearance": 0.3 },
                "features": []
            }] }
        });
        let (document, changed) = migrate_value(value.clone()).unwrap();
        assert!(changed);
        let mut expected = value;
        expected["formatVersion"] = json!(CURRENT_VERSION);
        assert_eq!(document, expected);
        assert!(document["hardware"]["pairs"][0].get("halves").is_none());
        assert!(
            document["hardware"]["pairs"][0]
                .get("constructionSource")
                .is_none()
        );
    }

    #[test]
    fn version_5_migration_only_stamps_the_format_version() {
        let value = json!({
            "format": "boardstudio/v2", "formatVersion": 5,
            "constraints": [{"kind":"offset", "id":"c", "sourcePartId":"a",
                "targetPartId":"b", "offset":{"x":-0.0,"y":2.0}, "rotation":-0.0}],
            "hardware": {"pairs": [{"id":"pair", "halves": {
                "sourceBoardId":"a", "mirrorBoardId":"b", "correspondences":[], "exceptions":[]
            }}]},
            "future": {"preserve": true}
        });
        let (migrated, changed) = migrate_value(value.clone()).unwrap();
        let mut expected = value;
        expected["formatVersion"] = json!(CURRENT_VERSION);
        assert!(changed);
        assert_eq!(migrated, expected);
    }

    #[test]
    fn version_5_legacy_offsets_and_mirrors_migrate_deserialize_and_resolve_unchanged() {
        let mut value = serde_json::to_value(crate::model::ProjectDoc::empty("v5", "V5")).unwrap();
        value["formatVersion"] = json!(5);
        value["parts"] = json!([
            {"id":"source", "definitionId":"key", "reference":"SW1", "side":"front",
             "pose":{"at":{"x":2.0,"y":5.0},"rotation":25.0}},
            {"id":"offset-target", "definitionId":"key", "reference":"SW2", "side":"front",
             "pose":{"at":{"x":0.0,"y":0.0},"rotation":0.0}},
            {"id":"vertical-target", "definitionId":"key", "reference":"SW3", "side":"front",
             "pose":{"at":{"x":0.0,"y":0.0},"rotation":0.0}},
            {"id":"horizontal-target", "definitionId":"key", "reference":"SW4", "side":"front",
             "pose":{"at":{"x":0.0,"y":0.0},"rotation":0.0}}
        ]);
        value["constraints"] = json!([
            {"kind":"offset", "id":"offset", "sourcePartId":"source", "targetPartId":"offset-target",
             "offset":{"x":1.0,"y":-4.0}, "rotation":7.5},
            {"kind":"mirror", "id":"vertical", "sourcePartId":"offset-target", "targetPartId":"vertical-target",
             "axis":"vertical", "coordinate":10.0},
            {"kind":"mirror", "id":"horizontal", "sourcePartId":"source", "targetPartId":"horizontal-target",
             "axis":"horizontal", "coordinate":10.0}
        ]);
        let original_constraints = value["constraints"].clone();

        let (migrated, changed) = migrate_value(value).unwrap();

        assert!(changed);
        assert_eq!(migrated["formatVersion"], json!(CURRENT_VERSION));
        assert_eq!(migrated["constraints"], original_constraints);
        let mut document: crate::model::ProjectDoc = serde_json::from_value(migrated).unwrap();
        crate::constraints::resolve(&mut document).unwrap();
        let pose = |id: &str| {
            document
                .parts
                .iter()
                .find(|part| part.id == id)
                .unwrap()
                .pose
        };
        assert_eq!(
            pose("offset-target"),
            crate::model::Pose2 {
                at: crate::model::Vec2 { x: 3.0, y: 1.0 },
                rotation: 32.5,
            }
        );
        assert_eq!(
            pose("vertical-target"),
            crate::model::Pose2 {
                at: crate::model::Vec2 { x: 17.0, y: 1.0 },
                rotation: 147.5,
            }
        );
        assert_eq!(
            pose("horizontal-target"),
            crate::model::Pose2 {
                at: crate::model::Vec2 { x: 2.0, y: 15.0 },
                rotation: -25.0,
            }
        );
    }

    #[test]
    fn current_documents_keep_their_text() {
        let text = format!(
            r#" {{ "format":"boardstudio/v2", "formatVersion":{}, "x": 1 }} "#,
            CURRENT_VERSION
        );
        assert_eq!(migrate_json(&text).unwrap(), (text, false));
    }

    #[test]
    fn rejects_newer_or_malformed_versions() {
        let newer = migrate_value(json!({ "formatVersion": 7 })).unwrap_err();
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

    #[test]
    fn version_3_saved_strings_become_numbers_and_render() {
        let mut definition = generators::catalogue()
            .unwrap()
            .into_iter()
            .find(|item| item.id == "generator:ceoloide/mounting_hole_npth")
            .unwrap();
        let parameters = &mut definition.generator.as_mut().unwrap().parameters;
        parameters.insert("hole_size".into(), json!("2.2"));
        parameters.insert("hole_drill".into(), json!("3"));
        let mut value =
            serde_json::to_value(crate::model::ProjectDoc::empty("saved", "Saved")).unwrap();
        value["formatVersion"] = json!(3);
        value["definitions"] = json!([definition]);

        let (value, changed) = migrate_value(value).unwrap();
        assert!(
            changed,
            "a version 3 document is rewritten at the current version"
        );
        assert_eq!(value["formatVersion"], CURRENT_VERSION);
        let document: crate::model::ProjectDoc = serde_json::from_value(value).unwrap();
        let definition = &document.definitions[0];
        let parameters = &definition.generator.as_ref().unwrap().parameters;
        assert_eq!(parameters["hole_size"], json!(2.2));
        assert_eq!(parameters["hole_drill"], json!(3));
        generators::render_forms(definition, None).expect("a migrated definition renders");
    }

    #[test]
    fn version_3_to_4_keeps_unknown_names_and_unconvertible_values() {
        let value = json!({
            "format": "boardstudio/v2", "formatVersion": 3, "id": "p", "name": "P", "revision": 1,
            "future": { "keep": true },
            "definitions": [
                { "id": "generator:ceoloide/mounting_hole_npth", "name": "hole",
                  "generator": { "source": "ceoloide/mounting_hole_npth", "version": "bundled-1",
                    "parameters": { "hole_size": "two", "hole_drill": "3", "not_a_parameter": "9" } } }
            ],
            "parts": [
                { "id": "hole", "definitionId": "generator:ceoloide/mounting_hole_npth", "side": "back",
                  "generatorParameters": { "hole_size": " 3.5 ", "not_a_parameter": "4" } }
            ],
            "assemblies": [{ "id": "a", "name": "A", "members": [
                { "id": "m", "definitionId": "generator:ceoloide/mounting_hole_npth", "side": "back",
                  "parameters": { "hole_drill": "3" } }
            ]}]
        });

        let (migrated, changed) = migrate_value(value).unwrap();
        assert!(changed);
        assert_eq!(migrated["formatVersion"], CURRENT_VERSION);
        assert_eq!(migrated["future"], json!({ "keep": true }));
        assert_eq!(
            migrated["definitions"][0]["generator"]["parameters"],
            json!({ "hole_size": "two", "hole_drill": 3, "not_a_parameter": "9" })
        );
        assert_eq!(
            migrated["parts"][0]["generatorParameters"],
            json!({ "hole_size": 3.5, "not_a_parameter": "4" })
        );
        assert_eq!(
            migrated["assemblies"][0]["members"][0]["parameters"],
            json!({ "hole_drill": 3 })
        );
        assert!(
            migrated["parts"][0]["generatorParameters"]
                .get("side")
                .is_none(),
            "the back-side default is a version 1 step, not a version 3 one"
        );
    }

    #[test]
    fn the_version_1_step_states_the_back_side_and_leaves_text_for_version_4() {
        let mut document = version_1().as_object().unwrap().clone();
        version_1_to_2(&mut document);
        assert_eq!(
            document["parts"][1]["generatorParameters"],
            json!({ "hotswap": false, "side": "B" })
        );
        assert_eq!(
            document["definitions"][1]["generator"]["parameters"]["hole_size"],
            "2.2"
        );
        assert_eq!(
            document["parts"][4]["generatorParameters"]["hole_size"],
            " 3.5 "
        );
    }
}

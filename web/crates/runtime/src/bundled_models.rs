#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BundledModel {
    pub id: &'static str,
    pub filename: &'static str,
    pub media_type: &'static str,
    pub source_relative_path: &'static str,
    pub url_path: &'static str,
    pub sha256: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/bundled_ergogen_models.rs"));

pub fn bundled_model(id: &str) -> Option<&'static BundledModel> {
    BUNDLED_MODELS.iter().find(|model| model.id == id)
}

pub fn preview_model_paths() -> impl Iterator<Item = (&'static str, &'static str)> {
    BUNDLED_MODELS
        .iter()
        .map(|model| (model.id, model.url_path))
}

pub async fn is_generator_source(source: &str) -> Result<bool, String> {
    Ok(boardstudio_core::generators::is_generator(source))
}

fn standalone_part(
    definition: &boardstudio_core::model::PartDefinition,
) -> boardstudio_core::model::Part {
    use boardstudio_core::model::{Part, Pose2, Side, Vec2};

    Part {
        id: format!("definition:{}", definition.id),
        definition_id: definition.id.clone(),
        reference: "REF**".into(),
        pose: Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        side: Side::Front,
        keycap: None,
        outline: None,
        locked: None,
        properties: None,
        generator_parameters: Some(
            definition
                .generator
                .as_ref()
                .map(|generator| generator.parameters.clone())
                .unwrap_or_default(),
        ),
    }
}

fn supported_generator(definition: &boardstudio_core::model::PartDefinition) -> bool {
    definition
        .generator
        .as_ref()
        .is_some_and(|generator| boardstudio_core::generators::is_generator(&generator.source))
}

/// Model assets rendered by every generator-backed part and assembly member of a document.
pub async fn generated_model_ids(
    document: &boardstudio_core::model::ProjectDoc,
) -> Result<Vec<String>, String> {
    use boardstudio_core::generators::model_asset_ids;
    use boardstudio_core::model::{AssemblyMember, Part, PartDefinition};

    fn assembly_model_part(member: &AssemblyMember, definition: &PartDefinition) -> Part {
        Part {
            keycap: None,
            outline: None,
            id: member.id.clone(),
            definition_id: definition.id.clone(),
            reference: member.id.clone(),
            pose: member.pose,
            side: member.side.clone(),
            locked: None,
            properties: None,
            generator_parameters: None,
        }
    }

    let mut ids = Vec::new();
    for definition in &document.definitions {
        if !supported_generator(definition) {
            continue;
        }
        for part in document
            .parts
            .iter()
            .filter(|part| part.definition_id == definition.id)
        {
            ids.extend(model_asset_ids(definition, Some(part))?);
        }
    }

    for assembly in &document.assemblies {
        for member in &assembly.members {
            let Some(definition_id) = member.definition_id.as_deref() else {
                continue;
            };
            let Some(mut definition) = document
                .definitions
                .iter()
                .find(|definition| definition.id == definition_id)
                .cloned()
            else {
                continue;
            };
            if !supported_generator(&definition) {
                continue;
            }
            if let Some(generator) = definition.generator.as_mut()
                && let Some(parameters) = &member.parameters
            {
                generator.parameters.extend(parameters.clone());
            }
            let part = assembly_model_part(member, &definition);
            ids.extend(model_asset_ids(&definition, Some(&part))?);
        }
    }
    Ok(ids)
}

/// Resolves model assets used by every standalone footprint definition. Like the reference
/// `modelFiles`, an unused definition is resolved with a default standalone part, while
/// definitions with instances resolve once per part.
pub async fn footprint_export_model_ids(
    document: &boardstudio_core::model::ProjectDoc,
) -> Result<Vec<String>, String> {
    use boardstudio_core::generators::model_asset_ids;
    use std::collections::HashSet;

    let mut seen = HashSet::new();
    let mut ids = Vec::new();
    let mut push_unique = |values: Vec<String>| {
        for id in values {
            if seen.insert(id.clone()) {
                ids.push(id);
            }
        }
    };

    for definition in &document.definitions {
        push_unique(
            definition
                .models
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|model| model.asset_id.clone())
                .collect(),
        );
        if !supported_generator(definition) {
            continue;
        }
        let parts = document
            .parts
            .iter()
            .filter(|part| part.definition_id == definition.id)
            .collect::<Vec<_>>();
        if parts.is_empty() {
            push_unique(model_asset_ids(
                definition,
                Some(&standalone_part(definition)),
            )?);
        } else {
            for part in parts {
                push_unique(model_asset_ids(definition, Some(part))?);
            }
        }
    }
    Ok(ids)
}

/// Resolve the generator-authored model bindings. Assembly authoring calls this when a
/// designer chooses to edit model defaults for a generated component.
pub async fn model_bindings(
    definition: &boardstudio_core::model::PartDefinition,
    part: &boardstudio_core::model::Part,
) -> Result<Vec<boardstudio_core::model::PartModel>, String> {
    if !supported_generator(definition) {
        return Ok(definition.models.clone().unwrap_or_default());
    }
    boardstudio_core::generators::model_bindings(definition, Some(part))
}

pub async fn generated_model_asset_ids_for_paths(
    paths: &[String],
) -> Result<Vec<Option<String>>, String> {
    Ok(boardstudio_core::generators::model_asset_ids_for_paths(
        paths,
    ))
}

#[cfg(target_arch = "wasm32")]
pub async fn bundled_model_bytes(id: &str) -> Result<Vec<u8>, String> {
    use js_sys::Uint8Array;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let model = bundled_model(id).ok_or_else(|| format!("Bundled model is unavailable: {id}"))?;
    let window = web_sys::window().ok_or_else(|| "window unavailable".to_owned())?;
    let url = crate::runtime::resource_url(model.url_path)?;
    let response = JsFuture::from(window.fetch_with_str(&url))
        .await
        .map_err(|error| format!("Could not load bundled model {}: {error:?}", model.filename))?
        .dyn_into::<web_sys::Response>()
        .map_err(|error| format!("Could not read bundled model {}: {error:?}", model.filename))?;
    if !response.ok() {
        return Err(format!(
            "Could not load bundled model {} ({})",
            model.filename,
            response.status()
        ));
    }
    let buffer =
        JsFuture::from(response.array_buffer().map_err(|error| {
            format!("Could not read bundled model {}: {error:?}", model.filename)
        })?)
        .await
        .map_err(|error| format!("Could not read bundled model {}: {error:?}", model.filename))?;
    Ok(Uint8Array::new(&buffer).to_vec())
}

#[cfg(test)]
mod tests {
    use super::{BUNDLED_MODELS, BundledModel, bundled_model};
    use sha2::{Digest, Sha256};
    use std::collections::BTreeSet;

    #[test]
    fn catalogue_covers_each_react_vendor_model_once_with_safe_static_paths() {
        let models = BUNDLED_MODELS;
        assert_eq!(models.len(), 88);
        let ids: BTreeSet<_> = models.iter().map(|model| model.id).collect();
        let source_paths: BTreeSet<_> = models
            .iter()
            .map(|model| model.source_relative_path)
            .collect();
        let urls: BTreeSet<_> = models.iter().map(|model| model.url_path).collect();
        assert_eq!(ids.len(), models.len());
        assert_eq!(source_paths.len(), models.len());
        assert_eq!(urls.len(), models.len());
        for model in models {
            assert!(model.id.starts_with("bundled-model:"));
            assert!(model.url_path.starts_with("assets/ergogen-models/model-"));
            assert_eq!(model.sha256.len(), 64);
            assert!(model.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()));
            assert!(
                !model
                    .url_path
                    .chars()
                    .any(|ch| matches!(ch, '%' | '#' | '?' | '\\' | ':'))
            );
            assert!(
                !model
                    .url_path
                    .split('/')
                    .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
            );
        }
    }

    #[test]
    fn catalogue_digest_matches_the_exact_staged_vendor_source_file() {
        let vendor_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../ergogen/library/vendor");
        for model in BUNDLED_MODELS {
            let bytes = std::fs::read(vendor_root.join(model.source_relative_path))
                .expect("catalogue source file is available");
            let digest = Sha256::digest(&bytes);
            let digest = digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            assert_eq!(model.sha256, digest, "{}", model.id);
        }
    }

    #[test]
    fn lookup_preserves_react_thqwgd_source_to_saved_name_aliases() {
        let cases = [
            (
                "bundled-model:thqwgd001/THQWGD001 #1.stp",
                "THQWGD001-rotation.stp",
                "THQWGD001 #1.stp",
            ),
            (
                "bundled-model:thqwgd001/THQWGD001C [2pin] #1.stp",
                "THQWGD001C-2pin.stp",
                "THQWGD001C [2pin] #1.stp",
            ),
            (
                "bundled-model:thqwgd001/THQWGD001C [4pin] #1.stp",
                "THQWGD001C-4pin.stp",
                "THQWGD001C [4pin] #1.stp",
            ),
        ];
        for (id, source_filename, saved_filename) in cases {
            let model: &BundledModel = bundled_model(id).expect("known React model alias");
            assert!(model.source_relative_path.ends_with(source_filename));
            assert_eq!(model.filename, saved_filename);
            assert_eq!(model.media_type, "model/step");
        }
    }

    #[test]
    fn lookup_does_not_turn_unknown_identifiers_into_static_paths() {
        assert!(bundled_model("bundled-model:unknown/secret.step").is_none());
        assert!(bundled_model("https://example.test/model.step").is_none());
        assert!(bundled_model("../model.step").is_none());
    }

    #[test]
    fn lookup_preserves_nested_reference_filenames_from_react_catalogue() {
        let model = bundled_model("bundled-model:infused-kim/trackpoint/TP_Cap_Green_T430.step")
            .expect("nested React model reference");
        assert_eq!(model.filename, "trackpoint/TP_Cap_Green_T430.step");
        assert!(
            model
                .source_relative_path
                .ends_with("trackpoint/TP_Cap_Green_T430.step")
        );
    }
}

#[cfg(test)]
mod generator_tests {
    use super::*;
    use boardstudio_core::model::ProjectDoc;
    use serde_json::json;

    #[test]
    fn generated_models_use_generator_part_overrides() {
        let definition = boardstudio_core::generators::catalogue()
            .unwrap()
            .into_iter()
            .find(|definition| {
                definition.generator.as_ref().is_some_and(|generator| {
                    generator.source == "ceoloide/switch_gateron_ks27_ks33"
                })
            })
            .unwrap();
        let mut document = ProjectDoc::empty("models-test", "Generated model override");
        document.parts.push(
            serde_json::from_value(json!({
                "id":"switch", "definitionId":definition.id, "reference":"SW1",
                "pose":{"at":{"x":0,"y":0},"rotation":0}, "side":"back",
                "generatorParameters":{
                    "switch_3dmodel_filename":"boardstudio-asset:local-switch",
                    "hotswap_3dmodel_filename":"",
                    "keycap_3dmodel_filename":""
                }
            }))
            .unwrap(),
        );
        document.definitions.push(definition);
        let ids = futures_block_on(generated_model_ids(&document)).unwrap();
        assert_eq!(ids, vec!["local-switch"]);
    }

    #[test]
    fn preview_paths_use_the_bundled_asset_identity_helper() {
        let paths = vec![
            "${KIPRJMOD}/models/boardstudio/kiswitch/SW_Cherry_MX_PCB.stp".to_owned(),
            "${KIPRJMOD}/models/boardstudio/thqwgd001/THQWGD001-rotation.stp".to_owned(),
            "${EG_INFUSED_KIM_3D_MODELS}/diode/example.wrl".to_owned(),
            "C:/untrusted/model.step".to_owned(),
        ];
        assert_eq!(
            futures_block_on(generated_model_asset_ids_for_paths(&paths)).unwrap(),
            vec![
                Some("bundled-model:kiswitch/SW_Cherry_MX_PCB.stp".to_owned()),
                Some("bundled-model:thqwgd001/THQWGD001-rotation.stp".to_owned()),
                Some("bundled-model:infused-kim/diode/example.wrl".to_owned()),
                None,
            ]
        );
    }

    /// These async wrappers do no asynchronous work, so one poll completes them.
    fn futures_block_on<T>(future: impl std::future::Future<Output = T>) -> T {
        let mut future = std::pin::pin!(future);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(value) => value,
            std::task::Poll::Pending => panic!("wrapper suspended"),
        }
    }
}

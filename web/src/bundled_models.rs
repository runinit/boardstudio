#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BundledModel {
    pub(crate) id: &'static str,
    pub(crate) filename: &'static str,
    pub(crate) media_type: &'static str,
    pub(crate) source_relative_path: &'static str,
    pub(crate) url_path: &'static str,
    pub(crate) sha256: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/bundled_ergogen_models.rs"));

pub(crate) fn bundled_model(id: &str) -> Option<&'static BundledModel> {
    BUNDLED_MODELS.iter().find(|model| model.id == id)
}

pub(crate) fn preview_model_paths() -> impl Iterator<Item = (&'static str, &'static str)> {
    BUNDLED_MODELS
        .iter()
        .map(|model| (model.id, model.url_path))
}

#[cfg(all(target_arch = "wasm32", feature = "page"))]
pub(crate) async fn generated_model_ids(
    document: &boardstudio_core::model::ProjectDoc,
) -> Result<Vec<String>, String> {
    generated_model_ids_with_module(document, layout_generator_module().await?)
}

#[cfg(all(target_arch = "wasm32", feature = "page"))]
pub(crate) async fn generated_model_asset_ids_for_paths(
    paths: &[String],
) -> Result<Vec<Option<String>>, String> {
    model_asset_ids_for_paths_with_module(paths, layout_generator_module().await?)
}

#[cfg(all(target_arch = "wasm32", feature = "page"))]
async fn layout_generator_module() -> Result<wasm_bindgen::JsValue, String> {
    use js_sys::Function;
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;

    let url = crate::runtime::resource_url("assets/layout-generators/src/index.js")?;
    let importer = Function::new_with_args("url", "return import(url)");
    JsFuture::from(
        importer
            .call1(&JsValue::NULL, &url.into())
            .map_err(|error| format!("Could not load Ergogen generator metadata: {error:?}"))?
            .dyn_into::<js_sys::Promise>()
            .map_err(|error| format!("Could not load Ergogen generator metadata: {error:?}"))?,
    )
    .await
    .map_err(|error| format!("Could not load Ergogen generator metadata: {error:?}"))
}

#[cfg(all(target_arch = "wasm32", feature = "page"))]
fn model_asset_ids_for_paths_with_module(
    paths: &[String],
    module: wasm_bindgen::JsValue,
) -> Result<Vec<Option<String>>, String> {
    use js_sys::{Array, Function, JsString};
    use wasm_bindgen::{JsCast, JsValue};

    let resolver = js_sys::Reflect::get(&module, &JsString::from("modelAssetIdsForPaths"))
        .map_err(|error| format!("Ergogen model path resolver is unavailable: {error:?}"))?
        .dyn_into::<Function>()
        .map_err(|error| format!("Ergogen model path resolver is unavailable: {error:?}"))?;
    let paths = Array::from_iter(paths.iter().map(|path| JsValue::from_str(path)));
    let result = resolver
        .call1(&module, &paths)
        .map_err(|error| format!("Could not resolve Ergogen model paths: {error:?}"))?;
    serde_wasm_bindgen::from_value(result)
        .map_err(|error| format!("Ergogen model path results are invalid: {error}"))
}

#[cfg(all(target_arch = "wasm32", feature = "page"))]
fn generated_model_ids_with_module(
    document: &boardstudio_core::model::ProjectDoc,
    module: wasm_bindgen::JsValue,
) -> Result<Vec<String>, String> {
    use boardstudio_core::model::{AssemblyMember, Part, PartDefinition};
    use js_sys::{Function, JsString};
    use serde::Serialize;
    use wasm_bindgen::{JsCast, JsValue};

    let is_ergogen = module_function(&module, "isErgogen")?;
    let model_asset_ids = module_function(&module, "modelAssetIds")?;
    let mut ids = Vec::new();
    for definition in &document.definitions {
        if !is_supported_ergogen(&is_ergogen, &module, definition)? {
            continue;
        }
        for part in document
            .parts
            .iter()
            .filter(|part| part.definition_id == definition.id)
        {
            ids.extend(rendered_model_ids(
                &model_asset_ids,
                &module,
                definition,
                part,
            )?);
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
            if !is_supported_ergogen(&is_ergogen, &module, &definition)? {
                continue;
            }
            if let Some(generator) = definition.generator.as_mut()
                && let Some(parameters) = &member.parameters
            {
                generator.parameters.extend(parameters.clone());
            }
            let part = assembly_model_part(member, &definition);
            ids.extend(rendered_model_ids(
                &model_asset_ids,
                &module,
                &definition,
                &part,
            )?);
        }
    }
    fn module_function(module: &JsValue, name: &str) -> Result<Function, String> {
        js_sys::Reflect::get(module, &JsString::from(name))
            .map_err(|error| format!("Ergogen {name} export is unavailable: {error:?}"))?
            .dyn_into::<Function>()
            .map_err(|error| format!("Ergogen {name} export is unavailable: {error:?}"))
    }

    fn is_supported_ergogen(
        predicate: &Function,
        module: &JsValue,
        definition: &PartDefinition,
    ) -> Result<bool, String> {
        let Some(generator) = &definition.generator else {
            return Ok(false);
        };
        predicate
            .call1(module, &generator.source.clone().into())
            .map_err(|error| format!("Could not inspect {} generator: {error:?}", definition.id))
            .map(|value| value.as_bool().unwrap_or(false))
    }

    fn rendered_model_ids(
        render: &Function,
        module: &JsValue,
        definition: &PartDefinition,
        part: &Part,
    ) -> Result<Vec<String>, String> {
        let definition = to_js_value(definition)?;
        let part = to_js_value(part)?;
        let ids = render
            .call2(module, &definition, &part)
            .map_err(|error| format!("Could not resolve generated model references: {error:?}"))?;
        serde_wasm_bindgen::from_value(ids)
            .map_err(|error| format!("Generated model references are invalid: {error}"))
    }

    fn to_js_value(value: &impl Serialize) -> Result<JsValue, String> {
        let json = serde_json::to_string(value).map_err(|error| error.to_string())?;
        js_sys::JSON::parse(&json)
            .map_err(|error| format!("Could not prepare Ergogen query: {error:?}"))
    }

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

    Ok(ids)
}

#[cfg(all(target_arch = "wasm32", feature = "page"))]
pub(crate) async fn bundled_model_bytes(id: &str) -> Result<Vec<u8>, String> {
    use js_sys::Uint8Array;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let model =
        bundled_model(id).ok_or_else(|| format!("Bundled Ergogen model is unavailable: {id}"))?;
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
            assert!(model.id.starts_with("ergogen:model:"));
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
        let vendor_root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ergogen/library/vendor");
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
                "ergogen:model:thqwgd001/THQWGD001 #1.stp",
                "THQWGD001-rotation.stp",
                "THQWGD001 #1.stp",
            ),
            (
                "ergogen:model:thqwgd001/THQWGD001C [2pin] #1.stp",
                "THQWGD001C-2pin.stp",
                "THQWGD001C [2pin] #1.stp",
            ),
            (
                "ergogen:model:thqwgd001/THQWGD001C [4pin] #1.stp",
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
        assert!(bundled_model("ergogen:model:unknown/secret.step").is_none());
        assert!(bundled_model("https://example.test/model.step").is_none());
        assert!(bundled_model("../model.step").is_none());
    }

    #[test]
    fn lookup_preserves_nested_reference_filenames_from_react_catalogue() {
        let model = bundled_model("ergogen:model:infused-kim/trackpoint/TP_Cap_Green_T430.step")
            .expect("nested React model reference");
        assert_eq!(model.filename, "trackpoint/TP_Cap_Green_T430.step");
        assert!(
            model
                .source_relative_path
                .ends_with("trackpoint/TP_Cap_Green_T430.step")
        );
    }
}

#[cfg(all(test, target_arch = "wasm32", feature = "page"))]
mod wasm_tests {
    use super::{generated_model_ids_with_module, model_asset_ids_for_paths_with_module};
    use boardstudio_core::model::{PartDefinition, ProjectDoc};
    use js_sys::Function;
    use serde_json::json;
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    async fn generated_models_use_packaged_generator_part_overrides() {
        let url = match option_env!("BOARDSTUDIO_TEST_LAYOUT_GENERATOR_MODULE_URL") {
            Some(url) => url,
            None => panic!("run scripts/web/test-portable-models.mjs to provide packaged module"),
        };
        let importer = Function::new_with_args("url", "return import(url)");
        let module = JsFuture::from(
            importer
                .call1(&JsValue::NULL, &url.into())
                .unwrap()
                .dyn_into::<js_sys::Promise>()
                .unwrap(),
        )
        .await
        .unwrap();
        let catalogue = js_sys::Reflect::get(&module, &"catalogue".into())
            .unwrap()
            .dyn_into::<Function>()
            .unwrap()
            .call0(&module)
            .unwrap();
        let definitions: Vec<PartDefinition> = serde_wasm_bindgen::from_value(catalogue).unwrap();
        let definition = definitions
            .into_iter()
            .find(|definition| definition.id == "ergogen:ceoloide/switch_gateron_ks27_ks33")
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
        assert_eq!(
            generated_model_ids_with_module(&document, module).unwrap(),
            vec!["local-switch"]
        );
    }

    #[wasm_bindgen_test]
    async fn native_preview_paths_use_the_packaged_ergogen_asset_identity_helper() {
        let url = match option_env!("BOARDSTUDIO_TEST_LAYOUT_GENERATOR_MODULE_URL") {
            Some(url) => url,
            None => panic!("run scripts/web/test-portable-models.mjs to provide packaged module"),
        };
        let importer = Function::new_with_args("url", "return import(url)");
        let module = JsFuture::from(
            importer
                .call1(&JsValue::NULL, &url.into())
                .unwrap()
                .dyn_into::<js_sys::Promise>()
                .unwrap(),
        )
        .await
        .unwrap();
        let paths = vec![
            "${KIPRJMOD}/models/boardstudio/kiswitch/SW_Cherry_MX_PCB.stp".to_owned(),
            "${KIPRJMOD}/models/boardstudio/thqwgd001/THQWGD001-rotation.stp".to_owned(),
            "${EG_INFUSED_KIM_3D_MODELS}/diode/example.wrl".to_owned(),
            "C:/untrusted/model.step".to_owned(),
        ];
        assert_eq!(
            model_asset_ids_for_paths_with_module(&paths, module).unwrap(),
            vec![
                Some("ergogen:model:kiswitch/SW_Cherry_MX_PCB.stp".to_owned()),
                Some("ergogen:model:thqwgd001/THQWGD001-rotation.stp".to_owned()),
                Some("ergogen:model:infused-kim/diode/example.wrl".to_owned()),
                None,
            ]
        );
    }
}

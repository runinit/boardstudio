use super::catalogue::{construction_definition_with_support, load_ergogen_module};
use crate::physical_setup::{self, SetupIntent};
use boardstudio_core::model::ProjectDoc;

/// Build a typed setup proposal through the retained, packaged Ergogen implementation.
/// This is private page-binary preparation only; it neither submits a Session edit nor
/// changes selection. The Editor owner is added in the dependent setup intent slice.
#[allow(dead_code)] // Consumed by the dependent Editor-owner ticket.
pub(crate) async fn prepare_proposal(
    accepted: &ProjectDoc,
    intent: SetupIntent,
) -> Result<ProjectDoc, String> {
    let module = if matches!(&intent, SetupIntent::ReversibleLayout(_)) {
        Some(load_ergogen_module().await?)
    } else {
        None
    };
    prepare_proposal_with_module(accepted, intent, module.as_ref())
}

fn prepare_proposal_with_module(
    accepted: &ProjectDoc,
    intent: SetupIntent,
    module: Option<&wasm_bindgen::JsValue>,
) -> Result<ProjectDoc, String> {
    match intent {
        SetupIntent::ReversibleLayout(enabled) => {
            let module = module.ok_or("Ergogen normalizer is unavailable")?;
            physical_setup::propose(
                accepted,
                SetupIntent::ReversibleLayout(enabled),
                |definition, enabled| {
                    construction_definition_with_support(module, definition.clone(), enabled)
                },
            )
        }
        intent => physical_setup::propose(accepted, intent, |definition, _| {
            Ok((definition.clone(), false))
        }),
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use super::super::catalogue::import_ergogen_module;
    use super::*;
    use boardstudio_core::model::HardwareConfiguration;
    use serde_json::json;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    async fn reversible_proposal_uses_the_packaged_gateron_normalizer() {
        let module_url = option_env!("BOARDSTUDIO_TEST_LAYOUT_GENERATOR_MODULE_URL").expect(
            "run scripts/web/test-physical-setup-proposal.mjs to provide packaged module URL",
        );
        let module = import_ergogen_module(module_url)
            .await
            .expect("import generated layout-generator asset");
        let catalogue = super::super::catalogue::call_catalogue(&module).unwrap();
        let gateron = catalogue
            .into_iter()
            .find(|definition| {
                definition.generator.as_ref().is_some_and(|generator| {
                    generator.source == "ceoloide/switch_gateron_ks27_ks33"
                })
            })
            .expect("packaged catalogue contains Gateron KS27/KS33");
        let original = gateron.clone();
        let mut document = ProjectDoc::empty("packaged-test", "Packaged module test");
        document.definitions = vec![gateron.clone()];
        document.parts.push(
            serde_json::from_value(json!({
                "id":"gateron-1","definitionId":gateron.id,"reference":"SW1",
                "pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front",
                "generatorParameters":{"reversible":false,"custom":"keep"}
            }))
            .unwrap(),
        );
        document.hardware = Some(HardwareConfiguration::default());
        let accepted = document.clone();

        let proposal = prepare_proposal_with_module(
            &document,
            SetupIntent::ReversibleLayout(true),
            Some(&module),
        )
        .expect("prepare reversible project proposal");

        assert_eq!(
            document, accepted,
            "proposal does not mutate accepted input"
        );
        assert_eq!(
            document.definitions[0], original,
            "source definition remains intact"
        );
        let generator = proposal.definitions[0].generator.as_ref().unwrap();
        assert_eq!(generator.parameters.get("reversible"), Some(&json!(true)));
        assert_eq!(generator.parameters.get("hotswap"), Some(&json!(false)));
        assert_eq!(generator.parameters.get("solder"), Some(&json!(true)));
        let part = &proposal.parts[0];
        assert_eq!(
            part.generator_parameters
                .as_ref()
                .unwrap()
                .get("reversible"),
            Some(&json!(true))
        );
        assert_eq!(
            part.generator_parameters.as_ref().unwrap().get("custom"),
            Some(&json!("keep"))
        );
        assert_eq!(
            proposal.parameters.get("reversibleLayout"),
            Some(&json!(true))
        );
    }
}

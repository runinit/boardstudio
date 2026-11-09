//! Supported native-half document used by mirror replacement and browser-boundary acceptance.

use super::synthetic_half;
use boardstudio_core::generators;
use boardstudio_core::model::PartDefinition;
use serde_json::{Value, json};

fn catalogue_definition(source: &str) -> PartDefinition {
    generators::catalogue()
        .expect("bundled generator catalogue")
        .into_iter()
        .find(|definition| {
            definition
                .generator
                .as_ref()
                .is_some_and(|generator| generator.source == source)
        })
        .expect("supported bundled generator")
}

/// Builds a matrix-bearing source board with named bundled switch/diode/controller semantics.
pub(crate) fn supported_source_document() -> Value {
    let mut document = synthetic_half::document();
    for source in ["ceoloide/switch_mx", "ceoloide/diode_tht_sod123"] {
        let replacement = serde_json::to_value(catalogue_definition(source)).unwrap();
        let definition_id = replacement["id"].clone();
        let existing = document["definitions"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|definition| definition["id"] == definition_id)
            .unwrap();
        *existing = replacement;
    }
    document["definitions"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::to_value(catalogue_definition("ceoloide/mcu_nice_nano")).unwrap());
    document["parts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|part| part["id"] == "left/U1")
        .unwrap()["definitionId"] = json!("generator:ceoloide/mcu_nice_nano");
    let electrical = &mut document["hardware"]["boards"][0];
    electrical["controllerPartId"] = json!("left/U1");
    electrical["mode"] = json!("matrix");
    document
}

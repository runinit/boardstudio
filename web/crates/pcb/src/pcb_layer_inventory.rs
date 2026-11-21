//! Generator-layer inventory requests and their reuse rules, kept free of browser APIs so the
//! rules run under native `cargo test`.
use boardstudio_application::Scope;
use boardstudio_model::model::{PartDefinition, Side};
use boardstudio_web_ui_shared::board_layer::resolve_board_layer;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GeneratorLayerSource {
    pub(crate) definition: PartDefinition,
    pub(crate) parameters: Option<std::collections::BTreeMap<String, serde_json::Value>>,
    pub(crate) side: Side,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GeneratorLayerRequest {
    pub(crate) scope: Scope,
    pub(crate) sources: Vec<GeneratorLayerSource>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GeneratorLayerInventory {
    pub(crate) request: GeneratorLayerRequest,
    pub(crate) layers: BTreeSet<String>,
}

pub(crate) fn current_generator_layers(
    request: &GeneratorLayerRequest,
    inventory: Option<&GeneratorLayerInventory>,
) -> BTreeSet<String> {
    inventory
        .filter(|inventory| inventory.request == *request)
        .map(|inventory| inventory.layers.clone())
        .unwrap_or_default()
}

pub(crate) fn extend_generator_layers(
    layers: &mut BTreeSet<String>,
    side: &Side,
    result: Result<Option<std::rc::Rc<Vec<crate::footprint_forms::Graphic>>>, String>,
) {
    if let Ok(Some(drawings)) = result {
        for graphic in drawings.iter() {
            layers.insert(resolve_board_layer(&graphic.layer, side));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::footprint_forms::{Graphic, Point, Shape};
    use boardstudio_application::SessionEpoch;
    use boardstudio_model::model::{PartGenerator, PartKind};
    use std::rc::Rc;

    #[test]
    fn completed_inventory_from_an_old_board_scope_is_not_reused() {
        let scope = |board_id: &str| Scope {
            session_epoch: SessionEpoch(2),
            document_id: "doc".into(),
            board_id: board_id.into(),
            instance_id: None,
        };
        let old_request = GeneratorLayerRequest {
            scope: scope("left"),
            sources: vec![],
        };
        let current_request = GeneratorLayerRequest {
            scope: scope("right"),
            sources: vec![],
        };
        let previous = GeneratorLayerInventory {
            request: old_request,
            layers: BTreeSet::from(["F.SilkS".into()]),
        };
        assert!(current_generator_layers(&current_request, Some(&previous)).is_empty());
    }

    #[test]
    fn completed_inventory_from_an_old_generator_definition_is_not_reused() {
        let scope = Scope {
            session_epoch: SessionEpoch(2),
            document_id: "doc".into(),
            board_id: "left".into(),
            instance_id: None,
        };
        let definition = |id: &str| PartDefinition {
            hardware_profile: None,
            input_profile: None,
            id: id.into(),
            name: id.into(),
            kind: PartKind::Custom,
            keycap: None,
            envelope_source: None,
            kicad_source: None,
            terminals: Default::default(),
            matrix_terminals: None,
            envelope_notice: None,
            courtyard: vec![],
            pads: vec![],
            models: None,
            generator: Some(PartGenerator {
                source: id.into(),
                version: "1".into(),
                parameters: Default::default(),
            }),
            mechanical_profile: None,
        };
        let source = |id: &str| GeneratorLayerSource {
            definition: definition(id),
            parameters: None,
            side: Side::Front,
        };
        let old_request = GeneratorLayerRequest {
            scope: scope.clone(),
            sources: vec![source("old")],
        };
        let current_request = GeneratorLayerRequest {
            scope,
            sources: vec![source("new")],
        };
        let previous = GeneratorLayerInventory {
            request: old_request,
            layers: BTreeSet::from(["F.SilkS".into()]),
        };
        assert!(current_generator_layers(&current_request, Some(&previous)).is_empty());
    }

    #[test]
    fn one_generator_failure_does_not_drop_other_emitted_layer_rows() {
        let mut layers = BTreeSet::new();
        extend_generator_layers(
            &mut layers,
            &Side::Front,
            Ok(Some(Rc::new(vec![Graphic {
                layer: "F.SilkS".into(),
                shape: Shape::Circle(Point(0.0, 0.0), 1.0),
            }]))),
        );
        extend_generator_layers(&mut layers, &Side::Front, Err("one bad source".into()));
        assert_eq!(layers, BTreeSet::from(["F.SilkS".into()]));
    }
}

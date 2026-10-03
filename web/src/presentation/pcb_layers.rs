//! Layer inventory and visibility grouping for accepted host PCB scene geometry.
use super::canvas_layers::{CanvasLayer, CanvasLayerGroup, CanvasLayers};
use super::footprint_graphics::{generator_drawings, resolve_board_layer};
use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_core::model::{Pad, Part, PartDefinition, ProjectDoc, Side};
use dioxus::prelude::*;
use std::collections::{BTreeSet, HashSet};

#[derive(Clone, Debug, PartialEq)]
struct GeneratorLayerSource {
    definition: PartDefinition,
    parameters: Option<std::collections::BTreeMap<String, serde_json::Value>>,
    side: Side,
}

#[derive(Clone, Debug, PartialEq)]
struct GeneratorLayerRequest {
    scope: Scope,
    sources: Vec<GeneratorLayerSource>,
}

#[derive(Clone, Debug, PartialEq)]
struct GeneratorLayerInventory {
    request: GeneratorLayerRequest,
    layers: BTreeSet<String>,
}

#[derive(Props, Clone, PartialEq)]
pub(super) struct PcbLayerControlsProps {
    snapshot: AcceptedSnapshot,
    scope: Scope,
}

#[component]
pub(super) fn PcbLayerControls(props: PcbLayerControlsProps) -> Element {
    let groups = use_layer_groups(&props.snapshot, &props.scope);
    rsx! {
        CanvasLayers {
            trigger_id: String::from("m1-pcb-layers-trigger"),
            list_id: String::from("m1-pcb-layers-list"),
            groups,
        }
    }
}

pub(super) fn use_layer_groups(
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
) -> Vec<CanvasLayerGroup> {
    let request = GeneratorLayerRequest {
        scope: scope.clone(),
        sources: generator_sources(snapshot, scope),
    };
    let inventory = use_resource(use_reactive(&request, |request| async move {
        collect_generator_layers(request).await
    }));
    let generated_layers = current_generator_layers(&request, inventory.read().as_ref());
    if snapshot.session_epoch != scope.session_epoch {
        return Vec::new();
    }
    layer_groups_for_scene(
        &snapshot.document,
        &snapshot.scene.board_contours,
        &snapshot.scene.contours,
        scope,
        generated_layers,
    )
}

fn generator_sources(snapshot: &AcceptedSnapshot, scope: &Scope) -> Vec<GeneratorLayerSource> {
    if snapshot.session_epoch != scope.session_epoch || snapshot.document.id != scope.document_id {
        return Vec::new();
    }
    let Some(board) = snapshot
        .document
        .boards
        .iter()
        .find(|board| board.id == scope.board_id)
    else {
        return Vec::new();
    };
    let members: HashSet<&str> = board.part_ids.iter().map(String::as_str).collect();
    snapshot
        .document
        .parts
        .iter()
        .filter_map(|part| {
            if !members.contains(part.id.as_str()) {
                return None;
            }
            let definition = snapshot
                .document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)?;
            definition.generator.as_ref()?;
            Some(GeneratorLayerSource {
                definition: definition.clone(),
                parameters: part.generator_parameters.clone(),
                side: part.side.clone(),
            })
        })
        .collect()
}

fn current_generator_layers(
    request: &GeneratorLayerRequest,
    inventory: Option<&GeneratorLayerInventory>,
) -> BTreeSet<String> {
    inventory
        .filter(|inventory| inventory.request == *request)
        .map(|inventory| inventory.layers.clone())
        .unwrap_or_default()
}

async fn collect_generator_layers(request: GeneratorLayerRequest) -> GeneratorLayerInventory {
    let mut layers = BTreeSet::new();
    for source in &request.sources {
        let result = generator_drawings(
            source.definition.clone(),
            source.parameters.clone(),
            Some(false),
        )
        .await;
        extend_generator_layers(&mut layers, &source.side, result);
    }
    GeneratorLayerInventory { request, layers }
}

fn extend_generator_layers(
    layers: &mut BTreeSet<String>,
    side: &Side,
    result: Result<Option<super::footprint_graphics::Drawings>, String>,
) {
    if let Ok(Some(drawings)) = result {
        for graphic in drawings.iter() {
            layers.insert(resolve_board_layer(&graphic.layer, side));
        }
    }
}

pub(super) fn layer_groups_for_scene(
    document: &ProjectDoc,
    board_contours: &[boardstudio_core::model::BoardContours],
    scene_contours: &[boardstudio_core::model::Contour],
    scope: &Scope,
    mut layers: BTreeSet<String>,
) -> Vec<CanvasLayerGroup> {
    if document.id != scope.document_id {
        return Vec::new();
    }
    let Some(board) = document
        .boards
        .iter()
        .find(|board| board.id == scope.board_id)
    else {
        return Vec::new();
    };
    let contours = board_contours
        .iter()
        .find(|contours| contours.board_id == board.id)
        .map(|entry| entry.contours.as_slice())
        .or_else(|| (document.boards.len() == 1).then_some(scene_contours))
        .unwrap_or_default();
    let has_edge_cuts = !contours.is_empty() || layers.contains("Edge.Cuts");
    let members: HashSet<&str> = board.part_ids.iter().map(String::as_str).collect();
    let parts: Vec<&Part> = document
        .parts
        .iter()
        .filter(|part| members.contains(part.id.as_str()))
        .collect();

    let (mut has_courtyards, mut has_pads, mut has_holes, mut has_references) =
        (false, false, false, false);
    for part in &parts {
        has_references |= !part.reference.is_empty();
        if let Some(definition) = document
            .definitions
            .iter()
            .find(|definition| definition.id == part.definition_id)
        {
            has_courtyards |= !definition.courtyard.is_empty();
            for pad in &definition.pads {
                has_holes |= pad.drill.is_some();
                if pad.plated != Some(false) {
                    layers.extend(pad_copper_layers(pad, &part.side));
                    has_pads = true;
                }
            }
        }
    }

    let copper = layers
        .iter()
        .filter(|layer| layer.ends_with(".Cu"))
        .map(|layer| CanvasLayer::hidden(layer.clone(), visible_layer_label(layer)))
        .collect::<Vec<_>>();
    let technical = layers
        .iter()
        .filter(|layer| {
            layer.contains('.') && !layer.ends_with(".Cu") && layer.as_str() != "Edge.Cuts"
        })
        .map(|layer| CanvasLayer::hidden(layer.clone(), layer.clone()))
        .collect::<Vec<_>>();
    let mut objects = layers
        .iter()
        .filter(|layer| !layer.contains('.') && layer.as_str() != "Edge.Cuts")
        .map(|layer| CanvasLayer::hidden(layer.clone(), visible_layer_label(layer)))
        .collect::<Vec<_>>();
    if has_edge_cuts {
        objects.push(CanvasLayer::hidden("Edge.Cuts", "Board outline"));
    }
    if has_courtyards {
        objects.push(CanvasLayer::hidden("Courtyards", "Courtyards"));
    }
    if has_pads {
        objects.push(CanvasLayer::hidden("Pads", "Pads"));
    }
    if has_holes {
        objects.push(CanvasLayer::hidden("Holes", "Holes"));
    }
    if has_references {
        objects.push(CanvasLayer::hidden("References", "References"));
    }
    let mut groups = Vec::new();
    if !copper.is_empty() {
        groups.push(CanvasLayerGroup::titled("Copper", copper));
    }
    if !technical.is_empty() {
        groups.push(CanvasLayerGroup::titled("Technical", technical));
    }
    if !objects.is_empty() {
        groups.push(CanvasLayerGroup::titled(
            "Objects",
            std::mem::take(&mut objects),
        ));
    }
    groups.push(CanvasLayerGroup::titled(
        "Mounted modules",
        [
            ("module-footprints", "Footprints"),
            ("module-outlines", "Board outlines"),
            ("module-clearances", "Clearance & service"),
            ("module-holes", "Mounting holes"),
            ("module-standoffs", "Standoffs"),
            ("module-silkscreen-front", "Front silkscreen"),
            ("module-silkscreen-back", "Back silkscreen"),
            ("module-fab-front", "Front fabrication"),
            ("module-fab-back", "Back fabrication"),
            ("module-findings", "Findings & clearances"),
        ]
        .into_iter()
        .map(|(id, label)| CanvasLayer::module_hidden(id, label))
        .collect(),
    ));
    groups
}

fn visible_layer_label(layer: &str) -> String {
    match layer {
        "F.Cu" => "Front copper".into(),
        "B.Cu" => "Back copper".into(),
        "Edge.Cuts" => "Board outline".into(),
        _ => layer.to_owned(),
    }
}

fn pad_copper_layers(pad: &Pad, part_side: &Side) -> Vec<String> {
    if pad.drill.is_some() {
        return vec!["F.Cu".into(), "B.Cu".into()];
    }
    let layer = if matches!(pad.side.as_ref(), Some(Side::Back)) {
        "B.Cu"
    } else {
        "F.Cu"
    };
    vec![resolve_board_layer(layer, part_side)]
}

pub(super) fn pad_is_visible(pad: &Pad, part_side: &Side, hidden: &BTreeSet<String>) -> bool {
    if pad.plated == Some(false) || hidden.contains("Pads") {
        return false;
    }
    if pad.drill.is_some() {
        !hidden.contains("F.Cu") || !hidden.contains("B.Cu")
    } else {
        let layer = if matches!(pad.side.as_ref(), Some(Side::Back)) {
            "B.Cu"
        } else {
            "F.Cu"
        };
        !hidden.contains(&resolve_board_layer(layer, part_side))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::footprint_forms::{Graphic, Point, Shape};
    use boardstudio_application::SessionEpoch;
    use boardstudio_core::model::{PadShape, PartGenerator, PartKind, Vec2};
    use std::rc::Rc;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    fn pad(side: Option<Side>, drill: Option<f64>) -> Pad {
        Pad {
            id: "pad".into(),
            number: "1".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
            size: Vec2 { x: 2.0, y: 2.0 },
            shape: PadShape::Rect,
            drill,
            plated: Some(true),
            side,
            rotation: None,
            net_id: None,
        }
    }

    #[wasm_bindgen_test]
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

    #[wasm_bindgen_test]
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

    #[wasm_bindgen_test]
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

    #[wasm_bindgen_test]
    fn pad_layers_follow_pad_and_part_sides_and_through_holes_span_both_copper_faces() {
        assert_eq!(
            pad_copper_layers(&pad(Some(Side::Front), None), &Side::Front),
            ["F.Cu"]
        );
        assert_eq!(
            pad_copper_layers(&pad(Some(Side::Back), None), &Side::Front),
            ["B.Cu"]
        );
        assert_eq!(
            pad_copper_layers(&pad(Some(Side::Front), None), &Side::Back),
            ["B.Cu"]
        );
        assert_eq!(
            pad_copper_layers(&pad(Some(Side::Back), None), &Side::Back),
            ["F.Cu"]
        );
        assert_eq!(
            pad_copper_layers(&pad(None, Some(0.8)), &Side::Front),
            ["F.Cu", "B.Cu"]
        );
    }

    #[wasm_bindgen_test]
    fn through_hole_pad_remains_visible_until_both_copper_faces_or_pads_are_hidden() {
        let through_hole = pad(Some(Side::Front), Some(0.8));
        let mut hidden = BTreeSet::from(["F.Cu".to_owned()]);
        assert!(pad_is_visible(&through_hole, &Side::Front, &hidden));
        hidden.insert("B.Cu".into());
        assert!(!pad_is_visible(&through_hole, &Side::Front, &hidden));
        hidden.remove("F.Cu");
        hidden.remove("B.Cu");
        hidden.insert("Pads".into());
        assert!(!pad_is_visible(&through_hole, &Side::Front, &hidden));
    }

    #[wasm_bindgen_test]
    fn back_graphics_swap_only_front_and_back_layer_prefixes() {
        assert_eq!(resolve_board_layer("F.SilkS", &Side::Back), "B.SilkS");
        assert_eq!(resolve_board_layer("B.Cu", &Side::Back), "F.Cu");
        assert_eq!(resolve_board_layer("Dwgs.User", &Side::Back), "Dwgs.User");
    }
}

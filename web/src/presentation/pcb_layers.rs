//! Layer inventory and visibility grouping for accepted host PCB scene geometry.
use super::canvas_layers::{CanvasLayer, CanvasLayerGroup};
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

pub(super) fn use_layer_groups(
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
) -> Vec<CanvasLayerGroup> {
    let generated_sources = generator_sources(snapshot, scope);
    let generated_layers = use_resource(use_reactive(&generated_sources, |sources| async move {
        collect_generator_layers(sources).await
    }));
    let generated_layers = match &*generated_layers.read() {
        Some(Ok(layers)) => layers.clone(),
        Some(Err(_)) | None => BTreeSet::new(),
    };
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

async fn collect_generator_layers(
    sources: Vec<GeneratorLayerSource>,
) -> Result<BTreeSet<String>, String> {
    let mut layers = BTreeSet::new();
    for source in sources {
        if let Some(drawings) =
            generator_drawings(source.definition, source.parameters, Some(false)).await?
        {
            for graphic in drawings.iter() {
                layers.insert(resolve_board_layer(&graphic.layer, &source.side));
            }
        }
    }
    Ok(layers)
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

    let mut controls = layers
        .iter()
        .map(|layer| CanvasLayer::hidden(layer.clone(), layer.clone()))
        .collect::<Vec<_>>();
    if !contours.is_empty() {
        controls.push(CanvasLayer::hidden("Edge.Cuts", "Edge.Cuts"));
    }
    if has_courtyards {
        controls.push(CanvasLayer::hidden("Courtyards", "Courtyards"));
    }
    if has_pads {
        controls.push(CanvasLayer::hidden("Pads", "Pads"));
    }
    if has_holes {
        controls.push(CanvasLayer::hidden("Holes", "Holes"));
    }
    if has_references {
        controls.push(CanvasLayer::hidden("References", "References"));
    }
    controls.sort_by(|left, right| left.id.cmp(&right.id));
    vec![CanvasLayerGroup::ungrouped(controls)]
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
    use boardstudio_core::model::{PadShape, Vec2};
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

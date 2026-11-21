//! Layer inventory and visibility grouping for accepted host PCB scene geometry.
use super::canvas_layers::{CanvasLayer, CanvasLayerGroup, CanvasLayers};
use super::footprint_graphics::generator_drawings;
use crate::pcb_layer_inventory::{
    GeneratorLayerInventory, GeneratorLayerRequest, GeneratorLayerSource, current_generator_layers,
    extend_generator_layers,
};
use crate::pcb_layer_projection::pad_copper_layers;
pub use crate::pcb_layer_projection::pad_is_visible;
use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_model::model::{Part, ProjectDoc};
use dioxus::prelude::*;
use std::collections::{BTreeSet, HashSet};

#[derive(Props, Clone, PartialEq)]
pub struct PcbLayerControlsProps {
    #[props(default)]
    initially_open: bool,
    snapshot: AcceptedSnapshot,
    scope: Scope,
}

#[component]
pub fn PcbLayerControls(props: PcbLayerControlsProps) -> Element {
    let groups = use_layer_groups(&props.snapshot, &props.scope);
    rsx! {
        CanvasLayers {
            initially_open: props.initially_open,
            trigger_label: "PCB layers".to_owned(),
            trigger_id: String::from("m1-pcb-layers-trigger"),
            list_id: String::from("m1-pcb-layers-list"),
            groups,
        }
    }
}

pub fn use_layer_groups(snapshot: &AcceptedSnapshot, scope: &Scope) -> Vec<CanvasLayerGroup> {
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

pub fn layer_groups_for_scene(
    document: &ProjectDoc,
    board_contours: &[boardstudio_model::model::BoardContours],
    scene_contours: &[boardstudio_model::model::Contour],
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

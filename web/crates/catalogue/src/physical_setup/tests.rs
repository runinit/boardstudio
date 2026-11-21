use super::*;
use boardstudio_core_mechanical::mechanical::change::MechanicalFeatureCorrespondence;
use boardstudio_model::model::{
    Board, ElectricalBoardConfiguration, HardwareConfiguration, HardwareTopology,
    HardwareTransport, Layout, LayoutMirrorLink, MechanicalConfiguration, MechanicalMirrorPair,
    MechanicalOpening, MechanicalOpeningVertex, Mount, MountKind, PartDefinition,
    PhysicalBoardInstance, ProjectDoc,
};
use serde_json::{Value, json};

pub(super) fn document() -> ProjectDoc {
    let mut doc = ProjectDoc::empty("project", "Test project");
    doc.boards.push(Board {
        id: "board-a".into(),
        name: "Board A".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc
}

pub(super) fn mechanical(doc: &ProjectDoc, thickness: f64) -> MechanicalConfiguration {
    let mut value = boardstudio_web_host::case_settings::initial_settings(doc, "board-a").unwrap();
    value.plate_thickness = thickness;
    value
}

pub(super) fn instance(
    doc: &ProjectDoc,
    id: &str,
    half: &str,
    flipped: bool,
) -> PhysicalBoardInstance {
    PhysicalBoardInstance {
        id: id.into(),
        name: format!("{half} assembly"),
        board_id: "board-a".into(),
        half: half.into(),
        role: "custom-role".into(),
        flipped,
        controller_part_id: Some(format!("{id}-controller")),
        mechanical: Some(mechanical(doc, 4.2)),
        construction_ownership: MechanicalConstructionOwnership::Local,
        mechanical_identity_reservation: None,
    }
}

pub(super) fn keep_definition(
    definition: &PartDefinition,
    _enabled: bool,
) -> Result<(PartDefinition, bool), String> {
    Ok((definition.clone(), false))
}

pub(super) fn definition(id: &str, source: &str) -> PartDefinition {
    serde_json::from_value(json!({
        "id":id,"name":id,"kind":"switch","courtyard":[],"pads":[],
        "generator":{"source":source,"version":"1","parameters":{}}
    }))
    .unwrap()
}

pub(super) fn part(id: &str, definition_id: &str, params: Value) -> boardstudio_model::model::Part {
    serde_json::from_value(json!({
        "id":id,"definitionId":definition_id,"reference":id,
        "pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front",
        "generatorParameters":params
    }))
    .unwrap()
}

#[path = "mounted_tests.rs"]
mod mounted_tests;
#[path = "unit_tests.rs"]
mod unit_tests;

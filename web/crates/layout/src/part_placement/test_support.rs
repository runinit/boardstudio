//! Shared fixtures for the placement plan (native) and hook (browser) tests.

use boardstudio_core::model::{
    Board, OutlineFeature, OutlineSettings, Part, PartDefinition, Pose2, ProjectDoc, Side, Vec2,
};

pub(crate) fn controller_definition(id: &str) -> PartDefinition {
    serde_json::from_value(serde_json::json!({
        "id": id,
        "name": "Controller",
        "kind": "controller",
        "courtyard": [{"x": -5.0, "y": -5.0}, {"x": 5.0, "y": -5.0}, {"x": 5.0, "y": 5.0}],
        "pads": []
    }))
    .unwrap()
}

pub(crate) fn passive_definition(id: &str) -> PartDefinition {
    serde_json::from_value(serde_json::json!({
        "id": id,
        "name": "Reset switch",
        "kind": "passive",
        "courtyard": [{"x": -3.0, "y": -3.0}, {"x": 3.0, "y": -3.0}, {"x": 3.0, "y": 3.0}],
        "pads": []
    }))
    .unwrap()
}

pub(crate) fn switch_definition(id: &str) -> PartDefinition {
    serde_json::from_value(serde_json::json!({
        "id": id,
        "name": "MX switch",
        "kind": "switch",
        "courtyard": [{"x": -3.0, "y": -3.0}, {"x": 3.0, "y": -3.0}, {"x": 3.0, "y": 3.0}],
        "pads": []
    }))
    .unwrap()
}

pub(crate) fn fixture() -> ProjectDoc {
    let mut document = ProjectDoc::empty("project", "Keyboard");
    document.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        settings: OutlineSettings::default(),
        id: "envelope-main".into(),
        part_ids: vec![],
        margin: 4.0,
        operation: boardstudio_core::model::Operation::Add,
    });
    document.boards.push(Board {
        id: "board-main".into(),
        name: "Main".into(),
        outline_ids: vec!["envelope-main".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document
}

pub(crate) fn mirrored_fixture() -> ProjectDoc {
    let mut document = fixture();
    document.outline.clear();
    document.boards[0].outline_ids.clear();
    document
        .definitions
        .push(passive_definition("catalog:previous-part"));
    let previous_part = part("catalog:previous-part", "part-previous", "J0");
    document.boards[0].part_ids.push(previous_part.id.clone());
    document.parts.push(previous_part);
    document.boards.push(Board {
        id: "board-mirror".into(),
        name: "Mirror".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.hardware = Some(
        serde_json::from_value(serde_json::json!({
            "topology": "split",
            "transport": "wired",
            "instances": [
                {
                    "id": "instance-main", "name": "Main", "boardId": "board-main",
                    "half": "left", "role": "central", "flipped": false,
                    "controllerPartId": null, "mechanical": null
                },
                {
                    "id": "instance-mirror", "name": "Mirror", "boardId": "board-mirror",
                    "half": "right", "role": "central", "flipped": false,
                    "controllerPartId": null, "mechanical": null
                }
            ],
            "boards": [],
            "pairs": [{
                "id": "pair-main-mirror",
                "members": ["instance-main", "instance-mirror"],
                "constructionSource": "instance-main",
                "mapping": {"at": {"x": 200.0, "y": 0.0}, "rotation": 0.0},
                "construction": {
                    "method": "printed", "mount": "rigid",
                    "integratedPlateFrame": false, "plateThickness": 1.5,
                    "plateFoamThickness": 0.0, "bottomFoamThickness": 0.0,
                    "bottomThickness": 2.0, "plateToPcb": 3.5,
                    "wallThickness": 2.0, "clearance": 0.2
                },
                "halves": {
                    "sourceBoardId": "board-main",
                    "mirrorBoardId": "board-mirror",
                    "correspondences": [{"kind": "part", "sourceId": "part-previous", "mirrorId": "part-previous-mirror"}],
                    "exceptions": [{"kind": "part", "mirrorId": "part-previous-mirror", "state": "omitted"}]
                }
            }]
        }))
        .unwrap(),
    );
    document
}

pub(crate) fn matrix_fixture() -> boardstudio_core::model::Matrix {
    serde_json::from_value(serde_json::json!({
        "id": "matrix-main",
        "rows": 2,
        "columns": 3,
        "pitch": {"x": 19.05, "y": 19.05},
        "origin": {"x": 0.0, "y": 0.0},
        "definitionId": "switch:base",
        "partIds": [],
        "boardId": "board-main",
        "cells": []
    }))
    .unwrap()
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn accepted(
    document: ProjectDoc,
    token: u64,
) -> boardstudio_application::AcceptedSnapshot {
    use boardstudio_application::{AcceptedSnapshot, SessionEpoch, SnapshotToken};
    use std::sync::Arc;
    let revision = document.revision;
    AcceptedSnapshot {
        token: SnapshotToken(token),
        session_epoch: SessionEpoch(7),
        document: Arc::new(document),
        scene: Arc::new(boardstudio_core::model::SceneDelta {
            module_scenes: vec![],
            revision,
            transaction_id: "controller-placement-test".into(),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![],
            board_readiness: vec![],
            board_outline_scenes: vec![],
            finding_markers: vec![],
            findings: vec![],
            readiness: boardstudio_core::model::Readiness {
                layout: false,
                outline: false,
                pcb: false,
                case_ready: false,
            },
        }),
    }
}

pub(crate) fn part(definition_id: &str, id: &str, reference: &str) -> Part {
    Part {
        keycap: None,
        outline: None,
        anchor: None,
        id: id.into(),
        definition_id: definition_id.into(),
        reference: reference.into(),
        pose: Pose2 {
            at: Vec2 { x: 12.0, y: -8.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        properties: None,
        generator_parameters: None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn replacement(operation: boardstudio_core::model::EditOperation) -> ProjectDoc {
    use boardstudio_core::model::EditOperation;
    let EditOperation::ReplaceDocument { document } = operation else {
        panic!("placement must be one ReplaceDocument edit");
    };
    *document
}

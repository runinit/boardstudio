//! One-key scene fixtures for the browser canvas rendering tests.
use boardstudio_core::model::{Matrix, ProjectDoc, SceneDelta};
use serde_json::{Value, json};
use std::sync::Arc;

pub(in crate::presentation) fn switch_document(
    id: &str,
    title: &str,
    part_ids: &[&str],
) -> ProjectDoc {
    let mut document = ProjectDoc::empty(id, title);
    document.parts = part_ids
        .iter()
        .map(|id| {
            serde_json::from_value(json!({
                "id": id, "definitionId": "switch", "reference": "SW1",
                "pose": {"at": {"x": 0.0, "y": 0.0}, "rotation": 0.0}, "side": "front",
                "keycap": {"x": 18.0, "y": 18.0}
            }))
            .unwrap()
        })
        .collect();
    document.definitions.push(
        serde_json::from_value(json!({
            "id": "switch", "name": "Switch", "kind": "switch", "courtyard": [], "pads": []
        }))
        .unwrap(),
    );
    document.boards.push(
        serde_json::from_value(json!({
            "id": "board", "name": "Board", "partIds": part_ids,
            "outlineIds": [], "netIds": [], "thickness": 1.6
        }))
        .unwrap(),
    );
    document
}

pub(in crate::presentation) fn one_key_matrix(id: &str, part_id: &str, x: f64) -> Matrix {
    serde_json::from_value(json!({
        "id": id, "rows": 1, "columns": 1,
        "pitch": {"x": 19.05, "y": 19.05}, "origin": {"x": x, "y": 0.0},
        "definitionId": "switch", "partIds": [part_id], "boardId": "board",
        "cells": [{"row": 0, "column": 0, "enabled": true, "definitionId": "switch", "assemblies": []}]
    })).unwrap()
}

pub(in crate::presentation) fn one_key_scene(
    matrix_id: &str,
    x: f64,
    y: f64,
    columns: &[Value],
) -> Value {
    let frame = json!({
        "origin": {"x": 0.0, "y": 0.0}, "rotation": 0.0,
        "axisX": {"x": 1.0, "y": 0.0}, "axisY": {"x": 0.0, "y": 1.0}
    });
    json!({
        "matrixId": matrix_id,
        "cells": [{
            "row": 0, "column": 0, "enabled": true,
            "memberId": format!("matrix/{matrix_id}/r0c0"),
            "pose": {"at": {"x": x, "y": y}, "rotation": 0.0},
            "shiftFrame": frame, "offsetFrame": frame
        }],
        "columns": columns
    })
}

pub(in crate::presentation) fn column_at_origin() -> Value {
    json!({
        "column": 0, "base": {"x": 0.0, "y": 0.0}, "rotation": 0.0,
        "relativeOrigin": {"x": 0.0, "y": 0.0},
        "splayOrigin": {"x": 0.0, "y": 0.0}, "splayAngle": 0.0, "customOrigin": false,
        "axisX": {"x": 1.0, "y": 0.0}, "axisY": {"x": 0.0, "y": 1.0},
        "staggerAxis": {"x": 0.0, "y": 1.0}
    })
}

pub(in crate::presentation) fn scene(matrix_scenes: &[Value]) -> Arc<SceneDelta> {
    Arc::new(
        serde_json::from_value(json!({
            "revision": 0, "transactionId": "fixture", "changedIds": [], "transforms": [],
            "matrixScenes": matrix_scenes,
            "contours": [], "boardContours": [], "boardReadiness": [], "findings": [],
            "readiness": {"layout": true, "outline": false, "pcb": false, "case": false}
        }))
        .unwrap(),
    )
}

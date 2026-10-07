//! A small real-Core matrix document shared by the matrix edit tests: three keys in one
//! matrix, opened through the in-process Session and Core.
#![cfg(all(test, target_arch = "wasm32"))]
use crate::runtime::{Runtime, project_name_test_support as support};
use boardstudio_core::model::{
    Board, OutlineFeature, OutlineSettings, Part, Pose2, ProjectDoc, Side, Vec2,
};
use std::rc::Rc;

pub const MATRIX_ID: &str = "matrix";
pub const BOARD_ID: &str = "board-main";

pub fn matrix_document() -> ProjectDoc {
    let key_ids = [
        "matrix/matrix/r0c0",
        "matrix/matrix/r0c1",
        "matrix/matrix/r0c2",
    ];
    let mut document = ProjectDoc::empty("project", "Matrix edits");
    document.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        settings: OutlineSettings::default(),
        id: "envelope-main".into(),
        part_ids: vec![],
        margin: 4.0,
        operation: boardstudio_core::model::Operation::Add,
    });
    document.boards.push(Board {
        id: BOARD_ID.into(),
        name: "Main".into(),
        outline_ids: vec!["envelope-main".into()],
        part_ids: key_ids.iter().map(|id| (*id).into()).collect(),
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.definitions.push(
        serde_json::from_value(serde_json::json!({
            "id": "switch:base",
            "name": "MX switch",
            "kind": "switch",
            "courtyard": [{"x": -3.0, "y": -3.0}, {"x": 3.0, "y": -3.0}, {"x": 3.0, "y": 3.0}],
            "pads": []
        }))
        .unwrap(),
    );
    for (column, id) in key_ids.iter().enumerate() {
        document.parts.push(Part {
            keycap: None,
            outline: None,
            id: (*id).into(),
            definition_id: "switch:base".into(),
            reference: format!("SW{}", column + 1),
            pose: Pose2 {
                at: Vec2 {
                    x: 19.05 * column as f64,
                    y: 0.0,
                },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        });
    }
    document.matrices.push(
        serde_json::from_value(serde_json::json!({
            "id": MATRIX_ID,
            "rows": 1,
            "columns": 3,
            "pitch": {"x": 19.05, "y": 19.05},
            "origin": {"x": 0.0, "y": 0.0},
            "definitionId": "switch:base",
            "partIds": key_ids,
            "boardId": BOARD_ID,
            "cells": (0..3).map(|column| serde_json::json!({
                "row": 0, "column": column, "enabled": true,
                "definitionId": "switch:base", "assemblies": []
            })).collect::<Vec<_>>()
        }))
        .unwrap(),
    );
    document
}

pub async fn open_matrix_runtime() -> Rc<Runtime> {
    let runtime = support::new_runtime();
    support::open_document(&runtime, matrix_document()).await;
    runtime
}

pub fn accepted_matrix(runtime: &Runtime) -> boardstudio_core::model::Matrix {
    runtime
        .model()
        .accepted
        .expect("an accepted document")
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == MATRIX_ID)
        .cloned()
        .expect("the matrix exists")
}

/// Drive the in-process runtime until `ticket` settles, polling the parked reply task.
pub async fn settle_ticket(
    runtime: &Rc<Runtime>,
    ticket: &boardstudio_web_runtime::edit_ticket::EditTicket,
) {
    for _ in 0..100 {
        support::run_pending(runtime).await;
        if !ticket.is_pending() {
            return;
        }
        gloo_timers::future::TimeoutFuture::new(10).await;
    }
}

pub async fn undo(runtime: &Rc<Runtime>) {
    runtime.submit(boardstudio_application::Event::Undo {
        operation_id: runtime.operation(),
    });
    support::run_pending(runtime).await;
}

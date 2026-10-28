//! A small real-Core matrix document shared by the matrix edit tests: three keys in one
//! matrix, opened through the in-process Session and Core.
#![cfg(all(test, target_arch = "wasm32"))]
use crate::runtime::{Runtime, project_name_test_support as support};
use boardstudio_core::model::{
    Board, Follow, Layout, LayoutMirrorLink, MirroredCorrespondence, MirroredItemKind,
    OutlineFeature, OutlineSettings, Part, PartAnchor, Pose2, ProjectDoc, Side, Vec2,
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
            anchor: None,
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

/// Ticket 07: the matrix document plus a layout homing the matrix and one
/// anchored independent part (the MCU), following the split-board-editing
/// column vocabulary: local Offset/Rotation plus the Spread and Stagger
/// carried from anchor key r0c1.
pub fn anchored_part(step: (f64, f64), anchored: bool, locked: bool) -> Part {
    Part {
        keycap: None,
        outline: None,
        id: "mcu".into(),
        definition_id: "switch:base".into(),
        reference: "U1".into(),
        pose: Pose2 {
            at: Vec2 { x: 60.0, y: 40.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        anchor: anchored.then(|| PartAnchor {
            matrix_id: MATRIX_ID.into(),
            row: 0,
            column: 1,
            follow: Follow {
                x: true,
                y: true,
                rotation: true,
            },
            spread: step.0,
            stagger: step.1,
            splay: 0.0,
            offset: Vec2::default(),
            rotation: 0.0,
        }),
        locked: locked.then_some(true),
        properties: None,
        generator_parameters: None,
    }
}

/// Ticket 07: one layout homing the matrix and the independent parts, so
/// part anchor edits resolve.
pub fn part_document(step: (f64, f64)) -> ProjectDoc {
    let mut document = matrix_document();
    document.boards[0].part_ids.push("mcu".into());
    document.layouts.push(Layout {
        id: "layout".into(),
        name: "Layout".into(),
        board_id: BOARD_ID.into(),
        matrix_ids: vec![MATRIX_ID.into()],
        part_ids: vec!["mcu".into()],
        mirror_link: None,
    });
    document.parts.push(anchored_part(step, true, false));
    document
}

pub async fn open_part_runtime(step: (f64, f64)) -> Rc<Runtime> {
    let runtime = support::new_runtime();
    support::open_document(&runtime, part_document(step)).await;
    runtime
}

/// Ticket 07: the single-layout part document with a detached (and unlocked)
/// MCU, for the detached rail.
pub fn detached_part_document() -> ProjectDoc {
    let mut document = part_document((0.0, 0.0));
    let part = document
        .parts
        .iter_mut()
        .find(|part| part.id == "mcu")
        .expect("the mcu part");
    part.anchor = None;
    document
}

pub async fn open_detached_part_runtime() -> Rc<Runtime> {
    let runtime = support::new_runtime();
    support::open_document(&runtime, detached_part_document()).await;
    runtime
}

/// Ticket 07: a mirrored pair whose halves each carry an anchored independent
/// part, with a Part correspondence so a rail drag on one half fans out to
/// the other per ADR-0038.
pub fn part_pair_document() -> ProjectDoc {
    let mut document = matrix_document();
    document.boards[0].part_ids.push("mcu".into());
    document.boards[0].part_ids.push("mcu-mirror".into());
    document.matrices.push(
        serde_json::from_value(serde_json::json!({
            "id": "matrix-mirror",
            "rows": 1,
            "columns": 3,
            "pitch": {"x": 19.05, "y": 19.05},
            "origin": {"x": 0.0, "y": 0.0},
            "definitionId": "switch:base",
            "partIds": ["matrix/matrix-mirror/r0c0", "matrix/matrix-mirror/r0c1", "matrix/matrix-mirror/r0c2"],
            "mirror": "x",
            "boardId": BOARD_ID,
            "cells": (0..3).map(|column| serde_json::json!({
                "row": 0, "column": column, "enabled": true,
                "definitionId": "switch:base", "assemblies": []
            })).collect::<Vec<_>>()
        }))
        .unwrap(),
    );
    for (index, column) in [0, 1, 2].iter().enumerate() {
        document.parts.push(Part {
            keycap: None,
            outline: None,
            anchor: None,
            id: format!("matrix/matrix-mirror/r0c{column}"),
            definition_id: "switch:base".into(),
            reference: format!("SW{}M", index + 1),
            pose: Pose2 {
                at: Vec2 {
                    x: 19.05 * *column as f64,
                    y: 0.0,
                },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        });
        document.boards[0]
            .part_ids
            .push(format!("matrix/matrix-mirror/r0c{column}"));
    }
    document.layouts.push(Layout {
        id: "left".into(),
        name: "Left".into(),
        board_id: BOARD_ID.into(),
        matrix_ids: vec![MATRIX_ID.into()],
        part_ids: vec!["mcu".into()],
        mirror_link: None,
    });
    document.layouts.push(Layout {
        id: "right".into(),
        name: "Right".into(),
        board_id: BOARD_ID.into(),
        matrix_ids: vec!["matrix-mirror".into()],
        part_ids: vec!["mcu-mirror".into()],
        mirror_link: Some(LayoutMirrorLink {
            source_id: "left".into(),
            axis_x: 40.0,
            split_gap: Some(80.0),
            spacing: None,
            correspondences: vec![
                MirroredCorrespondence {
                    kind: MirroredItemKind::Matrix,
                    source_id: MATRIX_ID.into(),
                    mirror_id: "matrix-mirror".into(),
                    companion_mode: None,
                    companion_tweak: None,
                },
                MirroredCorrespondence {
                    kind: MirroredItemKind::Part,
                    source_id: "mcu".into(),
                    mirror_id: "mcu-mirror".into(),
                    companion_mode: None,
                    companion_tweak: None,
                },
            ],
            exceptions: vec![],
        }),
    });
    let mut mcu = anchored_part((19.0, 0.0), true, false);
    document.parts.push(mcu.clone());
    mcu.id = "mcu-mirror".into();
    mcu.reference = "U1M".into();
    if let Some(anchor) = mcu.anchor.as_mut() {
        anchor.matrix_id = "matrix-mirror".into();
        anchor.spread = -19.0;
    }
    document.parts.push(mcu);
    document
}

pub async fn open_part_pair_runtime() -> Rc<Runtime> {
    let runtime = support::new_runtime();
    support::open_document(&runtime, part_pair_document()).await;
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

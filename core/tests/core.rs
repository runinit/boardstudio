use crate::fixtures::matrix_doc;
use boardstudio_core::CoreEngine;
use boardstudio_core::model::*;

#[path = "core/core_chunk0.rs"]
mod core_chunk0;
#[path = "core/core_chunk1.rs"]
mod core_chunk1;
#[path = "core/core_chunk2.rs"]
mod core_chunk2;
#[path = "core/core_chunk3.rs"]
mod core_chunk3;
fn rect(id: &str, x: f64, y: f64, width: f64, height: f64, operation: Operation) -> OutlineFeature {
    OutlineFeature::Rect {
        rotation: None,
        anchor_part_id: None,
        id: id.into(),
        center: Vec2 { x, y },
        size: Vec2 {
            x: width,
            y: height,
        },
        radius: 0.0,
        operation,
    }
}
fn scene(reply: CoreReply) -> (SceneDelta, ProjectDoc) {
    match reply {
        CoreReply::Scene {
            scene, document, ..
        } => (scene, *document),
        other => panic!("expected scene: {other:?}"),
    }
}
fn preview(reply: CoreReply) -> SceneDelta {
    match reply {
        CoreReply::Preview { scene, .. } => scene,
        other => panic!("expected preview: {other:?}"),
    }
}
fn edit(base: u64, phase: EditPhase, op: EditOperation) -> CoreRequest {
    CoreRequest::Edit {
        id: "request".into(),
        command: EditCommand {
            base_revision: base,
            transaction_id: "drag-1".into(),
            phase,
            target_ids: vec![],
            operation: op,
        },
    }
}

fn mirrored_pair_request(base: u64, phase: &str) -> String {
    serde_json::json!({
        "kind": "edit", "id": "pair",
        "command": {"baseRevision": base, "transactionId": "pair", "phase": phase, "targetIds": [],
            "operation": {"kind": "create-mirrored-pair",
                "matrix": {"id": "left-matrix", "rows": 2, "columns": 3, "pitch": {"x": 19, "y": 19},
                    "origin": {"x": -20, "y": 5}, "mirror": "x", "definitionId": "switch", "boardId": "board", "partIds": []},
                "left": {"id": "left-layout", "name": "Left half", "boardId": "board", "matrixIds": ["left-matrix"], "partIds": []},
                "right": {"id": "right-layout", "name": "Right half", "boardId": "board", "matrixIds": ["right-matrix"], "partIds": [],
                    "mirrorLink": {"sourceId": "left-layout", "axisX": 10}}
            }
        }
    }).to_string()
}

fn linked_pair() -> (CoreEngine, ProjectDoc) {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let response = engine.request(&mirrored_pair_request(0, "commit"));
    let (_, doc) = scene(serde_json::from_str(&response).unwrap());
    (engine, doc)
}

fn matrix(rows: u32, columns: u32) -> Matrix {
    Matrix {
        id: "main".into(),
        name: None,
        rows,
        columns,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2 { x: 0.0, y: 0.0 },
        definition_id: "switch".into(),
        part_ids: vec![],
        board_id: None,
        mirror: None,
        rotation: None,
        edge_gap: None,

        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: vec![],
    }
}

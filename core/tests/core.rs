use crate::support::fixture::{self, matrix_document as matrix_doc, scene_and_document as scene};
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
fn preview(reply: CoreReply) -> SceneDelta {
    match reply {
        CoreReply::Preview { scene, .. } => scene,
        other => panic!("expected preview: {other:?}"),
    }
}
fn edit(base: u64, phase: EditPhase, op: EditOperation) -> CoreRequest {
    fixture::edit("request", "drag-1", base, phase, op)
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
    engine.handle(fixture::open(matrix_doc()));
    let response = engine.request(&mirrored_pair_request(0, "commit"));
    let (_, doc) = scene(serde_json::from_str(&response).unwrap());
    (engine, doc)
}

fn matrix(rows: u32, columns: u32) -> Matrix {
    fixture::matrix("main", rows, columns)
}

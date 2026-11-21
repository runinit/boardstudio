#[path = "module_import/board_source.rs"]
mod board_source;
#[path = "module_import/case_preview.rs"]
mod case_preview;
#[path = "module_import/circuit_identity.rs"]
mod circuit_identity;
#[path = "module_import/imported_geometry.rs"]
mod imported_geometry;

use boardstudio_core::CoreEngine;
use serde_json::{Value, json};

const FRONT_SLOT: &str = include_str!("fixtures/pcb_front_slot.kicad_pcb");
const VIK_SPLITTER: &str = include_str!(
    "../../catalogue/modules/sources/sadekbaroudi-vik/pcb/vik-splitter/vik-splitter.kicad_pcb"
);

fn artifact(request: Value) -> Value {
    serde_json::from_str(&boardstudio_core::artifact::request(&request.to_string())).unwrap()
}

fn import(source: String, id: &str) -> Value {
    artifact(json!({
        "id":"import","kind":"import-module-board","definitionId":id,
        "name":"Fixture module","source":source,"family":"fixture","variant":"source",
        "provenance":{"repository":"https://example.invalid/repo","revision":"abc","path":"board.kicad_pcb","license":"MIT"}
    }))
}

fn source_board_with(extra: &str) -> String {
    let source = FRONT_SLOT.trim_end();
    format!("{}\n{extra}\n)", source.strip_suffix(')').unwrap())
}

fn resolve_source_modules(definitions: Vec<Value>) -> Value {
    let mut document = serde_json::to_value(boardstudio_core::model::ProjectDoc::empty(
        "source-preview",
        "Source preview",
    ))
    .unwrap();
    document["boards"] = json!([{"id":"host","name":"Host","partIds":[],"netIds":[],"outlineIds":["outline"],"thickness":1.6}]);
    document["outline"] = json!([{"kind":"rect","id":"outline","center":{"x":0,"y":0},"size":{"x":200,"y":200},"radius":0,"operation":"add"}]);
    document["modules"] = definitions.iter().enumerate().map(|(index, definition)| json!({
        "id":format!("mounted-{index}"),"definitionId":definition["id"],"hostBoardId":"host",
        "hostFace":"front","facingFace":"back","at":{"x":index*60,"y":0},"rotation":0,
        "gap":3,"attachment":"board","detached":false,"serviceClearance":0
    })).collect();
    document["moduleDefinitions"] = definitions.into();
    serde_json::from_str(
        &CoreEngine::new().request(
            &json!({
                "id":"resolve","kind":"resolve-modules","document":document,"boardId":"host"
            })
            .to_string(),
        ),
    )
    .unwrap()
}

fn read_source(source: &str) -> Value {
    read_placed_source(source, (0, 0), 0, false)
}

fn read_placed_source(source: &str, (x, y): (i32, i32), rotation: i32, flipped: bool) -> Value {
    artifact(
        json!({"id":"read", "kind":"read-board-source", "source":source,
        "placement":{"at":{"x":x,"y":y},"rotation":rotation}, "flipped":flipped}),
    )
}

fn preview(source: &str, revision: u64) -> Value {
    artifact(json!({"id":"preview", "kind":"preview-board", "source":source, "revision":revision}))
}

fn error_of(reply: &Value) -> (String, String) {
    assert_eq!(reply["kind"], "error", "{reply}");
    (
        reply["error"]["code"].as_str().unwrap().to_owned(),
        reply["error"]["message"].as_str().unwrap().to_owned(),
    )
}

//! Ticket 03: the public Core boundary refuses over-budget constraint traversal
//! on Open and edit without changing accepted state.
use boardstudio_core::CoreEngine;
use serde_json::{Value, json};

/// Mirrors Core's private budget so this public-boundary test pins the documented value.
const MAX_DEPENDENCY_DEPTH: usize = 256;

fn request(engine: &mut CoreEngine, input: Value) -> Value {
    serde_json::from_str(&engine.request(&input.to_string())).unwrap()
}

/// A linear chain whose lexicographically first target depends on every later
/// part: `p00000 -> p00001 -> ...`. `BTreeMap` target iteration starts at
/// `p00000`, so memoization cannot resolve predecessors first and hide the depth.
fn chain_document(count: usize, with_board: bool) -> Value {
    let mut document = serde_json::to_value(boardstudio_core::model::ProjectDoc::empty(
        "constraint-depth",
        "Constraint depth",
    ))
    .unwrap();
    document["definitions"] = json!([{
        "id":"def","name":"Switch","kind":"switch","courtyard":[],"pads":[]
    }]);
    let parts: Vec<Value> = (0..count)
        .map(|index| {
            json!({
                "id": format!("p{index:05}"),
                "definitionId": "def",
                "reference": format!("SW{index}"),
                "pose": {"at": {"x": index as f64 * 10.0, "y": 0.0}, "rotation": 0.0},
                "side": "front"
            })
        })
        .collect();
    document["parts"] = json!(parts);
    let constraints: Vec<Value> = (0..count.saturating_sub(1))
        .map(|index| {
            json!({
                "kind": "offset",
                "id": format!("c{index:05}"),
                "sourcePartId": format!("p{:05}", index + 1),
                "targetPartId": format!("p{index:05}"),
                "offset": {"x": 10.0, "y": 0.0},
                "rotation": 0.0
            })
        })
        .collect();
    document["constraints"] = json!(constraints);
    if with_board {
        document["boards"] = json!([{
            "id":"board","name":"Board","outlineIds":[],"partIds":[],"netIds":[],
            "thickness":1.6
        }]);
    }
    document
}

fn open(engine: &mut CoreEngine, id: &str, document: &Value) -> Value {
    request(engine, json!({"kind":"open","id":id,"document":document}))
}

fn snapshot(engine: &mut CoreEngine) -> Value {
    request(engine, json!({"kind":"snapshot","id":"snapshot"}))["document"].clone()
}

#[test]
fn deeply_acyclic_open_is_refused_without_overflowing_the_native_stack() {
    let mut engine = CoreEngine::new();
    let reply = open(&mut engine, "deep", &chain_document(12_000, false));
    assert_eq!(reply["kind"], "error");
    assert!(
        reply["message"].as_str().unwrap().contains("depth exceeds"),
        "{}",
        reply["message"]
    );
    assert_eq!(snapshot(&mut engine)["revision"], 0);
}

#[test]
fn at_budget_open_succeeds_and_an_edit_above_budget_is_refused_atomically() {
    let mut document = chain_document(MAX_DEPENDENCY_DEPTH, true);
    document["parts"].as_array_mut().unwrap().push(json!({
        "id":"extra","definitionId":"def","reference":"SW-extra",
        "pose":{"at":{"x":-10.0,"y":0.0},"rotation":0.0},"side":"front"
    }));
    let mut engine = CoreEngine::new();
    assert_eq!(open(&mut engine, "open", &document)["kind"], "scene");

    let committed = request(
        &mut engine,
        json!({
            "kind":"edit","id":"commit","command":{
                "baseRevision":0,"transactionId":"commit","phase":"commit","targetIds":["board"],
                "operation":{"kind":"set-wiring-mode","boardId":"board","mode":"direct"}
            }
        }),
    );
    assert_eq!(committed["scene"]["revision"], 1);
    let accepted = snapshot(&mut engine);

    let refused = request(
        &mut engine,
        json!({
            "kind":"edit","id":"deep-edit","command":{
                "baseRevision":1,"transactionId":"deep-edit","phase":"commit","targetIds":["extra"],
                "operation":{"kind":"set-constraint","constraint":{
                    "kind":"offset","id":"deep","sourcePartId":"p00000","targetPartId":"extra",
                    "offset":{"x":10,"y":0},"rotation":0
                }}
            }
        }),
    );
    assert_eq!(refused["kind"], "error");
    assert!(
        refused["message"]
            .as_str()
            .unwrap()
            .contains("depth exceeds"),
        "{}",
        refused["message"]
    );
    assert_eq!(snapshot(&mut engine), accepted);

    let undone = request(&mut engine, json!({"kind":"undo","id":"undo"}));
    assert_eq!(undone["kind"], "scene");
    assert_eq!(undone["scene"]["revision"], 2);
    assert!(undone["document"].get("hardware").is_none());
}

#[test]
fn refused_deep_open_preserves_the_accepted_document_and_history() {
    let mut engine = CoreEngine::new();
    let mut accepted_document = chain_document(MAX_DEPENDENCY_DEPTH, true);
    accepted_document["revision"] = json!(4);
    assert_eq!(
        open(&mut engine, "open", &accepted_document)["kind"],
        "scene"
    );
    assert_eq!(
        request(
            &mut engine,
            json!({
                "kind":"edit","id":"commit","command":{
                    "baseRevision":4,"transactionId":"commit","phase":"commit","targetIds":["board"],
                    "operation":{"kind":"set-wiring-mode","boardId":"board","mode":"direct"}
                }
            })
        )["scene"]["revision"],
        5
    );
    let accepted = snapshot(&mut engine);

    let mut deep = chain_document(MAX_DEPENDENCY_DEPTH + 1, true);
    deep["revision"] = json!(accepted["revision"]);
    let refused = open(&mut engine, "deep", &deep);
    assert_eq!(refused["kind"], "error");
    assert!(refused["message"].as_str().unwrap().contains("depth"));
    assert_eq!(snapshot(&mut engine), accepted);

    let undone = request(&mut engine, json!({"kind":"undo","id":"undo"}));
    assert_eq!(undone["kind"], "scene");
    assert_eq!(undone["scene"]["revision"], 6);
    assert!(undone["document"].get("hardware").is_none());
}

#[test]
fn cycle_detection_keeps_precedence_over_the_depth_budget() {
    let count = MAX_DEPENDENCY_DEPTH;
    let mut document = chain_document(count, false);
    let constraints = document["constraints"].as_array_mut().unwrap();
    constraints.push(json!({
        "kind":"offset","id":"cycle","sourcePartId":format!("p{:05}", 0),
        "targetPartId":format!("p{:05}", count - 1),
        "offset":{"x":10,"y":0},"rotation":0
    }));
    let mut engine = CoreEngine::new();
    let reply = open(&mut engine, "cycle", &document);
    assert_eq!(reply["kind"], "error");
    assert!(
        reply["message"].as_str().unwrap().contains("cycle"),
        "{}",
        reply["message"]
    );
}

//! Ticket 28: aggregate matrix anchors (average / intersection) through the
//! public Core routes. Aggregate anchors resolve N single-key parts to one
//! pose before the matrix Origin/Rotation and Follow apply, exactly as
//! upstream Ergogen resolves `aggregate` before `affect`.
use boardstudio_core::CoreEngine;
use serde_json::{Value, json};

fn request(engine: &mut CoreEngine, input: Value) -> Value {
    serde_json::from_str(&engine.request(&input.to_string())).unwrap()
}

fn open(engine: &mut CoreEngine, id: &str, document: &Value) -> Value {
    request(engine, json!({"kind":"open","id":id,"document":document}))
}

fn document() -> Value {
    let mut document = serde_json::to_value(boardstudio_core::model::ProjectDoc::empty(
        "aggregate-anchors",
        "Aggregate anchors",
    ))
    .unwrap();
    document["definitions"] = json!([{
        "id": "def", "name": "Switch", "kind": "switch",
        "courtyard": [], "pads": []
    }]);
    document
}

fn board(document: &mut Value, part_ids: Value) {
    document["boards"] = json!([{
        "id": "board", "name": "Board", "outlineIds": [],
        "partIds": part_ids, "netIds": [], "thickness": 1.6
    }]);
    document["nets"] = json!([]);
    document["constraints"] = json!([]);
    document["layouts"] = json!([{
        "id": "layout", "name": "Layout", "boardId": "board",
        "matrixIds": ["main", "thumb"], "partIds": []
    }]);
}

fn main_matrix(document: &mut Value, cells: Value) {
    document["parts"] = json!([
        {"id":"matrix/main/r0c0","definitionId":"def","reference":"SW1",
         "pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front"},
        {"id":"matrix/main/r0c1","definitionId":"def","reference":"SW2",
         "pose":{"at":{"x":19,"y":0},"rotation":0},"side":"front"},
        {"id":"matrix/main/r0c2","definitionId":"def","reference":"SW3",
         "pose":{"at":{"x":38,"y":0},"rotation":0},"side":"front"}
    ]);
    document["matrices"] = json!([{
        "id": "main", "rows": 1, "columns": 3,
        "pitch": {"x": 19, "y": 19}, "origin": {"x": 0, "y": 0},
        "definitionId": "def",
        "partIds": ["matrix/main/r0c0", "matrix/main/r0c1", "matrix/main/r0c2"],
        "boardId": "board", "cells": cells
    }]);
}

fn thumb(document: &mut Value, aggregate: Value) {
    document["matrices"].as_array_mut().unwrap().push(json!({
        "id": "thumb", "rows": 1, "columns": 3,
        "pitch": {"x": 19, "y": 19}, "origin": {"x": 0, "y": -19},
        "definitionId": "def", "partIds": [], "boardId": "board",
        "anchor": {
            "matrixId": "main", "row": 0, "column": 0,
            "follow": ["x", "y", "rotation"],
            "aggregate": aggregate
        }
    }));
}

fn part_at(reply: &Value, id: &str) -> (f64, f64, f64) {
    let part = reply["document"]["parts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|part| part["id"] == id)
        .unwrap_or_else(|| panic!("missing part {id} in {}", reply["document"]["parts"]));
    (
        part["pose"]["at"]["x"].as_f64().unwrap(),
        part["pose"]["at"]["y"].as_f64().unwrap(),
        part["pose"]["rotation"].as_f64().unwrap(),
    )
}

fn assert_pose(reply: &Value, id: &str, x: f64, y: f64, rotation: f64) {
    let (ax, ay, ar) = part_at(reply, id);
    assert!(
        (ax - x).abs() < 1e-9 && (ay - y).abs() < 1e-9 && (ar - rotation).abs() < 1e-9,
        "part {id}: expected ({x}, {y}, {rotation}), got ({ax}, {ay}, {ar})"
    );
}

#[test]
fn aggregate_average_anchor_opens_with_the_part_mean_pose() {
    let mut document = document();
    main_matrix(&mut document, json!([]));
    thumb(
        &mut document,
        json!({
            "method": "average",
            "parts": [
                {"matrixId": "main", "row": 0, "column": 0},
                {"matrixId": "main", "row": 0, "column": 2}
            ]
        }),
    );
    board(
        &mut document,
        json!(["matrix/main/r0c0", "matrix/main/r0c1", "matrix/main/r0c2"]),
    );
    let mut engine = CoreEngine::new();
    let reply = open(&mut engine, "average", &document);
    assert_eq!(reply["kind"], "scene", "{reply}");
    // Mean of (0,0) and (38,0) is (19,0); the relative origin hangs (0,-19)
    // below it, so the first thumb key lands on (19,-19) with rotation 0.
    assert_pose(&reply, "matrix/thumb/r0c0", 19.0, -19.0, 0.0);
    assert_pose(&reply, "matrix/thumb/r0c2", 57.0, -19.0, 0.0);
}

#[test]
fn aggregate_intersect_anchor_opens_with_the_heading_crossing_pose() {
    let mut document = document();
    // Second key carries key rotation -90 and an offset to (10,10): its
    // heading axis runs horizontal through y=10 while the first key's axis
    // runs vertical through x=0, crossing at (0,10) like upstream's
    // `o`/`ten` intersect vector.
    main_matrix(
        &mut document,
        json!([
            {"row": 0, "column": 1, "enabled": true,
             "offset": {"x": -9.0, "y": 10.0}, "rotation": -90.0}
        ]),
    );
    thumb(
        &mut document,
        json!({
            "method": "intersect",
            "parts": [
                {"matrixId": "main", "row": 0, "column": 0},
                {"matrixId": "main", "row": 0, "column": 1}
            ]
        }),
    );
    board(
        &mut document,
        json!(["matrix/main/r0c0", "matrix/main/r0c1", "matrix/main/r0c2"]),
    );
    let mut engine = CoreEngine::new();
    let reply = open(&mut engine, "intersect", &document);
    assert_eq!(reply["kind"], "scene", "{reply}");
    // The heading axes cross at (0,10) with the rotation forced to 0; the
    // relative origin hangs (0,-19) below it.
    assert_pose(&reply, "matrix/thumb/r0c0", 0.0, -9.0, 0.0);
}

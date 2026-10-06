//! Requests, semantic assertions and golden comparison for the native/WASM
//! boundary parity check. The native test (`core/tests/boundary_parity.rs`)
//! records and verifies the golden transcript; the browser test
//! (`web/tests/boundary_parity.rs`) replays the same requests through the real
//! `CoreEngine` and `archive_request` WASM exports and must reproduce it.
//! Ported from `scripts/check-rust-boundaries.mjs`.
#![allow(dead_code)]

use boardstudio_core::model::ProjectDoc;
use serde_json::{Value, json};

/// SHA-256 of [`ASSET`]; the native test recomputes it to guard the constant.
pub const ASSET: &[u8] = b"STEP fixture bytes\n";
pub const ASSET_SHA256: &str = "900b068e090cf03c8263fe9f2d1e3aa9c5a7d91ecf8530e60a8257c0aa03a048";
/// Incompressible 4 MiB payload from a fixed linear congruential generator.
/// SHA-256 of [`payload`]; the native test recomputes it to guard the constant.
pub const PAYLOAD_SHA256: &str = "decef75c559cfb399caa8eef4120d22d8320bde71d2cb78820248367dbb6b7de";
pub const PAYLOAD_LEN: usize = 4 * 1024 * 1024;

pub fn payload() -> Vec<u8> {
    let mut seed: u32 = 12345;
    (0..PAYLOAD_LEN)
        .map(|_| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed >> 24) as u8
        })
        .collect()
}

pub fn matrix() -> Value {
    json!({
        "id": "matrix", "rows": 3, "columns": 3, "definitionId": "switch",
        "origin": { "x": 7, "y": -3 }, "pitch": { "x": 19, "y": 19 },
        "partIds": ["matrix/matrix/r0c0", "matrix/matrix/r0c1", "matrix/matrix/r0c2", "matrix/matrix/r2c0", "matrix/matrix/r2c1", "matrix/matrix/r2c2"],
        "cells": (0..3).map(|column| json!({ "row": 1, "column": column, "enabled": false })).collect::<Vec<_>>(),
        "columnStaggers": [0, 2, -1],
        "columnOrigins": [null, { "x": 4, "y": -20 }],
        "columnSplays": [0, 15, -8],
    })
}

pub fn document() -> Value {
    let matrix = matrix();
    let mut doc = serde_json::to_value(ProjectDoc::empty("boundary-fixture", "Boundary fixture")).unwrap();
    doc["definitions"] = json!([{ "id": "switch", "name": "Switch", "kind": "switch", "pads": [], "courtyard": [] }]);
    doc["matrices"] = json!([matrix]);
    let part_ids = matrix["partIds"].as_array().unwrap().clone();
    doc["parts"] = Value::Array(
        part_ids.iter().enumerate().map(|(index, id)| json!({
            "id": id, "definitionId": "switch", "reference": format!("S{}", index + 1), "side": "front",
            "pose": { "at": { "x": (index % 3) * 19, "y": if index < 3 { 0 } else { -38 } }, "rotation": index * 3 },
        })).collect(),
    );
    doc["boards"] = json!([{ "id": "board", "name": "Boundary board", "partIds": part_ids, "outlineIds": [], "netIds": [], "thickness": 1.6 }]);
    doc
}

fn move_command(phase: &str) -> Value {
    json!({
        "baseRevision": 0, "transactionId": "move", "phase": phase, "targetIds": ["matrix/matrix/r0c0"],
        "operation": { "kind": "move-parts", "positions": [{ "id": "matrix/matrix/r0c0", "at": { "x": 10, "y": 12 } }] },
    })
}

pub fn core_requests() -> Vec<Value> {
    let matrix = matrix();
    let with = |overrides: Value| {
        let mut copy = matrix.clone();
        for (key, value) in overrides.as_object().unwrap() {
            copy[key] = value.clone();
        }
        copy
    };
    let mut drafts = Vec::new();
    for mirror in ["none", "x", "y"] {
        for (index, origin) in [json!({ "x": 0, "y": 0 }), json!({ "x": 42, "y": -17 })].into_iter().enumerate() {
            drafts.push(with(json!({ "id": format!("draft-{mirror}-{index}"), "origin": origin, "mirror": mirror, "rotation": 37 })));
        }
    }
    let mut requests = vec![
        json!({ "id": "open", "kind": "open", "document": document() }),
        json!({ "id": "snapshot", "kind": "snapshot" }),
        json!({ "id": "draft", "kind": "project-matrices", "baseRevision": 0, "matrices": drafts }),
        json!({ "id": "invalid-draft", "kind": "project-matrices", "baseRevision": 0, "matrices": [with(json!({ "rows": 0 }))] }),
        json!({ "id": "preview", "kind": "edit", "command": move_command("preview") }),
        json!({ "id": "unchanged", "kind": "snapshot" }),
        json!({ "id": "commit", "kind": "edit", "command": move_command("commit") }),
        json!({ "id": "stale-draft", "kind": "project-matrices", "baseRevision": 0, "matrices": [matrix.clone()] }),
        json!({ "id": "undo", "kind": "undo" }),
        json!({ "id": "redo", "kind": "redo" }),
        json!({ "id": "large-draft", "kind": "project-matrices", "baseRevision": 3, "matrices": [with(json!({ "rows": 50, "columns": 10, "cells": [], "partIds": [] }))] }),
    ];
    for phase in ["preview", "commit"] {
        requests.push(json!({
            "id": format!("splay-{phase}"), "kind": "edit",
            "command": { "baseRevision": 3, "transactionId": "splay", "phase": phase, "targetIds": ["matrix"],
                "operation": { "kind": "set-matrix-splay", "matrixId": "matrix", "column": 1, "change": { "kind": "origin", "world": { "x": 5, "y": -12 } } } },
        }));
    }
    for (index, operation) in [
        json!({ "kind": "set-key-binding", "boardId": "board", "keyId": "matrix/matrix/r0c0", "binding": "&kp A" }),
        json!({ "kind": "set-matrix-keycaps", "matrixId": "matrix", "change": { "kind": "profile", "value": "dsa" } }),
        json!({ "kind": "set-keycap-board", "boardId": "board", "change": { "kind": "color", "value": "#123456" } }),
        json!({ "kind": "set-keycap-key", "keyId": "matrix/matrix/r0c0", "change": { "kind": "legend", "value": "" } }),
    ].into_iter().enumerate() {
        requests.push(json!({
            "id": format!("keymap-{index}"), "kind": "edit",
            "command": { "baseRevision": 4 + index, "transactionId": format!("keymap-{index}"), "phase": "commit", "targetIds": [], "operation": operation },
        }));
    }
    requests
}

/// Numbers match to a relative 1e-9; everything else exactly.
pub fn equivalent(actual: &Value, expected: &Value, label: &str) {
    match expected {
        Value::Number(want) => {
            let (got, want) = (actual.as_f64().unwrap_or_else(|| panic!("{label}: not a number: {actual}")), want.as_f64().unwrap());
            assert!((got - want).abs() <= 1e-9 * want.abs().max(1.0), "{label}: {got} versus {want}");
        }
        Value::Object(want) => {
            let got = actual.as_object().unwrap_or_else(|| panic!("{label}: not an object"));
            let (mut left, mut right): (Vec<_>, Vec<_>) = (got.keys().collect(), want.keys().collect());
            left.sort();
            right.sort();
            assert_eq!(left, right, "{label} keys");
            for (key, value) in want {
                equivalent(&got[key], value, &format!("{label}.{key}"));
            }
        }
        Value::Array(want) => {
            let got = actual.as_array().unwrap_or_else(|| panic!("{label}: not an array"));
            assert_eq!(got.len(), want.len(), "{label} length");
            for (index, value) in want.iter().enumerate() {
                equivalent(&got[index], value, &format!("{label}[{index}]"));
            }
        }
        _ => assert_eq!(actual, expected, "{label}"),
    }
}

fn fnv1a(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash = (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn digest(bytes: &[u8]) -> String {
    format!("{}:{}", bytes.len(), fnv1a(bytes))
}

pub type CoreTransport<'a> = &'a mut dyn FnMut(&str) -> String;
pub type ArchiveTransport<'a> = &'a mut dyn FnMut(&Value, &[Vec<u8>]) -> (Value, Vec<Vec<u8>>);

fn check_core(replies: &[Value]) {
    assert_eq!(replies[0]["scene"]["matrixScenes"][0]["cells"].as_array().unwrap().len(), 9);
    assert_eq!(replies[2]["kind"], "matrix-projections");
    assert_eq!(replies[4]["scene"]["matrixScenes"], replies[6]["scene"]["matrixScenes"]);
    assert_eq!(replies[8]["scene"]["matrixScenes"], replies[0]["scene"]["matrixScenes"]);
    assert_eq!(replies[9]["scene"]["matrixScenes"], replies[6]["scene"]["matrixScenes"]);
    let drafts = replies[2]["matrixScenes"].as_array().unwrap();
    assert_eq!(drafts.len(), 6);
    for pair in (0..drafts.len()).step_by(2) {
        for (index, cell) in drafts[pair]["cells"].as_array().unwrap().iter().enumerate() {
            let translated = &drafts[pair + 1]["cells"][index];
            let at = &cell["pose"]["at"];
            equivalent(
                &translated["pose"],
                &json!({ "at": { "x": at["x"].as_f64().unwrap() + 42.0, "y": at["y"].as_f64().unwrap() - 17.0 }, "rotation": cell["pose"]["rotation"] }),
                "draft translation",
            );
            assert!(cell.get("memberId").is_none());
        }
    }
    assert_eq!(replies[10]["matrixScenes"][0]["cells"].as_array().unwrap().len(), 500);
    assert_eq!(replies[11]["kind"], "preview");
    assert_eq!(replies[12]["kind"], "scene");
    assert_eq!(replies[11]["scene"]["matrixScenes"], replies[12]["scene"]["matrixScenes"]);
    assert_eq!(replies[3]["kind"], "error");
    assert_eq!(replies[4]["kind"], "preview");
    assert_eq!(replies[1]["scene"]["matrixScenes"], replies[5]["scene"]["matrixScenes"]);
    assert_eq!(replies[7]["kind"], "error");
    assert_eq!(replies[13]["document"]["hardware"]["boards"][0]["keyBindings"]["matrix/matrix/r0c0"], "&kp A");
    assert_eq!(replies[14]["document"]["keycaps"]["matrices"]["matrix"]["wallThickness"].as_f64(), Some(1.2));
    assert_eq!(replies[15]["document"]["keycaps"]["boards"]["board"]["color"], "#123456");
    assert_eq!(replies[16]["document"]["keycaps"]["keys"]["matrix/matrix/r0c0"]["legend"], "");
}

fn compact(replies: &mut [Value]) {
    // The 500-cell draft is only recorded by size to keep the golden file small.
    let cells = replies[10]["matrixScenes"][0]["cells"].as_array().unwrap().len();
    replies[10] = json!({ "kind": replies[10]["kind"], "cells": cells });
}

/// Runs every request and archive case through the transports, asserting the
/// boundary's behaviour, and returns the transcript recorded as the golden.
pub fn run(core: CoreTransport, archive: ArchiveTransport) -> Value {
    let mut replies: Vec<Value> = core_requests()
        .iter()
        .map(|request| serde_json::from_str(&core(&request.to_string())).expect("core reply is JSON"))
        .collect();
    check_core(&replies);
    compact(&mut replies);

    let mut transcript = Vec::new();
    let mut call = |request: Value, buffers: &[Vec<u8>]| -> (Value, Vec<Vec<u8>>) {
        let (reply, outputs) = archive(&request, buffers);
        transcript.push(json!({ "kind": request["kind"], "reply": reply, "buffers": outputs.iter().map(|bytes| digest(bytes)).collect::<Vec<_>>() }));
        (reply, outputs)
    };

    let mut project = document();
    project["assets"] = json!([{ "id": "asset-id", "name": "component.step", "mediaType": "model/step", "sha256": ASSET_SHA256 }]);
    project["futureField"] = json!({ "preserve": true });
    let project_json = project.to_string();
    let asset_path = format!("assets/{ASSET_SHA256}");
    let asset = ASSET.to_vec();

    let (reply, packed) = call(json!({ "kind": "pack-project", "projectJson": project_json, "archiveJson": json!({ "embedUsedModels": true }).to_string(), "assets": [{ "path": asset_path, "bufferIndex": 0 }] }), &[asset.clone()]);
    assert_eq!(reply["kind"], "packed");
    let (reply, unpacked) = call(json!({ "kind": "unpack-project" }), &packed);
    assert_eq!(reply["projectJson"], project_json);
    assert_eq!(unpacked[0], asset);

    // Archives from another producer: an old project without archive.json,
    // wrong asset bytes, and a truncated archive.
    let (_, old) = call(json!({ "kind": "pack-files", "entries": [{ "path": "project.json", "bufferIndex": 0 }, { "path": asset_path, "bufferIndex": 1 }] }), &[project_json.clone().into_bytes(), asset.clone()]);
    let (reply, _) = call(json!({ "kind": "unpack-project" }), &old);
    assert_eq!(reply["projectJson"], project_json);
    let (_, wrong) = call(json!({ "kind": "pack-files", "entries": [{ "path": "project.json", "bufferIndex": 0 }, { "path": asset_path, "bufferIndex": 1 }] }), &[project_json.clone().into_bytes(), b"wrong".to_vec()]);
    assert_eq!(call(json!({ "kind": "unpack-project" }), &wrong).0["kind"], "error");
    let truncated = vec![old[0][..old[0].len() - 12].to_vec()];
    assert_eq!(call(json!({ "kind": "unpack-project" }), &truncated).0["kind"], "error");

    let (reply, generic) = call(json!({ "kind": "pack-files", "entries": [{ "path": "BoardStudio.pretty/part.kicad_mod", "bufferIndex": 0 }, { "path": "models/component.step", "bufferIndex": 1 }] }), &[b"(footprint \"part\")".to_vec(), asset.clone()]);
    assert_eq!(reply["kind"], "packed");
    assert_eq!(generic.len(), 1);
    let duplicate = call(json!({ "kind": "pack-files", "entries": [{ "path": "same", "bufferIndex": 0 }, { "path": "same", "bufferIndex": 1 }] }), &[asset.clone(), asset.clone()]);
    assert_eq!(duplicate.0["kind"], "error");

    let payload = payload();
    let mut doc = document();
    doc["assets"] = json!([{ "id": "memory-fixture", "sha256": PAYLOAD_SHA256 }]);
    let large = json!({ "kind": "pack-project", "projectJson": doc.to_string(), "assets": [{ "path": format!("assets/{PAYLOAD_SHA256}"), "bufferIndex": 0 }] });
    let (reply, measured) = call(large, &[payload]);
    assert_eq!(reply["kind"], "packed");
    assert_eq!(call(json!({ "kind": "unpack-project" }), &measured).0["kind"], "unpacked");

    json!({ "core": replies, "archive": transcript })
}

pub fn assert_golden(actual: &Value, golden: &Value) {
    equivalent(actual, golden, "transcript");
}

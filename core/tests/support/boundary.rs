//! Requests, semantic assertions and golden comparison for the native/WASM
//! boundary parity check. The native test (`core/tests/boundary_parity.rs`)
//! records and verifies the golden transcript; the browser test
//! (`web/crates/host/src/boundary_parity.rs`) replays the same requests through the real
//! `CoreEngine` and `archive_request` Rust entry points on `wasm32` and must reproduce it.
//! Ported from `scripts/check-rust-boundaries.mjs`.
#![allow(dead_code)]

use boardstudio_core::model::ProjectDoc;
use serde_json::{Value, json};

/// SHA-256 of [`ASSET`]; the native test recomputes it to guard the constant.
#[path = "boundary/cases.rs"]
mod cases;
#[path = "boundary/scenarios.rs"]
mod scenarios;
#[path = "boundary/transcript.rs"]
mod transcript;

pub use self::cases::core_requests;
pub use self::scenarios::derived_mirror_case;
#[allow(unused_imports)]
pub use self::transcript::{assert_golden, equivalent, run};
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
            "columnLayout": [{"stagger": 0, "splay": 0}, {"stagger": 2, "splayOrigin": {"x": -15, "y": -22}, "splay": 15}, {"stagger": -1, "splay": -8}],
    })
}

/// The normalized bundled mounting-hole catalogue entry projection requires.
fn mounting_hole() -> Value {
    json!({
        "id": "generator:ceoloide/mounting_hole_npth", "name": "mounting hole npth",
        "kind": "custom",
        "pads": [{ "id": "pad-0", "number": "", "at": { "x": 0, "y": 0 },
            "size": { "x": 2.2, "y": 2.2 }, "shape": "circle", "drill": 2.2, "plated": false, "rotation": 0 }],
        "courtyard": [{ "x": -1.1, "y": -1.1 }, { "x": 1.1, "y": -1.1 }, { "x": 1.1, "y": 1.1 }, { "x": -1.1, "y": 1.1 }],
        "envelopeSource": { "courtyard": "generated" },
        "envelopeNotice": "No closed courtyard is available; the outline uses physical graphics and pad extents.",
        "generator": { "source": "ceoloide/mounting_hole_npth", "version": "bundled-1", "parameters": {} }
    })
}

/// One typed `ApplyMechanicalEdit` operation against the canonical board.
fn mechanical_operation(change: Value) -> Value {
    json!({
        "kind": "apply-mechanical-edit",
        "edit": {
            "target": { "kind": "board", "boardId": "board" },
            "change": change,
            "mountingHole": mounting_hole(),
        },
    })
}

pub fn document() -> Value {
    let matrix = matrix();
    let mut doc =
        serde_json::to_value(ProjectDoc::empty("boundary-fixture", "Boundary fixture")).unwrap();
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

/// Shared, current-format seed accepted by both the native and browser/WASM
/// boundary tests. `synthetic_half` is also used by Core integration tests, but
/// the boundary support module must not depend on the test crate's `support`
/// namespace because it is included from the separate host crate.
const GENERATED: &str = "board:board:automatic:outline:1";
const EDITED: &str = "outline-version-12";
const REFINED: &str = "v-refined";

fn board_target() -> Value {
    json!({ "kind": "board", "boardId": "board" })
}

fn version_target(version_id: &str) -> Value {
    json!({ "kind": "version", "boardId": "board", "versionId": version_id })
}

/// One typed `ApplyOutlineEdit` commit. `reviewed` is the generated source shape the
/// user authored the perimeter edit against (absent for every other change).
fn outline_edit(
    id: &str,
    base_revision: u64,
    target: Value,
    change: Value,
    seed: u64,
    reviewed: Option<Value>,
) -> Value {
    let mut edit = json!({ "target": target, "change": change, "seed": seed });
    if let Some(reviewed) = reviewed {
        edit["reviewed"] = reviewed;
    }
    json!({
        "id": id, "kind": "edit",
        "command": { "baseRevision": base_revision, "transactionId": id, "phase": "commit", "targetIds": ["board"],
            "operation": { "kind": "apply-outline-edit", "edit": edit } },
    })
}

/// One ordinary commit (the fixture setup that gives the boundary board a real
/// component to build an outline from).
fn commit(id: &str, base_revision: u64, target_ids: Value, operation: Value) -> Value {
    json!({
        "id": id, "kind": "edit",
        "command": { "baseRevision": base_revision, "transactionId": id, "phase": "commit", "targetIds": target_ids,
            "operation": operation },
    })
}

/// The matrix the transcript's own `set-matrix` left behind: one enabled cell with
/// three generated companions and every generated part identity.
fn matrix_state() -> Value {
    let matrix = matrix();
    let mut copy = matrix.clone();
    copy["cells"] = json!([{ "row": 0, "column": 0, "enabled": true,
        "assemblies": [
            { "id": "diode", "definitionId": "diode", "offset": { "x": 2, "y": 5 } },
            { "id": "led", "definitionId": "led", "offset": { "x": -3, "y": 6 } },
            { "id": "sensor", "definitionId": "sensor", "offset": { "x": 1, "y": -4 } }
        ] }]);
    copy["partIds"] = json!([
        "matrix/matrix/r0c0",
        "matrix/matrix/r0c0/diode",
        "matrix/matrix/r0c0/led",
        "matrix/matrix/r0c0/sensor",
        "matrix/matrix/r0c1",
        "matrix/matrix/r0c2",
        "matrix/matrix/r1c0",
        "matrix/matrix/r1c1",
        "matrix/matrix/r1c2",
        "matrix/matrix/r2c0",
        "matrix/matrix/r2c1",
        "matrix/matrix/r2c2",
    ]);
    copy
}

/// The typed outline vocabulary crossing the native/WASM seam, appended after the
/// matrix and mechanical evidence.
///
/// The fixture's `switch` definition carries no courtyard, so the block first authors
/// a courtyard-bearing definition, places one component on the board and prunes the
/// generated, geometry-less parts. That leaves the Generated envelope a real source:
/// the block then creates it, changes its settings (including a repeat that settles
/// unchanged), adds and removes an authored connection, sets and removes a protected
/// gap, and edits the generated perimeter from its reviewed shape into one fixed copy.
/// The remaining entries exercise the version lifecycle - a version-local setting, an
/// authored version feature, a rename, activation, a linked refinement with its own
/// margin, Freeze, a linked perimeter edit, a copy recording its origin, a delete and
/// the single Undo that restores it.
fn outline_requests() -> Vec<Value> {
    let mut requests = vec![
        // The boundary fixture's parts borrow a geometry-less definition, so the
        // outline block authors one with a courtyard and places a single component.
        commit(
            "outline-definition",
            17,
            json!(["matrix"]),
            json!({ "kind": "set-matrix", "matrix": matrix_state(), "definitions": [{
                "id": "outline-switch", "name": "Outline switch", "kind": "switch",
                "courtyard": [{ "x": -7.0, "y": -7.0 }, { "x": 7.0, "y": -7.0 }, { "x": 7.0, "y": 7.0 }, { "x": -7.0, "y": 7.0 }],
                "pads": [{ "id": "p0", "number": "1", "at": { "x": 0.0, "y": 0.0 },
                    "size": { "x": 4.0, "y": 4.0 }, "shape": "rect" }]
            }] }),
        ),
        commit(
            "outline-component",
            18,
            json!([]),
            json!({ "kind": "add-part", "boardId": "board", "part": {
                "id": "outline-part", "definitionId": "outline-switch", "reference": "OSW1", "side": "front",
                "pose": { "at": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 } } }),
        ),
        commit(
            "outline-prune",
            19,
            json!([]),
            json!({ "kind": "remove-parts", "ids": [
                "matrix/matrix/r0c0", "matrix/matrix/r0c0/diode", "matrix/matrix/r0c0/led",
                "matrix/matrix/r0c0/sensor", "matrix/matrix/r0c1", "matrix/matrix/r0c2",
                "matrix/matrix/r1c0", "matrix/matrix/r1c1", "matrix/matrix/r1c2",
                "matrix/matrix/r2c0", "matrix/matrix/r2c1", "matrix/matrix/r2c2" ] }),
        ),
    ];
    // Generate authors the Generated envelope and its board membership.
    // One row per typed outline edit: (id, base revision, target, change, seed, reviewed).
    let board = board_target();
    let edited = version_target(EDITED);
    let refined = version_target(REFINED);
    type OutlineEdit<'a> = (&'a str, u64, Value, Value, u64, Option<Value>);
    let edits: Vec<OutlineEdit<'_>> = vec![
        (
            "outline-generate",
            20,
            board.clone(),
            json!({ "create-generated": { "margin": 4.0, "settings": { "corners": "fillet", "size": 2.0, "bridgeWidth": 10.0 }, "part_ids": ["outline-part"] } }),
            1,
            None,
        ),
        (
            "outline-corners",
            21,
            board.clone(),
            json!({ "set-corners": { "corners": "sharp" } }),
            2,
            None,
        ),
        (
            "outline-corners-repeat",
            22,
            board.clone(),
            json!({ "set-corners": { "corners": "sharp" } }),
            3,
            None,
        ),
        (
            "outline-margin",
            22,
            board.clone(),
            json!({ "set-margin": { "margin": 6.0 } }),
            4,
            None,
        ),
        (
            "outline-bridge",
            23,
            board.clone(),
            json!({ "set-bridge-width": { "bridge_width": 3.0 } }),
            5,
            None,
        ),
        (
            "outline-connection-add",
            24,
            board.clone(),
            json!({ "add-connection": { "connection": { "id": "conn-a", "width": 3.0, "points": [{ "at": { "x": -6.0, "y": 0.0 } }, { "at": { "x": 6.0, "y": 0.0 } }] } } }),
            6,
            None,
        ),
        (
            "outline-connection-remove",
            25,
            board.clone(),
            json!({ "remove-connection": { "connection_id": "conn-a" } }),
            7,
            None,
        ),
        (
            "outline-gap-set",
            26,
            board.clone(),
            json!({ "set-protected-gap": { "gap_id": "gap-a", "points": [ { "at": { "x": -4.0, "y": -4.0 } }, { "at": { "x": 4.0, "y": -4.0 } }, { "at": { "x": 4.0, "y": 4.0 } }, { "at": { "x": -4.0, "y": 4.0 } }] } }),
            8,
            None,
        ),
        (
            "outline-gap-remove",
            27,
            board.clone(),
            json!({ "remove-protected-gap": { "gap_id": "gap-a" } }),
            9,
            None,
        ),
        (
            "outline-perimeter",
            28,
            board.clone(),
            json!({ "edit-generated-perimeter": { "contour": 0, "points": [{ "x": -12.0, "y": -12.0 }, { "x": 12.0, "y": -12.0 }, { "x": 12.0, "y": 12.0 }, { "x": -12.0, "y": 12.0 }] } }),
            12,
            Some(json!([{ "hole": false, "points": [
                { "x": -13.0, "y": 13.0 }, { "x": -13.0, "y": -13.0 }, { "x": 13.0, "y": -13.0 }, { "x": 13.0, "y": 13.0 }] }])),
        ),
        (
            "outline-version-bridge",
            29,
            edited.clone(),
            json!({ "set-bridge-width": { "bridge_width": 1.5 } }),
            12,
            None,
        ),
        (
            "outline-version-feature-add",
            30,
            edited.clone(),
            json!({ "add-feature": { "feature": { "kind": "polygon", "id": "cut-a", "operation": "subtract", "points": [{ "x": -3.0, "y": -3.0 }, { "x": 3.0, "y": -3.0 }, { "x": 3.0, "y": 3.0 }, { "x": -3.0, "y": 3.0 }] } } }),
            13,
            None,
        ),
        (
            "outline-version-feature-remove",
            31,
            edited.clone(),
            json!({ "remove-feature": { "feature_id": "cut-a" } }),
            14,
            None,
        ),
        (
            "outline-version-rename",
            32,
            edited.clone(),
            json!({ "rename": { "name": "Renamed" } }),
            15,
            None,
        ),
        (
            "outline-activate-generated",
            33,
            board.clone(),
            json!("activate"),
            16,
            None,
        ),
        (
            "outline-refine",
            34,
            board.clone(),
            json!({ "create-linked-refinement": { "version_id": REFINED, "name": "Refined" } }),
            17,
            None,
        ),
        (
            "outline-refine-margin",
            35,
            refined.clone(),
            json!({ "set-margin": { "margin": 7.0 } }),
            18,
            None,
        ),
        (
            "outline-freeze",
            36,
            refined.clone(),
            json!({ "freeze": { "name": "Frozen" } }),
            19,
            None,
        ),
        (
            "outline-activate-refined",
            37,
            refined.clone(),
            json!("activate"),
            20,
            None,
        ),
        (
            "outline-refine-perimeter",
            38,
            refined.clone(),
            json!({ "edit-feature-perimeter": { "feature_id": "board:board:version:v-refined:contour:0", "points": [{ "x": -13.0, "y": -13.0 }, { "x": 13.0, "y": -13.0 }, { "x": 13.0, "y": 13.0 }, { "x": -13.0, "y": 13.0 }] } }),
            21,
            None,
        ),
        (
            "outline-copy",
            39,
            edited.clone(),
            json!({ "copy": { "version_id": "v-copy", "name": "Copy" } }),
            22,
            None,
        ),
        (
            "outline-delete",
            40,
            version_target("v-copy"),
            json!("delete"),
            23,
            None,
        ),
    ];
    requests.extend(
        edits
            .into_iter()
            .map(|(id, revision, target, change, seed, reviewed)| {
                outline_edit(id, revision, target, change, seed, reviewed)
            }),
    );
    requests.push(json!({ "id": "outline-undo", "kind": "undo" }));
    // A rename to the accepted name settles unchanged: no revision step.
    requests.push(outline_edit(
        "outline-rename-repeat",
        42,
        edited,
        json!({ "rename": { "name": "Renamed" } }),
        24,
        None,
    ));
    requests.push(json!({ "id": "outline-snapshot", "kind": "snapshot" }));
    requests
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

/// The reply index of one named request. The mechanical and outline blocks append
/// entries, so their assertions locate their own replies instead of counting from the
/// end of the transcript.
fn position(replies: &[Value], id: &str) -> usize {
    replies
        .iter()
        .position(|reply| reply["id"] == id)
        .unwrap_or_else(|| panic!("reply {id}"))
}

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
        for (index, origin) in [json!({ "x": 0, "y": 0 }), json!({ "x": 42, "y": -17 })]
            .into_iter()
            .enumerate()
        {
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
    // Named placement changes cross the native/WASM seam for every variant.
    let mut revision = 8;
    for (name, change) in [
        (
            "stagger",
            json!({ "kind": "column-stagger", "column": 2, "value": 4.5 }),
        ),
        (
            "column-offset",
            json!({ "kind": "column-offset", "column": 1, "value": { "x": 4, "y": -2 } }),
        ),
        (
            "row-offset",
            json!({ "kind": "row-offset", "row": 0, "value": { "x": 3, "y": 5 } }),
        ),
    ] {
        for phase in ["preview", "commit"] {
            requests.push(json!({
                "id": format!("placement-{name}-{phase}"), "kind": "edit",
                "command": { "baseRevision": revision, "transactionId": name, "phase": phase, "targetIds": ["matrix"],
                    "operation": { "kind": "set-matrix-placement", "matrixId": "matrix", "change": change.clone() } },
            }));
        }
        revision += 1;
    }
    // Generated companions cross the seam with each placement mode.
    requests.push(json!({
        "id": "companion-matrix", "kind": "edit",
        "command": { "baseRevision": revision, "transactionId": "companion", "phase": "commit", "targetIds": ["matrix"],
            "operation": { "kind": "set-matrix",
                "matrix": with(json!({
                    "id": "matrix",
                    "cells": [{ "row": 0, "column": 0, "enabled": true,
                        "assemblies": [
                            { "id": "diode", "definitionId": "diode", "offset": { "x": 2, "y": 5 } },
                            { "id": "led", "definitionId": "led", "offset": { "x": -3, "y": 6 } },
                            { "id": "sensor", "definitionId": "sensor", "offset": { "x": 1, "y": -4 } }
                        ] }],
                    "partIds": [],
                })),
                "definitions": [
                    { "id": "diode", "name": "Diode", "kind": "passive", "pads": [], "courtyard": [] },
                    { "id": "led", "name": "LED", "kind": "passive", "pads": [], "courtyard": [] },
                    { "id": "sensor", "name": "Sensor", "kind": "passive", "pads": [], "courtyard": [] }
                ] } },
    }));
    requests.push(json!({
        "id": "companion-constraint", "kind": "edit",
        "command": { "baseRevision": revision + 1, "transactionId": "companion-constraint", "phase": "commit", "targetIds": [],
            "operation": { "kind": "set-constraint",
                "constraint": { "kind": "offset", "id": "sensor-key", "sourcePartId": "matrix/matrix/r0c0",
                    "targetPartId": "matrix/matrix/r0c0/sensor", "offset": { "x": 1, "y": -4 }, "rotation": 0 } } },
    }));
    requests.push(json!({
        "id": "companion-author", "kind": "edit",
        "command": { "baseRevision": revision + 2, "transactionId": "companion-author", "phase": "commit", "targetIds": [],
            "operation": { "kind": "move-parts", "positions": [{ "id": "matrix/matrix/r0c0/led", "at": { "x": -10, "y": 8 } }] } },
    }));
    requests.push(json!({ "id": "companion-snapshot", "kind": "snapshot" }));
    // A typed mechanical edit crosses the native/WASM seam carrying its target,
    // its typed change and the normalized mounting-hole catalogue definition the
    // generated closure clearance is projected from. `Enable` creates the
    // canonical configuration this project does not have yet; the closure
    // request then adds two suggested closure mounts and projects their
    // generated clearance; the identical repeat is a no-op with no revision
    // step; one Undo removes the mounts and every generated part together.
    requests.push(json!({
        "id": "mechanical-enable", "kind": "edit",
        "command": { "baseRevision": 14, "transactionId": "mechanical-enable", "phase": "commit", "targetIds": ["board"],
            "operation": mechanical_operation(json!("enable")) },
    }));
    let closure = json!({ "add-suggested-closure-mounts": [
        { "id": "left-front", "at": { "x": -30, "y": 25 }, "kind": "hole", "holeDiameter": 2.2 },
        { "id": "left-back", "at": { "x": 12.5, "y": -7.25 }, "kind": "hole", "holeDiameter": 2.2 },
    ] });
    for phase in ["preview", "commit"] {
        requests.push(json!({
            "id": format!("mechanical-closure-{phase}"), "kind": "edit",
            "command": { "baseRevision": 15, "transactionId": "mechanical-closure", "phase": phase, "targetIds": ["board"],
                "operation": mechanical_operation(closure.clone()) },
        }));
    }
    requests.push(json!({
        "id": "mechanical-closure-repeat", "kind": "edit",
        "command": { "baseRevision": 16, "transactionId": "mechanical-closure", "phase": "commit", "targetIds": ["board"],
            "operation": mechanical_operation(closure.clone()) },
    }));
    requests.push(json!({ "id": "mechanical-undo", "kind": "undo" }));
    requests.extend(outline_requests());
    // One-shot conversion crosses the seam last, so earlier requests keep their
    // revisions. Column 1 pivots on a custom origin, so the conversion is exact;
    // column 2 is splayed about its default origin, so Core refuses it.
    let placement = |id: &str, base: u64, phase: &str, change: Value| {
        json!({
            "id": id, "kind": "edit",
            "command": { "baseRevision": base, "transactionId": id, "phase": phase, "targetIds": ["matrix"],
                "operation": { "kind": "set-matrix-placement", "matrixId": "matrix", "change": change } },
        })
    };
    let convert = |column: u64| json!({ "kind": "column-offset-to-stagger", "column": column });
    requests.push(placement(
        "convert-offset-1",
        42,
        "commit",
        json!({ "kind": "column-offset", "column": 1, "value": { "x": 0, "y": 3 } }),
    ));
    requests.push(placement("convert-preview", 43, "preview", convert(1)));
    requests.push(placement("convert-commit", 43, "commit", convert(1)));
    requests.push(placement(
        "convert-offset-2",
        44,
        "commit",
        json!({ "kind": "column-offset", "column": 2, "value": { "x": 0, "y": 1 } }),
    ));
    requests.push(placement("convert-refused", 45, "commit", convert(2)));
    // The mirrored-half operation crosses the seam in both modes. This fixture has no physical
    // hardware, so each request is refused: the wire shape and the refusal are what is recorded.
    for (id, mode) in [
        (
            "mirrored-half-new-board",
            json!({ "kind": "new-board", "boardId": "mirror", "instanceId": "mirror" }),
        ),
        (
            "mirrored-half-reuse-board",
            json!({ "kind": "reuse-board", "instanceId": "flipped" }),
        ),
    ] {
        requests.push(json!({
            "id": id, "kind": "edit",
            "command": { "baseRevision": 45, "transactionId": id, "phase": "commit", "targetIds": [],
                "operation": { "kind": "create-mirrored-half", "pairId": "pair", "sourceInstanceId": "board",
                    "mode": mode, "mapping": { "at": { "x": 0, "y": 0 }, "rotation": 0 }, "idRule": "board-prefix" } },
        }));
    }
    requests
}

/// Runs the paired Preview → Undo → Redo case beside the golden transcript. Keeping it
/// separate leaves the existing cross-runtime golden untouched until a maintainer blesses it.
pub fn paired_preview_history_case(mut core: CoreTransport<'_>) -> Value {
    let call = |core: &mut CoreTransport<'_>, request: Value| {
        serde_json::from_str::<Value>(&core(&request.to_string())).expect("core reply is JSON")
    };
    let mut source = crate::support::synthetic_half::document();
    for cell in source["matrices"][0]["cells"].as_array_mut().unwrap() {
        let object = cell.as_object_mut().unwrap();
        object.remove("assembliesLocal");
        object.remove("definitionId");
    }
    let open = call(
        &mut core,
        json!({
            "id":"paired-open",
            "kind":"open",
            "document":source
        }),
    );
    assert_eq!(open["kind"], "scene", "{open}");
    let created = call(
        &mut core,
        json!({
            "id":"paired-create",
            "kind":"edit",
            "command":{
                "baseRevision":open["scene"]["revision"],
                "transactionId":"paired-create",
                "phase":"commit",
                "targetIds":[],
                "operation":crate::support::synthetic_half::create_mirrored_half_operation(
                    "boundary-pair",
                    "right"
                )
            }
        }),
    );
    assert_eq!(created["kind"], "scene", "{created}");
    let before_preview = call(&mut core, json!({"id":"paired-before","kind":"snapshot"}));
    let preview = call(
        &mut core,
        json!({
            "id":"paired-preview",
            "kind":"edit",
            "command":{
                "baseRevision":before_preview["document"]["revision"],
                "transactionId":"paired-preview",
                "phase":"preview",
                "targetIds":["right/U1"],
                "operation":{"kind":"move-parts","positions":[{
                    "id":"right/U1","at":{"x":216,"y":14}
                }]}
            }
        }),
    );
    assert_eq!(preview["kind"], "preview", "{preview}");
    let preview_part_x = |id: &str| {
        preview["document"]["parts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|part| part["id"] == id)
            .unwrap_or_else(|| panic!("preview Part {id} is missing"))["pose"]["at"]["x"]
            .as_f64()
            .unwrap()
    };
    assert_eq!(preview_part_x("left/U1"), -16.0);
    assert_eq!(preview_part_x("right/U1"), 216.0);
    for id in ["left/U1", "right/U1"] {
        assert!(
            preview["scene"]["changedIds"]
                .as_array()
                .unwrap()
                .iter()
                .any(|changed| changed == id),
            "paired preview omitted {id}"
        );
    }
    let after_preview = call(&mut core, json!({"id":"paired-after","kind":"snapshot"}));
    assert_eq!(after_preview["document"], before_preview["document"]);

    let undo = call(&mut core, json!({"id":"paired-undo","kind":"undo"}));
    assert_eq!(undo["kind"], "scene", "{undo}");
    assert!(
        undo["document"]["hardware"]
            .get("pairs")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
    );
    let redo = call(&mut core, json!({"id":"paired-redo","kind":"redo"}));
    assert_eq!(redo["kind"], "scene", "{redo}");
    assert_eq!(
        redo["document"]["hardware"]["pairs"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    json!({
        "open":open,
        "created":created,
        "beforePreview":before_preview,
        "preview":preview,
        "afterPreview":after_preview,
        "undo":undo,
        "redo":redo
    })
}

/// The board's generated envelope identity, derived by Core from the captured seed 1.
const GENERATED: &str = "board:board:automatic:outline:1";
/// The fixed copy `edit-generated-perimeter` created for the board (seed 12).
const EDITED: &str = "outline-version-12";
/// The linked refinement created from the board's current components.
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
    let requests = vec![
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
        // Generate authors the Generated envelope and its board membership.
        outline_edit(
            "outline-generate",
            20,
            board_target(),
            json!({ "create-generated": {
            "margin": 4.0,
            "settings": { "corners": "fillet", "size": 2.0, "bridgeWidth": 10.0 },
            "part_ids": ["outline-part"] } }),
            1,
            None,
        ),
        outline_edit(
            "outline-corners",
            21,
            board_target(),
            json!({ "set-corners": { "corners": "sharp" } }),
            2,
            None,
        ),
        outline_edit(
            "outline-corners-repeat",
            22,
            board_target(),
            json!({ "set-corners": { "corners": "sharp" } }),
            3,
            None,
        ),
        outline_edit(
            "outline-margin",
            22,
            board_target(),
            json!({ "set-margin": { "margin": 6.0 } }),
            4,
            None,
        ),
        outline_edit(
            "outline-bridge",
            23,
            board_target(),
            json!({ "set-bridge-width": { "bridge_width": 3.0 } }),
            5,
            None,
        ),
        outline_edit(
            "outline-connection-add",
            24,
            board_target(),
            json!({ "add-connection": { "connection": {
            "id": "conn-a", "width": 3.0,
            "points": [{ "at": { "x": -6.0, "y": 0.0 } }, { "at": { "x": 6.0, "y": 0.0 } }] } } }),
            6,
            None,
        ),
        outline_edit(
            "outline-connection-remove",
            25,
            board_target(),
            json!({ "remove-connection": { "connection_id": "conn-a" } }),
            7,
            None,
        ),
        outline_edit(
            "outline-gap-set",
            26,
            board_target(),
            json!({ "set-protected-gap": { "gap_id": "gap-a", "points": [
            { "at": { "x": -4.0, "y": -4.0 } }, { "at": { "x": 4.0, "y": -4.0 } },
            { "at": { "x": 4.0, "y": 4.0 } }, { "at": { "x": -4.0, "y": 4.0 } }] } }),
            8,
            None,
        ),
        outline_edit(
            "outline-gap-remove",
            27,
            board_target(),
            json!({ "remove-protected-gap": { "gap_id": "gap-a" } }),
            9,
            None,
        ),
        // The perimeter edit carries the generated source shape the user reviewed and
        // creates one fixed copy in the same accepted edit.
        outline_edit(
            "outline-perimeter",
            28,
            board_target(),
            json!({ "edit-generated-perimeter": {
            "contour": 0,
            "points": [{ "x": -12.0, "y": -12.0 }, { "x": 12.0, "y": -12.0 }, { "x": 12.0, "y": 12.0 }, { "x": -12.0, "y": 12.0 }] } }),
            12,
            Some(json!([{ "hole": false, "points": [
                { "x": -13.0, "y": 13.0 }, { "x": -13.0, "y": -13.0 }, { "x": 13.0, "y": -13.0 }, { "x": 13.0, "y": 13.0 }] }])),
        ),
        outline_edit(
            "outline-version-bridge",
            29,
            version_target(EDITED),
            json!({ "set-bridge-width": { "bridge_width": 1.5 } }),
            12,
            None,
        ),
        outline_edit(
            "outline-version-feature-add",
            30,
            version_target(EDITED),
            json!({ "add-feature": { "feature": {
            "kind": "polygon", "id": "cut-a", "operation": "subtract",
            "points": [{ "x": -3.0, "y": -3.0 }, { "x": 3.0, "y": -3.0 }, { "x": 3.0, "y": 3.0 }, { "x": -3.0, "y": 3.0 }] } } }),
            13,
            None,
        ),
        outline_edit(
            "outline-version-feature-remove",
            31,
            version_target(EDITED),
            json!({ "remove-feature": { "feature_id": "cut-a" } }),
            14,
            None,
        ),
        outline_edit(
            "outline-version-rename",
            32,
            version_target(EDITED),
            json!({ "rename": { "name": "Renamed" } }),
            15,
            None,
        ),
        outline_edit(
            "outline-activate-generated",
            33,
            board_target(),
            json!("activate"),
            16,
            None,
        ),
        // A linked refinement follows the board's current component and owns its settings.
        outline_edit(
            "outline-refine",
            34,
            board_target(),
            json!({ "create-linked-refinement": { "version_id": REFINED, "name": "Refined" } }),
            17,
            None,
        ),
        outline_edit(
            "outline-refine-margin",
            35,
            version_target(REFINED),
            json!({ "set-margin": { "margin": 7.0 } }),
            18,
            None,
        ),
        outline_edit(
            "outline-freeze",
            36,
            version_target(REFINED),
            json!({ "freeze": { "name": "Frozen" } }),
            19,
            None,
        ),
        outline_edit(
            "outline-activate-refined",
            37,
            version_target(REFINED),
            json!("activate"),
            20,
            None,
        ),
        // Editing the linked refinement's perimeter keeps the link and adds a fixed copy.
        outline_edit(
            "outline-refine-perimeter",
            38,
            version_target(REFINED),
            json!({ "edit-feature-perimeter": {
            "feature_id": "board:board:version:v-refined:contour:0",
            "points": [{ "x": -13.0, "y": -13.0 }, { "x": 13.0, "y": -13.0 }, { "x": 13.0, "y": 13.0 }, { "x": -13.0, "y": 13.0 }] } }),
            21,
            None,
        ),
        outline_edit(
            "outline-copy",
            39,
            version_target(EDITED),
            json!({ "copy": { "version_id": "v-copy", "name": "Copy" } }),
            22,
            None,
        ),
        outline_edit(
            "outline-delete",
            40,
            version_target("v-copy"),
            json!("delete"),
            23,
            None,
        ),
        json!({ "id": "outline-undo", "kind": "undo" }),
        // A rename to the accepted name settles unchanged: no revision step.
        outline_edit(
            "outline-rename-repeat",
            42,
            version_target(EDITED),
            json!({ "rename": { "name": "Renamed" } }),
            24,
            None,
        ),
        json!({ "id": "outline-snapshot", "kind": "snapshot" }),
    ];
    requests
}

/// Numbers match to a relative 1e-9; everything else exactly.
pub fn equivalent(actual: &Value, expected: &Value, label: &str) {
    match expected {
        Value::Number(want) => {
            let (got, want) = (
                actual
                    .as_f64()
                    .unwrap_or_else(|| panic!("{label}: not a number: {actual}")),
                want.as_f64().unwrap(),
            );
            assert!(
                (got - want).abs() <= 1e-9 * want.abs().max(1.0),
                "{label}: {got} versus {want}"
            );
        }
        Value::Object(want) => {
            let got = actual
                .as_object()
                .unwrap_or_else(|| panic!("{label}: not an object"));
            let (mut left, mut right): (Vec<_>, Vec<_>) =
                (got.keys().collect(), want.keys().collect());
            left.sort();
            right.sort();
            assert_eq!(left, right, "{label} keys");
            for (key, value) in want {
                equivalent(&got[key], value, &format!("{label}.{key}"));
            }
        }
        Value::Array(want) => {
            let got = actual
                .as_array()
                .unwrap_or_else(|| panic!("{label}: not an array"));
            assert_eq!(got.len(), want.len(), "{label} length");
            for (index, value) in want.iter().enumerate() {
                // An entry with an `id` is named by it, so a failure at `core[11]` is findable.
                let entry = match value.get("id").and_then(Value::as_str) {
                    Some(id) => format!("{label}[{index}:{id}]"),
                    None => format!("{label}[{index}]"),
                };
                equivalent(&got[index], value, &entry);
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

/// The reply index of one named request. The mechanical and outline blocks append
/// entries, so their assertions locate their own replies instead of counting from the
/// end of the transcript.
fn position(replies: &[Value], id: &str) -> usize {
    replies
        .iter()
        .position(|reply| reply["id"] == id)
        .unwrap_or_else(|| panic!("reply {id}"))
}

fn check_core(replies: &[Value]) {
    assert_eq!(
        replies[0]["scene"]["matrixScenes"][0]["cells"]
            .as_array()
            .unwrap()
            .len(),
        9
    );
    assert_eq!(replies[2]["kind"], "matrix-projections");
    assert_eq!(
        replies[4]["scene"]["matrixScenes"],
        replies[6]["scene"]["matrixScenes"]
    );
    assert_eq!(
        replies[8]["scene"]["matrixScenes"],
        replies[0]["scene"]["matrixScenes"]
    );
    assert_eq!(
        replies[9]["scene"]["matrixScenes"],
        replies[6]["scene"]["matrixScenes"]
    );
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
    assert_eq!(
        replies[10]["matrixScenes"][0]["cells"]
            .as_array()
            .unwrap()
            .len(),
        500
    );
    assert_eq!(replies[11]["kind"], "preview");
    assert_eq!(replies[12]["kind"], "scene");
    assert_eq!(
        replies[11]["scene"]["matrixScenes"],
        replies[12]["scene"]["matrixScenes"]
    );
    assert_eq!(replies[3]["kind"], "error");
    assert_eq!(replies[4]["kind"], "preview");
    assert_eq!(
        replies[1]["scene"]["matrixScenes"],
        replies[5]["scene"]["matrixScenes"]
    );
    assert_eq!(replies[7]["kind"], "error");
    assert_eq!(
        replies[13]["document"]["hardware"]["boards"][0]["keyBindings"]["matrix/matrix/r0c0"],
        "&kp A"
    );
    assert_eq!(
        replies[14]["document"]["keycaps"]["matrices"]["matrix"]["wallThickness"].as_f64(),
        Some(1.2)
    );
    assert_eq!(
        replies[15]["document"]["keycaps"]["boards"]["board"]["color"],
        "#123456"
    );
    assert_eq!(
        replies[16]["document"]["keycaps"]["keys"]["matrix/matrix/r0c0"]["legend"],
        ""
    );
    let companions = replies[26]["scene"]["matrixScenes"][0]["companions"]
        .as_array()
        .unwrap();
    assert_eq!(companions.len(), 3);
    assert!(companions.iter().any(|item| item["mode"] == "follows"));
    assert!(companions.iter().any(|item| item["mode"] == "authored"));
    assert!(companions.contains(&json!({
        "keyId": "matrix/matrix/r0c0",
        "mode": { "constrained": { "constraintId": "sensor-key", "sourcePartId": "matrix/matrix/r0c0" } },
        "partId": "matrix/matrix/r0c0/sensor"
    })));

    // The mechanical edit reports its configuration owner, the added closure
    // mounts and the generated clearance it projects, then a no-op keeps the
    // revision and one Undo removes the complete result.
    let mechanical = position(replies, "mechanical-enable");
    assert_eq!(replies[mechanical]["kind"], "scene");
    assert_eq!(
        replies[mechanical]["scene"]["changedIds"],
        json!(["board"]),
        "enabling reports the configuration owner"
    );
    assert_eq!(replies[mechanical + 1]["kind"], "preview");
    let changed = replies[mechanical + 2]["scene"]["changedIds"]
        .as_array()
        .unwrap();
    assert!(changed.iter().any(|id| id.as_str() == Some("board")));
    for mount in ["auto-closure/left-front", "auto-closure/left-back"] {
        assert!(
            changed.iter().any(|id| id.as_str() == Some(mount)),
            "changed ids name {mount}"
        );
    }
    let count_generated = |reply: &Value, collection: &str, prefix: &str| -> usize {
        reply["document"][collection]
            .as_array()
            .unwrap()
            .iter()
            .filter(|entry| {
                entry["id"]
                    .as_str()
                    .is_some_and(|id| id.starts_with(prefix))
            })
            .count()
    };
    assert_eq!(
        count_generated(&replies[mechanical + 2], "parts", "case-closure/"),
        2
    );
    assert_eq!(
        count_generated(
            &replies[mechanical + 2],
            "definitions",
            "assembly-closure/definition/"
        ),
        2
    );
    for entry in replies[mechanical + 2]["document"]["parts"]
        .as_array()
        .unwrap()
    {
        let id = entry["id"].as_str().unwrap();
        if id.starts_with("case-closure/") {
            assert!(
                changed
                    .iter()
                    .any(|candidate| candidate.as_str() == Some(id)),
                "generated {id} is reported"
            );
        }
    }
    assert_eq!(
        replies[mechanical + 3]["scene"]["changedIds"],
        json!([]),
        "an identical commit changes nothing"
    );
    assert_eq!(
        replies[mechanical + 3]["scene"]["revision"],
        replies[mechanical + 2]["scene"]["revision"],
        "a no-op edit adds no revision step"
    );
    assert_eq!(
        replies[mechanical + 4]["document"]["mechanical"]["closureMounts"],
        json!([]),
        "one Undo restores the closure mounts the edit added"
    );
    assert_eq!(
        count_generated(&replies[mechanical + 4], "parts", "case-closure/"),
        0
    );
}

/// The typed outline block's semantic assertions: generated/changed identities,
/// generated contours and the persisted document shape, verified before the golden
/// transcript is blessed.
fn check_outline(replies: &[Value]) {
    let by_id = |id: &str| {
        replies
            .iter()
            .find(|reply| reply["id"] == id)
            .unwrap_or_else(|| panic!("reply {id}"))
    };
    let changed = |id: &str| by_id(id)["scene"]["changedIds"].clone();
    let revision = |id: &str| by_id(id)["scene"]["revision"].clone();
    let gaps = |contours: &Value| contours.as_array().unwrap().len();
    let settings = |id: &str, feature_id: &str| -> Value {
        by_id(id)["document"]["outline"]
            .as_array()
            .unwrap()
            .iter()
            .find(|feature| feature["id"] == feature_id)
            .unwrap_or_else(|| panic!("feature {feature_id} in {id}"))["settings"]
            .clone()
    };
    let version = |id: &str, version_id: &str| -> Value {
        by_id(id)["document"]["boardOutlines"]
            .as_array()
            .unwrap()
            .iter()
            .find(|state| state["boardId"] == "board")
            .unwrap()["versions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|version| version["id"] == version_id)
            .unwrap_or_else(|| panic!("version {version_id} in {id}"))
            .clone()
    };
    let version_ids = |id: &str| -> Vec<String> {
        by_id(id)["document"]["boardOutlines"][0]["versions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|version| version["id"].as_str().unwrap().to_owned())
            .collect()
    };
    let board_contours = |id: &str| -> Vec<usize> {
        by_id(id)["scene"]["boardContours"][0]["contours"]
            .as_array()
            .unwrap()
            .iter()
            .map(|contour| contour["points"].as_array().unwrap().len())
            .collect()
    };

    // Generate authors the envelope and its membership in one edit, and the envelope
    // resolves against the board's own component.
    assert_eq!(
        changed("outline-generate"),
        json!(["board", GENERATED]),
        "Generate reports the board and the envelope identity Core derived from the seed"
    );
    let envelope = &by_id("outline-generate")["document"]["outline"][0];
    assert_eq!(envelope["kind"], "part-envelope");
    assert_eq!(envelope["partIds"], json!(["outline-part"]));
    assert_eq!(envelope["margin"], json!(4.0));
    assert_eq!(envelope["settings"]["corners"], "fillet");
    assert_eq!(
        by_id("outline-generate")["document"]["boards"][0]["outlineIds"],
        json!([GENERATED]),
        "one edit registers the envelope on the board"
    );
    assert_eq!(
        gaps(&by_id("outline-generate")["scene"]["boardContours"][0]["contours"]),
        1
    );
    assert_eq!(
        board_contours("outline-generate"),
        vec![20],
        "a filleted envelope"
    );

    // Field edits change the envelope; an identical edit adds no revision step.
    assert_eq!(changed("outline-corners"), json!(["board", GENERATED]));
    assert_eq!(settings("outline-corners", GENERATED)["corners"], "sharp");
    assert_eq!(board_contours("outline-corners"), vec![4], "sharp corners");
    assert_eq!(changed("outline-corners-repeat"), json!([]));
    assert_eq!(
        revision("outline-corners-repeat"),
        revision("outline-corners"),
        "an equal field edit adds no revision step"
    );
    assert_eq!(changed("outline-margin"), json!(["board", GENERATED]));
    assert_eq!(settings("outline-margin", GENERATED)["corners"], "sharp");
    assert_eq!(
        by_id("outline-margin")["document"]["outline"][0]["margin"],
        json!(6.0)
    );
    assert_eq!(
        settings("outline-bridge", GENERATED)["bridgeWidth"],
        json!(3.0)
    );

    // An authored connection and the protected gap report their own identities.
    assert_eq!(
        changed("outline-connection-add"),
        json!(["board", GENERATED, "conn-a"])
    );
    let feature = &by_id("outline-connection-add")["document"]["outline"][0];
    assert_eq!(feature["connections"].as_array().unwrap().len(), 1);
    assert_eq!(feature["connections"][0]["id"], "conn-a");
    assert_eq!(feature["connections"][0]["width"], json!(3.0));
    assert_eq!(
        changed("outline-connection-remove"),
        json!(["board", GENERATED, "conn-a"])
    );
    assert!(
        by_id("outline-connection-remove")["document"]["outline"][0]
            .get("connections")
            .is_none(),
        "the removed connection is gone"
    );
    assert_eq!(
        changed("outline-gap-set"),
        json!(["board", GENERATED, "gap-a"])
    );
    assert_eq!(
        by_id("outline-gap-set")["document"]["outline"][0]["settings"]["repair"]["keepGaps"][0]["id"],
        "gap-a"
    );
    assert_eq!(
        gaps(&by_id("outline-gap-set")["scene"]["boardContours"][0]["contours"]),
        2,
        "a protected gap opens a second region in the resolved boundary"
    );
    assert_eq!(
        changed("outline-gap-remove"),
        json!(["board", GENERATED, "gap-a"])
    );
    assert!(
        by_id("outline-gap-remove")["document"]["outline"][0]["settings"]["repair"]
            .get("keepGaps")
            .is_none(),
        "the removed gap is gone"
    );
    assert_eq!(board_contours("outline-gap-remove"), vec![4]);

    // The reviewed perimeter edit creates one fixed copy, selected, whose contour is
    // the submitted shape; the version carries its own bridge width.
    assert_eq!(changed("outline-perimeter"), json!(["board", EDITED]));
    let edited = version("outline-perimeter", EDITED);
    assert_eq!(edited["name"], "Edited outline 1");
    assert!(
        edited.get("ownership").is_none(),
        "a perimeter edit is a fixed copy"
    );
    assert_eq!(edited["geometry"]["features"].as_array().unwrap().len(), 1);
    assert_eq!(edited["geometry"]["features"][0]["kind"], "polygon");
    assert_eq!(
        edited["geometry"]["features"][0]["points"],
        json!([{ "x": -12.0, "y": -12.0 }, { "x": 12.0, "y": -12.0 }, { "x": 12.0, "y": 12.0 }, { "x": -12.0, "y": 12.0 }])
    );
    assert_eq!(
        edited["geometry"]["features"][0]["id"],
        "board:board:version:outline-version-12:contour:0"
    );
    assert_eq!(changed("outline-version-bridge"), json!(["board", EDITED]));
    assert_eq!(
        version("outline-version-bridge", EDITED)["geometry"]["settings"]["bridgeWidth"],
        json!(1.5),
        "a version keeps its own settings"
    );
    assert_eq!(
        settings("outline-version-bridge", GENERATED)["bridgeWidth"],
        json!(3.0),
        "the generated envelope keeps its own bridge width"
    );

    // An authored feature is added to and removed from the version.
    assert_eq!(
        changed("outline-version-feature-add"),
        json!(["board", "cut-a", EDITED])
    );
    assert_eq!(
        version("outline-version-feature-add", EDITED)["geometry"]["features"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        version("outline-version-feature-add", EDITED)["geometry"]["features"][1]["operation"],
        "subtract"
    );
    assert_eq!(
        changed("outline-version-feature-remove"),
        json!(["board", "cut-a", EDITED])
    );
    assert_eq!(
        version("outline-version-feature-remove", EDITED)["geometry"]["features"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(changed("outline-version-rename"), json!(["board", EDITED]));
    assert_eq!(version("outline-version-rename", EDITED)["name"], "Renamed");

    // Activation returns the board to Generated, then a linked refinement follows the
    // board's current component with its own settings.
    assert_eq!(changed("outline-activate-generated"), json!(["board"]));
    assert!(
        by_id("outline-activate-generated")["document"]["boardOutlines"][0]
            .get("activeVersionId")
            .is_none(),
        "Generated is active again"
    );
    assert_eq!(changed("outline-refine"), json!(["board", REFINED]));
    let refined = version("outline-refine", REFINED);
    assert_eq!(refined["ownership"], "linked");
    assert_eq!(refined["geometry"]["features"][0]["kind"], "part-envelope");
    assert_eq!(
        refined["geometry"]["features"][0]["partIds"],
        json!(["outline-part"]),
        "the refinement resolves from the board's current components"
    );
    assert_eq!(
        by_id("outline-refine")["document"]["boardOutlines"][0]["activeVersionId"],
        REFINED
    );
    assert_eq!(changed("outline-refine-margin"), json!(["board", REFINED]));
    assert_eq!(
        version("outline-refine-margin", REFINED)["geometry"]["features"][0]["margin"],
        json!(7.0),
        "the refinement owns its margin"
    );
    assert_eq!(
        by_id("outline-refine-margin")["document"]["outline"][0]["margin"],
        json!(6.0),
        "the generated envelope's margin is unchanged"
    );

    // Freeze keeps the link and adds a fixed copy from the resolved projection.
    assert_eq!(
        changed("outline-freeze"),
        json!(["board", "outline-version-19"])
    );
    assert_eq!(version("outline-freeze", REFINED)["ownership"], "linked");
    let frozen = version("outline-freeze", "outline-version-19");
    assert_eq!(frozen["name"], "Frozen");
    assert!(
        frozen.get("ownership").is_none(),
        "freeze writes a fixed copy"
    );
    assert_eq!(frozen["source"]["versionId"], REFINED);
    assert_eq!(
        by_id("outline-freeze")["document"]["boardOutlines"][0]["activeVersionId"],
        "outline-version-19"
    );
    assert_eq!(
        changed("outline-activate-refined"),
        json!(["board", REFINED])
    );

    // Editing a linked refinement's perimeter retains the link and adds a fixed copy.
    assert_eq!(
        changed("outline-refine-perimeter"),
        json!(["board", "outline-version-21"])
    );
    assert_eq!(version_ids("outline-refine-perimeter").len(), 4);
    assert_eq!(
        version("outline-refine-perimeter", REFINED)["ownership"],
        "linked"
    );
    assert_eq!(
        version("outline-refine-perimeter", "outline-version-21")["name"],
        "Edited outline 1"
    );
    assert_eq!(
        version("outline-refine-perimeter", "outline-version-21")["geometry"]["features"][0]["points"],
        json!([{ "x": -13.0, "y": -13.0 }, { "x": 13.0, "y": -13.0 }, { "x": 13.0, "y": 13.0 }, { "x": -13.0, "y": 13.0 }])
    );

    // A copy records its origin and stays fixed; deleting records the retired
    // identity, and one Undo restores the version and the selection together.
    assert_eq!(changed("outline-copy"), json!(["board", "v-copy"]));
    assert_eq!(
        version("outline-copy", "v-copy")["source"]["versionId"],
        "outline-version-21",
        "the copy records the version that was active when it was taken"
    );
    assert!(version("outline-copy", "v-copy").get("ownership").is_none());
    assert_eq!(changed("outline-delete"), json!(["board", "v-copy"]));
    assert_eq!(
        by_id("outline-delete")["document"]["boardOutlines"][0]["retiredVersionIds"],
        json!(["v-copy"])
    );
    assert!(
        by_id("outline-delete")["document"]["boardOutlines"][0]
            .get("activeVersionId")
            .is_none()
    );
    assert_eq!(
        version_ids("outline-undo"),
        vec![
            EDITED.to_owned(),
            REFINED.to_owned(),
            "outline-version-19".to_owned(),
            "outline-version-21".to_owned(),
            "v-copy".to_owned(),
        ]
    );
    assert!(
        by_id("outline-undo")["document"]["boardOutlines"][0]
            .get("retiredVersionIds")
            .is_none(),
        "one Undo clears the reservation the delete recorded"
    );
    assert_eq!(
        by_id("outline-undo")["document"]["boardOutlines"][0]["activeVersionId"],
        "v-copy"
    );
    assert_eq!(changed("outline-rename-repeat"), json!([]));
    assert_eq!(
        revision("outline-rename-repeat"),
        revision("outline-undo"),
        "a rename to the accepted name adds no revision step"
    );

    // The final document shape: the generated envelope, the version order and each
    // version's persisted ownership and name.
    let snapshot = &by_id("outline-snapshot")["document"];
    assert_eq!(snapshot["boards"][0]["outlineIds"], json!([GENERATED]));
    assert_eq!(snapshot["boards"][0]["partIds"], json!(["outline-part"]));
    assert_eq!(version_ids("outline-snapshot").len(), 5);
    let names: Vec<&str> = snapshot["boardOutlines"][0]["versions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|version| version["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        vec!["Renamed", "Refined", "Frozen", "Edited outline 1", "Copy"]
    );
    let ownership: Vec<Option<&str>> = snapshot["boardOutlines"][0]["versions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|version| version.get("ownership").and_then(Value::as_str))
        .collect();
    assert_eq!(
        ownership,
        vec![None, Some("linked"), None, None, None],
        "only the refinement persists a link"
    );
    assert_eq!(snapshot["boardOutlines"][0]["activeVersionId"], "v-copy");
}

fn compact(replies: &mut [Value]) {
    // The 500-cell draft is only recorded by size to keep the golden file small.
    let cells = replies[10]["matrixScenes"][0]["cells"]
        .as_array()
        .unwrap()
        .len();
    replies[10] = json!({ "kind": replies[10]["kind"], "cells": cells });
}

/// Runs every request and archive case through the transports, asserting the
/// boundary's behaviour, and returns the transcript recorded as the golden.
pub fn run(core: CoreTransport, archive: ArchiveTransport) -> Value {
    let mut replies: Vec<Value> = core_requests()
        .iter()
        .map(|request| {
            serde_json::from_str(&core(&request.to_string())).expect("core reply is JSON")
        })
        .collect();
    check_core(&replies);
    check_outline(&replies);
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

    let (reply, packed) = call(
        json!({ "kind": "pack-project", "projectJson": project_json, "archiveJson": json!({ "embedUsedModels": true }).to_string(), "assets": [{ "path": asset_path, "bufferIndex": 0 }] }),
        std::slice::from_ref(&asset),
    );
    assert_eq!(reply["kind"], "packed");
    let (reply, unpacked) = call(json!({ "kind": "unpack-project" }), &packed);
    assert_eq!(reply["projectJson"], project_json);
    assert_eq!(unpacked[0], asset);

    // Archives from another producer: an old project without archive.json,
    // wrong asset bytes, and a truncated archive.
    let (_, old) = call(
        json!({ "kind": "pack-files", "entries": [{ "path": "project.json", "bufferIndex": 0 }, { "path": asset_path, "bufferIndex": 1 }] }),
        &[project_json.clone().into_bytes(), asset.clone()],
    );
    let (reply, _) = call(json!({ "kind": "unpack-project" }), &old);
    assert_eq!(reply["projectJson"], project_json);
    let (_, wrong) = call(
        json!({ "kind": "pack-files", "entries": [{ "path": "project.json", "bufferIndex": 0 }, { "path": asset_path, "bufferIndex": 1 }] }),
        &[project_json.clone().into_bytes(), b"wrong".to_vec()],
    );
    assert_eq!(
        call(json!({ "kind": "unpack-project" }), &wrong).0["kind"],
        "error"
    );
    let truncated = vec![old[0][..old[0].len() - 12].to_vec()];
    assert_eq!(
        call(json!({ "kind": "unpack-project" }), &truncated).0["kind"],
        "error"
    );

    let (reply, generic) = call(
        json!({ "kind": "pack-files", "entries": [{ "path": "BoardStudio.pretty/part.kicad_mod", "bufferIndex": 0 }, { "path": "models/component.step", "bufferIndex": 1 }] }),
        &[b"(footprint \"part\")".to_vec(), asset.clone()],
    );
    assert_eq!(reply["kind"], "packed");
    assert_eq!(generic.len(), 1);
    let duplicate = call(
        json!({ "kind": "pack-files", "entries": [{ "path": "same", "bufferIndex": 0 }, { "path": "same", "bufferIndex": 1 }] }),
        &[asset.clone(), asset.clone()],
    );
    assert_eq!(duplicate.0["kind"], "error");

    let payload = payload();
    let mut doc = document();
    doc["assets"] = json!([{ "id": "memory-fixture", "sha256": PAYLOAD_SHA256 }]);
    let large = json!({ "kind": "pack-project", "projectJson": doc.to_string(), "assets": [{ "path": format!("assets/{PAYLOAD_SHA256}"), "bufferIndex": 0 }] });
    let (reply, measured) = call(large, &[payload]);
    assert_eq!(reply["kind"], "packed");
    assert_eq!(
        call(json!({ "kind": "unpack-project" }), &measured).0["kind"],
        "unpacked"
    );

    json!({ "core": replies, "archive": transcript })
}

pub fn assert_golden(actual: &Value, golden: &Value) {
    equivalent(actual, golden, "transcript");
}

//! Requests, semantic assertions and golden comparison for the native/WASM
//! boundary parity check. The native test (`core/tests/boundary_parity.rs`)
//! records and verifies the golden transcript; the browser test
//! (`web/crates/host/src/boundary_parity.rs`) replays the same requests through the real
//! `CoreEngine` and `archive_request` WASM exports and must reproduce it.
//! Ported from `scripts/check-rust-boundaries.mjs`.
#![allow(dead_code)]

use crate::support::{replacement_electrical, synthetic_half};
use boardstudio_core::model::ProjectDoc;
use serde_json::{Value, json};

/// SHA-256 of [`ASSET`]; the native test recomputes it to guard the constant.
#[path = "boundary/cases.rs"]
mod cases;
#[path = "boundary/scenarios.rs"]
mod scenarios;
#[path = "boundary/transcript.rs"]
mod transcript;

pub use self::cases::{core_requests, mirror_addition_boundary_case, paired_preview_history_case};
pub use self::scenarios::electrical_mirror_replacement_case;
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
fn supported_synthetic_half() -> Value {
    let mut source = synthetic_half::document();
    for cell in source["matrices"][0]["cells"].as_array_mut().unwrap() {
        let object = cell.as_object_mut().unwrap();
        object.remove("assembliesLocal");
        object.remove("definitionId");
    }
    source
}

fn boundary_exception(pair_id: &str, mirror_id: &str, state: &str) -> Value {
    json!({
        "kind":"set-half-exception",
        "pairId":pair_id,
        "itemKind":"part",
        "mirrorId":mirror_id,
        "state":state
    })
}

fn electrical_notice_id(kind: &str, identities: &[&str]) -> String {
    let mut id = format!("preview:electrical:{kind}");
    for identity in identities {
        id.push_str(&format!(":{}-", identity.len()));
        for byte in identity.as_bytes() {
            id.push_str(&format!("{byte:02x}"));
        }
    }
    id
}

fn replacement_catalogue_definition(source: &str) -> boardstudio_core::model::PartDefinition {
    boardstudio_core::generators::catalogue()
        .expect("the bundled hardware catalogue is available")
        .into_iter()
        .find(|definition| {
            definition
                .generator
                .as_ref()
                .is_some_and(|generator| generator.source == source)
        })
        .unwrap_or_else(|| panic!("bundled definition {source} exists"))
}

fn replacement_core_call(core: &mut dyn FnMut(&str) -> String, request: Value) -> Value {
    serde_json::from_str(&core(&request.to_string())).expect("Core replies with JSON")
}

fn replacement_edit(
    transaction_id: &str,
    base_revision: &Value,
    phase: &str,
    operation: Value,
) -> Value {
    json!({
        "id":transaction_id,"kind":"edit",
        "command":{"baseRevision":base_revision,"transactionId":transaction_id,
            "phase":phase,"targetIds":[],"operation":operation}
    })
}

fn replacement_pair_fixture(
    core: &mut dyn FnMut(&str) -> String,
    id_prefix: &str,
) -> (Value, String, String, String) {
    let mut source = replacement_electrical::supported_source_document();
    let nice_view = replacement_catalogue_definition("ceoloide/display_nice_view");
    let ssd1306 = replacement_catalogue_definition("ceoloide/display_ssd1306");
    let nice_view_id = nice_view.id.clone();
    let ssd1306_id = ssd1306.id.clone();
    source["definitions"]
        .as_array_mut()
        .expect("fixture definitions")
        .push(serde_json::to_value(nice_view).expect("serialize Nice View definition"));
    source["definitions"]
        .as_array_mut()
        .expect("fixture definitions")
        .push(serde_json::to_value(ssd1306).expect("serialize SSD1306 definition"));

    let opened = replacement_core_call(
        core,
        json!({"id":format!("{id_prefix}-open"),
        "kind":"open","document":source}),
    );
    assert_eq!(opened["kind"], "scene", "{opened}");
    let pair_id = format!("{id_prefix}-pair");
    let paired = replacement_core_call(
        core,
        replacement_edit(
            &format!("{id_prefix}-create-pair"),
            &opened["document"]["revision"],
            "commit",
            synthetic_half::create_mirrored_half_operation(&pair_id, "right"),
        ),
    );
    assert_eq!(paired["kind"], "scene", "{paired}");

    // A source-linked SPI display gives the Rejoin a distinct CS function. Its
    // shared SPI bus functions stay locked while omitted; only its per-Part CS
    // pin is available for the mirror-only I2C display to consume.
    let added = replacement_core_call(
        core,
        replacement_edit(
            &format!("{id_prefix}-add-source-spi"),
            &paired["document"]["revision"],
            "commit",
            json!({"kind":"add-part","boardId":"left","layoutId":null,
                "part":{"id":"left/NICEVIEW","definitionId":nice_view_id,
                    "reference":"OLED1","pose":{"at":{"x":-42.0,"y":-22.0},
                        "rotation":0.0},"side":"front"}}),
        ),
    );
    assert_eq!(added["kind"], "scene", "{added}");

    let resolved = replacement_core_call(
        core,
        json!({"id":format!("{id_prefix}-resolve-source"),"kind":"resolve-electrical",
            "request":{"document":added["document"],"boardId":"left","instanceId":"left",
                "mode":"matrix","locks":{},"controllerPartId":"left/U1",
                "controllerProfile":"ceoloide/mcu_nice_nano"}}),
    );
    assert_eq!(resolved["kind"], "electrical-resolved", "{resolved}");
    assert!(
        resolved["plan"]["diagnostics"]
            .as_array()
            .expect("resolver diagnostics")
            .iter()
            .all(|diagnostic| diagnostic["severity"] != "error"),
        "the bundled Nice Nano source fixture must be fully resolvable: {resolved}"
    );
    let applied = replacement_core_call(
        core,
        json!({"id":format!("{id_prefix}-apply-source"),"kind":"apply-electrical",
            "baseRevision":added["document"]["revision"],"plan":resolved["plan"],
            "draft":false}),
    );
    assert_eq!(applied["kind"], "scene", "{applied}");
    (
        applied["document"].clone(),
        pair_id,
        nice_view_id,
        ssd1306_id,
    )
}

fn replacement_board_config(document: &Value, board_id: &str) -> Value {
    document["hardware"]["boards"]
        .as_array()
        .expect("electrical board configurations")
        .iter()
        .find(|configuration| configuration["boardId"] == board_id)
        .unwrap_or_else(|| panic!("electrical config for {board_id} exists"))
        .clone()
}

fn replacement_generated_electrical_nets(document: &Value, board_id: &str) -> Vec<Value> {
    let prefix = format!("generated/electrical/{board_id}/");
    document["nets"]
        .as_array()
        .expect("nets")
        .iter()
        .filter(|net| net["id"].as_str().is_some_and(|id| id.starts_with(&prefix)))
        .cloned()
        .collect()
}

fn replacement_generator_parameters(document: &Value, part_id: &str) -> Value {
    document["parts"]
        .as_array()
        .expect("parts")
        .iter()
        .find(|part| part["id"] == part_id)
        .unwrap_or_else(|| panic!("controller Part {part_id} exists"))["generatorParameters"]
        .clone()
}

fn replacement_source_projection(document: &Value) -> Value {
    let board = document["boards"]
        .as_array()
        .expect("boards")
        .iter()
        .find(|board| board["id"] == "left")
        .expect("source board");
    let net_ids = board["netIds"].as_array().expect("source net membership");
    json!({
        "board":board,
        "electrical":replacement_board_config(document,"left"),
        "parts":document["parts"].as_array().unwrap().iter()
            .filter(|part| board["partIds"].as_array().unwrap().contains(&part["id"]))
            .cloned().collect::<Vec<_>>(),
        "nets":document["nets"].as_array().unwrap().iter()
            .filter(|net| net_ids.contains(&net["id"])).cloned().collect::<Vec<_>>()
    })
}

fn mirror_only_display_operation(board_id: &str, part_id: &str, definition_id: &str) -> Value {
    json!({"kind":"add-mirror-only-part","boardId":board_id,"layoutId":null,
        "definition":null,
        "part":{"id":part_id,"definitionId":definition_id,"reference":"OLED2",
            "pose":{"at":{"x":-40.0,"y":-12.0},"rotation":0.0},"side":"front"}})
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

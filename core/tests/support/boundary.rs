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

/// Exercise bilateral AddPart from each half and explicit mirror-only placement
/// through the transport shared by native Core and the WASM host.
pub fn mirror_addition_boundary_case(
    mut core: CoreTransport<'_>,
    archive: ArchiveTransport<'_>,
) -> Value {
    let call = |core: &mut CoreTransport<'_>, request: Value| {
        serde_json::from_str::<Value>(&core(&request.to_string())).expect("core reply is JSON")
    };
    let edit = |id: &str, base_revision: u64, phase: &str, operation: Value| {
        json!({
            "id":id,"kind":"edit",
            "command":{"baseRevision":base_revision,"transactionId":id,"phase":phase,
                "targetIds":[],"operation":operation}
        })
    };
    let source = supported_synthetic_half();
    let opened = call(
        &mut core,
        json!({"id":"placement-open","kind":"open","document":source}),
    );
    assert_eq!(opened["kind"], "scene", "{opened}");
    let paired = call(
        &mut core,
        edit(
            "placement-pair",
            opened["document"]["revision"].as_u64().unwrap(),
            "commit",
            synthetic_half::create_mirrored_half_operation("boundary-placement-pair", "right"),
        ),
    );
    assert_eq!(paired["kind"], "scene", "{paired}");
    let revision = || paired["document"]["revision"].as_u64().unwrap();

    let source_add = call(
        &mut core,
        edit(
            "placement-source-add",
            revision(),
            "commit",
            json!({"kind":"add-part","boardId":"left","layoutId":"left-synthetic-keys-layout",
                "definition":{"id":"boundary/placement-switch","name":"Boundary placement switch","kind":"switch",
                    "pads":[],"courtyard":[{"x":-3,"y":-3},{"x":3,"y":-3},{"x":3,"y":3},{"x":-3,"y":3}]},
                "part":{"id":"left/BOUNDARY-SOURCE","definitionId":"boundary/placement-switch",
                    "reference":"SRC1","pose":{"at":{"x":8,"y":12},"rotation":0},"side":"front"}}),
        ),
    );
    assert_eq!(source_add["kind"], "scene", "{source_add}");
    let source_correspondences =
        source_add["document"]["hardware"]["pairs"][0]["halves"]["correspondences"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| {
                item["kind"] == "part"
                    && item["sourceId"] == "left/BOUNDARY-SOURCE"
                    && item["mirrorId"] == "right/BOUNDARY-SOURCE"
            })
            .count();
    assert_eq!(
        source_correspondences, 1,
        "source-side AddPart must create one correspondence"
    );
    let source_linked_parts = source_add["document"]["parts"].as_array().unwrap();
    for id in ["left/BOUNDARY-SOURCE", "right/BOUNDARY-SOURCE"] {
        assert_eq!(
            source_linked_parts
                .iter()
                .filter(|part| part["id"] == id)
                .count(),
            1,
            "source-side AddPart must publish both linked Parts"
        );
    }
    for (layout_id, part_id) in [
        ("left-synthetic-keys-layout", "left/BOUNDARY-SOURCE"),
        ("right-synthetic-keys-layout", "right/BOUNDARY-SOURCE"),
    ] {
        assert!(
            source_add["document"]["layouts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|layout| layout["id"] == layout_id)
                .unwrap()["partIds"]
                .as_array()
                .unwrap()
                .iter()
                .any(|id| id == part_id),
            "linked AddPart omitted {part_id} from {layout_id}"
        );
    }
    let mirror_add = call(
        &mut core,
        edit(
            "placement-mirror-add",
            source_add["document"]["revision"].as_u64().unwrap(),
            "commit",
            json!({"kind":"add-part","boardId":"right","layoutId":"right-synthetic-keys-layout",
                "part":{"id":"right/BOUNDARY-MIRROR","definitionId":"boundary/placement-switch",
                    "reference":"MIR1","pose":{"at":{"x":218,"y":12},"rotation":0},"side":"front"}}),
        ),
    );
    assert_eq!(mirror_add["kind"], "scene", "{mirror_add}");
    let mirror_correspondences =
        mirror_add["document"]["hardware"]["pairs"][0]["halves"]["correspondences"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| {
                item["kind"] == "part"
                    && item["sourceId"] == "left/BOUNDARY-MIRROR"
                    && item["mirrorId"] == "right/BOUNDARY-MIRROR"
            })
            .count();
    assert_eq!(
        mirror_correspondences, 1,
        "mirror-side AddPart must create one correspondence"
    );
    for id in ["left/BOUNDARY-MIRROR", "right/BOUNDARY-MIRROR"] {
        assert_eq!(
            mirror_add["document"]["parts"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|part| part["id"] == id)
                .count(),
            1,
            "mirror-side AddPart must publish both linked Parts"
        );
    }
    for (layout_id, part_id) in [
        ("left-synthetic-keys-layout", "left/BOUNDARY-MIRROR"),
        ("right-synthetic-keys-layout", "right/BOUNDARY-MIRROR"),
    ] {
        assert!(
            mirror_add["document"]["layouts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|layout| layout["id"] == layout_id)
                .unwrap()["partIds"]
                .as_array()
                .unwrap()
                .iter()
                .any(|id| id == part_id),
            "linked AddPart omitted {part_id} from {layout_id}"
        );
    }

    let before = mirror_add["document"].clone();
    let halves = &before["hardware"]["pairs"][0]["halves"];
    let source_board_id = halves["sourceBoardId"].as_str().unwrap();
    let mirror_board_id = halves["mirrorBoardId"].as_str().unwrap();
    let part_id = "right/BOUNDARY-ONLY";
    let only_operation = json!({"kind":"add-mirror-only-part","boardId":mirror_board_id,
        "layoutId":"right-synthetic-keys-layout",
        "definition":{"id":"boundary/mirror-only-switch","name":"Boundary mirror-only switch","kind":"switch",
            "pads":[],"courtyard":[{"x":-3,"y":-3},{"x":3,"y":-3},{"x":3,"y":3},{"x":-3,"y":3}]},
        "part":{"id":part_id,"definitionId":"boundary/mirror-only-switch","reference":"ONLY1",
            "pose":{"at":{"x":232,"y":24},"rotation":0},"side":"front"}});
    let preview = call(
        &mut core,
        edit(
            "placement-only-preview",
            before["revision"].as_u64().unwrap(),
            "preview",
            only_operation.clone(),
        ),
    );
    assert_eq!(preview["kind"], "preview", "{preview}");
    let after_preview = call(
        &mut core,
        json!({"id":"placement-after-preview","kind":"snapshot"}),
    );
    assert_eq!(
        after_preview["document"], before,
        "mirror-only Preview must not publish"
    );

    let committed = call(
        &mut core,
        edit(
            "placement-only-commit",
            before["revision"].as_u64().unwrap(),
            "commit",
            only_operation,
        ),
    );
    assert_eq!(committed["kind"], "scene", "{committed}");
    let after = &committed["document"];
    let source_projection = |document: &Value| {
        let source_board = document["boards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|board| board["id"] == source_board_id)
            .unwrap();
        let source_ids = source_board["partIds"].as_array().unwrap();
        let parts: Vec<_> = document["parts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|part| source_ids.contains(&part["id"]))
            .cloned()
            .collect();
        let layouts: Vec<_> = document["layouts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|layout| layout["boardId"] == source_board_id)
            .cloned()
            .collect();
        let source_outlines: Vec<_> = document["outline"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|feature| {
                source_board["outlineIds"]
                    .as_array()
                    .unwrap()
                    .contains(&feature["id"])
            })
            .cloned()
            .collect();
        json!({"board":source_board,"parts":parts,"layouts":layouts,"outlines":source_outlines})
    };
    let source_unchanged = source_projection(&before) == source_projection(after);
    assert!(
        source_unchanged,
        "mirror-only placement changed source-half data"
    );
    assert_eq!(
        after["hardware"]["pairs"][0], before["hardware"]["pairs"][0],
        "mirror-only placement must not create correspondence or exceptions"
    );
    let only_parts: Vec<_> = after["parts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|part| part["id"] == part_id)
        .collect();
    assert_eq!(
        only_parts.len(),
        1,
        "mirror-only Part must be stored exactly once"
    );
    assert_eq!(
        after["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|definition| definition["id"] == "boundary/mirror-only-switch")
            .count(),
        1,
        "mirror-only definition must be registered atomically"
    );
    let mirror_board = after["boards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|board| board["id"] == mirror_board_id)
        .unwrap();
    assert_eq!(
        mirror_board["partIds"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|id| *id == part_id)
            .count(),
        1
    );
    let target_layout = after["layouts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|layout| layout["id"] == "right-synthetic-keys-layout")
        .unwrap();
    assert_eq!(
        target_layout["partIds"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|id| *id == part_id)
            .count(),
        1
    );
    let target_envelopes: Vec<_> = after["outline"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|feature| {
            mirror_board["outlineIds"]
                .as_array()
                .unwrap()
                .contains(&feature["id"])
        })
        .filter(|feature| feature["kind"] == "part-envelope")
        .collect();
    assert!(
        target_envelopes.iter().any(|feature| {
            feature["partIds"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|id| *id == part_id)
                .count()
                == 1
        }),
        "mirror-only Part must join the mirror board's existing PartEnvelope membership"
    );
    assert!(
        after["hardware"]["pairs"][0]["halves"]["correspondences"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["sourceId"] != part_id && item["mirrorId"] != part_id)
    );

    let undone = call(&mut core, json!({"id":"placement-only-undo","kind":"undo"}));
    assert_eq!(undone["kind"], "scene", "{undone}");
    assert!(
        undone["document"]["parts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|part| part["id"] != part_id),
        "one Undo must remove the mirror-only addition"
    );
    for id in [
        "left/BOUNDARY-SOURCE",
        "right/BOUNDARY-SOURCE",
        "left/BOUNDARY-MIRROR",
        "right/BOUNDARY-MIRROR",
    ] {
        assert!(
            undone["document"]["parts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|part| part["id"] == id),
            "one Undo must preserve prior linked addition {id}"
        );
    }
    assert_eq!(
        undone["document"]["hardware"]["pairs"][0], before["hardware"]["pairs"][0],
        "Undo of mirror-only placement must preserve linked provenance"
    );
    assert!(
        undone["document"]["boards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|board| board["id"] == mirror_board_id)
            .unwrap()["partIds"]
            .as_array()
            .unwrap()
            .iter()
            .all(|id| id != part_id)
    );
    assert!(
        undone["document"]["layouts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|layout| layout["id"] == "right-synthetic-keys-layout")
            .unwrap()["partIds"]
            .as_array()
            .unwrap()
            .iter()
            .all(|id| id != part_id)
    );
    assert_eq!(
        source_projection(&undone["document"]),
        source_projection(&before)
    );
    let redone = call(&mut core, json!({"id":"placement-only-redo","kind":"redo"}));
    assert_eq!(redone["kind"], "scene", "{redone}");
    assert_eq!(
        redone["document"]["parts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|part| part["id"] == part_id)
            .count(),
        1
    );
    let reopened = call(
        &mut core,
        json!({"id":"placement-only-reopen","kind":"open","document":redone["document"]}),
    );
    assert_eq!(reopened["kind"], "scene", "{reopened}");
    let reopened_doc = &reopened["document"];
    assert_eq!(
        reopened_doc["parts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|part| part["id"] == part_id)
            .count(),
        1
    );
    assert_eq!(
        reopened_doc["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|definition| definition["id"] == "boundary/mirror-only-switch")
            .count(),
        1
    );
    assert!(source_projection(reopened_doc) == source_projection(&before));
    assert_eq!(
        reopened_doc["hardware"]["pairs"][0],
        before["hardware"]["pairs"][0]
    );
    let reopened_mirror_board = reopened_doc["boards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|board| board["id"] == mirror_board_id)
        .unwrap();
    assert_eq!(
        reopened_mirror_board["partIds"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|id| *id == part_id)
            .count(),
        1
    );
    assert!(
        reopened_doc["outline"]
            .as_array()
            .unwrap()
            .iter()
            .any(|feature| {
                reopened_mirror_board["outlineIds"]
                    .as_array()
                    .unwrap()
                    .contains(&feature["id"])
                    && feature["kind"] == "part-envelope"
                    && feature["partIds"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|id| *id == part_id)
                        .count()
                        == 1
            })
    );
    assert_eq!(
        reopened_doc["layouts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|layout| layout["id"] == "right-synthetic-keys-layout")
            .unwrap()["partIds"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|id| *id == part_id)
            .count(),
        1
    );

    let saved_project_json = redone["document"].to_string();
    let (saved, saved_buffers) = archive(
        &json!({
            "kind":"pack-project",
            "projectJson":saved_project_json,
            "archiveJson":json!({"embedUsedModels":false}).to_string(),
            "assets":[]
        }),
        &[],
    );
    assert_eq!(saved["kind"], "packed", "{saved}");
    assert_eq!(
        saved_buffers.len(),
        1,
        "Save should produce one project archive"
    );
    let (opened_archive, unpacked_buffers) =
        archive(&json!({"kind":"unpack-project"}), &saved_buffers);
    assert_eq!(opened_archive["kind"], "unpacked", "{opened_archive}");
    assert!(unpacked_buffers.is_empty());
    let saved_document: Value =
        serde_json::from_str(opened_archive["projectJson"].as_str().unwrap()).unwrap();
    assert_eq!(
        saved_document, redone["document"],
        "archive roundtrip must retain the placed document"
    );
    let archive_open = call(
        &mut core,
        json!({"id":"placement-archive-open","kind":"open","document":saved_document}),
    );
    assert_eq!(archive_open["kind"], "scene", "{archive_open}");
    let archive_open_doc = &archive_open["document"];
    assert_eq!(
        archive_open_doc["parts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|part| part["id"] == part_id)
            .count(),
        1
    );
    assert_eq!(
        archive_open_doc["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|definition| definition["id"] == "boundary/mirror-only-switch")
            .count(),
        1
    );
    assert_eq!(
        archive_open_doc["hardware"]["pairs"][0],
        before["hardware"]["pairs"][0]
    );
    assert_eq!(
        source_projection(archive_open_doc),
        source_projection(&before)
    );
    assert_eq!(
        archive_open_doc["boards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|board| board["id"] == mirror_board_id)
            .unwrap()["partIds"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|id| *id == part_id)
            .count(),
        1
    );
    assert_eq!(
        archive_open_doc["layouts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|layout| layout["id"] == "right-synthetic-keys-layout")
            .unwrap()["partIds"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|id| *id == part_id)
            .count(),
        1
    );
    let archive_mirror_board = archive_open_doc["boards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|board| board["id"] == mirror_board_id)
        .unwrap();
    assert!(
        archive_open_doc["outline"]
            .as_array()
            .unwrap()
            .iter()
            .any(|feature| {
                archive_mirror_board["outlineIds"]
                    .as_array()
                    .unwrap()
                    .contains(&feature["id"])
                    && feature["kind"] == "part-envelope"
                    && feature["partIds"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|id| *id == part_id)
                        .count()
                        == 1
            })
    );

    json!({
        "linkedAdditions": {
            "sourceSideCorrespondence": source_add["document"]["hardware"]["pairs"][0]["halves"]["correspondences"].as_array().unwrap().iter()
                .any(|item| item["kind"] == "part" && item["sourceId"] == "left/BOUNDARY-SOURCE"),
            "mirrorSideCorrespondence": mirror_add["document"]["hardware"]["pairs"][0]["halves"]["correspondences"].as_array().unwrap().iter()
                .any(|item| item["kind"] == "part" && item["sourceId"] == "left/BOUNDARY-MIRROR" && item["mirrorId"] == "right/BOUNDARY-MIRROR"),
            "sourceAndMirrorLayoutsRegistered": true
        },
        "mirrorOnly": {
            "previewNonpublishing": true,
            "singleMirrorPart": true,
            "sourceUnchanged": source_unchanged,
            "noPairProvenance": true,
            "mirrorBoardLayoutEnvelopeMembership": true,
            "undoRemovedAndRedoRestored": true,
            "openRetained": true,
            "saveOpenRetained": true
        },
        "archiveRoundTrip": {
            "save": {
                "kind": saved["kind"],
                "buffers": saved_buffers.iter().map(|bytes| digest(bytes)).collect::<Vec<_>>()
            },
            "open": {
                "kind": opened_archive["kind"],
                "projectJson": digest(opened_archive["projectJson"].as_str().unwrap().as_bytes()),
                "buffers": unpacked_buffers.iter().map(|bytes| digest(bytes)).collect::<Vec<_>>()
            }
        }
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
    // revisions. Column 0 has zero Splay, Column rotation and carried Bend, as
    // spec §6 requires for conversion; column 2 stays splayed for the refusal.
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
        json!({ "kind": "column-offset", "column": 0, "value": { "x": 0, "y": 3 } }),
    ));
    requests.push(placement("convert-preview", 43, "preview", convert(0)));
    requests.push(placement("convert-commit", 43, "commit", convert(0)));
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

/// Runs the paired Preview → Undo → Redo case through the shared Core JSON transport.
/// Its compact projection is included in the transcript returned by `run`.
pub fn paired_preview_history_case(mut core: CoreTransport<'_>) -> Value {
    let call = |core: &mut CoreTransport<'_>, request: Value| {
        serde_json::from_str::<Value>(&core(&request.to_string())).expect("core reply is JSON")
    };
    let source = supported_synthetic_half();
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
                "operation":synthetic_half::create_mirrored_half_operation(
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
        preview["scene"]["transforms"]
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
        "open":{"kind":open["kind"]},
        "created":{"kind":created["kind"],"pairCount":created["document"]["hardware"]["pairs"].as_array().unwrap().len()},
        "beforePreview":{"kind":before_preview["kind"]},
        "preview":{
            "kind":preview["kind"],
            "partX":{"left/U1":preview_part_x("left/U1"),"right/U1":preview_part_x("right/U1")},
            "changedIds":["left/U1","right/U1"]
        },
        "afterPreview":{"kind":after_preview["kind"],"documentUnchanged":true},
        "undo":{"kind":undo["kind"],"pairCount":undo["document"]["hardware"]["pairs"].as_array().map_or(0, Vec::len)},
        "redo":{"kind":redo["kind"],"pairCount":redo["document"]["hardware"]["pairs"].as_array().unwrap().len()}
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

/// Exercises automatic pin assignment through the exact Core edit and archive
/// transports shared by native tests and the browser/WASM transcript replay.
pub fn electrical_mirror_replacement_case(
    core: CoreTransport<'_>,
    archive: ArchiveTransport<'_>,
) -> Value {
    let (mut accepted, pair_id, _nice_view_id, ssd1306_id) =
        replacement_pair_fixture(&mut *core, "boundary-electrical-replacement");
    let pair_before = accepted["hardware"]["pairs"][0].clone();
    let source_before = replacement_source_projection(&accepted);
    let source_assignments = replacement_board_config(&accepted, "left")["assignments"].clone();
    let mirror_config_before = replacement_board_config(&accepted, "right");
    let freed_cs = mirror_config_before["assignments"]["peripheral/right/NICEVIEW/CS"]
        .as_str()
        .expect("the source-linked Nice View has a mapped mirror CS pin")
        .to_owned();
    for function in ["spi/MOSI", "spi/SCK"] {
        assert_eq!(
            mirror_config_before["assignments"][function], source_assignments[function],
            "mapped bus function {function} starts on the source pin"
        );
    }

    let omitted = replacement_core_call(
        &mut *core,
        replacement_edit(
            "boundary-electrical-replacement-omit",
            &accepted["revision"],
            "commit",
            boundary_exception(&pair_id, "right/NICEVIEW", "omitted"),
        ),
    );
    assert_eq!(omitted["kind"], "scene", "{omitted}");
    accepted = omitted["document"].clone();
    let pair_after_omit = accepted["hardware"]["pairs"][0].clone();
    let omitted_config = replacement_board_config(&accepted, "right");
    assert_ne!(
        omitted_config["assignments"]["peripheral/right/NICEVIEW/CS"], freed_cs,
        "Omit releases the source-linked display's mirror CS function"
    );
    for function in ["spi/MOSI", "spi/SCK"] {
        assert!(
            omitted_config["assignments"][function].is_null(),
            "the last omitted SPI consumer releases inactive bus assignment {function}"
        );
        assert!(
            omitted_config["locks"][function].is_null(),
            "the last omitted SPI consumer releases derived inactive bus lock {function}"
        );
    }
    assert_eq!(replacement_source_projection(&accepted), source_before);

    let mirror_id = "right/BOUNDARY-OLED";
    let operation = mirror_only_display_operation("right", mirror_id, &ssd1306_id);
    let preview = replacement_core_call(
        &mut *core,
        replacement_edit(
            "boundary-electrical-replacement-preview",
            &accepted["revision"],
            "preview",
            operation.clone(),
        ),
    );
    assert_eq!(preview["kind"], "preview", "{preview}");
    assert_eq!(
        replacement_core_call(
            &mut *core,
            json!({"id":"boundary-electrical-replacement-after-preview",
        "kind":"snapshot"})
        )["document"],
        accepted,
        "placement Preview must not publish Part or electrical assignments"
    );
    let placed = replacement_core_call(
        &mut *core,
        replacement_edit(
            "boundary-electrical-replacement-place",
            &accepted["revision"],
            "commit",
            operation,
        ),
    );
    assert_eq!(placed["kind"], "scene", "{placed}");
    accepted = placed["document"].clone();
    assert_eq!(replacement_source_projection(&accepted), source_before);
    assert_eq!(
        accepted["hardware"]["pairs"][0], pair_after_omit,
        "explicit placement creates no pair provenance"
    );
    assert_eq!(
        accepted["parts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|part| part["id"] == mirror_id)
            .count(),
        1
    );
    let placed_config = replacement_board_config(&accepted, "right");
    let sda = placed_config["assignments"]["i2c/SDA"]
        .as_str()
        .expect("the mirror SSD1306 receives SDA");
    let scl = placed_config["assignments"]["i2c/SCL"]
        .as_str()
        .expect("the mirror SSD1306 receives SCL");
    assert_ne!(sda, scl, "SDA and SCL use distinct pins");
    assert!(
        [sda, scl].contains(&freed_cs.as_str()),
        "the pin released from per-Part CS is a valid I2C allocation"
    );
    assert_eq!(
        placed_config["locks"], omitted_config["locks"],
        "placement preserves the surviving mapped/user lock set"
    );
    for function in ["spi/MOSI", "spi/SCK"] {
        assert!(
            placed_config["locks"][function].is_null(),
            "inactive derived bus lock {function} remains absent without an explicit source lock"
        );
        assert!(
            placed_config["assignments"][function].is_null(),
            "inactive shared bus function {function} is omitted from materialized assignments"
        );
    }

    let mirror_nets_before_open = replacement_generated_electrical_nets(&accepted, "right");
    let controller_parameters_before_open = replacement_generator_parameters(&accepted, "right/U1");
    let (packed_omitted, packed_omitted_buffers) = archive(
        &json!({"kind":"pack-project","projectJson":accepted.to_string(),
            "archiveJson":json!({"embedUsedModels":false}).to_string(),"assets":[]}),
        &[],
    );
    assert_eq!(packed_omitted["kind"], "packed", "{packed_omitted}");
    let (unpacked_omitted, unpacked_omitted_buffers) =
        archive(&json!({"kind":"unpack-project"}), &packed_omitted_buffers);
    assert_eq!(unpacked_omitted["kind"], "unpacked", "{unpacked_omitted}");
    assert!(unpacked_omitted_buffers.is_empty());
    let omitted_archive: Value =
        serde_json::from_str(unpacked_omitted["projectJson"].as_str().unwrap())
            .expect("omitted replacement archive project JSON");
    let opened_omitted = replacement_core_call(
        &mut *core,
        json!({"id":"boundary-electrical-replacement-open-before-rejoin",
            "kind":"open","document":omitted_archive}),
    );
    assert_eq!(opened_omitted["kind"], "scene", "{opened_omitted}");
    let opened_omitted_doc = opened_omitted["document"].clone();
    let opened_omitted_config = replacement_board_config(&opened_omitted_doc, "right");
    assert_eq!(
        opened_omitted_config["assignments"], placed_config["assignments"],
        "Open before Rejoin retains replacement and inactive-role assignments"
    );
    assert_eq!(
        opened_omitted_config["locks"], placed_config["locks"],
        "Open before Rejoin retains effective and serialized lock state"
    );
    assert_eq!(
        replacement_generated_electrical_nets(&opened_omitted_doc, "right"),
        mirror_nets_before_open,
        "Open before Rejoin retains target-authored generated electrical nets"
    );
    assert_eq!(
        replacement_generator_parameters(&opened_omitted_doc, "right/U1"),
        controller_parameters_before_open,
        "Open before Rejoin retains controller terminal parameters derived from target nets"
    );
    assert_eq!(
        replacement_source_projection(&opened_omitted_doc),
        source_before,
        "Open before Rejoin leaves source electrical projection unchanged"
    );
    assert_eq!(
        opened_omitted_doc["hardware"]["pairs"][0], pair_after_omit,
        "Open before Rejoin preserves Omit lifecycle state"
    );
    accepted = opened_omitted_doc;

    let rejoin_operation = boundary_exception(&pair_id, "right/NICEVIEW", "linked");
    let rejoin_preview = replacement_core_call(
        &mut *core,
        replacement_edit(
            "boundary-electrical-replacement-rejoin-preview",
            &accepted["revision"],
            "preview",
            rejoin_operation.clone(),
        ),
    );
    assert_eq!(rejoin_preview["kind"], "preview", "{rejoin_preview}");
    assert_eq!(
        replacement_core_call(
            &mut *core,
            json!({"id":"boundary-electrical-replacement-after-rejoin-preview",
        "kind":"snapshot"})
        )["document"],
        accepted,
        "Rejoin Preview must not publish assignments"
    );
    let rejoined = replacement_core_call(
        &mut *core,
        replacement_edit(
            "boundary-electrical-replacement-rejoin",
            &accepted["revision"],
            "commit",
            rejoin_operation,
        ),
    );
    assert_eq!(rejoined["kind"], "scene", "{rejoined}");
    let rejoined_doc = rejoined["document"].clone();
    assert_eq!(replacement_source_projection(&rejoined_doc), source_before);
    assert_eq!(rejoined_doc["hardware"]["pairs"][0], pair_before);
    let rejoined_config = replacement_board_config(&rejoined_doc, "right");
    assert_eq!(
        rejoined_config["assignments"]["peripheral/right/NICEVIEW/CS"],
        source_assignments["peripheral/left/NICEVIEW/CS"],
        "Rejoin restores the source-linked Part's source CS pin"
    );
    for function in ["spi/MOSI", "spi/SCK"] {
        assert_eq!(
            rejoined_config["assignments"][function], source_assignments[function],
            "Rejoin keeps mapped bus function {function} on the source pin"
        );
    }
    for function in ["i2c/SDA", "i2c/SCL"] {
        let pin = rejoined_config["assignments"][function]
            .as_str()
            .unwrap_or_else(|| panic!("Rejoin retains assignment for {function}"));
        assert_ne!(
            pin, freed_cs,
            "the replacement is re-pinned after Rejoin reclaims its former pin"
        );
    }
    let i2c_net_prefix = "generated/electrical/right/i2c/";
    assert!(
        rejoined_doc["nets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|net| net["id"]
                .as_str()
                .is_some_and(|id| id.starts_with(i2c_net_prefix))
                && net["pins"]
                    .as_array()
                    .is_some_and(|pins| pins.iter().any(|pin| pin["partId"] == mirror_id))),
        "the replacement's generated mirror I2C net remains materialized"
    );

    let undo = replacement_core_call(
        &mut *core,
        json!({"id":"boundary-electrical-replacement-undo","kind":"undo"}),
    );
    assert_eq!(undo["kind"], "scene", "{undo}");
    assert_eq!(
        replacement_board_config(&undo["document"], "right")["assignments"],
        placed_config["assignments"],
        "one Undo restores pre-Rejoin pins"
    );
    let redo = replacement_core_call(
        &mut *core,
        json!({"id":"boundary-electrical-replacement-redo","kind":"redo"}),
    );
    assert_eq!(redo["kind"], "scene", "{redo}");
    assert_eq!(
        replacement_board_config(&redo["document"], "right")["assignments"],
        rejoined_config["assignments"],
        "one Redo restores re-pinned assignments"
    );

    let (packed, packed_buffers) = archive(
        &json!({"kind":"pack-project","projectJson":redo["document"].to_string(),
            "archiveJson":json!({"embedUsedModels":false}).to_string(),"assets":[]}),
        &[],
    );
    assert_eq!(packed["kind"], "packed", "{packed}");
    let (unpacked, unpacked_buffers) = archive(&json!({"kind":"unpack-project"}), &packed_buffers);
    assert_eq!(unpacked["kind"], "unpacked", "{unpacked}");
    assert!(unpacked_buffers.is_empty());
    let archived: Value = serde_json::from_str(unpacked["projectJson"].as_str().unwrap())
        .expect("archive project JSON");
    assert_eq!(
        archived, redo["document"],
        "archive roundtrip preserves all assignment state"
    );
    let reopened = replacement_core_call(
        &mut *core,
        json!({"id":"boundary-electrical-replacement-archive-open","kind":"open",
            "document":archived}),
    );
    assert_eq!(reopened["kind"], "scene", "{reopened}");
    assert!(
        reopened["document"]["parts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|part| part["id"] == mirror_id)
    );
    assert_eq!(
        replacement_board_config(&reopened["document"], "right")["assignments"],
        rejoined_config["assignments"],
        "Save/Open retains replacement re-pin"
    );
    assert_eq!(
        replacement_source_projection(&reopened["document"]),
        source_before
    );

    // A fresh fixture admits real catalogued loads until the same reviewed
    // Nice Nano profile is genuinely exhausted. Then Rejoin must refuse without
    // publishing the lifecycle change or any partial electrical plan.
    let (mut capacity_doc, capacity_pair_id, capacity_nice_view_id, capacity_ssd_id) =
        replacement_pair_fixture(&mut *core, "boundary-electrical-capacity");
    let capacity_source_before = replacement_source_projection(&capacity_doc);
    let capacity_omit = replacement_core_call(
        &mut *core,
        replacement_edit(
            "boundary-electrical-capacity-omit",
            &capacity_doc["revision"],
            "commit",
            boundary_exception(&capacity_pair_id, "right/NICEVIEW", "omitted"),
        ),
    );
    assert_eq!(capacity_omit["kind"], "scene", "{capacity_omit}");
    capacity_doc = capacity_omit["document"].clone();
    let capacity_place = replacement_core_call(
        &mut *core,
        replacement_edit(
            "boundary-electrical-capacity-place",
            &capacity_doc["revision"],
            "commit",
            mirror_only_display_operation("right", "right/CAPACITY-OLED", &capacity_ssd_id),
        ),
    );
    assert_eq!(capacity_place["kind"], "scene", "{capacity_place}");
    capacity_doc = capacity_place["document"].clone();
    let mut capacity_fill_refused = false;
    for index in 0..16 {
        let id = format!("right/BOUNDARY-CAPACITY{index}");
        let attempt = replacement_core_call(
            &mut *core,
            replacement_edit(
                &format!("boundary-electrical-capacity-fill-{index}"),
                &capacity_doc["revision"],
                "commit",
                mirror_only_display_operation("right", &id, &capacity_nice_view_id),
            ),
        );
        if attempt["kind"] == "error" {
            assert!(
                attempt["message"]
                    .as_str()
                    .is_some_and(|message| message.starts_with("No available GPIO for ")),
                "filling the reviewed profile refuses from pin capacity: {attempt}"
            );
            assert_eq!(
                replacement_core_call(
                    &mut *core,
                    json!({"id":"boundary-electrical-capacity-after-fill-refusal","kind":"snapshot"})
                )["document"],
                capacity_doc,
                "failed filler placement leaves accepted state untouched"
            );
            capacity_fill_refused = true;
            break;
        }
        assert_eq!(attempt["kind"], "scene", "{attempt}");
        capacity_doc = attempt["document"].clone();
    }
    assert!(
        capacity_fill_refused,
        "real Nice Nano GPIO capacity is exhausted by catalogued peripheral functions"
    );
    assert_eq!(
        replacement_source_projection(&capacity_doc),
        capacity_source_before
    );
    let rejoin_refusal = replacement_core_call(
        &mut *core,
        replacement_edit(
            "boundary-electrical-capacity-rejoin",
            &capacity_doc["revision"],
            "commit",
            boundary_exception(&capacity_pair_id, "right/NICEVIEW", "linked"),
        ),
    );
    assert_eq!(rejoin_refusal["kind"], "error", "{rejoin_refusal}");
    assert!(
        rejoin_refusal["message"]
            .as_str()
            .is_some_and(|message| message.starts_with("No available GPIO for ")),
        "capacity Rejoin returns the resolver pin finding: {rejoin_refusal}"
    );
    assert_eq!(
        replacement_core_call(
            &mut *core,
            json!({"id":"boundary-electrical-capacity-after-rejoin-refusal","kind":"snapshot"})
        )["document"],
        capacity_doc,
        "failed Rejoin publishes no lifecycle/electrical changes"
    );
    assert_eq!(
        replacement_source_projection(&capacity_doc),
        capacity_source_before
    );

    json!({
        "supportedController":"ceoloide/mcu_nice_nano",
        "omitReleasesPerPartCs":true,
        "mirrorOnlyPlacementAutomaticallyPinned":true,
        "freedCsPinWasAllocatedToReplacement":true,
        "rejoinRestoresSourcePinsAndRepinsReplacement":true,
        "sourceUnchanged":true,
        "previewNonpublishing":true,
        "singleStepUndoRedo":true,
        "archiveSaveOpenPreservesAssignments":true,
        "omittedReplacementArchiveOpenStable":true,
        "capacityPlacementRefusalAtomic":true,
        "capacityRejoinRefusalAtomic":true
    })
}

/// Proves ticket 160's exact Rejoin pin/net/registration loss through the same
/// JSON Core transport used by the native golden and the browser/WASM replay.
fn paired_electrical_preview_case(mut core: CoreTransport<'_>) -> Value {
    let call = |core: &mut CoreTransport<'_>, request: Value| {
        serde_json::from_str::<Value>(&core(&request.to_string())).expect("core reply is JSON")
    };
    let mut source = supported_synthetic_half();
    source["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|definition| definition["id"] == "synthetic/controller")
        .unwrap()["pads"] = json!([{
        "id":"pad-1","number":"1","at":{"x":0.0,"y":0.0},
        "size":{"x":1.0,"y":1.0},"shape":"circle"
    }]);
    source["nets"].as_array_mut().unwrap().push(json!({
        "id":"left/omit-controller-net","name":"OMIT_CONTROLLER",
        "pins":[{"partId":"left/RST","padId":"pad-1"}]
    }));
    source["boards"][0]["netIds"]
        .as_array_mut()
        .unwrap()
        .push(json!("left/omit-controller-net"));

    let open = call(
        &mut core,
        json!({"id":"electrical-open","kind":"open","document":source}),
    );
    assert_eq!(open["kind"], "scene", "{open}");
    let created = call(
        &mut core,
        json!({
            "id":"electrical-create",
            "kind":"edit",
            "command":{
                "baseRevision":open["document"]["revision"],
                "transactionId":"electrical-create",
                "phase":"commit",
                "targetIds":[],
                "operation":synthetic_half::create_mirrored_half_operation(
                    "boundary-electrical-pair",
                    "right"
                )
            }
        }),
    );
    assert_eq!(created["kind"], "scene", "{created}");
    let pair_id = "boundary-electrical-pair";
    let source_net_id = "left/omit-controller-net";
    let mirror_net_id = created["document"]["hardware"]["pairs"][0]["halves"]["correspondences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["kind"] == "net" && item["sourceId"] == source_net_id)
        .unwrap_or_else(|| panic!("plain net correspondence for {source_net_id} is missing"))["mirrorId"]
        .as_str()
        .unwrap()
        .to_owned();

    let localized = call(
        &mut core,
        json!({
            "id":"electrical-local-controller","kind":"edit",
            "command":{"baseRevision":created["document"]["revision"],
                "transactionId":"electrical-local-controller","phase":"commit",
                "targetIds":["right/U1"],
                "operation":boundary_exception(pair_id,"right/U1","local")}
        }),
    );
    assert_eq!(localized["kind"], "scene", "{localized}");
    let mut accepted_with_local_controller = localized["document"].clone();
    let mirror_net = accepted_with_local_controller["nets"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|net| net["id"] == mirror_net_id)
        .unwrap_or_else(|| panic!("mirror net {mirror_net_id} is missing"));
    mirror_net["pins"].as_array_mut().unwrap().push(json!({
        "partId":"right/U1","padId":"pad-1"
    }));
    let seeded = call(
        &mut core,
        json!({
            "id":"electrical-seed-mirror-pin","kind":"edit",
            "command":{"baseRevision":accepted_with_local_controller["revision"],
                "transactionId":"electrical-seed-mirror-pin","phase":"commit","targetIds":[],
                "operation":{"kind":"replace-document","document":accepted_with_local_controller}}
        }),
    );
    assert_eq!(seeded["kind"], "scene", "{seeded}");
    let accepted_with_mirror_pin = seeded["document"].clone();
    assert!(
        accepted_with_mirror_pin["nets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|net| net["id"] == mirror_net_id)
            .unwrap()["pins"]
            .as_array()
            .unwrap()
            .iter()
            .any(|pin| pin["partId"] == "right/U1" && pin["padId"] == "pad-1")
    );

    let omitted = call(
        &mut core,
        json!({
            "id":"electrical-omit-reset","kind":"edit",
            "command":{"baseRevision":accepted_with_mirror_pin["revision"],
                "transactionId":"electrical-omit-reset","phase":"commit",
                "targetIds":["right/RST"],
                "operation":boundary_exception(pair_id,"right/RST","omitted")}
        }),
    );
    assert_eq!(omitted["kind"], "scene", "{omitted}");
    let accepted = omitted["document"].clone();
    let right_board_name = accepted["boards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|board| board["id"] == "right")
        .unwrap()["name"]
        .as_str()
        .unwrap();
    let mirror_net_name = accepted["nets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|net| net["id"] == mirror_net_id)
        .unwrap()["name"]
        .as_str()
        .unwrap();
    let controller_reference = accepted["parts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|part| part["id"] == "right/U1")
        .unwrap()["reference"]
        .as_str()
        .unwrap();

    let preview = call(
        &mut core,
        json!({
            "id":"electrical-rejoin-preview","kind":"edit",
            "command":{"baseRevision":accepted["revision"],
                "transactionId":"electrical-rejoin","phase":"preview",
                "targetIds":["right/U1"],
                "operation":boundary_exception(pair_id,"right/U1","linked")}
        }),
    );
    assert_eq!(preview["kind"], "preview", "{preview}");
    let mut expected_notices = vec![
        json!({
            "id":electrical_notice_id("pin", &[&mirror_net_id,"right/U1","pad-1"]),
            "severity":"info","scope":"pcb",
            "message":format!("Preview removes pad pad-1 on {controller_reference} (right/U1) from net {mirror_net_name} ({mirror_net_id})."),
            "targetIds":[mirror_net_id,"right/U1"]
        }),
        json!({
            "id":electrical_notice_id("net", &[&mirror_net_id]),
            "severity":"info","scope":"pcb",
            "message":format!("Preview removes net {mirror_net_name} ({mirror_net_id})."),
            "targetIds":[mirror_net_id]
        }),
        json!({
            "id":electrical_notice_id("registration", &["right",&mirror_net_id]),
            "severity":"info","scope":"pcb",
            "message":format!("Preview removes net {mirror_net_name} ({mirror_net_id}) from board {right_board_name} (right)."),
            "targetIds":["right",mirror_net_id]
        }),
    ];
    expected_notices.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));
    let mut electrical_notices: Vec<Value> = preview["scene"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| {
            finding["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("preview:electrical:"))
        })
        .cloned()
        .collect();
    electrical_notices.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));
    assert_eq!(electrical_notices, expected_notices, "{preview}");
    let after_preview = call(
        &mut core,
        json!({"id":"electrical-after-preview","kind":"snapshot"}),
    );
    assert_eq!(
        after_preview["document"], accepted,
        "Preview must not publish Rejoin"
    );

    let committed = call(
        &mut core,
        json!({
            "id":"electrical-rejoin-commit","kind":"edit",
            "command":{"baseRevision":accepted["revision"],
                "transactionId":"electrical-rejoin","phase":"commit",
                "targetIds":["right/U1"],
                "operation":boundary_exception(pair_id,"right/U1","linked")}
        }),
    );
    assert_eq!(committed["kind"], "scene", "{committed}");
    assert!(
        committed["document"]["nets"]
            .as_array()
            .unwrap()
            .iter()
            .all(|net| net["id"] != mirror_net_id)
    );
    assert!(
        committed["document"]["boards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|board| board["id"] == "right")
            .unwrap()["netIds"]
            .as_array()
            .unwrap()
            .iter()
            .all(|net_id| net_id != &mirror_net_id)
    );
    assert!(
        committed["scene"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|finding| finding["id"]
                .as_str()
                .is_none_or(|id| !id.starts_with("preview:electrical:")))
    );

    json!({
        "open":{"kind":open["kind"]},
        "created":{"kind":created["kind"],"pairId":pair_id,"mirrorNetId":mirror_net_id},
        "preview":{"kind":preview["kind"],"findings":electrical_notices,"acceptedDocumentUnchanged":true},
        "afterPreview":{"kind":after_preview["kind"],"documentUnchanged":true},
        "commit":{"kind":committed["kind"],"mirrorNetRemoved":true,"previewNoticeCount":0}
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
    // These scenario outcomes must hold before transcript.json compaction or blessing.
    let requests = core_requests();
    assert_eq!(requests.len(), 67, "retain every existing primary scenario");
    assert_eq!(replies.len(), requests.len());
    let mut revision = 0;
    for (request, reply) in requests.iter().zip(replies) {
        let id = request["id"].as_str().unwrap();
        assert_eq!(reply["id"], id);
        let refusal = match id {
            "invalid-draft" => Some("invalid matrix dimensions"),
            "stale-draft" => Some("Stale base revision"),
            "convert-refused" => {
                Some("Converting requires zero Splay, Column rotation and carried Bend")
            }
            "mirrored-half-new-board" | "mirrored-half-reuse-board" => {
                Some("Physical instance \"board\" is unavailable.")
            }
            _ => None,
        };
        let kind = if let Some(reason) = refusal {
            assert_eq!(reply["message"], reason, "{id}");
            "error"
        } else {
            match request["kind"].as_str().unwrap() {
                "edit" if request["command"]["phase"] == "preview" => "preview",
                "edit" => {
                    assert_eq!(request["command"]["phase"], "commit");
                    if !matches!(
                        id,
                        "mechanical-closure-repeat"
                            | "outline-corners-repeat"
                            | "outline-rename-repeat"
                    ) {
                        revision += 1;
                    }
                    "scene"
                }
                "undo" | "redo" => {
                    revision += 1;
                    "scene"
                }
                "open" | "snapshot" => "scene",
                "project-matrices" => "matrix-projections",
                other => panic!("declare the intended outcome of {other}: {id}"),
            }
        };
        assert_eq!(reply["kind"], kind, "{id}: {reply}");
        let actual_revision = reply
            .get("scene")
            .map_or(&reply["revision"], |scene| &scene["revision"]);
        assert_eq!(*actual_revision, revision, "{id}: revision/history intent");
    }

    let initial = &replies[0]["document"];
    let committed = &replies[6]["document"];
    for index in [4, 6] {
        assert_eq!(
            replies[index]["scene"]["transforms"]
                .as_array()
                .unwrap()
                .iter()
                .map(|part| &part["id"])
                .collect::<Vec<_>>(),
            initial["parts"]
                .as_array()
                .unwrap()
                .iter()
                .map(|part| &part["id"])
                .collect::<Vec<_>>(),
            "preview/commit retain every visible member identity",
        );
        assert_eq!(
            replies[index]["scene"]["changedIds"],
            json!(["matrix", "matrix/matrix/r0c0"])
        );
        for transform in replies[index]["scene"]["transforms"].as_array().unwrap() {
            let original = initial["parts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|part| part["id"] == transform["id"])
                .unwrap();
            if transform["id"] == "matrix/matrix/r0c0" {
                assert_eq!(transform["pose"]["at"], json!({"x":10.0,"y":12.0}));
                assert_eq!(transform["pose"]["rotation"], original["pose"]["rotation"]);
            } else {
                assert_eq!(
                    transform["pose"], original["pose"],
                    "unselected key stays still"
                );
            }
        }
    }
    assert_eq!(replies[5]["document"], *initial, "preview must not publish");
    assert_eq!(
        committed["matrices"][0]["partIds"],
        initial["matrices"][0]["partIds"]
    );
    let moved = committed["matrices"][0]["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|cell| cell["row"] == 0 && cell["column"] == 0)
        .unwrap();
    assert_eq!(moved["offset"], json!({"x":3.0,"y":15.0}));
    for (index, expected) in [(8, initial), (9, committed)] {
        let mut actual = replies[index]["document"].clone();
        actual["revision"] = expected["revision"].clone();
        assert_eq!(actual, *expected, "history restores all authored state");
    }
    let before_origin = &replies[9]["document"];
    let held = &replies[12]["document"];
    for index in [11, 12] {
        let matrix_scene = &replies[index]["scene"]["matrixScenes"][0];
        let original_cells = replies[9]["scene"]["matrixScenes"][0]["cells"]
            .as_array()
            .unwrap();
        let cells = matrix_scene["cells"].as_array().unwrap();
        assert_eq!(cells.len(), original_cells.len());
        for (cell, original) in cells.iter().zip(original_cells) {
            assert_eq!(cell["memberId"], original["memberId"]);
            assert_eq!(cell["enabled"], original["enabled"]);
            for axis in ["x", "y"] {
                assert!(
                    (cell["pose"]["at"][axis].as_f64().unwrap()
                        - original["pose"]["at"][axis].as_f64().unwrap())
                    .abs()
                        < 1e-9,
                    "hold includes logical disabled keys"
                );
            }
            assert_eq!(cell["pose"]["rotation"], original["pose"]["rotation"]);
        }
        assert_eq!(
            matrix_scene["columns"][1]["splayOrigin"],
            json!({"x":5.0,"y":-12.0})
        );
        for transform in replies[index]["scene"]["transforms"].as_array().unwrap() {
            let original = before_origin["parts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|part| part["id"] == transform["id"])
                .unwrap();
            for axis in ["x", "y"] {
                assert!(
                    (transform["pose"]["at"][axis].as_f64().unwrap()
                        - original["pose"]["at"][axis].as_f64().unwrap())
                    .abs()
                        < 1e-9,
                    "origin hold: {} {axis}",
                    transform["id"]
                );
            }
            assert_eq!(transform["pose"]["rotation"], original["pose"]["rotation"]);
        }
    }
    assert_eq!(
        held["matrices"][0]["cells"],
        before_origin["matrices"][0]["cells"]
    );
    let mut allowed = before_origin["matrices"][0].clone();
    for field in ["spread", "stagger", "splayOrigin"] {
        allowed["columnLayout"][1][field] = held["matrices"][0]["columnLayout"][1][field].clone();
    }
    assert_eq!(
        held["matrices"][0], allowed,
        "hold writes only this step and relative origin when Column rotation is zero"
    );
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
    assert_eq!(replies[7]["message"], "Stale base revision");
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

    // Guard scenario intent before compaction or blessing can hide a refusal cascade.
    for (id, kind, revision) in [
        ("convert-offset-1", "scene", 43),
        ("convert-preview", "preview", 43),
        ("convert-commit", "scene", 44),
        ("convert-offset-2", "scene", 45),
    ] {
        let reply = &replies[position(replies, id)];
        assert_eq!(reply["kind"], kind, "{id}: {reply}");
        assert_eq!(reply["scene"]["revision"], revision, "{id}: {reply}");
    }
    let converted = &replies[position(replies, "convert-commit")]["document"]["matrices"][0];
    assert_eq!(converted["columnLayout"][0]["stagger"], 3.0);
    assert!(converted["columnLayout"][0].get("offset").is_none());
    assert_eq!(converted["columnLayout"][1]["stagger"], -1.0);
    for (id, message) in [
        (
            "convert-refused",
            "Converting requires zero Splay, Column rotation and carried Bend",
        ),
        (
            "mirrored-half-new-board",
            "Physical instance \"board\" is unavailable.",
        ),
        (
            "mirrored-half-reuse-board",
            "Physical instance \"board\" is unavailable.",
        ),
    ] {
        let reply = &replies[position(replies, id)];
        assert_eq!(reply["kind"], "error", "{id}: {reply}");
        assert_eq!(reply["message"], message, "{id}: {reply}");
        assert_eq!(reply["revision"], 45, "{id}: {reply}");
    }

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

fn compact_reply(reply: &mut Value, full_body: bool) {
    if full_body {
        return;
    }
    let body = serde_json::to_vec(reply).expect("reply serializes");
    let keys = |name| {
        reply[name].as_object().map(|object| {
            let mut keys = object.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            keys
        })
    };
    let mut compact = json!({
        "kind": reply["kind"],
        "bodyDigest": digest(&body),
    });
    for field in ["id", "message", "revision"] {
        if let Some(value) = reply.get(field) {
            compact[field] = value.clone();
        }
    }
    if let Some(keys) = keys("document") {
        compact["documentKeys"] = json!(keys);
    }
    if let Some(keys) = keys("scene") {
        compact["sceneKeys"] = json!(keys);
    }
    *reply = compact;
}

fn compact_replies(replies: &mut [Value]) {
    for reply in replies {
        let anchor = matches!(
            reply["id"].as_str(),
            Some("open" | "commit" | "invalid-draft" | "draft")
        );
        compact_reply(reply, anchor);
    }
}

fn compact_archive(transcript: &mut [Value]) {
    let mut kept_unpack = false;
    for entry in transcript {
        let is_unpack = entry["kind"] == "unpack-project" && entry["reply"]["kind"] == "unpacked";
        if is_unpack && !kept_unpack {
            kept_unpack = true;
            continue;
        }
        let reply = &mut entry["reply"];
        let body = serde_json::to_vec(reply).expect("archive reply serializes");
        let mut compact = json!({ "kind": reply["kind"], "bodyDigest": digest(&body) });
        if let Some(message) = reply.get("message") {
            compact["message"] = message.clone();
        }
        *reply = compact;
    }
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
    compact_replies(&mut replies);

    let paired_preview_history = paired_preview_history_case(&mut *core);
    let paired_electrical_preview = paired_electrical_preview_case(&mut *core);
    let mirror_addition = mirror_addition_boundary_case(&mut *core, &mut *archive);
    let electrical_mirror_replacement =
        electrical_mirror_replacement_case(&mut *core, &mut *archive);

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

    compact_archive(&mut transcript);

    json!({
        "core": replies,
        "archive": transcript,
        "pairedPreviewHistory": paired_preview_history,
        "pairedElectricalPreview": paired_electrical_preview,
        "mirrorAddition": mirror_addition,
        "electricalMirrorReplacement": electrical_mirror_replacement
    })
}

pub fn assert_golden(actual: &Value, golden: &Value) {
    equivalent(actual, golden, "transcript");
}

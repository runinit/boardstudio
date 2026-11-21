use super::*;

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

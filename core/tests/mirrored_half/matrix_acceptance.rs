//! Activated-route acceptance for matrix geometry, membership, and keycap edits.
//!
//! The shared synthetic fixture contains one matrix per pair. These cases exercise source and
//! mirror authorship, Preview isolation, exact paired effects, and one-step document history.

use super::*;

const SOURCE_MATRIX: &str = "left-synthetic-keys";
const MIRROR_MATRIX: &str = "right-synthetic-keys";
const SOURCE_THUMB_MATRIX: &str = "left-thumb-keys";
const MIRROR_THUMB_MATRIX: &str = "right-thumb-keys";

fn accepted_matrix_pair(pair_id: &str) -> (CoreEngine, Value) {
    let (mut engine, seed) = seed(&runtime_supported_document(), "left");
    let created = create(
        &mut engine,
        &seed,
        pair_id,
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let before_addition = created["document"].clone();
    let thumb = json!({
        "kind":"set-matrix",
        "matrix":{
            "id":SOURCE_THUMB_MATRIX,
            "name":"Synthetic thumb keys",
            "rows":1,
            "columns":2,
            "pitch":{"x":19.0,"y":21.0},
            "origin":{"x":50.0,"y":50.0},
            "definitionId":"generator:ceoloide/switch_mx",
            "boardId":"left",
            "partIds":[],
            "cells":[
                {"row":0,"column":0,"enabled":true},
                {"row":0,"column":1,"enabled":true}
            ]
        }
    });
    let added = matrix_commit(
        &mut engine,
        &before_addition,
        &format!("{pair_id}-add-thumb-matrix"),
        thumb,
        &[],
    );
    assert_eq!(added["kind"], "scene", "add sibling matrix: {added}");
    assert!(matrix(&added["document"], SOURCE_THUMB_MATRIX).is_object());
    assert!(matrix(&added["document"], MIRROR_THUMB_MATRIX).is_object());
    (engine, added["document"].clone())
}

fn matrix<'a>(document: &'a Value, id: &str) -> &'a Value {
    document["matrices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|matrix| matrix["id"] == id)
        .unwrap_or_else(|| panic!("matrix {id} is missing"))
}

fn matrix_commit(
    engine: &mut CoreEngine,
    before: &Value,
    id: &str,
    operation: Value,
    target_ids: &[&str],
) -> Value {
    request(
        engine,
        json!({"id":id,"kind":"edit","command":{
            "baseRevision":before["revision"],
            "transactionId":id,
            "phase":"commit",
            "targetIds":target_ids,
            "operation":operation
        }}),
    )
}

fn preview_without_publishing(
    engine: &mut CoreEngine,
    before: &Value,
    id: &str,
    operation: Value,
    target_ids: &[&str],
) {
    let preview = request(
        engine,
        json!({"id":id,"kind":"edit","command":{
            "baseRevision":before["revision"],
            "transactionId":id,
            "phase":"preview",
            "targetIds":target_ids,
            "operation":operation
        }}),
    );
    assert_eq!(preview["kind"], "preview", "{preview}");
    assert_eq!(snapshot(engine), before.clone(), "Preview must not publish");
}

fn preview_transform<'a>(preview: &'a Value, id: &str) -> &'a Value {
    preview["scene"]["transforms"]
        .as_array()
        .unwrap()
        .iter()
        .find(|transform| transform["id"] == id)
        .unwrap_or_else(|| panic!("preview transform for {id} is missing: {preview}"))
}

#[test]
fn parent19_matrix_move_preview_projects_from_either_authored_half_with_companions() {
    for author in ["left", "right"] {
        let pair_id = format!("parent19-matrix-move-preview-{author}");
        let (mut engine, before) = accepted_matrix_pair(&pair_id);
        let (source_key, mirror_key) = (
            "matrix/left-synthetic-keys/r0c0",
            "matrix/right-synthetic-keys/r0c0",
        );
        let (author_key, dx, dy) = if author == "left" {
            (source_key, 7.0, 3.0)
        } else {
            (mirror_key, -7.0, 3.0)
        };
        let old_author_pose = part(&before, author_key)["pose"]["at"].clone();
        let requested = json!({"x":old_author_pose["x"].as_f64().unwrap() + dx,
            "y":old_author_pose["y"].as_f64().unwrap() + dy});
        let operation = json!({"kind":"move-parts","positions":[{"id":author_key,"at":requested}]});
        let reply = request(
            &mut engine,
            json!({"id":format!("{pair_id}-preview"),"kind":"edit","command":{
                "baseRevision":before["revision"],"transactionId":format!("{pair_id}-preview"),
                "phase":"preview","targetIds":[author_key],"operation":operation
            }}),
        );
        assert_eq!(reply["kind"], "preview", "{reply}");

        let source_after = preview_transform(&reply, source_key);
        let mirror_after = preview_transform(&reply, mirror_key);
        let source_x = if author == "left" {
            requested["x"].as_f64().unwrap()
        } else {
            200.0 - requested["x"].as_f64().unwrap()
        };
        assert_eq!(source_after["pose"]["at"]["x"], json!(source_x));
        assert_eq!(source_after["pose"]["at"]["y"], requested["y"]);
        assert_eq!(mirror_after["pose"]["at"]["x"], json!(200.0 - source_x));
        assert_eq!(mirror_after["pose"]["at"]["y"], requested["y"]);

        // Authored diode and LED geometry must stay fixed relative to each moved key.
        for (key_id, preview_key) in [(source_key, source_after), (mirror_key, mirror_after)] {
            let old_key = part(&before, key_id);
            for child_id in [format!("{key_id}/diode"), format!("{key_id}/led")] {
                let old_child = part(&before, &child_id);
                let new_child = preview_transform(&reply, &child_id);
                let x_offset = new_child["pose"]["at"]["x"].as_f64().unwrap()
                    - preview_key["pose"]["at"]["x"].as_f64().unwrap();
                let old_x_offset = old_child["pose"]["at"]["x"].as_f64().unwrap()
                    - old_key["pose"]["at"]["x"].as_f64().unwrap();
                assert!(
                    (x_offset - old_x_offset).abs() < 1e-9,
                    "{child_id} x offset from {key_id}: {x_offset} != {old_x_offset}"
                );
                let y_offset = new_child["pose"]["at"]["y"].as_f64().unwrap()
                    - preview_key["pose"]["at"]["y"].as_f64().unwrap();
                let old_y_offset = old_child["pose"]["at"]["y"].as_f64().unwrap()
                    - old_key["pose"]["at"]["y"].as_f64().unwrap();
                assert!(
                    (y_offset - old_y_offset).abs() < 1e-9,
                    "{child_id} y offset from {key_id}: {y_offset} != {old_y_offset}"
                );
                assert_eq!(new_child["pose"]["rotation"], old_child["pose"]["rotation"]);
            }
        }
        assert_eq!(snapshot(&mut engine), before, "Preview must not publish");
    }
}

#[test]
fn parent19_noop_preview_and_retired_target_refusal_preserve_history_cursor() {
    let (mut engine, seed_document) = seed(&runtime_supported_document(), "left");
    let created = create(
        &mut engine,
        &seed_document,
        "parent19-preview-history-pair",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let accepted = created["document"].clone();

    // Leave a Redo entry, then verify a no-op Preview doesn't consume it.
    let setup_operation = json!({"kind":"set-matrix-placement","matrixId":SOURCE_MATRIX,
        "change":{"kind":"column-stagger","column":1,"value":3.0}});
    let changed = matrix_commit(
        &mut engine,
        &accepted,
        "parent19-preview-history-setup",
        setup_operation,
        &[],
    );
    assert_eq!(changed["kind"], "scene", "{changed}");
    let undone = request(
        &mut engine,
        json!({"id":"parent19-preview-history-undo","kind":"undo"}),
    );
    assert_eq!(undone["kind"], "scene", "{undone}");
    let mut accepted_at_cursor = accepted.clone();
    accepted_at_cursor["revision"] = undone["document"]["revision"].clone();
    assert_eq!(undone["document"], accepted_at_cursor);

    preview_without_publishing(
        &mut engine,
        &accepted_at_cursor,
        "parent19-noop-preview",
        json!({"kind":"move-parts","positions":[{"id":"matrix/left-synthetic-keys/r0c0",
            "at":part(&accepted_at_cursor,"matrix/left-synthetic-keys/r0c0")["pose"]["at"]}]}),
        &["matrix/left-synthetic-keys/r0c0"],
    );
    let redone = request(
        &mut engine,
        json!({"id":"parent19-preview-history-redo","kind":"redo"}),
    );
    assert_eq!(redone["kind"], "scene", "{redone}");
    let mut expected_redo = changed["document"].clone();
    expected_redo["revision"] = redone["document"]["revision"].clone();
    assert_eq!(
        redone["document"], expected_redo,
        "no-op Preview preserves Redo"
    );

    // Remove a key, Preview against its retired ID, then prove Undo and Redo still work.
    let before_removal = redone["document"].clone();
    let removal = matrix_commit(
        &mut engine,
        &before_removal,
        "parent19-retire-matrix-key",
        json!({"kind":"remove-matrix-cells","matrixId":SOURCE_MATRIX,"cells":[[0,0]]}),
        &["matrix/left-synthetic-keys/r0c0"],
    );
    assert_eq!(removal["kind"], "scene", "{removal}");
    let retired_id = "matrix/left-synthetic-keys/r0c0";
    let retired_refusal: Value = serde_json::from_str(
        &engine.request(
            &json!({"id":"parent19-retired-target-preview","kind":"edit","command":{
                "baseRevision":removal["document"]["revision"],
                "transactionId":"parent19-retired-target-preview","phase":"preview",
                "targetIds":[retired_id],"operation":{"kind":"move-parts",
                    "positions":[{"id":retired_id,"at":{"x":12.0,"y":5.0}}]}
            }})
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(retired_refusal["kind"], "error", "{retired_refusal}");
    assert_eq!(snapshot(&mut engine), removal["document"]);

    let undo = request(
        &mut engine,
        json!({"id":"parent19-retired-target-undo","kind":"undo"}),
    );
    assert_eq!(undo["kind"], "scene", "{undo}");
    let mut expected_undo = before_removal;
    expected_undo["revision"] = undo["document"]["revision"].clone();
    assert_eq!(
        undo["document"], expected_undo,
        "retired-target Preview preserves Undo"
    );
    let redo = request(
        &mut engine,
        json!({"id":"parent19-retired-target-redo","kind":"redo"}),
    );
    assert_eq!(redo["kind"], "scene", "{redo}");
    let mut expected_redo = removal["document"].clone();
    expected_redo["revision"] = redo["document"]["revision"].clone();
    assert_eq!(
        redo["document"], expected_redo,
        "retired-target Preview preserves Redo"
    );
}

fn assert_document_history(
    engine: &mut CoreEngine,
    before: &Value,
    committed: &Value,
    label: &str,
) {
    let undone = request(engine, json!({"id":format!("{label}-undo"),"kind":"undo"}));
    assert_eq!(undone["kind"], "scene", "{label} Undo: {undone}");
    let mut expected_before = before.clone();
    expected_before["revision"] = undone["document"]["revision"].clone();
    assert_eq!(
        undone["document"], expected_before,
        "{label} Undo restores the full document"
    );

    let redone = request(engine, json!({"id":format!("{label}-redo"),"kind":"redo"}));
    assert_eq!(redone["kind"], "scene", "{label} Redo: {redone}");
    let mut expected_after = committed.clone();
    expected_after["revision"] = redone["document"]["revision"].clone();
    assert_eq!(
        redone["document"], expected_after,
        "{label} Redo restores the full document"
    );
}

fn assert_projected_key_pose(document: &Value, source_id: &str, mirror_id: &str, x: f64, y: f64) {
    assert_eq!(part(document, source_id)["pose"]["at"]["x"], json!(x));
    assert_eq!(part(document, source_id)["pose"]["at"]["y"], json!(y));
    assert_eq!(
        part(document, mirror_id)["pose"]["at"]["x"],
        json!(200.0 - x)
    );
    assert_eq!(part(document, mirror_id)["pose"]["at"]["y"], json!(y));
}

fn assert_thumb_matrix_unchanged(before: &Value, after: &Value) {
    for matrix_id in [SOURCE_THUMB_MATRIX, MIRROR_THUMB_MATRIX] {
        let sibling = matrix(before, matrix_id);
        assert_eq!(
            matrix(after, matrix_id),
            sibling,
            "sibling matrix {matrix_id}"
        );
        for part_id in sibling["partIds"].as_array().unwrap() {
            let part_id = part_id.as_str().unwrap();
            assert_eq!(
                part(after, part_id),
                part(before, part_id),
                "sibling matrix member {part_id}"
            );
            assert_eq!(
                after["keycaps"]["keys"][part_id], before["keycaps"]["keys"][part_id],
                "sibling keycap setting {part_id}"
            );
        }
        assert_eq!(
            after["keycaps"]["matrices"][matrix_id], before["keycaps"]["matrices"][matrix_id],
            "sibling matrix keycap setting {matrix_id}"
        );
    }
}

#[test]
fn parent19_matrix_placement_projects_from_either_half_with_one_step_history() {
    for author in ["left", "right"] {
        let pair_id = format!("parent19-matrix-placement-{author}");
        let (mut engine, before) = accepted_matrix_pair(&pair_id);
        let matrix_id = if author == "left" {
            SOURCE_MATRIX
        } else {
            MIRROR_MATRIX
        };
        let source_key = "matrix/left-synthetic-keys/r0c1";
        let mirror_key = "matrix/right-synthetic-keys/r0c1";
        let operation = json!({"kind":"set-matrix-placement","matrixId":matrix_id,
            "change":{"kind":"column-stagger","column":1,"value":4.0}});

        preview_without_publishing(
            &mut engine,
            &before,
            &format!("parent19-placement-preview-{author}"),
            operation.clone(),
            &[source_key, mirror_key],
        );
        let committed = matrix_commit(
            &mut engine,
            &before,
            &format!("parent19-placement-commit-{author}"),
            operation,
            &[source_key, mirror_key],
        );
        assert_eq!(committed["kind"], "scene", "{committed}");
        assert_eq!(
            committed["document"]["revision"],
            before["revision"].as_u64().unwrap() + 1
        );
        assert_projected_key_pose(&committed["document"], source_key, mirror_key, 19.0, 4.0);
        assert_eq!(
            matrix(&committed["document"], SOURCE_MATRIX)["cells"],
            matrix(&before, SOURCE_MATRIX)["cells"],
            "placement changes retain source membership"
        );
        assert_eq!(
            matrix(&committed["document"], MIRROR_MATRIX)["cells"],
            matrix(&before, MIRROR_MATRIX)["cells"],
            "placement changes retain mirror membership"
        );
        assert_thumb_matrix_unchanged(&before, &committed["document"]);
        assert_document_history(&mut engine, &before, &committed["document"], &pair_id);
    }
}

#[test]
fn parent19_matrix_splay_projects_member_geometry_from_either_half() {
    let x = 19.0 - 21.0 * 12.0_f64.to_radians().sin();
    let y = 21.0 * 12.0_f64.to_radians().cos();
    for author in ["left", "right"] {
        let pair_id = format!("parent19-matrix-splay-{author}");
        let (mut engine, before) = accepted_matrix_pair(&pair_id);
        let matrix_id = if author == "left" {
            SOURCE_MATRIX
        } else {
            MIRROR_MATRIX
        };
        let source_key = "matrix/left-synthetic-keys/r1c1";
        let mirror_key = "matrix/right-synthetic-keys/r1c1";
        let operation = json!({"kind":"set-matrix-splay","matrixId":matrix_id,"column":1,
            "change":{"kind":"angle","angle":12.0,"affect":"column"}});

        preview_without_publishing(
            &mut engine,
            &before,
            &format!("parent19-splay-preview-{author}"),
            operation.clone(),
            &[source_key, mirror_key],
        );
        let committed = matrix_commit(
            &mut engine,
            &before,
            &format!("parent19-splay-commit-{author}"),
            operation,
            &[source_key, mirror_key],
        );
        assert_eq!(committed["kind"], "scene", "{committed}");
        assert_projected_key_pose(&committed["document"], source_key, mirror_key, x, y);
        assert_thumb_matrix_unchanged(&before, &committed["document"]);
        assert_document_history(&mut engine, &before, &committed["document"], &pair_id);
    }
}

#[test]
fn parent19_matrix_membership_removal_projects_from_either_half_once() {
    for author in ["left", "right"] {
        let pair_id = format!("parent19-matrix-membership-{author}");
        let (mut engine, before) = accepted_matrix_pair(&pair_id);
        let matrix_id = if author == "left" {
            SOURCE_MATRIX
        } else {
            MIRROR_MATRIX
        };
        let removed_source_key = "matrix/left-synthetic-keys/r0c0";
        let removed_mirror_key = "matrix/right-synthetic-keys/r0c0";
        let operation = json!({"kind":"remove-selected-matrix-cells",
            "cells":{matrix_id:[[0,0]]}});

        preview_without_publishing(
            &mut engine,
            &before,
            &format!("parent19-membership-preview-{author}"),
            operation.clone(),
            &[removed_source_key, removed_mirror_key],
        );
        let committed = matrix_commit(
            &mut engine,
            &before,
            &format!("parent19-membership-commit-{author}"),
            operation,
            &[removed_source_key, removed_mirror_key],
        );
        assert_eq!(committed["kind"], "scene", "{committed}");
        let after = &committed["document"];
        for (matrix_id, key_id) in [
            (SOURCE_MATRIX, removed_source_key),
            (MIRROR_MATRIX, removed_mirror_key),
        ] {
            assert!(
                find_part(after, key_id).is_none(),
                "removed key {key_id} remains"
            );
            assert_deleted_cell_tombstone(after, matrix_id, 0, 0);
            let cells = matrix(after, matrix_id)["cells"].as_array().unwrap();
            assert!(
                cells
                    .iter()
                    .any(|cell| cell["row"] == 0 && cell["column"] == 1),
                "{matrix_id} lost the unaffected sibling cell"
            );
        }
        assert!(find_part(after, "matrix/left-synthetic-keys/r0c0/diode").is_none());
        assert!(find_part(after, "matrix/right-synthetic-keys/r0c0/diode").is_none());
        assert!(find_part(after, "matrix/left-synthetic-keys/r0c0/led").is_none());
        assert!(find_part(after, "matrix/right-synthetic-keys/r0c0/led").is_none());
        assert!(
            find_part(after, "matrix/left-synthetic-keys/r0c1").is_some()
                && find_part(after, "matrix/right-synthetic-keys/r0c1").is_some(),
            "an unaffected sibling key remains on both halves"
        );
        assert_thumb_matrix_unchanged(&before, after);
        assert_document_history(&mut engine, &before, after, &pair_id);
    }
}

#[test]
fn parent19_matrix_keycap_settings_and_key_legend_project_from_either_half() {
    for author in ["left", "right"] {
        let pair_id = format!("parent19-matrix-keycaps-{author}");
        let (mut engine, before) = accepted_matrix_pair(&pair_id);
        let matrix_id = if author == "left" {
            SOURCE_MATRIX
        } else {
            MIRROR_MATRIX
        };
        let source_key = "matrix/left-synthetic-keys/r0c1";
        let mirror_key = "matrix/right-synthetic-keys/r0c1";
        let matrix_operation = json!({"kind":"set-matrix-keycaps","matrixId":matrix_id,
            "change":{"kind":"first-row","value":3}});

        preview_without_publishing(
            &mut engine,
            &before,
            &format!("parent19-keycap-matrix-preview-{author}"),
            matrix_operation.clone(),
            &[source_key, mirror_key],
        );
        let matrix_applied = matrix_commit(
            &mut engine,
            &before,
            &format!("parent19-keycap-matrix-commit-{author}"),
            matrix_operation,
            &[source_key, mirror_key],
        );
        assert_eq!(matrix_applied["kind"], "scene", "{matrix_applied}");
        for id in [SOURCE_MATRIX, MIRROR_MATRIX] {
            assert_eq!(
                matrix_applied["document"]["keycaps"]["matrices"][id]["firstRow"], 3,
                "matrix keycap setting for {id}"
            );
        }
        assert_thumb_matrix_unchanged(&before, &matrix_applied["document"]);
        assert_document_history(&mut engine, &before, &matrix_applied["document"], &pair_id);

        let before_legend = snapshot(&mut engine);
        let key_id = if author == "left" {
            source_key
        } else {
            mirror_key
        };
        let legend_operation = json!({"kind":"set-keycap-key","keyId":key_id,
            "change":{"kind":"legend","value":"Esc"}});
        preview_without_publishing(
            &mut engine,
            &before_legend,
            &format!("parent19-keycap-key-preview-{author}"),
            legend_operation.clone(),
            &[source_key, mirror_key],
        );
        let legend_commit = matrix_commit(
            &mut engine,
            &before_legend,
            &format!("parent19-keycap-key-commit-{author}"),
            legend_operation,
            &[source_key, mirror_key],
        );
        assert_eq!(legend_commit["kind"], "scene", "{legend_commit}");
        for id in [source_key, mirror_key] {
            assert_eq!(
                legend_commit["document"]["keycaps"]["keys"][id]["legend"], "Esc",
                "keycap legend for {id}"
            );
        }
        assert_thumb_matrix_unchanged(&before_legend, &legend_commit["document"]);
        assert_document_history(
            &mut engine,
            &before_legend,
            &legend_commit["document"],
            &format!("{pair_id}-legend"),
        );
    }
}

fn expanded_matrix(document: &Value, matrix_id: &str) -> Value {
    let mut expanded = matrix(document, matrix_id).clone();
    expanded["columns"] = json!(3);
    let additions: Vec<Value> = (0..2)
        .map(|row| {
            let mut cell = expanded["cells"]
                .as_array()
                .unwrap()
                .iter()
                .find(|cell| cell["row"] == row && cell["column"] == 1)
                .unwrap()
                .clone();
            cell["column"] = json!(2);
            cell
        })
        .collect();
    expanded["cells"].as_array_mut().unwrap().extend(additions);
    expanded
}

#[test]
fn parent19_set_matrix_replacement_projects_from_either_half_and_preserves_thumb_matrix() {
    for author in ["left", "right"] {
        let pair_id = format!("parent19-matrix-replacement-{author}");
        let (mut engine, before) = accepted_matrix_pair(&pair_id);
        let author_matrix = if author == "left" {
            SOURCE_MATRIX
        } else {
            MIRROR_MATRIX
        };
        let source_origin_x = if author == "left" { 2.0 } else { -2.0 };
        let mut replacement = matrix(&before, author_matrix).clone();
        replacement["origin"]["x"] = json!(replacement["origin"]["x"].as_f64().unwrap() + 2.0);
        let operation = json!({"kind":"set-matrix","matrix":replacement});

        preview_without_publishing(
            &mut engine,
            &before,
            &format!("parent19-matrix-replacement-preview-{author}"),
            operation.clone(),
            &[
                "matrix/left-synthetic-keys/r0c0",
                "matrix/right-synthetic-keys/r0c0",
            ],
        );
        let committed = matrix_commit(
            &mut engine,
            &before,
            &format!("parent19-matrix-replacement-commit-{author}"),
            operation,
            &[
                "matrix/left-synthetic-keys/r0c0",
                "matrix/right-synthetic-keys/r0c0",
            ],
        );
        assert_eq!(committed["kind"], "scene", "{committed}");
        let after = &committed["document"];
        assert_eq!(matrix(after, SOURCE_MATRIX)["origin"]["x"], source_origin_x);
        assert_eq!(
            matrix(after, MIRROR_MATRIX)["origin"]["x"],
            200.0 - source_origin_x
        );
        assert_projected_key_pose(
            after,
            "matrix/left-synthetic-keys/r0c0",
            "matrix/right-synthetic-keys/r0c0",
            source_origin_x,
            0.0,
        );
        assert_thumb_matrix_unchanged(&before, after);
        assert_document_history(&mut engine, &before, after, &pair_id);
    }
}

#[test]
fn parent19_set_matrix_expansion_projects_from_either_half_and_preserves_thumb_matrix() {
    for author in ["left", "right"] {
        let pair_id = format!("parent19-matrix-expansion-{author}");
        let (mut engine, before) = accepted_matrix_pair(&pair_id);
        let author_matrix = if author == "left" {
            SOURCE_MATRIX
        } else {
            MIRROR_MATRIX
        };
        let source_new_key = "matrix/left-synthetic-keys/r0c2";
        let mirror_new_key = "matrix/right-synthetic-keys/r0c2";
        let operation =
            json!({"kind":"set-matrix","matrix":expanded_matrix(&before, author_matrix)});

        preview_without_publishing(
            &mut engine,
            &before,
            &format!("parent19-matrix-expansion-preview-{author}"),
            operation.clone(),
            &[source_new_key, mirror_new_key],
        );
        let committed = matrix_commit(
            &mut engine,
            &before,
            &format!("parent19-matrix-expansion-commit-{author}"),
            operation,
            &[source_new_key, mirror_new_key],
        );
        assert_eq!(committed["kind"], "scene", "{committed}");
        let after = &committed["document"];
        for matrix_id in [SOURCE_MATRIX, MIRROR_MATRIX] {
            assert_eq!(matrix(after, matrix_id)["columns"], 3, "{matrix_id}");
            assert!(
                matrix(after, matrix_id)["cells"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|cell| cell["row"] == 0 && cell["column"] == 2),
                "{matrix_id} lacks the new cell"
            );
        }
        assert_projected_key_pose(after, source_new_key, mirror_new_key, 38.0, 0.0);
        assert!(find_part(after, &format!("{source_new_key}/diode")).is_some());
        assert!(find_part(after, &format!("{mirror_new_key}/diode")).is_some());
        assert!(find_part(after, &format!("{source_new_key}/led")).is_some());
        assert!(find_part(after, &format!("{mirror_new_key}/led")).is_some());
        assert_thumb_matrix_unchanged(&before, after);
        assert_document_history(&mut engine, &before, after, &pair_id);
    }
}

fn one_cell_selection(matrix_id: &str) -> Value {
    json!({(matrix_id): [[0, 0]]})
}

fn assert_cell_enabled(document: &Value, matrix_id: &str, expected: bool) {
    let cell = matrix(document, matrix_id)["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|cell| cell["row"] == 0 && cell["column"] == 0)
        .unwrap_or_else(|| panic!("{matrix_id} lacks cell r0c0"));
    assert_eq!(cell["enabled"], expected, "{matrix_id} r0c0");
}

fn assert_deleted_cell_tombstone(document: &Value, matrix_id: &str, row: u32, column: u32) {
    let cells = matrix(document, matrix_id)["cells"].as_array().unwrap();
    let matches: Vec<_> = cells
        .iter()
        .filter(|cell| cell["row"] == row && cell["column"] == column)
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "{matrix_id} must contain exactly one tombstone for r{row}c{column}"
    );
    let tombstone = matches[0];
    assert_eq!(tombstone["deleted"], true);
    assert_eq!(tombstone["enabled"], false);
    assert!(
        tombstone
            .get("assemblies")
            .is_none_or(|assemblies| { assemblies.as_array().is_some_and(Vec::is_empty) }),
        "empty tombstone assemblies are omitted during serialization: {tombstone}"
    );
}

fn assert_key_absent(document: &Value, key_id: &str) {
    assert!(
        find_part(document, key_id).is_none(),
        "key {key_id} remains"
    );
    assert!(find_part(document, &format!("{key_id}/diode")).is_none());
    assert!(find_part(document, &format!("{key_id}/led")).is_none());
}

fn assert_key_present(document: &Value, key_id: &str) {
    assert!(
        find_part(document, key_id).is_some(),
        "key {key_id} is missing"
    );
    assert!(find_part(document, &format!("{key_id}/diode")).is_some());
    assert!(find_part(document, &format!("{key_id}/led")).is_some());
}

#[test]
fn parent19_matrix_cell_enable_and_disable_project_from_either_half() {
    for author in ["left", "right"] {
        let pair_id = format!("parent19-matrix-enable-{author}");
        let (mut engine, before) = accepted_matrix_pair(&pair_id);
        let author_matrix = if author == "left" {
            SOURCE_MATRIX
        } else {
            MIRROR_MATRIX
        };
        let source_key = "matrix/left-synthetic-keys/r0c0";
        let mirror_key = "matrix/right-synthetic-keys/r0c0";
        let disable = json!({"kind":"set-matrix-cells-enabled",
            "cells":one_cell_selection(author_matrix),"enabled":false});

        preview_without_publishing(
            &mut engine,
            &before,
            &format!("parent19-matrix-disable-preview-{author}"),
            disable.clone(),
            &[source_key, mirror_key],
        );
        let disabled = matrix_commit(
            &mut engine,
            &before,
            &format!("parent19-matrix-disable-commit-{author}"),
            disable,
            &[source_key, mirror_key],
        );
        assert_eq!(disabled["kind"], "scene", "{disabled}");
        for id in [SOURCE_MATRIX, MIRROR_MATRIX] {
            assert_cell_enabled(&disabled["document"], id, false);
        }
        assert_key_absent(&disabled["document"], source_key);
        assert_key_absent(&disabled["document"], mirror_key);
        assert_thumb_matrix_unchanged(&before, &disabled["document"]);
        assert_document_history(&mut engine, &before, &disabled["document"], &pair_id);

        let before_enable = snapshot(&mut engine);
        let enable = json!({"kind":"set-matrix-cells-enabled",
            "cells":one_cell_selection(author_matrix),"enabled":true});
        preview_without_publishing(
            &mut engine,
            &before_enable,
            &format!("parent19-matrix-enable-preview-{author}"),
            enable.clone(),
            &[source_key, mirror_key],
        );
        let enabled = matrix_commit(
            &mut engine,
            &before_enable,
            &format!("parent19-matrix-enable-commit-{author}"),
            enable,
            &[source_key, mirror_key],
        );
        assert_eq!(enabled["kind"], "scene", "{enabled}");
        for id in [SOURCE_MATRIX, MIRROR_MATRIX] {
            assert_cell_enabled(&enabled["document"], id, true);
        }
        assert_key_present(&enabled["document"], source_key);
        assert_key_present(&enabled["document"], mirror_key);
        assert_thumb_matrix_unchanged(&before_enable, &enabled["document"]);
        assert_document_history(
            &mut engine,
            &before_enable,
            &enabled["document"],
            &format!("{pair_id}-reenable"),
        );
    }
}

#[test]
fn parent19_matrix_cell_deletion_projects_from_either_half_and_keeps_sibling_matrix() {
    for author in ["left", "right"] {
        let pair_id = format!("parent19-matrix-cell-delete-{author}");
        let (mut engine, before) = accepted_matrix_pair(&pair_id);
        let author_matrix = if author == "left" {
            SOURCE_MATRIX
        } else {
            MIRROR_MATRIX
        };
        let source_key = "matrix/left-synthetic-keys/r0c0";
        let mirror_key = "matrix/right-synthetic-keys/r0c0";
        let operation = json!({"kind":"remove-matrix-cells","matrixId":author_matrix,
            "cells":[[0, 0]]});

        preview_without_publishing(
            &mut engine,
            &before,
            &format!("parent19-matrix-cell-delete-preview-{author}"),
            operation.clone(),
            &[source_key, mirror_key],
        );
        let committed = matrix_commit(
            &mut engine,
            &before,
            &format!("parent19-matrix-cell-delete-commit-{author}"),
            operation,
            &[source_key, mirror_key],
        );
        assert_eq!(committed["kind"], "scene", "{committed}");
        for (matrix_id, key_id) in [(SOURCE_MATRIX, source_key), (MIRROR_MATRIX, mirror_key)] {
            assert_key_absent(&committed["document"], key_id);
            assert_deleted_cell_tombstone(&committed["document"], matrix_id, 0, 0);
        }
        assert_thumb_matrix_unchanged(&before, &committed["document"]);
        assert_document_history(&mut engine, &before, &committed["document"], &pair_id);
    }
}

#[test]
fn parent19_matrix_row_and_column_removal_project_from_either_half() {
    for author in ["left", "right"] {
        for axis in ["row", "column"] {
            let pair_id = format!("parent19-matrix-remove-{axis}-{author}");
            let (mut engine, before) = accepted_matrix_pair(&pair_id);
            let author_matrix = if author == "left" {
                SOURCE_MATRIX
            } else {
                MIRROR_MATRIX
            };
            let (operation, removed_source, removed_mirror) = if axis == "row" {
                (
                    json!({"kind":"remove-matrix-row","matrixId":author_matrix,"row":1}),
                    [
                        "matrix/left-synthetic-keys/r1c0",
                        "matrix/left-synthetic-keys/r1c1",
                    ],
                    [
                        "matrix/right-synthetic-keys/r1c0",
                        "matrix/right-synthetic-keys/r1c1",
                    ],
                )
            } else {
                (
                    json!({"kind":"remove-matrix-column","matrixId":author_matrix,"column":1}),
                    [
                        "matrix/left-synthetic-keys/r0c1",
                        "matrix/left-synthetic-keys/r1c1",
                    ],
                    [
                        "matrix/right-synthetic-keys/r0c1",
                        "matrix/right-synthetic-keys/r1c1",
                    ],
                )
            };

            preview_without_publishing(
                &mut engine,
                &before,
                &format!("parent19-matrix-remove-{axis}-preview-{author}"),
                operation.clone(),
                &[],
            );
            let committed = matrix_commit(
                &mut engine,
                &before,
                &format!("parent19-matrix-remove-{axis}-commit-{author}"),
                operation,
                &[],
            );
            assert_eq!(committed["kind"], "scene", "{committed}");
            let after = &committed["document"];
            let dimension = if axis == "row" { "rows" } else { "columns" };
            for matrix_id in [SOURCE_MATRIX, MIRROR_MATRIX] {
                assert_eq!(matrix(after, matrix_id)[dimension], 1, "{matrix_id}");
            }
            for key_id in removed_source.into_iter().chain(removed_mirror) {
                assert_key_absent(after, key_id);
            }
            assert!(find_part(after, "matrix/left-synthetic-keys/r0c0").is_some());
            assert!(find_part(after, "matrix/right-synthetic-keys/r0c0").is_some());
            assert_thumb_matrix_unchanged(&before, after);
            assert_document_history(&mut engine, &before, after, &pair_id);
        }
    }
}

#[test]
fn parent19_whole_matrix_removal_projects_from_either_half_and_keeps_thumb_pair() {
    for author in ["left", "right"] {
        let pair_id = format!("parent19-matrix-remove-whole-{author}");
        let (mut engine, before) = accepted_matrix_pair(&pair_id);
        let author_matrix = if author == "left" {
            SOURCE_MATRIX
        } else {
            MIRROR_MATRIX
        };
        let operation = json!({"kind":"remove-matrix","id":author_matrix});

        preview_without_publishing(
            &mut engine,
            &before,
            &format!("parent19-matrix-remove-whole-preview-{author}"),
            operation.clone(),
            &[],
        );
        let committed = matrix_commit(
            &mut engine,
            &before,
            &format!("parent19-matrix-remove-whole-commit-{author}"),
            operation,
            &[],
        );
        assert_eq!(committed["kind"], "scene", "{committed}");
        for matrix_id in [SOURCE_MATRIX, MIRROR_MATRIX] {
            assert!(
                committed["document"]["matrices"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|matrix| matrix["id"] != matrix_id),
                "{matrix_id} remains"
            );
        }
        for row in 0..2 {
            for column in 0..2 {
                assert_key_absent(
                    &committed["document"],
                    &format!("matrix/left-synthetic-keys/r{row}c{column}"),
                );
                assert_key_absent(
                    &committed["document"],
                    &format!("matrix/right-synthetic-keys/r{row}c{column}"),
                );
            }
        }
        assert_thumb_matrix_unchanged(&before, &committed["document"]);
        assert_document_history(&mut engine, &before, &committed["document"], &pair_id);
    }
}

//! Activated-route acceptance for matrix geometry, membership, and keycap edits.
//!
//! The shared synthetic fixture contains one matrix per pair. These cases exercise source and
//! mirror authorship, Preview isolation, exact paired effects, and one-step document history.

use super::parent_acceptance::{assert_history_document, save_archive_and_open};
use super::*;

const SOURCE_MATRIX: &str = "left-synthetic-keys";
const MIRROR_MATRIX: &str = "right-synthetic-keys";
const SOURCE_THUMB_MATRIX: &str = "left-thumb-keys";
const MIRROR_THUMB_MATRIX: &str = "right-thumb-keys";
const SRC_R0C0: &str = "matrix/left-synthetic-keys/r0c0";
const MIR_R0C0: &str = "matrix/right-synthetic-keys/r0c0";
const SRC_R0C1: &str = "matrix/left-synthetic-keys/r0c1";
const MIR_R0C1: &str = "matrix/right-synthetic-keys/r0c1";
const SRC_R1C1: &str = "matrix/left-synthetic-keys/r1c1";
const MIR_R1C1: &str = "matrix/right-synthetic-keys/r1c1";
const SRC_R0C2: &str = "matrix/left-synthetic-keys/r0c2";
const MIR_R0C2: &str = "matrix/right-synthetic-keys/r0c2";

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
        let (source_key, mirror_key) = (SRC_R0C0, MIR_R0C0);
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
        json!({"kind":"move-parts","positions":[{"id":SRC_R0C0,
            "at":part(&accepted_at_cursor,SRC_R0C0)["pose"]["at"]}]}),
        &[SRC_R0C0],
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
        &[SRC_R0C0],
    );
    assert_eq!(removal["kind"], "scene", "{removal}");
    let retired_id = SRC_R0C0;
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
    assert_history_document(&undone["document"], before, &format!("{label} Undo"));

    let redone = request(engine, json!({"id":format!("{label}-redo"),"kind":"redo"}));
    assert_eq!(redone["kind"], "scene", "{label} Redo: {redone}");
    assert_history_document(&redone["document"], committed, &format!("{label} Redo"));
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

// W5 table-driven operation sequences: one runner covers the eight repetitive
// matrix geometry/membership cases (poses/tombstones/dimensions preserved below)
// plus linked/local/omitted, reverse authorship, matrixless U1 move, no-op,
// preview/commit, archive reopen and history. Refusals stay in the untouched
// retired-target test (exact messages, no count-only proof).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MatrixSequenceKind {
    Placement,
    Splay,
    MembershipRemoval,
    CellDeletion,
    RowRemoval,
    ColumnRemoval,
    WholeRemoval,
    Replacement,
    Expansion,
    MatrixlessMove,
}
fn sequence_operation(
    kind: MatrixSequenceKind,
    author: &str,
    before: &Value,
) -> (Value, Vec<&'static str>) {
    let m = if author == "left" {
        SOURCE_MATRIX
    } else {
        MIRROR_MATRIX
    };
    match kind {
        MatrixSequenceKind::Placement => (
            json!({"kind":"set-matrix-placement","matrixId":m,
                "change":{"kind":"column-stagger","column":1,"value":4.0}}),
            vec![SRC_R0C1, MIR_R0C1],
        ),
        MatrixSequenceKind::Splay => (
            json!({"kind":"set-matrix-splay","matrixId":m,"column":1,
                "change":{"kind":"angle","angle":12.0,"affect":"column"}}),
            vec![SRC_R1C1, MIR_R1C1],
        ),
        MatrixSequenceKind::MembershipRemoval => (
            json!({"kind":"remove-selected-matrix-cells","cells":{m:[[0,0]]}}),
            vec![SRC_R0C0, MIR_R0C0],
        ),
        MatrixSequenceKind::CellDeletion => (
            json!({"kind":"remove-matrix-cells","matrixId":m,"cells":[[0,0]]}),
            vec![SRC_R0C0, MIR_R0C0],
        ),
        MatrixSequenceKind::RowRemoval => (
            json!({"kind":"remove-matrix-row","matrixId":m,"row":1}),
            vec![],
        ),
        MatrixSequenceKind::ColumnRemoval => (
            json!({"kind":"remove-matrix-column","matrixId":m,"column":1}),
            vec![],
        ),
        MatrixSequenceKind::WholeRemoval => (json!({"kind":"remove-matrix","id":m}), vec![]),
        MatrixSequenceKind::Replacement => {
            let mut r = matrix(before, m).clone();
            r["origin"]["x"] = json!(r["origin"]["x"].as_f64().unwrap() + 2.0);
            (
                json!({"kind":"set-matrix","matrix":r}),
                vec![SRC_R0C0, MIR_R0C0],
            )
        }
        MatrixSequenceKind::Expansion => (
            json!({"kind":"set-matrix","matrix":expanded_matrix(before, m)}),
            vec![SRC_R0C2, MIR_R0C2],
        ),
        MatrixSequenceKind::MatrixlessMove => {
            let (id, at) = if author == "left" {
                ("left/U1", json!({"x":-16.0,"y":14.0}))
            } else {
                ("right/U1", json!({"x":216.0,"y":14.0}))
            };
            (
                json!({"kind":"move-parts","positions":[{"id":id,"at":at}]}),
                vec!["left/U1", "right/U1"],
            )
        }
    }
}
fn verify_matrix_sequence(kind: MatrixSequenceKind, author: &str, before: &Value, after: &Value) {
    match kind {
        MatrixSequenceKind::Placement => {
            assert_projected_key_pose(after, SRC_R0C1, MIR_R0C1, 19.0, 4.0);
            assert_eq!(
                matrix(after, SOURCE_MATRIX)["cells"],
                matrix(before, SOURCE_MATRIX)["cells"]
            );
            assert_eq!(
                matrix(after, MIRROR_MATRIX)["cells"],
                matrix(before, MIRROR_MATRIX)["cells"]
            );
        }
        MatrixSequenceKind::Splay => {
            let x = 19.0 - 21.0 * 12.0_f64.to_radians().sin();
            let y = 21.0 * 12.0_f64.to_radians().cos();
            assert_projected_key_pose(after, SRC_R1C1, MIR_R1C1, x, y);
        }
        MatrixSequenceKind::MembershipRemoval => {
            for (mid, key) in [(SOURCE_MATRIX, SRC_R0C0), (MIRROR_MATRIX, MIR_R0C0)] {
                assert!(find_part(after, key).is_none(), "removed {key} remains");
                assert_deleted_cell_tombstone(after, mid, 0, 0);
            }
            for key in [
                "matrix/left-synthetic-keys/r0c0/diode",
                "matrix/right-synthetic-keys/r0c0/diode",
                "matrix/left-synthetic-keys/r0c0/led",
                "matrix/right-synthetic-keys/r0c0/led",
            ] {
                assert!(find_part(after, key).is_none(), "companion {key} remains");
            }
            assert!(find_part(after, SRC_R0C1).is_some());
            assert!(find_part(after, MIR_R0C1).is_some());
        }
        MatrixSequenceKind::CellDeletion => {
            for (mid, key) in [(SOURCE_MATRIX, SRC_R0C0), (MIRROR_MATRIX, MIR_R0C0)] {
                assert_key_absent(after, key);
                assert_deleted_cell_tombstone(after, mid, 0, 0);
            }
        }
        MatrixSequenceKind::RowRemoval => {
            for mid in [SOURCE_MATRIX, MIRROR_MATRIX] {
                assert_eq!(matrix(after, mid)["rows"], 1, "{mid}");
            }
            for key in [
                "matrix/left-synthetic-keys/r1c0",
                SRC_R1C1,
                "matrix/right-synthetic-keys/r1c0",
                MIR_R1C1,
            ] {
                assert_key_absent(after, key);
            }
            assert!(find_part(after, SRC_R0C0).is_some());
            assert!(find_part(after, MIR_R0C0).is_some());
        }
        MatrixSequenceKind::ColumnRemoval => {
            for mid in [SOURCE_MATRIX, MIRROR_MATRIX] {
                assert_eq!(matrix(after, mid)["columns"], 1, "{mid}");
            }
            for key in [SRC_R0C1, SRC_R1C1, MIR_R0C1, MIR_R1C1] {
                assert_key_absent(after, key);
            }
            assert!(find_part(after, SRC_R0C0).is_some());
            assert!(find_part(after, MIR_R0C0).is_some());
        }
        MatrixSequenceKind::WholeRemoval => {
            for mid in [SOURCE_MATRIX, MIRROR_MATRIX] {
                assert!(
                    after["matrices"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .all(|m| m["id"] != mid),
                    "{mid} remains"
                );
            }
            for row in 0..2 {
                for col in 0..2 {
                    assert_key_absent(after, &format!("matrix/left-synthetic-keys/r{row}c{col}"));
                    assert_key_absent(after, &format!("matrix/right-synthetic-keys/r{row}c{col}"));
                }
            }
        }
        MatrixSequenceKind::Replacement => {
            let sx = if author == "left" { 2.0 } else { -2.0 };
            assert_eq!(matrix(after, SOURCE_MATRIX)["origin"]["x"], sx);
            assert_eq!(matrix(after, MIRROR_MATRIX)["origin"]["x"], 200.0 - sx);
            assert_projected_key_pose(after, SRC_R0C0, MIR_R0C0, sx, 0.0);
        }
        MatrixSequenceKind::Expansion => {
            for mid in [SOURCE_MATRIX, MIRROR_MATRIX] {
                assert_eq!(matrix(after, mid)["columns"], 3, "{mid}");
            }
            assert_projected_key_pose(after, SRC_R0C2, MIR_R0C2, 38.0, 0.0);
            for key in [
                "matrix/left-synthetic-keys/r0c2/diode",
                "matrix/right-synthetic-keys/r0c2/diode",
                "matrix/left-synthetic-keys/r0c2/led",
                "matrix/right-synthetic-keys/r0c2/led",
            ] {
                assert!(find_part(after, key).is_some(), "{key} missing");
            }
        }
        MatrixSequenceKind::MatrixlessMove => {
            assert_eq!(
                part(after, "left/U1")["pose"]["at"],
                json!({"x":-16.0,"y":14.0})
            );
            assert_eq!(
                part(after, "right/U1")["pose"]["at"],
                json!({"x":216.0,"y":14.0})
            );
        }
    }
}
#[test]
fn parent19_matrix_operation_sequences_cover_linked_local_omitted_reverse_matrixless_noop_preview_commit_reopen_history()
 {
    let kinds = [
        MatrixSequenceKind::Placement,
        MatrixSequenceKind::Splay,
        MatrixSequenceKind::MembershipRemoval,
        MatrixSequenceKind::CellDeletion,
        MatrixSequenceKind::RowRemoval,
        MatrixSequenceKind::ColumnRemoval,
        MatrixSequenceKind::WholeRemoval,
        MatrixSequenceKind::Replacement,
        MatrixSequenceKind::Expansion,
        MatrixSequenceKind::MatrixlessMove,
    ];
    for kind in kinds {
        for author in ["left", "right"] {
            let states: &[&str] = match kind {
                MatrixSequenceKind::Placement | MatrixSequenceKind::MatrixlessMove => {
                    &["linked", "local", "omitted"]
                }
                _ => &["linked"],
            };
            for &state in states {
                let label = format!("{kind:?}-{author}-{state}");
                let pair_id =
                    format!("parent19-matrix-seq-{kind:?}-{author}-{state}").to_lowercase();
                let (mut engine, linked) = accepted_matrix_pair(&pair_id);
                let before = if state == "linked" {
                    linked.clone()
                } else {
                    let reply = request(
                        &mut engine,
                        json!({"id":format!("{pair_id}-exc"),"kind":"edit","command":{
                            "baseRevision":linked["revision"],
                            "transactionId":format!("{pair_id}-exc"),
                            "phase":"commit","targetIds":["right/RST"],
                            "operation":{"kind":"set-half-exception","pairId":pair_id,
                                "itemKind":"part","mirrorId":"right/RST","state":state}}}),
                    );
                    assert_eq!(reply["kind"], "scene", "{label}: {reply}");
                    reply["document"].clone()
                };
                let original_rst = find_part(&linked, "right/RST").cloned();
                let (operation, targets) = sequence_operation(kind, author, &before);
                preview_without_publishing(
                    &mut engine,
                    &before,
                    &format!("{pair_id}-preview"),
                    operation.clone(),
                    &targets,
                );
                let committed = matrix_commit(
                    &mut engine,
                    &before,
                    &format!("{pair_id}-commit"),
                    operation.clone(),
                    &targets,
                );
                assert_eq!(committed["kind"], "scene", "{label}: {committed}");
                let after = committed["document"].clone();
                assert_eq!(
                    after["revision"],
                    before["revision"].as_u64().unwrap() + 1,
                    "{label}"
                );
                verify_matrix_sequence(kind, author, &before, &after);
                assert_thumb_matrix_unchanged(&before, &after);
                let changed: Vec<String> = committed["scene"]["changedIds"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|id| id.as_str().unwrap().to_owned())
                    .collect();
                let mut sorted = changed.clone();
                sorted.sort();
                assert_eq!(changed, sorted, "{label}: changedIds ordered {changed:?}");
                assert_eq!(
                    changed.len(),
                    sorted
                        .iter()
                        .collect::<std::collections::BTreeSet<_>>()
                        .len(),
                    "{label}: changedIds unique"
                );
                for &id in &targets {
                    assert!(
                        changed.iter().any(|c| c.as_str() == id),
                        "{label}: {id} in {changed:?}"
                    );
                }
                let exceptions = pair(&after, &pair_id)["halves"]
                    .get("exceptions")
                    .cloned()
                    .unwrap_or(json!([]));
                if state == "linked" {
                    assert!(find_part(&after, "right/RST").is_some(), "{label}");
                } else {
                    assert_eq!(
                        exceptions
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|e| e["mirrorId"] == "right/RST")
                            .unwrap()["state"],
                        state,
                        "{label}"
                    );
                    if state == "local" {
                        assert_eq!(
                            find_part(&after, "right/RST"),
                            original_rst.as_ref(),
                            "{label}"
                        );
                    } else {
                        assert!(find_part(&after, "right/RST").is_none(), "{label}");
                    }
                }
                if matches!(
                    kind,
                    MatrixSequenceKind::Placement
                        | MatrixSequenceKind::Splay
                        | MatrixSequenceKind::Replacement
                        | MatrixSequenceKind::Expansion
                        | MatrixSequenceKind::MatrixlessMove
                ) {
                    let noop = matrix_commit(
                        &mut engine,
                        &after,
                        &format!("{pair_id}-noop"),
                        operation.clone(),
                        &targets,
                    );
                    assert_eq!(noop["kind"], "scene", "{label}: {noop}");
                    assert_eq!(noop["document"], after, "{label}: no-op preserves document");
                }
                let (_, reopened) = save_archive_and_open(&after);
                assert_eq!(
                    pair(&reopened, &pair_id)["halves"]["correspondences"],
                    pair(&after, &pair_id)["halves"]["correspondences"],
                    "{label}: correspondences persist"
                );
                assert_thumb_matrix_unchanged(&after, &reopened);
                assert_document_history(&mut engine, &before, &after, &pair_id);
            }
        }
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
        let source_key = SRC_R0C1;
        let mirror_key = MIR_R0C1;
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

#[test]
fn keycap_edits_follow_current_exception_state_through_history_and_open() {
    let pair_id = "keycap-settings-history";
    let (mut engine, before) = accepted_matrix_pair(pair_id);
    let board_color = |board_id| {
        json!({"kind":"set-keycap-board","boardId":board_id,
            "change":{"kind":"color","value":"#123456"}})
    };
    preview_without_publishing(
        &mut engine,
        &before,
        "preview-board-color",
        board_color("right"),
        &["right"],
    );
    let colored = matrix_commit(
        &mut engine,
        &before,
        "board-color",
        board_color("right"),
        &["right"],
    )["document"]
        .clone();
    for board in ["left", "right"] {
        assert_eq!(colored["keycaps"]["boards"][board]["color"], "#123456");
    }
    let repeated = matrix_commit(
        &mut engine,
        &colored,
        "repeat-color",
        board_color("left"),
        &["left"],
    );
    assert_eq!(repeated["document"], colored);
    assert_document_history(&mut engine, &before, &colored, "board-color");

    let legend = |part_id, value| json!({"kind":"set-keycap-key","keyId":part_id,"change":{"kind":"legend","value":value}});
    for (state, mirror_legend) in [
        ("local", json!("OWN")),
        ("omitted", Value::Null),
        ("linked", json!("linked")),
    ] {
        let before = snapshot(&mut engine);
        let exception = matrix_commit(
            &mut engine,
            &before,
            state,
            json!({"kind":"set-half-exception","pairId":pair_id,
                "itemKind":"part","mirrorId":"right/RST","state":state}),
            &["right/RST"],
        )["document"]
            .clone();
        let before_source_edit = if state == "local" {
            matrix_commit(
                &mut engine,
                &exception,
                "mirror-legend",
                legend("right/RST", "OWN"),
                &["right/RST"],
            )["document"]
                .clone()
        } else {
            exception
        };
        let after = matrix_commit(
            &mut engine,
            &before_source_edit,
            "source-legend",
            legend("left/RST", state),
            &["left/RST"],
        )["document"]
            .clone();
        assert_eq!(after["keycaps"]["keys"]["left/RST"]["legend"], state);
        assert_eq!(
            after["keycaps"]["keys"]["right/RST"]["legend"],
            mirror_legend
        );
        assert_eq!(after["hardware"], before_source_edit["hardware"]);
        assert_eq!(after["parts"], before_source_edit["parts"]);
        if state == "omitted" {
            assert!(after["keycaps"]["keys"].get("right/RST").is_none());
        }
        assert_document_history(&mut engine, &before_source_edit, &after, state);
        let (_, reopened) = open(&after);
        assert_eq!(reopened["keycaps"], after["keycaps"]);
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
        let source_key = SRC_R0C0;
        let mirror_key = MIR_R0C0;
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

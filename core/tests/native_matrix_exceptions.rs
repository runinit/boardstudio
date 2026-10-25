//! Native matrix-level exception transitions (ticket 29).
//!
//! Full ticket-21 mirror on the mirrored-half pair: matrix-level Local/Omitted
//! plus membership-change transitions go through the reconciliation kernel
//! instead of refusing at `validate_matrix_association`. Same-board behavior
//! stays owned by ticket 21; every case below runs through the public
//! `Open`/`Edit` routes (spec §11 seam rule).

use boardstudio_core::CoreEngine;
use serde_json::{Value, json};

use crate::support::synthetic_half;

const PAIR_ID: &str = "native-exceptions-pair";
const SOURCE_MATRIX: &str = "left-synthetic-keys";
const MIRROR_MATRIX: &str = "right-synthetic-keys";

fn request(engine: &mut CoreEngine, input: Value) -> Value {
    let reply: Value = serde_json::from_str(&engine.request(&input.to_string())).unwrap();
    assert_ne!(reply["kind"], "error", "{reply}");
    reply
}

fn open(document: &Value) -> (CoreEngine, Value) {
    let mut engine = CoreEngine::new();
    let reply = request(
        &mut engine,
        json!({"id":"open","kind":"open","document":document}),
    );
    assert_eq!(reply["kind"], "scene", "{reply}");
    let document = reply["document"].clone();
    (engine, document)
}

fn snapshot(engine: &mut CoreEngine) -> Value {
    request(engine, json!({"id":"snapshot","kind":"snapshot"}))["document"].clone()
}

fn seed(document: &Value) -> (CoreEngine, Value) {
    open(document)
}

/// Supported prepared-route fixture: cell defaults inherit the matrix
/// definition, matching the mirrored-half acceptance harness.
fn runtime_supported_document() -> Value {
    let mut document = synthetic_half::document();
    for cell in document["matrices"][0]["cells"].as_array_mut().unwrap() {
        let object = cell.as_object_mut().unwrap();
        object.remove("definitionId");
        object.remove("variant");
        object.remove("assembliesLocal");
    }
    document
}

fn accepted_pair(pair_id: &str) -> (CoreEngine, Value) {
    let (mut engine, seed) = seed(&runtime_supported_document());
    let reply: Value = serde_json::from_str(
        &engine
            .request(
                &json!({"id":"half","kind":"edit","command":{
                    "baseRevision":seed["revision"],"transactionId":"half","phase":"commit",
                    "targetIds":[],
                    "operation":{
                        "kind":"create-mirrored-half","pairId":pair_id,
                        "sourceInstanceId":"left",
                        "mode":{"kind":"new-board","boardId":"right","instanceId":"right"},
                        "mapping":{"at":{"x":200.0,"y":0.0},"rotation":0.0},
                        "idRule":"board-prefix"}}})
                .to_string(),
            )
            .to_string(),
    )
    .unwrap();
    assert_eq!(reply["kind"], "scene", "{reply}");
    let document = reply["document"].clone();
    assert!(matrix(&document, MIRROR_MATRIX).is_object());
    (engine, document)
}

fn matrix<'a>(document: &'a Value, id: &str) -> &'a Value {
    document["matrices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|matrix| matrix["id"] == id)
        .unwrap_or_else(|| panic!("matrix {id} is missing"))
}

fn part<'a>(document: &'a Value, id: &str) -> &'a Value {
    document["parts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == id)
        .unwrap_or_else(|| panic!("part {id} is unavailable"))
}

fn find_part<'a>(document: &'a Value, id: &str) -> Option<&'a Value> {
    document["parts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == id)
}

fn pair<'a>(document: &'a Value, id: &str) -> &'a Value {
    document["hardware"]["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == id)
        .unwrap_or_else(|| panic!("pair {id} is missing"))
}

fn commit(
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

fn undo(engine: &mut CoreEngine, id: &str) -> Value {
    let reply = request(engine, json!({"id":id,"kind":"undo"}));
    assert_eq!(reply["kind"], "scene", "{reply}");
    reply["document"].clone()
}

fn redo(engine: &mut CoreEngine, id: &str) -> Value {
    let reply = request(engine, json!({"id":id,"kind":"redo"}));
    assert_eq!(reply["kind"], "scene", "{reply}");
    reply["document"].clone()
}

fn poses(document: &Value) -> Vec<(String, Value)> {
    document["parts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|part| {
            (
                part["id"].as_str().unwrap().to_owned(),
                part["pose"].clone(),
            )
        })
        .collect()
}

fn assert_poses_unchanged(before: &[(String, Value)], after: &Value) {
    for (id, pose) in before {
        assert_eq!(&part(after, id)["pose"], pose, "{id}");
    }
}

fn assert_mirror_correspondences(document: &Value, label: &str) {
    let halves = &pair(document, PAIR_ID)["halves"];
    for part_id in matrix(document, MIRROR_MATRIX)["partIds"]
        .as_array()
        .unwrap()
    {
        let part_id = part_id.as_str().unwrap();
        assert!(
            halves["correspondences"]
                .as_array()
                .unwrap()
                .iter()
                .any(|record| record["kind"] == "part" && record["mirrorId"] == part_id),
            "{label}: mirror member {part_id} lost its correspondence"
        );
    }
}

#[test]
fn native_resize_keeps_local_mirror_dimensions_and_poses() {
    // Ticket 29 slice B1 (mirrors ticket 21's first slice natively): a public
    // source resize must not rewrite a matrix-level Local mirror. Source grows
    // 2 -> 3 columns; the Local mirror keeps its own 2x2 dimensions and every
    // world pose, with valid correspondences.
    let (mut engine, before) = accepted_pair(PAIR_ID);
    // Make the mirror matrix Local via the public exception route.
    let made_local = commit(
        &mut engine,
        &before,
        "native-make-local",
        json!({"kind":"set-half-exception","pairId":PAIR_ID,
            "itemKind":"matrix","mirrorId":MIRROR_MATRIX,"state":"local"}),
        &[MIRROR_MATRIX],
    );
    assert_eq!(made_local["kind"], "scene", "{made_local}");
    let local_doc = made_local["document"].clone();
    let mirror_before = matrix(&local_doc, MIRROR_MATRIX).clone();
    assert_eq!(mirror_before["rows"], json!(2));
    assert_eq!(mirror_before["columns"], json!(2));
    let poses_before = poses(&local_doc);
    // Grow the source through the public prepared route.
    let mut grown = matrix(&local_doc, SOURCE_MATRIX).clone();
    grown["columns"] = json!(3);
    let resized = commit(
        &mut engine,
        &local_doc,
        "native-grow-source",
        json!({"kind":"set-matrix","matrix":grown}),
        &[],
    );
    assert_eq!(resized["kind"], "scene", "{resized}");
    let after = &resized["document"];
    assert_eq!(matrix(after, SOURCE_MATRIX)["columns"], json!(3));
    // The Local mirror kept its own descriptor byte-for-byte.
    assert_eq!(
        matrix(after, MIRROR_MATRIX),
        &mirror_before,
        "Local mirror descriptor changed"
    );
    // Every pre-existing pose is unchanged, including the Local mirror's.
    assert_poses_unchanged(&poses_before, after);
    // No mirrored newcomer was fabricated for the Local mirror.
    assert!(
        find_part(after, "matrix/right-synthetic-keys/r0c2").is_none()
            && find_part(after, "matrix/right-synthetic-keys/r1c2").is_none(),
        "Local mirror gained projected members"
    );
    assert_mirror_correspondences(after, "resize");
    // One transaction: Undo restores the pre-resize document, Redo restores
    // the resize (revision bumps, so compare content, not whole docs).
    let mut expected_undo = local_doc.clone();
    let undone = undo(&mut engine, "native-resize-undo");
    expected_undo["revision"] = undone["revision"].clone();
    assert_eq!(undone, expected_undo, "Undo must restore the Local document");
    let mut expected_redo = after.clone();
    let redone = redo(&mut engine, "native-resize-redo");
    expected_redo["revision"] = redone["revision"].clone();
    assert_eq!(redone, expected_redo, "Redo must restore the resize");
    assert_eq!(snapshot(&mut engine), redone);
}

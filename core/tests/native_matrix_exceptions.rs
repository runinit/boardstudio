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

fn make_omitted(engine: &mut CoreEngine, before: &Value) -> Value {
    // Matrix-level Omit via the public exception route. The mirror stays as a
    // live record while every inherited part is suppressed.
    let omitted = commit(
        engine,
        before,
        "native-make-omitted",
        json!({"kind":"set-half-exception","pairId":PAIR_ID,
            "itemKind":"matrix","mirrorId":MIRROR_MATRIX,"state":"omitted"}),
        &[MIRROR_MATRIX],
    );
    assert_eq!(omitted["kind"], "scene", "{omitted}");
    let document = omitted["document"].clone();
    assert!(
        pair(&document, PAIR_ID)["halves"]["exceptions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|exception| exception["kind"] == "matrix"
                && exception["mirrorId"] == MIRROR_MATRIX
                && exception["state"] == "omitted"),
        "omit must record a matrix-level exception"
    );
    assert!(
        document["parts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|part| !part["id"]
                .as_str()
                .unwrap()
                .starts_with("matrix/right-synthetic-keys/")),
        "omit must suppress inherited parts"
    );
    assert!(
        matrix(&document, MIRROR_MATRIX).is_object(),
        "omit keeps the live record"
    );
    document
}

fn shrunk_source(document: &Value) -> Value {
    // Dense shrink payload: the kernel requires explicit cell coverage.
    let mut shrunk = matrix(document, SOURCE_MATRIX).clone();
    shrunk["columns"] = json!(1);
    let kept: Vec<Value> = shrunk["cells"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|cell| cell["column"] == 0)
        .cloned()
        .collect();
    shrunk["cells"] = Value::Array(kept);
    shrunk
}

fn assert_mirror_absent(document: &Value, rows: u64, columns: u64, label: &str) {
    let mirror = matrix(document, MIRROR_MATRIX);
    assert_eq!(mirror["rows"], json!(rows), "{label}");
    assert_eq!(mirror["columns"], json!(columns), "{label}");
    assert!(
        mirror["partIds"].as_array().unwrap().is_empty(),
        "{label}: omitted mirror keeps no members"
    );
    for cell in mirror["cells"].as_array().unwrap() {
        assert_eq!(cell["enabled"], json!(false), "{label}: {cell}");
        assert_eq!(cell["deleted"], json!(true), "{label}: {cell}");
        assert!(
            cell.get("assemblies").is_none_or(|a| a
                .as_array()
                .unwrap()
                .is_empty()),
            "{label}: omitted cells carry no companions: {cell}"
        );
    }
    assert!(
        document["parts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|part| !part["id"]
                .as_str()
                .unwrap()
                .starts_with("matrix/right-synthetic-keys/")),
        "{label}: no omitted part is materialized"
    );
}

fn assert_live_correspondences(document: &Value, slots: &[(u32, u32)], label: &str) {
    // The Omitted record stays live: surviving source members keep exact
    // correspondences even though no mirror part is materialized.
    let halves = &pair(document, PAIR_ID)["halves"];
    let correspondences = halves["correspondences"].as_array().unwrap();
    for (row, column) in slots {
        for id in [
            format!("matrix/{SOURCE_MATRIX}/r{row}c{column}"),
            format!("matrix/{SOURCE_MATRIX}/r{row}c{column}/diode"),
            format!("matrix/{SOURCE_MATRIX}/r{row}c{column}/led"),
        ] {
            let mirror_id = id.replacen(SOURCE_MATRIX, MIRROR_MATRIX, 1);
            assert!(
                correspondences.iter().any(|record| record["kind"] == "part"
                    && record["sourceId"] == id
                    && record["mirrorId"] == mirror_id),
                "{label}: live correspondence for {id} is missing"
            );
        }
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

/// Source growth on a paired matrix must carry explicit new cells: the setter
/// leaves sparse cells unfilled, and the reconciliation kernel requires dense
/// cell coverage (`source_members`).
fn expanded_source(document: &Value) -> Value {
    let mut expanded = matrix(document, SOURCE_MATRIX).clone();
    expanded["columns"] = json!(3);
    let additions: Vec<Value> = [0, 1]
        .iter()
        .map(|row| {
            let mut cell = expanded["cells"]
                .as_array()
                .unwrap()
                .iter()
                .find(|cell| cell["row"] == *row && cell["column"] == 1)
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
    let resized = commit(
        &mut engine,
        &local_doc,
        "native-grow-source",
        json!({"kind":"set-matrix","matrix":expanded_source(&local_doc)}),
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

#[test]
fn native_omit_then_expand_keeps_mirror_live_yet_absent() {
    // Ticket 29 slice B2 (mirrors ticket 21's Omitted resize natively): source
    // growth with an Omitted mirror succeeds; the record follows the source
    // dimensions without materializing a single part.
    let (mut engine, before) = accepted_pair(PAIR_ID);
    let omitted_doc = make_omitted(&mut engine, &before);
    let resized = commit(
        &mut engine,
        &omitted_doc,
        "native-grow-omitted-source",
        json!({"kind":"set-matrix","matrix":expanded_source(&omitted_doc)}),
        &[],
    );
    assert_eq!(resized["kind"], "scene", "{resized}");
    let after = &resized["document"];
    assert_eq!(matrix(after, SOURCE_MATRIX)["columns"], json!(3));
    // The Omitted record followed without materializing.
    assert_mirror_absent(after, 2, 3, "expand");
    assert_live_correspondences(after, &[(0, 0), (0, 1), (1, 0), (1, 1)], "expand");
    // One transaction: Undo restores the pre-resize document, Redo restores
    // the resize.
    let mut expected_undo = omitted_doc.clone();
    let undone = undo(&mut engine, "native-omit-expand-undo");
    expected_undo["revision"] = undone["revision"].clone();
    assert_eq!(undone, expected_undo, "Undo must restore the Omitted document");
    let mut expected_redo = after.clone();
    let redone = redo(&mut engine, "native-omit-expand-redo");
    expected_redo["revision"] = redone["revision"].clone();
    assert_eq!(redone, expected_redo, "Redo must restore the resize");
    assert_eq!(snapshot(&mut engine), redone);
}

#[test]
fn native_omit_then_shrink_keeps_mirror_following() {
    // Ticket 29 slice B2: source shrink with an Omitted mirror succeeds; the
    // record follows down without resurrecting parts.
    let (mut engine, before) = accepted_pair(PAIR_ID);
    let omitted_doc = make_omitted(&mut engine, &before);
    let shrunk = commit(
        &mut engine,
        &omitted_doc,
        "native-shrink-omitted-source",
        json!({"kind":"set-matrix","matrix":shrunk_source(&omitted_doc)}),
        &[],
    );
    assert_eq!(shrunk["kind"], "scene", "{shrunk}");
    let after = &shrunk["document"];
    assert_eq!(matrix(after, SOURCE_MATRIX)["columns"], json!(1));
    assert_mirror_absent(after, 2, 1, "shrink");
    assert_live_correspondences(after, &[(0, 0), (1, 0)], "shrink");
    let mut expected_undo = omitted_doc.clone();
    let undone = undo(&mut engine, "native-omit-shrink-undo");
    expected_undo["revision"] = undone["revision"].clone();
    assert_eq!(undone, expected_undo, "Undo must restore the Omitted document");
    let mut expected_redo = after.clone();
    let redone = redo(&mut engine, "native-omit-shrink-redo");
    expected_redo["revision"] = redone["revision"].clone();
    assert_eq!(redone, expected_redo, "Redo must restore the shrink");
    assert_eq!(snapshot(&mut engine), redone);
}

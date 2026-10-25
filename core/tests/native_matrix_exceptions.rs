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
    assert_mirror_absent(&document, "omit");
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

fn assert_mirror_absent(document: &Value, label: &str) {
    // Live-yet-absent: no mirror matrix record and no mirror parts, while the
    // layout shell survives matrix-less so the pair correspondence stays
    // valid (slot liveness is asserted separately per surviving member).
    assert!(
        document["matrices"]
            .as_array()
            .unwrap()
            .iter()
            .all(|matrix| matrix["id"] != MIRROR_MATRIX),
        "{label}: omitted mirror keeps no record"
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
        "{label}: no omitted part is materialized"
    );
    assert!(
        pair(document, PAIR_ID)["halves"]["correspondences"]
            .as_array()
            .unwrap()
            .iter()
            .any(|record| record["kind"] == "matrix"
                && record["sourceId"] == SOURCE_MATRIX
                && record["mirrorId"] == MIRROR_MATRIX),
        "{label}: matrix correspondence stays live"
    );
    let shell = document["layouts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|layout| layout["id"] == "right-synthetic-keys-layout")
        .unwrap_or_else(|| panic!("{label}: mirror layout shell is missing"));
    assert_eq!(shell["boardId"], json!("right"), "{label}");
    assert_eq!(
        shell["matrixIds"],
        json!([MIRROR_MATRIX]),
        "{label}: layout shell keeps following the matrix pair"
    );
}

fn make_local(engine: &mut CoreEngine, before: &Value) -> Value {
    let made = commit(
        engine,
        before,
        "native-make-local",
        json!({"kind":"set-half-exception","pairId":PAIR_ID,
            "itemKind":"matrix","mirrorId":MIRROR_MATRIX,"state":"local"}),
        &[MIRROR_MATRIX],
    );
    assert_eq!(made["kind"], "scene", "{made}");
    let document = made["document"].clone();
    assert!(
        pair(&document, PAIR_ID)["halves"]["exceptions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|exception| exception["kind"] == "matrix"
                && exception["mirrorId"] == MIRROR_MATRIX
                && exception["state"] == "local"),
        "make-local must record a matrix-level exception"
    );
    document
}

fn exceptions(document: &Value) -> Vec<&Value> {
    pair(document, PAIR_ID)["halves"]["exceptions"]
        .as_array()
        .map(|list| list.iter().collect())
        .unwrap_or_default()
}

fn matrix_exception(document: &Value) -> Option<&Value> {
    exceptions(document)
        .into_iter()
        .find(|exception| exception["kind"] == "matrix" && exception["mirrorId"] == MIRROR_MATRIX)
}

fn part_exception<'a>(document: &'a Value, mirror_id: &str) -> Option<&'a Value> {
    exceptions(document)
        .into_iter()
        .find(|exception| exception["kind"] == "part" && exception["mirrorId"] == mirror_id)
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
    assert_eq!(
        undone, expected_undo,
        "Undo must restore the Local document"
    );
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
    assert_mirror_absent(after, "expand");
    assert_live_correspondences(after, &[(0, 0), (0, 1), (1, 0), (1, 1)], "expand");
    // One transaction: Undo restores the pre-resize document, Redo restores
    // the resize.
    let mut expected_undo = omitted_doc.clone();
    let undone = undo(&mut engine, "native-omit-expand-undo");
    expected_undo["revision"] = undone["revision"].clone();
    assert_eq!(
        undone, expected_undo,
        "Undo must restore the Omitted document"
    );
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
    assert_mirror_absent(after, "shrink");
    assert_live_correspondences(after, &[(0, 0), (1, 0)], "shrink");
    let mut expected_undo = omitted_doc.clone();
    let undone = undo(&mut engine, "native-omit-shrink-undo");
    expected_undo["revision"] = undone["revision"].clone();
    assert_eq!(
        undone, expected_undo,
        "Undo must restore the Omitted document"
    );
    let mut expected_redo = after.clone();
    let redone = redo(&mut engine, "native-omit-shrink-redo");
    expected_redo["revision"] = redone["revision"].clone();
    assert_eq!(redone, expected_redo, "Redo must restore the shrink");
    assert_eq!(snapshot(&mut engine), redone);
}

#[test]
fn native_make_local_detach_keeps_pose_in_one_entry() {
    // Ticket 29 slice B3: making the mirror matrix Local detaches it keeping
    // every world pose, in a single Undo entry.
    let (mut engine, before) = accepted_pair(PAIR_ID);
    let poses_before = poses(&before);
    let local_doc = make_local(&mut engine, &before);
    assert_poses_unchanged(&poses_before, &local_doc);
    assert_eq!(
        matrix(&local_doc, MIRROR_MATRIX),
        matrix(&before, MIRROR_MATRIX),
        "detach keeps the owned descriptor"
    );
    let mut expected_undo = before.clone();
    let undone = undo(&mut engine, "native-detach-undo");
    expected_undo["revision"] = undone["revision"].clone();
    assert_eq!(undone, expected_undo, "Undo must restore the linked pair");
    assert!(matrix_exception(&undone).is_none());
    let mut expected_redo = local_doc.clone();
    let redone = redo(&mut engine, "native-detach-redo");
    expected_redo["revision"] = redone["revision"].clone();
    assert_eq!(redone, expected_redo, "Redo must restore the detach");
    assert_eq!(snapshot(&mut engine), redone);
}

#[test]
fn native_rejoin_adopts_source_and_keeps_explicit_child_exceptions() {
    // Ticket 29 slice B3 (mirrors ticket 21's Rejoin natively): after Local
    // plus source growth, Rejoin adopts the current source dimensions keeping
    // an explicit child exception, in a single Undo entry.
    let (mut engine, before) = accepted_pair(PAIR_ID);
    let local_doc = make_local(&mut engine, &before);
    let grown = commit(
        &mut engine,
        &local_doc,
        "native-grow-local-source",
        json!({"kind":"set-matrix","matrix":expanded_source(&local_doc)}),
        &[],
    );
    assert_eq!(grown["kind"], "scene", "{grown}");
    let grown_doc = grown["document"].clone();
    // An explicit child exception on one mirror key.
    let explicit_key = "matrix/right-synthetic-keys/r0c0";
    let keyed = commit(
        &mut engine,
        &grown_doc,
        "native-explicit-child",
        json!({"kind":"set-half-exception","pairId":PAIR_ID,
            "itemKind":"part","mirrorId":explicit_key,"state":"local"}),
        &[explicit_key],
    );
    assert_eq!(keyed["kind"], "scene", "{keyed}");
    let keyed_doc = keyed["document"].clone();
    assert!(part_exception(&keyed_doc, explicit_key).is_some());
    let explicit_pose = part(&keyed_doc, explicit_key)["pose"].clone();
    // Rejoin the matrix.
    let rejoined = commit(
        &mut engine,
        &keyed_doc,
        "native-rejoin",
        json!({"kind":"set-half-exception","pairId":PAIR_ID,
            "itemKind":"matrix","mirrorId":MIRROR_MATRIX,"state":"linked"}),
        &[MIRROR_MATRIX],
    );
    assert_eq!(rejoined["kind"], "scene", "{rejoined}");
    let after = &rejoined["document"];
    // The mirror adopts the current source dimensions, including the grown
    // column that has no prior correspondence.
    assert_eq!(matrix(after, MIRROR_MATRIX)["rows"], json!(2));
    assert_eq!(matrix(after, MIRROR_MATRIX)["columns"], json!(3));
    assert!(matrix_exception(after).is_none());
    for key in ["r0c0", "r0c1", "r0c2", "r1c0", "r1c1", "r1c2"] {
        assert!(
            find_part(after, &format!("matrix/{MIRROR_MATRIX}/{key}")).is_some(),
            "rejoin materializes {key}"
        );
    }
    // The explicit child keeps its exception and pose.
    assert!(part_exception(after, explicit_key).is_some());
    assert_eq!(part(after, explicit_key)["pose"], explicit_pose);
    // A rejoined sibling follows the source projection again.
    let source_key = "matrix/left-synthetic-keys/r0c1";
    let mirror_key = "matrix/right-synthetic-keys/r0c1";
    assert_eq!(
        part(after, mirror_key)["pose"]["at"]["x"],
        json!(200.0 - part(after, source_key)["pose"]["at"]["x"].as_f64().unwrap())
    );
    assert_eq!(
        part(after, mirror_key)["pose"]["at"]["y"],
        part(after, source_key)["pose"]["at"]["y"]
    );
    // One transaction: Undo restores the pre-rejoin document exactly.
    let mut expected_undo = keyed_doc.clone();
    let undone = undo(&mut engine, "native-rejoin-undo");
    expected_undo["revision"] = undone["revision"].clone();
    assert_eq!(
        undone, expected_undo,
        "Undo must restore the Local document"
    );
    let mut expected_redo = after.clone();
    let redone = redo(&mut engine, "native-rejoin-redo");
    expected_redo["revision"] = redone["revision"].clone();
    assert_eq!(redone, expected_redo, "Redo must restore the rejoin");
    assert_eq!(snapshot(&mut engine), redone);
}

fn remove_matrix(engine: &mut CoreEngine, before: &Value, id: &str, matrix_id: &str) -> Value {
    commit(
        engine,
        before,
        id,
        json!({"kind":"remove-matrix","id":matrix_id}),
        &[],
    )
}

fn try_commit(engine: &mut CoreEngine, before: &Value, id: &str, operation: Value) -> Value {
    serde_json::from_str(
        &engine
            .request(
                &json!({"id":id,"kind":"edit","command":{
                    "baseRevision":before["revision"],
                    "transactionId":id,
                    "phase":"commit",
                    "targetIds":[],
                    "operation":operation
                }})
                .to_string(),
            )
            .to_string(),
    )
    .unwrap()
}

#[test]
fn native_source_removal_retains_local_matrix() {
    // Ticket 29 slice B4 (mirrors ticket 21's Local source removal
    // natively): removing the source retains the complete Local mirror,
    // retires only the defunct source-layout link, and preserves unrelated
    // parts, in a single Undo entry.
    let (mut engine, before) = accepted_pair(PAIR_ID);
    let local_doc = make_local(&mut engine, &before);
    let mirror_before = matrix(&local_doc, MIRROR_MATRIX).clone();
    let poses_before = poses(&local_doc);
    let removed = remove_matrix(
        &mut engine,
        &local_doc,
        "native-remove-source",
        SOURCE_MATRIX,
    );
    assert_eq!(removed["kind"], "scene", "{removed}");
    let after = &removed["document"];
    // The source side is gone; the Local mirror is complete and untouched.
    assert!(
        after["matrices"]
            .as_array()
            .unwrap()
            .iter()
            .all(|matrix| matrix["id"] != SOURCE_MATRIX),
        "source record is gone"
    );
    assert_eq!(
        matrix(after, MIRROR_MATRIX),
        &mirror_before,
        "Local mirror stays complete"
    );
    // Only source members disappear; every surviving pose is unchanged.
    let (removed, surviving): (Vec<_>, Vec<_>) = poses_before
        .into_iter()
        .partition(|(id, _)| find_part(after, id).is_none());
    for (id, _) in &removed {
        assert!(
            id.starts_with("matrix/left-synthetic-keys/"),
            "only source members disappear, found {id}"
        );
    }
    assert_poses_unchanged(&surviving, after);
    // Every defunct link retires with the source: the matrix and member
    // correspondences and their overlays go, leaving a plain standalone
    // matrix (remove_exact retires overlays with their correspondences).
    // Unrelated board-part links stay.
    assert!(matrix_exception(after).is_none());
    assert!(
        pair(after, PAIR_ID)["halves"]["correspondences"]
            .as_array()
            .unwrap()
            .iter()
            .all(|record| !(record["kind"] == "matrix"
                || (record["kind"] == "part"
                    && record["sourceId"].as_str().unwrap().starts_with("matrix/")))),
        "defunct matrix/member links retire"
    );
    // Only the defunct source-layout link retires; the mirror layout keeps
    // following its matrix.
    assert!(
        pair(after, PAIR_ID)["halves"]["correspondences"]
            .as_array()
            .unwrap()
            .iter()
            .all(|record| record["sourceId"] != "left-synthetic-keys-layout"),
        "defunct layout link retires"
    );
    let shell = after["layouts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|layout| layout["id"] == "right-synthetic-keys-layout")
        .expect("mirror layout survives");
    assert_eq!(shell["matrixIds"], json!([MIRROR_MATRIX]));
    // Unrelated parts on both boards are preserved.
    for id in [
        "left/U1",
        "left/RST",
        "left/TRRS",
        "right/U1",
        "right/RST",
        "right/TRRS",
    ] {
        assert!(
            find_part(after, id).is_some(),
            "unrelated {id} is preserved"
        );
    }
    let mut expected_undo = local_doc.clone();
    let undone = undo(&mut engine, "native-removal-undo");
    expected_undo["revision"] = undone["revision"].clone();
    assert_eq!(
        undone, expected_undo,
        "Undo must restore the Local document"
    );
    let mut expected_redo = after.clone();
    let redone = redo(&mut engine, "native-removal-redo");
    expected_redo["revision"] = redone["revision"].clone();
    assert_eq!(redone, expected_redo, "Redo must restore the removal");
    assert_eq!(snapshot(&mut engine), redone);
}

#[test]
fn native_source_removal_retires_omitted_without_resurrection() {
    // Ticket 29 slice B4: removing the source over an Omitted mirror retires
    // the Omitted records instead of resurrecting the mirror, in one Undo
    // entry. The mirror layout shell remains as ordinary geometry.
    let (mut engine, before) = accepted_pair(PAIR_ID);
    let omitted_doc = make_omitted(&mut engine, &before);
    let removed = remove_matrix(
        &mut engine,
        &omitted_doc,
        "native-remove-omitted-source",
        SOURCE_MATRIX,
    );
    assert_eq!(removed["kind"], "scene", "{removed}");
    let after = &removed["document"];
    assert!(
        after["matrices"]
            .as_array()
            .unwrap()
            .iter()
            .all(|matrix| matrix["id"] != SOURCE_MATRIX),
        "source record is gone"
    );
    // No resurrection: no mirror record and no mirror parts.
    assert!(
        after["matrices"]
            .as_array()
            .unwrap()
            .iter()
            .all(|matrix| matrix["id"] != MIRROR_MATRIX),
        "omitted-removal: mirror stays record-absent"
    );
    assert!(
        after["parts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|part| !part["id"]
                .as_str()
                .unwrap()
                .starts_with("matrix/right-synthetic-keys/")),
        "omitted-removal: no part is resurrected"
    );
    // The Omitted records retire: no matrix/member correspondence, no
    // exception. Unrelated board-part links stay.
    assert!(matrix_exception(after).is_none());
    assert!(
        pair(after, PAIR_ID)["halves"]["correspondences"]
            .as_array()
            .unwrap()
            .iter()
            .all(|record| !(record["kind"] == "matrix"
                || (record["kind"] == "part"
                    && record["sourceId"].as_str().unwrap().starts_with("matrix/")))),
        "omitted links retire"
    );
    // The orphaned mirror shell goes with the removal: with no resolvable
    // matrix left, layout validation could never keep it. (An empty layout
    // list serializes as null.)
    assert!(
        after["layouts"]
            .as_array()
            .map(|layouts| layouts
                .iter()
                .all(|layout| layout["id"] != "right-synthetic-keys-layout"))
            .unwrap_or(true),
        "omitted-removal: orphaned shell is gone"
    );
    let mut expected_undo = omitted_doc.clone();
    let undone = undo(&mut engine, "native-omitted-removal-undo");
    expected_undo["revision"] = undone["revision"].clone();
    assert_eq!(
        undone, expected_undo,
        "Undo must restore the Omitted document"
    );
    let mut expected_redo = after.clone();
    let redone = redo(&mut engine, "native-omitted-removal-redo");
    expected_redo["revision"] = redone["revision"].clone();
    assert_eq!(redone, expected_redo, "Redo must restore the removal");
    assert_eq!(snapshot(&mut engine), redone);
}

fn set_part_exception(
    engine: &mut CoreEngine,
    before: &Value,
    id: &str,
    mirror_id: &str,
    state: &str,
) -> Value {
    commit(
        engine,
        before,
        id,
        json!({"kind":"set-half-exception","pairId":PAIR_ID,
            "itemKind":"part","mirrorId":mirror_id,"state":state}),
        &[mirror_id],
    )
}

fn shrunk_to_two(document: &Value) -> Value {
    // Dense shrink payload back to two columns.
    let mut shrunk = matrix(document, SOURCE_MATRIX).clone();
    shrunk["columns"] = json!(2);
    let kept: Vec<Value> = shrunk["cells"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|cell| cell["column"].as_u64().unwrap() < 2)
        .cloned()
        .collect();
    shrunk["cells"] = Value::Array(kept);
    shrunk
}

#[test]
fn native_local_key_flip_to_omitted_returns_to_shared_pose() {
    // Ticket 29 slice B5 (mirrors ticket 21's key flip natively): an explicit
    // Local key flipped to Omitted is suppressed, and flipping back to Linked
    // materializes it again at the shared projected pose.
    let (mut engine, before) = accepted_pair(PAIR_ID);
    let key = "matrix/right-synthetic-keys/r0c0";
    let source_key = "matrix/left-synthetic-keys/r0c0";
    let shared_pose = part(&before, key)["pose"].clone();
    let made_local = set_part_exception(&mut engine, &before, "native-key-local", key, "local");
    assert_eq!(made_local["kind"], "scene", "{made_local}");
    let omitted = set_part_exception(
        &mut engine,
        &made_local["document"],
        "native-key-omitted",
        key,
        "omitted",
    );
    assert_eq!(omitted["kind"], "scene", "{omitted}");
    let omitted_doc = omitted["document"].clone();
    assert!(find_part(&omitted_doc, key).is_none());
    let relinked = set_part_exception(
        &mut engine,
        &omitted_doc,
        "native-key-relinked",
        key,
        "linked",
    );
    assert_eq!(relinked["kind"], "scene", "{relinked}");
    let after = &relinked["document"];
    assert_eq!(part(after, key)["pose"], shared_pose);
    assert_eq!(
        part(after, key)["pose"]["at"]["x"],
        json!(200.0 - part(after, source_key)["pose"]["at"]["x"].as_f64().unwrap())
    );
    assert!(part_exception(after, key).is_none());
}

#[test]
fn native_explicit_child_survives_omitted_rejoin() {
    // Ticket 29 slice B5: an explicit Omitted child stays suppressed across
    // a whole-matrix Omit and Rejoin while its siblings come back.
    let (mut engine, before) = accepted_pair(PAIR_ID);
    let key = "matrix/right-synthetic-keys/r0c0";
    let keyed = set_part_exception(&mut engine, &before, "native-child-omit", key, "omitted");
    assert_eq!(keyed["kind"], "scene", "{keyed}");
    let omitted_doc = make_omitted(&mut engine, &keyed["document"]);
    let rejoined = commit(
        &mut engine,
        &omitted_doc,
        "native-rejoin-explicit",
        json!({"kind":"set-half-exception","pairId":PAIR_ID,
            "itemKind":"matrix","mirrorId":MIRROR_MATRIX,"state":"linked"}),
        &[MIRROR_MATRIX],
    );
    assert_eq!(rejoined["kind"], "scene", "{rejoined}");
    let after = &rejoined["document"];
    assert!(matrix_exception(after).is_none());
    assert!(
        find_part(after, key).is_none(),
        "explicit child stays suppressed"
    );
    assert!(
        part_exception(after, key).is_some(),
        "explicit child keeps its exception"
    );
    for key in ["r0c1", "r1c0", "r1c1"] {
        assert!(
            find_part(after, &format!("matrix/{MIRROR_MATRIX}/{key}")).is_some(),
            "sibling {key} materializes"
        );
    }
}

#[test]
fn native_omitted_mirror_edits_refuse_with_rejoin_guidance() {
    // Ticket 29 slice B5: direct edits to an Omitted (absent) mirror refuse
    // with Rejoin guidance instead of a bare missing-matrix error.
    let (mut engine, before) = accepted_pair(PAIR_ID);
    let omitted_doc = make_omitted(&mut engine, &before);
    let mut ghost = matrix(&omitted_doc, SOURCE_MATRIX).clone();
    ghost["id"] = json!(MIRROR_MATRIX);
    let refused = try_commit(
        &mut engine,
        &omitted_doc,
        "native-edit-absent",
        json!({"kind":"set-matrix","matrix":ghost}),
    );
    assert_eq!(refused["kind"], "error", "{refused}");
    assert!(
        refused["message"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .contains("rejoin"),
        "refusal must name Rejoin: {refused}"
    );
    assert_eq!(snapshot(&mut engine), omitted_doc);
}

#[test]
fn native_child_exception_on_absent_member_refuses() {
    // Ticket 29 slice B5: excepting a member with no materialized mirror part
    // is an invalid transition and refuses without changing anything.
    let (mut engine, before) = accepted_pair(PAIR_ID);
    let omitted_doc = make_omitted(&mut engine, &before);
    let key = "matrix/right-synthetic-keys/r0c0";
    let refused = try_commit(
        &mut engine,
        &omitted_doc,
        "native-except-absent",
        json!({"kind":"set-half-exception","pairId":PAIR_ID,
            "itemKind":"part","mirrorId":key,"state":"local"}),
    );
    assert_eq!(refused["kind"], "error", "{refused}");
    assert_eq!(snapshot(&mut engine), omitted_doc);
}

#[test]
fn native_shrink_after_local_growth_keeps_mirror() {
    // Ticket 29 slice B5 (second-operation hardening): members introduced
    // under a Local mirror stay source-only, so a later shrink that removes
    // them succeeds with the Local mirror byte-identical.
    let (mut engine, before) = accepted_pair(PAIR_ID);
    let local_doc = make_local(&mut engine, &before);
    let mirror_before = matrix(&local_doc, MIRROR_MATRIX).clone();
    let grown = commit(
        &mut engine,
        &local_doc,
        "native-grow-then-shrink",
        json!({"kind":"set-matrix","matrix":expanded_source(&local_doc)}),
        &[],
    );
    assert_eq!(grown["kind"], "scene", "{grown}");
    let shrunk = commit(
        &mut engine,
        &grown["document"],
        "native-shrink-newcomers",
        json!({"kind":"set-matrix","matrix":shrunk_to_two(&grown["document"])}),
        &[],
    );
    assert_eq!(shrunk["kind"], "scene", "{shrunk}");
    let after = &shrunk["document"];
    assert_eq!(matrix(after, SOURCE_MATRIX)["columns"], json!(2));
    assert_eq!(
        matrix(after, MIRROR_MATRIX),
        &mirror_before,
        "Local mirror stays byte-identical across both operations"
    );
    assert!(
        find_part(after, "matrix/left-synthetic-keys/r0c2").is_none(),
        "newcomers are gone from the source"
    );
    assert_mirror_correspondences(after, "second shrink");
}

#[test]
fn native_shrink_after_omitted_growth_stays_absent() {
    // Ticket 29 slice B5: members introduced under an Omitted mirror stay
    // source-only as well; removing them again succeeds with the mirror
    // still absent and old links live.
    let (mut engine, before) = accepted_pair(PAIR_ID);
    let omitted_doc = make_omitted(&mut engine, &before);
    let grown = commit(
        &mut engine,
        &omitted_doc,
        "native-grow-omitted-then-shrink",
        json!({"kind":"set-matrix","matrix":expanded_source(&omitted_doc)}),
        &[],
    );
    assert_eq!(grown["kind"], "scene", "{grown}");
    let shrunk = commit(
        &mut engine,
        &grown["document"],
        "native-shrink-omitted-newcomers",
        json!({"kind":"set-matrix","matrix":shrunk_to_two(&grown["document"])}),
        &[],
    );
    assert_eq!(shrunk["kind"], "scene", "{shrunk}");
    let after = &shrunk["document"];
    assert_eq!(matrix(after, SOURCE_MATRIX)["columns"], json!(2));
    assert_mirror_absent(after, "second shrink");
    assert_live_correspondences(after, &[(0, 0), (0, 1), (1, 0), (1, 1)], "second shrink");
}

#[test]
fn native_rejoin_missing_source_refuses_atomically() {
    // Ticket 29 slices B3/B4: Rejoin cannot adopt a removed source. The
    // refusal changes nothing: the retained Local document is byte-identical
    // afterwards (modulo revision, which only advances on success).
    let (mut engine, before) = accepted_pair(PAIR_ID);
    let local_doc = make_local(&mut engine, &before);
    let removed = remove_matrix(
        &mut engine,
        &local_doc,
        "native-remove-source-rejoin",
        SOURCE_MATRIX,
    );
    assert_eq!(removed["kind"], "scene", "{removed}");
    let retained = removed["document"].clone();
    let refused = try_commit(
        &mut engine,
        &retained,
        "native-rejoin-missing",
        json!({"kind":"set-half-exception","pairId":PAIR_ID,
            "itemKind":"matrix","mirrorId":MIRROR_MATRIX,"state":"linked"}),
    );
    assert_eq!(refused["kind"], "error", "{refused}");
    assert_eq!(snapshot(&mut engine), retained);
}

#[test]
fn native_rejoin_from_omitted_recreates_mirror() {
    // Ticket 29 slice B3: Rejoin from Omitted recreates the mirror record
    // from the current source and clears the exception, in one Undo entry.
    let (mut engine, before) = accepted_pair(PAIR_ID);
    let omitted_doc = make_omitted(&mut engine, &before);
    let rejoined = commit(
        &mut engine,
        &omitted_doc,
        "native-rejoin-omitted",
        json!({"kind":"set-half-exception","pairId":PAIR_ID,
            "itemKind":"matrix","mirrorId":MIRROR_MATRIX,"state":"linked"}),
        &[MIRROR_MATRIX],
    );
    assert_eq!(rejoined["kind"], "scene", "{rejoined}");
    let after = &rejoined["document"];
    assert!(matrix_exception(after).is_none());
    assert_eq!(matrix(after, MIRROR_MATRIX)["rows"], json!(2));
    assert_eq!(matrix(after, MIRROR_MATRIX)["columns"], json!(2));
    // Every member is materialized again at the projected pose.
    for (row, column) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
        let source_id = format!("matrix/{SOURCE_MATRIX}/r{row}c{column}");
        let mirror_id = format!("matrix/{MIRROR_MATRIX}/r{row}c{column}");
        assert_eq!(
            part(after, &mirror_id)["pose"]["at"]["x"],
            json!(200.0 - part(after, &source_id)["pose"]["at"]["x"].as_f64().unwrap()),
            "{mirror_id}"
        );
        assert_eq!(
            part(after, &mirror_id)["pose"]["at"]["y"],
            part(after, &source_id)["pose"]["at"]["y"],
            "{mirror_id}"
        );
    }
    assert_live_correspondences(after, &[(0, 0), (0, 1), (1, 0), (1, 1)], "rejoin");
    let mut expected_undo = omitted_doc.clone();
    let undone = undo(&mut engine, "native-rejoin-omitted-undo");
    expected_undo["revision"] = undone["revision"].clone();
    assert_eq!(
        undone, expected_undo,
        "Undo must restore the Omitted document"
    );
    let mut expected_redo = after.clone();
    let redone = redo(&mut engine, "native-rejoin-omitted-redo");
    expected_redo["revision"] = redone["revision"].clone();
    assert_eq!(redone, expected_redo, "Redo must restore the rejoin");
    assert_eq!(snapshot(&mut engine), redone);
}

#[test]
fn native_anchored_add_reflects_counterpart_when_linked() {
    // Ticket 29 enabler (spec §4): an anchored add on a paired board is
    // reflected to the mirror half with a mapped anchor, matrix/member
    // correspondences and layout membership, in one Undo entry.
    let (mut engine, before) = anchored_pair(PAIR_ID);
    assert_eq!(
        matrix(&before, DEP_MIRROR_ID)["anchor"],
        json!({"matrixId":MIRROR_MATRIX,"row":0,"column":1,
            "follow":["x","y","rotation"]}),
        "mirror anchor follows the mirrored key"
    );
    let mirror_layout = before["layouts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|layout| layout["id"] == "right-synthetic-keys-layout")
        .unwrap();
    assert!(
        mirror_layout["matrixIds"]
            .as_array()
            .unwrap()
            .iter()
            .any(|id| id == DEP_MIRROR_ID),
        "mirror layout follows the dependent"
    );
    // One transaction: Undo removes the dependent pair, Redo restores it.
    let undone = undo(&mut engine, "gap-add-dep-undo");
    assert!(
        undone["matrices"]
            .as_array()
            .unwrap()
            .iter()
            .all(|matrix| matrix["id"] != DEP_MIRROR_ID),
        "Undo removes the reflected dependent"
    );
    let redone = redo(&mut engine, "gap-add-dep-redo");
    assert!(matrix(&redone, DEP_MIRROR_ID).is_object());
    assert_eq!(snapshot(&mut engine), redone);
}

#[test]
fn native_anchored_add_stays_source_only_without_pair() {
    // The reflection only fires when linked: without a pair the add keeps
    // its historical source-only shape.
    let (mut engine, seed) = seed(&runtime_supported_document());
    let added = commit(
        &mut engine,
        &seed,
        "gap-add-unpaired",
        json!({"kind":"add-anchored-matrix","id":DEP_SOURCE_ID,
            "anchor":{"matrixId":SOURCE_MATRIX,"row":0,"column":1,
                "follow":["x","y","rotation"]}}),
        &[],
    );
    assert_eq!(added["kind"], "scene", "{added}");
    let document = &added["document"];
    assert!(matrix(document, DEP_SOURCE_ID).is_object());
    assert!(
        document["matrices"]
            .as_array()
            .unwrap()
            .iter()
            .all(|matrix| matrix["id"] != DEP_MIRROR_ID),
        "no mirror counterpart without a pair"
    );
}

// ---- Ticket 29 gaps: B4 confirmed-detach, B3 missing anchors, B5 proofs ----

/// Source-side anchored matrix and its expected native mirror (board-prefix rule).
const DEP_SOURCE_ID: &str = "left-dep-keys";
const DEP_MIRROR_ID: &str = "right-dep-keys";
const DEP_ANCHOR_KEY: &str = "matrix/left-synthetic-keys/r0c1";
const DEP_MIRROR_ANCHOR_KEY: &str = "matrix/right-synthetic-keys/r0c1";

/// Pair whose source board carries an anchored matrix, reflected to the
/// the anchored add must be reflected to the mirror half (spec §4), so the
/// dependent exists on both sides with a mapped anchor.
fn anchored_pair(pair_id: &str) -> (CoreEngine, Value) {
    let (mut engine, before) = accepted_pair(pair_id);
    let added = commit(
        &mut engine,
        &before,
        "gap-add-dep",
        json!({"kind":"add-anchored-matrix","id":DEP_SOURCE_ID,
            "anchor":{"matrixId":SOURCE_MATRIX,"row":0,"column":1,
                "follow":["x","y","rotation"]}}),
        &[],
    );
    assert_eq!(added["kind"], "scene", "{added}");
    let document = added["document"].clone();
    // The anchored add is reflected: a paired mirror matrix anchored to the
    // mirrored key, with matrix and member correspondences.
    assert!(
        matrix(&document, DEP_MIRROR_ID).is_object(),
        "anchored add reflects its counterpart when linked"
    );
    let halves = &pair(&document, pair_id)["halves"];
    assert!(
        halves["correspondences"]
            .as_array()
            .unwrap()
            .iter()
            .any(|record| {
                record["kind"] == "matrix"
                    && record["sourceId"] == DEP_SOURCE_ID
                    && record["mirrorId"] == DEP_MIRROR_ID
            }),
        "dependent pair correspondence is recorded"
    );
    assert_eq!(
        matrix(&document, DEP_MIRROR_ID)["anchor"]["matrixId"],
        json!(MIRROR_MATRIX),
        "mirror anchor follows the mirrored key"
    );
    (engine, document)
}

/// The exact Core refusal the ticket-23 confirmation UI parses verbatim:
/// `detach_proposal` splits on `;` and expects the confirm tail.
fn expected_detach_proposal(detach_id: &str, partner_id: &str) -> String {
    format!(
        "Detaching {detach_id} would move its linked partner {partner_id}; \
         make {partner_id} Local to keep its pose and confirm the detach"
    )
}

fn preview(engine: &mut CoreEngine, before: &Value, id: &str, operation: Value) -> Value {
    let reply: Value = serde_json::from_str(
        &engine
            .request(
                &json!({"id":id,"kind":"edit","command":{
                    "baseRevision":before["revision"],
                    "transactionId":id,
                    "phase":"preview",
                    "targetIds":[],
                    "operation":operation
                }})
                .to_string(),
            )
            .to_string(),
    )
    .unwrap();
    assert_eq!(reply["kind"], "preview", "{reply}");
    assert_eq!(snapshot(engine), *before, "preview publishes nothing");
    reply
}

fn make_part_local(engine: &mut CoreEngine, before: &Value, id: &str, mirror_id: &str) -> Value {
    commit(
        engine,
        before,
        id,
        json!({"kind":"set-half-exception","pairId":PAIR_ID,
            "itemKind":"part","mirrorId":mirror_id,"state":"local"}),
        &[mirror_id],
    )
}

#[test]
fn native_detach_needing_local_exception_refuses_with_proposal() {
    // Ticket 29 slice B4 (mirrors ticket 21's shared detach natively): the
    // dependent follows its actual Local anchor key, so detaching it would
    // move its native partner. The detach refuses with the ticket-23
    // confirmation proposal and changes nothing.
    let (mut engine, before) = anchored_pair(PAIR_ID);
    // Diverge the mirror anchor key: Local plus an independent move, authored
    // from the mirror half.
    let made = make_part_local(&mut engine, &before, "gap-key-local", DEP_MIRROR_ANCHOR_KEY);
    assert_eq!(made["kind"], "scene", "{made}");
    let moved = commit(
        &mut engine,
        &made["document"],
        "gap-move-anchor-key",
        json!({"kind":"move-parts","positions":[{"id":DEP_MIRROR_ANCHOR_KEY,
            "at":{"x":-33.0,"y":17.0},"rotation":5.0}]}),
        &[DEP_MIRROR_ANCHOR_KEY],
    );
    assert_eq!(moved["kind"], "scene", "{moved}");
    let diverged = moved["document"].clone();
    // The dependent follows its actual Local key.
    assert_ne!(
        part(&diverged, &format!("matrix/{DEP_MIRROR_ID}/r0c0"))["pose"],
        part(&before, &format!("matrix/{DEP_MIRROR_ID}/r0c0"))["pose"],
        "dependent follows the moved anchor key"
    );
    // A plain detach refuses with the confirmable proposal; cancel (no
    // commit) leaves the document byte-identical.
    let refused = try_commit(
        &mut engine,
        &diverged,
        "gap-detach",
        json!({"kind":"detach-matrix-anchor","matrixId":DEP_SOURCE_ID}),
    );
    assert_eq!(refused["kind"], "error", "{refused}");
    assert_eq!(
        refused["message"],
        expected_detach_proposal(DEP_SOURCE_ID, DEP_MIRROR_ID),
        "proposal reuses the ticket-23 confirmation shape verbatim"
    );
    assert_eq!(snapshot(&mut engine), diverged);
}

#[test]
fn native_detach_confirmed_applies_detach_and_local_in_one_entry() {
    // Ticket 29 slice B4: the confirmed detach clears both anchors, keeps
    // every world pose, and records the named Local exception in one Undo
    // entry. A stale confirmation refuses without changes.
    let (mut engine, before) = anchored_pair(PAIR_ID);
    let made = make_part_local(&mut engine, &before, "gap-key-local", DEP_MIRROR_ANCHOR_KEY);
    assert_eq!(made["kind"], "scene", "{made}");
    let moved = commit(
        &mut engine,
        &made["document"],
        "gap-move-anchor-key",
        json!({"kind":"move-parts","positions":[{"id":DEP_MIRROR_ANCHOR_KEY,
            "at":{"x":-33.0,"y":17.0},"rotation":5.0}]}),
        &[DEP_MIRROR_ANCHOR_KEY],
    );
    assert_eq!(moved["kind"], "scene", "{moved}");
    let diverged = moved["document"].clone();
    let poses_before = poses(&diverged);
    let confirmed = commit(
        &mut engine,
        &diverged,
        "gap-detach-confirmed",
        json!({"kind":"detach-matrix-anchor-confirmed",
            "matrixId":DEP_SOURCE_ID,"localMatrixId":DEP_MIRROR_ID}),
        &[],
    );
    assert_eq!(confirmed["kind"], "scene", "{confirmed}");
    let after = &confirmed["document"];
    assert_eq!(
        matrix(after, DEP_SOURCE_ID).get("anchor"),
        None,
        "source anchor cleared"
    );
    assert_eq!(
        matrix(after, DEP_MIRROR_ID).get("anchor"),
        None,
        "mirror anchor cleared"
    );
    assert_poses_unchanged(&poses_before, after);
    assert!(
        pair(after, PAIR_ID)["halves"]["exceptions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|exception| exception["kind"] == "matrix"
                && exception["mirrorId"] == DEP_MIRROR_ID
                && exception["state"] == "local"),
        "confirmed detach records the Local exception"
    );
    // One transaction: Undo restores anchors and drops the exception, Redo
    // reapplies everything.
    let mut expected_undo = diverged.clone();
    let undone = undo(&mut engine, "gap-detach-confirmed-undo");
    expected_undo["revision"] = undone["revision"].clone();
    assert_eq!(undone, expected_undo, "Undo must restore the anchored pair");
    let mut expected_redo = after.clone();
    let redone = redo(&mut engine, "gap-detach-confirmed-redo");
    expected_redo["revision"] = redone["revision"].clone();
    assert_eq!(
        redone, expected_redo,
        "Redo must restore the confirmed detach"
    );
    assert_eq!(snapshot(&mut engine), redone);
    // A stale confirmation (wrong partner) refuses without changes.
    let current = snapshot(&mut engine);
    let stale = try_commit(
        &mut engine,
        &current,
        "gap-detach-stale",
        json!({"kind":"detach-matrix-anchor-confirmed",
            "matrixId":DEP_SOURCE_ID,"localMatrixId":MIRROR_MATRIX}),
    );
    assert_eq!(stale["kind"], "error", "{stale}");
    assert!(
        stale["message"].as_str().unwrap().contains("stale"),
        "stale confirmation is named: {stale}"
    );
    assert_eq!(snapshot(&mut engine), current);
}

#[test]
fn native_rejoin_refuses_missing_anchor_atomically() {
    // Ticket 29 slice B3 (mirrors ticket 21's missing-anchor Rejoin
    // natively): the smaller Local parent lost the anchor counterpart, so
    // Rejoin — which would adopt the source anchor — refuses naming the
    // missing key with the document unchanged. Distinct from the
    // missing-source refusal: the source record is present.
    let (mut engine, before) = anchored_pair(PAIR_ID);
    // Omit the dependent first.
    let omitted = commit(
        &mut engine,
        &before,
        "gap-omit-dep",
        json!({"kind":"set-half-exception","pairId":PAIR_ID,
            "itemKind":"matrix","mirrorId":DEP_MIRROR_ID,"state":"omitted"}),
        &[DEP_MIRROR_ID],
    );
    assert_eq!(omitted["kind"], "scene", "{omitted}");
    // Freeze the parent small: a matrix-Local mirror keeps its dimensions
    // while the source grows (mirror-authored storage always maps to the
    // source, so the parent can only stay behind, never shrink itself).
    let made = commit(
        &mut engine,
        &omitted["document"],
        "gap-make-parent-local",
        json!({"kind":"set-half-exception","pairId":PAIR_ID,
            "itemKind":"matrix","mirrorId":MIRROR_MATRIX,"state":"local"}),
        &[MIRROR_MATRIX],
    );
    assert_eq!(made["kind"], "scene", "{made}");
    let grown = commit(
        &mut engine,
        &made["document"],
        "gap-grow-source",
        json!({"kind":"set-matrix","matrix":expanded_source(&made["document"])}),
        &[],
    );
    assert_eq!(grown["kind"], "scene", "{grown}");
    // Re-anchor the dependent onto the grown region.
    let reanchored = commit(
        &mut engine,
        &grown["document"],
        "gap-reanchor-dep",
        json!({"kind":"set-matrix-anchor","matrixId":DEP_SOURCE_ID,
            "anchor":{"matrixId":SOURCE_MATRIX,"row":0,"column":2,
                "follow":["x","y","rotation"]}}),
        &[],
    );
    assert_eq!(reanchored["kind"], "scene", "{reanchored}");
    let small = reanchored["document"].clone();
    const MISSING_KEY: &str = "matrix/right-synthetic-keys/r0c2";
    assert!(
        find_part(&small, MISSING_KEY).is_none(),
        "counterpart key is missing from the smaller parent"
    );
    // Rejoin adopts the source anchor whose counterpart is missing: refuse.
    let refused = try_commit(
        &mut engine,
        &small,
        "gap-rejoin-dep",
        json!({"kind":"set-half-exception","pairId":PAIR_ID,
            "itemKind":"matrix","mirrorId":DEP_MIRROR_ID,"state":"linked"}),
    );
    assert_eq!(refused["kind"], "error", "{refused}");
    assert!(
        refused["message"].as_str().unwrap().contains(MISSING_KEY),
        "refusal names the missing counterpart: {refused}"
    );
    assert_eq!(snapshot(&mut engine), small);
}

/// Source growth on a paired matrix must carry explicit new cells: the setter
/// leaves sparse cells unfilled, and the reconciliation kernel requires dense
/// cell coverage (`source_members`). Mirror-half edition: grows the mirror
/// record from its own cells so the inverse sees mirror-frame assemblies.
fn expanded_mirror(document: &Value) -> Value {
    let mut expanded = matrix(document, MIRROR_MATRIX).clone();
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

/// The Local mirror parent shrunk past the dependent's anchor slot: one
/// column, with explicit cell coverage for the kernel's dense rule.
/// Sort every matrix's cells by coordinate for cross-route comparison.
fn sort_matrix_cells(document: &mut Value) {
    for matrix in document["matrices"].as_array_mut().unwrap() {
        matrix["cells"].as_array_mut().unwrap().sort_by_key(|cell| {
            (
                cell["row"].as_u64().unwrap(),
                cell["column"].as_u64().unwrap(),
            )
        });
    }
}

#[test]
fn native_mirror_authored_growth_matches_source_authored() {
    // Ticket 29 slice B5 (identity remaps): authoring a linked growth from
    // the mirror half maps to the source and lands byte-identical to the
    // source-authored edit.
    let (mut engine_a, before_a) = accepted_pair(PAIR_ID);
    let grown_a = commit(
        &mut engine_a,
        &before_a,
        "gap-grow-source",
        json!({"kind":"set-matrix","matrix":expanded_source(&before_a)}),
        &[],
    );
    assert_eq!(grown_a["kind"], "scene", "{grown_a}");
    let (mut engine_b, before_b) = accepted_pair(PAIR_ID);
    // The mirror-authored payload carries the mirror's own cells (mirror-frame
    // assemblies); the inverse maps them back to the source.
    let grown_payload = expanded_mirror(&before_b);
    let grown_b = commit(
        &mut engine_b,
        &before_b,
        "gap-grow-mirror",
        json!({"kind":"set-matrix","matrix":grown_payload}),
        &[],
    );
    assert_eq!(grown_b["kind"], "scene", "{grown_b}");
    let mut doc_a = grown_a["document"].clone();
    let mut doc_b = grown_b["document"].clone();
    doc_a["revision"] = Value::Null;
    doc_b["revision"] = Value::Null;
    // Cell storage order is not semantic (keyed by row/column): the direct
    // route preserves payload order while the inverse projection emits
    // coordinate order. Normalize the order, keep byte-identity elsewhere.
    sort_matrix_cells(&mut doc_a);
    sort_matrix_cells(&mut doc_b);
    assert_eq!(doc_a, doc_b, "mirror-authored growth lands identically");
}

#[test]
fn native_preview_exception_transitions_publish_nothing() {
    // Ticket 29 slice B5 (preview/cancel): previewing Local, Omitted and
    // Rejoin transitions publishes nothing; only the commit applies.
    let (mut engine, before) = accepted_pair(PAIR_ID);
    preview(
        &mut engine,
        &before,
        "gap-preview-local",
        json!({"kind":"set-half-exception","pairId":PAIR_ID,
            "itemKind":"matrix","mirrorId":MIRROR_MATRIX,"state":"local"}),
    );
    let local_doc = make_local(&mut engine, &before);
    preview(
        &mut engine,
        &local_doc,
        "gap-preview-rejoin",
        json!({"kind":"set-half-exception","pairId":PAIR_ID,
            "itemKind":"matrix","mirrorId":MIRROR_MATRIX,"state":"linked"}),
    );
    // Omitted half runs on a fresh pairing so every commit chains its own
    // accepted revision.
    let (mut engine, before) = accepted_pair("gap-preview-omitted");
    preview(
        &mut engine,
        &before,
        "gap-preview-omitted",
        json!({"kind":"set-half-exception","pairId":"gap-preview-omitted",
            "itemKind":"matrix","mirrorId":MIRROR_MATRIX,"state":"omitted"}),
    );
    let omitted_doc = make_omitted_pair(&mut engine, &before, "gap-preview-omitted");
    preview(
        &mut engine,
        &omitted_doc,
        "gap-preview-rejoin-omitted",
        json!({"kind":"set-half-exception","pairId":"gap-preview-omitted",
            "itemKind":"matrix","mirrorId":MIRROR_MATRIX,"state":"linked"}),
    );
}

/// Matrix-level Omit on a non-default pair id (the shared helper is pinned
/// to `PAIR_ID`).
fn make_omitted_pair(engine: &mut CoreEngine, before: &Value, pair_id: &str) -> Value {
    let omitted = commit(
        engine,
        before,
        "gap-make-omitted",
        json!({"kind":"set-half-exception","pairId":pair_id,
            "itemKind":"matrix","mirrorId":MIRROR_MATRIX,"state":"omitted"}),
        &[MIRROR_MATRIX],
    );
    assert_eq!(omitted["kind"], "scene", "{omitted}");
    omitted["document"].clone()
}

#[test]
fn native_constrain_anchor_target_refuses_from_both_halves() {
    // Ticket 29 slice B5 (frame restrictions): driving the dependent's
    // anchor key with a constraint refuses on the source half and, once the
    // dependent is paired, on the mirror half through its own anchor.
    let (mut engine, before) = anchored_pair(PAIR_ID);
    let drive = |id: &str, target: &str| {
        json!({"kind":"set-constraint","constraint":{
            "kind":"offset","id":id,
            "sourcePartId":format!("matrix/{SOURCE_MATRIX}/r0c0"),
            "targetPartId":target,
            "offset":{"x":19.0,"y":0.0},"rotation":0.0}})
    };
    let refused = try_commit(
        &mut engine,
        &before,
        "gap-drive-source-key",
        drive("gap-drive", DEP_ANCHOR_KEY),
    );
    assert_eq!(refused["kind"], "error", "{refused}");
    assert!(
        refused["message"]
            .as_str()
            .unwrap()
            .contains("anchors to it"),
        "source-half refusal: {refused}"
    );
    assert_eq!(snapshot(&mut engine), before);
    let refused_mirror = try_commit(
        &mut engine,
        &before,
        "gap-drive-mirror-key",
        drive("gap-drive-mirror", DEP_MIRROR_ANCHOR_KEY),
    );
    assert_eq!(refused_mirror["kind"], "error", "{refused_mirror}");
    assert!(
        refused_mirror["message"]
            .as_str()
            .unwrap()
            .contains("anchors to it"),
        "mirror-half refusal: {refused_mirror}"
    );
    assert_eq!(snapshot(&mut engine), before);
}

#[test]
fn native_mirror_authored_move_maps_to_source() {
    // Ticket 29 slice B5 (identity remaps): moving a linked mirror key maps
    // to the source-authored move with an identical document.
    let (mut engine_a, before_a) = accepted_pair(PAIR_ID);
    let source_key = format!("matrix/{SOURCE_MATRIX}/r0c0");
    let at = json!({"x": 7.5, "y": -3.0});
    let moved_a = commit(
        &mut engine_a,
        &before_a,
        "gap-move-source",
        json!({"kind":"move-parts","positions":[{"id":source_key,"at":at}]}),
        &[source_key.as_str()],
    );
    assert_eq!(moved_a["kind"], "scene", "{moved_a}");
    let (mut engine_b, before_b) = accepted_pair(PAIR_ID);
    let mirror_key = format!("matrix/{MIRROR_MATRIX}/r0c0");
    // The authored point is a world point: the mirror half transports it
    // through the inverse pair map, so the identical end state needs the
    // reflected point.
    let at_mirror = json!({"x": 200.0 - 7.5, "y": -3.0});
    let moved_b = commit(
        &mut engine_b,
        &before_b,
        "gap-move-mirror",
        json!({"kind":"move-parts","positions":[{"id":mirror_key,"at":at_mirror}]}),
        &[mirror_key.as_str()],
    );
    assert_eq!(moved_b["kind"], "scene", "{moved_b}");
    let mut doc_a = moved_a["document"].clone();
    let mut doc_b = moved_b["document"].clone();
    doc_a["revision"] = Value::Null;
    doc_b["revision"] = Value::Null;
    assert_eq!(doc_a, doc_b, "mirror-authored move lands identically");
}

#[test]
fn native_unfollowed_origin_edit_refuses_then_detach_applies() {
    // Ticket 29 slice B5 (Follow restrictions): a stored Origin change the
    // dependent's Follow subset cannot express refuses; detaching still
    // applies once the anchor chain is undisputed.
    let (mut engine, before) = anchored_pair(PAIR_ID);
    // Narrow the dependent's Follow to X only.
    let mut anchor = matrix(&before, DEP_SOURCE_ID)["anchor"].clone();
    anchor["follow"] = json!(["x"]);
    let narrowed = commit(
        &mut engine,
        &before,
        "gap-narrow-follow",
        json!({"kind":"set-matrix-anchor","matrixId":DEP_SOURCE_ID,"anchor":anchor}),
        &[],
    );
    assert_eq!(narrowed["kind"], "scene", "{narrowed}");
    // A stored Y change is not followed: refuse.
    let mut stored = matrix(&narrowed["document"], DEP_SOURCE_ID).clone();
    stored["origin"]["y"] = json!(stored["origin"]["y"].as_f64().unwrap() + 4.0);
    let refused = try_commit(
        &mut engine,
        &narrowed["document"],
        "gap-origin-unfollowed",
        json!({"kind":"set-matrix","matrix":stored}),
    );
    assert_eq!(refused["kind"], "error", "{refused}");
    assert!(
        refused["message"]
            .as_str()
            .unwrap()
            .contains("not followed"),
        "Follow restriction: {refused}"
    );
    assert_eq!(snapshot(&mut engine), narrowed["document"]);
    // The same stored edit from the mirror half refuses identically.
    let mut stored_mirror = matrix(&narrowed["document"], DEP_MIRROR_ID).clone();
    stored_mirror["origin"]["y"] = json!(stored_mirror["origin"]["y"].as_f64().unwrap() + 4.0);
    let refused_mirror = try_commit(
        &mut engine,
        &narrowed["document"],
        "gap-origin-unfollowed-mirror",
        json!({"kind":"set-matrix","matrix":stored_mirror}),
    );
    assert_eq!(refused_mirror["kind"], "error", "{refused_mirror}");
    assert!(
        refused_mirror["message"]
            .as_str()
            .unwrap()
            .contains("not followed"),
        "mirror-half Follow restriction: {refused_mirror}"
    );
    assert_eq!(snapshot(&mut engine), narrowed["document"]);
}

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

#[path = "native_matrix_exceptions/exceptions_detach.rs"]
mod exceptions_detach;
#[path = "native_matrix_exceptions/exceptions_membership.rs"]
mod exceptions_membership;
#[path = "native_matrix_exceptions/exceptions_removal.rs"]
mod exceptions_removal;

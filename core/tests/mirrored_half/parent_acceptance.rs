//! Fresh public-Core lifecycle acceptance for parent ticket 19.
use super::*;

const PAIR_ID: &str = "parent19-lifecycle-pair";

pub(super) fn commit_operation(
    engine: &mut CoreEngine,
    document: &Value,
    transaction: &str,
    target_ids: &[&str],
    operation: Value,
) -> Value {
    request(
        engine,
        json!({"id":transaction,"kind":"edit","command":{
            "baseRevision":document["revision"],
            "transactionId":transaction,
            "phase":"commit",
            "targetIds":target_ids,
            "operation":operation
        }}),
    )
}

fn set_exception(engine: &mut CoreEngine, document: &Value, mirror_id: &str, state: &str) -> Value {
    commit_operation(
        engine,
        document,
        &format!("parent19-{state}-{mirror_id}"),
        &[mirror_id],
        json!({
            "kind":"set-half-exception",
            "pairId":PAIR_ID,
            "itemKind":"part",
            "mirrorId":mirror_id,
            "state":state
        }),
    )
}

fn move_part(engine: &mut CoreEngine, document: &Value, id: &str, x: f64, y: f64) -> Value {
    commit_operation(
        engine,
        document,
        &format!("parent19-move-{id}"),
        &[id],
        json!({
            "kind":"move-parts",
            "positions":[{"id":id,"at":{"x":x,"y":y}}]
        }),
    )
}

fn exception<'a>(document: &'a Value, mirror_id: &str) -> Option<&'a Value> {
    pair(document, PAIR_ID)["halves"]["exceptions"]
        .as_array()?
        .iter()
        .find(|item| item["mirrorId"] == mirror_id)
}

fn correspondence<'a>(document: &'a Value, mirror_id: &str) -> &'a Value {
    pair(document, PAIR_ID)["halves"]["correspondences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["kind"] == "part" && item["mirrorId"] == mirror_id)
        .unwrap_or_else(|| panic!("part correspondence for {mirror_id} is missing"))
}

pub(super) fn assert_history_document(actual: &Value, expected: &Value, label: &str) {
    let mut expected = expected.clone();
    expected["revision"] = actual["revision"].clone();
    assert_eq!(actual, &expected, "{label} must restore the exact document");
}

pub(super) fn save_archive_and_open(document: &Value) -> (CoreEngine, Value) {
    let (packed_reply, packed_buffers) = boardstudio_core::archive::request(
        &json!({
            "kind":"pack-project",
            "projectJson":document.to_string(),
            "assets":[]
        })
        .to_string(),
        &[],
    );
    let packed: Value = serde_json::from_str(&packed_reply).unwrap();
    assert_eq!(packed["kind"], "packed", "{packed}");
    assert_eq!(packed_buffers.len(), 1, "Save returns one project archive");

    let (unpacked_reply, asset_buffers) =
        boardstudio_core::archive::request(r#"{"kind":"unpack-project"}"#, &packed_buffers);
    let unpacked: Value = serde_json::from_str(&unpacked_reply).unwrap();
    assert_eq!(unpacked["kind"], "unpacked", "{unpacked}");
    assert!(asset_buffers.is_empty(), "fixture has no embedded assets");
    let saved_document: Value =
        serde_json::from_str(unpacked["projectJson"].as_str().unwrap()).unwrap();
    assert_eq!(
        saved_document, *document,
        "archive roundtrip retains the saved project"
    );
    open(&saved_document)
}

fn accepted_pair() -> (CoreEngine, Value) {
    let source = synthetic_half::with_construction_features(runtime_supported_document());
    let (mut engine, seed) = seed(&source, "left");
    let created = create(
        &mut engine,
        &seed,
        PAIR_ID,
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let document = created["document"].clone();
    assert!(
        !closure_holes(&document, "left").is_empty()
            && !closure_holes(&document, "right").is_empty(),
        "lifecycle history must include the fixture's nonempty related closure"
    );
    (engine, document)
}

/// A matrix-owned companion exception changes only that child. Its configured companion mode
/// and tweak, sibling correspondence, and unrelated matrix parts survive source edits and Rejoin.
#[test]
fn parent19_matrix_companion_exception_is_selective_and_rejoins_latest_source() {
    for (child, state) in [("diode", "local"), ("led", "omitted")] {
        let (creator, linked) = accepted_pair();
        let target_id = format!("matrix/right-synthetic-keys/r0c0/{child}");
        let sibling_id = format!(
            "matrix/right-synthetic-keys/r0c0/{}",
            if child == "diode" { "led" } else { "diode" }
        );
        let mut configured = linked.clone();
        let halves = &mut configured["hardware"]["pairs"][0]["halves"];
        halves["companionMode"] = json!("duplicate-position");
        let links = halves["correspondences"].as_array_mut().unwrap();
        let selected = links
            .iter_mut()
            .find(|item| item["mirrorId"].as_str() == Some(target_id.as_str()))
            .unwrap();
        selected["companionMode"] = json!("literal-mirror-with-tweak");
        selected["companionTweak"] = json!({"x":1.25,"y":-2.0,"rotation":13.0});
        let configured_selected = correspondence(&configured, &target_id).clone();
        let configured_sibling = correspondence(&configured, &sibling_id).clone();
        drop(creator);

        let (mut engine, linked) = open(&configured);
        let (mut reference_engine, reference_start) = open(&configured);
        let localized = set_exception(&mut engine, &linked, &target_id, state);
        assert_eq!(localized["kind"], "scene", "{localized}");
        let exceptional = localized["document"].clone();
        let original_target = find_part(&linked, &target_id).unwrap().clone();
        let source_child_id = format!("matrix/left-synthetic-keys/r0c0/{child}");
        assert!(
            find_part(&exceptional, &source_child_id).is_some(),
            "{state} preserves source companion geometry"
        );
        assert!(
            exceptional["matrices"]
                .as_array()
                .unwrap()
                .iter()
                .find(|matrix| matrix["id"] == "left-synthetic-keys")
                .unwrap()["partIds"]
                .as_array()
                .unwrap()
                .iter()
                .any(|id| id == &source_child_id),
            "{state} preserves source companion matrix membership"
        );
        if state == "local" {
            assert_eq!(
                find_part(&exceptional, &target_id),
                Some(&original_target),
                "Make Local preserves the visible companion before later source edits"
            );
        }

        let moved = move_part(
            &mut engine,
            &exceptional,
            "matrix/left-synthetic-keys/r0c0",
            7.0,
            11.0,
        );
        let moved_reference = move_part(
            &mut reference_engine,
            &reference_start,
            "matrix/left-synthetic-keys/r0c0",
            7.0,
            11.0,
        );
        assert_eq!(moved["kind"], "scene", "{moved}");
        assert_eq!(moved_reference["kind"], "scene", "{moved_reference}");
        let edited = moved["document"].clone();
        let moved_reference_document = moved_reference["document"].clone();
        let source_key_id = "matrix/left-synthetic-keys/r0c0";
        let source_companion_id = format!("{source_key_id}/{child}");
        let mirror_key_id = "matrix/right-synthetic-keys/r0c0";
        let source_cell = |document: &Value| {
            document["matrices"]
                .as_array()
                .unwrap()
                .iter()
                .find(|matrix| matrix["id"] == "left-synthetic-keys")
                .unwrap()["cells"]
                .as_array()
                .unwrap()
                .iter()
                .find(|cell| cell["row"] == 0 && cell["column"] == 0)
                .unwrap()["offset"]
                .clone()
        };
        let expected_source_pose = part(&moved_reference_document, source_key_id)["pose"].clone();
        let expected_source_companion_pose =
            part(&moved_reference_document, &source_companion_id)["pose"].clone();
        let expected_source_offset = source_cell(&moved_reference_document);
        let (_reopened_reference_engine, reopened_reference) =
            save_archive_and_open(&moved_reference_document);
        let expected_from_latest_source = &reopened_reference;
        assert_eq!(
            part(&reopened_reference, source_key_id)["pose"],
            expected_source_pose,
            "Open preserves the accepted source key move in the reference"
        );
        assert_eq!(
            source_cell(&reopened_reference),
            expected_source_offset,
            "Open preserves the accepted source cell offset in the reference"
        );
        assert_eq!(
            part(&reopened_reference, &source_companion_id)["pose"],
            expected_source_companion_pose,
            "Open preserves the accepted source diode/LED pose in the reference"
        );
        assert_eq!(
            part(&edited, source_key_id)["pose"],
            expected_source_pose,
            "the exception branch accepts the same source key move"
        );
        assert_eq!(
            source_cell(&edited),
            expected_source_offset,
            "the exception branch accepts the same source cell offset"
        );
        assert_eq!(
            part(&edited, &source_companion_id)["pose"],
            expected_source_companion_pose,
            "the exception branch accepts the same source diode/LED pose"
        );
        assert_eq!(
            part(&edited, &sibling_id)["pose"],
            part(&moved_reference_document, &sibling_id)["pose"],
            "an unselected sibling continues to follow its linked source"
        );
        assert_eq!(
            correspondence(&edited, &sibling_id),
            &configured_sibling,
            "unselected sibling correspondence and its placement mode remain intact"
        );
        assert_eq!(
            correspondence(&edited, &target_id)["companionTweak"],
            configured_selected["companionTweak"],
            "selected companion tweak remains associated with its durable exception"
        );
        if state == "local" {
            assert_eq!(find_part(&edited, &target_id).unwrap(), &original_target);
        } else {
            assert!(find_part(&edited, &target_id).is_none());
        }
        assert!(
            find_part(&edited, &sibling_id).is_some(),
            "unselected sibling stays present"
        );

        let (mut reopened_engine, reopened) = save_archive_and_open(&edited);
        if state == "local" {
            assert_eq!(
                find_part(&reopened, &target_id),
                Some(&original_target),
                "Open preserves the Local companion's visible pose and part data"
            );
        }
        assert_eq!(
            part(&reopened, source_key_id)["pose"],
            part(&edited, source_key_id)["pose"],
            "Open preserves the accepted source key move with the exception"
        );
        assert_eq!(
            source_cell(&reopened),
            source_cell(&edited),
            "Open preserves the accepted source cell offset with the exception"
        );
        assert_eq!(
            part(&reopened, &source_companion_id)["pose"],
            expected_source_companion_pose,
            "Open preserves the accepted source diode/LED pose with the exception"
        );
        let source_key_pose = &part(&reopened, source_key_id)["pose"];
        let source_companion_pose = &part(&reopened, &source_companion_id)["pose"];
        let mirrored_key_pose = &part(&reopened_reference, mirror_key_id)["pose"];
        let source_key_rotation = source_key_pose["rotation"].as_f64().unwrap();
        let source_companion_rotation = source_companion_pose["rotation"].as_f64().unwrap();
        let mirrored_key_rotation = mirrored_key_pose["rotation"].as_f64().unwrap();
        let source_dx = source_companion_pose["at"]["x"].as_f64().unwrap()
            - source_key_pose["at"]["x"].as_f64().unwrap();
        let source_dy = source_companion_pose["at"]["y"].as_f64().unwrap()
            - source_key_pose["at"]["y"].as_f64().unwrap();
        let source_radians = source_key_rotation.to_radians();
        let (source_sin, source_cos) = source_radians.sin_cos();
        let tweak = &configured_selected["companionTweak"];
        let local_x =
            -(source_dx * source_cos + source_dy * source_sin) + tweak["x"].as_f64().unwrap();
        let local_y =
            -source_dx * source_sin + source_dy * source_cos + tweak["y"].as_f64().unwrap();
        let mirrored_radians = mirrored_key_rotation.to_radians();
        let (mirrored_sin, mirrored_cos) = mirrored_radians.sin_cos();
        let expected_rejoined_x = mirrored_key_pose["at"]["x"].as_f64().unwrap()
            + local_x * mirrored_cos
            - local_y * mirrored_sin;
        let expected_rejoined_y = mirrored_key_pose["at"]["y"].as_f64().unwrap()
            + local_x * mirrored_sin
            + local_y * mirrored_cos;
        let expected_rejoined_rotation = mirrored_key_rotation
            - (source_companion_rotation - source_key_rotation)
            + tweak["rotation"].as_f64().unwrap();
        assert!(
            (mirrored_key_pose["at"]["x"].as_f64().unwrap()
                - (200.0 - source_key_pose["at"]["x"].as_f64().unwrap()))
            .abs()
                < 1e-9
        );
        assert!(
            (mirrored_key_pose["at"]["y"].as_f64().unwrap()
                - source_key_pose["at"]["y"].as_f64().unwrap())
            .abs()
                < 1e-9
        );
        assert!((mirrored_key_rotation + source_key_rotation).abs() < 1e-9);
        assert_eq!(exception(&reopened, &target_id).unwrap()["state"], state);
        assert_eq!(
            correspondence(&reopened, &target_id)["companionTweak"],
            configured_selected["companionTweak"]
        );
        assert_eq!(correspondence(&reopened, &sibling_id), &configured_sibling);

        let before_rejoin = snapshot(&mut reopened_engine);
        let rejoined = set_exception(&mut reopened_engine, &before_rejoin, &target_id, "linked");
        assert_eq!(rejoined["kind"], "scene", "{rejoined}");
        let rejoined_document = rejoined["document"].clone();
        let rejoined_pose = &part(&rejoined_document, &target_id)["pose"];
        assert!((rejoined_pose["at"]["x"].as_f64().unwrap() - expected_rejoined_x).abs() < 1e-9);
        assert!((rejoined_pose["at"]["y"].as_f64().unwrap() - expected_rejoined_y).abs() < 1e-9);
        assert!(
            (rejoined_pose["rotation"].as_f64().unwrap() - expected_rejoined_rotation).abs() < 1e-9
        );
        assert_eq!(
            part(&rejoined_document, &target_id)["pose"],
            part(expected_from_latest_source, &target_id)["pose"],
            "Rejoin projects the latest source using the retained mode and tweak"
        );
        assert!(exception(&rejoined_document, &target_id).is_none());
        assert_eq!(
            correspondence(&rejoined_document, &target_id),
            &configured_selected
        );
        assert_eq!(
            correspondence(&rejoined_document, &sibling_id),
            &configured_sibling
        );

        let undone = request(
            &mut reopened_engine,
            json!({"id":"parent19-companion-undo","kind":"undo"}),
        );
        assert_eq!(undone["kind"], "scene", "{undone}");
        assert_history_document(&undone["document"], &before_rejoin, "companion Rejoin Undo");
        assert_eq!(
            exception(&undone["document"], &target_id).unwrap()["state"],
            state
        );
        let redone = request(
            &mut reopened_engine,
            json!({"id":"parent19-companion-redo","kind":"redo"}),
        );
        assert_eq!(redone["kind"], "scene", "{redone}");
        assert_history_document(
            &redone["document"],
            &rejoined_document,
            "companion Rejoin Redo",
        );
    }
}

/// Mirror-only placement refuses invalid optional definition/layout fields atomically.
/// The source half and the accepted document remain unchanged for every refusal.
#[test]
fn parent19_mirror_only_optional_definition_and_layout_refusals_are_atomic() {
    let (_, fixture) = accepted_pair();
    let existing_definition = fixture["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|definition| definition["id"] == "synthetic/reset")
        .unwrap()
        .clone();

    let mut conflicting_definition = existing_definition.clone();
    conflicting_definition["name"] = json!("Conflicting reset definition");
    let mut mismatched_definition = existing_definition.clone();
    mismatched_definition["id"] = json!("synthetic/other-reset");
    let cases = [
        (
            "definition-conflict",
            Some(conflicting_definition),
            Some("right-synthetic-keys-layout"),
        ),
        (
            "definition-id-mismatch",
            Some(mismatched_definition),
            Some("right-synthetic-keys-layout"),
        ),
        ("missing-layout", None, Some("missing-mirror-layout")),
        (
            "layout-owned-by-source-board",
            None,
            Some("left-synthetic-keys-layout"),
        ),
    ];

    for (case, definition, layout_id) in cases {
        let (mut engine, document) = accepted_pair();
        let mut placed = part(&document, "right/RST").clone();
        placed["id"] = json!(format!("right/ONLY-{case}"));
        placed["reference"] = json!(format!("ONLY-{}", case.to_uppercase()));
        let transaction = format!("parent19-mirror-only-refusal-{case}");
        let operation = json!({
            "kind":"add-mirror-only-part",
            "part":placed,
            "boardId":"right",
            "definition":definition,
            "layoutId":layout_id
        });
        let reply: Value = serde_json::from_str(
            &engine.request(
                &json!({"id":transaction,"kind":"edit","command":{
                    "baseRevision":document["revision"],
                    "transactionId":transaction,
                    "phase":"commit",
                    "targetIds":[],
                    "operation":operation
                }})
                .to_string(),
            ),
        )
        .unwrap();
        assert_eq!(reply["kind"], "error", "{case}: {reply}");

        let after_refusal = snapshot(&mut engine);
        assert_eq!(
            after_refusal, document,
            "{case} refusal preserves the complete accepted document and revision"
        );
        let source_board = |document: &Value| {
            document["boards"]
                .as_array()
                .unwrap()
                .iter()
                .find(|board| board["id"] == "left")
                .unwrap()
                .clone()
        };
        assert_eq!(
            source_board(&after_refusal)["partIds"],
            source_board(&document)["partIds"],
            "{case} refusal leaves source board membership unchanged"
        );
        for source_id in source_board(&document)["partIds"].as_array().unwrap() {
            assert_eq!(
                part(&after_refusal, source_id.as_str().unwrap()),
                part(&document, source_id.as_str().unwrap()),
                "{case} refusal leaves source Part {source_id} unchanged"
            );
        }
    }
}

#[test]
fn parent19_local_matrix_companions_survive_matrix_edits_and_keep_explicit_local_moves() {
    let target_id = "matrix/right-synthetic-keys/r0c0/diode";
    for matrix_id in ["left-synthetic-keys", "right-synthetic-keys"] {
        let (_creator, linked) = accepted_pair();
        let (mut engine, linked) = open(&linked);
        let localized = set_exception(&mut engine, &linked, target_id, "local");
        assert_eq!(localized["kind"], "scene", "{localized}");
        let local_document = localized["document"].clone();
        let original = find_part(&local_document, target_id).unwrap().clone();
        let before_preview = snapshot(&mut engine);
        let placement = json!({
            "kind":"set-matrix-placement",
            "matrixId":matrix_id,
            "change":{"kind":"row-offset","row":0,"value":{"x":8.0,"y":3.0}}
        });
        let preview = request(
            &mut engine,
            json!({"id":format!("parent19-local-matrix-preview-{matrix_id}"),"kind":"edit","command":{
                "baseRevision":local_document["revision"],
                "transactionId":format!("parent19-local-matrix-preview-{matrix_id}"),
                "phase":"preview",
                "targetIds":[matrix_id],
                "operation":placement
            }}),
        );
        assert_eq!(preview["kind"], "preview", "{preview}");
        let preview_transform = |id: &str| {
            preview["scene"]["transforms"]
                .as_array()
                .unwrap()
                .iter()
                .find(|transform| transform["id"] == id)
                .unwrap_or_else(|| panic!("preview transform for {id} is missing: {preview}"))
                .clone()
        };
        assert_eq!(
            preview_transform(target_id)["pose"],
            original["pose"],
            "matrix placement Preview keeps the Local child at its accepted world pose"
        );
        let source_sibling = "matrix/left-synthetic-keys/r0c0/led";
        let mirror_sibling = "matrix/right-synthetic-keys/r0c0/led";
        let source_sibling_pose = preview_transform(source_sibling)["pose"].clone();
        let mirror_sibling_pose = preview_transform(mirror_sibling)["pose"].clone();
        assert_eq!(
            mirror_sibling_pose["at"]["x"],
            json!(200.0 - source_sibling_pose["at"]["x"].as_f64().unwrap()),
            "the linked sibling Preview remains projected across the pair"
        );
        assert_eq!(
            mirror_sibling_pose["at"]["y"],
            source_sibling_pose["at"]["y"]
        );
        assert_eq!(
            snapshot(&mut engine),
            before_preview,
            "Preview must not publish"
        );
        let edited = commit_operation(
            &mut engine,
            &local_document,
            &format!("parent19-local-matrix-placement-{matrix_id}"),
            &[matrix_id],
            json!({
                "kind":"set-matrix-placement",
                "matrixId":matrix_id,
                "change":{"kind":"row-offset","row":0,"value":{"x":8.0,"y":3.0}}
            }),
        );
        assert_eq!(edited["kind"], "scene", "{edited}");
        let edited_document = edited["document"].clone();
        assert_eq!(
            find_part(&edited_document, target_id),
            Some(&original),
            "source and mirror matrix placement preserve Local companion data"
        );
        assert_eq!(
            exception(&edited_document, target_id).unwrap()["state"],
            "local"
        );
        let (_reopened_engine, reopened) = save_archive_and_open(&edited_document);
        assert_eq!(find_part(&reopened, target_id), Some(&original));
    }

    let (_creator, linked) = accepted_pair();
    let (mut engine, linked) = open(&linked);
    let localized = set_exception(&mut engine, &linked, target_id, "local");
    let local_document = localized["document"].clone();
    let moved = move_part(&mut engine, &local_document, target_id, 186.0, 4.0);
    assert_eq!(moved["kind"], "scene", "{moved}");
    let moved_document = moved["document"].clone();
    assert_eq!(
        find_part(&moved_document, target_id).unwrap()["pose"]["at"],
        json!({"x":186.0,"y":4.0}),
        "an explicit Local child move updates its visible pose"
    );
    let (_reopened_engine, reopened) = save_archive_and_open(&moved_document);
    assert_eq!(
        find_part(&reopened, target_id).unwrap()["pose"]["at"],
        json!({"x":186.0,"y":4.0}),
        "Open retains the rebased Local child move"
    );
}

#[test]
fn parent19_replace_document_preserves_authored_local_companion_fields() {
    let target_id = "matrix/right-synthetic-keys/r0c0/diode";
    let (_creator, linked) = accepted_pair();
    let (mut engine, linked) = open(&linked);
    let localized = set_exception(&mut engine, &linked, target_id, "local");
    let mut replacement = localized["document"].clone();
    let target = replacement["parts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|part| part["id"] == target_id)
        .unwrap();
    target["pose"]["rotation"] = json!(target["pose"]["rotation"].as_f64().unwrap() + 17.0);
    let expected = target.clone();
    let replaced = commit_operation(
        &mut engine,
        &localized["document"],
        "parent19-replace-local-companion-fields",
        &[target_id],
        json!({"kind":"replace-document","document":replacement}),
    );
    assert_eq!(replaced["kind"], "scene", "{replaced}");
    assert_eq!(
        find_part(&replaced["document"], target_id),
        Some(&expected),
        "ReplaceDocument retains Local authored fields while reconciliation rebases its cell"
    );
}

fn select_matrix(engine: &mut CoreEngine, doc: &Value, state: &str) -> Value {
    let id = "right-synthetic-keys";
    let reply = commit_operation(
        engine,
        doc,
        state,
        &[id],
        json!({
            "kind":"set-half-exception", "pairId":PAIR_ID, "itemKind":"matrix", "mirrorId":id, "state":state
        }),
    );
    assert_eq!(reply["kind"], "scene", "{reply}");
    reply["document"].clone()
}

fn check_slot(doc: &Value, old: &Value, from: &str, to: &str) {
    let mut typed: ProjectDoc = serde_json::from_value(doc.clone()).unwrap();
    let matrix = typed
        .matrices
        .iter()
        .find(|m| m.id == "right-synthetic-keys")
        .unwrap()
        .clone();
    // Force regeneration from the saved cell even when the Part has an authored pose.
    for part in &mut typed.parts {
        if let Some(properties) = &mut part.properties {
            properties.remove("layoutOverride");
        }
    }
    boardstudio_core_kernel::matrix::set_matrix(&mut typed, &matrix).unwrap();
    for suffix in ["", "/diode", "/led"] {
        let id = format!("{to}{suffix}");
        let expected = &part(old, &format!("{from}{suffix}"))["pose"];
        let saved = &part(doc, &id)["pose"];
        let generated =
            serde_json::to_value(typed.parts.iter().find(|p| p.id == id).unwrap().pose).unwrap();
        for pose in [saved, &generated] {
            for path in ["/at/x", "/at/y", "/rotation"] {
                let coord =
                    |value: &Value| value.pointer(path).and_then(Value::as_f64).unwrap_or(0.0);
                assert!(
                    (coord(pose) - coord(expected)).abs() < 1e-9,
                    "{id} {path}: {pose} expected {expected}"
                );
            }
        }
    }
}

#[test]
fn cutover_column_compaction_preserves_local_world_pose() {
    check_compaction("column", "r0c1");
}

#[test]
fn cutover_row_compaction_preserves_local_world_pose() {
    check_compaction("row", "r1c0");
}

fn check_compaction(axis: &str, coordinate: &str) {
    let (mut engine, doc) = accepted_pair();
    let old = format!("matrix/right-synthetic-keys/{coordinate}");
    let localized = set_exception(&mut engine, &doc, &old, "local");
    let before = &localized["document"];
    let reply = commit_operation(
        &mut engine,
        before,
        axis,
        &["left-synthetic-keys"],
        json!({
            "kind":format!("remove-matrix-{axis}"), "matrixId":"left-synthetic-keys", (axis):0
        }),
    );
    assert_eq!(reply["kind"], "scene", "{reply}");
    let new = "matrix/right-synthetic-keys/r0c0";
    check_slot(&reply["document"], before, &old, new);
    let (_, reopened) = save_archive_and_open(&reply["document"]);
    check_slot(&reopened, before, &old, new);
}

#[test]
fn cutover_omitted_owner_rejoins_rotated_local_slot() {
    let (mut engine, doc) = accepted_pair();
    let source = "left-synthetic-keys";
    let mut matrix = doc["matrices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == source)
        .unwrap()
        .clone();
    matrix["cells"][0]["rotation"] = json!(27.0);
    let rotated = commit_operation(
        &mut engine,
        &doc,
        "rotate-cell",
        &[source],
        json!({"kind":"set-matrix","matrix":matrix}),
    );
    assert_eq!(rotated["kind"], "scene", "{rotated}");
    let id = "matrix/right-synthetic-keys/r0c0";
    let diode = "matrix/right-synthetic-keys/r0c0/diode";
    let local_diode = set_exception(&mut engine, &rotated["document"], diode, "local");
    let moved = move_part(&mut engine, &local_diode["document"], diode, 187.0, 13.0);
    assert_eq!(moved["kind"], "scene", "{moved}");
    let localized = set_exception(&mut engine, &moved["document"], id, "local");
    let before = &localized["document"];
    assert_eq!(
        part(before, diode)["pose"]["at"],
        json!({"x":187.0,"y":13.0})
    );
    let omitted = select_matrix(&mut engine, before, "omitted");
    let (mut engine, reopened) = save_archive_and_open(&omitted);
    let rejoined = select_matrix(&mut engine, &reopened, "linked");
    check_slot(&rejoined, before, id, id);
    let (_, reopened) = save_archive_and_open(&rejoined);
    check_slot(&reopened, before, id, id);
}

#[test]
fn cutover_local_owner_rejoins_overlapping_growth() {
    let (mut engine, doc) = accepted_pair();
    let mut doc = select_matrix(&mut engine, &doc, "local");
    for (id, columns) in [("left-synthetic-keys", 3), ("right-synthetic-keys", 4)] {
        let mut matrix = doc["matrices"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["id"] == id)
            .unwrap()
            .clone();
        matrix["columns"] = json!(columns);
        for column in 2..columns {
            for row in 0..2 {
                let mut cell = matrix["cells"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|c| c["row"] == row && c["column"] == 0)
                    .unwrap()
                    .clone();
                cell["column"] = json!(column);
                matrix["cells"].as_array_mut().unwrap().push(cell);
            }
        }
        let reply = commit_operation(
            &mut engine,
            &doc,
            id,
            &[id],
            json!({"kind":"set-matrix", "matrix":matrix}),
        );
        assert_eq!(reply["kind"], "scene", "{reply}");
        doc = reply["document"].clone();
    }
    let rejoined = select_matrix(&mut engine, &doc, "linked");
    let (_, reopened) = save_archive_and_open(&rejoined);
    for doc in [&rejoined, &reopened] {
        for row in 0..2 {
            let adopted = format!("matrix/right-synthetic-keys/r{row}c2");
            let discarded = format!("matrix/right-synthetic-keys/r{row}c3");
            for suffix in ["", "/diode", "/led"] {
                let id = format!("{adopted}{suffix}");
                assert!(find_part(doc, &id).is_some());
                assert_eq!(
                    correspondence(doc, &id)["sourceId"],
                    format!("matrix/left-synthetic-keys/r{row}c2{suffix}")
                );
                assert!(find_part(doc, &format!("{discarded}{suffix}")).is_none());
            }
        }
    }
}

/// Rename only the independent mirror Part and references to it in the persisted document.
fn pair_with_decoy(id: &str) -> (CoreEngine, Value) {
    let (_, linked) = accepted_pair();
    let renamed = linked
        .to_string()
        .replace("\"right/RST\"", &json!(id).to_string());
    open(&serde_json::from_str(&renamed).unwrap())
}

#[test]
fn independent_decoy_is_not_the_parent_of_a_linked_key() {
    let (mut engine, linked) = pair_with_decoy("matrix/right-synthetic-keys");
    let local = set_exception(&mut engine, &linked, "matrix/right-synthetic-keys", "local");
    let key = "matrix/right-synthetic-keys/r1c0";
    let source = "matrix/left-synthetic-keys/r1c0";
    let moved = move_part(&mut engine, &local["document"], key, 212., 12.);
    assert_eq!(
        part(&moved["document"], source)["pose"]["at"],
        json!({"x":-12.0,"y":12.0})
    );
    assert_eq!(
        part(&moved["document"], key)["pose"]["at"],
        json!({"x":212.0,"y":12.0})
    );
}

#[test]
fn independent_decoy_is_not_a_matrix_companion() {
    let id = "matrix/right-synthetic-keys/r9c9/spare";
    let (mut engine, linked) = pair_with_decoy(id);
    let source = "left/RST";
    let authored = move_part(&mut engine, &linked, source, -8., 16.);
    let (mut engine, reopened) = save_archive_and_open(&authored["document"]);
    assert_history_document(&reopened, &authored["document"], "archive");
    let moved = move_part(&mut engine, &reopened, id, 212., 12.);
    assert_eq!(
        part(&moved["document"], source)["pose"]["at"],
        json!({"x":-12.0,"y":12.0})
    );
    assert_eq!(
        part(&moved["document"], id)["pose"]["at"],
        json!({"x":212.0,"y":12.0})
    );
    let undo = request(&mut engine, json!({"id":"undo","kind":"undo"}));
    let redo = request(&mut engine, json!({"id":"redo","kind":"redo"}));
    assert_history_document(&undo["document"], &reopened, "undo");
    assert_history_document(&redo["document"], &moved["document"], "redo");
    for reply in [&authored, &moved, &undo, &redo] {
        let changed = reply["scene"]["changedIds"].as_array().unwrap();
        assert!(
            changed.contains(&json!(source)) && changed.contains(&json!(id)),
            "{changed:?}"
        );
    }
    // History invalidates the full scene. Committed moves report just affected entities.
    for reply in [&authored, &moved] {
        assert_eq!(reply["scene"]["changedIds"], json!([source, id]));
    }
}

#[test]
fn a_linked_independent_part_under_a_local_keys_prefix_still_edits_its_source() {
    let id = "matrix/right-synthetic-keys/r1c0/spare";
    let (mut engine, linked) = pair_with_decoy(id);
    let local = set_exception(
        &mut engine,
        &linked,
        "matrix/right-synthetic-keys/r1c0",
        "local",
    );
    let moved = move_part(&mut engine, &local["document"], id, 212., 12.);
    assert_eq!(
        part(&moved["document"], "left/RST")["pose"]["at"],
        json!({"x":-12.0,"y":12.0})
    );
    assert_eq!(
        part(&moved["document"], id)["pose"]["at"],
        json!({"x":212.0,"y":12.0})
    );
}

#[test]
fn ownership_review_noncanonical_slot_spelling_remains_independent() {
    let source = "matrix/left-synthetic-keys/r01c0/spare";
    let mirror = "matrix/right-synthetic-keys/r1c0/spare";
    let (_, linked) = accepted_pair();
    let renamed = linked
        .to_string()
        .replace("\"left/RST\"", &json!(source).to_string())
        .replace("\"right/RST\"", &json!(mirror).to_string());
    let (mut engine, accepted) = open(&serde_json::from_str(&renamed).unwrap());
    let moved = move_part(&mut engine, &accepted, "left/U1", -22.0, 4.0);
    assert_eq!(moved["kind"], "scene", "{moved}");
    let moved = &moved["document"];
    let mut replacement = moved.clone();
    replacement["name"] = json!("Independent slot-like names");
    let reply = commit_operation(
        &mut engine,
        moved,
        "replace-with-independent-slot-like-names",
        &[],
        json!({"kind":"replace-document","document":replacement}),
    );
    assert_eq!(reply["kind"], "scene", "{reply}");
    for id in [source, mirror] {
        assert_eq!(part(&reply["document"], id), part(moved, id));
    }
}

#[test]
fn ownership_review_detached_local_companion_can_be_omitted() {
    let key = "matrix/right-synthetic-keys/r0c0";
    let diode = "matrix/right-synthetic-keys/r0c0/diode";
    let (mut engine, linked) = accepted_pair();
    let local = set_exception(&mut engine, &linked, diode, "local");
    assert_eq!(local["kind"], "scene", "{local}");
    let moved = move_part(&mut engine, &local["document"], diode, 123.0, 47.0);
    assert_eq!(moved["kind"], "scene", "{moved}");
    let detached = set_exception(&mut engine, &moved["document"], key, "omitted");
    assert_eq!(detached["kind"], "scene", "{detached}");
    let detached = &detached["document"];
    assert_eq!(part(detached, diode), part(&moved["document"], diode));
    let (mut engine, reopened) = save_archive_and_open(detached);
    let omitted = set_exception(&mut engine, &reopened, diode, "omitted");
    assert_eq!(omitted["kind"], "scene", "{omitted}");
    let omitted = &omitted["document"];
    assert!(find_part(omitted, diode).is_none());
    assert_eq!(exception(omitted, diode).unwrap()["state"], "omitted");
    assert_eq!(exception(omitted, key).unwrap()["state"], "omitted");
    let undo = request(&mut engine, json!({"id":"undo","kind":"undo"}));
    let redo = request(&mut engine, json!({"id":"redo","kind":"redo"}));
    assert_history_document(&undo["document"], &reopened, "undo");
    assert_history_document(&redo["document"], omitted, "redo");
    let (_, reopened) = save_archive_and_open(&redo["document"]);
    assert_history_document(&reopened, omitted, "archive");
}

#[test]
fn an_unrelated_move_preserves_the_saved_binding_of_an_omitted_key() {
    let key = "matrix/right-synthetic-keys/r0c0";
    let (mut engine, linked) = accepted_pair();
    let mut omitted = set_exception(&mut engine, &linked, key, "omitted")["document"].clone();
    assert!(find_part(&omitted, key).is_none());
    omitted["keymap"] = json!({
        "layers":[{"id":"base","name":"Base","bindings":{
            key:{"kind":"key-press","keycode":"W"}
        },"sensors":{}}],
        "macros":[]
    });
    let (mut engine, reopened) = save_archive_and_open(&omitted);
    assert_eq!(reopened["keymap"], omitted["keymap"]);
    let moved = move_part(&mut engine, &reopened, "left/U1", -22.0, 4.0);
    assert_eq!(moved["kind"], "scene", "{moved}");
    assert_eq!(moved["document"]["keymap"], reopened["keymap"]);
}

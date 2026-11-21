//! Commit no-op behavior through the public paired Core route.
use super::*;

fn paired_engine(pair_id: &str) -> (CoreEngine, Value) {
    let (mut engine, source) = seed(&runtime_supported_document(), "left");
    let created = create(
        &mut engine,
        &source,
        pair_id,
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    (engine, created["document"].clone())
}

fn commit(
    engine: &mut CoreEngine,
    document: &Value,
    transaction_id: &str,
    target_ids: &[&str],
    operation: Value,
) -> Value {
    request(
        engine,
        json!({"id":transaction_id,"kind":"edit","command":{
            "baseRevision":document["revision"],
            "transactionId":transaction_id,
            "phase":"commit",
            "targetIds":target_ids,
            "operation":operation
        }}),
    )
}

fn outline_feature(document: &Value, id: &str) -> Value {
    document["outline"]
        .as_array()
        .unwrap()
        .iter()
        .find(|feature| feature["id"] == id)
        .unwrap_or_else(|| panic!("outline feature {id} is missing"))
        .clone()
}

fn top_level_deltas(before: &Value, after: &Value) -> Vec<String> {
    let (Some(before), Some(after)) = (before.as_object(), after.as_object()) else {
        return vec!["<non-object document>".into()];
    };
    before
        .keys()
        .chain(after.keys())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .filter(|key| before.get(*key) != after.get(*key))
        .cloned()
        .collect()
}

#[test]
fn prepared_same_value_commit_preserves_redo() {
    let (mut engine, initial) = paired_engine("parent19-noop-redo");
    let baseline_feature = outline_feature(&initial, "synthetic-boundary");
    let mut edited_feature = baseline_feature.clone();
    edited_feature["center"] = json!({"x":14.0,"y":9.0});
    let committed = commit(
        &mut engine,
        &initial,
        "parent19-noop-redo-change",
        &["synthetic-boundary"],
        // Ticket 37: raw SetOutline is deleted; SetFeature is the live typed write.
        json!({"kind":"apply-outline-edit","edit":{
            "target":{"kind":"board","boardId":"left"},
            "change":{"set-feature":{"feature":edited_feature}},
            "seed":51, "reviewed": null}}),
    );
    assert_eq!(committed["kind"], "scene", "{committed}");

    let undone = request(
        &mut engine,
        json!({"id":"parent19-noop-redo-undo","kind":"undo"}),
    );
    assert_eq!(undone["kind"], "scene", "{undone}");
    assert_eq!(
        outline_feature(&undone["document"], "synthetic-boundary"),
        baseline_feature
    );
    let before_noop = undone["document"].clone();

    let noop = commit(
        &mut engine,
        &before_noop,
        "parent19-noop-redo-repeat",
        &["synthetic-boundary"],
        json!({"kind":"apply-outline-edit","edit":{
            "target":{"kind":"board","boardId":"left"},
            "change":{"set-feature":{"feature":baseline_feature}},
            "seed":52, "reviewed": null}}),
    );
    assert_eq!(noop["kind"], "scene", "{noop}");
    assert!(
        noop["document"] == before_noop,
        "same-value Commit must preserve document and revision; changed fields: {:?}",
        top_level_deltas(&before_noop, &noop["document"])
    );

    let redone = request(
        &mut engine,
        json!({"id":"parent19-noop-redo-redo","kind":"redo"}),
    );
    assert_eq!(redone["kind"], "scene", "{redone}");
    let mut expected_redone = committed["document"].clone();
    expected_redone["revision"] = redone["document"]["revision"].clone();
    assert_eq!(
        redone["document"], expected_redone,
        "same-value Commit must not clear Redo"
    );
}

#[test]
fn prepared_global_only_definition_change_is_committed_and_undoable() {
    let (mut engine, before) = paired_engine("parent19-global-only");
    assert!(
        before
            .get("moduleDefinitions")
            .is_none_or(|definitions| definitions.as_array().is_some_and(Vec::is_empty))
    );
    let mut definition = imported_haptic();
    definition["name"] = json!("Global-only prepared definition");

    let committed = commit(
        &mut engine,
        &before,
        "parent19-global-definition-add",
        &[],
        json!({"kind":"set-module-definition","definition":definition}),
    );
    assert_eq!(committed["kind"], "scene", "{committed}");
    assert_eq!(
        committed["document"]["revision"],
        before["revision"].as_u64().unwrap() + 1,
        "a changed document-level field must be accepted even when it has no changed entity ID"
    );
    assert_eq!(
        committed["document"]["moduleDefinitions"][0]["name"],
        "Global-only prepared definition"
    );
    assert!(
        committed["scene"]["changedIds"]
            .as_array()
            .unwrap()
            .is_empty(),
        "expected no changed entity IDs for global module metadata; got {:?}; changed fields: {:?}",
        committed["scene"]["changedIds"],
        top_level_deltas(&before, &committed["document"])
    );
    let mut expected = before.clone();
    expected["moduleDefinitions"] = committed["document"]["moduleDefinitions"].clone();
    expected["revision"] = committed["document"]["revision"].clone();
    assert_eq!(
        committed["document"], expected,
        "only global module metadata changed"
    );

    let undone = request(
        &mut engine,
        json!({"id":"parent19-global-definition-undo","kind":"undo"}),
    );
    assert_eq!(undone["kind"], "scene", "{undone}");
    let mut expected_undo = before.clone();
    expected_undo["revision"] = undone["document"]["revision"].clone();
    assert_eq!(undone["document"], expected_undo);
    let redone = request(
        &mut engine,
        json!({"id":"parent19-global-definition-redo","kind":"redo"}),
    );
    assert_eq!(redone["kind"], "scene", "{redone}");
    let mut expected_redo = committed["document"].clone();
    expected_redo["revision"] = redone["document"]["revision"].clone();
    assert_eq!(redone["document"], expected_redo);
}

#[test]
fn linked_matrix_placement_preview_and_history_keep_serialized_identities() {
    let (mut engine, linked) =
        super::open_acceptance::open_two_pairs("placement-history", "placement-history-other");
    let companion = "matrix/right-synthetic-keys/r0c0/diode";
    let tweak = json!({"x":1.25,"y":-0.5,"rotation":4.0});
    let set_tweak = json!({"kind":"set-companion-placement","pairId":"placement-history",
        "mode":"literal-mirror-with-tweak","tweak":tweak,
        "targets":[{"kind":"item","itemKind":"part","mirrorId":companion}]});
    let before = commit(
        &mut engine,
        &linked,
        "placement-tweak",
        &[companion],
        set_tweak.clone(),
    )["document"]
        .clone();
    let link = pair(&before, "placement-history")["halves"]["correspondences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|link| link["mirrorId"] == companion)
        .unwrap();
    assert_eq!(link["companionTweak"], tweak);
    let matrix_id = "right-synthetic-keys";
    let placement = json!({"kind":"set-matrix-placement","matrixId":matrix_id,
        "change":{"kind":"column-stagger","column":1,"value":4.0}});
    let preview = request(
        &mut engine,
        json!({"id":"placement-preview","kind":"edit","command":{
            "baseRevision":before["revision"],"transactionId":"placement","phase":"preview",
            "targetIds":[matrix_id],"operation":placement
        }}),
    );
    assert_eq!(preview["kind"], "preview");
    assert_eq!(snapshot(&mut engine), before);

    let committed = commit(&mut engine, &before, "placement", &[matrix_id], placement);
    for field in ["transforms", "matrixScenes", "changedIds"] {
        assert!(
            preview["scene"][field].is_array(),
            "missing Preview {field}"
        );
        assert_eq!(
            preview["scene"][field], committed["scene"][field],
            "{field}"
        );
    }
    let changed = committed["scene"]["changedIds"].as_array().unwrap();
    for id in ["left-synthetic-keys", matrix_id] {
        assert!(changed.contains(&json!(id)), "missing {id}: {changed:?}");
    }
    assert!(
        changed
            .iter()
            .all(|id| !id.as_str().unwrap().contains("other-"))
    );
    let committed_document = committed["document"].clone();
    assert_eq!(
        committed_document["hardware"]["pairs"],
        before["hardware"]["pairs"]
    );

    let undone = request(&mut engine, json!({"kind":"undo","id":"placement-undo"}));
    let mut expected = before;
    expected["revision"] = undone["document"]["revision"].clone();
    assert_eq!(undone["document"], expected);
    let noop = commit(
        &mut engine,
        &expected,
        "placement-noop",
        &[companion],
        set_tweak,
    );
    assert_eq!(noop["document"], expected);
    assert_eq!(noop["scene"]["changedIds"], json!([]));

    let redone = request(&mut engine, json!({"kind":"redo","id":"placement-redo"}));
    let mut expected = committed_document;
    expected["revision"] = redone["document"]["revision"].clone();
    assert_eq!(redone["document"], expected);
}

use super::*;

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

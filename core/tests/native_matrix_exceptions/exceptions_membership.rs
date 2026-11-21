use super::*;

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

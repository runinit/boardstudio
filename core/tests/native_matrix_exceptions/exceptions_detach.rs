use super::*;

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

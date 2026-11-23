use super::*;

#[test]
fn reanchoring_a_matrix_moves_it_and_keeps_its_stored_frame() {
    let (mut engine, doc) = anchored_fixture();
    let origin_before = matrix(&doc, "thumb").origin;
    let anchor_before = anchor_of(&doc, "thumb");
    let key_before = at(&doc, &member("thumb", 0, 0));
    let moved = key_anchor("main", 0, 0);
    let (_, committed) = commit_edit(&mut engine, &doc, reanchor("thumb", moved.clone()));
    // Re-anchoring moves the matrix onto the new anchor but keeps its stored
    // frame: Origin is untouched.
    assert_eq!(anchor_of(&committed, "thumb"), Some(moved));
    assert_eq!(matrix(&committed, "thumb").origin, origin_before);
    assert_ne!(at(&committed, &member("thumb", 0, 0)), key_before);
    // One Undo entry restores the old anchor and pose.
    let undone_doc = undone(&mut engine);
    assert_eq!(anchor_of(&undone_doc, "thumb"), anchor_before);
    assert_eq!(at(&undone_doc, &member("thumb", 0, 0)), key_before);
}

#[test]
fn add_anchored_matrix_previews_without_mutation_and_undoes_in_one_entry() {
    let (mut engine, doc) = fixture();
    let operation = add_anchored("thumb", key_anchor("main", 0, 2));
    // Preview evaluates the new members but stores nothing.
    let previewed = preview(&mut engine, &doc, operation.clone());
    assert!(
        previewed
            .matrix_scenes
            .iter()
            .any(|scene| scene.matrix_id == "thumb"),
        "preview must evaluate the added matrix"
    );
    assert_eq!(snapshot(&mut engine), doc);
    let (_, committed) = commit_edit(&mut engine, &doc, operation);
    assert!(committed.matrices.iter().any(|matrix| matrix.id == "thumb"));
    assert!(
        committed
            .layouts
            .iter()
            .find(|layout| layout.id == "layout")
            .is_some_and(|layout| layout.matrix_ids.contains(&"thumb".to_string()))
    );
    // One Undo restores pre-Add membership: matrix, Layout entry and members.
    let undone_doc = undone(&mut engine);
    assert_eq!(undone_doc.matrices, doc.matrices);
    assert_eq!(undone_doc.layouts, doc.layouts);
    assert_eq!(undone_doc.parts, doc.parts);
    // The Add was a single history entry: nothing remains to undo.
    match engine.handle(CoreRequest::Undo {
        id: "undo-empty".into(),
    }) {
        CoreReply::Error { message, .. } => assert!(
            message.contains("History is empty"),
            "unexpected message: {message:?}"
        ),
        other => panic!("expected empty history, got: {other:?}"),
    }
    assert_eq!(redone(&mut engine).matrices, committed.matrices);
}

#[test]
fn anchor_graph_refusals_leave_state_unchanged() {
    let (mut engine, doc) = anchored_rich_fixture();
    for (name, operation, reason) in [
        (
            "self",
            reanchor("thumb", key_anchor("thumb", 0, 0)),
            "cannot anchor to itself",
        ),
        (
            "cycle",
            reanchor("main", key_anchor("thumb", 0, 0)),
            "anchor cycle",
        ),
        (
            "missing-target",
            reanchor("thumb", key_anchor("ghost", 0, 0)),
            "anchor target ghost is missing",
        ),
        (
            "out-of-range",
            reanchor("thumb", key_anchor("main", 0, 9)),
            "anchor key matrix/main/r0c9 is missing",
        ),
        (
            "cross-layout",
            reanchor("thumb", key_anchor("aux", 0, 0)),
            "same board and Layout",
        ),
        (
            "cross-board",
            reanchor("thumb", key_anchor("far", 0, 0)),
            "same board and Layout",
        ),
        (
            "duplicate-id",
            add_anchored("thumb", key_anchor("main", 0, 0)),
            "Duplicate matrix identity",
        ),
        (
            "bad-id",
            add_anchored("bad id", key_anchor("main", 0, 0)),
            "letters, digits",
        ),
        (
            "missing-matrix",
            reanchor("ghost", key_anchor("main", 0, 0)),
            "Matrix not found",
        ),
        (
            "detach-unanchored",
            EditOperation::DetachMatrixAnchor {
                matrix_id: "main".into(),
            },
            "has no anchor",
        ),
        (
            "constrain-anchor-key",
            EditOperation::SetConstraint {
                constraint: Constraint::Offset {
                    id: "drive".into(),
                    source_part_id: member("main", 0, 0),
                    target_part_id: member("main", 0, 2),
                    offset: Vec2 { x: 19.0, y: 0.0 },
                    rotation: 0.0,
                },
            },
            "anchors to it",
        ),
    ] {
        let message = error(&mut engine, &doc, operation);
        assert!(
            message.contains(reason),
            "{name}: expected refusal containing {reason:?}, got {message:?}"
        );
        assert_eq!(snapshot(&mut engine), doc, "{name}: refusal changed state");
    }
}

#[test]
fn anchor_follow_covers_every_subset_with_zeroing() {
    // Each subset keeps exactly its followed components of the anchor motion;
    // unfollowed components read world zero, even for the relative origin.
    for (x, y, rotation) in [
        (false, false, false),
        (true, false, false),
        (false, true, false),
        (false, false, true),
        (true, true, false),
        (true, false, true),
        (false, true, true),
        (true, true, true),
    ] {
        let (mut engine, doc) = anchored_fixture();
        let doc = set_follow(&mut engine, &doc, x, y, rotation);
        let (_, moved) = commit_edit(
            &mut engine,
            &doc,
            move_part(member("main", 0, 2), Vec2 { x: 43.0, y: 7.0 }),
        );
        let expected = Vec2 {
            x: if x { 43.0 } else { 0.0 },
            y: if y { 7.0 - 19.0 } else { 0.0 },
        };
        assert_eq!(
            pose_of(&moved, &member("thumb", 0, 0)).at,
            expected,
            "follow x={x} y={y} rotation={rotation}"
        );
        assert_eq!(
            pose_of(&moved, &member("thumb", 0, 0)).rotation,
            0.0,
            "follow x={x} y={y} rotation={rotation}"
        );
    }
}

#[test]
fn compaction_remaps_anchors_onto_the_surviving_key() {
    let (mut engine, doc) = anchored_fixture();
    let (_, compacted) = commit_edit(
        &mut engine,
        &doc,
        EditOperation::RemoveMatrixColumn {
            matrix_id: "main".into(),
            column: 0,
        },
    );
    // The anchor followed its physical key from (0,2) to (0,1), never
    // retargeting the new key at the old coordinate.
    assert_eq!(
        anchor_of(&compacted, "thumb"),
        Some(key_anchor("main", 0, 1))
    );
    assert_at(&compacted, &member("thumb", 0, 0), 19.0, -19.0);
    assert_at(&compacted, &member("thumb", 0, 2), 57.0, -19.0);
}

#[test]
fn disabled_and_deleted_anchor_keys_keep_anchoring() {
    for (name, operation) in [
        (
            "disabled",
            EditOperation::SetMatrixCellsEnabled {
                cells: [(String::from("main"), vec![(0, 2)])].into_iter().collect(),
                enabled: false,
            },
        ),
        (
            "deleted",
            EditOperation::RemoveMatrixCells {
                matrix_id: "main".into(),
                cells: vec![(0, 2)],
            },
        ),
    ] {
        let (mut engine, doc) = anchored_fixture();
        let (_, edited) = commit_edit(&mut engine, &doc, operation);
        assert!(
            anchor_of(&edited, "thumb").is_some(),
            "{name}: anchor was dropped"
        );
        assert_eq!(
            pose_of(&edited, &member("thumb", 0, 0)).at,
            Vec2 { x: 38.0, y: -19.0 },
            "{name}"
        );
        assert!(
            edited
                .parts
                .iter()
                .all(|part| part.id != member("main", 0, 2)),
            "{name}: invisible key was materialized"
        );
    }
}

#[test]
fn column_align_moves_through_the_actual_anchor_frame() {
    let (mut engine, doc) = anchored_fixture();
    let (_, moved) = commit_edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixPlacement {
            matrix_id: "thumb".into(),
            change: MatrixPlacementChange::ColumnTranslation {
                column: 1,
                delta: Vec2 { x: 0.0, y: 5.0 },
            },
        },
    );
    assert_at(&moved, &member("thumb", 0, 1), 57.0, -14.0);
    assert_at(&moved, &member("thumb", 0, 0), 38.0, -19.0);
    assert_at(&moved, &member("thumb", 0, 2), 76.0, -19.0);
}

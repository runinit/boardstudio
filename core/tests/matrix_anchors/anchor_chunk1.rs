use super::*;

#[test]
fn reanchoring_a_matrix_moves_it_and_keeps_its_stored_frame() {
    let (mut engine, doc) = anchored_fixture();
    let origin_before = matrix(&doc, "thumb").origin;
    let anchor_before = anchor_of(&doc, "thumb");
    let key_before = at(&doc, &member("thumb", 0, 0));
    let moved = key_anchor("main", 0, 0);
    let (_, committed) = edit(&mut engine, &doc, reanchor("thumb", moved.clone()));
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
    let (_, committed) = edit(&mut engine, &doc, operation);
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
        let (_, moved) = edit(
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
    let (_, compacted) = edit(
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
        let (_, edited) = edit(&mut engine, &doc, operation);
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
    let (_, moved) = edit(
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

#[test]
fn linked_dependent_follows_actual_local_anchor() {
    // The dependent keeps its own linked parameters but follows the actual
    // Local anchor key pose, not a substitute invisible pose.
    let (mut engine, doc) = linked_anchored_fixture();
    // Make the mirror anchor key Local via its Layout link.
    let doc = with_mirror_exception(
        &mut engine,
        &doc,
        MirroredItemKind::Part,
        member("main2", 0, 2),
        MirroredExceptionState::Local,
    );
    assert!(
        doc.layouts
            .iter()
            .find(|layout| layout.id == "mirror-layout")
            .and_then(|layout| layout.mirror_link.as_ref())
            .is_some_and(|link| link
                .exceptions
                .iter()
                .any(|exception| exception.mirror_id == member("main2", 0, 2)
                    && exception.state == MirroredExceptionState::Local))
    );
    // Move the actual Local anchor key independently from the mirror side.
    let (_, moved) = edit(
        &mut engine,
        &doc,
        move_part(member("main2", 0, 2), Vec2 { x: -10.0, y: 5.0 }),
    );
    // Source stays where it was; the mirror anchor moved locally.
    assert_at(&moved, &member("main", 0, 2), 38.0, 0.0);
    assert_at(&moved, &member("main2", 0, 2), -10.0, 5.0);
    // The mirror dependent follows its actual Local anchor; its own layout
    // values stay linked (empty, like the source thumb).
    assert_at(&moved, &member("thumb2", 0, 0), -10.0, -14.0);
    let thumb = moved
        .matrices
        .iter()
        .find(|matrix| matrix.id == "thumb")
        .unwrap();
    let thumb2 = moved
        .matrices
        .iter()
        .find(|matrix| matrix.id == "thumb2")
        .unwrap();
    assert_eq!(thumb.row_layout, thumb2.row_layout);
    assert_eq!(thumb.column_layout, thumb2.column_layout);
}

#[test]
fn linked_column_zero_hold_still_reflects_compensation_once() {
    // Column-0 hold-still writes matrix Origin; the follower gets the
    // reflected Origin once, and both halves hold their keys still.
    let (mut engine, doc) = linked_anchored_fixture();
    // Give column 0 a splay so the origin edit has something to hold still.
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixSplay {
            matrix_id: "main".into(),
            column: 1,
            change: MatrixSplayChange::Angle {
                angle: 15.0,
                affect: MatrixSplayAffect::Following,
            },
        },
    );
    let before: Vec<(String, Pose2)> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    // Hold-still origin edit from the mirror half.
    let (_, moved) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixSplay {
            matrix_id: "main2".into(),
            column: 0,
            change: MatrixSplayChange::OriginRelative {
                value: Vec2 { x: 2.0, y: 1.0 },
            },
        },
    );
    // Stored relative origins copy unchanged; Origins are point reflections.
    let source_origin = moved
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main")
        .unwrap()
        .column_layout
        .first()
        .map(|record| record.splay_origin)
        .unwrap_or_default();
    let mirror_origin = moved
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main2")
        .unwrap()
        .column_layout
        .first()
        .map(|record| record.splay_origin)
        .unwrap_or_default();
    assert_eq!(source_origin, Vec2 { x: 2.0, y: 1.0 });
    assert_eq!(mirror_origin, Vec2 { x: 2.0, y: 1.0 });
    // Keys held still: every pre-existing pose is unchanged within tolerance.
    for (id, pose) in &before {
        let after = pose_of(&moved, id);
        assert!(
            (after.at.x - pose.at.x).abs() < 1e-9 && (after.at.y - pose.at.y).abs() < 1e-9,
            "{id} moved: {pose:?} vs {after:?}"
        );
    }
}

#[test]
fn linked_resize_keeps_omitted_partner_live_and_absent() {
    // Ticket 21: source growth with an Omitted partner succeeds; the partner
    // stays absent (no parts fabricated) while its record follows the source
    // so anchors through it keep a live logical pose.
    let (mut engine, doc) = linked_anchored_fixture();
    let mut mirror_layout = layout(&doc, "mirror-layout").clone();
    mirror_layout
        .mirror_link
        .as_mut()
        .expect("mirror link")
        .exceptions
        .push(MirroredException {
            kind: MirroredItemKind::Matrix,
            mirror_id: "main2".into(),
            state: MirroredExceptionState::Omitted,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    // The Omit transition itself suppresses the inherited parts.
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: mirror_layout,
        },
    );
    assert!(
        doc.parts
            .iter()
            .all(|part| !part.id.starts_with("matrix/main2/")),
        "omit must suppress inherited parts"
    );
    assert!(
        doc.matrices.iter().any(|matrix| matrix.id == "main2"),
        "omit keeps the live record"
    );
    // Grow the source through the public prepared route.
    let (_, resized) = edit(
        &mut engine,
        &doc,
        change_matrix(&doc, "main", |matrix| matrix.columns = 4),
    );
    // The source grew; the Omitted record followed without materializing.
    assert_eq!(matrix(&resized, "main").columns, 4);
    let partner = matrix(&resized, "main2");
    assert_eq!((partner.rows, partner.columns), (1, 4));
    assert!(
        resized
            .parts
            .iter()
            .all(|part| !part.id.starts_with("matrix/main2/")),
        "no mirrored newcomer fabricated for the Omitted partner"
    );
    // Anchors through the Omitted record still evaluate at the live pose.
    assert_at(&resized, &member("thumb2", 0, 0), -18.0, -19.0);
    // One transaction: Undo restores the pre-resize document.
    let undone_doc = undone(&mut engine);
    assert_eq!(undone_doc.matrices, doc.matrices);
    assert_eq!(undone_doc.layouts, doc.layouts);
    assert_eq!(undone_doc.parts, doc.parts);
    let redone_doc = redone(&mut engine);
    assert_eq!(redone_doc.matrices, resized.matrices);
    assert_eq!(redone_doc.layouts, resized.layouts);
    assert_eq!(redone_doc.parts, resized.parts);
}

#[test]
fn linked_make_local_on_anchored_matrix_detaches_and_keeps_pose() {
    // Ticket 21: making an anchored matrix Local detaches its own anchor and
    // keeps its world pose; its future layout stays its own.
    let (mut engine, doc) = linked_anchored_fixture();
    let local = with_mirror_exception(
        &mut engine,
        &doc,
        MirroredItemKind::Matrix,
        "thumb2",
        MirroredExceptionState::Local,
    );
    // Its own anchor cleared, every pose kept.
    assert_eq!(anchor_of(&local, "thumb2"), None);
    assert!(anchor_of(&local, "thumb").is_some());
    for (id, pose) in poses(&doc) {
        assert_eq!(pose_of(&local, id), pose, "{id}");
    }
    // One Undo restores the anchor and the exception state together.
    let undone_doc = undone(&mut engine);
    assert_eq!(anchor_of(&undone_doc, "thumb2"), anchor_of(&doc, "thumb2"));
    assert_eq!(undone_doc.layouts, doc.layouts);
    let redone_doc = redone(&mut engine);
    assert_eq!(anchor_of(&redone_doc, "thumb2"), None);
    assert_eq!(redone_doc.layouts, local.layouts);
}

#[test]
fn linked_rejoin_adopts_current_source_and_keeps_child_exceptions() {
    // Ticket 21: Rejoin adopts the current source dimensions/layout/anchor
    // and preserves explicit child exceptions.
    let (mut engine, doc) = linked_anchored_fixture();
    // Child exception first: the mirror key goes Local and moves.
    let doc = with_mirror_exception(
        &mut engine,
        &doc,
        MirroredItemKind::Part,
        member("main2", 0, 0),
        MirroredExceptionState::Local,
    );
    let (_, doc) = edit(
        &mut engine,
        &doc,
        move_part(member("main2", 0, 0), Vec2 { x: 30.0, y: 5.0 }),
    );
    // Omit the whole matrix, grow the source while absent, then Rejoin.
    let mut omitted_layout = layout(&doc, "mirror-layout").clone();
    omitted_layout
        .mirror_link
        .as_mut()
        .expect("mirror link")
        .exceptions
        .push(MirroredException {
            kind: MirroredItemKind::Matrix,
            mirror_id: "main2".into(),
            state: MirroredExceptionState::Omitted,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: omitted_layout,
        },
    );
    // Inherited parts shed; the explicit Local child keeps its own state.
    assert!(
        doc.parts
            .iter()
            .filter(|part| part.id.starts_with("matrix/main2/"))
            .map(|part| part.id.clone())
            .collect::<Vec<_>>()
            == vec![member("main2", 0, 0)],
        "omit sheds inherited parts but keeps explicit child exceptions"
    );
    assert_at(&doc, &member("main2", 0, 0), 30.0, 5.0);
    let (_, doc) = edit(
        &mut engine,
        &doc,
        change_matrix(&doc, "main", |matrix| matrix.columns = 4),
    );
    // Rejoin: remove the matrix overlay, keep the child.
    let mut rejoined_layout = layout(&doc, "mirror-layout").clone();
    if let Some(link) = rejoined_layout.mirror_link.as_mut() {
        link.exceptions.retain(|exception| {
            !(exception.kind == MirroredItemKind::Matrix && exception.mirror_id == "main2")
        });
    }
    let (_, rejoined) = edit(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: rejoined_layout,
        },
    );
    // Current source dimensions adopted with materialized parts.
    let partner = matrix(&rejoined, "main2");
    assert_eq!((partner.rows, partner.columns), (1, 4));
    for column in 0..4 {
        assert!(
            rejoined
                .parts
                .iter()
                .any(|part| part.id == member("main2", 0, column)),
            "rejoined part (0,{column}) materialized"
        );
    }
    // Explicit child exception preserved with its Local pose.
    assert!(
        rejoined
            .layouts
            .iter()
            .find(|layout| layout.id == "mirror-layout")
            .and_then(|layout| layout.mirror_link.as_ref())
            .is_some_and(|link| link.exceptions.iter().any(|exception| {
                exception.kind == MirroredItemKind::Part
                    && exception.mirror_id == member("main2", 0, 0)
                    && exception.state == MirroredExceptionState::Local
            }))
    );
    assert_at(&rejoined, &member("main2", 0, 0), 30.0, 5.0);
    // Shared members track the source again.
    assert_at(&rejoined, &member("main2", 0, 1), 1.0, 0.0);
}

#[test]
fn linked_detach_needing_local_exception_refuses_with_proposal() {
    // Ticket 21: a shared detach that would move the partner refuses naming
    // the required Local exception; cancel (no commit) changes nothing.
    let (mut engine, doc) = linked_anchored_fixture();
    // The mirror anchor key goes Local and moves: the dependent follows its
    // actual Local anchor, so a plain detach would jump the partner.
    let doc = with_mirror_exception(
        &mut engine,
        &doc,
        MirroredItemKind::Part,
        member("main2", 0, 2),
        MirroredExceptionState::Local,
    );
    let (_, doc) = edit(
        &mut engine,
        &doc,
        move_part(member("main2", 0, 2), Vec2 { x: -10.0, y: 5.0 }),
    );
    assert_at(&doc, &member("thumb2", 0, 0), -10.0, -14.0);
    // Plain detach refuses naming the needed Local exception.
    let message = error(
        &mut engine,
        &doc,
        EditOperation::DetachMatrixAnchor {
            matrix_id: "thumb".into(),
        },
    );
    assert!(
        message.contains("thumb2") && message.contains("Local"),
        "names the required exception: {message:?}"
    );
    // Cancel changes nothing: the document is byte-identical.
    assert_eq!(snapshot(&mut engine), doc);
    assert!(anchor_of(&doc, "thumb").is_some());
    assert!(anchor_of(&doc, "thumb2").is_some());
}

use super::*;

#[test]
fn change_anchor_retargets_dependants_with_preview_cancel_and_undo() {
    let (mut engine, doc) = anchored_fixture();
    let moved = key_anchor("main", 0, 0);
    let operation = reanchor("thumb", moved.clone());
    let before = at(&doc, &member("thumb", 0, 0));
    assert_eq!(before, Vec2 { x: 38.0, y: -19.0 });
    let previewed = preview(&mut engine, &doc, operation.clone());
    let previewed_pose = previewed
        .matrix_scenes
        .iter()
        .find(|scene| scene.matrix_id == "thumb")
        .expect("thumb scene")
        .cells
        .iter()
        .find(|cell| cell.row == 0 && cell.column == 0)
        .expect("thumb cell")
        .pose
        .at;
    assert_eq!(previewed_pose, Vec2 { x: 0.0, y: -19.0 });
    // Preview leaves accepted state, revision and history unchanged: the whole
    // document (not just the matrix list) is byte-identical.
    assert_eq!(snapshot(&mut engine), doc);
    let (_, committed) = edit(&mut engine, &doc, operation);
    assert_eq!(anchor_of(&committed, "thumb"), Some(moved));
    assert_at(&committed, &member("thumb", 0, 0), 0.0, -19.0);
    let undone_doc = undone(&mut engine);
    assert_eq!(anchor_of(&undone_doc, "thumb"), anchor_of(&doc, "thumb"));
    assert_eq!(redone(&mut engine).matrices, committed.matrices);
}

#[test]
fn reordered_matrix_lists_evaluate_to_the_same_poses() {
    let (mut engine, doc) = fixture();
    let doc = add_thumb(&mut engine, &doc);
    let forward: Vec<(String, Vec2)> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose.at))
        .collect();
    let mut reordered = doc.clone();
    reordered.matrices.reverse();
    let (_, _, opened) = open(reordered);
    let backward: Vec<(String, Vec2)> = opened
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose.at))
        .collect();
    assert_eq!(forward, backward);
}

#[test]
fn detach_keeps_world_frame_and_clears_the_anchor() {
    let (mut engine, doc) = anchored_fixture();
    // Turn the whole group so the kept frame carries a nonzero rotation.
    let (_, doc) = edit(
        &mut engine,
        &doc,
        change_matrix(&doc, "main", |matrix| matrix.rotation = Some(30.0)),
    );
    let (_, detached) = edit(
        &mut engine,
        &doc,
        EditOperation::DetachMatrixAnchor {
            matrix_id: "thumb".into(),
        },
    );
    assert_eq!(anchor_of(&detached, "thumb"), None);
    for (id, pose) in poses(&doc) {
        assert_eq!(pose_of(&detached, id), pose, "{id}");
    }
    let thumb = matrix(&detached, "thumb");
    // The stored frame is the actual evaluated frame, not world zero.
    let cos30 = 30f64.to_radians().cos();
    let sin30 = 30f64.to_radians().sin();
    near(
        thumb.origin,
        38.0 * cos30 + 19.0 * sin30,
        38.0 * sin30 - 19.0 * cos30,
    );
    assert_eq!(thumb.rotation, Some(30.0));
    // Moving the old anchor key no longer moves the detached members.
    let (_, moved) = edit(
        &mut engine,
        &detached,
        move_part(member("main", 0, 2), Vec2 { x: 100.0, y: 100.0 }),
    );
    for (id, pose) in poses(&doc) {
        if id.starts_with("matrix/thumb/") {
            assert_eq!(pose_of(&moved, id), pose, "{id}");
        }
    }
    near(pose_of(&moved, &member("main", 0, 2)).at, 100.0, 100.0);
    let undone_move = undone(&mut engine);
    assert_eq!(anchor_of(&undone_move, "thumb"), None);
    let undone_detach = undone(&mut engine);
    assert_eq!(anchor_of(&undone_detach, "thumb"), anchor_of(&doc, "thumb"));
    assert_eq!(redone(&mut engine).matrices, detached.matrices);
    assert_eq!(redone(&mut engine).matrices, moved.matrices);
}

#[test]
fn anchor_follow_applies_origin_and_rotation_before_filtering() {
    let (mut engine, doc) = anchored_fixture();
    // Widen the relative origin so the order is observable.
    let (_, doc) = edit(
        &mut engine,
        &doc,
        change_matrix(&doc, "thumb", |matrix| {
            matrix.origin = Vec2 { x: 10.0, y: -19.0 }
        }),
    );
    assert_at(&doc, &member("thumb", 0, 0), 48.0, -19.0);
    // Dropping Follow X zeroes the whole candidate X, including the relative
    // origin: Origin runs before Follow, not after it.
    let doc = set_follow(&mut engine, &doc, false, true, true);
    assert_at(&doc, &member("thumb", 0, 0), 0.0, -19.0);
    assert_at(&doc, &member("thumb", 0, 2), 38.0, -19.0);
}

#[test]
fn removing_an_anchor_key_detaches_dependants_without_moving_them() {
    #[derive(Debug)]
    enum Removal {
        Truncate,
        Column,
        Matrix,
    }

    for case in [Removal::Truncate, Removal::Column, Removal::Matrix] {
        let (mut engine, doc) = anchored_fixture();
        let operation = match case {
            Removal::Truncate => change_matrix(&doc, "main", |matrix| matrix.columns = 2),
            Removal::Column => EditOperation::RemoveMatrixColumn {
                matrix_id: "main".into(),
                column: 2,
            },
            Removal::Matrix => EditOperation::RemoveMatrix { id: "main".into() },
        };
        let (_, detached) = edit(&mut engine, &doc, operation);
        assert_eq!(anchor_of(&detached, "thumb"), None, "{case:?}");
        assert_eq!(
            matrix(&detached, "thumb").origin,
            Vec2 { x: 38.0, y: -19.0 },
            "{case:?}"
        );
        if matches!(case, Removal::Matrix) {
            assert_eq!(
                layout(&detached, "layout").matrix_ids,
                ["thumb"],
                "{case:?}"
            );
        }
        for (id, pose) in poses(&doc).filter(|(id, _)| id.starts_with("matrix/thumb/")) {
            assert_eq!(pose_of(&detached, id), pose, "{case:?}: {id}");
        }
    }
}

#[test]
fn linked_follow_transports_source_basis_with_non_origin_axis() {
    // Axis x=10 is not the origin: Follow X off keeps source X=0 while the
    // partner keeps the transported X=20, not an independent zero.
    let (mut engine, doc) = linked_anchored_fixture();
    // Drop Follow X on the source thumb; sync carries the subset to the mirror.
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixAnchor {
            matrix_id: "thumb".into(),
            anchor: MatrixAnchor {
                matrix_id: "main".into(),
                row: 0,
                column: 2,
                follow: Follow {
                    x: false,
                    y: true,
                    rotation: true,
                },
                aggregate: None,
            },
        },
    );
    assert!(
        !doc.matrices
            .iter()
            .find(|matrix| matrix.id == "thumb2")
            .and_then(|matrix| matrix.anchor.clone())
            .expect("thumb2 anchor")
            .follow
            .x
    );
    let (_, moved) = edit(
        &mut engine,
        &doc,
        move_part(member("main", 0, 2), Vec2 { x: 43.0, y: 7.0 }),
    );
    // Source keeps world-zero X, partner keeps the transported axis image.
    assert_at(&moved, &member("thumb", 0, 0), 0.0, -12.0);
    assert_at(&moved, &member("thumb2", 0, 0), 20.0, -12.0);
    // Preview leaves the document unchanged and Undo restores one entry.
    let before = snapshot(&mut engine);
    let _ = preview(
        &mut engine,
        &moved,
        move_part(member("main", 0, 2), Vec2 { x: 44.0, y: 8.0 }),
    );
    assert_eq!(snapshot(&mut engine), before);
}

#[test]
fn linked_anchor_retarget_maps_by_identity_not_list_position() {
    let (_, doc) = linked_anchored_fixture();
    // Reorder both the stored matrices and the Layout membership lists; the
    // group must still resolve by identity and physical key.
    let mut reordered = doc.clone();
    reordered.matrices.reverse();
    for layout in &mut reordered.layouts {
        layout.matrix_ids.reverse();
    }
    let (mut engine, _, doc) = open(reordered);
    let (_, doc) = edit(
        &mut engine,
        &doc,
        reanchor("thumb", key_anchor("main", 0, 0)),
    );
    // Mirror follows via the matrix correspondence, not the stale anchor.
    assert_eq!(
        anchor_of(&doc, "thumb2").expect("thumb2 anchor").matrix_id,
        "main2"
    );
    assert_eq!(anchor_of(&doc, "thumb2").expect("thumb2 anchor").row, 0);
    assert_eq!(anchor_of(&doc, "thumb2").expect("thumb2 anchor").column, 0);
    // Poses track the new anchor keys on both halves.
    assert_at(&doc, &member("thumb", 0, 0), 0.0, -19.0);
    // main2(0,0) mirrors main(0,0) about x=10: 2*10-0=20.
    assert_at(&doc, &member("thumb2", 0, 0), 20.0, -19.0);
}

#[test]
fn linked_y_mirror_change_is_refused() {
    // Ticket 20 preserved refusal: a horizontal local frame needs an
    // independent layout. Proves the `layout.rs::reflected` guard via public
    // Edit; GREEN on current code is characterization (RED without the guard
    // at layout.rs:381-382 is by code-path inspection only: `reflected`
    // would otherwise accept Y and silently map the partner to X).
    let (mut engine, doc) = linked_anchored_fixture();
    for matrix_id in ["thumb", "thumb2"] {
        let mut attempted = doc
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .expect("linked matrix")
            .clone();
        attempted.mirror = Some(Mirror::Y);
        let message = error(
            &mut engine,
            &doc,
            EditOperation::SetMatrix {
                matrix: attempted,
                definitions: None,
            },
        );
        assert!(
            message.contains("Unlink the halves before changing to a horizontal matrix mirror"),
            "{matrix_id}: unexpected message: {message:?}"
        );
        assert_eq!(
            snapshot(&mut engine),
            doc,
            "{matrix_id}: refusal changed state"
        );
    }
    // Narrow guard: the ordinary None/X toggle still succeeds on the linked
    // source (claw-mirror toggle), so the refusal is Y-specific.
    let (_, changed) = edit(
        &mut engine,
        &doc,
        change_matrix(&doc, "thumb", |matrix| matrix.mirror = Some(Mirror::X)),
    );
    assert_eq!(
        changed
            .matrices
            .iter()
            .find(|matrix| matrix.id == "thumb")
            .and_then(|matrix| matrix.mirror),
        Some(Mirror::X)
    );
}

#[test]
fn linked_resize_keeps_local_partner_dimensions_and_poses() {
    // Ticket 21 first slice: a public source resize must not rewrite a Local
    // partner matrix. Source `main` grows 3 -> 4 columns; Local `main2` keeps
    // its own 1x3 dimensions and every world pose, with valid correspondences.
    let (mut engine, doc) = linked_anchored_fixture();
    // Make the mirror matrix Local via its Layout link (public route).
    let doc = with_mirror_exception(
        &mut engine,
        &doc,
        MirroredItemKind::Matrix,
        "main2",
        MirroredExceptionState::Local,
    );
    // Grow the source through the public prepared route.
    let (_, resized) = edit(
        &mut engine,
        &doc,
        change_matrix(&doc, "main", |matrix| matrix.columns = 4),
    );
    // The source grew; the Local partner kept its own dimensions.
    assert_eq!(matrix(&resized, "main").columns, 4);
    let partner = matrix(&resized, "main2");
    assert_eq!(partner.rows, 1);
    assert_eq!(partner.columns, 3);
    // Every pre-existing pose is unchanged, including the Local partner's.
    for (id, pose) in poses(&doc) {
        assert_eq!(pose_of(&resized, id), pose, "{id}");
    }
    // No mirrored newcomer was fabricated for the Local partner.
    assert!(
        resized
            .parts
            .iter()
            .all(|part| part.id != member("main2", 0, 3))
    );
    // One transaction: Undo restores the pre-resize matrices, Redo restores
    // the resize (revision bumps, so compare content, not whole docs).
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
fn linked_local_to_omitted_key_returns_to_shared_pose() {
    // Ticket 21: Local->Omitted replaces Local state instead of freezing the
    // former Local placement: the key's logical pose returns to the current
    // shared layout while explicit child exceptions are preserved.
    let (mut engine, doc) = linked_anchored_fixture();
    // Make the mirror anchor key Local, then move it independently.
    let doc = with_mirror_exception(
        &mut engine,
        &doc,
        MirroredItemKind::Part,
        member("main2", 0, 2),
        MirroredExceptionState::Local,
    );
    let (_, moved) = edit(
        &mut engine,
        &doc,
        move_part(member("main2", 0, 2), Vec2 { x: -10.0, y: 5.0 }),
    );
    assert_at(&moved, &member("main2", 0, 2), -10.0, 5.0);
    // Switch the same key from Local to Omitted via the public route.
    let mut omitted_layout = layout(&moved, "mirror-layout").clone();
    if let Some(link) = omitted_layout.mirror_link.as_mut() {
        for exception in &mut link.exceptions {
            if exception.mirror_id == member("main2", 0, 2) {
                exception.state = MirroredExceptionState::Omitted;
            }
        }
    }
    let (_, omitted) = edit(
        &mut engine,
        &moved,
        EditOperation::SetLayout {
            layout: omitted_layout,
        },
    );
    // The Omitted key's part is suppressed, not frozen at its Local pose.
    assert!(
        omitted
            .parts
            .iter()
            .all(|part| part.id != member("main2", 0, 2)),
        "omit must suppress the key part, not freeze its Local placement"
    );
    // The dependent still follows the live shared anchor: the x=10
    // reflection of main(0,2) at (38,0), not the former Local (-10,5).
    assert_at(&omitted, &member("thumb2", 0, 0), -18.0, -19.0);
    // Rejoin restores the part at the shared pose in the same transaction.
    let mut rejoined_layout = layout(&omitted, "mirror-layout").clone();
    if let Some(link) = rejoined_layout.mirror_link.as_mut() {
        link.exceptions
            .retain(|exception| exception.mirror_id != member("main2", 0, 2));
    }
    let (_, rejoined) = edit(
        &mut engine,
        &omitted,
        EditOperation::SetLayout {
            layout: rejoined_layout,
        },
    );
    assert_at(&rejoined, &member("main2", 0, 2), -18.0, 0.0);
    assert_at(&rejoined, &member("thumb2", 0, 0), -18.0, -19.0);
    // Rejoin is one Undo entry: Undo returns to the Omitted state.
    let undone_doc = undone(&mut engine);
    assert_eq!(undone_doc.layouts, omitted.layouts);
    assert_eq!(undone_doc.parts, omitted.parts);
    let redone_doc = redone(&mut engine);
    assert_eq!(redone_doc.parts, rejoined.parts);
}

#[test]
fn omitted_storage_edits_are_refused_with_rejoin_guidance() {
    // Ticket 21: an absent matrix has no independent storage state; direct
    // edits refuse naming Rejoin and change nothing.
    let (mut engine, doc) = linked_anchored_fixture();
    let doc = with_mirror_exception(
        &mut engine,
        &doc,
        MirroredItemKind::Matrix,
        "main2",
        MirroredExceptionState::Omitted,
    );
    let message = error(
        &mut engine,
        &doc,
        change_matrix(&doc, "main2", |matrix| matrix.columns = 5),
    );
    assert!(
        message.contains("Rejoin"),
        "unexpected message: {message:?}"
    );
    assert_eq!(snapshot(&mut engine), doc, "refusal changed state");
}

#[test]
fn linked_rejoin_refuses_missing_anchor_atomically() {
    // Ticket 21: Rejoin validates the resulting graph atomically. Shrinking
    // the Local mirror parent retires the Omitted dependent's anchor; a
    // later Rejoin adopting the current source anchor refuses naming the
    // missing counterpart without touching the Local parent or other state.
    let (mut engine, doc) = linked_anchored_fixture();
    // Omit the mirror dependent first.
    let mut mirror_layout = layout(&doc, "mirror-layout").clone();
    mirror_layout
        .mirror_link
        .as_mut()
        .expect("mirror link")
        .exceptions
        .push(MirroredException {
            kind: MirroredItemKind::Matrix,
            mirror_id: "thumb2".into(),
            state: MirroredExceptionState::Omitted,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    // Shrink the Local mirror parent; the Omitted dependent detaches.
    mirror_layout
        .mirror_link
        .as_mut()
        .expect("mirror link")
        .exceptions
        .push(MirroredException {
            kind: MirroredItemKind::Matrix,
            mirror_id: "main2".into(),
            state: MirroredExceptionState::Local,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: mirror_layout,
        },
    );
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::RemoveMatrixColumn {
            matrix_id: "main2".into(),
            column: 2,
        },
    );
    assert_eq!(
        doc.matrices
            .iter()
            .find(|matrix| matrix.id == "main2")
            .expect("local parent")
            .columns,
        2
    );
    assert_eq!(anchor_of(&doc, "thumb2"), None);
    // Rejoin adopts the source anchor (main (0,2)) whose counterpart in the
    // smaller Local parent is missing: refuse atomically.
    let mut rejoined_layout = layout(&doc, "mirror-layout").clone();
    if let Some(link) = rejoined_layout.mirror_link.as_mut() {
        link.exceptions.retain(|exception| {
            !(exception.kind == MirroredItemKind::Matrix && exception.mirror_id == "thumb2")
        });
    }
    let message = error(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: rejoined_layout,
        },
    );
    assert!(
        message.contains("matrix/main2/r0c2"),
        "names the missing counterpart: {message:?}"
    );
    // Nothing changed: the Local parent keeps its size and state, and the
    // rest of the document is byte-identical.
    assert_eq!(snapshot(&mut engine), doc);
    assert_eq!(
        doc.matrices
            .iter()
            .find(|matrix| matrix.id == "main2")
            .expect("local parent")
            .columns,
        2
    );
}

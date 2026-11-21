use super::*;

#[test]
fn change_anchor_retargets_dependants_with_preview_cancel_and_undo() {
    let (mut engine, doc) = anchored_fixture();
    let moved = MatrixAnchor {
        matrix_id: "main".into(),
        row: 0,
        column: 0,
        follow: Follow {
            x: true,
            y: true,
            rotation: true,
        },
        aggregate: None,
    };
    let operation = EditOperation::SetMatrixAnchor {
        matrix_id: "thumb".into(),
        anchor: moved.clone(),
    };
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
    assert_eq!(
        at(&committed, &member("thumb", 0, 0)),
        Vec2 { x: 0.0, y: -19.0 }
    );
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
    let mut turned = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main")
        .expect("main matrix")
        .clone();
    turned.rotation = Some(30.0);
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: turned,
            definitions: None,
        },
    );
    let poses_before: Vec<(String, Pose2)> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let (_, detached) = edit(
        &mut engine,
        &doc,
        EditOperation::DetachMatrixAnchor {
            matrix_id: "thumb".into(),
        },
    );
    assert_eq!(anchor_of(&detached, "thumb"), None);
    for (id, pose) in &poses_before {
        assert_eq!(pose_of(&detached, id), *pose, "{id}");
    }
    let thumb = detached
        .matrices
        .iter()
        .find(|matrix| matrix.id == "thumb")
        .expect("thumb matrix");
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
        EditOperation::MoveParts {
            positions: vec![Position {
                id: member("main", 0, 2),
                at: Vec2 { x: 100.0, y: 100.0 },
            }],
        },
    );
    for (id, pose) in &poses_before {
        if id.starts_with("matrix/thumb/") {
            assert_eq!(pose_of(&moved, id), *pose, "{id}");
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
    let mut widened = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "thumb")
        .expect("thumb matrix")
        .clone();
    widened.origin = Vec2 { x: 10.0, y: -19.0 };
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: widened,
            definitions: None,
        },
    );
    assert_eq!(
        pose_of(&doc, &member("thumb", 0, 0)).at,
        Vec2 { x: 48.0, y: -19.0 }
    );
    // Dropping Follow X zeroes the whole candidate X, including the relative
    // origin: Origin runs before Follow, not after it.
    let doc = set_follow(&mut engine, &doc, false, true, true);
    assert_eq!(
        pose_of(&doc, &member("thumb", 0, 0)).at,
        Vec2 { x: 0.0, y: -19.0 }
    );
    assert_eq!(
        pose_of(&doc, &member("thumb", 0, 2)).at,
        Vec2 { x: 38.0, y: -19.0 }
    );
}

#[test]
fn removing_the_anchor_slot_detaches_dependants_with_their_frame() {
    let (mut engine, doc) = anchored_fixture();
    let poses_before: Vec<(String, Pose2)> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let (_, detached) = edit(
        &mut engine,
        &doc,
        EditOperation::RemoveMatrixColumn {
            matrix_id: "main".into(),
            column: 2,
        },
    );
    assert_eq!(anchor_of(&detached, "thumb"), None);
    let thumb = detached
        .matrices
        .iter()
        .find(|matrix| matrix.id == "thumb")
        .expect("thumb matrix");
    assert_eq!(thumb.origin, Vec2 { x: 38.0, y: -19.0 });
    for (id, pose) in &poses_before {
        if id.starts_with("matrix/thumb/") {
            assert_eq!(pose_of(&detached, id), *pose, "{id}");
        }
    }
}

#[test]
fn removing_the_anchor_matrix_detaches_dependants() {
    let (mut engine, doc) = anchored_fixture();
    let poses_before: Vec<(String, Pose2)> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let (_, detached) = edit(
        &mut engine,
        &doc,
        EditOperation::RemoveMatrix { id: "main".into() },
    );
    assert_eq!(anchor_of(&detached, "thumb"), None);
    assert!(
        detached
            .layouts
            .iter()
            .find(|layout| layout.id == "layout")
            .is_some_and(|layout| layout.matrix_ids == vec!["thumb".to_string()])
    );
    for (id, pose) in &poses_before {
        if id.starts_with("matrix/thumb/") {
            assert_eq!(pose_of(&detached, id), *pose, "{id}");
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
        EditOperation::MoveParts {
            positions: vec![Position {
                id: member("main", 0, 2),
                at: Vec2 { x: 43.0, y: 7.0 },
            }],
        },
    );
    // Source keeps world-zero X, partner keeps the transported axis image.
    assert_eq!(
        pose_of(&moved, &member("thumb", 0, 0)).at,
        Vec2 { x: 0.0, y: -12.0 }
    );
    assert_eq!(
        pose_of(&moved, &member("thumb2", 0, 0)).at,
        Vec2 { x: 20.0, y: -12.0 }
    );
    // Preview leaves the document unchanged and Undo restores one entry.
    let before = snapshot(&mut engine);
    let _ = preview(
        &mut engine,
        &moved,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: member("main", 0, 2),
                at: Vec2 { x: 44.0, y: 8.0 },
            }],
        },
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
        EditOperation::SetMatrixAnchor {
            matrix_id: "thumb".into(),
            anchor: MatrixAnchor {
                matrix_id: "main".into(),
                row: 0,
                column: 0,
                follow: Follow {
                    x: true,
                    y: true,
                    rotation: true,
                },
                aggregate: None,
            },
        },
    );
    // Mirror follows via the matrix correspondence, not the stale anchor.
    assert_eq!(
        anchor_of(&doc, "thumb2").expect("thumb2 anchor").matrix_id,
        "main2"
    );
    assert_eq!(anchor_of(&doc, "thumb2").expect("thumb2 anchor").row, 0);
    assert_eq!(anchor_of(&doc, "thumb2").expect("thumb2 anchor").column, 0);
    // Poses track the new anchor keys on both halves.
    assert_eq!(
        pose_of(&doc, &member("thumb", 0, 0)).at,
        Vec2 { x: 0.0, y: -19.0 }
    );
    // main2(0,0) mirrors main(0,0) about x=10: 2*10-0=20.
    assert_eq!(
        pose_of(&doc, &member("thumb2", 0, 0)).at,
        Vec2 { x: 20.0, y: -19.0 }
    );
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
    let mut toggled = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "thumb")
        .expect("thumb matrix")
        .clone();
    toggled.mirror = Some(Mirror::X);
    let (_, changed) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: toggled,
            definitions: None,
        },
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
    let mut mirror_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
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
    let poses_before: Vec<(String, Pose2)> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    // Grow the source through the public prepared route.
    let mut grown = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main")
        .expect("main matrix")
        .clone();
    grown.columns = 4;
    let (_, resized) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: grown,
            definitions: None,
        },
    );
    // The source grew; the Local partner kept its own dimensions.
    assert_eq!(
        resized
            .matrices
            .iter()
            .find(|matrix| matrix.id == "main")
            .expect("grown source")
            .columns,
        4
    );
    let partner = resized
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main2")
        .expect("local partner");
    assert_eq!(partner.rows, 1);
    assert_eq!(partner.columns, 3);
    // Every pre-existing pose is unchanged, including the Local partner's.
    for (id, pose) in &poses_before {
        assert_eq!(pose_of(&resized, id), *pose, "{id}");
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
    let mut mirror_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
    mirror_layout
        .mirror_link
        .as_mut()
        .expect("mirror link")
        .exceptions
        .push(MirroredException {
            kind: MirroredItemKind::Part,
            mirror_id: member("main2", 0, 2),
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
    let (_, moved) = edit(
        &mut engine,
        &doc,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: member("main2", 0, 2),
                at: Vec2 { x: -10.0, y: 5.0 },
            }],
        },
    );
    assert_eq!(
        pose_of(&moved, &member("main2", 0, 2)).at,
        Vec2 { x: -10.0, y: 5.0 }
    );
    // Switch the same key from Local to Omitted via the public route.
    let mut omitted_layout = moved
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
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
    assert_eq!(
        pose_of(&omitted, &member("thumb2", 0, 0)).at,
        Vec2 { x: -18.0, y: -19.0 }
    );
    // Rejoin restores the part at the shared pose in the same transaction.
    let mut rejoined_layout = omitted
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
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
    assert_eq!(
        pose_of(&rejoined, &member("main2", 0, 2)).at,
        Vec2 { x: -18.0, y: 0.0 }
    );
    assert_eq!(
        pose_of(&rejoined, &member("thumb2", 0, 0)).at,
        Vec2 { x: -18.0, y: -19.0 }
    );
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
    let mut mirror_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
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
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: mirror_layout,
        },
    );
    let mut attempted = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main2")
        .expect("omitted record")
        .clone();
    attempted.columns = 5;
    let message = error(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: attempted,
            definitions: None,
        },
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
    let mut mirror_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
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
    let mut rejoined_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
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

#[test]
fn linked_source_removal_retains_local_matrix_and_retires_only_defunct_links() {
    // Ticket 21: deleting a source matrix retains its complete Local
    // counterpart (not loose parts), retires only the defunct records, and
    // preserves surviving ownership, anchors and unrelated links.
    let (mut engine, doc) = linked_anchored_fixture();
    let mut mirror_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
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
    let poses_before: Vec<(String, Pose2)> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let (_, removed) = edit(
        &mut engine,
        &doc,
        EditOperation::RemoveMatrix { id: "main".into() },
    );
    // Source gone with its parts; the Local matrix survives complete.
    assert!(
        removed.matrices.iter().all(|matrix| matrix.id != "main"),
        "source matrix removed"
    );
    assert!(
        removed
            .parts
            .iter()
            .all(|part| !part.id.starts_with("matrix/main/")),
        "source parts removed"
    );
    let survivor = removed
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main2")
        .expect("complete Local survivor");
    assert_eq!((survivor.rows, survivor.columns), (1, 3));
    for column in 0..3 {
        assert!(
            removed
                .parts
                .iter()
                .any(|part| part.id == member("main2", 0, column)),
            "survivor part (0,{column}) retained"
        );
    }
    for (id, pose) in &poses_before {
        if id.starts_with("matrix/main2/") {
            assert_eq!(pose_of(&removed, id), *pose, "{id}");
        }
    }
    // Only the defunct correspondence retired; the unrelated pair intact.
    let link = removed
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .and_then(|layout| layout.mirror_link.as_ref())
        .expect("surviving link");
    assert!(
        link.correspondences.iter().all(|record| {
            !(record.kind == MirroredItemKind::Matrix
                && (record.source_id == "main" || record.mirror_id == "main2"))
        }),
        "defunct matrix correspondence retired"
    );
    assert!(
        link.correspondences.iter().any(|record| {
            record.kind == MirroredItemKind::Matrix
                && record.source_id == "thumb"
                && record.mirror_id == "thumb2"
        }),
        "unrelated link preserved"
    );
    // The source-anchored dependent detached with its frame; the surviving
    // anchored pair still evaluates.
    assert_eq!(anchor_of(&removed, "thumb"), None);
    assert_eq!(
        pose_of(&removed, &member("thumb2", 0, 0)).at,
        Vec2 { x: -18.0, y: -19.0 }
    );
    // One Undo restores the source; Redo removes it again.
    let undone_doc = undone(&mut engine);
    assert!(undone_doc.matrices.iter().any(|matrix| matrix.id == "main"));
    let redone_doc = redone(&mut engine);
    assert!(redone_doc.matrices.iter().all(|matrix| matrix.id != "main"));
    assert!(
        redone_doc
            .matrices
            .iter()
            .any(|matrix| matrix.id == "main2")
    );
}

use super::*;

#[test]
fn add_anchored_matrix_creates_1x3_in_same_layout_and_follows_its_anchor_key() {
    let (mut engine, doc) = fixture();
    let anchor_key = member("main", 0, 2);
    let before = at(&doc, &anchor_key);
    assert_eq!(before, Vec2 { x: 38.0, y: 0.0 });

    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::AddAnchoredMatrix {
            id: "thumb".into(),
            anchor: MatrixAnchor {
                matrix_id: "main".into(),
                row: 0,
                column: 2,
                follow: Follow {
                    x: true,
                    y: true,
                    rotation: true,
                },
                aggregate: None,
            },
        },
    );
    let thumb = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "thumb")
        .expect("added matrix is stored");
    assert_eq!((thumb.rows, thumb.columns), (1, 3));
    assert_eq!(thumb.origin, Vec2 { x: 0.0, y: -19.0 });
    assert_eq!(thumb.board_id.as_deref(), Some("board"));
    assert!(
        doc.layouts
            .iter()
            .find(|layout| layout.id == "layout")
            .is_some_and(|layout| layout.matrix_ids.contains(&"thumb".to_string()))
    );
    // Dependent members hang off the anchor key's final pose plus the relative origin.
    assert_eq!(at(&doc, &member("thumb", 0, 0)), Vec2 { x: 38.0, y: -19.0 });
    assert_eq!(at(&doc, &member("thumb", 0, 2)), Vec2 { x: 76.0, y: -19.0 });

    // Moving the anchor key moves the anchored members with it.
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: anchor_key.clone(),
                at: Vec2 { x: 43.0, y: 7.0 },
            }],
        },
    );
    assert_eq!(at(&doc, &anchor_key), Vec2 { x: 43.0, y: 7.0 });
    assert_eq!(at(&doc, &member("thumb", 0, 0)), Vec2 { x: 43.0, y: -12.0 });
    assert_eq!(at(&doc, &member("thumb", 0, 2)), Vec2 { x: 81.0, y: -12.0 });
}

#[test]
fn linked_detach_clears_both_anchors_and_keeps_both_poses() {
    let (mut engine, doc) = linked_anchored_fixture();
    assert!(anchor_of(&doc, "thumb").is_some());
    assert!(anchor_of(&doc, "thumb2").is_some());
    // Mirror sanity: the partner group is the x=10 reflection of the source.
    assert_eq!(
        pose_of(&doc, &member("thumb2", 0, 0)).at,
        Vec2 { x: -18.0, y: -19.0 }
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
    // Group-level detach rides the normal pair sync: both anchors clear.
    assert_eq!(anchor_of(&detached, "thumb"), None);
    assert_eq!(anchor_of(&detached, "thumb2"), None);
    // Every materialized pose on both halves is unchanged.
    for (id, pose) in &poses_before {
        assert_eq!(pose_of(&detached, id), *pose, "{id}");
    }
    // One transaction: Undo restores both anchors, Redo clears both again.
    let undone_doc = undone(&mut engine);
    assert_eq!(anchor_of(&undone_doc, "thumb"), anchor_of(&doc, "thumb"));
    assert_eq!(anchor_of(&undone_doc, "thumb2"), anchor_of(&doc, "thumb2"));
    let redone_doc = redone(&mut engine);
    assert_eq!(anchor_of(&redone_doc, "thumb"), None);
    assert_eq!(anchor_of(&redone_doc, "thumb2"), None);
}

#[test]
fn anchoring_to_a_constraint_driven_key_is_refused() {
    let (mut engine, doc) = rich_fixture();
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetConstraint {
            constraint: Constraint::Offset {
                id: "drive".into(),
                source_part_id: member("main", 0, 1),
                target_part_id: member("main", 0, 0),
                offset: Vec2 { x: -19.0, y: 0.0 },
                rotation: 0.0,
            },
        },
    );
    let message = error(
        &mut engine,
        &doc,
        EditOperation::AddAnchoredMatrix {
            id: "thumb".into(),
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
    assert!(
        message.contains("constraint-driven"),
        "unexpected message: {message:?}"
    );
    assert_eq!(snapshot(&mut engine), doc);
}

#[test]
// Ticket 20: Adding to a linked group now creates the mirrored counterpart
// in the same edit. The name is retained for the native baseline so the new
// `matrix_anchors` names stay purely additive.
fn adding_to_a_linked_layout_is_refused_until_ticket_20() {
    let (mut engine, doc) = fixture();
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::CreateMirroredPair {
            left: Layout {
                id: "left-layout".into(),
                name: "Left half".into(),
                board_id: "board".into(),
                matrix_ids: vec!["left-matrix".into()],
                part_ids: vec![],
                mirror_link: None,
            },
            right: Layout {
                id: "right-layout".into(),
                name: "Right half".into(),
                board_id: "board".into(),
                matrix_ids: vec!["right-matrix".into()],
                part_ids: vec![],
                mirror_link: Some(LayoutMirrorLink {
                    source_id: "left-layout".into(),
                    axis_x: 10.0,
                    split_gap: None,
                    spacing: None,
                    correspondences: vec![],
                    exceptions: vec![],
                }),
            },
            matrix: Box::new(Matrix {
                id: "left-matrix".into(),
                name: None,
                rows: 1,
                columns: 1,
                pitch: Vec2 { x: 19.0, y: 19.0 },
                origin: Vec2::default(),
                definition_id: "switch".into(),
                part_ids: vec![],
                board_id: Some("board".into()),
                mirror: None,
                rotation: None,
                edge_gap: None,
                diode_direction: None,
                row_layout: vec![],
                column_layout: vec![],
                anchor: None,
                cells: vec![],
            }),
            definitions: None,
        },
    );
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::AddAnchoredMatrix {
            id: "claw".into(),
            anchor: MatrixAnchor {
                matrix_id: "left-matrix".into(),
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
    // Both halves gain a 1x3 thumb; the mirror anchors to the counterpart.
    let claw = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "claw")
        .expect("claw matrix");
    let mirror = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "claw-mirror")
        .expect("claw mirror");
    assert_eq!(
        claw.anchor.as_ref().expect("claw anchor").matrix_id,
        "left-matrix"
    );
    assert_eq!(
        mirror.anchor.as_ref().expect("mirror anchor").matrix_id,
        "right-matrix"
    );
    // Relative Origin X negates while Y stays; the default (0,-19) is fixed.
    assert_eq!(claw.origin, Vec2 { x: 0.0, y: -19.0 });
    assert_eq!(mirror.origin, Vec2 { x: -0.0, y: -19.0 });
    // Partner poses are the x=10 reflection of the source. Ticket 11: the
    // 1x1 creation requested a 2 mm gap, so the stored gap is the 40 mm
    // default and both anchored thumbs ride 19 mm farther out.
    assert_eq!(
        pose_of(&doc, &member("claw", 0, 0)).at,
        Vec2 { x: -19.0, y: -19.0 }
    );
    assert_eq!(
        pose_of(&doc, &member("claw-mirror", 0, 0)).at,
        Vec2 { x: 39.0, y: -19.0 }
    );
    // Layout membership and matrix correspondences cover both.
    let left = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "left-layout")
        .unwrap();
    let right = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "right-layout")
        .unwrap();
    assert!(left.matrix_ids.contains(&"claw".to_string()));
    assert!(right.matrix_ids.contains(&"claw-mirror".to_string()));
    let link = right.mirror_link.as_ref().expect("mirror link");
    assert!(link.correspondences.iter().any(|record| {
        record.kind == MirroredItemKind::Matrix
            && record.source_id == "claw"
            && record.mirror_id == "claw-mirror"
    }));
    // One Undo removes both, Redo restores both.
    let undone_doc = undone(&mut engine);
    assert!(undone_doc.matrices.iter().all(|matrix| matrix.id != "claw"));
    assert!(
        undone_doc
            .matrices
            .iter()
            .all(|matrix| matrix.id != "claw-mirror")
    );
    let redone_doc = redone(&mut engine);
    assert!(redone_doc.matrices.iter().any(|matrix| matrix.id == "claw"));
    assert!(
        redone_doc
            .matrices
            .iter()
            .any(|matrix| matrix.id == "claw-mirror")
    );
}

#[test]
fn anchor_follow_rotation_filters_the_completed_heading() {
    let (mut engine, doc) = anchored_fixture();
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
    let followed = pose_of(&doc, &member("thumb", 0, 0));
    assert_eq!(followed.rotation, 30.0);
    let cos30 = 30f64.to_radians().cos();
    let sin30 = 30f64.to_radians().sin();
    near(
        followed.at,
        38.0 * cos30 + 19.0 * sin30,
        38.0 * sin30 - 19.0 * cos30,
    );
    // Without Follow rotation the heading zeroes but the rotated candidate
    // position stands.
    let doc = set_follow(&mut engine, &doc, true, true, false);
    let unfollowed = pose_of(&doc, &member("thumb", 0, 0));
    assert_eq!(unfollowed.rotation, 0.0);
    assert_eq!(unfollowed.at, followed.at);
}

#[test]
fn anchored_matrix_translation_respects_follow_and_refuses_impossible_drags() {
    let (mut engine, doc) = anchored_fixture();
    let doc = set_follow(&mut engine, &doc, false, true, true);
    // A drag along the unfollowed X cannot be represented: refuse atomically.
    let message = error(
        &mut engine,
        &doc,
        EditOperation::SetMatrixPlacement {
            matrix_id: "thumb".into(),
            change: MatrixPlacementChange::MatrixTranslation {
                delta: Vec2 { x: 5.0, y: 0.0 },
            },
        },
    );
    assert!(
        message.contains("Follow"),
        "unexpected message: {message:?}"
    );
    assert_eq!(snapshot(&mut engine), doc);
    // A drag along the followed Y succeeds and moves every member.
    let (_, moved) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixPlacement {
            matrix_id: "thumb".into(),
            change: MatrixPlacementChange::MatrixTranslation {
                delta: Vec2 { x: 0.0, y: 5.0 },
            },
        },
    );
    assert_eq!(
        pose_of(&moved, &member("thumb", 0, 0)).at,
        Vec2 { x: 0.0, y: -14.0 }
    );
    assert_eq!(
        pose_of(&moved, &member("thumb", 0, 2)).at,
        Vec2 { x: 38.0, y: -14.0 }
    );
}

#[test]
fn anchored_frame_edits_refuse_unfollowed_components() {
    let (mut engine, doc) = anchored_fixture();
    let doc = set_follow(&mut engine, &doc, true, true, false);
    let mut rotated = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "thumb")
        .expect("thumb matrix")
        .clone();
    rotated.rotation = Some(15.0);
    let message = error(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: rotated,
            definitions: None,
        },
    );
    assert!(
        message.contains("not followed"),
        "unexpected message: {message:?}"
    );
    assert_eq!(snapshot(&mut engine), doc);
}

#[test]
fn constraints_consume_final_anchored_poses() {
    let (mut engine, doc) = anchored_fixture();
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetConstraint {
            constraint: Constraint::Offset {
                id: "ride".into(),
                source_part_id: member("main", 0, 0),
                target_part_id: member("thumb", 0, 1),
                offset: Vec2 { x: 5.0, y: 5.0 },
                rotation: 0.0,
            },
        },
    );
    // The anchored member solves from its laid-out pose, not from a stale one.
    assert_eq!(
        pose_of(&doc, &member("thumb", 0, 1)).at,
        Vec2 { x: 5.0, y: 5.0 }
    );
}

#[test]
fn linked_mirror_originating_shift_bend_reflects_heading_once() {
    // Edit from the mirror half: stored layout values copy unchanged while
    // the kernel negates the whole local heading exactly once.
    let (mut engine, doc) = linked_anchored_fixture();
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixPlacement {
            matrix_id: "main2".into(),
            change: MatrixPlacementChange::RowShift {
                row: 0,
                value: Vec2 { x: 5.0, y: 3.0 },
            },
        },
    );
    let source_shift = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main")
        .and_then(|matrix| matrix.row_layout.first().map(|record| record.shift))
        .unwrap_or_default();
    let mirror_shift = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main2")
        .and_then(|matrix| matrix.row_layout.first().map(|record| record.shift))
        .unwrap_or_default();
    // Stored values copy unchanged; the mirror does not negate them twice.
    assert_eq!(source_shift, Vec2 { x: 5.0, y: 3.0 });
    assert_eq!(mirror_shift, Vec2 { x: 5.0, y: 3.0 });
    // World poses are exact reflections about x=10 (sum 20), not copies.
    let source_pose = pose_of(&doc, &member("main", 0, 1));
    let mirror_pose = pose_of(&doc, &member("main2", 0, 1));
    assert!(
        (source_pose.at.x + mirror_pose.at.x - 20.0).abs() < 1e-9
            && (source_pose.at.y - mirror_pose.at.y).abs() < 1e-9,
        "expected reflection, got {source_pose:?} vs {mirror_pose:?}"
    );
    // Key Bend from the mirror half copies too, with single heading negation.
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixPlacement {
            matrix_id: "main2".into(),
            change: MatrixPlacementChange::KeyBend {
                row: 0,
                column: 1,
                value: 15.0,
            },
        },
    );
    let source_bend = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main")
        .and_then(|matrix| {
            matrix
                .cells
                .iter()
                .find(|cell| cell.row == 0 && cell.column == 1)
                .and_then(|cell| cell.bend)
        })
        .unwrap_or_default();
    let mirror_bend = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main2")
        .and_then(|matrix| {
            matrix
                .cells
                .iter()
                .find(|cell| cell.row == 0 && cell.column == 1)
                .and_then(|cell| cell.bend)
        })
        .unwrap_or_default();
    assert_eq!(source_bend, 15.0);
    assert_eq!(mirror_bend, 15.0);
    // Headings are opposite-signed (single negation), not equal or doubled.
    let source_pose = pose_of(&doc, &member("main", 0, 1));
    let mirror_pose = pose_of(&doc, &member("main2", 0, 1));
    assert!(
        (source_pose.rotation + mirror_pose.rotation).abs() < 1e-9,
        "expected opposite headings, got {source_pose:?} vs {mirror_pose:?}"
    );
}

#[test]
fn linked_omitted_anchor_keeps_live_logical_pose() {
    // Omitted mirrors suppress inherited parts but keep a live logical pose
    // for anchors, without freezing or detaching. Open with the exception
    // from the start (removal lifecycle belongs to ticket 21).
    let (_, doc) = linked_anchored_fixture();
    let mut omitted = doc.clone();
    // Drop the omitted matrix's materialized parts; Open must not recreate
    // them, but anchors through it stay live.
    omitted
        .parts
        .retain(|part| !part.id.starts_with("matrix/main2/"));
    for board in &mut omitted.boards {
        board.part_ids.retain(|id| !id.starts_with("matrix/main2/"));
    }
    if let Some(layout) = omitted
        .layouts
        .iter_mut()
        .find(|layout| layout.id == "mirror-layout")
        && let Some(link) = layout.mirror_link.as_mut()
    {
        link.exceptions.push(MirroredException {
            kind: MirroredItemKind::Matrix,
            mirror_id: "main2".into(),
            state: MirroredExceptionState::Omitted,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    }
    let (_, _, doc) = open(omitted);
    // No materialized parts for the omitted anchor target.
    assert!(
        doc.parts
            .iter()
            .all(|part| part.id != member("main2", 0, 2))
    );
    // Both dependants still evaluate through the live logical anchor: source
    // thumb at its anchor key plus relative origin, mirror thumb at the
    // reflected logical pose (axis x=10).
    assert_eq!(
        pose_of(&doc, &member("thumb", 0, 0)).at,
        Vec2 { x: 38.0, y: -19.0 }
    );
    assert_eq!(
        pose_of(&doc, &member("thumb2", 0, 0)).at,
        Vec2 { x: -18.0, y: -19.0 }
    );
}

#[test]
fn linked_compaction_excludes_local_partner() {
    // Ticket 21: a grouped column removal edits the linked source alone; the
    // Local partner keeps its own dimensions and every world pose.
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
    let (_, compacted) = edit(
        &mut engine,
        &doc,
        EditOperation::RemoveMatrixColumn {
            matrix_id: "main".into(),
            column: 0,
        },
    );
    // Source compacted; Local partner untouched.
    assert_eq!(
        compacted
            .matrices
            .iter()
            .find(|matrix| matrix.id == "main")
            .expect("compacted source")
            .columns,
        2
    );
    let partner = compacted
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main2")
        .expect("local partner");
    assert_eq!((partner.rows, partner.columns), (1, 3));
    // The Local partner keeps every world pose; the linked thumb pair
    // follows the compacted anchor (ticket 19 remap), so only main2 stays.
    for (id, pose) in &poses_before {
        if id.starts_with("matrix/main2/") {
            assert_eq!(pose_of(&compacted, id), *pose, "{id}");
        }
    }
    assert!(
        compacted
            .parts
            .iter()
            .any(|part| part.id == member("main2", 0, 2))
    );
    // The source anchor compacted onto its surviving physical key.
    assert_eq!(
        anchor_of(&compacted, "thumb"),
        Some(MatrixAnchor {
            matrix_id: "main".into(),
            row: 0,
            column: 1,
            follow: Follow {
                x: true,
                y: true,
                rotation: true,
            },
            aggregate: None,
        })
    );
    let undone_doc = undone(&mut engine);
    assert_eq!(undone_doc.matrices, doc.matrices);
}

#[test]
fn linked_detach_confirmed_applies_detach_and_local_in_one_entry() {
    // Ticket 21: the confirmed detach clears both anchors, keeps both world
    // poses, and records the Local exception in one Undo entry.
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
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: member("main2", 0, 2),
                at: Vec2 { x: -10.0, y: 5.0 },
            }],
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
        EditOperation::DetachMatrixAnchorConfirmed {
            matrix_id: "thumb".into(),
            local_matrix_id: "thumb2".into(),
        },
    );
    // Both anchors cleared, every world pose kept, exception recorded.
    assert_eq!(anchor_of(&detached, "thumb"), None);
    assert_eq!(anchor_of(&detached, "thumb2"), None);
    for (id, pose) in &poses_before {
        assert_eq!(pose_of(&detached, id), *pose, "{id}");
    }
    assert!(
        detached
            .layouts
            .iter()
            .find(|layout| layout.id == "mirror-layout")
            .and_then(|layout| layout.mirror_link.as_ref())
            .is_some_and(|link| link.exceptions.iter().any(|exception| {
                exception.kind == MirroredItemKind::Matrix
                    && exception.mirror_id == "thumb2"
                    && exception.state == MirroredExceptionState::Local
            }))
    );
    // One Undo restores anchors and drops the exception; Redo reapplies all.
    let undone_doc = undone(&mut engine);
    assert_eq!(anchor_of(&undone_doc, "thumb"), anchor_of(&doc, "thumb"));
    assert_eq!(anchor_of(&undone_doc, "thumb2"), anchor_of(&doc, "thumb2"));
    assert_eq!(undone_doc.layouts, doc.layouts);
    let redone_doc = redone(&mut engine);
    assert_eq!(anchor_of(&redone_doc, "thumb"), None);
    assert_eq!(anchor_of(&redone_doc, "thumb2"), None);
    assert_eq!(redone_doc.layouts, detached.layouts);
    // A stale confirmation (wrong partner) refuses without changes.
    let current = snapshot(&mut engine);
    let stale = error(
        &mut engine,
        &current,
        EditOperation::DetachMatrixAnchorConfirmed {
            matrix_id: "thumb".into(),
            local_matrix_id: "main2".into(),
        },
    );
    assert!(stale.contains("stale"), "unexpected message: {stale:?}");
    assert_eq!(snapshot(&mut engine), current);
}

#[test]
fn linked_source_removal_deletes_linked_partner_and_keeps_unrelated_pair() {
    // Ticket 21: without an exception the linked counterpart follows the
    // source deletion; unrelated pairs and their poses survive.
    let (mut engine, doc) = linked_anchored_fixture();
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
    // Both halves of the removed pair are gone with their parts.
    for gone in ["main", "main2"] {
        assert!(
            removed.matrices.iter().all(|matrix| matrix.id != gone),
            "{gone} removed"
        );
        assert!(
            removed
                .parts
                .iter()
                .all(|part| !part.id.starts_with(&format!("matrix/{gone}/"))),
            "{gone} parts removed"
        );
    }
    // The unrelated pair survives with its poses and correspondence.
    for kept in ["thumb", "thumb2"] {
        assert!(
            removed.matrices.iter().any(|matrix| matrix.id == kept),
            "{kept} survives"
        );
    }
    for (id, pose) in &poses_before {
        if id.starts_with("matrix/thumb") {
            assert_eq!(pose_of(&removed, id), *pose, "{id}");
        }
    }
    let link = removed
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .and_then(|layout| layout.mirror_link.as_ref())
        .expect("surviving link");
    assert!(
        link.correspondences.iter().any(|record| {
            record.kind == MirroredItemKind::Matrix
                && record.source_id == "thumb"
                && record.mirror_id == "thumb2"
        }),
        "unrelated correspondence preserved"
    );
    assert!(
        link.correspondences
            .iter()
            .all(|record| { record.source_id != "main" && record.mirror_id != "main2" }),
        "defunct records retired"
    );
    let undone_doc = undone(&mut engine);
    assert!(undone_doc.matrices.iter().any(|matrix| matrix.id == "main"));
    assert!(
        undone_doc
            .matrices
            .iter()
            .any(|matrix| matrix.id == "main2")
    );
}

#[test]
fn linked_source_removal_retires_omitted_records_without_resurrection() {
    // Ticket 21: an Omitted counterpart stays absent and its record retires
    // with the source; nothing resurrects it.
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
    let (_, removed) = edit(
        &mut engine,
        &doc,
        EditOperation::RemoveMatrix { id: "main".into() },
    );
    assert!(
        removed.matrices.iter().all(|matrix| matrix.id != "main"),
        "source removed"
    );
    assert!(
        removed.matrices.iter().all(|matrix| matrix.id != "main2"),
        "omitted record retired"
    );
    assert!(
        removed
            .parts
            .iter()
            .all(|part| !part.id.starts_with("matrix/main2/")),
        "nothing resurrected"
    );
    // The surviving pair still evaluates through its live anchors.
    assert_eq!(
        pose_of(&removed, &member("thumb2", 0, 0)).at,
        Vec2 { x: -18.0, y: -19.0 }
    );
    let undone_doc = undone(&mut engine);
    assert!(undone_doc.matrices.iter().any(|matrix| matrix.id == "main"));
}

use super::*;

#[test]
fn add_anchored_matrix_creates_1x3_in_same_layout_and_follows_its_anchor_key() {
    let (mut engine, doc) = fixture();
    let anchor_key = member("main", 0, 2);
    let before = at(&doc, &anchor_key);
    assert_eq!(before, Vec2 { x: 38.0, y: 0.0 });

    let (_, doc) = commit_edit(
        &mut engine,
        &doc,
        add_anchored("thumb", key_anchor("main", 0, 2)),
    );
    let thumb = matrix(&doc, "thumb");
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
    assert_at(&doc, &member("thumb", 0, 0), 38.0, -19.0);
    assert_at(&doc, &member("thumb", 0, 2), 76.0, -19.0);

    // Moving the anchor key moves the anchored members with it.
    let (_, doc) = commit_edit(
        &mut engine,
        &doc,
        move_part(anchor_key.clone(), Vec2 { x: 43.0, y: 7.0 }),
    );
    assert_at(&doc, &anchor_key, 43.0, 7.0);
    assert_at(&doc, &member("thumb", 0, 0), 43.0, -12.0);
    assert_at(&doc, &member("thumb", 0, 2), 81.0, -12.0);
}

#[test]
fn anchoring_to_a_constraint_driven_key_is_refused() {
    let (mut engine, doc) = rich_fixture();
    let (_, doc) = commit_edit(
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
        add_anchored("thumb", key_anchor("main", 0, 0)),
    );
    assert!(
        message.contains("constraint-driven"),
        "unexpected message: {message:?}"
    );
    assert_eq!(snapshot(&mut engine), doc);
}

#[test]
fn anchor_follow_rotation_filters_the_completed_heading() {
    let (mut engine, doc) = anchored_fixture();
    let (_, doc) = commit_edit(
        &mut engine,
        &doc,
        change_matrix(&doc, "main", |matrix| matrix.rotation = Some(30.0)),
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
    let (_, moved) = commit_edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixPlacement {
            matrix_id: "thumb".into(),
            change: MatrixPlacementChange::MatrixTranslation {
                delta: Vec2 { x: 0.0, y: 5.0 },
            },
        },
    );
    assert_at(&moved, &member("thumb", 0, 0), 0.0, -14.0);
    assert_at(&moved, &member("thumb", 0, 2), 38.0, -14.0);
}

#[test]
fn anchored_frame_edits_refuse_unfollowed_components() {
    let (mut engine, doc) = anchored_fixture();
    let doc = set_follow(&mut engine, &doc, true, true, false);
    let message = error(
        &mut engine,
        &doc,
        change_matrix(&doc, "thumb", |matrix| matrix.rotation = Some(15.0)),
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
    let (_, doc) = commit_edit(
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
    assert_at(&doc, &member("thumb", 0, 1), 5.0, 5.0);
}

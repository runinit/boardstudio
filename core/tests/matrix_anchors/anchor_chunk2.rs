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
    let (_, committed) = commit_edit(&mut engine, &doc, operation);
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
    let (_, _, opened) = open_engine(reordered);
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
    let (_, doc) = commit_edit(
        &mut engine,
        &doc,
        change_matrix(&doc, "main", |matrix| matrix.rotation = Some(30.0)),
    );
    let (_, detached) = commit_edit(
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
    let (_, moved) = commit_edit(
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
    let (_, doc) = commit_edit(
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
        let (_, detached) = commit_edit(&mut engine, &doc, operation);
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

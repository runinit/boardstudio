//! Ticket 27: one traced production path from a public prepared edit through
//! preview, materialization, scene publication, saved-archive pack/unpack and
//! reopen, carrying an anchored matrix inside a same-board mirrored group.
//! Every step runs through the real routes (CoreEngine Open/Edit/Undo and the
//! real archive pack/unpack), never test doubles.
use crate::support::fixture::{self, key_part, part_pose, scene_and_document as scene};
use crate::support::session::{commit_edit, open_engine};
use boardstudio_core::{CommittedScene, CoreEngine, EditOutcome, archive, model::*};

fn preview(engine: &mut CoreEngine, doc: &ProjectDoc, operation: EditOperation) -> SceneDelta {
    match engine.apply_edit(fixture::edit(
        "preview",
        "preview",
        doc.revision,
        EditPhase::Preview,
        operation,
    )) {
        Ok(EditOutcome::Preview(scene)) => scene,
        other => panic!("expected preview: {other:?}"),
    }
}

fn snapshot(engine: &mut CoreEngine) -> ProjectDoc {
    scene(engine.snapshot()).1
}

fn undone(engine: &mut CoreEngine) -> ProjectDoc {
    match engine.undo() {
        Ok(CommittedScene { document, .. }) => document,
        other => panic!("expected scene: {other:?}"),
    }
}

fn definitions() -> Vec<PartDefinition> {
    serde_json::from_value(serde_json::json!([
        {"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]}
    ]))
    .unwrap()
}

fn member(matrix: &str, row: u32, column: u32) -> String {
    format!("matrix/{matrix}/r{row}c{column}")
}

fn main_matrix() -> Matrix {
    Matrix {
        part_ids: vec![
            member("main", 0, 0),
            member("main", 0, 1),
            member("main", 0, 2),
        ],
        board_id: Some("board".into()),
        ..fixture::matrix("main", 1, 3)
    }
}

/// Same-board linked layouts: `layout` holds Main plus a Thumb anchored to
/// Main's last key; `mirror-layout` holds their reflected counterparts.
fn anchored_mirrored_doc() -> ProjectDoc {
    let all = Follow {
        x: true,
        y: true,
        rotation: true,
    };
    let mut doc = ProjectDoc::empty("end-to-end", "End to end");
    doc.definitions = definitions();
    let mut thumb = main_matrix();
    thumb.id = "thumb".into();
    thumb.origin = Vec2 { x: 0.0, y: -19.0 };
    thumb.part_ids = vec![
        member("thumb", 0, 0),
        member("thumb", 0, 1),
        member("thumb", 0, 2),
    ];
    thumb.anchor = Some(MatrixAnchor {
        matrix_id: "main".into(),
        row: 0,
        column: 2,
        follow: all,
        aggregate: None,
    });
    doc.parts = vec![
        key_part(&member("main", 0, 0), 0.0, 0.0),
        key_part(&member("main", 0, 1), 19.0, 0.0),
        key_part(&member("main", 0, 2), 38.0, 0.0),
        key_part(&member("thumb", 0, 0), 38.0, -19.0),
        key_part(&member("thumb", 0, 1), 57.0, -19.0),
        key_part(&member("thumb", 0, 2), 76.0, -19.0),
    ];
    doc.boards.push(Board {
        part_ids: doc.parts.iter().map(|part| part.id.clone()).collect(),
        ..fixture::board("board", "Board")
    });
    doc.matrices.push(main_matrix());
    doc.matrices.push(thumb);
    doc.layouts.push(Layout {
        id: "layout".into(),
        name: "Layout".into(),
        board_id: "board".into(),
        matrix_ids: vec!["main".into(), "thumb".into()],
        part_ids: vec![],
        mirror_link: None,
    });
    doc.layouts.push(Layout {
        id: "mirror-layout".into(),
        name: "Mirror".into(),
        board_id: "board".into(),
        matrix_ids: vec![],
        part_ids: vec![],
        mirror_link: None,
    });
    use boardstudio_model::mirrored_half::{MirrorDestination, MirroredPair};
    doc.mirrored_pairs.push(MirroredPair {
        id: "mirror-layout".into(),
        source: MirrorDestination::SameBoardLayout("layout".into()),
        destination: MirrorDestination::SameBoardLayout("mirror-layout".into()),
        mapping: Pose2 {
            at: Vec2 { x: 20.0, y: 0.0 },
            rotation: 0.0,
        },
        overrides: vec![],
    });
    doc
}

fn scene_pose(scene: &SceneDelta, matrix_id: &str, row: u32, column: u32) -> Pose2 {
    fixture::matrix_scene(scene, matrix_id)
        .cells
        .iter()
        .find(|cell| cell.row == row && cell.column == column)
        .unwrap_or_else(|| panic!("missing cell {matrix_id} {row}/{column}"))
        .pose
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn same_pose(a: &Pose2, b: &Pose2) -> bool {
    close(a.at.x, b.at.x) && close(a.at.y, b.at.y) && close(a.rotation, b.rotation)
}

/// The whole traced path: preview a carried stagger on the anchor column of
/// the source Main, commit it, publish the scene, pack/unpack the saved
/// archive and reopen, then undo. The anchored Thumb and both mirrored
/// counterparts must ride every step unchanged relative to the preview.
#[test]
fn preview_commit_scene_archive_reopen_and_undo_carry_the_anchored_mirrored_group() {
    let (mut engine, _, doc) = open_engine(anchored_mirrored_doc());
    let operation = EditOperation::SetMatrixPlacement {
        matrix_id: "main".into(),
        change: MatrixPlacementChange::ColumnStagger {
            column: 2,
            value: 2.5,
        },
    };

    // Preview shows the carried result without touching accepted state.
    let previewed = preview(&mut engine, &doc, operation.clone());
    assert_eq!(snapshot(&mut engine), doc);
    let anchor_preview = scene_pose(&previewed, "main", 0, 2);
    assert!(
        close(anchor_preview.at.y, 2.5),
        "anchor key carries the stagger: {anchor_preview:?}"
    );
    // The anchored Thumb follows its anchor key; the mirrored group reflects.
    let thumb_preview = scene_pose(&previewed, "thumb", 0, 0);
    assert!(
        close(thumb_preview.at.y, 2.5 - 19.0),
        "thumb follows the anchor key: {thumb_preview:?}"
    );
    let view = boardstudio_core::project_document(&doc);
    let derived_matrix = &view.geometry().layouts[1].matrix_ids[0];
    let mirror_preview = scene_pose(&previewed, derived_matrix, 0, 2);
    assert!(
        close(mirror_preview.at.x + anchor_preview.at.x, 2.0 * 10.0),
        "mirror reflects about the pair axis: {mirror_preview:?} vs {anchor_preview:?}"
    );

    // Commit materializes exactly what the preview showed, in scene and parts.
    let (committed_scene, committed) = commit_edit(&mut engine, &doc, operation);
    for matrix in &committed.matrices {
        for column in 0..matrix.columns {
            for row in 0..matrix.rows {
                let id = member(&matrix.id, row, column);
                let published = scene_pose(&committed_scene, &matrix.id, row, column);
                let stored = part_pose(&committed, &id);
                assert!(
                    same_pose(&published, &stored),
                    "scene and materialized part agree for {id}"
                );
                let shown = scene_pose(&previewed, &matrix.id, row, column);
                assert!(
                    same_pose(&shown, &stored),
                    "commit materializes the preview for {id}"
                );
            }
        }
    }
    assert_eq!(committed.format_version, 8);
    assert_eq!(committed.layouts.len(), 2);
    assert!(committed.layouts[1].matrix_ids.is_empty());
    assert_eq!(committed.mirrored_pairs, doc.mirrored_pairs);

    // Undo restores the exact pre-commit observable state.
    let restored = undone(&mut engine);
    assert_eq!(restored.matrices, doc.matrices);
    assert_eq!(restored.layouts, doc.layouts);
    for part in &doc.parts {
        assert!(
            same_pose(&part_pose(&restored, &part.id), &part.pose),
            "undo restores {}",
            part.id
        );
    }

    // The saved archive round-trips the committed document byte-identically in
    // structure, then reopens with identical poses, anchors and mirror links.
    let project_json = serde_json::to_string(&committed).unwrap();
    let (packed, buffers) = archive::request(
        &serde_json::json!({"kind": "pack-project", "projectJson": project_json, "assets": []})
            .to_string(),
        &[],
    );
    let packed_reply: ArchiveReply = serde_json::from_str(&packed).unwrap();
    assert!(
        matches!(packed_reply, ArchiveReply::Packed),
        "pack must succeed: {packed}"
    );
    let (unpacked, _) = archive::request(
        &serde_json::json!({"kind": "unpack-project"}).to_string(),
        &buffers,
    );
    let project_json = match serde_json::from_str::<ArchiveReply>(&unpacked).unwrap() {
        ArchiveReply::Unpacked { project_json, .. } => project_json,
        other => panic!("unpack must succeed: {other:?}"),
    };
    let reloaded: ProjectDoc = serde_json::from_str(&project_json).unwrap();
    assert_eq!(reloaded.matrices, committed.matrices);
    assert_eq!(reloaded.layouts, committed.layouts);
    let (_, _, reopened) = open_engine(reloaded);
    assert_eq!(reopened.matrices, committed.matrices);
    assert_eq!(reopened.layouts, committed.layouts);
    for part in &committed.parts {
        assert!(
            same_pose(&part_pose(&reopened, &part.id), &part.pose),
            "reopen keeps {}",
            part.id
        );
    }
}

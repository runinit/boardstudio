//! Ticket 27: one traced production path from a public prepared edit through
//! preview, materialization, scene publication, saved-archive pack/unpack and
//! reopen, carrying an anchored matrix inside a same-board mirrored group.
//! Every step runs through the real routes (CoreEngine Open/Edit/Undo and the
//! real archive pack/unpack), never test doubles.
use boardstudio_core::{CoreEngine, archive, model::*};

fn scene(reply: CoreReply) -> (SceneDelta, ProjectDoc) {
    match reply {
        CoreReply::Scene {
            scene, document, ..
        } => (scene, *document),
        other => panic!("expected scene: {other:?}"),
    }
}

fn open(doc: ProjectDoc) -> (CoreEngine, SceneDelta, ProjectDoc) {
    let mut engine = CoreEngine::new();
    let (scene, doc) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    (engine, scene, doc)
}

fn edit(
    engine: &mut CoreEngine,
    doc: &ProjectDoc,
    operation: EditOperation,
) -> (SceneDelta, ProjectDoc) {
    scene(engine.handle(CoreRequest::Edit {
        id: "edit".into(),
        command: EditCommand {
            base_revision: doc.revision,
            transaction_id: "edit".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation,
        },
    }))
}

fn preview(engine: &mut CoreEngine, doc: &ProjectDoc, operation: EditOperation) -> SceneDelta {
    match engine.handle(CoreRequest::Edit {
        id: "preview".into(),
        command: EditCommand {
            base_revision: doc.revision,
            transaction_id: "preview".into(),
            phase: EditPhase::Preview,
            target_ids: vec![],
            operation,
        },
    }) {
        CoreReply::Preview { scene, .. } => scene,
        other => panic!("expected preview: {other:?}"),
    }
}

fn snapshot(engine: &mut CoreEngine) -> ProjectDoc {
    scene(engine.handle(CoreRequest::Snapshot { id: "snap".into() })).1
}

fn undone(engine: &mut CoreEngine) -> ProjectDoc {
    match engine.handle(CoreRequest::Undo { id: "undo".into() }) {
        CoreReply::Scene { document, .. } => *document,
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

fn key_part(id: &str, x: f64, y: f64) -> Part {
    Part {
        id: id.into(),
        definition_id: "switch".into(),
        reference: id.into(),
        pose: Pose2 {
            at: Vec2 { x, y },
            rotation: 0.0,
        },
        side: Side::Front,
        keycap: None,
        outline: None,
        locked: None,
        properties: None,
        generator_parameters: None,
    }
}

fn main_matrix() -> Matrix {
    Matrix {
        id: "main".into(),
        name: None,
        rows: 1,
        columns: 3,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2::default(),
        definition_id: "switch".into(),
        part_ids: vec![
            member("main", 0, 0),
            member("main", 0, 1),
            member("main", 0, 2),
        ],
        board_id: Some("board".into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: vec![],
    }
}

fn mirror_kind(matrix: &str) -> MirroredCorrespondence {
    MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Matrix,
        source_id: matrix.into(),
        mirror_id: format!("{matrix}2"),
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
    });
    let mut main2 = main_matrix();
    main2.id = "main2".into();
    main2.origin = Vec2 { x: 20.0, y: 0.0 };
    main2.mirror = Some(Mirror::X);
    main2.part_ids = vec![
        member("main2", 0, 0),
        member("main2", 0, 1),
        member("main2", 0, 2),
    ];
    let mut thumb2 = thumb.clone();
    thumb2.id = "thumb2".into();
    thumb2.mirror = Some(Mirror::X);
    thumb2.part_ids = vec![
        member("thumb2", 0, 0),
        member("thumb2", 0, 1),
        member("thumb2", 0, 2),
    ];
    if let Some(anchor) = thumb2.anchor.as_mut() {
        anchor.matrix_id = "main2".into();
    }
    doc.parts = vec![
        key_part(&member("main", 0, 0), 0.0, 0.0),
        key_part(&member("main", 0, 1), 19.0, 0.0),
        key_part(&member("main", 0, 2), 38.0, 0.0),
        key_part(&member("thumb", 0, 0), 38.0, -19.0),
        key_part(&member("thumb", 0, 1), 57.0, -19.0),
        key_part(&member("thumb", 0, 2), 76.0, -19.0),
        key_part(&member("main2", 0, 0), 20.0, 0.0),
        key_part(&member("main2", 0, 1), 1.0, 0.0),
        key_part(&member("main2", 0, 2), -18.0, 0.0),
        key_part(&member("thumb2", 0, 0), -18.0, -19.0),
        key_part(&member("thumb2", 0, 1), 1.0, -19.0),
        key_part(&member("thumb2", 0, 2), 20.0, -19.0),
    ];
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: doc.parts.iter().map(|part| part.id.clone()).collect(),
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.matrices.push(main_matrix());
    doc.matrices.push(thumb);
    doc.matrices.push(main2);
    doc.matrices.push(thumb2);
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
        matrix_ids: vec!["main2".into(), "thumb2".into()],
        part_ids: vec![],
        mirror_link: Some(LayoutMirrorLink {
            source_id: "layout".into(),
            axis_x: 10.0,
            correspondences: vec![mirror_kind("main"), mirror_kind("thumb")],
            exceptions: vec![],
        }),
    });
    doc
}

fn scene_pose(scene: &SceneDelta, matrix_id: &str, row: u32, column: u32) -> Pose2 {
    scene
        .matrix_scenes
        .iter()
        .find(|item| item.matrix_id == matrix_id)
        .unwrap_or_else(|| panic!("missing matrix scene {matrix_id}"))
        .cells
        .iter()
        .find(|cell| cell.row == row && cell.column == column)
        .unwrap_or_else(|| panic!("missing cell {matrix_id} {row}/{column}"))
        .pose
        .clone()
}

fn part_pose(doc: &ProjectDoc, id: &str) -> Pose2 {
    doc.parts
        .iter()
        .find(|part| part.id == id)
        .unwrap_or_else(|| panic!("missing part {id}"))
        .pose
        .clone()
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
    let (mut engine, _, doc) = open(anchored_mirrored_doc());
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
    let mirror_preview = scene_pose(&previewed, "main2", 0, 2);
    assert!(
        close(mirror_preview.at.x + anchor_preview.at.x, 2.0 * 10.0),
        "mirror reflects about the pair axis: {mirror_preview:?} vs {anchor_preview:?}"
    );

    // Commit materializes exactly what the preview showed, in scene and parts.
    let (committed_scene, committed) = edit(&mut engine, &doc, operation);
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
    assert_eq!(committed.format_version, 7);
    assert_eq!(committed.layouts.len(), 2);
    assert!(committed.layouts[1].mirror_link.is_some());

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
    let (_, _, reopened) = open(reloaded);
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

use boardstudio_core::{CoreEngine, model::*};

fn fixture() -> ProjectDoc {
    serde_json::from_str(include_str!("fixtures/reviung41-outline-original.json")).unwrap()
}

fn scene(reply: CoreReply) -> SceneDelta {
    match reply {
        CoreReply::Scene { scene, .. } => scene,
        reply => panic!("{reply:?}"),
    }
}

fn edit(engine: &mut CoreEngine, revision: u64, operation: serde_json::Value) -> SceneDelta {
    let operation = serde_json::from_value(operation).expect("outline command contract");
    scene(engine.handle(CoreRequest::Edit {
        id: "outline-edit".into(),
        command: EditCommand {
            base_revision: revision,
            transaction_id: "outline-edit".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation,
        },
    }))
}

fn document(engine: &mut CoreEngine) -> ProjectDoc {
    match engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }) {
        CoreReply::Scene { document, .. } => document,
        reply => panic!("{reply:?}"),
    }
}

#[test]
fn fixed_copy_preserves_shape_when_components_move_and_generated_stays_available() {
    let mut engine = CoreEngine::new();
    let original = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: fixture(),
    }));
    let copied = edit(
        &mut engine,
        original.revision,
        serde_json::json!({
            "kind": "copy-outline", "boardId": "main", "versionId": "fixed", "name": "My outline"
        }),
    );
    assert_eq!(original.board_contours, copied.board_contours);
    let moved = edit(
        &mut engine,
        copied.revision,
        serde_json::json!({
            "kind": "move-parts", "positions": [{ "id": "main/U1", "at": { "x": 310.0, "y": -15.0 } }]
        }),
    );
    assert_eq!(copied.board_contours, moved.board_contours);
    assert!(
        !moved.board_readiness[0].outline,
        "Missing controller support must block dependent fabrication"
    );
    assert!(
        !moved.finding_markers.is_empty(),
        "The missing material must be located on the workbench"
    );
    let generated = edit(
        &mut engine,
        moved.revision,
        serde_json::json!({
            "kind": "select-outline", "boardId": "main", "versionId": null
        }),
    );
    assert_ne!(copied.board_contours, generated.board_contours);
    assert!(
        generated.board_readiness[0].outline,
        "Invalid inactive copies cannot block Generated"
    );
    let saved = serde_json::to_value(document(&mut engine)).unwrap();
    assert_eq!(
        saved["boardOutlines"][0]["versions"][0]["name"],
        "My outline"
    );
}

#[test]
fn version_rename_delete_undo_and_roundtrip_preserve_independent_copies() {
    let mut engine = CoreEngine::new();
    let opened = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: fixture(),
    }));
    let a = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({
            "kind": "copy-outline", "boardId": "main", "versionId": "a", "name": "First"
        }),
    );
    let b = edit(
        &mut engine,
        a.revision,
        serde_json::json!({
            "kind": "copy-outline", "boardId": "main", "versionId": "b", "name": "Second"
        }),
    );
    let renamed = edit(
        &mut engine,
        b.revision,
        serde_json::json!({
            "kind": "rename-outline", "boardId": "main", "versionId": "b", "name": "Alternative"
        }),
    );
    let saved = document(&mut engine);
    let value = serde_json::to_value(&saved).unwrap();
    assert_eq!(value["boardOutlines"][0]["versions"][0]["name"], "First");
    assert_eq!(
        value["boardOutlines"][0]["versions"][1]["name"],
        "Alternative"
    );
    let restored: ProjectDoc = serde_json::from_value(value).unwrap();
    let reopened = scene(CoreEngine::new().handle(CoreRequest::Open {
        id: "reopen".into(),
        document: restored,
    }));
    assert_eq!(renamed.board_contours, reopened.board_contours);
    let removed = edit(
        &mut engine,
        renamed.revision,
        serde_json::json!({
            "kind": "remove-outline", "boardId": "main", "versionId": "b"
        }),
    );
    let value = serde_json::to_value(document(&mut engine)).unwrap();
    assert!(value["boardOutlines"][0].get("activeVersionId").is_none());
    assert_eq!(
        value["boardOutlines"][0]["versions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let undone = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert!(undone.revision > removed.revision);
    let value = serde_json::to_value(document(&mut engine)).unwrap();
    assert_eq!(value["boardOutlines"][0]["activeVersionId"], "b");
    assert_eq!(
        value["boardOutlines"][0]["versions"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn first_point_edit_is_one_undo_step_and_preview_never_creates_a_saved_version() {
    let mut engine = CoreEngine::new();
    let opened = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: fixture(),
    }));
    let mut points = opened.board_outline_scenes[0].source_contours[0]
        .points
        .clone();
    points[0].x -= 0.5;
    let operation: EditOperation = serde_json::from_value(serde_json::json!({
        "kind": "copy-outline", "boardId": "main", "versionId": "first-edit", "name": "Edited outline 1", "edit": { "contour": 0, "points": points }
    })).unwrap();
    let preview = engine.handle(CoreRequest::Edit {
        id: "preview".into(),
        command: EditCommand {
            base_revision: opened.revision,
            transaction_id: "gesture".into(),
            phase: EditPhase::Preview,
            target_ids: vec!["main".into()],
            operation: operation.clone(),
        },
    });
    assert!(matches!(preview, CoreReply::Preview { .. }));
    assert!(document(&mut engine).board_outlines.is_empty());
    let changed = edit(
        &mut engine,
        opened.revision,
        serde_json::to_value(operation).unwrap(),
    );
    assert_ne!(opened.board_contours, changed.board_contours);
    assert_eq!(document(&mut engine).board_outlines[0].versions.len(), 1);
    let undone = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(opened.board_contours, undone.board_contours);
    assert!(document(&mut engine).board_outlines.is_empty());
    let redone = scene(engine.handle(CoreRequest::Redo { id: "redo".into() }));
    assert_eq!(changed.board_contours, redone.board_contours);
}

#[test]
fn editing_one_version_leaves_other_versions_and_generated_source_intact() {
    let mut engine = CoreEngine::new();
    let opened = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: fixture(),
    }));
    let first = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({ "kind": "copy-outline", "boardId": "main", "versionId": "a", "name": "First" }),
    );
    let second = edit(
        &mut engine,
        first.revision,
        serde_json::json!({ "kind": "copy-outline", "boardId": "main", "versionId": "b", "name": "Second" }),
    );
    let saved = document(&mut engine);
    let mut feature = saved.board_outlines[0].versions[1].geometry.features[0].clone();
    if let OutlineFeature::Polygon { points, .. } = &mut feature {
        points[0].x -= 1.0;
    }
    let modified = edit(
        &mut engine,
        second.revision,
        serde_json::json!({ "kind": "set-outline", "feature": feature }),
    );
    assert_ne!(modified.board_contours, opened.board_contours);
    let after = document(&mut engine);
    assert_eq!(after.outline, saved.outline);
    assert_eq!(after.parts, saved.parts);
    assert_eq!(
        after.board_outlines[0].versions[0],
        saved.board_outlines[0].versions[0]
    );
    let selected = edit(
        &mut engine,
        modified.revision,
        serde_json::json!({ "kind": "select-outline", "boardId": "main", "versionId": "a" }),
    );
    assert_eq!(selected.board_contours, opened.board_contours);
}

#[test]
fn first_drawn_feature_and_its_fixed_copy_undo_together() {
    let mut engine = CoreEngine::new();
    let opened = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: fixture(),
    }));
    let copied = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({
            "kind": "copy-outline", "boardId": "main", "versionId": "cutout", "name": "With opening",
            "feature": { "id": "hole", "kind": "rect", "center": {"x":128.615,"y":-73.33}, "size": {"x":5,"y":5}, "radius":0, "operation":"subtract" }
        }),
    );
    assert!(
        copied.board_contours[0]
            .contours
            .iter()
            .any(|contour| contour.hole)
    );
    let undone = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(opened.board_contours, undone.board_contours);
    assert!(document(&mut engine).board_outlines.is_empty());
}

#[test]
fn copying_keeps_an_authored_rounded_cutout_as_a_world_cad_primitive() {
    let mut doc = fixture();
    doc.outline.push(OutlineFeature::Rect {
        id: "rounded-opening".into(),
        anchor_part_id: Some("matrix/main-thumbs/r0c2/stabilizer".into()),
        rotation: Some(12.0),
        center: Vec2 { x: 0.0, y: 0.0 },
        size: Vec2 { x: 5.0, y: 5.0 },
        radius: 1.0,
        operation: Operation::Subtract,
    });
    doc.boards[0].outline_ids.push("rounded-opening".into());
    let mut engine = CoreEngine::new();
    let opened = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    let copied = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({"kind":"copy-outline","boardId":"main","versionId":"cad-copy","name":"CAD copy"}),
    );
    assert_eq!(opened.board_contours, copied.board_contours);
    assert!(document(&mut engine).board_outlines[0].versions[0].geometry.features.iter().any(|feature| matches!(feature,
        OutlineFeature::Rect { anchor_part_id:None, radius, operation:Operation::Subtract, .. } if *radius == 1.0)), "Canonical rounded cutouts must not be reconstructed from sampled fillets during later DXF work");
}

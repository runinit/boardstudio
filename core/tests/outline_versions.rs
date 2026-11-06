use boardstudio_core::{CoreEngine, model::*};

fn fixture() -> ProjectDoc {
    let mut document: ProjectDoc =
        serde_json::from_str(include_str!("fixtures/synthetic-outline.json")).unwrap();
    document.format_version = boardstudio_core::migrate::CURRENT_VERSION;
    document
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
        CoreReply::Scene { document, .. } => *document,
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
            // Ticket 37: raw CopyOutline is deleted; Copy is the live typed copy.
            "kind": "apply-outline-edit", "edit": {
                "target": {"kind": "board", "boardId": "main"},
                "change": {"copy": {"version_id": "fixed", "name": "My outline"}},
                "seed": 1, "reviewed": null
            }
        }),
    );
    assert_eq!(original.board_contours, copied.board_contours);
    let moved = edit(
        &mut engine,
        copied.revision,
        serde_json::json!({
            "kind": "move-parts", "positions": [{ "id": "main/U1", "at": { "x": 24.0, "y": 35.0 } }]
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
    // Persist a genuinely invalid inactive fixed copy: its feature follows U1, which a
    // fixed snapshot is forbidden to do. The attachment is entirely inside the board,
    // so it isolates the ownership finding from generated geometry and fabrication fit.
    let mut with_invalid_inactive_copy = serde_json::to_value(document(&mut engine)).unwrap();
    with_invalid_inactive_copy["boardOutlines"][0]["versions"][0]["geometry"]["features"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "kind": "polygon",
            "id": "inactive-live-attachment",
            "anchorPartId": "main/U1",
            "points": [
                { "x": -1.0, "y": -1.0 },
                { "x": 1.0, "y": -1.0 },
                { "x": 0.0, "y": 1.0 }
            ],
            "operation": "add"
        }));
    let mut engine = CoreEngine::new();
    let active_fixed = scene(engine.handle(CoreRequest::Open {
        id: "invalid-inactive-copy".into(),
        document: serde_json::from_value(with_invalid_inactive_copy).unwrap(),
    }));
    let generated = edit(
        &mut engine,
        active_fixed.revision,
        serde_json::json!({
            // Ticket 37: raw SelectOutline is deleted; Activate is the live typed select.
            "kind": "apply-outline-edit", "edit": {
                "target": {"kind": "board", "boardId": "main"},
                "change": "activate", "seed": 2, "reviewed": null
            }
        }),
    );
    assert_ne!(copied.board_contours, generated.board_contours);
    assert!(
        generated.board_readiness[0].outline,
        "Invalid inactive copies cannot block Generated: {:?}",
        generated.findings
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
            "kind": "apply-outline-edit", "edit": {
                "target": {"kind": "board", "boardId": "main"},
                "change": {"copy": {"version_id": "a", "name": "First"}},
                "seed": 1, "reviewed": null
            }
        }),
    );
    let b = edit(
        &mut engine,
        a.revision,
        serde_json::json!({
            "kind": "apply-outline-edit", "edit": {
                "target": {"kind": "board", "boardId": "main"},
                "change": {"copy": {"version_id": "b", "name": "Second"}},
                "seed": 2, "reviewed": null
            }
        }),
    );
    let renamed = edit(
        &mut engine,
        b.revision,
        serde_json::json!({
            // Ticket 37: raw RenameOutline is deleted; Rename is the live typed rename.
            "kind": "apply-outline-edit", "edit": {
                "target": {"kind": "version", "boardId": "main", "versionId": "b"},
                "change": {"rename": {"name": "Alternative"}},
                "seed": 3, "reviewed": null
            }
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
            // Ticket 37: raw RemoveOutline is deleted; Delete is the live typed remove.
            "kind": "apply-outline-edit", "edit": {
                "target": {"kind": "version", "boardId": "main", "versionId": "b"},
                "change": "delete", "seed": 4, "reviewed": null
            }
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
        // Ticket 37: raw CopyOutline-with-edit is deleted; EditGeneratedPerimeter
        // authors the edited copy in one typed edit.
        "kind": "apply-outline-edit", "edit": {
            "target": {"kind": "board", "boardId": "main"},
            "change": {"edit-generated-perimeter": {"contour": 0, "points": points}},
            "seed": 7, "reviewed": null
        }
    }))
    .unwrap();
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
        serde_json::json!({ "kind": "apply-outline-edit", "edit": {
            "target": {"kind": "board", "boardId": "main"},
            "change": {"copy": {"version_id": "a", "name": "First"}},
            "seed": 1, "reviewed": null } }),
    );
    let second = edit(
        &mut engine,
        first.revision,
        serde_json::json!({ "kind": "apply-outline-edit", "edit": {
            "target": {"kind": "board", "boardId": "main"},
            "change": {"copy": {"version_id": "b", "name": "Second"}},
            "seed": 2, "reviewed": null } }),
    );
    let saved = document(&mut engine);
    let mut feature = saved.board_outlines[0].versions[1].geometry.features[0].clone();
    if let OutlineFeature::Polygon { points, .. } = &mut feature {
        points[0].x -= 1.0;
    }
    let modified = edit(
        &mut engine,
        second.revision,
        // Ticket 37: raw SetOutline is deleted; SetFeature on the version is live.
        serde_json::json!({ "kind": "apply-outline-edit", "edit": {
            "target": {"kind": "version", "boardId": "main", "versionId": "b"},
            "change": {"set-feature": {"feature": feature}},
            "seed": 3, "reviewed": null } }),
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
        serde_json::json!({ "kind": "apply-outline-edit", "edit": {
            "target": {"kind": "version", "boardId": "main", "versionId": "a"},
            "change": "activate", "seed": 4, "reviewed": null } }),
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
            // Ticket 37: raw CopyOutline-with-feature is deleted; AddFeature
            // authors the fixed copy with the hole in one typed edit.
            "kind": "apply-outline-edit", "edit": {
                "target": {"kind": "board", "boardId": "main"},
                "change": {"add-feature": {"feature": { "id": "hole", "kind": "polygon",
                    "points": [{"x":-2.5,"y":-2.5},{"x":2.5,"y":-2.5},{"x":2.5,"y":2.5},{"x":-2.5,"y":2.5}],
                    "operation":"subtract" }}},
                "seed": 5, "reviewed": null
            }
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
        anchor_part_id: Some("main/U1".into()),
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
        serde_json::json!({"kind":"apply-outline-edit","edit": {
            "target": {"kind": "board", "boardId": "main"},
            "change": {"copy": {"version_id": "cad-copy", "name": "CAD copy"}},
            "seed": 6, "reviewed": null}}),
    );
    assert_eq!(opened.board_contours, copied.board_contours);
    assert!(document(&mut engine).board_outlines[0].versions[0].geometry.features.iter().any(|feature| matches!(feature,
        OutlineFeature::Rect { anchor_part_id:None, radius, operation:Operation::Subtract, .. } if *radius == 1.0)), "Canonical rounded cutouts must not be reconstructed from sampled fillets during later DXF work");
}

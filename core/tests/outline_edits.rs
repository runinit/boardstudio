//! Typed Core outline edits and atomic version lifecycle (typed Core edits 13).
//!
//! One `apply-outline-edit` operation carries a typed target and named change; Core owns
//! validation, identity allocation from the captured seed, mutation and the single Undo
//! step. Expectations here are hand-derived from the fixture's millimetre geometry
//! (a switch courtyard of 14x14 with margin 1 gives a 16x16 envelope centred on its pose).
use boardstudio_core::outline_edit::{OutlineEdit, OutlineEditDisposition};
use boardstudio_core::{CoreEngine, model::*};

const ENVELOPE: &str = "board:board:version:refined:contour:0";

fn settings() -> serde_json::Value {
    serde_json::json!({
        "corners": "sharp", "size": 2.0, "bridgeWidth": 10.0,
        "repair": {
            "enabled": true, "maximumGapSpan": 20.0,
            "minimumConnectionWidth": 2.0, "edgeClearance": 0.0, "keepGaps": []
        }
    })
}

fn definition() -> serde_json::Value {
    serde_json::json!({
        "id": "switch", "name": "Switch", "kind": "switch",
        "courtyard": [
            { "x": -7.0, "y": -7.0 }, { "x": 7.0, "y": -7.0 },
            { "x": 7.0, "y": 7.0 }, { "x": -7.0, "y": 7.0 }
        ],
        "pads": [{ "id": "p0", "number": "1", "at": { "x": 0.0, "y": 0.0 },
                   "size": { "x": 4.0, "y": 4.0 }, "shape": "rect" }],
    })
}

/// Board "board" owns parts a (origin) and b (14,0); board "spare" owns part c far away.
fn fixture(b_at: (f64, f64)) -> serde_json::Value {
    let mut document = serde_json::to_value(ProjectDoc::empty("edits", "Edits")).unwrap();
    document["definitions"] = serde_json::json!([definition()]);
    document["parts"] = serde_json::json!([
        { "id": "a", "definitionId": "switch", "reference": "SW1", "side": "front",
          "pose": { "at": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 } },
        { "id": "b", "definitionId": "switch", "reference": "SW2", "side": "front",
          "pose": { "at": { "x": b_at.0, "y": b_at.1 }, "rotation": 0.0 } },
        { "id": "c", "definitionId": "switch", "reference": "SW3", "side": "front",
          "pose": { "at": { "x": 100.0, "y": 0.0 }, "rotation": 0.0 } }
    ]);
    document["boards"] = serde_json::json!([
        { "id": "board", "name": "Board", "outlineIds": ["gen"], "partIds": ["a", "b"],
          "netIds": [], "thickness": 1.6 },
        { "id": "spare", "name": "Spare", "outlineIds": ["spare-gen"], "partIds": ["c"],
          "netIds": [], "thickness": 1.6 }
    ]);
    document["outline"] = serde_json::json!([
        { "kind": "part-envelope", "id": "gen", "partIds": ["a", "b"], "margin": 1.0,
          "operation": "add", "settings": settings() },
        { "kind": "part-envelope", "id": "spare-gen", "partIds": ["c"], "margin": 1.0,
          "operation": "add", "settings": settings() }
    ]);
    document
}

fn scene(reply: CoreReply) -> SceneDelta {
    match reply {
        CoreReply::Scene { scene, .. } => scene,
        reply => panic!("{reply:?}"),
    }
}

fn open(b_at: (f64, f64)) -> (CoreEngine, SceneDelta) {
    let mut engine = CoreEngine::new();
    let opened = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: serde_json::from_value(fixture(b_at)).unwrap(),
    }));
    (engine, opened)
}

fn reply(engine: &mut CoreEngine, revision: u64, edit: serde_json::Value) -> CoreReply {
    let operation = serde_json::from_value(serde_json::json!({
        "kind": "apply-outline-edit", "edit": edit
    }))
    .expect("the typed outline operation parses");
    engine.handle(CoreRequest::Edit {
        id: "outline-edit".into(),
        command: EditCommand {
            base_revision: revision,
            transaction_id: "outline-edit".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation,
        },
    })
}

fn edit(engine: &mut CoreEngine, revision: u64, edit: serde_json::Value) -> SceneDelta {
    scene(reply(engine, revision, edit))
}

/// A non-outline operation (for example a component move) submitted at the same seam.
fn raw_edit(engine: &mut CoreEngine, revision: u64, operation: serde_json::Value) -> SceneDelta {
    let operation = serde_json::from_value(operation).expect("the operation parses");
    scene(engine.handle(CoreRequest::Edit {
        id: "raw-edit".into(),
        command: EditCommand {
            base_revision: revision,
            transaction_id: "raw-edit".into(),
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

fn undo(engine: &mut CoreEngine) -> SceneDelta {
    scene(engine.handle(CoreRequest::Undo { id: "undo".into() }))
}

fn redo(engine: &mut CoreEngine) -> SceneDelta {
    scene(engine.handle(CoreRequest::Redo { id: "redo".into() }))
}

fn board_bounds(scene: &SceneDelta, board_id: &str) -> (f64, f64, f64, f64) {
    let contours = scene
        .board_contours
        .iter()
        .find(|board| board.board_id == board_id)
        .map(|board| board.contours.as_slice())
        .unwrap_or(&[]);
    bounds(contours)
}

fn bounds(contours: &[Contour]) -> (f64, f64, f64, f64) {
    contours.iter().flat_map(|contour| &contour.points).fold(
        (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ),
        |(min_x, min_y, max_x, max_y), point| {
            (
                min_x.min(point.x),
                min_y.min(point.y),
                max_x.max(point.x),
                max_y.max(point.y),
            )
        },
    )
}

/// Case 1: named settings, connection and gap changes cross the typed operation and
/// report precise owners instead of a replacement document.
#[test]
fn named_settings_connection_and_gap_edits_report_precise_owners() {
    let (mut engine, opened) = open((14.0, 0.0));
    // Generated settings change: the board and the generated feature it changed.
    let changed = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "set-margin": { "margin": 2.5 } },
            "seed": 7
        }),
    );
    assert_eq!(changed.changed_ids, vec!["board", "gen"]);
    assert_eq!(document(&mut engine).outline[0].part_envelope_margin(), 2.5);

    // A connection is Core-allocated from the seed and reports its own identity.
    let changed = edit(
        &mut engine,
        changed.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "add-connection": { "connection": {
                "id": "outline-connection-9",
                "width": 3.0,
                "points": [
                    { "at": { "x": 0.0, "y": 0.0 } },
                    { "at": { "x": 14.0, "y": 0.0 } }
                ]
            } } },
            "seed": 9
        }),
    );
    assert_eq!(
        changed.changed_ids,
        vec!["board", "gen", "outline-connection-9"]
    );
    let accepted = document(&mut engine);
    assert_eq!(
        accepted.outline[0].connections()[0].id,
        "outline-connection-9"
    );
    assert_eq!(accepted.outline[0].connections()[0].width, 3.0);

    // An authored protected gap reports its own identity and is removable.
    let changed = edit(
        &mut engine,
        changed.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "set-protected-gap": { "gap_id": "gap-1", "points": [
                { "at": { "x": -6.0, "y": -6.0 } },
                { "at": { "x": 6.0, "y": -6.0 } },
                { "at": { "x": 6.0, "y": 6.0 } }
            ] } },
            "seed": 11
        }),
    );
    assert_eq!(changed.changed_ids, vec!["board", "gap-1", "gen"]);
    let accepted = document(&mut engine);
    assert!(
        accepted.outline[0]
            .keep_gap_ids()
            .contains(&"gap-1".to_owned())
    );

    let removed = edit(
        &mut engine,
        changed.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "remove-protected-gap": { "gap_id": "gap-1" } },
            "seed": 12
        }),
    );
    assert_eq!(removed.changed_ids, vec!["board", "gap-1", "gen"]);
    assert!(document(&mut engine).outline[0].keep_gap_ids().is_empty());
}

/// Case 3: equal values settle without a revision step and a no-op keeps redo intact.
#[test]
fn equal_values_settle_unchanged_and_preserve_redo() {
    let (mut engine, opened) = open((14.0, 0.0));
    let committed = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "set-margin": { "margin": 2.5 } },
            "seed": 7
        }),
    );
    let undone = undo(&mut engine);
    assert_ne!(committed.revision, undone.revision);
    // Generated is already active: this typed activation changes nothing.
    let unchanged = edit(
        &mut engine,
        undone.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": "activate",
            "seed": 7
        }),
    );
    assert!(unchanged.changed_ids.is_empty());
    assert_eq!(unchanged.revision, undone.revision, "no revision step");
    let restored = redo(&mut engine);
    assert_eq!(restored.revision, undone.revision + 1);
    assert_eq!(document(&mut engine).outline[0].part_envelope_margin(), 2.5);

    // An equal settings value is also Unchanged, through the shared evaluation rule.
    let accepted = document(&mut engine);
    let disposition = boardstudio_core::outline_edit::evaluate(
        &accepted,
        &OutlineEdit {
            target: boardstudio_core::outline_edit::OutlineEditTarget::Board {
                board_id: "board".into(),
            },
            change: boardstudio_core::outline_edit::OutlineChange::SetMargin { margin: 2.5 },
            seed: 7,
            reviewed: None,
        },
    )
    .unwrap();
    assert_eq!(disposition, OutlineEditDisposition::Unchanged);
}

/// Case 3: malformed submitted geometry fails with a Core error and changes nothing.
#[test]
fn malformed_polygon_fails_without_mutation() {
    let (mut engine, opened) = open((14.0, 0.0));
    let before = serde_json::to_value(document(&mut engine)).unwrap();
    let failed = reply(
        &mut engine,
        opened.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "add-feature": { "feature": {
                "kind": "polygon", "id": "drawn",
                "points": [
                    { "x": 0.0, "y": 0.0 }, { "x": 10.0, "y": 0.0 }
                ],
                "operation": "add"
            } } },
            "seed": 5
        }),
    );
    let CoreReply::Error {
        message, revision, ..
    } = failed
    else {
        panic!("malformed geometry must fail");
    };
    assert!(message.contains("three finite points"), "{message}");
    assert_eq!(revision, opened.revision);
    let after = serde_json::to_value(document(&mut engine)).unwrap();
    assert_eq!(before, after, "a malformed edit changes nothing");
}

/// Case 5: new identities avoid persisted collisions, and a removed identity can never
/// redirect a pending edit at its replacement.
#[test]
fn new_ids_avoid_collisions_and_removed_ids_never_redirect() {
    let (mut engine, opened) = open((14.0, 0.0));
    let created = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "create-linked-refinement": {
                "version_id": "refined", "name": "Refined"
            } },
            "seed": 4
        }),
    );
    // The requested id already names the version's own envelope feature: Core allocates.
    let added = edit(
        &mut engine,
        created.revision,
        serde_json::json!({
            "target": { "kind": "version", "boardId": "board", "versionId": "refined" },
            "change": { "add-feature": { "feature": {
                "kind": "polygon", "id": ENVELOPE,
                "points": [
                    { "x": 40.0, "y": 40.0 }, { "x": 50.0, "y": 40.0 },
                    { "x": 50.0, "y": 50.0 }, { "x": 40.0, "y": 50.0 }
                ],
                "operation": "add"
            } } },
            "seed": 11
        }),
    );
    assert_eq!(
        added.changed_ids,
        vec!["board", "outline-feature-11", "refined"]
    );
    let accepted = document(&mut engine);
    let version = accepted
        .board_outlines
        .iter()
        .find(|state| state.board_id == "board")
        .and_then(|state| {
            state
                .versions
                .iter()
                .find(|version| version.id == "refined")
        })
        .unwrap();
    assert!(
        version
            .geometry
            .features
            .iter()
            .any(|feature| feature.id() == "outline-feature-11"),
        "the collision was allocated a free identity"
    );

    // A queued edit for the removed version is never redirected at its replacement.
    edit(
        &mut engine,
        added.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "copy": { "version_id": "copy", "name": "Copy" } },
            "seed": 20
        }),
    );
    let accepted = document(&mut engine);
    let pending = OutlineEdit {
        target: boardstudio_core::outline_edit::OutlineEditTarget::Version {
            board_id: "board".into(),
            version_id: "copy".into(),
        },
        change: boardstudio_core::outline_edit::OutlineChange::SetBridgeWidth { bridge_width: 4.0 },
        seed: 21,
        reviewed: None,
    };
    let before = serde_json::to_value(&accepted).unwrap();
    assert_eq!(
        boardstudio_core::outline_edit::evaluate(&accepted, &pending).unwrap(),
        OutlineEditDisposition::Changed
    );
    let deleted = edit(
        &mut engine,
        accepted.revision,
        serde_json::json!({
            "target": { "kind": "version", "boardId": "board", "versionId": "copy" },
            "change": "delete",
            "seed": 22
        }),
    );
    let after = document(&mut engine);
    let refined_before = serde_json::to_value(
        accepted
            .board_outlines
            .iter()
            .find(|state| state.board_id == "board")
            .and_then(|state| {
                state
                    .versions
                    .iter()
                    .find(|version| version.id == "refined")
            })
            .unwrap(),
    )
    .unwrap();
    let refined_after = serde_json::to_value(
        after
            .board_outlines
            .iter()
            .find(|state| state.board_id == "board")
            .and_then(|state| {
                state
                    .versions
                    .iter()
                    .find(|version| version.id == "refined")
            })
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        refined_before, refined_after,
        "removing another version changes no other version"
    );
    assert!(
        after
            .board_outlines
            .iter()
            .find(|state| state.board_id == "board")
            .unwrap()
            .retired_version_ids
            .contains(&"copy".to_owned())
    );
    assert_eq!(
        boardstudio_core::outline_edit::evaluate(&after, &pending).unwrap(),
        OutlineEditDisposition::Retire
    );
    // Recreating the removed identity is refused, so nothing can redirect the intent.
    let refused = reply(
        &mut engine,
        deleted.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "copy": { "version_id": "copy", "name": "Copy again" } },
            "seed": 23
        }),
    );
    let CoreReply::Error { message, .. } = refused else {
        panic!("a retired identity must stay reserved");
    };
    assert!(message.contains("stays reserved"), "{message}");
    let _ = before;
}

/// Case 4: a reviewed generated-shape edit retires when the relevant source geometry
/// changed, while unrelated metadata or another board's edit does not.
#[test]
fn a_relevant_generated_shape_change_retires_a_reviewed_edit() {
    let (mut engine, opened) = open((14.0, 0.0));
    let scene = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }));
    let reviewed = scene
        .board_outline_scenes
        .iter()
        .find(|scene| scene.board_id == "board")
        .unwrap()
        .source_contours
        .clone();
    let pending = OutlineEdit {
        target: boardstudio_core::outline_edit::OutlineEditTarget::Board {
            board_id: "board".into(),
        },
        change: boardstudio_core::outline_edit::OutlineChange::EditGeneratedPerimeter {
            contour: 0,
            points: vec![
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 { x: 5.0, y: 0.0 },
                Vec2 { x: 5.0, y: 5.0 },
            ],
        },
        seed: 30,
        reviewed: Some(reviewed),
    };
    let accepted = document(&mut engine);
    assert_eq!(
        boardstudio_core::outline_edit::evaluate(&accepted, &pending).unwrap(),
        OutlineEditDisposition::Changed
    );

    // Unrelated metadata on another board does not retire it.
    let unrelated = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "spare" },
            "change": { "set-margin": { "margin": 4.0 } },
            "seed": 31
        }),
    );
    let accepted = document(&mut engine);
    assert_eq!(
        boardstudio_core::outline_edit::evaluate(&accepted, &pending).unwrap(),
        OutlineEditDisposition::Changed
    );

    // A relevant component move changes the reviewed generated shape.
    raw_edit(
        &mut engine,
        unrelated.revision,
        serde_json::json!({
            "kind": "move-parts",
            "positions": [{ "id": "b", "at": { "x": 20.0, "y": 0.0 } }]
        }),
    );
    let accepted = document(&mut engine);
    assert_eq!(
        boardstudio_core::outline_edit::evaluate(&accepted, &pending).unwrap(),
        OutlineEditDisposition::Retire
    );
}

/// Case 2: copy/edit/activate of the generated perimeter is one accepted edit and one
/// Undo step.
#[test]
fn copy_edit_and_activate_is_one_operation_one_undo() {
    let (mut engine, opened) = open((14.0, 0.0));
    let committed = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "edit-generated-perimeter": { "contour": 0, "points": [
                { "x": -10.0, "y": -10.0 }, { "x": 30.0, "y": -10.0 },
                { "x": 30.0, "y": 10.0 }, { "x": -10.0, "y": 10.0 }
            ] } },
            "seed": 5
        }),
    );
    assert_eq!(committed.revision, opened.revision + 1);
    assert_eq!(committed.changed_ids, vec!["board", "outline-version-5"]);
    let accepted = document(&mut engine);
    let state = &accepted.board_outlines[0];
    assert_eq!(
        state.active_version_id.as_deref(),
        Some("outline-version-5")
    );
    let version = state
        .versions
        .iter()
        .find(|v| v.id == "outline-version-5")
        .unwrap();
    assert!(version.ownership.is_fixed());
    assert_eq!(
        bounds(&[Contour::from_feature(&version.geometry.features[0])]).0,
        -10.0
    );

    let undone = undo(&mut engine);
    assert!(
        document(&mut engine)
            .board_outlines
            .iter()
            .all(|state| state.versions.is_empty()),
        "one Undo removes the created version"
    );
    let redone = redo(&mut engine);
    assert_eq!(redone.revision, undone.revision + 1);
    assert_eq!(
        document(&mut engine).board_outlines[0]
            .active_version_id
            .as_deref(),
        Some("outline-version-5")
    );
    assert_eq!(redone.contours, committed.contours);
}

/// Case 2: creating a linked refinement and selecting it Undo atomically; Freeze retains
/// the link and produces a fixed selected copy.
#[test]
fn linked_refinement_creation_and_freeze_are_atomic() {
    let (mut engine, opened) = open((14.0, 0.0));
    let created = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "create-linked-refinement": {
                "version_id": "refined", "name": "Refined"
            } },
            "seed": 4
        }),
    );
    assert_eq!(created.changed_ids, vec!["board", "refined"]);
    let accepted = document(&mut engine);
    let state = &accepted.board_outlines[0];
    assert_eq!(state.active_version_id.as_deref(), Some("refined"));
    let linked = state.versions.iter().find(|v| v.id == "refined").unwrap();
    assert!(!linked.ownership.is_fixed(), "the refinement stays linked");
    let linked_before = board_bounds(&created, "board");

    // The link follows its source.
    let moved = raw_edit(
        &mut engine,
        created.revision,
        serde_json::json!({
            "kind": "move-parts",
            "positions": [{ "id": "b", "at": { "x": 22.0, "y": 0.0 } }]
        }),
    );
    let linked_after = board_bounds(&moved, "board");
    assert!(
        linked_after.2 > linked_before.2,
        "the refinement follows the moved source"
    );

    // Freeze copies the current resolved projection and retains the linked version.
    let frozen = edit(
        &mut engine,
        moved.revision,
        serde_json::json!({
            "target": { "kind": "version", "boardId": "board", "versionId": "refined" },
            "change": { "freeze": { "name": "Frozen" } },
            "seed": 6
        }),
    );
    assert_eq!(frozen.changed_ids, vec!["board", "outline-version-6"]);
    let accepted = document(&mut engine);
    let state = &accepted.board_outlines[0];
    assert_eq!(
        state.active_version_id.as_deref(),
        Some("outline-version-6")
    );
    let linked = state.versions.iter().find(|v| v.id == "refined").unwrap();
    assert!(
        !linked.ownership.is_fixed(),
        "freeze retains the linked refinement"
    );
    let fixed = state
        .versions
        .iter()
        .find(|v| v.id == "outline-version-6")
        .unwrap();
    assert!(fixed.ownership.is_fixed());
    let fixed_before = serde_json::to_value(&fixed.geometry).unwrap();

    // Moving the source again reshapes the linked refinement, not the frozen copy.
    raw_edit(
        &mut engine,
        frozen.revision,
        serde_json::json!({
            "kind": "move-parts",
            "positions": [{ "id": "b", "at": { "x": 14.0, "y": 0.0 } }]
        }),
    );
    let accepted = document(&mut engine);
    let fixed = accepted.board_outlines[0]
        .versions
        .iter()
        .find(|v| v.id == "outline-version-6")
        .unwrap();
    assert_eq!(
        serde_json::to_value(&fixed.geometry).unwrap(),
        fixed_before,
        "the fixed edited copy keeps its geometry"
    );

    let _undone = undo(&mut engine);
}

/// Case 6: geometry-affecting edits refresh contours while metadata-only naming does
/// not, and Undo/Redo returns ownership and geometry exactly.
#[test]
fn geometry_edits_refresh_contours_and_undo_restores_exactly() {
    let (mut engine, opened) = open((14.0, 0.0));
    let created = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "create-linked-refinement": {
                "version_id": "refined", "name": "Refined"
            } },
            "seed": 4
        }),
    );
    let before_document = serde_json::to_value(document(&mut engine)).unwrap();
    let before_contours = board_bounds(&created, "board");

    let renamed = edit(
        &mut engine,
        created.revision,
        serde_json::json!({
            "target": { "kind": "version", "boardId": "board", "versionId": "refined" },
            "change": { "rename": { "name": "Renamed" } },
            "seed": 4
        }),
    );
    assert_eq!(renamed.changed_ids, vec!["board", "refined"]);
    assert_eq!(
        renamed.contours, created.contours,
        "naming preserves the geometry cache"
    );
    let renamed_document = serde_json::to_value(document(&mut engine)).unwrap();
    assert_eq!(
        renamed_document["boardOutlines"][0]["versions"][0]["geometry"],
        before_document["boardOutlines"][0]["versions"][0]["geometry"],
        "naming changes no geometry"
    );

    let grown = edit(
        &mut engine,
        renamed.revision,
        serde_json::json!({
            "target": { "kind": "version", "boardId": "board", "versionId": "refined" },
            "change": { "set-margin": { "margin": 5.0 } },
            "seed": 4
        }),
    );
    assert_ne!(
        grown.contours, created.contours,
        "a geometry edit refreshes contours"
    );
    assert!(
        board_bounds(&grown, "board").0 < before_contours.0,
        "the larger margin grows the resolved boundary"
    );
    let grown_document = serde_json::to_value(document(&mut engine)).unwrap();

    undo(&mut engine);
    let after_first_undo = serde_json::to_value(document(&mut engine)).unwrap();
    assert_eq!(
        after_first_undo["boardOutlines"], renamed_document["boardOutlines"],
        "one Undo returns the accepted geometry"
    );
    undo(&mut engine);
    let after_second_undo = serde_json::to_value(document(&mut engine)).unwrap();
    assert_eq!(
        after_second_undo["boardOutlines"], before_document["boardOutlines"],
        "Undo returns ownership and geometry exactly"
    );
    let redone = redo(&mut engine);
    assert_eq!(redone.contours, created.contours);
    let redone_document = serde_json::to_value(document(&mut engine)).unwrap();
    assert_eq!(
        redone_document["boardOutlines"][0]["versions"][0]["geometry"],
        renamed_document["boardOutlines"][0]["versions"][0]["geometry"],
        "Redo returns the accepted geometry"
    );
    let redone_again = redo(&mut engine);
    assert_eq!(redone_again.contours, grown.contours);
    assert_eq!(
        serde_json::to_value(document(&mut engine)).unwrap()["boardOutlines"][0]["versions"][0]["geometry"],
        grown_document["boardOutlines"][0]["versions"][0]["geometry"],
        "Redo returns the edited geometry"
    );
}

trait FixtureAccess {
    fn part_envelope_margin(&self) -> f64;
    fn connections(&self) -> Vec<OutlineConnection>;
    fn keep_gap_ids(&self) -> Vec<String>;
}

impl FixtureAccess for OutlineFeature {
    fn part_envelope_margin(&self) -> f64 {
        match self {
            OutlineFeature::PartEnvelope { margin, .. } => *margin,
            _ => panic!("expected the generated envelope"),
        }
    }
    fn connections(&self) -> Vec<OutlineConnection> {
        match self {
            OutlineFeature::PartEnvelope { connections, .. } => connections.clone(),
            _ => panic!("expected the generated envelope"),
        }
    }
    fn keep_gap_ids(&self) -> Vec<String> {
        match self {
            OutlineFeature::PartEnvelope { settings, .. } => settings
                .repair
                .iter()
                .flat_map(|repair| repair.keep_gaps.iter().map(|gap| gap.id.clone()))
                .collect(),
            _ => panic!("expected the generated envelope"),
        }
    }
}

trait ContourFromFeature {
    fn from_feature(feature: &OutlineFeature) -> Contour;
}

impl ContourFromFeature for Contour {
    fn from_feature(feature: &OutlineFeature) -> Contour {
        match feature {
            OutlineFeature::Polygon { points, .. } => Contour {
                points: points.clone(),
                hole: false,
            },
            other => panic!("expected an authored polygon, got {other:?}"),
        }
    }
}

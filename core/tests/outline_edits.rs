//! Typed Core outline edits and atomic version lifecycle (typed Core edits 13).
//!
//! One `apply-outline-edit` operation carries a typed target and named change; Core owns
//! validation, identity allocation from the captured seed, mutation and the single Undo
//! step. Expectations here are hand-derived from the fixture's millimetre geometry
//! (a switch courtyard of 14x14 with margin 1 gives a 16x16 envelope centred on its pose).
use boardstudio_core::outline_edit::{
    OutlineChange, OutlineEdit, OutlineEditDisposition, OutlineEditTarget, evaluate,
};
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

/// Board "board" with no Generated envelope at all (the CreateGenerated starting state).
fn fixture_without_generated() -> serde_json::Value {
    let mut document = fixture((14.0, 0.0));
    document["outline"] = serde_json::json!([{
        "kind": "part-envelope", "id": "spare-gen", "partIds": ["c"], "margin": 1.0,
        "operation": "add", "settings": settings()
    }]);
    document["boards"][0]["outlineIds"] = serde_json::json!([]);
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

fn board_state(doc: &ProjectDoc) -> Option<&BoardOutline> {
    doc.board_outlines
        .iter()
        .find(|state| state.board_id == "board")
}

fn version_of<'a>(doc: &'a ProjectDoc, id: &str) -> Option<&'a OutlineVersion> {
    board_state(doc)?
        .versions
        .iter()
        .find(|version| version.id == id)
}

fn create_refined(engine: &mut CoreEngine, revision: u64) -> SceneDelta {
    edit(
        engine,
        revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "create-linked-refinement": {
                "version_id": "refined", "name": "Refined"
            } },
            "seed": 4
        }),
    )
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids
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

/// Gap 2: a queued edit addressing a LINKED version reviews that version's resolved
/// source geometry. When the relevant source changes before execution the edit retires;
/// unrelated boards and metadata do not retire it. Never whole-document revision
/// equality.
#[test]
fn a_reviewed_linked_version_edit_retires_when_its_source_changes() {
    let (mut engine, opened) = open((14.0, 0.0));
    let created = create_refined(&mut engine, opened.revision);
    // Authored version-local geometry: the reviewed projection is this version's own
    // resolved shape, which now differs from the board's generated source.
    let authored = edit(
        &mut engine,
        created.revision,
        serde_json::json!({
            "target": { "kind": "version", "boardId": "board", "versionId": "refined" },
            "change": { "add-feature": { "feature": {
                "kind": "polygon", "id": "cutout-1",
                "points": [
                    { "x": -3.0, "y": -3.0 }, { "x": 3.0, "y": -3.0 },
                    { "x": 3.0, "y": 3.0 }, { "x": -3.0, "y": 3.0 }
                ],
                "operation": "subtract"
            } } },
            "seed": 43
        }),
    );
    let reviewed = authored
        .board_outline_scenes
        .iter()
        .find(|scene| scene.board_id == "board")
        .unwrap()
        .source_contours
        .clone();
    assert!(
        reviewed.iter().any(|contour| contour.hole),
        "the reviewed projection carries the authored cutout"
    );
    let pending = OutlineEdit {
        target: OutlineEditTarget::Version {
            board_id: "board".into(),
            version_id: "refined".into(),
        },
        change: OutlineChange::EditFeaturePerimeter {
            feature_id: ENVELOPE.into(),
            points: vec![
                Vec2 { x: -10.0, y: -10.0 },
                Vec2 { x: 30.0, y: -10.0 },
                Vec2 { x: 30.0, y: 10.0 },
                Vec2 { x: -10.0, y: 10.0 },
            ],
        },
        seed: 40,
        reviewed: Some(reviewed.clone()),
    };
    let accepted = document(&mut engine);
    assert_eq!(
        evaluate(&accepted, &pending).unwrap(),
        OutlineEditDisposition::Changed,
        "the reviewed linked version still resolves to what was reviewed"
    );

    // Another board's settings edit and a metadata-only rename do not retire it.
    let unrelated = edit(
        &mut engine,
        authored.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "spare" },
            "change": { "set-margin": { "margin": 4.0 } },
            "seed": 41
        }),
    );
    let accepted = document(&mut engine);
    assert_eq!(
        evaluate(&accepted, &pending).unwrap(),
        OutlineEditDisposition::Changed
    );

    // A relevant component move changes the linked version's resolved source.
    raw_edit(
        &mut engine,
        unrelated.revision,
        serde_json::json!({
            "kind": "move-parts",
            "positions": [{ "id": "b", "at": { "x": 22.0, "y": 0.0 } }]
        }),
    );
    let accepted = document(&mut engine);
    assert_eq!(
        evaluate(&accepted, &pending).unwrap(),
        OutlineEditDisposition::Retire,
        "the moved source retires the queued linked-version edit"
    );
}

/// Gap 2: removing a source component that the linked version resolves from also
/// retires the queued edit.
#[test]
fn a_removed_source_component_retires_a_reviewed_linked_version_edit() {
    let (mut engine, opened) = open((14.0, 0.0));
    let created = create_refined(&mut engine, opened.revision);
    let reviewed = created
        .board_outline_scenes
        .iter()
        .find(|scene| scene.board_id == "board")
        .unwrap()
        .source_contours
        .clone();
    let pending = OutlineEdit {
        target: OutlineEditTarget::Version {
            board_id: "board".into(),
            version_id: "refined".into(),
        },
        change: OutlineChange::AddFeature {
            feature: OutlineFeature::Polygon {
                id: "queued-drawing".into(),
                points: vec![
                    Vec2 { x: 40.0, y: 40.0 },
                    Vec2 { x: 50.0, y: 40.0 },
                    Vec2 { x: 50.0, y: 50.0 },
                ],
                anchor_part_id: None,
                operation: Operation::Add,
            },
        },
        seed: 42,
        reviewed: Some(reviewed),
    };
    let accepted = document(&mut engine);
    assert_eq!(
        evaluate(&accepted, &pending).unwrap(),
        OutlineEditDisposition::Changed
    );

    raw_edit(
        &mut engine,
        created.revision,
        serde_json::json!({ "kind": "remove-parts", "ids": ["b"] }),
    );
    let accepted = document(&mut engine);
    assert_eq!(
        evaluate(&accepted, &pending).unwrap(),
        OutlineEditDisposition::Retire
    );
}

/// Gap 4: two ordinary field edits queued in order both settle in submission order with
/// precise owners (ticket case 4's explicit clause).
#[test]
fn two_ordinary_field_edits_settle_in_submission_order() {
    let (mut engine, opened) = open((14.0, 0.0));
    let first = OutlineEdit {
        target: OutlineEditTarget::Board {
            board_id: "board".into(),
        },
        change: OutlineChange::SetMargin { margin: 3.0 },
        seed: 50,
        reviewed: None,
    };
    let second = OutlineEdit {
        target: OutlineEditTarget::Board {
            board_id: "board".into(),
        },
        change: OutlineChange::SetMargin { margin: 4.0 },
        seed: 51,
        reviewed: None,
    };
    let accepted = document(&mut engine);
    assert_eq!(
        evaluate(&accepted, &first).unwrap(),
        OutlineEditDisposition::Changed
    );
    assert_eq!(
        evaluate(&accepted, &second).unwrap(),
        OutlineEditDisposition::Changed,
        "the later queued value is still a change against the older accepted state"
    );

    let after_first = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "set-margin": { "margin": 3.0 } },
            "seed": 50
        }),
    );
    assert_eq!(after_first.changed_ids, vec!["board", "gen"]);
    assert_eq!(document(&mut engine).outline[0].part_envelope_margin(), 3.0);

    let accepted = document(&mut engine);
    assert_eq!(
        evaluate(&accepted, &second).unwrap(),
        OutlineEditDisposition::Changed
    );
    let after_second = edit(
        &mut engine,
        after_first.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "set-margin": { "margin": 4.0 } },
            "seed": 51
        }),
    );
    assert_eq!(after_second.changed_ids, vec!["board", "gen"]);
    assert_eq!(document(&mut engine).outline[0].part_envelope_margin(), 4.0);
}

/// Gaps 3 + 6: the version-target feature-perimeter variant edits a fixed version in
/// place and produces a fixed edited copy for a linked refinement (retaining the link),
/// and version-target protected gaps are authored and mirrored into the geometry.
#[test]
fn version_feature_perimeter_and_protected_gaps_are_covered_directly() {
    let (mut engine, opened) = open((14.0, 0.0));
    let created = create_refined(&mut engine, opened.revision);
    let copied = edit(
        &mut engine,
        created.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "copy": { "version_id": "fixed", "name": "Fixed" } },
            "seed": 60
        }),
    );
    assert_eq!(copied.changed_ids, vec!["board", "fixed"]);
    let accepted = document(&mut engine);
    assert_eq!(
        board_state(&accepted).unwrap().active_version_id.as_deref(),
        Some("fixed")
    );
    let fixed_feature = version_of(&accepted, "fixed").unwrap().geometry.features[0]
        .id()
        .to_owned();
    let refined_before = serde_json::to_value(version_of(&accepted, "refined").unwrap()).unwrap();

    let ring = serde_json::json!([
        { "x": -10.0, "y": -10.0 }, { "x": 30.0, "y": -10.0 },
        { "x": 30.0, "y": 10.0 }, { "x": -10.0, "y": 10.0 }
    ]);
    let edited = edit(
        &mut engine,
        copied.revision,
        serde_json::json!({
            "target": { "kind": "version", "boardId": "board", "versionId": "fixed" },
            "change": { "edit-feature-perimeter": {
                "feature_id": fixed_feature, "points": ring
            } },
            "seed": 61
        }),
    );
    assert_eq!(
        edited.changed_ids,
        sorted(vec!["board".into(), fixed_feature.clone(), "fixed".into()])
    );
    let accepted = document(&mut engine);
    let fixed = version_of(&accepted, "fixed").unwrap();
    assert!(fixed.ownership.is_fixed());
    assert_eq!(
        Contour::from_feature(&fixed.geometry.features[0]).points[0],
        Vec2 { x: -10.0, y: -10.0 },
        "a fixed version is edited in place"
    );
    assert_eq!(
        serde_json::to_value(version_of(&accepted, "refined").unwrap()).unwrap(),
        refined_before,
        "editing one version leaves the other's choices alone"
    );

    // An equal perimeter re-submission is Unchanged.
    let pending = OutlineEdit {
        target: OutlineEditTarget::Version {
            board_id: "board".into(),
            version_id: "fixed".into(),
        },
        change: OutlineChange::EditFeaturePerimeter {
            feature_id: fixed_feature.clone(),
            points: vec![
                Vec2 { x: -10.0, y: -10.0 },
                Vec2 { x: 30.0, y: -10.0 },
                Vec2 { x: 30.0, y: 10.0 },
                Vec2 { x: -10.0, y: 10.0 },
            ],
        },
        seed: 61,
        reviewed: None,
    };
    assert_eq!(
        evaluate(&accepted, &pending).unwrap(),
        OutlineEditDisposition::Unchanged
    );

    // Manual perimeter editing of a linked refinement freezes a fixed copy and keeps
    // the link.
    let activated = edit(
        &mut engine,
        edited.revision,
        serde_json::json!({
            "target": { "kind": "version", "boardId": "board", "versionId": "refined" },
            "change": "activate",
            "seed": 62
        }),
    );
    let linked_ring = serde_json::json!([
        { "x": -12.0, "y": -12.0 }, { "x": 32.0, "y": -12.0 },
        { "x": 32.0, "y": 12.0 }, { "x": -12.0, "y": 12.0 }
    ]);
    let frozen = edit(
        &mut engine,
        activated.revision,
        serde_json::json!({
            "target": { "kind": "version", "boardId": "board", "versionId": "refined" },
            "change": { "edit-feature-perimeter": {
                "feature_id": ENVELOPE, "points": linked_ring
            } },
            "seed": 63
        }),
    );
    assert_eq!(frozen.changed_ids.len(), 2);
    let new_id = frozen
        .changed_ids
        .iter()
        .find(|id| *id != "board")
        .unwrap()
        .clone();
    let accepted = document(&mut engine);
    let state = board_state(&accepted).unwrap();
    assert_eq!(state.versions.len(), 3);
    assert_eq!(state.active_version_id.as_deref(), Some(new_id.as_str()));
    let linked = version_of(&accepted, "refined").unwrap();
    assert!(
        !linked.ownership.is_fixed(),
        "the linked refinement keeps its link"
    );
    let copy = version_of(&accepted, &new_id).unwrap();
    assert!(copy.ownership.is_fixed());
    assert_eq!(
        Contour::from_feature(&copy.geometry.features[0]).points[0],
        Vec2 { x: -12.0, y: -12.0 }
    );

    // Version-target protected gaps: authored on the version's settings and mirrored
    // into its geometry; an identical gap is Unchanged and removal clears both homes.
    // The gap edit addresses the linked refinement, so it is selected again first.
    let reactivated = edit(
        &mut engine,
        frozen.revision,
        serde_json::json!({
            "target": { "kind": "version", "boardId": "board", "versionId": "refined" },
            "change": "activate",
            "seed": 72
        }),
    );
    let gap_points = serde_json::json!([
        { "at": { "x": -6.0, "y": -6.0 } },
        { "at": { "x": 6.0, "y": -6.0 } },
        { "at": { "x": 6.0, "y": 6.0 } }
    ]);
    let accepted = document(&mut engine);
    let gap_pending = OutlineEdit {
        target: OutlineEditTarget::Version {
            board_id: "board".into(),
            version_id: "refined".into(),
        },
        change: OutlineChange::SetProtectedGap {
            gap_id: "gap-v1".into(),
            points: vec![
                OutlineControlPoint {
                    at: Vec2 { x: -6.0, y: -6.0 },
                    part_id: None,
                },
                OutlineControlPoint {
                    at: Vec2 { x: 6.0, y: -6.0 },
                    part_id: None,
                },
                OutlineControlPoint {
                    at: Vec2 { x: 6.0, y: 6.0 },
                    part_id: None,
                },
            ],
        },
        seed: 70,
        reviewed: None,
    };
    assert_eq!(
        evaluate(&accepted, &gap_pending).unwrap(),
        OutlineEditDisposition::Changed
    );
    let added = edit(
        &mut engine,
        reactivated.revision,
        serde_json::json!({
            "target": { "kind": "version", "boardId": "board", "versionId": "refined" },
            "change": { "set-protected-gap": { "gap_id": "gap-v1", "points": gap_points } },
            "seed": 70
        }),
    );
    assert_eq!(
        added.changed_ids,
        sorted(vec!["board".into(), "gap-v1".into(), "refined".into()])
    );
    let accepted = document(&mut engine);
    let linked = version_of(&accepted, "refined").unwrap();
    assert!(
        linked
            .geometry
            .settings
            .repair
            .as_ref()
            .is_some_and(|repair| repair.keep_gaps.iter().any(|gap| gap.id == "gap-v1"))
    );
    assert!(
        linked
            .geometry
            .protected_gaps
            .iter()
            .any(|gap| gap.id == "gap-v1"),
        "the authored gap is mirrored into the version geometry"
    );
    assert_eq!(
        evaluate(&accepted, &gap_pending).unwrap(),
        OutlineEditDisposition::Unchanged,
        "an identical gap re-submission is a no-op"
    );

    let removed = edit(
        &mut engine,
        added.revision,
        serde_json::json!({
            "target": { "kind": "version", "boardId": "board", "versionId": "refined" },
            "change": { "remove-protected-gap": { "gap_id": "gap-v1" } },
            "seed": 71
        }),
    );
    assert_eq!(
        removed.changed_ids,
        sorted(vec!["board".into(), "gap-v1".into(), "refined".into()])
    );
    let accepted = document(&mut engine);
    let linked = version_of(&accepted, "refined").unwrap();
    assert!(
        linked
            .geometry
            .settings
            .repair
            .as_ref()
            .is_none_or(|repair| repair.keep_gaps.is_empty())
    );
    assert!(linked.geometry.protected_gaps.is_empty());
}

/// Gap 7: a non-positive or non-finite submitted connection width is malformed input and
/// must error with no mutation, not silently default to the bridge width.
#[test]
fn a_non_positive_connection_width_is_malformed_input() {
    let (mut engine, opened) = open((14.0, 0.0));
    let before = serde_json::to_value(document(&mut engine)).unwrap();
    for width in [0.0, -2.5] {
        let failed = reply(
            &mut engine,
            opened.revision,
            serde_json::json!({
                "target": { "kind": "board", "boardId": "board" },
                "change": { "add-connection": { "connection": {
                    "id": "c1", "width": width,
                    "points": [
                        { "at": { "x": 0.0, "y": 0.0 } },
                        { "at": { "x": 14.0, "y": 0.0 } }
                    ]
                } } },
                "seed": 80
            }),
        );
        let CoreReply::Error {
            message, revision, ..
        } = failed
        else {
            panic!("a non-positive connection width must fail ({width})");
        };
        assert!(message.contains("width"), "{message}");
        assert_eq!(revision, opened.revision);
    }
    assert_eq!(
        serde_json::to_value(document(&mut engine)).unwrap(),
        before,
        "a malformed connection changes nothing"
    );

    let committed = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "add-connection": { "connection": {
                "id": "c1", "width": 3.0,
                "points": [
                    { "at": { "x": 0.0, "y": 0.0 } },
                    { "at": { "x": 14.0, "y": 0.0 } }
                ]
            } } },
            "seed": 80
        }),
    );
    assert_eq!(
        committed.changed_ids,
        sorted(vec!["board".into(), "c1".into(), "gen".into()])
    );
    assert_eq!(document(&mut engine).outline[0].connections()[0].width, 3.0);
}

/// Gap 8: Core-owned default version names never duplicate the board's existing names,
/// using the old planner's incrementing rule for every creation path.
#[test]
fn core_default_version_names_never_duplicate() {
    let (mut engine, opened) = open((14.0, 0.0));
    let drawing = serde_json::json!([
        { "x": 40.0, "y": 40.0 }, { "x": 50.0, "y": 40.0 },
        { "x": 50.0, "y": 50.0 }, { "x": 40.0, "y": 50.0 }
    ]);
    let first = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "add-feature": { "feature": {
                "kind": "polygon", "id": "drawn-1", "points": drawing,
                "operation": "add"
            } } },
            "seed": 11
        }),
    );
    assert_eq!(
        first.changed_ids,
        vec!["board", "drawn-1", "outline-version-11"]
    );
    // Back to Generated, then a second default-named creation and a perimeter copy.
    let generated = edit(
        &mut engine,
        first.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": "activate",
            "seed": 12
        }),
    );
    let second = edit(
        &mut engine,
        generated.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "add-feature": { "feature": {
                "kind": "polygon", "id": "drawn-2", "points": drawing,
                "operation": "add"
            } } },
            "seed": 11
        }),
    );
    let generated = edit(
        &mut engine,
        second.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": "activate",
            "seed": 13
        }),
    );
    let third = edit(
        &mut engine,
        generated.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "edit-generated-perimeter": { "contour": 0, "points": [
                { "x": -10.0, "y": -10.0 }, { "x": 30.0, "y": -10.0 },
                { "x": 30.0, "y": 10.0 }, { "x": -10.0, "y": 10.0 }
            ] } },
            "seed": 30
        }),
    );
    assert_eq!(third.changed_ids, vec!["board", "outline-version-30"]);

    let accepted = document(&mut engine);
    let names: Vec<&str> = board_state(&accepted)
        .unwrap()
        .versions
        .iter()
        .map(|version| version.name.as_str())
        .collect();
    assert_eq!(
        names,
        vec!["Edited outline 1", "Edited outline 2", "Edited outline 3"]
    );
}

/// `create-generated`: one accepted edit adds the board's Generated envelope AND
/// registers its board membership (the gap `SetOutline` has today), and one Undo removes
/// both. A second creation is Unchanged, and a version target is malformed.
#[test]
fn create_generated_registers_the_envelope_and_board_membership() {
    let mut engine = CoreEngine::new();
    let opened = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: serde_json::from_value(fixture_without_generated()).unwrap(),
    }));
    let feature_id = "board:board:automatic:outline:91";
    let change = serde_json::json!({
        "create-generated": {
            "margin": 4.0,
            "settings": { "corners": "fillet", "size": 2.0, "bridgeWidth": 10.0 },
            "part_ids": ["a", "b"]
        }
    });
    let committed = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": change,
            "seed": 91
        }),
    );
    assert_eq!(committed.changed_ids, vec!["board", feature_id]);
    assert!(
        !committed.contours.is_empty(),
        "the created generated envelope renders a boundary"
    );

    let accepted = document(&mut engine);
    let feature = accepted
        .outline
        .iter()
        .find(|feature| feature.id() == feature_id)
        .expect("the envelope is authored");
    let OutlineFeature::PartEnvelope {
        connections,
        settings,
        part_ids,
        margin,
        operation,
        ..
    } = feature
    else {
        panic!("an envelope is authored")
    };
    assert_eq!(*margin, 4.0);
    assert_eq!(settings.corners, CornerStyle::Fillet);
    assert_eq!(settings.size, 2.0);
    assert_eq!(settings.bridge_width, 10.0);
    assert!(settings.repair.is_none());
    assert!(connections.is_empty());
    assert_eq!(*operation, Operation::Add);
    assert_eq!(part_ids, &vec!["a".to_owned(), "b".to_owned()]);
    assert!(
        accepted
            .boards
            .iter()
            .find(|board| board.id == "board")
            .unwrap()
            .outline_ids
            .contains(&feature_id.to_owned()),
        "the created envelope is registered as the board's generated outline"
    );

    // A second creation is Unchanged and adds no revision or history step.
    let again = edit(
        &mut engine,
        committed.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": change,
            "seed": 91
        }),
    );
    assert!(again.changed_ids.is_empty());
    assert_eq!(again.revision, committed.revision);
    let accepted = document(&mut engine);
    let pending = OutlineEdit {
        target: OutlineEditTarget::Board {
            board_id: "board".into(),
        },
        change: OutlineChange::CreateGenerated {
            margin: 4.0,
            settings: reference_generated_settings(),
            part_ids: vec!["a".into(), "b".into()],
        },
        seed: 91,
        reviewed: None,
    };
    assert_eq!(
        evaluate(&accepted, &pending).unwrap(),
        OutlineEditDisposition::Unchanged
    );

    // One Undo removes the feature and the membership together.
    let undone = undo(&mut engine);
    assert_eq!(undone.revision, committed.revision + 1);
    let accepted = document(&mut engine);
    assert!(
        !accepted
            .outline
            .iter()
            .any(|feature| feature.id() == feature_id)
    );
    assert!(
        !accepted
            .boards
            .iter()
            .find(|board| board.id == "board")
            .unwrap()
            .outline_ids
            .contains(&feature_id.to_owned()),
        "one Undo removes the membership too"
    );
    let redone = redo(&mut engine);
    assert_eq!(redone.revision, undone.revision + 1);
    let accepted = document(&mut engine);
    assert!(
        accepted
            .outline
            .iter()
            .any(|feature| feature.id() == feature_id)
    );
    assert!(
        accepted
            .boards
            .iter()
            .find(|board| board.id == "board")
            .unwrap()
            .outline_ids
            .contains(&feature_id.to_owned())
    );

    // Malformed: a version target cannot create the board's Generated envelope, and an
    // unknown part is refused with no mutation.
    let before = serde_json::to_value(document(&mut engine)).unwrap();
    let version_target = reply(
        &mut engine,
        redone.revision,
        serde_json::json!({
            "target": { "kind": "version", "boardId": "board", "versionId": "refined" },
            "change": { "create-generated": {
                "margin": 4.0,
                "settings": { "corners": "fillet", "size": 2.0, "bridgeWidth": 10.0 },
                "part_ids": ["a"]
            } },
            "seed": 92
        }),
    );
    assert!(matches!(version_target, CoreReply::Error { .. }));
    let unknown_part = reply(
        &mut engine,
        redone.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "create-generated": {
                "margin": 4.0,
                "settings": { "corners": "fillet", "size": 2.0, "bridgeWidth": 10.0 },
                "part_ids": ["a", "nope"]
            } },
            "seed": 93
        }),
    );
    assert!(matches!(unknown_part, CoreReply::Error { .. }));
    assert_eq!(
        serde_json::to_value(document(&mut engine)).unwrap(),
        before,
        "a malformed creation changes nothing"
    );
}

fn reference_generated_settings() -> OutlineSettings {
    OutlineSettings {
        corners: CornerStyle::Fillet,
        size: 2.0,
        bridge_width: 10.0,
        repair: None,
    }
}

/// Removing a gap clears the protection by the gap's identity, not one alias string: a
/// derived candidate protected by authored keep gaps whose ids differ from it (aliases)
/// loses all of them, which is how the old planner's uncheck behaved.
#[test]
fn removing_a_gap_clears_every_alias_protecting_it() {
    let (mut engine, opened) = open((20.0, 0.0));
    let before = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }));
    let candidates = before
        .board_outline_scenes
        .iter()
        .find(|scene| scene.board_id == "board")
        .unwrap()
        .gaps
        .clone();
    assert_eq!(
        candidates.len(),
        2,
        "the fixture's spaced components expose two gap candidates"
    );
    let candidate = candidates[0].clone();
    let candidate_id = candidate.id.clone();
    assert!(!candidate.protected);
    let points = serde_json::to_value(&candidate.points).unwrap();

    // Author the protection under two ids that are not the candidate's own id.
    let first = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "set-protected-gap": {
                "gap_id": "gap-alias-a", "points": points
            } },
            "seed": 100
        }),
    );
    let second = edit(
        &mut engine,
        first.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "set-protected-gap": {
                "gap_id": "gap-alias-b", "points": points
            } },
            "seed": 101
        }),
    );
    let accepted = document(&mut engine);
    assert_eq!(accepted.outline[0].keep_gap_ids().len(), 2);
    let protected = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }));
    let candidate_now = protected
        .board_outline_scenes
        .iter()
        .find(|scene| scene.board_id == "board")
        .unwrap()
        .gaps
        .iter()
        .find(|gap| gap.id == candidate_id)
        .expect("the candidate is still derived")
        .clone();
    assert!(candidate_now.protected);
    assert!(
        candidate_now
            .protected_ids
            .contains(&"gap-alias-a".to_owned())
    );
    assert!(
        candidate_now
            .protected_ids
            .contains(&"gap-alias-b".to_owned())
    );
    assert!(
        !candidate_now.protected_ids.contains(&candidate_id),
        "the protection is authored under aliases, not the candidate id"
    );

    // Removing the gap by its own identity clears every alias that protects it.
    let removed = edit(
        &mut engine,
        second.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "remove-protected-gap": { "gap_id": candidate_id } },
            "seed": 102
        }),
    );
    assert_eq!(
        removed.changed_ids,
        sorted(vec![
            "board".into(),
            "gap-alias-a".into(),
            "gap-alias-b".into(),
            "gen".into()
        ])
    );
    let accepted = document(&mut engine);
    assert!(accepted.outline[0].keep_gap_ids().is_empty());
    let after = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }));
    let unrelated = after
        .board_outline_scenes
        .iter()
        .find(|scene| scene.board_id == "board")
        .unwrap()
        .gaps
        .iter()
        .find(|gap| gap.id == candidate_id)
        .map(|gap| gap.protected);
    assert_ne!(unrelated, Some(true), "no alias protects it any more");
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

/// Settings edits are validated by Core, not by whichever surface submits them: a value
/// outside a field's range is refused with no mutation, on the Generated envelope and on a
/// linked version alike.
#[test]
fn out_of_range_settings_are_refused_with_no_mutation() {
    let (mut engine, opened) = open((14.0, 0.0));
    let created = create_refined(&mut engine, opened.revision);
    let before = serde_json::to_value(document(&mut engine)).unwrap();
    let cases = [
        ("set-margin", "margin", -1.0),
        ("set-size", "size", -1.0),
        ("set-bridge-width", "bridge_width", 0.0),
        ("set-maximum-gap-span", "value", -1.0),
        ("set-minimum-connection-width", "value", -1.0),
        ("set-edge-clearance", "value", -1.0),
    ];
    let targets = [
        serde_json::json!({ "kind": "board", "boardId": "board" }),
        serde_json::json!({ "kind": "version", "boardId": "board", "versionId": "refined" }),
    ];
    for target in &targets {
        for (change, field, value) in cases {
            let refused = reply(
                &mut engine,
                created.revision,
                serde_json::json!({
                    "target": target,
                    "change": { change: { field: value } },
                    "seed": 90
                }),
            );
            let CoreReply::Error {
                message, revision, ..
            } = refused
            else {
                panic!("{change} {value} on {target} must be refused");
            };
            assert!(message.contains("must be"), "{change}: {message}");
            assert_eq!(revision, created.revision);
        }
    }
    assert_eq!(
        serde_json::to_value(document(&mut engine)).unwrap(),
        before,
        "a refused setting changes nothing"
    );
}

/// A version's base contour cannot be removed, whatever its feature kind: removing it
/// would leave the outline empty or hole-only.
#[test]
fn a_fixed_versions_base_contour_cannot_be_removed() {
    let (mut engine, opened) = open((14.0, 0.0));
    let created = create_refined(&mut engine, opened.revision);
    let copied = edit(
        &mut engine,
        created.revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "copy": { "version_id": "fixed", "name": "Fixed" } },
            "seed": 91
        }),
    );
    // A perimeter edit turns the copy's envelope into a plain polygon contour.
    let envelope = version_of(&document(&mut engine), "fixed")
        .unwrap()
        .geometry
        .features[0]
        .id()
        .to_owned();
    let ring = serde_json::json!([
        { "x": -10.0, "y": -10.0 }, { "x": 30.0, "y": -10.0 },
        { "x": 30.0, "y": 10.0 }, { "x": -10.0, "y": 10.0 }
    ]);
    let edited = edit(
        &mut engine,
        copied.revision,
        serde_json::json!({
            "target": { "kind": "version", "boardId": "board", "versionId": "fixed" },
            "change": { "edit-feature-perimeter": { "feature_id": envelope, "points": ring } },
            "seed": 92
        }),
    );
    let accepted = document(&mut engine);
    let base = version_of(&accepted, "fixed").unwrap().geometry.features[0].clone();
    assert!(
        !matches!(base, OutlineFeature::PartEnvelope { .. }),
        "the base contour is now a plain polygon"
    );
    let before = serde_json::to_value(&accepted).unwrap();
    let refused = reply(
        &mut engine,
        edited.revision,
        serde_json::json!({
            "target": { "kind": "version", "boardId": "board", "versionId": "fixed" },
            "change": { "remove-feature": { "feature_id": base.id() } },
            "seed": 93
        }),
    );
    let CoreReply::Error { message, .. } = refused else {
        panic!("the base contour must not be removable");
    };
    assert!(message.contains("base outline feature"), "{message}");
    assert_eq!(
        serde_json::to_value(document(&mut engine)).unwrap(),
        before,
        "a refused removal changes nothing"
    );
}

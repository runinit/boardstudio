//! Linked outline resolution, source cleanup and scoped findings (typed Core edits 12).
//!
//! A linked refinement ("outline refinement") resolves from its board's *current*
//! included components and its own settings/choices; a fixed edited copy keeps the
//! geometry it was saved with. Expectations in this file are worked by hand from the
//! fixture's millimetre geometry; none of them is computed by the resolver under test.
//!
//! Fixture geometry (all hand-derived):
//!   switch definition courtyard 14x14 ([-7,7]^2), one 4x4 pad at the part origin.
//!   envelope margin 1.0 -> a switch envelope is 16x16 ([-8,8]^2 around its pose).
//! Board "board": part a at (0,0), part b at (14,0); board outline feature "gen";
//! active version "refined" (linked) and fixed version "edited" (30x30 fixed square
//! polygon, bounds -30..30).
use boardstudio_core::{CoreEngine, model::*};

const ENVELOPE: &str = "board:board:version:refined:contour:0";
const CUTOUT: &str = "board:board:version:refined:cutout:0";

fn settings() -> serde_json::Value {
    serde_json::json!({
        "corners": "sharp", "size": 2.0, "bridgeWidth": 10.0,
        "repair": {
            "enabled": true, "maximumGapSpan": 20.0,
            "minimumConnectionWidth": 2.0, "edgeClearance": 0.0, "keepGaps": []
        }
    })
}

fn switch_definition(courtyard: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "id": "switch", "name": "Switch", "kind": "switch",
        "courtyard": courtyard,
        "pads": [{ "id": "p0", "number": "1", "at": { "x": 0.0, "y": 0.0 },
                   "size": { "x": 4.0, "y": 4.0 }, "shape": "rect" }],
    })
}

fn definition() -> serde_json::Value {
    switch_definition(serde_json::json!([
        { "x": -7.0, "y": -7.0 }, { "x": 7.0, "y": -7.0 },
        { "x": 7.0, "y": 7.0 }, { "x": -7.0, "y": 7.0 }
    ]))
}

/// Part `b`'s pose; every fixture keeps `a` at the origin.
fn fixture(b_at: (f64, f64), b_rotation: f64) -> serde_json::Value {
    let mut document = serde_json::to_value(ProjectDoc::empty("linked", "Linked")).unwrap();
    document["definitions"] = serde_json::json!([definition()]);
    document["parts"] = serde_json::json!([
        { "id": "a", "definitionId": "switch", "reference": "SW1", "side": "front",
          "pose": { "at": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 } },
        { "id": "b", "definitionId": "switch", "reference": "SW2", "side": "front",
          "pose": { "at": { "x": b_at.0, "y": b_at.1 }, "rotation": b_rotation } }
    ]);
    document["boards"] = serde_json::json!([{ "id": "board", "name": "Board",
        "outlineIds": ["gen"], "partIds": ["a", "b"], "netIds": [], "thickness": 1.6 }]);
    document["outline"] = serde_json::json!([{ "kind": "part-envelope", "id": "gen",
        "partIds": ["a", "b"], "margin": 1.0, "operation": "add", "settings": settings() }]);
    document["boardOutlines"] = serde_json::json!([{
        "boardId": "board", "activeVersionId": "refined",
        "versions": [
            { "id": "refined", "name": "Refined", "source": { "revision": 0 },
              "ownership": "linked",
              "geometry": { "features": [
                    { "kind": "part-envelope", "id": ENVELOPE, "partIds": ["a", "b"],
                      "margin": 1.0, "operation": "add", "settings": settings() }
                ], "settings": settings(), "expectedRegions": 1 } },
            { "id": "edited", "name": "Edited", "source": { "revision": 0 },
              "geometry": { "features": [
                    { "kind": "polygon", "id": "board:board:version:edited:contour:0",
                      "points": [
                        { "x": -30.0, "y": -30.0 }, { "x": 30.0, "y": -30.0 },
                        { "x": 30.0, "y": 30.0 }, { "x": -30.0, "y": 30.0 }
                      ], "operation": "add" }
                ], "settings": settings(), "expectedRegions": 1 } }
        ]
    }]);
    document
}

/// The same fixture with authored choices that name their source components.
fn fixture_with_choices() -> serde_json::Value {
    let mut document = fixture((14.0, 0.0), 0.0);
    let quad = |part: &str| {
        serde_json::json!([
            { "at": { "x": -5.0, "y": -5.0 }, "partId": part },
            { "at": { "x": 5.0, "y": -5.0 }, "partId": part },
            { "at": { "x": 5.0, "y": 5.0 }, "partId": part },
            { "at": { "x": -5.0, "y": 5.0 }, "partId": part }
        ])
    };
    let mut envelope = serde_json::json!({
        "kind": "part-envelope", "id": ENVELOPE, "partIds": ["a", "b"],
        "margin": 1.0, "operation": "add", "settings": settings(),
        "connections": [
            { "id": "link-ab", "width": 4.0, "points": [
                { "at": { "x": 0.0, "y": 0.0 }, "partId": "a" },
                { "at": { "x": 0.0, "y": 0.0 }, "partId": "b" }] },
            { "id": "link-a", "width": 4.0, "points": [
                { "at": { "x": -4.0, "y": 0.0 }, "partId": "a" },
                { "at": { "x": 4.0, "y": 0.0 }, "partId": "a" }] }
        ]
    });
    envelope["settings"]["repair"]["keepGaps"] = serde_json::json!([]);
    let cutout = serde_json::json!({
        "kind": "polygon", "id": CUTOUT, "anchorPartId": "b", "operation": "subtract",
        "points": [{ "x": -2.0, "y": -2.0 }, { "x": 2.0, "y": -2.0 },
                   { "x": 2.0, "y": 2.0 }, { "x": -2.0, "y": 2.0 }]
    });
    let mut version_settings = settings();
    // A refinement's protected gaps are version settings, so they follow the
    // refinement rather than a single contour.
    version_settings["repair"]["keepGaps"] = serde_json::json!([
        { "id": "gap-a", "points": quad("a") },
        { "id": "gap-b", "points": quad("b") }
    ]);
    document["boardOutlines"][0]["versions"][0]["geometry"]["features"] =
        serde_json::json!([envelope, cutout]);
    document["boardOutlines"][0]["versions"][0]["geometry"]["settings"] =
        version_settings;
    document
}

fn open(engine: &mut CoreEngine, document: serde_json::Value) -> SceneDelta {
    scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: serde_json::from_value(document).expect("fixture is a valid document"),
    }))
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
        id: "edit".into(),
        command: EditCommand {
            base_revision: revision,
            transaction_id: "edit".into(),
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

fn saved(engine: &mut CoreEngine) -> serde_json::Value {
    serde_json::to_value(document(engine)).expect("document serializes")
}

fn move_part(engine: &mut CoreEngine, revision: u64, id: &str, at: (f64, f64)) -> SceneDelta {
    edit(
        engine,
        revision,
        serde_json::json!({
            "kind": "move-parts",
            "positions": [{ "id": id, "at": { "x": at.0, "y": at.1 } }]
        }),
    )
}

/// Hand-checked bounding box of the board's drawn metallised contours.
fn bounds(scene: &SceneDelta, board_id: &str) -> (f64, f64, f64, f64) {
    let contours = scene
        .board_contours
        .iter()
        .find(|entry| entry.board_id == board_id)
        .expect("the board has contours");
    let points: Vec<Vec2> = contours
        .contours
        .iter()
        .filter(|contour| !contour.hole)
        .flat_map(|contour| contour.points.clone())
        .collect();
    assert!(!points.is_empty(), "the board has a drawn outline");
    let round = |value: f64| (value * 1000.0).round() / 1000.0;
    (
        round(points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min)),
        round(points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min)),
        round(points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max)),
        round(points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max)),
    )
}

/// Assert a hand-derived box, allowing only the document's 0.001 mm snap grid.
fn assert_bounds(scene: &SceneDelta, board_id: &str, expected: (f64, f64, f64, f64), tolerance: f64) {
    let actual = bounds(scene, board_id);
    let expected = [
        expected.0, expected.1, expected.2, expected.3,
    ];
    for (actual, expected) in [actual.0, actual.1, actual.2, actual.3].iter().zip(expected) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "hand-derived bound {expected} vs resolved {actual} in {actual:?}"
        );
    }
}

fn readiness(scene: &SceneDelta, board_id: &str) -> BoardReadiness {
    scene
        .board_readiness
        .iter()
        .find(|entry| entry.board_id == board_id)
        .expect("the board has readiness")
        .clone()
}

fn version_geometry(saved: &serde_json::Value, id: &str) -> serde_json::Value {
    saved["boardOutlines"][0]["versions"]
        .as_array()
        .expect("versions")
        .iter()
        .find(|version| version["id"] == id)
        .expect("the named version exists")["geometry"]
        .clone()
}

/// Case 1: movement, rotation and definition geometry changes flow into the linked
/// refinement while the fixed edited alternative keeps its geometry.
#[test]
fn linked_refinements_follow_placement_rotation_and_definition_geometry_while_fixed_copies_stay() {
    let mut engine = CoreEngine::new();
    let opened = open(&mut engine, fixture((14.0, 0.0), 0.0));
    // a [-8,8]^2, b [6,22]x[-8,8] -> union x -8..22, y -8..8.
    assert_eq!(bounds(&opened, "board"), (-8.0, -8.0, 22.0, 8.0));
    let fixed_before = version_geometry(&saved(&mut engine), "edited");

    // Placement through the accepted typed edit: b -> (14, 9) keeps the overlap
    // x 6..8 and lifts y to 9 + 8 = 17.
    let moved = move_part(&mut engine, opened.revision, "b", (14.0, 9.0));
    assert_eq!(bounds(&moved, "board"), (-8.0, -8.0, 22.0, 17.0));
    assert_eq!(
        version_geometry(&saved(&mut engine), "edited"),
        fixed_before,
        "an edited outline keeps its geometry when a component moves"
    );

    // Rotation: b's 14x14 courtyard at (14,9) turned 45 degrees reaches a half
    // diagonal of 7*sqrt(2) = 9.89949 mm. The margin is offset with a miter join, so
    // the sharp 90-degree corner extends a further 1.0*sqrt(2) = 1.41421 mm: the
    // rightmost point is 14 + 11.31370 = 25.3137 (x) and 9 + 11.31370 = 20.3137 (y).
    // The bound is asserted to the document's 0.001 mm grid plus one grid step.
    let mut rotated_document = fixture((14.0, 9.0), 45.0);
    rotated_document["revision"] = serde_json::json!(moved.revision);
    let mut engine = CoreEngine::new();
    let rotated = open(&mut engine, rotated_document);
    assert_bounds(&rotated, "board", (-8.0, -8.0, 25.3137, 20.3137), 0.002);
    assert_eq!(version_geometry(&saved(&mut engine), "edited"), fixed_before);

    // Definition geometry: an 18x18 courtyard at margin 1.0 gives a 20x20 envelope,
    // so a spans -10..10 and b (at 14,0) spans 4..24.
    let mut resized = fixture((14.0, 0.0), 0.0);
    resized["definitions"] = serde_json::json!([switch_definition(serde_json::json!([
        { "x": -9.0, "y": -9.0 }, { "x": 9.0, "y": -9.0 },
        { "x": 9.0, "y": 9.0 }, { "x": -9.0, "y": 9.0 }
    ]))]);
    let mut engine = CoreEngine::new();
    let resized = open(&mut engine, resized);
    assert_eq!(bounds(&resized, "board"), (-10.0, -10.0, 24.0, 10.0));
    assert_eq!(version_geometry(&saved(&mut engine), "edited"), fixed_before);
}

/// Case 2: additions, removals and inclusion changes use the live included set.
#[test]
fn linked_refinements_use_the_live_included_component_set() {
    let mut engine = CoreEngine::new();
    let opened = open(&mut engine, fixture((14.0, 0.0), 0.0));
    assert_eq!(bounds(&opened, "board"), (-8.0, -8.0, 22.0, 8.0));

    // Adding c at (14,9) overlaps the existing union in y 1..8, so the boundary
    // becomes one shape spanning y -8..17.
    let added = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({
            "kind": "add-part",
            "boardId": "board",
            "part": { "id": "c", "definitionId": "switch", "reference": "SW3", "side": "front",
                      "pose": { "at": { "x": 14.0, "y": 9.0 }, "rotation": 0.0 } }
        }),
    );
    assert_eq!(bounds(&added, "board"), (-8.0, -8.0, 22.0, 17.0));

    // Removing c recalculates: the boundary must not keep the stale 17 mm extent.
    let removed = edit(
        &mut engine,
        added.revision,
        serde_json::json!({ "kind": "remove-parts", "ids": ["c"] }),
    );
    assert_eq!(bounds(&removed, "board"), (-8.0, -8.0, 22.0, 8.0));

    // Excluding b leaves only a's 16x16 envelope, without reshaping the source.
    let mut excluded = fixture((14.0, 0.0), 0.0);
    excluded["revision"] = serde_json::json!(removed.revision);
    excluded["parts"][1]["outline"] = serde_json::json!({ "excluded": true });
    let excluded = open(&mut engine, excluded);
    assert_eq!(bounds(&excluded, "board"), (-8.0, -8.0, 8.0, 8.0));
}

/// Case 3: removing a named source removes only dependent authored choices, in the
/// same accepted edit, and one Undo restores source, choices and geometry together.
#[test]
fn removing_a_source_cleans_only_its_dependent_choices_in_one_undo() {
    let mut engine = CoreEngine::new();
    let opened = open(&mut engine, fixture_with_choices());
    let before = saved(&mut engine);

    let removed = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({ "kind": "remove-parts", "ids": ["b"] }),
    );
    for cleaned in ["b", "link-ab", "gap-b", CUTOUT] {
        assert!(
            removed.changed_ids.iter().any(|id| id == cleaned),
            "{cleaned} changed in the removal edit: {:?}",
            removed.changed_ids
        );
    }
    let after = saved(&mut engine);
    let geometry = &after["boardOutlines"][0]["versions"][0]["geometry"];
    let connection_ids: Vec<&str> = geometry["features"][0]["connections"]
        .as_array()
        .expect("connections")
        .iter()
        .map(|connection| connection["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        connection_ids,
        vec!["link-a"],
        "only the connection needing the removed source is dropped; the surviving \
         connection keeps its authored endpoints"
    );
    assert_eq!(
        geometry["features"].as_array().unwrap().len(),
        1,
        "the cutout anchored to the removed source is dropped"
    );
    let gap_ids: Vec<&str> = geometry["settings"]["repair"]["keepGaps"]
        .as_array()
        .expect("keep gaps")
        .iter()
        .map(|gap| gap["id"].as_str().unwrap())
        .collect();
    assert_eq!(gap_ids, vec!["gap-a"]);
    assert_eq!(
        bounds(&removed, "board"),
        (-8.0, -8.0, 8.0, 8.0),
        "the refinement resolves from the remaining component"
    );

    // One Undo restores the removal, the choices and the resolved geometry together.
    let undone = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    let restored = saved(&mut engine);
    let mut expected = before.clone();
    expected["revision"] = restored["revision"].clone();
    assert_eq!(
        restored, expected,
        "Undo restores parts, authored choices and the resolved refinement"
    );
    assert_eq!(bounds(&undone, "board"), (-8.0, -8.0, 22.0, 8.0));

    let redone = scene(engine.handle(CoreRequest::Redo { id: "redo".into() }));
    assert_eq!(bounds(&redone, "board"), (-8.0, -8.0, 8.0, 8.0));
}

/// Case 4: a settings change updates only that refinement and preserves its link and
/// every other version's choices.
#[test]
fn changing_one_refinements_margin_updates_only_that_refinement() {
    let mut engine = CoreEngine::new();
    let opened = open(&mut engine, fixture((14.0, 0.0), 0.0));
    let before = saved(&mut engine);

    // margin 3.0 -> half extent 7 + 3 = 10: a spans -10..10, b (14,0) spans 4..24.
    let adjusted = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({
            "kind": "set-outline",
            "feature": { "kind": "part-envelope", "id": ENVELOPE, "partIds": ["a", "b"],
                         "margin": 3.0, "operation": "add", "settings": settings() }
        }),
    );
    assert_eq!(bounds(&adjusted, "board"), (-10.0, -10.0, 24.0, 10.0));

    let after = saved(&mut engine);
    assert_eq!(
        after["boardOutlines"][0]["versions"][0]["ownership"],
        serde_json::json!("linked"),
        "a settings change keeps the refinement linked"
    );
    assert_eq!(
        after["boardOutlines"][0]["versions"][0]["name"],
        before["boardOutlines"][0]["versions"][0]["name"]
    );
    assert_eq!(
        version_geometry(&after, "edited"),
        version_geometry(&before, "edited"),
        "another version keeps its geometry"
    );
    assert_eq!(
        after["outline"], before["outline"],
        "the generated source and its settings are untouched"
    );
}

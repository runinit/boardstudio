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
    document["boardOutlines"][0]["versions"][0]["geometry"]["settings"] = version_settings;
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
fn assert_bounds(
    scene: &SceneDelta,
    board_id: &str,
    expected: (f64, f64, f64, f64),
    tolerance: f64,
) {
    let actual = bounds(scene, board_id);
    let expected = [expected.0, expected.1, expected.2, expected.3];
    for (actual, expected) in [actual.0, actual.1, actual.2, actual.3]
        .iter()
        .zip(expected)
    {
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
    assert_eq!(
        version_geometry(&saved(&mut engine), "edited"),
        fixed_before
    );

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
    assert_eq!(
        version_geometry(&saved(&mut engine), "edited"),
        fixed_before
    );
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
            // Ticket 37: raw SetOutline is deleted; SetFeature on the version is live.
            "kind": "apply-outline-edit", "edit": {
                "target": {"kind": "version", "boardId": "board", "versionId": "refined"},
                "change": {"set-feature": {"feature": { "kind": "part-envelope", "id": ENVELOPE, "partIds": ["a", "b"],
                             "margin": 3.0, "operation": "add", "settings": settings() }}},
                "seed": 21, "reviewed": null
            }
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

/// Case 5: a geometric failure keeps the refinement linked, editable and located;
/// only the affected output is blocked, and a later repair resumes current geometry.
#[test]
fn a_failed_resolution_stays_linked_with_located_findings_and_resumes_after_a_repair() {
    const BAD_CUTOUT: &str = "board:board:version:refined:cutout:bad";
    // A self-intersecting authored cutout anchored to component a cannot resolve.
    let mut document = fixture((14.0, 0.0), 0.0);
    document["boardOutlines"][0]["versions"][0]["geometry"]["features"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "kind": "polygon", "id": BAD_CUTOUT, "anchorPartId": "a",
            "operation": "add",
            "points": [{ "x": 0.0, "y": 0.0 }, { "x": 4.0, "y": 4.0 },
                       { "x": 4.0, "y": 0.0 }, { "x": 0.0, "y": 4.0 }]
        }));
    let mut engine = CoreEngine::new();
    let broken = open(&mut engine, document);
    let failure_id = "board:board:outline:linked:refined";
    let finding = broken
        .findings
        .iter()
        .find(|finding| finding.id == failure_id)
        .expect("a linked resolution failure is reported");
    assert_eq!(finding.severity, Severity::Error);
    assert_eq!(finding.scope, Scope::Pcb);
    assert!(finding.target_ids.iter().any(|id| id == "board"));
    assert!(finding.target_ids.iter().any(|id| id == "refined"));
    assert!(finding.message.contains("last valid"));
    let marker = broken
        .finding_markers
        .iter()
        .find(|marker| marker.finding_id == failure_id)
        .expect("the failure is located on the workbench");
    assert!(
        marker
            .contours
            .iter()
            .any(|contour| contour.points.len() >= 3),
        "the located region is drawn"
    );
    let blocked = readiness(&broken, "board");
    assert!(
        !blocked.outline && !blocked.pcb,
        "dependent output is blocked"
    );
    assert!(
        !broken.board_contours[0].contours.is_empty(),
        "a display remains"
    );

    // Saving and further editing stay available, and the refinement stays linked.
    let moved = move_part(&mut engine, broken.revision, "a", (0.0, 0.0));
    assert!(
        moved
            .findings
            .iter()
            .any(|finding| finding.id == failure_id)
    );
    assert_eq!(
        saved(&mut engine)["boardOutlines"][0]["versions"][0]["ownership"],
        serde_json::json!("linked"),
        "a failed resolution never unlinks or freezes the refinement"
    );

    // Repairing the authored shape resumes the current geometry without relinking.
    let repaired = edit(
        &mut engine,
        moved.revision,
        serde_json::json!({
            "kind": "apply-outline-edit", "edit": {
                "target": {"kind": "version", "boardId": "board", "versionId": "refined"},
                "change": {"set-feature": {"feature": { "kind": "polygon", "id": "board:board:version:refined:cutout:bad",
                             "anchorPartId": "a", "operation": "add",
                             "points": [{ "x": 3.0, "y": 3.0 }, { "x": 7.0, "y": 3.0 },
                                        { "x": 7.0, "y": 7.0 }, { "x": 3.0, "y": 7.0 }] }}},
                "seed": 22, "reviewed": null
            }
        }),
    );
    assert!(
        !repaired
            .findings
            .iter()
            .any(|finding| finding.id == failure_id)
    );
    assert!(
        readiness(&repaired, "board").outline && readiness(&repaired, "board").pcb,
        "{:?}",
        repaired
            .findings
            .iter()
            .map(|finding| (finding.id.clone(), finding.severity.clone()))
            .collect::<Vec<_>>()
    );
    assert_eq!(bounds(&repaired, "board"), (-8.0, -8.0, 22.0, 8.0));
    assert_eq!(
        saved(&mut engine)["boardOutlines"][0]["versions"][0]["ownership"],
        serde_json::json!("linked")
    );
}

/// Case 6: a failure of an inactive alternative or of another board cannot block the
/// selected outline's valid output.
#[test]
fn inactive_and_unrelated_findings_do_not_block_valid_selected_output() {
    let mut document = fixture((14.0, 0.0), 0.0);
    // An unselected linked alternative whose cutout names a missing component.
    document["boardOutlines"][0]["versions"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "id": "alt", "name": "Alternative", "source": { "revision": 0 },
            "ownership": "linked",
            "geometry": {
                "features": [
                    { "kind": "part-envelope", "id": "board:board:version:alt:contour:0",
                      "partIds": ["a"], "margin": 1.0, "operation": "add",
                      "settings": settings() },
                    { "kind": "polygon", "id": "board:board:version:alt:cutout:0",
                      "anchorPartId": "ghost", "operation": "subtract",
                      "points": [{ "x": -2.0, "y": -2.0 }, { "x": 2.0, "y": -2.0 },
                                 { "x": 2.0, "y": 2.0 }, { "x": -2.0, "y": 2.0 }] }
                ],
                "settings": settings(), "expectedRegions": 1
            }
        }));
    // An unrelated second board whose generated outline resolves to no area.
    document["boards"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "id": "other", "name": "Other", "outlineIds": ["other-outline"],
            "partIds": [], "netIds": [], "thickness": 1.6
        }));
    document["outline"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "kind": "part-envelope", "id": "other-outline", "partIds": [],
            "margin": 1.0, "operation": "add", "settings": settings()
        }));

    let mut engine = CoreEngine::new();
    let opened = open(&mut engine, document);
    let alternative = "board:board:outline:linked:alt";
    assert!(
        opened
            .findings
            .iter()
            .any(|finding| finding.id == alternative),
        "the broken inactive alternative stays visible: {:?}",
        opened
            .findings
            .iter()
            .map(|finding| &finding.id)
            .collect::<Vec<_>>()
    );
    assert!(
        readiness(&opened, "board").outline && readiness(&opened, "board").pcb,
        "an unselected alternative cannot block the selected outline"
    );
    assert_eq!(bounds(&opened, "board"), (-8.0, -8.0, 22.0, 8.0));
    assert!(
        !readiness(&opened, "other").pcb,
        "the broken board still reports"
    );
}

/// Case 7: existing clearance rules keep ADR-0001's blocker and advisory meanings
/// when the selected outline is a linked refinement.
#[test]
fn linked_outlines_keep_edge_clearance_blocker_and_permitted_overhang_advisory_meanings() {
    // A required pad placed outside the resolved envelope is a blocker.
    let mut document = fixture((14.0, 0.0), 0.0);
    document["definitions"][0]["pads"] = serde_json::json!([
        { "id": "p0", "number": "1", "at": { "x": 0.0, "y": 0.0 },
          "size": { "x": 4.0, "y": 4.0 }, "shape": "rect" },
        { "id": "p1", "number": "2", "at": { "x": 0.0, "y": 20.0 },
          "size": { "x": 4.0, "y": 4.0 }, "shape": "rect" }
    ]);
    let mut engine = CoreEngine::new();
    let unsupported = open(&mut engine, document);
    assert!(
        unsupported
            .findings
            .iter()
            .any(|finding| finding.id == "board:board:outline:support:a"
                && finding.severity == Severity::Error),
        "unsupported required pad material is a blocker"
    );
    assert!(!readiness(&unsupported, "board").pcb);

    // A switch whose keycap envelope is smaller than its refused courtyard: the
    // courtyard overhang is advisory only when the part explicitly permits it.
    let mut overhang = fixture((14.0, 0.0), 0.0);
    overhang["definitions"][0]["courtyard"] = serde_json::json!([
        { "x": -15.0, "y": -15.0 }, { "x": 15.0, "y": -15.0 },
        { "x": 15.0, "y": 15.0 }, { "x": -15.0, "y": 15.0 }
    ]);
    // The physical envelope comes from the 14x14 keycap, so the 30x30 courtyard
    // sticks out of the resolved outline.
    overhang["definitions"][0]["keycap"] = serde_json::json!({ "x": 14.0, "y": 14.0 });
    let mut engine = CoreEngine::new();
    let refused = open(&mut engine, overhang.clone());
    let body = refused
        .findings
        .iter()
        .find(|finding| finding.id == "board:board:outline:body:a")
        .expect("the courtyard overhang is reported");
    assert_eq!(
        (body.severity.clone(), body.scope.clone()),
        (Severity::Error, Scope::Pcb),
        "an unpermitted body overhang blocks its output"
    );
    assert!(!readiness(&refused, "board").pcb);

    overhang["parts"][0]["outline"] = serde_json::json!({ "allowBodyOverhang": true });
    overhang["parts"][1]["outline"] = serde_json::json!({ "allowBodyOverhang": true });
    let mut engine = CoreEngine::new();
    let permitted = open(&mut engine, overhang);
    let body = permitted
        .findings
        .iter()
        .find(|finding| finding.id == "board:board:outline:body:a")
        .expect("the permitted overhang is still reported");
    assert_eq!(
        (body.severity.clone(), body.scope.clone()),
        (Severity::Warning, Scope::Pcb),
        "an explicitly permitted body overhang stays advisory"
    );
    assert!(
        readiness(&permitted, "board").pcb,
        "{:?}",
        permitted
            .findings
            .iter()
            .map(|finding| (finding.id.clone(), finding.severity.clone()))
            .collect::<Vec<_>>()
    );
}

/// Case 7: a protected gap keeps deliberate geometry without waiving support.
#[test]
fn protecting_a_linked_gap_cannot_waive_required_pad_support() {
    let mut document = fixture((14.0, 0.0), 0.0);
    // Protecting a recess that contains component a's required pad removes the
    // material under it, so the support rule still reports a blocker.
    document["boardOutlines"][0]["versions"][0]["geometry"]["settings"]["repair"]["keepGaps"] = serde_json::json!([{ "id": "gap-a", "points": [
            { "at": { "x": -5.0, "y": -5.0 }, "partId": "a" },
            { "at": { "x": 5.0, "y": -5.0 }, "partId": "a" },
            { "at": { "x": 5.0, "y": 5.0 }, "partId": "a" },
            { "at": { "x": -5.0, "y": 5.0 }, "partId": "a" }
        ] }]);
    let mut engine = CoreEngine::new();
    let protected = open(&mut engine, document);
    assert!(
        protected
            .findings
            .iter()
            .any(|finding| finding.id == "board:board:outline:support:a"
                && finding.severity == Severity::Error),
        "protecting the gap does not waive required support: {:?}",
        protected
            .findings
            .iter()
            .map(|finding| &finding.id)
            .collect::<Vec<_>>()
    );
    assert!(!readiness(&protected, "board").pcb);
}

/// A mirror-linked board: `a`/`c` form the left half, their mirrored copies the right.
/// The two halves resolve independently, so the current structure has two islands even
/// though the refinement was accepted with a single expected region before the split.
fn split_fixture(broken: bool) -> serde_json::Value {
    let mut document = fixture((100.0, 0.0), 0.0);
    document["parts"] = serde_json::json!([
        { "id": "a", "definitionId": "switch", "reference": "SW1", "side": "front",
          "pose": { "at": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 } },
        { "id": "c", "definitionId": "switch", "reference": "SW2", "side": "front",
          "pose": { "at": { "x": 40.0, "y": 0.0 }, "rotation": 0.0 } },
        { "id": "mirror-a", "definitionId": "switch", "reference": "SW1_M", "side": "front",
          "pose": { "at": { "x": 100.0, "y": 0.0 }, "rotation": 180.0 },
          "properties": { "boardstudio.mirroredComponentSource": "a" } },
        { "id": "mirror-c", "definitionId": "switch", "reference": "SW2_M", "side": "front",
          "pose": { "at": { "x": 60.0, "y": 0.0 }, "rotation": 180.0 },
          "properties": { "boardstudio.mirroredComponentSource": "c" } }
    ]);
    let members = serde_json::json!(["a", "c", "mirror-a", "mirror-c"]);
    document["boards"][0]["partIds"] = members.clone();
    document["outline"][0]["partIds"] = members.clone();
    document["boardOutlines"][0]["versions"][0]["geometry"]["features"][0]["partIds"] =
        members.clone();
    document["layouts"] = serde_json::json!([
        { "id": "left-keys", "name": "Left keys", "boardId": "board",
          "matrixIds": ["left-matrix"], "partIds": ["a", "c"] },
        { "id": "right-keys", "name": "Right keys", "boardId": "board",
          "matrixIds": ["right-matrix"], "partIds": ["mirror-a", "mirror-c"],
          "mirrorLink": { "sourceId": "left-keys", "axisX": 50.0,
            "correspondences": [{"kind":"matrix","sourceId":"left-matrix","mirrorId":"right-matrix"}] } }
    ]);
    let matrix = |id: &str, origin: f64| {
        serde_json::json!({
            "id": id, "boardId": "board", "definitionId": "switch",
            "rows": 1, "columns": 1, "origin": { "x": origin, "y": 0.0 },
            "pitch": { "x": 19.05, "y": 19.05 }, "partIds": [],
            "cells": [{ "row": 0, "column": 0, "enabled": false }]
        })
    };
    document["matrices"] =
        serde_json::json!([matrix("left-matrix", 0.0), matrix("right-matrix", 100.0)]);
    if broken {
        document["boardOutlines"][0]["versions"][0]["geometry"]["features"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "kind": "polygon", "id": "board:board:version:refined:cutout:bad",
                "anchorPartId": "a", "operation": "add",
                "points": [{ "x": 0.0, "y": 0.0 }, { "x": 4.0, "y": 4.0 },
                           { "x": 4.0, "y": 0.0 }, { "x": 0.0, "y": 4.0 }]
            }));
    }
    document
}

/// The same four components in one layout: the structure is a single island again.
fn merged_fixture() -> serde_json::Value {
    let mut document = split_fixture(false);
    document["layouts"] = serde_json::json!([
        { "id": "keys", "name": "Keys", "boardId": "board",
          "matrixIds": ["keys-matrix"],
          "partIds": ["a", "c", "mirror-a", "mirror-c"] }
    ]);
    document["matrices"] = serde_json::json!([{
        "id": "keys-matrix", "boardId": "board", "definitionId": "switch",
        "rows": 1, "columns": 1, "origin": { "x": 0.0, "y": 0.0 },
        "pitch": { "x": 19.05, "y": 19.05 }, "partIds": [],
        "cells": [{ "row": 0, "column": 0, "enabled": false }]
    }]);
    document
}

fn islands(scene: &SceneDelta, board_id: &str) -> usize {
    scene
        .board_contours
        .iter()
        .find(|entry| entry.board_id == board_id)
        .expect("the board has contours")
        .contours
        .iter()
        .filter(|contour| !contour.hole)
        .count()
}

fn findings_ending(scene: &SceneDelta, suffix: &str) -> Vec<String> {
    scene
        .findings
        .iter()
        .filter(|finding| finding.id.ends_with(suffix))
        .map(|finding| finding.id.clone())
        .collect()
}

/// A linked refinement of a split board is valid: the expected region count comes from
/// the current structure, not from the value accepted before the split.
#[test]
fn a_split_linked_refinement_derives_its_region_count_and_never_blocks_as_disconnected() {
    let mut engine = CoreEngine::new();
    let opened = open(&mut engine, split_fixture(false));
    assert_eq!(
        islands(&opened, "board"),
        2,
        "one island per mirrored half: {:?}",
        opened
            .findings
            .iter()
            .map(|finding| finding.id.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        findings_ending(&opened, ":disconnected"),
        Vec::<String>::new(),
        "the derived count accepts the two islands"
    );
    assert!(
        readiness(&opened, "board").outline && readiness(&opened, "board").pcb,
        "a split refinement stays valid output"
    );
    // Bridges are recalculated inside each half; none crosses the split axis at x = 50.
    let bridges = &opened
        .board_outline_scenes
        .iter()
        .find(|scene| scene.board_id == "board")
        .expect("the board has an outline scene")
        .bridges;
    assert!(
        bridges.len() >= 2,
        "each half recalculates its own bridges: {:?}",
        bridges
    );
    for bridge in bridges {
        assert!(
            bridge.points.iter().all(|point| point.x < 50.0)
                || bridge.points.iter().all(|point| point.x > 50.0),
            "bridge {} crosses the split axis: {:?}",
            bridge.id,
            bridge.points
        );
    }
}

/// Merging the halves back into one layout is valid again, and neither a fixed edited
/// version nor a generated outline changes its region-count semantics.
#[test]
fn merged_fixed_and_generated_outlines_keep_their_region_count_semantics() {
    let mut engine = CoreEngine::new();
    let merged = open(&mut engine, merged_fixture());
    assert_eq!(islands(&merged, "board"), 1);
    assert_eq!(
        findings_ending(&merged, ":disconnected"),
        Vec::<String>::new()
    );
    assert!(readiness(&merged, "board").outline && readiness(&merged, "board").pcb);

    // A fixed edited version keeps the count its authored geometry was saved with.
    let mut accepted = fixture((14.0, 0.0), 0.0);
    accepted["boardOutlines"][0]["activeVersionId"] = serde_json::json!("edited");
    accepted["boardOutlines"][0]["versions"][1]["geometry"]["features"] = serde_json::json!([
        { "kind": "polygon", "id": "board:board:version:edited:contour:0", "operation": "add",
          "points": [{ "x": -30.0, "y": -30.0 }, { "x": -10.0, "y": -30.0 },
                     { "x": -10.0, "y": -10.0 }, { "x": -30.0, "y": -10.0 }] },
        { "kind": "polygon", "id": "board:board:version:edited:contour:1", "operation": "add",
          "points": [{ "x": 10.0, "y": 10.0 }, { "x": 30.0, "y": 10.0 },
                     { "x": 30.0, "y": 30.0 }, { "x": 10.0, "y": 30.0 }] }
    ]);
    accepted["boardOutlines"][0]["versions"][1]["geometry"]["expectedRegions"] =
        serde_json::json!(2);
    let mut engine = CoreEngine::new();
    let authored = open(&mut engine, accepted.clone());
    assert_eq!(islands(&authored, "board"), 2);
    assert_eq!(
        findings_ending(&authored, ":disconnected"),
        Vec::<String>::new(),
        "the authored count of two covers the authored two islands"
    );
    accepted["boardOutlines"][0]["versions"][1]["geometry"]["expectedRegions"] =
        serde_json::json!(1);
    let mut engine = CoreEngine::new();
    let narrowed = open(&mut engine, accepted);
    assert_eq!(
        findings_ending(&narrowed, ":disconnected").len(),
        1,
        "a fixed version keeps its persisted count, so one expected island blocks two"
    );

    // Generated outlines keep today's derived behaviour.
    let mut generated = split_fixture(false);
    generated["boardOutlines"][0]["activeVersionId"] = serde_json::json!(null);
    let mut engine = CoreEngine::new();
    let generated = open(&mut engine, generated);
    assert_eq!(islands(&generated, "board"), 2);
    assert_eq!(
        findings_ending(&generated, ":disconnected"),
        Vec::<String>::new()
    );
    assert!(readiness(&generated, "board").pcb);
}

/// The last-valid fallback display must not crash or falsely block on the derived count.
#[test]
fn a_failed_split_resolution_keeps_its_last_valid_display_without_a_false_blocker() {
    let mut engine = CoreEngine::new();
    let failed = open(&mut engine, split_fixture(true));
    assert!(
        failed
            .findings
            .iter()
            .any(|finding| finding.id == "board:board:outline:linked:refined"),
        "the resolution failure is still reported: {:?}",
        failed
            .findings
            .iter()
            .map(|finding| finding.id.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        findings_ending(&failed, ":disconnected"),
        Vec::<String>::new(),
        "the derived count must not add a second, false blocker on the fallback display"
    );
    assert!(
        !failed.board_contours[0].contours.is_empty(),
        "a display remains"
    );
}

/// A document with a 1x2 matrix whose keys the linked choices of `fixture_with_choices`
/// name, so a matrix operation removes components that linked choices require.
fn matrix_choices_fixture() -> serde_json::Value {
    let text = serde_json::to_string(&fixture_with_choices())
        .unwrap()
        .replace("\"a\"", "\"matrix/main/r0c0\"")
        .replace("\"b\"", "\"matrix/main/r0c1\"");
    let mut document: serde_json::Value = serde_json::from_str(&text).unwrap();
    document["matrices"] = serde_json::json!([{
        "id": "main", "rows": 1, "columns": 2,
        "pitch": { "x": 14.0, "y": 14.0 }, "origin": { "x": 0.0, "y": 0.0 },
        "definitionId": "switch", "boardId": "board",
        "partIds": ["matrix/main/r0c0", "matrix/main/r0c1"]
    }]);
    document
}

/// Case 3 across removal paths: deleting a matrix key removes a component through a path
/// other than `remove-parts`, and a linked refinement still drops exactly the choices that
/// required it, in the same accepted edit, rather than detaching them to world points.
#[test]
fn deleting_a_matrix_key_cleans_linked_choices_in_the_same_edit() {
    let mut engine = CoreEngine::new();
    let opened = open(&mut engine, matrix_choices_fixture());

    let removed = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({ "kind": "remove-matrix-cells", "matrixId": "main", "cells": [[0, 1]] }),
    );
    for cleaned in ["link-ab", "gap-b", CUTOUT] {
        assert!(
            removed.changed_ids.iter().any(|id| id == cleaned),
            "{cleaned} changed in the deletion edit: {:?}",
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
    assert_eq!(connection_ids, vec!["link-a"]);
    assert_eq!(geometry["features"].as_array().unwrap().len(), 1);
    assert!(
        !serde_json::to_string(&after["boardOutlines"][0]["versions"][0])
            .unwrap()
            .contains("matrix/main/r0c1"),
        "no linked choice still names the deleted key"
    );
}

/// The same cleanup holds for a removal that does not go through key deletion: removing
/// the whole matrix removes both keys, and no linked choice keeps naming them.
#[test]
fn removing_a_matrix_cleans_linked_choices_in_the_same_edit() {
    let mut engine = CoreEngine::new();
    let opened = open(&mut engine, matrix_choices_fixture());
    let removed = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({ "kind": "remove-matrix", "id": "main" }),
    );
    for cleaned in ["link-ab", "link-a", "gap-a", "gap-b", CUTOUT] {
        assert!(
            removed.changed_ids.iter().any(|id| id == cleaned),
            "{cleaned} changed in the removal edit: {:?}",
            removed.changed_ids
        );
    }
    let after = saved(&mut engine);
    let version = serde_json::to_string(&after["boardOutlines"][0]["versions"][0]).unwrap();
    assert!(!version.contains("matrix/main/"), "{version}");
}

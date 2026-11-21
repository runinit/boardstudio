use super::*;

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

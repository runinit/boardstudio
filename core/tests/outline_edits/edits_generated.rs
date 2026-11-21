use super::*;

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

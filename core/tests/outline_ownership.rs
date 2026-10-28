//! Explicit outline ownership, identity and persistence (typed Core edits ticket 11).
//!
//! Ownership is persisted per outline version, not inferred from a copied snapshot or
//! from origin provenance. A linked refinement records that it follows the board's
//! current included components; an edited copy stays fixed. Documents written before
//! this state existed keep their serialized shape and never gain a link.
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

fn version<'a>(document: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    document["boardOutlines"][0]["versions"]
        .as_array()
        .expect("saved versions")
        .iter()
        .find(|version| version["id"] == id)
        .expect("the named version exists")
}

fn document(engine: &mut CoreEngine) -> ProjectDoc {
    match engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }) {
        CoreReply::Scene { document, .. } => *document,
        reply => panic!("{reply:?}"),
    }
}

/// Two fixed versions created by this build; the serialized document is exactly the
/// shape an older build wrote, because fixed ownership is the absent default.
fn two_versions() -> (CoreEngine, serde_json::Value) {
    let mut engine = CoreEngine::new();
    let opened = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: fixture(),
    }));
    let first = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({
            // Ticket 37: raw CopyOutline is deleted; Copy is the live typed copy.
            "kind": "apply-outline-edit", "edit": {
                "target": {"kind": "board", "boardId": "main"},
                "change": {"copy": {"version_id": "refined", "name": "Linked refinement"}},
                "seed": 1, "reviewed": null
            }
        }),
    );
    edit(
        &mut engine,
        first.revision,
        serde_json::json!({
            "kind": "apply-outline-edit", "edit": {
                "target": {"kind": "board", "boardId": "main"},
                "change": {"copy": {"version_id": "edited", "name": "Edited outline"}},
                "seed": 2, "reviewed": null
            }
        }),
    );
    let saved = serde_json::to_value(document(&mut engine)).unwrap();
    (engine, saved)
}

#[test]
fn a_legacy_project_reopens_with_its_generated_outline_versions_selection_and_identities_unchanged()
{
    let (_engine, legacy) = two_versions();
    let state = &legacy["boardOutlines"][0];
    assert_eq!(state["activeVersionId"], "edited");
    for version in state["versions"].as_array().unwrap() {
        assert!(
            version.get("ownership").is_none(),
            "the fixture is a legacy document: fixed ownership is not written"
        );
    }
    assert!(state.get("retiredVersionIds").is_none());
    let input: ProjectDoc = serde_json::from_value(legacy.clone()).unwrap();
    let mut engine = CoreEngine::new();
    let opened = scene(engine.handle(CoreRequest::Open {
        id: "legacy".into(),
        document: input,
    }));
    let after = serde_json::to_value(document(&mut engine)).unwrap();
    let reopened = &after["boardOutlines"][0];

    assert_eq!(
        reopened["versions"], state["versions"],
        "geometry, identities and source references are unchanged"
    );
    assert_eq!(
        reopened["activeVersionId"], "edited",
        "selection is unchanged"
    );
    for version in reopened["versions"].as_array().unwrap() {
        assert!(
            version.get("ownership").is_none(),
            "absence of link state must not opt an existing fixed snapshot into linkage"
        );
    }
    assert!(!opened.board_contours.is_empty(), "the saved shape renders");

    // The permanent Generated version stays available and follows the layout, while the
    // legacy fixed copies keep the geometry they were saved with.
    let generated = edit(
        &mut engine,
        opened.revision,
        // Ticket 37: raw SelectOutline is deleted; Activate is the live typed select.
        serde_json::json!({"kind": "apply-outline-edit", "edit": {
            "target": {"kind": "board", "boardId": "main"},
            "change": "activate", "seed": 3, "reviewed": null}}),
    );
    assert!(
        generated.board_readiness[0].outline,
        "Generated should be valid after clearing the legacy fixed selection: {:?}",
        generated.findings
    );
    let moved = edit(
        &mut engine,
        generated.revision,
        serde_json::json!({
            "kind": "move-parts",
            "positions": [{ "id": "main/U1", "at": { "x": 24.0, "y": 35.0 } }]
        }),
    );
    assert_ne!(
        moved.board_contours, generated.board_contours,
        "Generated follows the component layout"
    );
    let fixed = edit(
        &mut engine,
        moved.revision,
        serde_json::json!({"kind": "apply-outline-edit", "edit": {
            "target": {"kind": "version", "boardId": "main", "versionId": "edited"},
            "change": "activate", "seed": 4, "reviewed": null}}),
    );
    assert_eq!(
        fixed.board_contours, opened.board_contours,
        "an existing fixed copy keeps the geometry it was saved with"
    );
    let after = serde_json::to_value(document(&mut engine)).unwrap();
    assert_eq!(
        after["boardOutlines"][0]["versions"], state["versions"],
        "moving components does not rewrite saved versions"
    );
}

#[test]
fn linked_and_fixed_versions_keep_their_own_ownership_settings_connections_gaps_and_sources() {
    let (_engine, mut saved) = two_versions();
    let versions = saved["boardOutlines"][0]["versions"]
        .as_array_mut()
        .expect("two saved versions");
    assert_eq!(versions.len(), 2);
    // A refinement follows components: it is linked and owns its own settings, protected
    // gap and the component references those choices require.
    versions[0]["ownership"] = serde_json::json!("linked");
    versions[0]["geometry"]["settings"]["size"] = serde_json::json!(3.5);
    versions[0]["geometry"]["settings"]["repair"]["keepGaps"] = serde_json::json!([]);
    versions[0]["geometry"]["protectedGaps"] = serde_json::json!([{
        "id": "refined-gap",
        "points": [{ "at": { "x": 1.0, "y": 2.0 }, "partId": "main/U1" }]
    }]);
    versions[1]["geometry"]["settings"]["size"] = serde_json::json!(6.0);
    saved["boardOutlines"][0]["activeVersionId"] = serde_json::json!("refined");
    let input: ProjectDoc = serde_json::from_value(saved.clone()).unwrap();

    let mut engine = CoreEngine::new();
    scene(engine.handle(CoreRequest::Open {
        id: "reopen".into(),
        document: input,
    }));
    let after = serde_json::to_value(document(&mut engine)).unwrap();
    let _ = after["boardOutlines"][0]["versions"].as_array();

    assert_eq!(
        version(&after, "refined")["ownership"],
        "linked",
        "a linked refinement must reopen linked; origin provenance is not link authority"
    );
    assert!(
        version(&after, "edited").get("ownership").is_none(),
        "an edited copy stays fixed and keeps the older document's shape"
    );
    assert_eq!(version(&after, "refined")["name"], "Linked refinement");
    assert_eq!(version(&after, "edited")["name"], "Edited outline");
    assert_eq!(
        version(&after, "refined")["geometry"]["settings"]["size"],
        3.5,
        "each version keeps its own settings"
    );
    assert_eq!(
        version(&after, "edited")["geometry"]["settings"]["size"],
        6.0
    );
    assert_eq!(
        version(&after, "refined")["geometry"]["protectedGaps"][0]["points"][0]["partId"],
        "main/U1",
        "a linked version keeps the component references its choices require"
    );
    assert!(
        version(&after, "edited")["geometry"]
            .get("protectedGaps")
            .is_none()
    );
    assert_eq!(
        version(&after, "refined")["source"]["versionId"],
        serde_json::Value::Null,
        "the refinement was copied from Generated"
    );
    let source = &version(&after, "edited")["source"];
    assert_eq!(
        source["versionId"], "refined",
        "a copy records its origin without gaining control over its shape"
    );
    assert_eq!(after["boardOutlines"][0]["activeVersionId"], "refined");
}

/// A linked refinement persisted by this format: it follows the board's components
/// and owns its choices, and the fixture records that with explicit ownership.
fn linked_source() -> (ProjectDoc, serde_json::Value) {
    let (_engine, mut saved) = two_versions();
    saved["boardOutlines"][0]["versions"][0]["ownership"] = serde_json::json!("linked");
    saved["boardOutlines"][0]["versions"][0]["geometry"]["settings"]["size"] =
        serde_json::json!(3.5);
    saved["boardOutlines"][0]["versions"][0]["geometry"]["protectedGaps"] = serde_json::json!([{
        "id": "refined-gap",
        "points": [{ "at": { "x": 1.0, "y": 2.0 }, "partId": "main/U1" }]
    }]);
    saved["boardOutlines"][0]["activeVersionId"] = serde_json::json!("refined");
    let input: ProjectDoc = serde_json::from_value(saved.clone()).unwrap();
    (input, saved)
}

#[test]
fn origin_provenance_does_not_make_an_edited_copy_follow_its_source_version_or_components() {
    let (input, saved) = linked_source();
    let mut engine = CoreEngine::new();
    let opened = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: input,
    }));
    let copied = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({
            "kind": "apply-outline-edit", "edit": {
                "target": {"kind": "board", "boardId": "main"},
                "change": {"copy": {"version_id": "copy", "name": "Edited copy"}},
                "seed": 5, "reviewed": null
            }
        }),
    );
    assert_eq!(copied.board_contours, opened.board_contours);
    let accepted = document(&mut engine);
    let after = serde_json::to_value(&accepted).unwrap();
    assert_eq!(
        version(&after, "refined")["ownership"],
        "linked",
        "the persisted refinement owns its link"
    );
    assert_eq!(
        version(&after, "copy")["source"]["versionId"],
        "refined",
        "a copy records where it came from"
    );
    assert!(
        version(&after, "copy").get("ownership").is_none(),
        "origin information grants the source no control over the copy's shape"
    );
    let copy_geometry = version(&after, "copy")["geometry"].clone();

    // Editing the source version leaves the copy's own choices alone.
    let mut feature = accepted.board_outlines[0].versions[0].geometry.features[0].clone();
    if let OutlineFeature::Polygon { points, .. } = &mut feature {
        points[0].x -= 2.0;
    }
    let source_edited = edit(
        &mut engine,
        copied.revision,
        // Ticket 37: raw SetOutline is deleted; SetFeature on the version is live.
        serde_json::json!({ "kind": "apply-outline-edit", "edit": {
            "target": {"kind": "version", "boardId": "main", "versionId": "refined"},
            "change": {"set-feature": {"feature": feature}},
            "seed": 6, "reviewed": null } }),
    );
    let after = serde_json::to_value(document(&mut engine)).unwrap();
    assert_ne!(
        version(&after, "refined")["geometry"]["features"][0],
        saved["boardOutlines"][0]["versions"][0]["geometry"]["features"][0],
        "the source version really changed"
    );
    assert_eq!(
        version(&after, "copy")["geometry"],
        copy_geometry,
        "the copy does not follow its source's later edits"
    );

    // Generated follows the layout while the edited copy stays where it was.
    let generated = edit(
        &mut engine,
        source_edited.revision,
        serde_json::json!({ "kind": "apply-outline-edit", "edit": {
            "target": {"kind": "board", "boardId": "main"},
            "change": "activate", "seed": 7, "reviewed": null } }),
    );
    let moved = edit(
        &mut engine,
        generated.revision,
        serde_json::json!({
            "kind": "move-parts",
            "positions": [{ "id": "main/U1", "at": { "x": 24.0, "y": 35.0 } }]
        }),
    );
    assert_ne!(
        moved.board_contours, generated.board_contours,
        "Generated follows the component layout"
    );
    let fixed = edit(
        &mut engine,
        moved.revision,
        serde_json::json!({ "kind": "apply-outline-edit", "edit": {
            "target": {"kind": "version", "boardId": "main", "versionId": "copy"},
            "change": "activate", "seed": 8, "reviewed": null } }),
    );
    assert_eq!(
        fixed.board_contours, opened.board_contours,
        "the edited copy keeps the shape it was created with"
    );
    let after = serde_json::to_value(document(&mut engine)).unwrap();
    assert_eq!(
        version(&after, "copy")["geometry"],
        copy_geometry,
        "moving components does not reshape an edited copy"
    );
    assert_eq!(
        version(&after, "copy")["source"]["versionId"],
        "refined",
        "origin information survives later edits"
    );
}

#[test]
fn a_linked_refinement_may_keep_live_component_attachments_that_a_fixed_copy_may_not() {
    let (mut input, _saved) = linked_source();
    // An authored cutout that follows its component is legitimate for a refinement and
    // invalid for a fixed copy, which cannot contain live attachments.
    input.board_outlines[0].versions[0]
        .geometry
        .features
        .push(OutlineFeature::Polygon {
            anchor_part_id: Some("main/U1".into()),
            id: "refined-cutout".into(),
            points: vec![
                Vec2 { x: -2.0, y: -2.0 },
                Vec2 { x: 2.0, y: -2.0 },
                Vec2 { x: 0.0, y: 2.0 },
            ],
            operation: Operation::Add,
        });
    input.board_outlines[0].active_version_id = Some("refined".into());
    let mut engine = CoreEngine::new();
    let refinement = scene(engine.handle(CoreRequest::Open {
        id: "linked-attachment".into(),
        document: input,
    }));
    assert!(
        refinement.board_readiness[0].outline,
        "a linked refinement's component attachment must not block its outline: {:?}",
        refinement.findings
    );
    // The same attachment on a fixed copy stays an error.
    let (mut fixed, _saved) = linked_source();
    fixed.board_outlines[0].versions[1]
        .geometry
        .features
        .push(OutlineFeature::Polygon {
            anchor_part_id: Some("main/U1".into()),
            id: "fixed-attachment".into(),
            points: vec![
                Vec2 { x: -2.0, y: -2.0 },
                Vec2 { x: 2.0, y: -2.0 },
                Vec2 { x: 0.0, y: 2.0 },
            ],
            operation: Operation::Add,
        });
    fixed.board_outlines[0].active_version_id = Some("edited".into());
    let blocked = scene(CoreEngine::new().handle(CoreRequest::Open {
        id: "fixed-attachment".into(),
        document: fixed,
    }));
    assert!(
        !blocked.board_readiness[0].outline,
        "a fixed copy cannot contain live component attachments"
    );
}

fn error(reply: CoreReply) -> String {
    match reply {
        CoreReply::Error { message, .. } => message,
        reply => panic!("expected a refused edit, got {reply:?}"),
    }
}

fn copy(engine: &mut CoreEngine, revision: u64, version_id: &str) -> CoreReply {
    engine.handle(CoreRequest::Edit {
        id: "copy-outline".into(),
        command: EditCommand {
            base_revision: revision,
            transaction_id: "copy-outline".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation: serde_json::from_value(serde_json::json!({
                // Ticket 37: raw CopyOutline is deleted; Copy is the live typed copy.
                "kind": "apply-outline-edit", "edit": {
                    "target": {"kind": "board", "boardId": "main"},
                    "change": {"copy": {"version_id": version_id, "name": version_id}},
                    "seed": 9, "reviewed": null
                }
            }))
            .expect("outline command contract"),
        },
    })
}

#[test]
fn a_removed_version_identity_cannot_be_reused_so_a_pending_edit_can_never_reach_a_later_version() {
    let mut engine = CoreEngine::new();
    let opened = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: fixture(),
    }));
    let first = edit(
        &mut engine,
        opened.revision,
        serde_json::json!({
            "kind": "apply-outline-edit", "edit": {
                "target": {"kind": "board", "boardId": "main"},
                "change": {"copy": {"version_id": "refined", "name": "Refined"}},
                "seed": 11, "reviewed": null
            }
        }),
    );
    let feature_id = document(&mut engine).board_outlines[0].versions[0]
        .geometry
        .features[0]
        .id()
        .to_owned();
    // An edit that names the version's feature identity lands while the version exists.
    let mut feature = document(&mut engine).board_outlines[0].versions[0]
        .geometry
        .features[0]
        .clone();
    if let OutlineFeature::Polygon { points, .. } = &mut feature {
        points[0].x -= 1.0;
    }
    let named = edit(
        &mut engine,
        first.revision,
        serde_json::json!({ "kind": "apply-outline-edit", "edit": {
            "target": {"kind": "version", "boardId": "main", "versionId": "refined"},
            "change": {"set-feature": {"feature": feature}},
            "seed": 12, "reviewed": null } }),
    );
    let removed = edit(
        &mut engine,
        named.revision,
        serde_json::json!({ "kind": "apply-outline-edit", "edit": {
            "target": {"kind": "version", "boardId": "main", "versionId": "refined"},
            "change": "delete", "seed": 13, "reviewed": null } }),
    );
    let after = serde_json::to_value(document(&mut engine)).unwrap();
    assert_eq!(
        after["boardOutlines"][0]["retiredVersionIds"],
        serde_json::json!(["refined"]),
        "a removed version identity is reserved"
    );
    // Reissuing the removed identity would let a queued edit that names `feature_id`
    // reach whatever version took its place.
    let refused = error(copy(&mut engine, removed.revision, "refined"));
    assert!(
        refused.contains("refined"),
        "a removed identity is refused with a reason: {refused}"
    );
    let after = serde_json::to_value(document(&mut engine)).unwrap();
    assert!(
        !after["boardOutlines"][0]["versions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|version| version["id"] == "refined"),
        "the removed identity is not reissued"
    );
    assert!(
        !after["boardOutlines"][0]["versions"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|version| version["geometry"]["features"].as_array().unwrap())
            .any(|feature| feature["id"] == feature_id.as_str()),
        "no later version reuses the retired feature identity"
    );
    // A fresh identity is still accepted.
    let fresh = copy(&mut engine, removed.revision, "replacement");
    scene(fresh);
    let after = serde_json::to_value(document(&mut engine)).unwrap();
    assert_eq!(version(&after, "replacement")["name"], "replacement");
}

#[test]
fn a_new_version_identity_cannot_collide_with_another_persisted_outline_namespace() {
    // Ticket 37: raw SetOutline is deleted. The authored generated state below is
    // setup, not behavior: hand-edit the snapshotted document the same way the
    // fixed-copy suite persists invalid inactive geometry, then reopen.
    let mut engine = CoreEngine::new();
    scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: fixture(),
    }));
    let mut authored = serde_json::to_value(document(&mut engine)).unwrap();
    let outline = authored["outline"]
        .as_array_mut()
        .expect("generated outline");
    outline.push(serde_json::json!({
        "kind": "polygon", "id": "saved-shape",
        "points": [
            { "x": 0.5, "y": 0.5 }, { "x": 2.5, "y": 0.5 }, { "x": 1.5, "y": 2.5 }
        ],
        "operation": "subtract"
    }));
    let envelope = outline
        .iter_mut()
        .find(|feature| feature.get("partIds").is_some())
        .expect("the generated outline");
    envelope["connections"] = serde_json::json!([{
        "id": "connection-identity", "width": 2.0, "points": [
            { "at": { "x": 1.0, "y": 2.0 }, "partId": envelope["partIds"][0].clone() },
            { "at": { "x": 3.0, "y": 4.0 }, "partId": null }
        ]
    }]);
    envelope["settings"]["repair"] = serde_json::json!({"keepGaps": [{
        "id": "gap-identity", "points": [
            { "at": { "x": 1.0, "y": 2.0 }, "partId": envelope["partIds"][0].clone() },
            { "at": { "x": 4.0, "y": 2.0 }, "partId": null },
            { "at": { "x": 1.0, "y": 5.0 }, "partId": null }
        ]
    }]});
    let mut engine = CoreEngine::new();
    let opened = scene(engine.handle(CoreRequest::Open {
        id: "authored-identities".into(),
        document: serde_json::from_value(authored).unwrap(),
    }));
    // The reopened document carries the authored connection and protected gap as
    // persisted identities beside version IDs.
    let generated = opened;
    let copied = edit(
        &mut engine,
        generated.revision,
        serde_json::json!({
            "kind": "apply-outline-edit", "edit": {
                "target": {"kind": "board", "boardId": "main"},
                "change": {"copy": {"version_id": "first", "name": "First"}},
                "seed": 14, "reviewed": null
            }
        }),
    );
    let accepted = document(&mut engine);
    let saved_version = &accepted.board_outlines[0].versions[0];
    let version_feature_id = saved_version.geometry.features[0].id().to_owned();
    let gap_id = saved_version.geometry.protected_gaps[0].id.clone();
    assert_eq!(gap_id, "gap-identity");

    for identity in [
        "saved-shape",
        version_feature_id.as_str(),
        "connection-identity",
        gap_id.as_str(),
    ] {
        let message = error(copy(&mut engine, copied.revision, identity));
        assert!(
            message.contains(identity),
            "the refusal names the colliding identity `{identity}`: {message}"
        );
    }
    let after = serde_json::to_value(document(&mut engine)).unwrap();
    assert_eq!(
        after["boardOutlines"][0]["versions"]
            .as_array()
            .unwrap()
            .len(),
        1,
        "no refused creation added a version"
    );
    // A distinct identity beside those namespaces is accepted.
    copy(&mut engine, copied.revision, "distinct-identity");
    let after = serde_json::to_value(document(&mut engine)).unwrap();
    assert_eq!(
        version(&after, "distinct-identity")["name"],
        "distinct-identity"
    );
}

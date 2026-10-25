//! Outline ownership and geometry across Core's persistence boundaries (ticket 11).
//!
//! The archive boundary and browser persistence use the same document boundary: browser
//! storage loads through [`boardstudio_core::migrate`] and writes back the serialized
//! accepted document, exactly as an archive round trip does. Both must reproduce the
//! same explicit ownership and geometry, and a document this build writes must already
//! be at the current integrated format version.
use boardstudio_core::model::{
    ArchiveAssetBuffer, ArchiveReply, CoreReply, CoreRequest, EditCommand, EditPhase, ProjectDoc,
    SceneDelta,
};
use boardstudio_core::{CoreEngine, archive, migrate};
use sha2::{Digest, Sha256};

fn scene(reply: CoreReply) -> SceneDelta {
    match reply {
        CoreReply::Scene { scene, .. } => scene,
        reply => panic!("{reply:?}"),
    }
}

fn accepted(engine: &mut CoreEngine) -> ProjectDoc {
    match engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }) {
        CoreReply::Scene { document, .. } => *document,
        reply => panic!("{reply:?}"),
    }
}

fn edit(engine: &mut CoreEngine, revision: u64, operation: &serde_json::Value) -> SceneDelta {
    let operation = serde_json::from_value(operation.clone()).expect("outline command contract");
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

fn unpack_project(bytes: &[u8]) -> (String, Vec<ArchiveAssetBuffer>, Vec<Vec<u8>>) {
    let (reply, buffers) = archive::request(r#"{"kind":"unpack-project"}"#, &[bytes.to_vec()]);
    match serde_json::from_str(&reply).expect("archive reply") {
        ArchiveReply::Unpacked {
            project_json,
            assets,
        } => (project_json, assets, buffers),
        other => panic!("expected an unpacked project, got {other:?}"),
    }
}

/// A stored document carrying linked ownership and an edited copy, built from the small
/// authored outline fixture and a synthetic archive asset.
fn stored_document() -> (String, Vec<ArchiveAssetBuffer>, Vec<Vec<u8>>) {
    let mut document: ProjectDoc =
        serde_json::from_str(include_str!("fixtures/synthetic-outline.json"))
            .expect("synthetic outline fixture");
    document.format_version = migrate::CURRENT_VERSION;
    let asset_bytes = b"synthetic persistence asset".to_vec();
    let hash = Sha256::digest(&asset_bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let mut document_value = serde_json::to_value(&document).unwrap();
    document_value["assets"] = serde_json::json!([{
        "id": "synthetic/persistence-asset",
        "name": "persistence.bin",
        "mediaType": "application/octet-stream",
        "sha256": hash
    }]);
    document = serde_json::from_value(document_value).unwrap();
    let assets = vec![ArchiveAssetBuffer {
        sha256: hash,
        buffer_index: 0,
    }];
    let buffers = vec![asset_bytes];
    let mut engine = CoreEngine::new();
    let opened = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document,
    }));
    let linked = edit(
        &mut engine,
        opened.revision,
        &serde_json::json!({
            "kind": "apply-outline-edit", "edit": {
                "target": {"kind": "board", "boardId": "main"},
                "change": {"copy": {"version_id": "refined", "name": "Linked refinement"}},
                "seed": 31, "reviewed": null
            }
        }),
    );
    edit(
        &mut engine,
        linked.revision,
        &serde_json::json!({
            "kind": "apply-outline-edit", "edit": {
                "target": {"kind": "board", "boardId": "main"},
                "change": {"copy": {"version_id": "edited", "name": "Edited outline"}},
                "seed": 32, "reviewed": null
            }
        }),
    );
    let mut stored = serde_json::to_value(accepted(&mut engine)).unwrap();
    let versions = stored["boardOutlines"][0]["versions"]
        .as_array_mut()
        .unwrap();
    versions[0]["ownership"] = serde_json::json!("linked");
    versions[0]["geometry"]["settings"]["size"] = serde_json::json!(3.5);
    versions[0]["geometry"]["protectedGaps"] = serde_json::json!([{
        "id": "refined-gap",
        "points": [{ "at": { "x": 1.0, "y": 2.0 }, "partId": "main/U1" }]
    }]);
    versions[1]["geometry"]["settings"]["size"] = serde_json::json!(6.0);
    stored["boardOutlines"][0]["activeVersionId"] = serde_json::json!("refined");
    // A document that has already removed a version keeps that identity reserved.
    stored["boardOutlines"][0]["retiredVersionIds"] = serde_json::json!(["retired-version"]);
    (serde_json::to_string(&stored).unwrap(), assets, buffers)
}

/// Browser persistence: the host migrates stored JSON at load and writes back the
/// serialized accepted document, then reads it again on the next load.
fn browser_writeback(stored: &str) -> ProjectDoc {
    let (loaded, _) = migrate::migrate_json(stored).unwrap_or_else(|error| panic!("{error}"));
    let document: ProjectDoc = serde_json::from_str(&loaded).unwrap();
    let written_back = serde_json::to_string(&serde_json::to_value(&document).unwrap()).unwrap();
    serde_json::from_str(&written_back).unwrap()
}

#[test]
fn archive_migration_and_browser_writeback_agree_on_ownership_reservation_and_geometry() {
    let (stored, assets, buffers) = stored_document();
    let before: serde_json::Value = serde_json::from_str(&stored).unwrap();

    // Archive path: pack the stored document with its assets, then unpack it again.
    let entries = assets
        .iter()
        .map(|asset| {
            serde_json::json!({
                "path": format!("assets/{}", asset.sha256),
                "bufferIndex": asset.buffer_index,
            })
        })
        .collect::<Vec<_>>();
    let (reply, packed) = archive::request(
        &serde_json::json!({
            "kind": "pack-project", "projectJson": stored, "assets": entries
        })
        .to_string(),
        &buffers,
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&reply).unwrap()["kind"],
        "packed",
        "{reply}"
    );
    let (archived_json, _, _) = unpack_project(&packed[0]);

    // Browser path: the same stored JSON through the storage load and writeback.
    let mut archive_engine = CoreEngine::new();
    let archive_scene = scene(archive_engine.handle(CoreRequest::Open {
        id: "archive".into(),
        document: serde_json::from_str(&archived_json).unwrap(),
    }));
    let mut browser_engine = CoreEngine::new();
    let browser_scene = scene(browser_engine.handle(CoreRequest::Open {
        id: "browser".into(),
        document: browser_writeback(&stored),
    }));

    let archive_document = serde_json::to_value(accepted(&mut archive_engine)).unwrap();
    let browser_document = serde_json::to_value(accepted(&mut browser_engine)).unwrap();
    assert_eq!(
        archive_document["boardOutlines"], browser_document["boardOutlines"],
        "both transports must persist the same ownership, reservation and geometry"
    );
    assert_eq!(
        archive_scene.board_contours, browser_scene.board_contours,
        "both transports must render the same geometry"
    );

    let versions = archive_document["boardOutlines"][0]["versions"]
        .as_array()
        .unwrap();
    let version = |id: &str| {
        versions
            .iter()
            .find(|version| version["id"] == id)
            .unwrap_or_else(|| panic!("version {id}"))
    };
    assert_eq!(
        version("refined")["ownership"],
        "linked",
        "the link survives both transports"
    );
    assert!(
        version("edited").get("ownership").is_none(),
        "the edited copy stays fixed through both transports"
    );
    assert_eq!(
        archive_document["boardOutlines"][0]["retiredVersionIds"],
        before["boardOutlines"][0]["retiredVersionIds"],
        "the identity reservation survives both transports"
    );
    for id in ["refined", "edited"] {
        assert_eq!(
            version(id)["geometry"],
            before["boardOutlines"][0]["versions"]
                .as_array()
                .unwrap()
                .iter()
                .find(|version| version["id"] == id)
                .unwrap()["geometry"],
            "transport does not rewrite version {id}'s geometry"
        );
    }
    // Nothing is left to migrate in either transport's document.
    assert_eq!(
        migrate::migrate_json(&archived_json).unwrap(),
        (archived_json.clone(), false)
    );
}

#[test]
fn a_document_this_build_writes_is_at_the_current_integrated_format_version() {
    let (stored, _, _) = stored_document();
    let document: ProjectDoc = serde_json::from_str(&stored).unwrap();
    assert_eq!(
        document.format_version,
        migrate::CURRENT_VERSION,
        "a new project writes the integrated current format"
    );
    assert_eq!(
        migrate::migrate_json(&stored).unwrap(),
        (stored.clone(), false),
        "the current format needs no migration"
    );

    // An empty new project is written the same way, and reopens losslessly.
    let mut engine = CoreEngine::new();
    let empty = ProjectDoc::empty("new-project", "New project");
    let text = serde_json::to_string(&empty).unwrap();
    assert_eq!(empty.format_version, migrate::CURRENT_VERSION);
    assert_eq!(migrate::migrate_json(&text).unwrap(), (text.clone(), false));
    scene(engine.handle(CoreRequest::Open {
        id: "new".into(),
        document: serde_json::from_str(&text).unwrap(),
    }));

    // A version created in the current format keeps no migration debt either.
    let mut engine = CoreEngine::new();
    let mut fixture: ProjectDoc =
        serde_json::from_str(include_str!("fixtures/synthetic-outline.json")).unwrap();
    fixture.format_version = migrate::CURRENT_VERSION;
    let opened = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: fixture,
    }));
    let created = edit(
        &mut engine,
        opened.revision,
        &serde_json::json!({
            "kind": "apply-outline-edit", "edit": {
                "target": {"kind": "board", "boardId": "main"},
                "change": {"copy": {"version_id": "fresh", "name": "Fresh"}},
                "seed": 33, "reviewed": null
            }
        }),
    );
    assert!(
        created.revision > opened.revision,
        "the created version is an accepted revision"
    );
    assert_eq!(
        accepted(&mut engine).board_outlines[0].versions.len(),
        1,
        "the created version is persisted"
    );
    let text = serde_json::to_string(&accepted(&mut engine)).unwrap();
    assert_eq!(
        migrate::migrate_json(&text).unwrap(),
        (text, false),
        "a project created by this build already writes the current format"
    );
}

/// The saved versions' features are addressed by their own identities, so both
/// transports keep the identities the accepted document reports.
#[test]
fn both_transports_keep_version_and_feature_identities() {
    let (stored, _, _) = stored_document();
    let document: ProjectDoc = serde_json::from_str(&stored).unwrap();
    let ids = |document: &ProjectDoc| {
        document.board_outlines[0]
            .versions
            .iter()
            .map(|version| {
                (
                    version.id.clone(),
                    version
                        .geometry
                        .features
                        .iter()
                        .map(|feature| feature.id().to_owned())
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>()
    };
    let expected = ids(&document);
    assert!(
        expected.iter().all(|(_, features)| !features.is_empty()),
        "the fixture persisted contour identities"
    );
    let mut engine = CoreEngine::new();
    scene(engine.handle(CoreRequest::Open {
        id: "browser".into(),
        document: browser_writeback(&stored),
    }));
    assert_eq!(ids(&accepted(&mut engine)), expected);
}

/// A version 2 document that already carries outline versions migrates to the current
/// integrated version without adding, dropping or reshaping them.
#[test]
fn refusing_an_older_document_preserves_its_outline_versions_and_identity_reservations() {
    let (stored, _, _) = stored_document();
    let mut older: serde_json::Value = serde_json::from_str(&stored).unwrap();
    older["formatVersion"] = serde_json::json!(2);
    let original = older.to_string();
    let error = migrate::migrate_json(&original).unwrap_err();
    assert!(
        error.contains("format 2") && error.contains("No conversion"),
        "{error}"
    );
    assert_eq!(older.to_string(), original);
    let mut engine = CoreEngine::new();
    let before = scene(engine.handle(CoreRequest::Open {
        id: "valid".into(),
        document: browser_writeback(&stored),
    }));
    assert!(matches!(
        engine.handle(CoreRequest::Open {
            id: "old".into(),
            document: serde_json::from_value(older).unwrap()
        }),
        CoreReply::Error { .. }
    ));
    let after = scene(engine.handle(CoreRequest::Snapshot {
        id: "after-refusal".into(),
    }));
    assert_eq!(after.board_contours, before.board_contours);
}

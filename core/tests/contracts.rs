use boardstudio_core::model::*;
use serde_json::json;

#[test]
fn project_document_serialization_keeps_format_and_numeric_revision() {
    let document = ProjectDoc::empty("contract", "Contract fixture");
    let value = serde_json::to_value(document).unwrap();

    assert_eq!(value["format"], "boardstudio/v2");
    assert_eq!(value["revision"], 0);
    assert_eq!(value["constraints"], json!([]));
}

#[test]
fn optional_and_defaulted_fields_keep_their_json_behavior() {
    let envelope = serde_json::to_value(EnvelopeSource::default()).unwrap();
    assert_eq!(envelope, json!({ "courtyard": null, "keycap": null }));

    let outline = serde_json::to_value(PartOutline::default()).unwrap();
    assert_eq!(outline, json!({ "excluded": false }));

    let pad = Pad {
        id: "pad".into(),
        number: "1".into(),
        at: Vec2::default(),
        size: Vec2::default(),
        shape: PadShape::Circle,
        drill: None,
        plated: None,
        side: None,
        rotation: None,
        net_id: None,
    };
    let pad = serde_json::to_value(pad).unwrap();
    assert!(pad.get("drill").is_none());
    assert!(pad.get("netId").is_none());
}

#[test]
fn case_protocol_uses_stable_hyphenated_tags_and_ir_fields() {
    let request = CoreRequest::PrepareCase {
        id: "request".into(),
        ir: CaseAssemblyIR {
            revision: 8,
            bodies: vec![],
        },
    };
    let request = serde_json::to_value(request).unwrap();
    assert_eq!(request["kind"], "prepare-case");
    assert_eq!(request["ir"]["revision"], 8);

    let reply = CoreReply::CasePrepared {
        id: "request".into(),
        ir: PreparedCaseAssemblyIR {
            revision: 8,
            bodies: vec![],
        },
    };
    let reply = serde_json::to_value(reply).unwrap();
    assert_eq!(reply["kind"], "case-prepared");
    assert_eq!(reply["ir"]["revision"], 8);
}

#[test]
fn artifact_import_protocol_preserves_authoritative_source_in_document_json() {
    let source = "(footprint \"Fixture\" (layer \"F.Cu\") (fp_rect (start -2 -2) (end 2 2) (layer \"F.CrtYd\") (width 0.05)))";
    let request = ArtifactRequest::ImportFootprint {
        id: "import-1".into(),
        definition_id: "fixture".into(),
        source: source.into(),
    };
    let request_value = serde_json::to_value(request).unwrap();
    assert_eq!(request_value["kind"], "import-footprint");
    assert_eq!(request_value["definitionId"], "fixture");
    let reply: ArtifactReply = serde_json::from_str(&boardstudio_core::artifact::request(
        &request_value.to_string(),
    ))
    .unwrap();
    let ArtifactReply::ImportFootprint {
        result: imported, ..
    } = reply
    else {
        panic!("expected imported footprint reply");
    };
    let mut authored_envelope = imported.definition.clone();
    authored_envelope
        .envelope_source
        .as_mut()
        .unwrap()
        .courtyard = Some(EnvelopeOrigin::Authored);
    authored_envelope.courtyard = vec![
        Vec2 { x: -10.0, y: -4.0 },
        Vec2 { x: 10.0, y: -4.0 },
        Vec2 { x: 10.0, y: 4.0 },
        Vec2 { x: -10.0, y: 4.0 },
    ];
    let projected: ArtifactReply = serde_json::from_str(&boardstudio_core::artifact::request(
        &serde_json::to_string(&ArtifactRequest::CompileFootprints {
            id: "authored-envelope".into(),
            jobs: vec![FootprintCompileJob {
                id: "authored-envelope-job".into(),
                definition: authored_envelope.clone(),
                side: Side::Back,
            }],
        })
        .unwrap(),
    ))
    .unwrap();
    let ArtifactReply::CompileFootprints {
        result: compiled, ..
    } = projected
    else {
        panic!("expected compiled imported footprint reply");
    };
    assert_eq!(compiled[0].geometry.courtyard, authored_envelope.courtyard);
    assert_eq!(compiled[0].geometry.side, Side::Back);
    assert_eq!(
        compiled[0].definition.kicad_source,
        authored_envelope.kicad_source
    );

    let mut unsupported = imported.definition.clone();
    unsupported.kicad_source.as_mut().unwrap().format_version = 2;
    let unsupported_reply: ArtifactReply =
        serde_json::from_str(&boardstudio_core::artifact::request(
            &serde_json::to_string(&ArtifactRequest::CompileFootprints {
                id: "unsupported".into(),
                jobs: vec![FootprintCompileJob {
                    id: "unsupported-job".into(),
                    definition: unsupported,
                    side: Side::Front,
                }],
            })
            .unwrap(),
        ))
        .unwrap();
    assert!(matches!(
        unsupported_reply,
        ArtifactReply::Error {
            error: ArtifactError {
                code: ArtifactErrorCode::Unsupported,
                ..
            },
            ..
        }
    ));
    let mut document = ProjectDoc::empty("source-fixture", "Source fixture");
    document.definitions.push(imported.definition);

    let serialized = serde_json::to_string(&document).unwrap();
    let value: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    assert_eq!(value["definitions"][0]["kicadSource"]["source"], source);
    let restored: ProjectDoc = serde_json::from_str(&serialized).unwrap();
    assert_eq!(
        restored.definitions[0]
            .kicad_source
            .as_ref()
            .unwrap()
            .source,
        source
    );
    assert_eq!(restored, document);
    let mut authored = serde_json::to_value(ProjectDoc::empty("authored", "Authored")).unwrap();
    authored["definitions"] = serde_json::json!([{
        "id":"authored-part", "name":"Authored", "kind":"custom", "courtyard":[], "pads":[]
    }]);
    let authored: ProjectDoc = serde_json::from_value(authored).unwrap();
    assert!(authored.definitions[0].kicad_source.is_none());

    let mut engine = boardstudio_core::CoreEngine::new();
    engine.request(
        &serde_json::to_string(&CoreRequest::Open {
            id: "open".into(),
            document: document.clone(),
        })
        .unwrap(),
    );
    let mut replacement = document.clone();
    replacement.name = "Replaced".into();
    let command = EditCommand {
        base_revision: 0,
        transaction_id: "replace".into(),
        phase: EditPhase::Commit,
        target_ids: vec![],
        operation: EditOperation::ReplaceDocument {
            document: replacement,
        },
    };
    let commit: CoreReply = serde_json::from_str(
        &engine.request(
            &serde_json::to_string(&CoreRequest::Edit {
                id: "replace".into(),
                command,
            })
            .unwrap(),
        ),
    )
    .unwrap();
    assert!(matches!(commit, CoreReply::Scene { .. }));
    let undo: CoreReply = serde_json::from_str(
        &engine.request(&serde_json::to_string(&CoreRequest::Undo { id: "undo".into() }).unwrap()),
    )
    .unwrap();
    let CoreReply::Scene {
        document: undone, ..
    } = undo
    else {
        panic!("expected undo scene");
    };
    assert_eq!(
        undone.definitions[0].kicad_source.as_ref().unwrap().source,
        source
    );
    let redo: CoreReply = serde_json::from_str(
        &engine.request(&serde_json::to_string(&CoreRequest::Redo { id: "redo".into() }).unwrap()),
    )
    .unwrap();
    let CoreReply::Scene {
        document: redone, ..
    } = redo
    else {
        panic!("expected redo scene");
    };
    assert_eq!(
        redone.definitions[0].kicad_source.as_ref().unwrap().source,
        source
    );
}

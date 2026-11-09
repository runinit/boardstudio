//! Strict archive admission and deliberately authored current-schema assets.
use boardstudio_core::model::{ArchiveReply, CoreReply, CoreRequest, ProjectDoc};
use boardstudio_core::{CoreEngine, archive, generators, migrate};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::io::{Cursor, Write};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

const ASSET: &[u8] = b"synthetic model bytes";

fn archive_bytes(current: bool) -> Vec<u8> {
    let hash = Sha256::digest(ASSET)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let mut document = serde_json::to_value(ProjectDoc::empty(
        "synthetic-legacy",
        "Synthetic legacy project",
    ))
    .unwrap();
    document.as_object_mut().unwrap().remove("formatVersion");
    let mut definition = serde_json::to_value(
        generators::catalogue()
            .unwrap()
            .into_iter()
            .find(|definition| definition.id == "generator:ceoloide/switch_mx")
            .expect("bundled switch definition"),
    )
    .unwrap();
    definition["id"] = json!("ergogen:ceoloide/switch_mx");
    definition["models"] = json!([{
        "assetId": "ergogen:model:synthetic/switch.step",
        "offset": {"x": 0.0, "y": 0.0, "z": 0.0},
        "rotation": {"x": 0.0, "y": 0.0, "z": 0.0},
        "scale": {"x": 1.0, "y": 1.0, "z": 1.0}
    }]);
    document["definitions"] = json!([definition]);
    document["parts"] = json!([{
        "id": "synthetic/back-switch",
        "definitionId": "ergogen:ceoloide/switch_mx",
        "reference": "SW1",
        "pose": {"at": {"x": 4.0, "y": 7.0}, "rotation": 0.0},
        "side": "back"
    }]);
    document["assets"] = json!([{
        "id": "ergogen:model:synthetic/switch.step",
        "name": "switch.step",
        "mediaType": "model/step",
        "sha256": hash,
        "source": "bundled Ergogen library"
    }]);

    if current {
        // Rebuilt current fixture: identities and back-side placement are authored, not migrated.
        document["formatVersion"] = json!(migrate::CURRENT_VERSION);
        document["definitions"][0]["id"] = json!("generator:ceoloide/switch_mx");
        document["definitions"][0]["models"][0]["assetId"] =
            json!("bundled-model:synthetic/switch.step");
        document["parts"][0]["definitionId"] = json!("generator:ceoloide/switch_mx");
        document["parts"][0]["generatorParameters"] = json!({"side": "B"});
        document["assets"][0]["id"] = json!("bundled-model:synthetic/switch.step");
        document["assets"][0]["source"] = json!("Synthetic test asset");
    }
    let project = serde_json::to_vec(&document).unwrap();
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    writer.start_file("project.json", options).unwrap();
    writer.write_all(&project).unwrap();
    writer
        .start_file(format!("assets/{hash}"), options)
        .unwrap();
    writer.write_all(ASSET).unwrap();
    writer.finish().unwrap().into_inner()
}

fn unpacked() -> String {
    let (reply, buffers) = archive::request(r#"{"kind":"unpack-project"}"#, &[archive_bytes(true)]);
    assert_eq!(
        buffers,
        vec![ASSET.to_vec()],
        "the synthetic asset survives"
    );
    match serde_json::from_str(&reply).expect("archive reply") {
        ArchiveReply::Unpacked { project_json, .. } => project_json,
        other => panic!("expected an unpacked project, got {other:?}"),
    }
}

#[test]
fn an_unversioned_archive_is_refused_without_releasing_assets() {
    let input = archive_bytes(false);
    let (reply, buffers) = archive::request(r#"{"kind":"unpack-project"}"#, &[input]);
    assert!(
        buffers.is_empty(),
        "refused import must release no asset buffers"
    );
    match serde_json::from_str(&reply).unwrap() {
        ArchiveReply::Error { message } => assert!(
            message.contains("legacy") && message.contains("No conversion"),
            "{message}"
        ),
        other => panic!("expected refusal, got {other:?}"),
    }
}

#[test]
fn back_side_generator_parts_keep_their_layer() {
    let document: ProjectDoc = serde_json::from_str(&unpacked()).unwrap();
    let part = document
        .parts
        .iter()
        .find(|part| part.id == "synthetic/back-switch")
        .unwrap();
    assert_eq!(format!("{:?}", part.side), "Back");
    assert_eq!(
        part.generator_parameters
            .as_ref()
            .and_then(|parameters| parameters.get("side")),
        Some(&json!("B"))
    );
}

#[test]
fn the_rebuilt_current_project_opens_and_every_generated_part_renders() {
    let document: ProjectDoc = serde_json::from_str(&unpacked()).unwrap();
    for part in &document.parts {
        let definition = document
            .definitions
            .iter()
            .find(|definition| definition.id == part.definition_id)
            .expect("definition");
        if definition.generator.is_some() {
            generators::render_forms(definition, Some(part))
                .unwrap_or_else(|error| panic!("{}: {error}", part.id));
        }
    }
    match CoreEngine::new().handle(CoreRequest::Open {
        id: "synthetic-legacy".into(),
        document,
    }) {
        CoreReply::Scene { .. } => {}
        other => panic!("current project must open: {other:?}"),
    }
}

#[test]
fn an_unmigrated_document_is_refused_by_open() {
    let mut document: ProjectDoc = serde_json::from_str(&unpacked()).unwrap();
    document.format_version = migrate::LEGACY_VERSION;
    match CoreEngine::new().handle(CoreRequest::Open {
        id: "synthetic-legacy".into(),
        document,
    }) {
        CoreReply::Error { message, .. } => {
            assert!(message.contains("No conversion is available"), "{message}")
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
}

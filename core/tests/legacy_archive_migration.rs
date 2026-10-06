//! A version 1 archive (`content/archives/reviung41-original.boardstudio`, saved before
//! documents carried a format version) loads through Core's archive boundary as a
//! version 2 project that opens and renders.
use boardstudio_core::model::{ArchiveReply, CoreReply, CoreRequest, ProjectDoc};
use boardstudio_core::{CoreEngine, archive, generators, migrate};

const ARCHIVE: &[u8] = include_bytes!("../../content/archives/reviung41-original.boardstudio");

fn unpacked() -> String {
    let (reply, _) = archive::request(r#"{"kind":"unpack-project"}"#, &[ARCHIVE.to_vec()]);
    match serde_json::from_str(&reply).expect("archive reply") {
        ArchiveReply::Unpacked { project_json, .. } => project_json,
        other => panic!("expected an unpacked project, got {other:?}"),
    }
}

#[test]
fn a_version_1_archive_unpacks_as_version_2_without_the_old_name() {
    let json = unpacked();
    let document: ProjectDoc = serde_json::from_str(&json).expect("migrated project decodes");
    assert_eq!(document.format_version, migrate::CURRENT_VERSION);
    assert!(
        !json.contains("ergogen"),
        "no persisted identifier keeps the old name"
    );
    assert!(json.contains("bundled-model:kiswitch/SW_Cherry_MX_PCB.stp"));
    // Unpacking again changes nothing: the migration is idempotent.
    assert_eq!(migrate::migrate_json(&json).unwrap(), (json, false));
}

#[test]
fn back_side_generator_parts_keep_their_layer() {
    let document: ProjectDoc = serde_json::from_str(&unpacked()).unwrap();
    let back = document
        .parts
        .iter()
        .filter(|part| format!("{:?}", part.side) == "Back")
        .filter(|part| {
            document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)
                .is_some_and(|definition| definition.generator.is_some())
        })
        .collect::<Vec<_>>();
    assert!(!back.is_empty());
    for part in back {
        let definition = document
            .definitions
            .iter()
            .find(|definition| definition.id == part.definition_id)
            .unwrap();
        if definition
            .generator
            .as_ref()
            .unwrap()
            .parameters
            .contains_key("side")
        {
            continue;
        }
        let side = part
            .generator_parameters
            .as_ref()
            .and_then(|parameters| parameters.get("side"));
        assert_eq!(side, Some(&serde_json::json!("B")), "{}", part.id);
    }
}

#[test]
fn the_migrated_project_opens_and_every_generated_part_renders() {
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
        id: "legacy".into(),
        document,
    }) {
        CoreReply::Scene { .. } => {}
        other => panic!("migrated project must open: {other:?}"),
    }
}

#[test]
fn an_unmigrated_document_is_refused_by_open() {
    let mut document: ProjectDoc = serde_json::from_str(&unpacked()).unwrap();
    document.format_version = migrate::LEGACY_VERSION;
    match CoreEngine::new().handle(CoreRequest::Open {
        id: "legacy".into(),
        document,
    }) {
        CoreReply::Error { message, .. } => assert!(message.contains("migrated"), "{message}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

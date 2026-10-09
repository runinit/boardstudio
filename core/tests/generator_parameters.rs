//! Core checks saved generator parameters at its boundary (ADR-0009).
//!
//! A value of the wrong type is refused on Open, ReplaceDocument and every edit. A name the
//! generator does not declare is a located warning on Open and ReplaceDocument, and is refused
//! on an edit. Non-empty list text for a list parameter is a warning on Open and ReplaceDocument
//! (saved data may hold it), and is refused when an edit writes it. An edit is checked only for
//! the values it writes, so a legacy warning does not block an unrelated edit.
use boardstudio_core::archive;
use boardstudio_core::generators;
use boardstudio_core::model::{
    CoreReply, CoreRequest, EditCommand, EditOperation, EditPhase, Finding, Part, PartDefinition,
    Position, ProjectDoc, Severity,
};
use boardstudio_core::{CoreEngine, migrate};
use serde_json::{Value, json};

const HOLE: &str = "ceoloide/mounting_hole_npth";
const HOLE_DEFINITION: &str = "generator:ceoloide/mounting_hole_npth";
const MCU: &str = "ceoloide/mcu_nice_nano";
const MCU_DEFINITION: &str = "generator:ceoloide/mcu_nice_nano";

fn catalogue_definition(source: &str) -> PartDefinition {
    generators::catalogue()
        .expect("catalogue")
        .into_iter()
        .find(|definition| {
            definition
                .generator
                .as_ref()
                .is_some_and(|generator| generator.source == source)
        })
        .expect("bundled generator")
}

fn part(id: &str, definition: &str, parameters: Value) -> Part {
    serde_json::from_value(json!({
        "id": id,
        "definitionId": definition,
        "reference": id.to_uppercase(),
        "side": "front",
        "pose": { "at": { "x": 0, "y": 0 }, "rotation": 0 },
        "generatorParameters": parameters,
    }))
    .expect("part")
}

fn document(parts: Vec<Part>) -> ProjectDoc {
    let mut doc = ProjectDoc::empty("generators", "Generators");
    doc.format_version = migrate::CURRENT_VERSION;
    doc.definitions.push(catalogue_definition(HOLE));
    doc.definitions.push(catalogue_definition(MCU));
    doc.parts = parts;
    doc
}

fn open(engine: &mut CoreEngine, doc: ProjectDoc) -> CoreReply {
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    })
}

fn edit(
    engine: &mut CoreEngine,
    base_revision: u64,
    phase: EditPhase,
    operation: EditOperation,
) -> CoreReply {
    engine.handle(CoreRequest::Edit {
        id: "edit".into(),
        command: EditCommand {
            base_revision,
            transaction_id: "transaction".into(),
            phase,
            target_ids: Vec::new(),
            operation,
        },
    })
}

fn add_part(part: Part) -> EditOperation {
    EditOperation::AddPart {
        part,
        board_id: None,
    }
}

fn error_message(reply: CoreReply) -> String {
    match reply {
        CoreReply::Error { message, .. } => message,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn scene_findings(reply: CoreReply) -> Vec<Finding> {
    match reply {
        CoreReply::Scene { scene, .. } => scene.findings,
        other => panic!("expected a scene, got {other:?}"),
    }
}

fn generator_findings(findings: &[Finding]) -> Vec<&Finding> {
    findings
        .iter()
        .filter(|finding| finding.id.starts_with("generator-parameter:"))
        .collect()
}

#[test]
fn open_refuses_text_for_a_number_and_names_the_generator_and_parameter() {
    let mut engine = CoreEngine::new();
    let doc = document(vec![part(
        "h1",
        HOLE_DEFINITION,
        json!({ "hole_size": "2" }),
    )]);
    let message = error_message(open(&mut engine, doc));
    assert!(message.contains(HOLE), "{message}");
    assert!(message.contains("hole_size"), "{message}");
}

#[test]
fn open_warns_on_an_undeclared_name_with_a_finding_on_the_part() {
    let mut engine = CoreEngine::new();
    let doc = document(vec![part(
        "h1",
        HOLE_DEFINITION,
        json!({ "hole_sise": 2.0 }),
    )]);
    let findings = scene_findings(open(&mut engine, doc));
    let found = generator_findings(&findings);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].severity, Severity::Warning);
    assert_eq!(found[0].target_ids, vec!["h1".to_string()]);
    assert!(
        found[0].message.contains("hole_sise"),
        "{}",
        found[0].message
    );
    assert!(found[0].message.contains(HOLE), "{}", found[0].message);
}

#[test]
fn open_keeps_non_empty_list_text_as_a_warning_not_a_refusal() {
    let mut engine = CoreEngine::new();
    let doc = document(vec![part(
        "mcu1",
        MCU_DEFINITION,
        json!({ "mcu_3dmodel_xyz_offset": "[1,2,3]" }),
    )]);
    let findings = scene_findings(open(&mut engine, doc));
    let found = generator_findings(&findings);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].severity, Severity::Warning);
    assert_eq!(found[0].target_ids, vec!["mcu1".to_string()]);
    assert!(
        found[0].message.contains("mcu_3dmodel_xyz_offset"),
        "{}",
        found[0].message
    );
}

#[test]
fn an_edit_that_writes_text_for_a_number_is_refused_and_applies_nothing() {
    let mut engine = CoreEngine::new();
    assert!(matches!(
        open(&mut engine, document(Vec::new())),
        CoreReply::Scene { .. }
    ));
    let bad = part("h1", HOLE_DEFINITION, json!({ "hole_size": "2" }));
    let message = error_message(edit(&mut engine, 0, EditPhase::Commit, add_part(bad)));
    assert!(message.contains(HOLE), "{message}");
    assert!(message.contains("hole_size"), "{message}");
    // Nothing was applied, so the same base revision still takes a valid edit.
    let good = part("h1", HOLE_DEFINITION, json!({ "hole_size": 2.0 }));
    assert!(matches!(
        edit(&mut engine, 0, EditPhase::Commit, add_part(good)),
        CoreReply::Scene { .. }
    ));
}

#[test]
fn an_edit_that_writes_an_undeclared_name_is_refused() {
    let mut engine = CoreEngine::new();
    assert!(matches!(
        open(&mut engine, document(Vec::new())),
        CoreReply::Scene { .. }
    ));
    let bad = part("h1", HOLE_DEFINITION, json!({ "hole_sise": 2.0 }));
    let message = error_message(edit(&mut engine, 0, EditPhase::Commit, add_part(bad)));
    assert!(message.contains("hole_sise"), "{message}");
    assert!(message.contains(HOLE), "{message}");
}

#[test]
fn an_edit_that_writes_list_text_is_refused() {
    let mut engine = CoreEngine::new();
    assert!(matches!(
        open(&mut engine, document(Vec::new())),
        CoreReply::Scene { .. }
    ));
    let bad = part(
        "mcu1",
        MCU_DEFINITION,
        json!({ "mcu_3dmodel_xyz_offset": "[1,2,3]" }),
    );
    let message = error_message(edit(&mut engine, 0, EditPhase::Commit, add_part(bad)));
    assert!(message.contains("mcu_3dmodel_xyz_offset"), "{message}");
}

#[test]
fn a_preview_that_writes_text_for_a_number_is_refused_the_same_way() {
    let mut engine = CoreEngine::new();
    assert!(matches!(
        open(&mut engine, document(Vec::new())),
        CoreReply::Scene { .. }
    ));
    let bad = part("h1", HOLE_DEFINITION, json!({ "hole_size": "2" }));
    let message = error_message(edit(&mut engine, 0, EditPhase::Preview, add_part(bad)));
    assert!(message.contains("hole_size"), "{message}");
}

#[test]
fn an_unrelated_edit_is_not_refused_for_a_legacy_name_it_does_not_write() {
    let mut engine = CoreEngine::new();
    let legacy = document(vec![part(
        "h1",
        HOLE_DEFINITION,
        json!({ "hole_sise": 2.0 }),
    )]);
    assert!(matches!(open(&mut engine, legacy), CoreReply::Scene { .. }));
    let other = part("h2", HOLE_DEFINITION, json!({ "hole_size": 2.0 }));
    let reply = edit(&mut engine, 0, EditPhase::Commit, add_part(other));
    let findings = scene_findings(reply);
    let found = generator_findings(&findings);
    assert_eq!(
        found.len(),
        1,
        "the legacy warning stays, the edit is accepted"
    );
    assert_eq!(found[0].target_ids, vec!["h1".to_string()]);
}

#[test]
fn an_unrelated_edit_is_not_refused_for_legacy_list_text() {
    let mut engine = CoreEngine::new();
    let legacy = document(vec![part(
        "mcu1",
        MCU_DEFINITION,
        json!({ "mcu_3dmodel_xyz_offset": "[1,2,3]" }),
    )]);
    assert!(matches!(open(&mut engine, legacy), CoreReply::Scene { .. }));
    let other = part("h2", HOLE_DEFINITION, json!({ "hole_size": 2.0 }));
    assert!(matches!(
        edit(&mut engine, 0, EditPhase::Commit, add_part(other)),
        CoreReply::Scene { .. }
    ));
}

#[test]
fn a_move_preview_is_not_refused_for_a_legacy_name() {
    let mut engine = CoreEngine::new();
    let legacy = document(vec![part(
        "h1",
        HOLE_DEFINITION,
        json!({ "hole_sise": 2.0 }),
    )]);
    assert!(matches!(open(&mut engine, legacy), CoreReply::Scene { .. }));
    let operation = EditOperation::MoveParts {
        positions: vec![Position {
            id: "h1".into(),
            at: boardstudio_core::model::Vec2 { x: 1.0, y: 2.0 },
        }],
    };
    assert!(matches!(
        edit(&mut engine, 0, EditPhase::Preview, operation),
        CoreReply::Preview { .. }
    ));
}

#[test]
fn replace_document_refuses_text_for_a_number() {
    let mut engine = CoreEngine::new();
    assert!(matches!(
        open(&mut engine, document(Vec::new())),
        CoreReply::Scene { .. }
    ));
    let replacement = document(vec![part(
        "h1",
        HOLE_DEFINITION,
        json!({ "hole_size": "2" }),
    )]);
    let message = error_message(edit(
        &mut engine,
        0,
        EditPhase::Commit,
        EditOperation::ReplaceDocument {
            document: Box::new(replacement),
        },
    ));
    assert!(message.contains("hole_size"), "{message}");
}

#[test]
fn replace_document_warns_on_an_undeclared_name_instead_of_refusing() {
    let mut engine = CoreEngine::new();
    assert!(matches!(
        open(&mut engine, document(Vec::new())),
        CoreReply::Scene { .. }
    ));
    let replacement = document(vec![part(
        "h1",
        HOLE_DEFINITION,
        json!({ "hole_sise": 2.0 }),
    )]);
    let findings = scene_findings(edit(
        &mut engine,
        0,
        EditPhase::Commit,
        EditOperation::ReplaceDocument {
            document: Box::new(replacement),
        },
    ));
    let found = generator_findings(&findings);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].severity, Severity::Warning);
}

#[test]
fn the_shipped_reviung41_archive_opens_without_a_generator_finding() {
    let bytes = include_bytes!("../../content/archives/reviung41-original.boardstudio").to_vec();
    let (reply, _buffers) = archive::request(r#"{"kind":"unpack-project"}"#, &[bytes]);
    let reply: Value = serde_json::from_str(&reply).unwrap();
    assert_eq!(reply["kind"], "unpacked", "{reply}");
    let project: Value = serde_json::from_str(reply["projectJson"].as_str().unwrap()).unwrap();
    let mut engine = CoreEngine::new();
    let opened: Value = serde_json::from_str(
        &engine.request(&json!({ "id": "open", "kind": "open", "document": project }).to_string()),
    )
    .unwrap();
    assert_eq!(opened["kind"], "scene", "{opened}");
    assert!(
        opened["scene"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|finding| !finding["id"]
                .as_str()
                .unwrap()
                .starts_with("generator-parameter:")),
        "{opened}"
    );
}

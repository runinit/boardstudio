//! Core checks saved generator parameters at its boundary (ADR-0009).
//!
//! A value of the wrong type is refused on Open, ReplaceDocument and every edit. Two things are
//! warnings on Open and ReplaceDocument instead, because saved data can hold them: a name the
//! generator does not declare, and text where a number or a list is declared (the version 4
//! migration keeps both). An edit refuses all of these, but only for the values it writes, so a
//! legacy warning does not block an unrelated edit.
use crate::included_keyboards;
use boardstudio_core::archive;
use boardstudio_core::artifact::kicad;
use boardstudio_core::generators;
use boardstudio_core::mechanical::MechanicalTarget;
use boardstudio_core::mechanical::inputs::effective_document;
use boardstudio_core::model::{
    CoreReply, CoreRequest, EditCommand, EditOperation, EditPhase, Finding, Matrix, Part,
    PartDefinition, Position, ProjectDoc, Severity,
};
use boardstudio_core::{CoreEngine, migrate};
use boardstudio_footprints::bundled;
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
fn open_refuses_a_boolean_for_a_number_and_names_the_generator_and_parameter() {
    let mut engine = CoreEngine::new();
    let doc = document(vec![part(
        "h1",
        HOLE_DEFINITION,
        json!({ "hole_size": true }),
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
fn replace_document_refuses_a_boolean_for_a_number() {
    let mut engine = CoreEngine::new();
    assert!(matches!(
        open(&mut engine, document(Vec::new())),
        CoreReply::Scene { .. }
    ));
    let replacement = document(vec![part(
        "h1",
        HOLE_DEFINITION,
        json!({ "hole_size": true }),
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

const CHOC: &str = "ceoloide/switch_choc_v1_v2";
const CHOC_DEFINITION: &str = "generator:ceoloide/switch_choc_v1_v2";
const ENCODER: &str = "ceoloide/rotary_encoder_ec11_ec12";
const ENCODER_DEFINITION: &str = "generator:ceoloide/rotary_encoder_ec11_ec12";

fn matrix(definition: &str) -> Matrix {
    serde_json::from_value(json!({
        "id": "main", "rows": 1, "columns": 1,
        "pitch": { "x": 19, "y": 19 }, "origin": { "x": 0, "y": 0 },
        "definitionId": definition, "partIds": [],
    }))
    .expect("matrix")
}

fn scene_document(reply: CoreReply) -> ProjectDoc {
    match reply {
        CoreReply::Scene { document, .. } => *document,
        other => panic!("expected a scene, got {other:?}"),
    }
}

/// Saved version 3 data as an older Parts editor could have left it, after the migration the
/// page runs before Open: it keeps the text that is not a number and the name no generator declares.
fn migrated_version_3_with_unconvertible_text() -> Value {
    let mut definition = catalogue_definition(HOLE);
    definition
        .generator
        .as_mut()
        .unwrap()
        .parameters
        .insert("hole_size".into(), json!("two"));
    let held = part(
        "hole",
        HOLE_DEFINITION,
        json!({ "hole_size": " 3.5 ", "not_a_parameter": "4" }),
    );
    let mut saved = serde_json::to_value(ProjectDoc::empty("saved", "Saved")).unwrap();
    saved["formatVersion"] = json!(3);
    saved["definitions"] = json!([definition]);
    saved["parts"] = json!([held]);
    migrate::migrate_value(saved).expect("migrates").0
}

#[test]
fn a_migrated_project_that_keeps_unconvertible_number_text_opens_with_a_warning() {
    let mut engine = CoreEngine::new();
    let opened: Value = serde_json::from_str(&engine.request(
        &json!({ "id": "open", "kind": "open", "document": migrated_version_3_with_unconvertible_text() })
            .to_string(),
    ))
    .unwrap();
    assert_eq!(
        opened["kind"], "scene",
        "migration keeps it, so Open must: {opened}"
    );
    let found: Vec<&Value> = opened["scene"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| {
            finding["id"]
                .as_str()
                .unwrap()
                .starts_with("generator-parameter:")
        })
        .collect();
    let text = found
        .iter()
        .find(|finding| finding["message"].as_str().unwrap().contains("hole_size"))
        .unwrap_or_else(|| panic!("a finding names hole_size: {found:?}"));
    assert_eq!(text["severity"], "warning", "{text}");
    assert_eq!(text["targetIds"], json!([HOLE_DEFINITION]), "{text}");
    assert!(
        found.iter().any(|finding| finding["message"]
            .as_str()
            .unwrap()
            .contains("not_a_parameter")),
        "{found:?}"
    );
}

#[test]
fn a_project_that_opened_with_number_text_still_takes_replace_document_and_unrelated_edits() {
    let mut engine = CoreEngine::new();
    let opened: Value = serde_json::from_str(&engine.request(
        &json!({ "id": "open", "kind": "open", "document": migrated_version_3_with_unconvertible_text() })
            .to_string(),
    ))
    .unwrap();
    assert_eq!(opened["kind"], "scene", "{opened}");
    let document: ProjectDoc = serde_json::from_value(opened["document"].clone()).unwrap();
    // The page replaces the whole document for many settings; the legacy text is still in it.
    let mut renamed = document.clone();
    renamed.name = "Renamed".into();
    let replaced = edit(
        &mut engine,
        document.revision,
        EditPhase::Commit,
        EditOperation::ReplaceDocument {
            document: Box::new(renamed),
        },
    );
    let document = scene_document(replaced);
    assert_eq!(document.name, "Renamed");
    let other = part("h2", HOLE_DEFINITION, json!({ "hole_size": 2.0 }));
    assert!(matches!(
        edit(
            &mut engine,
            document.revision,
            EditPhase::Commit,
            add_part(other)
        ),
        CoreReply::Scene { .. }
    ));
}

#[test]
fn text_for_a_number_is_still_refused_when_an_edit_writes_it() {
    let mut engine = CoreEngine::new();
    let opened: Value = serde_json::from_str(&engine.request(
        &json!({ "id": "open", "kind": "open", "document": migrated_version_3_with_unconvertible_text() })
            .to_string(),
    ))
    .unwrap();
    let revision = opened["document"]["revision"].as_u64().unwrap();
    // The legacy text is "two"; writing a different text value is a new bad value.
    let bad = part("h3", HOLE_DEFINITION, json!({ "hole_size": "three" }));
    let message = error_message(edit(
        &mut engine,
        revision,
        EditPhase::Commit,
        add_part(bad),
    ));
    assert!(message.contains("hole_size"), "{message}");
}

#[test]
fn a_copy_of_a_part_with_a_legacy_name_is_accepted_and_a_new_typo_is_not() {
    let mut engine = CoreEngine::new();
    let legacy = document(vec![part(
        "h1",
        HOLE_DEFINITION,
        json!({ "hole_sise": 2.0 }),
    )]);
    assert!(matches!(open(&mut engine, legacy), CoreReply::Scene { .. }));
    // A copy repeats the value the document already holds; the user cannot remove the name
    // from the Parts editor, so refusing the copy would be a dead end.
    let copy = part("h2", HOLE_DEFINITION, json!({ "hole_sise": 2.0 }));
    let document = scene_document(edit(&mut engine, 0, EditPhase::Commit, add_part(copy)));
    let revision = document.revision;
    let typo = part("h3", HOLE_DEFINITION, json!({ "hole_sise2": 2.0 }));
    let message = error_message(edit(
        &mut engine,
        revision,
        EditPhase::Commit,
        add_part(typo),
    ));
    assert!(message.contains("hole_sise2"), "{message}");
}

#[test]
fn changing_a_matrix_key_to_another_generator_drops_parameters_it_does_not_declare() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("keys", "Keys");
    doc.format_version = migrate::CURRENT_VERSION;
    doc.definitions.push(catalogue_definition(CHOC));
    doc.definitions.push(catalogue_definition(ENCODER));
    assert!(matches!(open(&mut engine, doc), CoreReply::Scene { .. }));
    let placed = scene_document(edit(
        &mut engine,
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: matrix(CHOC_DEFINITION),
            definitions: None,
        },
    ));
    assert_eq!(placed.parts.len(), 1, "one key is placed");
    // Electrical wiring stores each terminal's net as a parameter of the placed key.
    let mut wired = placed.clone();
    wired.parts[0].generator_parameters =
        Some(serde_json::from_value(json!({ "from": "ROW0", "to": "COL0" })).unwrap());
    assert!(matches!(
        open(&mut engine, wired.clone()),
        CoreReply::Scene { .. }
    ));
    for phase in [EditPhase::Preview, EditPhase::Commit] {
        let reply = edit(
            &mut engine,
            wired.revision,
            phase,
            EditOperation::SetMatrix {
                matrix: matrix(ENCODER_DEFINITION),
                definitions: None,
            },
        );
        match (phase, reply) {
            (EditPhase::Commit, CoreReply::Scene { document, .. }) => {
                let key = &document.parts[0];
                assert_eq!(key.definition_id, ENCODER_DEFINITION);
                let parameters = key.generator_parameters.clone().unwrap_or_default();
                assert!(!parameters.contains_key("from"), "{parameters:?}");
                assert!(!parameters.contains_key("to"), "{parameters:?}");
            }
            (EditPhase::Preview, CoreReply::Preview { .. }) => {}
            (phase, other) => panic!("{phase:?}: the switch change is refused: {other:?}"),
        }
    }
}

#[test]
fn a_matrix_preview_that_brings_a_wrong_typed_definition_is_refused_and_leaves_no_trace() {
    let mut engine = CoreEngine::new();
    assert!(matches!(
        open(&mut engine, document(Vec::new())),
        CoreReply::Scene { .. }
    ));
    // `SetMatrix` previews run through the preview path, which has its own check.
    let mut bad = catalogue_definition(CHOC);
    bad.id = "generator:bad-choc".into();
    bad.generator
        .as_mut()
        .unwrap()
        .parameters
        .insert("hotswap".into(), json!("yes"));
    let message = error_message(edit(
        &mut engine,
        0,
        EditPhase::Preview,
        EditOperation::SetMatrix {
            matrix: matrix("generator:bad-choc"),
            definitions: Some(vec![bad]),
        },
    ));
    assert!(message.contains("hotswap"), "{message}");
    assert!(message.contains(CHOC), "{message}");
    // The refused preview restored the document: the same base revision takes a valid edit.
    let good = part("h1", HOLE_DEFINITION, json!({ "hole_size": 2.0 }));
    assert!(matches!(
        edit(&mut engine, 0, EditPhase::Commit, add_part(good)),
        CoreReply::Scene { .. }
    ));
}

#[test]
fn every_terminal_of_a_bundled_generator_is_a_declared_net_parameter() {
    // Electrical wiring writes each terminal's net into the placed part's parameters, so a
    // terminal that is not a declared net would make that write fail the edit check.
    for definition in generators::catalogue().expect("catalogue") {
        let Some(generator) = &definition.generator else {
            continue;
        };
        let schema = generators::parameter_schema(&generator.source).expect("schema");
        for terminal in definition.terminals.keys() {
            assert_eq!(
                schema.get(terminal).map(|spec| &spec["type"]),
                Some(&json!("net")),
                "{} terminal {terminal}",
                generator.source
            );
        }
    }
}

/// Asserts that a check found nothing, naming the first problem and how many there were.
fn assert_accepted(problems: &[generators::GeneratorParameterProblem], context: &str) {
    assert!(
        problems.is_empty(),
        "{context}: {} problems, first {:?}",
        problems.len(),
        problems.first()
    );
}

/// Every value a Core writer puts into a generator parameter passes the check (ADR-0009).
///
/// The writers, in each included split keyboard and the reused PCB, for each half: the closure
/// holes are numbers on their definitions (ticket 01), electrical wiring and KiCad planning write
/// terminal net names, and a flipped half's effective read writes `side` as `F` or `B`
/// (`mechanical/inputs.rs`). The saved document, the effective read, the captured planning
/// document and every job of the export plan are judged, so a value the check refuses fails here.
#[test]
fn every_value_a_core_writer_produces_passes_the_parameter_check() {
    for (name, project) in included_keyboards::projects() {
        let typed: ProjectDoc = serde_json::from_value(project.clone()).unwrap();
        assert_eq!(
            generators::parameter_problems(&typed),
            Vec::new(),
            "{name}: the saved document"
        );
        let instances = project["hardware"]["instances"].as_array().unwrap();
        assert!(!instances.is_empty(), "{name} has no physical instance");
        for instance in instances {
            let half = instance["id"].as_str().unwrap();
            let target = MechanicalTarget::Instance {
                instance_id: half.into(),
            };
            let (effective, _) = effective_document(&typed, &target).unwrap();
            assert_accepted(
                &generators::parameter_problems(&effective),
                &format!("{name} {half}: the effective read"),
            );
            // An instance that shares another instance's board (the reused PCB's flipped side)
            // has no export of its own; its effective read above is its check.
            if instance["boardId"] != instance["id"] {
                continue;
            }
            let plan = kicad::prepare_export(included_keyboards::export_request(project, half))
                .unwrap_or_else(|error| panic!("{name} {half}: {}", error.message));
            assert_accepted(
                &generators::parameter_problems(&plan.captured_document),
                &format!("{name} {half}: the captured document"),
            );
            for job in &plan.jobs {
                assert_eq!(
                    generators::definition_error(&job.definition),
                    Ok(()),
                    "{name} {half}: definition of {}",
                    job.part.id
                );
                let source = &job.definition.generator.as_ref().unwrap().source;
                let parameters = job.part.generator_parameters.clone().unwrap_or_default();
                assert_accepted(
                    &bundled()
                        .check_parameters(source, &parameters)
                        .unwrap()
                        .into_iter()
                        .map(|problem| generators::GeneratorParameterProblem {
                            owner: generators::ParameterOwner::Part(job.part.id.clone()),
                            generator: source.clone(),
                            problem,
                        })
                        .collect::<Vec<_>>(),
                    &format!("{name} {half}: part {}", job.part.id),
                );
            }
        }
    }
}

/// Applying a board's wiring writes each terminal's net name into the part's parameters
/// (`electrical::materialize`). Run on every board of the bundled projects that names a
/// controller, the saved document it returns must still pass the check, and the terminal nets
/// must actually have been written (so the test cannot pass by writing nothing).
#[test]
fn applying_a_boards_wiring_writes_only_values_the_parameter_check_accepts() {
    let (mut applied, mut written) = (0, 0);
    for (name, project) in included_keyboards::projects() {
        let typed: ProjectDoc = serde_json::from_value(project.clone()).unwrap();
        let boards = typed.hardware.as_ref().map_or(&[][..], |h| &h.boards[..]);
        for board in boards {
            let Some(controller_id) = board.controller_part_id.clone() else {
                continue;
            };
            let controller = typed
                .parts
                .iter()
                .find(|part| part.id == controller_id)
                .unwrap_or_else(|| panic!("{name}: controller part {controller_id}"));
            let source = typed
                .definitions
                .iter()
                .find(|definition| definition.id == controller.definition_id)
                .and_then(|definition| definition.generator.as_ref())
                .map(|generator| generator.source.clone());
            let mut engine = CoreEngine::new();
            let CoreReply::Scene { document, .. } = engine.handle(CoreRequest::Open {
                id: "open".into(),
                document: typed.clone(),
            }) else {
                panic!("{name}: the project does not open");
            };
            let plan = boardstudio_core::electrical::resolve(
                boardstudio_core::electrical::ElectricalPlanRequest {
                    document: (*document).clone(),
                    instance_id: None,
                    mode: board.mode,
                    locks: Default::default(),
                    controller_profile: source,
                    board_id: Some(board.board_id.clone()),
                    controller_part_id: Some(controller_id.clone()),
                },
            );
            let reply = engine.handle(CoreRequest::ApplyElectrical {
                id: "apply".into(),
                base_revision: document.revision,
                plan,
                draft: true,
            });
            let CoreReply::Scene { document, .. } = reply else {
                panic!("{name} {}: applying the wiring: {reply:?}", board.board_id);
            };
            assert_accepted(
                &generators::parameter_problems(&document),
                &format!("{name} {}: after applying the wiring", board.board_id),
            );
            written += document
                .parts
                .iter()
                .filter(|part| {
                    let terminals = document
                        .definitions
                        .iter()
                        .find(|definition| definition.id == part.definition_id)
                        .map(|definition| &definition.terminals);
                    terminals.is_some_and(|terminals| {
                        terminals.keys().any(|terminal| {
                            part.generator_parameters
                                .as_ref()
                                .and_then(|parameters| parameters.get(terminal))
                                .is_some_and(Value::is_string)
                        })
                    })
                })
                .count();
            applied += 1;
        }
    }
    assert!(applied > 0, "no bundled board names a controller");
    assert!(
        written > 0,
        "applying the wiring wrote no terminal net name"
    );
}

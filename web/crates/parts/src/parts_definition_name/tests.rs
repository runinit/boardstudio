use super::*;
use crate::parts_custom_definition::DefinitionPanelCapture;
use boardstudio_application::{Durability, Event, OperationId, TerminalOutcome};
use boardstudio_model::model::PartDefinition;
use boardstudio_model::{
    model::ProjectDoc,
    model::{AssemblyDefinition, Asset, Net, Part, Pin, Pose2, Side, Vec2},
};
use boardstudio_web_runtime::{operation_outcomes::OutcomeSlot, runtime::Runtime};
use std::rc::Rc;

fn fixed_commit(
    operation_id: OperationId,
    command: boardstudio_model::model::EditCommand,
) -> Event {
    let boardstudio_model::model::EditCommand {
        transaction_id,
        target_ids,
        operation,
        ..
    } = command;
    Event::ResolveEdit {
        operation_id,
        label: "test fixed command".into(),
        resolver: EditResolver::new("test fixed command", move |_| {
            if transaction_id.is_empty() {
                Resolution::submit(target_ids.clone(), operation.clone())
            } else {
                Resolution::submit_with_transaction_id(
                    target_ids.clone(),
                    transaction_id.clone(),
                    operation.clone(),
                )
            }
        }),
    }
}

fn definition(id: &str, name: &str) -> PartDefinition {
    serde_json::from_value(serde_json::json!({
        "id": id,
        "name": name,
        "kind": "custom",
        "courtyard": [],
        "pads": []
    }))
    .unwrap()
}

fn document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("parts-name-project", "Parts name fixture");
    document.definitions = vec![
        definition("selected", "Original name"),
        definition("other", "Unaffected"),
    ];
    document.parts.push(Part {
        id: "placed-part".into(),
        definition_id: "selected".into(),
        reference: "U1".into(),
        pose: Pose2 {
            at: Vec2 { x: 5.0, y: 8.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        anchor: None,
        keycap: None,
        outline: None,
        locked: None,
        properties: None,
        generator_parameters: None,
    });
    document.nets.push(Net {
        id: "net-1".into(),
        name: "DATA".into(),
        pins: vec![Pin {
            part_id: "placed-part".into(),
            pad_id: "1".into(),
        }],
    });
    document.assets.push(Asset {
        id: "asset-1".into(),
        name: "model.step".into(),
        media_type: "model/step".into(),
        sha256: "0".repeat(64),
        license: None,
        source: None,
    });
    document.assemblies.push(AssemblyDefinition {
        id: "assembly-1".into(),
        name: "Saved assembly".into(),
        members: vec![],
    });
    document
}

fn open_document(document: ProjectDoc) -> Rc<Runtime> {
    let runtime = Runtime::with_core(boardstudio_web_test_runtime::engine_core_services());
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document,
    });
    assert!(runtime.model().accepted.is_some());
    runtime
}

/// Submit the event built for a fresh operation. The slot holds its settlement once
/// settled; a gated Runtime leaves it empty until the gate is released.
fn submit(runtime: &Runtime, event: impl FnOnce(OperationId) -> Event) -> OutcomeSlot {
    let operation = runtime.operation();
    let slot = runtime.observe_operation(operation);
    runtime.submit(event(operation));
    slot
}

fn settled(slot: &OutcomeSlot) -> Option<TerminalOutcome> {
    slot.borrow().clone()
}

fn accepted_snapshot(runtime: &Runtime) -> AcceptedSnapshot {
    runtime.model().accepted.expect("accepted document")
}

fn submit_name_edit(runtime: &Runtime, definition_id: &str, name: &str) -> OutcomeSlot {
    submit(runtime, |operation_id| Event::ResolveEdit {
        operation_id,
        label: "parts-definition-name".into(),
        resolver: definition_name_resolver(definition_id.into(), name.into()),
    })
}

fn assert_same_content_ignoring_revision(actual: &ProjectDoc, expected: &ProjectDoc) {
    let mut actual = actual.clone();
    actual.revision = expected.revision;
    assert_eq!(actual, *expected);
}

fn selected_name(document: &ProjectDoc) -> &str {
    document
        .definitions
        .iter()
        .find(|definition| definition.id == "selected")
        .unwrap()
        .name
        .as_str()
}

#[test]
fn production_name_edit_commits_one_field_and_round_trips_through_session_history() {
    let original = document();
    let runtime = open_document(original.clone());
    assert_eq!(
        settled(&submit_name_edit(&runtime, "selected", "Renamed device")),
        Some(TerminalOutcome::Completed),
        "a changed name lands as one accepted edit"
    );
    let accepted = accepted_snapshot(&runtime);
    let mut expected_renamed = original.clone();
    expected_renamed.definitions[0].name = "Renamed device".into();
    assert_same_content_ignoring_revision(&accepted.document, &expected_renamed);
    assert_eq!(
        runtime.model().durability,
        Durability::Saved {
            revision: accepted.document.revision
        }
    );

    submit(&runtime, |operation_id| Event::Undo { operation_id });
    assert_same_content_ignoring_revision(&accepted_snapshot(&runtime).document, &original);
    submit(&runtime, |operation_id| Event::Redo { operation_id });
    let redone = accepted_snapshot(&runtime).document;
    assert_same_content_ignoring_revision(&redone, &expected_renamed);

    let persisted_round_trip: ProjectDoc =
        serde_json::from_str(&serde_json::to_string(&*redone).unwrap()).unwrap();
    let reopened = open_document(persisted_round_trip);
    let reopened_document = accepted_snapshot(&reopened).document;
    assert_same_content_ignoring_revision(&reopened_document, &expected_renamed);
    assert_eq!(
        reopened_document
            .definitions
            .iter()
            .find(|definition| definition.id == "other")
            .unwrap()
            .name,
        "Unaffected"
    );
}

#[test]
fn admission_keeps_departed_owners_out_but_newer_revisions_queue_freely() {
    let runtime = open_document(document());
    let snapshot = accepted_snapshot(&runtime);
    let scope = runtime.scope();
    let selected = Some((scope.clone(), "selected".into()));
    let capture =
        DefinitionPanelCapture::new(&snapshot, scope.clone(), &snapshot.document.definitions[0]);

    assert!(capture.owner_matches(&snapshot, scope.clone(), selected.clone()));
    assert!(!capture.owner_matches(
        &snapshot,
        scope.clone(),
        Some((scope.clone(), "other".into()))
    ));
    let other_scope = scope.as_ref().map(|scope| {
        let mut changed = scope.clone();
        changed.instance_id = Some("other-instance".into());
        changed
    });
    assert!(!capture.owner_matches(&snapshot, other_scope, selected.clone()));

    let mut changed_identity_doc = snapshot.document.as_ref().clone();
    changed_identity_doc.id = "another-project".into();
    let changed_identity_snapshot = AcceptedSnapshot {
        document: std::sync::Arc::new(changed_identity_doc),
        ..snapshot.clone()
    };
    assert!(!capture.owner_matches(&changed_identity_snapshot, scope.clone(), selected));

    // A newer accepted revision is not a departed owner: field edits queue freely
    // and resolve against the document accepted when they run (ADR-0005).
    let mut newer_doc = snapshot.document.as_ref().clone();
    newer_doc.revision += 1;
    let newer = AcceptedSnapshot {
        token: boardstudio_application::SnapshotToken(snapshot.token.0 + 1),
        document: std::sync::Arc::new(newer_doc),
        ..snapshot.clone()
    };
    assert!(capture.owner_matches(
        &newer,
        runtime.scope(),
        Some((runtime.scope().clone(), "selected".into()))
    ));
}

#[test]
fn generator_definitions_and_vanished_targets_retire_a_name_commit() {
    let mut generator_doc = document();
    generator_doc.definitions[0].generator = Some(boardstudio_model::model::PartGenerator {
        source: "generator/source".into(),
        version: "1".into(),
        parameters: Default::default(),
    });
    let runtime = open_document(generator_doc);
    assert_eq!(
        settled(&submit_name_edit(&runtime, "selected", "Generator name")),
        Some(TerminalOutcome::Rejected(GENERATOR_LOCKED.into()))
    );
    assert_eq!(
        selected_name(&accepted_snapshot(&runtime).document),
        "Original name"
    );

    let runtime = open_document(document());
    assert_eq!(
        settled(&submit_name_edit(&runtime, "vanished", "No target")),
        Some(TerminalOutcome::Rejected(DEFINITION_GONE.into()))
    );
}

#[test]
fn an_unchanged_name_commit_lands_without_moving_the_revision() {
    let runtime = open_document(document());
    let before = accepted_snapshot(&runtime);
    assert_eq!(
        settled(&submit_name_edit(&runtime, "selected", "Original name")),
        Some(TerminalOutcome::Completed)
    );
    let after = accepted_snapshot(&runtime);
    assert_eq!(after.document.revision, before.document.revision);
    assert_eq!(after.token, before.token);
}

#[test]
fn a_name_commit_resolves_against_the_latest_accepted_document() {
    let runtime = open_document(document());
    let initial = accepted_snapshot(&runtime);

    let mut unrelated_document = initial.document.as_ref().clone();
    unrelated_document
        .parameters
        .insert("unrelated-edit".into(), serde_json::json!(true));
    submit(&runtime, |operation_id| {
        fixed_commit(
            operation_id,
            boardstudio_model::model::EditCommand {
                base_revision: initial.document.revision,
                transaction_id: "parts-name-unrelated-edit".into(),
                phase: boardstudio_model::model::EditPhase::Commit,
                target_ids: vec!["unrelated".into()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(unrelated_document),
                },
            },
        )
    });

    assert_eq!(
        settled(&submit_name_edit(&runtime, "selected", "Dirty name draft")),
        Some(TerminalOutcome::Completed)
    );
    let accepted = accepted_snapshot(&runtime);
    assert_eq!(
        accepted.document.parameters.get("unrelated-edit"),
        Some(&serde_json::json!(true)),
        "the name commit must retain the edit accepted while its local draft was dirty"
    );
    assert_eq!(selected_name(&accepted.document), "Dirty name draft");
}

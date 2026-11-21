use super::*;
use boardstudio_application::{EditResolver, Event, Resolution};
use boardstudio_model::model::{
    Board, EditOperation, Part, PartDefinition, PartKind, Pose2, ProjectDoc, Side, Vec2,
};
use std::rc::Rc;

#[derive(Debug, PartialEq)]
enum Field {
    Name,
    Position,
}

fn document(id: &str, name: &str) -> ProjectDoc {
    let mut document = ProjectDoc::empty(id, name);
    document.definitions.push(PartDefinition {
        mechanical_profile: None,
        id: "key".into(),
        name: "Key".into(),
        kind: PartKind::Switch,
        courtyard: vec![],
        pads: vec![],
        models: None,
        hardware_profile: None,
        input_profile: None,
        keycap: Some(Vec2 { x: 18.0, y: 18.0 }),
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        generator: None,
    });
    document.parts.push(Part {
        anchor: None,
        id: "key".into(),
        definition_id: "key".into(),
        reference: "SW1".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    });
    document.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec!["key".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document
}

fn opened_runtime() -> Rc<crate::runtime::Runtime> {
    let runtime = boardstudio_web_test_runtime::new_native_runtime();
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: document("pending-edits", "Original"),
    });
    runtime
}

fn rename_resolver(name: &'static str) -> EditResolver {
    EditResolver::new("pending-edits-test", move |accepted| {
        let mut document = (*accepted.document).clone();
        document.name = name.into();
        Resolution::submit(
            vec![document.id.clone()],
            EditOperation::ReplaceDocument {
                document: Box::new(document),
            },
        )
    })
}

fn unchanged_resolver() -> EditResolver {
    EditResolver::new("pending-edits-test", |_accepted| Resolution::Unchanged)
}

#[test]
fn latest_ticket_replaces_only_its_observation_and_both_session_edits_run() {
    let runtime = opened_runtime();
    let edits = DraftEdits::<_, &str>::default();
    runtime.hold_next_core();
    edits.submit_value(
        &runtime,
        Field::Name,
        "rename first",
        Some("project".into()),
        rename_resolver("First"),
        "First draft",
    );
    assert!(runtime.core_entered());
    assert_eq!(edits.pending_submission(&Field::Name), Some("First draft"));

    edits.submit_value(
        &runtime,
        Field::Name,
        "rename latest",
        Some("project".into()),
        rename_resolver("Latest"),
        "Latest draft",
    );
    assert_eq!(edits.pending_submission(&Field::Name), Some("Latest draft"));
    runtime.release_core();

    let accepted = runtime.model().accepted.unwrap();
    assert_eq!(accepted.document.name, "Latest");
    assert_eq!(accepted.document.revision, 2);
    let resolve_count = runtime
        .events
        .borrow()
        .iter()
        .filter(|event| matches!(event, Event::ResolveEdit { .. }))
        .count();
    assert_eq!(
        resolve_count, 2,
        "replacing observation must not cancel the first edit"
    );
    assert_eq!(
        edits.settle_values_where(|_| true),
        vec![(
            PendingEditResult::Landed {
                key: Field::Name,
                revision: 2,
            },
            "Latest draft"
        )]
    );
    assert!(!edits.is_pending(&Field::Name));
    assert!(edits.settle_values_where(|_| true).is_empty());
}

#[test]
fn independent_keys_do_not_block_each_other_and_one_shot_pending_ends_at_terminal() {
    let runtime = opened_runtime();
    let edits = DraftEdits::<_, ()>::default();
    runtime.hold_next_core();
    edits.begin(
        &runtime,
        Field::Name,
        "rename",
        Some("project".into()),
        rename_resolver("Renamed"),
    );
    edits.begin(
        &runtime,
        Field::Position,
        "move",
        Some("layout".into()),
        unchanged_resolver(),
    );
    assert!(edits.is_pending(&Field::Name));
    assert!(edits.is_pending(&Field::Position));

    runtime.release_core();
    assert!(!edits.is_pending(&Field::Name));
    assert!(!edits.is_pending(&Field::Position));
    assert_eq!(
        edits.discard_drafts(true),
        vec![
            PendingEditResult::Landed {
                key: Field::Name,
                revision: 1,
            },
            PendingEditResult::Landed {
                key: Field::Position,
                revision: 1,
            },
        ]
    );
}

#[test]
fn core_and_save_failures_keep_ticket_wording() {
    let runtime = opened_runtime();
    let edits = DraftEdits::<_, ()>::default();
    runtime.fail_next_core("engine died");
    edits.begin(
        &runtime,
        Field::Name,
        "rename",
        Some("layout".into()),
        rename_resolver("Core failure"),
    );
    assert_eq!(
        edits.discard_drafts(true),
        vec![PendingEditResult::Failed {
            key: Field::Name,
            message: "The layout change could not be applied: engine died".into(),
        }]
    );

    let save_runtime = opened_runtime();
    save_runtime.fail_next_save("quota exceeded");
    edits.begin(
        &save_runtime,
        Field::Name,
        "rename after recovery",
        Some("layout".into()),
        rename_resolver("Save failure"),
    );
    assert_eq!(
        edits.discard_drafts(true),
        vec![PendingEditResult::Failed {
            key: Field::Name,
            message: "The layout completed, but the accepted document did not save: quota exceeded. Retry after recovery.".into(),
        }]
    );
}

#[test]
fn unchanged_and_landed_edits_report_the_session_revision() {
    let runtime = opened_runtime();
    let edits = DraftEdits::<_, ()>::default();
    edits.begin(
        &runtime,
        Field::Name,
        "rename",
        Some("layout".into()),
        rename_resolver("Changed"),
    );
    edits.begin(
        &runtime,
        Field::Position,
        "confirm",
        Some("layout".into()),
        unchanged_resolver(),
    );
    assert_eq!(
        edits.discard_drafts(true),
        vec![
            PendingEditResult::Landed {
                key: Field::Name,
                revision: 1,
            },
            PendingEditResult::Landed {
                key: Field::Position,
                revision: 1,
            },
        ]
    );
}

#[test]
fn owner_departure_and_scope_change_retire_without_failure_messages() {
    let runtime = opened_runtime();
    let edits = DraftEdits::<_, ()>::default();
    runtime.hold_next_core();
    edits.begin(
        &runtime,
        Field::Name,
        "rename",
        Some("layout".into()),
        rename_resolver("Retired"),
    );
    assert_eq!(
        edits.discard_drafts(false),
        vec![PendingEditResult::Retired { key: Field::Name }]
    );
    runtime.release_core();

    runtime.hold_next_core();
    edits.begin(
        &runtime,
        Field::Name,
        "rename another scope",
        Some("layout".into()),
        rename_resolver("Retired by navigation"),
    );
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: document("second-scope", "Second scope"),
    });
    assert!(edits.is_pending(&Field::Name));
    runtime.release_core();
    assert_eq!(
        edits.discard_drafts(true),
        vec![PendingEditResult::Retired { key: Field::Name }]
    );

    runtime.submit(Event::Close {
        operation_id: runtime.operation(),
    });
    edits.begin(
        &runtime,
        Field::Position,
        "closed-session edit",
        Some("layout".into()),
        unchanged_resolver(),
    );
    assert_eq!(
        edits.discard_drafts(true),
        vec![PendingEditResult::Retired {
            key: Field::Position
        }]
    );
}

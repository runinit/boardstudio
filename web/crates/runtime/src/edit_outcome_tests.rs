use super::*;
use boardstudio_application::{EditResolver, Event, Landing, Resolution, SnapshotToken};
use boardstudio_model::model::{
    Board, EditOperation, EditPhase, Part, PartDefinition, PartKind, Pose2, ProjectDoc, Side, Vec2,
};
use std::rc::Rc;

fn fixture_document(id: &str, name: &str) -> ProjectDoc {
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
    document
}

fn runtime_with_project(id: &str, name: &str) -> Rc<crate::runtime::Runtime> {
    let runtime = crate::runtime::Runtime::new();
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: fixture_document(id, name),
    });
    runtime
}

fn runtime_with_two_boards() -> Rc<crate::runtime::Runtime> {
    let mut document = fixture_document("two-board-project", "Two board project");
    document.boards = ["board-1", "board-2"]
        .into_iter()
        .map(|id| Board {
            id: id.into(),
            name: id.into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        })
        .collect();
    let runtime = crate::runtime::Runtime::new();
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document,
    });
    runtime
}

fn rename_resolver(name: &'static str) -> EditResolver {
    EditResolver::new(
        "layout-inspector",
        move |accepted: &boardstudio_application::AcceptedSnapshot| {
            let mut document = (*accepted.document).clone();
            document.name = name.into();
            Resolution::submit(
                vec![document.id.clone()],
                EditOperation::ReplaceDocument {
                    document: Box::new(document),
                },
            )
        },
    )
}

#[test]
fn native_runtime_settles_a_ticket_through_the_real_session_and_core() {
    let runtime = runtime_with_project("ticket-test", "Ticket test");
    let ticket = runtime.resolve_edit(
        "native-owner",
        Some("field".into()),
        rename_resolver("Renamed"),
    );
    assert_eq!(ticket.settlement(true), Settlement::Landed { revision: 1 });
    assert_eq!(runtime.model().accepted.unwrap().document.name, "Renamed");
    assert_eq!(
        runtime.saved_document("ticket-test").unwrap().name,
        "Renamed"
    );
}

#[test]
fn held_core_and_save_keep_a_ticket_pending_until_released() {
    let runtime = runtime_with_project("ticket-test", "Ticket test");
    runtime.hold_next_core();
    let core_ticket = runtime.resolve_edit(
        "layout-inspector",
        Some("layout".into()),
        rename_resolver("Core held"),
    );
    assert!(runtime.core_entered());
    assert_eq!(core_ticket.settlement(true), Settlement::Pending);
    runtime.release_core();
    assert_eq!(
        core_ticket.settlement(true),
        Settlement::Landed { revision: 1 }
    );

    runtime.hold_next_save();
    let save_ticket = runtime.resolve_edit(
        "layout-inspector",
        Some("layout".into()),
        rename_resolver("Save held"),
    );
    assert!(runtime.save_entered());
    assert_eq!(save_ticket.settlement(true), Settlement::Pending);
    runtime.release_save();
    assert_eq!(
        save_ticket.settlement(true),
        Settlement::Landed { revision: 2 }
    );
}

#[test]
fn dropping_the_observer_during_save_does_not_cancel_publication() {
    let runtime = runtime_with_project("edit-test", "Before");
    runtime.hold_next_save();
    let outcome = runtime.resolve_edit("rename", None, rename_resolver("After"));
    assert!(runtime.save_entered());
    assert!(outcome.is_pending());
    assert_eq!(runtime.model().accepted.unwrap().document.name, "Before");

    drop(outcome);
    runtime.release_save();

    let accepted = runtime.model().accepted.unwrap();
    assert_eq!(accepted.document.name, "After");
    assert_eq!(accepted.document.revision, 1);
    assert_eq!(
        runtime.saved_document("edit-test").as_ref(),
        Some(accepted.document.as_ref()),
    );
}

#[test]
fn core_and_save_failures_settle_the_ticket_without_a_resolver() {
    let runtime = runtime_with_project("ticket-test", "Ticket test");
    runtime.fail_next_core("engine died");
    let core_failure = runtime.resolve_edit(
        "layout-inspector",
        Some("layout".into()),
        rename_resolver("Core failure"),
    );
    assert_eq!(
        core_failure.settlement(true),
        Settlement::Failed {
            message: "The layout change could not be applied: engine died".into()
        }
    );

    let runtime = runtime_with_project("ticket-save-test", "Ticket save test");
    runtime.fail_next_save("quota exceeded");
    let save_failure = runtime.resolve_edit(
        "layout-inspector",
        Some("layout".into()),
        rename_resolver("Save failure"),
    );
    assert_eq!(
        save_failure.settlement(true),
        Settlement::Failed {
            message: "The layout completed, but the accepted document did not save: quota exceeded. Retry after recovery.".into()
        }
    );
}

#[test]
fn unchanged_and_retired_resolutions_use_session_settlement() {
    let runtime = runtime_with_project("ticket-test", "Ticket test");
    let before = runtime.model().accepted.unwrap();
    let unchanged = runtime.resolve_edit(
        "layout-inspector",
        Some("layout".into()),
        EditResolver::new("layout-inspector", |_accepted| Resolution::Unchanged),
    );
    assert_eq!(
        unchanged.settlement(true),
        Settlement::Landed {
            revision: before.document.revision
        }
    );
    let retired = runtime.resolve_edit(
        "layout-inspector",
        Some("layout".into()),
        EditResolver::new("layout-inspector", |_accepted| {
            Resolution::Retire("the selected part was deleted".into())
        }),
    );
    assert_eq!(
        retired.settlement(true),
        Settlement::Failed {
            message: "The layout change was refused: the selected part was deleted".into()
        }
    );
}

#[test]
fn changing_the_session_retires_its_held_ticket() {
    let runtime = runtime_with_project("ticket-test", "Ticket test");
    runtime.hold_next_core();
    let ticket = runtime.resolve_edit(
        "layout-inspector",
        Some("layout".into()),
        rename_resolver("Old project edit"),
    );
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: fixture_document("ticket-test-2", "Second project"),
    });
    let queued = runtime.resolve_edit(
        "layout-inspector",
        Some("layout".into()),
        rename_resolver("Queued edit"),
    );
    runtime.release_core();
    assert_eq!(ticket.settlement(true), Settlement::Retired);
    assert_eq!(queued.settlement(true), Settlement::Retired);
}

#[test]
fn a_scope_departure_retires_every_ticket_clone_permanently() {
    let runtime = runtime_with_project("ticket-test", "Ticket test");
    runtime.hold_next_core();
    let ticket = runtime.resolve_edit(
        "layout-inspector",
        Some("layout".into()),
        rename_resolver("Old project edit"),
    );
    let clone = ticket.clone();
    assert_eq!(ticket.settlement(true), Settlement::Pending);

    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: fixture_document("ticket-test-2", "Second project"),
    });
    assert_eq!(ticket.settlement(true), Settlement::Pending);
    runtime.release_core();
    assert_eq!(ticket.settlement(true), Settlement::Retired);
    assert_eq!(clone.settlement(true), Settlement::Retired);

    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: fixture_document("ticket-test", "Ticket test again"),
    });
    assert_eq!(ticket.settlement(true), Settlement::Retired);
    assert_eq!(clone.settlement(true), Settlement::Retired);
}

#[test]
fn returning_to_the_captured_board_scope_does_not_revive_observation() {
    let runtime = runtime_with_two_boards();
    let captured_scope = runtime.scope().unwrap();
    runtime.hold_next_core();
    let ticket = runtime.resolve_edit(
        "layout-inspector",
        Some("layout".into()),
        rename_resolver("Edit while navigating"),
    );
    runtime.submit(Event::Navigate {
        operation_id: runtime.operation(),
        board_id: "board-2".into(),
        instance_id: None,
    });
    // The panel skips this render and first reads the result after returning.
    runtime.submit(Event::Navigate {
        operation_id: runtime.operation(),
        board_id: "board-1".into(),
        instance_id: None,
    });
    assert_eq!(runtime.scope(), Some(captured_scope));
    assert_eq!(ticket.settlement(true), Settlement::Retired);
    runtime.release_core();
}

#[test]
fn open_waits_for_a_held_save_then_retires_the_landed_ticket() {
    let runtime = runtime_with_project("ticket-test", "Ticket test");
    runtime.hold_next_save();
    let ticket = runtime.resolve_edit(
        "layout-inspector",
        Some("layout".into()),
        rename_resolver("Saved before open"),
    );
    assert_eq!(ticket.settlement(true), Settlement::Pending);
    assert!(runtime.save_entered());

    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: fixture_document("ticket-test-2", "Second project"),
    });
    assert_eq!(ticket.settlement(true), Settlement::Pending);
    runtime.release_save();
    assert_eq!(ticket.settlement(true), Settlement::Retired);
}

#[test]
fn close_drains_held_core_work_while_the_accepted_scope_remains_current() {
    let runtime = runtime_with_project("ticket-test", "Ticket test");
    let captured_scope = runtime.scope();
    runtime.hold_next_core();
    let ticket = runtime.resolve_edit(
        "layout-inspector",
        Some("layout".into()),
        rename_resolver("Complete before close"),
    );

    runtime.submit(Event::Close {
        operation_id: runtime.operation(),
    });
    assert_eq!(ticket.settlement(true), Settlement::Pending);
    assert_eq!(runtime.scope(), captured_scope);
    assert_eq!(
        runtime.model().lifecycle,
        boardstudio_application::Lifecycle::Closing
    );

    runtime.release_core();
    assert_eq!(
        runtime.model().lifecycle,
        boardstudio_application::Lifecycle::Closed
    );
    assert_eq!(runtime.scope(), captured_scope);
    assert_eq!(ticket.settlement(true), Settlement::Landed { revision: 1 });
}

#[test]
fn same_scope_revision_changes_and_owner_departure_have_separate_lifetimes() {
    let runtime = runtime_with_project("ticket-test", "Ticket test");
    let ticket = runtime.resolve_edit(
        "layout-inspector",
        Some("layout".into()),
        rename_resolver("First revision"),
    );
    assert_eq!(ticket.settlement(true), Settlement::Landed { revision: 1 });
    let newer = runtime.resolve_edit(
        "layout-inspector",
        Some("layout".into()),
        rename_resolver("A later revision"),
    );
    assert_eq!(newer.settlement(true), Settlement::Landed { revision: 2 });
    assert_eq!(ticket.settlement(true), Settlement::Landed { revision: 1 });
    assert_eq!(ticket.settlement(false), Settlement::Retired);
    assert_eq!(ticket.settlement(true), Settlement::Retired);
}

#[test]
fn every_outcome_maps_to_one_settlement() {
    let landing = Some(Landing {
        revision: 4,
        token: SnapshotToken(9),
    });
    let table: Vec<(TerminalOutcome, Option<Landing>, Settlement)> = vec![
        (TerminalOutcome::Completed, landing, Settlement::Landed { revision: 4 }),
        (TerminalOutcome::Rejected(DOCUMENT_SESSION_CHANGED.into()), None, Settlement::Retired),
        (TerminalOutcome::Rejected("stale".into()), None, Settlement::Failed { message: "The edit change was refused: stale".into() }),
        (TerminalOutcome::BlockedByRecovery("recovery".into()), None, Settlement::Failed { message: "The edit change was refused while saving is in recovery: recovery".into() }),
        (TerminalOutcome::PersistenceFailed("disk".into()), None, Settlement::Failed { message: "The edit completed, but the accepted document did not save: disk. Retry after recovery.".into() }),
        (TerminalOutcome::ExecutorFailed("engine".into()), None, Settlement::Failed { message: "The edit change could not be applied: engine".into() }),
        (TerminalOutcome::Superseded, None, Settlement::Retired),
        (TerminalOutcome::Cancelled, None, Settlement::Retired),
        (TerminalOutcome::Closed, None, Settlement::Retired),
        (TerminalOutcome::Completed, None, Settlement::Failed { message: "The edit change could not be confirmed where it landed. The accepted value was restored.".into() }),
    ];
    for (outcome, landing, expected) in table {
        assert_eq!(settlement_of(&outcome, landing, None), expected);
    }
}

#[path = "../crates/catalogue/src/matrix_setup_operation.rs"]
mod matrix_setup_operation;

use boardstudio_application::{
    Completion, Durability, Effect, Event, OperationId, SaveResult, SelectionMode, Session,
    TerminalOutcome,
};
use boardstudio_core::{CoreEngine, model::*};
use matrix_setup_operation::{MatrixSetupPreset, MatrixSetupRequest, prepare_matrix};

#[path = "../crates/catalogue/src/matrix_setup_operation/test_catalogue.rs"]
mod matrix_catalogue;
use matrix_catalogue::catalogue;

fn session_document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("matrix-session", "Matrix session");
    document.boards.push(Board {
        id: "board-main".into(),
        name: "Main".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document
}

fn persist_session_committed(session: &mut Session, effects: Vec<Effect>) -> Vec<Effect> {
    let (save_attempt_id, document) = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Persist {
                save_attempt_id,
                document,
                ..
            } => Some((*save_attempt_id, (**document).clone())),
            _ => None,
        })
        .expect("the accepted edit schedules persistence");
    assert_eq!(
        session.read_model().durability,
        Durability::Saving {
            revision: document.revision
        }
    );
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    })
}

fn open_session_saved(session: &mut Session, document: ProjectDoc, id: u64) {
    let effects = session.submit(Event::Open {
        operation_id: OperationId(id),
        document,
    });
    persist_session_committed(session, effects);
    let model = session.read_model();
    assert_eq!(model.lifecycle, boardstudio_application::Lifecycle::Ready);
    assert!(model.accepted.is_some());
    assert!(matches!(model.durability, Durability::Saved { .. }));
}

fn session_history_action(session: &mut Session, event: Event) {
    let effects = session.submit(event);
    persist_session_committed(session, effects);
}

#[test]
fn matrix_setup_uses_saved_session_history_selection_and_reopen() {
    let mut session = Session::new(boardstudio_core::CoreEngine::validate_open);
    open_session_saved(&mut session, session_document(), 1);

    let prepared = prepare_matrix(
        "matrix-new".into(),
        "board-main".into(),
        MatrixSetupRequest {
            rows: 2,
            columns: 3,
            preset: MatrixSetupPreset::MxSolder,
        },
        false,
        &catalogue(),
    )
    .expect("the selected MX solder recipe exists");
    let matrix_id = prepared.matrix.id.clone();
    let resolution = boardstudio_application::Resolution::submit(
        vec![matrix_id.clone()],
        EditOperation::SetMatrix {
            matrix: prepared.matrix,
            definitions: Some(prepared.definitions),
        },
    );
    let edit_effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(2),
        label: "fixed matrix setup test".into(),
        resolver: boardstudio_application::EditResolver::new(
            "fixed matrix setup test",
            move |_| resolution.clone(),
        ),
    });
    persist_session_committed(&mut session, edit_effects.clone());
    let completion_effects = edit_effects;
    assert!(completion_effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(2),
            outcome: TerminalOutcome::Completed,

            ..
        }
    )));

    let accepted = session
        .read_model()
        .accepted
        .as_ref()
        .expect("saved matrix");
    let matrix = accepted
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)
        .expect("Core accepted the requested matrix");
    let matrix_part_ids = matrix.part_ids.clone();
    assert_eq!((matrix.rows, matrix.columns), (2, 3));
    assert_eq!(matrix.board_id.as_deref(), Some("board-main"));
    assert_eq!(
        matrix_part_ids.len(),
        12,
        "six switches plus six diodes are live"
    );
    assert!(matrix_part_ids.iter().all(|id| {
        accepted.document.parts.iter().any(|part| part.id == *id)
            && accepted
                .document
                .boards
                .iter()
                .find(|board| board.id == "board-main")
                .is_some_and(|board| board.part_ids.contains(id))
    }));

    let selection = session.submit(Event::SelectParts {
        operation_id: OperationId(3),
        part_ids: matrix_part_ids.clone(),
        range_part_ids: vec![],
        mode: SelectionMode::Replace,
    });
    assert!(
        selection.iter().any(|effect| matches!(
            effect,
            Effect::Settled {
                operation_id: OperationId(3),
                outcome: TerminalOutcome::Completed,

                ..
            }
        )),
        "selection settles synchronously in Session"
    );
    assert_eq!(session.read_model().selected_part_ids, matrix_part_ids);

    session_history_action(
        &mut session,
        Event::Undo {
            operation_id: OperationId(4),
        },
    );
    let undone = session.read_model().accepted.as_ref().expect("saved undo");
    assert!(
        !undone
            .document
            .matrices
            .iter()
            .any(|matrix| matrix.id == matrix_id)
    );
    assert!(
        undone
            .document
            .parts
            .iter()
            .all(|part| !part.id.starts_with("matrix/matrix-new/"))
    );
    assert!(matches!(
        session.read_model().durability,
        Durability::Saved { revision: 2 }
    ));

    session_history_action(
        &mut session,
        Event::Redo {
            operation_id: OperationId(5),
        },
    );
    let redone = session.read_model().accepted.as_ref().expect("saved redo");
    assert!(
        redone
            .document
            .matrices
            .iter()
            .any(|matrix| matrix.id == matrix_id)
    );
    assert_eq!(redone.document.revision, 3);

    let durable_document = (*redone.document).clone();
    let mut reopened_session = Session::new(boardstudio_core::CoreEngine::validate_open);
    open_session_saved(&mut reopened_session, durable_document, 6);
    let reopened = reopened_session
        .read_model()
        .accepted
        .as_ref()
        .expect("reopened saved document");
    assert!(
        reopened
            .document
            .matrices
            .iter()
            .any(|matrix| matrix.id == matrix_id)
    );
    assert_eq!(reopened.document.revision, 3);
}

//! Host-compatible matrix-setup resolver for creating a prepared matrix.
//!
//! The production controller (wasm) delegates to this module so the same
//! resolver runs under `cargo test` without a browser.

use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution};
use boardstudio_model::model::{EditOperation, Matrix, PartDefinition};

pub(super) fn create_matrix_resolver(
    board_id: String,
    matrix: Matrix,
    definitions: Vec<PartDefinition>,
) -> EditResolver {
    EditResolver::new("layout-matrix-setup", move |accepted: &AcceptedSnapshot| {
        let document = &accepted.document;
        if !document.boards.iter().any(|board| board.id == board_id) {
            return Resolution::Retire("The board no longer exists.".into());
        }
        if document
            .matrices
            .iter()
            .any(|existing| existing.id == matrix.id)
        {
            return Resolution::Retire("The new matrix identity is already in use.".into());
        }
        if definitions.iter().any(|definition| {
            document
                .definitions
                .iter()
                .any(|existing| existing.id == definition.id)
        }) {
            return Resolution::Retire(
                "The prepared matrix definitions conflict with the accepted document.".into(),
            );
        }
        if document.hardware.as_ref().is_some_and(|hardware| {
            hardware.topology == boardstudio_model::model::HardwareTopology::Split
        }) {
            let ids = boardstudio_web_layout_core::mirrored_pair_geometry::MirroredPairIds {
                right_matrix_id: format!("{}-right", matrix.id),
                left_layout_id: format!("{}-left-layout", matrix.id),
                right_layout_id: format!("{}-right-layout", matrix.id),
            };
            if document
                .matrices
                .iter()
                .any(|item| item.id == ids.right_matrix_id)
                || document
                    .layouts
                    .iter()
                    .any(|item| item.id == ids.left_layout_id || item.id == ids.right_layout_id)
            {
                return Resolution::Retire(
                    "The mirrored layout identity is already in use.".into(),
                );
            }
            let pair =
                match boardstudio_web_layout_core::mirrored_pair_geometry::project_mirrored_pair(
                    matrix.clone(),
                    &ids,
                    &boardstudio_web_layout_core::mirrored_pair_geometry::MirroredPairGeometryInput {
                        left_name: "Left half".into(),
                        right_name: "Right half".into(),
                        gap_mm: 40.0,
                    },
                    matrix.origin,
                ) {
                    Ok(pair) => pair,
                    Err(error) => return Resolution::Retire(error),
                };
            return Resolution::submit(
                vec![
                    matrix.id.clone(),
                    ids.right_matrix_id,
                    ids.left_layout_id,
                    ids.right_layout_id,
                ],
                EditOperation::CreateMirroredPair {
                    left: pair.left,
                    right: pair.right,
                    matrix: Box::new(pair.matrix),
                    definitions: Some(definitions.clone()),
                },
            );
        }
        Resolution::submit(
            vec![matrix.id.clone()],
            EditOperation::SetMatrix {
                matrix: matrix.clone(),
                definitions: Some(definitions.clone()),
            },
        )
    })
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{
        Completion, Durability, Effect, Event, OperationId, SaveResult, SelectionMode, Session,
        TerminalOutcome,
    };
    use boardstudio_model::model::*;
    use boardstudio_web_catalogue::matrix_setup_operation::{
        MatrixSetupPreset, MatrixSetupRequest, prepare_matrix,
    };
    use std::sync::Arc;

    use super::super::test_fixtures::catalogue;

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

    fn persist_open_committed(session: &mut Session, effects: Vec<Effect>) -> Vec<Effect> {
        let save_attempt_id = effects
            .iter()
            .find_map(|effect| match effect {
                Effect::Persist {
                    save_attempt_id, ..
                } => Some(*save_attempt_id),
                _ => None,
            })
            .expect("open schedules persistence");
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
        // Opening commits through the Persist round-trip before the snapshot is accepted.
        persist_open_committed(session, effects);
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
        let mut session = Session::new();
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
        let mut reopened_session = Session::new();
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

    #[test]
    fn split_setup_creates_a_linked_layout_pair() {
        let mut document = ProjectDoc::empty("setup", "Setup");
        document.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Split,
            ..Default::default()
        });
        let matrix: Matrix = serde_json::from_value(serde_json::json!({
            "id": "main", "rows": 3, "columns": 5, "pitch": {"x": 19.05, "y": 19.05},
            "origin": {"x": 0.0, "y": 0.0}, "definitionId": "switch", "partIds": [], "boardId": "board"
        }))
        .unwrap();
        let snapshot = AcceptedSnapshot {
            token: boardstudio_application::SnapshotToken(1),
            session_epoch: boardstudio_application::SessionEpoch(1),
            document: Arc::new(document),
            scene: Arc::new(SceneDelta {
                module_scenes: vec![],
                revision: 0,
                transaction_id: "setup".into(),
                changed_ids: vec![],
                transforms: vec![],
                matrix_scenes: vec![],
                contours: vec![],
                board_contours: vec![],
                board_readiness: vec![],
                board_outline_scenes: vec![],
                finding_markers: vec![],
                findings: vec![],
                readiness: Readiness {
                    layout: false,
                    outline: false,
                    pcb: false,
                    case_ready: false,
                },
            }),
        };
        let Resolution::Submit(command) =
            create_matrix_resolver("board".into(), matrix, vec![]).resolve(&snapshot)
        else {
            panic!("setup must submit");
        };
        let EditOperation::CreateMirroredPair {
            left,
            right,
            matrix,
            ..
        } = command.operation
        else {
            panic!("split setup must create linked layouts, not a single matrix");
        };
        let link = right.mirror_link.as_ref().unwrap();
        assert_eq!(link.source_id, left.id);
        assert_ne!(left.matrix_ids, right.matrix_ids);
        assert!(matrix.origin.x < link.axis_x);
        let gap = 2.0 * (link.axis_x - matrix.origin.x) - (matrix.pitch.x - 1.0);
        assert!((gap - 40.0).abs() < 1e-9);
    }
}

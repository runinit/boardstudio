use boardstudio_application::{
    AcceptedSnapshot, Completion, EditResolver, Effect, Event, ExecutorEpoch, Landing, Lifecycle,
    OperationId, RequestId, Resolution, SaveAttemptId, SaveResult, Session, TerminalOutcome,
};
use boardstudio_core::mechanical::change::{
    MechanicalChange, MechanicalCriticalFitTextField, MechanicalEdit,
};
use boardstudio_core::{CoreEngine, model::*};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

fn fixture() -> ProjectDoc {
    let mut document = ProjectDoc::empty("project", "Project");
    document.definitions.push(PartDefinition {
        mechanical_profile: None,
        id: "switch".into(),
        name: "Switch".into(),
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
        id: "key".into(),
        definition_id: "switch".into(),
        reference: "SW1".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        anchor: None,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    });
    document
}

#[test]
fn resolution_constructor_keeps_session_owned_fields_out_of_the_resolver() {
    let operation = EditOperation::MoveParts { positions: vec![] };

    let Resolution::Submit(command) = Resolution::submit(vec!["key".into()], operation.clone())
    else {
        panic!("submit constructor must produce an edit intent");
    };

    assert_eq!(command.base_revision, 0);
    assert!(command.transaction_id.is_empty());
    assert!(matches!(command.phase, EditPhase::Commit));
    assert_eq!(command.target_ids, ["key"]);
    assert_eq!(command.operation, operation);

    let Resolution::Submit(grouped) = Resolution::submit_with_transaction_id(
        vec!["key".into()],
        "outline-transaction",
        EditOperation::MoveParts { positions: vec![] },
    ) else {
        panic!("grouped submit constructor must produce an edit intent");
    };
    assert_eq!(grouped.transaction_id, "outline-transaction");
}

fn matrix_range_fixture() -> (ProjectDoc, Vec<String>) {
    let mut document = fixture();
    let mut member_ids = Vec::new();
    let mut board_part_ids = Vec::new();
    for row in 0..3 {
        for column in 0..3 {
            if (row, column) == (1, 1) {
                continue;
            }
            let id = format!("matrix/main/r{row}c{column}");
            let mut part = document.parts[0].clone();
            part.id = id.clone();
            part.reference = format!("SW{}", member_ids.len() + 1);
            member_ids.push(id.clone());
            board_part_ids.push(id.clone());
            document.parts.push(part);
            if (row, column) == (0, 1) {
                let companion_id = format!("{id}/diode");
                let mut companion = document.parts.last().unwrap().clone();
                companion.id = companion_id.clone();
                companion.reference = "D1".into();
                member_ids.push(companion_id.clone());
                board_part_ids.push(companion_id);
                document.parts.push(companion);
            }
        }
    }
    document.boards.push(Board {
        id: "main".into(),
        name: "Main".into(),
        outline_ids: vec![],
        part_ids: board_part_ids,
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.matrices.push(Matrix {
        id: "main".into(),
        name: None,
        rows: 3,
        columns: 3,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2 { x: 0.0, y: 0.0 },
        definition_id: "switch".into(),
        part_ids: member_ids.clone(),
        board_id: Some("main".into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: vec![MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            row: 1,
            column: 1,
            enabled: false,
            definition_id: None,
            variant: None,
            offset: None,
            rotation: None,
            assemblies: vec![],
            assemblies_local: None,
        }],
    });
    (document, member_ids)
}

fn protected_fixture() -> ProjectDoc {
    let mut document = fixture();
    document.hardware = Some(HardwareConfiguration {
        boards: vec![
            ElectricalBoardConfiguration {
                board_id: "board-a".into(),
                controller_part_id: Some("mcu-left".into()),
                locks: BTreeMap::from([("row/0".into(), "P1".into())]),
                assignments: BTreeMap::from([
                    ("row/0".into(), "P1".into()),
                    ("column/0".into(), "P2".into()),
                ]),
                key_bindings: BTreeMap::from([("key".into(), "KC_A".into())]),
                protected_handoff: Some(ElectricalHandoffBaseline {
                    fingerprint: "handoff-1".into(),
                    revision: 0,
                    assignments: BTreeMap::from([
                        ("row/0".into(), "P1".into()),
                        ("column/0".into(), "P2".into()),
                    ]),
                }),
                ..Default::default()
            },
            ElectricalBoardConfiguration {
                board_id: "board-b".into(),
                locks: BTreeMap::from([("row/1".into(), "P9".into())]),
                assignments: BTreeMap::from([("row/1".into(), "P9".into())]),
                ..Default::default()
            },
        ],
        ..Default::default()
    });
    document
}

fn core_effect(effects: &[Effect]) -> (RequestId, ExecutorEpoch, CoreRequest) {
    effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Core {
                request_id,
                executor_epoch,
                request,
                ..
            } => Some((*request_id, *executor_epoch, (**request).clone())),
            _ => None,
        })
        .expect("one core request")
}

fn add_preview_electrical_notice(reply: &mut CoreReply, id: &str) {
    let CoreReply::Preview { scene, .. } = reply else {
        panic!("an edit preview must return a preview scene");
    };
    scene.findings.push(Finding {
        id: format!("preview:electrical:{id}"),
        severity: Severity::Info,
        scope: Scope::Pcb,
        message: "Preview removed a pin connection".into(),
        target_ids: vec!["key".into()],
    });
}

fn save_effect(effects: &[Effect]) -> (SaveAttemptId, ProjectDoc) {
    effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Persist {
                save_attempt_id,
                document,
                ..
            } => Some((*save_attempt_id, (**document).clone())),
            _ => None,
        })
        .expect("one persistence request")
}

fn settle_core_and_save(
    session: &mut Session,
    engine: &mut CoreEngine,
    effects: Vec<Effect>,
) -> (ProjectDoc, Vec<Effect>) {
    let (request_id, executor_epoch, request) = core_effect(&effects);
    let reply = engine.handle(request);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, saved) = save_effect(&effects);
    let settled = session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    (saved, settled)
}

#[test]
fn protected_handoff_review_uses_core_operation_after_normal_edits_preserve_it() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: protected_fixture(),
    });
    let (opened, _) = settle_core_and_save(&mut session, &mut engine, effects);
    assert_eq!(opened.revision, 0);

    // Ordinary document replacement is still an edit, so Core restores the protected handoff.
    let mut attempted_clear = opened.clone();
    attempted_clear.hardware.as_mut().unwrap().boards[0].protected_handoff = None;
    let effects = session.submit(resolved_edit(
        OperationId(2),
        EditCommand {
            base_revision: 0,
            transaction_id: "ordinary-clear-attempt".into(),
            phase: EditPhase::Commit,
            target_ids: vec!["board-a".into()],
            operation: EditOperation::ReplaceDocument {
                document: Box::new(attempted_clear),
            },
        },
    ));
    let (after_edit, _) = settle_core_and_save(&mut session, &mut engine, effects);
    assert_eq!(after_edit.revision, 1);
    assert_eq!(
        after_edit.hardware.as_ref().unwrap().boards[0]
            .protected_handoff
            .as_ref()
            .unwrap()
            .fingerprint,
        "handoff-1"
    );

    let effects = session.submit(Event::ReviewElectricalRemap {
        operation_id: OperationId(3),
        base_revision: 1,
        board_id: "board-a".into(),
        expected_fingerprint: "handoff-1".into(),
    });
    let (request_id, executor_epoch, request) = core_effect(&effects);
    assert!(matches!(request, CoreRequest::ReviewElectricalRemap { .. }));
    let reply = engine.handle(request);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, reviewed) = save_effect(&effects);
    let configuration = &reviewed.hardware.as_ref().unwrap().boards[0];
    assert_eq!(reviewed.revision, 2);
    assert!(configuration.protected_handoff.is_none());
    assert_eq!(configuration.locks["row/0"], "P1");
    assert_eq!(configuration.assignments["row/0"], "P1");
    assert_eq!(configuration.assignments["column/0"], "P2");
    assert_eq!(configuration.key_bindings["key"], "KC_A");
    assert_eq!(
        &reviewed.hardware.as_ref().unwrap().boards[1],
        &opened.hardware.as_ref().unwrap().boards[1]
    );
    let settled = session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    assert!(settled.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(3),
            outcome: TerminalOutcome::Completed,

            ..
        }
    )));
    assert_eq!(
        session
            .read_model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .as_ref(),
        &reviewed
    );
}

#[test]
fn protected_handoff_review_rejects_a_stale_fingerprint_without_saving() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let opened = session.submit(Event::Open {
        operation_id: OperationId(10),
        document: protected_fixture(),
    });
    settle_core_and_save(&mut session, &mut engine, opened);

    let stale_revision = session.submit(Event::ReviewElectricalRemap {
        operation_id: OperationId(11),
        base_revision: 1,
        board_id: "board-a".into(),
        expected_fingerprint: "handoff-1".into(),
    });
    assert!(
        !stale_revision
            .iter()
            .any(|effect| matches!(effect, Effect::Core { .. }))
    );
    assert!(stale_revision.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(11),
            outcome: TerminalOutcome::Rejected(_),

            ..
        }
    )));

    let effects = session.submit(Event::ReviewElectricalRemap {
        operation_id: OperationId(12),
        base_revision: 0,
        board_id: "board-a".into(),
        expected_fingerprint: "outdated-handoff".into(),
    });
    let (request_id, executor_epoch, request) = core_effect(&effects);
    let reply = engine.handle(request);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(reply),
    });
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::Persist { .. }))
    );
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(12),
            outcome: TerminalOutcome::Rejected(message),

        ..} if message.contains("handoff changed")
    )));
    let accepted = session.read_model().accepted.as_ref().unwrap();
    assert_eq!(accepted.document.revision, 0);
    assert_eq!(
        accepted.document.hardware.as_ref().unwrap().boards[0]
            .protected_handoff
            .as_ref()
            .unwrap()
            .fingerprint,
        "handoff-1"
    );
}

#[test]
fn retry_persists_retained_commit_without_replaying_engine_edit() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();

    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: fixture(),
    });
    let (open_request_id, open_epoch, open) = core_effect(&effects);
    let open_reply = engine.handle(open);
    let effects = session.complete(Completion::Core {
        request_id: open_request_id,
        executor_epoch: open_epoch,
        reply: Box::new(open_reply),
    });
    let (open_save_id, opened) = save_effect(&effects);
    assert_eq!(opened.revision, 0);
    let effects = session.complete(Completion::Persist {
        save_attempt_id: open_save_id,
        result: SaveResult::Committed,
    });
    assert_eq!(
        session
            .read_model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .revision,
        0
    );
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(1),
            outcome: TerminalOutcome::Completed,

            ..
        }
    )));

    let event = resolved_edit(
        OperationId(2),
        EditCommand {
            base_revision: 0,
            transaction_id: "move-1".into(),
            phase: EditPhase::Commit,
            target_ids: vec!["key".into()],
            operation: EditOperation::MoveParts {
                positions: vec![Position {
                    id: "key".into(),
                    at: Vec2 { x: 3.0, y: 0.0 },
                }],
            },
        },
    );
    let effects = session.submit(event);
    let (edit_request_id, edit_epoch, edit) = core_effect(&effects);
    let committed_reply = engine.handle(edit);
    let effects = session.complete(Completion::Core {
        request_id: edit_request_id,
        executor_epoch: edit_epoch,
        reply: Box::new(committed_reply),
    });
    let (failed_save_id, pending) = save_effect(&effects);
    assert_eq!(pending.revision, 1);
    assert_eq!(
        session
            .read_model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .revision,
        0
    );
    let effects = session.complete(Completion::Persist {
        save_attempt_id: failed_save_id,
        result: SaveResult::Aborted("quota".into()),
    });
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(2),
            outcome: TerminalOutcome::PersistenceFailed(_),

            ..
        }
    )));
    assert_eq!(
        session
            .read_model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .revision,
        0
    );

    let (outcome, landing) = settled_landing(&effects, OperationId(2));
    assert!(matches!(outcome, TerminalOutcome::PersistenceFailed(_)));
    assert!(landing.is_none(), "a failed save has no landing");

    let effects = session.submit(Event::RetrySave {
        operation_id: OperationId(3),
    });
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::Core { .. }))
    );
    let (retry_save_id, retried) = save_effect(&effects);
    assert_ne!(retry_save_id, failed_save_id);
    assert_eq!(retried, pending);
    let effects = session.complete(Completion::Persist {
        save_attempt_id: retry_save_id,
        result: SaveResult::Committed,
    });
    assert_eq!(
        session
            .read_model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .revision,
        1
    );
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(3),
            outcome: TerminalOutcome::Completed,

            ..
        }
    )));

    let accepted = session.read_model().accepted.clone().unwrap();
    let (retry_outcome, retry_landing) = settled_landing(&effects, OperationId(3));
    let (original_outcome, original_landing) = settled_landing(&effects, OperationId(2));
    assert_eq!(retry_outcome, TerminalOutcome::Completed);
    assert_eq!(original_outcome, TerminalOutcome::Completed);
    let retry_landing = retry_landing.expect("the retry reports where it landed");
    assert_eq!(retry_landing.revision, accepted.document.revision);
    assert_eq!(retry_landing.token, accepted.token);
    assert_eq!(
        original_landing,
        Some(retry_landing),
        "the original edit operation reports the same landing"
    );

    let snapshot_request = CoreRequest::Snapshot {
        id: "inspect".into(),
    };
    assert!(
        matches!(engine.handle(snapshot_request), CoreReply::Scene { scene, .. } if scene.revision == 1)
    );
}

fn resolved_edit(operation_id: OperationId, command: EditCommand) -> Event {
    let resolver_command = command;
    Event::ResolveEdit {
        operation_id,
        label: "test fixed command".into(),
        resolver: EditResolver::new("test fixed command", move |_| {
            Resolution::Submit(resolver_command.clone())
        }),
    }
}

fn move_edit(operation_id: u64, base_revision: u64, x: f64) -> Event {
    resolved_edit(
        OperationId(operation_id),
        EditCommand {
            base_revision,
            transaction_id: format!("move-{operation_id}"),
            phase: EditPhase::Commit,
            target_ids: vec!["key".into()],
            operation: EditOperation::MoveParts {
                positions: vec![Position {
                    id: "key".into(),
                    at: Vec2 { x, y: 0.0 },
                }],
            },
        },
    )
}

fn open_ready(session: &mut Session, engine: &mut CoreEngine) {
    let effects = session.submit(Event::Open {
        operation_id: OperationId(10),
        document: fixture(),
    });
    let (request_id, epoch, request) = core_effect(&effects);
    let reply = engine.handle(request);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch: epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
}

#[test]
fn owned_gesture_commit_finishes_before_project_open() {
    for preview_first in [true, false] {
        let mut session = Session::new();
        let mut engine = CoreEngine::new();
        open_ready(&mut session, &mut engine);
        session.submit(Event::GestureBegin {
            operation_id: OperationId(21),
            pointer_id: 7,
            target_ids: vec!["key".into()],
            transaction_id: "owned-before-open".into(),
            start: vec![Position {
                id: "key".into(),
                at: Vec2 { x: 0.0, y: 0.0 },
            }],
            pitch: Vec2 { x: 19.05, y: 19.05 },
            snap_fraction: 0.0,
            geometry_snap: false,
            gap: None,
            alt: false,
        });
        let preview = if preview_first {
            session.submit(Event::GestureSample {
                pointer_id: 7,
                positions: vec![Position {
                    id: "key".into(),
                    at: Vec2 { x: 4.0, y: 0.0 },
                }],
                alt: false,
            });
            let generation = session.read_model().gesture.as_ref().unwrap().generation;
            Some(session.submit(Event::GestureFrame { generation }))
        } else {
            None
        };
        let commit = session.submit(Event::GestureEnd {
            pointer_id: 7,
            final_positions: vec![Position {
                id: "key".into(),
                at: Vec2 { x: 4.0, y: 0.0 },
            }],
            alt: false,
        });
        let (request_id, executor_epoch, request) =
            core_effect(preview.as_ref().unwrap_or(&commit));
        let next = ProjectDoc::empty("next-project", "Next");
        let open = session.submit(Event::Open {
            operation_id: OperationId(22),
            document: next.clone(),
        });
        assert!(
            !open.iter().any(|effect| matches!(
                effect,
                Effect::Settled {
                    operation_id: OperationId(21),
                    outcome: TerminalOutcome::Cancelled,
                    ..
                }
            )),
            "project switching must not cancel an owned commit"
        );
        let mut effects = session.complete(Completion::Core {
            request_id,
            executor_epoch,
            reply: Box::new(engine.handle(request)),
        });
        if preview_first {
            let (request_id, executor_epoch, request) = core_effect(&effects);
            effects = session.complete(Completion::Core {
                request_id,
                executor_epoch,
                reply: Box::new(engine.handle(request)),
            });
        }
        let (save_attempt_id, saved) = save_effect(&effects);
        assert_eq!(saved.id, "project");
        assert_eq!(saved.parts[0].pose.at.x, 4.0);
        let effects = session.complete(Completion::Persist {
            save_attempt_id,
            result: SaveResult::Committed,
        });
        assert_eq!(
            settled_landing(&effects, OperationId(21)).0,
            TerminalOutcome::Completed
        );
        assert_eq!(
            session
                .read_model()
                .accepted
                .as_ref()
                .unwrap()
                .document
                .as_ref(),
            &saved
        );
        let (replacement, _) = settle_core_and_save(&mut session, &mut engine, effects);
        assert_eq!(replacement, next);
        assert_eq!(
            session
                .read_model()
                .accepted
                .as_ref()
                .unwrap()
                .document
                .as_ref(),
            &next
        );
    }
}

#[test]
fn invalid_open_preserves_the_current_gesture_and_accepted_session() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    session.submit(Event::GestureBegin {
        operation_id: OperationId(21),
        pointer_id: 7,
        target_ids: vec!["key".into()],
        transaction_id: "keep-draft".into(),
        start: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
        }],
        pitch: Vec2 { x: 19.05, y: 19.05 },
        snap_fraction: 0.0,
        geometry_snap: false,
        gap: None,
        alt: false,
    });
    let before = session.read_model().clone();
    let mut invalid = fixture();
    invalid.format_version = 6;
    let mut effects = session.submit(Event::Open {
        operation_id: OperationId(22),
        document: invalid,
    });
    if effects
        .iter()
        .any(|effect| matches!(effect, Effect::Core { .. }))
    {
        let (request_id, epoch, request) = core_effect(&effects);
        effects.extend(session.complete(Completion::Core {
            request_id,
            executor_epoch: epoch,
            reply: Box::new(engine.handle(request)),
        }));
    }
    assert!(effects.iter().any(|effect| matches!(effect,
        Effect::Settled { operation_id: OperationId(22), outcome: TerminalOutcome::Rejected(reason), .. }
            if reason.contains("No conversion"))));
    assert!(!effects.iter().any(|effect| matches!(
        effect,
        Effect::Persist { .. } | Effect::ReleasePointer { .. }
    )));
    assert_eq!(session.read_model().accepted, before.accepted);
    assert_eq!(session.read_model().gesture, before.gesture);
    assert_eq!(
        session.scope(),
        Some(boardstudio_application::Scope {
            session_epoch: before.accepted.unwrap().session_epoch,
            document_id: "project".into(),
            board_id: String::new(),
            instance_id: None
        })
    );
}

#[test]
fn group_position_edit_moves_selected_parts_together_and_undo_restores_both() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let mut document = fixture();
    let mut second = document.parts[0].clone();
    second.id = "key-2".into();
    second.reference = "SW2".into();
    second.pose.at.x = 10.0;
    document.parts.push(second);
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document,
    });
    settle_core_and_save(&mut session, &mut engine, effects);

    let effects = session.submit(resolved_edit(
        OperationId(2),
        EditCommand {
            base_revision: 0,
            transaction_id: "group-position-blur".into(),
            phase: EditPhase::Commit,
            target_ids: vec!["key".into(), "key-2".into()],
            operation: EditOperation::MoveParts {
                positions: vec![
                    Position {
                        id: "key".into(),
                        at: Vec2 { x: 3.0, y: 0.0 },
                    },
                    Position {
                        id: "key-2".into(),
                        at: Vec2 { x: 13.0, y: 0.0 },
                    },
                ],
            },
        },
    ));
    let (moved, _) = settle_core_and_save(&mut session, &mut engine, effects);
    assert_eq!(moved.parts[0].pose.at.x, 3.0);
    assert_eq!(moved.parts[1].pose.at.x, 13.0);

    let effects = session.submit(Event::Undo {
        operation_id: OperationId(3),
    });
    let (restored, _) = settle_core_and_save(&mut session, &mut engine, effects);
    assert_eq!(restored.parts[0].pose.at.x, 0.0);
    assert_eq!(restored.parts[1].pose.at.x, 10.0);
}

#[test]
fn preview_edit_displays_without_changing_the_accepted_document_or_history() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let accepted = session
        .read_model()
        .accepted
        .as_ref()
        .unwrap()
        .document
        .as_ref()
        .clone();

    let effects = session.submit(Event::PreviewEdit {
        operation_id: OperationId(11),
        transaction_id: "preview-position".into(),
        target_ids: vec!["key".into()],
        operation: EditOperation::MoveParts {
            positions: vec![Position {
                id: "key".into(),
                at: Vec2 { x: 8.0, y: 0.0 },
            }],
        },
    });
    let (request_id, executor_epoch, request) = core_effect(&effects);
    assert!(matches!(
        &request,
        CoreRequest::Edit { command, .. } if command.phase == EditPhase::Preview
    ));
    let mut reply = engine.handle(request);
    add_preview_electrical_notice(&mut reply, "first");
    session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(reply),
    });

    let after_preview = session.read_model().accepted.as_ref().unwrap();
    assert_eq!(after_preview.document.as_ref(), &accepted);
    assert_eq!(after_preview.document.revision, 0);
    let accepted_token = after_preview.token;
    let display_preview = session.read_model().display_preview.as_ref().unwrap();
    assert!(display_preview.findings.iter().any(|finding| {
        finding.id == "preview:electrical:first"
            && finding.severity == Severity::Info
            && finding.scope == Scope::Pcb
    }));

    let board_id = session.read_model().active_board_id.clone();
    let clear = |operation_id, transaction_id: &str| Event::ClearPreview {
        operation_id: OperationId(operation_id),
        token: accepted_token,
        revision: 0,
        board_id: board_id.clone(),
        transaction_id: transaction_id.into(),
    };

    let effects = session.submit(Event::PreviewEdit {
        operation_id: OperationId(13),
        transaction_id: "replacement-preview".into(),
        target_ids: vec!["key".into()],
        operation: EditOperation::MoveParts {
            positions: vec![Position {
                id: "key".into(),
                at: Vec2 { x: 12.0, y: 0.0 },
            }],
        },
    });
    let (request_id, executor_epoch, request) = core_effect(&effects);
    let mut reply = engine.handle(request);
    add_preview_electrical_notice(&mut reply, "replacement");
    session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(reply),
    });
    let replacement_preview = session.read_model().display_preview.as_ref().unwrap();
    assert!(
        replacement_preview
            .findings
            .iter()
            .any(|finding| finding.id == "preview:electrical:replacement")
    );
    assert!(
        !replacement_preview
            .findings
            .iter()
            .any(|finding| finding.id == "preview:electrical:first")
    );

    session.submit(clear(14, "preview-position"));
    assert!(
        session
            .read_model()
            .display_preview
            .as_ref()
            .unwrap()
            .findings
            .iter()
            .any(|finding| finding.id == "preview:electrical:replacement")
    );
    session.submit(clear(15, "replacement-preview"));
    assert!(session.read_model().display_preview.is_none());

    let accepted_after_clears = session.read_model().accepted.as_ref().unwrap();
    assert_eq!(accepted_after_clears.document.as_ref(), &accepted);
    assert_eq!(accepted_after_clears.document.revision, 0);
    assert_eq!(accepted_after_clears.token, accepted_token);

    let undo = engine.handle(CoreRequest::Undo {
        id: "preview-history-check".into(),
    });
    assert!(matches!(
        undo,
        CoreReply::Error { message, .. } if message == "History is empty"
    ));
}

#[test]
fn part_drag_uses_gap_geometry_snap_at_the_two_millimeter_tolerance() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let mut document = fixture();
    let mut moving = document.parts[0].clone();
    moving.id = "moving".into();
    moving.reference = "SW2".into();
    moving.pose.at = Vec2 { x: 19.6, y: 0.0 };
    document.parts.push(moving);
    let effects = session.submit(Event::Open {
        operation_id: OperationId(20),
        document,
    });
    let (request_id, epoch, request) = core_effect(&effects);
    let reply = engine.handle(request);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch: epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    session.submit(Event::GestureBegin {
        operation_id: OperationId(21),
        pointer_id: 7,
        target_ids: vec!["moving".into()],
        transaction_id: "drag-gap".into(),
        start: vec![Position {
            id: "moving".into(),
            at: Vec2 { x: 19.6, y: 0.0 },
        }],
        pitch: Vec2 { x: 19.0, y: 19.0 },
        snap_fraction: 0.0,
        geometry_snap: true,
        gap: Some(1.0),
        alt: false,
    });
    session.submit(Event::GestureSample {
        pointer_id: 7,
        positions: vec![Position {
            id: "moving".into(),
            at: Vec2 { x: 19.6, y: 0.0 },
        }],
        alt: false,
    });
    let generation = session.read_model().gesture.as_ref().unwrap().generation;
    let effects = session.submit(Event::GestureFrame { generation });
    assert!(
        matches!(core_effect(&effects).2, CoreRequest::Edit { command: EditCommand { operation: EditOperation::MoveParts { positions }, phase: EditPhase::Preview, .. }, .. } if positions[0].at.x == 19.0)
    );
}

#[test]
fn geometry_snap_ignores_parts_owned_by_another_board() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let mut document = fixture();
    let mut moving = document.parts[0].clone();
    moving.id = "moving".into();
    moving.reference = "SW2".into();
    moving.pose.at = Vec2 { x: 19.6, y: 0.0 };
    document.parts.push(moving);
    document.boards = vec![
        Board {
            id: "left".into(),
            name: "Left".into(),
            outline_ids: vec![],
            part_ids: vec!["moving".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        },
        Board {
            id: "right".into(),
            name: "Right".into(),
            outline_ids: vec![],
            part_ids: vec!["key".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        },
    ];
    let effects = session.submit(Event::Open {
        operation_id: OperationId(22),
        document,
    });
    let (request_id, epoch, request) = core_effect(&effects);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch: epoch,
        reply: Box::new(engine.handle(request)),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });

    session.submit(Event::Navigate {
        operation_id: OperationId(24),
        board_id: "left".into(),
        instance_id: None,
    });

    session.submit(Event::GestureBegin {
        operation_id: OperationId(23),
        pointer_id: 8,
        target_ids: vec!["moving".into()],
        transaction_id: "cross-board-snap".into(),
        start: vec![Position {
            id: "moving".into(),
            at: Vec2 { x: 19.6, y: 0.0 },
        }],
        pitch: Vec2 { x: 19.0, y: 19.0 },
        snap_fraction: 0.0,
        geometry_snap: true,
        gap: Some(1.0),
        alt: false,
    });
    session.submit(Event::GestureSample {
        pointer_id: 8,
        positions: vec![Position {
            id: "moving".into(),
            at: Vec2 { x: 19.4, y: 0.0 },
        }],
        alt: false,
    });
    let generation = session.read_model().gesture.as_ref().unwrap().generation;
    let effects = session.submit(Event::GestureFrame { generation });
    assert!(matches!(
        core_effect(&effects).2,
        CoreRequest::Edit {
            command: EditCommand {
                operation: EditOperation::MoveParts { positions },
                phase: EditPhase::Preview,
                ..
            },
            ..
        } if positions[0].at.x == 19.4
    ));
}

#[test]
fn drag_coalesces_previews_and_commits_the_pointerup_sample_once() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    session.submit(Event::GestureBegin {
        operation_id: OperationId(30),
        pointer_id: 77,
        target_ids: vec!["key".into()],
        transaction_id: "drag-once".into(),
        start: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
        }],
        pitch: Vec2 { x: 19.0, y: 19.0 },
        snap_fraction: 0.0,
        geometry_snap: false,
        gap: None,
        alt: true,
    });
    session.submit(Event::GestureSample {
        pointer_id: 77,
        positions: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 1.0, y: 0.0 },
        }],
        alt: true,
    });
    let generation = session.read_model().gesture.as_ref().unwrap().generation;
    let effects = session.submit(Event::GestureFrame { generation });
    let (preview_id, preview_epoch, preview_request) = core_effect(&effects);
    assert!(matches!(
        preview_request,
        CoreRequest::Edit {
            command: EditCommand {
                phase: EditPhase::Preview,
                ..
            },
            ..
        }
    ));

    session.submit(Event::GestureSample {
        pointer_id: 77,
        positions: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 3.0, y: 0.0 },
        }],
        alt: true,
    });
    session.submit(Event::GestureSample {
        pointer_id: 77,
        positions: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 5.0, y: 0.0 },
        }],
        alt: true,
    });
    session.submit(Event::GestureEnd {
        pointer_id: 77,
        final_positions: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 9.0, y: 0.0 },
        }],
        alt: true,
    });
    let preview_reply = engine.handle(preview_request);
    let effects = session.complete(Completion::Core {
        request_id: preview_id,
        executor_epoch: preview_epoch,
        reply: Box::new(preview_reply),
    });
    let (commit_id, commit_epoch, commit_request) = core_effect(&effects);
    assert!(
        matches!(&commit_request, CoreRequest::Edit { command: EditCommand { phase: EditPhase::Commit, operation: EditOperation::MoveParts { positions }, .. }, .. } if positions[0].at.x == 9.0)
    );
    let commit_reply = engine.handle(commit_request);
    let effects = session.complete(Completion::Core {
        request_id: commit_id,
        executor_epoch: commit_epoch,
        reply: Box::new(commit_reply),
    });
    let (save_attempt_id, saved) = save_effect(&effects);
    assert_eq!(saved.revision, 1);
    let effects = session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    assert_eq!(
        session
            .read_model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .parts[0]
            .pose
            .at
            .x,
        9.0
    );
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(30),
            outcome: TerminalOutcome::Completed,

            ..
        }
    )));
    assert!(
        matches!(engine.handle(CoreRequest::Undo { id: "undo-check".into() }), CoreReply::Scene { scene, document, .. } if scene.revision == 2 && document.parts[0].pose.at.x == 0.0)
    );
}

#[test]
fn uncertain_worker_outcome_is_never_replayed_and_requires_explicit_reopen() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let accepted = session
        .read_model()
        .accepted
        .as_ref()
        .unwrap()
        .document
        .as_ref()
        .clone();

    let effects = session.submit(move_edit(40, 0, 4.0));
    let (request_id, epoch, request) = core_effect(&effects);
    let _uncertain_reply = engine.handle(request);
    let effects = session.complete(Completion::CoreFailed {
        request_id,
        executor_epoch: epoch,
        reason: "worker terminated after mutation".into(),
    });
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(40),
            outcome: TerminalOutcome::ExecutorFailed(_),

            ..
        }
    )));
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::RestartCoreExecutor { .. }))
    );
    assert_eq!(
        session
            .read_model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .revision,
        0
    );
    let blocked = session.submit(move_edit(41, 0, 5.0));
    assert!(matches!(
        blocked.as_slice(),
        [Effect::Settled {
            outcome: TerminalOutcome::BlockedByRecovery(_),
            ..
        }]
    ));
    assert!(
        !blocked
            .iter()
            .any(|effect| matches!(effect, Effect::Core { .. }))
    );

    let effects = session.submit(Event::RecoverWithDocument {
        operation_id: OperationId(42),
        document: accepted,
    });
    let (open_id, new_epoch, open) = core_effect(&effects);
    assert!(matches!(open, CoreRequest::Open { .. }));
    let mut restarted = CoreEngine::new();
    let reply = restarted.handle(open);
    let effects = session.complete(Completion::Core {
        request_id: open_id,
        executor_epoch: new_epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, recovered) = save_effect(&effects);
    assert_eq!(recovered.revision, 0);
    let effects = session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    assert_eq!(
        session
            .read_model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .revision,
        0
    );
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(42),
            outcome: TerminalOutcome::Completed,

            ..
        }
    )));
}

#[test]
fn failed_replacement_retries_the_exact_candidate_without_core_or_early_publication() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let effects = session.submit(move_edit(70, 0, 4.0));
    settle_core_and_save(&mut session, &mut engine, effects);
    let before = session.read_model().accepted.clone().unwrap();
    let scope = session.scope();
    let candidate = ProjectDoc::empty("fresh-replacement", "Fresh");
    let effects = session.submit(Event::Open {
        operation_id: OperationId(71),
        document: candidate.clone(),
    });
    let (request_id, executor_epoch, request) = core_effect(&effects);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(engine.handle(request)),
    });
    let (save_attempt_id, pending) = save_effect(&effects);
    assert_eq!(pending, candidate);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Aborted("quota".into()),
    });
    assert_eq!(session.read_model().accepted.as_ref(), Some(&before));
    assert_eq!(session.scope(), scope);
    assert_eq!(session.read_model().lifecycle, Lifecycle::RecoveryRequired);
    let effects = session.submit(Event::RetrySave {
        operation_id: OperationId(72),
    });
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::Core { .. }))
    );
    let (retry_id, retried) = save_effect(&effects);
    assert_eq!(retried, candidate);
    session.complete(Completion::Persist {
        save_attempt_id: retry_id,
        result: SaveResult::Committed,
    });
    let accepted = session.read_model().accepted.clone().unwrap();
    assert_eq!(accepted.document.as_ref(), &candidate);
    assert_ne!(accepted.session_epoch, before.session_epoch);
    let effects = session.submit(Event::Undo {
        operation_id: OperationId(73),
    });
    let (request_id, executor_epoch, request) = core_effect(&effects);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(engine.handle(request)),
    });
    assert!(
        matches!(settled_landing(&effects, OperationId(73)).0, TerminalOutcome::Rejected(reason) if reason == "History is empty")
    );
    assert_eq!(session.read_model().accepted.as_ref(), Some(&accepted));
}

#[test]
fn invalid_recovery_keeps_the_exact_failed_result_for_storage_only_retry() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let effects = session.submit(move_edit(70, 0, 4.0));
    let (request_id, executor_epoch, request) = core_effect(&effects);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(engine.handle(request)),
    });
    let (save_attempt_id, pending) = save_effect(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Aborted("quota".into()),
    });
    let before = session.read_model().clone();
    let mut invalid = fixture();
    invalid.format_version = 6;
    let effects = session.submit(Event::RecoverWithDocument {
        operation_id: OperationId(71),
        document: invalid,
    });
    assert!(
        matches!(settled_landing(&effects, OperationId(71)).0, TerminalOutcome::Rejected(reason) if reason.contains("No conversion"))
    );
    assert_eq!(session.read_model(), &before);
    let effects = session.submit(Event::RetrySave {
        operation_id: OperationId(72),
    });
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::Core { .. }))
    );
    let (retry_id, retried) = save_effect(&effects);
    assert_eq!(retried, pending);
    session.complete(Completion::Persist {
        save_attempt_id: retry_id,
        result: SaveResult::Committed,
    });
    assert_eq!(
        session
            .read_model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .as_ref(),
        &pending
    );
}

#[test]
fn recovery_reopen_discards_failed_save_and_ignores_stale_executor_restart() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let accepted = session
        .read_model()
        .accepted
        .as_ref()
        .unwrap()
        .document
        .as_ref()
        .clone();
    let effects = session.submit(move_edit(70, 0, 4.0));
    let (request_id, executor_epoch, request) = core_effect(&effects);
    let reply = engine.handle(request);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Aborted("quota".into()),
    });

    let effects = session.submit(Event::RecoverWithDocument {
        operation_id: OperationId(71),
        document: accepted,
    });
    assert!(
        matches!(effects.first(), Some(Effect::Core { request, .. }) if matches!(**request, CoreRequest::Open { .. }))
    );
    let active_epoch = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Core { executor_epoch, .. } => Some(*executor_epoch),
            _ => None,
        })
        .unwrap();

    session.complete(Completion::ExecutorRestarted {
        executor_epoch: ExecutorEpoch(active_epoch.0 - 1),
    });
    session.submit(Event::Open {
        operation_id: OperationId(72),
        document: fixture(),
    });
    let open = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Core { request, .. } => Some((**request).clone()),
            _ => None,
        })
        .unwrap();
    let mut restarted = CoreEngine::new();
    let reply = restarted.handle(open);
    let effects = session.complete(Completion::Core {
        request_id: core_effect(&effects).0,
        executor_epoch: active_epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    let effects = session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    let next_epoch = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Core { executor_epoch, .. } => Some(*executor_epoch),
            _ => None,
        })
        .unwrap();
    assert_eq!(next_epoch, active_epoch);
}

// Characterization for the IDB-quota UI escape hatch: after a quota-aborted save
// latches RecoveryRequired, opening a fresh document discards the retained oversized
// candidate without recommitting it, and RetrySave stays an explicit-only action.
// The Runtime create-new/delete recovery flows rely on this Session contract.
#[test]
fn quota_escape_recover_discards_oversized_candidate_and_retry_stays_explicit() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let effects = session.submit(move_edit(70, 0, 4.0));
    let (request_id, executor_epoch, request) = core_effect(&effects);
    let reply = engine.handle(request);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, oversized) = save_effect(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Aborted("QuotaExceededError: origin storage is full".into()),
    });
    assert_eq!(session.read_model().lifecycle, Lifecycle::RecoveryRequired);

    let fresh = ProjectDoc::empty("quota-escape-fresh", "Untitled keyboard");
    let effects = session.submit(Event::RecoverWithDocument {
        operation_id: OperationId(71),
        document: fresh.clone(),
    });
    assert!(
        !effects.iter().any(
            |effect| matches!(effect, Effect::Persist { document, .. } if **document == oversized)
        ),
        "recovery must not recommit the quota-exceeding candidate"
    );
    let (request_id, executor_epoch, request) = core_effect(&effects);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(engine.handle(request)),
    });
    let (save_attempt_id, persisted) = save_effect(&effects);
    assert_eq!(
        persisted, fresh,
        "the only persistence after escape carries the fresh document"
    );
    let effects = session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    assert_eq!(
        session
            .read_model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .as_ref(),
        &fresh
    );
    assert!(
        matches!(
            settled_landing(&effects, OperationId(71)).0,
            TerminalOutcome::Completed
        ),
        "the escape hatch settles the fresh open as completed"
    );

    let effects = session.submit(Event::RetrySave {
        operation_id: OperationId(72),
    });
    assert!(
        matches!(settled_landing(&effects, OperationId(72)).0, TerminalOutcome::Rejected(reason) if reason == "there is no retained save to retry"),
        "after the escape hatch there is no retained save left to retry"
    );
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::Persist { .. })),
        "no automatic save retries the discarded document"
    );
}

#[test]
fn selection_navigation_and_camera_updates_are_session_only() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let mut document = fixture();
    let mut second = document.parts[0].clone();
    second.id = "key-2".into();
    second.reference = "SW2".into();
    second.pose.at = Vec2 { x: 19.0, y: 0.0 };
    document.parts.push(second);
    document.boards.push(Board {
        id: "main".into(),
        name: "Main".into(),
        outline_ids: vec![],
        part_ids: vec!["key".into(), "key-2".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let effects = session.submit(Event::Open {
        operation_id: OperationId(50),
        document,
    });
    let (request_id, epoch, request) = core_effect(&effects);
    let reply = engine.handle(request);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch: epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    let token = session.read_model().accepted.as_ref().unwrap().token;

    let effects = session.submit(Event::SelectParts {
        operation_id: OperationId(51),
        part_ids: vec!["key".into()],
        range_part_ids: vec![],
        mode: boardstudio_application::SelectionMode::Replace,
    });
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            outcome: TerminalOutcome::Completed,
            ..
        }
    )));
    assert!(
        settled_landing(&effects, OperationId(51)).1.is_none(),
        "selection changes no document"
    );
    session.submit(Event::SelectParts {
        operation_id: OperationId(60),
        part_ids: vec!["key-2".into()],
        range_part_ids: vec![],
        mode: boardstudio_application::SelectionMode::Add,
    });
    assert_eq!(session.read_model().selected_part_ids, vec!["key", "key-2"]);
    session.submit(Event::SelectParts {
        operation_id: OperationId(61),
        part_ids: vec!["key".into()],
        range_part_ids: vec![],
        mode: boardstudio_application::SelectionMode::Toggle,
    });
    assert_eq!(session.read_model().selected_part_ids, vec!["key-2"]);
    session.submit(Event::SelectParts {
        operation_id: OperationId(52),
        part_ids: vec!["key-2".into()],
        range_part_ids: vec!["key".into(), "key-2".into()],
        mode: boardstudio_application::SelectionMode::Range,
    });
    let effects = session.submit(Event::Navigate {
        operation_id: OperationId(53),
        board_id: "main".into(),
        instance_id: None,
    });
    assert!(
        settled_landing(&effects, OperationId(53)).1.is_none(),
        "navigation changes no document"
    );
    let effects = session.submit(Event::SetCamera {
        operation_id: OperationId(54),
        center: Vec2 { x: 10.0, y: 12.0 },
        zoom: 2.0,
    });
    assert!(
        settled_landing(&effects, OperationId(54)).1.is_none(),
        "camera changes no document"
    );
    let effects = session.submit(Event::CancelGeneration {
        operation_id: OperationId(55),
    });
    assert!(
        settled_landing(&effects, OperationId(55)).1.is_none(),
        "cancelling a job changes no document"
    );

    assert_eq!(session.read_model().selected_part_ids, vec!["key", "key-2"]);
    assert_eq!(session.read_model().active_board_id, "main");
    assert_eq!(
        session.read_model().camera,
        boardstudio_application::CameraState {
            center: Vec2 { x: 10.0, y: 12.0 },
            zoom: 2.0
        }
    );
    assert_eq!(session.read_model().accepted.as_ref().unwrap().token, token);
}

#[test]
fn preview_queued_behind_a_commit_is_discarded_after_its_revision_changes() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);

    let effects = session.submit(move_edit(20, 0, 3.0));
    let (commit_id, commit_epoch, commit_request) = core_effect(&effects);
    let commit_reply = engine.handle(commit_request);
    session.submit(Event::PreviewEdit {
        operation_id: OperationId(21),
        transaction_id: "stale-position-preview".into(),
        target_ids: vec!["key".into()],
        operation: EditOperation::MoveParts {
            positions: vec![Position {
                id: "key".into(),
                at: Vec2 { x: 8.0, y: 0.0 },
            }],
        },
    });

    let effects = session.complete(Completion::Core {
        request_id: commit_id,
        executor_epoch: commit_epoch,
        reply: Box::new(commit_reply),
    });
    let (save_attempt_id, committed) = save_effect(&effects);
    assert_eq!(committed.parts[0].pose.at.x, 3.0);

    let effects = session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::Core { .. }))
    );
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(21),
            outcome: TerminalOutcome::Superseded,
            ..
        }
    )));
    assert!(session.read_model().display_preview.is_none());
    assert_eq!(
        session
            .read_model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .parts[0]
            .pose
            .at
            .x,
        3.0
    );
}

#[test]
fn matrix_range_selects_rectangle_of_live_primary_members_and_keeps_anchor() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let (document, matrix_scope_ids) = matrix_range_fixture();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(501),
        document,
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);
    let accepted = session.read_model().accepted.as_ref().unwrap();
    let scope = boardstudio_application::Scope {
        session_epoch: accepted.session_epoch,
        document_id: accepted.document.id.clone(),
        board_id: "main".into(),
        instance_id: None,
    };

    let anchor = "matrix/main/r0c1".to_owned();
    let target = "matrix/main/r2c1".to_owned();
    session.submit(Event::SelectMatrixCell {
        operation_id: OperationId(502),
        scope: scope.clone(),
        matrix_id: "main".into(),
        target_part_id: anchor.clone(),
        // The projection is an extent whose first member differs from the clicked key.
        part_ids: matrix_scope_ids.clone(),
        mode: boardstudio_application::SelectionMode::Replace,
    });
    session.submit(Event::SelectMatrixCell {
        operation_id: OperationId(503),
        scope: scope.clone(),
        matrix_id: "main".into(),
        target_part_id: target.clone(),
        part_ids: vec![target.clone()],
        mode: boardstudio_application::SelectionMode::Range,
    });

    assert_eq!(
        session.read_model().selected_part_ids,
        vec![anchor.clone(), target]
    );
    assert_eq!(
        session.read_model().selection_anchor_id.as_deref(),
        Some(anchor.as_str())
    );
    assert_eq!(
        session.read_model().selection_mode,
        boardstudio_application::SelectionMode::Range
    );

    let repeated_target = "matrix/main/r2c0".to_owned();
    session.submit(Event::SelectMatrixCell {
        operation_id: OperationId(504),
        scope,
        matrix_id: "main".into(),
        target_part_id: repeated_target,
        part_ids: vec![],
        mode: boardstudio_application::SelectionMode::Range,
    });
    assert_eq!(
        session.read_model().selected_part_ids,
        vec![
            "matrix/main/r0c0",
            "matrix/main/r0c1",
            "matrix/main/r1c0",
            "matrix/main/r2c0",
            "matrix/main/r2c1"
        ]
    );
    assert_eq!(
        session.read_model().selection_anchor_id.as_deref(),
        Some(anchor.as_str())
    );
}

#[test]
fn cancelled_pointer_makes_late_preview_inert_and_releases_capture() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let effects = session.submit(Event::PreviewEdit {
        operation_id: OperationId(59),
        transaction_id: "notice-before-gesture".into(),
        target_ids: vec!["key".into()],
        operation: EditOperation::MoveParts {
            positions: vec![Position {
                id: "key".into(),
                at: Vec2 { x: 2.0, y: 0.0 },
            }],
        },
    });
    let (notice_id, notice_epoch, notice_request) = core_effect(&effects);
    let mut notice_reply = engine.handle(notice_request);
    add_preview_electrical_notice(&mut notice_reply, "gesture-current");
    session.complete(Completion::Core {
        request_id: notice_id,
        executor_epoch: notice_epoch,
        reply: Box::new(notice_reply),
    });
    assert!(
        session
            .read_model()
            .display_preview
            .as_ref()
            .unwrap()
            .findings
            .iter()
            .any(|finding| finding.id == "preview:electrical:gesture-current")
    );

    session.submit(Event::GestureBegin {
        operation_id: OperationId(60),
        pointer_id: 7,
        target_ids: vec!["key".into()],
        transaction_id: "cancel-me".into(),
        start: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
        }],
        pitch: Vec2 { x: 19.0, y: 19.0 },
        snap_fraction: 0.0,
        geometry_snap: false,
        gap: None,
        alt: true,
    });
    session.submit(Event::GestureSample {
        pointer_id: 7,
        positions: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 4.0, y: 0.0 },
        }],
        alt: true,
    });
    let generation = session.read_model().gesture.as_ref().unwrap().generation;
    let effects = session.submit(Event::GestureFrame { generation });
    let (preview_id, preview_epoch, preview_request) = core_effect(&effects);
    let effects = session.submit(Event::GestureCancel { pointer_id: 7 });
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::ReleasePointer { pointer_id: 7 }))
    );
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(60),
            outcome: TerminalOutcome::Cancelled,

            ..
        }
    )));
    assert!(session.read_model().display_preview.is_none());
    let accepted = session.read_model().accepted.as_ref().unwrap().clone();
    let mut stale_reply = engine.handle(preview_request);
    add_preview_electrical_notice(&mut stale_reply, "gesture-late");
    let duplicate_stale_reply = stale_reply.clone();

    let effects = session.submit(Event::PreviewEdit {
        operation_id: OperationId(61),
        transaction_id: "after-cancel-preview".into(),
        target_ids: vec!["key".into()],
        operation: EditOperation::MoveParts {
            positions: vec![Position {
                id: "key".into(),
                at: Vec2 { x: 9.0, y: 0.0 },
            }],
        },
    });
    assert!(
        !has_core_request(&effects),
        "the newer preview waits for the canceled request"
    );

    let effects = session.complete(Completion::Core {
        request_id: preview_id,
        executor_epoch: preview_epoch,
        reply: Box::new(stale_reply),
    });
    assert!(session.read_model().display_preview.is_none());
    let (newer_id, newer_epoch, newer_request) = core_effect(&effects);
    let mut newer_reply = engine.handle(newer_request);
    add_preview_electrical_notice(&mut newer_reply, "after-cancel-newer");
    session.complete(Completion::Core {
        request_id: newer_id,
        executor_epoch: newer_epoch,
        reply: Box::new(newer_reply),
    });

    let newer_preview = session.read_model().display_preview.as_ref().unwrap();
    assert_eq!(newer_preview.revision, accepted.document.revision);
    assert!(
        newer_preview
            .findings
            .iter()
            .any(|finding| finding.id == "preview:electrical:after-cancel-newer")
    );
    assert!(
        !newer_preview
            .findings
            .iter()
            .any(|finding| finding.id == "preview:electrical:gesture-late")
    );

    // Core is single-flight; replay after dispatch to exercise staleness against the newer display.
    session.complete(Completion::Core {
        request_id: preview_id,
        executor_epoch: preview_epoch,
        reply: Box::new(duplicate_stale_reply),
    });
    assert!(session.read_model().gesture.is_none());
    assert_eq!(session.read_model().accepted.as_ref(), Some(&accepted));
    let display_preview = session.read_model().display_preview.as_ref().unwrap();
    assert!(
        display_preview
            .findings
            .iter()
            .any(|finding| finding.id == "preview:electrical:after-cancel-newer")
    );
    assert!(
        !display_preview
            .findings
            .iter()
            .any(|finding| finding.id == "preview:electrical:gesture-late")
    );
}

#[test]
fn same_id_revision_reopen_invalidates_generation_and_export_tokens() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let scope = session.scope().unwrap();
    let generation_effects = session.submit(Event::StartGeneration {
        operation_id: OperationId(70),
        scope: scope.clone(),
    });
    let job_id = generation_effects
        .iter()
        .find_map(|effect| match effect {
            Effect::RunGeneration { job_id, .. } => Some(*job_id),
            _ => None,
        })
        .unwrap();
    let export_effects = session.submit(Event::StartExport {
        operation_id: OperationId(71),
        scope: scope.clone(),
    });
    let old_token = export_effects
        .iter()
        .find_map(|effect| match effect {
            Effect::RunExport { snapshot, .. } => Some(snapshot.token),
            _ => None,
        })
        .unwrap();

    assert!(session.export_is_current(OperationId(71), old_token, &scope));

    let durable = session
        .read_model()
        .accepted
        .as_ref()
        .unwrap()
        .document
        .as_ref()
        .clone();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(72),
        document: durable.clone(),
    });
    assert!(effects.iter().any(
        |effect| matches!(effect, Effect::CancelJob { job_id: cancelled } if *cancelled == job_id)
    ));
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::CancelExport {
            operation_id: OperationId(71)
        }
    )));
    assert!(!session.export_is_current(OperationId(71), old_token, &scope));
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(71),
            outcome: TerminalOutcome::Cancelled,

            ..
        }
    )));
    let (request_id, epoch, request) = core_effect(&effects);
    let reply = engine.handle(request);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch: epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    let current = session.read_model().accepted.as_ref().unwrap();
    assert_eq!(current.document.id, durable.id);
    assert_eq!(current.document.revision, durable.revision);
    assert_ne!(current.token, old_token);
    assert_ne!(current.session_epoch, scope.session_epoch);

    let stale = session.complete(Completion::GenerationFinished {
        job_id,
        scope: scope.clone(),
        exact: true,
    });
    assert!(stale.is_empty());
    let stale_export = session.complete(Completion::ExportFinished {
        operation_id: OperationId(71),
        token: old_token,
        scope,
        artifact_id: "wrong-scope.step".into(),
    });
    assert!(
        !stale_export
            .iter()
            .any(|effect| matches!(effect, Effect::DeliverExport { .. }))
    );
}

#[test]
fn instance_navigation_updates_scope_and_cancels_in_flight_case_work() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let mut document = fixture();
    document.boards.push(Board {
        id: "main".into(),
        name: "Main".into(),
        outline_ids: vec![],
        part_ids: vec!["key".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.hardware = Some(HardwareConfiguration {
        instances: vec!["left", "right"]
            .into_iter()
            .map(|half| PhysicalBoardInstance {
                id: format!("instance-{half}"),
                name: format!("{half} instance"),
                board_id: "main".into(),
                half: half.into(),
                role: "primary".into(),
                flipped: half == "right",
                controller_part_id: None,
                mechanical: None,
                construction_ownership:
                    boardstudio_core::model::MechanicalConstructionOwnership::Local,
                mechanical_identity_reservation: None,
            })
            .collect(),
        ..HardwareConfiguration::default()
    });

    let effects = session.submit(Event::Open {
        operation_id: OperationId(10),
        document,
    });
    let (request_id, epoch, request) = core_effect(&effects);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch: epoch,
        reply: Box::new(engine.handle(request)),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });

    let canonical_scope = session.scope().unwrap();
    assert_eq!(canonical_scope.board_id, "main");
    assert_eq!(canonical_scope.instance_id, None);
    let generation_effects = session.submit(Event::StartGeneration {
        operation_id: OperationId(11),
        scope: canonical_scope.clone(),
    });
    let job_id = generation_effects
        .iter()
        .find_map(|effect| match effect {
            Effect::RunGeneration { job_id, .. } => Some(*job_id),
            _ => None,
        })
        .expect("generation should start in canonical scope");
    let export_effects = session.submit(Event::StartExport {
        operation_id: OperationId(12),
        scope: canonical_scope.clone(),
    });
    assert!(export_effects.iter().any(|effect| matches!(
        effect,
        Effect::RunExport {
            operation_id: OperationId(12),
            ..
        }
    )));

    let effects = session.submit(Event::Navigate {
        operation_id: OperationId(13),
        board_id: "main".into(),
        instance_id: Some("instance-left".into()),
    });
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::CancelJob { job_id: cancelled } if *cancelled == job_id
    )));
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::CancelExport {
            operation_id: OperationId(12)
        }
    )));
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(13),
            outcome: TerminalOutcome::Completed,

            ..
        }
    )));
    let physical_scope = session.scope().unwrap();
    assert_eq!(physical_scope.board_id, "main");
    assert_eq!(physical_scope.instance_id.as_deref(), Some("instance-left"));
    assert_ne!(physical_scope, canonical_scope);

    let effects = session.submit(Event::Navigate {
        operation_id: OperationId(14),
        board_id: "main".into(),
        instance_id: None,
    });
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(14),
            outcome: TerminalOutcome::Completed,

            ..
        }
    )));
    assert_eq!(session.scope().unwrap(), canonical_scope);

    for (operation_id, board_id, instance_id) in [
        (15, "missing-board", None),
        (16, "main", Some("missing-instance")),
    ] {
        let effects = session.submit(Event::Navigate {
            operation_id: OperationId(operation_id),
            board_id: board_id.into(),
            instance_id: instance_id.map(str::to_owned),
        });
        assert!(effects.iter().any(|effect| matches!(
            effect,
            Effect::Settled {
                operation_id: id,
                outcome: TerminalOutcome::Rejected(_),

            ..} if *id == OperationId(operation_id)
        )));
        assert_eq!(session.scope().unwrap(), canonical_scope);
    }
}

#[test]
fn failed_export_is_removed_and_not_cancelled_again_after_reopen() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let scope = session.scope().unwrap();
    let effects = session.submit(Event::StartExport {
        operation_id: OperationId(83),
        scope: scope.clone(),
    });
    let token = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::RunExport { snapshot, .. } => Some(snapshot.token),
            _ => None,
        })
        .expect("export dispatch captures the accepted snapshot");

    session.complete(Completion::ExportFailed {
        operation_id: OperationId(83),
        reason: "archive writer failed".into(),
    });

    assert!(!session.export_is_current(OperationId(83), token, &scope));
    let durable = session
        .read_model()
        .accepted
        .as_ref()
        .unwrap()
        .document
        .as_ref()
        .clone();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(84),
        document: durable,
    });
    assert!(!effects.iter().any(|effect| matches!(
        effect,
        Effect::CancelExport {
            operation_id: OperationId(83)
        }
    )));
}

#[test]
fn generation_block_is_typed_and_stale_block_completion_is_ignored() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let scope = session.scope().unwrap();
    let effects = session.submit(Event::StartGeneration {
        operation_id: OperationId(80),
        scope: scope.clone(),
    });
    let job_id = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::RunGeneration { job_id, .. } => Some(*job_id),
            _ => None,
        })
        .unwrap();
    let mut stale_scope = scope.clone();
    stale_scope.board_id = "other-board".into();
    session.complete(Completion::GenerationBlocked {
        job_id,
        scope: stale_scope,
        reason: "stale blocked outcome".into(),
    });
    assert!(matches!(
        session.read_model().generation,
        boardstudio_application::GenerationStatus::Preparing { job_id: current } if current == job_id
    ));
    let effects = session.complete(Completion::GenerationBlocked {
        job_id,
        scope,
        reason: "case has blocking findings".into(),
    });
    assert!(matches!(
        session.read_model().generation,
        boardstudio_application::GenerationStatus::Blocked { job_id: current, .. } if current == job_id
    ));
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(80),
            outcome: TerminalOutcome::Rejected(reason),

        ..} if reason == "case has blocking findings"
    )));
}

fn rename_edit_command(base_revision: u64, name: &str) -> EditCommand {
    EditCommand {
        base_revision,
        transaction_id: format!("rename-{name}"),
        phase: EditPhase::Commit,
        target_ids: vec!["project".into()],
        operation: EditOperation::ReplaceDocument {
            document: Box::new(rename_document(base_revision, name)),
        },
    }
}

fn rename_document(base_revision: u64, name: &str) -> ProjectDoc {
    let mut document = fixture();
    document.revision = base_revision;
    document.name = name.into();
    document
}

fn settled_landing(
    effects: &[Effect],
    operation_id: OperationId,
) -> (TerminalOutcome, Option<Landing>) {
    effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Settled {
                operation_id: settled,
                outcome,
                landing,
            } if *settled == operation_id => Some((outcome.clone(), *landing)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("operation {operation_id:?} never settled"))
}

#[test]
fn document_completions_carry_the_landing_of_the_installed_snapshot() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: fixture(),
    });
    let (_, settled) = settle_core_and_save(&mut session, &mut engine, effects);
    let (outcome, landing) = settled_landing(&settled, OperationId(1));
    assert_eq!(outcome, TerminalOutcome::Completed);
    let accepted = session.read_model().accepted.clone().unwrap();
    let open_landing = landing.expect("the open reports where it landed");
    assert_eq!(open_landing.revision, accepted.document.revision);
    assert_eq!(open_landing.token, accepted.token);

    let effects = session.submit(resolved_edit(
        OperationId(2),
        rename_edit_command(0, "Renamed"),
    ));
    let (_, settled) = settle_core_and_save(&mut session, &mut engine, effects);
    let (outcome, landing) = settled_landing(&settled, OperationId(2));
    assert_eq!(outcome, TerminalOutcome::Completed);
    let accepted = session.read_model().accepted.clone().unwrap();
    let edit_landing = landing.expect("the edit commit reports where it landed");
    assert_eq!(edit_landing.revision, accepted.document.revision);
    assert_eq!(edit_landing.token, accepted.token);
    assert_ne!(
        edit_landing.token, open_landing.token,
        "each landing names its own snapshot"
    );

    let effects = session.submit(Event::Undo {
        operation_id: OperationId(3),
    });
    let (_, settled) = settle_core_and_save(&mut session, &mut engine, effects);
    let (outcome, landing) = settled_landing(&settled, OperationId(3));
    assert_eq!(outcome, TerminalOutcome::Completed);
    let accepted = session.read_model().accepted.clone().unwrap();
    let undo_landing = landing.expect("the undo reports where it landed");
    assert_eq!(
        accepted.document.name, "Project",
        "the undo restored the earlier name"
    );
    assert_eq!(undo_landing.revision, accepted.document.revision);
    assert_eq!(undo_landing.token, accepted.token);

    let effects = session.submit(Event::Redo {
        operation_id: OperationId(4),
    });
    let (_, settled) = settle_core_and_save(&mut session, &mut engine, effects);
    let (outcome, landing) = settled_landing(&settled, OperationId(4));
    assert_eq!(outcome, TerminalOutcome::Completed);
    let accepted = session.read_model().accepted.clone().unwrap();
    let redo_landing = landing.expect("the redo reports where it landed");
    assert_eq!(
        accepted.document.name, "Renamed",
        "the redo restored the edit"
    );
    assert_eq!(redo_landing.revision, accepted.document.revision);
    assert_eq!(redo_landing.token, accepted.token);
    assert_ne!(
        redo_landing.token, undo_landing.token,
        "each landing names its own snapshot"
    );
}

#[test]
fn rejections_and_closing_carry_no_landing() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: fixture(),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);

    let effects = session.submit(Event::RetrySave {
        operation_id: OperationId(2),
    });
    let (outcome, landing) = settled_landing(&effects, OperationId(2));
    assert!(matches!(outcome, TerminalOutcome::Rejected(_)));
    assert!(landing.is_none(), "a rejected operation has no landing");

    let effects = session.submit(Event::Close {
        operation_id: OperationId(3),
    });
    let (outcome, landing) = settled_landing(&effects, OperationId(3));
    assert_eq!(outcome, TerminalOutcome::Completed);
    assert!(landing.is_none(), "closing is not a document landing");
}

fn positioned_fixture(x: f64, y: f64) -> ProjectDoc {
    let mut document = fixture();
    document.parts[0].pose.at = Vec2 { x, y };
    document
}

fn position_resolver(
    label: &str,
    axis: char,
    value: f64,
    seen: Rc<RefCell<Vec<(f64, f64)>>>,
) -> EditResolver {
    EditResolver::new(label, move |accepted: &AcceptedSnapshot| {
        let document = &accepted.document;
        let Some(part) = document.parts.iter().find(|part| part.id == "key") else {
            return Resolution::Retire("the selected part was deleted".into());
        };
        seen.borrow_mut().push((part.pose.at.x, part.pose.at.y));
        let mut positions: Vec<Position> = document
            .parts
            .iter()
            .map(|part| Position {
                id: part.id.clone(),
                at: part.pose.at,
            })
            .collect();
        if let Some(position) = positions.iter_mut().find(|position| position.id == "key") {
            if axis == 'x' {
                position.at.x = value;
            } else {
                position.at.y = value;
            }
        }
        Resolution::Submit(EditCommand {
            base_revision: accepted.document.revision,
            transaction_id: String::new(),
            phase: EditPhase::Commit,
            target_ids: vec!["key".into()],
            operation: EditOperation::MoveParts { positions },
        })
    })
}

fn resolved_position(accepted: &AcceptedSnapshot) -> &Vec2 {
    &accepted
        .document
        .parts
        .iter()
        .find(|part| part.id == "key")
        .unwrap()
        .pose
        .at
}

fn has_core_request(effects: &[Effect]) -> bool {
    effects
        .iter()
        .any(|effect| matches!(effect, Effect::Core { .. }))
}

#[test]
fn queued_position_edits_resolve_against_the_accepted_document() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: positioned_fixture(66.675, -47.625),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);

    let seen = Rc::new(RefCell::new(Vec::new()));
    let effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(2),
        label: "inspector-x".into(),
        resolver: position_resolver("inspector-x", 'x', 60.0, seen.clone()),
    });
    assert!(
        has_core_request(&effects),
        "an intent at the head of an idle queue resolves immediately"
    );
    let (x_request_id, x_epoch, x_request) = core_effect(&effects);
    assert!(
        matches!(&x_request, CoreRequest::Edit { command, .. } if command.transaction_id == "m1-inspector-x-2"),
        "the transaction id is derived from the label and operation id"
    );

    let effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(3),
        label: "inspector-y".into(),
        resolver: position_resolver("inspector-y", 'y', -40.0, seen.clone()),
    });
    assert!(
        !has_core_request(&effects),
        "the queued edit waits for the running one"
    );

    let reply = engine.handle(x_request);
    let effects = session.complete(Completion::Core {
        request_id: x_request_id,
        executor_epoch: x_epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    let effects = session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    let (y_request_id, y_epoch, y_request) = core_effect(&effects);
    assert!(
        matches!(&y_request, CoreRequest::Edit { command: EditCommand { operation: EditOperation::MoveParts { positions }, .. }, .. } if positions[0].at == Vec2 { x: 60.0, y: -40.0 }),
        "the y edit is built from the document the x edit produced"
    );
    let reply = engine.handle(y_request);
    let effects = session.complete(Completion::Core {
        request_id: y_request_id,
        executor_epoch: y_epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });

    let accepted = session.read_model().accepted.clone().unwrap();
    assert_eq!(
        resolved_position(&accepted),
        &Vec2 { x: 60.0, y: -40.0 },
        "rapid x then y entry keeps both coordinates"
    );
    assert_eq!(
        seen.borrow().as_slice(),
        &[(66.675, -47.625), (60.0, -47.625)],
        "each resolver observes the commit accepted before it"
    );

    let effects = session.submit(Event::Undo {
        operation_id: OperationId(4),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);
    let accepted = session.read_model().accepted.clone().unwrap();
    assert_eq!(
        resolved_position(&accepted),
        &Vec2 {
            x: 60.0,
            y: -47.625
        }
    );

    let effects = session.submit(Event::Undo {
        operation_id: OperationId(5),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);
    let accepted = session.read_model().accepted.clone().unwrap();
    assert_eq!(
        resolved_position(&accepted),
        &Vec2 {
            x: 66.675,
            y: -47.625
        },
        "the second undo restores the original position"
    );
}

#[test]
fn a_vanished_target_retires_with_the_resolvers_reason() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: fixture(),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);

    let effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(2),
        label: "inspector".into(),
        resolver: EditResolver::new("inspector", |_accepted| {
            Resolution::Retire("the selected part was deleted".into())
        }),
    });
    let (outcome, landing) = settled_landing(&effects, OperationId(2));
    assert_eq!(
        outcome,
        TerminalOutcome::Rejected("the selected part was deleted".into())
    );
    assert!(landing.is_none(), "a retired edit never landed");
    assert!(
        !has_core_request(&effects),
        "a retired edit never reaches Core"
    );
}

#[test]
fn an_unchanged_resolution_lands_at_the_current_snapshot_and_keeps_draining() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: fixture(),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);

    let effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(2),
        label: "inspector".into(),
        resolver: EditResolver::new("inspector", |_accepted| Resolution::Unchanged),
    });
    let (outcome, landing) = settled_landing(&effects, OperationId(2));
    assert_eq!(outcome, TerminalOutcome::Completed);
    let accepted = session.read_model().accepted.clone().unwrap();
    assert_eq!(
        landing,
        Some(Landing {
            revision: accepted.document.revision,
            token: accepted.token,
        }),
        "an unchanged edit lands at the current accepted snapshot"
    );
    assert!(
        !has_core_request(&effects),
        "an unchanged edit never reaches Core"
    );

    let effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(3),
        label: "inspector".into(),
        resolver: EditResolver::new("inspector", |accepted: &AcceptedSnapshot| {
            let mut document = (*accepted.document).clone();
            document.name = "Renamed".into();
            Resolution::Submit(EditCommand {
                base_revision: accepted.document.revision,
                transaction_id: "supplied-transaction".into(),
                phase: EditPhase::Commit,
                target_ids: vec![document.id.clone()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(document),
                },
            })
        }),
    });
    assert!(
        has_core_request(&effects),
        "the queue keeps draining after an unchanged resolution"
    );
    let (_request_id, _epoch, request) = core_effect(&effects);
    assert!(
        matches!(&request, CoreRequest::Edit { command, .. } if command.transaction_id == "supplied-transaction"),
        "a supplied transaction id is kept"
    );
    let (_, settled) = settle_core_and_save(&mut session, &mut engine, effects);
    let (outcome, _) = settled_landing(&settled, OperationId(3));
    assert_eq!(outcome, TerminalOutcome::Completed);
    assert_eq!(
        session.read_model().accepted.clone().unwrap().document.name,
        "Renamed"
    );
}

#[test]
fn a_resolved_command_core_rejects_settles_rejected_with_cores_message() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: fixture(),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);

    let effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(2),
        label: "inspector".into(),
        resolver: EditResolver::new("inspector", |accepted: &AcceptedSnapshot| {
            Resolution::Submit(EditCommand {
                base_revision: accepted.document.revision,
                transaction_id: String::new(),
                phase: EditPhase::Commit,
                target_ids: vec!["gone".into()],
                operation: EditOperation::MoveParts {
                    positions: vec![Position {
                        id: "gone".into(),
                        at: Vec2 { x: 1.0, y: 1.0 },
                    }],
                },
            })
        }),
    });
    let (request_id, epoch, request) = core_effect(&effects);
    let reply = engine.handle(request);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch: epoch,
        reply: Box::new(reply),
    });
    let (outcome, landing) = settled_landing(&effects, OperationId(2));
    assert!(matches!(outcome, TerminalOutcome::Rejected(_)));
    assert!(landing.is_none(), "a rejected resolution never landed");
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::Persist { .. })),
        "a rejected resolution never saves"
    );
}

#[test]
fn recovery_and_closing_refuse_resolvers_without_calling_them() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: fixture(),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);

    let called = Rc::new(Cell::new(false));
    let resolver = {
        let called = called.clone();
        EditResolver::new("inspector", move |_accepted| {
            called.set(true);
            Resolution::Unchanged
        })
    };

    let effects = session.submit(resolved_edit(
        OperationId(2),
        rename_edit_command(0, "Renamed"),
    ));
    let (request_id, epoch, request) = core_effect(&effects);
    let reply = engine.handle(request);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch: epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    let _effects = session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Aborted("quota".into()),
    });
    assert!(matches!(
        session.read_model().lifecycle,
        Lifecycle::RecoveryRequired
    ));

    let effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(3),
        label: "inspector".into(),
        resolver,
    });
    let (outcome, landing) = settled_landing(&effects, OperationId(3));
    assert!(matches!(outcome, TerminalOutcome::BlockedByRecovery(_)));
    assert!(landing.is_none());
    assert!(!called.get(), "recovery refuses the resolver");

    let effects = session.submit(Event::RetrySave {
        operation_id: OperationId(4),
    });
    let (retry_save_id, _) = save_effect(&effects);
    session.submit(Event::Close {
        operation_id: OperationId(5),
    });
    assert!(matches!(session.read_model().lifecycle, Lifecycle::Closing));

    let effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(6),
        label: "inspector".into(),
        resolver: {
            let called = called.clone();
            EditResolver::new("inspector", move |_accepted| {
                called.set(true);
                Resolution::Unchanged
            })
        },
    });
    let (outcome, landing) = settled_landing(&effects, OperationId(6));
    assert!(matches!(
        &outcome,
        TerminalOutcome::BlockedByRecovery(reason) if reason == "session is closing"
    ));
    assert!(landing.is_none());
    assert!(!called.get(), "closing refuses the resolver");

    let effects = session.complete(Completion::Persist {
        save_attempt_id: retry_save_id,
        result: SaveResult::Committed,
    });
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(5),
            outcome: TerminalOutcome::Completed,
            ..
        }
    )));
}

#[test]
fn a_reopen_rejects_queued_resolvers_without_calling_them() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: fixture(),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);

    let mut second = fixture();
    second.name = "Second".into();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(2),
        document: second,
    });
    let (open_request_id, open_epoch, open_request) = core_effect(&effects);

    let called = Rc::new(Cell::new(false));
    let effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(3),
        label: "inspector".into(),
        resolver: {
            let called = called.clone();
            EditResolver::new("inspector", move |_accepted| {
                called.set(true);
                Resolution::Unchanged
            })
        },
    });
    assert!(
        !has_core_request(&effects),
        "the intent queues behind the running open"
    );

    let reply = engine.handle(open_request);
    let effects = session.complete(Completion::Core {
        request_id: open_request_id,
        executor_epoch: open_epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    let effects = session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    let (outcome, landing) = settled_landing(&effects, OperationId(3));
    assert!(matches!(
        &outcome,
        TerminalOutcome::Rejected(reason) if reason == "document session changed before command began"
    ));
    assert!(landing.is_none());
    assert!(
        !called.get(),
        "an epoch change drops the intent before its resolver runs"
    );
}

#[test]
fn a_gesture_commit_ahead_is_not_disturbed_and_the_intent_behind_resolves_on_its_result() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: fixture(),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);

    session.submit(Event::GestureBegin {
        operation_id: OperationId(2),
        pointer_id: 77,
        target_ids: vec!["key".into()],
        transaction_id: "drag-once".into(),
        start: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
        }],
        pitch: Vec2 { x: 19.0, y: 19.0 },
        snap_fraction: 0.0,
        geometry_snap: false,
        gap: None,
        alt: true,
    });
    session.submit(Event::GestureSample {
        pointer_id: 77,
        positions: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 9.0, y: 0.0 },
        }],
        alt: true,
    });
    let effects = session.submit(Event::GestureEnd {
        pointer_id: 77,
        final_positions: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 9.0, y: 0.0 },
        }],
        alt: true,
    });
    let (commit_id, commit_epoch, commit_request) = core_effect(&effects);
    assert!(
        matches!(
            &commit_request,
            CoreRequest::Edit { command: EditCommand { phase: EditPhase::Commit, operation: EditOperation::MoveParts { positions }, .. }, .. }
                if positions[0].at == Vec2 { x: 9.0, y: 0.0 }
        ),
        "the gesture commit runs first, untouched"
    );

    let seen = Rc::new(RefCell::new(Vec::new()));
    let effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(3),
        label: "inspector-y".into(),
        resolver: position_resolver("inspector-y", 'y', 5.0, seen),
    });
    assert!(
        !has_core_request(&effects),
        "the resolved edit queues behind the running gesture commit"
    );

    let reply = engine.handle(commit_request);
    let effects = session.complete(Completion::Core {
        request_id: commit_id,
        executor_epoch: commit_epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    let effects = session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    let (edit_id, edit_epoch, edit_request) = core_effect(&effects);
    assert!(
        matches!(
            &edit_request,
            CoreRequest::Edit { command: EditCommand { operation: EditOperation::MoveParts { positions }, .. }, .. }
                if positions[0].at == Vec2 { x: 9.0, y: 5.0 }
        ),
        "the intent behind the gesture resolves against the gesture's result"
    );

    let reply = engine.handle(edit_request);
    let effects = session.complete(Completion::Core {
        request_id: edit_id,
        executor_epoch: edit_epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    let accepted = session.read_model().accepted.clone().unwrap();
    assert_eq!(resolved_position(&accepted), &Vec2 { x: 9.0, y: 5.0 });
}

fn mechanical_fixture(fit_ids: &[&str]) -> ProjectDoc {
    let mut document = fixture();
    document.boards.push(Board {
        id: "main".into(),
        name: "Main".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let mut configuration: MechanicalConfiguration = serde_json::from_value(serde_json::json!({
        "boardId":"main","method":"printed","mount":"rigid",
        "plateThickness":1.5,"plateFoamThickness":0.5,"pcbThickness":1.6,
        "bottomFoamThickness":0.4,"batteryHeight":4.0,"bottomThickness":3.0,
        "plateToPcb":3.5,"wallThickness":2.0,"clearance":0.3,
        "openings":[],"mounts":[],"closureMounts":[],"profiles":[]
    }))
    .unwrap();
    configuration.critical_fits = Some(
        fit_ids
            .iter()
            .map(|id| MechanicalCriticalFit {
                id: (*id).into(),
                part_id: "case-closure/part".into(),
                label: "Critical dimension".into(),
                from: Vec2 { x: 0.0, y: 0.0 },
                to: Vec2 { x: 10.0, y: 0.0 },
                tolerance: "±0.2 mm".into(),
            })
            .collect(),
    );
    document.mechanical = Some(configuration);
    document
}

fn mechanical_template() -> PartDefinition {
    serde_json::from_value(serde_json::json!({
        "id":"generator:ceoloide/mounting_hole_npth", "name":"mounting hole npth",
        "kind":"custom", "pads":[{"id":"pad-0", "number": "", "at":{"x":0,"y":0},
            "size":{"x":2.2,"y":2.2}, "shape":"circle", "drill":2.2,"plated":false,"rotation":0}],
        "courtyard":[{"x":-1.1,"y":-1.1},{"x":1.1,"y":-1.1},{"x":1.1,"y":1.1},{"x":-1.1,"y":1.1}],
        "envelopeSource":{"courtyard":"generated"},
        "envelopeNotice":"No closed courtyard is available; the outline uses physical graphics and pad extents.",
        "generator":{"source":"ceoloide/mounting_hole_npth", "version":"bundled-1", "parameters":{}}
    }))
    .unwrap()
}

fn mechanical_command(transaction: &str, change: MechanicalChange) -> EditCommand {
    EditCommand {
        base_revision: 0,
        transaction_id: transaction.into(),
        phase: EditPhase::Commit,
        target_ids: vec!["main".into()],
        operation: EditOperation::ApplyMechanicalEdit {
            edit: Box::new(MechanicalEdit {
                target: boardstudio_core::mechanical::MechanicalTarget::Board {
                    board_id: "main".into(),
                },
                change,
                mounting_hole: Some(Box::new(mechanical_template())),
            }),
        },
    }
}

fn add_fit_command(transaction: &str) -> EditCommand {
    mechanical_command(
        transaction,
        MechanicalChange::AddCriticalFit {
            part_id: "case-closure/part".into(),
        },
    )
}

fn remove_fit_command(transaction: &str, fit_id: &str) -> EditCommand {
    mechanical_command(
        transaction,
        MechanicalChange::RemoveCriticalFit {
            fit_id: fit_id.into(),
        },
    )
}

/// A setter authored against `fit_id`. It retires when that exact identity is gone
/// instead of redirecting its value at whatever later addition reused the place.
fn stale_fit_label_resolver(fit_id: &str, label: &str) -> EditResolver {
    let fit_id = fit_id.to_owned();
    let label = label.to_owned();
    EditResolver::new("critical-fit-label", move |accepted: &AcceptedSnapshot| {
        let edit = MechanicalEdit {
            target: boardstudio_core::mechanical::MechanicalTarget::Board {
                board_id: "main".into(),
            },
            change: MechanicalChange::SetCriticalFitText {
                fit_id: fit_id.clone(),
                field: MechanicalCriticalFitTextField::Label,
                value: label.clone(),
            },
            mounting_hole: Some(Box::new(mechanical_template())),
        };
        // The production controller runs Core's read-only evaluator first: an
        // Unchanged or Retire answer settles locally; a removed target the evaluator
        // cannot classify reaches Core as a rejected edit.
        match boardstudio_core::mechanical::evaluate_edit(
            &accepted.document,
            &accepted.scene.board_contours,
            &edit,
        ) {
            Ok(boardstudio_core::mechanical::MechanicalEditDisposition::Unchanged) => {
                Resolution::Unchanged
            }
            Ok(boardstudio_core::mechanical::MechanicalEditDisposition::Retire) => {
                Resolution::Retire("The mechanical target is no longer available.".into())
            }
            _ => Resolution::submit(
                vec!["main".into()],
                EditOperation::ApplyMechanicalEdit {
                    edit: Box::new(edit),
                },
            ),
        }
    })
}

fn fit_labels(document: &ProjectDoc) -> Vec<(String, String)> {
    document
        .mechanical
        .as_ref()
        .and_then(|configuration| configuration.critical_fits.as_ref())
        .into_iter()
        .flatten()
        .map(|fit| (fit.id.clone(), fit.label.clone()))
        .collect()
}

#[test]
fn queued_stale_fit_setter_retires_after_remove_add_first_gap_reuse() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: mechanical_fixture(&["case-critical-fit-1"]),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);

    let effects = session.submit(resolved_edit(
        OperationId(2),
        remove_fit_command("remove-fit", "case-critical-fit-1"),
    ));
    assert!(has_core_request(&effects), "the removal starts immediately");
    let queued = session.submit(resolved_edit(OperationId(3), add_fit_command("add-fit")));
    assert!(
        !has_core_request(&queued),
        "the addition is queued behind it"
    );
    let queued = session.submit(Event::ResolveEdit {
        operation_id: OperationId(4),
        label: "stale-critical-fit-label".into(),
        resolver: stale_fit_label_resolver("case-critical-fit-1", "stale value"),
    });
    assert!(
        !has_core_request(&queued),
        "the stale setter is queued behind the replacement sequence"
    );

    let (_, add_effects) = settle_core_and_save(&mut session, &mut engine, effects);
    assert!(
        has_core_request(&add_effects),
        "the queued addition runs after the removal"
    );
    let (_, stale_effects) = settle_core_and_save(&mut session, &mut engine, add_effects);
    let (request_id, epoch, request) = core_effect(&stale_effects);
    let reply = engine.handle(request);
    assert!(
        matches!(reply, CoreReply::Error { .. }),
        "the stale setter must be rejected: {reply:?}"
    );
    let resolved = session.complete(Completion::Core {
        request_id,
        executor_epoch: epoch,
        reply: Box::new(reply),
    });
    let (outcome, landing) = settled_landing(&resolved, OperationId(4));
    assert!(matches!(outcome, TerminalOutcome::Rejected(_)));
    assert!(landing.is_none());

    let accepted = session.read_model().accepted.clone().unwrap();
    let labels = fit_labels(&accepted.document);
    assert_eq!(
        labels,
        [("case-critical-fit-2".into(), "Critical dimension".into())]
    );
}

#[test]
fn queued_stale_fit_setter_retires_after_undo_then_add() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: mechanical_fixture(&["case-critical-fit-1"]),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);

    let effects = session.submit(resolved_edit(OperationId(2), add_fit_command("first-add")));
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);
    let accepted = session.read_model().accepted.clone().unwrap();
    assert!(
        fit_labels(&accepted.document)
            .iter()
            .any(|(id, _)| id == "case-critical-fit-2")
    );

    let effects = session.submit(Event::Undo {
        operation_id: OperationId(3),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);

    let effects = session.submit(resolved_edit(OperationId(4), add_fit_command("second-add")));
    assert!(has_core_request(&effects));
    let queued = session.submit(Event::ResolveEdit {
        operation_id: OperationId(5),
        label: "stale-critical-fit-label".into(),
        resolver: stale_fit_label_resolver("case-critical-fit-2", "stale value"),
    });
    assert!(!has_core_request(&queued));

    let (_, stale_effects) = settle_core_and_save(&mut session, &mut engine, effects);
    let (request_id, epoch, request) = core_effect(&stale_effects);
    let reply = engine.handle(request);
    assert!(
        matches!(reply, CoreReply::Error { .. }),
        "the stale setter must be rejected: {reply:?}"
    );
    let resolved = session.complete(Completion::Core {
        request_id,
        executor_epoch: epoch,
        reply: Box::new(reply),
    });
    let (outcome, landing) = settled_landing(&resolved, OperationId(5));
    assert!(matches!(outcome, TerminalOutcome::Rejected(_)));
    assert!(landing.is_none());

    let accepted = session.read_model().accepted.clone().unwrap();
    assert_eq!(
        fit_labels(&accepted.document),
        [
            ("case-critical-fit-1".into(), "Critical dimension".into()),
            ("case-critical-fit-3".into(), "Critical dimension".into()),
        ],
        "undo/add must mint a fresh identity and the stale setter must not redirect"
    );
}

#[test]
fn carried_shift_preview_cancel_commit_and_history_use_the_accepted_matrix() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let (document, _) = matrix_range_fixture();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document,
    });
    settle_core_and_save(&mut session, &mut engine, effects);
    let before = session.read_model().accepted.clone().unwrap();
    let key = "matrix/main/r1c0";
    assert_eq!(
        before
            .document
            .parts
            .iter()
            .find(|p| p.id == key)
            .unwrap()
            .pose
            .at,
        Vec2 { x: 0.0, y: 19.0 }
    );
    let operation = EditOperation::SetMatrixPlacement {
        matrix_id: "main".into(),
        change: MatrixPlacementChange::RowShift {
            row: 1,
            value: Vec2 { x: 4.0, y: -3.0 },
        },
    };
    let effects = session.submit(Event::PreviewEdit {
        operation_id: OperationId(2),
        transaction_id: "shift-preview".into(),
        target_ids: vec![key.into()],
        operation: operation.clone(),
    });
    let (request_id, executor_epoch, request) = core_effect(&effects);
    assert!(
        matches!(&request, CoreRequest::Edit { command, .. } if command.phase == EditPhase::Preview)
    );
    let reply = engine.handle(request);
    assert!(matches!(&reply, CoreReply::Preview { .. }));
    session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(reply),
    });
    let preview = session.read_model().display_preview.clone().unwrap();
    assert_eq!(
        preview.matrix_scenes[0]
            .cells
            .iter()
            .find(|c| c.row == 1 && c.column == 0)
            .unwrap()
            .pose
            .at,
        Vec2 { x: 4.0, y: 16.0 }
    );
    assert_eq!(
        session.read_model().accepted.as_ref().unwrap().document,
        before.document
    );
    let effects = session.submit(Event::ClearPreview {
        operation_id: OperationId(3),
        token: before.token,
        revision: before.document.revision,
        board_id: session.read_model().active_board_id.clone(),
        transaction_id: "shift-preview".into(),
    });
    // Clearing a completed preview is view-only and never persists a document.
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::Persist { .. }))
    );
    assert!(session.read_model().display_preview.is_none());
    let effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(4),
        label: "Shift field".into(),
        resolver: EditResolver::new("Shift field", move |_| {
            Resolution::submit(vec![key.into()], operation.clone())
        }),
    });
    let (after, _) = settle_core_and_save(&mut session, &mut engine, effects);
    assert_eq!(
        session
            .read_model()
            .accepted
            .as_ref()
            .unwrap()
            .scene
            .matrix_scenes,
        preview.matrix_scenes
    );
    assert_eq!(
        after.parts.iter().find(|p| p.id == key).unwrap().pose.at,
        Vec2 { x: 4.0, y: 16.0 }
    );
    assert_eq!(after.matrices[0].cells, before.document.matrices[0].cells);
    let effects = session.submit(Event::Undo {
        operation_id: OperationId(5),
    });
    let (undone, _) = settle_core_and_save(&mut session, &mut engine, effects);
    assert_eq!(undone.matrices, before.document.matrices);
    assert_eq!(undone.parts, before.document.parts);
    let effects = session.submit(Event::Redo {
        operation_id: OperationId(6),
    });
    let (redone, _) = settle_core_and_save(&mut session, &mut engine, effects);
    assert_eq!(redone.matrices, after.matrices);
    assert_eq!(redone.parts, after.parts);
}

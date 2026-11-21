#[path = "durable_session/gestures.rs"]
mod gestures;
#[path = "durable_session/mechanical.rs"]
mod mechanical;
#[path = "durable_session/navigation.rs"]
mod navigation;
#[path = "durable_session/recovery.rs"]
mod recovery;
#[path = "durable_session/resolved_edits.rs"]
mod resolved_edits;

use boardstudio_application::{
    AcceptedSnapshot, Completion, Durability, EditResolver, Effect, Event, ExecutorEpoch, Landing,
    Lifecycle, OperationId, RequestId, Resolution, SaveAttemptId, SaveResult, Session,
    TerminalOutcome,
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
    let mut session = Session::new(CoreEngine::validate_open);
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
    let mut session = Session::new(CoreEngine::validate_open);
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
    let mut session = Session::new(CoreEngine::validate_open);
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

fn rename_edit_command(base_revision: u64, name: &str) -> EditCommand {
    EditCommand {
        base_revision,
        transaction_id: format!("rename-{name}"),
        phase: EditPhase::Commit,
        target_ids: vec!["project".into()],
        operation: EditOperation::RenameProject { name: name.into() },
    }
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
    let mut session = Session::new(CoreEngine::validate_open);
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
    let mut session = Session::new(CoreEngine::validate_open);
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
fn carried_shift_preview_cancel_commit_and_history_use_the_accepted_matrix() {
    let mut session = Session::new(CoreEngine::validate_open);
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

#[test]
fn unrelated_rejection_does_not_cancel_active_gesture() {
    let mut session = Session::new(CoreEngine::validate_open);
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    session.submit(Event::GestureBegin {
        operation_id: OperationId(900),
        pointer_id: 7,
        target_ids: vec!["key".into()],
        transaction_id: "gesture".into(),
        start: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
        }],
        pitch: Vec2 { x: 19.0, y: 19.0 },
        snap_fraction: 0.0,
        geometry_snap: false,
        gap: None,
        pixels_per_world: None,
        alt: false,
    });
    let generation = session.read_model().gesture.as_ref().unwrap().generation;
    let effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(901),
        label: "invalid".into(),
        resolver: EditResolver::new("invalid", |accepted: &AcceptedSnapshot| {
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
    let (request_id, executor_epoch, request) = core_effect(&effects);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(engine.handle(request)),
    });
    assert!(matches!(
        settled_landing(&effects, OperationId(901)).0,
        TerminalOutcome::Rejected(_)
    ));
    assert_eq!(
        session.read_model().gesture.as_ref().unwrap().generation,
        generation
    );
    assert!(
        !effects
            .iter()
            .any(|e| matches!(e, Effect::ReleasePointer { pointer_id: 7 }))
    );
}

#[test]
fn ending_gesture_at_origin_preserves_other_transaction_preview() {
    let mut session = Session::new(CoreEngine::validate_open);
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let effects = session.submit(Event::PreviewEdit {
        operation_id: OperationId(902),
        transaction_id: "other".into(),
        target_ids: vec!["key".into()],
        operation: EditOperation::MoveParts {
            positions: vec![Position {
                id: "key".into(),
                at: Vec2 { x: 2.0, y: 0.0 },
            }],
        },
    });
    let (request_id, executor_epoch, request) = core_effect(&effects);
    let mut reply = engine.handle(request);
    add_preview_electrical_notice(&mut reply, "other");
    session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(reply),
    });
    session.submit(Event::GestureBegin {
        operation_id: OperationId(903),
        pointer_id: 7,
        target_ids: vec!["key".into()],
        transaction_id: "gesture".into(),
        start: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
        }],
        pitch: Vec2 { x: 19.0, y: 19.0 },
        snap_fraction: 0.0,
        geometry_snap: false,
        gap: None,
        pixels_per_world: None,
        alt: false,
    });
    session.submit(Event::GestureEnd {
        pointer_id: 7,
        final_positions: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
        }],
        pixels_per_world: None,
        alt: false,
    });
    assert!(session.read_model().gesture.is_none());
    assert!(
        session
            .read_model()
            .display_preview
            .as_ref()
            .unwrap()
            .findings
            .iter()
            .any(|finding| finding.id == "preview:electrical:other")
    );
}

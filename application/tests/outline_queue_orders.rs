//! Integrated queue orders across matrix, mechanical and outline actions (typed Core
//! edits 15, required case 1).
//!
//! Matrix placement, a typed mechanical change and a typed outline change are queued
//! at the real Session seam in both orders. Each queued resolver must resolve against
//! the document the *previous* accepted edit produced - not the state at submission
//! time - and every accepted action must own exactly one Undo step.
use boardstudio_application::{
    AcceptedSnapshot, Completion, EditResolver, Effect, Event, ExecutorEpoch, OperationId,
    RequestId, Resolution, SaveAttemptId, SaveResult, Session,
};
use boardstudio_core::mechanical::change::{MechanicalChange, MechanicalEdit, MechanicalTarget};
use boardstudio_core::outline_edit::{OutlineChange, OutlineEdit, OutlineEditTarget};
use boardstudio_core::{CoreEngine, model::*};
use serde_json::json;
use std::cell::RefCell;
use std::rc::Rc;

/// The three action kinds, so both queue orders share one driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Matrix,
    Mechanical,
    Outline,
}

/// What each resolver observed in the accepted document when it ran.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Observed {
    stagger: f64,
    mechanical: bool,
    margin: f64,
}

/// One board carrying a matrix, two components and the board's Generated envelope.
fn fixture() -> ProjectDoc {
    let mut document = ProjectDoc::empty("queue-orders", "Queue orders");
    document.definitions.push(PartDefinition {
        mechanical_profile: None,
        id: "switch".into(),
        name: "Switch".into(),
        kind: PartKind::Switch,
        courtyard: vec![
            Vec2 { x: -7.0, y: -7.0 },
            Vec2 { x: 7.0, y: -7.0 },
            Vec2 { x: 7.0, y: 7.0 },
            Vec2 { x: -7.0, y: 7.0 },
        ],
        pads: vec![],
        models: None,
        hardware_profile: None,
        input_profile: None,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        generator: None,
    });
    let mut part_ids = Vec::new();
    for (index, at) in [0.0, 19.0].into_iter().enumerate() {
        let id = format!("matrix/main/r0c{index}");
        document.parts.push(Part {
            id: id.clone(),
            definition_id: "switch".into(),
            reference: format!("SW{}", index + 1),
            pose: Pose2 {
                at: Vec2 { x: at, y: 0.0 },
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
        part_ids.push(id);
    }
    document.boards.push(Board {
        id: "main".into(),
        name: "Main".into(),
        outline_ids: vec!["main-gen".into()],
        part_ids: part_ids.clone(),
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.matrices.push(Matrix {
        id: "main".into(),
        name: None,
        rows: 1,
        columns: 2,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2 { x: 0.0, y: 0.0 },
        definition_id: "switch".into(),
        part_ids: part_ids.clone(),
        board_id: Some("main".into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: vec![],
    });
    document.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        settings: OutlineSettings {
            corners: CornerStyle::Fillet,
            size: 2.0,
            bridge_width: 10.0,
            repair: None,
        },
        id: "main-gen".into(),
        part_ids,
        margin: 4.0,
        operation: Operation::Add,
    });
    document
}

/// The accepted document's three observables, one per action kind.
fn observed(accepted: &AcceptedSnapshot) -> Observed {
    let document = &accepted.document;
    Observed {
        stagger: document
            .matrices
            .first()
            .and_then(|matrix| {
                matrix
                    .column_layout
                    .first()
                    .map(|record| &record.stagger)
                    .copied()
            })
            .unwrap_or(0.0),
        mechanical: document
            .mechanical
            .as_ref()
            .is_some_and(|configuration| configuration.board_id == "main"),
        margin: document
            .outline
            .iter()
            .find_map(|feature| match feature {
                OutlineFeature::PartEnvelope { id, margin, .. } if id == "main-gen" => {
                    Some(*margin)
                }
                _ => None,
            })
            .unwrap_or_default(),
    }
}

/// The typed command for one action, derived from the accepted document: the matrix
/// edit adds to the accepted stagger and the outline edit adds to the accepted margin,
/// so a stale read is observable.
fn operation(action: Action, accepted: &AcceptedSnapshot) -> EditOperation {
    match action {
        Action::Matrix => {
            let stagger = accepted
                .document
                .matrices
                .first()
                .and_then(|matrix| matrix.column_layout.first().map(|record| &record.stagger).copied())
                .unwrap_or(0.0);
            EditOperation::SetMatrixPlacement {
                matrix_id: "main".into(),
                change: MatrixPlacementChange::ColumnStagger {
                    column: 0,
                    value: stagger + 4.5,
                },
            }
        }
        Action::Mechanical => EditOperation::ApplyMechanicalEdit {
            edit: Box::new(MechanicalEdit {
                target: MechanicalTarget::Board {
                    board_id: "main".into(),
                },
                change: MechanicalChange::Enable,
                mounting_hole: Some(Box::new(
                    serde_json::from_value(json!({
                        "id": "generator:ceoloide/mounting_hole_npth", "name": "mounting hole npth",
                        "kind": "custom",
                        "pads": [{
                            "id": "pad-0", "number": "", "at": { "x": 0, "y": 0 },
                            "size": { "x": 2.2, "y": 2.2 }, "shape": "circle", "drill": 2.2,
                            "plated": false, "rotation": 0
                        }],
                        "courtyard": [
                            { "x": -1.1, "y": -1.1 }, { "x": 1.1, "y": -1.1 },
                            { "x": 1.1, "y": 1.1 }, { "x": -1.1, "y": 1.1 }
                        ],
                        "envelopeSource": { "courtyard": "generated" },
                        "envelopeNotice": "No closed courtyard is available; the outline uses physical graphics and pad extents.",
                        "generator": { "source": "ceoloide/mounting_hole_npth", "version": "bundled-1", "parameters": {} }
                    }))
                    .expect("the mounting-hole template parses"),
                )),
            }),
        },
        Action::Outline => {
            let margin = accepted
                .document
                .outline
                .iter()
                .find_map(|feature| match feature {
                    OutlineFeature::PartEnvelope { id, margin, .. } if id == "main-gen" => {
                        Some(*margin)
                    }
                    _ => None,
                })
                .unwrap_or_default();
            EditOperation::ApplyOutlineEdit {
                edit: Box::new(OutlineEdit {
                    target: OutlineEditTarget::Board {
                        board_id: "main".into(),
                    },
                    change: OutlineChange::SetMargin {
                        margin: margin + 1.0,
                    },
                    seed: 7,
                    reviewed: None,
                }),
            }
        }
    }
}

fn label(action: Action) -> &'static str {
    match action {
        Action::Matrix => "matrix-stagger",
        Action::Mechanical => "mechanical-enable",
        Action::Outline => "outline-margin",
    }
}

/// A resolver that records what it observed and submits the typed command for its
/// action against the revision it was handed.
fn resolver(action: Action, seen: Rc<RefCell<Vec<Observed>>>) -> EditResolver {
    EditResolver::new(label(action), move |accepted: &AcceptedSnapshot| {
        seen.borrow_mut().push(observed(accepted));
        Resolution::Submit(EditCommand {
            base_revision: accepted.document.revision,
            transaction_id: String::new(),
            phase: EditPhase::Commit,
            target_ids: vec!["main".into()],
            operation: operation(action, accepted),
        })
    })
}

fn core_effect(
    effects: &[Effect],
) -> (
    RequestId,
    ExecutorEpoch,
    boardstudio_core::model::CoreRequest,
) {
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
        .unwrap_or_else(|| panic!("one persistence request, got {effects:#?}"))
}

fn has_core_request(effects: &[Effect]) -> bool {
    effects
        .iter()
        .any(|effect| matches!(effect, Effect::Core { .. }))
}

/// Settles one core round and its save, returning the effects the completion
/// produced (which carry the next queued edit's core request).
fn settle_one(session: &mut Session, engine: &mut CoreEngine, effects: Vec<Effect>) -> Vec<Effect> {
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
        result: SaveResult::Committed,
    })
}

/// Queues the actions in the given order and asserts the whole queue contract.
fn run(order: [Action; 3]) {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: fixture(),
    });
    settle_one(&mut session, &mut engine, effects);

    let seen = Rc::new(RefCell::new(Vec::new()));
    let mut head: Option<Vec<Effect>> = None;
    for (index, action) in order.into_iter().enumerate() {
        let effects = session.submit(Event::ResolveEdit {
            operation_id: OperationId(2 + index as u64),
            label: label(action).into(),
            resolver: resolver(action, seen.clone()),
        });
        if index == 0 {
            assert!(
                has_core_request(&effects),
                "the head of an idle queue resolves immediately"
            );
            head = Some(effects);
        } else {
            assert!(
                !has_core_request(&effects),
                "{action:?} waits for the running edit"
            );
        }
    }

    // The completions drain the queue: each settled edit starts the next queued one.
    let mut effects = head.expect("the first edit runs");
    for position in 1..=3 {
        effects = settle_one(&mut session, &mut engine, effects);
        if position == 3 {
            assert!(
                !has_core_request(&effects),
                "the queue is empty after the last edit"
            );
        } else {
            assert!(
                has_core_request(&effects),
                "the next queued edit starts when its predecessor settles"
            );
        }
    }

    // Each resolver observed the commit accepted before it, and the observed state
    // moved in the order the actions were queued.
    let seen = seen.borrow().clone();
    assert_eq!(seen.len(), 3, "every queued resolver ran exactly once");
    assert_eq!(
        seen[0],
        Observed {
            stagger: 0.0,
            mechanical: false,
            margin: 4.0
        }
    );
    let mut states = vec![seen[0]];
    for action in order {
        let mut state = *states.last().unwrap();
        match action {
            Action::Matrix => state.stagger = 4.5,
            Action::Mechanical => state.mechanical = true,
            Action::Outline => state.margin = 5.0,
        }
        states.push(state);
    }
    assert_eq!(
        seen,
        states[..3].to_vec(),
        "each queued edit resolves against the document its predecessor produced"
    );

    let accepted = session.read_model().accepted.clone().unwrap();
    assert_eq!(
        observed(&accepted),
        Observed {
            stagger: 4.5,
            mechanical: true,
            margin: 5.0
        },
        "the queue accepted the matrix, mechanical and outline edits"
    );

    // One Undo step per accepted action, in reverse queue order.
    let mut undone = Vec::new();
    for (index, operation_id) in (5..8).enumerate() {
        let effects = session.submit(Event::Undo {
            operation_id: OperationId(operation_id),
        });
        settle_one(&mut session, &mut engine, effects);
        let accepted = session.read_model().accepted.clone().unwrap();
        undone.push(observed(&accepted));
        assert_eq!(
            undone.len(),
            index + 1,
            "each Undo settles one step of the queue"
        );
    }
    // Undo removes the last queued action first, so the states walk back to the start.
    let expected: Vec<Observed> = (0..3).map(|step| states[2 - step]).collect();
    assert_eq!(
        undone, expected,
        "one Undo steps back exactly one accepted action"
    );
}

#[test]
fn matrix_mechanical_and_outline_edits_queued_in_order_resolve_against_the_accepted_document() {
    run([Action::Matrix, Action::Mechanical, Action::Outline]);
}

#[test]
fn the_same_three_actions_queued_in_reverse_order_hold_the_same_contract() {
    run([Action::Outline, Action::Mechanical, Action::Matrix]);
}

/// The fixture plus two outline versions on the board: a linked refinement (active) that
/// follows the board's components, and a fixed edited copy that keeps its authored shape.
fn fixture_with_versions() -> ProjectDoc {
    let mut document = fixture();
    let sharp = json!({
        "corners": "sharp", "size": 2.0, "bridgeWidth": 10.0,
    });
    document.board_outlines = vec![
        serde_json::from_value(json!({
            "boardId": "main",
            "activeVersionId": "linked",
            "versions": [
                { "id": "linked", "name": "Linked", "source": { "revision": 0 },
                  "ownership": "linked",
                  "geometry": { "features": [{
                        "kind": "part-envelope", "id": "board:main:version:linked:contour:0",
                        "partIds": ["matrix/main/r0c0", "matrix/main/r0c1"],
                        "margin": 1.0, "operation": "add", "settings": sharp
                    }], "settings": sharp, "expectedRegions": 1 } },
                { "id": "fixed", "name": "Fixed", "source": { "revision": 0 },
                  "geometry": { "features": [{
                        "kind": "polygon", "id": "board:main:version:fixed:contour:0",
                        "points": [
                            { "x": -30.0, "y": -30.0 }, { "x": 30.0, "y": -30.0 },
                            { "x": 30.0, "y": 30.0 }, { "x": -30.0, "y": 30.0 }
                        ], "operation": "add"
                    }], "settings": sharp, "expectedRegions": 1 } }
            ]
        }))
        .expect("the outline versions parse"),
    ];
    document
}

fn board_bounds(accepted: &AcceptedSnapshot) -> (f64, f64, f64, f64) {
    accepted
        .scene
        .board_contours
        .iter()
        .find(|board| board.board_id == "main")
        .expect("the board has a resolved boundary")
        .contours
        .iter()
        .flat_map(|contour| &contour.points)
        .fold(
            (
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ),
            |(min_x, min_y, max_x, max_y), point| {
                (
                    min_x.min(point.x),
                    min_y.min(point.y),
                    max_x.max(point.x),
                    max_y.max(point.y),
                )
            },
        )
}

fn version_json(accepted: &AcceptedSnapshot, id: &str) -> serde_json::Value {
    let version = accepted.document.board_outlines[0]
        .versions
        .iter()
        .find(|version| version.id == id)
        .expect("the version exists");
    serde_json::to_value(version).unwrap()
}

/// A matrix placement change and a settings edit on the linked refinement, queued in
/// either order, leave the linked alternative following the new placement and settings
/// while the fixed alternative keeps exactly the shape it was saved with. Expected
/// extents are worked by hand: margin 2 makes a switch envelope 7 + 2 = 9 mm each side,
/// the two switches sit 19 mm apart (x -9..28), and the 4.5 mm column stagger lifts the
/// grid to y = 4.5, so the boundary spans y -4.5..13.5.
fn run_linked(order: [bool; 2]) {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: fixture_with_versions(),
    });
    settle_one(&mut session, &mut engine, effects);
    let opened = session.read_model().accepted.clone().unwrap();
    let fixed_before = version_json(&opened, "fixed");
    assert_eq!(board_bounds(&opened), (-8.0, -8.0, 27.0, 8.0));

    let mut head = None;
    for (index, is_matrix) in order.into_iter().enumerate() {
        let resolver = if is_matrix {
            EditResolver::new("matrix-stagger", |accepted: &AcceptedSnapshot| {
                Resolution::Submit(EditCommand {
                    base_revision: accepted.document.revision,
                    transaction_id: String::new(),
                    phase: EditPhase::Commit,
                    target_ids: vec!["main".into()],
                    operation: operation(Action::Matrix, accepted),
                })
            })
        } else {
            EditResolver::new("linked-margin", |accepted: &AcceptedSnapshot| {
                Resolution::Submit(EditCommand {
                    base_revision: accepted.document.revision,
                    transaction_id: String::new(),
                    phase: EditPhase::Commit,
                    target_ids: vec!["main".into()],
                    operation: EditOperation::ApplyOutlineEdit {
                        edit: Box::new(OutlineEdit {
                            target: OutlineEditTarget::Version {
                                board_id: "main".into(),
                                version_id: "linked".into(),
                            },
                            change: OutlineChange::SetMargin { margin: 2.0 },
                            seed: 7,
                            reviewed: None,
                        }),
                    },
                })
            })
        };
        let effects = session.submit(Event::ResolveEdit {
            operation_id: OperationId(2 + index as u64),
            label: if is_matrix {
                "matrix-stagger"
            } else {
                "linked-margin"
            }
            .into(),
            resolver,
        });
        if index == 0 {
            head = Some(effects);
        }
    }
    let effects = settle_one(
        &mut session,
        &mut engine,
        head.expect("the first edit runs"),
    );
    settle_one(&mut session, &mut engine, effects);

    let accepted = session.read_model().accepted.clone().unwrap();
    assert!(
        accepted
            .document
            .parts
            .iter()
            .all(|part| part.pose.at.y == 4.5),
        "the stagger moved the components the linked refinement is built from"
    );
    assert_eq!(
        board_bounds(&accepted),
        (-9.0, -4.5, 28.0, 13.5),
        "the linked refinement follows the moved components and its own margin"
    );
    assert_eq!(
        version_json(&accepted, "fixed"),
        fixed_before,
        "the fixed alternative keeps the shape it was saved with"
    );
}

#[test]
fn a_linked_refinement_follows_a_queued_placement_and_its_fixed_alternative_stays() {
    run_linked([true, false]);
}

#[test]
fn the_same_linked_and_fixed_contract_holds_in_reverse_queue_order() {
    run_linked([false, true]);
}

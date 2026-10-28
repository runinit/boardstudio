//! Native Session/Core coverage for the mechanical operation.
//!
//! These tests drive the real `Session` and real `CoreEngine` through the native test
//! Runtime, so they prove queue ordering, history and durable-target behaviour rather
//! than the operation in isolation. The mechanical submissions are built exactly as the
//! Case adapter builds them: Core's read-only evaluator decides Changed/Unchanged/
//! Retire and the accepted edit is one `ApplyMechanicalEdit` with its template.
#![cfg(all(test, not(target_arch = "wasm32")))]

use boardstudio_application::{EditResolver, Event, Resolution, Scope};
use boardstudio_core::mechanical::change::{MechanicalChange, MechanicalDimension, MechanicalEdit};
use boardstudio_core::mechanical::{MechanicalEditDisposition, MechanicalTarget, evaluate_edit};
use boardstudio_core::model::{EditOperation, MatrixPlacementChange, PartDefinition, ProjectDoc};
use boardstudio_web_runtime::pending_edits::{PendingEditResult, PendingEdits};
use boardstudio_web_runtime::runtime::Runtime;
use serde_json::json;
use std::cell::Cell;
use std::rc::Rc;

/// The normalized mounting-hole definition Core validates on every supplied template.
fn mounting_hole() -> PartDefinition {
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
    .expect("the mounting-hole fixture parses")
}

/// A board with one two-column matrix and a canonical mechanical configuration.
fn document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("mechanical-session", "Mechanical session");
    document.boards.push(
        serde_json::from_value(json!({
            "id": "board", "name": "Board", "outlineIds": [], "partIds": [],
            "netIds": [], "thickness": 1.6, "traces": [], "vias": []
        }))
        .expect("the board parses"),
    );
    document.definitions.push(
        serde_json::from_value(json!({
            "id": "switch-definition", "name": "Switch", "kind": "switch",
            "pads": [], "courtyard": [],
            "terminals": { "1": ["1"], "2": ["2"] }
        }))
        .expect("the switch definition parses"),
    );
    document.matrices.push(
        serde_json::from_value(json!({
            "id": "matrix", "rows": 1, "columns": 2,
            "pitch": { "x": 19.0, "y": 19.0 }, "origin": { "x": 0.0, "y": 0.0 },
            "definitionId": "switch-definition", "partIds": [], "boardId": "board",
            "diodeDirection": "row2col", "rowLayout": [], "columnLayout": [],
        "cells": []
        }))
        .expect("the matrix parses"),
    );
    document.mechanical = Some(
        serde_json::from_value(json!({
            "boardId": "board", "method": "printed", "mount": "tray",
            "integratedPlateFrame": false, "bottomStyle": "shell", "middleFrame": false,
            "plateThickness": 1.5, "plateFoamThickness": 0.5, "pcbThickness": 1.6,
            "bottomFoamThickness": 2.0, "batteryHeight": 0.0, "bottomThickness": 3.0,
            "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3,
            "mounts": [], "closureMounts": [], "openings": [], "profiles": []
        }))
        .expect("the mechanical configuration parses"),
    );
    document
}

fn open() -> (Rc<Runtime>, Scope) {
    let runtime = Runtime::new();
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: document(),
    });
    let mut scope = runtime.scope().expect("the open document has a scope");
    scope.board_id = "board".into();
    (runtime, scope)
}

fn wall_thickness(runtime: &Runtime) -> f64 {
    runtime
        .model()
        .accepted
        .expect("an accepted document")
        .document
        .mechanical
        .as_ref()
        .expect("the canonical mechanical configuration")
        .wall_thickness
}

fn column_stagger(runtime: &Runtime) -> f64 {
    runtime
        .model()
        .accepted
        .expect("an accepted document")
        .document
        .matrices[0]
        .column_layout
        .first()
        .map(|record| &record.stagger)
        .copied()
        .unwrap_or(0.0)
}

/// The mechanical submission the Case adapter builds: evaluator disposition first, then
/// one typed edit with its template.
fn mechanical_resolver(
    scope: Scope,
    change: MechanicalChange,
    template: PartDefinition,
    saw: Rc<Cell<bool>>,
    expect: Box<dyn Fn(&ProjectDoc) -> bool>,
) -> EditResolver {
    EditResolver::new("mechanical", move |accepted| {
        saw.set(expect(&accepted.document));
        let owner = scope
            .instance_id
            .clone()
            .unwrap_or_else(|| scope.board_id.clone());
        let target = match &scope.instance_id {
            Some(instance_id) => MechanicalTarget::Instance {
                instance_id: instance_id.clone(),
            },
            None => MechanicalTarget::Board {
                board_id: scope.board_id.clone(),
            },
        };
        let edit = MechanicalEdit {
            target,
            change: change.clone(),
            mounting_hole: Some(Box::new(template.clone())),
        };
        match evaluate_edit(&accepted.document, &accepted.scene.board_contours, &edit) {
            Ok(MechanicalEditDisposition::Unchanged) => Resolution::Unchanged,
            Ok(MechanicalEditDisposition::Retire) => {
                Resolution::Retire("The mechanical target is no longer available.".into())
            }
            // A malformed value stays a Core edit error: submitting lets Core report it.
            Ok(MechanicalEditDisposition::Changed) | Err(_) => Resolution::submit(
                vec![owner],
                EditOperation::ApplyMechanicalEdit {
                    edit: Box::new(edit),
                },
            ),
        }
    })
}

fn matrix_resolver(
    matrix_id: &str,
    stagger: f64,
    saw: Rc<Cell<bool>>,
    expect: Box<dyn Fn(&ProjectDoc) -> bool>,
) -> EditResolver {
    let matrix_id = matrix_id.to_owned();
    EditResolver::new("matrix", move |accepted| {
        saw.set(expect(&accepted.document));
        Resolution::submit(
            vec![matrix_id.clone()],
            EditOperation::SetMatrixPlacement {
                matrix_id: matrix_id.clone(),
                change: MatrixPlacementChange::ColumnStagger {
                    column: 0,
                    value: stagger,
                },
            },
        )
    })
}

/// The visible configuration, ignoring the identity reservations.
///
/// Reservations are bookkeeping that must persist and may advance when an operation
/// reserves an identity; they never move geometry or change a setting.
fn without_reservations(
    mut configuration: boardstudio_core::model::MechanicalConfiguration,
) -> boardstudio_core::model::MechanicalConfiguration {
    configuration.next_opening_identity = 0;
    if let Some(openings) = configuration.openings.as_mut() {
        for opening in openings.iter_mut() {
            opening.next_vertex_identity = 0;
        }
    }
    configuration
}

fn undo(runtime: &Rc<Runtime>) {
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
}

fn redo(runtime: &Rc<Runtime>) {
    runtime.submit(Event::Redo {
        operation_id: runtime.operation(),
    });
}

#[test]
fn matrix_then_mechanical_settles_in_order_and_undoes_one_step_each() {
    let (runtime, scope) = open();
    // The matrix edit's Core reply is held so the mechanical edit queues behind it.
    runtime.hold_next_core();
    let matrix_saw = Rc::new(Cell::new(false));
    let mechanical_saw = Rc::new(Cell::new(false));
    let mut matrix_edits = PendingEdits::default();
    matrix_edits.begin(
        &runtime,
        1u8,
        "matrix",
        Some("matrix placement".into()),
        matrix_resolver("matrix", 1.5, matrix_saw.clone(), Box::new(|_| true)),
    );
    let mut mechanical_edits = PendingEdits::default();
    mechanical_edits.begin(
        &runtime,
        2u8,
        "mechanical",
        Some("mechanical settings".into()),
        mechanical_resolver(
            scope.clone(),
            MechanicalChange::SetDimension {
                field: MechanicalDimension::WallThickness,
                value: 3.0,
            },
            mounting_hole(),
            mechanical_saw.clone(),
            // The mechanical resolver only sees the matrix edit once it has landed.
            Box::new(|document| {
                document.matrices[0]
                    .column_layout
                    .first()
                    .map(|record| &record.stagger)
                    .copied()
                    == Some(1.5)
            }),
        ),
    );
    assert!(
        mechanical_edits.settle(true).is_empty(),
        "the queued mechanical edit cannot settle while the matrix edit holds Core"
    );
    assert_eq!(wall_thickness(&runtime), 2.0);

    runtime.release_core();
    let matrix_results = matrix_edits.settle(true);
    let mechanical_results = mechanical_edits.settle(true);
    assert!(matrix_saw.get(), "the matrix edit resolved first");
    assert!(
        matrix_results
            .iter()
            .any(|result| matches!(result, PendingEditResult::Landed { .. })),
        "the matrix edit landed, got {matrix_results:?}"
    );
    assert!(
        mechanical_results
            .iter()
            .any(|result| matches!(result, PendingEditResult::Landed { .. })),
        "the mechanical edit landed, got {mechanical_results:?}"
    );
    assert!(
        mechanical_saw.get(),
        "the mechanical resolver saw the landed matrix edit, proving the queue order"
    );
    assert_eq!(column_stagger(&runtime), 1.5);
    assert_eq!(wall_thickness(&runtime), 3.0);

    // One Undo reverts the mechanical edit only; the matrix edit is still there.
    undo(&runtime);
    assert_eq!(wall_thickness(&runtime), 2.0);
    assert_eq!(column_stagger(&runtime), 1.5);
    undo(&runtime);
    assert_eq!(column_stagger(&runtime), 0.0);
}

#[test]
fn mechanical_then_matrix_settles_in_order_and_undoes_one_step_each() {
    let (runtime, scope) = open();
    runtime.hold_next_core();
    let mechanical_saw = Rc::new(Cell::new(false));
    let matrix_saw = Rc::new(Cell::new(false));
    let mut mechanical_edits = PendingEdits::default();
    mechanical_edits.begin(
        &runtime,
        1u8,
        "mechanical",
        Some("mechanical settings".into()),
        mechanical_resolver(
            scope.clone(),
            MechanicalChange::SetDimension {
                field: MechanicalDimension::WallThickness,
                value: 3.0,
            },
            mounting_hole(),
            mechanical_saw.clone(),
            Box::new(|document| document.matrices[0].column_layout.is_empty()),
        ),
    );
    let mut matrix_edits = PendingEdits::default();
    matrix_edits.begin(
        &runtime,
        2u8,
        "matrix",
        Some("matrix placement".into()),
        matrix_resolver(
            "matrix",
            1.5,
            matrix_saw.clone(),
            Box::new(|document| {
                document
                    .mechanical
                    .as_ref()
                    .is_some_and(|configuration| configuration.wall_thickness == 3.0)
            }),
        ),
    );
    assert!(
        matrix_edits.settle(true).is_empty(),
        "the queued matrix edit cannot settle while the mechanical edit holds Core"
    );

    runtime.release_core();
    let mechanical_results = mechanical_edits.settle(true);
    let matrix_results = matrix_edits.settle(true);
    assert!(
        mechanical_results
            .iter()
            .any(|result| matches!(result, PendingEditResult::Landed { .. })),
        "the mechanical edit landed, got {mechanical_results:?}"
    );
    assert!(
        matrix_results
            .iter()
            .any(|result| matches!(result, PendingEditResult::Landed { .. })),
        "the matrix edit landed, got {matrix_results:?}"
    );
    assert!(
        mechanical_saw.get(),
        "the mechanical resolver saw no earlier matrix change"
    );
    assert!(
        matrix_saw.get(),
        "the matrix resolver saw the landed mechanical edit, proving the queue order"
    );
    assert_eq!(column_stagger(&runtime), 1.5);
    assert_eq!(wall_thickness(&runtime), 3.0);

    // One Undo reverts the matrix edit only; the mechanical edit is still there.
    undo(&runtime);
    assert_eq!(column_stagger(&runtime), 0.0);
    assert_eq!(wall_thickness(&runtime), 3.0);
    undo(&runtime);
    assert_eq!(wall_thickness(&runtime), 2.0);
}

#[test]
fn an_equal_value_mechanical_edit_adds_no_history_step_and_keeps_redo() {
    let (runtime, scope) = open();
    let revisions = |runtime: &Runtime| {
        runtime
            .model()
            .accepted
            .expect("an accepted document")
            .document
            .revision
    };
    let submit = |value: f64| {
        let mut edits = PendingEdits::default();
        edits.begin(
            &runtime,
            1u8,
            "mechanical",
            Some("mechanical settings".into()),
            mechanical_resolver(
                scope.clone(),
                MechanicalChange::SetDimension {
                    field: MechanicalDimension::WallThickness,
                    value,
                },
                mounting_hole(),
                Rc::new(Cell::new(false)),
                Box::new(|_| true),
            ),
        );
        edits.settle(true)
    };

    let landed = submit(3.0);
    assert!(
        landed
            .iter()
            .any(|result| matches!(result, PendingEditResult::Landed { .. })),
        "the real value landed"
    );
    let after_change = revisions(&runtime);
    assert_eq!(wall_thickness(&runtime), 3.0);

    // Undo, then an edit equal to the accepted value: no revision step, no history entry.
    undo(&runtime);
    assert_eq!(wall_thickness(&runtime), 2.0);
    let after_undo = revisions(&runtime);
    let equal = submit(2.0);
    assert!(
        equal
            .iter()
            .any(|result| matches!(result, PendingEditResult::Landed { .. })),
        "an equal value still settles as landed"
    );
    assert_eq!(
        revisions(&runtime),
        after_undo,
        "an equal value needs no revision step, so it adds no history entry"
    );
    assert_ne!(
        after_change, after_undo,
        "the earlier real edit and the undo are separate history steps"
    );

    // The no-op must not have cleared the redo entry for the earlier edit.
    redo(&runtime);
    assert_eq!(
        wall_thickness(&runtime),
        3.0,
        "redo still restores the earlier edit after a no-op submission"
    );
}

/// A queued edit addressed to a removed opening or vertex must never be redirected at a
/// recreated one, and the removed identity must stay retired.
///
/// The durable address is the opening id together with the vertex id. Core reserves both
/// monotonically, so a removed opening identity is never handed to a later opening and a
/// removed vertex identity is never handed to a later vertex of the same opening; the
/// evaluator retires an edit that addresses either.
#[test]
fn a_recreated_opening_does_not_receive_a_queued_edit_to_the_removed_one() {
    let (runtime, scope) = open();
    let submit = |change: MechanicalChange| {
        let mut edits = PendingEdits::default();
        edits.begin(
            &runtime,
            1u8,
            "mechanical",
            Some("mechanical settings".into()),
            mechanical_resolver(
                scope.clone(),
                change,
                mounting_hole(),
                Rc::new(Cell::new(false)),
                Box::new(|_| true),
            ),
        );
        edits.settle(true)
    };
    let openings = |runtime: &Runtime| {
        runtime
            .model()
            .accepted
            .expect("an accepted document")
            .document
            .mechanical
            .as_ref()
            .expect("the canonical mechanical configuration")
            .openings
            .clone()
            .unwrap_or_default()
    };
    let landed = |results: &[PendingEditResult<u8>]| {
        results
            .iter()
            .any(|result| matches!(result, PendingEditResult::Landed { .. }))
    };

    // Add, remove, then add again: Core mints every opening's identity.
    assert!(landed(&submit(MechanicalChange::AddOpening)));
    let (removed_id, removed_vertex) = {
        let opening = openings(&runtime)
            .first()
            .expect("the added opening")
            .clone();
        (opening.id.clone(), opening.vertices[0].id.clone())
    };
    submit(MechanicalChange::RemoveOpening {
        opening_id: removed_id.clone(),
    });
    assert!(
        openings(&runtime).is_empty(),
        "the removal cleared the opening"
    );
    assert!(landed(&submit(MechanicalChange::AddOpening)));
    let recreated = openings(&runtime);
    let recreated_opening = recreated.first().expect("the recreated opening").clone();
    assert_ne!(
        recreated_opening.id, removed_id,
        "a recreated opening must not reuse the removed opening's id"
    );

    // The stale identity retires here rather than landing on the new geometry.
    let before = recreated_opening.points();
    let queued = submit(MechanicalChange::SetOpeningDimension {
        opening_id: removed_id.clone(),
        vertex_id: Some(removed_vertex.clone()),
        field: MechanicalDimension::OpeningPointX,
        value: 4.0,
    });
    assert!(
        queued.iter().any(|result| match result {
            PendingEditResult::Retired { .. } => true,
            PendingEditResult::Failed { message, .. } => message.contains("no longer available"),
            PendingEditResult::Landed { .. } => false,
        }),
        "a queued edit to a removed opening must be refused as gone rather than land, got {queued:?}"
    );
    assert_eq!(
        openings(&runtime)
            .first()
            .expect("the recreated opening")
            .points(),
        before,
        "the stale edit must not be redirected at the recreated opening"
    );

    // The same holds for a vertex removed from the opening that is still present.
    let live_id = recreated_opening.id.clone();
    let removed_live_vertex = recreated_opening.vertices[0].id.clone();
    assert!(landed(&submit(MechanicalChange::RemoveOpeningVertex {
        opening_id: live_id.clone(),
        vertex_id: removed_live_vertex.clone(),
    })));
    assert!(landed(&submit(MechanicalChange::AddOpeningVertex {
        opening_id: live_id.clone(),
    })));
    let after_vertex = openings(&runtime)
        .into_iter()
        .find(|opening| opening.id == live_id)
        .expect("the live opening");
    assert!(
        after_vertex
            .vertices
            .iter()
            .all(|vertex| vertex.id != removed_live_vertex),
        "a removed vertex identity must not be handed to the added vertex, got {:?}",
        after_vertex
            .vertices
            .iter()
            .map(|vertex| vertex.id.as_str())
            .collect::<Vec<_>>()
    );
    let before_vertex = after_vertex.points();
    let queued_vertex = submit(MechanicalChange::SetOpeningDimension {
        opening_id: live_id.clone(),
        vertex_id: Some(removed_live_vertex.clone()),
        field: MechanicalDimension::OpeningPointX,
        value: 9.0,
    });
    assert!(
        queued_vertex.iter().any(|result| match result {
            PendingEditResult::Retired { .. } => true,
            PendingEditResult::Failed { message, .. } => message.contains("no longer available"),
            PendingEditResult::Landed { .. } => false,
        }),
        "a queued edit to a removed vertex must be refused as gone rather than land, got {queued_vertex:?}"
    );
    assert_eq!(
        openings(&runtime)
            .into_iter()
            .find(|opening| opening.id == live_id)
            .expect("the live opening")
            .points(),
        before_vertex,
        "the stale vertex edit must not be redirected at the added vertex"
    );
}

fn paired_document() -> ProjectDoc {
    let mut document = document();
    document.hardware = Some(
        serde_json::from_value(json!({
            "topology": "split", "transport": "none",
            "instances": [
                {
                    "id": "left", "name": "Left", "boardId": "board", "half": "left",
                    "role": "central", "flipped": false,
                    "mechanical": {
                        "boardId": "board", "method": "printed", "mount": "tray",
                        "plateThickness": 1.5, "plateFoamThickness": 0.5, "pcbThickness": 1.6,
                        "bottomFoamThickness": 2.0, "batteryHeight": 0.0, "bottomThickness": 3.0,
                        "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3,
                        "mounts": [{
                            "id": "mount-left", "at": { "x": 2.0, "y": 3.0 },
                            "kind": "hole", "holeDiameter": 2.2
                        }],
                        "closureMounts": [], "openings": [], "profiles": []
                    },
                    "constructionOwnership": "pair"
                },
                {
                    "id": "right", "name": "Right", "boardId": "board", "half": "right",
                    "role": "peripheral", "flipped": true,
                    "mechanical": {
                        "boardId": "board", "method": "printed", "mount": "tray",
                        "plateThickness": 1.5, "plateFoamThickness": 0.5, "pcbThickness": 1.6,
                        "bottomFoamThickness": 2.0, "batteryHeight": 0.0, "bottomThickness": 3.0,
                        "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3,
                        "mounts": [], "closureMounts": [], "openings": [], "profiles": []
                    },
                    "constructionOwnership": "pair"
                },
                {
                    "id": "third", "name": "Third", "boardId": "board", "half": "third",
                    "role": "peripheral", "flipped": false,
                    "constructionOwnership": "legacy"
                }
            ],
            "boards": [],
            "sharedConstruction": {
                "boardId": "board", "method": "printed", "mount": "tray",
                "plateThickness": 1.5, "plateFoamThickness": 0.5, "pcbThickness": 1.6,
                "bottomFoamThickness": 2.0, "batteryHeight": 0.0, "bottomThickness": 3.0,
                "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3,
                "mounts": [], "closureMounts": [], "openings": [], "profiles": []
            },
            "pairs": [{
                "id": "pair-left-right",
                "members": ["left", "right"],
                "mapping": { "at": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 },
                "construction": {
                    "method": "printed", "mount": "tray", "integratedPlateFrame": false,
                    "bottomStyle": "shell", "middleFrame": false,
                    "plateThickness": 1.5, "plateFoamThickness": 0.5,
                    "bottomFoamThickness": 2.0, "bottomThickness": 3.0,
                    "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3
                },
                "features": [{
                    "id": "mount-left",
                    "kind": "mount",
                    "author": "left",
                    "value": {
                        "kind": "mount",
                        "mount": {
                            "id": "mount-left", "at": { "x": 2.0, "y": 3.0 },
                            "kind": "hole", "holeDiameter": 2.2
                        }
                    },
                    "members": [
                        { "instanceId": "left", "featureId": "mount-left", "state": "local" },
                        { "instanceId": "right", "featureId": "mount-right", "state": "linked" }
                    ]
                }]
            }]
        }))
        .expect("the paired fixture parses"),
    );
    document
}

fn open_document(document: ProjectDoc) -> Rc<Runtime> {
    let runtime = Runtime::new();
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document,
    });
    runtime
}

#[test]
fn a_pairs_identity_states_and_geometry_survive_save_and_reopen() {
    let runtime = open_document(paired_document());
    let saved = runtime
        .saved_document("mechanical-session")
        .expect("an accepted edit is saved to the in-memory store");

    // Reopening the saved project is the real reopen path: a fresh engine over the
    // persisted document, not a serde round trip of a struct.
    let reopened = open_document(saved);
    let before = runtime.model().accepted.expect("accepted").document;
    let after = reopened.model().accepted.expect("accepted").document;

    let pair = |document: &ProjectDoc| document.hardware.as_ref().unwrap().pairs[0].clone();
    assert_eq!(
        pair(&after),
        pair(&before),
        "pair identity, mapping, construction and linked features round-trip"
    );
    assert_eq!(
        after.hardware.as_ref().unwrap().instances,
        before.hardware.as_ref().unwrap().instances,
        "member identities, configuration and construction ownership round-trip"
    );

    // The linked feature still derives the same visible geometry on both halves, and
    // the half-local exception is still local.
    let geometry = |runtime: &Runtime, instance_id: &str| {
        let accepted = runtime.model().accepted.expect("accepted");
        boardstudio_core::mechanical::effective_inputs(
            &accepted.document,
            &accepted.scene.board_contours,
            &MechanicalTarget::Instance {
                instance_id: instance_id.into(),
            },
        )
        .expect("the member resolves")
        .configuration
    };
    assert_eq!(geometry(&reopened, "left"), geometry(&runtime, "left"));
    assert_eq!(geometry(&reopened, "right"), geometry(&runtime, "right"));
    assert_eq!(
        pair(&after).features[0].members[0].state,
        boardstudio_core::model::MechanicalFeatureState::Local,
        "the local exception survives the round trip"
    );
}

#[test]
fn reopening_a_project_without_pairs_infers_no_links_and_keeps_legacy_settings() {
    // An existing project: one instance with the legacy shared-construction overlay and
    // no pair. Opening it must not invent a mechanical relationship.
    let mut document = paired_document();
    {
        let hardware = document.hardware.as_mut().unwrap();
        hardware.pairs.clear();
        hardware.instances.truncate(2);
        for instance in &mut hardware.instances {
            instance.construction_ownership =
                boardstudio_core::model::MechanicalConstructionOwnership::Legacy;
            instance.mechanical = None;
        }
    }
    let legacy = {
        let hardware = document.hardware.as_ref().unwrap();
        boardstudio_core::mechanical::effective_inputs(
            &document,
            &[],
            &MechanicalTarget::Instance {
                instance_id: hardware.instances[1].id.clone(),
            },
        )
        .expect("the legacy member resolves its shared construction")
        .configuration
    };

    let runtime = open_document(document);
    let accepted = runtime.model().accepted.expect("accepted");
    assert!(
        accepted
            .document
            .hardware
            .as_ref()
            .expect("hardware")
            .pairs
            .is_empty(),
        "opening an existing project infers no mechanical links"
    );
    let reopened_legacy = boardstudio_core::mechanical::effective_inputs(
        &accepted.document,
        &accepted.scene.board_contours,
        &MechanicalTarget::Instance {
            instance_id: "right".into(),
        },
    )
    .expect("the legacy member still resolves")
    .configuration;
    assert_eq!(
        reopened_legacy.wall_thickness, legacy.wall_thickness,
        "the legacy effective settings are unchanged by opening the project"
    );
    assert_eq!(
        reopened_legacy.plate_thickness, legacy.plate_thickness,
        "the legacy shared construction still overlays the same values"
    );
}

/// Disabling a half that belongs to a pair removes the link and disables only that half.
///
/// The user's decision is "unlink and disable this half": one typed `Disable` carries it,
/// the other half keeps its geometry and settings, no physical instance or PCB is removed,
/// one Undo restores the link and the stack, a queued edit on the disabled stack retires,
/// and Enable stays eligible without inferring a relink.
#[test]
fn disabling_a_paired_half_unlinks_it_and_keeps_the_survivor_whole() {
    let runtime = open_document(paired_document());
    let mut scope = runtime.scope().expect("the open document has a scope");
    scope.board_id = "board".into();
    scope.instance_id = Some("left".into());

    let submit = |change: MechanicalChange| {
        let mut edits = PendingEdits::default();
        edits.begin(
            &runtime,
            1u8,
            "mechanical",
            Some("mechanical settings".into()),
            mechanical_resolver(
                scope.clone(),
                change,
                mounting_hole(),
                Rc::new(Cell::new(false)),
                Box::new(|_| true),
            ),
        );
        edits.settle(true)
    };
    let accepted = || runtime.model().accepted.expect("accepted");
    let configuration = |instance_id: &str| {
        let accepted = accepted();
        boardstudio_core::mechanical::effective_inputs(
            &accepted.document,
            &accepted.scene.board_contours,
            &MechanicalTarget::Instance {
                instance_id: instance_id.into(),
            },
        )
        .map(|inputs| inputs.configuration)
        .ok()
    };
    let pair_count = || {
        accepted()
            .document
            .hardware
            .as_ref()
            .expect("hardware")
            .pairs
            .len()
    };
    let instance_count = || {
        accepted()
            .document
            .hardware
            .as_ref()
            .expect("hardware")
            .instances
            .len()
    };
    let landed = |results: &[PendingEditResult<u8>]| {
        results
            .iter()
            .any(|result| matches!(result, PendingEditResult::Landed { .. }))
    };

    let survivor_before = configuration("right").expect("the survivor resolves");
    let instances_before = instance_count();
    assert_eq!(pair_count(), 1);

    assert!(
        landed(&submit(MechanicalChange::Disable)),
        "the linked half disables in one accepted edit"
    );
    assert_eq!(pair_count(), 0, "the link is removed");
    assert_eq!(
        instance_count(),
        instances_before,
        "no physical instance or PCB is removed"
    );
    assert_eq!(
        without_reservations(configuration("right").expect("the survivor still resolves")),
        without_reservations(survivor_before.clone()),
        "the other half keeps its geometry and settings"
    );
    assert!(
        configuration("left").is_none(),
        "only this half's mechanical stack is disabled"
    );

    // A queued edit addressed to the disabled stack cannot land.
    let queued = submit(MechanicalChange::SetDimension {
        field: MechanicalDimension::WallThickness,
        value: 3.0,
    });
    assert!(
        queued.iter().any(|result| matches!(
            result,
            PendingEditResult::Retired { .. } | PendingEditResult::Failed { .. }
        )),
        "a queued edit on the disabled stack must not land, got {queued:?}"
    );

    // Undo restores the link and the stack; Redo re-applies the whole change.
    undo(&runtime);
    assert_eq!(pair_count(), 1, "one Undo restores the link");
    assert!(
        configuration("left").is_some(),
        "one Undo restores this half's mechanical stack"
    );
    redo(&runtime);
    assert_eq!(pair_count(), 0, "redo re-applies the whole change");

    // Enable is still eligible for the unpaired half and infers no relink.
    assert!(
        landed(&submit(MechanicalChange::Enable)),
        "the disabled half can be enabled again"
    );
    assert_eq!(
        pair_count(),
        0,
        "enabling the half does not infer a mechanical link"
    );
}

/// Removing one half of a pair materializes the survivor, clears the pair and is one step.
///
/// Core stages the survivor's accepted visible model locally before the pair goes, so the
/// other half keeps exactly the geometry and settings it showed, its construction becomes
/// local, and the boards, PCBs and parts stay untouched. A queued edit addressed to the
/// removed member cannot land, and one Undo restores the assembly and the link.
#[test]
fn removing_a_paired_member_materializes_the_survivor_and_undo_restores_everything() {
    let runtime = open_document(paired_document());
    let mut scope = runtime.scope().expect("the open document has a scope");
    scope.board_id = "board".into();
    scope.instance_id = Some("left".into());
    let submit = |change: MechanicalChange| {
        let mut edits = PendingEdits::default();
        edits.begin(
            &runtime,
            1u8,
            "mechanical",
            Some("mechanical settings".into()),
            mechanical_resolver(
                scope.clone(),
                change,
                mounting_hole(),
                Rc::new(Cell::new(false)),
                Box::new(|_| true),
            ),
        );
        edits.settle(true)
    };
    let accepted = || runtime.model().accepted.expect("accepted");
    let visible = |instance_id: &str| {
        let accepted = accepted();
        boardstudio_core::mechanical::effective_inputs(
            &accepted.document,
            &accepted.scene.board_contours,
            &MechanicalTarget::Instance {
                instance_id: instance_id.into(),
            },
        )
        .map(|inputs| inputs.configuration)
        .ok()
    };
    let hardware = || {
        accepted()
            .document
            .hardware
            .as_ref()
            .expect("hardware")
            .clone()
    };
    let landed = |results: &[PendingEditResult<u8>]| {
        results
            .iter()
            .any(|result| matches!(result, PendingEditResult::Landed { .. }))
    };
    let refused_as_gone = |results: &[PendingEditResult<u8>]| {
        results.iter().any(|result| match result {
            PendingEditResult::Retired { .. } => true,
            PendingEditResult::Failed { message, .. } => message.contains("no longer available"),
            PendingEditResult::Landed { .. } => false,
        })
    };

    let survivor_visible_before = visible("right").expect("the survivor resolves");
    let removed_stack_before = hardware()
        .instances
        .iter()
        .find(|instance| instance.id == "left")
        .expect("the removed half")
        .mechanical
        .clone();
    let instances_before = hardware().instances.len();
    let boards_before = accepted().document.boards.len();
    let parts_before = accepted().document.parts.len();
    assert_eq!(hardware().pairs.len(), 1);

    assert!(
        landed(&submit(MechanicalChange::RemovePhysicalInstance)),
        "the paired half is removed in one accepted edit"
    );

    let after = hardware();
    assert!(
        after.pairs.is_empty(),
        "the removal clears the pair instead of leaving it dangling"
    );
    assert_eq!(
        after.instances.len(),
        instances_before - 1,
        "only the removed member leaves the project"
    );
    assert!(
        after.instances.iter().all(|instance| instance.id != "left"),
        "the removed member is gone"
    );
    let survivor = after
        .instances
        .iter()
        .find(|instance| instance.id == "right")
        .expect("the survivor stays");
    assert_eq!(
        survivor.construction_ownership,
        boardstudio_core::model::MechanicalConstructionOwnership::Local,
        "the survivor owns its materialized construction"
    );
    assert_eq!(
        without_reservations(visible("right").expect("the survivor still resolves")),
        without_reservations(survivor_visible_before.clone()),
        "the survivor keeps the geometry and settings it showed while linked"
    );
    assert_eq!(
        accepted().document.boards.len(),
        boards_before,
        "no board or PCB is removed"
    );
    assert_eq!(
        accepted().document.parts.len(),
        parts_before,
        "no part is removed"
    );

    // A queued edit addressed to the removed member cannot land.
    let queued = submit(MechanicalChange::SetDimension {
        field: MechanicalDimension::WallThickness,
        value: 3.0,
    });
    assert!(
        refused_as_gone(&queued),
        "a queued edit to the removed member must not land, got {queued:?}"
    );

    // One Undo restores the assembly and the link; redo re-applies the removal.
    undo(&runtime);
    let restored = hardware();
    assert_eq!(restored.pairs.len(), 1, "one Undo restores the link");
    assert_eq!(
        restored.instances.len(),
        instances_before,
        "one Undo restores the removed assembly"
    );
    let restored_left = restored
        .instances
        .iter()
        .find(|instance| instance.id == "left")
        .expect("the removed half is back")
        .mechanical
        .clone()
        .expect("the removed half's stack is restored");
    assert_eq!(
        without_reservations(restored_left),
        without_reservations(removed_stack_before.expect("the removed half had a stack")),
        "the removed half's stack is restored unchanged"
    );
    redo(&runtime);
    assert!(
        hardware().pairs.is_empty(),
        "redo re-applies the whole removal"
    );
}

/// Rejoining a local exception can deliberately take that half's own geometry.
///
/// The selected half's placement becomes the pair's single authored value and the other
/// half re-derives from it, instead of the selected half being forced to accept the
/// counterpart's geometry.
#[test]
fn a_rejoin_can_take_the_selected_halves_own_local_geometry_as_the_source() {
    use boardstudio_core::model::{MechanicalFeatureState, Mount, MountKind, Vec2};
    let mut document = paired_document();
    {
        let hardware = document.hardware.as_mut().expect("hardware");
        let feature = &mut hardware.pairs[0].features[0];
        feature.members[0].state = MechanicalFeatureState::Linked;
        feature.members[1].state = MechanicalFeatureState::Local;
        let right = hardware
            .instances
            .iter_mut()
            .find(|instance| instance.id == "right")
            .expect("the right half");
        right
            .mechanical
            .as_mut()
            .expect("the right half's configuration")
            .mounts = vec![Mount {
            id: "mount-right".into(),
            at: Vec2 { x: 7.0, y: 1.0 },
            kind: MountKind::Hole,
            hole_diameter: 2.2,
            boss_diameter: None,
            height: None,
        }];
    }
    let runtime = open_document(document);
    let mut scope = runtime.scope().expect("the open document has a scope");
    scope.board_id = "board".into();
    scope.instance_id = Some("right".into());
    let mut edits = PendingEdits::default();
    edits.begin(
        &runtime,
        1u8,
        "mechanical",
        Some("mechanical settings".into()),
        mechanical_resolver(
            scope.clone(),
            MechanicalChange::RejoinFeatureMember {
                feature_id: "mount-right".into(),
                instance_id: "right".into(),
                source_instance_id: "right".into(),
            },
            mounting_hole(),
            Rc::new(Cell::new(false)),
            Box::new(|_| true),
        ),
    );
    let results = edits.settle(true);
    assert!(
        results
            .iter()
            .any(|result| matches!(result, PendingEditResult::Landed { .. })),
        "the own-source rejoin lands, got {results:?}"
    );

    let accepted = runtime.model().accepted.expect("accepted");
    let pair = &accepted.document.hardware.as_ref().expect("hardware").pairs[0];
    let feature = &pair.features[0];
    assert_eq!(
        feature.author, "right",
        "the selected half now authors the shared value"
    );
    match &feature.value {
        boardstudio_core::model::MechanicalFeatureValue::Mount { mount } => {
            assert_eq!(
                mount.at,
                Vec2 { x: 7.0, y: 1.0 },
                "the shared value is the selected half's own geometry"
            );
        }
        other => panic!("a mount feature keeps a mount value, got {other:?}"),
    }
    assert!(
        feature
            .members
            .iter()
            .all(|member| member.state == MechanicalFeatureState::Linked),
        "the exception is cleared on the rejoined half"
    );
    let left_visible = boardstudio_core::mechanical::effective_inputs(
        &accepted.document,
        &accepted.scene.board_contours,
        &MechanicalTarget::Instance {
            instance_id: "left".into(),
        },
    )
    .expect("the other half resolves")
    .configuration;
    assert_eq!(
        left_visible.mounts[0].at,
        Vec2 { x: -7.0, y: 1.0 },
        "the other half re-derives the reflection of the chosen source"
    );
}

/// Converting one legacy half's construction materializes exactly what it showed.
///
/// The half's own values become local (including the overlaid hardware, critical fits
/// and profiles), the other legacy halves keep their baseline, one typed edit carries the
/// whole change with one Undo, and a queued construction edit for a half that was not
/// converted still lands.
#[test]
fn converting_a_legacy_halves_construction_materializes_it_without_moving_anything() {
    use boardstudio_core::model::MechanicalConstructionOwnership;
    let mut document = paired_document();
    {
        let hardware = document.hardware.as_mut().expect("hardware");
        hardware.pairs.clear();
        hardware.instances.truncate(3);
        for instance in hardware.instances.iter_mut() {
            instance.construction_ownership = MechanicalConstructionOwnership::Legacy;
            instance.mechanical = None;
        }
    }
    let runtime = open_document(document);
    let scope_for = |instance_id: &str| {
        let mut scope = runtime.scope().expect("the open document has a scope");
        scope.board_id = "board".into();
        scope.instance_id = Some(instance_id.into());
        scope
    };
    let submit = |instance_id: &str, change: MechanicalChange| {
        let mut edits = PendingEdits::default();
        edits.begin(
            &runtime,
            1u8,
            "mechanical",
            Some("mechanical settings".into()),
            mechanical_resolver(
                scope_for(instance_id),
                change,
                mounting_hole(),
                Rc::new(Cell::new(false)),
                Box::new(|_| true),
            ),
        );
        edits.settle(true)
    };
    let accepted = || runtime.model().accepted.expect("accepted");
    let visible = |instance_id: &str| {
        let accepted = accepted();
        boardstudio_core::mechanical::effective_inputs(
            &accepted.document,
            &accepted.scene.board_contours,
            &MechanicalTarget::Instance {
                instance_id: instance_id.into(),
            },
        )
        .ok()
        .map(|inputs| without_reservations(inputs.configuration))
    };
    let ownership = |instance_id: &str| {
        accepted()
            .document
            .hardware
            .as_ref()
            .expect("hardware")
            .instances
            .iter()
            .find(|instance| instance.id == instance_id)
            .expect("the half")
            .construction_ownership
    };
    let landed = |results: &[PendingEditResult<u8>]| {
        results
            .iter()
            .any(|result| matches!(result, PendingEditResult::Landed { .. }))
    };
    let conversion = || MechanicalChange::MaterializeLegacyConstruction {
        instance_ids: vec!["left".into()],
    };

    let before_left = visible("left").expect("the half resolves through the overlay");
    let before_third = visible("third").expect("the third half resolves");
    let before_shared = accepted()
        .document
        .hardware
        .as_ref()
        .expect("hardware")
        .shared_construction
        .clone();
    assert_eq!(ownership("left"), MechanicalConstructionOwnership::Legacy);

    assert!(
        landed(&submit("left", conversion())),
        "one typed conversion converts the selected half"
    );
    assert_eq!(
        ownership("left"),
        MechanicalConstructionOwnership::Local,
        "the converted half owns its construction"
    );
    assert_eq!(
        visible("left"),
        Some(before_left.clone()),
        "the converted half shows exactly the values it showed before"
    );
    assert_eq!(
        visible("third"),
        Some(before_third.clone()),
        "a legacy half that was not converted keeps its baseline"
    );
    assert_eq!(
        accepted()
            .document
            .hardware
            .as_ref()
            .expect("hardware")
            .shared_construction,
        before_shared,
        "the legacy shared construction stays the baseline for the remaining halves"
    );

    // Queued construction edits keep working after the conversion. A converted half is
    // Local, so its own values are authoritative and read back.
    assert!(
        landed(&submit(
            "left",
            MechanicalChange::SetDimension {
                field: MechanicalDimension::WallThickness,
                value: 3.0,
            },
        )),
        "a converted half's construction edit is unaffected"
    );
    assert_eq!(
        visible("left")
            .expect("the converted half still resolves")
            .wall_thickness,
        3.0,
        "the converted half's own value is authoritative"
    );
    // A legacy half that was not converted stays legacy and keeps its baseline.
    assert!(
        landed(&submit(
            "third",
            MechanicalChange::SetDimension {
                field: MechanicalDimension::WallThickness,
                value: 4.0,
            },
        )),
        "an edit for a half that was not converted still lands"
    );
    assert_eq!(visible("third").unwrap().wall_thickness, 4.0);
    assert_eq!(
        ownership("third"),
        MechanicalConstructionOwnership::Legacy,
        "editing a legacy half's construction does not convert it"
    );
    assert_eq!(
        ownership("left"),
        MechanicalConstructionOwnership::Local,
        "another half's edit does not disturb the conversion"
    );

    // One Undo restores the ownership and the overlay state; redo re-applies it.
    // Three edits are accepted (the conversion, then the two construction edits), so
    // each Undo reverts exactly one of them.
    undo(&runtime);
    undo(&runtime);
    assert_eq!(
        ownership("left"),
        MechanicalConstructionOwnership::Local,
        "undoing the later edits leaves the conversion"
    );
    undo(&runtime);
    assert_eq!(
        ownership("left"),
        MechanicalConstructionOwnership::Legacy,
        "undoing past the conversion restores the legacy ownership"
    );
    assert_eq!(
        visible("left"),
        Some(before_left.clone()),
        "and restores the overlaid values exactly"
    );
    redo(&runtime);
    assert_eq!(
        ownership("left"),
        MechanicalConstructionOwnership::Local,
        "redo re-applies the conversion"
    );

    // An ineligible or missing target surfaces Core's error and mutates nothing.
    let before = accepted().document.hardware.clone();
    for ids in [vec!["left".to_owned()], vec!["ghost".to_owned()], vec![]] {
        let results = submit(
            "left",
            MechanicalChange::MaterializeLegacyConstruction { instance_ids: ids },
        );
        assert!(
            results.iter().any(|result| match result {
                PendingEditResult::Failed { .. } | PendingEditResult::Retired { .. } => true,
                PendingEditResult::Landed { .. } => false,
            }),
            "an ineligible conversion must not land, got {results:?}"
        );
        assert_eq!(
            accepted().document.hardware,
            before,
            "a rejected conversion mutates nothing"
        );
    }
}

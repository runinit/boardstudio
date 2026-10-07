//! The Layout actions outside the Component Inspector land through resolution: keyboard
//! nudges, the old position Inspector's commit, and constraint set/remove. Every test runs
//! against the real Session and Core with the first Core reply gated, so edits queue.
use super::layout_component_edits::{
    commit_position_resolver, constraint_resolver, nudge_resolver, remove_constraint_resolver,
};
use super::inspector::LayoutConstraintValues;
use crate::edit_ticket::{EditTicket, Settlement};
use crate::runtime::{Runtime, project_name_test_support as support};
use boardstudio_core::model::{Board, Part, Pose2, ProjectDoc, Side, Vec2};
use std::rc::Rc;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn part(id: &str, x: f64) -> Part {
    Part {
        keycap: None,
        outline: None,
        id: id.into(),
        definition_id: "switch:base".into(),
        reference: id.to_uppercase(),
        pose: Pose2 {
            at: Vec2 { x, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        properties: None,
        generator_parameters: None,
    }
}

fn document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("remainder-doc", "Remainder");
    document.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec!["a".into(), "b".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.definitions.push(
        serde_json::from_value(serde_json::json!({
            "id": "switch:base",
            "name": "MX switch",
            "kind": "switch",
            "courtyard": [{"x": -3.0, "y": -3.0}, {"x": 3.0, "y": -3.0}, {"x": 3.0, "y": 3.0}],
            "pads": []
        }))
        .unwrap(),
    );
    document.parts.push(part("a", 0.0));
    document.parts.push(part("b", 40.0));
    document
}

async fn open() -> Rc<Runtime> {
    let runtime = support::new_runtime();
    support::open_document(&runtime, document()).await;
    runtime
}

fn position(runtime: &Rc<Runtime>, id: &str) -> Option<Vec2> {
    runtime
        .model()
        .accepted?
        .document
        .parts
        .iter()
        .find(|part| part.id == id)
        .map(|part| part.pose.at)
}

async fn settle(runtime: &Rc<Runtime>, ticket: &EditTicket) {
    for _ in 0..100 {
        support::run_pending(runtime).await;
        if !ticket.is_pending() {
            return;
        }
        gloo_timers::future::TimeoutFuture::new(10).await;
    }
}

async fn undo(runtime: &Rc<Runtime>) {
    runtime.submit(boardstudio_application::Event::Undo {
        operation_id: runtime.operation(),
    });
    support::run_pending(runtime).await;
}

fn nudge(runtime: &Rc<Runtime>, delta: Vec2) -> EditTicket {
    EditTicket::begin(
        runtime,
        "layout-nudge",
        Some("move".into()),
        nudge_resolver(vec!["a".into()], "board".into(), delta),
    )
}

#[wasm_bindgen_test]
async fn three_nudges_queued_behind_a_gated_reply_move_the_part_three_steps_and_undo_restores_it() {
    let runtime = open().await;
    let step = Vec2 { x: 0.1, y: 0.0 };
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let first = nudge(&runtime, step);
    support::drive_pending(&runtime);
    entered.await.expect("the first nudge reached Core");
    let second = nudge(&runtime, step);
    let third = nudge(&runtime, step);
    support::drive_pending(&runtime);
    release.send(()).expect("release the held reply");
    settle(&runtime, &third).await;

    for ticket in [&first, &second, &third] {
        assert!(matches!(ticket.settlement(true), Settlement::Landed { .. }));
    }
    let moved = position(&runtime, "a").unwrap();
    assert!((moved.x - 0.3).abs() < 1e-9, "three steps, got {}", moved.x);
    for _ in 0..3 {
        undo(&runtime).await;
    }
    assert_eq!(position(&runtime, "a"), Some(Vec2 { x: 0.0, y: 0.0 }));
}

#[wasm_bindgen_test]
async fn a_nudge_retires_when_its_part_was_deleted_before_it_ran() {
    let runtime = open().await;
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let accepted = runtime.model().accepted.unwrap();
    let mut without_a = accepted.document.as_ref().clone();
    without_a.parts.retain(|part| part.id != "a");
    without_a.boards[0].part_ids.retain(|id| id != "a");
    runtime.submit(boardstudio_application::Event::Edit {
        operation_id: runtime.operation(),
        command: boardstudio_core::model::EditCommand {
            base_revision: accepted.document.revision,
            transaction_id: "delete-a".into(),
            phase: boardstudio_core::model::EditPhase::Commit,
            target_ids: vec!["a".into()],
            operation: boardstudio_core::model::EditOperation::ReplaceDocument {
                document: Box::new(without_a),
            },
        },
    });
    support::drive_pending(&runtime);
    entered.await.expect("the delete reached Core");
    let ticket = nudge(&runtime, Vec2 { x: 0.1, y: 0.0 });
    support::drive_pending(&runtime);
    release.send(()).expect("release the held reply");
    settle(&runtime, &ticket).await;
    assert!(matches!(
        ticket.settlement(true),
        Settlement::Failed { ref message } if message.contains("no longer exists")
    ));
}

#[wasm_bindgen_test]
async fn old_position_inspector_commits_queue_and_the_latest_value_wins() {
    let runtime = open().await;
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let x = EditTicket::begin(
        &runtime,
        "layout-old-position",
        Some("position".into()),
        commit_position_resolver("a".into(), Vec2 { x: 5.0, y: 0.0 }, "position-1".into()),
    );
    support::drive_pending(&runtime);
    entered.await.expect("the X commit reached Core");
    let y = EditTicket::begin(
        &runtime,
        "layout-old-position",
        Some("position".into()),
        commit_position_resolver("a".into(), Vec2 { x: 5.0, y: 7.0 }, "position-2".into()),
    );
    support::drive_pending(&runtime);
    release.send(()).expect("release the held reply");
    settle(&runtime, &y).await;
    assert!(matches!(x.settlement(true), Settlement::Landed { .. }));
    assert!(matches!(y.settlement(true), Settlement::Landed { .. }));
    assert_eq!(position(&runtime, "a"), Some(Vec2 { x: 5.0, y: 7.0 }));
    undo(&runtime).await;
    assert_eq!(position(&runtime, "a"), Some(Vec2 { x: 5.0, y: 0.0 }));
}

#[wasm_bindgen_test]
async fn constraint_set_then_remove_queued_back_to_back_both_land() {
    let runtime = open().await;
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let set = EditTicket::begin(
        &runtime,
        "layout-inspector-constraint",
        Some("layout".into()),
        constraint_resolver(
            "b".into(),
            "board".into(),
            "a".into(),
            LayoutConstraintValues::Offset {
                offset: Vec2 { x: 20.0, y: 0.0 },
                rotation: 0.0,
            },
            9,
        ),
    );
    support::drive_pending(&runtime);
    entered.await.expect("the constraint reached Core");
    let remove = EditTicket::begin(
        &runtime,
        "layout-inspector-remove-constraint",
        Some("layout".into()),
        remove_constraint_resolver("b".into(), "layout-component-constraint-9".into()),
    );
    support::drive_pending(&runtime);
    release.send(()).expect("release the held reply");
    settle(&runtime, &remove).await;
    assert!(matches!(set.settlement(true), Settlement::Landed { .. }));
    assert!(matches!(remove.settlement(true), Settlement::Landed { .. }));
    assert!(
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .constraints
            .is_empty(),
        "the queued removal applied on top of the queued set"
    );
}

#[wasm_bindgen_test]
async fn removing_a_constraint_whose_part_was_deleted_retires_with_a_reason() {
    let runtime = open().await;
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let accepted = runtime.model().accepted.unwrap();
    let mut without_b = accepted.document.as_ref().clone();
    without_b.parts.retain(|part| part.id != "b");
    without_b.boards[0].part_ids.retain(|id| id != "b");
    runtime.submit(boardstudio_application::Event::Edit {
        operation_id: runtime.operation(),
        command: boardstudio_core::model::EditCommand {
            base_revision: accepted.document.revision,
            transaction_id: "delete-b".into(),
            phase: boardstudio_core::model::EditPhase::Commit,
            target_ids: vec!["b".into()],
            operation: boardstudio_core::model::EditOperation::ReplaceDocument {
                document: Box::new(without_b),
            },
        },
    });
    support::drive_pending(&runtime);
    entered.await.expect("the delete reached Core");
    let ticket = EditTicket::begin(
        &runtime,
        "layout-inspector-remove-constraint",
        Some("layout".into()),
        remove_constraint_resolver("b".into(), "layout-component-constraint-9".into()),
    );
    support::drive_pending(&runtime);
    release.send(()).expect("release the held reply");
    settle(&runtime, &ticket).await;
    assert!(matches!(
        ticket.settlement(true),
        Settlement::Failed { ref message } if message.contains("no longer exists")
    ));
}

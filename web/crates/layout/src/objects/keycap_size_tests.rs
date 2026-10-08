//! Mounted browser regressions for key-size timer ownership and feedback attribution.
use super::super::{TreeContext, keycap_size::KeySizeControls};
use super::*;
use boardstudio_application::{
    Completion, Effect, Event as SessionEvent, OperationId, Scope, SelectionMode, Session,
    SessionEpoch, SnapshotToken,
};
use boardstudio_core::CoreEngine;
use boardstudio_core::model::Vec2;
use boardstudio_core::model::{Part, Pose2, Side};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::{Event, HtmlInputElement, KeyboardEvent};

wasm_bindgen_test_configure!(run_in_browser);

#[derive(Clone)]
struct Probe {
    projection: Rc<RefCell<KeySizeProjection>>,
    feedback: Rc<RefCell<Option<KeySizeFeedback>>>,
    live_owner: Rc<RefCell<KeySizeOwner>>,
    accepted: Rc<RefCell<Vec<KeySizeRequest>>>,
    version: Rc<RefCell<Option<Signal<u64>>>>,
}

fn host() -> Element {
    let probe = use_context::<Probe>();
    let version = use_signal(|| 0u64);
    let _ = version();
    *probe.version.borrow_mut() = Some(version);
    let request_sequence = use_signal(|| 0u64);
    let on_resize = use_callback({
        let accepted = probe.accepted.clone();
        let live_owner = probe.live_owner.clone();
        move |request: KeySizeRequest| {
            // This is the controller's current-owner admission check at the real control seam.
            if request.owner == *live_owner.borrow() {
                accepted.borrow_mut().push(request);
            }
        }
    });
    let mount = KeySizeMount {
        projection: Some(probe.projection.borrow().clone()),
        request_sequence,
        editable: true,
        feedback: probe.feedback.borrow().clone(),
        inspector_mounted: use_signal(|| true),
        on_bind_draft: use_callback(|_| {}),
        on_resize,
    };
    rsx! { div { id: "key-size-regression-root", KeySizeControls { mount } } }
}

fn owner(context: TreeContext, selected_ids: &[&str], generation: u64) -> KeySizeOwner {
    KeySizeOwner {
        editor_instance_id: 7,
        context_generation: generation,
        scope_generation: 3,
        scope: Scope {
            session_epoch: SessionEpoch(1),
            document_id: "key-size-owner-fixture".into(),
            board_id: "board".into(),
            instance_id: None,
        },
        context,
        selected_ids: selected_ids.iter().map(|id| (*id).into()).collect(),
    }
}

fn projection(owner: KeySizeOwner, ids: &[&str], sizes: &[f64]) -> KeySizeProjection {
    let items = sizes
        .iter()
        .enumerate()
        .map(|(index, width)| KeySizeItem {
            id: ids[index].into(),
            size: Vec2 { x: *width, y: 18.0 },
            pitch: Vec2 { x: 19.0, y: 19.0 },
            gap: Vec2 { x: 1.0, y: 1.0 },
        })
        .collect::<Vec<_>>();
    let units = Vec2 {
        x: ((items[0].size.x + items[0].gap.x) / items[0].pitch.x * 4.0).round() / 4.0,
        y: 1.0,
    };
    let mixed_x = items
        .iter()
        .skip(1)
        .any(|item| ((item.size.x + item.gap.x) / item.pitch.x * 4.0).round() / 4.0 != units.x);
    KeySizeProjection {
        owner,
        snapshot_token: SnapshotToken(12),
        revision: 4,
        items,
        units,
        mixed: mixed_x,
        mixed_x,
        mixed_y: false,
        overlap_references: Vec::new(),
    }
}

fn mounted(
    initial: KeySizeProjection,
    feedback: Option<KeySizeFeedback>,
) -> (Probe, web_sys::Element) {
    let owner = initial.owner.clone();
    let probe = Probe {
        projection: Rc::new(RefCell::new(initial)),
        feedback: Rc::new(RefCell::new(feedback)),
        live_owner: Rc::new(RefCell::new(owner)),
        accepted: Rc::default(),
        version: Rc::default(),
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(probe.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    (probe, root)
}

fn rerender(probe: &Probe) {
    let mut version = {
        let stored = probe.version.borrow();
        stored.expect("host renders a version signal")
    };
    version.set(1);
}

fn element(root: &web_sys::Element, selector: &str) -> web_sys::Element {
    root.query_selector(selector).unwrap().unwrap()
}

async fn rendered(ms: u32) {
    gloo_timers::future::TimeoutFuture::new(ms).await;
}

#[wasm_bindgen_test]
async fn delayed_keyboard_resize_cannot_be_retargeted_to_a_new_mixed_selection() {
    let first_owner = owner(
        TreeContext::Row {
            matrix_id: "m".into(),
            row: 0,
        },
        &["a", "b"],
        1,
    );
    let second_owner = owner(
        TreeContext::Row {
            matrix_id: "m".into(),
            row: 1,
        },
        &["c", "d"],
        2,
    );
    let (probe, root) = mounted(projection(first_owner, &["a", "b"], &[18.0, 18.0]), None);
    rendered(20).await;

    let width = element(&root, "input[aria-label='Key width']")
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    width.focus().unwrap();
    width.set_value("2");
    let input = Event::new("input").unwrap();
    input.init_event_with_bubbles("input", true);
    width.dispatch_event(&input).unwrap();
    rendered(20).await;
    assert_eq!(
        element(&root, "output").text_content().as_deref(),
        Some("2u × 1u")
    );
    let keyup = KeyboardEvent::new("keyup").unwrap();
    keyup.init_event_with_bubbles("keyup", true);
    width.dispatch_event(&keyup).unwrap();

    // Change owner before the control's 150 ms keyboard debounce expires.
    *probe.live_owner.borrow_mut() = second_owner.clone();
    *probe.projection.borrow_mut() = projection(second_owner, &["c", "d"], &[18.0, 37.0]);
    rerender(&probe);
    rendered(220).await;

    assert!(
        probe.accepted.borrow().is_empty(),
        "an A-owned delayed keyup must not turn into a B-owned edit after selection changes"
    );
    root.remove();
}

#[wasm_bindgen_test]
fn overlap_filter_tracks_authoritative_add_and_toggle_selection() {
    let parts = vec![
        Part {
            keycap: None,
            outline: None,
            id: "a".into(),
            definition_id: "switch".into(),
            reference: "A".into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        },
        Part {
            keycap: None,
            outline: None,
            id: "c".into(),
            definition_id: "switch".into(),
            reference: "C".into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        },
        Part {
            keycap: None,
            outline: None,
            id: "b".into(),
            definition_id: "switch".into(),
            reference: "B".into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        },
    ];
    let placements = vec![
        KeycapPlacement {
            id: "a".into(),
            matrix_id: "m1".into(),
            row: 0,
            column: 0,
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
            size: Vec2 { x: 18.0, y: 18.0 },
        },
        KeycapPlacement {
            id: "c".into(),
            matrix_id: "m2".into(),
            row: 0,
            column: 0,
            at: Vec2 { x: 10.0, y: 0.0 },
            rotation: 0.0,
            size: Vec2 { x: 18.0, y: 18.0 },
        },
        KeycapPlacement {
            id: "b".into(),
            matrix_id: "m3".into(),
            row: 0,
            column: 0,
            at: Vec2 { x: 100.0, y: 0.0 },
            rotation: 0.0,
            size: Vec2 { x: 18.0, y: 18.0 },
        },
    ];
    let labels = |selected: &[&str]| {
        let ids: Vec<String> = selected.iter().map(|id| (*id).to_owned()).collect();
        let selected = super::selected_keycap_ids(&ids, &placements);
        super::overlap_references(&parts, &placements, &selected)
    };

    // The last-hit context is B, but the selected set contains A+B after Add.
    assert!(labels(&["b"]).is_empty());
    assert_eq!(labels(&["a", "b"]), ["A", "C"]);
    // Toggling A off leaves B, so its unrelated context must clear the warning.
    assert!(labels(&["b"]).is_empty());
    // Selecting C after that still reports both members across matrix contexts.
    assert_eq!(labels(&["b", "c"]), ["A", "C"]);
}

#[wasm_bindgen_test]
fn accepted_projection_uses_session_add_toggle_selection_across_hit_contexts() {
    let mut document: boardstudio_core::model::ProjectDoc = serde_json::from_str(include_str!(
        "../../../../../core/tests/fixtures/reviung41-outline-original.json"
    ))
    .unwrap();
    let first_matrix_id = document.matrices[0].id.clone();
    let first_ids: Vec<_> = document.matrices[0]
        .part_ids
        .iter()
        .filter(|id| !id.ends_with("/diode"))
        .cloned()
        .collect();
    let other_id = document.matrices[1]
        .part_ids
        .iter()
        .find(|id| !id.ends_with("/diode"))
        .unwrap()
        .clone();
    let first_id = first_ids[0].clone();
    let context_id = first_ids[1].clone();
    for id in [&first_id, &other_id] {
        let part = document
            .parts
            .iter_mut()
            .find(|part| part.id == *id)
            .unwrap();
        part.keycap = Some(Vec2 {
            x: 1000.0,
            y: 1000.0,
        });
    }
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let effects = session.submit(SessionEvent::Open {
        operation_id: OperationId(900),
        document,
    });
    let (request_id, executor_epoch, request) = effects
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
        .unwrap();
    let reply = engine.handle(request);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(reply),
    });
    let save_attempt_id = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Persist {
                save_attempt_id, ..
            } => Some(*save_attempt_id),
            _ => None,
        })
        .unwrap();
    session.complete(Completion::Persist {
        save_attempt_id,
        result: boardstudio_application::SaveResult::Committed,
    });
    let scope = session.scope().unwrap_or_else(|| {
        panic!(
            "Session has no live scope after Open: lifecycle={:?}, error={:?}",
            session.read_model().lifecycle,
            session.read_model().last_error
        )
    });

    session.submit(SessionEvent::SelectParts {
        operation_id: OperationId(901),
        part_ids: vec![first_id.clone()],
        range_part_ids: Vec::new(),
        mode: SelectionMode::Replace,
    });
    session.submit(SessionEvent::SelectParts {
        operation_id: OperationId(902),
        part_ids: vec![context_id.clone()],
        range_part_ids: Vec::new(),
        mode: SelectionMode::Add,
    });
    let model = session.read_model().clone();
    assert_eq!(
        model.selected_part_ids,
        vec![first_id.clone(), context_id.clone()]
    );

    let accepted = model.accepted.as_ref().unwrap();
    let context_cell = accepted
        .scene
        .matrix_scenes
        .iter()
        .find(|scene| scene.matrix_id == first_matrix_id)
        .unwrap()
        .cells
        .iter()
        .find(|cell| cell.member_id.as_deref() == Some(context_id.as_str()))
        .unwrap_or_else(|| {
            panic!(
                "accepted scene has no cell for {context_id} in {first_matrix_id}; members={:?}",
                accepted
                    .scene
                    .matrix_scenes
                    .iter()
                    .find(|scene| scene.matrix_id == first_matrix_id)
                    .map(|scene| scene
                        .cells
                        .iter()
                        .map(|cell| cell.member_id.clone())
                        .collect::<Vec<_>>())
            )
        });
    let hit_context = ScopedTreeContext {
        scope: scope.clone(),
        context: TreeContext::Key {
            matrix_id: first_matrix_id,
            row: context_cell.row,
            column: context_cell.column,
        },
    };
    let project = |model: &boardstudio_application::ReadModel| {
        super::project_for_scope(
            model,
            Some(&scope),
            Some(&scope),
            Some(&hit_context),
            super::ProjectionContext {
                editor: 7,
                generation: 1,
                scope_generation: 1,
                workspace: "Layout",
            },
        )
        .unwrap()
        .0
    };

    let projection = project(&model);
    assert_eq!(
        projection
            .items
            .iter()
            .map(|item| item.id.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        [first_id.as_str(), context_id.as_str()].into()
    );
    let other_reference = accepted
        .document
        .parts
        .iter()
        .find(|part| part.id == other_id)
        .unwrap()
        .reference
        .clone();
    assert!(
        projection.overlap_references.contains(&other_reference),
        "the accepted selection includes {first_id}, even though the Inspector hit context is only {context_id}"
    );

    session.submit(SessionEvent::SelectParts {
        operation_id: OperationId(903),
        part_ids: vec![first_id.clone()],
        range_part_ids: Vec::new(),
        mode: SelectionMode::Toggle,
    });
    let toggled = session.read_model().clone();
    assert_eq!(toggled.selected_part_ids, vec![context_id.clone()]);
    let projection = project(&toggled);
    assert_eq!(projection.items.len(), 1);
    assert_eq!(projection.items[0].id, context_id);

    session.submit(SessionEvent::SelectParts {
        operation_id: OperationId(904),
        part_ids: Vec::new(),
        range_part_ids: Vec::new(),
        mode: SelectionMode::Replace,
    });
    let cleared = session.read_model().clone();
    assert!(
        super::project_for_scope(
            &cleared,
            Some(&scope),
            Some(&scope),
            Some(&hit_context),
            super::ProjectionContext {
                editor: 7,
                generation: 1,
                scope_generation: 1,
                workspace: "Layout",
            },
        )
        .is_none()
    );
}

#[wasm_bindgen_test]
async fn delayed_keyboard_resize_commits_for_its_unchanged_owner() {
    let first_owner = owner(
        TreeContext::Row {
            matrix_id: "m".into(),
            row: 0,
        },
        &["a"],
        1,
    );
    let (probe, root) = mounted(projection(first_owner.clone(), &["a"], &[18.0]), None);
    rendered(20).await;
    let width = element(&root, "input[aria-label='Key width']")
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    width.focus().unwrap();
    width.set_value("2");
    let input = Event::new("input").unwrap();
    input.init_event_with_bubbles("input", true);
    width.dispatch_event(&input).unwrap();
    rendered(20).await;
    let keyup = KeyboardEvent::new("keyup").unwrap();
    keyup.init_event_with_bubbles("keyup", true);
    width.dispatch_event(&keyup).unwrap();
    rendered(220).await;

    assert_eq!(
        probe.accepted.borrow().len(),
        1,
        "the ordinary same-owner debounced keyboard edit must still reach admission"
    );
    let accepted = probe.accepted.borrow()[0].clone();
    assert_eq!(accepted.owner, first_owner);
    assert_eq!(accepted.units.x, 2.0);
    root.remove();
}

#[wasm_bindgen_test]
async fn feedback_from_another_selection_is_not_shown_in_the_inspector() {
    let first_owner = owner(
        TreeContext::Row {
            matrix_id: "m".into(),
            row: 0,
        },
        &["a"],
        1,
    );
    let second_owner = owner(
        TreeContext::Row {
            matrix_id: "m".into(),
            row: 1,
        },
        &["b"],
        2,
    );
    let old_feedback = KeySizeFeedback {
        owner: first_owner.clone(),
        request_id: 0,
        field: KeySizeField::Width,
        message: Some("Old selection failed".into()),
    };
    let (probe, root) = mounted(projection(first_owner, &["a"], &[18.0]), Some(old_feedback));
    rendered(20).await;
    *probe.live_owner.borrow_mut() = second_owner.clone();
    *probe.projection.borrow_mut() = projection(second_owner.clone(), &["b"], &[18.0]);
    rerender(&probe);
    rendered(30).await;

    assert!(
        root.query_selector("[role='alert']").unwrap().is_none(),
        "failure feedback must remain attributed to its original owner after selection changes"
    );
    assert!(
        root.query_selector("[role='status']").unwrap().is_none(),
        "saved/pending feedback from another owner must not be shown either"
    );
    *probe.feedback.borrow_mut() = Some(KeySizeFeedback {
        owner: second_owner,
        request_id: 0,
        field: KeySizeField::Width,
        message: Some("Current selection failed".into()),
    });
    rerender(&probe);
    rendered(20).await;
    assert_eq!(
        element(&root, "[role='alert']").text_content().as_deref(),
        Some("Current selection failed"),
        "feedback from the displayed owner must remain visible"
    );
    root.remove();
}

#[wasm_bindgen_test]
fn mixed_size_draft_uses_react_visible_document_part_order() {
    let make_part = |id: &str, reference: &str| Part {
        keycap: None,
        outline: None,
        id: id.into(),
        definition_id: "switch".into(),
        reference: reference.into(),
        pose: boardstudio_core::model::Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        properties: None,
        generator_parameters: None,
    };
    let make_placement = |id: &str, width: f64, matrix_id: &str| KeycapPlacement {
        id: id.into(),
        matrix_id: matrix_id.into(),
        row: 0,
        column: 0,
        at: Vec2 { x: 0.0, y: 0.0 },
        rotation: 0.0,
        size: Vec2 { x: width, y: 18.0 },
    };
    let document_parts = vec![make_part("part-a", "A"), make_part("part-b", "B")];
    // Matrix traversal is deliberately opposite to the TypeScript visibleParts order.
    let mut accepted_placements = vec![
        make_placement("part-b", 18.0, "matrix-b"),
        make_placement("part-a", 37.0, "matrix-a"),
    ];

    super::order_placements_by_document_parts(&document_parts, &mut accepted_placements);

    let selected: BTreeSet<_> = ["part-a", "part-b"].into_iter().collect();
    let selected_items: Vec<_> = accepted_placements
        .iter()
        .filter(|placement| selected.contains(placement.id.as_str()))
        .collect();
    let first = selected_items
        .first()
        .expect("both selected caps are projected");
    let first_units = ((first.size.x + 1.0) / 19.0 * 4.0).round() / 4.0;
    assert_eq!(first.id, "part-a");
    assert_eq!(first_units, 2.0, "Wide/Tall drafts from React's first item");
    assert_eq!(selected_items.len(), 2);
}

#[wasm_bindgen_test]
async fn overlap_status_matches_react_selection_deduplication_and_truncation() {
    let references = [
        "Selected",
        "Duplicate",
        "Duplicate",
        "Ref 3",
        "Ref 4",
        "Ref 5",
        "Ref 6",
        "Ref 7",
        "Ref 8",
        "Ref 9",
        "Ref 10",
    ];
    let parts: Vec<_> = references
        .iter()
        .enumerate()
        .map(|(index, reference)| Part {
            keycap: None,
            outline: None,
            id: format!("part-{index}"),
            definition_id: "switch".into(),
            reference: (*reference).into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        })
        .collect();
    let mut placements: Vec<_> = parts
        .iter()
        .enumerate()
        .map(|(index, part)| KeycapPlacement {
            id: part.id.clone(),
            matrix_id: format!("matrix-{}", index % 3),
            row: 0,
            column: index as u32,
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
            size: Vec2 { x: 18.0, y: 18.0 },
        })
        .collect();
    // Matrix traversal interleaves ownership; document order is the stable React tie-breaker.
    placements.reverse();
    super::order_placements_by_document_parts(&parts, &mut placements);
    let selected = BTreeSet::from(["part-0".to_owned()]);
    let labels = super::overlap_references(&parts, &placements, &selected);
    assert_eq!(
        labels,
        [
            "Selected",
            "Duplicate",
            "Ref 3",
            "Ref 4",
            "Ref 5",
            "Ref 6",
            "Ref 7",
            "Ref 8",
            "Ref 9",
            "Ref 10",
        ]
    );

    let current_owner = owner(
        TreeContext::Key {
            matrix_id: "matrix-0".into(),
            row: 0,
            column: 0,
        },
        &["part-0"],
        1,
    );
    let mut initial = projection(current_owner, &["part-0"], &[18.0]);
    initial.overlap_references = labels;
    let (_probe, root) = mounted(initial, None);
    rendered(20).await;
    assert_eq!(
        element(&root, ".m1-key-size-warning[role='status']")
            .text_content()
            .as_deref(),
        Some(
            "Keycaps overlap: Selected, Duplicate, Ref 3, Ref 4, Ref 5, Ref 6, Ref 7, Ref 8 and 2 more. Adjust their rows or columns to clear the overlap."
        )
    );
    root.remove();
}

const KEY_ID: &str = "matrix/matrix-main/r0c0";

fn key_fixture() -> boardstudio_core::model::ProjectDoc {
    use boardstudio_core::model::{Board, OutlineFeature, OutlineSettings};
    let mut document = boardstudio_core::model::ProjectDoc::empty("project", "Keyboard");
    document.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        settings: OutlineSettings::default(),
        id: "envelope-main".into(),
        part_ids: vec![],
        margin: 4.0,
        operation: boardstudio_core::model::Operation::Add,
    });
    document.boards.push(Board {
        id: "board-main".into(),
        name: "Main".into(),
        outline_ids: vec!["envelope-main".into()],
        part_ids: vec![KEY_ID.into()],
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
    document.parts.push(Part {
        keycap: None,
        outline: None,
        id: KEY_ID.into(),
        definition_id: "switch:base".into(),
        reference: "SW1".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        properties: None,
        generator_parameters: None,
    });
    document.matrices.push(
        serde_json::from_value(serde_json::json!({
            "id": "matrix-main",
            "rows": 1,
            "columns": 1,
            "pitch": {"x": 19.05, "y": 19.05},
            "origin": {"x": 0.0, "y": 0.0},
            "definitionId": "switch:base",
            "partIds": [KEY_ID],
            "boardId": "board-main",
            "cells": [{
                "row": 0, "column": 0, "enabled": true,
                "definitionId": "switch:base",
                "assemblies": []
            }]
        }))
        .unwrap(),
    );
    document
}

fn key_width(runtime: &Rc<crate::runtime::Runtime>) -> Option<Vec2> {
    runtime
        .model()
        .accepted
        .as_ref()?
        .document
        .parts
        .iter()
        .find(|part| part.id == KEY_ID)?
        .keycap
}

fn board_name(runtime: &Rc<crate::runtime::Runtime>) -> String {
    runtime.model().accepted.as_ref().unwrap().document.boards[0]
        .name
        .clone()
}

async fn undo(runtime: &Rc<crate::runtime::Runtime>) {
    runtime.submit(SessionEvent::Undo {
        operation_id: runtime.operation(),
    });
    crate::runtime::project_name_test_support::run_pending(runtime).await;
}

#[wasm_bindgen_test]
async fn rapid_key_size_then_an_unrelated_edit_both_survive_and_undo_removes_them_in_order() {
    use crate::runtime::project_name_test_support as support;
    use boardstudio_core::model::EditOperation;
    use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};

    let runtime = support::new_runtime();
    support::open_document(&runtime, key_fixture()).await;
    let accepted = runtime.model().accepted.expect("the fixture opens");
    assert!(
        !placements(
            &accepted.document,
            &accepted.scene.matrix_scenes,
            "board-main"
        )
        .is_empty(),
        "Core projects the fixture key into the matrix scene: scenes={:?} parts={:?} matrices={:?}",
        accepted.scene.matrix_scenes,
        accepted
            .document
            .parts
            .iter()
            .map(|p| &p.id)
            .collect::<Vec<_>>(),
        accepted
            .document
            .matrices
            .iter()
            .map(|m| (&m.id, &m.part_ids, &m.board_id))
            .collect::<Vec<_>>(),
    );
    let original_width = key_width(&runtime);

    // Hold the first Core reply so the second edit queues behind the key-size edit.
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let resize = EditTicket::begin(
        &runtime,
        "layout-key-size",
        Some("key size".into()),
        resize_resolver(
            "board-main".into(),
            vec![KEY_ID.into()],
            Vec2 { x: 2.0, y: 1.0 },
            Some(ResizeAxis::X),
        ),
    );
    support::drive_pending(&runtime);
    entered.await.expect("the key-size edit reached Core");
    assert!(resize.is_pending(), "the held edit has not settled");

    let rename = EditTicket::begin(
        &runtime,
        "board-rename",
        Some("board".into()),
        boardstudio_application::EditResolver::new(
            "board-rename",
            |accepted: &boardstudio_application::AcceptedSnapshot| {
                let mut replacement = accepted.document.as_ref().clone();
                replacement.boards[0].name = "Renamed".into();
                boardstudio_application::Resolution::submit(
                    vec!["board-main".into()],
                    EditOperation::ReplaceDocument {
                        document: Box::new(replacement),
                    },
                )
            },
        ),
    );
    support::drive_pending(&runtime);
    release.send(()).expect("release the held key-size reply");
    support::run_pending(&runtime).await;
    rendered(20).await;

    assert!(matches!(resize.settlement(true), Settlement::Landed { .. }));
    assert!(matches!(rename.settlement(true), Settlement::Landed { .. }));
    assert_ne!(
        key_width(&runtime),
        original_width,
        "the key-size edit landed"
    );
    assert_eq!(board_name(&runtime), "Renamed", "the queued edit survived");

    undo(&runtime).await;
    assert_eq!(
        board_name(&runtime),
        "Main",
        "one Undo removes the later edit"
    );
    assert_ne!(
        key_width(&runtime),
        original_width,
        "the key size is still applied"
    );
    undo(&runtime).await;
    assert_eq!(
        key_width(&runtime),
        original_width,
        "the second Undo removes the key size"
    );
}

#[derive(Clone)]
struct RuntimeProbe {
    runtime: Rc<Runtime>,
    selected: ScopedTreeContext,
}

fn runtime_host() -> Element {
    let probe = use_context::<RuntimeProbe>();
    let version = use_signal(|| 0u64);
    use_hook({
        let runtime = probe.runtime.clone();
        move || {
            runtime.subscribe(Rc::new(move || {
                let mut version = version;
                version += 1;
            }));
        }
    });
    use_drop({
        let runtime = probe.runtime.clone();
        move || runtime.unsubscribe()
    });
    let _ = version();
    let selected = use_signal(|| Some(probe.selected.clone()));
    let workspace = use_signal(|| "Layout");
    let generation = use_signal(|| 1u64);
    let mount = use_key_size(
        probe.runtime.clone(),
        version,
        selected,
        workspace,
        generation,
    );
    rsx! { KeySizeControls { mount } }
}

fn enter_width(input: &HtmlInputElement, value: &str) {
    input.set_value(value);
    let event = Event::new("input").unwrap();
    event.init_event_with_bubbles("input", true);
    input.dispatch_event(&event).unwrap();
}

fn commit_width(input: &HtmlInputElement) {
    let event = web_sys::PointerEvent::new("pointerup").unwrap();
    event.init_event_with_bubbles("pointerup", true);
    input.dispatch_event(&event).unwrap();
}

#[wasm_bindgen_test]
async fn key_size_settlement_preserves_newer_drafts_restores_failures_and_tracks_undo_redo() {
    use crate::runtime::project_name_test_support as support;

    let runtime = support::new_runtime();
    support::open_document(&runtime, key_fixture()).await;
    runtime.submit(SessionEvent::SelectParts {
        operation_id: runtime.operation(),
        part_ids: vec![KEY_ID.into()],
        range_part_ids: Vec::new(),
        mode: SelectionMode::Replace,
    });
    support::run_pending(&runtime).await;
    let original_width = key_width(&runtime);
    let probe = RuntimeProbe {
        selected: ScopedTreeContext {
            scope: runtime.scope().unwrap(),
            context: TreeContext::Key {
                matrix_id: "matrix-main".into(),
                row: 0,
                column: 0,
            },
        },
        runtime,
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(runtime_host);
    dom.provide_root_context(probe.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    rendered(30).await;
    let width = element(&root, "input[aria-label='Key width']")
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    assert_eq!(width.value(), "1");

    enter_width(&width, "2");
    rendered(20).await;
    let (entered, release) = support::gate_next_core_reply(&probe.runtime);
    commit_width(&width);
    rendered(20).await;
    assert_eq!(
        probe.runtime.model().lifecycle,
        Lifecycle::Applying,
        "the real pointer release admits a pending resize"
    );
    support::drive_pending(&probe.runtime);
    entered.await.expect("the UI resize reached Core");
    enter_width(&width, "3");
    rendered(20).await;
    release.send(()).expect("release the held UI resize");
    for _ in 0..20 {
        support::run_pending(&probe.runtime).await;
        if probe.runtime.model().lifecycle == Lifecycle::Ready {
            break;
        }
        rendered(10).await;
    }
    rendered(30).await;
    assert!(
        key_width(&probe.runtime).is_some_and(|size| (size.x - 37.1).abs() < 1e-9),
        "the UI committed accepted 2u before testing newer drafts and Undo"
    );
    let width = element(&root, "input[aria-label='Key width']")
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    assert_eq!(
        width.value(),
        "3",
        "an older landed resize must leave the newer slider draft visible"
    );
    enter_width(&width, "2");
    rendered(20).await;
    undo(&probe.runtime).await;
    rendered(30).await;
    assert_eq!(
        key_width(&probe.runtime),
        original_width,
        "Runtime accepted Undo before the clean Inspector updates"
    );
    let width = element(&root, "input[aria-label='Key width']")
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    assert_eq!(width.value(), "1", "a clean field follows Undo");
    probe.runtime.submit(SessionEvent::Redo {
        operation_id: probe.runtime.operation(),
    });
    support::run_pending(&probe.runtime).await;
    rendered(30).await;
    let width = element(&root, "input[aria-label='Key width']")
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    assert_eq!(width.value(), "2", "a clean field follows Redo");
    let accepted_revision = probe.runtime.model().accepted.unwrap().document.revision;

    support::fail_next_core_reply(&probe.runtime, "injected key-size failure");
    enter_width(&width, "3");
    rendered(20).await;
    commit_width(&width);
    rendered(20).await;
    support::run_pending(&probe.runtime).await;
    rendered(30).await;
    assert_eq!(
        probe.runtime.model().accepted.unwrap().document.revision,
        accepted_revision
    );
    let width = element(&root, "input[aria-label='Key width']")
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    assert_eq!(
        width.value(),
        "2",
        "a failed unchanged draft restores the accepted width"
    );
    assert!(
        element(&root, "[role='alert']")
            .text_content()
            .unwrap()
            .contains("injected key-size failure")
    );

    probe.runtime.unsubscribe();
    root.remove();
}

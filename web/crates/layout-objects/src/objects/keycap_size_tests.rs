//! Mounted browser regressions for key-size timer ownership and feedback attribution.
use super::super::{TreeContext, keycap_size::KeySizeControls};
use super::*;
use boardstudio_application::{
    Completion, Effect, Event as SessionEvent, OperationId, Scope, SelectionMode, Session,
    SessionEpoch, SnapshotToken,
};
use boardstudio_core::CoreEngine;
use boardstudio_model::model::Vec2;
use boardstudio_model::model::{Part, Pose2, Side};
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

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
        on_resize,
    };
    rsx! {
        div { id: "key-size-regression-root",
            KeySizeControls { mount }
        }
    }
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
) -> (Probe, web_sys::Element, MountedFixture) {
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
    let mounted = MountedFixture::new(root.clone(), dom);
    (probe, root, mounted)
}

#[wasm_bindgen_test]
async fn key_size_preview_keeps_its_geometry_without_legacy_classes() {
    let document = web_sys::window().unwrap().document().unwrap();
    let stylesheet = document.create_element("style").unwrap();
    stylesheet.set_text_content(Some(&format!(
        "{}\n{}",
        include_str!("../../../../assets/workbench-base.css"),
        include_str!("../../../../assets/tailwind.css")
    )));
    document.head().unwrap().append_child(&stylesheet).unwrap();
    let key_owner = owner(
        TreeContext::Row {
            matrix_id: "m".into(),
            row: 0,
        },
        &["a"],
        1,
    );
    let (_, root, mut mounted) = mounted(projection(key_owner, &["a"], &[18.0]), None);
    mounted.own_node(stylesheet.clone());
    wait_until("key size mounts", || {
        root.query_selector("section[aria-label='Key size']")
            .unwrap()
            .is_some()
    })
    .await;
    let section = root
        .query_selector("section[aria-label='Key size']")
        .unwrap()
        .unwrap();
    let descendants = section.query_selector_all("[class]").unwrap();
    for node in (0..descendants.length()).filter_map(|i| descendants.item(i)) {
        let element = node.dyn_into::<web_sys::Element>().unwrap();
        let classes = element.get_attribute("class").unwrap_or_default();
        element
            .set_attribute(
                "class",
                &classes
                    .split_whitespace()
                    .filter(|class| !class.starts_with("m1-"))
                    .collect::<Vec<_>>()
                    .join(" "),
            )
            .unwrap();
    }
    let classes = section.get_attribute("class").unwrap_or_default();
    section
        .set_attribute(
            "class",
            &classes
                .split_whitespace()
                .filter(|class| !class.starts_with("m1-"))
                .collect::<Vec<_>>()
                .join(" "),
        )
        .unwrap();
    let svg = section.query_selector("[role=img]").unwrap().unwrap();
    let window = web_sys::window().unwrap();
    assert_eq!(
        window
            .get_computed_style(&section)
            .unwrap()
            .unwrap()
            .get_property_value("display")
            .unwrap(),
        "grid"
    );
    assert_eq!(
        window
            .get_computed_style(&svg)
            .unwrap()
            .unwrap()
            .get_property_value("height")
            .unwrap(),
        "100px"
    );
    stylesheet.remove();
    root.remove();
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
async fn size_selection_is_submitted_once_for_its_owner_before_selection_changes() {
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
    let (probe, root, _mounted) = mounted(
        projection(first_owner.clone(), &["a", "b"], &[18.0, 18.0]),
        None,
    );
    wait_until("the first owner's size control mounts", || {
        by_role_name(&root, "combobox", "Key size").is_some()
    })
    .await;
    change_size(&root, "2").await;
    wait_until("the first owner's size is submitted", || {
        !probe.accepted.borrow().is_empty()
    })
    .await;
    assert_eq!(probe.accepted.borrow().len(), 1);
    assert_eq!(probe.accepted.borrow()[0].owner, first_owner);
    *probe.live_owner.borrow_mut() = second_owner.clone();
    *probe.projection.borrow_mut() = projection(second_owner, &["c", "d"], &[18.0, 37.0]);
    rerender(&probe);
    wait_until("the new owner's mixed-size preview renders", || {
        root.query_selector("[role=img][aria-label='Keycap size Mixed']")
            .unwrap()
            .is_some()
    })
    .await;
    // Observe the new owner for the original interval to catch a late stale submission.
    rendered(220).await;
    assert_eq!(
        probe.accepted.borrow().len(),
        1,
        "a selection change must not resubmit the previous size for a new owner"
    );
    root.remove();
}

#[wasm_bindgen_test]
fn accepted_projection_uses_session_add_toggle_selection_across_hit_contexts() {
    let mut document: boardstudio_model::model::ProjectDoc = serde_json::from_str(include_str!(
        "../../../../../core/tests/fixtures/synthetic-outline.json"
    ))
    .unwrap();
    document.format_version = boardstudio_model::migrate::CURRENT_VERSION;
    let first_matrix_id = document.matrices[0].id.clone();
    let overlap_id = "matrix/matrix/r0c2".to_owned();
    let mut overlap_part = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/matrix/r0c1")
        .expect("the synthetic matrix has an authored key to clone")
        .clone();
    overlap_part.id = overlap_id.clone();
    overlap_part.reference = "M3".into();
    overlap_part.keycap = Some(Vec2 {
        x: 1000.0,
        y: 1000.0,
    });
    document.parts.push(overlap_part);
    document.matrices[0].part_ids.push(overlap_id.clone());
    document.matrices[0].columns = 3;
    document.boards[0].part_ids.push(overlap_id.clone());
    let overlap_definition = document.matrices[0].definition_id.clone();
    document.matrices[0]
        .cells
        .push(boardstudio_model::model::MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            row: 0,
            column: 2,
            enabled: true,
            definition_id: Some(overlap_definition),
            variant: None,
            offset: None,
            rotation: None,
            assemblies: Vec::new(),
            assemblies_local: Some(true),
        });
    let first_ids: Vec<_> = document.matrices[0]
        .part_ids
        .iter()
        .filter(|id| !id.ends_with("/diode"))
        .cloned()
        .collect();
    let other_id = overlap_id;
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
    let mut session = Session::new(CoreEngine::validate_open);
    let effects = session.submit(SessionEvent::Open {
        operation_id: OperationId(900),
        document,
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
async fn size_selector_and_rotate_checkbox_submit_through_the_same_owner() {
    let first_owner = owner(
        TreeContext::Row {
            matrix_id: "m".into(),
            row: 0,
        },
        &["a"],
        1,
    );
    let (probe, root, _mounted) = mounted(projection(first_owner.clone(), &["a"], &[18.0]), None);
    rendered(20).await;
    change_size(&root, "2").await;
    rendered(20).await;
    by_role_name(&root, "checkbox", "Rotate keycap")
        .unwrap()
        .click();
    rendered(20).await;
    assert_eq!(probe.accepted.borrow().len(), 2);
    let request = probe.accepted.borrow()[1].clone();
    assert_eq!(request.owner, first_owner);
    assert_eq!(request.units, Vec2 { x: 1.0, y: 2.0 });
    assert!(root.query_selector("[role=slider]").unwrap().is_none());
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
    let (probe, root, _mounted) =
        mounted(projection(first_owner, &["a"], &[18.0]), Some(old_feedback));
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
            anchor: None,
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
        .map(
            |(index, part)| super::super::keycap_resize::KeycapPlacement {
                id: part.id.clone(),
                matrix_id: format!("matrix-{}", index % 3),
                row: 0,
                column: index as u32,
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
                size: Vec2 { x: 18.0, y: 18.0 },
            },
        )
        .collect();
    // Matrix traversal interleaves ownership; document order is the stable React tie-breaker.
    placements.reverse();
    super::super::keycap_size_logic::order_placements_by_document_parts(&parts, &mut placements);
    let selected = std::collections::BTreeSet::from(["part-0".to_owned()]);
    let labels =
        super::super::keycap_size_logic::overlap_references(&parts, &placements, &selected);
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
    let (_probe, root, _mounted) = mounted(initial, None);
    rendered(20).await;
    assert!(
        element(&root, "[role='status']")
            .query_selector("[data-slot=callout]")
            .unwrap()
            .is_some()
    );
    assert_eq!(
        element(&root, "[role='status']").text_content().as_deref(),
        Some(
            "Keycaps overlap: Selected, Duplicate, Ref 3, Ref 4, Ref 5, Ref 6, Ref 7, Ref 8 and 2 more. Adjust their rows or columns to clear the overlap."
        )
    );
    root.remove();
}

async fn change_size(root: &web_sys::Element, value: &str) {
    let input = by_role_name(root, "combobox", "Key size").unwrap();
    input.focus().unwrap();
    let init = web_sys::KeyboardEventInit::new();
    init.set_key("ArrowDown");
    init.set_bubbles(true);
    input
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap(),
        )
        .unwrap();
    let name = format!("{value}u");
    wait_until("key-size option opens", || {
        by_role_name(root, "option", &name).is_some()
    })
    .await;
    let option = by_role_name(root, "option", &name).unwrap();
    // The primitive selects on a complete pointer gesture, not a synthetic click.
    for phase in ["pointerdown", "pointerup"] {
        let pointer = web_sys::PointerEvent::new(phase).unwrap();
        pointer.init_event_with_bubbles_and_cancelable(phase, true, true);
        option.dispatch_event(&pointer).unwrap();
    }
    wait_until("key-size choice closes", || {
        input.get_attribute("aria-expanded").as_deref() == Some("false")
    })
    .await;
}

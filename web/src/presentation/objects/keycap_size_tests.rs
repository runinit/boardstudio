//! Mounted browser regressions for key-size timer ownership and feedback attribution.
use super::super::{TreeContext, keycap_size::KeySizeControls};
use super::*;
use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
use boardstudio_core::model::Vec2;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::{Event, HtmlInputElement, KeyboardEvent};
use boardstudio_core::model::{Part, Side};

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
        busy: false,
        feedback: probe.feedback.borrow().clone(),
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
        state: KeySizeState::Failed,
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
        state: KeySizeState::Failed,
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
    let make_part = |id: &str| Part {
        keycap: None,
        outline: None,
        id: id.into(),
        definition_id: "switch".into(),
        reference: id.into(),
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
    let document_parts = vec![make_part("part-a"), make_part("part-b")];
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
    let first = selected_items.first().expect("both selected caps are projected");
    let first_units = ((first.size.x + 1.0) / 19.0 * 4.0).round() / 4.0;
    assert_eq!(first.id, "part-a");
    assert_eq!(first_units, 2.0, "Wide/Tall drafts from React's first item");
    assert_eq!(selected_items.len(), 2);
}

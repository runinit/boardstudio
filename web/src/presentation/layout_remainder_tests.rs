//! The Layout actions outside the Component Inspector land through resolution: keyboard
//! nudges, the old position Inspector's commit, and constraint set/remove. Every test runs
//! against the real Session and Core with the first Core reply gated, so edits queue.
use super::inspector::LayoutConstraintValues;
use super::layout_component_edits::{
    constraint_resolver, offset_axes_resolver, remove_constraint_resolver,
};
use crate::operation_outcomes::{EditOutcome, Settlement};
use crate::runtime::Runtime;
use boardstudio_core::model::{
    Board, Constraint, ConstraintEndpoint, ConstraintRelation, OffsetAxes, Part, Pose2, ProjectDoc,
    Side, Vec2,
};
use boardstudio_web_test_runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::test_wait::{wait_until, wait_until_step};
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
        anchor: None,
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

async fn settle(runtime: &Rc<Runtime>, _ticket: &EditOutcome) {
    support::run_pending(runtime).await;
}

async fn undo(runtime: &Rc<Runtime>) {
    runtime.submit(boardstudio_application::Event::Undo {
        operation_id: runtime.operation(),
    });
    support::run_pending(runtime).await;
}

#[wasm_bindgen_test]
async fn typed_constraint_update_preserves_id_and_endpoints_for_every_relation_then_removes() {
    let runtime = open().await;
    let values = [
        ConstraintRelation::Offset {
            offset: Vec2 { x: 5.0, y: -2.0 },
            rotation: 17.0,
        },
        ConstraintRelation::PartOffset {
            offset: Vec2 { x: -3.0, y: 4.0 },
            rotation: -11.0,
        },
        ConstraintRelation::MirrorLine {
            at: Vec2 { x: 12.0, y: -9.0 },
            angle: 31.0,
        },
    ];

    for (index, relation) in values.into_iter().enumerate() {
        let ticket = runtime.resolve_edit(
            "layout-inspector-constraint",
            Some("layout".into()),
            constraint_resolver(
                "b".into(),
                "board".into(),
                "a".into(),
                LayoutConstraintValues::Framed {
                    relation: relation.clone(),
                },
                if index == 0 { 9 } else { 77 },
            ),
        );
        support::drive_pending(&runtime);
        settle(&runtime, &ticket).await;
        assert!(matches!(ticket.settlement(true), Settlement::Landed { .. }));

        let accepted = runtime.model().accepted.unwrap();
        assert!(
            matches!(
                accepted.document.constraints.as_slice(),
                [Constraint::Framed { id, source: ConstraintEndpoint::Part { part_id: source }, target: ConstraintEndpoint::Part { part_id: target }, relation: saved }]
                    if id == "layout-component-constraint-9"
                        && source == "a"
                        && target == "b"
                        && saved == &relation
            ),
            "typed update preserves the durable id, endpoint frames and exact relation: {:?}",
            accepted.document.constraints
        );
    }

    let remove = runtime.resolve_edit(
        "layout-inspector-remove-constraint",
        Some("layout".into()),
        remove_constraint_resolver("b".into(), "layout-component-constraint-9".into()),
    );
    support::drive_pending(&runtime);
    settle(&runtime, &remove).await;
    assert!(matches!(remove.settlement(true), Settlement::Landed { .. }));
    assert!(
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .constraints
            .is_empty(),
        "the same stable record can be removed through the facade-backed resolver"
    );
}

fn old_inspector_host() -> dioxus::prelude::Element {
    use dioxus::prelude::*;
    let runtime = use_context::<Rc<Runtime>>();
    let version = use_signal(|| 0_u64);
    use_context_provider(|| version);
    use_hook(move || {
        runtime.subscribe(Rc::new(move || {
            let mut version = version;
            version += 1;
        }))
    });
    rsx! { super::inspector::Inspector {} }
}

#[wasm_bindgen_test]
async fn mounted_apply_position_commits_both_axes_as_one_undo_entry() {
    use wasm_bindgen::JsCast;
    let runtime = open().await;
    runtime.submit(boardstudio_application::Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: vec!["b".into(), "a".into()],
        range_part_ids: vec![],
        mode: boardstudio_application::SelectionMode::Replace,
    });
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = dioxus::prelude::VirtualDom::new(old_inspector_host);
    dom.provide_root_context(runtime.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    use boardstudio_web_ui_shared::test_role::by_role_name;
    wait_until("the old position inputs mount", || {
        by_role_name(&root, "spinbutton", "X (mm)").is_some()
    })
    .await;
    let input = |name: &str| {
        by_role_name(&root, "spinbutton", name)
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap()
    };
    let bubbling = web_sys::EventInit::new();
    bubbling.set_bubbles(true);
    for (name, value) in [("X (mm)", "45"), ("Y (mm)", "7")] {
        let field = input(name);
        field.set_value(value);
        field
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
            .unwrap();
    }
    by_role_name(&root, "button", "Apply position")
        .unwrap()
        .click();
    wait_until_step("Apply commits both position axes", async || {
        support::run_pending(&runtime).await;
        position(&runtime, "b") == Some(Vec2 { x: 45.0, y: 7.0 })
            && position(&runtime, "a") == Some(Vec2 { x: 5.0, y: 7.0 })
    })
    .await;
    assert_eq!(position(&runtime, "b"), Some(Vec2 { x: 45.0, y: 7.0 }));
    assert_eq!(position(&runtime, "a"), Some(Vec2 { x: 5.0, y: 7.0 }));
    undo(&runtime).await;
    assert_eq!(position(&runtime, "b"), Some(Vec2 { x: 40.0, y: 0.0 }));
    assert_eq!(position(&runtime, "a"), Some(Vec2 { x: 0.0, y: 0.0 }));
    runtime.unsubscribe();
    root.remove();
}

#[wasm_bindgen_test]
async fn converting_a_constraint_between_axes_leaves_the_target_where_it_was() {
    let runtime = open().await;
    let set = runtime.resolve_edit(
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
    settle(&runtime, &set).await;
    assert!(matches!(set.settlement(true), Settlement::Landed { .. }));
    let target_before = position(&runtime, "b");
    let convert = runtime.resolve_edit(
        "layout-inspector-offset-axes",
        Some("layout".into()),
        offset_axes_resolver(
            "b".into(),
            "layout-component-constraint-9".into(),
            OffsetAxes::Part,
        ),
    );
    support::drive_pending(&runtime);
    settle(&runtime, &convert).await;
    assert!(matches!(
        convert.settlement(true),
        Settlement::Landed { .. }
    ));
    assert_eq!(position(&runtime, "b"), target_before);
    let document = runtime.model().accepted.unwrap().document;
    assert!(
        matches!(
            document.constraints.as_slice(),
            [Constraint::PartOffset { id, .. }] if id == "layout-component-constraint-9"
        ),
        "the constraint did not become part-offset with its id: {:?}",
        document.constraints
    );
}

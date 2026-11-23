//! The legacy position Inspector applies both position fields as one edit.
use crate::presentation::test_mount_helpers;
use crate::runtime::Runtime;
use boardstudio_core::model::Vec2;
use boardstudio_web_test_runtime::layout_fixtures::{part_position, position_edit_document};
use boardstudio_web_test_runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::test_wait::{wait_until, wait_until_step};
use std::rc::Rc;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

async fn undo(runtime: &Rc<Runtime>) {
    runtime.submit(boardstudio_application::Event::Undo {
        operation_id: runtime.operation(),
    });
    support::run_pending(runtime).await;
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
    let runtime = test_mount_helpers::open_document(position_edit_document()).await;
    runtime.submit(boardstudio_application::Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: vec!["b".into(), "a".into()],
        range_part_ids: vec![],
        mode: boardstudio_application::SelectionMode::Replace,
    });
    let dom = dioxus::prelude::VirtualDom::new(old_inspector_host);
    dom.provide_root_context(runtime.clone());
    let root = test_mount_helpers::mount(dom, None);
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
        part_position(&runtime, "b") == Some(Vec2 { x: 45.0, y: 7.0 })
            && part_position(&runtime, "a") == Some(Vec2 { x: 5.0, y: 7.0 })
    })
    .await;
    assert_eq!(part_position(&runtime, "b"), Some(Vec2 { x: 45.0, y: 7.0 }));
    assert_eq!(part_position(&runtime, "a"), Some(Vec2 { x: 5.0, y: 7.0 }));
    undo(&runtime).await;
    assert_eq!(part_position(&runtime, "b"), Some(Vec2 { x: 40.0, y: 0.0 }));
    assert_eq!(part_position(&runtime, "a"), Some(Vec2 { x: 0.0, y: 0.0 }));
    runtime.unsubscribe();
}

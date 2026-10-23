use super::*;
use crate::runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
use wasm_bindgen_test::wasm_bindgen_test;

fn host() -> Element {
    let mut version = use_signal(|| 0_u64);
    use_context_provider(|| version);
    let workspace = use_signal(|| "Layout");
    use_context_provider(|| crate::WorkspaceState(workspace));
    let selected = use_signal(|| None::<ScopedTreeContext>);
    let anchor = use_signal(|| None::<Scope>);
    let generation = use_signal(|| 0);
    use_context_provider(|| crate::SelectionAdapter::new(selected, anchor, generation));
    rsx! {
        button { id: "refresh-tree", onclick: move |_| version += 1, "Refresh" }
        output { id: "tree-version", "{version()}" }
        Objects {
            selected_context: selected,
            on_select: EventHandler::default(), on_navigate: EventHandler::default(),
            on_nudge: EventHandler::default(), on_open_geometry_scripts: EventHandler::default(),
        }
    }
}

#[wasm_bindgen_test]
async fn tree_projection_skips_runtime_updates_and_tracks_disclosures_grouping_and_commits() {
    let runtime = support::new_runtime();
    support::open_document(&runtime, super::matrix_edit_test_support::matrix_document()).await;
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let mut dom = VirtualDom::new(host);
    dom.insert_any_root_context(Box::new(runtime.clone()));
    TREE_BUILDS.set(0);
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("Objects tree mounts", || {
        root.query_selector("[role=treeitem]").unwrap().is_some()
    })
    .await;
    let initial = TREE_BUILDS.get();
    let click = |name: &str| by_role_name(&root, "button", name).unwrap().click();
    click("Refresh");
    wait_until("runtime update renders", || {
        root.query_selector("#tree-version")
            .unwrap()
            .unwrap()
            .text_content()
            .as_deref()
            == Some("1")
    })
    .await;
    assert_eq!(
        TREE_BUILDS.get(),
        initial,
        "preview-only runtime updates must not rebuild accepted tree data"
    );
    click("Collapse Main");
    wait_until("board collapses", || {
        root.query_selector_all("[role=treeitem]").unwrap().length() == 1
    })
    .await;
    assert!(TREE_BUILDS.get() > initial);
    click("Expand Main");
    wait_until("board expands", || {
        root.query_selector_all("[role=treeitem]").unwrap().length() > 1
    })
    .await;
    let select = by_role_name(&root, "button", "Group objects by").unwrap();
    select.click();
    wait_until("the grouping choices open", || {
        by_role_name(&root, "option", "By part").is_some()
    })
    .await;
    let option = by_role_name(&root, "option", "By part").unwrap();
    // The primitive selects on a complete pointer gesture, not a synthetic click.
    for phase in ["pointerdown", "pointerup"] {
        let pointer = web_sys::PointerEvent::new(phase).unwrap();
        pointer.init_event_with_bubbles_and_cancelable(phase, true, true);
        option.dispatch_event(&pointer).unwrap();
    }
    wait_until("grouping changes", || {
        root.text_content().unwrap().contains("Switches")
    })
    .await;
    let mut changed = runtime.model().accepted.unwrap().document.as_ref().clone();
    changed.boards[0].name = "Updated board".into();
    support::replace_document(&runtime, "tree-update", &changed.id.clone(), changed).await;
    click("Refresh");
    wait_until("accepted tree updates", || {
        root.text_content().unwrap().contains("Updated board")
    })
    .await;
    web_sys::window()
        .unwrap()
        .local_storage()
        .unwrap()
        .unwrap()
        .remove_item("boardstudio:v2:object-view")
        .unwrap();
    root.remove();
}

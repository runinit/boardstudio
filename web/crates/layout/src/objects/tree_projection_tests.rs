use super::*;
use crate::runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
use wasm_bindgen::JsCast;
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

fn setup_host() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let version = use_signal(|| 0_u64);
    use_context_provider(|| version);
    use_hook({
        let runtime = runtime.clone();
        move || {
            runtime.subscribe(Rc::new(move || {
                let mut version = version;
                version += 1;
            }))
        }
    });
    use_drop({
        let runtime = runtime.clone();
        move || runtime.unsubscribe()
    });
    let workspace = use_signal(|| "Layout");
    use_context_provider(|| crate::WorkspaceState(workspace));
    let mut selected = use_signal(|| None::<ScopedTreeContext>);
    let anchor = use_signal(|| None::<Scope>);
    let generation = use_signal(|| 0);
    use_context_provider(|| crate::SelectionAdapter::new(selected, anchor, generation));
    let setup = use_matrix_setup(runtime, version, selected, anchor, workspace, generation);
    rsx! {
        button { onclick: move |_| setup.on_open.call(()), "Open matrix" }
        if let Some(projection) = setup.projection {
            MatrixSetup { projection, on_create: setup.on_create, on_cancel: setup.on_cancel }
        }
        Objects {
            selected_context: selected,
            on_select: move |request: TreeSelectRequest| selected.set(Some(ScopedTreeContext {
                scope: request.scope, context: request.context,
            })),
            on_navigate: EventHandler::default(), on_nudge: EventHandler::default(),
            on_open_geometry_scripts: EventHandler::default(),
        }
    }
}

#[wasm_bindgen_test]
async fn setup_create_seeds_new_split_depth_without_reopening_existing_disclosures() {
    use boardstudio_core::model::{HardwareConfiguration, HardwareTopology};
    let runtime = support::new_runtime();
    let mut empty = super::matrix_edit_test_support::matrix_document();
    empty.matrices.clear();
    empty.parts.clear();
    empty.boards[0].part_ids.clear();
    empty.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Split,
        ..Default::default()
    });
    support::open_document(&runtime, empty).await;
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let mut dom = VirtualDom::new(setup_host);
    dom.insert_any_root_context(Box::new(runtime.clone()));
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("setup tree mounts before Create", || {
        by_role_name(&root, "button", "Open matrix").is_some()
    })
    .await;
    let click = |name: &str| by_role_name(&root, "button", name).unwrap().click();
    click("Open matrix");
    wait_until("real setup opens", || {
        by_role_name(&root, "button", "Create matrix").is_some()
    })
    .await;
    click("Create matrix");
    support::run_pending(&runtime).await;
    wait_until("split creation lands", || {
        support::drive_pending(&runtime);
        runtime
            .model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .layouts
            .len()
            == 2
    })
    .await;
    wait_until(
        "new split exposes columns without remounting Objects",
        || {
            root.query_selector_all("button[id^='m1-object-tree-column:']:not([id$='-disclosure'])")
                .unwrap()
                .length()
                == 12
        },
    )
    .await;
    let first_matrix = runtime.model().accepted.unwrap().document.matrices[0]
        .id
        .clone();
    root.query_selector(&format!(
        "[id='m1-object-tree-column:{first_matrix}:0-disclosure']"
    ))
    .unwrap()
    .unwrap()
    .unchecked_into::<web_sys::HtmlElement>()
    .click();
    wait_until("column exposes real key nodes", || {
        root.query_selector(&format!("[id='m1-object-tree-key:{first_matrix}:0:0']"))
            .unwrap()
            .is_some()
    })
    .await;
    let key = root
        .query_selector(&format!("[id='m1-object-tree-key:{first_matrix}:0:0']"))
        .unwrap()
        .unwrap();
    key.clone().unchecked_into::<web_sys::HtmlElement>().click();
    wait_until("key is selectable", || {
        key.parent_element()
            .unwrap()
            .get_attribute("aria-selected")
            .as_deref()
            == Some("true")
    })
    .await;
    // A second Create changes the defaults; it must seed only new branches, not reopen old ones.
    let old_layout = runtime.model().accepted.unwrap().document.layouts[0]
        .id
        .clone();
    let old_disclosure = root
        .query_selector(&format!(
            "[id='m1-object-tree-half:{old_layout}-disclosure']"
        ))
        .unwrap()
        .unwrap();
    old_disclosure
        .clone()
        .unchecked_into::<web_sys::HtmlElement>()
        .click();
    wait_until("existing layout stays closed", || {
        old_disclosure.get_attribute("aria-expanded").as_deref() == Some("false")
    })
    .await;
    click("Open matrix");
    wait_until("second setup opens", || {
        by_role_name(&root, "button", "Create matrix").is_some()
    })
    .await;
    click("Create matrix");
    support::run_pending(&runtime).await;
    wait_until("second split creation lands", || {
        support::drive_pending(&runtime);
        runtime
            .model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .layouts
            .len()
            == 4
    })
    .await;
    let new_matrix = runtime.model().accepted.unwrap().document.matrices[2]
        .id
        .clone();
    wait_until("new split defaults settle", || {
        root.query_selector(&format!("[id='m1-object-tree-column:{new_matrix}:0']"))
            .unwrap()
            .is_some()
    })
    .await;
    assert_eq!(
        old_disclosure.get_attribute("aria-expanded").as_deref(),
        Some("false"),
        "Create must preserve the user's collapsed layout"
    );
    let new_layout = runtime.model().accepted.unwrap().document.layouts[2]
        .id
        .clone();
    wait_until("setup retains its new-matrix selection", || {
        root.query_selector(&format!("[id='m1-object-tree-half:{new_layout}']"))
            .unwrap()
            .unwrap()
            .parent_element()
            .unwrap()
            .get_attribute("aria-selected")
            .as_deref()
            == Some("true")
    })
    .await;
    root.remove();
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
    assert_eq!(
        select.text_content().as_deref(),
        Some("By part"),
        "the actual controlled trigger must follow the accepted local grouping"
    );
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

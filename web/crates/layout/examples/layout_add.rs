//! Mounts the actual standalone Objects Add entry; the Editor covers its shell sibling.
#[cfg(target_arch = "wasm32")]
use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
fn fixture() -> Element {
    use boardstudio_core::model::{Board, ProjectDoc};
    use boardstudio_web_runtime::runtime::project_name_test_support as support;
    use boardstudio_web_ui_model::{selection::SelectionAdapter, state::WorkspaceState};

    let runtime = use_hook(support::new_runtime);
    use_context_provider(|| runtime.clone());
    let version = use_signal(|| 0_u64);
    use_context_provider(|| version);
    let workspace = use_signal(|| "Layout");
    use_context_provider(|| WorkspaceState(workspace));
    let selected = use_signal(|| None);
    let anchor = use_signal(|| None);
    let generation = use_signal(|| 1_u64);
    use_context_provider(|| SelectionAdapter::new(selected, anchor, generation));
    let layout_target = use_signal(|| None);
    let parts_query = use_signal(String::new);
    let mut ready = use_signal(|| false);
    use_future(move || {
        let runtime = runtime.clone();
        async move {
            let mut document = ProjectDoc::empty("standalone-add-016", "Standalone Add");
            document.boards.push(Board {
                id: "board".into(),
                name: "Main board".into(),
                outline_ids: vec![],
                part_ids: vec![],
                net_ids: vec![],
                thickness: 1.6,
                traces: vec![],
                vias: vec![],
            });
            support::open_document(&runtime, document).await;
            ready.set(true);
        }
    });
    rsx! {
        if ready() {
            boardstudio_web_layout::objects::Objects {
                selected_context: selected,
                on_select: |_| {}, on_navigate: |_| {}, on_nudge: |_| {},
                on_open_geometry_scripts: |_| {},
                on_place_component: Some(EventHandler::default()),
                layout_target: Some(layout_target), parts_query: Some(parts_query),
                on_browse_parts: Some(EventHandler::default()),
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    dioxus::launch(fixture);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {}

//! Mount the public Inspector; ModuleAttachment retains its own connection draft and validation.
#[cfg(target_arch = "wasm32")]
use boardstudio_model::model::*;
#[cfg(target_arch = "wasm32")]
use boardstudio_web_parts::parts::*;
#[cfg(target_arch = "wasm32")]
use boardstudio_web_runtime::runtime::Runtime;
#[cfg(target_arch = "wasm32")]
use boardstudio_web_test_runtime::project_name_test_support as support;
#[cfg(target_arch = "wasm32")]
use boardstudio_web_ui_model::{Workspace, state::WorkspaceState};
#[cfg(target_arch = "wasm32")]
use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
fn fixture_document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("inspector-020", "Inspector control check");
    document.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    for (id, interfaces) in [
        (
            "module",
            serde_json::json!([{
                "id":"module-port", "role":"module",
                "signals":["sclk","miso","cs","gpio2","mosi","gpio1","v5","rgb","scl","sda","gnd","v3v3"]
            }]),
        ),
        ("no-port", serde_json::json!([])),
    ] {
        document.module_definitions.push(serde_json::from_value(serde_json::json!({
            "id":id, "name":id, "family":"check", "variant":id,
            "source":{"repository":"check", "revision":"check", "path":"check", "license":"check"},
            "board":{"contours":[]}, "interfaces":interfaces, "electrical":{"protocol":"gpio"}
        })).unwrap());
    }
    document
}

#[cfg(target_arch = "wasm32")]
#[component]
fn InspectorHost() -> Element {
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
    let _ = version();
    let workspace = use_signal(|| Workspace::Parts);
    use_context_provider(|| WorkspaceState(workspace));
    let assembly = use_signal(|| None::<MatrixPresetId>);
    use_context_provider(|| PartsAssemblySelection(assembly));
    let orientation = use_signal(SwitchOrientation::default);
    use_context_provider(|| PartsAssemblyOrientation(orientation));
    let mut generation = use_signal(|| 1_u64);
    use_context_provider(|| PartsSelectionGeneration(generation));
    let activation = use_signal(|| 0_u64);
    use_context_provider(|| PartsPreviewActivation(activation));
    let mut selected = use_signal(|| Some((runtime.scope(), "module:module".into())));
    let query = use_signal(String::new);
    let selected_context = use_signal(|| None);
    let layout_target = use_signal(|| None);
    let snapshot = runtime.model().accepted.unwrap();
    let scope = runtime.scope();
    let choose_scope = scope.clone();
    rsx! {
        boardstudio_web_ui_shared::ui::toast::ToastProvider {
            boardstudio_web_ui_shared::ui::button::Button {
                onclick: move |_| { selected.set(Some((choose_scope.clone(), "module:no-port".into()))); generation += 1; },
                "Choose module without port"
            }
            boardstudio_web_ui_shared::ui::button::Button { onclick: move |_| { let mut version = version; version += 1; }, "Refresh parent" }
            PartsInspectorPanel {
                snapshot, scope, query, selected, selected_context, layout_target,
                on_place_controller: |_| {}, on_place_component: |_| {},
                controller_placement_enabled: true, placement_busy: false, placement_error: None,
                on_place_assembly: |_| {}, on_board_placed: |_| {},
                on_open_module_placement: |_| {}, on_module_attached: |_| {},
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn fixture() -> Element {
    let runtime = use_hook(support::new_runtime);
    use_context_provider(|| runtime.clone());
    let opened = use_resource({
        let runtime = runtime.clone();
        move || {
            let runtime = runtime.clone();
            async move { support::open_document(&runtime, fixture_document()).await }
        }
    });
    if opened.read().is_none() {
        return rsx! { p { role: "status", "Opening accepted fixture" } };
    }
    rsx! { InspectorHost {} }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    dioxus_web::launch::launch_virtual_dom(VirtualDom::new(fixture), dioxus_web::Config::new());
}
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    panic!("Inspector control check requires wasm32");
}

use crate::runtime::Runtime;
use boardstudio_web_layout::board_inspector::{BoardInspector, use_board_inspector};
use boardstudio_web_test_runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use dioxus::prelude::*;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

fn host() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let version = use_signal(|| 0_u64);
    use_context_provider(|| version);
    let _ = version();
    let selected = use_signal(|| None);
    let workspace = use_signal(|| "Layout");
    let generation = use_signal(|| 0);
    use_hook({
        let runtime = runtime.clone();
        move || {
            runtime.subscribe(Rc::new(move || {
                let mut version = version;
                version += 1;
            }))
        }
    });
    let mount = use_board_inspector(runtime, selected, workspace, generation);
    rsx! {
        if mount.projection.is_some() {
            BoardInspector { mount }
        }
    }
}

#[wasm_bindgen_test]
async fn board_inspector_measurements_keep_layout_without_legacy_classes() {
    let runtime = support::new_runtime();
    let mut project = boardstudio_core::model::ProjectDoc::empty("board-measure", "Project");
    project.boards.push(serde_json::from_value(serde_json::json!({
        "id": "board", "name": "Board", "outlineIds": [], "partIds": [], "netIds": [], "thickness": 1.6, "traces": [], "vias": []
    })).unwrap());
    support::open_document(&runtime, project).await;
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let stylesheet = document.create_element("style").unwrap();
    stylesheet.set_text_content(Some(include_str!("../../assets/tailwind.css")));
    document.head().unwrap().append_child(&stylesheet).unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(runtime.clone());
    let mut mounted = MountedFixture::new(root.clone(), dom);
    mounted.own_node(stylesheet.clone());
    boardstudio_web_ui_shared::test_wait::wait_until("board inspector mounts", || {
        boardstudio_web_ui_shared::test_role::by_role_name(&root, "textbox", "Board name").is_some()
    })
    .await;
    let labels = root.query_selector_all("span").unwrap();
    let label = (0..labels.length())
        .filter_map(|index| labels.item(index))
        .find(|node| node.text_content().as_deref() == Some("Placed parts"))
        .unwrap()
        .dyn_into::<web_sys::Element>()
        .unwrap();
    let row = label.parent_element().unwrap();
    let classes = row.get_attribute("class").unwrap_or_default();
    row.set_attribute(
        "class",
        &classes
            .split_whitespace()
            .filter(|class| !class.starts_with("m1-"))
            .collect::<Vec<_>>()
            .join(" "),
    )
    .unwrap();
    let style = web_sys::window()
        .unwrap()
        .get_computed_style(&row)
        .unwrap()
        .unwrap();
    assert_eq!(style.get_property_value("display").unwrap(), "flex");
    assert_eq!(
        style.get_property_value("justify-content").unwrap(),
        "space-between"
    );
    assert_eq!(style.get_property_value("padding-top").unwrap(), "8px");
    let input =
        boardstudio_web_ui_shared::test_role::by_role_name(&root, "textbox", "Board name").unwrap();
    let field = input.closest("[data-slot=field]").unwrap().unwrap();
    let classes = field.get_attribute("class").unwrap_or_default();
    field
        .set_attribute(
            "class",
            &classes
                .split_whitespace()
                .filter(|class| !class.starts_with("m1-"))
                .collect::<Vec<_>>()
                .join(" "),
        )
        .unwrap();
    let field_style = web_sys::window()
        .unwrap()
        .get_computed_style(&field)
        .unwrap()
        .unwrap();
    assert_eq!(field_style.get_property_value("display").unwrap(), "grid");
    assert_eq!(field_style.get_property_value("gap").unwrap(), "6px");
    runtime.unsubscribe();
    stylesheet.remove();
    root.remove();
}

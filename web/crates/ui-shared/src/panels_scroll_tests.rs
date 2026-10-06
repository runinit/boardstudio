use super::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn composition() -> Element {
    let open = use_signal(|| true);
    let settings = use_signal(|| PanelSettings {
        mode: PanelMode::Pinned,
        width: None,
    });
    rsx! {
        style { {include_str!("../../../assets/m1.css")} }
        style { "@media (min-width:761px) {{ #inspector-scroll-test-root {{ height:600px; }} }}" }
        div { class: "m1-workbench",
            div { class: "m1-editor",
                div { class: "m1-editor-body",
                    ObjectsPanel { compact_open: open, settings,
                        aside { class: "m1-objects", "Objects fixture" }
                    }
                    section { class: "m1-workspace-content", id: "scroll-test-canvas",
                        "Canvas fixture"
                    }
                    InspectorPanel { compact_open: open, settings,
                        section { class: "m1-selected-context", "First contextual section" }
                        section { id: "scroll-test-controls",
                            for index in 0..40 {
                                button { id: "scroll-test-control-{index}", style: "display:block;height:42px;width:100%", "Control {index}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn mount_fixture(component: fn() -> Element) -> web_sys::Element {
    let document = web_sys::window().unwrap().document().unwrap();
    if let Some(previous) = document.get_element_by_id("inspector-scroll-test-root") {
        previous.remove();
    }
    let root = document.create_element("div").unwrap();
    root.set_id("inspector-scroll-test-root");
    root.set_attribute("style", "width:min(100%,1000px)")
        .unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(component),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    root
}

fn element(selector: &str) -> HtmlElement {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(&format!("#inspector-scroll-test-root {selector}"))
        .unwrap()
        .unwrap()
        .dyn_into()
        .unwrap()
}

fn overflow_y(node: &web_sys::Element) -> String {
    let window = web_sys::window().unwrap();
    let style = js_sys::Reflect::get(window.as_ref(), &"getComputedStyle".into())
        .unwrap()
        .dyn_into::<js_sys::Function>()
        .unwrap()
        .call1(window.as_ref(), node.as_ref())
        .unwrap();
    js_sys::Reflect::get(&style, &"overflowY".into())
        .unwrap()
        .as_string()
        .unwrap()
}

fn scroll_owner(element: &HtmlElement) -> Option<HtmlElement> {
    let mut next = element.parent_element();
    while let Some(node) = next {
        let overflow = overflow_y(&node);
        if matches!(overflow.as_str(), "auto" | "scroll")
            && node.scroll_height() > node.client_height()
        {
            return Some(node.dyn_into().unwrap());
        }
        next = node.parent_element();
    }
    None
}

#[wasm_bindgen_test]
async fn inspector_content_has_a_bounded_scroll_owner_outside_its_heading_and_menu() {
    let root = mount_fixture(composition);
    gloo_timers::future::TimeoutFuture::new(80).await;
    let last = element("#scroll-test-control-39");
    let owner = scroll_owner(&last).expect("long Inspector content must have a wheel-scrollable ancestor before the clipped editor frame");
    let frame = element(".m1-editor-body");
    assert!(owner.client_height() > 0 && owner.client_height() < frame.client_height());
    assert_eq!(
        frame.scroll_height(),
        frame.client_height(),
        "Inspector cannot spill into the clipped editor frame"
    );
    let heading = element("#m1-inspector-panel .m1-panel-heading");
    let heading_top = heading.get_bounding_client_rect().top();
    let canvas = element("#scroll-test-canvas").get_bounding_client_rect();
    // Native focus scrolling proves keyboard targets stay inside the content viewport.
    last.focus().unwrap();
    assert!(owner.scroll_top() > 0);
    assert!(
        last.get_bounding_client_rect().bottom() <= owner.get_bounding_client_rect().bottom() + 1.0
    );
    assert_eq!(heading.get_bounding_client_rect().top(), heading_top);
    element("#m1-inspector-panel-options").click();
    gloo_timers::future::TimeoutFuture::new(40).await;
    let menu = element("#m1-inspector-panel-options-group");
    assert!(
        !owner.contains(Some(menu.as_ref())),
        "panel options must remain outside scrolling content"
    );
    assert!(menu.get_bounding_client_rect().top() >= heading_top);
    let current_canvas = element("#scroll-test-canvas").get_bounding_client_rect();
    assert_eq!(
        (current_canvas.width(), current_canvas.height()),
        (canvas.width(), canvas.height())
    );
    root.remove();
}

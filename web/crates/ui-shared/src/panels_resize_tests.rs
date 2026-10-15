use super::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn composition() -> Element {
    let warning = use_signal(|| false);
    let objects = use_panel_settings(PanelSide::Objects, warning);
    let inspector = use_panel_settings(PanelSide::Inspector, warning);
    let open = use_signal(|| true);
    rsx! {
        style { {include_str!("../../../assets/m1.css")} }
        div { class: "m1-workbench",
            div { class: "m1-editor",
                div { class: "m1-editor-body",
                    ObjectsPanel { compact_open: open, settings: objects,
                        header_action: None,
                        aside { class: "m1-objects", "Objects fixture" }
                    }
                    section { class: "m1-workspace-content", "Canvas fixture" }
                    InspectorPanel { compact_open: open, settings: inspector,
                        section { class: "m1-selected-context", "Inspector fixture" }
                    }
                }
            }
        }
    }
}

fn element(selector: &str) -> HtmlElement {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(&format!("#panel-resize-test-root {selector}"))
        .unwrap()
        .unwrap()
        .dyn_into()
        .unwrap()
}

fn dispatch_pointer(target: &HtmlElement, kind: &str, pointer_id: i32, client_x: f64) {
    let event = js_sys::eval(&format!(
        "new PointerEvent('{kind}', {{bubbles:true, pointerId:{pointer_id}, clientX:{client_x}, clientY:200, button:0}})"
    ))
    .unwrap()
    .dyn_into::<web_sys::Event>()
    .unwrap();
    target.dispatch_event(&event).unwrap();
}

fn install_browser_spies() {
    js_sys::eval(
        "(() => {\
          const element = Element.prototype;\
          window.__panelResizeCapture = null;\
          window.__panelResizeOriginalCapture = element.setPointerCapture;\
          window.__panelResizeOriginalHasCapture = element.hasPointerCapture;\
          window.__panelResizeOriginalReleaseCapture = element.releasePointerCapture;\
          element.setPointerCapture = id => { window.__panelResizeCapture = id; };\
          element.hasPointerCapture = id => window.__panelResizeCapture === id;\
          element.releasePointerCapture = id => { if (window.__panelResizeCapture === id) window.__panelResizeCapture = null; };\
          window.__panelResizeOriginalSetItem = Storage.prototype.setItem;\
          window.__panelResizeWrites = 0;\
          Storage.prototype.setItem = function(key, value) {\
            if (key === 'boardstudio:v2:panel:left') window.__panelResizeWrites++;\
            return window.__panelResizeOriginalSetItem.call(this, key, value);\
          };\
        })()",
    )
    .unwrap();
}

fn restore_browser_spies() {
    js_sys::eval(
        "(() => {\
          const element = Element.prototype;\
          element.setPointerCapture = window.__panelResizeOriginalCapture;\
          element.hasPointerCapture = window.__panelResizeOriginalHasCapture;\
          element.releasePointerCapture = window.__panelResizeOriginalReleaseCapture;\
          Storage.prototype.setItem = window.__panelResizeOriginalSetItem;\
          delete window.__panelResizeCapture;\
          delete window.__panelResizeOriginalCapture;\
          delete window.__panelResizeOriginalHasCapture;\
          delete window.__panelResizeOriginalReleaseCapture;\
          delete window.__panelResizeOriginalSetItem;\
          delete window.__panelResizeWrites;\
        })()",
    )
    .unwrap();
}

fn panel_resize_write_count() -> u32 {
    js_sys::Reflect::get(
        web_sys::window().unwrap().as_ref(),
        &JsValue::from_str("__panelResizeWrites"),
    )
    .unwrap()
    .as_f64()
    .unwrap() as u32
}

#[wasm_bindgen_test]
async fn panel_drag_persists_only_its_final_width_on_pointer_cancel() {
    let window = web_sys::window().unwrap();
    let storage = window.local_storage().unwrap().unwrap();
    storage.remove_item("boardstudio:v2:panel:left").unwrap();
    let document = window.document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("panel-resize-test-root");
    root.set_attribute("style", "width:1200px;height:700px")
        .unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(composition),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    gloo_timers::future::TimeoutFuture::new(80).await;
    install_browser_spies();

    let handle = element(".m1-panel-resize.is-left");
    let frame = element(".m1-editor-body");
    let frame_left = frame.get_bounding_client_rect().left();
    dispatch_pointer(&handle, "pointerdown", 17, frame_left + 240.0);
    for width in [290.0, 310.0, 330.0, 350.0] {
        dispatch_pointer(&handle, "pointermove", 17, frame_left + width);
    }
    let preview_width = frame
        .style()
        .get_property_value("--m1-left-panel-width")
        .unwrap();
    let writes_during_drag = panel_resize_write_count();
    dispatch_pointer(&handle, "pointercancel", 17, frame_left + 350.0);
    gloo_timers::future::TimeoutFuture::new(80).await;

    let writes_after_cancel = panel_resize_write_count();
    let stored = storage
        .get_item("boardstudio:v2:panel:left")
        .unwrap()
        .unwrap_or_default();
    restore_browser_spies();
    root.remove();

    assert_eq!(preview_width, "350px");
    assert_eq!(writes_during_drag, 0);
    assert_eq!(writes_after_cancel, 1);
    assert!(
        stored.contains("\"width\":350.0"),
        "unexpected stored panel settings: {stored}"
    );
}

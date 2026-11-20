use super::*;
use crate::test_mount::{MountedFixture, SavedLocalStorage};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn composition() -> Element {
    let warning = use_signal(|| false);
    let objects = use_panel_settings(PanelSide::Objects, warning);
    let inspector = use_panel_settings(PanelSide::Inspector, warning);
    let open = use_signal(|| true);
    rsx! {
        style { {include_str!("../../../assets/workbench-base.css")} }
        style { {include_str!("../../../assets/tailwind.css")} }
        div { class: "flex flex-col w-full h-full min-h-0 overflow-hidden",
            div { class: "flex flex-1 flex-col min-h-0",
                div { class: "dock-desk relative flex flex-1 flex-row gap-2 p-2 min-h-0 overflow-hidden bg-background",
                    ObjectsPanel { compact_open: open, settings: objects,
                        header_action: None,
                        aside { class: "flex flex-col min-w-0 min-h-0 bg-card", "Objects fixture" }
                    }
                    section { class: "order-2 flex-1 min-w-0 min-h-0 overflow-hidden rounded-xl bg-card shadow-sm", "Canvas fixture" }
                    InspectorPanel { compact_open: open, settings: inspector,
                        section { class: "px-3 py-3.5 border-b border-border", "Inspector fixture" }
                    }
                }
            }
        }
    }
}

fn dispatch_pointer(
    target: &HtmlElement,
    kind: &str,
    pointer_id: i32,
    client_x: f64,
) -> Result<(), JsValue> {
    let event = js_sys::eval(&format!(
        "new PointerEvent('{kind}', {{bubbles:true, pointerId:{pointer_id}, clientX:{client_x}, clientY:200, button:0}})"
    ))?
    .dyn_into::<web_sys::Event>()?;
    target.dispatch_event(&event)?;
    Ok(())
}

struct BrowserSpies;

impl Drop for BrowserSpies {
    fn drop(&mut self) {
        restore_browser_spies();
    }
}

fn install_browser_spies() -> BrowserSpies {
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
    BrowserSpies
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

fn panel_resize_write_count(window: &web_sys::Window) -> Result<u32, JsValue> {
    js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("__panelResizeWrites"))?
        .as_f64()
        .map(|count| count as u32)
        .ok_or_else(|| JsValue::from_str("missing panel resize write count"))
}

#[wasm_bindgen_test]
async fn panel_drag_persists_only_its_final_width_on_pointer_cancel() {
    let window = web_sys::window().unwrap();
    const KEY: &str = "boardstudio:v2:panel:left";
    let storage = window.local_storage().unwrap().unwrap();
    let saved = SavedLocalStorage::new(storage.clone(), &[KEY]).unwrap();
    storage.remove_item(KEY).unwrap();
    let document = window.document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_attribute("style", "width:1200px;height:700px")
        .unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let mounted = MountedFixture::new(root.clone(), VirtualDom::new(composition));
    gloo_timers::future::TimeoutFuture::new(80).await;

    // Capture DOM and JS failures as values so cleanup precedes assertions even on an aborting wasm panic.
    let observed = async {
        let handle = crate::test_role::by_role_name(&root, "separator", "Resize Objects")
            .ok_or_else(|| JsValue::from_str("missing Objects resize handle"))?;
        let frame = root
            .query_selector(".dock-desk")?
            .ok_or_else(|| JsValue::from_str("missing desktop dock"))?;
        let frame_left = frame.get_bounding_client_rect().left();
        let _spies = install_browser_spies();
        dispatch_pointer(&handle, "pointerdown", 17, frame_left + 260.0)?;
        for width in [290.0, 310.0, 330.0, 350.0] {
            dispatch_pointer(&handle, "pointermove", 17, frame_left + width)?;
        }
        gloo_timers::future::TimeoutFuture::new(40).await;
        let preview_width = handle.get_attribute("aria-valuenow");
        let writes_during_drag = panel_resize_write_count(&window)?;
        dispatch_pointer(&handle, "pointercancel", 17, frame_left + 350.0)?;
        gloo_timers::future::TimeoutFuture::new(80).await;
        Ok::<_, JsValue>((
            preview_width,
            writes_during_drag,
            panel_resize_write_count(&window)?,
            storage.get_item(KEY)?.unwrap_or_default(),
        ))
    }
    .await;
    drop(mounted);
    drop(saved);
    let (preview_width, writes_during_drag, writes_after_cancel, stored) = observed.unwrap();
    assert_eq!(preview_width.unwrap(), "350");
    assert_eq!(writes_during_drag, 0);
    assert_eq!(writes_after_cancel, 1);
    assert!(
        stored.contains("\"width\":350.0"),
        "unexpected stored panel settings: {stored}"
    );
}

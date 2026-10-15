//! Render viewer-owned layer controls in the dock without moving DOM or owning visibility.
use dioxus::prelude::*;

#[derive(Clone, Copy)]
struct LayerDock(Signal<Option<(ScopeId, Element)>>);

pub fn use_layer_dock_provider() {
    let content = use_signal(|| None);
    use_context_provider(|| LayerDock(content));
}

/// Standalone viewers retain their controls when there is no workbench outlet.
pub fn use_docked_layers(content: Element) -> Element {
    let dock = try_consume_context::<LayerDock>();
    let owner = dioxus::dioxus_core::current_scope_id();
    use_effect(use_reactive((&content,), move |(content,)| {
        if let Some(LayerDock(mut slot)) = dock {
            slot.set(Some((owner, content)));
        }
    }));
    use_drop(move || {
        if let Some(LayerDock(mut slot)) = dock {
            let owned = slot
                .peek()
                .as_ref()
                .is_some_and(|(scope, _)| *scope == owner);
            if owned {
                slot.set(None);
            }
        }
    });
    if dock.is_some() {
        rsx! {}
    } else {
        content
    }
}

#[component]
pub fn LayerDockOutlet() -> Element {
    let LayerDock(slot) = use_context();
    rsx! { div { class: "m1-layer-dock", {slot.read().as_ref().map(|(_, content)| content.clone())} } }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    #[component]
    fn Source() -> Element {
        let mut visible = use_signal(|| true);
        use_docked_layers(rsx! {
            button { id: "layer-dock-toggle", aria_pressed: "{visible()}",
                onclick: move |_| visible.set(!visible()), "Switches" }
        })
    }

    fn fixture() -> Element {
        use_layer_dock_provider();
        let mut mounted = use_signal(|| true);
        rsx! {
            button { id: "layer-dock-unmount", onclick: move |_| mounted.set(false), "Unmount viewer" }
            aside { LayerDockOutlet {} }
            section { if mounted() { Source {} } }
        }
    }

    #[wasm_bindgen_test]
    async fn docked_controls_keep_viewer_callbacks_and_clear_on_unmount() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        crate::test_wait::wait_until("layer outlet is mounted", || {
            root.query_selector("aside #layer-dock-toggle")
                .unwrap()
                .is_some()
        })
        .await;
        assert!(
            root.query_selector("section #layer-dock-toggle")
                .unwrap()
                .is_none()
        );
        let click = |selector: &str| {
            root.query_selector(selector)
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::HtmlElement>()
                .unwrap()
                .click()
        };
        click("#layer-dock-toggle");
        crate::test_wait::wait_until("docked callback updates its owner", || {
            root.query_selector("#layer-dock-toggle[aria-pressed='false']")
                .unwrap()
                .is_some()
        })
        .await;
        click("#layer-dock-unmount");
        crate::test_wait::wait_until("old viewer controls are removed", || {
            root.query_selector("#layer-dock-toggle").unwrap().is_none()
        })
        .await;
        root.remove();
    }
}

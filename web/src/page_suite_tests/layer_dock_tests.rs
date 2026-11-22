use boardstudio_web_ui_shared::layer_dock::*;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::ui::button::Button;
use dioxus::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

#[component]
fn Source() -> Element {
    let mut visible = use_signal(|| true);
    use_docked_layers(rsx! {
        Button { id: "layer-dock-toggle", aria_pressed: visible(),
            onclick: move |_| visible.set(!visible()), "Switches" }
    })
}

fn fixture() -> Element {
    use_layer_dock_provider();
    let mut mounted = use_signal(|| true);
    rsx! {
        Button { id: "layer-dock-unmount", onclick: move |_| mounted.set(false), "Unmount viewer" }
        aside { LayerDockOutlet {} }
        section { if mounted() { Source {} } }
    }
}

#[wasm_bindgen_test]
async fn docked_controls_keep_viewer_callbacks_and_clear_on_unmount() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let _mounted = MountedFixture::new(root.clone(), VirtualDom::new(fixture));
    boardstudio_web_ui_shared::test_wait::wait_until("layer outlet is mounted", || {
        root.query_selector("aside #layer-dock-toggle")
            .unwrap()
            .is_some()
    })
    .await;
    let outlet = root.query_selector("aside > div").unwrap().unwrap();
    assert!(
        outlet
            .get_attribute("class")
            .unwrap_or_default()
            .split_whitespace()
            .any(|class| class == "empty:hidden"),
        "an empty outlet must hide without a legacy selector"
    );
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
    boardstudio_web_ui_shared::test_wait::wait_until("docked callback updates its owner", || {
        root.query_selector("#layer-dock-toggle[aria-pressed='false']")
            .unwrap()
            .is_some()
    })
    .await;
    click("#layer-dock-unmount");
    boardstudio_web_ui_shared::test_wait::wait_until("old viewer controls are removed", || {
        root.query_selector("#layer-dock-toggle").unwrap().is_none()
    })
    .await;
    assert!(
        outlet.matches(":empty").unwrap(),
        "unmounted outlet is empty"
    );
    root.remove();
}

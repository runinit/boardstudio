//! Trusted browser fixture for the shared layer controls; no production shell entry point.
#[cfg(target_arch = "wasm32")]
use boardstudio_web_ui_model::state::LayerVisibility;
#[cfg(target_arch = "wasm32")]
use boardstudio_web_ui_shared::{
    canvas_layers::{CanvasLayer, CanvasLayerGroup, CanvasLayers, LayerPanel, LayerToggleButton},
    ui::{button::Button, input::Input, label::Label},
};
#[cfg(target_arch = "wasm32")]
use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
#[component]
fn RetainedDraft() -> Element {
    let mut draft = use_signal(|| "Local draft".to_owned());
    rsx! {
        Label { html_for: "layer-retained-draft", "Layer note" }
        Input {
            id: "layer-retained-draft",
            value: draft(),
            oninput: move |event: FormEvent| draft.set(event
                    .value()),
        }
    }
}
#[cfg(target_arch = "wasm32")]
fn fixture() -> Element {
    let hidden = use_signal(std::collections::BTreeSet::new);
    let modules_hidden = use_signal(std::collections::BTreeSet::new);
    let footprints = use_signal(|| false);
    use_context_provider(|| LayerVisibility {
        hidden,
        modules_hidden,
        footprints,
    });
    let mut mounted = use_signal(|| true);
    let mut disabled_calls = use_signal(|| 0_u32);
    rsx! {
        Button { onclick: move |_| mounted.toggle(), "Mount or unmount layers" }
        if mounted() {
            div { id: "m1-objects-panel",
            div { CanvasLayers {
                initially_open: false,
                trigger_label: "Canvas visibility",
                trigger_id: "layers-016",
                list_id: "layer-list-016",
                groups: vec![
                    CanvasLayerGroup::ungrouped(
                        vec![
                            CanvasLayer::hidden("Keys", "Switches"),
                            CanvasLayer::layout_footprints(),
                            CanvasLayer::module_hidden("module", "Module"),
                        ],
                    ),
                ],
            } }
            div { LayerPanel {
                initially_open: true,
                trigger_label: "Layer notes",
                trigger_id: "layer-notes-016",
                list_id: "layer-notes-content-016",
                RetainedDraft {}
            } }
            fieldset { disabled: true,
                div { LayerPanel {
                    initially_open: true,
                    trigger_label: "Disabled layers",
                    trigger_id: "disabled-layers-016",
                    list_id: "disabled-layer-list-016",
                    LayerToggleButton {
                        label: "Disabled layer",
                        swatch: "Board",
                        aria_label: "Hide disabled layer",
                        visible: true,
                        on_toggle: move |_| disabled_calls += 1,
                    }
                } }
            }
            }
        }
        Button { "After layers" }
        output { role: "status", aria_label: "Hidden layers", "{hidden:?}" }
        output { role: "status", aria_label: "Hidden modules", "{modules_hidden:?}" }
        output { role: "status", aria_label: "Footprints visible", "{footprints}" }
        output { role: "status", aria_label: "Disabled callbacks", "{disabled_calls}" }
    }
}
#[cfg(target_arch = "wasm32")]
fn main() {
    dioxus_web::launch::launch_virtual_dom(VirtualDom::new(fixture), dioxus_web::Config::new());
}
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    panic!("the layer fixture requires wasm32-unknown-unknown");
}

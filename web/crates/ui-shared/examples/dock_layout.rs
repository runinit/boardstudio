//! Mounted production dock integration, without application/runtime fixtures.
#[cfg(target_arch = "wasm32")]
use boardstudio_web_ui_shared::{
    panels::{self, InspectorPanel, ObjectsPanel, PanelSide, use_panel_settings},
    ui::{button::Button, popover::{Popover, PopoverContent, PopoverTrigger}},
};
#[cfg(target_arch = "wasm32")]
use boardstudio_web_ui_model::state::{CompactPanelState, WorkspaceState};
#[cfg(target_arch = "wasm32")]
use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
fn fixture() -> Element {
    let mut workspace = use_signal(|| "Layout");
    use_context_provider(|| WorkspaceState(workspace));
    let objects_open = use_signal(|| false);
    let inspector_open = use_signal(|| false);
    use_context_provider(|| CompactPanelState { objects_open, inspector_open });
    let warning = use_signal(|| false);
    let objects = use_panel_settings(PanelSide::Objects, warning);
    let inspector = use_panel_settings(PanelSide::Inspector, warning);
    let mut clicks = use_signal(|| 0);
    let mut draft = use_signal(String::new);
    rsx! {
        div { class: "m1-workbench",
            header { style: "height:56px;display:flex;gap:4px;flex-wrap:wrap;flex-shrink:0",
                for name in ["Layout", "PCB", "Keymap", "Keycaps", "Case", "Parts"] {
                    Button { onclick: move |_| workspace.set(name), "{name}" }
                }
                panels::CompactPanelControls {}
            }
            section { class: "m1-editor",
                panels::CompactPanelScrim {}
                div { class: if workspace() == "Layout" { "m1-editor-body m1-editor-body--layout" } else { "m1-editor-body" },
                    if workspace() == "Layout" { panels::LayoutPanelSheet {} }
                    ObjectsPanel { compact_open: objects_open, settings: objects,
                        header_action: Some(rsx! { Button { "Add object" } }),
                        p { "Objects content" }
                    }
                    section { class: "m1-workspace-content", role: "region", aria_label: "Canvas",
                        Button { onclick: move |_| clicks += 1, "Canvas action" }
                        output { role: "status", aria_label: "Canvas clicks", "{clicks}" }
                    }
                    InspectorPanel { compact_open: inspector_open, settings: inspector,
                        label { "Draft"
                            input { aria_label: "Dock draft", value: draft(), oninput: move |e| draft.set(e.value()) }
                        }
                        Popover { is_modal: false,
                            PopoverTrigger { "Open details" }
                            PopoverContent { aria_label: "Dock details", Button { "Details action" } }
                        }
                        for index in 0..40 { Button { "Inspector action {index}" } }
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    dioxus_web::launch::launch_virtual_dom(VirtualDom::new(fixture), dioxus_web::Config::new());
}

#[cfg(not(target_arch = "wasm32"))]
fn main() { panic!("the dock fixture requires wasm32-unknown-unknown"); }

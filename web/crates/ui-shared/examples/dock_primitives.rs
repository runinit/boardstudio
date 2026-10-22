//! Standalone mounted fixture for ticket 009; never launched by the production shell.
#[cfg(target_arch = "wasm32")]
use boardstudio_web_ui_shared::{
    panels::{PanelSide, use_panel_settings},
    ui::{
        button::Button,
        popover::{Popover, PopoverContent, PopoverTrigger},
        select::{Select, SelectOption},
        sheet::{Sheet, SheetClose, SheetContent, SheetDirection, SheetTrigger},
        sidenav::Sidenav,
    },
};
#[cfg(target_arch = "wasm32")]
use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
fn fixture() -> Element {
    let warning = use_signal(|| false);
    let objects = use_panel_settings(PanelSide::Objects, warning);
    let inspector = use_panel_settings(PanelSide::Inspector, warning);
    let mut mounted = use_signal(|| true);
    let mut clicks = use_signal(|| 0);
    let mut direction = use_signal(|| SheetDirection::Bottom);
    let mut open = use_signal(|| false);
    rsx! {
        header { style: "height:56px;display:flex;gap:8px;align-items:center",
            Button { onclick: move |_| mounted.toggle(), "Remount docks" }
            Button { onclick: move |_| open.set(false), "External close" }
            for edge in [SheetDirection::Top, SheetDirection::Bottom, SheetDirection::Left, SheetDirection::Right] {
                Button { onclick: move |_| direction.set(edge), "{edge:?}" }
            }
        }
        main { style: "display:flex;gap:16px;height:540px",
            if mounted() {
                Sidenav { side: PanelSide::Objects, settings: objects, aria_label: "Objects", "Objects content" }
            }
            section { style: "flex:1;min-width:0",
                Button { onclick: move |_| clicks += 1, "Canvas action" }
                output { role: "status", aria_label: "Canvas clicks", "{clicks}" }
                Sheet { open,
                    SheetTrigger { "Open settings" }
                    SheetContent { direction: direction(), aria_label: "Dock settings",
                        p { "The canvas remains interactive." }
                        Button { onclick: move |_| {}, "Sheet action" }
                        SheetClose { "Close settings" }
                        Select::<String> { aria_label: "Sheet choice",
                            SelectOption::<String> { index: 0usize, value: "one", "One" }
                            SelectOption::<String> { index: 1usize, value: "two", "Two" }
                        }
                        Popover { is_modal: false,
                            PopoverTrigger { "Open details" }
                            PopoverContent { aria_label: "Sheet details",
                                Button { onclick: move |_| {}, "Details action" }
                            }
                        }
                    }
                }
            }
            if mounted() {
                Sidenav { side: PanelSide::Inspector, settings: inspector, aria_label: "Inspector", "Inspector content" }
            }
        }
        output { role: "status", aria_label: "Storage warning", "{warning}" }
        div { style: "height:1200px", "Scrollable page" }
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    dioxus_web::launch::launch_virtual_dom(VirtualDom::new(fixture), dioxus_web::Config::new());
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    panic!("the dock fixture requires wasm32-unknown-unknown");
}

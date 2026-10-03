//! Shared empty-board entry; Session and the existing setup/Parts owners own actions.
use dioxus::prelude::*;

#[component]
pub(super) fn EmptyBoardCanvas(
    editable: bool,
    on_matrix: EventHandler<()>,
    on_parts: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "m1-canvas-empty",
            div { class: "m1-empty-cursor", aria_hidden: "true",
                svg { view_box: "0 0 24 24",
                    path { d: "M5 3l14 9-6 1-3 6z" }
                }
            }
            h2 { "Build your keyboard layout" }
            p { "Start with a key matrix. Its switches and companions create the board outline as you edit." }
            button { type: "button", class: "m1-empty-primary", disabled: !editable,
                onclick: move |_| on_matrix.call(()),
                "Add key matrix",
                svg { view_box: "0 0 24 24", aria_hidden: "true",
                    path { d: "M5 12h14m-6-6 6 6-6 6" }
                }
            }
            button { type: "button", class: "m1-empty-secondary",
                onclick: move |_| on_parts.call(()), "Browse individual parts"
            }
        }
    }
}

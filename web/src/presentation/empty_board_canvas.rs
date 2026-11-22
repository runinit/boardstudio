//! Shared empty-board entry; Session and the existing setup/Parts owners own actions.
use boardstudio_web_ui_shared::ui::{
    button::{Button, ButtonVariant},
    empty::{Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyTitle},
};
use dioxus::prelude::*;

#[component]
pub(super) fn EmptyBoardCanvas(
    editable: bool,
    on_matrix: EventHandler<()>,
    on_parts: EventHandler<()>,
) -> Element {
    rsx! {
        Empty { class: "shell-empty",
            EmptyHeader {
                EmptyTitle { "Build your keyboard layout" }
                EmptyDescription { "Start with a key matrix. Its switches and companions create the board outline as you edit." }
            }
            EmptyContent { class: "w-full items-stretch",
                Button { class: "h-11 min-w-0 flex-1 whitespace-normal", disabled: !editable, onclick: move |_| on_matrix.call(()), "Add key matrix" }
                Button { variant: ButtonVariant::Secondary, class: "h-11 min-w-0 flex-1 whitespace-normal", onclick: move |_| on_parts.call(()), "Browse parts" }
            }
        }
    }
}

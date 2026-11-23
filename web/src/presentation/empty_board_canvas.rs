//! Shared empty-board entry; Session and the existing setup/Parts owners own actions.
use boardstudio_application::{Durability, Lifecycle};
use boardstudio_web_ui_shared::ui::{
    button::{Button, ButtonVariant},
    empty::{Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyTitle},
};
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub(super) fn EmptyBoardCanvas(
    revision: u64,
    on_matrix: EventHandler<()>,
    on_parts: EventHandler<()>,
) -> Element {
    let runtime = use_context::<Rc<crate::runtime::Runtime>>();
    let _ = try_use_context::<Signal<u64>>().map(|v| v());
    let _ = try_use_context::<boardstudio_web_ui_shared::DurabilityVersion>().map(|d| d.0());
    let editable = runtime.with_model(|model| {
        model.lifecycle == Lifecycle::Ready
            && matches!(model.durability, Durability::Saved { revision: saved } | Durability::Saving { revision: saved } if saved == revision)
            && model.gesture.is_none()
            && model.display_preview.is_none()
    });
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

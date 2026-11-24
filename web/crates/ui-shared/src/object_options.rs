//! The Objects panel's tree-grouping option, shared by the panel chrome and the tree.
use crate::ui::select::{Select, SelectOption};
use boardstudio_web_ui_model::tree::Grouping;
use dioxus::prelude::*;

#[derive(Clone, Copy)]
pub struct ObjectTreePreferences {
    pub grouping: Signal<Grouping>,
}

pub fn use_object_options() -> Element {
    let grouping = use_signal(|| Grouping::from_storage(read_tree_grouping()));
    use_context_provider(|| ObjectTreePreferences { grouping });
    rsx! { ObjectOptions { grouping } }
}

#[component]
fn ObjectOptions(mut grouping: Signal<Grouping>) -> Element {
    let selected = use_memo(move || {
        Some(
            if grouping() == Grouping::Row {
                "row"
            } else {
                "column"
            }
            .to_string(),
        )
    });
    rsx! {
        div { class: "grid gap-1",
            span { "Group objects" }
            Select {
                aria_label: "Tree grouping",
                value: Some(selected.into()),
                on_value_change: move |value: Option<String>| {
                    let Some(value) = value else { return; };
                    let next = Grouping::from_storage(Some(value));
                    grouping.set(next);
                    write_tree_grouping(next);
                },
                SelectOption { index: 0usize, value: "column", text_value: "Columns", "Columns" }
                SelectOption { index: 1usize, value: "row", text_value: "Rows", "Rows" }
            }
        }
    }
}

fn read_tree_grouping() -> Option<String> {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| {
            storage
                .get_item("boardstudio:v2:tree-grouping")
                .ok()
                .flatten()
        })
}

fn write_tree_grouping(grouping: Grouping) {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.set_item(
            "boardstudio:v2:tree-grouping",
            if grouping == Grouping::Row {
                "row"
            } else {
                "column"
            },
        );
    }
}

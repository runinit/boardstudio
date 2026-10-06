//! The Objects panel's tree-grouping option, shared by the panel chrome and the tree.
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
    rsx! {
        label { "Group objects"
            select {
                "aria-label": "Tree grouping",
                value: if grouping() == Grouping::Row { "row" } else { "column" },
                onchange: move |event: FormEvent| {
                    let next = Grouping::from_storage(Some(event.value()));
                    grouping.set(next);
                    write_tree_grouping(next);
                },
                option { value: "column", "Columns" }
                option { value: "row", "Rows" }
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

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
            Select::<String> {
                aria_label: "Tree grouping",
                value: Some(selected.into()),
                on_value_change: move |value: Option<String>| {
                    let Some(value) = value else { return; };
                    let next = Grouping::from_storage(Some(value));
                    grouping.set(next);
                    write_tree_grouping(next);
                },
                SelectOption::<String> { index: 0usize, value: "column", text_value: "Columns", "Columns" }
                SelectOption::<String> { index: 1usize, value: "row", text_value: "Rows", "Rows" }
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

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use crate::test_mount::MountedFixture;
    use crate::{test_role::by_role_name, test_wait::wait_until};

    fn fixture() -> Element {
        let grouping = use_signal(|| Grouping::Column);
        rsx! { ObjectOptions { grouping } }
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn grouping_choice_updates_and_persists() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let _mounted = MountedFixture::new(root.clone(), VirtualDom::new(fixture));
        wait_until("grouping picker mounts", || {
            by_role_name(&root, "button", "Tree grouping").is_some()
        })
        .await;
        let trigger = by_role_name(&root, "button", "Tree grouping").unwrap();
        trigger.click();
        wait_until("grouping choices open", || {
            by_role_name(&root, "option", "Rows").is_some()
        })
        .await;
        let option = by_role_name(&root, "option", "Rows").unwrap();
        for phase in ["pointerdown", "pointerup"] {
            let event = web_sys::PointerEvent::new(phase).unwrap();
            event.init_event_with_bubbles_and_cancelable(phase, true, true);
            option.dispatch_event(&event).unwrap();
        }
        wait_until("grouping updates", || {
            trigger.text_content().as_deref() == Some("Rows")
        })
        .await;
        assert_eq!(read_tree_grouping().as_deref(), Some("row"));
        web_sys::window()
            .unwrap()
            .local_storage()
            .unwrap()
            .unwrap()
            .remove_item("boardstudio:v2:tree-grouping")
            .unwrap();
        root.remove();
    }
}

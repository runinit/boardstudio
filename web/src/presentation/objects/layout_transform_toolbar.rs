//! Transform-property navigation for the Layout command pill.
use super::layout_toolbar::LayoutSelectionKind;
use super::layout_toolbar::{LayoutCommandMenu, close_layout_command_menu};
use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub(in crate::presentation) struct LayoutTransformMenuMount {
    pub properties_available: bool,
    pub column_available: bool,
    pub row_available: bool,
    pub on_selection_kind: EventHandler<LayoutSelectionKind>,
    pub on_show_properties: EventHandler<()>,
}

#[component]
pub(in crate::presentation) fn LayoutTransformToolbar(
    mount: LayoutTransformMenuMount,
    open_menu: Signal<Option<LayoutCommandMenu>>,
) -> Element {
    let is_open = open_menu() == Some(LayoutCommandMenu::Transform);
    let position_label = "Position & rotation";

    rsx! {
        details {
            class: "m1-layout-command-menu m1-layout-transform-menu",
            "data-layout-menu": "transform",
            open: is_open,
            onkeydown: move |event: KeyboardEvent| {
                if event.data().key().to_string() == "Escape" && open_menu() == Some(LayoutCommandMenu::Transform) {
                    event.prevent_default();
                    event.stop_propagation();
                    close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
                }
            },
            summary {
                id: "m1-layout-transform-trigger",
                "aria-controls": "m1-layout-transform-menu",
                "aria-expanded": "{is_open}",
                onclick: move |event: MouseEvent| {
                    event.prevent_default();
                    let mut open_menu = open_menu;
                    open_menu.set((open_menu() != Some(LayoutCommandMenu::Transform)).then_some(LayoutCommandMenu::Transform));
                },
                "Transform"
            }
            div { id: "m1-layout-transform-menu", class: "m1-layout-transform-popover",
                if mount.properties_available {
                    button {
                        r#type: "button",
                        "data-close-menu": "true",
                        onclick: move |_| {
                            mount.on_show_properties.call(());
                            close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
                        },
                        "{position_label}"
                    }
                } else {
                    button { r#type: "button", disabled: true, "{position_label}" }
                }
                button {
                    r#type: "button",
                    disabled: !mount.column_available,
                    "data-close-menu": "true",
                    onclick: move |_| {
                        mount.on_selection_kind.call(LayoutSelectionKind::Column);
                        close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
                    },
                    "Splay & origin"
                }
                button {
                    r#type: "button",
                    disabled: !mount.row_available,
                    "data-close-menu": "true",
                    onclick: move |_| {
                        mount.on_selection_kind.call(LayoutSelectionKind::Row);
                        close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
                    },
                    "Row offsets"
                }
                if !mount.properties_available {
                    p { "Select a matrix, row, column, key or part to open its current transform properties." }
                }
            }
        }
    }
}

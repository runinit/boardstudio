//! Shared empty-board entry; Session and the existing setup/Parts owners own actions.
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
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
            div { class: "m1-empty-actions",
                Button { class: "h-11 min-w-0 flex-1 whitespace-normal", disabled: !editable,
                    onclick: move |_| on_matrix.call(()),
                    "Add key matrix",
                    svg { view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.5", "aria-hidden": "true",
                        path { d: "M5 12h14m-6-6 6 6-6 6" }
                    }
                }
                Button { variant: ButtonVariant::Secondary, class: "h-11 min-w-0 flex-1 whitespace-normal",
                    onclick: move |_| on_parts.call(()), "Browse parts"
                }
            }
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use boardstudio_web_ui_shared::test_role::by_role_name;
    use boardstudio_web_ui_shared::test_wait::wait_until;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn fixture() -> Element {
        let mut matrix = use_signal(|| 0);
        let mut parts = use_signal(|| 0);
        rsx! {
            EmptyBoardCanvas {
                editable: matrix() == 0,
                on_matrix: move |_| matrix += 1,
                on_parts: move |_| parts += 1,
            }
            output { id: "choices", "{matrix()},{parts()}" }
        }
    }

    #[wasm_bindgen_test]
    async fn empty_board_actions_are_found_by_role_and_name() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("the empty board actions render", || {
            by_role_name(&root, "button", "Browse parts").is_some()
        })
        .await;
        let choices = || {
            root.query_selector("#choices")
                .unwrap()
                .unwrap()
                .text_content()
        };
        by_role_name(&root, "button", "Browse parts")
            .unwrap()
            .click();
        wait_until("Browse parts reaches its owner", || {
            choices().as_deref() == Some("0,1")
        })
        .await;
        let matrix = by_role_name(&root, "button", "Add key matrix").unwrap();
        assert_eq!(matrix.get_attribute("data-slot").as_deref(), Some("button"));
        matrix.click();
        wait_until(
            "Add key matrix reaches its owner and disables itself",
            || {
                choices().as_deref() == Some("1,1")
                    && by_role_name(&root, "button", "Add key matrix")
                        .is_some_and(|button| button.has_attribute("disabled"))
            },
        )
        .await;
        root.remove();
    }
}

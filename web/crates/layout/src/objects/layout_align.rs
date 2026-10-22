//! Private Layout Align toolbar controls.
pub use super::layout_align_geometry::{
    AlignCommand, alignment_delta, local_matrix_delta, reconcile_reference_choice,
    transformed_envelope,
};
use super::layout_toolbar::{LayoutCommandMenu, close_layout_command_menu};
use super::tree::TreeContext;
use boardstudio_application::Scope;
use boardstudio_web_ui_shared::ui::{
    button::{Button, ButtonSize, ButtonVariant},
    callout::Callout,
    combobox::{Combobox, ComboboxOption},
    label::Label,
    select::{Select, SelectOption},
    toast::{ToastOptions, use_toast},
};
use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlignReference {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlignFeedback {
    pub reference: String,
    pub command: AlignCommand,
    pub message: String,
    pub succeeded: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlignAction {
    pub workspace: &'static str,
    pub scope: Scope,
    pub scope_generation: u64,
    pub context: TreeContext,
    pub moving_ids: Vec<String>,
    pub reference_id: String,
    pub command: AlignCommand,
}

#[derive(Clone, PartialEq)]
pub struct LayoutAlignMount {
    pub references: Vec<AlignReference>,
    pub selected_reference: Option<String>,
    pub action: Option<AlignAction>,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
    pub busy: bool,
    pub feedback: Option<AlignFeedback>,
    pub on_reference: EventHandler<String>,
    pub on_align: EventHandler<AlignAction>,
}

#[component]
pub fn LayoutAlignToolbar(
    mount: LayoutAlignMount,
    open_menu: Signal<Option<LayoutCommandMenu>>,
    show_relationships: bool,
    has_selection_context: bool,
    on_show_relationships: EventHandler<()>,
) -> Element {
    let on_align = mount.on_align;
    let selected_reference = use_memo(use_reactive((&mount.selected_reference,), |(value,)| value));
    let toast = use_toast();
    use_effect(use_reactive((&mount.feedback,), move |(feedback,)| {
        if let Some(feedback) = feedback {
            if feedback.succeeded {
                toast.success(feedback.message, ToastOptions::new());
            } else {
                toast.error(feedback.message, ToastOptions::new());
            }
        }
    }));
    rsx! {
        super::layout_toolbar::LayoutCommandPopup {
            open_menu, menu: LayoutCommandMenu::Align, label: "Align",
            trigger: rsx! { span { "Align" } },
            Label { "Reference part" }
            if mount.references.len() > 12 {
                Combobox::<String> {
                    aria_label: "Alignment reference", list_aria_label: "Alignment reference",
                    value: Some(selected_reference.into()),
                    on_value_change: move |value: Option<String>| if let Some(value) = value { mount.on_reference.call(value); },
                    for (index, part) in mount.references.iter().enumerate() {
                        ComboboxOption::<String> { index, value: part.id.clone(), text_value: part.label.clone(), "{part.label}" }
                    }
                }
            } else {
                Select::<String> {
                    aria_label: "Alignment reference", value: Some(selected_reference.into()), disabled: mount.references.is_empty(),
                    on_value_change: move |value: Option<String>| if let Some(value) = value { mount.on_reference.call(value); },
                    for (index, part) in mount.references.iter().enumerate() {
                        SelectOption::<String> { index, value: part.id.clone(), text_value: part.label.clone(), "{part.label}" }
                    }
                }
            }
            if let Some(reason) = mount.disabled_reason.as_ref() { Callout { "{reason}" } }
            div { class: "layout-alignment-actions", role: "group", aria_label: "Align selection",
                for command in AlignCommand::ALL {
                    Button {
                        key: "{command.label()}", size: ButtonSize::Sm, variant: ButtonVariant::Secondary,
                        disabled: !mount.enabled || mount.busy || mount.selected_reference.is_none(),
                        onclick: {
                            let action = mount.action.as_ref().map(|action| AlignAction { command, ..action.clone() });
                            move |_| if let Some(action) = action.clone() {
                                on_align.call(action);
                                close_layout_command_menu(open_menu, LayoutCommandMenu::Align);
                            }
                        },
                        "{command.label()}"
                    }
                }
            }
            if show_relationships {
                Button {
                    size: ButtonSize::Sm, variant: ButtonVariant::Secondary, disabled: !has_selection_context,
                    onclick: move |_| { on_show_relationships.call(()); close_layout_command_menu(open_menu, LayoutCommandMenu::Align); },
                    "Relationships"
                }
            }
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[component]
    fn align_host() -> Element {
        let open_menu = use_signal(|| Some(LayoutCommandMenu::Align));
        let reference = use_signal(|| "reference".to_owned());
        let mut feedback = use_signal(|| None::<AlignFeedback>);
        rsx! {
            boardstudio_web_ui_shared::ui::toast::ToastProvider {
            LayoutAlignToolbar {
                mount: LayoutAlignMount {
                    references: vec![AlignReference { id: "reference".into(), label: "Reference switch".into() }],
                    selected_reference: Some(reference()), action: None, enabled: false,
                    disabled_reason: Some("Select a part to align.".into()), busy: false, feedback: feedback(),
                    on_reference: EventHandler::new(move |_| {}), on_align: EventHandler::new(move |_| {}),
                },
                open_menu, show_relationships: false, has_selection_context: false,
                on_show_relationships: move |_| {},
            }
            for (label, succeeded) in [("Alignment complete", true), ("Alignment failed", false)] {
                Button { onclick: move |_| feedback.set(Some(AlignFeedback { reference: "reference".into(), command: AlignCommand::Left, message: label.into(), succeeded })), "{label}" }
            }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn alignment_results_announce_toasts_while_unavailability_stays_inline() {
        use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_cfg(
            align_host,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("align renders", || {
            by_role_name(&root, "button", "Alignment complete").is_some()
        })
        .await;
        let menu = by_role_name(&root, "dialog", "Align").unwrap();
        assert!(
            menu.query_selector("[data-slot=callout]")
                .unwrap()
                .unwrap()
                .text_content()
                .unwrap()
                .contains("Select a part to align.")
        );
        for message in ["Alignment complete", "Alignment failed"] {
            by_role_name(&root, "button", message).unwrap().click();
            wait_until("alignment result is announced", || {
                let statuses = root.query_selector_all("[role=status]").unwrap();
                (0..statuses.length())
                    .filter_map(|i| statuses.item(i))
                    .any(|status| status.text_content().unwrap_or_default().contains(message))
            })
            .await;
        }
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn alignment_popover_exposes_reference_and_restores_focus_on_escape() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_cfg(
            align_host,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(80).await;
        use boardstudio_web_ui_shared::test_role::by_role_name;
        let menu = by_role_name(&root, "dialog", "Align").unwrap();
        assert_eq!(menu.get_attribute("role").as_deref(), Some("dialog"));
        assert!(by_role_name(&menu, "button", "Alignment reference").is_some());
        assert!(
            by_role_name(&menu, "button", "Left")
                .unwrap()
                .has_attribute("disabled")
        );
        let event = web_sys::KeyboardEventInit::new();
        event.set_key("Escape");
        event.set_bubbles(true);
        menu.dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &event).unwrap(),
        )
        .unwrap();
        gloo_timers::future::TimeoutFuture::new(80).await;
        assert!(by_role_name(&root, "dialog", "Align").is_none());
        assert_eq!(
            document.active_element().unwrap().id(),
            "m1-layout-align-trigger"
        );
        by_role_name(&root, "button", "Align").unwrap().click();
        gloo_timers::future::TimeoutFuture::new(80).await;
        assert!(by_role_name(&root, "dialog", "Align").is_some());
        root.remove();
    }
}

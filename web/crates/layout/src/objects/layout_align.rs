//! Private Layout Align toolbar controls.
pub use super::layout_align_geometry::{
    AlignCommand, alignment_delta, reconcile_reference_choice, transformed_envelope,
};
use super::layout_toolbar::{LayoutCommandMenu, close_layout_command_menu};
use super::tree::TreeContext;
use boardstudio_application::Scope;
use boardstudio_web_ui_shared::controls::{
    BoardButton, BoardButtonDensity, BoardPopover, BoardSelectField,
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
    let is_open = open_menu() == Some(LayoutCommandMenu::Align);
    rsx! {
        BoardPopover {
            open: is_open,
            on_open_change: move |next: bool| {
                let mut open_menu = open_menu;
                if next {
                    open_menu.set(Some(LayoutCommandMenu::Align));
                } else if open_menu() == Some(LayoutCommandMenu::Align) {
                    open_menu.set(None);
                }
            },
            trigger_id: "m1-layout-align-trigger",
            content_id: "m1-layout-align-menu",
            trigger_label: "Align",
            root_class: "m1-layout-command-menu m1-layout-align-menu",
            menu_kind: "align",
            content_class: "m1-layout-align-popover grid gap-1",
            content_role: "group",
            content_label: "Alignment tools",
            trigger: rsx! {
                svg { class: "m1-layout-command-trigger-icon", view_box: "0 0 20 20", "aria-hidden": "true",
                    path { d: "M3 2v16M6 4h10v4H6zM6 12h6v4H6z" }
                }
                span { class: "layout-action-label", "Align" }
            },
            super::layout_toolbar::LayoutCommandMenuHeader {
                label: "Align".to_owned(),
                close_label: "Close Align".to_owned(),
                open_menu,
                menu: LayoutCommandMenu::Align,
            }
            BoardSelectField {
                label: "Reference part",
                aria_label: "Alignment reference",
                value: mount.selected_reference.clone().unwrap_or_default(),
                disabled: mount.references.is_empty(),
                onchange: move |event: FormEvent| mount.on_reference.call(event.value()),
                if mount.references.is_empty() {
                    option { value: "", disabled: true, "No independent reference parts" }
                }
                for part in &mount.references {
                    option { key: "{part.id}", value: "{part.id}", "{part.label}" }
                }
            }
            if let Some(reason) = mount.disabled_reason.as_ref() {
                p { class: "m1-layout-align-status", role: "status", "{reason}" }
            }
            div { class: "m1-layout-align-actions", role: "group", aria_label: "Align selection",
                for command in AlignCommand::ALL {
                    BoardButton {
                        key: "{command.label()}",
                        density: BoardButtonDensity::Compact,
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
            if let Some(feedback) = mount.feedback.as_ref() {
                p {
                    title: "{feedback.command.label()} alignment",
                    class: if feedback.succeeded { "m1-layout-align-feedback" } else { "m1-layout-align-feedback is-error" },
                    role: if feedback.succeeded { "status" } else { "alert" },
                    "{feedback.message}"
                }
            }
            if show_relationships {
                BoardButton {
                    density: BoardButtonDensity::Compact,
                    disabled: !has_selection_context,
                    onclick: move |_| {
                        on_show_relationships.call(());
                        close_layout_command_menu(open_menu, LayoutCommandMenu::Align);
                    },
                    "Relationships"
                }
            }
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[component]
    fn align_host() -> Element {
        let open_menu = use_signal(|| Some(LayoutCommandMenu::Align));
        let reference = use_signal(|| "reference".to_owned());
        rsx! {
            LayoutAlignToolbar {
                mount: LayoutAlignMount {
                    references: vec![AlignReference { id: "reference".into(), label: "Reference switch".into() }],
                    selected_reference: Some(reference()), action: None, enabled: false,
                    disabled_reason: Some("Select a part to align.".into()), busy: false, feedback: None,
                    on_reference: EventHandler::new(move |_| {}), on_align: EventHandler::new(move |_| {}),
                },
                open_menu, show_relationships: false, has_selection_context: false,
                on_show_relationships: move |_| {},
            }
        }
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
        let menu = root
            .query_selector("#m1-layout-align-menu")
            .unwrap()
            .unwrap();
        assert_eq!(menu.get_attribute("role").as_deref(), Some("group"));
        assert!(
            menu.query_selector("select[aria-label='Alignment reference']")
                .unwrap()
                .is_some()
        );
        assert!(menu.query_selector("button:disabled").unwrap().is_some());
        let event = web_sys::KeyboardEventInit::new();
        event.set_key("Escape");
        event.set_bubbles(true);
        menu.dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &event).unwrap(),
        )
        .unwrap();
        gloo_timers::future::TimeoutFuture::new(80).await;
        assert!(
            root.query_selector("#m1-layout-align-menu")
                .unwrap()
                .is_none()
        );
        assert_eq!(
            document.active_element().unwrap().id(),
            "m1-layout-align-trigger"
        );
        root.query_selector("#m1-layout-align-trigger")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        gloo_timers::future::TimeoutFuture::new(80).await;
        assert!(
            root.query_selector("#m1-layout-align-menu")
                .unwrap()
                .is_some()
        );
        root.remove();
    }
}

//! Private Layout Align toolbar controls.
pub use super::layout_align_geometry::{
    AlignCommand, alignment_delta, reconcile_reference_choice, transformed_envelope,
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
                        disabled: !mount.enabled || mount.selected_reference.is_none(),
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

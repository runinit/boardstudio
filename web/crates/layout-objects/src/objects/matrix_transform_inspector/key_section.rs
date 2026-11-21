//! Key section of the transform Inspector: enable, omit, assembly, components.
use super::*;

#[derive(Props, Clone, PartialEq)]
pub(super) struct EnabledTransformFieldProps {
    pub owner: MatrixTransformInspectorOwner,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub value: bool,
    pub request_sequence: Signal<u64>,
    pub editable: bool,
    pub feedback: Vec<MatrixTransformFeedback>,
    pub on_edit: EventHandler<MatrixTransformRequest>,
    pub splay_affect: Signal<MatrixSplayAffect>,
}

#[component]
pub(super) fn EnabledTransformField(props: EnabledTransformFieldProps) -> Element {
    let mut submitted = use_signal(|| None::<u64>);
    let error = props
        .feedback
        .iter()
        .rev()
        .find(|item| {
            item.owner == props.owner
                && item.field == MatrixTransformField::KeyEnabled
                && submitted() == Some(item.request_id)
                && item.message.is_some()
        })
        .and_then(|item| item.message.clone());
    let disabled = !props.editable;
    let mut sequence = props.request_sequence;
    let owner = props.owner.clone();
    let current = props.value;
    let token = props.snapshot_token;
    let revision = props.revision;
    let on_edit = props.on_edit;
    let affect = props.splay_affect;
    let check_id = format!(
        "layout-check-{}-{}",
        dioxus::core::current_scope_id().0,
        ("Key enabled").replace(' ', "-"),
    );
    let on_checked_change = EventHandler::new(move |checked: CheckboxState| {
        let Some(id) = sequence().checked_add(1) else {
            return;
        };
        sequence.set(id);
        submitted.set(Some(id));
        on_edit.call(MatrixTransformRequest {
            owner: owner.clone(),
            request_id: id,
            snapshot_token: token,
            revision,
            field: MatrixTransformField::KeyEnabled,
            baseline: MatrixTransformValue::Bool(current),
            value: MatrixTransformValue::Bool(bool::from(checked)),
            splay_affect: affect(),
            draft: None,
            failure: None,
            submitted_text: None,
            one_shot: false,
        });
    });
    rsx! {
        TransformCheckRow {
            id: check_id,
            label: "Enabled",
            aria_label: "Key enabled",
            checked: current,
            disabled,
            error,
            on_checked_change,
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub(super) struct OmitTransformFieldProps {
    pub owner: MatrixTransformInspectorOwner,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub omitted: bool,
    pub request_sequence: Signal<u64>,
    pub editable: bool,
    pub feedback: Vec<MatrixTransformFeedback>,
    pub on_edit: EventHandler<MatrixTransformRequest>,
    pub splay_affect: Signal<MatrixSplayAffect>,
}

/// Ticket 34: per-key Omit/Rejoin for a mirrored key. Omitting hides the key
/// from the matrix surface but keeps its logical pose; clearing the omission
/// rejoins the key. One Undo entry either way.
#[component]
pub(super) fn OmitTransformField(props: OmitTransformFieldProps) -> Element {
    let mut submitted = use_signal(|| None::<u64>);
    let error = props
        .feedback
        .iter()
        .rev()
        .find(|item| {
            item.owner == props.owner
                && item.field == MatrixTransformField::KeyOmit
                && submitted() == Some(item.request_id)
                && item.message.is_some()
        })
        .and_then(|item| item.message.clone());
    let disabled = !props.editable;
    let mut sequence = props.request_sequence;
    let owner = props.owner.clone();
    let current = props.omitted;
    let token = props.snapshot_token;
    let revision = props.revision;
    let on_edit = props.on_edit;
    let affect = props.splay_affect;
    let on_checked_change = EventHandler::new(move |next: CheckboxState| {
        let Some(id) = sequence().checked_add(1) else {
            return;
        };
        sequence.set(id);
        submitted.set(Some(id));
        on_edit.call(MatrixTransformRequest {
            owner: owner.clone(),
            request_id: id,
            snapshot_token: token,
            revision,
            field: MatrixTransformField::KeyOmit,
            baseline: MatrixTransformValue::Bool(current),
            value: MatrixTransformValue::Bool(next == CheckboxState::Checked),
            splay_affect: affect(),
            draft: None,
            failure: None,
            submitted_text: None,
            one_shot: false,
        });
    });
    rsx! {
        TransformCheckRow {
            id: "matrix-key-omit",
            label: "Omit from mirror",
            aria_label: "Omit from mirror",
            checked: current,
            disabled,
            help: "Hides the key from the matrix surface but keeps its logical pose, in one Undo entry. Clearing the omission rejoins the key.",
            error,
            on_checked_change,
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub(super) struct KeyAssemblyFieldProps {
    pub owner: MatrixTransformInspectorOwner,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub value: String,
    pub choices: Vec<(String, String)>,
    pub request_sequence: Signal<u64>,
    pub editable: bool,
    pub feedback: Vec<MatrixTransformFeedback>,
    pub on_edit: EventHandler<MatrixTransformRequest>,
    pub splay_affect: Signal<MatrixSplayAffect>,
}

#[component]
pub(super) fn KeyAssemblyField(props: KeyAssemblyFieldProps) -> Element {
    let assembly_choice = use_memo(use_reactive((&props.value,), |(value,)| Some(value)));
    let mut submitted = use_signal(|| None::<u64>);
    let error = props
        .feedback
        .iter()
        .rev()
        .find(|item| {
            item.owner == props.owner
                && item.field == MatrixTransformField::KeyAssembly
                && submitted() == Some(item.request_id)
                && item.message.is_some()
        })
        .and_then(|item| item.message.clone());
    let disabled = !props.editable;
    let mut sequence = props.request_sequence;
    let owner = props.owner.clone();
    let current = props.value.clone();
    let token = props.snapshot_token;
    let revision = props.revision;
    let on_edit = props.on_edit;
    let affect = props.splay_affect;
    rsx! {
        {
            let control_label = "Key Assembly".to_owned();
            let control_aria_label = "Key Assembly".to_owned();
            let control_id = format!(
                "layout-choice-{}-{}",
                dioxus::core::current_scope_id().0,
                control_aria_label.replace(' ', "-"),
            );

            let mut control_choices: Vec<(String, String, bool)> = Vec::new();
            for (id, label) in props.choices.iter() {
                control_choices.push((id.to_string(), label.to_string(), false));
            }

            let on_value_change = EventHandler::new(move |choice: Option<String>| {
        let Some(choice) = choice else {
            return;
        };
        let Some(id) = sequence().checked_add(1) else {
            return;
        };
        sequence.set(id);
        submitted.set(Some(id));
        on_edit.call(MatrixTransformRequest {
            owner: owner.clone(),
            request_id: id,
            snapshot_token: token,
            revision,
            field: MatrixTransformField::KeyAssembly,
            baseline: MatrixTransformValue::Text(current.clone()),
            value: MatrixTransformValue::Text(choice),
            splay_affect: affect(),
            draft: None,
            failure: None,
            submitted_text: None,
            one_shot: false,
        });
    });
    rsx! {

                Field {
                    FieldLabel { html_for: control_id.clone(), "{control_label}" }
                    Combobox::<String> {
                        id: control_id,
                        aria_label: control_aria_label.clone(),
                        list_aria_label: control_aria_label.clone(),
                        value: Some(assembly_choice.into()),
                        disabled,
                        on_value_change,
                        for (index, (value, label, disabled)) in control_choices.iter().enumerate() {
                            ComboboxOption::<String> {
                                index,
                                value: value.clone(),
                                text_value: label.clone(),
                                disabled: *disabled,
                                "{label}"
                            }
                        }
                    }
                }
            }
        }
        if let Some(message) = error.as_deref() {
            FieldError { "{message}"  }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub(super) struct AttachedComponentsFieldProps {
    pub owner: MatrixTransformInspectorOwner,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub value: Vec<(String, String)>,
    pub choices: Vec<boardstudio_model::model::PartDefinition>,
    pub mirror_target: bool,
    pub assemblies_local: bool,
    pub request_sequence: Signal<u64>,
    pub editable: bool,
    pub remove_disabled: Signal<bool>,
    pub mirror_reset_disabled: Signal<bool>,
    pub feedback: Vec<MatrixTransformFeedback>,
    pub on_edit: EventHandler<MatrixTransformRequest>,
    pub splay_affect: Signal<MatrixSplayAffect>,
}

#[component]
pub(super) fn AttachedComponentsField(props: AttachedComponentsFieldProps) -> Element {
    let attachment_choices = use_memo(use_reactive((&props.value,), |(values,)| {
        values
            .into_iter()
            .map(|(assembly, definition)| (assembly, Some(definition)))
            .collect::<std::collections::BTreeMap<_, _>>()
    }));
    let mut submitted = use_signal(|| None::<u64>);
    let error = props
        .feedback
        .iter()
        .rev()
        .find(|item| {
            item.owner == props.owner
                && item.field == MatrixTransformField::KeyAttached
                && submitted() == Some(item.request_id)
                && item.message.is_some()
        })
        .and_then(|item| item.message.clone());
    let disabled = !props.editable;
    let remove_disabled = disabled || (props.remove_disabled)();
    let sequence = props.request_sequence;
    let owner = props.owner.clone();
    let token = props.snapshot_token;
    let revision = props.revision;
    let on_edit = props.on_edit;
    let affect = props.splay_affect;
    let current = props.value.clone();
    let send = move |next: Vec<(String, String)>, one_shot: bool| {
        let mut sequence = sequence;
        let Some(id) = sequence().checked_add(1) else {
            return;
        };
        sequence.set(id);
        submitted.set(Some(id));
        on_edit.call(MatrixTransformRequest {
            owner: owner.clone(),
            request_id: id,
            snapshot_token: token,
            revision,
            field: MatrixTransformField::KeyAttached,
            baseline: MatrixTransformValue::Attached(current.clone()),
            value: MatrixTransformValue::Attached(next),
            splay_affect: affect(),
            draft: None,
            failure: None,
            submitted_text: None,
            one_shot,
        });
    };
    rsx! {
            h3 { "Attached components" }
            if props.mirror_target {
                if props.assemblies_local {
                    ResetTransformButton {
                        label: "Use mirrored components",
                        owner: props.owner.clone(),
                        snapshot_token: props.snapshot_token,
                        revision: props.revision,
                        field: MatrixTransformField::KeyAssembliesLocal,
                        pending_disabled: props.mirror_reset_disabled,
                        baseline: MatrixTransformValue::Bool(true),
                        value: MatrixTransformValue::Bool(false),
                        request_sequence: props.request_sequence,
                        editable: props.editable,
                        feedback: props.feedback.clone(),
                        splay_affect: props.splay_affect,
                        on_edit: props.on_edit,
                    }
                } else {
                    Callout { variant: CalloutVariant::Info,
                        "The key assembly and attached components follow the paired half. Replacing one here keeps this key local."
                    }
                }
            }
            if props.value.is_empty() {
                Callout { variant: CalloutVariant::Info, "No attached components. Apply a component in Parts." }
            }
            for (assembly_id, definition_id) in props.value.iter().cloned() {
                {
                    let choices = crate::matrix_transform_operation::attachment_component_choices(
                        &props.choices,
                        &definition_id,
                    );
                    rsx! {
                        div { key: "{assembly_id}",
                            {
                                let control_label = format!("Replace {}", assembly_id);
                                let control_aria_label = format!("Replace {}", assembly_id);
                                let control_id = format!(
                                    "layout-choice-{}-{}",
                                    dioxus::core::current_scope_id().0,
                                    control_aria_label.replace(' ', "-"),
                                );

                                let mut control_choices: Vec<(String, String, bool)> = Vec::new();
                                for (id, label) in choices.iter() {
                                    control_choices.push((id.to_string(), label.to_string(), false));
                                }

                                let on_value_change = EventHandler::new({
            let assembly_id = assembly_id.clone();
            let all = props.value.clone();
            let mut send = send.clone();
            move |choice: Option<String>| {
                let Some(choice) = choice else {
                    return;
                };
                send(
                    all.iter()
                        .map(|(id, def)| {
                            if *id == assembly_id {
                                (id.clone(), choice.clone())
                            } else {
                                (id.clone(), def.clone())
                            }
                        })
                        .collect(),
                    false,
                );
            }
        });
        rsx! {

                                    Field { class: "",
                                        FieldLabel { html_for: control_id.clone(), "{control_label}" }
                                        Combobox::<String> {
                                            id: control_id,
                                            aria_label: control_aria_label.clone(),
                                            list_aria_label: control_aria_label.clone(),
                                            value: Some(attachment_choices.map({ let id = assembly_id.clone(); move |values| values.get(&id).unwrap_or(&None) }).into()),
                                            disabled,
                                            on_value_change,
                                            for (index, (value, label, disabled)) in control_choices.iter().enumerate() {
                                                ComboboxOption::<String> {
                                                    index,
                                                    value: value.clone(),
                                                    text_value: label.clone(),
                                                    disabled: *disabled,
                                                    "{label}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Button { variant: ButtonVariant::Secondary,
    button_type: "button",
    disabled: remove_disabled,
    aria_label: "Remove {assembly_id}",
    onclick: {
                                    let assembly_id = assembly_id.clone();
                                    let all = props.value.clone();
                                    let mut send = send.clone();
                                    move |_| send(all.iter().filter(|(id, _)| *id != assembly_id).cloned().collect(), true)
                                },

                                "Remove"

                             }
                        }
                    }
                }
            }
            if let Some(message) = error.as_deref() {
                FieldError { "{message}"  }
            }
        }
}

//! Shared per-field editor for the Matrix Inspector form.
//!
//! Every numeric, text, and choice row renders through `MatrixFieldEditor` so draft,
//! pending, and failure handling stay identical across fields.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum MatrixFieldKind {
    Name,
    PositiveInteger,
    PositiveNumber,
    NonnegativeNumber,
    Choice,
}

#[derive(Props, Clone, PartialEq)]
pub(super) struct MatrixFieldEditorProps {
    pub owner: MatrixInspectorOwner,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub field: MatrixEditField,
    pub label: &'static str,
    pub value: String,
    pub baseline: MatrixEditValue,
    pub kind: MatrixFieldKind,
    #[props(default)]
    pub choices: Vec<(String, String)>,
    pub request_sequence: Signal<u64>,
    pub editable: bool,
    pub feedback: Vec<MatrixEditFeedback>,
    pub view: MatrixFieldView,
    pub pending: bool,
    pub on_edit: EventHandler<MatrixEditRequest>,
}

#[component]
pub(super) fn MatrixFieldEditor(props: MatrixFieldEditorProps) -> Element {
    use_hook({
        let mut draft = props.view.draft;
        let mut failure = props.view.failure;
        let value = props.value.clone();
        let pending = props.pending;
        move || {
            if !pending && draft.peek().as_str() != value.as_str() {
                draft.set(value);
            }
            if failure.peek().is_some() {
                failure.set(None);
            }
        }
    });
    let mut draft = props.view.draft;
    let mut draft_baseline = use_signal(|| props.baseline.clone());
    let mut dirty = use_signal(|| false);
    let mut error = props.view.failure;
    let mut submitted_request_id = use_signal(|| None::<u64>);
    let mut status = use_signal(|| None::<String>);
    let accepted_value = props.value.clone();
    let accepted_baseline = props.baseline.clone();
    let pending = props.pending;
    let owner = props.owner.clone();
    let field = props.field;
    let feedback = props.feedback.clone();
    let editable = props.editable;
    let mut draft_for_effect = draft;
    let mut baseline_for_effect = draft_baseline;
    let mut dirty_for_effect = dirty;
    let mut error_for_effect = error;
    let mut submitted_for_effect = submitted_request_id;
    let mut status_for_effect = status;
    let submitted_id = submitted_request_id();
    let matching_feedback = feedback.into_iter().find(|feedback| {
        feedback.owner == owner
            && feedback.field == field
            && submitted_id == Some(feedback.request_id)
    });
    use_effect(use_reactive(
        (
            &accepted_value,
            &accepted_baseline,
            &matching_feedback,
            &pending,
        ),
        move |(value, baseline, feedback, pending)| {
            if pending {
                if status_for_effect.peek().as_deref() != Some("Saving…") {
                    status_for_effect.set(Some("Saving…".to_owned()));
                }
                return;
            }

            if status_for_effect.peek().as_deref() == Some("Saving…") {
                status_for_effect.set(None);
            }

            if submitted_for_effect().is_some() {
                submitted_for_effect.set(None);
                if status_for_effect.peek().is_some() {
                    status_for_effect.set(None);
                }
            }

            if let Some(feedback) = feedback.as_ref() {
                if submitted_for_effect.peek().is_some() {
                    submitted_for_effect.set(None);
                }
                if status_for_effect.peek().is_some() {
                    status_for_effect.set(None);
                }
                if error_for_effect.peek().as_deref() != Some(feedback.message.as_str()) {
                    error_for_effect.set(Some(feedback.message.clone()));
                }
            }

            if draft_for_effect() == value {
                if *baseline_for_effect.peek() != baseline {
                    baseline_for_effect.set(baseline.clone());
                }
                if *dirty_for_effect.peek() {
                    dirty_for_effect.set(false);
                }
                if submitted_for_effect.peek().is_some() {
                    submitted_for_effect.set(None);
                }
                if status_for_effect.peek().is_some() {
                    status_for_effect.set(None);
                }
            }

            // A dirty draft keeps the user's value: the latest committed value wins.
            if baseline_for_effect() != baseline && !dirty_for_effect() {
                draft_for_effect.set(value.clone());
                baseline_for_effect.set(baseline.clone());
                error_for_effect.set(None);
                if status_for_effect.peek().is_some() {
                    status_for_effect.set(None);
                }
                if submitted_for_effect.peek().is_some() {
                    submitted_for_effect.set(None);
                }
            }
        },
    ));

    let commit = {
        let owner = props.owner.clone();
        let field = props.field;
        let kind = props.kind;
        let sequence = props.request_sequence;
        let snapshot_token = props.snapshot_token;
        let revision = props.revision;
        let on_edit = props.on_edit;
        let accepted_display = props.value.clone();
        let accepted_baseline_now = props.baseline.clone();
        move |replacement_text: Option<String>| {
            if !editable || submitted_request_id().is_some() {
                return;
            }
            let text = match replacement_text {
                Some(text) => text,
                None => draft(),
            };
            let value = match kind {
                MatrixFieldKind::Name => {
                    let name = text.trim();
                    if name.is_empty() {
                        error.set(Some("Enter a name.".to_owned()));
                        return;
                    }
                    MatrixEditValue::Name(Some(name.to_owned()))
                }
                MatrixFieldKind::PositiveInteger => match text.trim().parse::<u32>() {
                    Ok(value) if value > 0 => match field {
                        MatrixEditField::Rows => MatrixEditValue::Rows(value),
                        MatrixEditField::Columns => MatrixEditValue::Columns(value),
                        _ => return,
                    },
                    _ => {
                        error.set(Some("Enter a positive whole number.".to_owned()));
                        return;
                    }
                },
                MatrixFieldKind::PositiveNumber => match text.trim().parse::<f64>() {
                    Ok(value) if value.is_finite() && value > 0.0 => match field {
                        MatrixEditField::PitchX => MatrixEditValue::PitchX(value),
                        MatrixEditField::PitchY => MatrixEditValue::PitchY(value),
                        _ => return,
                    },
                    _ => {
                        error.set(Some("Enter a finite number greater than zero.".to_owned()));
                        return;
                    }
                },
                MatrixFieldKind::NonnegativeNumber => match text.trim().parse::<f64>() {
                    Ok(value) if value.is_finite() && value >= 0.0 => match field {
                        MatrixEditField::EdgeGapX => MatrixEditValue::EdgeGapX(value),
                        MatrixEditField::EdgeGapY => MatrixEditValue::EdgeGapY(value),
                        _ => return,
                    },
                    _ => {
                        error.set(Some(
                            "Enter a finite number greater than or equal to zero.".to_owned(),
                        ));
                        return;
                    }
                },
                MatrixFieldKind::Choice => match field {
                    MatrixEditField::SwitchDefinition => {
                        MatrixEditValue::SwitchDefinition(text.clone())
                    }
                    MatrixEditField::DiodeDirection => match text.as_str() {
                        "row2col" => MatrixEditValue::DiodeDirection(DiodeDirection::Row2col),
                        "col2row" => MatrixEditValue::DiodeDirection(DiodeDirection::Col2row),
                        _ => return,
                    },
                    _ => return,
                },
            };
            let baseline = draft_baseline();
            let accepted_now = accepted_baseline_now.clone();
            let displayed_name_is_unchanged = field == MatrixEditField::Name
                && matches!(&value, MatrixEditValue::Name(Some(name)) if name == accepted_display.trim());
            if value == accepted_now || displayed_name_is_unchanged {
                draft.set(accepted_display.clone());
                dirty.set(false);
                error.set(None);
                status.set(None);
                submitted_request_id.set(None);
                return;
            }
            let mut sequence = sequence;
            let Some(request_id) = sequence().checked_add(1) else {
                error.set(Some(
                    "Matrix request identity is exhausted; reopen the inspector.".to_owned(),
                ));
                return;
            };
            sequence.set(request_id);
            let request = MatrixEditRequest {
                owner: owner.clone(),
                request_id,
                snapshot_token,
                revision,
                field,
                baseline,
                value,
            };
            submitted_request_id.set(Some(request_id));
            dirty.set(true);
            error.set(None);
            status.set(Some("Saving…".to_owned()));
            on_edit.call(request);
        }
    };

    let on_keydown = {
        let mut commit = commit.clone();
        let accepted_baseline = props.baseline.clone();
        move |event: KeyboardEvent| match event.data().key() {
            Key::Enter => {
                event.prevent_default();
                commit(None);
            }
            Key::Escape => {
                event.prevent_default();
                draft.set(props.value.clone());
                draft_baseline.set(accepted_baseline.clone());
                dirty.set(false);
                error.set(None);
                status.set(None);
                submitted_request_id.set(None);
            }
            _ => {}
        }
    };
    let feedback_error = props
        .feedback
        .iter()
        .find(|feedback| {
            feedback.owner == props.owner
                && feedback.field == props.field
                && submitted_request_id() == Some(feedback.request_id)
        })
        .map(|feedback| feedback.message.clone());
    let error_text = error.read().clone().or(feedback_error);
    let error_id = format!("matrix-field-error-{}", dioxus::core::current_scope_id().0);
    let current_status = status();
    let selected_choice = use_memo(move || Some(draft()));
    let choice_selected = draft();
    let input_value = draft();
    let current_baseline = props.baseline.clone();
    let input_type = if props.kind == MatrixFieldKind::Name {
        "text"
    } else {
        "number"
    };
    let step = match props.kind {
        MatrixFieldKind::Name => "any",
        MatrixFieldKind::PositiveInteger => "1",
        MatrixFieldKind::PositiveNumber | MatrixFieldKind::NonnegativeNumber => "any",
        MatrixFieldKind::Choice => "any",
    };
    let min = match props.kind {
        MatrixFieldKind::Name => None,
        MatrixFieldKind::PositiveInteger => Some("1"),
        MatrixFieldKind::PositiveNumber => Some("0"),
        MatrixFieldKind::NonnegativeNumber => Some("0"),
        MatrixFieldKind::Choice => None,
    };
    let mut commit_choice = commit.clone();
    let choices = props.choices.clone();
    let is_select = props.kind == MatrixFieldKind::Choice;
    rsx! {
            div { class: "grid min-w-0 content-start gap-[5px] text-xs text-muted-foreground",
                span { class: "flex min-w-0 items-center gap-1.5",
                    if is_select {
                        {
                            let control_label = (if props.field == MatrixEditField::SwitchDefinition {
                                "Matrix part definition"
                            } else {
                                props.label
                            })
                                .to_string();
                            let control_aria_label = (if props.field == MatrixEditField::SwitchDefinition {
                                "Matrix part definition"
                            } else {
                                props.label
                            })
                                .to_string();
                            let control_id = format!(
                                "layout-choice-{}-{}",
                                dioxus::core::current_scope_id().0,
                                control_aria_label.replace(' ', "-"),
                            );

                            let mut control_choices: Vec<(String, String, bool)> = Vec::new();
                            for (value, label) in choices {
                                control_choices.push((value.to_string(), label.to_string(), false));
                            }

                            let on_value_change = EventHandler::new(move |choice: Option<String>| {
            let Some(choice) = choice else {
                return;
            };
            let value = choice;
            draft.set(value.clone());
            dirty.set(true);
            error.set(None);
            status.set(Some("Saving…".to_owned()));
            commit_choice(Some(value));
        });
        rsx! {

                                Field { invalid: error_text.is_some(),
                                    FieldLabel { html_for: control_id.clone(), "{control_label}" }
                                    if props.field == MatrixEditField::SwitchDefinition {
                                        Combobox::<String> {
                                            id: control_id.clone(),
                                            aria_label: control_aria_label.clone(),
                                            aria_invalid: error_text.is_some(),
                                            aria_describedby: error_text.as_ref().map(|_| error_id.clone()),
                                            list_aria_label: control_aria_label.clone(),
                                            value: Some(selected_choice.into()),
                                            disabled: !props.editable,
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
                                    } else {
                                        InspectorChoiceRow {
                                            id: control_id,
                                            label: control_label,
                                            aria_label: control_aria_label,
                                            choices: control_choices,
                                            selected: choice_selected,
                                            disabled: !props.editable,
                                            invalid: error_text.is_some(),
                                            describedby: error_text.as_ref().map(|_| error_id.clone()),
                                            on_value_change,
                                        }
                                    }
                                }

                            }
                        }
                    } else {
                        {
                            let control_input_id = format!(
                                "layout-matrix-field-{}",
                                dioxus::core::current_scope_id().0,
                            );
                            rsx! {
                                Field { invalid: error_text.is_some(),
                                    FieldLabel { html_for: control_input_id.clone(), "{props.label}" }
                                    InputGroup { class: "max-[760px]:h-control-touch",
                                        InputGroupInput { class: "max-[760px]:h-control-touch", r#type: if input_type == "number" { InputType::Number } else { InputType::Text },
    step: step,
    min: min.map(str::to_owned),
    value: "{input_value}",
    readonly: !props.editable,
    aria_label: if props.field == MatrixEditField::SwitchDefinition { "Matrix part definition" } else { props.label },
    oninput: move |event: FormEvent| {
                            if !dirty() {
                                draft_baseline.set(current_baseline.clone());
                            }
                            draft.set(event.value());
                            dirty.set(true);
                            error.set(None);
                            status.set(None);
                            submitted_request_id.set(None);
                        },
    id: control_input_id.clone(),
    aria_describedby: error_text.as_ref().map(|_| error_id.clone()),
    attributes: vec![dioxus::core::Attribute::new("aria-invalid", error_text.is_some(), None, false),dioxus::events::onblur({
                            let mut commit = commit.clone();
                            move |_| commit(None)
                        }),dioxus::events::onkeydown(on_keydown)], }
                                        if matches!(
                                            props.kind,
                                            MatrixFieldKind::PositiveNumber | MatrixFieldKind::NonnegativeNumber
                                        )
                                        {
                                            InputGroupAddon { align: InputGroupAddonAlign::InlineEnd,
                                                InputGroupText { "mm" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                }
                if let Some(message) = error_text.as_deref() {
                    div { id: error_id,
                        FieldError { "{message}" }
                    }
                }
                if let Some(message) = current_status.as_deref() {
                    div { role: "status",  Callout {  variant: CalloutVariant::Info,  "{message}"  } }
                }
            }
        }
}

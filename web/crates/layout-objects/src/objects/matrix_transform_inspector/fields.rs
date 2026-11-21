//! Shared numeric, choice, toggle, and reset molecules for the matrix transform Inspector.
use super::*;

#[derive(Props, Clone, PartialEq)]
pub struct NumericTransformFieldProps {
    owner: MatrixTransformInspectorOwner,
    snapshot_token: SnapshotToken,
    revision: u64,
    field: MatrixTransformField,
    label: &'static str,
    unit: &'static str,
    value: f64,
    request_sequence: Signal<u64>,
    editable: bool,
    feedback: Vec<MatrixTransformFeedback>,
    on_edit: EventHandler<MatrixTransformRequest>,
    splay_affect: Signal<MatrixSplayAffect>,
    draft: Signal<String>,
    failure: Signal<Option<String>>,
    /// Hard unavailability with its own explained reason, e.g. a rotation the
    /// anchor does not follow. Unlike `editable`, this disables the input.
    #[props(default)]
    disabled: bool,
    #[props(default)]
    display_precision: Option<usize>,
    /// A muted badge beside the value, e.g. the pitch a default step follows.
    #[props(default)]
    badge: Option<String>,
}

#[component]
pub fn NumericTransformField(props: NumericTransformFieldProps) -> Element {
    let display = move |value: f64| match props.display_precision {
        Some(0) => format!("{value:.0}"),
        Some(precision) => {
            let text = format!("{value:.precision$}");
            let text = text.trim_end_matches('0').trim_end_matches('.');
            if text == "-0" {
                "0".to_owned()
            } else {
                text.to_owned()
            }
        }
        None => value.to_string(),
    };
    let mut draft = props.draft;
    let mut baseline = use_signal(|| props.value);
    let mut dirty = use_signal(|| false);
    let mut error = props.failure;
    let accepted_value = props.value;
    let mut draft_effect = draft;
    let mut baseline_effect = baseline;
    let mut dirty_effect = dirty;
    let mut error_effect = error;
    let mut observed_owner = use_signal(|| props.owner.clone());
    let field_affect =
        (props.field == MatrixTransformField::ColumnSplay).then(|| (props.splay_affect)());
    let mut observed_affect = use_signal(|| field_affect.clone());
    use_effect(use_reactive(
        (&accepted_value, &draft(), &props.owner, &field_affect),
        move |(value, current_draft, owner, affect)| {
            if *observed_owner.peek() != owner || *observed_affect.peek() != affect {
                observed_owner.set(owner);
                observed_affect.set(affect);
                draft_effect.set(display(value));
                baseline_effect.set(value);
                dirty_effect.set(false);
                error_effect.set(None);
                return;
            }
            if current_draft.is_empty() && !(*dirty_effect.peek()) {
                draft_effect.set(display(value));
                if *baseline_effect.peek() != value {
                    baseline_effect.set(value);
                }
                return;
            }
            // PendingEditSignals restores the accepted value only while the current text
            // still equals the submitted text. This effect handles unrelated accepted edits.
            if *baseline_effect.peek() != value && !(*dirty_effect.peek()) {
                draft_effect.set(display(value));
                baseline_effect.set(value);
                error_effect.set(None);
            } else if current_draft == value.to_string() {
                if current_draft != display(value) {
                    draft_effect.set(display(value));
                }
                if *baseline_effect.peek() != value {
                    baseline_effect.set(value);
                }
                if *dirty_effect.peek() {
                    dirty_effect.set(false);
                }
            }
        },
    ));
    let commit = {
        let owner = props.owner.clone();
        let field = props.field;
        let sequence = props.request_sequence;
        let token = props.snapshot_token;
        let revision = props.revision;
        let on_edit = props.on_edit;
        let affect = props.splay_affect;
        let editable = props.editable;
        let mut sequence = sequence;
        move || {
            // Formatting an accepted value must never turn focus/blur into a rounded edit.
            if !editable || !dirty() {
                return;
            }
            let next = match draft().trim().parse::<f64>() {
                Ok(number) if number.is_finite() => number,
                _ => {
                    error.set(Some("Enter a finite number.".to_owned()));
                    return;
                }
            };
            let accepted_baseline = baseline();
            if next == accepted_value {
                draft.set(display(accepted_value));
                baseline.set(accepted_value);
                dirty.set(false);
                error.set(None);
                return;
            }
            let Some(id) = sequence().checked_add(1) else {
                error.set(Some(
                    "Transform request identity is exhausted; reopen the inspector.".to_owned(),
                ));
                return;
            };
            sequence.set(id);
            let submitted_text = draft.peek().clone();
            let request = MatrixTransformRequest {
                owner: owner.clone(),
                request_id: id,
                snapshot_token: token,
                revision,
                field,
                baseline: MatrixTransformValue::Number(accepted_baseline),
                value: MatrixTransformValue::Number(next),
                splay_affect: affect(),
                draft: Some(draft),
                failure: Some(error),
                submitted_text: Some(submitted_text),
                one_shot: false,
            };
            dirty.set(true);
            error.set(None);
            on_edit.call(request);
        }
    };
    let current = draft();
    let error_message = error();
    let badge = props.badge.clone();
    let inherited_pitch = badge.is_some();
    // Ticket 16: hovering or focusing this number field previews, in amber,
    // the keys a change to it would move. The preview is transient view
    // state: it never writes, and it clears on leave, blur, unmount or
    // selection change. Contexts are optional so unit harnesses without an
    // Editor root simply show no preview.
    let field = props.field;
    let preview_context = props.owner.context.clone();
    let affect_signal = props.splay_affect;
    let preview = try_use_context::<FieldPreview>();
    let preview_source = try_use_context::<Option<FieldPreviewSource>>().flatten();
    let shown = use_hook(|| std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new())));
    let show_preview = {
        let shown = shown.clone();
        move || {
            let (Some(target), Some(source)) = (preview, preview_source.clone()) else {
                return;
            };
            let Some(facts) = crate::matrix_relations::field_preview(
                &source.document,
                &source.scene.matrix_scenes,
                &preview_context,
                field,
                affect_signal(),
            ) else {
                return;
            };
            *shown.borrow_mut() = facts.parts.clone();
            target.show(FieldPreviewState {
                parts: facts.parts,
                hint: facts.hint,
            });
        }
    };
    let release_preview = {
        let shown = shown.clone();
        move || {
            if let Some(target) = preview {
                target.release(&shown.borrow());
            }
        }
    };
    use_drop({
        let shown = shown.clone();
        move || {
            if let Some(target) = preview {
                target.release(&shown.borrow());
            }
        }
    });
    rsx! {
        {
            let input_id = format!("layout-number-{}", dioxus::core::current_scope_id().0);
            let unit_id = format!("{input_id}-unit");
            let error_id = format!("{input_id}-error");
            let described_by = match (props.unit.is_empty(), error_message.is_some()) {
                (false, true) => Some(format!("{unit_id} {error_id}")),
                (false, false) => Some(unit_id.clone()),
                (true, true) => Some(error_id.clone()),
                (true, false) => None,
            };
            rsx! {
                Field { class: "grid min-w-0 content-start gap-[5px] text-xs text-muted-foreground", invalid: error_message.is_some(),
                    FieldLabel { html_for: input_id.clone(), "{props.label}" }
                    InputGroup { class: "max-[760px]:h-control-touch",
                        InputGroupInput { class: if inherited_pitch { "max-[760px]:h-control-touch italic text-muted-foreground" } else { "max-[760px]:h-control-touch" },
                            id: input_id,
                            r#type: InputType::Number,
                            step: "any",
                            value: current,
                            readonly: !props.editable,
                            disabled: props.disabled,
                            aria_label: props.label,
                            aria_invalid: error_message.is_some(),
                            aria_describedby: described_by,
                            oninput: move |event: FormEvent| {
                                if !dirty() {
                                    baseline.set(accepted_value);
                                }
                                draft.set(event.value());
                                dirty.set(true);
                                error.set(None);
                            },
                            attributes: vec![
                                dioxus::events::onpointerenter({ let show = show_preview.clone(); move |_| show() }),
                                dioxus::events::onpointerleave({ let release = release_preview.clone(); move |_| release() }),
                                dioxus::events::onfocus({ let show = show_preview.clone(); move |_| show() }),
                                dioxus::events::onblur({
                                    let mut commit = commit.clone();
                                    let release = release_preview.clone();
                                    move |_| { commit(); release(); }
                                }),
                                dioxus::events::onkeydown({
                                    let mut commit = commit.clone();
                                    move |event: KeyboardEvent| match event.data().key() {
                                        Key::Enter => {
                                            event.prevent_default();
                                            commit();
                                        }
                                        Key::Escape => {
                                            event.prevent_default();
                                            draft.set(display(accepted_value));
                                            baseline.set(accepted_value);
                                            dirty.set(false);
                                            error.set(None);
                                        }
                                        _ => {}
                                    }
                                }),
                            ],
                        }
                        if !props.unit.is_empty() {
                            InputGroupAddon { align: InputGroupAddonAlign::InlineEnd,
                                InputGroupText {
                                    span { id: unit_id, "{props.unit}" }
                                }
                            }
                        }
                    }
                    if let Some(message) = error_message {
                        div { id: error_id,
                            FieldError { "{message}" }
                        }
                    }
                }
            }
        }
        if let Some(badge) = badge {
            small { "data-pitch-badge": "true", class: "mt-0.5 inline-block rounded-sm bg-muted px-1.5 text-xs italic text-muted-foreground", "{badge}" }
        }
    }
}
#[derive(Props, Clone, PartialEq)]
pub(super) struct ResetTransformButtonProps {
    pub label: &'static str,
    pub owner: MatrixTransformInspectorOwner,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub field: MatrixTransformField,
    pub baseline: MatrixTransformValue,
    pub value: MatrixTransformValue,
    pub request_sequence: Signal<u64>,
    pub editable: bool,
    pub pending_disabled: Signal<bool>,
    pub feedback: Vec<MatrixTransformFeedback>,
    pub splay_affect: Signal<MatrixSplayAffect>,
    pub on_edit: EventHandler<MatrixTransformRequest>,
    /// Why the action is unavailable; disables the button and is shown beside it.
    #[props(default)]
    pub blocked: Option<String>,
}

#[component]
pub(super) fn ResetTransformButton(props: ResetTransformButtonProps) -> Element {
    let mut submitted = use_signal(|| None::<u64>);
    let current_baseline = props.baseline.clone();
    let observed_baseline = use_signal(|| current_baseline.clone());
    let mut submitted_effect = submitted;
    let mut observed_effect = observed_baseline;
    use_effect(use_reactive((&current_baseline,), move |(baseline,)| {
        if observed_effect() != baseline {
            observed_effect.set(baseline);
            submitted_effect.set(None);
        }
    }));
    let error = props
        .feedback
        .iter()
        .rev()
        .find(|item| {
            item.owner == props.owner
                && item.field == props.field
                && submitted() == Some(item.request_id)
                && item.message.is_some()
        })
        .and_then(|item| item.message.clone());
    let disabled = !props.editable || (props.pending_disabled)() || props.blocked.is_some();
    let mut sequence = props.request_sequence;
    let owner = props.owner.clone();
    let baseline = props.baseline.clone();
    let value = props.value.clone();
    let field = props.field;
    let token = props.snapshot_token;
    let revision = props.revision;
    let on_edit = props.on_edit;
    let affect = props.splay_affect;
    rsx! {
            Button { variant: ButtonVariant::Secondary,
    button_type: "button",
    disabled: disabled,
    onclick: move |_| {
                        let Some(id) = sequence().checked_add(1) else { return; };
                        sequence.set(id);
                        submitted.set(Some(id));
                        on_edit.call(MatrixTransformRequest::one_shot(
                            owner.clone(), (token, revision), id,
                            (field, baseline.clone(), value.clone()), affect(),
                        ));
                    },

                "{props.label}"

             }
            if let Some(reason) = props.blocked.as_deref() {
                small { "{reason}" }
            }
            if let Some(message) = error.as_deref() {
                FieldError { "{message}"  }
            }
        }
}

/// One labeled dropdown row: `Field` + `FieldLabel` + `Select` + `FieldError`.
///
/// The caller computes `id` with the surrounding scope
/// (`layout-choice-{scope}-{label}`) so element ids are unchanged by the
/// extraction; only the repeated composition moves here.
#[derive(Props, Clone, PartialEq)]
pub(super) struct TransformChoiceRowProps {
    pub id: String,
    pub label: String,
    pub aria_label: String,
    pub choices: Vec<(String, String, bool)>,
    pub selected: String,
    pub disabled: bool,
    #[props(default)]
    pub error: Option<String>,
    pub on_value_change: EventHandler<Option<String>>,
}

#[component]
pub(super) fn TransformChoiceRow(props: TransformChoiceRowProps) -> Element {
    let selected = use_memo(use_reactive((&props.selected,), |(selected,)| {
        Some(selected.clone())
    }));
    let error_id = format!("{}-error", props.id);
    rsx! {
        Field { class: "grid min-w-0 content-start gap-[5px] text-xs text-muted-foreground",
            invalid: props.error.is_some(),
            FieldLabel { html_for: props.id.clone(), "{props.label}" }
            Select::<String> {
                id: props.id.clone(),
                aria_label: props.aria_label.clone(),
                aria_invalid: props.error.is_some(),
                aria_describedby: props.error.as_ref().map(|_| error_id.clone()),
                value: Some(selected.into()),
                disabled: props.disabled,
                on_value_change: props.on_value_change,
                for (index, (value, label, disabled)) in props.choices.iter().enumerate() {
                    SelectOption::<String> {
                        index,
                        value: value.clone(),
                        text_value: label.clone(),
                        disabled: *disabled,
                        "{label}"
                    }
                }
            }
            if let Some(message) = props.error {
                div { id: error_id,
                    FieldError { "{message}" }
                }
            }
        }
    }
}

/// One labeled checkbox row: `Label` + `Checkbox` with optional help and error.
///
/// `id` keeps the caller's scope-based value (`layout-check-{scope}-{label}`
/// or the fixed `matrix-key-omit`); only the repeated composition moves here.
#[derive(Props, Clone, PartialEq)]
pub(super) struct TransformCheckRowProps {
    pub id: String,
    pub label: String,
    pub aria_label: String,
    pub checked: bool,
    pub disabled: bool,
    #[props(default)]
    pub help: Option<String>,
    #[props(default)]
    pub error: Option<String>,
    pub on_checked_change: EventHandler<CheckboxState>,
}

#[component]
pub(super) fn TransformCheckRow(props: TransformCheckRowProps) -> Element {
    let checked = props.checked;
    rsx! {
        Label { html_for: props.id.clone(),
            Checkbox {
                id: props.id.clone(),
                aria_label: props.aria_label.clone(),
                disabled: props.disabled,
                checked: Some(if checked { CheckboxState::Checked } else { CheckboxState::Unchecked }),
                on_checked_change: props.on_checked_change,
            }
            " {props.label}"
        }
        if let Some(help) = props.help {
            p { class: "text-xs text-muted-foreground", "{help}" }
        }
        if let Some(message) = props.error {
            FieldError { "{message}" }
        }
    }
}

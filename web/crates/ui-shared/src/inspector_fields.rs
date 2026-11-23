//! Reusable declarative inspector field molecules and primitive input wrappers.
use crate::ui::{
    badge::{Badge, BadgeVariant},
    button::{Button, ButtonSize, ButtonVariant},
    checkbox::{Checkbox, CheckboxState},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    combobox::{Combobox, ComboboxOption},
    field::{
        Field, FieldContent, FieldDescription, FieldError, FieldGroup, FieldLabel, FieldVariant,
    },
    input::InputType,
    input_group::{
        InputGroup, InputGroupAddon, InputGroupAddonAlign, InputGroupInput, InputGroupText,
    },
    label::Label,
    select::{Select, SelectOption},
};
use dioxus::prelude::*;

/// Touch-target sizing shared by inspector controls on narrow/coarse-pointer viewports.
pub const TOUCH_CLS: &str =
    "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch";
const SELECT_CLS: &str = "max-[960px]:[&_[data-slot=select-trigger]]:min-h-control-touch [@media(pointer:coarse)]:[&_[data-slot=select-trigger]]:min-h-control-touch";
const COMBO_CLS: &str = "max-[960px]:[&_[data-slot=combobox-input]]:min-h-control-touch [@media(pointer:coarse)]:[&_[data-slot=combobox-input]]:min-h-control-touch";

/// Parse finite user numeric text and clamp to optional inclusive bounds.
pub fn parse_clamped_f64(raw: &str, min: Option<f64>, max: Option<f64>) -> Option<f64> {
    let mut v = raw.trim().parse::<f64>().ok().filter(|v| v.is_finite())?;
    if let Some(lo) = min {
        v = v.max(lo);
    }
    if let Some(hi) = max {
        v = v.min(hi);
    }
    Some(v)
}

/// Format finite floats concisely without trailing fractional zeros.
pub fn format_numeric(value: f64, precision: usize) -> String {
    if !value.is_finite() {
        return String::new();
    }
    let text = format!("{:.precision$}", if value == 0.0 { 0.0 } else { value });
    if text.contains('.') {
        let trimmed = text.trim_end_matches('0').trim_end_matches('.');
        if trimmed == "-0" {
            "0".into()
        } else {
            trimmed.into()
        }
    } else {
        text
    }
}

struct CommitFieldSpec {
    r#type: InputType,
    accepted: String,
    placeholder: Option<String>,
    aria_label: Option<String>,
    disabled_reason: Option<String>,
    description: Option<String>,
    unit: Option<String>,
    min: Option<String>,
    max: Option<String>,
    step: Option<String>,
    error: Option<String>,
}

/// Attempt a commit: no-op when pristine (Enter already committed and cleared
/// dirty, so a trailing blur is ignored); refuse invalid text or commit valid.
#[derive(Clone, Copy)]
struct CommitState {
    dirty: Signal<bool>,
    focused: Signal<bool>,
    failed_at: Signal<Option<String>>,
}

fn attempt_commit(
    mut state: CommitState,
    draft: Signal<String>,
    accepted: &str,
    validate: &dyn Fn(&str) -> Option<String>,
    mut commit: impl FnMut(String),
) {
    if !state.dirty.replace(false) {
        state.focused.set(false);
        return;
    }
    state.focused.set(false);
    let current_draft = draft.peek().clone();
    if validate(&current_draft).is_some() {
        state.failed_at.set(Some(accepted.to_owned()));
    } else {
        state.failed_at.set(None);
        commit(current_draft);
    }
}

/// Revert an edit: restore the accepted text and clear dirty/error/focus.
fn revert_commit(mut state: CommitState, mut draft: Signal<String>, accepted: &str) {
    draft.set(accepted.to_owned());
    state.dirty.set(false);
    state.failed_at.set(None);
    state.focused.set(false);
}

/// Shared Enter/blur-commit row: `dirty` makes blur after Enter a no-op, refused
/// input keeps the draft with an inline `FieldError`, and `Escape` reverts.
fn commit_field_row(
    id: String,
    label: String,
    spec: CommitFieldSpec,
    validate: impl Fn(&str) -> Option<String> + Clone + 'static,
    on_commit_text: EventHandler<String>,
) -> Element {
    let mut focused = use_signal(|| false);
    let mut draft = use_signal(|| spec.accepted.clone());
    let mut dirty = use_signal(|| false);
    let mut failed_at = use_signal(|| None::<String>);
    let accepted = spec.accepted.clone();
    let focus_value = accepted.clone();
    let escape_value = accepted.clone();
    let accepted_blur = accepted.clone();
    let accepted_enter = accepted.clone();
    let validate_blur = validate.clone();
    let state = CommitState {
        dirty,
        focused,
        failed_at,
    };
    let error_id = format!("{id}-error");
    let internal = failed_at()
        .filter(|at| at == &accepted)
        .and_then(|_| validate(&draft()));
    let has_internal_error = internal.is_some();
    let commit = on_commit_text;
    let on_input = move |e: FormEvent| {
        focused.set(true);
        dirty.set(true);
        draft.set(e.value());
    };
    let on_focus = move |_| {
        if failed_at.peek().as_deref() != Some(focus_value.as_str()) {
            draft.set(focus_value.clone());
            dirty.set(false);
            failed_at.set(None);
        } else {
            dirty.set(true);
        }
        focused.set(true);
    };
    let on_blur = move |_| {
        attempt_commit(state, draft, &accepted_blur, &validate_blur, |next| {
            commit.call(next)
        });
    };
    let validate_enter = validate;
    let on_key = move |e: KeyboardEvent| match e.key() {
        Key::Enter => {
            e.prevent_default();
            attempt_commit(state, draft, &accepted_enter, &validate_enter, |next| {
                commit.call(next)
            });
        }
        Key::Escape => {
            e.prevent_default();
            revert_commit(state, draft, &escape_value);
        }
        _ => {}
    };
    let message = spec.error.clone().or(internal);
    let failed = message.is_some();
    let shown = if focused() || has_internal_error {
        draft()
    } else {
        accepted
    };
    let disabled = spec.disabled_reason.is_some();
    let help = spec.disabled_reason.or(spec.description);
    rsx! {
        Field { invalid: failed, disabled, class: "gap-1 min-w-0",
            FieldLabel { html_for: id.clone(), "{label}" }
            InputGroup {
                InputGroupInput {
                    id, r#type: spec.r#type, class: TOUCH_CLS, value: "{shown}", disabled,
                    placeholder: spec.placeholder, min: spec.min, max: spec.max, step: spec.step,
                    aria_label: spec.aria_label.unwrap_or(label),
                    aria_invalid: failed.then_some(true),
                    aria_describedby: message.as_ref().map(|_| error_id.clone()),
                    oninput: on_input,
                    attributes: vec![
                        dioxus::events::onfocus(on_focus),
                        dioxus::events::onblur(on_blur),
                        dioxus::events::onkeydown(on_key),
                    ],
                }
                if let Some(u) = spec.unit { InputGroupAddon { align: InputGroupAddonAlign::InlineEnd, InputGroupText { "{u}" } } }
            }
            if let Some(help) = help { FieldDescription { "{help}" } }
            if let Some(reason) = message { div { id: error_id, FieldError { "{reason}" } } }
        }
    }
}

/// Stateless declarative numeric commit molecule (Enter/blur parse+clamp, Escape revert).
#[component]
pub fn NumericField(
    label: String,
    #[props(into, optional)] unit: Option<String>,
    value: f64,
    #[props(default)] min: Option<f64>,
    #[props(default)] max: Option<f64>,
    #[props(default)] step: Option<f64>,
    #[props(default = 3)] precision: usize,
    #[props(into, optional)] placeholder: Option<String>,
    #[props(default)] mixed: bool,
    #[props(into, optional)] disabled_reason: Option<String>,
    #[props(into, optional)] id: Option<String>,
    #[props(into, optional)] aria_label: Option<String>,
    #[props(into, optional)] description: Option<String>,
    #[props(into, optional)] error: Option<String>,
    on_commit: EventHandler<f64>,
) -> Element {
    let id = id.unwrap_or_else(|| format!("inspector-num-{}", dioxus::core::current_scope_id().0));
    let accepted = if mixed || !value.is_finite() {
        String::new()
    } else {
        format_numeric(value, precision)
    };
    let validate = move |raw: &str| {
        parse_clamped_f64(raw, min, max)
            .is_none()
            .then(|| "Enter a valid number.".to_owned())
    };
    commit_field_row(
        id,
        label,
        CommitFieldSpec {
            r#type: InputType::Number,
            accepted,
            placeholder: placeholder.or_else(|| mixed.then(|| "Mixed".to_owned())),
            aria_label,
            disabled_reason,
            description,
            error,
            unit,
            min: min.map(|v| format_numeric(v, precision)),
            max: max.map(|v| format_numeric(v, precision)),
            step: step
                .map(|v| format_numeric(v, precision))
                .or_else(|| Some("any".to_owned())),
        },
        validate,
        EventHandler::new(move |raw: String| {
            if let Some(next) = parse_clamped_f64(&raw, min, max)
                && (mixed || (next - value).abs() > f64::EPSILON)
            {
                on_commit.call(next);
            }
        }),
    )
}

/// Stateless single-line text commit molecule (Enter/blur trim+commit, Escape revert).
#[component]
pub fn TextCommitField(
    label: String,
    value: String,
    #[props(default = false)] allow_empty: bool,
    #[props(into, optional)] placeholder: Option<String>,
    #[props(into, optional)] disabled_reason: Option<String>,
    #[props(into, optional)] id: Option<String>,
    #[props(into, optional)] aria_label: Option<String>,
    #[props(into, optional)] error: Option<String>,
    on_commit: EventHandler<String>,
) -> Element {
    let id = id.unwrap_or_else(|| format!("inspector-text-{}", dioxus::core::current_scope_id().0));
    let current = value.clone();
    let validate = move |raw: &str| {
        (!allow_empty && raw.trim().is_empty()).then(|| "This field is required.".to_owned())
    };
    commit_field_row(
        id,
        label,
        CommitFieldSpec {
            r#type: InputType::Text,
            accepted: value,
            placeholder,
            aria_label,
            disabled_reason,
            description: None,
            unit: None,
            min: None,
            max: None,
            step: None,
            error,
        },
        validate,
        EventHandler::new(move |raw: String| {
            let next = raw.trim().to_owned();
            if (allow_empty || !next.is_empty()) && next != current {
                on_commit.call(next);
            }
        }),
    )
}

/// Declarative choice selector (`Select` up to 12 options, filterable `Combobox` above).
#[component]
pub fn SelectField<T: Clone + PartialEq + 'static>(
    label: String,
    value: T,
    options: Vec<(T, String)>,
    #[props(into, optional)] placeholder: Option<String>,
    #[props(into, optional)] disabled_reason: Option<String>,
    #[props(into, optional)] id: Option<String>,
    #[props(into, optional)] aria_label: Option<String>,
    #[props(default)] disabled: bool,
    on_commit: EventHandler<T>,
) -> Element {
    let id =
        id.unwrap_or_else(|| format!("inspector-select-{}", dioxus::core::current_scope_id().0));
    let disabled = disabled || disabled_reason.is_some();
    let long = options.len() > 12;
    let has_match = options.iter().any(|(candidate, _)| *candidate == value);
    let selected = has_match.then(|| value.clone());
    let current = use_memo(use_reactive((&selected,), |(s,)| s));
    let name = aria_label.unwrap_or_else(|| label.clone());
    let empty_placeholder = (!has_match).then(|| placeholder.clone()).flatten();
    let offset = usize::from(empty_placeholder.is_some());
    rsx! {
        Field { class: "gap-1.5 min-w-0", disabled,
            if long {
                Label { class: "flex-col items-stretch gap-2 w-full", "{label}"
                    Combobox::<T> {
                        id, class: COMBO_CLS, aria_label: name.clone(), list_aria_label: name,
                        placeholder: placeholder.unwrap_or_default(), value: Some(current.into()), disabled,
                        on_value_change: move |next: Option<T>| if let Some(next) = next { on_commit.call(next); },
                        for (index, (v, t)) in options.into_iter().enumerate() {
                            ComboboxOption::<T> { class: TOUCH_CLS, index, value: v, text_value: t.clone(), "{t}" }
                        }
                    }
                }
            } else {
                FieldLabel { html_for: id.clone(), "{label}" }
                Select::<T> {
                    id, class: SELECT_CLS, aria_label: name, value: Some(current.into()), disabled,
                    on_value_change: move |next: Option<T>| if let Some(next) = next { on_commit.call(next); },
                    if let Some(text) = empty_placeholder {
                        SelectOption::<T> { class: TOUCH_CLS, index: 0usize, value, disabled: true, text_value: text.clone(), "{text}" }
                    }
                    for (index, (v, t)) in options.into_iter().enumerate() {
                        SelectOption::<T> { class: TOUCH_CLS, index: index + offset, value: v, text_value: t.clone(), "{t}" }
                    }
                }
            }
            if let Some(reason) = disabled_reason { FieldDescription { "{reason}" } }
        }
    }
}

/// Declarative boolean toggle molecule.
#[component]
pub fn ToggleField(
    label: String,
    checked: bool,
    #[props(into, optional)] description: Option<String>,
    #[props(into, optional)] disabled_reason: Option<String>,
    #[props(into, optional)] id: Option<String>,
    #[props(into, optional)] aria_label: Option<String>,
    on_commit: EventHandler<bool>,
) -> Element {
    let id =
        id.unwrap_or_else(|| format!("inspector-toggle-{}", dioxus::core::current_scope_id().0));
    let disabled = disabled_reason.is_some();
    rsx! {
        CheckboxField {
            id, label, checked: if checked { CheckboxState::Checked } else { CheckboxState::Unchecked },
            disabled, aria_label, description: disabled_reason.or(description),
            on_checked_change: move |next: CheckboxState| on_commit.call(bool::from(next)),
        }
    }
}

/// Collapsible inspector card section with optional badge and reset action.
#[component]
pub fn InspectorSection(
    title: String,
    #[props(default = true)] default_open: bool,
    #[props(into, optional)] open: Option<bool>,
    #[props(default)] on_open_change: Option<EventHandler<bool>>,
    #[props(into, optional)] badge: Option<String>,
    #[props(default)] on_reset: Option<EventHandler<()>>,
    children: Element,
) -> Element {
    let mut local_open = use_signal(|| default_open);
    let is_open = if let Some(open) = open {
        open
    } else {
        local_open()
    };
    rsx! {
        Collapsible {
            keep_mounted: true, open: Some(is_open),
            on_open_change: move |next| { local_open.set(next); if let Some(h) = on_open_change { h.call(next); } },
            div { class: "flex items-center justify-between gap-2",
                CollapsibleTrigger { class: "flex-1 justify-start {TOUCH_CLS}", aria_label: "{title}",
                    span { "{title}" }
                    if let Some(badge) = badge { Badge { variant: BadgeVariant::Secondary, "{badge}" } }
                }
                if let Some(reset) = on_reset {
                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, onclick: move |_| reset.call(()), "Reset" }
                }
            }
            CollapsibleContent { class: "data-[open=false]:hidden data-[open=true]:overflow-visible", FieldGroup { {children} } }
        }
    }
}

#[derive(Default)]
struct TextFieldSpec {
    r#type: InputType,
    description: Option<String>,
    error: Option<String>,
    disabled: bool,
    readonly: bool,
    aria_label: Option<String>,
    placeholder: Option<String>,
    maxlength: Option<String>,
    class: Option<String>,
    control_class: Option<String>,
    aria_invalid: Option<bool>,
    message_id: Option<String>,
    plain_label: bool,
}

struct TextFieldEvents {
    oninput: EventHandler<FormEvent>,
    onblur: EventHandler<FocusEvent>,
    onkeydown: EventHandler<KeyboardEvent>,
}

fn text_input_row(
    id: String,
    label: String,
    value: String,
    s: TextFieldSpec,
    ev: TextFieldEvents,
) -> Element {
    let message_id = s.message_id.unwrap_or_else(|| format!("{id}-message"));
    let has_error = s.error.is_some();
    let mut extra = vec![
        dioxus::events::onblur(move |e| ev.onblur.call(e)),
        dioxus::events::onkeydown(move |e| ev.onkeydown.call(e)),
    ];
    if let Some(ml) = s.maxlength {
        extra.push(Attribute::new("maxlength", ml.as_str(), None, false));
    }
    rsx! {
        Field { invalid: has_error, disabled: s.disabled, class: s.class.as_deref().unwrap_or("gap-1 min-w-0"),
            if s.plain_label { Label { html_for: id.clone(), "{label}" } } else { FieldLabel { html_for: id.clone(), "{label}" } }
            InputGroup { class: s.control_class.clone(),
                InputGroupInput {
                    id,
                    step: (s.r#type == InputType::Number).then(|| "any".to_owned()),
                    r#type: s.r#type,
                    class: s.control_class.as_deref().unwrap_or(TOUCH_CLS),
                    value: "{value}",
                    disabled: s.disabled, readonly: s.readonly, placeholder: s.placeholder,
                    aria_label: s.aria_label,
                    aria_invalid: has_error.then_some(true).or(s.aria_invalid),
                    aria_describedby: has_error.then(|| message_id.clone()),
                    oninput: move |e| ev.oninput.call(e), attributes: extra,
                }
            }
            if let Some(desc) = s.description { FieldDescription { "{desc}" } }
            if let Some(err) = s.error { div { id: message_id, FieldError { "{err}" } } }
        }
    }
}

#[component]
pub fn TextField(
    id: String,
    label: String,
    value: String,
    oninput: EventHandler<FormEvent>,
    #[props(default)] r#type: InputType,
    #[props(default)] description: Option<String>,
    #[props(default)] error: Option<String>,
    #[props(default)] disabled: bool,
    #[props(into, optional)] aria_label: Option<String>,
    #[props(into, optional)] placeholder: Option<String>,
    #[props(default)] readonly: bool,
    #[props(default)] maxlength: Option<String>,
    #[props(into, optional)] class: Option<String>,
    #[props(into, optional)] control_class: Option<String>,
    #[props(default)] plain_label: bool,
    #[props(default = Some(false))] aria_invalid: Option<bool>,
    #[props(into, optional)] message_id: Option<String>,
    #[props(default)] onblur: EventHandler<FocusEvent>,
    #[props(default)] onkeydown: EventHandler<KeyboardEvent>,
) -> Element {
    text_input_row(
        id,
        label,
        value,
        TextFieldSpec {
            r#type,
            description,
            error,
            disabled,
            readonly,
            aria_label,
            placeholder,
            maxlength,
            class,
            control_class,
            aria_invalid,
            message_id,
            plain_label,
        },
        TextFieldEvents {
            oninput,
            onblur,
            onkeydown,
        },
    )
}

#[component]
pub fn TextareaField(
    label: String,
    value: String,
    oninput: EventHandler<FormEvent>,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default = 3)] rows: u8,
    #[props(default = true)] spellcheck: bool,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] error: Option<String>,
    #[props(default)] description: Option<String>,
) -> Element {
    let id = format!("inspector-textarea-{}", dioxus::core::current_scope_id().0);
    let message_id = format!("{id}-message");
    let has_error = error.is_some();
    let described_by = has_error.then(|| message_id.clone());
    rsx! {
        Field { invalid: has_error, disabled, class: "gap-1.5 min-w-0",
            FieldLabel { html_for: id.clone(), "{label}" }
            textarea { id, class: "text-foreground placeholder:text-muted-foreground flex min-h-control w-full min-w-0 rounded-md bg-field px-3 py-2 text-base", rows: rows as i64, spellcheck, value: "{value}", disabled, aria_label, placeholder, aria_invalid: has_error, aria_describedby: described_by, oninput: move |e| oninput.call(e) }
            if let Some(description) = description { FieldDescription { "{description}" } }
            if let Some(error) = error { div { id: message_id, FieldError { "{error}" } } }
        }
    }
}

#[component]
pub fn CheckboxField(
    id: String,
    label: String,
    checked: CheckboxState,
    on_checked_change: EventHandler<CheckboxState>,
    #[props(default)] description: Option<String>,
    #[props(default)] error: Option<String>,
    #[props(default)] disabled: bool,
    #[props(into, optional)] aria_label: Option<String>,
    #[props(default)] inline: bool,
) -> Element {
    let cb = rsx! { Checkbox { id: id.clone(), aria_label, checked: Some(checked), disabled, on_checked_change } };
    if inline {
        return rsx! { Label { html_for: id, {cb} "{label}" } };
    }
    rsx! {
        Field { variant: FieldVariant::Horizontal, invalid: error.is_some(), disabled, {cb}
            FieldContent {
                FieldLabel { html_for: id, class: "min-h-control-touch", "{label}" }
                if let Some(description) = description { FieldDescription { "{description}" } }
                if let Some(error) = error { FieldError { "{error}" } }
            }
        }
    }
}

/// Hidden native file input behind a named button; reports the picked files and
/// resets the input so picking the same file again re-fires. Browser-only: the
/// picked `web_sys::File`s only exist on `wasm32`.
#[cfg(target_arch = "wasm32")]
#[component]
pub fn FilePickerButton(
    label: String,
    #[props(into, optional)] accept: Option<String>,
    #[props(default)] directory: bool,
    #[props(default)] disabled: bool,
    #[props(into, optional)] aria_label: Option<String>,
    #[props(default = ButtonVariant::Secondary)] variant: ButtonVariant,
    on_files: EventHandler<Vec<web_sys::File>>,
) -> Element {
    use dioxus_web::WebEventExt;
    use wasm_bindgen::JsCast;
    let mut input = use_signal(|| None::<web_sys::HtmlInputElement>);
    rsx! {
        Button { variant, class: TOUCH_CLS, disabled, aria_label: aria_label.clone(),
            onclick: move |_| if let Some(input) = input() { input.click(); }, "{label}" }
        input { r#type: "file", hidden: true, accept, multiple: directory, disabled,
            "webkitdirectory": directory.then_some(""), aria_label: aria_label.unwrap_or(label),
            onmounted: move |event: MountedEvent| input.set(event.data().downcast::<web_sys::Element>().and_then(|e| e.dyn_ref::<web_sys::HtmlInputElement>()).cloned()),
            onchange: move |event: FormEvent| {
                let Some(input) = event.data().try_as_web_event().and_then(|e| e.target()).and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok()) else { return; };
                let files: Vec<_> = input.files().map(|list| (0..list.length()).filter_map(|i| list.get(i)).collect()).unwrap_or_default();
                input.set_value("");
                if !files.is_empty() { on_files.call(files); }
            },
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn declarative_molecules_render_short_long_select_and_sections_on_host() {
        fn fixture() -> Element {
            let long: Vec<_> = (0..14)
                .map(|i| (format!("v{i}"), format!("Opt {i}")))
                .collect();
            rsx! {
                InspectorSection { title: "Geometry", badge: "Custom", on_reset: move |_| {},
                    NumericField { label: "Width", unit: "mm", value: 19.05, min: Some(1.0), max: Some(50.0), on_commit: |_| {} }
                    SelectField { label: "Side", value: "front".to_owned(), options: vec![("front".into(), "Front".into())], on_commit: |_| {} }
                    SelectField { label: "Preset", value: "".to_owned(), placeholder: "Choose...", options: long, on_commit: |_| {} }
                    ToggleField { label: "Enabled", checked: true, on_commit: |_| {} }
                    TextCommitField { label: "Name", value: "Main", on_commit: |_| {} }
                }
            }
        }
        let mut dom = VirtualDom::new(fixture);
        dom.rebuild_in_place();
    }
}

/// Commit-row behavior: dirty-gated idempotent commits, Escape revert, and
/// inline `FieldError` refusal. Behavior runs through the real commit
/// functions; error wiring renders through a real `VirtualDom`.
#[cfg(test)]
mod commit_field_tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    fn validate(raw: &str) -> Option<String> {
        parse_clamped_f64(raw, Some(1.0), Some(50.0))
            .is_none()
            .then(|| "Enter a valid number.".to_owned())
    }

    #[test]
    fn numeric_parse_clamp_and_format_behave_cleanly() {
        assert_eq!(
            parse_clamped_f64(" 12.5 ", Some(0.0), Some(20.0)),
            Some(12.5)
        );
        assert_eq!(parse_clamped_f64("-4", Some(0.0), Some(10.0)), Some(0.0));
        assert_eq!(parse_clamped_f64("99", Some(0.0), Some(10.0)), Some(10.0));
        assert_eq!(parse_clamped_f64("NaN", None, None), None);
        assert_eq!(format_numeric(19.0, 3), "19");
        assert_eq!(format_numeric(19.050, 3), "19.05");
        assert_eq!(format_numeric(-0.0, 2), "0");
    }

    thread_local! {
        static STASH: RefCell<Option<(CommitState, Signal<String>)>> =
            const { RefCell::new(None) };
    }

    fn fixture() -> Element {
        let state = CommitState {
            dirty: use_signal(|| false),
            focused: use_signal(|| false),
            failed_at: use_signal(|| None),
        };
        let draft = use_signal(|| "19.0".to_owned());
        STASH.set(Some((state, draft)));
        rsx! {}
    }

    fn harness() -> (
        VirtualDom,
        CommitState,
        Signal<String>,
        Rc<RefCell<Vec<String>>>,
    ) {
        let mut dom = VirtualDom::new(fixture);
        dom.rebuild_in_place();
        let (state, draft) = STASH.take().expect("fixture stashes commit state");
        (dom, state, draft, Rc::new(RefCell::new(Vec::new())))
    }

    fn attempt(state: CommitState, draft: Signal<String>, commits: &Rc<RefCell<Vec<String>>>) {
        let pending = commits.clone();
        attempt_commit(state, draft, "19.0", &validate, |next| {
            pending.borrow_mut().push(next)
        });
    }

    #[test]
    fn enter_then_blur_commits_once() {
        let (_dom, mut state, mut draft, commits) = harness();
        state.dirty.set(true);
        draft.set("20".into());
        attempt(state, draft, &commits);
        attempt(state, draft, &commits);
        assert_eq!(
            commits.borrow().as_slice(),
            ["20"],
            "Enter commit must not double-fire on blur"
        );
    }

    #[test]
    fn escape_reverts_without_commit() {
        let (_dom, mut state, mut draft, commits) = harness();
        state.dirty.set(true);
        draft.set("99".into());
        revert_commit(state, draft, "19.0");
        attempt(state, draft, &commits);
        assert!(
            commits.borrow().is_empty(),
            "Escape must revert without committing"
        );
        assert_eq!(draft(), "19.0");
        assert_eq!(state.failed_at.peek().clone(), None);
    }

    #[test]
    fn invalid_refusal_keeps_draft_and_valid_retry_commits() {
        let (_dom, mut state, mut draft, commits) = harness();
        state.dirty.set(true);
        draft.set("abc".into());
        attempt(state, draft, &commits);
        assert!(commits.borrow().is_empty(), "invalid input must not commit");
        assert_eq!(draft(), "abc");
        assert_eq!(state.failed_at.peek().clone(), Some("19.0".to_owned()));
        state.dirty.set(true);
        draft.set("20".into());
        attempt(state, draft, &commits);
        assert_eq!(
            commits.borrow().as_slice(),
            ["20"],
            "valid follow-up must commit"
        );
        assert_eq!(state.failed_at.peek().clone(), None);
    }

    #[test]
    fn external_error_renders_field_error_wiring_and_tracks_accepted() {
        #[component]
        fn Probe(value: String) -> Element {
            rsx! {
                TextCommitField {
                    label: "Name",
                    id: "probe-text",
                    value,
                    error: "Required",
                    on_commit: |_| {},
                }
            }
        }
        fn fixture() -> Element {
            let mut val = use_signal(|| "Initial".to_owned());
            if val.peek().as_str() == "Initial" {
                val.set("Updated".to_owned());
            }
            rsx! { Probe { value: val() } }
        }
        let mut dom = VirtualDom::new(fixture);
        let mut mutations = dioxus_core::Mutations::default();
        dom.rebuild(&mut mutations);
        dom.render_immediate(&mut mutations);
        let rendered = format!("{mutations:?}");
        assert!(
            rendered.contains("Updated"),
            "unfocused field with external error must track accepted updates: {rendered}"
        );
        let dom_view = format!("{:?}", dom.rebuild_to_vec());
        assert!(
            dom_view.contains("aria-invalid"),
            "error flags aria-invalid"
        );
        assert!(
            dom_view.contains("probe-text-error"),
            "error links describedby"
        );
    }

    #[test]
    fn field_error_marks_data_slot() {
        let node = FieldError(crate::ui::field::FieldErrorProps {
            class: None,
            children: None,
            errors: Some(vec!["boom".into()]),
        });
        let rendered = format!("{node:?}");
        assert!(
            rendered.contains("field-error"),
            "FieldError marks data-slot: {rendered}"
        );
    }
}

//! Reusable declarative inspector field molecules and temporary draft-input adapters.
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

const TOUCH_CLS: &str =
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

fn commit_field_row(
    id: String,
    label: String,
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
    on_commit_text: EventHandler<String>,
) -> Element {
    let mut focused = use_signal(|| false);
    let mut draft = use_signal(|| accepted.clone());
    let shown = if focused() { draft() } else { accepted.clone() };
    let disabled = disabled_reason.is_some();
    let help = disabled_reason.or(description);
    let (on_f, on_e) = (accepted.clone(), accepted);
    rsx! {
        Field { disabled, class: "gap-1 min-w-0",
            FieldLabel { html_for: id.clone(), "{label}" }
            InputGroup {
                InputGroupInput {
                    id, r#type, class: TOUCH_CLS, value: "{shown}", disabled, placeholder,
                    min, max, step, aria_label: aria_label.unwrap_or(label),
                    oninput: move |e: FormEvent| { focused.set(true); draft.set(e.value()); },
                    attributes: vec![
                        dioxus::events::onfocus(move |_| { draft.set(on_f.clone()); focused.set(true); }),
                        dioxus::events::onblur(move |_| { focused.set(false); on_commit_text.call(draft.peek().clone()); }),
                        dioxus::events::onkeydown(move |e: KeyboardEvent| match e.key() {
                            Key::Enter => { e.prevent_default(); focused.set(false); on_commit_text.call(draft.peek().clone()); }
                            Key::Escape => { e.prevent_default(); draft.set(on_e.clone()); focused.set(false); }
                            _ => {}
                        }),
                    ],
                }
                if let Some(u) = unit { InputGroupAddon { align: InputGroupAddonAlign::InlineEnd, InputGroupText { "{u}" } } }
            }
            if let Some(help) = help { FieldDescription { "{help}" } }
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
    on_commit: EventHandler<f64>,
) -> Element {
    let id = id.unwrap_or_else(|| format!("inspector-num-{}", dioxus::core::current_scope_id().0));
    let accepted = if mixed || !value.is_finite() {
        String::new()
    } else {
        format_numeric(value, precision)
    };
    commit_field_row(
        id,
        label,
        InputType::Number,
        accepted,
        placeholder.or_else(|| mixed.then(|| "Mixed".to_owned())),
        aria_label,
        disabled_reason,
        description,
        unit,
        min.map(|v| format_numeric(v, precision)),
        max.map(|v| format_numeric(v, precision)),
        step.map(|v| format_numeric(v, precision))
            .or_else(|| Some("any".to_owned())),
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
    #[props(into, optional)] placeholder: Option<String>,
    #[props(into, optional)] disabled_reason: Option<String>,
    #[props(into, optional)] id: Option<String>,
    #[props(into, optional)] aria_label: Option<String>,
    on_commit: EventHandler<String>,
) -> Element {
    let id = id.unwrap_or_else(|| format!("inspector-text-{}", dioxus::core::current_scope_id().0));
    let current = value.clone();
    commit_field_row(
        id,
        label,
        InputType::Text,
        value,
        placeholder,
        aria_label,
        disabled_reason,
        None,
        None,
        None,
        None,
        None,
        EventHandler::new(move |raw: String| {
            let next = raw.trim().to_owned();
            if !next.is_empty() && next != current {
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
    let is_open = open.unwrap_or_else(|| local_open());
    rsx! {
        Collapsible {
            keep_mounted: true, open: Some(is_open),
            on_open_change: move |next| { local_open.set(next); if let Some(h) = on_open_change { h.call(next); } },
            div { class: "flex items-center justify-between gap-2",
                CollapsibleTrigger { class: "flex-1 justify-start {TOUCH_CLS}",
                    span { "{title}" }
                    if let Some(badge) = badge { Badge { variant: BadgeVariant::Secondary, "{badge}" } }
                }
                if let Some(reset) = on_reset {
                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, onclick: move |_| reset.call(()), "Reset" }
                }
            }
            CollapsibleContent { class: "data-[open=false]:hidden", FieldGroup { {children} } }
        }
    }
}

#[derive(Default)]
struct DraftSpec {
    unit: Option<String>,
    description: Option<String>,
    error: Option<String>,
    disabled: bool,
    readonly: bool,
    required: bool,
    aria_label: Option<String>,
    placeholder: Option<String>,
    min: Option<String>,
    max: Option<String>,
    step: Option<String>,
    maxlength: Option<String>,
    class: Option<String>,
    control_class: Option<String>,
    aria_invalid: Option<bool>,
    message_id: Option<String>,
    plain_label: bool,
}

fn draft_input_row(
    id: String,
    label: String,
    r#type: InputType,
    value: String,
    s: DraftSpec,
    oninput: EventHandler<FormEvent>,
    onblur: EventHandler<FocusEvent>,
    onkeydown: EventHandler<KeyboardEvent>,
) -> Element {
    let message_id = s.message_id.unwrap_or_else(|| format!("{id}-message"));
    let has_error = s.error.is_some();
    let mut extra = vec![
        dioxus::events::onblur(move |e| onblur.call(e)),
        dioxus::events::onkeydown(move |e| onkeydown.call(e)),
    ];
    if let Some(ml) = s.maxlength {
        extra.push(Attribute::new("maxlength", ml.as_str(), None, false));
    }
    rsx! {
        Field { invalid: has_error, disabled: s.disabled, class: s.class.as_deref().unwrap_or("gap-1 min-w-0"),
            if s.plain_label { Label { html_for: id.clone(), "{label}" } } else { FieldLabel { html_for: id.clone(), "{label}" } }
            InputGroup { class: s.control_class.clone(),
                InputGroupInput {
                    id, r#type, class: s.control_class.as_deref().unwrap_or(TOUCH_CLS), value: "{value}",
                    disabled: s.disabled, readonly: s.readonly, required: s.required, placeholder: s.placeholder,
                    min: s.min, max: s.max, step: s.step, aria_label: s.aria_label,
                    aria_invalid: has_error.then_some(true).or(s.aria_invalid),
                    aria_describedby: has_error.then(|| message_id.clone()),
                    oninput: move |e| oninput.call(e), attributes: extra,
                }
                if let Some(u) = s.unit { InputGroupAddon { align: InputGroupAddonAlign::InlineEnd, InputGroupText { "{u}" } } }
            }
            if let Some(desc) = s.description { FieldDescription { "{desc}" } }
            if let Some(err) = s.error { div { id: message_id, FieldError { "{err}" } } }
        }
    }
}

#[component]
pub fn DraftNumericField(
    id: String,
    label: String,
    value: String,
    oninput: EventHandler<FormEvent>,
    #[props(default)] unit: Option<String>,
    #[props(default)] description: Option<String>,
    #[props(default)] error: Option<String>,
    #[props(default)] disabled: bool,
    #[props(into, optional)] aria_label: Option<String>,
    #[props(into, optional)] placeholder: Option<String>,
    #[props(into, optional)] min: Option<String>,
    #[props(into, optional)] max: Option<String>,
    #[props(into, default = Some("any".to_owned()))] step: Option<String>,
    #[props(default)] readonly: bool,
    #[props(default)] required: bool,
    #[props(into, optional)] class: Option<String>,
    #[props(into, optional)] control_class: Option<String>,
    #[props(default = Some(false))] aria_invalid: Option<bool>,
    #[props(into, optional)] message_id: Option<String>,
    #[props(default)] onblur: EventHandler<FocusEvent>,
    #[props(default)] onkeydown: EventHandler<KeyboardEvent>,
) -> Element {
    draft_input_row(
        id,
        label,
        InputType::Number,
        value,
        DraftSpec {
            unit,
            description,
            error,
            disabled,
            readonly,
            required,
            aria_label,
            placeholder,
            min,
            max,
            step,
            class,
            control_class,
            aria_invalid,
            message_id,
            ..Default::default()
        },
        oninput,
        onblur,
        onkeydown,
    )
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
    draft_input_row(
        id,
        label,
        r#type,
        value,
        DraftSpec {
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
            ..Default::default()
        },
        oninput,
        onblur,
        onkeydown,
    )
}

#[component]
pub fn DraftSelectField(
    label: String,
    value: String,
    options: Vec<(String, String)>,
    on_select: EventHandler<String>,
    #[props(into, optional)] id: Option<String>,
    #[props(into, optional)] aria_label: Option<String>,
    #[props(default)] disabled: bool,
    #[props(into, optional)] class: Option<String>,
    #[props(default = true)] touch_targets: bool,
) -> Element {
    let _ = (class, touch_targets);
    rsx! { SelectField::<String> { label, value, options, id, aria_label, disabled, on_commit: on_select } }
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

#[component]
pub fn AxisGrid(children: Element) -> Element {
    rsx! { div { class: "grid grid-cols-3 gap-1.5 min-w-0", {children} } }
}

#[component]
pub fn AxisField(
    axis: &'static str,
    value: String,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] min: Option<String>,
    #[props(default)] error: Option<String>,
    #[props(default)] oninput: EventHandler<FormEvent>,
    #[props(default)] onblur: EventHandler<FocusEvent>,
    #[props(default)] onkeydown: EventHandler<KeyboardEvent>,
) -> Element {
    let id = format!(
        "inspector-axis-{}-{}",
        axis.to_lowercase(),
        dioxus::core::current_scope_id().0
    );
    draft_input_row(
        id,
        axis.to_owned(),
        InputType::Number,
        value,
        DraftSpec {
            error,
            disabled,
            min,
            step: Some("0.1".to_owned()),
            aria_label,
            ..Default::default()
        },
        oninput,
        onblur,
        onkeydown,
    )
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

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

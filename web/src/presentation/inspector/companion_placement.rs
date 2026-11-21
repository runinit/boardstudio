use boardstudio_core::model::{
    CompanionPlacementTarget, MirroredCompanionMode, MirroredCompanionTweak,
};
use boardstudio_web_ui_shared::draft_edits::PendingEditResult;
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertDescription, AlertVariant};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::callout::{Callout, CalloutVariant};
use boardstudio_web_ui_shared::ui::checkbox::{Checkbox, CheckboxState};
use boardstudio_web_ui_shared::ui::collapsible::{
    Collapsible, CollapsibleContent, CollapsibleTrigger,
};
use boardstudio_web_ui_shared::ui::label::Label;
use boardstudio_web_ui_shared::ui::{
    field::{Field, FieldLabel},
    select::{Select, SelectOption},
};
use boardstudio_web_ui_shared::ui::{
    input::InputType,
    input_group::{InputGroup, InputGroupInput},
};

use boardstudio_web_ui_shared::ui::icons::ChevronDown;
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::{cell::RefCell, rc::Rc};

pub use boardstudio_web_layout::inspector_types::{
    CompanionPlacementOwner, CompanionPlacementProjection,
};

#[derive(Clone, Debug, PartialEq)]
pub struct CompanionPlacementAction {
    pub owner: CompanionPlacementOwner,
    pub pair_id: String,
    pub targets: Vec<CompanionPlacementTarget>,
    pub mode: MirroredCompanionMode,
    pub tweak: Option<MirroredCompanionTweak>,
}

#[derive(Props, Clone, PartialEq)]
pub struct CompanionPlacementInspectorProps {
    pub projection: CompanionPlacementProjection,
    pub pending: Signal<bool>,
    pub error: Signal<Option<String>>,
    pub pending_edits: Signal<super::super::layout_component_edits::LayoutComponentInspectorEdits>,
    pub on_apply: EventHandler<CompanionPlacementAction>,
}

#[component]
pub fn CompanionPlacementInspector(mut props: CompanionPlacementInspectorProps) -> Element {
    let mut placement_section_open = use_signal(|| false);

    use super::super::layout_component_edits::InspectorField;
    let mut mode = use_signal(|| {
        if props.projection.mixed_mode {
            String::new()
        } else {
            props.projection.mode.map(mode_value).unwrap_or_default()
        }
    });
    let mut pair_default = use_signal(|| false);
    let mut x = use_signal(|| {
        tweak_value(
            props.projection.tweak.map(|tweak| tweak.x),
            props.projection.mixed_tweak,
        )
    });
    let mut y = use_signal(|| {
        tweak_value(
            props.projection.tweak.map(|tweak| tweak.y),
            props.projection.mixed_tweak,
        )
    });
    let mut rotation = use_signal(|| {
        tweak_value(
            props.projection.tweak.map(|tweak| tweak.rotation),
            props.projection.mixed_tweak,
        )
    });
    let mut draft_owner = use_signal(|| props.projection.owner.clone());
    let observed_projection = props.projection.clone();
    use_effect(use_reactive!(|observed_projection| {
        if *draft_owner.peek() != observed_projection.owner {
            draft_owner.set(observed_projection.owner.clone());
            mode.set(if observed_projection.mixed_mode {
                String::new()
            } else {
                observed_projection.mode.map(mode_value).unwrap_or_default()
            });
            pair_default.set(false);
            x.set(tweak_value(
                observed_projection.tweak.map(|tweak| tweak.x),
                observed_projection.mixed_tweak,
            ));
            y.set(tweak_value(
                observed_projection.tweak.map(|tweak| tweak.y),
                observed_projection.mixed_tweak,
            ));
            rotation.set(tweak_value(
                observed_projection.tweak.map(|tweak| tweak.rotation),
                observed_projection.mixed_tweak,
            ));
            props.error.set(None);
        }
    }));
    let pending_edits = props.pending_edits.peek().clone();
    let last_owner = use_hook(|| Rc::new(RefCell::new(None::<CompanionPlacementOwner>)));
    let owner_is_live = {
        let mut previous = last_owner.borrow_mut();
        let current = props.projection.owner.clone();
        let changed = previous.as_ref().is_some_and(|owner| owner != &current);
        *previous = Some(current);
        !changed
    };
    let mut error = props.error;
    let error_messages = (props.error)();
    for result in pending_edits.settle(owner_is_live, |_| String::new()) {
        match result {
            PendingEditResult::Failed {
                key: InspectorField::CompanionPlacement,
                message,
            } => error.set(Some(message)),
            PendingEditResult::Landed {
                key: InspectorField::CompanionPlacement,
                ..
            }
            | PendingEditResult::Retired {
                key: InspectorField::CompanionPlacement,
            } => error.set(None),
            _ => {}
        }
    }
    let retired_pending = pending_edits.clone();
    use_drop(move || {
        retired_pending.settle(false, |_| String::new());
    });

    let selected_mode = parse_mode(&mode());
    let wants_tweak = selected_mode == Some(MirroredCompanionMode::LiteralMirrorWithTweak);
    let tweak = if wants_tweak {
        Some(MirroredCompanionTweak {
            x: x().parse().unwrap_or(f64::NAN),
            y: y().parse().unwrap_or(f64::NAN),
            rotation: rotation().parse().unwrap_or(f64::NAN),
        })
    } else {
        None
    };
    let valid_tweak = tweak.is_none_or(|tweak| {
        tweak.x.is_finite() && tweak.y.is_finite() && tweak.rotation.is_finite()
    });
    let pair_tweak_refusal = pair_default() && tweak.is_some();
    let disabled_reason = props.projection.disabled_reason;
    let disabled = (props.pending)() || disabled_reason.is_some();
    let can_apply = !disabled
        && !props.projection.targets.is_empty()
        && selected_mode.is_some()
        && valid_tweak
        && !pair_tweak_refusal;
    let apply = {
        let projection = props.projection.clone();
        let on_apply = props.on_apply;
        move |_| {
            let Some(mode) = parse_mode(&mode()) else {
                return;
            };
            let mut targets = projection.targets.clone();
            if pair_default() {
                targets.push(CompanionPlacementTarget::Pair);
            }
            targets.sort_by_key(|target| serde_json::to_string(target).unwrap_or_default());
            targets.dedup();
            let tweak = if mode == MirroredCompanionMode::LiteralMirrorWithTweak {
                Some(MirroredCompanionTweak {
                    x: x().parse().unwrap_or(f64::NAN),
                    y: y().parse().unwrap_or(f64::NAN),
                    rotation: rotation().parse().unwrap_or(f64::NAN),
                })
            } else {
                None
            };
            if tweak.is_some_and(|tweak| {
                !tweak.x.is_finite() || !tweak.y.is_finite() || !tweak.rotation.is_finite()
            }) || (pair_default() && tweak.is_some())
            {
                return;
            }
            on_apply.call(CompanionPlacementAction {
                owner: projection.owner.clone(),
                pair_id: projection.pair_id.clone(),
                targets,
                mode,
                tweak,
            });
        }
    };
    let mode_choice = use_memo(move || Some(mode()));
    let summary = match selected_mode {
        Some(MirroredCompanionMode::LiteralMirror) if !props.projection.mixed_mode => {
            "Literal mirror"
        }
        Some(MirroredCompanionMode::DuplicatePosition) if !props.projection.mixed_mode => {
            "Duplicate position"
        }
        Some(MirroredCompanionMode::LiteralMirrorWithTweak) if !props.projection.mixed_mode => {
            "Mirror with tweak"
        }
        _ => "Mixed",
    };

    rsx! {
            Collapsible {
                attributes: vec![dioxus::events::onkeydown(move |event: KeyboardEvent| {
                    if event.key().to_string() == " " {
                        event.stop_propagation();
                        if let Some(raw) = event.data().try_as_web_event() {
                            raw.stop_propagation();
                        }
                    }
                })],
                keep_mounted: true,
                open: Some(placement_section_open()),
                on_open_change: move |next| placement_section_open.set(next),
                CollapsibleTrigger { class: "group min-h-control-touch w-full justify-start px-3 py-2 text-left text-sm font-semibold", aria_label: "Companion placement",
                    span { "Companion placement" }
                    small { "{summary}" }
                 ChevronDown { class: "ml-auto size-4 shrink-0 transition-transform group-data-[state=open]:rotate-180" } }
                CollapsibleContent {
                    class: "data-[open=true]:overflow-visible",
                    hidden: !placement_section_open(),
                    inert: (!placement_section_open()).then_some(""),

                    div {
                        {
                            let control_label = "Mode".to_owned();
                            let control_aria_label = "Mode".to_owned();
                            let control_id = format!(
                                "layout-choice-{}-{}",
                                dioxus::core::current_scope_id().0,
                                control_aria_label.replace(' ', "-"),
                            );

                            let control_choices = [
                                ("".to_owned(), "Mixed values".to_owned(), false),
                                ("literal-mirror".to_owned(), "Literal mirror".to_owned(), false),
                                ("duplicate-position".to_owned(), "Duplicate position".to_owned(), false),
                                (
                                    "literal-mirror-with-tweak".to_owned(),
                                    "Mirror with tweak".to_owned(),
                                    false,
                                ),
                            ];
                            let on_value_change = EventHandler::new(move |choice: Option<String>| {
            let Some(choice) = choice else {
                return;
            };
            mode.set(choice)
        });
        rsx! {

                                Field { class: "",
                                    FieldLabel { html_for: control_id.clone(), "{control_label}" }
                                    Select::<String> {
                                        id: control_id,
                                        aria_label: control_aria_label,
                                        value: Some(mode_choice.into()),
                                        disabled,
                                        on_value_change,
                                        for (index, (value, label, disabled)) in control_choices.iter().enumerate() {
                                            SelectOption::<String> {
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
                        if let Some(reason) = disabled_reason {
                            div { role: "status",  Callout {  variant: CalloutVariant::Info,  "{reason}"  } }
                        } else {
                            if wants_tweak {
                                {
                                    let control_input_id = format!(
                                        "layout-input-{}-3",
                                        dioxus::core::current_scope_id().0,
                                    );
                                    rsx! {
                                        Field {
                                            FieldLabel { html_for: control_input_id.clone(), "X tweak (mm)" }
                                            InputGroup {
                                                InputGroupInput { r#type: InputType::Number,
    step: "any",
    placeholder: if props.projection.mixed_tweak { "Mixed values" } else { "0" },
    value: "{x()}",
    disabled: disabled,
    oninput: move |event: FormEvent| x.set(event.value()),
    id: control_input_id.clone(), }
                                            }
                                        }
                                    }
                                }
                                {
                                    let control_input_id = format!(
                                        "layout-input-{}-2",
                                        dioxus::core::current_scope_id().0,
                                    );
                                    rsx! {
                                        Field {
                                            FieldLabel { html_for: control_input_id.clone(), "Y tweak (mm)" }
                                            InputGroup {
                                                InputGroupInput { r#type: InputType::Number,
    step: "any",
    placeholder: if props.projection.mixed_tweak { "Mixed values" } else { "0" },
    value: "{y()}",
    disabled: disabled,
    oninput: move |event: FormEvent| y.set(event.value()),
    id: control_input_id.clone(), }
                                            }
                                        }
                                    }
                                }
                                {
                                    let control_input_id = format!(
                                        "layout-input-{}-1",
                                        dioxus::core::current_scope_id().0,
                                    );
                                    rsx! {
                                        Field {
                                            FieldLabel { html_for: control_input_id.clone(), "Rotation tweak (degrees)" }
                                            InputGroup {
                                                InputGroupInput { r#type: InputType::Number,
    step: "any",
    placeholder: if props.projection.mixed_tweak { "Mixed values" } else { "0" },
    value: "{rotation()}",
    disabled: disabled,
    oninput: move |event: FormEvent| rotation.set(event.value()),
    id: control_input_id.clone(), }
                                            }
                                        }
                                    }
                                }
                            }
                            {
                                let check_id = format!(
                                    "layout-check-{}-{}",
                                    dioxus::core::current_scope_id().0,
                                    ("Also set the pair default").replace(' ', "-"),
                                );
                                rsx! {
                                    Label {
                                        html_for: check_id.clone(),
                                        Checkbox {
                                            id: check_id.clone(),
                                            checked: Some(
                                                if pair_default() {
                                                    CheckboxState::Checked
                                                } else {
                                                    CheckboxState::Unchecked
                                                },
                                            ),
                                            disabled,
                                            on_checked_change: move |checked: CheckboxState| {
                                                pair_default.set(bool::from(checked))
                                            },
                                            aria_label: "Also set the pair default",
                                        }
                                        "Also set the pair default"
                                    }
                                }
                            }
                            if pair_tweak_refusal {
                                div { role: "status",  Callout {  variant: CalloutVariant::Info,  "Pair defaults cannot carry a position tweak."  } }
                            }
                        }
                        if let Some(message) = error_messages.as_ref() {
                            Alert { variant: AlertVariant::Destructive,AlertDescription {  "{message}"  } }
                        }
                        if disabled_reason.is_none() {
                            Button {
                                variant: ButtonVariant::Secondary,
                                button_type: "button",
                                disabled: !can_apply,
                                onclick: apply,
                                "Apply placement"
                            }
                        }
                    }
                }
            }
        }
}

fn mode_value(mode: MirroredCompanionMode) -> String {
    match mode {
        MirroredCompanionMode::LiteralMirror => "literal-mirror".into(),
        MirroredCompanionMode::DuplicatePosition => "duplicate-position".into(),
        MirroredCompanionMode::LiteralMirrorWithTweak => "literal-mirror-with-tweak".into(),
    }
}

fn parse_mode(value: &str) -> Option<MirroredCompanionMode> {
    match value {
        "literal-mirror" => Some(MirroredCompanionMode::LiteralMirror),
        "duplicate-position" => Some(MirroredCompanionMode::DuplicatePosition),
        "literal-mirror-with-tweak" => Some(MirroredCompanionMode::LiteralMirrorWithTweak),
        _ => None,
    }
}

fn tweak_value(value: Option<f64>, mixed: bool) -> String {
    if mixed {
        String::new()
    } else {
        value.map_or_else(|| "0".into(), |value| value.to_string())
    }
}

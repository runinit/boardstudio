use boardstudio_core::model::{
    CompanionPlacementTarget, MirroredCompanionMode, MirroredCompanionTweak,
};
use boardstudio_web_ui_shared::inspector_fields::{
    CheckboxField, DraftNumericField as NumericField, DraftSelectField as SelectField,
};
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertDescription, AlertVariant};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::callout::{Callout, CalloutVariant};
use boardstudio_web_ui_shared::ui::checkbox::CheckboxState;
use boardstudio_web_ui_shared::ui::collapsible::{
    Collapsible, CollapsibleContent, CollapsibleTrigger,
};

use boardstudio_web_ui_shared::ui::icons::ChevronDown;
use dioxus::prelude::*;
use dioxus_web::WebEventExt;

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
    pub error: Signal<Option<String>>,
    pub on_apply: EventHandler<CompanionPlacementAction>,
}

#[component]
pub fn CompanionPlacementInspector(mut props: CompanionPlacementInspectorProps) -> Element {
    let mut placement_section_open = use_signal(|| false);

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
    let error_messages = (props.error)();

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
    let disabled = disabled_reason.is_some();
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
                        let id = format!("layout-choice-{}-Mode", dioxus::core::current_scope_id().0);
                        rsx! {
                            SelectField {
                                id, label: "Mode", value: mode(), disabled,
                                class: "", touch_targets: false,
                                options: vec![
                                    ("".into(), "Mixed values".into()),
                                    ("literal-mirror".into(), "Literal mirror".into()),
                                    ("duplicate-position".into(), "Duplicate position".into()),
                                    ("literal-mirror-with-tweak".into(), "Mirror with tweak".into()),
                                ],
                                on_select: move |choice: String| mode.set(choice),
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
                                    NumericField {
                                        id: control_input_id, label: "X tweak (mm)", class: "", aria_invalid: None,
                                        control_class: "",
                                        placeholder: if props.projection.mixed_tweak { "Mixed values" } else { "0" },
                                        value: "{x()}",
                                        disabled,
                                        oninput: move |event: FormEvent| x.set(event.value()),
                                    }
                                }
                            }
                            {
                                let control_input_id = format!(
                                    "layout-input-{}-2",
                                    dioxus::core::current_scope_id().0,
                                );
                                rsx! {
                                    NumericField {
                                        id: control_input_id, label: "Y tweak (mm)", class: "", aria_invalid: None,
                                        control_class: "",
                                        placeholder: if props.projection.mixed_tweak { "Mixed values" } else { "0" },
                                        value: "{y()}",
                                        disabled,
                                        oninput: move |event: FormEvent| y.set(event.value()),
                                    }
                                }
                            }
                            {
                                let control_input_id = format!(
                                    "layout-input-{}-1",
                                    dioxus::core::current_scope_id().0,
                                );
                                rsx! {
                                    NumericField {
                                        id: control_input_id, label: "Rotation tweak (degrees)", class: "", aria_invalid: None,
                                        control_class: "",
                                        placeholder: if props.projection.mixed_tweak { "Mixed values" } else { "0" },
                                        value: "{rotation()}",
                                        disabled,
                                        oninput: move |event: FormEvent| rotation.set(event.value()),
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
                                CheckboxField {
                                    inline: true, label: "Also set the pair default",
                                    id: check_id,
                                    checked: if pair_default() {
                                            CheckboxState::Checked
                                        } else {
                                            CheckboxState::Unchecked
                                        },
                                    disabled,
                                    on_checked_change: move |checked: CheckboxState| {
                                        pair_default.set(bool::from(checked))
                                    },
                                    aria_label: "Also set the pair default",
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

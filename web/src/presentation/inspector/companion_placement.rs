use boardstudio_core::model::{
    CompanionPlacementTarget, MirroredCompanionMode, MirroredCompanionTweak,
};
use boardstudio_web_ui_shared::inspector_fields::{
    InspectorSection, NumericField, SelectField, ToggleField,
};
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertDescription, AlertVariant},
    button::{Button, ButtonVariant},
    callout::{Callout, CalloutVariant},
};
use dioxus::prelude::*;

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
    let mut mode = use_signal(|| {
        if props.projection.mixed_mode {
            String::new()
        } else {
            props.projection.mode.map(mode_value).unwrap_or_default()
        }
    });
    let mut pair_default = use_signal(|| false);
    let mut x = use_signal(|| {
        if props.projection.mixed_tweak {
            f64::NAN
        } else {
            props.projection.tweak.map_or(0.0, |t| t.x)
        }
    });
    let mut y = use_signal(|| {
        if props.projection.mixed_tweak {
            f64::NAN
        } else {
            props.projection.tweak.map_or(0.0, |t| t.y)
        }
    });
    let mut rotation = use_signal(|| {
        if props.projection.mixed_tweak {
            f64::NAN
        } else {
            props.projection.tweak.map_or(0.0, |t| t.rotation)
        }
    });
    let mut draft_owner = use_signal(|| props.projection.owner.clone());
    let obs = props.projection.clone();
    use_effect(use_reactive!(|obs| {
        if *draft_owner.peek() != obs.owner {
            draft_owner.set(obs.owner.clone());
            mode.set(if obs.mixed_mode {
                String::new()
            } else {
                obs.mode.map(mode_value).unwrap_or_default()
            });
            pair_default.set(false);
            x.set(if obs.mixed_tweak {
                f64::NAN
            } else {
                obs.tweak.map_or(0.0, |t| t.x)
            });
            y.set(if obs.mixed_tweak {
                f64::NAN
            } else {
                obs.tweak.map_or(0.0, |t| t.y)
            });
            rotation.set(if obs.mixed_tweak {
                f64::NAN
            } else {
                obs.tweak.map_or(0.0, |t| t.rotation)
            });
            props.error.set(None);
        }
    }));

    let selected_mode = parse_mode(&mode());
    let wants_tweak = selected_mode == Some(MirroredCompanionMode::LiteralMirrorWithTweak);
    let tweak = wants_tweak.then(|| MirroredCompanionTweak {
        x: x(),
        y: y(),
        rotation: rotation(),
    });
    let valid_tweak =
        tweak.is_none_or(|t| t.x.is_finite() && t.y.is_finite() && t.rotation.is_finite());
    let pair_tweak_refusal = pair_default() && tweak.is_some();
    let disabled_reason = props.projection.disabled_reason;
    let can_apply = disabled_reason.is_none()
        && !props.projection.targets.is_empty()
        && selected_mode.is_some()
        && valid_tweak
        && !pair_tweak_refusal;
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
    let (proj, on_apply) = (props.projection.clone(), props.on_apply);
    rsx! {
        InspectorSection { title: "Companion placement", badge: summary, default_open: false,
            SelectField::<String> {
                label: "Mode".to_owned(), value: mode(), disabled: disabled_reason.is_some(),
                options: vec![
                    ("".into(), "Mixed values".into()),
                    ("literal-mirror".into(), "Literal mirror".into()),
                    ("duplicate-position".into(), "Duplicate position".into()),
                    ("literal-mirror-with-tweak".into(), "Mirror with tweak".into()),
                ],
                on_commit: move |c: String| mode.set(c),
            }
            if let Some(reason) = disabled_reason {
                div { role: "status", Callout { variant: CalloutVariant::Info, "{reason}" } }
            } else {
                if wants_tweak {
                    NumericField { label: "X tweak".to_owned(), unit: "mm", value: x(), mixed: props.projection.mixed_tweak && !x().is_finite(), on_commit: move |v| x.set(v) }
                    NumericField { label: "Y tweak".to_owned(), unit: "mm", value: y(), mixed: props.projection.mixed_tweak && !y().is_finite(), on_commit: move |v| y.set(v) }
                    NumericField { label: "Rotation tweak".to_owned(), unit: "°", value: rotation(), mixed: props.projection.mixed_tweak && !rotation().is_finite(), on_commit: move |v| rotation.set(v) }
                }
                ToggleField { label: "Also set the pair default".to_owned(), checked: pair_default(), on_commit: move |v| pair_default.set(v) }
                if pair_tweak_refusal {
                    div { role: "status", Callout { variant: CalloutVariant::Info, "Pair defaults cannot carry a position tweak." } }
                }
            }
            if let Some(msg) = (props.error)() { Alert { variant: AlertVariant::Destructive, AlertDescription { "{msg}" } } }
            if disabled_reason.is_none() {
                Button {
                    variant: ButtonVariant::Secondary, button_type: "button", disabled: !can_apply,
                    onclick: move |_| {
                        let Some(mode) = parse_mode(&mode()) else { return; };
                        let mut targets = proj.targets.clone();
                        if pair_default() { targets.push(CompanionPlacementTarget::Pair); }
                        targets.sort_by_key(|t| serde_json::to_string(t).unwrap_or_default());
                        targets.dedup();
                        let tw = (mode == MirroredCompanionMode::LiteralMirrorWithTweak)
                            .then(|| MirroredCompanionTweak { x: x(), y: y(), rotation: rotation() });
                        if tw.is_some_and(|t| !t.x.is_finite() || !t.y.is_finite() || !t.rotation.is_finite()) || (pair_default() && tw.is_some()) { return; }
                        on_apply.call(CompanionPlacementAction { owner: proj.owner.clone(), pair_id: proj.pair_id.clone(), targets, mode, tweak: tw });
                    },
                    "Apply placement"
                }
            }
        }
    }
}

fn mode_value(mode: MirroredCompanionMode) -> String {
    match mode {
        MirroredCompanionMode::LiteralMirror => "literal-mirror",
        MirroredCompanionMode::DuplicatePosition => "duplicate-position",
        MirroredCompanionMode::LiteralMirrorWithTweak => "literal-mirror-with-tweak",
    }
    .into()
}

fn parse_mode(value: &str) -> Option<MirroredCompanionMode> {
    match value {
        "literal-mirror" => Some(MirroredCompanionMode::LiteralMirror),
        "duplicate-position" => Some(MirroredCompanionMode::DuplicatePosition),
        "literal-mirror-with-tweak" => Some(MirroredCompanionMode::LiteralMirrorWithTweak),
        _ => None,
    }
}

use boardstudio_core::model::{
    CompanionPlacementTarget, MirroredCompanionMode, MirroredCompanionTweak,
};
use boardstudio_web_runtime::pending_edits::PendingEditResult;
use boardstudio_web_ui_shared::controls::BoardSelectField;
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompanionPlacementOwner {
    pub scope: boardstudio_application::Scope,
    pub context: crate::presentation::objects::ScopedTreeContext,
    pub selected_part_ids: Vec<String>,
    pub snapshot_token: boardstudio_application::SnapshotToken,
    pub revision: u64,
    pub context_generation: u64,
    pub scope_generation: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompanionPlacementProjection {
    pub owner: CompanionPlacementOwner,
    pub pair_id: String,
    pub targets: Vec<CompanionPlacementTarget>,
    pub mode: Option<MirroredCompanionMode>,
    pub mixed_mode: bool,
    pub tweak: Option<MirroredCompanionTweak>,
    pub mixed_tweak: bool,
    pub disabled_reason: Option<&'static str>,
}

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
    pending_edits.bind_one_shot(InspectorField::CompanionPlacement, props.pending);
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
        retired_pending.unbind_one_shot(&InspectorField::CompanionPlacement);
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
        details { class: "m1-layout-companion-placement",
            summary { span { "Companion placement" } small { "{summary}" } }
            div { class: "m1-layout-component-fields",
                BoardSelectField {
                    label: "Mode",
                    value: "{mode()}",
                    disabled,
                        onchange: move |event: FormEvent| mode.set(event.value()),
                        option { value: "", "Mixed values" }
                        option { value: "literal-mirror", "Literal mirror" }
                        option { value: "duplicate-position", "Duplicate position" }
                        option { value: "literal-mirror-with-tweak", "Mirror with tweak" }
                }
                if let Some(reason) = disabled_reason {
                    p { role: "status", "{reason}" }
                } else {
                    if wants_tweak {
                        label { "X tweak (mm)" input { r#type: "number", step: "any", placeholder: if props.projection.mixed_tweak { "Mixed values" } else { "0" }, value: "{x()}", disabled, oninput: move |event| x.set(event.value()) } }
                        label { "Y tweak (mm)" input { r#type: "number", step: "any", placeholder: if props.projection.mixed_tweak { "Mixed values" } else { "0" }, value: "{y()}", disabled, oninput: move |event| y.set(event.value()) } }
                        label { "Rotation tweak (degrees)" input { r#type: "number", step: "any", placeholder: if props.projection.mixed_tweak { "Mixed values" } else { "0" }, value: "{rotation()}", disabled, oninput: move |event| rotation.set(event.value()) } }
                    }
                    label { class: "m1-layout-companion-placement-default",
                        input { r#type: "checkbox", checked: pair_default(), disabled, onchange: move |event| pair_default.set(event.checked()) }
                        "Also set the pair default"
                    }
                    if pair_tweak_refusal {
                        p { role: "status", "Pair defaults cannot carry a position tweak." }
                    }
                }
                if let Some(message) = error_messages.as_ref() { p { role: "alert", "{message}" } }
                if disabled_reason.is_none() {
                    button { r#type: "button", disabled: !can_apply, onclick: apply, "Apply placement" }
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

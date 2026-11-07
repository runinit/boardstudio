use boardstudio_core::model::{
    CompanionPlacementTarget, MirroredCompanionMode, MirroredCompanionTweak,
};
use boardstudio_web_runtime::pending_edits::PendingEditResult;
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

use crate::runtime::Runtime;
use boardstudio_web_ui_shared::ui::icons::ChevronDown;
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
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

// Accepted revisions refresh admission, but do not create a new draft owner.
#[derive(Clone, Debug, PartialEq, Eq)]
struct CompanionPlacementBinding {
    context: crate::presentation::objects::ScopedTreeContext,
    selected_part_ids: Vec<String>,
    context_generation: u64,
    scope_generation: u64,
    pair_id: String,
    targets: String,
}

impl CompanionPlacementProjection {
    fn binding(&self) -> CompanionPlacementBinding {
        CompanionPlacementBinding {
            context: self.owner.context.clone(),
            selected_part_ids: self.owner.selected_part_ids.clone(),
            context_generation: self.owner.context_generation,
            scope_generation: self.owner.scope_generation,
            pair_id: self.pair_id.clone(),
            targets: companion_targets_canonical(&self.targets, false),
        }
    }
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
    let mut placement_section_open = use_signal(|| false);
    let binding = props.projection.binding();

    use super::super::layout_component_edits::InspectorField;
    let mut mode = use_signal(|| {
        if props.projection.mixed_mode {
            String::new()
        } else {
            props.projection.mode.map(mode_value).unwrap_or_default()
        }
    });
    let mut pair_default = use_signal(|| false);
    // 004's Pair-only amendment: keep the original sticky parameter; advance
    // its safety baseline only from this binding's own successful submission.
    let mut pair_baseline = use_signal(|| false);
    let mut submitted_pair = use_signal(|| None::<(CompanionPlacementBinding, bool)>);
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
    let mut draft_owner = use_signal(|| binding.clone());
    // Ticket 011: the canonical value the Apply button last submitted (set only
    // on the validated path), the previously seen accepted baseline for the
    // clean-gated follow, and the draft-owner registry guard.
    // Production mounts inherit WorkbenchApp's Runtime provider; isolated
    // display-only fixtures may omit it without registering a safety owner.
    let draft_owner_runtime = try_use_context::<Rc<Runtime>>();
    let submitted_companion = use_signal(|| None::<String>);
    let companion_baseline_seen = use_signal(|| companion_baseline_canonical(&props.projection));
    // Ticket 011: the target set the live draft folds the pair default into.
    // This is the rendered form's target value, not its accepted baseline.
    let mut companion_form_targets = use_signal(|| props.projection.targets.clone());
    let draft_owner_guard =
        use_signal(|| None::<boardstudio_web_runtime::draft_owners::DraftOwnerGuard>);
    let draft_owner_scope = draft_owner_runtime.as_ref().and_then(|runtime| {
        runtime
            .model()
            .accepted
            .as_ref()
            .map(|snapshot| (snapshot.document.id.clone(), snapshot.session_epoch))
    });
    let observed_projection = props.projection.clone();
    let mut companion_baseline_seen = companion_baseline_seen;
    use_effect(use_reactive!(|observed_projection| {
        if *draft_owner.peek() != observed_projection.binding() {
            draft_owner.set(observed_projection.binding());
            mode.set(if observed_projection.mixed_mode {
                String::new()
            } else {
                observed_projection.mode.map(mode_value).unwrap_or_default()
            });
            pair_default.set(false);
            pair_baseline.set(false);
            submitted_pair.set(None);
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
            companion_baseline_seen.set(companion_baseline_canonical(&observed_projection));
        }
    }));
    // Ticket 011: settlement-time draft preservation. While the live form still
    // matches the previously seen baseline, a clean form follows a new accepted
    // value; an open draft survives any accepted update until its own Apply
    // lands or the designer restores it. The seen baseline always advances to
    // the current accepted value so a later restore compares against the
    // latest. All reads inside the effect peek so the effect never subscribes
    // to the signals it writes.
    let follow_projection = props.projection.clone();
    let follow_baseline_now = companion_baseline_for_pair(&follow_projection, pair_baseline());
    if follow_projection.targets != *companion_form_targets.peek() {
        companion_form_targets.set(follow_projection.targets.clone());
    }
    use_effect(use_reactive((&follow_baseline_now,), {
        let follow_projection = follow_projection.clone();
        let follow_baseline_now = follow_baseline_now.clone();
        let mut mode = mode;
        let mut x = x;
        let mut y = y;
        let mut rotation = rotation;
        let follow_targets = follow_projection.targets.clone();
        let mut companion_baseline_seen = companion_baseline_seen;
        move |(baseline,)| {
            let _ = baseline;
            let pair = *pair_default.peek();
            let live = companion_state_canonical(
                &mode.peek(),
                pair,
                &x.peek(),
                &y.peek(),
                &rotation.peek(),
                &companion_targets_canonical(&follow_targets, pair),
            );
            if live == *companion_baseline_seen.peek() {
                if live != follow_baseline_now {
                    mode.set(if follow_projection.mixed_mode {
                        String::new()
                    } else {
                        follow_projection.mode.map(mode_value).unwrap_or_default()
                    });
                    x.set(tweak_value(
                        follow_projection.tweak.map(|tweak| tweak.x),
                        follow_projection.mixed_tweak,
                    ));
                    y.set(tweak_value(
                        follow_projection.tweak.map(|tweak| tweak.y),
                        follow_projection.mixed_tweak,
                    ));
                    rotation.set(tweak_value(
                        follow_projection.tweak.map(|tweak| tweak.rotation),
                        follow_projection.mixed_tweak,
                    ));
                }
                companion_baseline_seen.set(follow_baseline_now.clone());
            }
        }
    }));
    let pending_edits = props.pending_edits.peek().clone();
    pending_edits.bind_one_shot(InspectorField::CompanionPlacement, props.pending);
    let last_owner = use_hook(|| Rc::new(RefCell::new(None::<CompanionPlacementBinding>)));
    let owner_is_live = {
        let mut previous = last_owner.borrow_mut();
        let current = binding.clone();
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
            } => {
                submitted_pair.set(None);
                error.set(Some(message));
            }
            PendingEditResult::Landed {
                key: InspectorField::CompanionPlacement,
                ..
            } => {
                if owner_is_live
                    && let Some((owner, pair)) = submitted_pair.write().take()
                    && owner == *draft_owner.peek()
                {
                    pair_baseline.set(pair);
                }
                error.set(None);
            }
            PendingEditResult::Retired {
                key: InspectorField::CompanionPlacement,
            } => {
                submitted_pair.set(None);
                error.set(None);
            }
            _ => {}
        }
    }
    let retired_pending = pending_edits.clone();
    use_drop(move || {
        retired_pending.unbind_one_shot(&InspectorField::CompanionPlacement);
        retired_pending.settle(false, |_| String::new());
    });
    // Ticket 011: register the companion-placement admission key with the
    // Runtime-owned draft registry. Unresolved = the live form differs from
    // the live-projected accepted placement under owner-canonical equality,
    // and no in-flight Apply for this key carries the current draft value.
    // Invalid or refused states never equal the valid baseline, so they defer.
    // The baseline projects the accepted document at each poll through the
    // production projector (an unverifiable or stale projection fails closed);
    // registration rotates when the owner or target set changes so the closure
    // never compares against a stale selection. Hidden and keep-mounted
    // instances keep blocking through the same guard; genuine unmount retires
    // explicitly, and the gate's poll-prune drops anything left dead.
    // Registration is skipped without a Runtime context.
    use_effect(use_reactive((&draft_owner_scope, &binding), {
        let draft_owner_runtime = draft_owner_runtime.clone();
        let pending_edits = pending_edits.clone();
        let probe_owner = props.projection.owner.clone();
        let probe_pair_id = props.projection.pair_id.clone();
        let probe_targets_handle = companion_form_targets;
        let mut draft_owner_guard = draft_owner_guard;
        let draft_owner_scope = draft_owner_scope.clone();
        move |(scope, _binding)| {
            let _ = scope;
            let Some(runtime) = draft_owner_runtime.clone() else {
                return;
            };
            let retired = draft_owner_guard.write().take();
            if let Some(retired) = retired {
                runtime.retire_draft_owner(retired);
            }
            let Some(scope) = draft_owner_scope.clone() else {
                return;
            };
            let probe_runtime = runtime.clone();
            let probe_pending = pending_edits.clone();
            let probe_scope = scope.clone();
            let probe_owner = probe_owner.clone();
            let probe_pair_id = probe_pair_id.clone();
            let is_unresolved = std::rc::Rc::new(move || {
                let pair = *pair_default.peek();
                let live = companion_state_canonical(
                    &mode.peek(),
                    pair,
                    &x.peek(),
                    &y.peek(),
                    &rotation.peek(),
                    &companion_targets_canonical(&probe_targets_handle.peek(), pair),
                );
                let model = probe_runtime.model();
                if model.accepted.as_ref().is_none_or(|snapshot| {
                    snapshot.document.id != probe_scope.0 || snapshot.session_epoch != probe_scope.1
                }) {
                    return false;
                }
                let Some(accepted) = super::super::companion_placement_projection(
                    &model,
                    Some(&probe_owner.context),
                    probe_owner.context_generation,
                    probe_owner.scope_generation,
                ) else {
                    return true;
                };
                if accepted.pair_id != probe_pair_id {
                    return true;
                }
                if live == companion_baseline_for_pair(&accepted, *pair_baseline.peek()) {
                    return false;
                }
                if probe_pending.is_pending(&InspectorField::CompanionPlacement)
                    && submitted_companion.peek().as_ref() == Some(&live)
                {
                    return false;
                }
                true
            });
            draft_owner_guard.set(Some(runtime.register_draft_owner(
                boardstudio_web_runtime::draft_owners::DraftOwnerRegistration {
                    project_scope: scope,
                    workspace: "Layout",
                    owner: "companion-placement",
                    admission_key: "apply",
                    area: "Companion placement".to_owned(),
                    remedy:
                        "Apply the placement, or restore the saved values in the form".to_owned(),
                    is_unresolved,
                },
            )));
        }
    }));
    use_drop({
        let draft_owner_runtime = draft_owner_runtime.clone();
        let mut draft_owner_guard = draft_owner_guard;
        move || {
            if let Some(retired) = draft_owner_guard.write().take()
                && let Some(runtime) = draft_owner_runtime.clone()
            {
                runtime.retire_draft_owner(retired);
            }
        }
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
        let mut submitted_companion = submitted_companion;
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
            // Remember exactly the submitted draft; refusal must preserve the
            // original sticky Pair parameter so the designer can retry it.
            let (submitted_x, submitted_y, submitted_rotation) = tweak
                .map(|tweak| {
                    (
                        tweak.x.to_string(),
                        tweak.y.to_string(),
                        tweak.rotation.to_string(),
                    )
                })
                .unwrap_or(("none".to_owned(), "none".to_owned(), "none".to_owned()));
            submitted_pair.set(Some((projection.binding(), pair_default())));
            submitted_companion.set(Some(companion_state_canonical(
                &mode_value(mode),
                targets
                    .iter()
                    .any(|target| matches!(target, CompanionPlacementTarget::Pair)),
                &submitted_x,
                &submitted_y,
                &submitted_rotation,
                &companion_targets_canonical(&targets, false),
            )));
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

/// Ticket 011: one tweak number the way the Apply path parses it. A finite
/// parse encodes as its exact round-trip text; anything else encodes
/// distinctly from every valid baseline, so realistic invalid tweaks defer.
fn companion_number_canonical(raw: &str) -> String {
    match raw.parse::<f64>() {
        Ok(value) if value.is_finite() => format!("v:{value}"),
        _ => format!("x:{raw}"),
    }
}

/// Ticket 011: the target set as submitted, with the pair default folded in
/// exactly the way `apply` folds it (sorted, deduplicated, JSON-encoded).
fn companion_targets_canonical(targets: &[CompanionPlacementTarget], pair_default: bool) -> String {
    let mut targets = targets.to_vec();
    if pair_default {
        targets.push(CompanionPlacementTarget::Pair);
    }
    targets.sort_by_key(|target| serde_json::to_string(target).unwrap_or_default());
    targets.dedup();
    serde_json::to_string(&targets).unwrap_or_default()
}

/// Ticket 011: the canonical companion draft — exactly what Apply submits.
/// Only the active mode's fields participate: tweak text is encoded only in
/// the tweak mode (otherwise it is never submitted), and an unchecked pair
/// default never blocks. Invalid or refused states encode distinctly from
/// every valid baseline.
fn companion_state_canonical(
    mode: &str,
    pair_default: bool,
    x: &str,
    y: &str,
    rotation: &str,
    targets: &str,
) -> String {
    let mode_part = parse_mode(mode)
        .map(mode_value)
        .unwrap_or_else(|| format!("x:{mode}"));
    let tweak_part = if parse_mode(mode) == Some(MirroredCompanionMode::LiteralMirrorWithTweak) {
        format!(
            "{}|{}|{}",
            companion_number_canonical(x),
            companion_number_canonical(y),
            companion_number_canonical(rotation),
        )
    } else {
        "none|none|none".to_owned()
    };
    format!("{mode_part}|{pair_default}|{tweak_part}|{targets}")
}

/// Ticket 011: the canonical baseline — the live-projected placement encoded
/// through the same canonicalizer as the live draft, so equality means what
/// the owning form means by "unchanged". Pair initially baselines to unchecked.
fn companion_baseline_canonical(projection: &CompanionPlacementProjection) -> String {
    companion_baseline_for_pair(projection, false)
}

fn companion_baseline_for_pair(projection: &CompanionPlacementProjection, pair: bool) -> String {
    let mode = if projection.mixed_mode {
        String::new()
    } else {
        projection.mode.map(mode_value).unwrap_or_default()
    };
    companion_state_canonical(
        &mode,
        pair,
        &tweak_value(
            projection.tweak.map(|tweak| tweak.x),
            projection.mixed_tweak,
        ),
        &tweak_value(
            projection.tweak.map(|tweak| tweak.y),
            projection.mixed_tweak,
        ),
        &tweak_value(
            projection.tweak.map(|tweak| tweak.rotation),
            projection.mixed_tweak,
        ),
        &companion_targets_canonical(&projection.targets, pair),
    )
}

#[cfg(test)]
mod companion_canonical_tests {
    use super::{CompanionPlacementOwner, CompanionPlacementProjection};
    use super::{
        companion_baseline_canonical, companion_state_canonical, companion_targets_canonical,
    };
    use boardstudio_core::model::{
        CompanionPlacementTarget, MirroredCompanionMode, MirroredCompanionTweak, MirroredItemKind,
    };

    fn item_target() -> Vec<CompanionPlacementTarget> {
        vec![CompanionPlacementTarget::Item {
            item_kind: MirroredItemKind::Part,
            mirror_id: "matrix/mirror/r0c0/led".into(),
        }]
    }

    fn tweak_draft(x: &str) -> String {
        companion_state_canonical(
            "literal-mirror-with-tweak",
            false,
            x,
            "0",
            "0",
            &companion_targets_canonical(&item_target(), false),
        )
    }

    fn tweak_baseline(value: f64) -> String {
        companion_state_canonical(
            "literal-mirror-with-tweak",
            false,
            &value.to_string(),
            "0",
            "0",
            &companion_targets_canonical(&item_target(), false),
        )
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn formatting_variants_of_one_tweak_compare_equal() {
        assert_eq!(tweak_draft("0.50"), tweak_baseline(0.5));
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn invalid_or_non_finite_tweaks_never_match_a_valid_baseline() {
        let baseline = tweak_baseline(0.5);
        for raw in ["not-a-number", "", "NaN", "inf"] {
            assert_ne!(tweak_draft(raw), baseline, "{raw:?} must stay unresolved");
        }
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn tweak_text_is_ignored_outside_the_tweak_mode() {
        let baseline = companion_state_canonical(
            "literal-mirror",
            false,
            "none",
            "none",
            "none",
            &companion_targets_canonical(&item_target(), false),
        );
        // Stray tweak text while the mirror mode is active is never submitted.
        assert_eq!(
            companion_state_canonical(
                "literal-mirror",
                false,
                "not-a-number",
                "junk",
                "junk",
                &companion_targets_canonical(&item_target(), false),
            ),
            baseline
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn checking_the_pair_default_is_a_draft_change() {
        let baseline = companion_state_canonical(
            "duplicate-position",
            false,
            "none",
            "none",
            "none",
            &companion_targets_canonical(&item_target(), false),
        );
        assert_ne!(
            companion_state_canonical(
                "duplicate-position",
                true,
                "none",
                "none",
                "none",
                &companion_targets_canonical(&item_target(), true),
            ),
            baseline
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn mixed_baseline_matches_an_untouched_form() {
        let baseline = companion_state_canonical(
            "",
            false,
            "none",
            "none",
            "none",
            &companion_targets_canonical(&item_target(), false),
        );
        assert_eq!(
            companion_state_canonical(
                "",
                false,
                "none",
                "none",
                "none",
                &companion_targets_canonical(&item_target(), false),
            ),
            baseline
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn baseline_builder_encodes_the_live_placement_canonically() {
        use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
        let scope = Scope {
            session_epoch: SessionEpoch(7),
            document_id: "companion-placement-doc".into(),
            board_id: "board".into(),
            instance_id: None,
        };
        let projection = CompanionPlacementProjection {
            owner: CompanionPlacementOwner {
                scope: scope.clone(),
                context: crate::presentation::objects::ScopedTreeContext {
                    scope,
                    context: crate::presentation::objects::TreeContext::Component {
                        part_id: Some("matrix/source/r0c0/led".into()),
                        matrix_id: Some("source".into()),
                        row: Some(0),
                        column: Some(0),
                        assembly_id: Some("led".into()),
                    },
                },
                selected_part_ids: vec!["matrix/source/r0c0/led".into()],
                snapshot_token: SnapshotToken(13),
                revision: 9,
                context_generation: 7,
                scope_generation: 7,
            },
            pair_id: "pair/A".into(),
            targets: item_target(),
            mode: Some(MirroredCompanionMode::LiteralMirror),
            mixed_mode: false,
            tweak: Some(MirroredCompanionTweak {
                x: 0.5,
                y: 0.0,
                rotation: 0.0,
            }),
            mixed_tweak: false,
            disabled_reason: None,
        };
        // A tweak-carrying accepted placement still encodes tweak-mode text
        // only under the tweak mode: the live form only submits tweaks there.
        assert_eq!(
            companion_baseline_canonical(&projection),
            companion_state_canonical(
                "literal-mirror",
                false,
                "none",
                "none",
                "none",
                &companion_targets_canonical(&item_target(), false),
            )
        );
    }
}

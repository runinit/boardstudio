//! Explicit collision checks share the accepted Case scene and model ownership.
use crate::{
    case_preview::NativePreviewSnapshot,
    model_delivery::*,
    runtime::{CadScene, CollisionCheckInput, CollisionCheckReport, Runtime},
};
use boardstudio_web_ui_shared::inspector_fields::NumericField;
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertDescription, AlertVariant},
    button::{Button, ButtonVariant},
    callout::Callout,
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    item::{Item, ItemSize},
};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone, Default)]
struct CheckState {
    source: Option<CollisionCheckInput>,
    pending: bool,
    findings: Vec<CadCollisionPairFinding>,
    clearance_threshold_mm: f64,
    message: Option<String>,
    errors: Vec<String>,
    accuracy: Option<f64>,
}

#[component]
pub(crate) fn CaseCollisionChecks(
    scene: Rc<CadScene>,
    preview: Option<Rc<NativePreviewSnapshot>>,
    model_rows: Option<ModelDeliveryRows>,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let _ = use_context::<Signal<u64>>()();
    let input = CollisionCheckInput::new(scene, preview, model_rows);
    let mut state = use_signal(CheckState::default);
    let mut clearance = use_signal(|| 0.0_f64);
    let sequence = use_hook(|| Rc::new(Cell::new(0_u64)));
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    let latest = use_hook(|| Rc::new(RefCell::new(input.clone())));
    *latest.borrow_mut() = input.clone();
    use_effect(use_reactive((&input,), {
        let sequence = sequence.clone();
        let runtime = runtime.clone();
        move |_| {
            sequence.set(sequence.get().wrapping_add(1));
            runtime.cancel_collision_checks();
            state.set(CheckState::default());
        }
    }));
    use_drop({
        let alive = alive.clone();
        let runtime = runtime.clone();
        move || {
            alive.set(false);
            runtime.cancel_collision_checks();
        }
    });
    let ready = runtime.collision_check_ready(&input);
    let threshold = Some(clearance());
    let mut start = {
        let runtime = runtime.clone();
        let input = input.clone();
        let sequence = sequence.clone();
        let alive = alive.clone();
        let latest = latest.clone();
        move |accuracy: Option<f64>| {
            let Some(threshold) = threshold else {
                return;
            };
            if !ready {
                return;
            }
            sequence.set(sequence.get().wrapping_add(1));
            let request = sequence.get();
            state.set(CheckState {
                source: Some(input.clone()),
                pending: true,
                accuracy,
                clearance_threshold_mm: threshold,
                ..Default::default()
            });
            let runtime = runtime.clone();
            let input = input.clone();
            let sequence = sequence.clone();
            let alive = alive.clone();
            let latest = latest.clone();
            spawn(async move {
                let current: Rc<dyn Fn() -> bool> = {
                    let alive = alive.clone();
                    let sequence = sequence.clone();
                    let latest = latest.clone();
                    let input = input.clone();
                    Rc::new(move || {
                        alive.get() && sequence.get() == request && *latest.borrow() == input
                    })
                };
                let report: Option<CollisionCheckReport> = runtime
                    .check_collisions(input.clone(), threshold, accuracy, current.clone())
                    .await;
                if current() {
                    state.set(match report {
                        Some(report) => CheckState {
                            source: Some(input),
                            pending: false,
                            findings: report.findings,
                            errors: report.errors,
                            accuracy,
                            clearance_threshold_mm: threshold,
                            message: report.message,
                        },
                        None => CheckState::default(),
                    });
                }
            });
        }
    };
    let shown = state.read().clone();
    let shown = if shown.source.as_ref() == Some(&input) {
        shown
    } else {
        CheckState::default()
    };
    let mut check = start.clone();
    let clearance_change = {
        let sequence = sequence.clone();
        let runtime = runtime.clone();
        move |value: f64| {
            sequence.set(sequence.get().wrapping_add(1));
            runtime.cancel_collision_checks();
            clearance.set(value);
            state.set(CheckState::default());
        }
    };
    rsx! {
        Collapsible { class: "flex-none max-h-[30%] overflow-auto", keep_mounted: true, default_open: true,
            CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground", aria_label: "Collision check", "Collision check" }
            CollapsibleContent { class: "data-[open=false]:hidden",
                NumericField {
                    label: "Required clearance (mm)", min: 0.0, step: 0.01, value: clearance(),
                    on_commit: clearance_change,
                }
                Button { variant: ButtonVariant::Secondary, button_type: "button",
                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    disabled: !ready || threshold.is_none() || shown.pending,
                    onclick: move |_| check(None), "Check collisions" }
                Button { variant: ButtonVariant::Secondary, button_type: "button",
                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    disabled: !ready || threshold.is_none() || shown.pending,
                    onclick: move |_| start(Some(TIGHT_COLLISION_ACCURACY_MM)), "Check at 0.01 mm" }
                if shown.pending {
                    p { role: "status", "Checking current assembly…" }
                    Button { variant: ButtonVariant::Secondary, button_type: "button",
                        class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                        onclick: move |_| {
                            sequence.set(sequence.get().wrapping_add(1)); runtime.cancel_collision_checks(); state.set(CheckState::default());
                        }, "Cancel check" }
                }
                if !ready { Callout { "Waiting for the current exact assembly and part models." } }
                if shown.accuracy.is_some() { Callout { "Requested accuracy: 0.01 mm. Unsupported accuracy remains uncertain." } }
                for error in &shown.errors { Alert { variant: AlertVariant::Destructive, AlertDescription { "{error}" } } }
                if let Some(message) = &shown.message { Callout { "{message}" } }
                for (index, finding) in shown.findings.iter().enumerate() {
                    CollisionFindingView { key: "{index}", finding: finding.clone(), clearance_threshold_mm: shown.clearance_threshold_mm }
                }
            }
        }
    }
}

#[component]
fn CollisionFindingView(finding: CadCollisionPairFinding, clearance_threshold_mm: f64) -> Element {
    rsx! {
        Item { size: ItemSize::Sm, class: "items-start",
            p { "{finding.summary(clearance_threshold_mm)}" }
            Collapsible { keep_mounted: true, default_open: false,
                CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground", aria_label: "Witness identity", "Witness identity" }
                CollapsibleContent { class: "data-[open=false]:hidden",
                    for identity in finding.provenance_details() { p { "{identity}" } }
                    for detail in finding.qualification_details() { p { "{detail}" } }
                }
            }
        }
    }
}

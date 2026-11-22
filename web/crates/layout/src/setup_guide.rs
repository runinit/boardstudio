use boardstudio_core_mechanical::electrical::ElectricalPlan;
use boardstudio_model::model::{PartKind, ProjectDoc, SceneDelta, Severity};
pub use boardstudio_web_ui_model::state::{PendingNewKeyboard, SetupGuideRequest};
use boardstudio_web_ui_shared::inspector_fields::TextField;
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::stepper::{
    Stepper, StepperIndicator, StepperItem, StepperOrientation, StepperTitle,
};
use boardstudio_web_ui_shared::ui::tooltip::{Tooltip, TooltipContent, TooltipTrigger};

use boardstudio_web_ui_shared::ui::accordion::{Accordion, AccordionItem, AccordionTrigger};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupGuideStage {
    Project,
    Layout,
    Wiring,
    Case,
    Review,
}

impl SetupGuideStage {
    fn as_storage(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Layout => "layout",
            Self::Wiring => "wiring",
            Self::Case => "case",
            Self::Review => "review",
        }
    }

    fn from_storage(value: &str) -> Self {
        match value {
            "layout" => Self::Layout,
            "wiring" => Self::Wiring,
            "case" => Self::Case,
            "review" => Self::Case,
            _ => Self::Project,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupGuidePreferences {
    pub project_id: String,
    pub open: bool,
    pub current_stage: SetupGuideStage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupGuideStageStatus {
    pub ready: bool,
    pub detail: String,
}

fn sync_stage_trigger(root: web_sys::Element, stage: SetupGuideStage) {
    let index = match stage {
        SetupGuideStage::Project => 0,
        SetupGuideStage::Layout => 1,
        SetupGuideStage::Wiring => 2,
        SetupGuideStage::Case => 3,
        SetupGuideStage::Review => 4,
    };
    // Accordion exposes default_open only, so sync after Dioxus commits the new stage.
    spawn(async move {
        gloo_timers::future::TimeoutFuture::new(0).await;
        // A click can change stages before this task runs. Opening the old
        // stage's trigger would dispatch another navigation and undo that click.
        let selector = format!(
            "[aria-controls='m1-setup-panel-{index}'][aria-current='step'][aria-expanded='false']"
        );
        if let Ok(Some(trigger)) = root.query_selector(&selector)
            && let Ok(trigger) = trigger.dyn_into::<web_sys::HtmlElement>()
        {
            trigger.click();
        }
    });
}

pub fn stage_statuses(
    document: &ProjectDoc,
    board_id: &str,
    scene: &SceneDelta,
    wiring_plan: Option<&ElectricalPlan>,
) -> [SetupGuideStageStatus; 5] {
    let board = document.boards.iter().find(|board| board.id == board_id);
    let board_parts = board.map(|board| &board.part_ids[..]).unwrap_or_default();
    let matrices: Vec<_> = document
        .matrices
        .iter()
        .filter(|matrix| {
            matrix.board_id.as_deref() == Some(board_id)
                || (matrix.board_id.is_none()
                    && matrix
                        .part_ids
                        .iter()
                        .any(|part_id| board_parts.contains(part_id)))
        })
        .collect();
    let enabled_cells = matrices.iter().fold(0_u64, |count, matrix| {
        let total = u64::from(matrix.rows) * u64::from(matrix.columns);
        let disabled = matrix
            .cells
            .iter()
            .filter(|cell| !cell.enabled && cell.row < matrix.rows && cell.column < matrix.columns)
            .map(|cell| (cell.row, cell.column))
            .collect::<std::collections::BTreeSet<_>>()
            .len() as u64;
        count + total.saturating_sub(disabled)
    });
    let matrix_part_ids = matrices
        .iter()
        .flat_map(|matrix| matrix.part_ids.iter().cloned())
        .collect::<std::collections::BTreeSet<_>>();
    let standalone_keys = document
        .parts
        .iter()
        .filter(|part| {
            board_parts.contains(&part.id)
                && !matrix_part_ids.contains(&part.id)
                && document.definitions.iter().any(|definition| {
                    definition.id == part.definition_id && definition.kind == PartKind::Switch
                })
        })
        .count() as u64;
    let key_count = enabled_cells + standalone_keys;
    let mut guide_targets =
        std::collections::BTreeSet::from([document.id.clone(), board_id.to_owned()]);
    guide_targets.extend(board_parts.iter().cloned());
    if let Some(board) = board {
        guide_targets.extend(board.outline_ids.iter().cloned());
    }
    guide_targets.extend(matrices.iter().map(|matrix| matrix.id.clone()));
    let layout_error_count = scene
        .findings
        .iter()
        .filter(|finding| {
            finding.severity == Severity::Error
                && (finding.target_ids.is_empty()
                    || finding
                        .target_ids
                        .iter()
                        .any(|target| guide_targets.contains(target)))
        })
        .count();
    let project_ready = board.is_some()
        && document.hardware.as_ref().is_some_and(|hardware| {
            hardware
                .boards
                .iter()
                .any(|entry| entry.board_id == board_id)
                || hardware
                    .instances
                    .iter()
                    .any(|instance| instance.board_id == board_id)
        });
    let layout_ready = board.is_some()
        && key_count > 0
        && (standalone_keys > 0
            || matrix_part_ids
                .iter()
                .any(|part_id| board_parts.contains(part_id)))
        && layout_error_count == 0;
    let wiring_current = wiring_plan.is_some_and(|plan| {
        plan.board_id.as_deref() == Some(board_id)
            && plan.revision == document.revision
            && plan.instance_id.is_none()
    });
    let wiring_ready = wiring_plan.is_some_and(|plan| {
        plan.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != "error")
    });
    let wiring_applied = wiring_plan.is_some_and(|plan| {
        let Some(board) = document.boards.iter().find(|board| board.id == board_id) else {
            return false;
        };
        let Some(configuration) = document.hardware.as_ref().and_then(|hardware| {
            hardware
                .boards
                .iter()
                .find(|configuration| configuration.board_id == board_id)
        }) else {
            return false;
        };
        let prefix = format!("generated/electrical/{board_id}/");
        let current_nets = document
            .nets
            .iter()
            .filter(|net| net.id.starts_with(&prefix))
            .collect::<Vec<_>>();
        plan.revision == document.revision
            && current_nets.len() == plan.nets.len()
            && configuration.mode == plan.mode
            && configuration.controller_part_id == plan.controller_part_id
            && plan
                .nets
                .iter()
                .all(|net| current_nets.contains(&net) && board.net_ids.contains(&net.id))
    });
    let wiring_stage_ready = wiring_current && wiring_ready && wiring_applied;
    let case_configured = document
        .case_bodies
        .iter()
        .any(|body| body.board_id == board_id);
    let case_ready = scene
        .board_readiness
        .iter()
        .find(|readiness| readiness.board_id == board_id)
        .is_some_and(|readiness| readiness.case_ready);
    let review_ready =
        board.is_some() && layout_ready && wiring_stage_ready && (!case_configured || case_ready);

    [
        SetupGuideStageStatus {
            ready: project_ready,
            detail: stage_detail(SetupGuideStage::Project, document, board_id),
        },
        SetupGuideStageStatus {
            ready: layout_ready,
            detail: if layout_ready {
                format!(
                    "{key_count} enabled key position{} {} ready.",
                    if key_count == 1 { "" } else { "s" },
                    if key_count == 1 { "is" } else { "are" }
                )
            } else if layout_error_count > 0 {
                format!(
                    "{layout_error_count} layout finding{} need attention.",
                    if layout_error_count == 1 { "" } else { "s" }
                )
            } else {
                "Create a layout and place its assemblies.".into()
            },
        },
        SetupGuideStageStatus {
            ready: wiring_stage_ready,
            detail: if wiring_stage_ready {
                "Controller pins and board nets are applied.".into()
            } else if wiring_current && wiring_ready {
                "Apply the resolved wiring before export.".into()
            } else {
                "Resolve the controller and board wiring.".into()
            },
        },
        SetupGuideStageStatus {
            ready: case_ready,
            detail: if case_configured {
                if case_ready {
                    "Authored case geometry is ready.".into()
                } else {
                    "Review the authored case geometry.".into()
                }
            } else {
                "Optional: configure a case or continue without one.".into()
            },
        },
        SetupGuideStageStatus {
            ready: review_ready,
            detail: if review_ready {
                "The selected board is ready for review and export.".into()
            } else {
                "Complete the required steps for this board before export.".into()
            },
        },
    ]
}

#[component]
pub fn ProjectSetupGuide(
    stage: SetupGuideStage,
    stage_readiness: [bool; 5],
    stage_detail: String,
    project_name: String,
    on_name_change: EventHandler<String>,
    on_name_commit: EventHandler<()>,
    on_stage_change: EventHandler<SetupGuideStage>,
    on_open_workspace: EventHandler<&'static str>,
    on_open_matrix_setup: Option<EventHandler<()>>,
    #[props(default)] matrix_setup: Option<Element>,
    #[props(default)] case_basics: Option<Element>,
    #[props(default)] controller_picker: Option<Element>,
    #[props(default)] has_matrix: bool,
    on_choose_controller: Option<EventHandler<()>>,
    on_dismiss: EventHandler<()>,
    on_finish: EventHandler<()>,
    project_controls: Option<Element>,
) -> Element {
    let mut expanded = use_signal(|| Some(stage));
    let mut guide_root = use_signal(|| None::<web_sys::Element>);
    use_effect(use_reactive((&stage,), move |(stage,)| {
        expanded.set(Some(stage));
        if let Some(root) = guide_root() {
            sync_stage_trigger(root, stage);
        }
    }));
    let stage_change = on_stage_change;
    let footer = match stage {
        SetupGuideStage::Project => "Continue to layout",
        SetupGuideStage::Layout => "Continue to wiring",
        SetupGuideStage::Wiring => "Continue to case",
        SetupGuideStage::Case | SetupGuideStage::Review => "Continue to layout",
    };
    let next_stage = match stage {
        SetupGuideStage::Project => Some(SetupGuideStage::Layout),
        SetupGuideStage::Layout => Some(SetupGuideStage::Wiring),
        SetupGuideStage::Wiring => Some(SetupGuideStage::Case),
        SetupGuideStage::Case => None,
        SetupGuideStage::Review => None,
    };
    let previous_stage = match stage {
        SetupGuideStage::Project => None,
        SetupGuideStage::Layout => Some(SetupGuideStage::Project),
        SetupGuideStage::Wiring => Some(SetupGuideStage::Layout),
        SetupGuideStage::Case => Some(SetupGuideStage::Wiring),
        SetupGuideStage::Review => Some(SetupGuideStage::Case),
    };
    let mut expanded_for_trigger = expanded;
    rsx! {
        aside { class: "grid min-h-0 min-w-0 flex-1 grid-rows-[auto_minmax(0,1fr)_auto] content-start gap-3 overflow-hidden bg-card p-4 text-foreground [&_button]:min-h-control-touch", "aria-label": "Project setup guide",
        onmounted: move |event| {
            guide_root.set(event.data().downcast::<web_sys::Element>().cloned());
        },
        onkeydown: move |event: KeyboardEvent| {
            if event.key().to_string() == " " {
                event.stop_propagation();
                if let Some(raw) = event.data().try_as_web_event() {
                    raw.stop_propagation();
                }
            }
        },
            header { class: "flex items-start justify-between gap-2 [&_h2]:text-[18px]! [&_p]:mt-1 [&_p]:text-muted-foreground",
                div {
                    h2 {
                        id: "m1-setup-guide-heading",
                        tabindex: "-1",
                        onmounted: move |event| async move {
                            let _ = event.set_focus(true).await;
                        },
                        "Setup guide"
                    }
                }
                Button {
                    variant: ButtonVariant::Secondary,
                    class: "min-h-control-touch shrink-0 whitespace-nowrap px-2.5",
                    button_type: "button",
                    onclick: move |_| on_dismiss.call(()),
                    "Hide guide"
                }
            }
            Accordion { class: "grid content-start gap-2 overflow-y-auto overscroll-contain p-0.5 [scrollbar-width:thin]", allow_multiple_open: false,
                for (index, item_stage, title, ready) in [
                    (0usize, SetupGuideStage::Project, "Project & hardware", stage_readiness[0]),
                    (1usize, SetupGuideStage::Layout, "Layout & assemblies", stage_readiness[1]),
                    (2usize, SetupGuideStage::Wiring, "Controller & wiring", stage_readiness[2]),
                    (3usize, SetupGuideStage::Case, "Case (optional)", stage_readiness[3]),
                ]
                {
                    AccordionItem {
                        index,
                        default_open: stage == item_stage,
                        class: "block",
                        on_change: move |open| {
                            if open {
                                expanded_for_trigger.set(Some(item_stage));
                            } else if expanded_for_trigger() == Some(item_stage) {
                                expanded_for_trigger.set(None);
                            }
                        },
                        on_trigger_click: move || {
                            if stage != item_stage {
                                stage_change.call(item_stage);
                            }
                        },
                        AccordionTrigger {
                            aria_controls: "m1-setup-panel-{index}",
                            aria_label: title,
                            aria_current: (stage == item_stage).then_some("step"),
                            "aria-description": if ready { "Ready" } else { "Not ready" },
                            class: if stage == item_stage { "flex min-h-11 w-full items-center gap-2.5 bg-selected px-2 py-1.5 text-left [&_[data-slot=stepper-item]]:items-center [&_[data-slot=stepper-title]]:text-left" } else { "flex min-h-11 w-full items-center gap-2.5 px-2 py-1.5 text-left [&_[data-slot=stepper-item]]:items-center [&_[data-slot=stepper-title]]:text-left" },
                            // Readiness is independent of stage order; only this status presentation remounts.
                            Stepper {
                                key: "{stage == item_stage}-{ready}",
                                total_steps: 2,
                                default_step: usize::from(ready && stage != item_stage),
                                orientation: StepperOrientation::Vertical,
                                StepperItem { step: usize::from(!ready && stage != item_stage),
                                    StepperIndicator {
                                        if ready { "✓" } else { "{index + 1}" }
                                    }
                                    StepperTitle { "{title}" }
                                }
                            }

                        }
                        div {
                            id: "m1-setup-panel-{index}",
                            class: if expanded() == Some(item_stage) { "grid grid-rows-[1fr] visible opacity-100 transition-[grid-template-rows,opacity,visibility] motion-reduce:transition-none [&>section]:min-h-0 [&>section]:overflow-visible" } else { "grid grid-rows-[0fr] invisible opacity-0 transition-[grid-template-rows,opacity,visibility] motion-reduce:transition-none [&>section]:min-h-0 [&>section]:overflow-hidden [&>section]:border-0 [&>section]:py-0" },
                            aria_hidden: expanded() != Some(item_stage),
                            inert: (expanded() != Some(item_stage)).then_some(""),
                            section { class: "grid content-start gap-4 border-0 px-1 py-4 [&>p]:mt-1 [&>p]:text-muted-foreground [&>[aria-label='New_matrix']]:static! [&>[aria-label='New_matrix']]:grid! [&>[aria-label='New_matrix']]:w-full! [&>[aria-label='New_matrix']]:overflow-visible! [&>[aria-label='New_matrix']]:border-0! [&>[aria-label='New_matrix']]:bg-transparent! [&>[aria-label='New_matrix']]:p-0! [&>[aria-label='New_matrix']]:shadow-none!",
                                if item_stage == SetupGuideStage::Project {
                                    {
                                        let control_input_id = format!(
                                            "layout-input-{}-1",
                                            dioxus::core::current_scope_id().0,
                                        );
                                        rsx! {
                                            TextField {
                                                id: control_input_id, label: "Project name", aria_label: "Project name",
                                                class: "", control_class: "", aria_invalid: None,
                                                value: "{project_name}",
                                                oninput: move |event: FormEvent| on_name_change.call(event.value()),
                                                onblur: move |_| on_name_commit.call(()),
                                                onkeydown: move |event: KeyboardEvent| {
                                                    if event.key() == Key::Enter
                                                        && let Some(input) = event.data().try_as_web_event()
                                                            .and_then(|event| event.target())
                                                            .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
                                                    { let _ = input.blur(); }
                                                },
                                            }
                                        }
                                    }
                                    if let Some(controls) = project_controls.clone() {
                                        {controls}
                                    }
                                } else if item_stage == SetupGuideStage::Layout {
                                    if let Some(matrix_setup) = matrix_setup.clone() {
                                        {matrix_setup}
                                    } else if let Some(on_open) = on_open_matrix_setup {
                                        Button {
                                            variant: ButtonVariant::Default,
                                            onclick: move |_| on_open.call(()),
                                            if has_matrix {
                                                "Add Thumb cluster"
                                            } else {
                                                "Set up key matrix"
                                            }
                                        }
                                    }
                                } else if item_stage == SetupGuideStage::Wiring {
                                    p {
                                        "Choose a controller. BoardStudio resolves pins and wiring from its footprint."
                                    }
                                    if let Some(controller_picker) = controller_picker.clone() {
                                        {controller_picker}
                                    } else if let Some(on_choose) = on_choose_controller {
                                        Button {
                                            variant: ButtonVariant::Secondary,
                                            onclick: move |_| on_choose.call(()),
                                            "Choose controller"
                                        }
                                    }
                                } else if item_stage == SetupGuideStage::Case {
                                    p {
                                        "Optional. Set the basics here, then refine your case in the full editor."
                                    }
                                    if let Some(case_basics) = case_basics.clone() {
                                        {case_basics}
                                    }
                                    Button {
                                        variant: ButtonVariant::Secondary,
                                        onclick: move |_| on_open_workspace.call("Case"),
                                        "Open full case settings"
                                    }
                                } else {
                                    p { "Review the selected board and its available outputs." }
                                }
                            }
                        }
                    }
                }
            }
            footer { class: "sticky bottom-0 z-2 mt-auto flex items-center justify-between gap-2 border-t border-border bg-card pt-3",
                if let Some(previous) = previous_stage {
                    Tooltip {
                        TooltipTrigger {
                            r#as: move |attributes: Vec<Attribute>| rsx! {
                                Button {
                                    attributes,
                                    variant: ButtonVariant::Secondary,
                                    class: "grid w-[42px] min-w-[42px] place-items-center p-2 [&_svg]:size-5 [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.8] [&_svg]:[stroke-linecap:round] [&_svg]:[stroke-linejoin:round]",
                                    aria_label: "Previous step",
                                    onclick: move |_| on_stage_change.call(previous),
                                    svg { view_box: "0 0 24 24", "aria-hidden": "true",
                                        path { d: "M19 12H5m7 7-7-7 7-7" }
                                    }
                                }
                            },
                        }
                        TooltipContent { "Previous step" }
                    }
                } else {
                    span {}
                }
                if let Some(next) = next_stage {
                    Button {
                        variant: ButtonVariant::Default,
                        onclick: move |_| on_stage_change.call(next),
                        "{footer}"
                    }
                } else {
                    Button {
                        variant: ButtonVariant::Default,
                        onclick: move |_| on_finish.call(()),
                        "{footer}"
                    }
                }
            }
        }
    }
}

pub fn stage_detail(stage: SetupGuideStage, document: &ProjectDoc, board_id: &str) -> String {
    let board = document.boards.iter().find(|board| board.id == board_id);
    match stage {
        SetupGuideStage::Project => {
            let configured = document.hardware.as_ref().is_some_and(|hardware| {
                hardware
                    .boards
                    .iter()
                    .any(|board| board.board_id == board_id)
                    || hardware
                        .instances
                        .iter()
                        .any(|instance| instance.board_id == board_id)
            });
            if configured {
                let topology = document.hardware.as_ref().is_some_and(|hardware| {
                    matches!(
                        hardware.topology,
                        boardstudio_model::model::HardwareTopology::Split
                    )
                });
                format!(
                    "{} keyboard · {}.",
                    if topology { "Split" } else { "One" },
                    board.map(|board| board.name.as_str()).unwrap_or("Board")
                )
            } else {
                "Choose one keyboard or a split keyboard.".into()
            }
        }
        SetupGuideStage::Layout => "Create a layout and place its assemblies.".into(),
        SetupGuideStage::Wiring => "Resolve the controller and board wiring.".into(),
        SetupGuideStage::Case => {
            let configured = document
                .case_bodies
                .iter()
                .any(|body| body.board_id == board_id);
            if configured {
                "Configure construction and clearances, then generate geometry when you are ready."
                    .into()
            } else {
                "Optional: configure a case or continue without one.".into()
            }
        }
        SetupGuideStage::Review => {
            "Complete the required steps for this board before export.".into()
        }
    }
}

pub fn read_preferences(project_id: &str) -> SetupGuidePreferences {
    let key = format!("boardstudio:v2:setup-guide:{project_id}");
    let raw = web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(&key).ok().flatten());
    let parsed = raw
        .as_deref()
        .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok());
    SetupGuidePreferences {
        project_id: project_id.to_owned(),
        open: parsed
            .as_ref()
            .and_then(|value| value.get("open"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        current_stage: parsed
            .as_ref()
            .and_then(|value| value.get("currentStep"))
            .and_then(serde_json::Value::as_str)
            .map(SetupGuideStage::from_storage)
            .unwrap_or(SetupGuideStage::Project),
    }
}

pub fn write_preferences(preferences: &SetupGuidePreferences) {
    let key = format!("boardstudio:v2:setup-guide:{}", preferences.project_id);
    let value = serde_json::json!({
        "open": preferences.open,
        "currentStep": preferences.current_stage.as_storage(),
    });
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.set_item(&key, &value.to_string());
    }
}

/// Keep stage navigation and its panel reveal in one production transition.
pub fn activate_stage(
    stage: SetupGuideStage,
    mut workspace: Signal<&'static str>,
    mut requested_workspace: Signal<Option<&'static str>>,
    objects_open: Signal<bool>,
    inspector_open: Signal<bool>,
    objects_settings: Signal<super::panels::PanelSettings>,
    inspector_settings: Signal<super::panels::PanelSettings>,
) {
    let target = match stage {
        SetupGuideStage::Project | SetupGuideStage::Layout => "Layout",
        SetupGuideStage::Wiring => "Layout",
        SetupGuideStage::Case => "Case",
        SetupGuideStage::Review => "Export",
    };
    if *workspace.peek() != target {
        requested_workspace.set(Some(target));
    }
    workspace.set(target);
    reveal_panels(
        crate::setup_guide_state::GuideReveal::Guide,
        objects_open,
        inspector_open,
        objects_settings,
        inspector_settings,
    );
}

/// Reveal the requested panel without changing compact-mode stored preferences.
pub fn reveal_panels(
    intent: crate::setup_guide_state::GuideReveal,
    mut objects_open: Signal<bool>,
    mut inspector_open: Signal<bool>,
    mut objects_settings: Signal<super::panels::PanelSettings>,
    mut inspector_settings: Signal<super::panels::PanelSettings>,
) {
    let compact = web_sys::window()
        .and_then(|window| window.match_media("(max-width: 760px)").ok().flatten())
        .is_some_and(|query| query.matches());
    let reveal = crate::setup_guide_state::panel_reveal(intent, compact);
    if *objects_open.peek() != reveal.objects_open {
        objects_open.set(reveal.objects_open);
    }
    if compact && *inspector_open.peek() != reveal.inspector_open {
        inspector_open.set(reveal.inspector_open);
    }
    if reveal.pin_objects && objects_settings.peek().mode != super::panels::PanelMode::Pinned {
        objects_settings.with_mut(|settings| settings.mode = super::panels::PanelMode::Pinned);
    }
    if reveal.pin_inspector && inspector_settings.peek().mode != super::panels::PanelMode::Pinned {
        inspector_settings.with_mut(|settings| settings.mode = super::panels::PanelMode::Pinned);
    }
}

pub fn focus_settings(workspace: &str) {
    let selector = if workspace == "Export" {
        "#m1-workspace-panel :is(input, select, button):not(:disabled)"
    } else {
        "#m1-inspector-panel-content :is(input, select, button):not(:disabled)"
    };
    if let Some(element) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.query_selector(selector).ok().flatten())
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let _ = element.focus();
    }
}

/// Physical topology only: these are schematic boards, not generated outlines.
#[component]
pub fn SetupHardwarePreview(split: bool, wired: bool) -> Element {
    rsx! {
        section {
            class: "flex min-h-0 flex-1 flex-col justify-center p-8 [&_h2]:text-[18px]! [&_p]:mt-3 [&_p]:text-muted-foreground [&_svg]:max-h-[500px] [&_svg]:w-full [&_svg]:flex-1 [&_rect]:fill-canvas [&_rect]:stroke-geometry-board [&_rect]:stroke-2 [&_text]:fill-muted-foreground [&_text]:text-[16px] [&_text]:[text-anchor:middle]",
            aria_label: "Assembly preview",
            h2 { "Assembly preview" }
            p {
                if split {
                    "Split keyboard"
                } else {
                    "One keyboard"
                }
                if split {
                    if wired {
                        " · Wired connection"
                    } else {
                        " · Wireless connection"
                    }
                }
            }
            svg {
                view_box: "0 0 600 340",
                role: "img",
                "aria-label": "Schematic physical board arrangement",
                rect {
                    x: if split { "40" } else { "110" },
                    y: "70",
                    width: if split { "220" } else { "380" },
                    height: "180",
                    rx: "12",
                }
                if split {
                    rect {
                        x: "340",
                        y: "70",
                        width: "220",
                        height: "180",
                        rx: "12",
                    }
                    if wired {
                        path { class: "fill-none stroke-primary stroke-4", d: "M260 200H340" }
                    }
                    text { x: "150", y: "290", "Left assembly" }
                    text { x: "450", y: "290", "Right assembly" }
                } else {
                    text { x: "300", y: "290", "Keyboard assembly" }
                }
            }
            p { "Schematic arrangement. Add keys in Layout to create board geometry." }
        }
    }
}

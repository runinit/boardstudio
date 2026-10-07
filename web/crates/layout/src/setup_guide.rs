use boardstudio_core::{
    electrical::ElectricalPlan,
    model::{PartKind, ProjectDoc, SceneDelta, Severity},
};
pub use boardstudio_web_ui_model::state::{PendingNewKeyboard, SetupGuideRequest};
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
            "review" => Self::Review,
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

fn step_class(current: bool, ready: bool) -> &'static str {
    match (current, ready) {
        (true, true) => "m1-setup-guide__step is-current is-ready",
        (true, false) => "m1-setup-guide__step is-current",
        (false, true) => "m1-setup-guide__step is-ready",
        (false, false) => "m1-setup-guide__step",
    }
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
            && plan.nets.iter().all(|net| {
                current_nets.iter().any(|current| *current == net)
                    && board.net_ids.contains(&net.id)
            })
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
    on_choose_controller: Option<EventHandler<()>>,
    on_dismiss: EventHandler<()>,
    project_controls: Option<Element>,
) -> Element {
    let stage_index = match stage {
        SetupGuideStage::Project => 0,
        SetupGuideStage::Layout => 1,
        SetupGuideStage::Wiring => 2,
        SetupGuideStage::Case => 3,
        SetupGuideStage::Review => 4,
    };
    let heading = match stage {
        SetupGuideStage::Project => "Project & hardware",
        SetupGuideStage::Layout => "Layout & assemblies",
        SetupGuideStage::Wiring => "Controller & wiring",
        SetupGuideStage::Case => "Case (optional)",
        SetupGuideStage::Review => "Review & export",
    };
    rsx! {
        aside { class: "m1-setup-guide", "aria-label": "Project setup guide",
            header { class: "m1-setup-guide__header",
                div {
                    h2 {
                        tabindex: "-1",
                        onmounted: move |event| async move { let _ = event.set_focus(true).await; },
                        "Setup guide"
                    }
                    p { "A step-by-step guide to your keyboard." }
                }
                button { class: "m1-setup-guide__dismiss", r#type: "button", onclick: move |_| on_dismiss.call(()), "Back to objects" }
            }
            p { class: "m1-setup-guide__intro", "Step {stage_index + 1} of 5. Move between steps freely; your work is kept." }
            nav { class: "m1-setup-guide__steps", "aria-label": "Setup steps",
                button {
                    class: step_class(stage == SetupGuideStage::Project, stage_readiness[0]),
                    aria_current: (stage == SetupGuideStage::Project).then_some("step"),
                    onclick: move |_| on_stage_change.call(SetupGuideStage::Project),
                    span { class: "m1-setup-guide__step-marker", aria_label: stage_readiness[0].then_some("Ready"), if stage_readiness[0] { "✓" } else { "1" } }
                    strong { "Project & hardware" }
                }
                button {
                    class: step_class(stage == SetupGuideStage::Layout, stage_readiness[1]),
                    aria_current: (stage == SetupGuideStage::Layout).then_some("step"),
                    onclick: move |_| on_stage_change.call(SetupGuideStage::Layout),
                    span { class: "m1-setup-guide__step-marker", aria_label: stage_readiness[1].then_some("Ready"), if stage_readiness[1] { "✓" } else { "2" } }
                    strong { "Layout & assemblies" }
                }
                button {
                    class: step_class(stage == SetupGuideStage::Wiring, stage_readiness[2]),
                    aria_current: (stage == SetupGuideStage::Wiring).then_some("step"),
                    onclick: move |_| on_stage_change.call(SetupGuideStage::Wiring),
                    span { class: "m1-setup-guide__step-marker", aria_label: stage_readiness[2].then_some("Ready"), if stage_readiness[2] { "✓" } else { "3" } }
                    strong { "Controller & wiring" }
                }
                button {
                    class: step_class(stage == SetupGuideStage::Case, stage_readiness[3]),
                    aria_current: (stage == SetupGuideStage::Case).then_some("step"),
                    onclick: move |_| on_stage_change.call(SetupGuideStage::Case),
                    span { class: "m1-setup-guide__step-marker", aria_label: stage_readiness[3].then_some("Ready"), if stage_readiness[3] { "✓" } else { "4" } }
                    strong { "Case (optional)" }
                }
                button {
                    class: step_class(stage == SetupGuideStage::Review, stage_readiness[4]),
                    aria_current: (stage == SetupGuideStage::Review).then_some("step"),
                    onclick: move |_| on_stage_change.call(SetupGuideStage::Review),
                    span { class: "m1-setup-guide__step-marker", aria_label: stage_readiness[4].then_some("Ready"), if stage_readiness[4] { "✓" } else { "5" } }
                    strong { "Review & export" }
                }
            }
            section { class: "m1-setup-guide__content",
                h3 { "{heading}" }
                p { class: "m1-setup-guide__status", "{stage_detail}" }
                if stage == SetupGuideStage::Project {
                    label { "Project name"
                        input {
                            aria_label: "Project name",
                            value: "{project_name}",
                            oninput: move |event| on_name_change.call(event.value()),
                            onblur: move |_| on_name_commit.call(()),
                            onkeydown: move |event| {
                                if event.key() == Key::Enter
                                    && let Some(input) = event.data().try_as_web_event()
                                        .and_then(|event| event.target())
                                        .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
                                    {
                                        let _ = input.blur();
                                }
                            },
                        }
                    }
                    if let Some(controls) = project_controls { {controls} }
                    button { class: "m1-setup-guide__primary", r#type: "button", onclick: move |_| on_stage_change.call(SetupGuideStage::Layout), "Continue to layout" }
                } else if stage == SetupGuideStage::Layout {
                    p { "Place keys with their switch, diode, and optional lighting assembly. Edit the layout on the canvas." }
                    if let Some(on_open) = on_open_matrix_setup {
                        button { class: "m1-setup-guide__primary", r#type: "button", onclick: move |_| on_open.call(()), "Add key matrix" }
                    }
                    button { class: "m1-setup-guide__secondary", r#type: "button", onclick: move |_| on_dismiss.call(()), "Edit existing objects" }
                    button { class: "m1-setup-guide__primary", r#type: "button", onclick: move |_| on_stage_change.call(SetupGuideStage::Wiring), "Continue to wiring" }
                    button { class: "m1-setup-guide__secondary", r#type: "button", onclick: move |_| on_stage_change.call(SetupGuideStage::Project), "Previous step" }
                } else if stage == SetupGuideStage::Wiring {
                    p { "Place a controller, then review automatic pin assignments and reversible jumpers in PCB settings." }
                    if let Some(on_choose) = on_choose_controller {
                        button { class: "m1-setup-guide__secondary", r#type: "button", onclick: move |_| on_choose.call(()), "Choose controller" }
                    }
                    button { class: "m1-setup-guide__secondary", r#type: "button", onclick: move |_| on_open_workspace.call("PCB"), "Review controller & wiring" }
                    button { class: "m1-setup-guide__primary", r#type: "button", onclick: move |_| on_stage_change.call(SetupGuideStage::Case), "Continue to case" }
                    button { class: "m1-setup-guide__secondary", r#type: "button", onclick: move |_| on_stage_change.call(SetupGuideStage::Layout), "Previous step" }
                } else if stage == SetupGuideStage::Case {
                    p { "Configure construction and clearances, then generate geometry when you are ready. PCB and firmware exports are available separately." }
                    button { class: "m1-setup-guide__secondary", r#type: "button", onclick: move |_| on_open_workspace.call("Case"), "Open case settings" }
                    button { class: "m1-setup-guide__primary", r#type: "button", onclick: move |_| on_stage_change.call(SetupGuideStage::Review), "Continue to review" }
                    button { class: "m1-setup-guide__secondary", r#type: "button", onclick: move |_| on_stage_change.call(SetupGuideStage::Wiring), "Previous step" }
                } else {
                    p { "Review the selected board’s findings and available outputs. Export checks still apply to each output." }
                    button { class: "m1-setup-guide__secondary", r#type: "button", onclick: move |_| on_open_workspace.call("Export"), "Open export options" }
                    button { class: "m1-setup-guide__primary", r#type: "button", onclick: move |_| on_dismiss.call(()), "Finish guide" }
                    button { class: "m1-setup-guide__secondary", r#type: "button", onclick: move |_| on_stage_change.call(SetupGuideStage::Case), "Previous step" }
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
                        boardstudio_core::model::HardwareTopology::Split
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
        SetupGuideStage::Wiring => "PCB",
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
        ".m1-workspace-content :is(input, select, button):not(:disabled)"
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

#[cfg(test)]
mod tests;

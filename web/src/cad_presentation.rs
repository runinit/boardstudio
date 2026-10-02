//! Canvas lifetime and case controls consume immutable session snapshots.
use crate::runtime::Runtime;
use boardstudio_application::{Durability, Event, GenerationStatus, Lifecycle};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase};
use boardstudio_web::{cad_jobs::captured_case_document, case_settings};
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub fn CasePanel() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let instance_selection = use_context::<crate::presentation::InstanceSelection>();
    let _ = use_context::<Signal<u64>>()();
    let model = runtime.model();
    let Some(snapshot) = model.accepted.as_ref() else {
        return rsx! {};
    };
    let settings_state = use_memo(use_reactive(
        (
            &snapshot.token,
            &model.active_board_id,
            &model.active_instance_id,
        ),
        {
            let runtime = runtime.clone();
            move |_| {
                let model = runtime.model();
                let effective = model
                    .accepted
                    .as_ref()
                    .ok_or("Case snapshot unavailable".to_owned())
                    .and_then(|snapshot| {
                        runtime
                            .scope()
                            .ok_or("Case scope unavailable".to_owned())
                            .and_then(|scope| {
                                captured_case_document(snapshot, &scope)
                                    .map_err(|error| format!("{error:?}"))
                            })
                    });
                let has_settings = effective.as_ref().is_ok_and(|document| {
                    document
                        .mechanical
                        .as_ref()
                        .is_some_and(|config| config.board_id == model.active_board_id)
                });
                let mismatch = effective.as_ref().is_ok_and(|document| {
                    document
                        .mechanical
                        .as_ref()
                        .is_some_and(|config| config.board_id != model.active_board_id)
                });
                let settings = effective.and_then(|document| {
                    case_settings::initial_settings(&document, &model.active_board_id)
                });
                (has_settings, mismatch, settings)
            }
        },
    ));
    let (has_settings, mismatch, settings) = settings_state();
    let can_edit_settings = !mismatch
        && model.lifecycle == Lifecycle::Ready
        && model.display_preview.is_none()
        && model.gesture.is_none()
        && model.durability
            == Durability::Saved {
                revision: snapshot.document.revision,
            };
    let initialize = runtime.clone();
    let generate = runtime.clone();
    let cancel = runtime.clone();
    let update = runtime.clone();
    let scene = runtime.cad_scene();
    let stale = scene
        .as_ref()
        .is_some_and(|scene| scene.token != snapshot.token);
    let title = match &model.generation {
        GenerationStatus::Preparing { .. } | GenerationStatus::Running { .. } => {
            "Generating case…".to_owned()
        }
        GenerationStatus::Blocked { reason, .. } => format!("Case generation blocked: {reason}"),
        GenerationStatus::Failed { reason, .. } => format!("Case generation failed: {reason}"),
        GenerationStatus::Cancelled { .. } => "Case generation cancelled.".to_owned(),
        _ if stale => "Previous case geometry — regenerate for current changes.".to_owned(),
        _ if scene.as_ref().is_some_and(|s| s.exact) => "Exact case geometry ready.".to_owned(),
        _ if scene.is_some() => {
            "Case preview ready; exact assembly is still being built.".to_owned()
        }
        _ => "Generate a case from the saved keyboard.".to_owned(),
    };
    rsx! {
        section { class: "m1-case-panel", "aria-label": "Case assembly",
            div { class: "m1-case-header",
                h2 { "Case assembly" }
                button { onclick: move |_| if instance_selection.is_current(&generate.model()) && let Some(scope) = generate.scope() { generate.submit(Event::StartGeneration { operation_id: generate.operation(), scope }); }, "Generate case" }
                button { disabled: !matches!(model.generation, GenerationStatus::Preparing {..} | GenerationStatus::Running {..}), onclick: move |_| cancel.submit(Event::CancelGeneration { operation_id: cancel.operation() }), "Cancel generation" }
                details { class: "m1-case-settings",
                    summary { "Case settings" }
                    div { class: "m1-case-settings-body",
                        if !has_settings && !mismatch {
                            button { disabled: !can_edit_settings, onclick: move |_| set_settings(&initialize, instance_selection, None), "Add case settings" }
                        }
                        if let Ok(config) = settings && !mismatch {
                            label { "Bottom thickness (mm)"
                                input { disabled: !can_edit_settings, r#type: "number", min: "0.1", step: "0.1", value: "{config.bottom_thickness}", onchange: move |event: FormEvent| {
                                    if let Ok(value) = event.value().parse::<f64>() { set_settings(&update, instance_selection, Some(value)); }
                                } }
                            }
                        }
                        p { "PCB reference is unpopulated; case bodies use exact CAD geometry." }
                    }
                }
            }
            p { role: "status", "aria-live": "polite", "{title}" }
            if let Some(scene) = scene {
                crate::presentation::CaseViewer { key: "{scene.scope.session_epoch.0}:{scene.scope.board_id}:{scene.scope.instance_id:?}", scene }
            }
        }
    }
}
fn set_settings(
    runtime: &Rc<Runtime>,
    instance_selection: crate::presentation::InstanceSelection,
    bottom: Option<f64>,
) {
    let model = runtime.model();
    if !instance_selection.is_current(&model) {
        runtime.report("Wait for physical assembly selection before changing case settings.");
        return;
    }
    let Some(snapshot) = model.accepted else {
        return;
    };
    if model.lifecycle != Lifecycle::Ready
        || model.display_preview.is_some()
        || model.gesture.is_some()
        || model.durability
            != (Durability::Saved {
                revision: snapshot.document.revision,
            })
    {
        runtime.report(
            "Finish or cancel the position edit and wait for saving before changing case settings.",
        );
        return;
    }
    let Some(scope) = runtime.scope() else {
        return;
    };
    let result = captured_case_document(&snapshot, &scope)
        .map_err(|error| format!("{error:?}"))
        .and_then(|document| {
            if document
                .mechanical
                .as_ref()
                .is_some_and(|config| config.board_id != model.active_board_id)
            {
                return Err(
                    "Show the configured board before changing its case settings.".to_owned(),
                );
            }
            case_settings::initial_settings(&document, &model.active_board_id)
        })
        .and_then(|mut config| {
            if let Some(bottom) = bottom {
                case_settings::update_bottom_thickness(&mut config, bottom)?;
            }
            if let Some(instance_id) = model.active_instance_id.as_ref() {
                case_settings::update_instance_settings(
                    &snapshot.document,
                    instance_id,
                    Some(config),
                )
                .map(|document| EditOperation::ReplaceDocument {
                    document: Box::new(document),
                })
            } else {
                Ok(EditOperation::SetMechanical {
                    configuration: Some(Box::new(config)),
                })
            }
        });
    match result {
        Ok(edit_operation) => {
            let operation_id = runtime.operation();
            runtime.submit(Event::Edit {
                operation_id,
                command: EditCommand {
                    base_revision: snapshot.document.revision,
                    transaction_id: format!("case-settings-{}", operation_id.0),
                    phase: EditPhase::Commit,
                    target_ids: vec![],
                    operation: edit_operation,
                },
            });
        }
        Err(error) => runtime.report(error),
    }
}

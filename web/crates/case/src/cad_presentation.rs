//! Canvas lifetime and case controls consume immutable session snapshots.
use crate::runtime::{CadScene, Runtime};
use crate::viewport_camera::current_exact_case;
use boardstudio_application::{Event, GenerationStatus};
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertDescription, AlertVariant},
    button::{Button, ButtonVariant},
    checkbox::{Checkbox, CheckboxState},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    field::FieldLabel,
};
use dioxus::prelude::*;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;

use crate::case_generation_lifecycle::{
    CaseGenerationOwner, CaseGeometryStatus, case_generation_title,
};

#[component]
pub fn CasePanel(
    generation_ready: bool,
    mechanical_settings: Option<crate::presentation::MechanicalSettingsProps>,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let instance_selection = use_context::<crate::presentation::InstanceSelection>();
    let runtime_version = use_context::<Signal<u64>>();
    let _ = runtime_version();
    crate::case_preview_lifecycle::use_native_case_preview(
        runtime.clone(),
        runtime.native_case_preview_key(),
    );
    let accepted_preview = runtime.native_case_preview();
    use_effect(use_reactive((&runtime_version(),), {
        let runtime = runtime.clone();
        move |_| {
            // Runtime snapshots are not Dioxus signals. Re-read them when its
            // subscribed version changes so a preview published after mount
            // starts delivery, and so the completed batch becomes visible.
            if let Some(preview) = runtime.native_case_preview() {
                let runtime = runtime.clone();
                spawn_local(async move {
                    if let Err(error) = runtime.deliver_native_case_models(preview).await {
                        runtime.report(format!("Case model delivery failed: {error}"));
                    }
                });
            }
        }
    }));
    let model = runtime.model();
    let Some(snapshot) = model.accepted.as_ref() else {
        return rsx! {};
    };
    let case_generation = use_context::<crate::presentation::CaseGenerationState>();
    let mut live_preview = case_generation.live_preview;
    let mut automatic_generation = case_generation.automatic;
    let current_scope = runtime.scope();
    let generation_owner = current_scope.clone().map(|scope| CaseGenerationOwner {
        scope,
        token: snapshot.token,
        revision: snapshot.document.revision,
    });
    let current_scene = runtime.cad_scene();
    let has_reusable_result = generation_owner
        .as_ref()
        .zip(current_scene.as_deref())
        .is_some_and(|(owner, scene)| exact_scene(owner, scene));
    let generation_busy = matches!(
        model.generation,
        GenerationStatus::Preparing { .. } | GenerationStatus::Running { .. }
    );
    let mechanical_export_configured = current_scope.as_ref().is_some_and(|scope| {
        boardstudio_web_host::cad_jobs::captured_case_document(snapshot, scope)
            .ok()
            .and_then(|document| {
                document
                    .mechanical
                    .as_ref()
                    .map(|configuration| configuration.board_id == scope.board_id)
            })
            .unwrap_or(false)
    });
    let export_outline_ready = snapshot
        .scene
        .board_readiness
        .iter()
        .find(|item| item.board_id == model.active_board_id)
        .map_or(
            snapshot.document.boards.len() <= 1 && snapshot.scene.readiness.outline,
            |item| item.outline,
        );
    let exact_generation = matches!(
        model.generation,
        GenerationStatus::Ready { exact: true, .. }
    );
    let mechanical_export_ready = mechanical_export_configured
        && generation_ready
        && !generation_busy
        && export_outline_ready
        && exact_generation
        && has_reusable_result
        && current_scene.as_ref().is_some_and(|scene| {
            scene.mechanical.as_ref().is_some_and(|assembly| {
                assembly.revision == snapshot.document.revision
                    && !assembly.generation_blocked
                    && !assembly.diagnostics.iter().any(|finding| {
                        finding.severity == boardstudio_model::model::Severity::Error
                    })
            })
        });
    use_effect(use_reactive(
        (
            &runtime_version(),
            &generation_ready,
            &generation_owner,
            &live_preview(),
        ),
        {
            let runtime = runtime.clone();
            move |(version, generation_ready, owner, live)| {
                let _ = version;
                let owner = owner.clone();
                let Some(owner) = owner else { return };
                let accepted_is_current = runtime.scope().as_ref() == Some(&owner.scope)
                    && runtime.model().accepted.as_ref().is_some_and(|accepted| {
                        accepted.token == owner.token
                            && accepted.document.revision == owner.revision
                    });
                let mut generation = automatic_generation;
                if generation.write().observe(
                    Some(owner.clone()),
                    generation_ready && live && accepted_is_current,
                    has_reusable_result,
                    generation_busy,
                ) {
                    runtime.submit(Event::StartGeneration {
                        operation_id: runtime.operation(),
                        scope: owner.scope,
                    });
                }
            }
        },
    ));
    let generate = runtime.clone();
    let cancel = runtime.clone();
    let export_mechanical = runtime.clone();
    let export_owner = generation_owner.clone();
    let export_instance_selection = instance_selection;
    let export_mechanical_click = move |_| {
        let model = export_mechanical.model();
        if let Some(owner) = export_owner.as_ref()
            && export_mechanical.scope().as_ref() == Some(&owner.scope)
            && export_instance_selection.is_current(&model)
            && model.accepted.as_ref().is_some_and(|accepted| {
                accepted.token == owner.token
                    && accepted.document.revision == owner.revision
                    && accepted.session_epoch == owner.scope.session_epoch
            })
            && export_mechanical
                .cad_scene()
                .is_some_and(|scene| exact_scene(owner, &scene))
        {
            export_mechanical.export_mechanical();
        }
    };
    let scene = current_scene;
    let mut live_preview_control = live_preview;
    let mut automatic_generation_control = automatic_generation;
    let toggle_runtime = runtime.clone();
    let generation_enabled = automatic_generation.read().enabled();
    let stale = scene
        .as_ref()
        .is_some_and(|scene| scene.token != snapshot.token);
    let geometry = if has_reusable_result {
        CaseGeometryStatus::CurrentExact
    } else if stale {
        CaseGeometryStatus::Previous
    } else if scene.is_some() {
        CaseGeometryStatus::Preview
    } else {
        CaseGeometryStatus::Missing
    };
    let title = case_generation_title(&model.generation, geometry);
    let on_generate = use_callback(move |()| {
        if instance_selection.is_current(&generate.model())
            && let Some(scope) = generate.scope()
        {
            generate.submit(Event::StartGeneration {
                operation_id: generate.operation(),
                scope,
            });
        }
    });
    let on_cancel = use_callback(move |()| {
        live_preview.set(false);
        automatic_generation.write().set_enabled(false);
        cancel.submit(Event::CancelGeneration {
            operation_id: cancel.operation(),
        });
    });
    use boardstudio_web_ui_shared::ui::command::{Command, use_commands};
    use_commands(vec![
        Command {
            group: "Current Workspace",
            label: "Update preview".into(),
            reason: (!generation_ready || generation_busy).then(|| title.to_string()),
            hint: None,
            on_select: on_generate,
        },
        Command {
            group: "Current Workspace",
            label: "Cancel".into(),
            reason: (!generation_busy).then(|| title.to_string()),
            hint: None,
            on_select: on_cancel,
        },
    ]);
    let live_preview_id = format!("case-live-preview-{}", dioxus::core::current_scope_id().0);
    rsx! {
        section { class: "relative flex h-full min-h-0 flex-1 flex-col gap-0 overflow-hidden bg-canvas p-0 [&_h2]:m-0 [&_p]:m-0", "aria-label": "Case assembly",
            p { class: "order-[9] m-0 flex-none px-3 py-1 text-xs leading-[1.35] text-muted-foreground", role: "status", "aria-live": "polite", "{title}" }
            if runtime.native_case_preview_pending() {
                p { class: "order-[9] m-0 flex-none px-3 py-1 text-xs leading-[1.35] text-muted-foreground", role: "status", "aria-live": "polite", "Preparing the accepted PCB preview…" }
            } else if runtime.native_case_preview_error().is_some() {
                p { class: "order-[9] m-0 flex-none px-3 py-1 text-xs leading-[1.35] text-destructive", role: "alert", "The accepted PCB preview could not be prepared." }
            }
            if let Some(preview) = accepted_preview.as_ref()
                && let Some(rows) = runtime.native_model_delivery(preview)
                && (rows.delivered.len() != preview.preview.models.len() || !rows.failures.is_empty())
            {
                Collapsible { class: "absolute top-[46px] right-3 z-[6] max-w-[min(420px,calc(100%-24px))] border border-border bg-card px-[9px] py-1.5 max-[760px]:top-14 max-[760px]:right-2 max-[760px]:max-w-[calc(100%-16px)]", keep_mounted: true, default_open: false,
                    CollapsibleTrigger { class: "text-xs text-muted-foreground", aria_label: "Board models: {rows.delivered.len()} of {preview.preview.models.len()} decoded", "Board models: {rows.delivered.len()} of {preview.preview.models.len()} decoded" }
                    CollapsibleContent { class: "data-[open=false]:hidden",
                        for failure in rows.failures.iter() {
                            Alert { variant: AlertVariant::Destructive, AlertDescription { "{failure.reference}: {failure.reason}" } }
                        }
                    }
                }
            }
            if let Some(scope) = scene.as_ref().map(|scene| &scene.scope)
                .or_else(|| accepted_preview.as_ref().map(|preview| &preview.owner.scope))
            {
                crate::shared_viewer::CaseViewport {
                    key: "{scope.session_epoch.0}:{scope.board_id}:{scope.instance_id:?}:{scene.is_none()}",
                    model_rows: accepted_preview.as_ref().and_then(|preview| runtime.native_model_delivery(preview)),
                    mechanical_settings: scene.as_ref().and(mechanical_settings),
                    scene,
                    preview: accepted_preview,
                }
            }
            footer { class: "relative order-10 m-0 flex flex-none flex-wrap items-center gap-1.5 border-0 border-t border-border bg-card px-3 py-1.5 [&_h2]:hidden max-[760px]:gap-2 max-[760px]:[&>button]:min-h-11",
                h2 { "Case assembly" }
                FieldLabel { class: "mx-1.5 flex items-center gap-1.5 text-xs text-muted-foreground max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", html_for: live_preview_id.clone(),
                    Checkbox {
                        id: live_preview_id.clone(),
                        aria_label: "Live preview",
                        checked: Some(if generation_enabled { CheckboxState::Checked } else { CheckboxState::Unchecked }),
                        on_checked_change: move |state: CheckboxState| {
                            let enabled = bool::from(state);
                            live_preview_control.set(enabled);
                            automatic_generation_control.write().set_enabled(enabled);
                            if !enabled && generation_busy {
                                toggle_runtime.submit(Event::CancelGeneration {
                                    operation_id: toggle_runtime.operation(),
                                });
                            }
                        },
                    }
                    span { "Live preview" }
                }
                Button { variant: ButtonVariant::Secondary, button_type: "button",
                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    disabled: !generation_ready || generation_busy, onclick: move |_| on_generate.call(()), "Update preview" }
                Button { variant: ButtonVariant::Secondary, button_type: "button",
                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    disabled: !generation_busy,
                    onclick: move |_| on_cancel.call(()),
                    "Cancel"
                }
                if mechanical_export_configured {
                    Button { variant: ButtonVariant::Secondary, button_type: "button",
                        class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                        disabled: !mechanical_export_ready,
                        onclick: export_mechanical_click,
                        "Export geometry"
                    }
                }
            }

        }
    }
}

fn exact_scene(owner: &CaseGenerationOwner, scene: &CadScene) -> bool {
    current_exact_case(
        owner,
        &scene.scope,
        scene.token,
        scene.snapshot.document.revision,
        scene.prepared.revision,
        scene.exact,
    )
}

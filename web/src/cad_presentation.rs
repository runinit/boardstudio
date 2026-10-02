//! Canvas lifetime and case controls consume immutable session snapshots.
use crate::runtime::Runtime;
use boardstudio_application::{Event, GenerationStatus};
use dioxus::prelude::*;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn CasePanel(generation_ready: bool) -> Element {
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
    let generate = runtime.clone();
    let cancel = runtime.clone();
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
                button { disabled: !generation_ready, onclick: move |_| if instance_selection.is_current(&generate.model()) && let Some(scope) = generate.scope() { generate.submit(Event::StartGeneration { operation_id: generate.operation(), scope }); }, "Generate case" }
                button { disabled: !matches!(model.generation, GenerationStatus::Preparing {..} | GenerationStatus::Running {..}), onclick: move |_| cancel.submit(Event::CancelGeneration { operation_id: cancel.operation() }), "Cancel generation" }
            }
            p { role: "status", "aria-live": "polite", "{title}" }
            if runtime.native_case_preview_pending() {
                p { role: "status", "aria-live": "polite", "Preparing the accepted PCB preview…" }
            } else if runtime.native_case_preview_error().is_some() {
                p { role: "alert", "The accepted PCB preview could not be prepared." }
            } else if runtime.native_case_preview().is_some() {
                p { role: "status", "Accepted PCB preview is ready for the Case viewer." }
            }
            if let Some(preview) = accepted_preview.as_ref()
                && let Some(rows) = runtime.native_model_delivery(&preview)
            {
                p { role: "status", "{rows.delivered.len()} of {preview.preview.models.len()} board models decoded." }
                for failure in rows.failures.iter().take(3) {
                    p { role: "status", "{failure.reference}: {failure.reason}" }
                }
            }
            if scene.is_none()
                && let Some(preview) = accepted_preview.clone()
            {
                crate::presentation::CasePreviewViewer {
                    key: "{preview.owner.scope.session_epoch.0}:{preview.owner.scope.board_id}:{preview.owner.scope.instance_id:?}:preview",
                    model_rows: runtime.native_model_delivery(&preview),
                    preview,
                }
            }
            if let Some(scene) = scene {
                crate::presentation::CaseViewer { key: "{scene.scope.session_epoch.0}:{scene.scope.board_id}:{scene.scope.instance_id:?}", scene }
            }
        }
    }
}

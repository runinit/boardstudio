//! Canvas lifetime and case controls consume immutable session snapshots.
use crate::runtime::Runtime;
use boardstudio_application::{Event, GenerationStatus};
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub fn CasePanel(generation_ready: bool) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let instance_selection = use_context::<crate::presentation::InstanceSelection>();
    let _ = use_context::<Signal<u64>>()();
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
            if let Some(scene) = scene {
                crate::presentation::CaseViewer { key: "{scene.scope.session_epoch.0}:{scene.scope.board_id}:{scene.scope.instance_id:?}", scene }
            }
        }
    }
}

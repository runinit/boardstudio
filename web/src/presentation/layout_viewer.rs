//! Layout's private consumer for the canonical board source and shared viewer.
use super::{
    case_display::CaseDisplay,
    objects::{self, TreeSelectRequest},
    selection::{self, SelectionAdapter},
    shared_viewer::{CaseSharedViewer, ScopedDisplayChange, ScopedViewerSignal, ViewerSignalKind},
};
use crate::runtime::Runtime;
use boardstudio_application::SelectionMode;
use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};
use wasm_bindgen_futures::spawn_local;

#[derive(Props, Clone, PartialEq)]
pub(crate) struct LayoutCanonicalViewerProps {
    #[props(default)]
    pub(crate) keycaps_fit: Option<super::keycaps_fit::KeycapsFitState>,
}

#[component]
pub(crate) fn LayoutCanonicalViewer(props: LayoutCanonicalViewerProps) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let selection = use_context::<SelectionAdapter>();
    let theme = use_context::<super::ResolvedTheme>().0;
    let _ = use_context::<Signal<u64>>()();
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        let runtime = runtime.clone();
        move || {
            alive.set(false);
            runtime.cancel_keycaps_cad_preview();
            if let Some(generation) = runtime.layout_source_generation() {
                runtime.retire_layout_source(generation);
            }
        }
    });

    let keycaps_input = props
        .keycaps_fit
        .as_ref()
        .and_then(super::keycaps_fit::KeycapsFitState::current_preview_input);
    let mut keycaps_preview = use_signal(|| None::<crate::runtime::KeycapsCadPreview>);
    let mut keycaps_preview_pending = use_signal(|| false);
    let mut keycaps_preview_error = use_signal(|| None::<String>);
    let mut keycaps_preview_sequence = use_signal(|| 0_u64);
    let mut keycaps_retry_generation = use_signal(|| 0_u64);
    use_effect(use_reactive(
        (&keycaps_input, &keycaps_retry_generation()),
        {
            let runtime = runtime.clone();
            let alive = alive.clone();
            move |(input, _retry)| {
                // This effect owns the sequence; reading it reactively here would make its
                // own write restart the effect and continually cancel the CAD request.
                let sequence = keycaps_preview_sequence.peek().saturating_add(1);
                keycaps_preview_sequence.set(sequence);
                runtime.cancel_keycaps_cad_preview();
                keycaps_preview.set(None);
                keycaps_preview_error.set(None);
                let Some(input) = input else {
                    keycaps_preview_pending.set(false);
                    return;
                };
                keycaps_preview_pending.set(true);
                let runtime = runtime.clone();
                let alive = alive.clone();
                spawn_local(async move {
                    let result = runtime.request_keycaps_cad_preview(input).await;
                    if !alive.get() || keycaps_preview_sequence.peek().ne(&sequence) {
                        return;
                    }
                    keycaps_preview_pending.set(false);
                    match result {
                        Ok(preview) => {
                            keycaps_preview.set(Some(preview));
                            keycaps_preview_error.set(None);
                        }
                        Err(error) => keycaps_preview_error.set(Some(error)),
                    }
                });
            }
        },
    ));

    let model = runtime.model();
    let request = runtime.scope().and_then(|scope| {
        model
            .accepted
            .as_ref()
            .map(|accepted| (scope, accepted.token, accepted.document.revision))
    });
    use_effect(use_reactive((&request,), {
        let runtime = runtime.clone();
        let alive = alive.clone();
        move |(request,)| {
            runtime.reconcile_layout_source_request(
                request
                    .as_ref()
                    .map(|(scope, token, revision)| (scope, *token, *revision)),
            );
            let Some((scope, token, revision)) = request else {
                return;
            };
            if runtime.layout_preview().is_some() || runtime.layout_preview_pending() {
                return;
            }
            let runtime = runtime.clone();
            let alive = alive.clone();
            spawn_local(async move {
                if !alive.get() {
                    return;
                }
                let _ = runtime.prepare_layout_preview(scope, token, revision).await;
            });
        }
    }));

    let preview = runtime.layout_preview();
    let model_rows = preview
        .as_ref()
        .and_then(|preview| runtime.layout_model_delivery(preview));
    let mut display = use_signal(CaseDisplay::default);
    let on_signal = {
        let runtime = runtime.clone();
        let selection = selection.clone();
        let preview = preview.clone();
        move |event: ScopedViewerSignal| {
            if !event.is_current() {
                return;
            }
            let Some(preview) = preview.as_ref() else {
                return;
            };
            let Some(current) = runtime.layout_preview() else {
                return;
            };
            if current.owner != preview.owner
                || !Rc::ptr_eq(&current.lease, &preview.lease)
                || event.identity.scope != preview.owner.scope
                || event.identity.snapshot_token != preview.owner.snapshot_token
            {
                return;
            }
            match event.kind {
                ViewerSignalKind::Picked(reference) => {
                    let model = runtime.model();
                    if model.active_board_id != preview.owner.scope.board_id
                        || model.active_instance_id != preview.owner.scope.instance_id
                    {
                        return;
                    }
                    let Some(accepted) = model.accepted.as_ref() else {
                        return;
                    };
                    let Some(part_id) = preview.part_for_current_pick(
                        accepted,
                        &preview.owner.scope,
                        preview.owner.source_generation,
                        &reference,
                    ) else {
                        return;
                    };
                    let Some(context) = objects::context_for_part(&model, &part_id) else {
                        return;
                    };
                    selection::submit_context(
                        &runtime,
                        &selection,
                        TreeSelectRequest {
                            scope: preview.owner.scope.clone(),
                            context,
                            mode: SelectionMode::Replace,
                            outline_action: None,
                        },
                    );
                }
                ViewerSignalKind::Failed(message) => runtime.report(message),
                _ => {}
            }
        }
    };
    let on_display_change = {
        let preview = preview.clone();
        move |event: ScopedDisplayChange| {
            if event.is_current()
                && preview.as_ref().is_some_and(|preview| {
                    event.identity.scope == preview.owner.scope
                        && event.identity.snapshot_token == preview.owner.snapshot_token
                        && preview.lease.matches(&preview.owner)
                })
            {
                display.set(event.display);
            }
        }
    };

    let Some(preview) = preview else {
        let message = if let Some(error) = runtime.layout_preview_error() {
            format!("3D preview unavailable: {error}")
        } else if runtime.layout_preview_pending() {
            "Preparing canonical board preview…".to_owned()
        } else if request.is_none() {
            "Open a project and select a board to view its assembly.".to_owned()
        } else {
            "Preparing canonical board preview…".to_owned()
        };
        return rsx! {
            div {
                p { role: "status", class: "m1-layout-viewer-status", "{message}" }
                if let Some(blocker) = props.keycaps_fit.as_ref().and_then(|fit| fit.preview_blocker_message()) {
                    p { role: "status", class: "m1-layout-viewer-status", "{blocker}" }
                } else if keycaps_preview_pending() {
                    p { role: "status", class: "m1-layout-viewer-status", "Generating keycap CAD…" }
                }
                if let Some(error) = keycaps_preview_error() {
                    p { role: "alert", class: "m1-layout-viewer-status", "Keycap preview failed: {error}" }
                    button {
                        r#type: "button",
                        onclick: move |_| keycaps_retry_generation.set(keycaps_retry_generation().saturating_add(1)),
                        "Retry keycaps"
                    }
                }
            }
        };
    };

    rsx! {
        div { class: "m1-layout-canonical-viewer",
            if let Some(blocker) = props.keycaps_fit.as_ref().and_then(|fit| fit.preview_blocker_message()) {
                p { role: "status", class: "m1-layout-viewer-status", "{blocker}" }
            } else if keycaps_preview_pending() {
                p { role: "status", class: "m1-layout-viewer-status", "Generating keycap CAD…" }
            }
            if let Some(error) = keycaps_preview_error() {
                div { role: "alert", class: "m1-layout-viewer-status",
                    p { "Keycap preview failed: {error}" }
                    button {
                        r#type: "button",
                        onclick: move |_| keycaps_retry_generation.set(keycaps_retry_generation().saturating_add(1)),
                        "Retry keycaps"
                    }
                }
            }
            CaseSharedViewer {
                scene: None,
                preview: None,
                layout_preview: Some(preview),
                keycaps_preview: keycaps_preview(),
                parts_preview: None,
                model_rows,
                selected_layer: "pcb".to_owned(),
                display: display(),
                resolved_theme: theme,
                on_signal,
                on_display_change,
                mechanical_settings: None,
            }
        }
    }
}

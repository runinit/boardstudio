//! Shared reactive preview sources for the 3D viewport: the canonical layout
//! board preview and the keycap CAD preview. Case, Layout and Keycaps all load
//! through these hooks so cancellation and staleness behave identically.

use boardstudio_web_runtime::layout_viewer_source::LayoutPreviewSnapshot;
use boardstudio_web_runtime::runtime::{KeycapsCadPreview, KeycapsPreviewInput, Runtime};
use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};
use wasm_bindgen_futures::spawn_local;

pub struct KeycapsCadPreviewState {
    pub preview: Option<KeycapsCadPreview>,
    pub pending: bool,
    pub error: Option<String>,
    pub retry: EventHandler<()>,
}

/// Request keycap CAD for `input`; a newer input (or unmount) cancels the
/// in-flight request and its late result is dropped.
pub fn use_keycaps_cad_preview(
    runtime: Rc<Runtime>,
    input: Option<KeycapsPreviewInput>,
) -> KeycapsCadPreviewState {
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        let runtime = runtime.clone();
        move || {
            alive.set(false);
            runtime.cancel_keycaps_cad_preview();
        }
    });
    let mut preview = use_signal(|| None::<KeycapsCadPreview>);
    let mut pending = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut sequence = use_signal(|| 0_u64);
    let mut retry_generation = use_signal(|| 0_u64);
    use_effect(use_reactive((&input, &retry_generation()), {
        let runtime = runtime.clone();
        let alive = alive.clone();
        move |(input, _retry)| {
            // This effect owns the sequence; reading it reactively here would make its
            // own write restart the effect and continually cancel the CAD request.
            let next = sequence.peek().saturating_add(1);
            sequence.set(next);
            runtime.cancel_keycaps_cad_preview();
            preview.set(None);
            error.set(None);
            let Some(input) = input else {
                pending.set(false);
                return;
            };
            pending.set(true);
            let runtime = runtime.clone();
            let alive = alive.clone();
            spawn_local(async move {
                let result = runtime.request_keycaps_cad_preview(input).await;
                if !alive.get() || sequence.peek().ne(&next) {
                    return;
                }
                pending.set(false);
                match result {
                    Ok(ready) => {
                        preview.set(Some(ready));
                        error.set(None);
                    }
                    Err(message) => error.set(Some(message)),
                }
            });
        }
    }));
    KeycapsCadPreviewState {
        preview: preview(),
        pending: pending(),
        error: error(),
        retry: EventHandler::new(move |_| retry_generation.set(retry_generation() + 1)),
    }
}

/// Reconcile the runtime's canonical layout preview with the accepted
/// snapshot and retire the source on unmount. Returns the current preview
/// and whether an accepted board is selected at all.
pub fn use_layout_preview_source(
    runtime: Rc<Runtime>,
) -> (Option<Rc<LayoutPreviewSnapshot>>, bool) {
    let _ = use_context::<Signal<u64>>();
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        let runtime = runtime.clone();
        move || {
            alive.set(false);
            if let Some(generation) = runtime.layout_source_generation() {
                runtime.retire_layout_source(generation);
            }
        }
    });
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
                request.as_ref().map(|(scope, token, _)| (scope, *token)),
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
    (runtime.layout_preview(), request.is_some())
}

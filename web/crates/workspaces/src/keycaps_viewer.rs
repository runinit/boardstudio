//! Keycaps 3D assembly viewer: mounts the shared `<Viewport3D />` host over
//! the canonical layout preview with the keycap CAD overlay, so Keycaps 3D
//! shares identical camera, focus and WebGL context-loss behavior with
//! Layout 3D, Case and Parts. Layout and Keymap mount this same component.

use crate::keycaps_finding_marker::FocusedFinding;
use boardstudio_application::SelectionMode;
use boardstudio_web_runtime::layout_viewer_source::LayoutKeycapPickSource;
use boardstudio_web_runtime::runtime::Runtime;
use boardstudio_web_ui_model::finding_navigation::{FindingNavigationTarget, KeycapsFitState};
use boardstudio_web_ui_model::owner::owner_is_current;
use boardstudio_web_ui_model::selection::{ContextRequest, SelectionAdapter, submit_context};
use boardstudio_web_ui_model::state::ResolvedAppearance;
use boardstudio_web_ui_model::tree::{TreeContext, context_for_part};
use boardstudio_web_ui_shared::case_display::CaseDisplay;
use boardstudio_web_ui_shared::keyboard::stop_space_pan;
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertDescription, AlertVariant},
    button::{Button, ButtonVariant},
    callout::{Callout, CalloutVariant},
    spinner::Spinner,
};
use boardstudio_web_ui_shared::viewport_3d::sources::{
    use_keycaps_cad_preview, use_layout_preview_source,
};
use boardstudio_web_ui_shared::viewport_3d::{
    ScopedDisplayChange, ScopedViewerSignal, ViewerFocusRequest, ViewerSignalKind, Viewport3D,
    ViewportSource,
};
use dioxus::prelude::*;
use std::rc::Rc;

fn keycap_status(
    blocker: Option<String>,
    pending: bool,
    error: Option<String>,
    on_retry: EventHandler,
) -> Element {
    rsx! {
        if let Some(blocker) = blocker {
            div { role: "status", class: "m-0 px-3 py-2 first:pt-28", Callout { variant: CalloutVariant::Warning, "{blocker}" } }
        } else if pending {
            div { role: "status", class: "m-0 px-3 py-2 first:pt-28",
                Callout { variant: CalloutVariant::Info, Spinner {} "Generating keycap CAD…" }
            }
        }
        if let Some(error) = error {
            Alert { class: "m-0 px-3 py-2 first:pt-28", variant: AlertVariant::Destructive,
                AlertDescription { "Keycap preview failed: {error}" }
            }
            Button {
                variant: ButtonVariant::Secondary,
                button_type: "button",
                onkeydown: stop_space_pan,
                onclick: move |_| on_retry.call(()),
                "Retry keycaps"
            }
        }
    }
}

#[component]
pub fn KeycapsAssemblyViewer(
    #[props(default)] keycaps_fit: Option<KeycapsFitState>,
    #[props(default)] focused_finding: Option<FocusedFinding>,
    #[props(default)] on_mounted_module_pick: Option<EventHandler<String>>,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let selection = use_context::<SelectionAdapter>();
    let theme = use_context::<ResolvedAppearance>().0;

    let (preview, has_request) = use_layout_preview_source(runtime.clone());
    let keycaps = use_keycaps_cad_preview(
        runtime.clone(),
        keycaps_fit
            .as_ref()
            .and_then(KeycapsFitState::current_preview_input),
    );
    let keycaps_preview = keycaps.preview.clone();
    let rendered_keycaps_generation = keycaps_preview.as_ref().map(|preview| preview.generation);

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
                || !owner_is_current(
                    &preview.owner.scope,
                    preview.owner.snapshot_token,
                    &event.identity.scope,
                    event.identity.snapshot_token,
                )
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
                    let submit = |context: TreeContext| {
                        submit_context(
                            &runtime,
                            &selection,
                            ContextRequest {
                                scope: preview.owner.scope.clone(),
                                context,
                                mode: SelectionMode::Replace,
                            },
                        );
                    };
                    if let Some(module_id) = preview.module_for_current_pick(
                        accepted,
                        &preview.owner.scope,
                        preview.owner.source_generation,
                        &reference,
                    ) {
                        if let Some(on_pick) = on_mounted_module_pick {
                            on_pick.call(module_id);
                        } else {
                            submit(TreeContext::MountedModule {
                                board_id: preview.owner.scope.board_id.clone(),
                                module_id,
                            });
                        }
                        return;
                    }
                    let part_id = {
                        let body_ids = keycaps_preview
                            .as_ref()
                            .map(|keycaps| {
                                keycaps
                                    .bodies
                                    .iter()
                                    .map(|body| body.id.clone())
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();
                        let pick_source = keycaps_preview
                            .as_ref()
                            .zip(rendered_keycaps_generation)
                            .map(|(keycaps, rendered_generation)| LayoutKeycapPickSource {
                                scope: &keycaps.scope,
                                snapshot_token: keycaps.token,
                                revision: keycaps.revision,
                                generation: keycaps.generation,
                                rendered_generation,
                                specs: &keycaps.specs,
                                body_ids: &body_ids,
                            });
                        preview.part_for_current_pick(
                            accepted,
                            &preview.owner.scope,
                            preview.owner.source_generation,
                            &reference,
                            pick_source.as_ref(),
                        )
                    };
                    let Some(part_id) = part_id else {
                        return;
                    };
                    let Some(context) = context_for_part(&model, &part_id) else {
                        return;
                    };
                    submit(context);
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
                    owner_is_current(
                        &preview.owner.scope,
                        preview.owner.snapshot_token,
                        &event.identity.scope,
                        event.identity.snapshot_token,
                    ) && preview.lease.matches(&preview.owner)
                })
            {
                display.set(event.display);
            }
        }
    };

    let keycap_status = keycap_status(
        keycaps_fit
            .as_ref()
            .and_then(|fit| fit.preview_blocker_message()),
        keycaps.pending,
        keycaps.error.clone(),
        keycaps.retry,
    );
    let Some(preview) = preview else {
        let preview_error = runtime.layout_preview_error();
        let message = if let Some(error) = preview_error.as_ref() {
            format!("3D preview unavailable: {error}")
        } else if !has_request {
            "Open a project and select a board to view its assembly.".to_owned()
        } else {
            "Preparing canonical board preview…".to_owned()
        };
        return rsx! {
            div {
                if preview_error.is_some() {
                    Alert { class: "m-0 px-3 py-2 first:pt-28", variant: AlertVariant::Destructive,
                        AlertDescription { "{message}" }
                    }
                } else {
                    div { role: "status", class: "m-0 px-3 py-2 first:pt-28",
                        Callout { variant: CalloutVariant::Info,
                            if runtime.layout_preview_pending() || has_request { Spinner {} }
                            "{message}"
                        }
                    }
                }
                {keycap_status}
            }
        };
    };

    let focus_request = focused_finding.as_ref().and_then(|focused| {
        let accepted = runtime.model().accepted?;
        let current_preview = runtime.layout_preview()?;
        if focused.scope != preview.owner.scope
            || focused.scope != current_preview.owner.scope
            || focused.token != preview.owner.snapshot_token
            || focused.token != accepted.token
            || focused.revision != preview.document.revision
            || focused.revision != accepted.document.revision
            || !preview.lease.matches(&preview.owner)
            || !current_preview.lease.matches(&current_preview.owner)
            || !Rc::ptr_eq(&current_preview.lease, &preview.lease)
        {
            return None;
        }
        let finding = accepted
            .scene
            .findings
            .iter()
            .find(|finding| finding.id == focused.finding_id)?;
        let target = boardstudio_web_ui_model::finding_navigation::finding_navigation_target(
            finding,
            &accepted.document,
        )?;
        if boardstudio_web_ui_model::finding_navigation::target_board_id(&target)
            != focused.scope.board_id
        {
            return None;
        }
        let target_ids = match target {
            FindingNavigationTarget::Outline { .. } => vec!["pcb-selection".to_owned()],
            FindingNavigationTarget::Board { .. } => vec!["pcb".to_owned()],
            _ if !finding.target_ids.is_empty() => finding.target_ids.clone(),
            _ => return None,
        };
        Some(ViewerFocusRequest {
            scope: focused.scope.clone(),
            snapshot_token: focused.token,
            revision: focused.revision,
            navigation_id: focused.navigation_id,
            target_ids,
        })
    });

    rsx! {
        div { class: "flex min-h-0 flex-1 flex-col",
            {keycap_status}
            div { class: "flex min-h-0 flex-1 [&>div]:relative [&>div]:flex [&>div]:min-h-0 [&>div]:min-w-0 [&>div]:flex-1 [&>div]:flex-col [&>div>div:has(>canvas)]:relative [&>div>div:has(>canvas)]:flex [&>div>div:has(>canvas)]:min-h-[160px] [&>div>div:has(>canvas)]:min-w-0 [&>div>div:has(>canvas)]:flex-1 [&_canvas]:min-h-0 [&_canvas]:min-w-0 [&_canvas]:flex-1 [&_canvas]:border-0",
            Viewport3D {
                source: ViewportSource::Layout(preview),
                keycaps_preview: keycaps.preview,
                model_rows,
                display: display(),
                resolved_theme: theme,
                on_signal,
                on_display_change,
                focus_request,
            }
            }
        }
    }
}

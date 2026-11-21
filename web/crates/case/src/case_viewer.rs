//! Case owns display persistence and domain selection; the viewer owns GPU state.
mod gesture;
use gesture::*;

use super::case_bodies::CaseBodyEdit;
use super::mechanical_settings::MechanicalSettingsPatchExt;
use super::shared_viewer::{
    CaseDisplay, CaseSharedViewer, HandleGesturePhase, ScopedDisplayChange, ScopedViewerSignal,
    ViewerFocusRequest, ViewerHandle, ViewerHandleTarget, ViewerIdentity, ViewerSignalKind,
};
use super::{InstanceSelection, ResolvedAppearance, selection::SelectionAdapter};
use crate::runtime::{CadScene, Runtime};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_model::model::polygon_contains;
use boardstudio_model::model::{
    CaseKind, GasketPlacement, MechanicalGasketAnchor, MechanicalGasketSupport,
    MechanicalGasketTrack, MechanicalMount, Mount, MountKind, ProjectDoc, Vec2,
};
use boardstudio_web_host::cad_jobs::captured_case_document;
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertDescription, AlertVariant},
    button::{Button, ButtonVariant},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    item::{Item, ItemSize},
};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

#[derive(Clone, PartialEq)]
pub struct BodySelection {
    pub scope: Scope,
    pub body_id: String,
}

#[derive(Clone, PartialEq)]
pub struct LayerSelection {
    scope: Scope,
    id: String,
    focus: Option<(SnapshotToken, u64)>,
}

#[derive(Clone, Copy)]
pub struct CaseSelection {
    pub body: Signal<Option<BodySelection>>,
    pub layer: Signal<Option<LayerSelection>>,
    pub display: Signal<BTreeMap<String, CaseDisplay>>,
    pub body_edit_portal: CaseBodyEditPortal,
}

#[derive(Clone, Copy)]
pub struct CaseBodyEditPortal {
    pub dispatch: Signal<Option<CaseBodyEditDispatch>>,
    pub editable: Signal<bool>,
}

pub type CaseBodyEditDispatch = Rc<dyn Fn(CaseBodyEdit, Scope, SnapshotToken, u64)>;

impl CaseSelection {
    pub fn layer_id(self, scope: &Scope) -> String {
        self.layer
            .read()
            .as_ref()
            .filter(|selection| &selection.scope == scope)
            .map(|selection| selection.id.clone())
            .unwrap_or_default()
    }

    pub fn select_layer(mut self, scope: Scope, id: String) {
        self.layer.set(Some(LayerSelection {
            scope,
            id,
            focus: None,
        }));
    }

    pub fn focus_layer(mut self, scope: Scope, token: SnapshotToken, revision: u64, id: String) {
        self.layer.set(Some(LayerSelection {
            scope,
            id,
            focus: Some((token, revision)),
        }));
    }

    fn viewer_focus_request(self, scene: &CadScene) -> Option<ViewerFocusRequest> {
        let layer = self.layer.read();
        let id = layer
            .as_ref()
            .filter(|selected| {
                selected.focus == Some((scene.token, scene.snapshot.document.revision))
                    && selected.scope == scene.scope
                    && is_layer(scene, &selected.id)
            })
            .map(|selected| selected.id.clone())?;
        Some(ViewerFocusRequest {
            scope: scene.scope.clone(),
            snapshot_token: scene.token,
            revision: scene.snapshot.document.revision,
            navigation_id: 0,
            target_ids: vec![id],
        })
    }

    pub fn clear_layer_for_scope(mut self, scope: &Scope) {
        if self
            .layer
            .read()
            .as_ref()
            .is_some_and(|selection| &selection.scope == scope)
        {
            self.layer.set(None);
        }
    }

    pub fn display_value(self, scope: &Scope) -> CaseDisplay {
        let key = display_key(scope);
        self.display
            .read()
            .get(&key)
            .cloned()
            .unwrap_or_else(|| read_display(&key))
    }

    pub fn save_display(mut self, scope: &Scope, display: CaseDisplay) {
        let key = display_key(scope);
        self.display.write().insert(key.clone(), display.clone());
        persist_display(&key, &display);
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
type EmitHandleGesture = Rc<dyn Fn(HandleGesturePhase, String, Option<[f32; 3]>)>;

#[cfg(all(test, target_arch = "wasm32"))]
#[derive(Clone, Default)]
struct CaseViewerGestureProbe(Rc<RefCell<Option<EmitHandleGesture>>>);

#[cfg(all(test, target_arch = "wasm32"))]
impl CaseViewerGestureProbe {
    fn emit(
        &self,
        phase: HandleGesturePhase,
        handle_id: impl Into<String>,
        point: Option<[f32; 3]>,
    ) {
        let emit = self
            .0
            .borrow()
            .clone()
            .expect("mounted CaseViewer publishes its gesture test probe");
        emit(phase, handle_id.into(), point);
    }
}

#[derive(Clone, PartialEq)]
struct CaseKeycapsSource {
    scope: Scope,
    token: SnapshotToken,
    revision: u64,
    prepared: Option<boardstudio_model::model::PreparedCaseAssemblyIR>,
}

#[derive(Clone, Default)]
struct CaseKeycapsState {
    source: Option<CaseKeycapsSource>,
    preview: Option<crate::runtime::KeycapsCadPreview>,
    findings: Vec<boardstudio_model::model::Finding>,
    pending: bool,
    error: Option<String>,
}

fn use_case_keycaps(
    runtime: Rc<Runtime>,
    scope: &Scope,
    token: SnapshotToken,
    revision: u64,
) -> (Option<crate::runtime::KeycapsCadPreview>, Element) {
    let _ = use_context::<Signal<u64>>()();
    let source = runtime
        .model()
        .accepted
        .filter(|accepted| {
            runtime.scope().as_ref() == Some(scope)
                && accepted.token == token
                && accepted.document.revision == revision
                && accepted.document.keycaps.is_some()
        })
        .map(|_| CaseKeycapsSource {
            scope: scope.clone(),
            token,
            revision,
            prepared: runtime
                .cad_scene()
                .filter(|scene| {
                    scene.exact
                        && scene.scope == *scope
                        && scene.token == token
                        && scene.snapshot.document.revision == revision
                        && scene.prepared.revision == revision
                })
                .map(|scene| scene.prepared.clone()),
        });
    let mut state = use_signal(CaseKeycapsState::default);
    let mut retry = use_signal(|| 0_u64);
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    let sequence = use_hook(|| Rc::new(Cell::new(0_u64)));
    use_drop({
        let alive = alive.clone();
        let runtime = runtime.clone();
        move || {
            alive.set(false);
            runtime.cancel_keycaps_cad_preview();
        }
    });
    use_effect(use_reactive((&source, &retry()), {
        let runtime = runtime.clone();
        let alive = alive.clone();
        let sequence = sequence.clone();
        move |(source, _)| {
            let request = sequence.get().saturating_add(1);
            sequence.set(request);
            runtime.cancel_keycaps_cad_preview();
            state.set(CaseKeycapsState {
                source: source.clone(),
                pending: source.is_some(),
                ..Default::default()
            });
            let Some(source) = source else {
                return;
            };
            let runtime = runtime.clone();
            let alive = alive.clone();
            let sequence = sequence.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let current = || {
                    alive.get()
                        && sequence.get() == request
                        && runtime.scope().as_ref() == Some(&source.scope)
                        && runtime.model().accepted.as_ref().is_some_and(|accepted| {
                            accepted.token == source.token
                                && accepted.document.revision == source.revision
                        })
                };
                let result = async {
                    let fit = runtime
                        .resolve_case_keycaps_preview(
                            source.scope.clone(),
                            source.token,
                            source.revision,
                        )
                        .await?;
                    if !current() {
                        return Err("Case keycap source changed during resolution.".to_owned());
                    }
                    if fit.specs.is_empty()
                        || fit.findings.iter().any(|finding| {
                            finding.severity == boardstudio_model::model::Severity::Error
                        })
                    {
                        return Ok((None, fit.findings));
                    }
                    let preview = runtime
                        .request_keycaps_cad_preview(crate::runtime::KeycapsPreviewInput {
                            scope: source.scope.clone(),
                            token: source.token,
                            revision: source.revision,
                            specs: fit.specs,
                        })
                        .await?;
                    Ok((Some(preview), fit.findings))
                }
                .await;
                if !current() {
                    return;
                }
                state.set(match result {
                    Ok((preview, findings)) => CaseKeycapsState {
                        source: Some(source),
                        preview,
                        findings,
                        ..Default::default()
                    },
                    Err(error) => CaseKeycapsState {
                        source: Some(source),
                        error: Some(error),
                        ..Default::default()
                    },
                });
            });
        }
    }));
    let state = state();
    let shown = if state.source == source {
        state
    } else {
        CaseKeycapsState::default()
    };
    let feedback = rsx! {
        if shown.pending { p { role: "status", "Generating keycap CAD…" } }
        if let Some(error) = &shown.error {
            Alert { variant: AlertVariant::Destructive,
                AlertDescription { "Keycap preview failed: {error}" }
                Button { variant: ButtonVariant::Secondary, button_type: "button",
                    class: "max-[960px]:min-h-11 [@media(pointer:coarse)]:min-h-11",
                    onclick: move |_| retry += 1, "Retry keycaps" }
            }
        }
        if !shown.findings.is_empty() {
            Collapsible { class: "flex-none max-h-[min(25vh,180px)] overflow-auto overscroll-contain", keep_mounted: true, default_open: true,
                CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground", aria_label: "Keycap clearance · {shown.findings.len()} findings", "Keycap clearance · {shown.findings.len()} findings" }
                CollapsibleContent { class: "data-[open=false]:hidden",
                    for finding in &shown.findings { Item { size: ItemSize::Sm, key: "{finding.id}", p { "{finding.message}" } } }
                }
            }
        }
    };
    (shown.preview, feedback)
}

#[component]
pub fn CaseViewer(
    scene: Rc<CadScene>,
    preview: Option<Rc<crate::case_preview::NativePreviewSnapshot>>,
    model_rows: Option<super::model_delivery::ModelDeliveryRows>,
    mechanical_settings: Option<super::MechanicalSettingsProps>,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let (keycaps_preview, keycaps_feedback) = use_case_keycaps(
        runtime.clone(),
        &scene.scope,
        scene.token,
        scene.snapshot.document.revision,
    );
    let instance_selection = use_context::<InstanceSelection>();
    let case_generation = use_context::<super::CaseGenerationState>();
    let runtime_version = use_context::<Signal<u64>>();
    let _ = runtime_version();
    let selection_adapter = use_context::<SelectionAdapter>();
    let selection = use_context::<CaseSelection>();
    let theme = use_context::<ResolvedAppearance>().0;
    #[cfg(all(test, target_arch = "wasm32"))]
    let gesture_probe = try_consume_context::<CaseViewerGestureProbe>();
    let selected_reference = {
        let model = runtime.model();
        let selected = selection_adapter
            .selected_context
            .read()
            .clone()
            .filter(|selected| selected.scope == scene.scope);
        crate::case_selection::selected_part_summary(&model, selected.as_ref())
            .map(|summary| summary.title)
    };
    let key = display_key(&scene.scope);
    let initial_display = use_hook({
        let key = key.clone();
        move || read_display(&key)
    });
    let display = selection
        .display
        .read()
        .get(&key)
        .cloned()
        .unwrap_or(initial_display);
    // The Case editor keeps its independently selected authored body, but a
    // newer explicit tree/viewer layer choice owns the viewer highlight.
    let selected_layer = selection
        .layer
        .read()
        .as_ref()
        .filter(|selected| selected.scope == scene.scope && is_layer(&scene, &selected.id))
        .map(|selected| selected.id.clone())
        .or_else(|| {
            selection
                .body
                .read()
                .as_ref()
                .filter(|selected| {
                    selected.scope == scene.scope && is_layer(&scene, &selected.body_id)
                })
                .map(|selected| selected.body_id.clone())
        })
        .unwrap_or_default();
    let focus_request = selection.viewer_focus_request(&scene);
    let direct_handles = case_viewer_handles(
        &scene,
        mechanical_settings.as_ref(),
        (selection.body_edit_portal.editable)(),
    );
    let gesture = use_hook(|| Rc::new(RefCell::new(None::<CaseGestureDraft>)));
    let draft_preview_owner = use_hook(|| {
        Rc::new(RefCell::new(
            None::<crate::case_gesture_preview::CaseGesturePreviewOwner>,
        ))
    });
    let handle_preview = use_signal(|| None::<Vec<ViewerHandle>>);
    let gesture_field = use_signal(|| None::<String>);
    let gesture_message = use_signal(|| None::<String>);
    use_effect(use_reactive(
        (
            &scene.scope,
            &scene.token,
            &scene.snapshot.document.revision,
            &(Rc::as_ptr(&scene) as usize),
        ),
        {
            let gesture = gesture.clone();
            let mut handle_preview = handle_preview;
            let mut gesture_field = gesture_field;
            let mut gesture_message = gesture_message;
            let draft_preview_owner = draft_preview_owner.clone();
            let runtime = runtime.clone();
            move |_| {
                if let Some(owner) = draft_preview_owner.borrow_mut().take() {
                    runtime.cancel_case_gesture_preview(&owner);
                }
                *gesture.borrow_mut() = None;
                handle_preview.set(None);
                gesture_field.set(None);
                gesture_message.set(None);
            }
        },
    ));
    use_effect(use_reactive((&(case_generation.live_preview)(),), {
        let draft_preview_owner = draft_preview_owner.clone();
        let runtime = runtime.clone();
        move |(live_preview,)| {
            if !live_preview && let Some(owner) = draft_preview_owner.borrow_mut().take() {
                runtime.cancel_case_gesture_preview(&owner);
            }
        }
    }));
    use_drop({
        let runtime = runtime.clone();
        let draft_preview_owner = draft_preview_owner.clone();
        move || {
            if let Some(owner) = draft_preview_owner.borrow_mut().take() {
                runtime.cancel_case_gesture_preview(&owner);
            }
        }
    });
    let on_signal = {
        let runtime = runtime.clone();
        let scene = scene.clone();
        let preview = preview.clone();
        let direct_handles = direct_handles.clone();
        let mechanical_settings = mechanical_settings.clone();
        let gesture = gesture.clone();
        let mut selection = selection;
        let mut handle_preview = handle_preview;
        let mut gesture_field = gesture_field;
        let mut gesture_message = gesture_message;
        let draft_preview_owner = draft_preview_owner.clone();
        let live_preview = (case_generation.live_preview)();
        move |event: super::shared_viewer::ScopedViewerSignal| {
            if !event.is_current()
                || !source_is_current(&runtime, instance_selection, &scene, &event.identity)
            {
                return;
            }
            if let ViewerSignalKind::Picked(id) = &event.kind
                && let Some(preview) = preview.as_ref()
                && select_native_preview_model(
                    &runtime,
                    instance_selection,
                    &selection_adapter,
                    preview,
                    &event.identity,
                    id,
                )
            {
                return;
            }
            match event.kind {
                ViewerSignalKind::Picked(id) | ViewerSignalKind::LayerSelected(id) => {
                    if !is_layer(&scene, &id) {
                        return;
                    }
                    selection.layer.set(Some(LayerSelection {
                        scope: scene.scope.clone(),
                        id: id.clone(),
                        focus: None,
                    }));
                    // CAD mesh IDs become authored body selections only when the
                    // current captured document proves that exact membership.
                    if let Ok(document) = captured_case_document(&scene.snapshot, &scene.scope)
                        && document
                            .case_bodies
                            .iter()
                            .any(|body| body.id == id && body.board_id == scene.scope.board_id)
                    {
                        selection.body.set(Some(BodySelection {
                            scope: scene.scope.clone(),
                            body_id: id,
                        }));
                    } else {
                        selection.body.set(None);
                    }
                }
                ViewerSignalKind::Failed(message) => runtime.report(message),
                ViewerSignalKind::HandleGesture {
                    phase,
                    handle_id,
                    point,
                } => handle_case_gesture(
                    CaseGestureContext {
                        scene: &scene,
                        identity: &event.identity,
                        settings: mechanical_settings.as_ref(),
                        handles: &direct_handles,
                        gesture: &gesture,
                        preview: &mut handle_preview,
                        feedback_field: &mut gesture_field,
                        message: &mut gesture_message,
                        selection,
                        runtime: &runtime,
                        draft_preview_owner: &draft_preview_owner,
                        live_preview,
                    },
                    phase,
                    &handle_id,
                    point,
                ),
                ViewerSignalKind::SceneAccepted(_)
                | ViewerSignalKind::Lifecycle(_)
                | ViewerSignalKind::WorldPoint(_) => {}
            }
        }
    };
    #[cfg(all(test, target_arch = "wasm32"))]
    let on_signal = EventHandler::new(on_signal);
    #[cfg(all(test, target_arch = "wasm32"))]
    if let Some(probe) = gesture_probe {
        let identity = ViewerIdentity {
            scope: scene.scope.clone(),
            snapshot_token: scene.token,
            revision: scene.snapshot.document.revision,
            viewer_instance: 0,
            projection_generation: 1,
            renderer_sequence: 1,
        };
        *probe.0.borrow_mut() = Some(Rc::new(move |phase, handle_id, point| {
            super::shared_viewer::emit_handle_gesture_for_test(
                on_signal, &identity, phase, handle_id, point,
            );
        }));
    }
    let on_display_change = {
        let runtime = runtime.clone();
        let expected_scene = scene.clone();
        let scope = scene.scope.clone();
        move |event: super::shared_viewer::ScopedDisplayChange| {
            if !event.is_current()
                || !source_is_current(
                    &runtime,
                    instance_selection,
                    &expected_scene,
                    &event.identity,
                )
            {
                return;
            }
            // Keep the entire preference value, including temporarily absent
            // geometry, so a regenerated scene or Undo can restore its display.
            selection.save_display(&scope, event.display.clone());
        }
    };
    let displayed_gesture_message = runtime.case_gesture_preview_message().or_else(|| {
        direct_gesture_message(
            gesture_field(),
            gesture_message(),
            mechanical_settings.as_ref(),
        )
    });
    let display_scene = runtime
        .case_gesture_preview_scene(&scene)
        .unwrap_or_else(|| scene.clone());
    rsx! {
        CaseSharedViewer {
            scene: Some(display_scene),
            handle_source: Some(scene.clone()),
            preview: preview.clone(),
            layout_preview: None,
            keycaps_preview,
            parts_preview: None,
            model_rows: model_rows.clone(),
            selected_layer,
            selected_reference,
            display,
            resolved_theme: theme().to_owned(),
            on_signal,
            on_display_change,
            mechanical_settings,
            inline_case_controls: true,
            handles: direct_handles,
            handle_preview: handle_preview(),
            gesture_message: displayed_gesture_message,
            focus_request,
        }
        {keycaps_feedback}
        super::case_collision::CaseCollisionChecks { scene, preview, model_rows }
    }
}

#[component]
pub fn CasePreviewViewer(
    preview: Rc<crate::case_preview::NativePreviewSnapshot>,
    model_rows: Option<super::model_delivery::ModelDeliveryRows>,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let (keycaps_preview, keycaps_feedback) = use_case_keycaps(
        runtime.clone(),
        &preview.owner.scope,
        preview.owner.snapshot_token,
        preview.owner.accepted_revision,
    );
    let instance_selection = use_context::<InstanceSelection>();
    let selection_adapter = use_context::<SelectionAdapter>();
    let selection = use_context::<CaseSelection>();
    let theme = use_context::<ResolvedAppearance>().0;
    let selected_reference = {
        let model = runtime.model();
        let selected = selection_adapter
            .selected_context
            .read()
            .clone()
            .filter(|selected| selected.scope == preview.owner.scope);
        crate::case_selection::selected_part_summary(&model, selected.as_ref())
            .map(|summary| summary.title)
    };
    let key = display_key(&preview.owner.scope);
    let initial_display = use_hook({
        let key = key.clone();
        move || read_display(&key)
    });
    let display = selection
        .display
        .read()
        .get(&key)
        .cloned()
        .unwrap_or(initial_display);
    let on_signal = {
        let runtime = runtime.clone();
        let preview = preview.clone();
        move |event: ScopedViewerSignal| {
            if !event.is_current()
                || runtime.native_case_preview().is_none_or(|current| {
                    current.owner != preview.owner || !Rc::ptr_eq(&current.lease, &preview.lease)
                })
                || event.identity.scope != preview.owner.scope
                || event.identity.snapshot_token != preview.owner.snapshot_token
            {
                return;
            }
            match event.kind {
                ViewerSignalKind::Picked(id) => {
                    select_native_preview_model(
                        &runtime,
                        instance_selection,
                        &selection_adapter,
                        &preview,
                        &event.identity,
                        &id,
                    );
                }
                ViewerSignalKind::Failed(message) => runtime.report(message),
                _ => {}
            }
        }
    };
    let on_display_change = {
        let runtime = runtime.clone();
        let preview = preview.clone();
        let scope = preview.owner.scope.clone();
        move |event: ScopedDisplayChange| {
            if !event.is_current()
                || runtime.native_case_preview().is_none_or(|current| {
                    current.owner != preview.owner || !Rc::ptr_eq(&current.lease, &preview.lease)
                })
                || event.identity.scope != preview.owner.scope
                || event.identity.snapshot_token != preview.owner.snapshot_token
            {
                return;
            }
            selection.save_display(&scope, event.display.clone());
        }
    };
    rsx! {
        CaseSharedViewer {
            scene: None,
            preview: Some(preview),
            layout_preview: None,
            keycaps_preview,
            parts_preview: None,
            model_rows,
            selected_layer: "pcb".to_owned(),
            selected_reference,
            display,
            resolved_theme: theme,
            on_signal,
            on_display_change,
            mechanical_settings: None,
            inline_case_controls: true,
            handles: Vec::new(),
            handle_preview: None,
            gesture_message: None,
        }
        {keycaps_feedback}
    }
}

fn select_native_preview_model(
    runtime: &Rc<Runtime>,
    instance_selection: InstanceSelection,
    adapter: &SelectionAdapter,
    preview: &Rc<crate::case_preview::NativePreviewSnapshot>,
    identity: &ViewerIdentity,
    model_reference: &str,
) -> bool {
    let owner = &preview.owner;
    if identity.scope != owner.scope
        || identity.snapshot_token != owner.snapshot_token
        || runtime.scope().as_ref() != Some(&owner.scope)
    {
        return false;
    }
    let model = runtime.model();
    if !instance_selection.is_current(&model)
        || model.active_board_id != owner.scope.board_id
        || model.active_instance_id != owner.scope.instance_id
    {
        return false;
    }
    let Some(accepted) = model.accepted.as_ref() else {
        return false;
    };
    if accepted.token != owner.snapshot_token
        || accepted.session_epoch != owner.scope.session_epoch
        || accepted.document.id != owner.scope.document_id
        || accepted.document.revision != owner.accepted_revision
    {
        return false;
    }
    if runtime.native_case_preview().is_none_or(|current| {
        current.owner != *owner || !Rc::ptr_eq(&current.lease, &preview.lease)
    }) {
        return false;
    }
    // Renderer event freshness is validated by ScopedViewerSignal::is_current.
    // Its instance/generation counters are independently allocated from the
    // producer lease and must not be compared across these identity domains.
    let Some(part_id) = crate::case_preview::native_preview_pick_part_id(
        preview,
        &identity.scope,
        identity.snapshot_token,
        accepted.document.revision,
        model_reference,
    ) else {
        return false;
    };
    let Some(context) = super::objects::context_for_part(&model, &part_id) else {
        return false;
    };
    super::selection::submit_context(
        runtime,
        adapter,
        super::selection::ContextRequest {
            scope: owner.scope.clone(),
            context,
            mode: boardstudio_application::SelectionMode::Replace,
        },
    );
    true
}

fn source_is_current(
    runtime: &Runtime,
    instance: InstanceSelection,
    expected: &Rc<CadScene>,
    identity: &ViewerIdentity,
) -> bool {
    let model = runtime.model();
    instance.is_current(&model)
        && runtime.scope().as_ref() == Some(&identity.scope)
        && identity.scope == expected.scope
        && identity.snapshot_token == expected.token
        && identity.revision == expected.snapshot.document.revision
        && model.accepted.as_ref().is_some_and(|accepted| {
            accepted.token == identity.snapshot_token
                && accepted.document.revision == identity.revision
        })
        && runtime
            .cad_scene()
            .as_ref()
            .is_some_and(|current| Rc::ptr_eq(current, expected))
}

fn is_layer(scene: &CadScene, id: &str) -> bool {
    id == "pcb"
        || scene.result.bodies.iter().any(|body| body.id == id)
        || scene.mechanical.as_ref().is_some_and(|assembly| {
            assembly.stack.iter().any(|layer| layer.id == id)
                || (id == "gaskets" && !assembly.gasket_supports.is_empty())
        })
        || id
            .strip_prefix("gasket:")
            .and_then(|id| id.rsplit_once(':'))
            .is_some_and(|(support_id, side)| {
                matches!(side, "lower" | "upper")
                    && scene.mechanical.as_ref().is_some_and(|assembly| {
                        assembly
                            .gasket_supports
                            .iter()
                            .any(|support| support.id == support_id)
                    })
            })
}

fn read_display(key: &str) -> CaseDisplay {
    let value = web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(key).ok().flatten())
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
    let Some(value) = value else {
        return CaseDisplay::default();
    };
    CaseDisplay {
        hidden: value
            .get("hidden")
            .and_then(|hidden| hidden.as_array())
            .map(|hidden| {
                hidden
                    .iter()
                    .filter_map(|id| id.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
        colors: value
            .get("colors")
            .and_then(|colors| colors.as_object())
            .map(|colors| {
                colors
                    .iter()
                    .filter_map(|(id, color)| {
                        color.as_str().map(|color| (id.clone(), color.to_owned()))
                    })
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn display_key(scope: &Scope) -> String {
    format!(
        "boardstudio:case-display:{}:{}",
        scope.document_id,
        scope.instance_id.as_deref().unwrap_or(&scope.board_id),
    )
}

#[cfg(all(test, target_arch = "wasm32"))]
mod keycap_consumer_tests;

#[cfg(all(test, target_arch = "wasm32"))]
mod gesture_cancellation_tests;

fn persist_display(key: &str, display: &CaseDisplay) {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let encoded = serde_json::json!({ "hidden": display.hidden, "colors": display.colors });
        let _ = storage.set_item(key, &encoded.to_string());
    }
}

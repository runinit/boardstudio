//! Case owns display persistence and domain selection; the viewer owns GPU state.
use super::case_bodies::CaseBodyEdit;
use super::shared_viewer::{
    CaseDisplay, CaseSharedViewer, HandleGesturePhase, ScopedDisplayChange, ScopedViewerSignal,
    ViewerFocusRequest, ViewerHandle, ViewerHandleTarget, ViewerIdentity, ViewerSignalKind,
};
use super::{InstanceSelection, ResolvedTheme, selection::SelectionAdapter};
use crate::runtime::{CadScene, Runtime};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{
    CaseKind, GasketPlacement, MechanicalGasketAnchor, MechanicalGasketSupport,
    MechanicalGasketTrack, MechanicalMount, Mount, MountKind, ProjectDoc, Vec2,
};
use boardstudio_web_host::cad_jobs::captured_case_document;
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
#[derive(Clone, Default)]
struct CaseViewerGestureProbe(
    Rc<RefCell<Option<Rc<dyn Fn(HandleGesturePhase, String, Option<[f32; 3]>)>>>>,
);

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
    prepared: Option<boardstudio_core::model::PreparedCaseAssemblyIR>,
}

#[derive(Clone, Default)]
struct CaseKeycapsState {
    source: Option<CaseKeycapsSource>,
    preview: Option<crate::runtime::KeycapsCadPreview>,
    findings: Vec<boardstudio_core::model::Finding>,
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
                            finding.severity == boardstudio_core::model::Severity::Error
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
            div { role: "alert",
                "Keycap preview failed: {error}"
                button { type: "button", onclick: move |_| retry += 1, "Retry keycaps" }
            }
        }
        if !shown.findings.is_empty() {
            details { class: "m1-case-keycap-findings", open: true,
                summary { "Keycap clearance · {shown.findings.len()} findings" }
                for finding in &shown.findings { p { key: "{finding.id}", "{finding.message}" } }
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
    let theme = use_context::<ResolvedTheme>().0;
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
        let on_signal = on_signal;
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
            preview,
            layout_preview: None,
            keycaps_preview,
            parts_preview: None,
            model_rows,
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
    let theme = use_context::<ResolvedTheme>().0;
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

#[derive(Clone)]
enum CaseGestureDraft {
    Gasket {
        support_id: String,
        before: Vec<MechanicalGasketSupport>,
        pending: Vec<MechanicalGasketSupport>,
        tracks: Vec<MechanicalGasketTrack>,
    },
    Mount {
        target: CaseMountTarget,
        mount_id: String,
        before: Vec<Mount>,
        pending: Vec<Mount>,
        constraints: MountMoveConstraints,
    },
}

#[derive(Clone)]
enum CaseMountTarget {
    Mechanical(super::mechanical_settings::MechanicalMountCollection),
    Authored { body_id: String },
}

#[derive(Clone)]
struct MountMoveConstraints {
    outer: Vec<Vec<Vec2>>,
    holes: Vec<Vec<Vec2>>,
}

fn case_viewer_handles(
    scene: &CadScene,
    settings: Option<&super::mechanical_settings::MechanicalSettingsProps>,
    authored_mounts_editable: bool,
) -> Vec<ViewerHandle> {
    let settings = settings.filter(|settings| {
        settings.editable
            && settings.identity.scope == scene.scope
            && settings.identity.snapshot_token == scene.token
            && settings.identity.revision == scene.snapshot.document.revision
    });
    if !scene.exact || scene.prepared.revision != scene.snapshot.document.revision {
        return Vec::new();
    }

    let mut handles = Vec::new();
    if settings.is_some()
        && let Some(assembly) = scene.mechanical.as_ref()
    {
        let z = assembly
            .stack
            .iter()
            .find(|layer| layer.id == "retainer")
            .map(|layer| layer.z + layer.thickness + 0.7)
            .unwrap_or(9.0);
        handles.extend(assembly.gasket_supports.iter().map(|support| ViewerHandle {
            id: format!("gasket-handle:{}", support.id),
            x: support.at.x as f32,
            y: support.at.y as f32,
            z: z as f32,
            tangent_x: support.tangent.x as f32,
            tangent_y: support.tangent.y as f32,
            normal_x: support.normal.x as f32,
            normal_y: support.normal.y as f32,
            length: support.length as f32,
            invalid: support.fit_error.is_some(),
            target: ViewerHandleTarget::Gasket {
                support_id: support.id.clone(),
            },
        }));
    }

    if let (Some(settings), Some(assembly)) = (settings, scene.mechanical.as_ref())
        && let Some(values) = settings.values.as_ref()
    {
        if values.mount != MechanicalMount::Gasket {
            let target_body = if values.mount == MechanicalMount::Rigid {
                "plate"
            } else {
                "bottom"
            };
            if mount_constraints(scene, target_body).is_some() {
                let z = assembly
                    .stack
                    .iter()
                    .find(|layer| layer.id == target_body)
                    .map(|layer| layer.z + 0.8)
                    .unwrap_or(0.8);
                handles.extend(values.suspension_mounts.iter().map(|mount| {
                    mount_handle(
                        mount,
                        z,
                        CaseMountTarget::Mechanical(
                            super::mechanical_settings::MechanicalMountCollection::Suspension,
                        ),
                    )
                }));
            }
        }
        if let Some(mounts) = values.closure_mounts.as_ref()
            && let Some(_constraints) = mount_constraints(scene, "bottom")
        {
            let z = assembly
                .stack
                .iter()
                .find(|layer| layer.id == "bottom")
                .map(|layer| layer.z + 0.8)
                .unwrap_or(0.8);
            handles.extend(mounts.iter().map(|mount| {
                mount_handle(
                    mount,
                    z,
                    CaseMountTarget::Mechanical(
                        super::mechanical_settings::MechanicalMountCollection::Closure,
                    ),
                )
            }));
        }
    }
    if authored_mounts_editable
        && let Ok(document) = captured_case_document(&scene.snapshot, &scene.scope)
    {
        for body in document
            .case_bodies
            .iter()
            .filter(|body| body.board_id == scene.scope.board_id)
        {
            let Some(mounts) = body.mounts.as_ref() else {
                continue;
            };
            let Some(_constraints) = mount_constraints(scene, &body.id) else {
                continue;
            };
            let z = body.z.unwrap_or(0.0) + 0.8;
            handles.extend(mounts.iter().map(|mount| {
                mount_handle(
                    mount,
                    z,
                    CaseMountTarget::Authored {
                        body_id: body.id.clone(),
                    },
                )
            }));
        }
    }
    handles
}

fn mount_handle(mount: &Mount, z: f64, target: CaseMountTarget) -> ViewerHandle {
    let (target_id, target) = match target {
        CaseMountTarget::Mechanical(collection) => (
            match collection {
                super::mechanical_settings::MechanicalMountCollection::Suspension => {
                    "suspension".to_owned()
                }
                super::mechanical_settings::MechanicalMountCollection::Closure => {
                    "closure".to_owned()
                }
            },
            ViewerHandleTarget::Mount {
                collection,
                mount_id: mount.id.clone(),
            },
        ),
        CaseMountTarget::Authored { body_id } => (
            body_id.clone(),
            ViewerHandleTarget::AuthoredMount {
                body_id,
                mount_id: mount.id.clone(),
            },
        ),
    };
    ViewerHandle {
        id: format!("case-mount:{target_id}/{}", mount.id),
        x: mount.at.x as f32,
        y: mount.at.y as f32,
        z: z as f32,
        tangent_x: 1.0,
        tangent_y: 0.0,
        normal_x: 0.0,
        normal_y: 1.0,
        length: (mount_radius(mount) * 2.0 + 3.0) as f32,
        invalid: false,
        target,
    }
}

struct CaseGestureContext<'a> {
    scene: &'a CadScene,
    identity: &'a ViewerIdentity,
    settings: Option<&'a super::mechanical_settings::MechanicalSettingsProps>,
    handles: &'a [ViewerHandle],
    gesture: &'a Rc<RefCell<Option<CaseGestureDraft>>>,
    preview: &'a mut Signal<Option<Vec<ViewerHandle>>>,
    feedback_field: &'a mut Signal<Option<String>>,
    message: &'a mut Signal<Option<String>>,
    selection: CaseSelection,
    runtime: &'a Rc<Runtime>,
    draft_preview_owner:
        &'a Rc<RefCell<Option<crate::case_gesture_preview::CaseGesturePreviewOwner>>>,
    live_preview: bool,
}

fn handle_case_gesture(
    context: CaseGestureContext<'_>,
    phase: HandleGesturePhase,
    handle_id: &str,
    point: Option<[f32; 3]>,
) {
    let CaseGestureContext {
        scene,
        identity,
        settings,
        handles,
        gesture,
        preview,
        feedback_field,
        message,
        selection,
        runtime,
        draft_preview_owner,
        live_preview,
    } = context;
    let identity_current = scene.exact
        && scene.scope == identity.scope
        && scene.token == identity.snapshot_token
        && scene.prepared.revision == scene.snapshot.document.revision;
    let settings = settings.filter(|settings| {
        settings.editable
            && settings.identity.scope == identity.scope
            && settings.identity.snapshot_token == identity.snapshot_token
            && settings.identity.revision == scene.snapshot.document.revision
            && identity_current
    });
    match phase {
        HandleGesturePhase::Start => {
            let Some(handle) = handles.iter().find(|handle| handle.id == handle_id) else {
                return;
            };
            *gesture.borrow_mut() = match &handle.target {
                ViewerHandleTarget::Gasket { support_id } => {
                    if settings.is_none() {
                        return;
                    }
                    let Some(assembly) = scene.mechanical.as_ref() else {
                        return;
                    };
                    let before = assembly.gasket_supports.clone();
                    if !before.iter().any(|support| support.id == *support_id) {
                        return;
                    }
                    selection
                        .select_layer(scene.scope.clone(), format!("gasket:{support_id}:lower"));
                    Some(CaseGestureDraft::Gasket {
                        support_id: support_id.clone(),
                        before: before.clone(),
                        pending: before,
                        tracks: assembly.gasket_tracks.clone(),
                    })
                }
                ViewerHandleTarget::Mount {
                    collection,
                    mount_id,
                } => {
                    let Some(settings) = settings else { return };
                    let Some(values) = settings.values.as_ref() else {
                        return;
                    };
                    let before = match collection {
                        super::mechanical_settings::MechanicalMountCollection::Suspension => {
                            values.suspension_mounts.clone()
                        }
                        super::mechanical_settings::MechanicalMountCollection::Closure => {
                            values.closure_mounts.clone().unwrap_or_default()
                        }
                    };
                    if !before.iter().any(|mount| mount.id == *mount_id) {
                        return;
                    }
                    let body_id = match collection {
                        super::mechanical_settings::MechanicalMountCollection::Suspension
                            if values.mount == MechanicalMount::Rigid =>
                        {
                            "plate"
                        }
                        _ => "bottom",
                    };
                    let Some(constraints) = mount_constraints(scene, body_id) else {
                        return;
                    };
                    Some(CaseGestureDraft::Mount {
                        target: CaseMountTarget::Mechanical(*collection),
                        mount_id: mount_id.clone(),
                        before: before.clone(),
                        pending: before,
                        constraints,
                    })
                }
                ViewerHandleTarget::AuthoredMount { body_id, mount_id } => {
                    if !identity_current || !(selection.body_edit_portal.editable)() {
                        return;
                    }
                    let Ok(document) = captured_case_document(&scene.snapshot, &scene.scope) else {
                        return;
                    };
                    let Some(body) = document
                        .case_bodies
                        .iter()
                        .find(|body| body.id == *body_id && body.board_id == scene.scope.board_id)
                    else {
                        return;
                    };
                    let before = body.mounts.clone().unwrap_or_default();
                    if !before.iter().any(|mount| mount.id == *mount_id) {
                        return;
                    }
                    let Some(constraints) = mount_constraints(scene, body_id) else {
                        return;
                    };
                    Some(CaseGestureDraft::Mount {
                        target: CaseMountTarget::Authored {
                            body_id: body_id.clone(),
                        },
                        mount_id: mount_id.clone(),
                        before: before.clone(),
                        pending: before,
                        constraints,
                    })
                }
            };
            feedback_field.set(None);
            message.set(Some(if handle_id.starts_with("gasket-handle:") {
                "Drag the gasket support; release to save its position.".to_owned()
            } else {
                "Drag the mount; release to save its case position.".to_owned()
            }));
        }
        HandleGesturePhase::Move => {
            let Some(point) = point.map(world_xy) else {
                return;
            };
            let Some(current) = gesture.borrow().clone() else {
                return;
            };
            let mut valid_point = false;
            match current {
                CaseGestureDraft::Gasket {
                    support_id,
                    before,
                    pending,
                    tracks,
                } => {
                    if let Some(next) = move_gasket_support(point, &support_id, &before, &tracks) {
                        valid_point = true;
                        *gesture.borrow_mut() = Some(CaseGestureDraft::Gasket {
                            support_id,
                            before,
                            pending: next.clone(),
                            tracks,
                        });
                        preview.set(Some(gasket_handles(handles, &next, None)));
                        message.set(Some(
                            "Release to place; fit is checked after placement.".into(),
                        ));
                    } else {
                        preview.set(Some(gasket_handles(handles, &pending, Some(&support_id))));
                        message.set(Some("No valid gasket perimeter position here.".into()));
                    }
                }
                CaseGestureDraft::Mount {
                    target,
                    mount_id,
                    before,
                    pending,
                    constraints,
                } => {
                    if let Some(next) = move_case_mount(point, &mount_id, &before, &constraints) {
                        valid_point = true;
                        *gesture.borrow_mut() = Some(CaseGestureDraft::Mount {
                            target: target.clone(),
                            mount_id,
                            before,
                            pending: next.clone(),
                            constraints,
                        });
                        preview.set(Some(mount_handles(handles, &target, &next, None)));
                        message.set(Some("Release to save mount position.".into()));
                    } else {
                        preview.set(Some(mount_handles(
                            handles,
                            &target,
                            &pending,
                            Some(&mount_id),
                        )));
                        message.set(Some(
                            "Placement blocked by the case edge, openings, or another mount."
                                .into(),
                        ));
                    }
                }
            }
            if valid_point
                && live_preview
                && let Some(draft) = gesture.borrow().clone()
                && let Some(document) = case_gesture_preview_document(scene, &draft)
                && let Some(owner) = runtime.update_case_gesture_preview(
                    scene.scope.clone(),
                    scene.token,
                    scene.snapshot.document.revision,
                    document,
                )
            {
                *draft_preview_owner.borrow_mut() = Some(owner);
            }
        }
        HandleGesturePhase::End => {
            if let Some(owner) = draft_preview_owner.borrow_mut().take() {
                runtime.cancel_case_gesture_preview(&owner);
            }
            if !identity_current {
                *gesture.borrow_mut() = None;
                preview.set(None);
                message.set(None);
                return;
            };
            let Some(current) = gesture.borrow().clone() else {
                return;
            };
            let current = if let Some(point) = point.map(world_xy) {
                match current {
                    CaseGestureDraft::Gasket {
                        support_id,
                        before,
                        tracks,
                        ..
                    } => move_gasket_support(point, &support_id, &before, &tracks).map(|pending| {
                        CaseGestureDraft::Gasket {
                            support_id,
                            before,
                            pending,
                            tracks,
                        }
                    }),
                    CaseGestureDraft::Mount {
                        target,
                        mount_id,
                        before,
                        constraints,
                        ..
                    } => move_case_mount(point, &mount_id, &before, &constraints).map(|pending| {
                        CaseGestureDraft::Mount {
                            target,
                            mount_id,
                            before,
                            pending,
                            constraints,
                        }
                    }),
                }
            } else {
                Some(current)
            };
            let Some(current) = current else {
                *gesture.borrow_mut() = None;
                preview.set(None);
                message.set(Some("Blocked move was not saved.".into()));
                return;
            };
            let mut body_edit = None;
            let patch = match current {
                CaseGestureDraft::Gasket {
                    support_id,
                    before,
                    pending,
                    ..
                } => {
                    if supports_equal_positions(&before, &pending) {
                        None
                    } else {
                        let anchors = pending
                            .iter()
                            .filter(|support| {
                                before
                                    .iter()
                                    .find(|old| old.id == support.id)
                                    .is_none_or(|old| (old.anchor - support.anchor).abs() > 1e-7)
                            })
                            .map(gasket_anchor)
                            .collect::<Vec<_>>();
                        (!anchors.is_empty()).then_some(
                            super::mechanical_settings::MechanicalSettingsPatch::SetGasketSupportPlacement {
                                support_id,
                                anchors,
                            },
                        )
                    }
                }
                CaseGestureDraft::Mount {
                    target,
                    mount_id,
                    before,
                    pending,
                    ..
                } => pending
                    .iter()
                    .find(|mount| mount.id == mount_id)
                    .zip(before.iter().find(|mount| mount.id == mount_id))
                    .filter(|(next, old)| {
                        (next.at.x - old.at.x).abs() > 1e-7 || (next.at.y - old.at.y).abs() > 1e-7
                    })
                    .and_then(|(mount, _)| match target {
                        CaseMountTarget::Mechanical(collection) => Some(
                            super::mechanical_settings::MechanicalSettingsPatch::SetMountPosition {
                                collection,
                                mount_id,
                                at: mount.at,
                            },
                        ),
                        CaseMountTarget::Authored { body_id } => {
                            body_edit = Some(CaseBodyEdit::SetMountPosition {
                                body_id,
                                mount_id,
                                at: mount.at,
                            });
                            None
                        }
                    }),
            };
            if let Some(patch) = patch {
                let field_id = patch.field_id();
                if settings.is_some_and(|settings| submit_settings_patch(settings, patch)) {
                    feedback_field.set(Some(field_id));
                    message.set(Some("Saving position…".into()));
                } else {
                    message.set(Some("Position could not be submitted.".into()));
                }
            } else if let Some(edit) = body_edit {
                if let Some(dispatch) = selection.body_edit_portal.dispatch.read().clone() {
                    dispatch(
                        edit,
                        identity.scope.clone(),
                        identity.snapshot_token,
                        scene.snapshot.document.revision,
                    );
                    message.set(Some("Saving body mount position…".into()));
                } else {
                    message.set(Some("The Case body edit owner is unavailable.".into()));
                }
            } else {
                message.set(None);
            }
            *gesture.borrow_mut() = None;
            preview.set(None);
        }
        HandleGesturePhase::Cancel => {
            if let Some(owner) = draft_preview_owner.borrow_mut().take() {
                runtime.cancel_case_gesture_preview(&owner);
            }
            *gesture.borrow_mut() = None;
            preview.set(None);
            feedback_field.set(None);
            message.set(Some("Move cancelled.".into()));
        }
    }
}

fn direct_gesture_message(
    field_id: Option<String>,
    transient: Option<String>,
    settings: Option<&super::mechanical_settings::MechanicalSettingsProps>,
) -> Option<String> {
    let Some(field_id) = field_id else {
        return transient;
    };
    let feedback = settings?
        .feedback
        .iter()
        .rev()
        .find(|entry| entry.field_id == field_id)?;
    match feedback.state {
        super::mechanical_settings::MechanicalSettingsFeedbackState::Pending => {
            Some("Saving position…".into())
        }
        super::mechanical_settings::MechanicalSettingsFeedbackState::Landed => None,
        super::mechanical_settings::MechanicalSettingsFeedbackState::Failed => Some(
            feedback
                .message
                .clone()
                .unwrap_or_else(|| "Position could not be saved.".into()),
        ),
    }
}

fn submit_settings_patch(
    settings: &super::mechanical_settings::MechanicalSettingsProps,
    patch: super::mechanical_settings::MechanicalSettingsPatch,
) -> bool {
    if !settings.editable {
        return false;
    }
    let mut sequence = settings.request_sequence;
    let Some(request_id) = sequence().checked_add(1) else {
        return false;
    };
    sequence.set(request_id);
    let field_id = patch.field_id();
    settings
        .on_request
        .call(super::mechanical_settings::MechanicalSettingsRequest {
            identity: settings.identity.clone(),
            request_id,
            field_id,
            patch,
        });
    true
}

fn world_xy(point: [f32; 3]) -> Vec2 {
    Vec2 {
        x: f64::from(point[0]),
        y: f64::from(point[1]),
    }
}

fn gasket_anchor(support: &MechanicalGasketSupport) -> MechanicalGasketAnchor {
    MechanicalGasketAnchor {
        id: support.id.clone(),
        region_id: support.region_id.clone(),
        outline_key: support.outline_key.clone(),
        anchor: support.anchor,
        length: Some(support.length),
        width: Some(support.width),
        placement: Some(GasketPlacement::User),
        unlinked: support.unlinked,
    }
}

fn case_gesture_preview_document(scene: &CadScene, draft: &CaseGestureDraft) -> Option<ProjectDoc> {
    let mut effective = captured_case_document(&scene.snapshot, &scene.scope).ok()?;
    let mut document = (*scene.snapshot.document).clone();
    let mut mechanical_draft = None;
    match draft {
        CaseGestureDraft::Gasket {
            before, pending, ..
        } => {
            let mut configuration = effective.mechanical.take()?;
            let layout = configuration.gasket_layout.as_mut()?;
            for support in pending.iter().filter(|support| {
                before
                    .iter()
                    .find(|old| old.id == support.id)
                    .is_none_or(|old| (old.anchor - support.anchor).abs() > 1e-7)
            }) {
                let anchor = gasket_anchor(support);
                if let Some(existing) = layout
                    .supports
                    .iter_mut()
                    .find(|existing| existing.id == support.id)
                {
                    *existing = anchor;
                } else {
                    layout.supports.push(anchor);
                }
            }
            mechanical_draft = Some(configuration);
        }
        CaseGestureDraft::Mount {
            target: CaseMountTarget::Mechanical(collection),
            pending,
            ..
        } => {
            let mut configuration = effective.mechanical.take()?;
            match collection {
                super::mechanical_settings::MechanicalMountCollection::Suspension => {
                    configuration.mounts = pending.clone();
                }
                super::mechanical_settings::MechanicalMountCollection::Closure => {
                    configuration.closure_mounts = Some(pending.clone());
                }
            }
            mechanical_draft = Some(configuration);
        }
        CaseGestureDraft::Mount {
            target: CaseMountTarget::Authored { body_id },
            pending,
            ..
        } => {
            let body = document
                .case_bodies
                .iter_mut()
                .find(|body| body.id == *body_id && body.board_id == scene.scope.board_id)?;
            body.mounts = Some(pending.clone());
        }
    }
    if let Some(configuration) = mechanical_draft {
        if let Some(instance_id) = scene.scope.instance_id.as_deref() {
            let instance = document
                .hardware
                .as_mut()?
                .instances
                .iter_mut()
                .find(|instance| instance.id == instance_id)?;
            // The effective configuration includes shared defaults. The
            // existing projection reapplies those defaults and retains the
            // instance-local mount and gasket placement fields.
            instance.mechanical = Some(configuration);
        } else {
            document.mechanical = Some(configuration);
        }
    }
    Some(document)
}

fn supports_equal_positions(a: &[MechanicalGasketSupport], b: &[MechanicalGasketSupport]) -> bool {
    a.len() == b.len()
        && a.iter().all(|support| {
            b.iter()
                .find(|candidate| candidate.id == support.id)
                .is_some_and(|candidate| (support.anchor - candidate.anchor).abs() <= 1e-7)
        })
}

fn gasket_handles(
    original_handles: &[ViewerHandle],
    supports: &[MechanicalGasketSupport],
    invalid: Option<&str>,
) -> Vec<ViewerHandle> {
    let mut result = original_handles.to_vec();
    for support in supports {
        let id = format!("gasket-handle:{}", support.id);
        if let Some(handle) = result.iter_mut().find(|handle| handle.id == id) {
            handle.x = support.at.x as f32;
            handle.y = support.at.y as f32;
            handle.tangent_x = support.tangent.x as f32;
            handle.tangent_y = support.tangent.y as f32;
            handle.normal_x = support.normal.x as f32;
            handle.normal_y = support.normal.y as f32;
            handle.invalid = invalid == Some(support.id.as_str()) || support.fit_error.is_some();
        }
    }
    result
}

fn mount_handles(
    original_handles: &[ViewerHandle],
    target: &CaseMountTarget,
    mounts: &[Mount],
    invalid: Option<&str>,
) -> Vec<ViewerHandle> {
    let mut result = original_handles.to_vec();
    for handle in &mut result {
        let (handle_target, mount_id) = match &handle.target {
            ViewerHandleTarget::Mount {
                collection,
                mount_id,
            } => (CaseMountTarget::Mechanical(*collection), mount_id),
            ViewerHandleTarget::AuthoredMount { body_id, mount_id } => (
                CaseMountTarget::Authored {
                    body_id: body_id.clone(),
                },
                mount_id,
            ),
            ViewerHandleTarget::Gasket { .. } => continue,
        };
        if !same_mount_target(&handle_target, target) {
            continue;
        }
        if let Some(mount) = mounts.iter().find(|mount| mount.id == *mount_id) {
            handle.x = mount.at.x as f32;
            handle.y = mount.at.y as f32;
            handle.invalid = invalid == Some(mount.id.as_str());
        }
    }
    result
}

fn same_mount_target(left: &CaseMountTarget, right: &CaseMountTarget) -> bool {
    match (left, right) {
        (CaseMountTarget::Mechanical(left), CaseMountTarget::Mechanical(right)) => left == right,
        (
            CaseMountTarget::Authored { body_id: left },
            CaseMountTarget::Authored { body_id: right },
        ) => left == right,
        _ => false,
    }
}

fn move_gasket_support(
    point: Vec2,
    id: &str,
    supports: &[MechanicalGasketSupport],
    tracks: &[MechanicalGasketTrack],
) -> Option<Vec<MechanicalGasketSupport>> {
    let original = supports.iter().find(|support| support.id == id)?;
    let moved = project_gasket(point, original, tracks)?;
    let mut replacements = BTreeMap::from([(id.to_owned(), moved)]);
    if let Some(pair_id) = original.pair_id.as_ref()
        && let Some(axis) = original.mirror_axis
        && !original.unlinked
    {
        let pair = supports.iter().find(|support| support.id == *pair_id)?;
        if !pair.unlinked {
            let moved = replacements.get(id)?;
            let reflected = project_gasket(
                Vec2 {
                    x: 2.0 * axis - moved.at.x,
                    y: moved.at.y,
                },
                pair,
                tracks,
            )?;
            if (reflected.at.x - (2.0 * axis - moved.at.x)).hypot(reflected.at.y - moved.at.y)
                > 0.001
            {
                return None;
            }
            replacements.insert(pair.id.clone(), reflected);
        }
    }
    Some(
        supports
            .iter()
            .map(|support| replacements.get(&support.id).unwrap_or(support).clone())
            .collect(),
    )
}

fn project_gasket(
    point: Vec2,
    support: &MechanicalGasketSupport,
    tracks: &[MechanicalGasketTrack],
) -> Option<MechanicalGasketSupport> {
    let mut best = None;
    let mut best_distance = f64::INFINITY;
    for track in tracks
        .iter()
        .filter(|track| track.region_id == support.region_id)
    {
        let dx = track.end.x - track.start.x;
        let dy = track.end.y - track.start.y;
        let length_squared = dx * dx + dy * dy;
        if length_squared <= f64::EPSILON {
            continue;
        }
        let length = length_squared.sqrt();
        let t = (((point.x - track.start.x) * dx + (point.y - track.start.y) * dy)
            / length_squared)
            .clamp(0.0, 1.0);
        let at = Vec2 {
            x: track.start.x + t * dx,
            y: track.start.y + t * dy,
        };
        let distance = (point.x - at.x).hypot(point.y - at.y);
        if distance >= best_distance {
            continue;
        }
        best_distance = distance;
        let mut next = support.clone();
        next.at = at;
        next.anchor = track.start_anchor + t * (track.end_anchor - track.start_anchor);
        next.tangent = Vec2 {
            x: dx / length,
            y: dy / length,
        };
        next.normal = Vec2 {
            x: dy / length,
            y: -dx / length,
        };
        best = Some(next);
    }
    best
}

fn mount_constraints(scene: &CadScene, body_id: &str) -> Option<MountMoveConstraints> {
    let body = scene
        .prepared
        .bodies
        .iter()
        .find(|body| body.body.id == body_id)?;
    let base = body.body.z.unwrap_or(0.0);
    let top = base
        + body.body.thickness
        + if body.body.kind == CaseKind::Plate {
            0.0
        } else {
            body.body.wall_height.unwrap_or(0.0)
        };
    let mut holes = body
        .regions
        .iter()
        .flat_map(|region| region.holes.iter().cloned())
        .collect::<Vec<_>>();
    holes.extend(
        body.body
            .openings
            .as_deref()
            .unwrap_or_default()
            .iter()
            .filter(|opening| opening.z < top && opening.z + opening.height > base)
            .map(|opening| opening.points.clone()),
    );
    let outer = body
        .regions
        .iter()
        .map(|region| region.outer.clone())
        .filter(|outer| outer.len() >= 3)
        .collect::<Vec<_>>();
    if outer.is_empty() {
        return None;
    }
    Some(MountMoveConstraints { outer, holes })
}

fn move_case_mount(
    point: Vec2,
    id: &str,
    mounts: &[Mount],
    constraints: &MountMoveConstraints,
) -> Option<Vec<Mount>> {
    if !point.x.is_finite() || !point.y.is_finite() {
        return None;
    }
    let original = mounts.iter().find(|mount| mount.id == id)?;
    let radius = mount_radius(original);
    let margin = radius + 0.5;
    if !radius.is_finite() || radius <= 0.0 {
        return None;
    }
    if !constraints
        .outer
        .iter()
        .any(|outer| inside_polygon(point, outer) && edge_distance(point, outer) >= margin)
    {
        return None;
    }
    if constraints
        .holes
        .iter()
        .any(|hole| inside_polygon(point, hole) || edge_distance(point, hole) < margin)
    {
        return None;
    }
    if mounts.iter().any(|other| {
        if other.id == id {
            return false;
        }
        let other_radius = mount_radius(other);
        !other_radius.is_finite()
            || other_radius <= 0.0
            || (point.x - other.at.x).hypot(point.y - other.at.y) < radius + other_radius + 0.5
    }) {
        return None;
    }
    Some(
        mounts
            .iter()
            .map(|mount| {
                if mount.id == id {
                    let mut moved = mount.clone();
                    moved.at = point;
                    moved
                } else {
                    mount.clone()
                }
            })
            .collect(),
    )
}

fn mount_radius(mount: &Mount) -> f64 {
    if mount.kind == MountKind::Boss {
        mount.boss_diameter.unwrap_or(mount.hole_diameter) / 2.0
    } else {
        mount.hole_diameter / 2.0
    }
}

fn inside_polygon(point: Vec2, polygon: &[Vec2]) -> bool {
    let mut hit = false;
    if polygon.len() < 3 {
        return false;
    }
    let mut j = polygon.len() - 1;
    for i in 0..polygon.len() {
        let a = &polygon[i];
        let b = &polygon[j];
        if (a.y > point.y) != (b.y > point.y)
            && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x
        {
            hit = !hit;
        }
        j = i;
    }
    hit
}

fn edge_distance(point: Vec2, polygon: &[Vec2]) -> f64 {
    let mut nearest = f64::INFINITY;
    for index in 0..polygon.len() {
        let a = &polygon[index];
        let b = &polygon[(index + 1) % polygon.len()];
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let length = dx * dx + dy * dy;
        let t = if length > 0.0 {
            (((point.x - a.x) * dx + (point.y - a.y) * dy) / length).clamp(0.0, 1.0)
        } else {
            0.0
        };
        nearest = nearest.min((point.x - a.x - t * dx).hypot(point.y - a.y - t * dy));
    }
    nearest
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
mod keycap_consumer_tests {
    use super::*;
    use crate::presentation::shared_viewer::SharedViewerProjectionProbe;
    use crate::runtime::{KeycapsPreviewInput, project_name_test_support as support};
    use boardstudio_application::{Event, OperationId, Session};
    use boardstudio_core::{
        CoreEngine,
        model::{CoreReply, CoreRequest, PreparedCaseAssemblyIR},
    };
    use boardstudio_web_host::cad_jobs::{CadBodyMesh, CadResult};
    use futures_channel::oneshot;
    use gloo_timers::future::TimeoutFuture;
    use js_sys::{Array, Float32Array, Reflect};
    use sha2::{Digest, Sha256};
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    const ROUTED_SOURCE: &[u8] = b"(kicad_pcb (version 20240108))";

    fn document() -> ProjectDoc {
        let mut value =
            serde_json::to_value(ProjectDoc::empty("case-keycaps-mounted", "Case caps")).unwrap();
        value["definitions"] =
            serde_json::json!([{"id":"mx","name":"MX","kind":"switch","courtyard":[],"pads":[]}]);
        value["parts"] = serde_json::json!([{"id":"key","definitionId":"mx","reference":"SW1","pose":{"at":{"x":20,"y":30},"rotation":15},"side":"front"}]);
        value["boards"] = serde_json::json!([{"id":"board","name":"Board","partIds":["key"],"outlineIds":[],"netIds":[],"thickness":1.6}]);
        value["keycaps"] = serde_json::json!({"boards":{},"matrices":{},"keys":{"key":{"profile":"sa","mount":"mx","units":{"x":7,"y":1},"color":"#22aa44","legend":""}}});
        value["assets"] = serde_json::json!([{"id":"routed","name":"board.kicad_pcb","mediaType":"application/vnd.kicad.pcb","sha256":Sha256::digest(ROUTED_SOURCE).iter().map(|byte| format!("{byte:02x}")).collect::<String>()}]);
        value["boardReferences"] = serde_json::json!([{"id":"ref","boardId":"board","assetId":"routed","enabled":true,"pose":{"at":{"x":4.2,"y":-3.1},"rotation":15},"elevation":2.5,"modelAssets":{}}]);
        value["hardware"] = serde_json::json!({"topology":"unibody","transport":"none","boards":[],"instances":[
            {"id":"primary","name":"Primary","boardId":"board","half":"left","role":"central","flipped":false,"controllerPartId":null,"mechanical":null,"constructionLinked":false},
            {"id":"flipped","name":"Flipped","boardId":"board","half":"right","role":"peripheral","flipped":true,"controllerPartId":null,"mechanical":null,"constructionLinked":false}
        ]});
        serde_json::from_value(value).unwrap()
    }

    fn mounted_host() -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let version = use_signal(|| 0_u64);
        use_context_provider(|| version);
        use_hook(|| {
            runtime.subscribe(Rc::new(move || {
                let mut version = version;
                version += 1;
            }))
        });
        let _ = version();
        super::super::use_empty_test_instance_selection();
        crate::test_contexts::use_case_viewer_test_contexts();
        super::super::use_test_case_generation_state();
        let mut mounted = use_signal(|| true);
        let mut cad = use_signal(|| false);
        let mut instance = use_context::<InstanceSelection>();
        let other = runtime.clone();
        let restore = runtime.clone();
        let use_cad = runtime.clone();
        rsx! {
            button { id: "case-caps-flipped", onclick: move |_| {
                let scope = other.scope().unwrap();
                instance.reconcile(scope.session_epoch, scope.document_id, "flipped".into());
                other.submit(Event::Navigate { operation_id: OperationId(202), board_id: "board".into(), instance_id: Some("flipped".into()) });
                other.install_case_pcb_preview_test();
                cad.set(false);
            }, "Flipped physical instance" }
            button { id: "case-caps-primary", onclick: move |_| {
                let scope = restore.scope().unwrap();
                instance.reconcile(scope.session_epoch, scope.document_id, "primary".into());
                restore.submit(Event::Navigate { operation_id: OperationId(203), board_id: "board".into(), instance_id: Some("primary".into()) });
                restore.install_case_pcb_preview_test();
                cad.set(false);
            }, "Original physical instance" }
            button { id: "case-caps-cad", onclick: move |_| {
                let accepted = use_cad.model().accepted.unwrap();
                use_cad.set_cad_scene_test(Some(Rc::new(CadScene {
                    scope: use_cad.scope().unwrap(), token: accepted.token,
                    result: CadResult { revision: accepted.document.revision, ..Default::default() },
                    prepared: PreparedCaseAssemblyIR { revision: accepted.document.revision, bodies: vec![] },
                    snapshot: accepted, physical_fingerprint: None, mechanical: None, exact: true, contours: vec![],
                })));
                cad.set(true);
            }, "Case geometry consumer" }
            button { id: "case-caps-unmount", onclick: move |_| mounted.set(false), "Leave Case" }
            if mounted() {
                if cad() {
                    CaseViewer { scene: runtime.cad_scene().unwrap(), preview: runtime.native_case_preview(), model_rows: None, mechanical_settings: None }
                } else if let Some(preview) = runtime.native_case_preview() {
                    CasePreviewViewer { preview, model_rows: None }
                }
            }
        }
    }

    fn meshes(input: &KeycapsPreviewInput) -> Vec<CadBodyMesh> {
        input
            .specs
            .iter()
            .map(|spec| CadBodyMesh {
                id: format!("keycap:{}", spec.id),
                name: spec.reference.clone(),
                positions: vec![
                    spec.pose.at.x as f32,
                    spec.pose.at.y as f32,
                    spec.z as f32,
                    21.0,
                    30.0,
                    10.0,
                    20.0,
                    31.0,
                    10.0,
                ],
                normals: vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
            })
            .collect()
    }

    fn cap(packet: &JsValue) -> Option<JsValue> {
        let bodies = Reflect::get(packet, &"bodies".into()).ok()?;
        if !Array::is_array(&bodies) {
            return None;
        }
        Array::from(&bodies).iter().find(|body| {
            Reflect::get(body, &"id".into())
                .ok()
                .and_then(|id| id.as_string())
                .as_deref()
                == Some("keycap:key")
        })
    }

    fn click(root: &web_sys::Element, selector: &str) {
        root.query_selector(selector)
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
    }

    #[wasm_bindgen_test]
    async fn mounted_case_keycaps_reach_both_packets_and_reject_late_physical_owner() {
        let runtime = support::new_runtime();
        support::install(&runtime, Session::new(), CoreEngine::new());
        runtime
            .store
            .save_asset(boardstudio_web_host::host::AssetBytes {
                sha256: Sha256::digest(ROUTED_SOURCE)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>(),
                bytes: ROUTED_SOURCE.to_vec(),
            })
            .await
            .expect("required reference bytes exist before actual Open/Persist");
        runtime.submit(Event::Open {
            operation_id: OperationId(200),
            document: document(),
        });
        support::run_pending(&runtime).await;
        runtime.submit(Event::Navigate {
            operation_id: OperationId(201),
            board_id: "board".into(),
            instance_id: Some("primary".into()),
        });
        support::run_pending(&runtime).await;
        let accepted = runtime.model().accepted.unwrap_or_else(|| {
            panic!(
                "fixture Open/Persist did not accept its document: {:?}",
                runtime.model()
            )
        });
        let physical = captured_case_document(&accepted, &runtime.scope().unwrap()).unwrap();
        let CoreReply::KeycapsResolved { result, .. } =
            CoreEngine::new().handle(CoreRequest::ResolveKeycaps {
                id: "fixture-proof".into(),
                document: physical,
                board_id: "board".into(),
                cases: None,
            })
        else {
            panic!("real Core resolves accepted physical keycaps");
        };
        assert_eq!(result.specs.len(), 1);
        assert!(
            result
                .findings
                .iter()
                .all(|finding| finding.severity != boardstudio_core::model::Severity::Error)
        );
        assert!(
            (result.specs[0].size.x - 132.5).abs() < 1e-9,
            "accepted 7u override resolves through Core"
        );
        runtime.install_case_pcb_preview_test();
        assert!(
            runtime
                .native_case_preview()
                .unwrap()
                .board_reference
                .is_some()
        );
        let requests = Rc::new(RefCell::new(Vec::<KeycapsPreviewInput>::new()));
        let (release, held) = oneshot::channel::<()>();
        let held = Rc::new(RefCell::new(Some(held)));
        runtime.set_keycaps_preview_executor_test(Rc::new({
            let requests = requests.clone();
            move |input| {
                requests.borrow_mut().push(input.clone());
                let wait = if input.scope.instance_id.as_deref() == Some("flipped") {
                    held.borrow_mut().take()
                } else {
                    None
                };
                Box::pin(async move {
                    if let Some(wait) = wait {
                        wait.await
                            .map_err(|_| "controlled CAD reply dropped".to_owned())?;
                    }
                    Ok(meshes(&input))
                })
            }
        }));
        let packets = Rc::new(RefCell::new(Vec::<(ViewerIdentity, JsValue)>::new()));
        let probe = SharedViewerProjectionProbe(Rc::new({
            let packets = packets.clone();
            move |id, packet| packets.borrow_mut().push((id, packet))
        }));
        let dom_document = web_sys::window().unwrap().document().unwrap();
        let root = dom_document.create_element("div").unwrap();
        root.set_id("mounted-case-keycap-consumer");
        dom_document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(mounted_host);
        dom.provide_root_context(runtime.clone());
        dom.provide_root_context(probe);
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        for _ in 0..200 {
            if packets
                .borrow()
                .last()
                .is_some_and(|(_, packet)| cap(packet).is_some())
            {
                break;
            }
            TimeoutFuture::new(10).await;
        }
        assert!(
            root.query_selector("canvas").unwrap().is_some(),
            "actual Case viewer mounted"
        );
        let body = packets.borrow().last().and_then(|(_, packet)| cap(packet));
        assert!(
            body.is_some(),
            "mounted Case renderer packet must contain generated authored keycaps, distinct from PCB model rows"
        );
        let body = body.unwrap();
        assert_eq!(
            Reflect::get(&body, &"color".into())
                .unwrap()
                .as_string()
                .as_deref(),
            Some("#22aa44")
        );
        let mesh = Reflect::get(&body, &"mesh".into()).unwrap();
        assert_eq!(
            Float32Array::new(&Reflect::get(&mesh, &"positions".into()).unwrap()).to_vec()[0],
            20.0
        );
        assert_eq!(
            requests.borrow()[0].scope.instance_id.as_deref(),
            Some("primary")
        );

        let before_cad = requests.borrow().len();
        click(&root, "#case-caps-cad");
        for _ in 0..200 {
            if requests.borrow().len() > before_cad
                && packets
                    .borrow()
                    .last()
                    .is_some_and(|(_, packet)| cap(packet).is_some())
            {
                break;
            }
            TimeoutFuture::new(10).await;
        }
        assert!(
            requests.borrow().len() > before_cad,
            "Case geometry consumer requests its own caps"
        );
        assert!(
            packets
                .borrow()
                .last()
                .is_some_and(|(_, packet)| cap(packet).is_some()),
            "Case geometry packet retains generated caps"
        );

        click(&root, "#case-caps-flipped");
        for _ in 0..200 {
            if requests
                .borrow()
                .iter()
                .any(|input| input.scope.instance_id.as_deref() == Some("flipped"))
            {
                break;
            }
            TimeoutFuture::new(10).await;
        }
        let flipped = requests
            .borrow()
            .iter()
            .find(|input| input.scope.instance_id.as_deref() == Some("flipped"))
            .cloned()
            .expect("new physical owner reaches CAD");
        assert_eq!(
            flipped.specs[0].pose.at.x, -20.0,
            "Case resolves physical coordinates rather than canonical Layout coordinates"
        );
        assert_eq!(flipped.specs[0].pose.rotation, -15.0);
        assert!(
            packets
                .borrow()
                .last()
                .is_some_and(
                    |(id, packet)| id.scope.instance_id.as_deref() == Some("flipped")
                        && cap(packet).is_none()
                ),
            "old caps retire while the new physical owner is pending"
        );
        click(&root, "#case-caps-primary");
        for _ in 0..200 {
            if packets.borrow().last().is_some_and(|(id, packet)| {
                id.scope.instance_id.as_deref() == Some("primary") && cap(packet).is_some()
            }) {
                break;
            }
            TimeoutFuture::new(10).await;
        }
        release.send(()).unwrap();
        TimeoutFuture::new(50).await;
        let packets = packets.borrow();
        let (id, packet) = packets.last().unwrap();
        assert_eq!(id.scope.instance_id.as_deref(), Some("primary"));
        let body = cap(packet).expect("late flipped CAD must not replace current keycaps");
        let mesh = Reflect::get(&body, &"mesh".into()).unwrap();
        assert_eq!(
            Float32Array::new(&Reflect::get(&mesh, &"positions".into()).unwrap()).to_vec()[0],
            20.0
        );
        drop(packets);
        assert_eq!(runtime.model().accepted.as_ref(), Some(&accepted));
        assert_eq!(
            runtime
                .store
                .load_document(accepted.document.id.clone())
                .await
                .unwrap()
                .as_ref(),
            Some(accepted.document.as_ref()),
            "display generation does not edit or persist the accepted design"
        );
        click(&root, "#case-caps-unmount");
        TimeoutFuture::new(30).await;
        runtime.unsubscribe();
        root.remove();
        runtime
            .store
            .delete_project(accepted.document.id.clone())
            .await
            .unwrap();
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod gesture_cancellation_tests {
    use super::*;
    use crate::presentation::shared_viewer::{HandleGesturePhase, SharedViewerEscapeProbe};
    use boardstudio_application::{AcceptedSnapshot, Event, OperationId, Session};
    use boardstudio_core::{
        CoreEngine,
        model::{
            Board, CaseKind, Mount, MountKind, PreparedCaseAssemblyIR, PreparedCaseIR,
            PreparedCaseRegion, ProjectDoc, Vec2,
        },
    };
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    fn outer() -> Vec<Vec2> {
        vec![
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 100.0, y: 0.0 },
            Vec2 { x: 100.0, y: 100.0 },
            Vec2 { x: 0.0, y: 100.0 },
        ]
    }

    async fn accepted_case_fixture() -> (Rc<Runtime>, AcceptedSnapshot, Scope, Rc<CadScene>) {
        let mount = Mount {
            id: "mount-1".into(),
            at: Vec2 { x: 20.0, y: 20.0 },
            kind: MountKind::Boss,
            hole_diameter: 2.0,
            boss_diameter: Some(4.0),
            height: Some(5.0),
        };
        let body = boardstudio_core::model::CaseBody {
            features: None,
            openings: None,
            id: "bottom".into(),
            name: "Bottom".into(),
            board_id: "board-1".into(),
            kind: CaseKind::Tray,
            thickness: 1.5,
            clearance: 0.2,
            material_id: None,
            z: Some(0.0),
            wall_height: Some(20.0),
            wall_thickness: Some(3.0),
            mounts: Some(vec![mount.clone()]),
            gasket: None,
        };
        let mut document = ProjectDoc::empty("case-gesture-cancel", "Gesture cancellation");
        document.boards.push(Board {
            id: "board-1".into(),
            name: "Board 1".into(),
            outline_ids: vec!["outline-1".into()],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document
            .outline
            .push(boardstudio_core::model::OutlineFeature::Polygon {
                anchor_part_id: None,
                id: "outline-1".into(),
                points: outer(),
                operation: boardstudio_core::model::Operation::Add,
            });
        document.case_bodies.push(body.clone());
        document.hardware = Some(boardstudio_core::model::HardwareConfiguration {
            instances: ["primary", "alternate"]
                .into_iter()
                .map(|id| boardstudio_core::model::PhysicalBoardInstance {
                    id: id.into(),
                    name: id.into(),
                    board_id: "board-1".into(),
                    half: "left".into(),
                    role: "central".into(),
                    flipped: false,
                    controller_part_id: None,
                    mechanical: None,
                    construction_linked: false,
                })
                .collect(),
            ..Default::default()
        });

        let runtime = crate::runtime::project_name_test_support::new_runtime();
        crate::runtime::project_name_test_support::install(
            &runtime,
            Session::new(),
            CoreEngine::new(),
        );
        runtime.submit(Event::Open {
            operation_id: OperationId(100),
            document,
        });
        crate::runtime::project_name_test_support::run_pending(&runtime).await;
        runtime.submit(Event::Navigate {
            operation_id: OperationId(101),
            board_id: "board-1".into(),
            instance_id: Some("primary".into()),
        });
        crate::runtime::project_name_test_support::run_pending(&runtime).await;
        let accepted = runtime
            .model()
            .accepted
            .expect("runtime accepted Case fixture");
        let scope = runtime.scope().expect("runtime accepted board scope");
        let polygon = outer();
        let scene = Rc::new(CadScene {
            scope: scope.clone(),
            token: accepted.token,
            snapshot: accepted.clone(),
            result: boardstudio_web_host::cad_jobs::CadResult {
                revision: accepted.document.revision,
                ..Default::default()
            },
            prepared: PreparedCaseAssemblyIR {
                revision: accepted.document.revision,
                bodies: vec![PreparedCaseIR {
                    revision: accepted.document.revision,
                    body,
                    regions: vec![PreparedCaseRegion {
                        outer: polygon,
                        holes: vec![],
                        cavities: vec![],
                        gaskets: vec![],
                        mounts: vec![mount],
                    }],
                }],
            },
            physical_fingerprint: None,
            mechanical: None,
            exact: true,
            contours: vec![],
        });
        (runtime, accepted, scope, scene)
    }

    // Keep real Core preparation; substitute only the CAD worker's mesh output.
    fn prepare_preview_for_test(
        snapshot: AcceptedSnapshot,
        scope: Scope,
    ) -> Result<CadScene, String> {
        use boardstudio_core::model::{CaseAssemblyIR, CaseIR, CoreReply, CoreRequest};
        use boardstudio_web_host::cad_jobs::{
            CadBodyMesh, CadBounds, CadResult, captured_case_scene,
        };
        let document =
            captured_case_document(&snapshot, &scope).map_err(|error| format!("{error:?}"))?;
        let contours = captured_case_scene(&snapshot, &scope)
            .map_err(|error| format!("{error:?}"))?
            .board_contours
            .into_iter()
            .find(|entry| entry.board_id == scope.board_id)
            .ok_or("accepted Case fixture has no board contours")?
            .contours;
        let ir = CaseAssemblyIR {
            revision: document.revision,
            bodies: document
                .case_bodies
                .iter()
                .filter(|body| body.board_id == scope.board_id)
                .map(|body| CaseIR {
                    revision: document.revision,
                    body: body.clone(),
                    contours: contours.clone(),
                })
                .collect(),
        };
        let prepared = match CoreEngine::new().handle(CoreRequest::PrepareCase {
            id: "case-gesture-test-prepare".into(),
            ir,
        }) {
            CoreReply::CasePrepared { ir, .. } => ir,
            reply => {
                return Err(format!(
                    "real Core Case preparation rejected fixture: {reply:?}"
                ));
            }
        };
        let result = CadResult {
            revision: document.revision,
            bodies: prepared
                .bodies
                .iter()
                .map(|body| CadBodyMesh {
                    id: body.body.id.clone(),
                    name: body.body.name.clone(),
                    positions: vec![0.0, 0.0, 0.0, 100.0, 0.0, 0.0, 0.0, 100.0, 0.0],
                    normals: vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
                })
                .collect(),
            bounds: Some(CadBounds {
                min: [0.0, 0.0, 0.0],
                max: [100.0, 100.0, 20.0],
            }),
            ..Default::default()
        };
        Ok(CadScene {
            scope,
            token: snapshot.token,
            snapshot,
            result,
            prepared,
            physical_fingerprint: None,
            mechanical: None,
            exact: false,
            contours,
        })
    }

    fn mounted_case_viewer() -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let version = use_signal(|| 0_u64);
        use_context_provider(|| version);
        use_hook(|| {
            runtime.subscribe(Rc::new(move || {
                let mut version = version;
                version += 1;
            }));
        });
        let _ = version();
        super::super::use_empty_test_instance_selection();
        crate::test_contexts::use_case_viewer_test_contexts();
        super::super::use_test_case_generation_state();
        let mut selection = use_context::<CaseSelection>();
        use_hook(move || selection.body_edit_portal.editable.set(true));
        let original = use_context::<Rc<CadScene>>();
        let mut scene = use_signal(|| original.clone());
        let mut mounted = use_signal(|| true);
        let navigate = runtime.clone();
        let away_source = original.clone();
        let restore = runtime.clone();
        let mut instance = use_context::<InstanceSelection>();
        rsx! {
            style { {include_str!("../../../assets/m1.css")} }
            button { id: "case-gesture-scope", onclick: move |_| {
                instance.reconcile(away_source.scope.session_epoch, away_source.scope.document_id.clone(), "alternate".into());
                navigate.submit(Event::Navigate {
                    operation_id: OperationId(102), board_id: away_source.scope.board_id.clone(),
                    instance_id: Some("alternate".into()),
                });
                let next = Rc::new(CadScene {
                    scope: navigate.scope().expect("accepted alternate physical scope"),
                    token: away_source.token,
                    snapshot: away_source.snapshot.clone(),
                    result: away_source.result.clone(),
                    prepared: away_source.prepared.clone(),
                    physical_fingerprint: away_source.physical_fingerprint,
                    mechanical: away_source.mechanical.clone(),
                    exact: away_source.exact,
                    contours: away_source.contours.clone(),
                });
                navigate.set_cad_scene_test(Some(next.clone()));
                scene.set(next);
            }, "Other physical assembly" }
            button { id: "case-gesture-unmount", onclick: move |_| mounted.set(false), "Leave Case viewer" }
            button { id: "case-gesture-restore", onclick: move |_| {
                instance.reconcile(original.scope.session_epoch, original.scope.document_id.clone(), original.scope.instance_id.clone().unwrap());
                restore.submit(Event::Navigate {
                    operation_id: OperationId(103), board_id: original.scope.board_id.clone(),
                    instance_id: original.scope.instance_id.clone(),
                });
                restore.set_cad_scene_test(Some(original.clone()));
                scene.set(original.clone());
                mounted.set(true);
            }, "Return to Case viewer" }
            if mounted() {
                CaseViewer {
                    scene: scene(),
                    preview: None,
                    model_rows: None,
                    mechanical_settings: None,
                }
            }
        }
    }

    async fn wait_for_provisional_scene(runtime: &Runtime, source: &CadScene) -> Rc<CadScene> {
        for _ in 0..300 {
            if let Some(scene) = runtime.case_gesture_preview_scene(source) {
                return scene;
            }
            if runtime
                .case_gesture_preview_message()
                .as_deref()
                .is_some_and(|message| message.starts_with("Case preview update failed:"))
            {
                panic!(
                    "real provisional Case preview failed: {:?}",
                    runtime.case_gesture_preview_message()
                );
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        panic!("real provisional Case preview did not publish within the bounded wait");
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Retirement {
        Escape,
        ScopeChange,
        Unmount,
    }

    fn click(root: &web_sys::Element, selector: &str) {
        use wasm_bindgen::JsCast;
        root.query_selector(selector)
            .unwrap()
            .expect("mounted retirement control")
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
    }

    async fn assert_provisional_retirement(retirement: Retirement) {
        let (runtime, accepted, _scope, scene) = accepted_case_fixture().await;
        runtime.set_cad_scene_test(Some(scene.clone()));
        runtime.set_case_gesture_preview_executor_test(Rc::new(prepare_preview_for_test));
        let saved_before = runtime
            .store
            .load_document(accepted.document.id.clone())
            .await
            .expect("read actual persisted Case fixture")
            .expect("Case Open persisted its accepted document");
        assert_eq!(&saved_before, accepted.document.as_ref());
        let before_model = runtime.model();
        assert_eq!(before_model.accepted.as_ref(), Some(&accepted));

        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("case-gesture-retirement-test");
        document.body().unwrap().append_child(&root).unwrap();
        let gesture_probe = CaseViewerGestureProbe::default();
        let escape_probe = SharedViewerEscapeProbe::default();
        let dom = VirtualDom::new(mounted_case_viewer);
        dom.provide_root_context(runtime.clone());
        dom.provide_root_context(scene.clone());
        dom.provide_root_context(gesture_probe.clone());
        dom.provide_root_context(escape_probe.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );

        let canvas = {
            let mut mounted = None;
            for _ in 0..100 {
                mounted = root.query_selector("canvas").unwrap();
                if mounted.is_some() {
                    break;
                }
                gloo_timers::future::TimeoutFuture::new(10).await;
            }
            mounted.expect("mounted CaseViewer exposes its real canvas")
        };
        let mount_handle = "case-mount:bottom/mount-1";
        gesture_probe.emit(HandleGesturePhase::Start, mount_handle, None);
        gesture_probe.emit(
            HandleGesturePhase::Move,
            mount_handle,
            Some([30.0, 30.0, 0.8]),
        );
        assert_eq!(runtime.model().accepted.as_ref(), Some(&accepted));
        assert_eq!(
            runtime.model().accepted.as_ref().unwrap().document.revision,
            accepted.document.revision
        );
        let provisional = wait_for_provisional_scene(&runtime, &scene).await;
        assert_eq!(provisional.token, accepted.token);
        assert_eq!(
            provisional.snapshot.document.revision,
            accepted.document.revision
        );
        assert!(!provisional.exact);
        assert_eq!(
            provisional.snapshot.document.case_bodies[0]
                .mounts
                .as_ref()
                .unwrap()[0]
                .at,
            Vec2 { x: 30.0, y: 30.0 }
        );
        assert_eq!(runtime.model().accepted.as_ref(), Some(&accepted));
        for _ in 0..100 {
            if root
                .text_content()
                .unwrap_or_default()
                .contains("Release to save mount position.")
            {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            root.text_content()
                .unwrap_or_default()
                .contains("Release to save mount position.")
        );

        assert!(
            runtime.case_gesture_preview_active_test(),
            "actual Runtime owner is active before retirement"
        );
        match retirement {
            Retirement::Escape => {
                escape_probe.arm_handle_capture(mount_handle);
                let init = web_sys::KeyboardEventInit::new();
                init.set_key("Escape");
                init.set_bubbles(true);
                init.set_cancelable(true);
                let escape =
                    web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
                        .unwrap();
                canvas.dispatch_event(&escape).unwrap();
                assert!(
                    escape.default_prevented(),
                    "Escape cancels the mounted handle capture"
                );
            }
            Retirement::ScopeChange => click(&root, "#case-gesture-scope"),
            Retirement::Unmount => click(&root, "#case-gesture-unmount"),
        }
        for _ in 0..100 {
            let text = root.text_content().unwrap_or_default();
            let view_settled = match retirement {
                Retirement::Escape => text.contains("Move cancelled."),
                Retirement::ScopeChange => {
                    runtime.scope().unwrap().instance_id.as_deref() == Some("alternate")
                        && !text.contains("Release to save mount position.")
                }
                Retirement::Unmount => root.query_selector("canvas").unwrap().is_none(),
            };
            if view_settled && !runtime.case_gesture_preview_active_test() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            !runtime.case_gesture_preview_active_test(),
            "retirement clears the actual owner, not merely a stale filtered getter"
        );
        assert!(runtime.case_gesture_preview_scene(&scene).is_none());
        assert!(runtime.case_gesture_preview_message().is_none());
        match retirement {
            Retirement::Escape => assert!(
                root.text_content()
                    .unwrap_or_default()
                    .contains("Move cancelled.")
            ),
            Retirement::ScopeChange => {
                assert_eq!(
                    runtime.scope().unwrap().instance_id.as_deref(),
                    Some("alternate")
                );
                assert!(
                    !root
                        .text_content()
                        .unwrap_or_default()
                        .contains("Release to save mount position.")
                );
            }
            Retirement::Unmount => assert!(root.query_selector("canvas").unwrap().is_none()),
        }
        assert_eq!(runtime.model().accepted.as_ref(), Some(&accepted));
        if retirement != Retirement::Escape {
            click(&root, "#case-gesture-restore");
            for _ in 0..100 {
                if runtime.scope().as_ref() == Some(&scene.scope)
                    && root.query_selector("canvas").unwrap().is_some()
                    && !root
                        .text_content()
                        .unwrap_or_default()
                        .contains("Release to save mount position.")
                {
                    break;
                }
                gloo_timers::future::TimeoutFuture::new(10).await;
            }
            assert_eq!(runtime.scope().as_ref(), Some(&scene.scope));
            assert!(root.query_selector("canvas").unwrap().is_some());
            assert!(
                !root
                    .text_content()
                    .unwrap_or_default()
                    .contains("Release to save mount position.")
            );
        }
        assert_eq!(
            runtime.model(),
            before_model,
            "retirement and return do not change accepted state, durability, or history-facing revision"
        );
        assert_eq!(runtime.model().accepted.as_ref(), Some(&accepted));

        // A Move after retirement/return cannot resume the discarded CaseViewer draft.
        gesture_probe.emit(
            HandleGesturePhase::Move,
            mount_handle,
            Some([35.0, 35.0, 0.8]),
        );
        assert!(runtime.case_gesture_preview_scene(&scene).is_none());
        assert!(runtime.case_gesture_preview_message().is_none());
        assert_eq!(runtime.model().accepted.as_ref(), Some(&accepted));
        assert_eq!(
            runtime
                .store
                .load_document(accepted.document.id.clone())
                .await
                .unwrap()
                .as_ref(),
            Some(&saved_before),
            "retirement does not persist a provisional document"
        );
        runtime.submit(Event::Undo {
            operation_id: OperationId(104),
        });
        crate::runtime::project_name_test_support::run_pending(&runtime).await;
        assert_eq!(
            runtime.model().accepted.as_ref().unwrap().document,
            accepted.document,
            "retirement does not add an undo entry to the freshly opened Session"
        );
        click(&root, "#case-gesture-unmount");
        for _ in 0..100 {
            if root.query_selector("canvas").unwrap().is_none() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(root.query_selector("canvas").unwrap().is_none());
        runtime.unsubscribe();
        root.remove();
        runtime
            .store
            .delete_project(accepted.document.id.clone())
            .await
            .unwrap();
    }

    #[wasm_bindgen_test]
    async fn mounted_escape_cancels_provisional_case_gesture_without_session_edit() {
        assert_provisional_retirement(Retirement::Escape).await;
    }

    #[wasm_bindgen_test]
    async fn mounted_scope_change_retires_provisional_case_gesture_without_session_edit() {
        assert_provisional_retirement(Retirement::ScopeChange).await;
    }

    #[wasm_bindgen_test]
    async fn mounted_unmount_retires_provisional_case_gesture_without_session_edit() {
        assert_provisional_retirement(Retirement::Unmount).await;
    }
}

fn persist_display(key: &str, display: &CaseDisplay) {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let encoded = serde_json::json!({ "hidden": display.hidden, "colors": display.colors });
        let _ = storage.set_item(key, &encoded.to_string());
    }
}

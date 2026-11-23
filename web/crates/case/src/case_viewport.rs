//! Case scene admission, selection, display preferences, and edit gestures.

pub(crate) mod controls;

use crate::case_handles::{
    gasket_handles, mechanical_mounts, mount_constraints, mount_handles, project_handles,
};
use crate::case_preview::NativePreviewSnapshot;
use crate::mechanical_settings::{
    self, MechanicalSettingsPatchExt, gasket_anchor, gasket_group, is_gasket_layer,
    submit_settings_patch,
};
use crate::runtime::{CadScene, Runtime};
use crate::shared_viewer::{
    CaseDisplay, CaseMountTarget, HandleGesturePhase, ScopedDisplayChange, ScopedViewerSignal,
    ViewerFocusRequest, ViewerHandle, ViewerHandleTarget, ViewerIdentity, ViewerSignalKind,
    Viewport3D,
};
use crate::{InstanceSelection, ResolvedAppearance, selection::SelectionAdapter};
use crate::{case_bodies::CaseBodyEdit, viewport_camera::Drag};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_model::model::{
    MechanicalGasketAnchor, MechanicalGasketSupport, MechanicalGasketTrack, Mount, ProjectDoc, Vec2,
};
use boardstudio_web_host::cad_jobs::captured_case_document;
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertDescription, AlertVariant},
    button::{Button, ButtonVariant},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    item::{Item, ItemSize},
};
use dioxus::prelude::*;
use mechanical_settings::{
    MechanicalMountCollection as Collection, MechanicalSettingsProps as Settings,
};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

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

#[derive(Clone, PartialEq)]
struct CaseKeycapsSource {
    scope: Scope,
    token: SnapshotToken,
    revision: u64,
    prepared: Option<boardstudio_model::model::PreparedCaseAssemblyIR>,
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
    let mut retry = use_signal(|| 0_u64);
    use_drop({
        let runtime = runtime.clone();
        move || runtime.cancel_keycaps_cad_preview()
    });
    let mut read_source = use_reactive((&source,), |(source,)| source);
    let result = use_resource(move || {
        let request = (read_source(), retry());
        let runtime = runtime.clone();
        async move {
            runtime.cancel_keycaps_cad_preview();
            let outcome = async {
                let Some(source) = request.0.as_ref() else {
                    return Ok((None, Vec::new()));
                };
                let fit = runtime.resolve_case_keycaps_preview(
                    source.scope.clone(),
                    source.token,
                    source.revision,
                )?;
                let preview = if fit.specs.is_empty()
                    || fit.findings.iter().any(|finding| {
                        finding.severity == boardstudio_model::model::Severity::Error
                    }) {
                    None
                } else {
                    Some(
                        runtime
                            .request_keycaps_cad_preview(crate::runtime::KeycapsPreviewInput {
                                scope: source.scope.clone(),
                                token: source.token,
                                revision: source.revision,
                                specs: fit.specs,
                            })
                            .await?,
                    )
                };
                Ok::<_, String>((preview, fit.findings))
            }
            .await;
            (request, outcome)
        }
    });
    let shown = result.read();
    let current = shown
        .as_ref()
        .filter(|((owner, attempt), _)| owner == &source && *attempt == retry());
    let (preview, findings, error) = match current.map(|(_, result)| result) {
        Some(Ok((preview, findings))) => (preview.clone(), findings.as_slice(), None),
        Some(Err(error)) => (None, &[][..], Some(error)),
        None => (None, &[][..], None),
    };
    let feedback = rsx! {
        if source.is_some() && current.is_none() { p { role: "status", "Generating keycap CAD…" } }
        if let Some(error) = error {
            Alert { variant: AlertVariant::Destructive,
                AlertDescription { "Keycap preview failed: {error}" }
                Button { variant: ButtonVariant::Secondary, button_type: "button",
                    class: "max-[960px]:min-h-11 [@media(pointer:coarse)]:min-h-11",
                    onclick: move |_| retry += 1, "Retry keycaps" }
            }
        }
        if !findings.is_empty() {
            Collapsible { class: "flex-none max-h-[min(25vh,180px)] overflow-auto overscroll-contain", keep_mounted: true, default_open: true,
                CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground", aria_label: "Keycap clearance · {findings.len()} findings", "Keycap clearance · {findings.len()} findings" }
                CollapsibleContent { class: "data-[open=false]:hidden",
                    for finding in findings { Item { size: ItemSize::Sm, key: "{finding.id}", p { "{finding.message}" } } }
                }
            }
        }
    };
    (preview, feedback)
}

#[derive(Clone, PartialEq)]
pub(crate) enum CaseViewportSource {
    Scene {
        scene: Rc<CadScene>,
        pcb_preview: Option<Rc<NativePreviewSnapshot>>,
    },
    Preview(Rc<NativePreviewSnapshot>),
}

impl CaseViewportSource {
    pub(crate) fn scope(&self) -> &Scope {
        match self {
            Self::Scene { scene, .. } => &scene.scope,
            Self::Preview(preview) => &preview.owner.scope,
        }
    }
}

#[component]
pub(crate) fn CaseViewport(
    source: CaseViewportSource,
    model_rows: Option<crate::model_delivery::ModelDeliveryRows>,
    mechanical_settings: Option<crate::MechanicalSettingsProps>,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let scope = source.scope();
    let (token, revision, scene, preview) = match &source {
        CaseViewportSource::Scene { scene, pcb_preview } => (
            scene.token,
            scene.snapshot.document.revision,
            Some(scene.clone()),
            pcb_preview.clone(),
        ),
        CaseViewportSource::Preview(preview) => (
            preview.owner.snapshot_token,
            preview.accepted_document.revision,
            None,
            Some(preview.clone()),
        ),
    };
    let (keycaps_preview, keycaps_feedback) =
        use_case_keycaps(runtime.clone(), scope, token, revision);
    let instance_selection = use_context::<InstanceSelection>();
    let live_preview = (use_context::<crate::CaseGenerationState>().live_preview)();
    let selection_adapter = use_context::<SelectionAdapter>();
    let mut selection = use_context::<CaseSelection>();
    let theme = use_context::<ResolvedAppearance>().0;
    let selected_reference = {
        let selected = selection_adapter
            .selected_context
            .read()
            .clone()
            .filter(|selected| selected.scope == *scope);
        crate::case_selection::selected_part_summary(&runtime.model(), selected.as_ref())
            .map(|summary| summary.title)
    };
    let key = display_key(scope);
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
    let selected_layer = scene.as_ref().map_or_else(
        || "pcb".to_owned(),
        |scene| {
            let layer = selection.layer_id(scope);
            if is_layer(scene, &layer) {
                return layer;
            }
            selection
                .body
                .read()
                .as_ref()
                .filter(|body| body.scope == *scope && is_layer(scene, &body.body_id))
                .map(|body| body.body_id.clone())
                .unwrap_or_default()
        },
    );
    let focus_request = scene
        .as_ref()
        .and_then(|scene| selection.viewer_focus_request(scene));
    let handles = scene.as_ref().map_or_else(Vec::new, |scene| {
        editable_handles(
            scene,
            mechanical_settings.as_ref(),
            (selection.body_edit_portal.editable)(),
        )
    });
    let gesture = use_hook(|| Rc::new(RefCell::new(CaseDrag::default())));
    let mut handle_preview = use_signal(|| None::<Vec<ViewerHandle>>);
    let mut gesture_field = use_signal(|| None::<String>);
    let mut gesture_message = use_signal(|| None::<String>);
    use_effect(use_reactive(
        (scope, &token, &revision, &scene.as_ref().map(Rc::as_ptr)),
        {
            let gesture = gesture.clone();
            let runtime = runtime.clone();
            move |_| {
                retire_drag(&runtime, &gesture);
                handle_preview.set(None);
                gesture_field.set(None);
                gesture_message.set(None);
            }
        },
    ));
    use_effect(use_reactive((&live_preview,), {
        let gesture = gesture.clone();
        let runtime = runtime.clone();
        move |(live_preview,)| {
            if !live_preview && let Some(owner) = gesture.borrow_mut().preview.take() {
                runtime.cancel_case_gesture_preview(&owner);
            }
        }
    }));
    use_drop({
        let runtime = runtime.clone();
        let gesture = gesture.clone();
        move || {
            retire_drag(&runtime, &gesture);
        }
    });
    let current = {
        let scene = scene.clone();
        let preview = preview.clone();
        let runtime = runtime.clone();
        Rc::new(move |identity: &ViewerIdentity| match scene.as_ref() {
            Some(scene) => source_is_current(&runtime, instance_selection, scene, identity),
            None => preview.as_ref().is_some_and(|preview| {
                identity.scope == preview.owner.scope
                    && identity.snapshot_token == preview.owner.snapshot_token
                    && runtime.native_case_preview().is_some_and(|current| {
                        current.owner == preview.owner && Rc::ptr_eq(&current.lease, &preview.lease)
                    })
            }),
        })
    };
    let on_signal =
        {
            let runtime = runtime.clone();
            let scene = scene.clone();
            let preview = preview.clone();
            let handles = handles.clone();
            let settings = mechanical_settings.clone();
            let current = current.clone();
            move |event: ScopedViewerSignal| {
                if !event.is_current() || !current(&event.identity) {
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
                if let ViewerSignalKind::Failed(message) = event.kind {
                    runtime.report(message);
                    return;
                }
                let Some(scene) = scene.as_ref() else { return };
                match event.kind {
                    ViewerSignalKind::Picked(id) | ViewerSignalKind::LayerSelected(id)
                        if is_layer(scene, &id) =>
                    {
                        selection.select_layer(scene.scope.clone(), id.clone());
                        let authored = captured_case_document(&scene.snapshot, &scene.scope)
                            .is_ok_and(|document| {
                                document.case_bodies.iter().any(|body| {
                                    body.id == id && body.board_id == scene.scope.board_id
                                })
                            });
                        selection.body.set(authored.then(|| BodySelection {
                            scope: scene.scope.clone(),
                            body_id: id,
                        }));
                    }
                    ViewerSignalKind::HandleGesture {
                        phase,
                        handle_id,
                        point,
                    } => handle_case_gesture(
                        CaseGestureContext {
                            scene,
                            identity: &event.identity,
                            settings: settings.as_ref(),
                            handles: &handles,
                            gesture: &gesture,
                            preview: &mut handle_preview,
                            feedback_field: &mut gesture_field,
                            message: &mut gesture_message,
                            selection,
                            runtime: &runtime,
                            live_preview,
                        },
                        phase,
                        &handle_id,
                        point,
                    ),
                    _ => {}
                }
            }
        };
    let on_display_change = move |event: ScopedDisplayChange| {
        if event.is_current() && current(&event.identity) {
            selection.save_display(&event.identity.scope, event.display);
        }
    };
    let displayed_scene = scene.as_ref().map(|scene| {
        runtime
            .case_gesture_preview_scene(scene)
            .unwrap_or_else(|| scene.clone())
    });
    let gesture_message = runtime.case_gesture_preview_message().or_else(|| {
        direct_gesture_message(
            gesture_field(),
            gesture_message(),
            mechanical_settings.as_ref(),
        )
    });
    rsx! {
        Viewport3D {
            scene: displayed_scene,
            handle_source: scene.clone(),
            preview: preview.clone(),
            layout_preview: None,
            keycaps_preview,
            parts_preview: None,
            model_rows: model_rows.clone(),
            selected_layer,
            selected_reference,
            display,
            resolved_theme: theme,
            on_signal,
            on_display_change,
            mechanical_settings,
            inline_case_controls: true,
            handles,
            handle_preview: handle_preview(),
            gesture_message,
            focus_request,
        }
        {keycaps_feedback}
        if let Some(scene) = scene {
            crate::case_collision::CaseCollisionChecks { scene, preview, model_rows }
        }
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
    let model = runtime.model();
    if !instance_selection.is_current(&model)
        || runtime.native_case_preview().is_none_or(|current| {
            current.owner != *owner || !Rc::ptr_eq(&current.lease, &preview.lease)
        })
    {
        return false;
    }
    let Some(part_id) = crate::case_preview::native_preview_pick_part_id(
        preview,
        &identity.scope,
        identity.snapshot_token,
        identity.revision,
        model_reference,
    ) else {
        return false;
    };
    let Some(context) = crate::objects::context_for_part(&model, &part_id) else {
        return false;
    };
    crate::selection::submit_context(
        runtime,
        adapter,
        crate::selection::ContextRequest {
            scope: owner.scope.clone(),
            context,
            mode: boardstudio_application::SelectionMode::Replace,
        },
    );
    true
}

// A disposable gesture preview inherits handles only from the current accepted scene.
pub(super) fn gesture_source_is_current(
    runtime: &Runtime,
    accepted: &Rc<CadScene>,
    displayed: &Rc<CadScene>,
) -> bool {
    runtime
        .cad_scene()
        .is_some_and(|current| Rc::ptr_eq(&current, accepted))
        && (Rc::ptr_eq(displayed, accepted)
            || runtime
                .case_gesture_preview_scene(accepted)
                .is_some_and(|preview| Rc::ptr_eq(displayed, &preview)))
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

pub(super) fn selectable_layers(scene: Option<&CadScene>) -> Vec<String> {
    let mut layers = vec!["pcb".to_owned()];
    let Some(scene) = scene else { return layers };
    if let Some(assembly) = &scene.mechanical {
        layers.extend(assembly.stack.iter().map(|layer| layer.id.clone()));
        layers.extend(gasket_group(&assembly.gasket_supports).map(str::to_owned));
    }
    layers.extend(
        scene
            .result
            .bodies
            .iter()
            .filter(|body| body.id.starts_with("gasket:"))
            .map(|body| body.id.clone()),
    );
    layers
}

fn is_layer(scene: &CadScene, id: &str) -> bool {
    id == "pcb"
        || scene.result.bodies.iter().any(|body| body.id == id)
        || scene.mechanical.as_ref().is_some_and(|assembly| {
            let supports = &assembly.gasket_supports;
            assembly.stack.iter().any(|layer| layer.id == id)
                || is_gasket_layer(id, supports, |support| &support.id)
        })
}

fn read_display(key: &str) -> CaseDisplay {
    let value = web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(key).ok().flatten())
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
    let field = |key| value.as_ref().and_then(|value| value.get(key));
    CaseDisplay {
        hidden: field("hidden")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|id| id.as_str().map(str::to_owned))
            .collect(),
        colors: field("colors")
            .and_then(serde_json::Value::as_object)
            .into_iter()
            .flatten()
            .filter_map(|(id, color)| Some((id.clone(), color.as_str()?.to_owned())))
            .collect(),
    }
}

fn display_key(scope: &Scope) -> String {
    format!(
        "boardstudio:case-display:{}:{}",
        scope.document_id,
        scope.instance_id.as_deref().unwrap_or(&scope.board_id),
    )
}

fn persist_display(key: &str, display: &CaseDisplay) {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let encoded = serde_json::json!({ "hidden": display.hidden, "colors": display.colors });
        let _ = storage.set_item(key, &encoded.to_string());
    }
}

use crate::case_gesture_math::{
    MountMoveConstraints, move_case_mount, move_gasket_support, supports_equal_positions,
};

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

impl CaseGestureDraft {
    fn move_to(&mut self, point: Vec2) -> bool {
        match self {
            Self::Gasket {
                support_id,
                before,
                pending,
                tracks,
            } => {
                if let Some(next) = move_gasket_support(point, support_id, before, tracks) {
                    *pending = next;
                    return true;
                }
            }
            Self::Mount {
                mount_id,
                before,
                pending,
                constraints,
                ..
            } => {
                if let Some(next) = move_case_mount(point, mount_id, before, constraints) {
                    *pending = next;
                    return true;
                }
            }
        }
        false
    }
}

fn editable_handles(
    scene: &CadScene,
    settings: Option<&mechanical_settings::MechanicalSettingsProps>,
    authored_mounts_editable: bool,
) -> Vec<ViewerHandle> {
    let settings = settings.filter(|settings| {
        settings.editable
            && settings.identity.scope == scene.scope
            && settings.identity.snapshot_token == scene.token
            && settings.identity.revision == scene.snapshot.document.revision
    });
    project_handles(
        scene,
        settings.is_some(),
        settings.and_then(|settings| settings.configuration.as_deref()),
        authored_mounts_editable,
    )
}

type CaseDrag = Drag<CaseGestureDraft, crate::case_gesture_preview::CaseGesturePreviewOwner>;

fn retire_drag(runtime: &Runtime, gesture: &RefCell<CaseDrag>) -> Option<CaseGestureDraft> {
    let (draft, preview) = gesture.borrow_mut().retire();
    if let Some(preview) = preview {
        runtime.cancel_case_gesture_preview(&preview);
    }
    draft
}

struct CaseGestureContext<'a> {
    pub(super) scene: &'a CadScene,
    pub(super) identity: &'a ViewerIdentity,
    pub(super) settings: Option<&'a mechanical_settings::MechanicalSettingsProps>,
    pub(super) handles: &'a [ViewerHandle],
    pub(super) gesture: &'a RefCell<CaseDrag>,
    pub(super) preview: &'a mut Signal<Option<Vec<ViewerHandle>>>,
    pub(super) feedback_field: &'a mut Signal<Option<String>>,
    pub(super) message: &'a mut Signal<Option<String>>,
    pub(super) selection: CaseSelection,
    pub(super) runtime: &'a Rc<Runtime>,
    pub(super) live_preview: bool,
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
        HandleGesturePhase::Start if identity_current => {
            let Some(handle) = handles.iter().find(|handle| handle.id == handle_id) else {
                return;
            };
            let Some(draft) = start_drag(scene, settings, selection, &handle.target) else {
                return;
            };
            gesture.borrow_mut().draft = Some(draft);
            feedback_field.set(None);
            message.set(Some(if handle_id.starts_with("gasket-handle:") {
                "Drag the gasket support; release to save its position.".to_owned()
            } else {
                "Drag the mount; release to save its case position.".to_owned()
            }));
        }
        HandleGesturePhase::Move if identity_current => {
            let Some(point) = point.map(world_xy) else {
                return;
            };
            let (handle_overlay, guidance, provisional) = {
                let mut gesture = gesture.borrow_mut();
                let Some(draft) = gesture.draft.as_mut() else {
                    return;
                };
                let valid = draft.move_to(point);
                let (overlay, guidance) = match draft {
                    CaseGestureDraft::Gasket {
                        support_id,
                        pending,
                        ..
                    } => (
                        gasket_handles(handles, pending, (!valid).then_some(support_id.as_str())),
                        if valid {
                            "Release to place; fit is checked after placement."
                        } else {
                            "No valid gasket perimeter position here."
                        },
                    ),
                    CaseGestureDraft::Mount {
                        target,
                        mount_id,
                        pending,
                        ..
                    } => (
                        mount_handles(
                            handles,
                            target,
                            pending,
                            (!valid).then_some(mount_id.as_str()),
                        ),
                        if valid {
                            "Release to save mount position."
                        } else {
                            "Placement blocked by the case edge, openings, or another mount."
                        },
                    ),
                };
                let provisional = (valid && live_preview)
                    .then(|| case_gesture_preview_document(scene, draft))
                    .flatten();
                (overlay, guidance, provisional)
            };
            preview.set(Some(handle_overlay));
            message.set(Some(guidance.into()));
            if let Some(document) = provisional
                && let Some(owner) = runtime.update_case_gesture_preview(
                    scene.scope.clone(),
                    scene.token,
                    scene.snapshot.document.revision,
                    document,
                )
            {
                gesture.borrow_mut().preview = Some(owner);
            }
        }
        HandleGesturePhase::End => {
            let current = retire_drag(runtime, gesture);
            preview.set(None);
            if !identity_current {
                message.set(None);
                return;
            }
            let Some(mut current) = current else { return };
            if point.is_some_and(|point| !current.move_to(world_xy(point))) {
                message.set(Some("Blocked move was not saved.".into()));
                return;
            }
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
                        let anchors = changed_anchors(&before, &pending).collect::<Vec<_>>();
                        (!anchors.is_empty()).then_some(
                            mechanical_settings::MechanicalSettingsPatch::SetGasketSupportPlacement {
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
                            mechanical_settings::MechanicalSettingsPatch::SetMountPosition {
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
        }
        HandleGesturePhase::Cancel => {
            retire_drag(runtime, gesture);
            preview.set(None);
            feedback_field.set(None);
            message.set(Some("Move cancelled.".into()));
        }
        _ => {}
    }
}

fn start_drag(
    scene: &CadScene,
    settings: Option<&Settings>,
    selection: CaseSelection,
    handle: &ViewerHandleTarget,
) -> Option<CaseGestureDraft> {
    let (body, mount_id, mounts, target) = match handle {
        ViewerHandleTarget::Gasket { support_id } => {
            settings?;
            let assembly = scene.mechanical.as_ref()?;
            let before = &assembly.gasket_supports;
            before.iter().find(|support| support.id == *support_id)?;
            selection.select_layer(scene.scope.clone(), format!("gasket:{support_id}:lower"));
            return Some(CaseGestureDraft::Gasket {
                support_id: support_id.clone(),
                before: before.clone(),
                pending: before.clone(),
                tracks: assembly.gasket_tracks.clone(),
            });
        }
        ViewerHandleTarget::Mount { owner, mount_id } => {
            let (body, mounts) = match owner {
                CaseMountTarget::Mechanical(collection) => {
                    let (body, mounts) =
                        mechanical_mounts(settings?.configuration.as_deref()?, *collection);
                    (body, mounts.to_vec())
                }
                CaseMountTarget::Authored { body_id } => {
                    if !(selection.body_edit_portal.editable)() {
                        return None;
                    }
                    let document = captured_case_document(&scene.snapshot, &scene.scope).ok()?;
                    let body = document.case_bodies.into_iter().find(|body| {
                        body.id == *body_id && body.board_id == scene.scope.board_id
                    })?;
                    (body_id.as_str(), body.mounts?)
                }
            };
            (body, mount_id, mounts, owner.clone())
        }
    };
    mounts.iter().find(|mount| mount.id == *mount_id)?;
    Some(CaseGestureDraft::Mount {
        target,
        mount_id: mount_id.clone(),
        pending: mounts.clone(),
        before: mounts,
        constraints: mount_constraints(scene, body)?,
    })
}

fn direct_gesture_message(
    field_id: Option<String>,
    transient: Option<String>,
    settings: Option<&mechanical_settings::MechanicalSettingsProps>,
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
        mechanical_settings::MechanicalSettingsFeedbackState::Landed => None,
        mechanical_settings::MechanicalSettingsFeedbackState::Failed => Some(
            feedback
                .message
                .clone()
                .unwrap_or_else(|| "Position could not be saved.".into()),
        ),
    }
}

fn world_xy(point: [f32; 3]) -> Vec2 {
    Vec2 {
        x: f64::from(point[0]),
        y: f64::from(point[1]),
    }
}

fn changed_anchors<'a>(
    before: &'a [MechanicalGasketSupport],
    pending: &'a [MechanicalGasketSupport],
) -> impl Iterator<Item = MechanicalGasketAnchor> + 'a {
    pending
        .iter()
        .filter(|support| {
            before
                .iter()
                .find(|old| old.id == support.id)
                .is_none_or(|old| (old.anchor - support.anchor).abs() > 1e-7)
        })
        .map(gasket_anchor)
}

fn case_gesture_preview_document(scene: &CadScene, draft: &CaseGestureDraft) -> Option<ProjectDoc> {
    let effective = captured_case_document(&scene.snapshot, &scene.scope).ok()?;
    let mut document = (*scene.snapshot.document).clone();
    let mut configuration = effective.mechanical;
    match draft {
        CaseGestureDraft::Gasket {
            before, pending, ..
        } => {
            let supports = &mut configuration.as_mut()?.gasket_layout.as_mut()?.supports;
            for anchor in changed_anchors(before, pending) {
                if let Some(existing) = supports
                    .iter_mut()
                    .find(|existing| existing.id == anchor.id)
                {
                    *existing = anchor;
                } else {
                    supports.push(anchor);
                }
            }
        }
        CaseGestureDraft::Mount {
            target, pending, ..
        } => match target {
            CaseMountTarget::Mechanical(Collection::Suspension) => {
                configuration.as_mut()?.mounts = pending.clone();
            }
            CaseMountTarget::Mechanical(Collection::Closure) => {
                configuration.as_mut()?.closure_mounts = Some(pending.clone());
            }
            CaseMountTarget::Authored { body_id } => {
                let body = document
                    .case_bodies
                    .iter_mut()
                    .find(|body| body.id == *body_id && body.board_id == scene.scope.board_id)?;
                body.mounts = Some(pending.clone());
                return Some(document);
            }
        },
    }
    // The projection reapplies shared defaults and retains instance-local placements.
    let target = if let Some(instance_id) = scene.scope.instance_id.as_deref() {
        &mut document
            .hardware
            .as_mut()?
            .instances
            .iter_mut()
            .find(|instance| instance.id == instance_id)?
            .mechanical
    } else {
        &mut document.mechanical
    };
    *target = Some(configuration?);
    Some(document)
}

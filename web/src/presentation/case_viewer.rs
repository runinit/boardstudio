//! Case owns display persistence and domain selection; the viewer owns GPU state.
use super::case_bodies::CaseBodyEdit;
use super::shared_viewer::{
    CaseDisplay, CaseSharedViewer, HandleGesturePhase, ScopedDisplayChange, ScopedViewerSignal,
    ViewerHandle, ViewerHandleTarget, ViewerIdentity, ViewerSignalKind,
};
use super::{InstanceSelection, ResolvedTheme, selection::SelectionAdapter};
use crate::runtime::{CadScene, Runtime};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{
    CaseKind, GasketPlacement, MechanicalGasketAnchor, MechanicalGasketSupport,
    MechanicalGasketTrack, MechanicalMount, Mount, MountKind, Vec2,
};
use boardstudio_web::cad_jobs::captured_case_document;
use dioxus::prelude::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

#[derive(Clone, PartialEq)]
pub(super) struct BodySelection {
    pub(super) scope: Scope,
    pub(super) body_id: String,
}

#[derive(Clone, PartialEq)]
pub(super) struct LayerSelection {
    scope: Scope,
    id: String,
}

#[derive(Clone, Copy)]
pub(super) struct CaseSelection {
    pub(super) body: Signal<Option<BodySelection>>,
    pub(super) layer: Signal<Option<LayerSelection>>,
    pub(super) display: Signal<BTreeMap<String, CaseDisplay>>,
    pub(super) body_edit_portal: CaseBodyEditPortal,
}

#[derive(Clone, Copy)]
pub(super) struct CaseBodyEditPortal {
    pub(super) dispatch: Signal<Option<CaseBodyEditDispatch>>,
    pub(super) editable: Signal<bool>,
}

pub(super) type CaseBodyEditDispatch = Rc<dyn Fn(CaseBodyEdit, Scope, SnapshotToken, u64)>;

impl CaseSelection {
    pub(super) fn layer_id(self, scope: &Scope) -> String {
        self.layer
            .read()
            .as_ref()
            .filter(|selection| &selection.scope == scope)
            .map(|selection| selection.id.clone())
            .unwrap_or_default()
    }

    pub(super) fn select_layer(mut self, scope: Scope, id: String) {
        self.layer.set(Some(LayerSelection { scope, id }));
    }

    pub(super) fn clear_layer_for_scope(mut self, scope: &Scope) {
        if self
            .layer
            .read()
            .as_ref()
            .is_some_and(|selection| &selection.scope == scope)
        {
            self.layer.set(None);
        }
    }

    pub(super) fn display_value(self, scope: &Scope) -> CaseDisplay {
        let key = display_key(scope);
        self.display
            .read()
            .get(&key)
            .cloned()
            .unwrap_or_else(|| read_display(&key))
    }

    pub(super) fn save_display(mut self, scope: &Scope, display: CaseDisplay) {
        let key = display_key(scope);
        self.display.write().insert(key.clone(), display.clone());
        persist_display(&key, &display);
    }
}

#[component]
pub(crate) fn CaseViewer(
    scene: Rc<CadScene>,
    preview: Option<Rc<crate::case_preview::NativePreviewSnapshot>>,
    model_rows: Option<super::model_delivery::ModelDeliveryRows>,
    mechanical_settings: Option<super::MechanicalSettingsProps>,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
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
            .filter(|selected| selected.scope == scene.scope);
        super::case_workspace::selected_part_summary(&model, selected.as_ref())
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
    let direct_handles = case_viewer_handles(
        &scene,
        mechanical_settings.as_ref(),
        (selection.body_edit_portal.editable)(),
    );
    let gesture = use_hook(|| Rc::new(RefCell::new(None::<CaseGestureDraft>)));
    let handle_preview = use_signal(|| None::<Vec<ViewerHandle>>);
    let gesture_field = use_signal(|| None::<String>);
    let gesture_message = use_signal(|| None::<String>);
    use_effect(use_reactive(
        (
            &scene.scope,
            &scene.token,
            &scene.snapshot.document.revision,
        ),
        {
            let gesture = gesture.clone();
            let mut handle_preview = handle_preview;
            let mut gesture_field = gesture_field;
            let mut gesture_message = gesture_message;
            move |_| {
                *gesture.borrow_mut() = None;
                handle_preview.set(None);
                gesture_field.set(None);
                gesture_message.set(None);
            }
        },
    ));
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
    let displayed_gesture_message = direct_gesture_message(
        gesture_field(),
        gesture_message(),
        mechanical_settings.as_ref(),
    );
    rsx! {
        CaseSharedViewer {
            scene: Some(scene),
            preview,
            layout_preview: None,
            keycaps_preview: None,
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
        }
    }
}

#[component]
pub(crate) fn CasePreviewViewer(
    preview: Rc<crate::case_preview::NativePreviewSnapshot>,
    model_rows: Option<super::model_delivery::ModelDeliveryRows>,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
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
        super::case_workspace::selected_part_summary(&model, selected.as_ref())
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
            keycaps_preview: None,
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
        mut selection,
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
            match current {
                CaseGestureDraft::Gasket {
                    support_id,
                    before,
                    pending,
                    tracks,
                } => {
                    if let Some(next) = move_gasket_support(point, &support_id, &before, &tracks) {
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
        }
        HandleGesturePhase::End => {
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
    Some(match feedback.state {
        super::mechanical_settings::MechanicalSettingsFeedbackState::Pending => {
            "Saving position…".into()
        }
        super::mechanical_settings::MechanicalSettingsFeedbackState::Saved => {
            "Position saved.".into()
        }
        super::mechanical_settings::MechanicalSettingsFeedbackState::Failed => feedback
            .message
            .clone()
            .unwrap_or_else(|| "Position could not be saved.".into()),
    })
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
        super::objects::TreeSelectRequest {
            scope: owner.scope.clone(),
            context,
            mode: boardstudio_application::SelectionMode::Replace,
            outline_action: None,
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
        && model
            .accepted
            .as_ref()
            .is_some_and(|accepted| accepted.token == identity.snapshot_token)
        && runtime
            .cad_scene()
            .as_ref()
            .is_some_and(|current| Rc::ptr_eq(current, expected))
}

fn is_layer(scene: &CadScene, id: &str) -> bool {
    id == "pcb"
        || scene.result.bodies.iter().any(|body| body.id == id)
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

fn persist_display(key: &str, display: &CaseDisplay) {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let encoded = serde_json::json!({ "hidden": display.hidden, "colors": display.colors });
        let _ = storage.set_item(key, &encoded.to_string());
    }
}

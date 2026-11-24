//! Outline planning, direct projection, and synchronous commit helpers.

pub use boardstudio_web_layout_core::outline::{
    self as planner, EditablePerimeter, OutlineAction, OutlineActionEnvelope, OutlineActionKind,
    OutlineDrawTool, OutlineFeedback, OutlinePointTarget, OutlineVersionChoice,
    OutlineVersionOwnership, PerimeterAnchor, active_outline, attach_connection_point,
    blocked_export_labels, connection_point_world, editable_perimeter, evaluate_outline_edit,
    gap_focus_camera, move_connection_point, outline_action_matches_saved_owner, outline_edit,
    outline_state, outline_version, target_ids, version_geometry,
};

#[cfg(target_arch = "wasm32")]
pub(crate) mod inspector;
#[cfg(target_arch = "wasm32")]
pub(crate) mod overlays;

#[cfg(target_arch = "wasm32")]
pub use inspector::OutlineVersionInspector;
#[cfg(target_arch = "wasm32")]
pub use overlays::{OutlineDraftCanvasOverlay, OutlinePointCanvasOverlay, OutlineRuntimeHandle};

#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
// Child presentation modules import these shared layout modules through `super::*`.
pub(crate) use crate::{coordinates, keycaps_fit, objects, selection};

#[cfg(target_arch = "wasm32")]
use crate::{
    outline_settings::{OutlineEdit, envelope_margin, generated_feature, generated_settings},
    runtime::Runtime,
};
#[cfg(target_arch = "wasm32")]
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Event, Lifecycle, Resolution, Scope,
};
#[cfg(target_arch = "wasm32")]
use boardstudio_model::model::{
    Contour, EditOperation, OutlineConnection, OutlineFeature, OutlineGap, OutlineRepairSettings,
    OutlineSettings, Part, Vec2,
};
#[cfg(target_arch = "wasm32")]
use boardstudio_web_ui_model::Workspace;
#[cfg(target_arch = "wasm32")]
use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Default, PartialEq, Eq)]
pub struct OutlineVersionUi {
    pub feedback: Option<OutlineFeedback>,
    pub version_name_draft: String,
    pub version_name_failure: Option<String>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Default, PartialEq)]
pub struct OutlineCanvasUi {
    pub selected_point: usize,
    pub drawing_points: Vec<Vec2>,
    pub selected_feature_id: Option<String>,
    pub selected_connection_id: Option<String>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, PartialEq)]
pub struct OutlineUiState {
    pub editing_points: Signal<bool>,
    pub drawing_operation: Signal<Option<OutlineDrawTool>>,
    pub version_ui: Signal<OutlineVersionUi>,
    pub canvas_ui: Signal<OutlineCanvasUi>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, PartialEq)]
pub struct OutlineInspectorProjection {
    pub runtime: OutlineRuntimeHandle,
    pub ui: OutlineUiState,
    pub selected_context: Signal<Option<crate::objects::ScopedTreeContext>>,
    pub workspace: Signal<Workspace>,
    pub scope_generation: Signal<u64>,
    pub captured_generation: u64,
    pub board_id: String,
    pub board_name: String,
    pub version_name: String,
    pub active_version_id: Option<String>,
    pub contours: Vec<Contour>,
    pub(super) perimeter: Option<EditablePerimeter>,
    pub(super) snap_paths: Vec<Vec<Vec2>>,
    pub geometry_features: Vec<OutlineFeature>,
    pub connections: Vec<OutlineConnection>,
    pub(super) connection_feature: Option<OutlineFeature>,
    pub(super) outline_parts: Vec<Part>,
    pub versions: Vec<OutlineVersionChoice>,
    pub settings: OutlineSettings,
    pub repair: OutlineRepairSettings,
    /// The margin the envelope-tied controls edit: Generated's, or a linked refinement's own.
    pub envelope_margin: Option<f64>,
    pub has_generated: bool,
    pub gaps: Vec<OutlineGap>,
    pub enabled: bool,
    pub feedback: Option<OutlineFeedback>,
    pub selection_context: crate::objects::TreeContext,
    pub scope: Scope,
    pub(super) token: boardstudio_application::SnapshotToken,
    pub(super) revision: u64,
    pub(super) generation: u64,
}

#[cfg(target_arch = "wasm32")]
impl OutlineInspectorProjection {
    pub fn canvas_edit_key(&self) -> String {
        format!(
            "{:?}-{}-{:?}-{}-{}-{}",
            self.scope,
            self.board_id,
            self.active_version_id,
            self.token.0,
            self.revision,
            self.generation
        )
    }

    pub fn action_envelope(&self) -> OutlineActionEnvelope {
        OutlineActionEnvelope {
            scope: self.scope.clone(),
            token: self.token,
            revision: self.revision,
            generation: self.generation,
            board_id: self.board_id.clone(),
        }
    }

    pub fn dispatch(&self, kind: OutlineActionKind) {
        commit_outline_action(
            &self.runtime.0,
            self.ui,
            self.selected_context,
            self.workspace,
            self.scope_generation,
            self.captured_generation,
            OutlineAction {
                envelope: self.action_envelope(),
                kind,
            },
        );
    }

    pub fn update(&self, edit: OutlineEdit) {
        self.dispatch(OutlineActionKind::Update { edit });
    }

    pub fn has_linked_active_version(&self) -> bool {
        self.active_version_id.as_ref().is_some_and(|vid| {
            self.versions
                .iter()
                .any(|c| &c.id == vid && c.ownership == OutlineVersionOwnership::Linked)
        })
    }
}

#[cfg(target_arch = "wasm32")]
pub fn outline_action_is_current(
    action: &OutlineAction,
    runtime: &Runtime,
    generation: u64,
) -> bool {
    outline_action_matches_saved_owner(
        action,
        &runtime.model(),
        runtime.scope().as_ref(),
        generation,
    )
}

#[cfg(target_arch = "wasm32")]
fn board_id_for_context(context: &crate::objects::TreeContext) -> Option<&str> {
    match context {
        crate::objects::TreeContext::Outline { board_id }
        | crate::objects::TreeContext::OutlineVersion { board_id, .. } => Some(board_id.as_str()),
        _ => None,
    }
}

#[cfg(target_arch = "wasm32")]
pub fn use_outline_state(
    runtime: &Rc<Runtime>,
    selected_context: Signal<Option<crate::objects::ScopedTreeContext>>,
    workspace: Signal<Workspace>,
    scope_generation: Signal<u64>,
) -> OutlineUiState {
    let version = use_context::<Signal<u64>>()();
    let captured_gen = scope_generation();
    let (editing_points, drawing_operation) =
        (use_signal(|| false), use_signal(|| None::<OutlineDrawTool>));
    let (version_ui, canvas_ui) = (
        use_signal(OutlineVersionUi::default),
        use_signal(OutlineCanvasUi::default),
    );
    let (draft_owner, draft_ws, draft_scope) = (
        selected_context.read().clone(),
        workspace(),
        runtime.scope(),
    );
    let owner_board = draft_owner
        .as_ref()
        .and_then(|o| board_id_for_context(&o.context));
    let (draft_active, draft_name) = runtime.model().accepted.as_ref().zip(owner_board).map_or(
        (None, String::new()),
        |(snap, bid)| {
            let (s, a) = active_outline(snap, bid);
            (
                s.and_then(|s| s.active_version_id.clone()),
                a.map(|v| v.name.clone()).unwrap_or_default(),
            )
        },
    );
    use_effect(use_reactive(
        (&version, &draft_owner, &draft_ws, &captured_gen),
        {
            let (mut drawing_op, mut canvas) = (drawing_operation, canvas_ui);
            move |_| {
                drawing_op.set(None);
                canvas.write().drawing_points.clear();
            }
        },
    ));
    use_effect(use_reactive(
        (
            &draft_owner,
            &draft_ws,
            &draft_active,
            &draft_scope,
            &captured_gen,
        ),
        {
            let (mut ver, mut canvas) = (version_ui, canvas_ui);
            move |_| {
                {
                    let mut c = canvas.write();
                    c.selected_feature_id = None;
                    c.selected_connection_id = None;
                }
                let mut v = ver.write();
                v.version_name_draft = draft_name.clone();
                v.version_name_failure = None;
            }
        },
    ));
    OutlineUiState {
        editing_points,
        drawing_operation,
        version_ui,
        canvas_ui,
    }
}

#[cfg(target_arch = "wasm32")]
pub fn project_outline_inspector(
    runtime: &Rc<Runtime>,
    ui: OutlineUiState,
    selected_context: Signal<Option<crate::objects::ScopedTreeContext>>,
    workspace: Signal<Workspace>,
    scope_generation: Signal<u64>,
) -> Option<OutlineInspectorProjection> {
    if workspace() != Workspace::Layout {
        return None;
    }
    let (captured_generation, model, scope) =
        (scope_generation(), runtime.model(), runtime.scope()?);
    let snapshot = model.accepted.as_ref()?;
    let selected = selected_context.read().clone()?;
    if selected.scope != scope
        || !crate::selection::context_is_current(&model, &scope, &selected.context)
    {
        return None;
    }
    let board_id = board_id_for_context(&selected.context)?.to_owned();
    if board_id != scope.board_id {
        return None;
    }
    let board = snapshot.document.boards.iter().find(|b| b.id == board_id)?;
    let (outline, active) = active_outline(snapshot, &board_id);
    let active_version_id = active.map(|v| v.id.clone());
    let blocked_exports = blocked_export_labels(snapshot, &board_id);
    let versions = outline
        .into_iter()
        .flat_map(|s| s.versions.iter())
        .map(|v| {
            let fid = format!("board:{board_id}:outline:linked:{}", v.id);
            let problem = snapshot.scene.findings.iter().find(|f| f.id == fid);
            OutlineVersionChoice {
                id: v.id.clone(),
                name: v.name.clone(),
                // A version written before ownership existed is Fixed, so it reads as Edited, never Linked.
                ownership: if v.ownership.is_fixed() {
                    OutlineVersionOwnership::Edited
                } else {
                    OutlineVersionOwnership::Linked
                },
                problem_finding_id: problem.map(|_| fid.clone()),
                problem_message: problem.map(|f| f.message.clone()),
                problem_selectable: snapshot
                    .scene
                    .finding_markers
                    .iter()
                    .any(|m| m.finding_id == fid && m.board_id == board_id),
                blocked_exports: if problem.is_some() {
                    blocked_exports.clone()
                } else {
                    Vec::new()
                },
            }
        })
        .collect();
    let generated = generated_feature(&snapshot.document, board);
    let envelope_margin = match active_version_id.as_deref() {
        None => generated.and_then(envelope_margin),
        Some(id) => outline_version(outline, id)
            .filter(|v| !v.ownership.is_fixed())
            .and_then(|v| v.geometry.features.iter().find_map(envelope_margin)),
    };
    let settings = active_version_id
        .as_deref()
        .and_then(|id| outline_version(outline, id))
        .map(|v| v.geometry.settings.clone())
        .or_else(|| generated.and_then(generated_settings))
        .unwrap_or_default();
    let repair = settings.repair.clone().unwrap_or_default();
    let scene = snapshot
        .scene
        .board_outline_scenes
        .iter()
        .find(|s| s.board_id == board_id);
    let (geometry_features, connections, connection_feature) = version_geometry(
        outline,
        active_version_id.as_deref(),
        board,
        &snapshot.document,
        generated,
    );
    let outline_parts = board
        .part_ids
        .iter()
        .filter_map(|id| snapshot.document.parts.iter().find(|p| &p.id == id))
        .cloned()
        .collect();
    let perimeter = editable_perimeter(
        snapshot,
        &board_id,
        active_version_id.as_deref(),
        ui.canvas_ui.read().selected_feature_id.as_deref(),
    );
    // Outline edits queue behind each other, so outline edits keep the panel editable while saving.
    let enabled = matches!(
        model.durability,
        Durability::Saved { .. } | Durability::Saving { .. }
    ) && model.lifecycle == Lifecycle::Ready
        && model.display_preview.is_none()
        && model.gesture.is_none();
    let feedback = ui
        .version_ui
        .read()
        .feedback
        .as_ref()
        .filter(|f| {
            f.board_id == board_id && f.scope == scope && f.generation == captured_generation
        })
        .cloned();
    Some(OutlineInspectorProjection {
        runtime: OutlineRuntimeHandle::new(runtime.clone()),
        ui,
        selected_context,
        workspace,
        scope_generation,
        captured_generation,
        board_id: board_id.clone(),
        board_name: board.name.clone(),
        version_name: active
            .map(|v| v.name.clone())
            .unwrap_or_else(|| "Generated".to_owned()),
        active_version_id,
        contours: snapshot
            .scene
            .board_contours
            .iter()
            .find(|b| b.board_id == board_id)
            .map_or_else(Vec::new, |b| b.contours.clone()),
        perimeter,
        snap_paths: scene
            .map(|s| s.source_contours.iter().map(|c| c.points.clone()).collect())
            .unwrap_or_default(),
        geometry_features,
        connections,
        connection_feature,
        outline_parts,
        versions,
        settings,
        repair,
        envelope_margin,
        has_generated: generated.is_some(),
        gaps: scene.map_or_else(Vec::new, |s| s.gaps.clone()),
        enabled,
        feedback,
        selection_context: selected.context,
        scope,
        token: snapshot.token,
        revision: snapshot.document.revision,
        generation: captured_generation,
    })
}

#[cfg(target_arch = "wasm32")]
pub fn focus_outline_gap(
    runtime: &Rc<Runtime>,
    snapshot: &AcceptedSnapshot,
    board_id: &str,
    gap_id: &str,
) {
    let Some(bounds) =
        crate::keycaps_fit::layout_canvas_bounds(&snapshot.document, &snapshot.scene, board_id)
    else {
        return;
    };
    let size = web_sys::window().and_then(|w| w.document())
        .and_then(|d| d.query_selector("[role='group'][aria-label^='Keyboard layout;'], [role='group'][aria-label^='Matrix placement canvas.']").ok().flatten())
        .map(|s| { let b = s.get_bounding_client_rect(); (b.width().max(1.0), b.height().max(1.0)) })
        .unwrap_or((800.0, 600.0));
    if let Some((center, zoom)) = gap_focus_camera(snapshot, board_id, gap_id, size, bounds) {
        runtime.submit(Event::SetCamera {
            operation_id: runtime.operation(),
            center,
            zoom,
        });
    }
}

#[cfg(target_arch = "wasm32")]
fn record_outline_error(
    ui: OutlineUiState,
    action: &OutlineAction,
    is_version_name: bool,
    message: Option<String>,
) {
    let mut version_ui = ui.version_ui;
    let mut ver = version_ui.write();
    if is_version_name {
        ver.version_name_failure = message;
    } else {
        ver.feedback = message.map(|msg| OutlineFeedback {
            scope: action.envelope.scope.clone(),
            generation: action.envelope.generation,
            board_id: action.envelope.board_id.clone(),
            message: Some(msg),
        });
    }
}

#[cfg(target_arch = "wasm32")]
pub fn preview_outline_perimeter(
    runtime: &Rc<Runtime>,
    ui: OutlineUiState,
    action: &OutlineAction,
    transaction_id: &str,
) -> Option<EditOperation> {
    if !outline_action_is_current(action, runtime, action.envelope.generation) {
        return None;
    }
    let snapshot = runtime.model().accepted?;
    let edit = match evaluate_outline_edit(&snapshot, action, runtime.operation().0) {
        Ok(Some(edit)) => edit,
        Ok(None) => return None,
        Err(msg) => {
            record_outline_error(ui, action, false, Some(msg));
            return None;
        }
    };
    let operation = EditOperation::ApplyOutlineEdit {
        edit: Box::new(edit),
    };
    runtime.submit(Event::PreviewEdit {
        operation_id: runtime.operation(),
        transaction_id: transaction_id.to_owned(),
        target_ids: target_ids(action),
        operation: operation.clone(),
    });
    Some(operation)
}

#[cfg(target_arch = "wasm32")]
pub fn commit_outline_action(
    runtime: &Rc<Runtime>,
    ui: OutlineUiState,
    mut selected_context: Signal<Option<crate::objects::ScopedTreeContext>>,
    workspace: Signal<Workspace>,
    scope_generation: Signal<u64>,
    captured_generation: u64,
    action: OutlineAction,
) {
    if workspace() != Workspace::Layout
        || scope_generation() != captured_generation
        || !outline_action_is_current(&action, runtime, captured_generation)
    {
        return;
    }
    let model = runtime.model();
    let Some(snapshot) = model.accepted.as_ref() else {
        return;
    };
    let env = &action.envelope;
    let sel_current = selected_context.read().as_ref().is_some_and(|sel| {
        sel.scope == env.scope
            && board_id_for_context(&sel.context) == Some(env.board_id.as_str())
            && crate::selection::context_is_current(&model, &env.scope, &sel.context)
    });
    if !sel_current {
        return;
    }
    if let OutlineActionKind::FocusGap { gap_id } = &action.kind {
        // Focusing a gap only moves the camera and never creates document history.
        focus_outline_gap(runtime, snapshot, &env.board_id, gap_id);
        return;
    }
    let is_version_name = matches!(
        &action.kind,
        OutlineActionKind::Update {
            edit: OutlineEdit::RenameVersion { .. }
        }
    );
    let edit = match evaluate_outline_edit(snapshot, &action, runtime.operation().0) {
        Ok(Some(edit)) => edit,
        Ok(None) => return,
        Err(msg) => {
            record_outline_error(ui, &action, is_version_name, Some(msg));
            return;
        }
    };
    if let OutlineActionKind::Delete { .. } = &action.kind {
        selected_context.set(Some(crate::objects::ScopedTreeContext {
            scope: env.scope.clone(),
            context: crate::objects::TreeContext::Outline {
                board_id: env.board_id.clone(),
            },
        }));
    }
    let tx_id = match &action.kind {
        OutlineActionKind::EditPerimeter { transaction_id, .. } => transaction_id.clone(),
        _ => String::new(),
    };
    let ids = target_ids(&action);
    let result = runtime.commit(
        "layout-outline",
        EditResolver::new("layout-outline", move |_| {
            Resolution::submit_with_transaction_id(
                ids.clone(),
                tx_id.clone(),
                EditOperation::ApplyOutlineEdit {
                    edit: Box::new(edit.clone()),
                },
            )
        }),
    );
    if is_version_name {
        let mut version_ui = ui.version_ui;
        let name = runtime
            .model()
            .accepted
            .as_ref()
            .and_then(|s| active_outline(s, &env.board_id).1)
            .map(|v| v.name.clone());
        let mut ver = version_ui.write();
        if let Some(name) = name {
            ver.version_name_draft = name;
        }
        ver.version_name_failure = result.err();
    } else {
        record_outline_error(ui, &action, false, result.err());
    }
}

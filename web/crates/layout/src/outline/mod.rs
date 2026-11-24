//! Outline planning, direct projection, and synchronous commit helpers.

pub use boardstudio_web_layout_core::outline::{
    self as planner, Admission, EditablePerimeter, OutlineAction, OutlineActionEnvelope,
    OutlineActionKind, OutlineDrawTool, OutlineFeedback, OutlinePointTarget, OutlineVersionChoice,
    OutlineVersionOwnership, PerimeterAnchor, active_outline, admission, attach_connection_point,
    blocked_export_labels, connection_point_world, editable_perimeter, gap_focus_camera,
    move_connection_point, outline_action_matches_saved_owner, outline_edit, outline_state,
    outline_version, retire_reason, settle, target_ids, version_geometry,
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
    AcceptedSnapshot, Durability, EditResolver, Event, Lifecycle, Scope,
};
#[cfg(target_arch = "wasm32")]
use boardstudio_model::model::{
    Contour, EditOperation, EditPhase, OutlineConnection, OutlineFeature, OutlineGap,
    OutlineRepairSettings, OutlineSettings, Part, Vec2,
};
#[cfg(target_arch = "wasm32")]
use boardstudio_web_ui_model::Workspace;
#[cfg(target_arch = "wasm32")]
use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, PartialEq)]
pub struct OutlineUiState {
    pub feedback: Signal<Option<OutlineFeedback>>,
    pub version_name_draft: Signal<String>,
    pub version_name_failure: Signal<Option<String>>,
    pub selected_point: Signal<usize>,
    pub editing_points: Signal<bool>,
    pub drawing_operation: Signal<Option<OutlineDrawTool>>,
    pub drawing_points: Signal<Vec<Vec2>>,
    pub selected_feature_id: Signal<Option<String>>,
    pub selected_connection_id: Signal<Option<String>>,
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
        self.active_version_id.as_ref().is_some_and(|version_id| {
            self.versions.iter().any(|choice| {
                &choice.id == version_id && choice.ownership == OutlineVersionOwnership::Linked
            })
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
    let captured_generation = scope_generation();
    let feedback = use_signal(|| None::<OutlineFeedback>);
    let version_name_draft = use_signal(String::new);
    let version_name_failure = use_signal(|| None::<String>);
    let selected_point = use_signal(|| 0usize);
    let editing_points = use_signal(|| false);
    let drawing_operation = use_signal(|| None::<OutlineDrawTool>);
    let drawing_points = use_signal(Vec::<Vec2>::new);
    let selected_feature_id = use_signal(|| None::<String>);
    let selected_connection_id = use_signal(|| None::<String>);
    let draft_owner = selected_context.read().clone();
    let draft_workspace = workspace();
    let model = runtime.model();
    let owner_board = draft_owner
        .as_ref()
        .and_then(|owner| board_id_for_context(&owner.context));
    let (draft_active_version, draft_version_name) = model
        .accepted
        .as_ref()
        .zip(owner_board)
        .map_or((None, String::new()), |(snapshot, board_id)| {
            let (state, active) = active_outline(snapshot, board_id);
            (
                state.and_then(|s| s.active_version_id.clone()),
                active.map(|v| v.name.clone()).unwrap_or_default(),
            )
        });
    let draft_scope = runtime.scope();
    use_effect(use_reactive(
        (
            &version,
            &draft_owner,
            &draft_workspace,
            &captured_generation,
        ),
        {
            let (mut drawing_operation, mut drawing_points) = (drawing_operation, drawing_points);
            move |_| {
                drawing_operation.set(None);
                drawing_points.set(Vec::new());
            }
        },
    ));
    use_effect(use_reactive(
        (
            &draft_owner,
            &draft_workspace,
            &draft_active_version,
            &draft_scope,
            &captured_generation,
        ),
        {
            let (mut feature_id, mut conn_id, mut draft, mut failure) = (
                selected_feature_id,
                selected_connection_id,
                version_name_draft,
                version_name_failure,
            );
            move |_| {
                feature_id.set(None);
                conn_id.set(None);
                draft.set(draft_version_name.clone());
                failure.set(None);
            }
        },
    ));
    OutlineUiState {
        feedback,
        version_name_draft,
        version_name_failure,
        selected_point,
        editing_points,
        drawing_operation,
        drawing_points,
        selected_feature_id,
        selected_connection_id,
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
    let captured_generation = scope_generation();
    let model = runtime.model();
    let scope = runtime.scope()?;
    if workspace() != Workspace::Layout || scope_generation() != captured_generation {
        return None;
    }
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
        .flat_map(|state| state.versions.iter())
        .map(|version| {
            let finding_id = format!("board:{board_id}:outline:linked:{}", version.id);
            let problem = snapshot.scene.findings.iter().find(|f| f.id == finding_id);
            OutlineVersionChoice {
                id: version.id.clone(),
                name: version.name.clone(),
                ownership: if version.ownership.is_fixed() {
                    OutlineVersionOwnership::Edited
                } else {
                    OutlineVersionOwnership::Linked
                },
                problem_finding_id: problem.map(|_| finding_id.clone()),
                problem_message: problem.map(|f| f.message.clone()),
                problem_selectable: snapshot
                    .scene
                    .finding_markers
                    .iter()
                    .any(|m| m.finding_id == finding_id && m.board_id == board_id),
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
        Some(active_id) => outline_version(outline, active_id)
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
    let outline_scene = snapshot
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
        ui.selected_feature_id.read().as_deref(),
    );
    let enabled = matches!(
        model.durability,
        Durability::Saved { .. } | Durability::Saving { .. }
    ) && model.lifecycle == Lifecycle::Ready
        && model.display_preview.is_none()
        && model.gesture.is_none();
    let feedback = ui
        .feedback
        .read()
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
        snap_paths: outline_scene
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
        gaps: outline_scene.map_or_else(Vec::new, |s| s.gaps.clone()),
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
    let Some(base_bounds) =
        crate::keycaps_fit::layout_canvas_bounds(&snapshot.document, &snapshot.scene, board_id)
    else {
        return;
    };
    let surface_size = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| {
            d.query_selector("[role='group'][aria-label^='Keyboard layout;'], [role='group'][aria-label^='Matrix placement canvas.']")
                .ok()
                .flatten()
        })
        .map(|s| {
            let b = s.get_bounding_client_rect();
            (b.width().max(1.0), b.height().max(1.0))
        })
        .unwrap_or((800.0, 600.0));
    if let Some((center, zoom)) =
        gap_focus_camera(snapshot, board_id, gap_id, surface_size, base_bounds)
    {
        runtime.submit(Event::SetCamera {
            operation_id: runtime.operation(),
            center,
            zoom,
        });
    }
}

#[cfg(target_arch = "wasm32")]
fn admit_outline_edit(
    runtime: &Rc<Runtime>,
    ui: OutlineUiState,
    action: &OutlineAction,
    is_version_name: bool,
) -> Option<boardstudio_core_kernel::outline_edit::OutlineEdit> {
    let snapshot = runtime.model().accepted?;
    let edit = outline_edit(&snapshot, action, runtime.operation().0)?;
    match admission(&snapshot, &edit) {
        Admission::Submit => Some(edit),
        Admission::Ignore => None,
        Admission::Malformed => {
            if let Err(message) =
                boardstudio_core_kernel::outline_edit::evaluate(snapshot.document.as_ref(), &edit)
            {
                let (scope, _, _, generation, board_id) = action.envelope();
                let (mut feedback, mut failure) = (ui.feedback, ui.version_name_failure);
                if is_version_name {
                    failure.set(Some(message));
                } else {
                    feedback.set(Some(OutlineFeedback {
                        scope: scope.clone(),
                        generation,
                        board_id: board_id.clone(),
                        message: Some(message),
                    }));
                }
            }
            None
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn preview_outline_perimeter(
    runtime: &Rc<Runtime>,
    ui: OutlineUiState,
    action: &OutlineAction,
    transaction_id: &str,
) -> Option<EditOperation> {
    if !outline_action_is_current(action, runtime, action.envelope().3) {
        return None;
    }
    let edit = admit_outline_edit(runtime, ui, action, false)?;
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
    let (action_scope, _, _, _, board_id) = action.envelope();
    let model = runtime.model();
    let Some(snapshot) = model.accepted.as_ref() else {
        return;
    };
    let selection_is_current = selected_context.read().as_ref().is_some_and(|selected| {
        selected.scope == *action_scope
            && board_id_for_context(&selected.context) == Some(board_id.as_str())
            && crate::selection::context_is_current(&model, action_scope, &selected.context)
    });
    if !selection_is_current {
        return;
    }
    if let OutlineActionKind::FocusGap { gap_id } = &action.kind {
        focus_outline_gap(runtime, snapshot, board_id, gap_id);
        return;
    }
    let is_version_name = matches!(
        &action.kind,
        OutlineActionKind::Update {
            edit: OutlineEdit::RenameVersion { .. },
        }
    );
    let Some(edit) = admit_outline_edit(runtime, ui, &action, is_version_name) else {
        return;
    };
    if let OutlineActionKind::Delete { .. } = &action.kind {
        selected_context.set(Some(crate::objects::ScopedTreeContext {
            scope: action_scope.clone(),
            context: crate::objects::TreeContext::Outline {
                board_id: board_id.clone(),
            },
        }));
    }
    if let OutlineActionKind::EditPerimeter {
        phase: EditPhase::Preview,
        transaction_id,
        ..
    } = &action.kind
    {
        runtime.submit(Event::PreviewEdit {
            operation_id: runtime.operation(),
            transaction_id: transaction_id.clone(),
            target_ids: target_ids(&action),
            operation: EditOperation::ApplyOutlineEdit {
                edit: Box::new(edit),
            },
        });
        return;
    }
    let transaction_id = match &action.kind {
        OutlineActionKind::EditPerimeter { transaction_id, .. } => transaction_id.clone(),
        _ => String::new(),
    };
    let (board_id, action_scope) = (board_id.clone(), action_scope.clone());
    let result = runtime.commit(
        "layout-outline",
        EditResolver::new("layout-outline", move |accepted: &AcceptedSnapshot| {
            settle(accepted, &edit, &action, transaction_id.clone())
        }),
    );
    let (mut feedback, mut version_name_draft, mut version_name_failure) =
        (ui.feedback, ui.version_name_draft, ui.version_name_failure);
    if is_version_name {
        if let Some(name) = runtime
            .model()
            .accepted
            .as_ref()
            .and_then(|s| active_outline(s, &board_id).1)
            .map(|v| v.name.clone())
        {
            version_name_draft.set(name);
        }
        version_name_failure.set(result.err());
    } else {
        feedback.set(result.err().map(|message| OutlineFeedback {
            scope: action_scope,
            generation: captured_generation,
            board_id,
            message: Some(message),
        }));
    }
}

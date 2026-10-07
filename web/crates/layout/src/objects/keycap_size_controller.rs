//! Accepted active-board projection and single edit owner for Layout key-size controls.
use super::{
    ScopedTreeContext, TreeContext,
    keycap_resize::{self, KeycapPlacement, KeycapResizeInput, ResizeAxis},
};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope, SnapshotToken,
};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase, ProjectDoc, Vec2};
use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
use dioxus::prelude::*;
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeySizeOwner {
    editor_instance_id: u64,
    context_generation: u64,
    scope_generation: u64,
    scope: Scope,
    context: TreeContext,
    selected_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeySizeItem {
    pub id: String,
    pub size: Vec2,
    pub pitch: Vec2,
    pub gap: Vec2,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeySizeProjection {
    pub owner: KeySizeOwner,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub items: Vec<KeySizeItem>,
    pub units: Vec2,
    pub mixed: bool,
    pub mixed_x: bool,
    pub mixed_y: bool,
    pub overlap_references: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeySizeRequest {
    pub owner: KeySizeOwner,
    pub request_id: u64,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub units: Vec2,
    pub axis: Option<ResizeAxis>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeySizeFeedback {
    pub owner: KeySizeOwner,
    pub request_id: u64,
    pub state: KeySizeState,
    pub message: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeySizeState {
    Pending,
    Saved,
    Failed,
}

#[derive(Clone, PartialEq)]
pub struct KeySizeMount {
    pub projection: Option<KeySizeProjection>,
    pub request_sequence: Signal<u64>,
    pub editable: bool,
    pub feedback: Option<KeySizeFeedback>,
    pub on_resize: EventHandler<KeySizeRequest>,
}

#[derive(Clone)]
struct PendingResize {
    request: KeySizeRequest,
    ticket: EditTicket,
}

/// Resolve a key-size change against the accepted document at execution time. The plan is
/// built from the accepted placements, so a resize queued behind other edits applies on
/// top of them; a selection that no longer exists retires, and a plan with nothing to
/// change resolves `Unchanged`.
fn resize_resolver(
    board_id: String,
    selected_ids: Vec<String>,
    units: Vec2,
    axis: Option<ResizeAxis>,
) -> EditResolver {
    EditResolver::new("layout-key-size", move |accepted: &AcceptedSnapshot| {
        let document = &accepted.document;
        let placements = placements(document, &accepted.scene.matrix_scenes, &board_id);
        if selected_ids
            .iter()
            .any(|id| !placements.iter().any(|placement| placement.id == *id))
        {
            return Resolution::Retire("A selected key no longer exists.".into());
        }
        let selected = selected_ids.iter().cloned().collect::<BTreeSet<_>>();
        let Some(plan) = keycap_resize::plan_resize(KeycapResizeInput {
            document,
            matrices: &document.matrices,
            scenes: &accepted.scene.matrix_scenes,
            layouts: &document.layouts,
            placements: &placements,
            selected_ids: &selected,
            units,
            axis,
        }) else {
            return Resolution::Unchanged;
        };
        Resolution::Submit(EditCommand {
            base_revision: 0,
            transaction_id: String::new(),
            phase: EditPhase::Commit,
            target_ids: plan.target_ids,
            operation: EditOperation::ReplaceDocument {
                document: Box::new(plan.document),
            },
        })
    })
}

#[derive(Default)]
struct ContextTracker {
    initialized: bool,
    identity: Option<(Scope, TreeContext, &'static str, u64)>,
    generation: u64,
}

pub fn use_key_size(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    selected_context: Signal<Option<ScopedTreeContext>>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
) -> KeySizeMount {
    let editor_instance_id = use_hook({
        let runtime = runtime.clone();
        move || runtime.operation().0
    });
    let tracker = use_hook(|| Rc::new(RefCell::new(ContextTracker::default())));
    let selected = selected_context.read().clone();
    let current_workspace = workspace();
    let current_scope_generation = scope_generation();
    let scope_context = selected.as_ref().map(|item| {
        (
            item.scope.clone(),
            item.context.clone(),
            current_workspace,
            current_scope_generation,
        )
    });
    let context_generation = {
        let mut state = tracker.borrow_mut();
        if !state.initialized || state.identity != scope_context {
            state.generation = state
                .generation
                .checked_add(1)
                .expect("key-size context generation exhausted");
            state.identity = scope_context.clone();
            state.initialized = true;
        }
        state.generation
    };
    let request_sequence = use_signal(|| 0u64);
    let last_request = use_signal(|| 0u64);
    let pending = use_signal(|| None::<PendingResize>);
    let feedback = use_signal(|| None::<KeySizeFeedback>);
    let (projection, editable) = project(
        &runtime,
        selected.as_ref(),
        editor_instance_id,
        context_generation,
        current_scope_generation,
        current_workspace,
    );

    use_effect(use_reactive(
        (&version(), &workspace(), &scope_generation()),
        {
            let runtime = runtime.clone();
            let mut pending = pending;
            let mut feedback = feedback;
            move |(_, _, generation)| settle(&runtime, generation, &mut pending, &mut feedback)
        },
    ));

    let on_resize = use_callback({
        let runtime = runtime.clone();
        let tracker = tracker.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        let mut last_request = last_request;
        move |request: KeySizeRequest| {
            let current_generation = tracker.borrow().generation;
            if request.request_id <= last_request()
                || request.owner.editor_instance_id != editor_instance_id
                || request.owner.context_generation != current_generation
                || request.owner.scope_generation != scope_generation()
                || workspace() != "Layout"
            {
                return;
            }
            last_request.set(request.request_id);
            let Some(current_selected) = selected_context.read().clone() else {
                return;
            };
            let model = runtime.model();
            let scope = runtime.scope();
            let Some((current, editable)) = project_for(
                &runtime,
                &model,
                scope.as_ref(),
                Some(&current_selected),
                ProjectionContext {
                    editor: editor_instance_id,
                    generation: current_generation,
                    scope_generation: scope_generation(),
                    workspace: workspace(),
                },
            ) else {
                return;
            };
            if current.owner != request.owner {
                return;
            }
            if !editable {
                feedback.set(Some(KeySizeFeedback { owner: request.owner.clone(), request_id: request.request_id, state: KeySizeState::Failed,
                    message: Some("The accepted layout is not ready to edit. Wait for it to save, then retry.".into()) }));
                return;
            }
            let ticket = EditTicket::begin(
                &runtime,
                "layout-key-size",
                Some("key size".into()),
                resize_resolver(
                    current_selected.scope.board_id.clone(),
                    current.owner.selected_ids.clone(),
                    request.units,
                    request.axis,
                ),
            );
            pending.set(Some(PendingResize {
                request: request.clone(),
                ticket,
            }));
            feedback.set(Some(KeySizeFeedback {
                owner: request.owner.clone(),
                request_id: request.request_id,
                state: KeySizeState::Pending,
                message: None,
            }));
        }
    });

    KeySizeMount {
        projection,
        request_sequence,
        editable,
        feedback: feedback.read().clone(),
        on_resize,
    }
}

fn project(
    runtime: &Runtime,
    selected: Option<&ScopedTreeContext>,
    editor: u64,
    generation: u64,
    scope_generation: u64,
    workspace: &'static str,
) -> (Option<KeySizeProjection>, bool) {
    if workspace != "Layout" {
        return (None, false);
    }
    let model = runtime.model();
    let scope = runtime.scope();
    project_for(
        runtime,
        &model,
        scope.as_ref(),
        selected,
        ProjectionContext {
            editor,
            generation,
            scope_generation,
            workspace,
        },
    )
    .map_or((None, false), |(projection, editable)| {
        (Some(projection), editable)
    })
}

struct ProjectionContext {
    editor: u64,
    generation: u64,
    scope_generation: u64,
    workspace: &'static str,
}

fn project_for(
    runtime: &Runtime,
    model: &boardstudio_application::ReadModel,
    live_scope: Option<&Scope>,
    selected: Option<&ScopedTreeContext>,
    context: ProjectionContext,
) -> Option<(KeySizeProjection, bool)> {
    let runtime_scope = runtime.scope();
    project_for_scope(model, live_scope, runtime_scope.as_ref(), selected, context)
}

fn project_for_scope(
    model: &boardstudio_application::ReadModel,
    live_scope: Option<&Scope>,
    runtime_scope: Option<&Scope>,
    selected: Option<&ScopedTreeContext>,
    context: ProjectionContext,
) -> Option<(KeySizeProjection, bool)> {
    let ProjectionContext {
        editor,
        generation,
        scope_generation,
        workspace,
    } = context;
    if workspace != "Layout" {
        return None;
    }
    let scope = live_scope?;
    let selected = selected?;
    if scope != &selected.scope || runtime_scope != Some(scope) {
        return None;
    }
    if !matches!(
        selected.context,
        TreeContext::Matrix { .. }
            | TreeContext::Row { .. }
            | TreeContext::Column { .. }
            | TreeContext::Key { .. }
    ) {
        return None;
    }
    let snapshot = model.accepted.as_ref()?;
    if snapshot.session_epoch != scope.session_epoch
        || snapshot.document.id != scope.document_id
        || model.active_board_id != scope.board_id
        || model.active_instance_id != scope.instance_id
    {
        return None;
    }
    // The tree context identifies the current Inspector section, but Ctrl/Shift canvas
    // selection can contain parts that are not members of the last hit context. Validate
    // that context independently and use Session's accepted selected IDs as the authority,
    // matching React's selectedKeycaps projection.
    super::resolve_selection(model, &selected.context)?;
    let placement_list = placements(
        &snapshot.document,
        &snapshot.scene.matrix_scenes,
        &scope.board_id,
    );
    let selected_set = selected_keycap_ids(&model.selected_part_ids, &placement_list);
    let overlap_references =
        overlap_references(&snapshot.document.parts, &placement_list, &selected_set);
    let items: Vec<_> = placement_list
        .iter()
        .filter(|placement| selected_set.contains(&placement.id))
        .filter_map(|placement| {
            let matrix = snapshot
                .document
                .matrices
                .iter()
                .find(|matrix| matrix.id == placement.matrix_id)?;
            Some(KeySizeItem {
                id: placement.id.clone(),
                size: placement.size,
                pitch: matrix.pitch,
                gap: matrix.edge_gap.unwrap_or(Vec2 { x: 1.0, y: 1.0 }),
            })
        })
        .collect();
    if items.is_empty() {
        return None;
    }
    let to_units = |item: &KeySizeItem| Vec2 {
        x: quarter((item.size.x + item.gap.x) / item.pitch.x),
        y: quarter((item.size.y + item.gap.y) / item.pitch.y),
    };
    let units = to_units(&items[0]);
    let unit_items: Vec<_> = items.iter().map(to_units).collect();
    let mixed_x = unit_items
        .iter()
        .skip(1)
        .any(|candidate| candidate.x != units.x);
    let mixed_y = unit_items
        .iter()
        .skip(1)
        .any(|candidate| candidate.y != units.y);
    let mixed = mixed_x || mixed_y;
    let owner = KeySizeOwner {
        editor_instance_id: editor,
        context_generation: generation,
        scope_generation,
        scope: scope.clone(),
        context: selected.context.clone(),
        selected_ids: items.iter().map(|item| item.id.clone()).collect(),
    };
    let editable = model.lifecycle == Lifecycle::Ready
        && model.durability
            == (Durability::Saved {
                revision: snapshot.document.revision,
            })
        && model.display_preview.is_none()
        && model.gesture.is_none();
    Some((
        KeySizeProjection {
            owner,
            snapshot_token: snapshot.token,
            revision: snapshot.document.revision,
            items,
            units,
            mixed,
            mixed_x,
            mixed_y,
            overlap_references,
        },
        editable,
    ))
}

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "keycap_size_tests.rs"]
mod tests;

fn placements(
    document: &ProjectDoc,
    scenes: &[boardstudio_core::model::MatrixScene],
    board_id: &str,
) -> Vec<KeycapPlacement> {
    let Some(board) = document.boards.iter().find(|board| board.id == board_id) else {
        return Vec::new();
    };
    let board_parts: BTreeSet<_> = board.part_ids.iter().map(String::as_str).collect();
    let mut result = Vec::new();
    for matrix in document
        .matrices
        .iter()
        .filter(|matrix| matrix.board_id.as_deref().is_none_or(|id| id == board_id))
    {
        let Some(scene) = scenes.iter().find(|scene| scene.matrix_id == matrix.id) else {
            continue;
        };
        for cell in scene.cells.iter().filter(|cell| cell.enabled) {
            let Some(id) = cell.member_id.as_deref() else {
                continue;
            };
            if !matrix.part_ids.iter().any(|member| member == id) || !board_parts.contains(id) {
                continue;
            }
            let Some(part) = document.parts.iter().find(|part| part.id == id) else {
                continue;
            };
            let definition = document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id);
            let gap = matrix.edge_gap.unwrap_or(Vec2 { x: 1.0, y: 1.0 });
            let size = part
                .keycap
                .or_else(|| definition.and_then(|definition| definition.keycap))
                .unwrap_or(Vec2 {
                    x: (matrix.pitch.x - gap.x).max(1.0),
                    y: (matrix.pitch.y - gap.y).max(1.0),
                });
            result.push(KeycapPlacement {
                id: id.to_owned(),
                matrix_id: matrix.id.clone(),
                row: cell.row,
                column: cell.column,
                at: cell.pose.at,
                rotation: cell.pose.rotation,
                size,
            });
        }
    }
    order_placements_by_document_parts(&document.parts, &mut result);
    result
}

fn order_placements_by_document_parts(
    parts: &[boardstudio_core::model::Part],
    placements: &mut [KeycapPlacement],
) {
    let order: std::collections::HashMap<_, _> = parts
        .iter()
        .enumerate()
        .map(|(index, part)| (part.id.as_str(), index))
        .collect();
    placements.sort_by_key(|placement| {
        order
            .get(placement.id.as_str())
            .copied()
            .unwrap_or(usize::MAX)
    });
}

fn selected_keycap_ids(
    selected_part_ids: &[String],
    placements: &[KeycapPlacement],
) -> BTreeSet<String> {
    let visible: BTreeSet<_> = placements
        .iter()
        .map(|placement| placement.id.as_str())
        .collect();
    selected_part_ids
        .iter()
        .filter(|id| visible.contains(id.as_str()))
        .cloned()
        .collect()
}

fn quarter(value: f64) -> f64 {
    (value * 4.0).round() / 4.0
}

fn overlap_references(
    parts: &[boardstudio_core::model::Part],
    placements: &[KeycapPlacement],
    selected_ids: &BTreeSet<String>,
) -> Vec<String> {
    let references: std::collections::HashMap<_, _> = parts
        .iter()
        .map(|part| (part.id.as_str(), part.reference.as_str()))
        .collect();
    let mut shown = BTreeSet::new();
    let mut result = Vec::new();
    for (first, second) in keycap_resize::overlapping_pairs(placements) {
        if !selected_ids.contains(&first) && !selected_ids.contains(&second) {
            continue;
        }
        for id in [first, second] {
            if let Some(reference) = references.get(id.as_str())
                && shown.insert(*reference)
            {
                result.push((*reference).to_owned());
            }
        }
    }
    result
}

fn settle(
    runtime: &Runtime,
    scope_generation: u64,
    pending: &mut Signal<Option<PendingResize>>,
    feedback: &mut Signal<Option<KeySizeFeedback>>,
) {
    let Some(waiting) = pending.read().clone() else {
        return;
    };
    let owner_is_live = runtime.scope().as_ref() == Some(&waiting.request.owner.scope)
        && scope_generation == waiting.request.owner.scope_generation;
    let (state, message) = match waiting.ticket.settlement(owner_is_live) {
        Settlement::Pending => return,
        Settlement::Landed { .. } => (KeySizeState::Saved, None),
        Settlement::Failed { message } => (KeySizeState::Failed, Some(message)),
        Settlement::Retired => {
            pending.set(None);
            feedback.set(None);
            return;
        }
    };
    pending.set(None);
    feedback.set(Some(KeySizeFeedback {
        owner: waiting.request.owner.clone(),
        request_id: waiting.request.request_id,
        state,
        message,
    }));
}

//! Accepted active-board projection and single edit owner for Layout key-size controls.
use super::keycap_size_logic::{
    overlap_references, placements, quarter, resize_resolver_for, selected_keycap_ids,
};
use super::{ScopedTreeContext, TreeContext, keycap_resize::ResizeAxis};
use crate::runtime::Runtime;
use boardstudio_application::{Durability, Lifecycle, Scope, SnapshotToken};
use boardstudio_model::model::Vec2;
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};

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
    pub field: KeySizeField,
    pub message: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeySizeField {
    Width,
    Height,
    Orientation,
}

#[derive(Clone, PartialEq)]
pub struct KeySizeMount {
    pub projection: Option<KeySizeProjection>,
    pub request_sequence: Signal<u64>,
    pub editable: bool,
    pub feedback: Option<KeySizeFeedback>,
    pub on_resize: EventHandler<KeySizeRequest>,
}

#[derive(Default)]
struct ContextTracker {
    initialized: bool,
    identity: Option<(Scope, TreeContext, &'static str, u64)>,
    generation: u64,
}

#[derive(Clone, PartialEq)]
struct KeySizeProjectionIdentity {
    accepted_token: Option<SnapshotToken>,
    live_scope: Option<Scope>,
    active_board_id: String,
    active_instance_id: Option<String>,
    selected_ids: Vec<String>,
    lifecycle: Lifecycle,
    durability: Durability,
    has_preview: bool,
    has_gesture: bool,
    selected: Option<ScopedTreeContext>,
    editor_instance_id: u64,
    context_generation: u64,
    scope_generation: u64,
    workspace: &'static str,
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
    let _ = version();
    let feedback = use_signal(|| None::<KeySizeFeedback>);
    let model = runtime.model();
    let accepted_token = model.accepted.as_ref().map(|snapshot| snapshot.token);
    let live_scope = runtime.scope();
    let active_board_id = model.active_board_id.clone();
    let active_instance_id = model.active_instance_id.clone();
    let selected_ids = model.selected_part_ids.clone();
    let lifecycle = model.lifecycle;
    let durability = model.durability.clone();
    let has_preview = model.display_preview.is_some();
    let has_gesture = model.gesture.is_some();
    let projection_identity = KeySizeProjectionIdentity {
        accepted_token,
        live_scope,
        active_board_id,
        active_instance_id,
        selected_ids,
        lifecycle,
        durability,
        has_preview,
        has_gesture,
        selected,
        editor_instance_id,
        context_generation,
        scope_generation: current_scope_generation,
        workspace: current_workspace,
    };
    let (projection, editable) = use_memo(use_reactive((&projection_identity,), {
        let runtime = runtime.clone();
        move |(identity,)| {
            project(
                &runtime,
                identity.selected.as_ref(),
                identity.editor_instance_id,
                identity.context_generation,
                identity.scope_generation,
                identity.workspace,
            )
        }
    }))
    .read()
    .clone();
    let on_resize = use_callback({
        let runtime = runtime.clone();
        let tracker = tracker.clone();
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
                feedback.set(Some(KeySizeFeedback { owner: request.owner.clone(), request_id: request.request_id, field: field_for_axis(request.axis),
                    message: Some("The accepted layout is not ready to edit. Wait for it to save, then retry.".into()) }));
                return;
            }
            let Some(accepted) = model.accepted.as_ref() else {
                return;
            };
            let result = runtime.commit(
                "layout-key-size",
                resize_resolver_for(
                    &accepted.document,
                    current_selected.scope.board_id.clone(),
                    current.owner.selected_ids.clone(),
                    request.units,
                    request.axis,
                ),
            );
            feedback.set(result.err().map(|message| KeySizeFeedback {
                owner: request.owner,
                request_id: request.request_id,
                field: field_for_axis(request.axis),
                message: Some(message),
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
    if !crate::selection::context_is_current(model, scope, &selected.context) {
        return None;
    }
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
    // A key-size change is a field edit: it queues freely while an earlier edit is applying
    // or saving, so only an unrecoverable or unopened session makes the control unavailable.
    let editable = model.lifecycle == Lifecycle::Ready
        && matches!(
            model.durability,
            Durability::Saved { .. } | Durability::Saving { .. }
        )
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

fn field_for_axis(axis: Option<ResizeAxis>) -> KeySizeField {
    match axis {
        Some(ResizeAxis::X) => KeySizeField::Width,
        Some(ResizeAxis::Y) => KeySizeField::Height,
        None => KeySizeField::Orientation,
    }
}

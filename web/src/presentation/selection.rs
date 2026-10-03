use super::objects::{self, ScopedTreeContext, TreeContext, TreeSelectRequest};
use crate::runtime::Runtime;
use boardstudio_application::{Event, Scope, SelectionMode};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub(super) type Cleanup = Rc<dyn Fn()>;

#[derive(Clone)]
pub(super) struct SelectionAdapter {
    pub selected_context: Signal<Option<ScopedTreeContext>>,
    pub anchor_scope: Signal<Option<Scope>>,
    pub generation: Signal<u64>,
    pub cleanup: Rc<RefCell<Option<(u64, Cleanup)>>>,
    pub next_cleanup_id: Rc<Cell<u64>>,
}

impl SelectionAdapter {
    pub fn new(
        selected_context: Signal<Option<ScopedTreeContext>>,
        anchor_scope: Signal<Option<Scope>>,
        generation: Signal<u64>,
    ) -> Self {
        Self {
            selected_context,
            anchor_scope,
            generation,
            cleanup: Rc::new(RefCell::new(None)),
            next_cleanup_id: Rc::new(Cell::new(1)),
        }
    }
}

pub(super) struct ReentrancyReset(Rc<Cell<bool>>);

impl ReentrancyReset {
    pub fn enter(flag: Rc<Cell<bool>>) -> Option<Self> {
        if flag.replace(true) {
            None
        } else {
            Some(Self(flag))
        }
    }
}

impl Drop for ReentrancyReset {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

pub(super) fn live_board_ids(model: &boardstudio_application::ReadModel) -> Vec<String> {
    let Some(snapshot) = model.accepted.as_ref() else {
        return Vec::new();
    };
    let Some(board) = snapshot
        .document
        .boards
        .iter()
        .find(|board| board.id == model.active_board_id)
    else {
        return Vec::new();
    };
    board
        .part_ids
        .iter()
        .filter(|id| snapshot.document.parts.iter().any(|part| part.id == **id))
        .cloned()
        .collect()
}

pub(super) fn eligible_live_ids(model: &boardstudio_application::ReadModel) -> Vec<String> {
    live_board_ids(model)
        .into_iter()
        .filter(|id| objects::context_for_part(model, id).is_some())
        .collect()
}

pub(super) fn context_is_current(
    model: &boardstudio_application::ReadModel,
    scope: &Scope,
    context: &TreeContext,
) -> bool {
    model.active_board_id == scope.board_id
        && model.active_instance_id == scope.instance_id
        && model.accepted.as_ref().is_some_and(|snapshot| {
            snapshot.session_epoch == scope.session_epoch
                && snapshot.document.id == scope.document_id
        })
        && objects::resolve_selection(model, context).is_some()
}

pub(super) fn resolve_context(
    model: &boardstudio_application::ReadModel,
    context: &TreeContext,
) -> Option<Vec<String>> {
    objects::resolve_selection(model, context)
}

pub(super) fn submit_context(
    runtime: &Rc<Runtime>,
    adapter: &SelectionAdapter,
    request: TreeSelectRequest,
) {
    if runtime.scope().as_ref() != Some(&request.scope) {
        return;
    }
    let model = runtime.model();
    if !context_is_current(&model, &request.scope, &request.context) {
        return;
    }
    let Some(ids) = objects::resolve_selection(&model, &request.context) else {
        return;
    };
    let matrix_id = match &request.context {
        TreeContext::Matrix { matrix_id }
        | TreeContext::Row { matrix_id, .. }
        | TreeContext::Column { matrix_id, .. }
        | TreeContext::Key { matrix_id, .. } => Some(matrix_id.clone()),
        _ => None,
    };
    if let Some(matrix_id) = matrix_id {
        let target_part_id = match &request.context {
            TreeContext::Key { row, column, .. } => {
                Some(format!("matrix/{matrix_id}/r{row}c{column}"))
            }
            _ => model.selection_anchor_id.clone().filter(|anchor| {
                matches!(
                    objects::context_for_part(&model, anchor),
                    Some(TreeContext::Key { matrix_id: anchor_matrix, .. }) if anchor_matrix == matrix_id
                )
            }),
        };
        if let Some(target_part_id) = target_part_id
            && let Some(hit_context) = objects::context_for_part(&model, &target_part_id)
            && submit_matrix_cell_selection(
                runtime,
                adapter,
                &request.scope,
                (adapter.generation)(),
                MatrixCellSelection {
                    matrix_id,
                    target_part_id,
                    hit_context: &hit_context,
                    context: request.context.clone(),
                    mode: request.mode,
                },
            )
            .is_some()
        {
            return;
        }
    }
    let eligible: Vec<_> = eligible_live_ids(&model);
    let ids: Vec<_> = ids
        .into_iter()
        .filter(|id| eligible.iter().any(|candidate| candidate == id))
        .collect();
    let mut selected_context = adapter.selected_context;
    selected_context.set(Some(ScopedTreeContext {
        scope: request.scope.clone(),
        context: request.context,
    }));
    let mut anchor_scope = adapter.anchor_scope;
    anchor_scope.set(None);
    runtime.submit(Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: ids.clone(),
        range_part_ids: Vec::new(),
        mode: if ids.is_empty() {
            SelectionMode::Replace
        } else {
            request.mode
        },
    });
    if !ids.is_empty() && runtime.scope().as_ref() == Some(&request.scope) {
        let current = runtime.model();
        if current.selection_anchor_id.as_ref() == ids.first()
            && current
                .selection_anchor_id
                .as_ref()
                .is_some_and(|anchor| eligible_live_ids(&current).iter().any(|id| id == anchor))
        {
            anchor_scope.set(Some(request.scope));
        }
    }
}

pub(super) fn submit_canvas_selection(
    runtime: &Rc<Runtime>,
    adapter: &SelectionAdapter,
    scope: &Scope,
    generation: u64,
    context: TreeContext,
    mode: SelectionMode,
    range_ids: Vec<String>,
) -> Option<Vec<String>> {
    if runtime.scope().as_ref() != Some(scope) || (adapter.generation)() != generation {
        return None;
    }
    let model = runtime.model();
    if !context_is_current(&model, scope, &context) {
        return None;
    }
    let eligible = eligible_live_ids(&model);
    let ids: Vec<_> = objects::resolve_selection(&model, &context)?
        .into_iter()
        .filter(|id| eligible.iter().any(|candidate| candidate == id))
        .collect();
    let range_ids: Vec<_> = range_ids
        .into_iter()
        .filter(|id| eligible.iter().any(|candidate| candidate == id))
        .collect();
    let invalid_range_anchor = mode == SelectionMode::Range
        && ((adapter.anchor_scope)().as_ref() != Some(scope)
            || model
                .selection_anchor_id
                .as_ref()
                .is_none_or(|anchor| !eligible_live_ids(&model).iter().any(|id| id == anchor)));
    let effective_mode = if ids.is_empty() || invalid_range_anchor {
        SelectionMode::Replace
    } else {
        mode
    };
    let mut selected_context = adapter.selected_context;
    selected_context.set(Some(ScopedTreeContext {
        scope: scope.clone(),
        context,
    }));
    let mut anchor_scope = adapter.anchor_scope;
    if effective_mode != SelectionMode::Range {
        anchor_scope.set(None);
    }
    runtime.submit(Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: ids.clone(),
        range_part_ids: if effective_mode == SelectionMode::Range {
            range_ids
        } else {
            Vec::new()
        },
        mode: effective_mode,
    });
    if !ids.is_empty() && runtime.scope().as_ref() == Some(scope) {
        let current = runtime.model();
        if current.selection_anchor_id.as_ref().is_some_and(|anchor| {
            current.selection_anchor_id.as_ref() == ids.first()
                && eligible_live_ids(&current).iter().any(|id| id == anchor)
        }) {
            anchor_scope.set(Some(scope.clone()));
        }
    }
    Some(ids)
}

pub(super) struct MatrixCellSelection<'a> {
    pub matrix_id: String,
    pub target_part_id: String,
    pub hit_context: &'a TreeContext,
    pub context: TreeContext,
    pub mode: SelectionMode,
}

pub(super) fn submit_matrix_cell_selection(
    runtime: &Rc<Runtime>,
    adapter: &SelectionAdapter,
    scope: &Scope,
    generation: u64,
    request: MatrixCellSelection<'_>,
) -> Option<Vec<String>> {
    let MatrixCellSelection {
        matrix_id,
        target_part_id,
        hit_context,
        context,
        mode,
    } = request;
    if runtime.scope().as_ref() != Some(scope) || (adapter.generation)() != generation {
        return None;
    }
    let TreeContext::Key {
        matrix_id: hit_matrix_id,
        row,
        column,
    } = hit_context
    else {
        return None;
    };
    if hit_matrix_id != &matrix_id
        || target_part_id != format!("matrix/{matrix_id}/r{row}c{column}")
    {
        return None;
    }
    let model = runtime.model();
    if !context_is_current(&model, scope, &context) {
        return None;
    }
    let eligible = eligible_live_ids(&model);
    if !eligible.iter().any(|id| id == &target_part_id) {
        return None;
    }
    let range_anchor_is_current = mode != SelectionMode::Range
        || ((adapter.anchor_scope)().as_ref() == Some(scope)
            && model.selection_anchor_id.as_ref().is_some_and(|anchor| {
                eligible.iter().any(|id| id == anchor)
                    && matches!(
                        objects::context_for_part(&model, anchor),
                        Some(TreeContext::Key { matrix_id: anchor_matrix, .. })
                            if anchor_matrix == matrix_id
                    )
            }));
    let (context, mode) = if mode == SelectionMode::Range && range_anchor_is_current {
        (hit_context.clone(), SelectionMode::Range)
    } else if mode == SelectionMode::Range {
        (context, SelectionMode::Replace)
    } else {
        (context, mode)
    };
    let ids = objects::resolve_selection(&model, &context)?
        .into_iter()
        .filter(|id| eligible.iter().any(|candidate| candidate == id))
        .collect::<Vec<_>>();
    let mut selected_context = adapter.selected_context;
    selected_context.set(Some(ScopedTreeContext {
        scope: scope.clone(),
        context,
    }));
    runtime.submit(Event::SelectMatrixCell {
        operation_id: runtime.operation(),
        scope: scope.clone(),
        matrix_id,
        target_part_id,
        part_ids: ids.clone(),
        mode,
    });
    let mut anchor_scope = adapter.anchor_scope;
    anchor_scope.set(Some(scope.clone()));
    Some(ids)
}

pub(super) fn cancel_scoped_drag(
    runtime: &Rc<Runtime>,
    drag: &Rc<RefCell<Option<super::Drag>>>,
    svg: &Rc<RefCell<Option<web_sys::SvgElement>>>,
    expected_scope: Option<&Scope>,
) {
    let pending = drag.borrow_mut().take();
    if let Some(pending) = pending {
        if expected_scope.is_some_and(|scope| scope != &pending.scope) {
            *drag.borrow_mut() = Some(pending);
            return;
        }
        if let Some(element) = svg.borrow().as_ref() {
            let _ = element.release_pointer_capture(pending.pointer as i32);
        }
        cancel_drag_if_owned(runtime, &pending);
    }
}

pub(super) fn cancel_drag_if_owned(runtime: &Rc<Runtime>, drag: &super::Drag) {
    if !drag.active || drag.pan || !owns_session_gesture(runtime, drag) {
        return;
    }
    runtime.submit(Event::GestureCancel {
        pointer_id: drag.pointer,
    });
}

pub(super) fn owns_session_gesture(runtime: &Rc<Runtime>, drag: &super::Drag) -> bool {
    runtime.scope().as_ref() == Some(&drag.scope)
        && runtime.model().gesture.is_some_and(|gesture| {
            gesture.pointer_id == drag.pointer
                && Some(gesture.generation) == drag.gesture_generation
        })
}

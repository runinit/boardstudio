use super::*;

/// A pending splay-origin pick and the pointer state that fires it: with Ctrl
/// held the pick swings the keys instead of holding them still.
pub(crate) struct PendingSplayOriginPick<'a> {
    pub(crate) pending: &'a mut Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub(crate) inspector: &'a objects::MatrixTransformInspectorMount,
    pub(crate) point: Vec2,
    pub(crate) swing: bool,
}

pub(crate) fn apply_pending_splay_origin_pick(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
    pick: PendingSplayOriginPick<'_>,
) -> bool {
    let PendingSplayOriginPick {
        pending,
        inspector,
        point,
        swing,
    } = pick;
    let Some(picked_owner) = pending.peek().clone() else {
        return false;
    };
    pending.set(None);
    let selected_is_current = adapter
        .selected_context
        .read()
        .as_ref()
        .is_some_and(|selected| {
            selected.scope == picked_owner.scope
                && selected.context == picked_owner.context
                && matches!(&selected.context, objects::TreeContext::Column { .. })
                && selection::context_is_current(
                    &runtime.model(),
                    &selected.scope,
                    &selected.context,
                )
        });
    let inspector_is_current = inspector
        .projection()
        .is_some_and(|projection| projection.owner == picked_owner);
    if !layout_owner_is_current(runtime, workspace, adapter, owner)
        || !selected_is_current
        || !inspector_is_current
        || !inspector.pick_splay_origin(point, swing)
    {
        runtime.report("Column selection changed. Pick the splay origin again.");
    }
    true
}

/// A pending anchor-key pick and the picked key identity: the canvas hit
/// already carries the key, so unlike the splay pick there is no world point
/// to resolve.
pub(crate) struct PendingAnchorPick<'a> {
    pub(crate) pending: &'a mut Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub(crate) inspector: &'a objects::MatrixTransformInspectorMount,
    pub(crate) matrix_id: String,
    pub(crate) row: u32,
    pub(crate) column: u32,
}

pub(crate) fn apply_pending_anchor_pick(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
    pick: PendingAnchorPick<'_>,
) -> bool {
    let PendingAnchorPick {
        pending,
        inspector,
        matrix_id,
        row,
        column,
    } = pick;
    let Some(picked_owner) = pending.peek().clone() else {
        return false;
    };
    pending.set(None);
    let selected_is_current = adapter
        .selected_context
        .read()
        .as_ref()
        .is_some_and(|selected| {
            selected.scope == picked_owner.scope
                && selected.context == picked_owner.context
                && matches!(&selected.context, objects::TreeContext::Matrix { .. })
                && selection::context_is_current(
                    &runtime.model(),
                    &selected.scope,
                    &selected.context,
                )
        });
    let inspector_is_current = inspector
        .projection()
        .is_some_and(|projection| projection.owner == picked_owner);
    if !layout_owner_is_current(runtime, workspace, adapter, owner)
        || !selected_is_current
        || !inspector_is_current
        || !inspector.pick_matrix_anchor(matrix_id, row, column)
    {
        runtime.report("Matrix selection changed. Pick the anchor key again.");
    }
    true
}

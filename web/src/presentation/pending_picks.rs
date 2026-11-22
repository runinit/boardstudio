use super::*;

pub(crate) struct PendingSplayOriginPick<'a> {
    pub(crate) pending: &'a mut Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub(crate) inspector: &'a objects::MatrixTransformInspectorMount,
    pub(crate) point: Vec2,
    pub(crate) swing: bool,
}

pub(crate) struct PendingAnchorPick<'a> {
    pub(crate) pending: &'a mut Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub(crate) inspector: &'a objects::MatrixTransformInspectorMount,
    pub(crate) matrix_id: String,
    pub(crate) row: u32,
    pub(crate) column: u32,
}

fn pick_owner_valid(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
    picked: &objects::MatrixTransformInspectorOwner,
    inspector: &objects::MatrixTransformInspectorMount,
    is_kind: impl Fn(&objects::TreeContext) -> bool,
) -> bool {
    layout_owner_is_current(runtime, workspace, adapter, owner)
        && inspector
            .projection
            .as_ref()
            .is_some_and(|p| p.owner == *picked)
        && adapter.selected_context.read().as_ref().is_some_and(|s| {
            s.scope == picked.scope
                && s.context == picked.context
                && is_kind(&s.context)
                && selection::context_is_current(&runtime.model(), &s.scope, &s.context)
        })
}

pub(crate) fn apply_pending_splay_origin_pick(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
    pick: PendingSplayOriginPick<'_>,
) -> bool {
    let Some(picked) = pick.pending.peek().clone() else {
        return false;
    };
    pick.pending.set(None);
    if !pick_owner_valid(
        runtime,
        workspace,
        adapter,
        owner,
        &picked,
        pick.inspector,
        |c| matches!(c, objects::TreeContext::Column { .. }),
    ) || !pick.inspector.pick_splay_origin(pick.point, pick.swing)
    {
        runtime.report("Column selection changed. Pick the splay origin again.");
    }
    true
}

pub(crate) fn apply_pending_anchor_pick(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
    pick: PendingAnchorPick<'_>,
) -> bool {
    let Some(picked) = pick.pending.peek().clone() else {
        return false;
    };
    pick.pending.set(None);
    if !pick_owner_valid(
        runtime,
        workspace,
        adapter,
        owner,
        &picked,
        pick.inspector,
        |c| matches!(c, objects::TreeContext::Matrix { .. }),
    ) || !pick
        .inspector
        .pick_matrix_anchor(pick.matrix_id, pick.row, pick.column)
    {
        runtime.report("Matrix selection changed. Pick the anchor key again.");
    }
    true
}

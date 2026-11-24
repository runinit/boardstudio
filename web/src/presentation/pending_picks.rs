use super::*;
use boardstudio_web_ui_model::Workspace;

pub(crate) type PickOwner<'a> = (
    &'a Runtime,
    Signal<Workspace>,
    &'a SelectionAdapter,
    &'a LayoutOwnerIdentity,
);

fn pick_owner_valid(
    (runtime, workspace, adapter, owner): PickOwner<'_>,
    picked: &objects::MatrixTransformInspectorOwner,
    inspector: &objects::MatrixTransformInspectorMount,
    is_kind: impl Fn(&objects::TreeContext) -> bool,
) -> bool {
    layout_owner_is_current(runtime, workspace, adapter, owner)
        && inspector.projection().is_some_and(|p| p.owner == *picked)
        && adapter.selected_context.read().as_ref().is_some_and(|s| {
            s.scope == picked.scope
                && s.context == picked.context
                && is_kind(&s.context)
                && selection::context_is_current(&runtime.model(), &s.scope, &s.context)
        })
}

pub(crate) fn apply_pending_splay_origin_pick(
    owner_ctx: PickOwner<'_>,
    pending: &mut Signal<Option<objects::MatrixTransformInspectorOwner>>,
    inspector: &objects::MatrixTransformInspectorMount,
    (point, swing): (Vec2, bool),
) -> bool {
    let Some(picked) = pending.peek().clone() else {
        return false;
    };
    pending.set(None);
    let ok = pick_owner_valid(owner_ctx, &picked, inspector, |c| {
        matches!(c, objects::TreeContext::Column { .. })
    });
    if !ok || !inspector.pick_splay_origin(point, swing) {
        owner_ctx
            .0
            .report("Column selection changed. Pick the splay origin again.");
    }
    true
}

pub(crate) fn apply_pending_anchor_pick(
    owner_ctx: PickOwner<'_>,
    pending: &mut Signal<Option<objects::MatrixTransformInspectorOwner>>,
    inspector: &objects::MatrixTransformInspectorMount,
    (matrix_id, row, column): (String, u32, u32),
) -> bool {
    let Some(picked) = pending.peek().clone() else {
        return false;
    };
    pending.set(None);
    let ok = pick_owner_valid(owner_ctx, &picked, inspector, |c| {
        matches!(c, objects::TreeContext::Matrix { .. })
    });
    if !ok || !inspector.pick_matrix_anchor(matrix_id, row, column) {
        owner_ctx
            .0
            .report("Matrix selection changed. Pick the anchor key again.");
    }
    true
}

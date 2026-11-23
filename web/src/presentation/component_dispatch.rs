use super::*;
use boardstudio_web_ui_model::Workspace;

pub(crate) fn layout_component_inspector_owner_key(
    model: &ReadModel,
    workspace: Workspace,
    selected: Option<&objects::ScopedTreeContext>,
) -> Option<inspector::LayoutComponentInspectorOwnerKey> {
    if workspace != Workspace::Layout {
        return None;
    }
    let selected = selected?;
    let objects::TreeContext::Component {
        part_id: Some(part_id),
        matrix_id,
        assembly_id,
        ..
    } = &selected.context
    else {
        return None;
    };
    let snapshot = model.accepted.as_ref()?;
    if matrix_id.is_some()
        && assembly_id.is_some()
        && mirrored_half_projection(&snapshot.document, part_id).is_none()
    {
        return None;
    }
    Some(inspector::LayoutComponentInspectorOwnerKey {
        scope: Some(selected.scope.clone()),
        workspace,
        part_id: Some(part_id.clone()),
        selected_part_ids: model.selected_part_ids.clone(),
    })
}

pub(crate) fn layout_component_inspector_owner_is_current(
    runtime: &Runtime,
    workspace: Signal<Workspace>,
    adapter: &SelectionAdapter,
    layout_owner: &LayoutOwnerIdentity,
    lifetime: &inspector::LayoutComponentInspectorLifetime,
    owner: &inspector::LayoutComponentInspectorOwner,
) -> bool {
    if !layout_owner_is_current(runtime, workspace, adapter, layout_owner)
        || owner.scope_generation != (adapter.generation)()
        || owner.context_generation != lifetime.current_generation()
        || layout_owner.scope.as_ref() != Some(&owner.scope)
        || layout_owner.token != Some(owner.snapshot_token)
        || layout_owner.revision != Some(owner.revision)
    {
        return false;
    }
    let model = runtime.model();
    let Some(snap) = model.accepted.as_ref() else {
        return false;
    };
    if snap.token != owner.snapshot_token || snap.document.revision != owner.revision {
        return false;
    }
    let selected = adapter.selected_context.read();
    let Some(selected) = selected.as_ref() else {
        return false;
    };
    selected.scope == owner.scope
        && matches!(&selected.context, objects::TreeContext::Component { part_id: Some(pid), .. } if pid == &owner.part_id)
        && selection::context_is_current(&model, &owner.scope, &selected.context)
        && model.selected_part_ids == owner.selected_part_ids
        && model.selected_part_ids.first() == Some(&owner.part_id)
}

pub(crate) fn companion_placement_owner_is_current(
    runtime: &Runtime,
    workspace: Signal<Workspace>,
    adapter: &SelectionAdapter,
    owner: &inspector::CompanionPlacementOwner,
    pair_id: &str,
    submitted_targets: &[CompanionPlacementTarget],
) -> bool {
    let cur = current_layout_owner(runtime, workspace, adapter);
    if cur.workspace != Workspace::Layout
        || cur.scope.as_ref() != Some(&owner.scope)
        || cur.token != Some(owner.snapshot_token)
        || cur.revision != Some(owner.revision)
        || owner.context_generation != (adapter.generation)()
        || owner.scope_generation != (adapter.generation)()
    {
        return false;
    }
    let model = runtime.model();
    let Some(selected) = adapter.selected_context.read().clone() else {
        return false;
    };
    if selected != owner.context
        || model.selected_part_ids != owner.selected_part_ids
        || !selection::context_is_current(&model, &owner.scope, &owner.context.context)
    {
        return false;
    }
    let Some(proj) = companion_placement_projection(
        &model,
        Some(&selected),
        owner.context_generation,
        owner.scope_generation,
    ) else {
        return false;
    };
    if proj.pair_id != pair_id {
        return false;
    }
    let mut expected = proj.targets;
    if submitted_targets
        .iter()
        .any(|t| matches!(t, CompanionPlacementTarget::Pair))
    {
        expected.push(CompanionPlacementTarget::Pair);
    }
    sort_companion_targets(&mut expected);
    let mut submitted = submitted_targets.to_vec();
    sort_companion_targets(&mut submitted);
    expected == submitted
}

pub(crate) fn sort_companion_targets(targets: &mut Vec<CompanionPlacementTarget>) {
    targets.sort_by_key(|t| serde_json::to_string(t).unwrap_or_default());
    targets.dedup();
}

pub(super) fn dispatch_layout_component_inspector_action(
    runtime: &Rc<Runtime>,
    adapter: &SelectionAdapter,
    layout_owner: &LayoutOwnerIdentity,
    lifetime: &inspector::LayoutComponentInspectorLifetime,
    (mut workspace, mut inspect_open): (Signal<Workspace>, Signal<bool>),
    views: layout_component_edits::ComponentEditViews,
    action: inspector::LayoutComponentInspectorAction,
) {
    use inspector::LayoutComponentInspectorAction as A;
    use layout_component_edits::{self as edits, InspectorField as F};
    let action_owner = action.owner();
    if !layout_component_inspector_owner_is_current(
        runtime,
        workspace,
        adapter,
        layout_owner,
        lifetime,
        action_owner,
    ) {
        return;
    }
    let current = runtime.model();
    let Some(snapshot) = current.accepted.as_ref() else {
        return;
    };
    let Some(board) = snapshot
        .document
        .boards
        .iter()
        .find(|b| b.id == action_owner.scope.board_id)
    else {
        return;
    };
    if action_owner.selected_part_ids.first() != Some(&action_owner.part_id)
        || action_owner.selected_part_ids.iter().any(|id| {
            !board.part_ids.contains(id) || !snapshot.document.parts.iter().any(|p| p.id == *id)
        })
    {
        return;
    }
    let part_id = action_owner.part_id.clone();
    let submit = |field: F, resolver| {
        let label = match field {
            F::X => "layout-inspector-x",
            F::Y => "layout-inspector-y",
            F::Margin => "layout-inspector-outline",
            F::Layout => "layout-inspector-layout",
            F::Constraint => "layout-inspector-constraint",
            F::RemoveConstraint => "layout-inspector-remove-constraint",
            F::MirroredHalfState => "layout-mirrored-half-state",
            F::PartMirror => "layout-part-mirror",
            F::PartAnchorValue => "layout-part-anchor-value",
            F::PartAnchorFollow => "layout-part-anchor-follow",
            F::PartAnchorPick => "layout-part-anchor-pick",
            F::PartAnchorDetach => "layout-part-anchor-detach",
        };
        let res = runtime.commit(label, resolver);
        let mut error = views.error;
        let view = match field {
            F::X => Some(views.x),
            F::Y => Some(views.y),
            F::Margin => Some(views.margin),
            _ => None,
        };
        if let Some((mut draft, mut failure)) = view {
            let acc = res.as_ref().unwrap_or(snapshot);
            if let Some(p) = acc.document.parts.iter().find(|p| p.id == part_id) {
                let pos = if p.anchor.is_some() {
                    boardstudio_core::evaluated_part_pose(&acc.document, p).at
                } else {
                    p.pose.at
                };
                draft.set(match field {
                    F::X => format!("{:.2}", pos.x),
                    F::Y => format!("{:.2}", pos.y),
                    F::Margin => p
                        .outline
                        .as_ref()
                        .and_then(|o| o.margin)
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    _ => unreachable!(),
                });
            }
            failure.set(res.err());
            error.set(None);
        } else {
            error.set(res.err());
        }
    };
    // Each action maps to one inspector field plus its edit resolver; the single
    // `submit` call below applies whichever pair the guards below accept.
    let submitted = match action {
        A::SetPosition {
            owner, axis, value, ..
        } if value.is_finite() => Some((
            match axis {
                inspector::ComponentPositionAxis::X => F::X,
                inspector::ComponentPositionAxis::Y => F::Y,
            },
            edits::position_resolver(
                owner.part_id,
                owner.selected_part_ids,
                owner.scope.board_id,
                axis,
                value,
            ),
        )),
        A::AssignLayout { owner, layout_id } => {
            let known = layout_id.as_ref().is_none_or(|lid| {
                snapshot
                    .document
                    .layouts
                    .iter()
                    .any(|l| l.id == *lid && l.board_id == board.id)
            });
            let cur = snapshot
                .document
                .layouts
                .iter()
                .find(|l| l.board_id == board.id && l.part_ids.contains(&owner.part_id));
            (known && cur.map(|l| l.id.as_str()) != layout_id.as_deref()).then(|| {
                (
                    F::Layout,
                    edits::assign_layout_resolver(owner.part_id, owner.scope.board_id, layout_id),
                )
            })
        }
        A::SetOutline { owner, outline, .. } => {
            Some((F::Margin, edits::outline_resolver(owner.part_id, outline)))
        }
        A::SetConstraint {
            owner,
            source_part_id,
            values,
        } if source_part_id != owner.part_id && board.part_ids.contains(&source_part_id) => Some((
            F::Constraint,
            edits::constraint_resolver(
                owner.part_id,
                owner.scope.board_id,
                source_part_id,
                values,
                runtime.operation().0,
            ),
        )),
        A::ConvertOffsetAxes {
            owner,
            constraint_id,
            to,
        } => Some((
            F::Constraint,
            edits::offset_axes_resolver(owner.part_id, constraint_id, to),
        )),
        A::RemoveConstraint {
            owner,
            constraint_id,
        } => Some((
            F::RemoveConstraint,
            edits::remove_constraint_resolver(owner.part_id, constraint_id),
        )),
        A::NavigateElectrical { .. } => {
            workspace.set(Workspace::Pcb);
            inspect_open.set(true);
            None
        }
        A::SetMirroredHalfState {
            pair_id,
            mirror_id,
            state,
            ..
        } => Some((
            F::MirroredHalfState,
            edits::mirrored_half_state_resolver(pair_id, mirror_id, state),
        )),
        A::SetPartMirrored {
            part_id, mirrored, ..
        } => Some((
            F::PartMirror,
            edits::part_mirror_resolver(part_id, mirrored),
        )),
        A::SetPartAnchorValue { owner, change } if part_anchor_value_finite(&change) => Some((
            F::PartAnchorValue,
            edits::part_anchor_value_resolver(owner.part_id, change),
        )),
        A::SetPartAnchorFollow { owner, follow } => Some((
            F::PartAnchorFollow,
            edits::part_anchor_follow_resolver(owner.part_id, follow),
        )),
        A::SetPartAnchor {
            owner,
            matrix_id,
            row,
            column,
            follow,
        } => Some((
            F::PartAnchorPick,
            edits::part_anchor_pick_resolver(owner.part_id, matrix_id, row, column, follow),
        )),
        A::DetachPartAnchor { owner } => Some((
            F::PartAnchorDetach,
            edits::part_anchor_detach_resolver(owner.part_id),
        )),
        _ => None,
    };
    if let Some((field, resolver)) = submitted {
        submit(field, resolver);
    }
}

pub(crate) fn part_anchor_value_finite(
    change: &boardstudio_core::model::PartAnchorValueChange,
) -> bool {
    use boardstudio_core::model::PartAnchorValueChange as C;
    match change {
        C::Spread(v) | C::Stagger(v) | C::Splay(v) | C::Rotation(v) => v.is_finite(),
        C::Offset(o) => o.x.is_finite() && o.y.is_finite(),
    }
}

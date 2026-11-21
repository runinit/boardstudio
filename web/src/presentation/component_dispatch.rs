use super::*;

pub(crate) fn layout_component_inspector_owner_key(
    model: &ReadModel,
    workspace: &'static str,
    selected: Option<&objects::ScopedTreeContext>,
) -> Option<inspector::LayoutComponentInspectorOwnerKey> {
    if workspace != "Layout" {
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
    workspace: Signal<&'static str>,
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
    let Some(snapshot) = model.accepted.as_ref() else {
        return false;
    };
    if snapshot.token != owner.snapshot_token || snapshot.document.revision != owner.revision {
        return false;
    }
    let selected = adapter.selected_context.read();
    let Some(selected) = selected.as_ref() else {
        return false;
    };
    selected.scope == owner.scope
        && matches!(
            &selected.context,
            objects::TreeContext::Component {
                part_id: Some(part_id),
                ..
            } if part_id == &owner.part_id
        )
        && selection::context_is_current(&model, &owner.scope, &selected.context)
        && model.selected_part_ids == owner.selected_part_ids
        && model.selected_part_ids.first() == Some(&owner.part_id)
}

pub(crate) fn companion_placement_owner_is_current(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    owner: &inspector::CompanionPlacementOwner,
    pair_id: &str,
    submitted_targets: &[CompanionPlacementTarget],
) -> bool {
    let current_owner = current_layout_owner(runtime, workspace, adapter);
    if current_owner.workspace != "Layout"
        || current_owner.scope.as_ref() != Some(&owner.scope)
        || current_owner.token != Some(owner.snapshot_token)
        || current_owner.revision != Some(owner.revision)
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
    let Some(projection) = companion_placement_projection(
        &model,
        Some(&selected),
        owner.context_generation,
        owner.scope_generation,
    ) else {
        return false;
    };
    if projection.pair_id != pair_id {
        return false;
    }
    let mut expected = projection.targets;
    if submitted_targets
        .iter()
        .any(|target| matches!(target, CompanionPlacementTarget::Pair))
    {
        expected.push(CompanionPlacementTarget::Pair);
    }
    sort_companion_targets(&mut expected);
    let mut submitted = submitted_targets.to_vec();
    sort_companion_targets(&mut submitted);
    expected == submitted
}

pub(crate) fn sort_companion_targets(targets: &mut Vec<CompanionPlacementTarget>) {
    targets.sort_by_key(|target| serde_json::to_string(target).unwrap_or_default());
    targets.dedup();
}

/// Begin the pending edit for an Inspector field and park it on its field of the
/// Inspector's pending-edit state, so the form renders the draft while it is pending and
/// the accepted value again when it settles.
pub(super) fn submit_layout_component_edit(
    runtime: &Rc<Runtime>,
    pending_edits: Signal<layout_component_edits::LayoutComponentInspectorEdits>,
    field: layout_component_edits::InspectorField,
    resolver: boardstudio_application::EditResolver,
    display: boardstudio_web_ui_shared::draft_edits::EditDisplay<String>,
) {
    let helper = pending_edits.peek().clone();
    let label = match field {
        layout_component_edits::InspectorField::X => "layout-inspector-x",
        layout_component_edits::InspectorField::Y => "layout-inspector-y",
        layout_component_edits::InspectorField::Margin => "layout-inspector-outline",
        layout_component_edits::InspectorField::Layout => "layout-inspector-layout",
        layout_component_edits::InspectorField::Constraint => "layout-inspector-constraint",
        layout_component_edits::InspectorField::RemoveConstraint => {
            "layout-inspector-remove-constraint"
        }
        layout_component_edits::InspectorField::MirroredHalfState => "layout-mirrored-half-state",
        layout_component_edits::InspectorField::PartMirror => "layout-part-mirror",
        layout_component_edits::InspectorField::CompanionPlacement => "layout-companion-placement",
        layout_component_edits::InspectorField::PartAnchorValue => "layout-part-anchor-value",
        layout_component_edits::InspectorField::PartAnchorFollow => "layout-part-anchor-follow",
        layout_component_edits::InspectorField::PartAnchorPick => "layout-part-anchor-pick",
        layout_component_edits::InspectorField::PartAnchorDetach => "layout-part-anchor-detach",
    };
    helper.submit(
        runtime,
        field,
        label,
        Some("layout".into()),
        resolver,
        display,
    );
}

pub(super) fn dispatch_layout_component_inspector_action(
    runtime: &Rc<Runtime>,
    adapter: &SelectionAdapter,
    layout_owner: &LayoutOwnerIdentity,
    lifetime: &inspector::LayoutComponentInspectorLifetime,
    (mut workspace, mut inspect_open): (Signal<&'static str>, Signal<bool>),
    (pending_edits, views): (
        Signal<layout_component_edits::LayoutComponentInspectorEdits>,
        layout_component_edits::ComponentEditViews,
    ),
    action: inspector::LayoutComponentInspectorAction,
) {
    let action_owner = match &action {
        inspector::LayoutComponentInspectorAction::SetPosition { owner, .. }
        | inspector::LayoutComponentInspectorAction::AssignLayout { owner, .. }
        | inspector::LayoutComponentInspectorAction::SetOutline { owner, .. }
        | inspector::LayoutComponentInspectorAction::SetConstraint { owner, .. }
        | inspector::LayoutComponentInspectorAction::RemoveConstraint { owner, .. }
        | inspector::LayoutComponentInspectorAction::ConvertOffsetAxes { owner, .. }
        | inspector::LayoutComponentInspectorAction::NavigateElectrical { owner }
        | inspector::LayoutComponentInspectorAction::SetMirroredHalfState { owner, .. }
        | inspector::LayoutComponentInspectorAction::SetPartMirrored { owner, .. }
        | inspector::LayoutComponentInspectorAction::SetPartAnchorValue { owner, .. }
        | inspector::LayoutComponentInspectorAction::SetPartAnchorFollow { owner, .. }
        | inspector::LayoutComponentInspectorAction::SetPartAnchor { owner, .. }
        | inspector::LayoutComponentInspectorAction::DetachPartAnchor { owner } => owner,
    };
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
        .find(|board| board.id == action_owner.scope.board_id)
    else {
        return;
    };
    if action_owner.selected_part_ids.is_empty()
        || action_owner.selected_part_ids.first() != Some(&action_owner.part_id)
        || action_owner.selected_part_ids.iter().any(|selected_id| {
            !board.part_ids.iter().any(|part_id| part_id == selected_id)
                || !snapshot
                    .document
                    .parts
                    .iter()
                    .any(|part| part.id == *selected_id)
        })
    {
        return;
    }
    let submit = |field, resolver, draft| {
        submit_layout_component_edit(
            runtime,
            pending_edits,
            field,
            resolver,
            views.display(field, draft),
        );
    };
    match action {
        inspector::LayoutComponentInspectorAction::SetPosition {
            owner,
            axis,
            value,
            submitted_draft,
        } => {
            if !value.is_finite() {
                return;
            }
            let field = match axis {
                inspector::ComponentPositionAxis::X => layout_component_edits::InspectorField::X,
                inspector::ComponentPositionAxis::Y => layout_component_edits::InspectorField::Y,
            };
            submit(
                field,
                layout_component_edits::position_resolver(
                    owner.part_id.clone(),
                    owner.selected_part_ids.clone(),
                    owner.scope.board_id.clone(),
                    axis,
                    value,
                ),
                Some(submitted_draft),
            );
        }
        inspector::LayoutComponentInspectorAction::AssignLayout { owner, layout_id } => {
            if let Some(layout_id) = layout_id.as_ref()
                && !snapshot
                    .document
                    .layouts
                    .iter()
                    .any(|layout| layout.id == *layout_id && layout.board_id == board.id)
            {
                return;
            }
            let current_layout = snapshot.document.layouts.iter().find(|layout| {
                layout.board_id == board.id && layout.part_ids.contains(&owner.part_id)
            });
            if current_layout.map(|layout| layout.id.as_str()) == layout_id.as_deref() {
                return;
            }
            submit(
                layout_component_edits::InspectorField::Layout,
                layout_component_edits::assign_layout_resolver(
                    owner.part_id.clone(),
                    owner.scope.board_id.clone(),
                    layout_id,
                ),
                None,
            );
        }
        inspector::LayoutComponentInspectorAction::SetOutline {
            owner,
            outline,
            submitted_draft,
        } => {
            submit(
                layout_component_edits::InspectorField::Margin,
                layout_component_edits::outline_resolver(owner.part_id.clone(), outline),
                Some(submitted_draft),
            );
        }
        inspector::LayoutComponentInspectorAction::SetConstraint {
            owner,
            source_part_id,
            values,
        } => {
            if source_part_id == owner.part_id
                || !board.part_ids.iter().any(|id| id == &source_part_id)
            {
                return;
            }
            let id_seed = runtime.operation().0;
            submit(
                layout_component_edits::InspectorField::Constraint,
                layout_component_edits::constraint_resolver(
                    owner.part_id.clone(),
                    owner.scope.board_id.clone(),
                    source_part_id,
                    values,
                    id_seed,
                ),
                None,
            );
        }
        inspector::LayoutComponentInspectorAction::ConvertOffsetAxes {
            owner,
            constraint_id,
            to,
        } => {
            submit(
                layout_component_edits::InspectorField::Constraint,
                layout_component_edits::offset_axes_resolver(
                    owner.part_id.clone(),
                    constraint_id,
                    to,
                ),
                None,
            );
        }
        inspector::LayoutComponentInspectorAction::RemoveConstraint {
            owner,
            constraint_id,
        } => {
            submit(
                layout_component_edits::InspectorField::RemoveConstraint,
                layout_component_edits::remove_constraint_resolver(
                    owner.part_id.clone(),
                    constraint_id,
                ),
                None,
            );
        }
        inspector::LayoutComponentInspectorAction::NavigateElectrical { .. } => {
            workspace.set("PCB");
            inspect_open.set(true);
        }
        inspector::LayoutComponentInspectorAction::SetMirroredHalfState {
            owner: _,
            pair_id,
            mirror_id,
            state,
        } => {
            submit(
                layout_component_edits::InspectorField::MirroredHalfState,
                layout_component_edits::mirrored_half_state_resolver(pair_id, mirror_id, state),
                None,
            );
        }
        inspector::LayoutComponentInspectorAction::SetPartMirrored {
            owner: _,
            part_id,
            mirrored,
        } => {
            submit(
                layout_component_edits::InspectorField::PartMirror,
                layout_component_edits::part_mirror_resolver(part_id, mirrored),
                None,
            );
        }
        inspector::LayoutComponentInspectorAction::SetPartAnchorValue { owner, change } => {
            if !part_anchor_value_finite(&change) {
                return;
            }
            submit(
                layout_component_edits::InspectorField::PartAnchorValue,
                layout_component_edits::part_anchor_value_resolver(owner.part_id, change),
                None,
            );
        }
        inspector::LayoutComponentInspectorAction::SetPartAnchorFollow { owner, follow } => {
            submit(
                layout_component_edits::InspectorField::PartAnchorFollow,
                layout_component_edits::part_anchor_follow_resolver(owner.part_id, follow),
                None,
            );
        }
        inspector::LayoutComponentInspectorAction::SetPartAnchor {
            owner,
            matrix_id,
            row,
            column,
            follow,
        } => {
            submit(
                layout_component_edits::InspectorField::PartAnchorPick,
                layout_component_edits::part_anchor_pick_resolver(
                    owner.part_id,
                    matrix_id,
                    row,
                    column,
                    follow,
                ),
                None,
            );
        }
        inspector::LayoutComponentInspectorAction::DetachPartAnchor { owner } => {
            submit(
                layout_component_edits::InspectorField::PartAnchorDetach,
                layout_component_edits::part_anchor_detach_resolver(owner.part_id),
                None,
            );
        }
    }
}

/// Ticket 13: finite check for anchor value edits; Follow, Pick and Detach
/// carry no floats.
pub(crate) fn part_anchor_value_finite(
    change: &boardstudio_core::model::PartAnchorValueChange,
) -> bool {
    use boardstudio_core::model::PartAnchorValueChange as Change;
    match change {
        Change::Spread(value)
        | Change::Stagger(value)
        | Change::Splay(value)
        | Change::Rotation(value) => value.is_finite(),
        Change::Offset(offset) => offset.x.is_finite() && offset.y.is_finite(),
    }
}

use super::*;

pub(super) fn project_current(
    runtime: &Runtime,
    selected: Option<&ScopedTreeContext>,
    editor_instance_id: u64,
    context_generation: u64,
    scope_generation: u64,
    workspace: &'static str,
    owner_workspace: &'static str,
) -> (Option<MatrixTransformProjection>, bool) {
    if !workspace_route_matches(workspace, owner_workspace) {
        return (None, false);
    }
    let model = runtime.model();
    let scope = runtime.scope();
    project_current_for(
        runtime,
        &model,
        scope.as_ref(),
        selected,
        ProjectionContext {
            editor_instance_id,
            context_generation,
            scope_generation,
            workspace,
        },
        owner_workspace,
    )
    .map_or((None, false), |(projection, editable)| {
        (Some(projection), editable)
    })
}

pub(super) struct ProjectionContext {
    pub(super) editor_instance_id: u64,
    pub(super) context_generation: u64,
    pub(super) scope_generation: u64,
    pub(super) workspace: &'static str,
}

pub(super) fn project_current_for(
    runtime: &Runtime,
    model: &boardstudio_application::ReadModel,
    live_scope: Option<&Scope>,
    selected: Option<&ScopedTreeContext>,
    context: ProjectionContext,
    owner_workspace: &'static str,
) -> Option<(MatrixTransformProjection, bool)> {
    if !workspace_route_matches(context.workspace, owner_workspace) {
        return None;
    }
    let scope = live_scope?;
    let selected = selected?;
    if &selected.scope != scope || runtime.scope().as_ref() != Some(scope) {
        return None;
    }
    let matrix_id = match &selected.context {
        TreeContext::Matrix { matrix_id }
        | TreeContext::Row { matrix_id, .. }
        | TreeContext::Column { matrix_id, .. }
        | TreeContext::Key { matrix_id, .. } => matrix_id,
        _ => return None,
    };
    let snapshot = model.accepted.as_ref()?;
    if snapshot.session_epoch != scope.session_epoch
        || snapshot.document.id != scope.document_id
        || model.active_board_id != scope.board_id
        || model.active_instance_id != scope.instance_id
        || super::super::resolve_selection(model, &selected.context).is_none()
    {
        return None;
    }
    let matrix = snapshot
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == *matrix_id)?;
    let fields = transform_fields(snapshot, &scope.board_id, &selected.context, matrix)?;
    // Transform edits queue behind each other: the inspector stays editable while an earlier
    // edit is applying or saving.
    let editable = matches!(
        model.lifecycle,
        Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
    ) && matches!(
        model.durability,
        Durability::Saved { .. } | Durability::Saving { .. }
    ) && model.display_preview.is_none()
        && model.gesture.is_none();
    Some((
        MatrixTransformProjection {
            owner: MatrixTransformInspectorOwner {
                editor_instance_id: context.editor_instance_id,
                workspace: context.workspace,
                context_generation: context.context_generation,
                scope_generation: context.scope_generation,
                scope: scope.clone(),
                matrix_id: matrix.id.clone(),
                context: selected.context.clone(),
            },
            snapshot_token: snapshot.token,
            revision: snapshot.document.revision,
            label: super::super::context_label(model, &selected.context)
                .unwrap_or_else(|| format!("Matrix {}", matrix.id)),
            fields,
        },
        editable,
    ))
}

/// The field projection of the selected row, column or key from an accepted snapshot. The
/// Inspector renders it and the edit resolver reads it again at execution time.
pub(super) fn transform_fields(
    snapshot: &AcceptedSnapshot,
    board_id: &str,
    context: &TreeContext,
    matrix: &Matrix,
) -> Option<MatrixTransformFields> {
    Some(match context {
        TreeContext::Matrix { .. } => MatrixTransformFields::Matrix {
            origin: matrix.origin,
            rotation: matrix.rotation.unwrap_or(0.0),
            mirror: matrix.mirror,
            mirror_y_locked: matrix_has_linked_layout(&snapshot.document, board_id, &matrix.id),
            rotation_followed: matrix
                .anchor
                .as_ref()
                .is_none_or(|anchor| anchor.follow.rotation),
            anchor: matrix.anchor.clone(),
        },
        TreeContext::Row { row, .. } => MatrixTransformFields::Row {
            row: *row,
            offset: matrix
                .row_layout
                .get(*row as usize)
                .map(|record| &record.offset)
                .copied()
                .unwrap_or(Vec2 { x: 0.0, y: 0.0 }),
            shift: matrix
                .row_layout
                .get(*row as usize)
                .map(|record| &record.shift)
                .copied()
                .unwrap_or(Vec2 { x: 0.0, y: 0.0 }),
            bend: matrix
                .row_layout
                .get(*row as usize)
                .map(|record| &record.bend)
                .copied()
                .unwrap_or(0.0),
            padding: matrix
                .row_layout
                .get(*row as usize)
                .and_then(|record| record.padding),
            pitch: matrix.pitch.y,
        },
        TreeContext::Column { column, .. } => {
            let scene = snapshot
                .scene
                .matrix_scenes
                .iter()
                .find(|scene| scene.matrix_id == matrix.id)?;
            let basis = scene.columns.iter().find(|basis| basis.column == *column)?;
            MatrixTransformFields::Column {
                column: *column,
                offset: matrix
                    .column_layout
                    .get(*column as usize)
                    .map(|record| &record.offset)
                    .copied()
                    .unwrap_or(Vec2 { x: 0.0, y: 0.0 }),
                stagger: matrix
                    .column_layout
                    .get(*column as usize)
                    .map(|record| &record.stagger)
                    .copied()
                    .unwrap_or(0.0),
                spread: matrix
                    .column_layout
                    .get(*column as usize)
                    .and_then(|record| record.spread),
                pitch: matrix.pitch.x,
                splay_angle: basis.splay_angle,
                rotation: basis.rotation,
                splay_origin: basis.splay_origin,
                custom_origin: basis.custom_origin,
                convert: convert_offset_available(matrix, *column),
            }
        }
        TreeContext::Key { row, column, .. } => {
            let scene = snapshot
                .scene
                .matrix_scenes
                .iter()
                .find(|scene| scene.matrix_id == matrix.id)?;
            let scene_cell = scene
                .cells
                .iter()
                .find(|cell| cell.row == *row && cell.column == *column);
            // Ticket 34: an omitted key left the scene cell behind; its
            // Omitted overlay still names the canonical member for the Key
            // context, so the rejoin affordance keeps working.
            let member_id = scene_cell
                .and_then(|cell| cell.member_id.clone())
                .or_else(|| {
                    key_half_exception_omitted(&snapshot.document, &matrix.id, *row, *column)
                        .map(|exception| exception.mirror_id)
                });
            let cell = matrix
                .cells
                .iter()
                .find(|cell| cell.row == *row && cell.column == *column);
            // Ticket 34: per-key Omit/Rejoin is owned by the Key context. The
            // member resolves here; the pair and overlay re-resolve at execution.
            let half_exception = member_id
                .as_deref()
                .and_then(|member| key_half_exception(&snapshot.document, member));
            MatrixTransformFields::Key {
                row: *row,
                column: *column,
                enabled: cell.is_none_or(|cell| cell.enabled),
                definition_id: cell
                    .and_then(|cell| cell.definition_id.clone())
                    .unwrap_or_else(|| matrix.definition_id.clone()),
                choices: Vec::new(),
                assemblies: cell
                    .map(|cell| {
                        cell.assemblies
                            .iter()
                            .map(|assembly| (assembly.id.clone(), assembly.definition_id.clone()))
                            .collect()
                    })
                    .unwrap_or_default(),
                component_choices: Vec::new(),
                mirror_target: snapshot.document.layouts.iter().any(|layout| {
                    layout.matrix_ids.iter().any(|id| id == &matrix.id)
                        && layout.mirror_link.is_some()
                }),
                assemblies_local: cell.and_then(|cell| cell.assemblies_local).unwrap_or(false),
                offset: cell
                    .and_then(|cell| cell.offset)
                    .unwrap_or(Vec2 { x: 0.0, y: 0.0 }),
                rotation: cell.and_then(|cell| cell.rotation).unwrap_or(0.0),
                shift: cell
                    .and_then(|cell| cell.shift)
                    .unwrap_or(Vec2 { x: 0.0, y: 0.0 }),
                bend: cell.and_then(|cell| cell.bend).unwrap_or(0.0),
                half_exception,
            }
        }
        _ => return None,
    })
}

pub(super) fn matrix_has_linked_layout(
    document: &ProjectDoc,
    board_id: &str,
    matrix_id: &str,
) -> bool {
    document
        .layouts
        .iter()
        .find(|layout| {
            layout.board_id == board_id && layout.matrix_ids.iter().any(|id| *id == matrix_id)
        })
        .is_some_and(|layout| {
            layout.mirror_link.is_some()
                || document.layouts.iter().any(|other| {
                    other
                        .mirror_link
                        .as_ref()
                        .is_some_and(|link| link.source_id == layout.id)
                })
        })
}

pub(super) fn workspace_route_matches(current: &str, owner: &str) -> bool {
    matches!(owner, "Layout" | "PCB") && current == owner
}

#[cfg(all(test, target_arch = "wasm32"))]
mod workspace_route_tests {
    use super::workspace_route_matches;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn matrix_transform_owner_cannot_cross_workspace_routes() {
        assert!(workspace_route_matches("Layout", "Layout"));
        assert!(workspace_route_matches("PCB", "PCB"));
        assert!(!workspace_route_matches("PCB", "Layout"));
        assert!(!workspace_route_matches("Layout", "PCB"));
        assert!(!workspace_route_matches("Case", "Case"));
    }
}

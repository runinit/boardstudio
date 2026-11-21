use super::*;
pub(crate) use boardstudio_web_layout::companion_placement::companion_placement_projection;

/// Resolve a Relations reference against the live accepted document. A component opens
/// its Inspector context, as for a finding. A target that has gone, or a request for a
/// session that has moved on, resolves to `None` and nothing is selected.
pub(crate) fn relation_target_request(
    runtime: &Runtime,
    mut request: objects::TreeSelectRequest,
) -> Option<objects::TreeSelectRequest> {
    if runtime.scope().as_ref() != Some(&request.scope) {
        return None;
    }
    let model = runtime.model();
    if let objects::TreeContext::Component {
        part_id: Some(part_id),
        ..
    } = &request.context
    {
        request.context = objects::component_context_for_finding_part(&model, part_id)?;
    }
    if !selection::context_is_current(&model, &request.scope, &request.context) {
        return None;
    }
    // A removed key keeps a resolvable (empty) context, so check the cell itself.
    if let objects::TreeContext::Key {
        matrix_id,
        row,
        column,
    } = &request.context
    {
        let document = &model.accepted.as_ref()?.document;
        let removed = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == *matrix_id)?
            .cells
            .iter()
            .any(|cell| cell.row == *row && cell.column == *column && cell.deleted);
        if removed {
            return None;
        }
    }
    Some(request)
}

/// The Relations projection for the selected matrix context, from the accepted document and
/// Core's matrix scene. `None` when nothing is selected, the selection is stale, or the
/// context names no live object of a matrix.
pub(crate) fn matrix_context_relations(
    model: &ReadModel,
    selected: Option<&objects::ScopedTreeContext>,
) -> Option<boardstudio_web_layout::matrix_relations::MatrixRelations> {
    let selected = selected?;
    if !selection::context_is_current(model, &selected.scope, &selected.context) {
        return None;
    }
    let matrix_id = match &selected.context {
        objects::TreeContext::Matrix { matrix_id }
        | objects::TreeContext::Row { matrix_id, .. }
        | objects::TreeContext::Column { matrix_id, .. }
        | objects::TreeContext::Key { matrix_id, .. } => matrix_id,
        objects::TreeContext::Component {
            matrix_id: Some(matrix_id),
            ..
        } => matrix_id,
        _ => return None,
    };
    let snapshot = model.accepted.as_ref()?;
    let scene = snapshot
        .scene
        .matrix_scenes
        .iter()
        .find(|scene| scene.matrix_id == *matrix_id)?;
    let mut relations = boardstudio_web_layout::matrix_relations::matrix_relations(
        &snapshot.document,
        scene,
        &selected.context,
        &snapshot.scene.matrix_scenes,
    )?;
    // The chain covers one matrix context only: selected parts in other
    // matrices get an explanatory note, never a merged frame.
    relations.selection_note = boardstudio_web_layout::matrix_relations::cross_matrix_note(
        &snapshot.scene.matrix_scenes,
        &selected.context,
        &model.selected_part_ids,
    );
    Some(relations)
}

use super::*;
pub(crate) use boardstudio_web_layout::companion_placement::companion_placement_projection;

pub(crate) fn relation_target_request(
    runtime: &Runtime,
    mut req: objects::TreeSelectRequest,
) -> Option<objects::TreeSelectRequest> {
    if runtime.scope().as_ref() != Some(&req.scope) {
        return None;
    }
    let model = runtime.model();
    if let objects::TreeContext::Component {
        part_id: Some(pid), ..
    } = &req.context
    {
        req.context = objects::component_context_for_finding_part(&model, pid)?;
    }
    if !selection::context_is_current(&model, &req.scope, &req.context) {
        return None;
    }
    if let objects::TreeContext::Key {
        matrix_id,
        row,
        column,
    } = &req.context
    {
        let doc = &model.accepted.as_ref()?.document;
        if doc
            .matrices
            .iter()
            .find(|m| m.id == *matrix_id)?
            .cells
            .iter()
            .any(|c| c.row == *row && c.column == *column && c.deleted)
        {
            return None;
        }
    }
    Some(req)
}

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
        | objects::TreeContext::Key { matrix_id, .. }
        | objects::TreeContext::Component {
            matrix_id: Some(matrix_id),
            ..
        } => matrix_id,
        _ => return None,
    };
    let snap = model.accepted.as_ref()?;
    let scene = snap
        .scene
        .matrix_scenes
        .iter()
        .find(|s| s.matrix_id == *matrix_id)?;
    let mut rels = boardstudio_web_layout::matrix_relations::matrix_relations(
        &snap.document,
        scene,
        &selected.context,
        &snap.scene.matrix_scenes,
    )?;
    rels.selection_note = boardstudio_web_layout::matrix_relations::cross_matrix_note(
        &snap.scene.matrix_scenes,
        &selected.context,
        &model.selected_part_ids,
    );
    Some(rels)
}

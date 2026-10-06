//! The Case Inspector's summary of the tree-owned PCB part selection.
use boardstudio_application::ReadModel;
use boardstudio_web_ui_model::tree::ScopedTreeContext;

pub struct SelectedPartSummary {
    pub title: String,
    pub breadcrumb: String,
    pub selected_count: usize,
}

/// Project the current tree-owned PCB selection for the Case Inspector.
/// Requiring the resolved context to equal the accepted selection prevents a
/// stale tree context from describing a different active selection.
pub fn selected_part_summary(
    model: &ReadModel,
    selected: Option<&ScopedTreeContext>,
) -> Option<SelectedPartSummary> {
    let selected = selected?;
    if !matches!(
        &selected.context,
        crate::objects::TreeContext::Key { .. } | crate::objects::TreeContext::Component { .. }
    ) {
        return None;
    }
    if !crate::selection::context_is_current(model, &selected.scope, &selected.context) {
        return None;
    }
    let ids = crate::objects::resolve_selection(model, &selected.context)?;
    if ids.is_empty() || ids != model.selected_part_ids {
        return None;
    }
    let document = &model.accepted.as_ref()?.document;
    let board = document
        .boards
        .iter()
        .find(|board| board.id == model.active_board_id)?;
    let references = ids
        .iter()
        .map(|id| {
            document
                .parts
                .iter()
                .find(|part| part.id == *id)
                .map(|part| part.reference.clone())
        })
        .collect::<Option<Vec<_>>>()?;
    let matrix_id = match &selected.context {
        crate::objects::TreeContext::Key { matrix_id, .. } => Some(matrix_id.as_str()),
        crate::objects::TreeContext::Component { matrix_id, .. } => matrix_id.as_deref(),
        _ => None,
    };
    let context_name = matrix_id
        .and_then(|matrix_id| {
            document
                .layouts
                .iter()
                .find(|layout| {
                    layout.board_id == model.active_board_id
                        && layout.matrix_id == matrix_id
                        && !layout.name.trim().is_empty()
                })
                .map(|layout| layout.name.clone())
                .or_else(|| {
                    document
                        .matrices
                        .iter()
                        .find(|matrix| matrix.id == matrix_id)
                        .and_then(|matrix| matrix.name.as_deref())
                        .map(str::trim)
                        .filter(|name| !name.is_empty())
                        .map(str::to_owned)
                })
        })
        .or_else(|| references.first().cloned());
    Some(SelectedPartSummary {
        title: references.join(", "),
        breadcrumb: context_name
            .map(|name| format!("{} / Case / {name}", board.name))
            .unwrap_or_else(|| format!("{} / Case", board.name)),
        selected_count: references.len(),
    })
}

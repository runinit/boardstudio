//! Read-only presentation of the current, validated selection context.
use super::objects::{self, TreeContext};
use boardstudio_application::ReadModel;

pub(super) struct ContextSummary {
    pub title: String,
    pub detail: Option<String>,
}

pub(super) fn summarize(model: &ReadModel, context: &TreeContext) -> Option<ContextSummary> {
    objects::resolve_selection(model, context)?;
    let document = &model.accepted.as_ref()?.document;
    let label = objects::context_label(model, context)?;
    let selected_count = model
        .selected_part_ids
        .iter()
        .filter(|id| document.parts.iter().any(|part| part.id == **id))
        .count();
    let matrix_title = |id: &str| {
        let index = document
            .matrices
            .iter()
            .position(|matrix| matrix.id == id)?;
        let matrix = &document.matrices[index];
        let title = document
            .layouts
            .iter()
            .find(|layout| {
                layout.matrix_id == id
                    && layout.board_id == model.active_board_id
                    && !layout.name.is_empty()
            })
            .map(|layout| layout.name.clone())
            .unwrap_or_else(|| {
                matrix
                    .name
                    .as_deref()
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("Matrix {}", index + 1))
            });
        Some((matrix, title))
    };
    let (title, detail) = match context {
        TreeContext::Matrix { matrix_id } => {
            let (matrix, title) = matrix_title(matrix_id)?;
            (
                title,
                Some(format!("{} rows · {} columns", matrix.rows, matrix.columns)),
            )
        }
        TreeContext::Row { matrix_id, row } => (
            format!("{} · Row {}", matrix_title(matrix_id)?.1, row + 1),
            Some(format!("{selected_count} keys selected")),
        ),
        TreeContext::Column { matrix_id, column } => (
            format!("{} · Column {}", matrix_title(matrix_id)?.1, column + 1),
            Some(format!("{selected_count} keys selected")),
        ),
        TreeContext::Key {
            matrix_id,
            row,
            column,
        } => (
            format!(
                "{} · Key {}.{}",
                matrix_title(matrix_id)?.1,
                column + 1,
                row + 1
            ),
            Some(format!("{selected_count} keys selected")),
        ),
        TreeContext::Component { .. }
        | TreeContext::MountedModule { .. }
        | TreeContext::Board { .. }
        | TreeContext::LayoutGroup { .. }
        | TreeContext::Outline { .. }
        | TreeContext::OutlineVersion { .. }
        | TreeContext::Bridge { .. } => (label, None),
    };
    Some(ContextSummary { title, detail })
}

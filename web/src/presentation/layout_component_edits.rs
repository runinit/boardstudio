//! Resolver builders and logical keys for the Layout Component Inspector.
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution};
use boardstudio_core::model::{
    Constraint, EditOperation, Part, PartOutline, Position, ProjectDoc, Vec2,
};

use super::inspector::{ComponentPositionAxis, LayoutConstraintValues};
use boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum InspectorField {
    X,
    Y,
    Margin,
    Layout,
    Constraint,
    RemoveConstraint,
}

impl InspectorField {
    pub(super) fn is_field(self) -> bool {
        matches!(self, Self::X | Self::Y | Self::Margin)
    }
}

pub(super) type LayoutComponentInspectorEdits = PendingEditSignals<InspectorField>;

fn commit(command: EditOperation, target_ids: Vec<String>) -> Resolution {
    Resolution::submit(target_ids, command)
}

fn part_of<'a>(document: &'a ProjectDoc, part_id: &str) -> Option<&'a Part> {
    document.parts.iter().find(|part| part.id == part_id)
}

fn board_of<'a>(
    document: &'a ProjectDoc,
    board_id: &str,
) -> Option<&'a boardstudio_core::model::Board> {
    document.boards.iter().find(|board| board.id == board_id)
}

/// A part is ineligible for direct positioning when it is locked or a saved constraint
/// on its board drives it (the relationship would overwrite any typed value).
fn position_ineligible_reason(
    document: &ProjectDoc,
    board_id: &str,
    part_id: &str,
) -> Option<&'static str> {
    let part = part_of(document, part_id)?;
    if part.locked == Some(true) {
        return Some("This part is locked.");
    }
    let drives = document.constraints.iter().any(|constraint| {
        constraint.target() == part_id
            && board_of(document, board_id)
                .is_some_and(|board| board.part_ids.iter().any(|id| id == constraint.source()))
    });
    drives.then_some("This part is driven by a relationship.")
}

/// Resolve a position edit: set one axis of the anchor part and translate the rest of
/// the selection by the same delta, from the accepted positions at execution time.
pub fn position_resolver(
    part_id: String,
    selected_part_ids: Vec<String>,
    board_id: String,
    axis: ComponentPositionAxis,
    value: f64,
) -> EditResolver {
    EditResolver::new(
        "layout-inspector-position",
        move |accepted: &AcceptedSnapshot| {
            let document = &accepted.document;
            let Some(part) = part_of(document, &part_id) else {
                return Resolution::Retire("The selected part no longer exists.".into());
            };
            if let Some(reason) = position_ineligible_reason(document, &board_id, &part_id) {
                return Resolution::Retire(reason.into());
            }
            if selected_part_ids.first() != Some(&part_id) {
                return Resolution::Retire(
                    "The selection changed before this edit ran; the position was not changed."
                        .into(),
                );
            }
            let Some(board) = board_of(document, &board_id) else {
                return Resolution::Retire("The board no longer exists.".into());
            };
            if selected_part_ids.iter().any(|selected_id| {
                !board.part_ids.contains(selected_id) || part_of(document, selected_id).is_none()
            }) {
                return Resolution::Retire(
                    "A selected part no longer exists; the position was not changed.".into(),
                );
            }
            let mut at = part.pose.at;
            match axis {
                ComponentPositionAxis::X => at.x = value,
                ComponentPositionAxis::Y => at.y = value,
            }
            if at == part.pose.at {
                return Resolution::Unchanged;
            }
            let delta = Vec2 {
                x: at.x - part.pose.at.x,
                y: at.y - part.pose.at.y,
            };
            let positions = selected_part_ids
                .iter()
                .map(|selected_id| {
                    let selected = part_of(document, selected_id).expect("checked above");
                    Position {
                        id: selected.id.clone(),
                        at: Vec2 {
                            x: selected.pose.at.x + delta.x,
                            y: selected.pose.at.y + delta.y,
                        },
                    }
                })
                .collect();
            commit(
                EditOperation::MoveParts { positions },
                selected_part_ids.clone(),
            )
        },
    )
}

/// Resolve an assign-layout edit against the accepted document at execution time. The
/// replacement document is cloned from the snapshot the resolver receives, so a queued
/// assign never reverts edits accepted after it was committed.
pub fn assign_layout_resolver(
    part_id: String,
    board_id: String,
    layout_id: Option<String>,
) -> EditResolver {
    EditResolver::new(
        "layout-inspector-layout",
        move |accepted: &AcceptedSnapshot| {
            let document = &accepted.document;
            let Some(_) = part_of(document, &part_id) else {
                return Resolution::Retire("The selected part no longer exists.".into());
            };
            let Some(board) = board_of(document, &board_id) else {
                return Resolution::Retire("The board no longer exists.".into());
            };
            if let Some(layout_id) = layout_id.as_ref()
                && !document
                    .layouts
                    .iter()
                    .any(|layout| layout.id == *layout_id && layout.board_id == board.id)
            {
                return Resolution::Retire("The layout no longer exists.".into());
            }
            let current_layout = document
                .layouts
                .iter()
                .find(|layout| layout.board_id == board.id && layout.part_ids.contains(&part_id));
            if current_layout.map(|layout| layout.id.as_str()) == layout_id.as_deref() {
                return Resolution::Unchanged;
            }
            let mut replacement = document.as_ref().clone();
            for layout in &mut replacement.layouts {
                if layout.board_id == board.id {
                    layout.part_ids.retain(|member_id| member_id != &part_id);
                }
            }
            if let Some(layout_id) = layout_id.as_ref()
                && let Some(layout) = replacement
                    .layouts
                    .iter_mut()
                    .find(|layout| layout.id == *layout_id && layout.board_id == board.id)
            {
                layout.part_ids.push(part_id.clone());
            }
            let mut target_ids = vec![part_id.clone()];
            if let Some(layout_id) = layout_id.clone() {
                target_ids.push(layout_id);
            }
            commit(
                EditOperation::ReplaceDocument {
                    document: Box::new(replacement),
                },
                target_ids,
            )
        },
    )
}

/// Resolve an outline edit against the accepted document at execution time.
pub fn outline_resolver(part_id: String, outline: PartOutline) -> EditResolver {
    EditResolver::new(
        "layout-inspector-outline",
        move |accepted: &AcceptedSnapshot| {
            let document = &accepted.document;
            if part_of(document, &part_id).is_none() {
                return Resolution::Retire("The selected part no longer exists.".into());
            }
            if part_of(document, &part_id).and_then(|part| part.outline.as_ref()) == Some(&outline)
            {
                return Resolution::Unchanged;
            }
            let mut replacement = document.as_ref().clone();
            let Some(part) = replacement.parts.iter_mut().find(|part| part.id == part_id) else {
                return Resolution::Retire("The selected part no longer exists.".into());
            };
            part.outline = Some(outline.clone());
            commit(
                EditOperation::ReplaceDocument {
                    document: Box::new(replacement),
                },
                vec![part_id.clone()],
            )
        },
    )
}

/// Resolve a constraint edit: reuse the existing constraint's id when one already
/// drives the part on this board, otherwise seed a fresh id.
pub fn constraint_resolver(
    part_id: String,
    board_id: String,
    source_part_id: String,
    values: LayoutConstraintValues,
    id_seed: u64,
) -> EditResolver {
    EditResolver::new(
        "layout-inspector-constraint",
        move |accepted: &AcceptedSnapshot| {
            let document = &accepted.document;
            let Some(_) = part_of(document, &part_id) else {
                return Resolution::Retire("The selected part no longer exists.".into());
            };
            let Some(board) = board_of(document, &board_id) else {
                return Resolution::Retire("The board no longer exists.".into());
            };
            if source_part_id == part_id
                || !board.part_ids.iter().any(|id| id == &source_part_id)
                || part_of(document, &source_part_id).is_none()
            {
                return Resolution::Retire(
                    "The constraint source is no longer available on this board.".into(),
                );
            }
            let existing_id = document
                .constraints
                .iter()
                .find(|constraint| {
                    constraint.target() == part_id
                        && board.part_ids.iter().any(|id| id == constraint.source())
                })
                .map(|constraint| constraint.id().to_owned());
            let constraint_id =
                existing_id.unwrap_or_else(|| format!("layout-component-constraint-{id_seed}"));
            let constraint = match &values {
                LayoutConstraintValues::Offset { offset, rotation } => Constraint::Offset {
                    id: constraint_id,
                    source_part_id: source_part_id.clone(),
                    target_part_id: part_id.clone(),
                    offset: *offset,
                    rotation: *rotation,
                },
                LayoutConstraintValues::Mirror { axis, coordinate } => Constraint::Mirror {
                    id: constraint_id,
                    source_part_id: source_part_id.clone(),
                    target_part_id: part_id.clone(),
                    axis: axis.clone(),
                    coordinate: *coordinate,
                },
            };
            if document.constraints.contains(&constraint) {
                return Resolution::Unchanged;
            }
            let mut target_ids = vec![constraint.id().to_owned(), part_id.clone()];
            if !target_ids.iter().any(|id| id == constraint.source()) {
                target_ids.push(constraint.source().to_owned());
            }
            commit(EditOperation::SetConstraint { constraint }, target_ids)
        },
    )
}

/// Resolve a constraint removal: the constraint must still drive the part.
pub fn remove_constraint_resolver(part_id: String, constraint_id: String) -> EditResolver {
    EditResolver::new(
        "layout-inspector-remove-constraint",
        move |accepted: &AcceptedSnapshot| {
            let document = &accepted.document;
            if part_of(document, &part_id).is_none() {
                return Resolution::Retire("The selected part no longer exists.".into());
            }
            if !document.constraints.iter().any(|constraint| {
                constraint.id() == constraint_id && constraint.target() == part_id
            }) {
                return Resolution::Retire("The relationship no longer exists.".into());
            }
            commit(
                EditOperation::RemoveConstraint {
                    id: constraint_id.clone(),
                },
                vec![constraint_id.clone(), part_id.clone()],
            )
        },
    )
}

/// Resolve a keyboard nudge: a delta intent. Each part's accepted position moves by the
/// step when the edit runs, so repeated presses queued behind each other compose. A part
/// that is gone, locked or relationship-driven by then retires the nudge.
pub fn nudge_resolver(part_ids: Vec<String>, board_id: String, delta: Vec2) -> EditResolver {
    EditResolver::new("layout-nudge", move |accepted: &AcceptedSnapshot| {
        let document = &accepted.document;
        let mut positions = Vec::with_capacity(part_ids.len());
        for part_id in &part_ids {
            let Some(part) = part_of(document, part_id) else {
                return Resolution::Retire("The selected part no longer exists.".into());
            };
            if let Some(reason) = position_ineligible_reason(document, &board_id, part_id) {
                return Resolution::Retire(reason.into());
            }
            positions.push(Position {
                id: part.id.clone(),
                at: Vec2 {
                    x: part.pose.at.x + delta.x,
                    y: part.pose.at.y + delta.y,
                },
            });
        }
        commit(EditOperation::MoveParts { positions }, part_ids.clone())
    })
}

/// Resolve only the coordinates edited in the old Inspector, translating the selection
/// from its first part. Keep the preview transaction when replacing the preview.
pub fn commit_position_resolver(
    part_ids: Vec<String>,
    board_id: String,
    x: Option<f64>,
    y: Option<f64>,
    transaction_id: String,
) -> EditResolver {
    EditResolver::new("layout-old-position", move |accepted: &AcceptedSnapshot| {
        let document = &accepted.document;
        let Some(anchor) = part_ids.first().and_then(|id| part_of(document, id)) else {
            return Resolution::Retire("The selected part no longer exists.".into());
        };
        let Some(board) = board_of(document, &board_id) else {
            return Resolution::Retire("The board no longer exists.".into());
        };
        let delta = Vec2 {
            x: x.map_or(0.0, |value| value - anchor.pose.at.x),
            y: y.map_or(0.0, |value| value - anchor.pose.at.y),
        };
        let mut positions = Vec::with_capacity(part_ids.len());
        for id in &part_ids {
            let Some(part) = part_of(document, id).filter(|_| board.part_ids.contains(id)) else {
                return Resolution::Retire(
                    "A selected part no longer belongs to this board.".into(),
                );
            };
            if let Some(reason) = position_ineligible_reason(document, &board_id, id) {
                return Resolution::Retire(reason.into());
            }
            positions.push(Position {
                id: id.clone(),
                at: Vec2 {
                    x: part.pose.at.x + delta.x,
                    y: part.pose.at.y + delta.y,
                },
            });
        }
        if delta == Vec2::default() {
            return Resolution::Unchanged;
        }
        Resolution::submit_with_transaction_id(
            part_ids.clone(),
            transaction_id.clone(),
            EditOperation::MoveParts { positions },
        )
    })
}

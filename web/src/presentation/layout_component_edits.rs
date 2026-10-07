//! The Layout Inspector's pending edits: one edit ticket per committed field, and the
//! resolver builders every action submits through. Each resolver is a pure function of
//! the accepted snapshot plus what the user captured when they committed — it re-checks
//! at execution time what admission checked when the action was dispatched (the part
//! still exists, the anchor is unchanged, locked and driven parts stay ineligible) and
//! builds the edit operation from the accepted document it is handed, so a queued edit
//! always applies on top of the edits accepted before it. This module is the pattern the
//! later cluster tickets copy.
use crate::edit_ticket::{EditTicket, Settlement};
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution};
use boardstudio_core::model::{
    Constraint, EditCommand, EditOperation, EditPhase, Part, PartOutline, Position, ProjectDoc,
    Vec2,
};
use dioxus::prelude::{ReadableExt, WritableExt};
use std::rc::Rc;

use super::inspector::{ComponentPositionAxis, LayoutConstraintValues};

/// One ticket per committed Inspector field. A new commit for a field replaces its
/// ticket; the field renders its draft while the ticket is pending.
#[derive(Clone, Default)]
pub struct LayoutComponentInspectorEdits {
    pub x: Option<EditTicket>,
    pub y: Option<EditTicket>,
    pub margin: Option<EditTicket>,
    pub layout: Option<EditTicket>,
    pub constraint: Option<EditTicket>,
    pub remove_constraint: Option<EditTicket>,
}

/// Begin a pending edit and park it on its field.
pub fn begin_inspector_edit(
    runtime: &Rc<crate::runtime::Runtime>,
    field: InspectorField,
    resolver: EditResolver,
) -> EditTicket {
    let ticket = EditTicket::begin(runtime, field.label(), Some("layout".into()), resolver);
    ticket
}

/// The Inspector fields that can hold a pending edit ticket.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InspectorField {
    X,
    Y,
    Margin,
    Layout,
    Constraint,
    RemoveConstraint,
}

impl InspectorField {
    fn label(self) -> &'static str {
        match self {
            InspectorField::X => "layout-inspector-x",
            InspectorField::Y => "layout-inspector-y",
            InspectorField::Margin => "layout-inspector-outline",
            InspectorField::Layout => "layout-inspector-layout",
            InspectorField::Constraint => "layout-inspector-constraint",
            InspectorField::RemoveConstraint => "layout-inspector-remove-constraint",
        }
    }
}

fn commit(command: EditOperation, target_ids: Vec<String>) -> Resolution {
    Resolution::Submit(EditCommand {
        base_revision: 0,
        transaction_id: String::new(),
        phase: EditPhase::Commit,
        target_ids,
        operation: command,
    })
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

/// Resolve the old position Inspector's commit: the part moves to the typed point. The
/// commit keeps the preview's transaction so the preview is replaced rather than stacked.
pub fn commit_position_resolver(part_id: String, at: Vec2, transaction_id: String) -> EditResolver {
    EditResolver::new("layout-old-position", move |accepted: &AcceptedSnapshot| {
        let Some(part) = part_of(&accepted.document, &part_id) else {
            return Resolution::Retire("The selected part no longer exists.".into());
        };
        if part.pose.at == at {
            return Resolution::Unchanged;
        }
        Resolution::Submit(EditCommand {
            base_revision: 0,
            transaction_id: transaction_id.clone(),
            phase: EditPhase::Commit,
            target_ids: vec![part_id.clone()],
            operation: EditOperation::MoveParts {
                positions: vec![Position {
                    id: part_id.clone(),
                    at,
                }],
            },
        })
    })
}

/// Read a field's settlement; a terminal settlement drops the ticket so the field shows
/// the accepted document again. Returns the settlement for the caller to render.
pub fn settle_field(ticket: &mut Option<EditTicket>, owner_is_live: bool) -> Option<Settlement> {
    let pending = ticket.as_ref()?;
    let settlement = pending.settlement(owner_is_live);
    if settlement != Settlement::Pending {
        ticket.take();
    }
    (settlement != Settlement::Pending).then_some(settlement)
}

/// Settle the Inspector's pending edits on each render. Pending keeps the draft; a
/// failure restores the accepted value and reports the message inline; landed and
/// retired tickets drop so the field follows the accepted document again. The Inspector
/// itself is the owner-liveness answer: a departed owner unmounts it, so the tickets
/// here always belong to the live selection.
pub fn settle_inspector_edits(
    mut pending_edits: dioxus::prelude::Signal<LayoutComponentInspectorEdits>,
    accepted_x: f64,
    accepted_y: f64,
    accepted_margin: Option<f64>,
    x: &mut dioxus::prelude::Signal<String>,
    y: &mut dioxus::prelude::Signal<String>,
    margin: &mut dioxus::prelude::Signal<String>,
    error: &mut dioxus::prelude::Signal<Option<String>>,
) {
    let mut edits = pending_edits.peek().clone();
    let mut changed = false;
    let mut failure: Option<String> = None;
    fn settle_text_field(
        ticket: &mut Option<EditTicket>,
        accepted: &str,
        draft: &mut dioxus::prelude::Signal<String>,
        changed: &mut bool,
        failure: &mut Option<String>,
    ) {
        if let Some(settlement) = settle_field(ticket, true) {
            *changed = true;
            if let Settlement::Failed { message } = settlement {
                *failure = Some(message);
            }
            if draft.peek().as_str() != accepted {
                draft.set(accepted.to_owned());
            }
        }
    }
    settle_text_field(
        &mut edits.x,
        &format!("{accepted_x:.2}"),
        x,
        &mut changed,
        &mut failure,
    );
    settle_text_field(
        &mut edits.y,
        &format!("{accepted_y:.2}"),
        y,
        &mut changed,
        &mut failure,
    );
    settle_text_field(
        &mut edits.margin,
        &accepted_margin
            .map(|value| value.to_string())
            .unwrap_or_default(),
        margin,
        &mut changed,
        &mut failure,
    );
    for ticket in [
        &mut edits.layout,
        &mut edits.constraint,
        &mut edits.remove_constraint,
    ] {
        if let Some(settlement) = settle_field(ticket, true) {
            changed = true;
            if let Settlement::Failed { message } = settlement {
                failure = Some(message);
            }
        }
    }
    if changed {
        pending_edits.set(edits);
    }
    if let Some(message) = failure {
        error.set(Some(message));
    }
}

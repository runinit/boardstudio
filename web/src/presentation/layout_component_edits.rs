//! Resolver builders and logical keys for the Layout Component Inspector.
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution};
use boardstudio_core::model::{
    CompanionPlacementTarget, Constraint, ConstraintEndpoint, ConstraintRelation, EditOperation,
    HalfExceptionState, MirroredCompanionMode, MirroredCompanionTweak, MirroredItemKind,
    OffsetAxes, Part, PartOutline, Position, ProjectDoc, Vec2,
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
    MirroredHalfState,
    PartMirror,
    CompanionPlacement,
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
    if board_of(document, board_id).is_none() {
        return Some("The board is unavailable.");
    }
    match document.constraint_driving_part(part_id) {
        Ok(Some(_)) => Some("This part is driven by a relationship."),
        Ok(None) => None,
        Err(_) => Some("The saved placement relationship could not be verified."),
    }
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
            let existing = match document.constraint_driving_part(&part_id) {
                Ok(existing) => existing,
                Err(_) => {
                    return Resolution::Retire(
                        "The saved placement relationship could not be verified.".into(),
                    );
                }
            };
            let source_is_existing_endpoint = existing.is_some_and(|constraint| match constraint {
                Constraint::Framed { source, .. } => match source {
                    ConstraintEndpoint::Part { part_id } => part_id == &source_part_id,
                    ConstraintEndpoint::LinkedMirror { mirror_part_id, .. } => {
                        mirror_part_id == &source_part_id
                    }
                },
                _ => false,
            });
            if source_part_id == part_id
                || (!source_is_existing_endpoint
                    && !board.part_ids.iter().any(|id| id == &source_part_id))
                || part_of(document, &source_part_id).is_none()
            {
                return Resolution::Retire(
                    "The constraint source is no longer available on this board.".into(),
                );
            }
            let constraint_id = existing
                .map(|constraint| constraint.id().to_owned())
                .unwrap_or_else(|| format!("layout-component-constraint-{id_seed}"));
            let framed_endpoints = existing.and_then(|constraint| match constraint {
                Constraint::Framed { source, target, .. } => Some((source.clone(), target.clone())),
                _ => None,
            });
            let typed_relation = match &values {
                LayoutConstraintValues::Framed { relation } => relation.clone(),
                LayoutConstraintValues::Offset { offset, rotation } => ConstraintRelation::Offset {
                    offset: *offset,
                    rotation: *rotation,
                },
                LayoutConstraintValues::PartOffset { offset, rotation } => {
                    ConstraintRelation::PartOffset {
                        offset: *offset,
                        rotation: *rotation,
                    }
                }
                LayoutConstraintValues::Mirror { axis, coordinate } => match axis {
                    boardstudio_core::model::MirrorAxis::Vertical => {
                        ConstraintRelation::MirrorLine {
                            at: Vec2 {
                                x: *coordinate,
                                y: 0.0,
                            },
                            angle: 90.0,
                        }
                    }
                    boardstudio_core::model::MirrorAxis::Horizontal => {
                        ConstraintRelation::MirrorLine {
                            at: Vec2 {
                                x: 0.0,
                                y: *coordinate,
                            },
                            angle: 0.0,
                        }
                    }
                },
            };
            let constraint = if matches!(&values, LayoutConstraintValues::Framed { .. })
                || framed_endpoints.is_some()
            {
                let (source, target) = framed_endpoints
                    .map(|(source, target)| {
                        let source_visible_id = match &source {
                            ConstraintEndpoint::Part { part_id } => part_id.as_str(),
                            ConstraintEndpoint::LinkedMirror { mirror_part_id, .. } => {
                                mirror_part_id.as_str()
                            }
                        };
                        let source = if source_visible_id == source_part_id {
                            source
                        } else {
                            ConstraintEndpoint::Part {
                                part_id: source_part_id.clone(),
                            }
                        };
                        (source, target)
                    })
                    .unwrap_or_else(|| {
                        (
                            ConstraintEndpoint::Part {
                                part_id: source_part_id.clone(),
                            },
                            ConstraintEndpoint::Part {
                                part_id: part_id.clone(),
                            },
                        )
                    });
                Constraint::Framed {
                    id: constraint_id,
                    source,
                    target,
                    relation: typed_relation,
                }
            } else {
                match &values {
                    LayoutConstraintValues::Offset { offset, rotation } => Constraint::offset(
                        OffsetAxes::Board,
                        constraint_id,
                        source_part_id.clone(),
                        part_id.clone(),
                        *offset,
                        *rotation,
                    ),
                    LayoutConstraintValues::PartOffset { offset, rotation } => Constraint::offset(
                        OffsetAxes::Part,
                        constraint_id,
                        source_part_id.clone(),
                        part_id.clone(),
                        *offset,
                        *rotation,
                    ),
                    LayoutConstraintValues::Mirror { axis, coordinate } => Constraint::Mirror {
                        id: constraint_id,
                        source_part_id: source_part_id.clone(),
                        target_part_id: part_id.clone(),
                        axis: axis.clone(),
                        coordinate: *coordinate,
                    },
                    LayoutConstraintValues::Framed { .. } => {
                        return Resolution::Retire(
                            "The typed relationship could not be created.".into(),
                        );
                    }
                }
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

/// Resolve a change of axes on a saved offset constraint: the constraint must still drive the
/// part, and a mirror has no axes to change. Core moves no part.
pub fn offset_axes_resolver(
    part_id: String,
    constraint_id: String,
    to: OffsetAxes,
) -> EditResolver {
    EditResolver::new(
        "layout-inspector-offset-axes",
        move |accepted: &AcceptedSnapshot| {
            let document = &accepted.document;
            let constraint = match document.constraint_driving_part(&part_id) {
                Ok(Some(constraint)) if constraint.id() == constraint_id => constraint,
                Ok(_) => {
                    return Resolution::Retire("The constraint no longer drives this part.".into());
                }
                Err(_) => {
                    return Resolution::Retire(
                        "The saved placement relationship could not be verified.".into(),
                    );
                }
            };
            if let Constraint::Framed {
                id,
                source,
                target,
                relation,
            } = constraint
            {
                let (offset, rotation) = match relation {
                    ConstraintRelation::Offset { offset, rotation } if to == OffsetAxes::Board => {
                        (*offset, *rotation)
                    }
                    ConstraintRelation::Offset { offset, rotation } => (*offset, *rotation),
                    ConstraintRelation::PartOffset { offset, rotation }
                        if to == OffsetAxes::Part =>
                    {
                        (*offset, *rotation)
                    }
                    ConstraintRelation::PartOffset { offset, rotation } => (*offset, *rotation),
                    ConstraintRelation::MirrorLine { .. } => {
                        return Resolution::Retire("A mirror line has no offset axes.".into());
                    }
                };
                let relation = match to {
                    OffsetAxes::Board => ConstraintRelation::Offset { offset, rotation },
                    OffsetAxes::Part => ConstraintRelation::PartOffset { offset, rotation },
                };
                let replacement = Constraint::Framed {
                    id: id.clone(),
                    source: source.clone(),
                    target: target.clone(),
                    relation,
                };
                if replacement == *constraint {
                    return Resolution::Unchanged;
                }
                return commit(
                    EditOperation::SetConstraint {
                        constraint: replacement,
                    },
                    vec![constraint_id.clone(), part_id.clone()],
                );
            }
            match constraint.offset_axes() {
                Some(axes) if axes == to => return Resolution::Unchanged,
                Some(_) => {}
                None => {
                    return Resolution::Retire("A mirror constraint has no offset axes.".into());
                }
            }
            commit(
                EditOperation::ConvertOffsetConstraint {
                    constraint_id: constraint_id.clone(),
                    to,
                },
                vec![constraint_id.clone(), part_id.clone()],
            )
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
            match document.constraint_driving_part(&part_id) {
                Ok(Some(constraint)) if constraint.id() == constraint_id => {}
                Ok(_) => return Resolution::Retire("The relationship no longer exists.".into()),
                Err(_) => {
                    return Resolution::Retire(
                        "The saved placement relationship could not be verified.".into(),
                    );
                }
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

/// Resolve a linked part's exception against the accepted mirrored-half record.
pub fn mirrored_half_state_resolver(
    pair_id: String,
    mirror_id: String,
    state: HalfExceptionState,
) -> EditResolver {
    EditResolver::new(
        "layout-mirrored-half-state",
        move |accepted: &AcceptedSnapshot| {
            let Some(pair) = accepted
                .document
                .hardware
                .as_ref()
                .and_then(|hardware| hardware.pairs.iter().find(|pair| pair.id == pair_id))
            else {
                return Resolution::Retire("The mirrored pair no longer exists.".into());
            };
            let Some(halves) = pair.halves.as_ref() else {
                return Resolution::Retire("The mirrored pair no longer links items.".into());
            };
            if !halves
                .correspondences
                .iter()
                .any(|link| link.kind == MirroredItemKind::Part && link.mirror_id == mirror_id)
            {
                return Resolution::Retire("The linked part no longer exists.".into());
            }
            let current = halves
                .exceptions
                .iter()
                .find(|exception| {
                    exception.kind == MirroredItemKind::Part && exception.mirror_id == mirror_id
                })
                .map(|exception| match exception.state {
                    boardstudio_core::model::MirroredExceptionState::Local => {
                        HalfExceptionState::Local
                    }
                    boardstudio_core::model::MirroredExceptionState::Omitted => {
                        HalfExceptionState::Omitted
                    }
                })
                .unwrap_or(HalfExceptionState::Linked);
            if current == state {
                return Resolution::Unchanged;
            }
            Resolution::submit(
                vec![pair_id.clone(), mirror_id.clone()],
                EditOperation::SetHalfException {
                    pair_id: pair_id.clone(),
                    kind: MirroredItemKind::Part,
                    mirror_id: mirror_id.clone(),
                    state,
                },
            )
        },
    )
}

/// Resolve an independent part's "Mirror on both halves" toggle against the
/// accepted Part correspondence: one Undo entry per flip via SetMirroredPart.
pub fn part_mirror_resolver(part_id: String, mirrored: bool) -> EditResolver {
    EditResolver::new("layout-part-mirror", move |accepted: &AcceptedSnapshot| {
        if part_of(&accepted.document, &part_id).is_none() {
            return Resolution::Retire("The part no longer exists.".into());
        }
        Resolution::submit(
            vec![part_id.clone()],
            EditOperation::SetMirroredPart {
                part_id: part_id.clone(),
                mirrored,
            },
        )
    })
}

pub fn companion_placement_resolver(
    pair_id: String,
    targets: Vec<CompanionPlacementTarget>,
    mode: MirroredCompanionMode,
    tweak: Option<MirroredCompanionTweak>,
) -> EditResolver {
    EditResolver::new(
        "layout-companion-placement",
        move |accepted: &AcceptedSnapshot| {
            let pairs: Vec<_> = accepted
                .document
                .hardware
                .as_ref()
                .into_iter()
                .flat_map(|hardware| &hardware.pairs)
                .filter(|pair| pair.id == pair_id)
                .collect();
            let [pair] = pairs.as_slice() else {
                return Resolution::Retire("The mirrored pair is unavailable or ambiguous.".into());
            };
            if pair.halves.is_none() {
                return Resolution::Retire(
                    "The mirrored pair no longer links matrix items.".into(),
                );
            }
            if targets.is_empty() {
                return Resolution::Retire(
                    "No companion placement targets remain selected.".into(),
                );
            }
            let mut target_ids = vec![pair_id.clone()];
            for target in &targets {
                let id = match target {
                    CompanionPlacementTarget::Pair => None,
                    CompanionPlacementTarget::Item { mirror_id, .. } => Some(mirror_id),
                    CompanionPlacementTarget::Row { matrix_id, .. }
                    | CompanionPlacementTarget::Column { matrix_id, .. }
                    | CompanionPlacementTarget::Matrix { matrix_id } => Some(matrix_id),
                };
                if let Some(id) = id
                    && !target_ids.contains(id)
                {
                    target_ids.push(id.clone());
                }
            }
            Resolution::submit(
                target_ids,
                EditOperation::SetCompanionPlacement {
                    pair_id: pair_id.clone(),
                    targets: targets.clone(),
                    mode,
                    tweak,
                },
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

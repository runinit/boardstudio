//! Private preparation for constraint edits (ticket 114).
//!
//! The public edit enum still carries legacy constraint values. This sidecar compiles the
//! authored literal IDs into the live endpoint frames the private solver already understands;
//! transaction work can then pass the same prepared value through apply and resolution without
//! changing the public operation or persisted schema.

use super::Refusal;
use crate::constraints::FramedConstraint;
use crate::model::{Constraint, EditOperation, OffsetAxes, ProjectDoc, Vec2};

#[derive(Clone, Debug, PartialEq)]
pub(in crate::mirrored_half) enum PreparedConstraintEdit {
    Upsert(FramedConstraint),
    Remove { id: String },
}

/// Prepares only constraint operations. `None` means the operation belongs to another family.
pub(super) fn prepare(
    document: &ProjectDoc,
    operation: &EditOperation,
) -> Result<Option<PreparedConstraintEdit>, Refusal> {
    use EditOperation as Op;
    match operation {
        Op::SetConstraint { constraint } => {
            if !constraint.is_finite() {
                return Err(Refusal::Request(format!(
                    "Constraint {} has a non-finite value",
                    constraint.id()
                )));
            }
            Ok(Some(PreparedConstraintEdit::Upsert(
                crate::constraints::compile_legacy(document, constraint)
                    .map_err(Refusal::Request)?,
            )))
        }
        Op::RemoveConstraint { id } => Ok(Some(PreparedConstraintEdit::Remove { id: id.clone() })),
        Op::ConvertOffsetConstraint { constraint_id, to } => {
            let Some(current) = document
                .constraints
                .iter()
                .find(|constraint| constraint.id() == constraint_id)
            else {
                // Keep the operation intact so the established apply path reports its exact
                // missing-ID wording and atomicity behavior.
                return Ok(None);
            };
            if !document
                .parts
                .iter()
                .any(|part| part.id == current.source())
            {
                return Err(Refusal::Request(format!(
                    "Constraint {constraint_id} source is missing"
                )));
            }
            if !document
                .parts
                .iter()
                .any(|part| part.id == current.target())
            {
                return Err(Refusal::Request(format!(
                    "Constraint {constraint_id} target is missing"
                )));
            }
            let Some(from) = current.offset_axes() else {
                return Err(Refusal::Request(format!(
                    "Constraint {constraint_id} is a mirror and has no offset axes"
                )));
            };
            let converted =
                convert_offset_values(document, current, from, *to).map_err(Refusal::Request)?;
            if !converted.is_finite() {
                return Err(Refusal::Request(format!(
                    "Constraint {constraint_id} has a non-finite value"
                )));
            }
            Ok(Some(PreparedConstraintEdit::Upsert(
                crate::constraints::compile_legacy(document, &converted)
                    .map_err(Refusal::Request)?,
            )))
        }
        _ => Ok(None),
    }
}

/// Matches `constraints::convert_offset` exactly, including its use of the currently authored
/// source pose. The result is compiled only after the stored axes have been converted.
fn convert_offset_values(
    document: &ProjectDoc,
    current: &Constraint,
    from: OffsetAxes,
    to: OffsetAxes,
) -> Result<Constraint, String> {
    let (Some((offset, rotation)), Some(source)) = (
        current.offset_numbers(),
        document
            .parts
            .iter()
            .find(|part| part.id == current.source()),
    ) else {
        return Err(format!("Constraint {} source is missing", current.id()));
    };
    if from == to {
        return Ok(current.clone());
    }
    let angle = match to {
        OffsetAxes::Part => -source.pose.rotation,
        OffsetAxes::Board => source.pose.rotation,
    };
    Ok(Constraint::offset(
        to,
        current.id().to_owned(),
        current.source().to_owned(),
        current.target().to_owned(),
        crate::matrix::splay::rotate(offset, angle, Vec2::default()),
        rotation,
    ))
}

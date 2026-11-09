//! Private preparation for constraint edits (ticket 114).
//!
//! The public edit enum still carries legacy constraint values. This sidecar compiles the
//! authored literal IDs into the live endpoint frames the private solver already understands;
//! transaction work can then pass the same prepared value through apply and resolution without
//! changing the public operation or persisted schema.

use super::Refusal;
use crate::constraints::FramedConstraint;
use crate::model::{
    Constraint, ConstraintEndpoint, EditOperation, OffsetAxes, Pose2, Position, ProjectDoc,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::mirrored_half) enum PreparedConstraintEdit {
    Upsert(FramedConstraint),
    Remove { id: String },
    DeferredTargetRetune(crate::constraints::DeferredTargetRetune),
}

/// Prepares target moves in their canonical batch order. Free source poses are advanced as the
/// same ordered `MoveParts` request would advance them; constrained targets retain their current
/// pose and contribute a retuned relation. Companion frames return deferred owner intent for
/// ticket 115 to bind after the batch's key pose is resolved.
pub(super) fn prepare_move_parts(
    document: &ProjectDoc,
    positions: &[Position],
) -> Result<Vec<PreparedConstraintEdit>, String> {
    let mut poses: BTreeMap<String, Pose2> = document
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let mut records = BTreeMap::<String, FramedConstraint>::new();
    let mut deferred = Vec::new();
    for position in positions {
        let Some(constraint) = crate::constraints::driving_constraint(document, &position.id)?
        else {
            if let Some(pose) = poses.get_mut(&position.id) {
                pose.at = position.at;
            }
            continue;
        };
        let id = constraint.id().to_owned();
        let record = if let Some(record) = records.get(&id) {
            record.clone()
        } else {
            crate::constraints::compile_legacy(document, constraint)?
        };
        let crate::constraints::FramedConstraint::Framed { source, target, .. } = &record;
        let source_id = authoritative_id(source);
        let target_id = authoritative_id(target);
        let source_pose = poses
            .get(source_id)
            .copied()
            .ok_or_else(|| format!("Constraint {id} source is missing"))?;
        let target_pose = poses
            .get(target_id)
            .copied()
            .ok_or_else(|| format!("Constraint {id} target is missing"))?;
        match crate::constraints::prepare_target_retune(
            document,
            &record,
            source_pose,
            target_pose,
            position,
            [None, None],
        )? {
            crate::constraints::PreparedTargetRetune::Retuned(record) => {
                records.insert(id, record);
            }
            crate::constraints::PreparedTargetRetune::Deferred(intent) => {
                deferred.push(PreparedConstraintEdit::DeferredTargetRetune(intent));
            }
        }
    }
    let mut prepared: Vec<_> = records
        .into_values()
        .map(PreparedConstraintEdit::Upsert)
        .collect();
    prepared.extend(deferred);
    Ok(prepared)
}

fn authoritative_id(endpoint: &crate::constraints::ConstraintEndpoint) -> &str {
    match endpoint {
        crate::constraints::ConstraintEndpoint::Part { part_id } => part_id,
        crate::constraints::ConstraintEndpoint::LinkedMirror { source_part_id, .. } => {
            source_part_id
        }
    }
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
            let converted =
                convert_offset_values(document, current, *to).map_err(Refusal::Request)?;
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

/// Normalizes only a newly authored Framed command. Loaded records keep their exact explicit
/// binding so stale or inconsistent endpoints continue to refuse in the shared solver.
pub(super) fn normalize_command(
    document: &ProjectDoc,
    links: &super::Links<'_>,
    operation: &EditOperation,
) -> Result<EditOperation, String> {
    let EditOperation::SetConstraint {
        constraint: constraint @ Constraint::Framed { .. },
    } = operation
    else {
        return Ok(operation.clone());
    };
    let Constraint::Framed {
        id,
        source,
        target,
        relation,
    } = constraint
    else {
        unreachable!("the SetConstraint match selected a Framed constraint");
    };
    let source = normalize_command_endpoint(document, links, source)?;
    let target = normalize_command_endpoint(document, links, target)?;
    Ok(EditOperation::SetConstraint {
        constraint: Constraint::Framed {
            id: id.clone(),
            source,
            target,
            relation: relation.clone(),
        },
    })
}

fn normalize_command_endpoint(
    document: &ProjectDoc,
    links: &super::Links<'_>,
    endpoint: &ConstraintEndpoint,
) -> Result<ConstraintEndpoint, String> {
    let ConstraintEndpoint::Part { part_id } = endpoint else {
        return Ok(endpoint.clone());
    };
    let Some(link) = links.linked_part(part_id)? else {
        return Ok(endpoint.clone());
    };
    let pair_id = link.pair.pair_id;
    let source_part_id = &link.correspondence.source_id;
    let mirror_part_id = &link.correspondence.mirror_id;
    if !document.parts.iter().any(|part| part.id == *source_part_id)
        || !document.parts.iter().any(|part| part.id == *mirror_part_id)
    {
        return Err(format!(
            "Constraint Part endpoint {part_id:?} has an unavailable linked binding"
        ));
    }
    Ok(ConstraintEndpoint::LinkedMirror {
        pair_id: pair_id.to_owned(),
        source_part_id: source_part_id.clone(),
        mirror_part_id: mirror_part_id.clone(),
    })
}

/// Reuses the legacy converter on a minimal document so preparation and apply share its exact
/// validation order and offset arithmetic. Only matching endpoint parts are copied; preserving
/// their document order keeps the legacy first-match behavior for duplicate IDs.
fn convert_offset_values(
    document: &ProjectDoc,
    current: &Constraint,
    to: OffsetAxes,
) -> Result<Constraint, String> {
    let mut scratch = ProjectDoc::empty("constraint-conversion", "Constraint conversion");
    scratch.parts = document
        .parts
        .iter()
        .filter(|part| part.id == current.source() || part.id == current.target())
        .cloned()
        .collect();
    scratch.constraints.push(current.clone());
    crate::constraints::convert_offset(&mut scratch, current.id(), to)?;
    scratch
        .constraints
        .pop()
        .ok_or_else(|| format!("Constraint {} disappeared during conversion", current.id()))
}

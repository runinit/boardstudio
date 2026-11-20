//! Retuning a framed relation when its target is dragged to a requested visible position.
//!
//! The stored numbers live in the endpoint views, so the drag is measured there: the old and
//! the requested target are both seen through the target's frame, and the visible difference
//! is added to the stored offset. The relation's identity, endpoints and direction are not
//! this module's to change; it returns the new numbers and the authority pose they imply.

use super::endpoints::EndpointFrame;
use super::relation::ConstraintRelation;
use crate::model::{Pose2, Vec2};

/// The relation with its numbers retuned, and the target authority pose they place.
#[derive(Debug, PartialEq)]
pub struct Retuned {
    pub relation: ConstraintRelation,
    pub target_authority: Pose2,
}

/// Retunes `relation` so the target, seen through `target_frame`, lands on `requested`.
///
/// `requested_heading` is the full requested visible heading when the operation supplies one;
/// `None` leaves the stored turn untouched, bit for bit. A reflecting relation has nothing to
/// retune and refuses, as a dragged mirror target already does.
pub fn retune_target(
    relation: &ConstraintRelation,
    source_authority: Pose2,
    source_frame: EndpointFrame,
    target_frame: EndpointFrame,
    target_authority: Pose2,
    requested: Vec2,
    requested_heading: Option<f64>,
) -> Result<Retuned, String> {
    relation.validate()?;
    if !(requested.x.is_finite()
        && requested.y.is_finite()
        && requested_heading.is_none_or(f64::is_finite))
    {
        return Err("A requested target pose is non-finite".into());
    }
    let source_view = source_frame.project(source_authority)?;
    let target_view = target_frame.project(target_authority)?;
    let delta = Vec2 {
        x: requested.x - target_view.at.x,
        y: requested.y - target_view.at.y,
    };
    let turn = requested_heading.map(|heading| heading - target_view.rotation);
    let retuned = match relation {
        ConstraintRelation::Offset { offset, rotation } => ConstraintRelation::Offset {
            offset: Vec2 {
                x: offset.x + delta.x,
                y: offset.y + delta.y,
            },
            rotation: turn.map_or(*rotation, |turn| rotation + turn),
        },
        ConstraintRelation::PartOffset { offset, rotation } => {
            let step = crate::matrix::splay::rotate(delta, -source_view.rotation, Vec2::default());
            ConstraintRelation::PartOffset {
                offset: Vec2 {
                    x: offset.x + step.x,
                    y: offset.y + step.y,
                },
                rotation: turn.map_or(*rotation, |turn| rotation + turn),
            }
        }
        ConstraintRelation::MirrorLine { .. } => {
            return Err(
                "The target is controlled by a mirror relation and cannot be dragged".into(),
            );
        }
    };
    let rotation = requested_heading.unwrap_or(target_view.rotation);
    let target_authority = target_frame.unproject(
        Pose2 {
            at: requested,
            rotation,
        },
        target_authority,
    )?;
    Ok(Retuned {
        relation: retuned,
        target_authority,
    })
}

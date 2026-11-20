use crate::model::{Constraint, Pose2, ProjectDoc, Vec2};
use endpoints::Side;
pub mod endpoints;
pub mod relation;
pub mod retune;
/// The refusal for a second driving record on one authority, worded once.
pub fn multiple_constraints(authority: &str) -> String {
    format!("Part {authority} has multiple constraints")
}

/// The one record that drives `part_id`'s geometry, found through whichever half is asked:
/// a part and its live mirror have one authority, so one driving record. Pure: nothing is
/// solved. More than one record on an authority is a located error rather than a pick.
///
/// Stored records keep their visible endpoint IDs, while the ownership compiler resolves their
/// current views and the solver orders them by authoritative target. Multiple records on one
/// authority are a located error rather than a pick.
pub fn driving_constraint<'a>(
    doc: &'a ProjectDoc,
    part_id: &str,
) -> Result<Option<&'a Constraint>, String> {
    let authority = endpoints::authoritative_part(doc, part_id).to_owned();
    let mut driving = None;
    for constraint in &doc.constraints {
        let owns_authority = match constraint {
            Constraint::Framed {
                id, source, target, ..
            } => {
                // Stored framed endpoints are authored bindings. Validate them against the live
                // correspondence instead of silently inferring a replacement from visible IDs.
                endpoints::resolve_endpoint(doc, id, Side::Source, source)?;
                let resolved_target = endpoints::resolve_endpoint(doc, id, Side::Target, target)?;
                resolved_target.authoritative_id == authority
            }
            _ => endpoints::authoritative_part(doc, constraint.target()) == authority,
        };
        if owns_authority && driving.replace(constraint).is_some() {
            return Err(multiple_constraints(&authority));
        }
    }
    Ok(driving)
}
pub fn follow_pose(source: Pose2, local: Pose2) -> Pose2 {
    let (sin, cos) = source.rotation.to_radians().sin_cos();
    Pose2 {
        at: Vec2 {
            x: source.at.x + local.at.x * cos - local.at.y * sin,
            y: source.at.y + local.at.x * sin + local.at.y * cos,
        },
        rotation: source.rotation + local.rotation,
    }
}

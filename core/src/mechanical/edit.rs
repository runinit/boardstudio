//! Applies one typed mechanical edit against the accepted document.
//!
//! Defaults, validation, linked-feature propagation and closure-clearance
//! projection live here, behind the mechanical module's seam. The Case adapter
//! submits a typed [`MechanicalEdit`] and the operation reports only the durable
//! ids whose values or memberships actually changed.
use crate::model::*;

use super::change::{MechanicalEdit, MechanicalTarget};

/// Applies one mechanical edit, returning the changed durable ids.
///
/// The full settings/feature/projection implementation lands here; until then
/// the operation is admitted but reports that it is unavailable.
pub fn apply(doc: &mut ProjectDoc, edit: &MechanicalEdit) -> Result<Vec<String>, String> {
    resolve_target(doc, &edit.target)?;
    Err("Mechanical edits are not available yet.".into())
}

/// Resolves an edit target to its board id and optional physical instance.
pub(crate) fn resolve_target<'a>(
    doc: &'a ProjectDoc,
    target: &MechanicalTarget,
) -> Result<(String, Option<&'a PhysicalBoardInstance>), String> {
    match target {
        MechanicalTarget::Board { board_id } => Ok((board_id.clone(), None)),
        MechanicalTarget::Instance { instance_id } => {
            let hardware = doc
                .hardware
                .as_ref()
                .ok_or("Physical instances are unavailable")?;
            let instance = hardware
                .instances
                .iter()
                .find(|instance| instance.id == *instance_id)
                .ok_or("The selected physical instance is unavailable")?;
            Ok((instance.board_id.clone(), Some(instance)))
        }
    }
}

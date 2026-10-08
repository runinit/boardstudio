//! Core effective mechanical inputs.
//!
//! The single read path that resolves the configuration a board or physical half
//! actually uses: canonical settings, the legacy shared overlay with
//! first-instance fallback, instance settings, board thickness, wireless battery
//! defaults and flipped-instance handling. Apply, evaluation, findings and
//! presentation all reuse it. Filled in by the effective-inputs slice.
use crate::model::*;

use super::change::MechanicalTarget;

/// The resolved mechanical configuration a target actually uses.
#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalInputs {
    pub board_id: String,
    pub configuration: MechanicalConfiguration,
}

/// Resolves the effective mechanical configuration for a board or physical half.
pub fn effective_inputs(
    document: &ProjectDoc,
    target: &MechanicalTarget,
) -> Result<MechanicalInputs, String> {
    let _ = (document, target);
    Err("Effective mechanical inputs are not available yet.".into())
}

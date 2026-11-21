//! Generated qualification in body-local coordinates, including native elevation and placement.

use crate::profile::{ExactModel, GeneratedAnalysisMesh};

mod cylinder_proof;

pub(super) fn qualified_bound(
    model: &ExactModel,
    mesh: &GeneratedAnalysisMesh,
) -> super::generated_qualification::GeneratedQualification {
    cylinder_proof::native::qualify(model, mesh)
}

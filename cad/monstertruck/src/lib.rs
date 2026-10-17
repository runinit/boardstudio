//! Exact BoardStudio case profiles backed by Monstertruck.

mod export;
pub use export::{StepBody, export_step, export_step_bodies};

mod mesh;
mod profile;

pub use mesh::MeshBuffers;
pub use profile::{
    ExactModel, ProfileInput, ProfileKind, ProfileRegion, build_profile, mesh_profile,
};

mod step;
pub use step::{
    ImportedStep, StepAccuracy, StepFaceOccurrence, StepLineOccurrence, StepLinePresentation,
    StepPrecision, StepSourceFace, StepUncertainty, import_step, mesh_imported_step,
};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod export_tests;

#[cfg(test)]
mod step_accuracy;

mod layered;

/// Builds exact case or PCB geometry from prepared millimeter inputs.
///
/// Returns an error for unsupported feature interactions or invalid geometry.
pub fn build_case(input: &boardstudio_contracts::PreparedCaseIR) -> Result<ExactModel, String> {
    layered::build_layered_case(input)
}

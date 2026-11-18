//! Renders the generator jobs of an export plan in process: net allocation,
//! model-path rewriting, arc upgrades and the split into footprints and board
//! objects, which the page used to delegate to a worker.
use boardstudio_footprints::bundled;
use boardstudio_footprints::export::export_forms;
use boardstudio_footprints::models::{model_bindings, preview_model_paths};
use boardstudio_footprints::nets::{NetAllocator, NetIndexer};
use boardstudio_footprints::types::{GeneratorRef, PartRef, ReservedNet as GeneratedNet};

use super::*;
use crate::generators::convert;

fn generator_error(error: boardstudio_footprints::GeneratorError) -> ArtifactError {
    validation(error.message())
}

fn conversion(error: String) -> ArtifactError {
    ArtifactError::new(ArtifactErrorCode::Internal, error)
}

/// One result per job, in plan order. Each result carries the net table as it
/// stood after that job, which `finish` checks against the previous one.
pub(super) fn render_jobs(plan: &ExportPlan) -> Result<Vec<RenderedJob>, ArtifactError> {
    let reserved: Vec<GeneratedNet> = convert(&plan.reserved_nets).map_err(conversion)?;
    let mut nets = NetAllocator::new(&reserved, plan.next_net_index);
    let mut results = Vec::with_capacity(plan.jobs.len());
    for job in &plan.jobs {
        let generator: GeneratorRef = convert(
            job.definition
                .generator
                .as_ref()
                .ok_or_else(|| validation("Generator job has no generator"))?,
        )
        .map_err(conversion)?;
        let part: PartRef = convert(&job.part).map_err(conversion)?;
        let forms = bundled()
            .render(
                &job.definition.id,
                &generator,
                Some(&part),
                &mut nets as &mut dyn NetIndexer,
            )
            .map_err(generator_error)?;
        let bindings = model_bindings(&forms).map_err(generator_error)?;
        let paths = preview_model_paths(&bindings, &plan.model_paths).map_err(generator_error)?;
        let exported = export_forms(forms, &paths).map_err(generator_error)?;
        let mut sources = Vec::new();
        for form in exported.footprints.iter().chain(&exported.objects) {
            sources.push(source::upgrade_legacy_arcs(form)?);
        }
        results.push(RenderedJob {
            snapshot_token: plan.snapshot_token.clone(),
            revision: plan.revision,
            job_id: job.job_id.clone(),
            source: sources.join("\n"),
            nets: convert(&nets.snapshot()).map_err(conversion)?,
        });
    }
    Ok(results)
}

/// Prepare, render and assemble a board preview in one step.
pub fn preview(request: PrepareExportRequest) -> Result<PcbPreview, ArtifactError> {
    let plan = planning::prepare_preview(request)?;
    let results = render_jobs(&plan)?;
    output::finish_preview(FinishExportRequest { plan, results })
}

/// Prepare, render and assemble KiCad files in one step.
pub fn export(request: PrepareExportRequest) -> Result<ExportArtifact, ArtifactError> {
    let plan = planning::prepare_export(request)?;
    let results = render_jobs(&plan)?;
    output::finish_export(FinishExportRequest { plan, results })
}

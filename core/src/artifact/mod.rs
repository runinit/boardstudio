pub mod compile;
pub mod kicad;
pub mod mechanical_extract;
mod mechanical_plate;
pub mod outline;
mod preview;
mod sexpr;
mod source;
mod source_geometry;

use crate::model::{
    ArtifactDiagnostic, ArtifactDiagnosticKind, ArtifactReply, ArtifactRequest, CompiledFootprint,
    FootprintCompileJob, PartDefinition,
};
pub use crate::model::{ArtifactError, ArtifactErrorCode};

/// Handle a stateless artifact operation without touching CoreEngine history or the document.
pub fn request(json: &str) -> String {
    let reply = match serde_json::from_str::<ArtifactRequest>(json) {
        Ok(request) => handle(request),
        Err(error) => {
            let id = serde_json::from_str::<serde_json::Value>(json)
                .ok()
                .and_then(|value| {
                    value
                        .get("id")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned)
                })
                .unwrap_or_default();
            ArtifactReply::Error {
                id,
                error: ArtifactError {
                    code: ArtifactErrorCode::ParseError,
                    message: error.to_string(),
                    diagnostics: vec![ArtifactDiagnostic {
                        kind: ArtifactDiagnosticKind::ParseError,
                        message: error.to_string(),
                        target_id: None,
                        source_start: None,
                        source_end: None,
                    }],
                },
            }
        }
    };
    serde_json::to_string(&reply).expect("artifact reply serializes")
}

fn handle(request: ArtifactRequest) -> ArtifactReply {
    match request {
        ArtifactRequest::ExportMechanicalPlate {
            id,
            document,
            contours,
        } => match mechanical_plate::export(&document, &contours) {
            Ok(result) => ArtifactReply::ExportMechanicalPlate { id, result },
            Err(error) => ArtifactReply::Error { id, error },
        },
        ArtifactRequest::ExtractMechanical {
            id,
            source,
            mappings,
            max_deviation_mm,
        } => {
            match mechanical_extract::extract(&source, &mappings).and_then(|geometry| {
                let plate_cutouts =
                    mechanical_extract::plate_cutout_contours(&geometry, max_deviation_mm)?;
                let clearance_envelopes =
                    mechanical_extract::clearance_envelopes(&geometry, max_deviation_mm)?;
                let pcb_holes = mechanical_extract::pcb_mounting_holes(&geometry)?;
                let source_geometry = mechanical_extract::profile_source(&source, &mappings)?;
                Ok(crate::model::MechanicalExtraction {
                    geometry,
                    plate_cutouts,
                    clearance_envelopes,
                    pcb_holes,
                    source_geometry,
                })
            }) {
                Ok(result) => ArtifactReply::ExtractMechanical { id, result },
                Err(error) => ArtifactReply::Error { id, error },
            }
        }
        ArtifactRequest::PreviewBoard {
            id,
            source,
            revision,
        } => match preview::board(&source, revision) {
            Ok(result) => ArtifactReply::PreviewBoard { id, result },
            Err(error) => ArtifactReply::Error { id, error },
        },
        ArtifactRequest::CompileFootprints { id, jobs } => compile_jobs(id, jobs),
        ArtifactRequest::ImportFootprint {
            id,
            definition_id,
            source,
        } => match source::import_footprint(&source, &definition_id) {
            Ok(result) => ArtifactReply::ImportFootprint { id, result },
            Err(error) => ArtifactReply::Error { id, error },
        },
        ArtifactRequest::PrepareExport { id, request } => match kicad::prepare_export(request) {
            Ok(result) => ArtifactReply::PrepareExport { id, result },
            Err(error) => ArtifactReply::Error { id, error },
        },
        ArtifactRequest::FinishExport { id, request } => match kicad::finish_export(request) {
            Ok(result) => ArtifactReply::FinishExport { id, result },
            Err(error) => ArtifactReply::Error { id, error },
        },
        ArtifactRequest::ExportOutline { id, request } => match outline::export_outline(request) {
            Ok(result) => ArtifactReply::ExportOutline { id, result },
            Err(error) => ArtifactReply::Error { id, error },
        },
    }
}

fn compile_jobs(id: String, jobs: Vec<FootprintCompileJob>) -> ArtifactReply {
    let compiled = jobs.iter().map(compile_job).collect::<Result<Vec<_>, _>>();
    match compiled {
        Ok(result) => ArtifactReply::CompileFootprints { id, result },
        Err(error) => ArtifactReply::Error { id, error },
    }
}

fn compile_job(job: &FootprintCompileJob) -> Result<CompiledFootprint, ArtifactError> {
    if let Some(source) = job.definition.kicad_source.as_ref() {
        validate_source_version(&job.definition)?;
        let mut compiled = source::import_footprint(&source.source, &job.definition.id)?;
        compiled.definition = job.definition.clone();
        compiled.geometry.side = job.side.clone();
        if job
            .definition
            .envelope_source
            .as_ref()
            .and_then(|source| source.courtyard.as_ref())
            == Some(&crate::model::EnvelopeOrigin::Authored)
        {
            compiled.geometry.courtyard = job.definition.courtyard.clone();
        }
        return Ok(compiled);
    }
    compile::compile_authored(&job.definition, job.side.clone())
        .map_err(|message| ArtifactError::new(ArtifactErrorCode::Validation, message))
}

pub(super) fn validate_source_version(definition: &PartDefinition) -> Result<(), ArtifactError> {
    if let Some(source) = &definition.kicad_source
        && source.format_version != 1
    {
        return Err(ArtifactError::new(
            ArtifactErrorCode::Unsupported,
            format!(
                "Unsupported KiCad source format version: {}",
                source.format_version
            ),
        ));
    }
    Ok(())
}

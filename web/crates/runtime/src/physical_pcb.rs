//! Source-backed PCB preparation shared by mechanical export and collision.
use crate::core_services::OpenScratchCore;
use boardstudio_core_kernel::hash::sha256_hex;
use boardstudio_model::model::{
    ArtifactReply, ArtifactRequest, Asset, CaseAssemblyIR, CaseIR, Contour, CoreReply, CoreRequest,
};
use boardstudio_web_host::cad_jobs::{CapturedPcbSource, PreparedCadSnapshot};

pub(crate) fn verify_source_digest(asset: &Asset, bytes: &[u8]) -> Result<(), String> {
    if sha256_hex(bytes) != asset.sha256 {
        return Err("Imported board asset bytes do not match the accepted SHA-256".into());
    }
    Ok(())
}

pub(crate) fn install_source_pcb(
    open_scratch: OpenScratchCore,
    request_prefix: &str,
    source: &CapturedPcbSource,
    bytes: Vec<u8>,
    prepared: &mut PreparedCadSnapshot,
) -> Result<Option<f64>, String> {
    let request_id = format!("{request_prefix}-pcb-source");
    let request = ArtifactRequest::ReadBoardSource {
        id: request_id.clone(),
        source: String::from_utf8(bytes)
            .map_err(|error| format!("PCB source is not valid UTF-8: {error}"))?,
        placement: source.placement,
        flipped: source.flipped,
    };
    let board = match artifact_reply(&request)? {
        ArtifactReply::ReadBoardSource { id, result } if id == request_id => result,
        _ => return Err("Core returned PCB source for another request.".into()),
    };
    let mut references = prepared
        .prepared
        .bodies
        .iter_mut()
        .filter(|body| body.body.id == "pcb-reference");
    let reference = match (references.next(), references.next()) {
        (Some(reference), None) => reference,
        _ => return Err("Prepared PCB reference is missing or ambiguous.".into()),
    };
    let mut body = reference.body.clone();
    if board
        .thickness
        .is_some_and(|thickness| thickness != body.thickness)
    {
        return Err("Imported PCB thickness does not match the mechanical PCB thickness.".into());
    }
    if board
        .pcb_source
        .as_ref()
        .is_none_or(|profile| profile.contours.is_empty())
        || board.contours.is_empty()
    {
        return Err("Core returned incomplete PCB source geometry.".into());
    }
    if body
        .features
        .as_ref()
        .is_some_and(|items| !items.is_empty())
        || body
            .openings
            .as_ref()
            .is_some_and(|items| !items.is_empty())
        || body.mounts.as_ref().is_some_and(|items| !items.is_empty())
        || body.gasket.is_some()
        || body.pcb_source.is_some()
    {
        return Err("Imported source cannot replace a modified nominal PCB reference.".into());
    }
    body.z = Some(body.z.unwrap_or(0.0) + source.reference.elevation);
    body.pcb_source = board.pcb_source;
    body.mounts = (!board.round_drills.is_empty()).then_some(board.round_drills);
    let mut contours = board.contours;
    contours.extend(
        board
            .holes
            .into_iter()
            .map(|points| Contour { points, hole: true }),
    );
    let source_ir = CaseIR {
        revision: prepared.identity.revision,
        body,
        contours,
    };
    let request_id = format!("{request_prefix}-pcb-prepare");
    let request = CoreRequest::PrepareCase {
        id: request_id.clone(),
        ir: CaseAssemblyIR {
            revision: source_ir.revision,
            bodies: vec![source_ir.clone()],
        },
    };
    let mut ir = match open_scratch()(request) {
        CoreReply::CasePrepared { id, ir } if id == request_id => ir,
        CoreReply::Error { id, message, .. } if id == request_id => return Err(message),
        _ => return Err("Core returned an unexpected source PCB preparation.".into()),
    };
    if ir.revision != prepared.identity.revision
        || ir.bodies.len() != 1
        || ir.bodies[0].revision != ir.revision
        || ir.bodies[0].body != source_ir.body
    {
        return Err("Core returned an incomplete or stale source PCB preparation.".into());
    }
    let mut pcb = ir.bodies.pop().expect("one source PCB body checked");
    // Compatibility polygons already contain these voids; exact CAD uses body mounts.
    for region in &mut pcb.regions {
        region.mounts.clear();
    }
    *reference = pcb;
    if let Some(assembly) = &mut prepared.mechanical_assembly {
        assembly.pcb_reference = Some(source_ir);
        for layer in assembly.stack.iter_mut().filter(|layer| layer.id == "pcb") {
            layer.z += source.reference.elevation;
        }
    }
    Ok(board.thickness)
}

pub(crate) fn artifact_reply(request: &ArtifactRequest) -> Result<ArtifactReply, String> {
    let request = serde_json::to_string(request).map_err(|error| error.to_string())?;
    let reply = serde_json::from_str(&boardstudio_core::artifact_request(&request))
        .map_err(|error| error.to_string())?;
    match reply {
        ArtifactReply::Error { error, .. } => {
            let mut message = error.message;
            for diagnostic in error.diagnostics {
                message.push(' ');
                message.push_str(&diagnostic.message);
            }
            Err(message)
        }
        reply => Ok(reply),
    }
}

//! Case readiness derived from a document and its accepted scene.
//!
//! These checks read only document and scene data, so hosts can gate Case work without
//! linking the document engine.

use crate::model::{ProjectDoc, SceneDelta, Scope, Severity};

/// Whether saved Case geometry for a board is current and usable as a Case
/// input with a current outer board contour, independently of PCB-coupled
/// `BoardReadiness::case_ready`.
///
/// This is a derived capability, not part of the serialized readiness
/// contract: authored Case geometry can be prepared/exported without claiming
/// that the board's PCB is ready.
pub fn authored_case_geometry_ready(
    document: &ProjectDoc,
    scene: &SceneDelta,
    board_id: &str,
) -> bool {
    if scene.revision != document.revision
        || !document.boards.iter().any(|board| board.id == board_id)
    {
        return false;
    }
    let Some(readiness) = scene
        .board_readiness
        .iter()
        .find(|readiness| readiness.board_id == board_id)
    else {
        return false;
    };
    if !readiness.outline
        || !scene
            .board_contours
            .iter()
            .find(|contours| contours.board_id == board_id)
            .is_some_and(|entry| entry.contours.iter().any(|contour| !contour.hole))
    {
        return false;
    }
    if !document
        .case_bodies
        .iter()
        .any(|body| body.board_id == board_id)
    {
        return false;
    }
    !scene.findings.iter().any(|finding| {
        finding.severity == Severity::Error
            && finding.scope == Scope::Case
            // Material references affect BOM metadata, not prepared solid
            // geometry; Core's case preparer intentionally accepts them.
            && !is_case_material_finding_id(&finding.id)
            && (finding.target_ids.is_empty()
                || finding.target_ids.iter().any(|target| {
                    target == board_id
                        || document.case_bodies.iter().any(|body| {
                            body.board_id == board_id && body.id.as_str() == target.as_str()
                        })
                }))
    })
}

/// Whether Case preparation may run for a selected board. A resolved
/// mechanical configuration remains sufficient; otherwise saved authored
/// Case geometry is admitted by its own capability above.
pub fn case_preparation_ready(
    document: &ProjectDoc,
    scene: &SceneDelta,
    board_id: &str,
    configured: bool,
) -> bool {
    scene
        .board_readiness
        .iter()
        .any(|readiness| readiness.board_id == board_id)
        && (configured || authored_case_geometry_ready(document, scene, board_id))
}

/// Id of the finding raised when a Case body references a missing material.
pub fn case_material_finding_id(body_id: &str) -> String {
    format!("case:{body_id}:material")
}

/// Whether a finding id was produced by [`case_material_finding_id`].
pub fn is_case_material_finding_id(id: &str) -> bool {
    id.ends_with(":material")
}

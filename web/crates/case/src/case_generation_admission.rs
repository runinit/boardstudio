//! Private page-action admission for Case generation.
//!
//! This is the page-level projection of the shared Core capability used again
//! by `cad_jobs::preparation_request`. Keep the presentation path allocation-
//! free because it evaluates during ordinary renders.

use boardstudio_core::model::{ProjectDoc, SceneDelta};

pub fn is_ready(
    document: &ProjectDoc,
    scene: &SceneDelta,
    board_id: &str,
    configured: bool,
) -> bool {
    boardstudio_core::case_preparation_ready(document, scene, board_id, configured)
}

/// Material id a new Case body may reference: only an existing PLA material,
/// never an id absent from the document.
pub fn default_case_material(document: &ProjectDoc) -> Option<String> {
    document
        .materials
        .iter()
        .find(|material| {
            material.id.eq_ignore_ascii_case("pla") || material.name.eq_ignore_ascii_case("pla")
        })
        .map(|material| material.id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::{
        Board, BoardContours, BoardReadiness, CaseBody, CaseKind, Contour, Finding, Readiness,
        Severity, Vec2,
    };

    #[test]
    fn authored_geometry_admits_generation_with_unresolved_material_metadata() {
        let scene = SceneDelta {
            revision: 9,
            transaction_id: String::new(),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![BoardContours {
                board_id: "left".into(),
                contours: vec![Contour {
                    points: vec![
                        Vec2 { x: 0.0, y: 0.0 },
                        Vec2 { x: 20.0, y: 0.0 },
                        Vec2 { x: 20.0, y: 15.0 },
                        Vec2 { x: 0.0, y: 15.0 },
                    ],
                    hole: false,
                }],
            }],
            board_readiness: vec![BoardReadiness {
                board_id: "left".into(),
                outline: true,
                pcb: true,
                case_ready: false,
            }],
            board_outline_scenes: vec![],
            module_scenes: vec![],
            finding_markers: vec![],
            findings: vec![Finding {
                id: "case:plate:material".into(),
                severity: Severity::Error,
                scope: boardstudio_core::model::Scope::Case,
                message: "Case material is missing".into(),
                target_ids: vec!["plate".into(), "pla".into()],
            }],
            readiness: Readiness {
                layout: true,
                outline: true,
                pcb: true,
                case_ready: false,
            },
        };

        let mut document = ProjectDoc::empty("fixture", "Fixture");
        document.revision = scene.revision;
        document.boards.push(Board {
            id: "left".into(),
            name: "Left".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.case_bodies.push(CaseBody {
            features: None,
            openings: None,
            id: "plate".into(),
            name: "Plate".into(),
            board_id: "left".into(),
            kind: CaseKind::Plate,
            thickness: 1.5,
            clearance: 0.0,
            material_id: Some("pla".into()),
            z: None,
            wall_height: None,
            wall_thickness: None,
            mounts: None,
            gasket: None,
        });

        // Authored geometry is admitted although board `case_ready` is false.
        assert!(!scene.board_readiness[0].case_ready);
        assert!(is_ready(&document, &scene, "left", false));
        assert!(!is_ready(&document, &scene, "right", false));

        // Stale scene revision is rejected.
        let mut stale = scene.clone();
        stale.revision += 1;
        assert!(!is_ready(&document, &stale, "left", false));

        // Missing outline readiness or outer contour is rejected.
        let mut no_outline = scene.clone();
        no_outline.board_readiness[0].outline = false;
        assert!(!is_ready(&document, &no_outline, "left", false));
        let mut no_contour = scene.clone();
        no_contour.board_contours.clear();
        assert!(!is_ready(&document, &no_contour, "left", false));

        // A non-material Case error on the body is rejected.
        let mut other_error = scene.clone();
        other_error.findings.push(Finding {
            id: "case:plate:gasket".into(),
            severity: Severity::Error,
            scope: boardstudio_core::model::Scope::Case,
            message: "Gasket inset, width, or depth is invalid".into(),
            target_ids: vec!["plate".into()],
        });
        assert!(!is_ready(&document, &other_error, "left", false));

        // A scene marked case_ready does not admit a board with no saved Case body.
        let mut case_ready_scene = scene.clone();
        case_ready_scene.board_readiness[0].case_ready = true;
        let mut bodiless = document.clone();
        bodiless.case_bodies.clear();
        assert!(!is_ready(&bodiless, &case_ready_scene, "left", false));
        // A resolved mechanical configuration remains sufficient.
        assert!(is_ready(&bodiless, &case_ready_scene, "left", true));

        // Missing board readiness is rejected even when configured.
        let mut missing_readiness_scene = scene;
        missing_readiness_scene.board_readiness.clear();
        assert!(!is_ready(&document, &missing_readiness_scene, "left", true));
    }

    #[test]
    fn default_case_body_does_not_reference_an_absent_material() {
        let mut document = ProjectDoc::empty("fixture", "Fixture");
        assert_eq!(default_case_material(&document), None);

        document.materials.push(boardstudio_core::model::Material {
            id: "pla-grade-a".into(),
            name: "PLA".into(),
            thickness: 1.75,
        });
        assert_eq!(
            default_case_material(&document).as_deref(),
            Some("pla-grade-a")
        );
    }
}

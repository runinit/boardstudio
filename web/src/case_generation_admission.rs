//! Private page-action admission for Case generation.
//!
//! This is the page-level projection of the shared Core capability used again
//! by `cad_jobs::preparation_request`. Keep the presentation path allocation-
//! free because it evaluates during ordinary renders.

use boardstudio_core::model::{ProjectDoc, SceneDelta};

pub(crate) fn is_ready(
    document: &ProjectDoc,
    scene: &SceneDelta,
    board_id: &str,
    configured: bool,
) -> bool {
    boardstudio_core::case_preparation_ready(document, scene, board_id, configured)
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::{
        Board, BoardContours, BoardReadiness, CaseBody, CaseKind, Contour, Readiness, Vec2,
    };

    #[test]
    fn authored_geometry_admits_generation_without_pcb_readiness() {
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
            findings: vec![],
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
            material_id: None,
            z: None,
            wall_height: None,
            wall_thickness: None,
            mounts: None,
            gasket: None,
        });

        assert!(is_ready(&document, &scene, "left", false));
        assert!(!is_ready(&document, &scene, "right", false));
        assert!(is_ready(&document, &scene, "left", true));

        let mut case_ready_scene = scene.clone();
        case_ready_scene.board_readiness[0].case_ready = true;
        document.case_bodies.clear();
        assert!(is_ready(&document, &case_ready_scene, "left", true));

        let mut missing_readiness_scene = scene;
        missing_readiness_scene.board_readiness.clear();
        assert!(!is_ready(&document, &missing_readiness_scene, "left", true));
    }
}

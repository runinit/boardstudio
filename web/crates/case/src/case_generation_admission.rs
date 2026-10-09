//! Private page-action admission for Case generation.
//!
//! This is the page-level projection of the shared Core capability used again
//! by `cad_jobs::preparation_request`. Keep the presentation path allocation-
//! free because it evaluates during ordinary renders.

use boardstudio_core::mechanical::{MechanicalTarget, effective_inputs};
use boardstudio_core::model::{ProjectDoc, SceneDelta, Scope, Severity};

/// Whether the selected target may generate Case output.
///
/// The shared prerequisites come from Core's board-level capability. On top of them the
/// selected target's own accepted findings decide: a Case-scope error that names the
/// selected half blocks only that half, while a finding naming only a sibling half of the
/// same board leaves this half ready. That is the same scoping the host's preparation
/// gate uses, and it is why two halves sharing one PCB keep independent readiness.
pub fn is_ready(
    document: &ProjectDoc,
    scene: &SceneDelta,
    board_id: &str,
    configured: bool,
    instance_id: Option<&str>,
) -> bool {
    if !boardstudio_core::case_preparation_ready(document, scene, board_id, configured) {
        return false;
    }
    let target = match instance_id {
        Some(instance_id) => MechanicalTarget::Instance {
            instance_id: instance_id.to_owned(),
        },
        None => MechanicalTarget::Board {
            board_id: board_id.to_owned(),
        },
    };
    // A target whose effective inputs cannot be resolved is not admitted.
    match effective_inputs(document, &scene.board_contours, &target) {
        Ok(inputs) => !inputs
            .findings
            .iter()
            .any(|finding| finding.severity == Severity::Error && finding.scope == Scope::Case),
        Err(_) => false,
    }
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
        Board, BoardContours, BoardReadiness, CaseBody, CaseKind, Contour, Finding, Readiness, Vec2,
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
        assert!(is_ready(&document, &scene, "left", false, None));
        assert!(!is_ready(&document, &scene, "right", false, None));

        // Stale scene revision is rejected.
        let mut stale = scene.clone();
        stale.revision += 1;
        assert!(!is_ready(&document, &stale, "left", false, None));

        // Missing outline readiness or outer contour is rejected.
        let mut no_outline = scene.clone();
        no_outline.board_readiness[0].outline = false;
        assert!(!is_ready(&document, &no_outline, "left", false, None));
        let mut no_contour = scene.clone();
        no_contour.board_contours.clear();
        assert!(!is_ready(&document, &no_contour, "left", false, None));

        // A non-material Case error on the body is rejected.
        let mut other_error = scene.clone();
        other_error.findings.push(Finding {
            id: "case:plate:gasket".into(),
            severity: Severity::Error,
            scope: boardstudio_core::model::Scope::Case,
            message: "Gasket inset, width, or depth is invalid".into(),
            target_ids: vec!["plate".into()],
        });
        assert!(!is_ready(&document, &other_error, "left", false, None));

        // A scene marked case_ready does not admit a board with no saved Case body.
        let mut case_ready_scene = scene.clone();
        case_ready_scene.board_readiness[0].case_ready = true;
        let mut bodiless = document.clone();
        bodiless.case_bodies.clear();
        assert!(!is_ready(&bodiless, &case_ready_scene, "left", false, None));
        // A resolved mechanical configuration remains sufficient.
        assert!(is_ready(&bodiless, &case_ready_scene, "left", true, None));

        // Missing board readiness is rejected even when configured.
        let mut missing_readiness_scene = scene;
        missing_readiness_scene.board_readiness.clear();
        assert!(!is_ready(
            &document,
            &missing_readiness_scene,
            "left",
            true,
            None
        ));
    }

    /// A pair whose linked mount lands outside one half's own outline.
    ///
    /// The finding names that half, so it must block only that half's Case output;
    /// each half keeps its own readiness even when both share one PCB.
    fn conflicted_pair_scene() -> (ProjectDoc, SceneDelta) {
        let mut document = ProjectDoc::empty("fixture", "Fixture");
        document.revision = 3;
        document.boards.push(Board {
            id: "main-board".into(),
            name: "Main board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let configuration = || {
            serde_json::from_value::<boardstudio_core::model::MechanicalConfiguration>(
                serde_json::json!({
                    "boardId": "main-board", "method": "printed", "mount": "tray",
                    "plateThickness": 1.5, "plateFoamThickness": 0.5, "pcbThickness": 1.6,
                    "bottomFoamThickness": 2.0, "batteryHeight": 0.0, "bottomThickness": 3.0,
                    "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3,
                    "mounts": [], "closureMounts": [], "profiles": []
                }),
            )
            .expect("the mechanical configuration parses")
        };
        document.hardware = Some(
            serde_json::from_value(serde_json::json!({
                "topology": "split", "transport": "none",
                "instances": [
                    {
                        "id": "left", "name": "Left", "boardId": "main-board", "half": "left",
                        "role": "central", "flipped": false,
                        "mechanical": configuration(),
                        "constructionOwnership": "pair"
                    },
                    {
                        "id": "right", "name": "Right", "boardId": "main-board", "half": "right",
                        "role": "peripheral", "flipped": false,
                        "mechanical": configuration(),
                        "constructionOwnership": "pair"
                    }
                ],
                "boards": [],
                "sharedConstruction": configuration(),
                "pairs": [{
                    "id": "pair-left-right",
                    "members": ["left", "right"],
                    // A mapping that carries the counterpart well outside the shared
                    // board, so the right half's linked mount cannot fit its outline.
                    "mapping": { "at": { "x": 100.0, "y": 0.0 }, "rotation": 0.0 },
                    "construction": {
                        "method": "printed", "mount": "tray", "integratedPlateFrame": false,
                        "bottomStyle": "shell", "middleFrame": false,
                        "plateThickness": 1.5, "plateFoamThickness": 0.5,
                        "bottomFoamThickness": 2.0, "bottomThickness": 3.0,
                        "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3
                    },
                    "features": [{
                        "id": "mount-shared",
                        "kind": "mount",
                        "author": "left",
                        "value": {
                            "kind": "mount",
                            "mount": {
                                "id": "mount-shared", "at": { "x": 2.0, "y": 3.0 },
                                "kind": "hole", "holeDiameter": 2.2
                            }
                        },
                        "members": [
                            { "instanceId": "left", "featureId": "mount-left", "state": "linked" },
                            { "instanceId": "right", "featureId": "mount-right", "state": "linked" }
                        ]
                    }]
                }]
            }))
            .expect("the paired fixture parses"),
        );
        let scene = SceneDelta {
            revision: document.revision,
            transaction_id: String::new(),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![BoardContours {
                board_id: "main-board".into(),
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
                board_id: "main-board".into(),
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
        (document, scene)
    }

    #[test]
    fn a_case_conflict_naming_one_half_blocks_only_that_half() {
        let (document, scene) = conflicted_pair_scene();
        // The conflicted half is not admitted...
        assert!(
            !is_ready(&document, &scene, "main-board", true, Some("right")),
            "the half whose linked feature cannot fit its outline is not admitted"
        );
        // ...while the half the finding does not name keeps its own readiness.
        assert!(
            is_ready(&document, &scene, "main-board", true, Some("left")),
            "a conflict naming only the sibling half cannot block this half"
        );
        // A board-level reading covers every half of that board.
        assert!(
            !is_ready(&document, &scene, "main-board", true, None),
            "the board-level admission covers the conflicted half"
        );
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

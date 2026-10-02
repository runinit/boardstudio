//! Private page-action admission for Case generation.
//!
//! This mirrors `cad_jobs::preparation_request` until RF-003 replaces both
//! checks with a shared capability contract. Keep it allocation-free: the
//! presentation evaluates it during ordinary renders.

use boardstudio_core::model::SceneDelta;

pub(crate) fn is_ready(scene: &SceneDelta, board_id: &str, configured: bool) -> bool {
    let readiness = scene
        .board_readiness
        .iter()
        .find(|item| item.board_id == board_id);
    readiness.is_some_and(|item| configured || item.case_ready)
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::{BoardReadiness, Readiness};

    #[test]
    fn unconfigured_unready_physical_scope_disables_case_generation() {
        let scene = SceneDelta {
            revision: 9,
            transaction_id: String::new(),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![],
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

        assert!(!is_ready(&scene, "left", false));
        assert!(!is_ready(&scene, "right", false));
        assert!(is_ready(&scene, "left", true));

        let mut case_ready_scene = scene.clone();
        case_ready_scene.board_readiness[0].case_ready = true;
        assert!(is_ready(&case_ready_scene, "left", false));

        let mut missing_readiness_scene = scene;
        missing_readiness_scene.board_readiness.clear();
        assert!(!is_ready(&missing_readiness_scene, "left", true));
    }
}

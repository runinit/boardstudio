//! Native tests for the outline action planner interface.

use crate::outline::planner::{OutlineAction, Skip, plan_action};
use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_core::model::{ProjectDoc, Readiness, SceneDelta};
use boardstudio_web_ui_model::tree::TreeContext;
use std::sync::Arc;

#[test]
fn planning_retires_activation_when_outline_version_is_gone() {
    let mut document = ProjectDoc::empty("outline-planner", "Outline planner");
    document.boards.push(boardstudio_core::model::Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let snapshot = AcceptedSnapshot {
        token: boardstudio_application::SnapshotToken(1),
        session_epoch: boardstudio_application::SessionEpoch(1),
        document: Arc::new(document),
        scene: Arc::new(SceneDelta {
            module_scenes: vec![],
            revision: 0,
            transaction_id: String::new(),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![],
            board_readiness: vec![],
            board_outline_scenes: vec![],
            finding_markers: vec![],
            findings: vec![],
            readiness: Readiness {
                layout: false,
                outline: false,
                pcb: false,
                case_ready: false,
            },
        }),
    };
    let action = OutlineAction::Activate {
        scope: Scope {
            session_epoch: boardstudio_application::SessionEpoch(1),
            document_id: "outline-planner".into(),
            board_id: "board".into(),
            instance_id: None,
        },
        token: snapshot.token,
        revision: 0,
        generation: 1,
        context: TreeContext::OutlineVersion {
            board_id: "board".into(),
            version_id: Some("deleted-version".into()),
        },
        require_selected_version: false,
        board_id: "board".into(),
        version_id: Some("deleted-version".into()),
    };

    assert!(matches!(
        plan_action(&snapshot, &action, 1),
        Err(Skip::Retire(reason)) if reason == "That outline version no longer exists."
    ));
}

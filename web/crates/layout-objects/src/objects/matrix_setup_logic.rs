//! Host-compatible matrix-setup resolver for creating a prepared matrix.
//!
//! The production controller (wasm) delegates to this module so the same
//! resolver runs under `cargo test` without a browser.

use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution};
use boardstudio_model::model::{EditOperation, Matrix, PartDefinition};

pub(super) fn create_matrix_resolver(
    board_id: String,
    matrix: Matrix,
    definitions: Vec<PartDefinition>,
) -> EditResolver {
    EditResolver::new("layout-matrix-setup", move |accepted: &AcceptedSnapshot| {
        let document = &accepted.document;
        if !document.boards.iter().any(|board| board.id == board_id) {
            return Resolution::Retire("The board no longer exists.".into());
        }
        if document
            .matrices
            .iter()
            .any(|existing| existing.id == matrix.id)
        {
            return Resolution::Retire("The new matrix identity is already in use.".into());
        }
        if definitions.iter().any(|definition| {
            document
                .definitions
                .iter()
                .any(|existing| existing.id == definition.id)
        }) {
            return Resolution::Retire(
                "The prepared matrix definitions conflict with the accepted document.".into(),
            );
        }
        if document.hardware.as_ref().is_some_and(|hardware| {
            hardware.topology == boardstudio_model::model::HardwareTopology::Split
        }) {
            let ids = boardstudio_web_layout_core::mirrored_pair_geometry::MirroredPairIds {
                right_matrix_id: format!("{}-right", matrix.id),
                left_layout_id: format!("{}-left-layout", matrix.id),
                right_layout_id: format!("{}-right-layout", matrix.id),
            };
            if document
                .matrices
                .iter()
                .any(|item| item.id == ids.right_matrix_id)
                || document
                    .layouts
                    .iter()
                    .any(|item| item.id == ids.left_layout_id || item.id == ids.right_layout_id)
            {
                return Resolution::Retire(
                    "The mirrored layout identity is already in use.".into(),
                );
            }
            let pair =
                match boardstudio_web_layout_core::mirrored_pair_geometry::project_mirrored_pair(
                    matrix.clone(),
                    &ids,
                    &boardstudio_web_layout_core::mirrored_pair_geometry::MirroredPairGeometryInput {
                        left_name: "Left half".into(),
                        right_name: "Right half".into(),
                        gap_mm: 40.0,
                    },
                    matrix.origin,
                ) {
                    Ok(pair) => pair,
                    Err(error) => return Resolution::Retire(error),
                };
            return Resolution::submit(
                vec![
                    matrix.id.clone(),
                    ids.right_matrix_id,
                    ids.left_layout_id,
                    ids.right_layout_id,
                ],
                EditOperation::CreateMirroredPair {
                    left: pair.left,
                    right: pair.right,
                    matrix: Box::new(pair.matrix),
                    definitions: Some(definitions.clone()),
                },
            );
        }
        Resolution::submit(
            vec![matrix.id.clone()],
            EditOperation::SetMatrix {
                matrix: matrix.clone(),
                definitions: Some(definitions.clone()),
            },
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_model::model::*;
    use std::sync::Arc;

    #[test]
    fn split_setup_creates_a_linked_layout_pair() {
        let mut document = ProjectDoc::empty("setup", "Setup");
        document.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Split,
            ..Default::default()
        });
        let matrix: Matrix = serde_json::from_value(serde_json::json!({
            "id": "main", "rows": 3, "columns": 5, "pitch": {"x": 19.05, "y": 19.05},
            "origin": {"x": 0.0, "y": 0.0}, "definitionId": "switch", "partIds": [], "boardId": "board"
        }))
        .unwrap();
        let snapshot = AcceptedSnapshot {
            token: boardstudio_application::SnapshotToken(1),
            session_epoch: boardstudio_application::SessionEpoch(1),
            document: Arc::new(document),
            scene: Arc::new(SceneDelta {
                module_scenes: vec![],
                revision: 0,
                transaction_id: "setup".into(),
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
        let Resolution::Submit(command) =
            create_matrix_resolver("board".into(), matrix, vec![]).resolve(&snapshot)
        else {
            panic!("setup must submit");
        };
        let EditOperation::CreateMirroredPair {
            left,
            right,
            matrix,
            ..
        } = command.operation
        else {
            panic!("split setup must create linked layouts, not a single matrix");
        };
        let link = right.mirror_link.as_ref().unwrap();
        assert_eq!(link.source_id, left.id);
        assert_ne!(left.matrix_ids, right.matrix_ids);
        assert!(matrix.origin.x < link.axis_x);
        let gap = 2.0 * (link.axis_x - matrix.origin.x) - (matrix.pitch.x - 1.0);
        assert!((gap - 40.0).abs() < 1e-9);
    }
}

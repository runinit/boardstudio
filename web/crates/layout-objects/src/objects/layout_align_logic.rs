use boardstudio_model::model::{EditOperation, ProjectDoc, Vec2};
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_model::tree::TreeContext;

pub(super) fn workspace_route_matches(current: Workspace, owner: Workspace) -> bool {
    matches!(owner, Workspace::Layout | Workspace::Pcb) && current == owner
}

pub(super) fn make_edit(
    document: &ProjectDoc,
    context: &TreeContext,
    moving_ids: &[String],
    delta: Vec2,
) -> Option<(EditOperation, Vec<String>)> {
    match context {
        TreeContext::Matrix { matrix_id } => {
            document
                .matrices
                .iter()
                .find(|matrix| matrix.id == *matrix_id)?;
            Some((
                EditOperation::SetMatrixPlacement {
                    matrix_id: matrix_id.clone(),
                    change: boardstudio_model::model::MatrixPlacementChange::MatrixTranslation {
                        delta,
                    },
                },
                vec![matrix_id.clone()],
            ))
        }
        TreeContext::Column { matrix_id, column } => {
            document
                .matrices
                .iter()
                .find(|matrix| matrix.id == *matrix_id)?;
            Some((
                EditOperation::SetMatrixPlacement {
                    matrix_id: matrix_id.clone(),
                    change: boardstudio_model::model::MatrixPlacementChange::ColumnTranslation {
                        column: *column,
                        delta,
                    },
                },
                vec![matrix_id.clone()],
            ))
        }
        TreeContext::Row { .. } => None,
        TreeContext::Key { .. } | TreeContext::Component { .. } => {
            let positions = moving_ids
                .iter()
                .map(|id| {
                    let part = document.parts.iter().find(|part| part.id == *id)?;
                    Some(boardstudio_model::model::Position {
                        id: id.clone(),
                        at: Vec2 {
                            x: part.pose.at.x + delta.x,
                            y: part.pose.at.y + delta.y,
                        },
                    })
                })
                .collect::<Option<Vec<_>>>()?;
            Some((EditOperation::MoveParts { positions }, moving_ids.to_vec()))
        }
        TreeContext::Board { .. }
        | TreeContext::LayoutGroup { .. }
        | TreeContext::Outline { .. }
        | TreeContext::OutlineVersion { .. }
        | TreeContext::Bridge { .. }
        | TreeContext::MountedModule { .. } => None,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use boardstudio_model::model::{
        Board, Layout, Matrix, MatrixCell, MatrixPlacementChange, OutlineFeature, OutlineSettings,
        Part, Pose2, Side,
    };

    #[test]
    fn align_route_is_exactly_scoped_to_its_workspace_owner() {
        // (workspace, owner, admitted): admission is exactly scoped to the owning workspace.
        for (workspace, owner, admitted) in [
            (Workspace::Layout, Workspace::Layout, true),
            (Workspace::Pcb, Workspace::Pcb, true),
            (Workspace::Pcb, Workspace::Layout, false),
            (Workspace::Layout, Workspace::Pcb, false),
            (Workspace::Parts, Workspace::Parts, false),
            (Workspace::Layout, Workspace::Parts, false),
        ] {
            assert_eq!(
                workspace_route_matches(workspace, owner),
                admitted,
                "route ({workspace}, {owner})"
            );
        }
    }

    fn switch_definition() -> serde_json::Value {
        serde_json::json!({
            "id": "switch:base",
            "name": "MX switch",
            "kind": "switch",
            "courtyard": [{"x": -3.0, "y": -3.0}, {"x": 3.0, "y": -3.0}, {"x": 3.0, "y": 3.0}],
            "pads": []
        })
    }

    pub(crate) fn key_part(id: &str, reference: &str, x: f64, y: f64) -> Part {
        Part {
            keycap: None,
            outline: None,
            anchor: None,
            id: id.into(),
            definition_id: "switch:base".into(),
            reference: reference.into(),
            pose: Pose2 {
                at: Vec2 { x, y },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        }
    }

    pub(crate) fn bent_column_document() -> ProjectDoc {
        let mut document = ProjectDoc::empty("project", "Keyboard");
        document.outline.push(OutlineFeature::PartEnvelope {
            connections: vec![],
            settings: OutlineSettings::default(),
            id: "envelope-main".into(),
            part_ids: vec![],
            margin: 4.0,
            operation: boardstudio_model::model::Operation::Add,
        });
        document.boards.push(Board {
            id: "board-main".into(),
            name: "Main".into(),
            outline_ids: vec!["envelope-main".into()],
            part_ids: vec![
                "matrix/m/r0c0".into(),
                "matrix/m/r1c0".into(),
                "ref-part".into(),
            ],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document
            .definitions
            .push(serde_json::from_value(switch_definition()).unwrap());
        document
            .parts
            .push(key_part("matrix/m/r0c0", "SW1", 0.0, 0.0));
        document
            .parts
            .push(key_part("matrix/m/r1c0", "SW2", 0.0, 19.05));
        document.parts.push(key_part("ref-part", "REF", 40.0, 0.0));
        document.matrices.push(Matrix {
            id: "m".into(),
            name: None,
            rows: 2,
            columns: 1,
            pitch: Vec2 { x: 19.05, y: 19.05 },
            origin: Vec2::default(),
            definition_id: "switch:base".into(),
            part_ids: vec!["matrix/m/r0c0".into(), "matrix/m/r1c0".into()],
            board_id: Some("board-main".into()),
            mirror: None,
            rotation: None,
            edge_gap: None,
            diode_direction: None,
            row_layout: vec![],
            column_layout: vec![],
            anchor: None,
            cells: vec![
                MatrixCell {
                    shift: None,
                    bend: None,
                    deleted: false,
                    row: 0,
                    column: 0,
                    enabled: true,
                    definition_id: None,
                    variant: None,
                    offset: None,
                    rotation: None,
                    assemblies: vec![],
                    assemblies_local: None,
                },
                MatrixCell {
                    shift: None,
                    bend: None,
                    deleted: false,
                    row: 1,
                    column: 0,
                    enabled: true,
                    definition_id: None,
                    variant: None,
                    offset: None,
                    rotation: None,
                    assemblies: vec![],
                    assemblies_local: None,
                },
            ],
        });
        document.layouts.push(Layout {
            id: "layout".into(),
            name: "Main".into(),
            board_id: "board-main".into(),
            matrix_ids: vec!["m".into()],
            part_ids: vec!["ref-part".into()],
            mirror_link: None,
        });
        document
    }

    /// The Core operation family `make_edit` submits for each alignable context.
    #[derive(Debug, PartialEq)]
    enum EditFamily {
        ColumnTranslation,
        MatrixTranslation,
        None,
        MoveParts,
    }

    fn edit_family(edit: &Option<(EditOperation, Vec<String>)>) -> EditFamily {
        match edit {
            Some((
                EditOperation::SetMatrixPlacement {
                    change: MatrixPlacementChange::ColumnTranslation { .. },
                    ..
                },
                _,
            )) => EditFamily::ColumnTranslation,
            Some((
                EditOperation::SetMatrixPlacement {
                    change: MatrixPlacementChange::MatrixTranslation { .. },
                    ..
                },
                _,
            )) => EditFamily::MatrixTranslation,
            Some((EditOperation::MoveParts { .. }, _)) => EditFamily::MoveParts,
            _ => EditFamily::None,
        }
    }

    /// Ticket 22: the Align consumer submits Core's exact edits, never client
    /// column offsets. Column goes through `ColumnTranslation`, Matrix through its
    /// frame, Row stays unavailable, and keys keep the `MoveParts` pose route.
    #[test]
    fn align_submits_the_exact_core_edits() {
        let document = bent_column_document();
        let delta = Vec2 { x: -3.0, y: 1.5 };
        let moving = vec!["matrix/m/r0c0".into(), "matrix/m/r1c0".into()];
        // (context, moving subset, expected Core operation family).
        let cases: Vec<(TreeContext, Vec<String>, EditFamily)> = vec![
            (
                TreeContext::Column {
                    matrix_id: "m".into(),
                    column: 0,
                },
                moving.clone(),
                EditFamily::ColumnTranslation,
            ),
            (
                TreeContext::Matrix {
                    matrix_id: "m".into(),
                },
                moving.clone(),
                EditFamily::MatrixTranslation,
            ),
            (
                TreeContext::Row {
                    matrix_id: "m".into(),
                    row: 0,
                },
                moving.clone(),
                EditFamily::None,
            ),
            (
                TreeContext::Key {
                    matrix_id: "m".into(),
                    row: 0,
                    column: 0,
                },
                moving[..1].to_vec(),
                EditFamily::MoveParts,
            ),
        ];
        for (context, moving, expected) in &cases {
            let edit = make_edit(&document, context, moving, delta);
            assert_eq!(
                edit_family(&edit),
                *expected,
                "{context:?} must route through {expected:?}, got {edit:?}"
            );
        }
        // A bent column must Align rigidly with its exact translation and target.
        match make_edit(&document, &cases[0].0, &cases[0].1, delta) {
            Some((
                EditOperation::SetMatrixPlacement {
                    matrix_id,
                    change: MatrixPlacementChange::ColumnTranslation { column, delta },
                },
                targets,
            )) => {
                assert_eq!(matrix_id, "m");
                assert_eq!(column, 0);
                assert_eq!(delta, Vec2 { x: -3.0, y: 1.5 });
                assert_eq!(targets, vec!["m".to_owned()]);
            }
            other => panic!("a bent column must Align rigidly, got {other:?}"),
        }
        // Keys keep the MoveParts pose route with the exact translated pose.
        match make_edit(&document, &cases[3].0, &cases[3].1, delta) {
            Some((EditOperation::MoveParts { positions }, _)) => {
                assert_eq!(positions.len(), 1);
                assert_eq!(positions[0].at, Vec2 { x: -3.0, y: 1.5 });
            }
            other => panic!("keys keep the MoveParts pose route, got {other:?}"),
        }
    }
}

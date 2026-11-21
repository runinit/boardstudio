//! The Case Inspector's summary of the tree-owned PCB part selection.
use boardstudio_application::{ReadModel, Scope};
use boardstudio_web_ui_model::tree::ScopedTreeContext;

/// Host-compatible copy of the ui-model selection currency check. The ui-model
/// selection module owns the presentation copy (wasm-only through its Dioxus
/// adapter); the Case summary keeps this copy so its unit tests run under
/// `cargo test`. Resolvability is covered by the `resolve_selection` step below:
/// an unresolvable context resolves to `None` there too.
fn selection_context_is_current(
    model: &ReadModel,
    scope: &Scope,
    _context: &crate::objects::TreeContext,
) -> bool {
    model.active_board_id == scope.board_id
        && model.active_instance_id == scope.instance_id
        && model.accepted.as_ref().is_some_and(|snapshot| {
            snapshot.session_epoch == scope.session_epoch
                && snapshot.document.id == scope.document_id
        })
}

pub struct SelectedPartSummary {
    pub title: String,
    pub breadcrumb: String,
    pub selected_count: usize,
}

/// Project the current tree-owned PCB selection for the Case Inspector.
/// Requiring the resolved context to equal the accepted selection prevents a
/// stale tree context from describing a different active selection.
pub fn selected_part_summary(
    model: &ReadModel,
    selected: Option<&ScopedTreeContext>,
) -> Option<SelectedPartSummary> {
    let selected = selected?;
    if !matches!(
        &selected.context,
        crate::objects::TreeContext::Key { .. } | crate::objects::TreeContext::Component { .. }
    ) {
        return None;
    }
    if !selection_context_is_current(model, &selected.scope, &selected.context) {
        return None;
    }
    let ids = crate::objects::resolve_selection(model, &selected.context)?;
    if ids.is_empty() || ids != model.selected_part_ids {
        return None;
    }
    let document = &model.accepted.as_ref()?.document;
    let board = document
        .boards
        .iter()
        .find(|board| board.id == model.active_board_id)?;
    let references = ids
        .iter()
        .map(|id| {
            document
                .parts
                .iter()
                .find(|part| part.id == *id)
                .map(|part| part.reference.clone())
        })
        .collect::<Option<Vec<_>>>()?;
    let matrix_id = match &selected.context {
        crate::objects::TreeContext::Key { matrix_id, .. } => Some(matrix_id.as_str()),
        crate::objects::TreeContext::Component { matrix_id, .. } => matrix_id.as_deref(),
        _ => None,
    };
    let context_name = matrix_id
        .and_then(|matrix_id| {
            document
                .layouts
                .iter()
                .find(|layout| {
                    layout.board_id == model.active_board_id
                        && layout.matrix_ids.iter().any(|id| *id == matrix_id)
                        && !layout.name.trim().is_empty()
                })
                .map(|layout| layout.name.clone())
                .or_else(|| {
                    document
                        .matrices
                        .iter()
                        .find(|matrix| matrix.id == matrix_id)
                        .and_then(|matrix| matrix.name.as_deref())
                        .map(str::trim)
                        .filter(|name| !name.is_empty())
                        .map(str::to_owned)
                })
        })
        .or_else(|| references.first().cloned());
    Some(SelectedPartSummary {
        title: references.join(", "),
        breadcrumb: context_name
            .map(|name| format!("{} / Case / {name}", board.name))
            .unwrap_or_else(|| format!("{} / Case", board.name)),
        selected_count: references.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::SessionEpoch;
    use boardstudio_model::model::{Board, Part, Pose2, ProjectDoc, Side, Vec2};
    use std::sync::Arc;

    fn snapshot(document: ProjectDoc) -> ReadModel {
        let scene: boardstudio_model::model::SceneDelta =
            serde_json::from_value(serde_json::json!({
                "revision": 0,
                "transactionId": "fixture",
                "changedIds": [],
                "transforms": [],
                "matrixScenes": [],
                "contours": [],
                "boardContours": [],
                "boardReadiness": [],
                "findings": [],
                "readiness": {"layout": true, "outline": true, "pcb": true, "case": false}
            }))
            .expect("fixture scene");
        ReadModel {
            accepted: Some(boardstudio_application::AcceptedSnapshot {
                session_epoch: SessionEpoch(1),
                token: boardstudio_application::SnapshotToken(7),
                document: Arc::new(document),
                scene: Arc::new(scene),
            }),
            active_board_id: "board-1".into(),
            selected_part_ids: vec!["part-1".into()],
            ..ReadModel::default()
        }
    }

    fn document() -> ProjectDoc {
        let mut document = ProjectDoc::empty("project", "Keyboard");
        document.boards.push(Board {
            id: "board-1".into(),
            name: "Main".into(),
            outline_ids: vec![],
            part_ids: vec!["part-1".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.parts.push(Part {
            id: "part-1".into(),
            definition_id: "switch:base".into(),
            reference: "SW1".into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            keycap: None,
            outline: None,
            anchor: None,
            locked: None,
            properties: None,
            generator_parameters: None,
        });
        document
    }

    fn scope() -> boardstudio_application::Scope {
        boardstudio_application::Scope {
            session_epoch: SessionEpoch(1),
            document_id: "project".into(),
            board_id: "board-1".into(),
            instance_id: None,
        }
    }

    fn component_selection() -> ScopedTreeContext {
        ScopedTreeContext {
            scope: scope(),
            context: crate::objects::TreeContext::Component {
                part_id: Some("part-1".into()),
                matrix_id: None,
                row: None,
                column: None,
                assembly_id: None,
            },
        }
    }

    #[test]
    fn component_selection_projects_title_and_breadcrumb() {
        let model = snapshot(document());
        let summary = selected_part_summary(&model, Some(&component_selection())).expect("summary");
        assert_eq!(summary.title, "SW1");
        assert_eq!(summary.breadcrumb, "Main / Case / SW1");
        assert_eq!(summary.selected_count, 1);
    }

    #[test]
    fn non_key_component_contexts_are_ignored() {
        let model = snapshot(document());
        for context in [
            crate::objects::TreeContext::Matrix {
                matrix_id: "m".into(),
            },
            crate::objects::TreeContext::Board {
                board_id: "board-1".into(),
            },
            crate::objects::TreeContext::Row {
                matrix_id: "m".into(),
                row: 0,
            },
        ] {
            let selected = ScopedTreeContext {
                scope: scope(),
                context,
            };
            assert!(
                selected_part_summary(&model, Some(&selected)).is_none(),
                "non-key/component contexts stay silent"
            );
        }
        assert!(selected_part_summary(&model, None).is_none());
    }

    #[test]
    fn stale_scope_or_mismatched_selection_returns_none() {
        let model = snapshot(document());
        // Wrong board scope.
        let mut elsewhere = component_selection();
        elsewhere.scope.board_id = "board-2".into();
        assert!(selected_part_summary(&model, Some(&elsewhere)).is_none());
        // Selection no longer matches the resolved ids.
        let mut diverged = model.clone();
        diverged.selected_part_ids = vec!["part-2".into()];
        assert!(selected_part_summary(&diverged, Some(&component_selection())).is_none());
        // Empty selection.
        let mut empty = model.clone();
        empty.selected_part_ids = vec![];
        assert!(selected_part_summary(&empty, Some(&component_selection())).is_none());
    }

    #[test]
    fn unknown_part_and_missing_board_return_none() {
        let model = snapshot(document());
        // Unknown part id resolves to nothing.
        let mut unknown = component_selection();
        unknown.context = crate::objects::TreeContext::Component {
            part_id: Some("missing".into()),
            matrix_id: None,
            row: None,
            column: None,
            assembly_id: None,
        };
        assert!(selected_part_summary(&model, Some(&unknown)).is_none());
        // No accepted document or board means no summary.
        let mut empty = model.clone();
        empty.accepted = None;
        assert!(selected_part_summary(&empty, Some(&component_selection())).is_none());
    }
}

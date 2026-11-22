use boardstudio_application::ReadModel;
use boardstudio_web_ui_model::{state::LayoutOwnerIdentity, tree as objects};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReturnTarget {
    pub owner: LayoutOwnerIdentity,
    pub selection: objects::ScopedTreeContext,
    pub destination: objects::TreeContext,
}

pub fn layout_finding_return_is_current(
    model: &ReadModel,
    owner: &LayoutOwnerIdentity,
    current_selection: Option<&objects::ScopedTreeContext>,
    target: &ReturnTarget,
) -> bool {
    if owner != &target.owner || owner.workspace != "Layout" {
        return false;
    }
    let Some(scope) = owner.scope.as_ref() else {
        return false;
    };
    let saved_selection_is_live =
        objects::selection_context_is_current(model, scope, &target.selection.context)
            && match &target.selection.context {
                objects::TreeContext::Component {
                    part_id: Some(_), ..
                } => objects::resolve_selection(model, &target.selection.context)
                    .is_some_and(|part_ids| !part_ids.is_empty()),
                _ => true,
            };
    target.selection.scope == *scope
        && saved_selection_is_live
        && current_selection.is_some_and(|selected| {
            selected.scope == *scope
                && selected.context == target.destination
                && objects::selection_context_is_current(model, scope, &selected.context)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{AcceptedSnapshot, Scope, SessionEpoch, SnapshotToken};
    use boardstudio_model::model::{
        Board, Part, PartDefinition, PartKind, Pose2, ProjectDoc, Readiness, SceneDelta, Side, Vec2,
    };
    use std::sync::Arc;

    fn fixture() -> (ReadModel, Scope, objects::ScopedTreeContext) {
        let scope = Scope {
            session_epoch: SessionEpoch(5),
            document_id: "finding-return-doc".into(),
            board_id: "board".into(),
            instance_id: None,
        };
        let mut document = ProjectDoc::empty("finding-return-doc", "Finding return fixture");
        document.revision = 9;
        document.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec!["left-U1".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.definitions.push(PartDefinition {
            hardware_profile: None,
            input_profile: None,
            id: "controller".into(),
            name: "Controller".into(),
            kind: PartKind::Controller,
            keycap: None,
            envelope_source: None,
            kicad_source: None,
            terminals: Default::default(),
            matrix_terminals: None,
            envelope_notice: None,
            courtyard: vec![],
            pads: vec![],
            models: None,
            generator: None,
            mechanical_profile: None,
        });
        document.parts.push(Part {
            keycap: None,
            outline: None,
            anchor: None,
            id: "left-U1".into(),
            definition_id: "controller".into(),
            reference: "U1".into(),
            pose: Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        });
        let model = ReadModel {
            accepted: Some(AcceptedSnapshot {
                token: SnapshotToken(13),
                session_epoch: scope.session_epoch,
                document: Arc::new(document),
                scene: Arc::new(SceneDelta {
                    module_scenes: vec![],
                    revision: 9,
                    transaction_id: "accepted-fixture".into(),
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
                        layout: true,
                        outline: true,
                        pcb: true,
                        case_ready: false,
                    },
                }),
            }),
            active_board_id: scope.board_id.clone(),
            selected_part_ids: vec!["left-U1".into()],
            ..ReadModel::default()
        };
        let selection = objects::ScopedTreeContext {
            scope: scope.clone(),
            context: objects::TreeContext::Component {
                part_id: Some("left-U1".into()),
                matrix_id: None,
                row: None,
                column: None,
                assembly_id: None,
            },
        };
        (model, scope, selection)
    }

    #[test]
    fn outline_return_requires_the_same_live_owner_and_saved_selection() {
        let (model, scope, selection) = fixture();
        let owner = LayoutOwnerIdentity {
            scope: Some(scope.clone()),
            token: Some(SnapshotToken(13)),
            revision: Some(9),
            generation: 2,
            workspace: "Layout",
        };
        let destination = objects::TreeContext::Outline {
            board_id: scope.board_id.clone(),
        };
        let target = ReturnTarget {
            owner: owner.clone(),
            selection: selection.clone(),
            destination: destination.clone(),
        };
        let outline_selection = objects::ScopedTreeContext {
            scope,
            context: destination,
        };

        assert!(layout_finding_return_is_current(
            &model,
            &owner,
            Some(&outline_selection),
            &target,
        ));
        let mut non_layout_owner = owner.clone();
        non_layout_owner.workspace = "PCB";
        let non_layout_target = ReturnTarget {
            owner: non_layout_owner.clone(),
            ..target.clone()
        };
        assert!(!layout_finding_return_is_current(
            &model,
            &non_layout_owner,
            Some(&outline_selection),
            &non_layout_target,
        ));

        let empty_outline_selection = objects::ScopedTreeContext {
            scope: outline_selection.scope.clone(),
            context: outline_selection.context.clone(),
        };
        let empty_outline_target = ReturnTarget {
            selection: empty_outline_selection.clone(),
            ..target.clone()
        };
        assert!(layout_finding_return_is_current(
            &model,
            &owner,
            Some(&empty_outline_selection),
            &empty_outline_target,
        ));

        let component_target = ReturnTarget {
            destination: selection.context.clone(),
            ..target.clone()
        };
        assert!(layout_finding_return_is_current(
            &model,
            &owner,
            Some(&selection),
            &component_target,
        ));

        let mut changed_owner = owner.clone();
        changed_owner.revision = Some(10);
        assert!(!layout_finding_return_is_current(
            &model,
            &changed_owner,
            Some(&outline_selection),
            &target,
        ));

        let mut changed_scope_owner = owner.clone();
        changed_scope_owner
            .scope
            .as_mut()
            .expect("fixture scope")
            .board_id = "other-board".into();
        assert!(!layout_finding_return_is_current(
            &model,
            &changed_scope_owner,
            Some(&outline_selection),
            &target,
        ));

        let mut changed_model = model;
        Arc::make_mut(
            &mut changed_model
                .accepted
                .as_mut()
                .expect("fixture snapshot")
                .document,
        )
        .boards[0]
            .part_ids
            .clear();
        assert!(!layout_finding_return_is_current(
            &changed_model,
            &owner,
            Some(&outline_selection),
            &target,
        ));
    }
}

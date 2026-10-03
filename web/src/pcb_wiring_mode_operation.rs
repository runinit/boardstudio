//! The narrow accepted-document proposal for changing a PCB board's wiring mode.
use crate::firmware_position_projection::FirmwarePlanIdentity;
use boardstudio_application::Scope;
use boardstudio_core::{
    electrical::ElectricalMode,
    model::{ElectricalBoardConfiguration, ProjectDoc},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BoardWiringModeIdentity {
    /// Selection-independent accepted board-plan identity.
    pub plan: FirmwarePlanIdentity,
    /// Selection and UI-scope identity of the rendered control.
    pub ui_scope: Scope,
    pub selected_part_id: Option<String>,
    pub scope_generation: u64,
}

impl BoardWiringModeIdentity {
    pub(crate) fn matches_action_context(
        &self,
        current_plan: &FirmwarePlanIdentity,
        current_scope: Option<&Scope>,
        current_selected_part_id: Option<&str>,
        current_generation: u64,
    ) -> bool {
        self.plan == *current_plan
            && current_scope == Some(&self.ui_scope)
            && self.selected_part_id.as_deref() == current_selected_part_id
            && self.scope_generation == current_generation
    }

    pub(crate) fn feedback_target(&self) -> BoardWiringModeFeedbackTarget {
        BoardWiringModeFeedbackTarget {
            ui_scope: self.ui_scope.clone(),
            selected_part_id: self.selected_part_id.clone(),
            scope_generation: self.scope_generation,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BoardWiringModeFeedbackTarget {
    pub ui_scope: Scope,
    pub selected_part_id: Option<String>,
    pub scope_generation: u64,
}

impl BoardWiringModeFeedbackTarget {
    pub(crate) fn is_visible(
        &self,
        current_scope: Option<&Scope>,
        current_selected_part_id: Option<&str>,
        current_generation: u64,
    ) -> bool {
        current_scope == Some(&self.ui_scope)
            && self.selected_part_id.as_deref() == current_selected_part_id
            && self.scope_generation == current_generation
    }
}

pub(crate) fn propose_mode(
    document: &ProjectDoc,
    board_id: &str,
    mode: ElectricalMode,
) -> Option<ProjectDoc> {
    if !document.boards.iter().any(|board| board.id == board_id) {
        return None;
    }
    if document
        .hardware
        .as_ref()
        .and_then(|hardware| {
            hardware
                .boards
                .iter()
                .find(|item| item.board_id == board_id)
        })
        .map_or(ElectricalMode::Matrix, |configuration| configuration.mode)
        == mode
    {
        return None;
    }
    let mut proposal = document.clone();
    let hardware = proposal.hardware.get_or_insert_with(Default::default);
    let configuration = hardware
        .boards
        .iter_mut()
        .find(|item| item.board_id == board_id);
    if let Some(configuration) = configuration {
        configuration.mode = mode;
    } else {
        hardware.boards.push(ElectricalBoardConfiguration {
            board_id: board_id.to_owned(),
            mode,
            ..Default::default()
        });
    }
    Some(proposal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{SessionEpoch, SnapshotToken};
    use boardstudio_core::model::{Board, ElectricalBoardConfiguration, ElectricalHandoffBaseline};

    fn doc() -> ProjectDoc {
        let mut document = ProjectDoc::empty("project", "Test");
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
        document
    }

    #[test]
    fn mode_proposal_preserves_board_configuration_and_other_project_data() {
        let mut document = doc();
        let mut configuration = ElectricalBoardConfiguration {
            board_id: "left".into(),
            controller_part_id: Some("controller".into()),
            mode: ElectricalMode::Matrix,
            ..Default::default()
        };
        configuration.locks.insert("row".into(), "P1".into());
        configuration.assignments.insert("K1".into(), "R1".into());
        configuration.key_bindings.insert("K1".into(), "A".into());
        configuration.jumper_states.insert(
            "J1".into(),
            boardstudio_core::electrical_profiles::JumperState::Bridged,
        );
        configuration.protected_handoff = Some(ElectricalHandoffBaseline {
            fingerprint: "handoff".into(),
            revision: 4,
            assignments: [("K1".into(), "R1".into())].into(),
        });
        let mut other = ElectricalBoardConfiguration {
            board_id: "right".into(),
            ..Default::default()
        };
        other.locks.insert("other".into(), "P2".into());
        let hardware = document.hardware.get_or_insert_with(Default::default);
        hardware.boards = vec![configuration.clone(), other.clone()];
        document
            .parameters
            .insert("kept".into(), serde_json::json!(true));

        let changed = propose_mode(&document, "left", ElectricalMode::Direct).unwrap();
        assert_eq!(changed.parameters, document.parameters);
        let boards = &changed.hardware.as_ref().unwrap().boards;
        let left = boards.iter().find(|item| item.board_id == "left").unwrap();
        assert_eq!(left.mode, ElectricalMode::Direct);
        assert_eq!(left.controller_part_id, configuration.controller_part_id);
        assert_eq!(left.locks, configuration.locks);
        assert_eq!(left.assignments, configuration.assignments);
        assert_eq!(left.key_bindings, configuration.key_bindings);
        assert_eq!(left.jumper_states, configuration.jumper_states);
        assert_eq!(left.protected_handoff, configuration.protected_handoff);
        assert_eq!(
            boards.iter().find(|item| item.board_id == "right").unwrap(),
            &other
        );
    }

    #[test]
    fn same_mode_is_a_noop_and_missing_configuration_is_created_for_target_board() {
        let document = doc();
        assert!(propose_mode(&document, "left", ElectricalMode::Matrix).is_none());
        let changed = propose_mode(&document, "left", ElectricalMode::Direct).unwrap();
        assert_eq!(
            changed.hardware.unwrap().boards,
            vec![ElectricalBoardConfiguration {
                board_id: "left".into(),
                mode: ElectricalMode::Direct,
                ..Default::default()
            }]
        );
        assert!(propose_mode(&document, "missing", ElectricalMode::Direct).is_none());
    }

    #[test]
    fn action_identity_keeps_board_plan_and_rendered_selection_as_separate_guards() {
        let board_scope = Scope {
            session_epoch: SessionEpoch(3),
            document_id: "project".into(),
            board_id: "left".into(),
            instance_id: None,
        };
        let ui_scope = Scope {
            instance_id: Some("primary".into()),
            ..board_scope.clone()
        };
        let plan = FirmwarePlanIdentity {
            scope: board_scope,
            token: SnapshotToken(8),
            revision: 12,
            executor_epoch: 4,
        };
        let rendered = BoardWiringModeIdentity {
            plan: plan.clone(),
            ui_scope: ui_scope.clone(),
            selected_part_id: Some("controller".into()),
            scope_generation: 5,
        };

        assert!(rendered.matches_action_context(&plan, Some(&ui_scope), Some("controller"), 5));
        assert!(!rendered.matches_action_context(&plan, Some(&ui_scope), Some("switch"), 5));
        assert!(!rendered.matches_action_context(&plan, Some(&ui_scope), Some("controller"), 6));
        let next_plan = FirmwarePlanIdentity {
            revision: 13,
            ..plan.clone()
        };
        assert!(!rendered.matches_action_context(
            &next_plan,
            Some(&ui_scope),
            Some("controller"),
            5
        ));
        assert_eq!(
            rendered.plan, plan,
            "the board plan is independent of selection"
        );
    }

    #[test]
    fn saved_feedback_survives_plan_revision_advance_but_not_selection_change() {
        let scope = Scope {
            session_epoch: SessionEpoch(3),
            document_id: "project".into(),
            board_id: "left".into(),
            instance_id: Some("primary".into()),
        };
        let identity = BoardWiringModeIdentity {
            plan: FirmwarePlanIdentity {
                scope: Scope {
                    instance_id: None,
                    ..scope.clone()
                },
                token: SnapshotToken(8),
                revision: 12,
                executor_epoch: 4,
            },
            ui_scope: scope.clone(),
            selected_part_id: Some("controller".into()),
            scope_generation: 5,
        };
        let target = identity.feedback_target();
        let advanced = BoardWiringModeIdentity {
            plan: FirmwarePlanIdentity {
                token: SnapshotToken(9),
                revision: 13,
                ..identity.plan.clone()
            },
            ..identity.clone()
        };
        assert_eq!(target, advanced.feedback_target());
        assert!(!identity.matches_action_context(
            &advanced.plan,
            Some(&scope),
            Some("controller"),
            5
        ));
        assert!(target.is_visible(Some(&scope), Some("controller"), 5));
        assert!(!target.is_visible(Some(&scope), Some("switch"), 5));
        assert!(!target.is_visible(Some(&scope), Some("controller"), 6));
    }
}

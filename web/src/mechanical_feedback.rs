//! Pure owner-scoped feedback delivery and global-summary relevance policy.
//!
//! The page mount records the accepted configuration basis when feedback is
//! published. This module decides which records reach their field owner and
//! which one remains relevant to the global summary; it does not own UI state.

use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::MechanicalConfiguration;
#[cfg(test)]
use boardstudio_core::model::{MechanicalMount, PlateMethod};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MechanicalSettingsIdentity {
    pub(crate) editor_instance_id: u64,
    /// External Scope lineage, including away-and-back navigation to an equal Scope.
    pub(crate) scope_generation: u64,
    /// Case presentation lineage, independently superseded by workspace changes.
    pub(crate) presentation_generation: u64,
    pub(crate) scope: Scope,
    pub(crate) snapshot_token: SnapshotToken,
    pub(crate) revision: u64,
    pub(crate) active_board_id: String,
    /// The accepted configuration's board, or the active board for Configure.
    pub(crate) configuration_board_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MechanicalSettingsFeedbackState {
    Pending,
    Saved,
    Failed,
}

/// Root echoes this full request identity and the field owner in all states.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MechanicalSettingsFeedback {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) request_id: u64,
    pub(crate) field_id: String,
    pub(crate) state: MechanicalSettingsFeedbackState,
    pub(crate) message: Option<String>,
}

#[derive(Clone)]
pub(crate) struct FeedbackRecord {
    pub(crate) feedback: MechanicalSettingsFeedback,
    /// Shared accepted configuration at publication, including the acknowledged basis of a
    /// reconciliation failure. It is relevance evidence, never a predicted document.
    pub(crate) basis: Option<Rc<MechanicalConfiguration>>,
}

pub(crate) fn field_feedback(
    entries: &[FeedbackRecord],
    identity: &MechanicalSettingsIdentity,
) -> Rc<[MechanicalSettingsFeedback]> {
    Rc::from(
        entries
            .iter()
            .filter(|record| same_feedback_owner(&record.feedback.identity, identity))
            .map(|record| record.feedback.clone())
            .collect::<Vec<_>>(),
    )
}

pub(crate) fn relevant_summary(
    entries: &[FeedbackRecord],
    identity: &MechanicalSettingsIdentity,
    configuration: Option<&MechanicalConfiguration>,
    busy: bool,
) -> Option<MechanicalSettingsFeedback> {
    entries
        .iter()
        .filter(|record| same_feedback_owner(&record.feedback.identity, identity))
        .filter(|record| {
            if busy {
                record.feedback.state == MechanicalSettingsFeedbackState::Pending
            } else {
                record.feedback.state != MechanicalSettingsFeedbackState::Pending
                    && same_field_value(
                        &record.feedback.field_id,
                        record.basis.as_deref(),
                        configuration,
                    )
            }
        })
        .max_by_key(|record| record.feedback.request_id)
        .map(|record| record.feedback.clone())
}

pub(crate) fn same_feedback_owner(
    feedback: &MechanicalSettingsIdentity,
    current: &MechanicalSettingsIdentity,
) -> bool {
    feedback.editor_instance_id == current.editor_instance_id
        && feedback.scope_generation == current.scope_generation
        && feedback.presentation_generation == current.presentation_generation
        && feedback.scope == current.scope
        && feedback.active_board_id == current.active_board_id
        && feedback.configuration_board_id == current.configuration_board_id
}

fn same_field_value(
    field: &str,
    basis: Option<&MechanicalConfiguration>,
    current: Option<&MechanicalConfiguration>,
) -> bool {
    if matches!(field, "configure" | "disable") {
        return basis.is_some() == current.is_some();
    }
    let (Some(basis), Some(current)) = (basis, current) else {
        return false;
    };
    match field {
        "initialize-closures" => {
            basis.mount == current.mount && basis.closure_mounts == current.closure_mounts
        }
        "method" => basis.method == current.method && same_process_methods(basis, current),
        "mount" => basis.mount == current.mount,
        "bottom-style" => basis.bottom_style == current.bottom_style,
        "middle-frame" => basis.middle_frame == current.middle_frame,
        "integrated-plate-frame" => basis.integrated_plate_frame == current.integrated_plate_frame,
        "plate-thickness" => basis.plate_thickness == current.plate_thickness,
        "plate-foam-thickness" => basis.plate_foam_thickness == current.plate_foam_thickness,
        "pcb-thickness" => basis.pcb_thickness == current.pcb_thickness,
        "bottom-foam-thickness" => basis.bottom_foam_thickness == current.bottom_foam_thickness,
        "bottom-thickness" => basis.bottom_thickness == current.bottom_thickness,
        "wall-thickness" => basis.wall_thickness == current.wall_thickness,
        "clearance" => basis.clearance == current.clearance,
        "opening-allowance" => basis.opening_allowance == current.opening_allowance,
        _ => field.strip_prefix("switch-family:").is_some_and(|id| {
            let before = basis
                .profiles
                .iter()
                .find(|profile| profile.definition_id == id);
            let after = current
                .profiles
                .iter()
                .find(|profile| profile.definition_id == id);
            before.is_some() && before == after
        }),
    }
}

fn same_process_methods(
    basis: &MechanicalConfiguration,
    current: &MechanicalConfiguration,
) -> bool {
    let standard = |id: &str| matches!(id, "plate" | "plate-foam" | "bottom-foam" | "bottom");
    basis
        .part_processes
        .iter()
        .flatten()
        .filter(|process| standard(&process.part_id))
        .map(|process| (&process.part_id, &process.method, &process.material))
        .eq(current
            .part_processes
            .iter()
            .flatten()
            .filter(|process| standard(&process.part_id))
            .map(|process| (&process.part_id, &process.method, &process.material)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::SessionEpoch;

    fn identity() -> MechanicalSettingsIdentity {
        MechanicalSettingsIdentity {
            editor_instance_id: 3,
            scope_generation: 4,
            presentation_generation: 5,
            scope: Scope {
                session_epoch: SessionEpoch(6),
                document_id: "document".into(),
                board_id: "board".into(),
                instance_id: None,
            },
            snapshot_token: SnapshotToken(7),
            revision: 8,
            active_board_id: "board".into(),
            configuration_board_id: "board".into(),
        }
    }

    fn configuration() -> MechanicalConfiguration {
        MechanicalConfiguration {
            internal_gasket: None,
            gasket_layout: None,
            hardware: None,
            critical_fits: None,
            bottom_style: None,
            middle_frame: None,
            gasket_travel: None,
            openings: None,
            opening_allowance: None,
            stabilizers: None,
            part_processes: None,
            gasket: None,
            closure_mounts: None,
            board_id: "board".into(),
            integrated_plate_frame: false,
            battery: None,
            mounts: Vec::new(),
            method: PlateMethod::Printed,
            mount: MechanicalMount::Tray,
            plate_thickness: 1.5,
            plate_foam_thickness: 0.5,
            pcb_thickness: 1.6,
            bottom_foam_thickness: 1.0,
            battery_height: 0.0,
            bottom_thickness: 1.8,
            plate_to_pcb: 0.2,
            wall_thickness: 2.0,
            clearance: 0.3,
            profiles: Vec::new(),
        }
    }

    fn record(
        owner: &MechanicalSettingsIdentity,
        request_id: u64,
        field_id: &str,
        state: MechanicalSettingsFeedbackState,
        basis: Option<MechanicalConfiguration>,
    ) -> FeedbackRecord {
        FeedbackRecord {
            feedback: MechanicalSettingsFeedback {
                identity: owner.clone(),
                request_id,
                field_id: field_id.into(),
                state,
                message: Some(format!("request {request_id}")),
            },
            basis: basis.map(Rc::new),
        }
    }

    #[test]
    fn rejected_request_reaches_its_field_while_older_request_is_pending() {
        let owner = identity();
        let basis = configuration();
        let mut entries = vec![
            record(
                &owner,
                1,
                "plate-thickness",
                MechanicalSettingsFeedbackState::Pending,
                Some(basis.clone()),
            ),
            record(
                &owner,
                2,
                "clearance",
                MechanicalSettingsFeedbackState::Failed,
                Some(basis.clone()),
            ),
        ];

        let field = field_feedback(&entries, &owner);
        assert_eq!(field.len(), 2);
        assert_eq!(field[1].request_id, 2);
        assert_eq!(field[1].field_id, "clearance");
        assert_eq!(field[1].state, MechanicalSettingsFeedbackState::Failed);
        assert_eq!(
            relevant_summary(&entries, &owner, Some(&basis), true).map(|item| item.request_id),
            Some(1)
        );

        // A can finish after B was rejected. The field owner still receives B's exact terminal
        // result, while the summary follows the accepted configuration values for each field.
        entries[0].feedback.state = MechanicalSettingsFeedbackState::Saved;
        let mut after_a = basis;
        after_a.plate_thickness += 0.25;
        let mut after_a_identity = owner.clone();
        after_a_identity.snapshot_token = SnapshotToken(9);
        after_a_identity.revision = 10;
        let clearance_feedback = field_feedback(&entries, &after_a_identity)
            .iter()
            .find(|item| item.request_id == 2 && item.field_id == "clearance")
            .cloned();
        assert_eq!(
            clearance_feedback.map(|item| item.state),
            Some(MechanicalSettingsFeedbackState::Failed)
        );
        assert_eq!(
            relevant_summary(&entries, &after_a_identity, Some(&after_a), false)
                .map(|item| (item.request_id, item.state)),
            Some((2, MechanicalSettingsFeedbackState::Failed))
        );
    }

    #[test]
    fn failed_feedback_survives_token_advance_and_unrelated_field_change() {
        let submitted = identity();
        let basis = configuration();
        let entries = [record(
            &submitted,
            2,
            "clearance",
            MechanicalSettingsFeedbackState::Failed,
            Some(basis.clone()),
        )];

        let mut current_owner = submitted;
        current_owner.snapshot_token = SnapshotToken(9);
        current_owner.revision = 10;
        let mut current = basis.clone();
        current.plate_thickness += 0.25;

        assert_eq!(
            relevant_summary(&entries, &current_owner, Some(&current), false)
                .map(|item| item.state),
            Some(MechanicalSettingsFeedbackState::Failed)
        );
    }

    #[test]
    fn saved_feedback_is_hidden_after_undo_changes_its_field_again() {
        let owner = identity();
        let mut saved_basis = configuration();
        saved_basis.clearance = 0.45;
        let entries = [record(
            &owner,
            3,
            "clearance",
            MechanicalSettingsFeedbackState::Saved,
            Some(saved_basis.clone()),
        )];

        let mut undone = saved_basis;
        undone.clearance = 0.3;
        assert!(relevant_summary(&entries, &owner, Some(&undone), false).is_none());
    }

    #[test]
    fn summary_relevance_tracks_its_field_not_unrelated_configuration_values() {
        let owner = identity();
        let basis = configuration();
        let entries = [record(
            &owner,
            4,
            "clearance",
            MechanicalSettingsFeedbackState::Saved,
            Some(basis.clone()),
        )];

        let mut unrelated = basis.clone();
        unrelated.bottom_thickness += 0.4;
        assert!(relevant_summary(&entries, &owner, Some(&unrelated), false).is_some());

        let mut changed_field = unrelated;
        changed_field.clearance += 0.1;
        assert!(relevant_summary(&entries, &owner, Some(&changed_field), false).is_none());
    }
}

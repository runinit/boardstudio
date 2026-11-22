use boardstudio_application::Scope;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartNetEditIdentity {
    pub board_id: String,
    pub ui_scope: Scope,
    pub part_id: String,
    pub token: boardstudio_application::SnapshotToken,
    pub revision: u64,
    pub generation: u64,
}

pub(super) fn request_matches_current_part(
    rendered: &PartNetEditIdentity,
    current: &PartNetEditIdentity,
) -> bool {
    rendered == current
}

pub(super) fn admit_classified_part_net_edit(
    in_current_workspace: bool,
    rendered: &PartNetEditIdentity,
    current: impl FnOnce() -> Option<PartNetEditIdentity>,
) -> bool {
    in_current_workspace
        && current().is_some_and(|current| request_matches_current_part(rendered, &current))
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{SessionEpoch, SnapshotToken};

    fn identity(
        board_id: &str,
        part_id: &str,
        token: u64,
        revision: u64,
        generation: u64,
    ) -> PartNetEditIdentity {
        PartNetEditIdentity {
            board_id: board_id.into(),
            ui_scope: Scope {
                session_epoch: SessionEpoch(3),
                document_id: "project".into(),
                board_id: board_id.into(),
                instance_id: None,
            },
            part_id: part_id.into(),
            token: SnapshotToken(token),
            revision,
            generation,
        }
    }

    #[test]
    fn classified_part_edit_requires_a_live_pcb_owner() {
        let rendered = identity("left", "left-J2", 7, 12, 4);
        assert!(!admit_classified_part_net_edit(false, &rendered, || {
            panic!("departed workspace must be rejected before resolving its owner")
        }));
        assert!(!admit_classified_part_net_edit(true, &rendered, || None));
        assert!(!admit_classified_part_net_edit(true, &rendered, || {
            Some(identity("left", "left-J2", 7, 12, 5))
        }));
        assert!(admit_classified_part_net_edit(true, &rendered, || Some(
            rendered.clone()
        )));
    }

    #[test]
    fn rendered_part_action_is_rejected_after_board_selection_or_snapshot_changes() {
        let rendered = identity("left", "left-J2", 7, 12, 4);
        for current in [
            identity("right", "left-J2", 7, 12, 4),
            identity("left", "left-SW1", 7, 12, 4),
            identity("left", "left-J2", 8, 12, 4),
            identity("left", "left-J2", 7, 13, 4),
            identity("left", "left-J2", 7, 12, 5),
        ] {
            assert!(!request_matches_current_part(&rendered, &current));
        }
        assert!(request_matches_current_part(&rendered, &rendered));
    }
}

use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
use boardstudio_web_ui_model::scope_transition::ScopeTransition;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnerContext {
    ProjectGuide,
    CaseInspector,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnerIdentity {
    pub context: OwnerContext,
    pub session_epoch: SessionEpoch,
    pub document_id: String,
    pub board_id: String,
    pub instance_id: Option<String>,
    pub token: SnapshotToken,
    pub revision: u64,
    pub generation: u64,
    pub scope_transition: Option<ScopeTransition>,
}

pub(super) fn expected_case_reassignment_owner(
    owner: &OwnerIdentity,
    target_board_id: &str,
    accepted_token: SnapshotToken,
    current_generation: u64,
) -> Option<OwnerIdentity> {
    if owner.context != OwnerContext::CaseInspector || target_board_id == owner.board_id {
        return None;
    }
    let instance_id = owner.instance_id.clone()?;
    let next_generation = owner.generation.checked_add(1)?;
    if current_generation != next_generation {
        return None;
    }
    let previous_scope = Scope {
        session_epoch: owner.session_epoch,
        document_id: owner.document_id.clone(),
        board_id: owner.board_id.clone(),
        instance_id: Some(instance_id),
    };
    let next_scope = Scope {
        instance_id: None,
        ..previous_scope.clone()
    };
    let transition = ScopeTransition {
        previous_scope: Some(previous_scope),
        previous_token: Some(owner.token),
        next_scope: Some(next_scope),
        next_token: Some(accepted_token),
        previous_generation: owner.generation,
        next_generation,
    };
    let mut current_owner = owner.clone();
    current_owner.instance_id = None;
    current_owner.generation = next_generation;
    current_owner.scope_transition = Some(transition);
    Some(current_owner)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owner() -> OwnerIdentity {
        OwnerIdentity {
            context: OwnerContext::CaseInspector,
            session_epoch: SessionEpoch(3),
            document_id: "project".into(),
            board_id: "board-a".into(),
            instance_id: Some("right-half".into()),
            token: SnapshotToken(41),
            revision: 8,
            generation: 12,
            scope_transition: None,
        }
    }

    #[test]
    fn only_the_accepted_reassignment_scope_transition_admits_followup_navigation() {
        let owner = owner();
        let accepted_token = SnapshotToken(42);
        let normalized_owner =
            expected_case_reassignment_owner(&owner, "board-b", accepted_token, 13)
                .expect("one accepted scope normalization advances generation once");
        let expected = normalized_owner
            .scope_transition
            .as_ref()
            .expect("reassignment identity carries its exact transition");

        assert!(expected.matches_current(Some(expected), Some(accepted_token), 13));

        let mut user_cleared = expected.clone();
        user_cleared.next_token = Some(owner.token);
        assert!(!expected.matches_current(Some(&user_cleared), Some(accepted_token), 13));

        let mut user_navigated = expected.clone();
        user_navigated.next_scope = Some(Scope {
            instance_id: Some("left-half".into()),
            ..expected
                .next_scope
                .clone()
                .expect("same-board scope exists")
        });
        assert!(!expected.matches_current(Some(&user_navigated), Some(accepted_token), 13));
        assert!(!expected.matches_current(Some(expected), Some(accepted_token), 12));
    }
}

//! Accepted-document proposal for deliberately reviewing a protected PCB handoff.
use boardstudio_core::model::ProjectDoc;

pub(crate) fn propose_review_remap(
    document: &ProjectDoc,
    board_id: &str,
    expected_fingerprint: &str,
) -> Option<ProjectDoc> {
    if !document.boards.iter().any(|board| board.id == board_id) {
        return None;
    }
    let mut proposal = document.clone();
    let configuration = proposal
        .hardware
        .as_mut()?
        .boards
        .iter_mut()
        .find(|configuration| configuration.board_id == board_id)?;
    if configuration
        .protected_handoff
        .as_ref()
        .map(|handoff| handoff.fingerprint.as_str())
        != Some(expected_fingerprint)
    {
        return None;
    }
    configuration.protected_handoff = None;
    Some(proposal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::{Board, ElectricalBoardConfiguration, ElectricalHandoffBaseline};

    fn protected_document() -> ProjectDoc {
        let mut document = ProjectDoc::empty("project", "Test");
        document.boards.extend([
            Board {
                id: "left".into(),
                name: "Left".into(),
                outline_ids: vec![],
                part_ids: vec![],
                net_ids: vec![],
                thickness: 1.6,
                traces: vec![],
                vias: vec![],
            },
            Board {
                id: "right".into(),
                name: "Right".into(),
                outline_ids: vec![],
                part_ids: vec![],
                net_ids: vec![],
                thickness: 1.6,
                traces: vec![],
                vias: vec![],
            },
        ]);
        let mut left = ElectricalBoardConfiguration {
            board_id: "left".into(),
            controller_part_id: Some("U1".into()),
            ..Default::default()
        };
        left.locks.insert("matrix/r0".into(), "P0".into());
        left.protected_handoff = Some(ElectricalHandoffBaseline {
            fingerprint: "approved-fingerprint".into(),
            revision: 7,
            assignments: [("matrix/r0".into(), "P0".into())].into(),
        });
        let mut right = ElectricalBoardConfiguration {
            board_id: "right".into(),
            ..Default::default()
        };
        right.key_bindings.insert("matrix/r1c1".into(), "B".into());
        document
            .hardware
            .get_or_insert_with(Default::default)
            .boards = vec![left, right];
        document.parameters.insert(
            "unknown-extension".into(),
            serde_json::json!({"keep": true}),
        );
        document
    }

    #[test]
    fn review_clears_only_the_matching_selected_board_protection() {
        let document = protected_document();
        let proposal = propose_review_remap(&document, "left", "approved-fingerprint").unwrap();
        assert_eq!(proposal.revision, document.revision);
        assert_eq!(proposal.parameters, document.parameters);
        assert_eq!(proposal.boards, document.boards);
        let configs = &proposal.hardware.as_ref().unwrap().boards;
        let left = configs
            .iter()
            .find(|config| config.board_id == "left")
            .unwrap();
        assert!(left.protected_handoff.is_none());
        assert_eq!(
            left.locks,
            document.hardware.as_ref().unwrap().boards[0].locks
        );
        assert_eq!(left.controller_part_id.as_deref(), Some("U1"));
        assert_eq!(
            configs
                .iter()
                .find(|config| config.board_id == "right")
                .unwrap(),
            &document.hardware.as_ref().unwrap().boards[1]
        );
    }

    #[test]
    fn review_rejects_changed_or_missing_handoff_without_proposing_an_edit() {
        let document = protected_document();
        assert!(propose_review_remap(&document, "left", "stale-fingerprint").is_none());
        assert!(propose_review_remap(&document, "right", "approved-fingerprint").is_none());
        let mut unprotected = document;
        unprotected.hardware.as_mut().unwrap().boards[0].protected_handoff = None;
        assert!(propose_review_remap(&unprotected, "left", "approved-fingerprint").is_none());
    }
}

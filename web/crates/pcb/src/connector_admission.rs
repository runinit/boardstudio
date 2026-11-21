//! Host-compatible VIK connector admission for mounted-module placement.
//!
//! The production module inspector (wasm) delegates to this module so the same
//! admission predicate runs under `cargo test` without a browser.

use boardstudio_model::model::VikRole;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectorAdmission {
    Reuse,
    CreateAutomatic,
    Reject,
}

pub fn connector_admission(
    connector_id: &str,
    automatic_id: &str,
    is_host_connector_on_board: bool,
    exists_in_document: bool,
) -> ConnectorAdmission {
    if is_host_connector_on_board {
        ConnectorAdmission::Reuse
    } else if connector_id == automatic_id && !exists_in_document {
        ConnectorAdmission::CreateAutomatic
    } else {
        ConnectorAdmission::Reject
    }
}

pub fn host_connector_is_on_board(
    document: &boardstudio_model::model::ProjectDoc,
    board_id: &str,
    connector_id: &str,
) -> bool {
    document.boards.iter().any(|board| {
        board.id == board_id && board.part_ids.iter().any(|part_id| part_id == connector_id)
    }) && document
        .parts
        .iter()
        .find(|part| part.id == connector_id)
        .and_then(|part| {
            document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)
        })
        .and_then(|definition| definition.hardware_profile.as_ref())
        .is_some_and(|profile| profile.vik_role == Some(VikRole::Host))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connector_admission_reuses_local_host_and_rejects_foreign_automatic_ids() {
        let automatic_id = "module-a/vik-host-connector";

        assert_eq!(
            connector_admission("host-on-board", automatic_id, true, true),
            ConnectorAdmission::Reuse
        );
        assert_eq!(
            connector_admission(automatic_id, automatic_id, false, true),
            ConnectorAdmission::Reject
        );
        assert_eq!(
            connector_admission("wrong-role", automatic_id, false, true),
            ConnectorAdmission::Reject
        );
        assert_eq!(
            connector_admission("missing-host", automatic_id, false, false),
            ConnectorAdmission::Reject
        );
        assert_eq!(
            connector_admission(automatic_id, automatic_id, false, false),
            ConnectorAdmission::CreateAutomatic
        );
    }
}

//! Accepted-document transformations for the contextual PCB part connection editor.
use boardstudio_core::model::{PartKind, ProjectDoc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum PartNetIntent {
    AssignPads {
        pad_ids: Vec<String>,
        net_id: Option<String>,
    },
    CreateNet {
        net_id: String,
        name: String,
    },
}

/// Build a proposal from the captured accepted document. The caller separately admits that
/// document against the live Editor/Session identity before registering and submitting it.
pub(super) fn propose(
    document: &ProjectDoc,
    board_id: &str,
    part_id: &str,
    intent: PartNetIntent,
) -> Result<ProjectDoc, String> {
    let board = document
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .ok_or_else(|| "The selected board no longer exists.".to_owned())?;
    let part = document
        .parts
        .iter()
        .find(|part| part.id == part_id && board.part_ids.contains(&part.id))
        .ok_or_else(|| "The selected part is not on this board.".to_owned())?;
    let definition = document
        .definitions
        .iter()
        .find(|definition| definition.id == part.definition_id)
        .ok_or_else(|| "The selected part definition is unavailable.".to_owned())?;
    if matches!(&definition.kind, PartKind::Switch | PartKind::Controller) {
        return Err("Switches inherit wiring and controllers use board wiring.".into());
    }

    match intent {
        PartNetIntent::AssignPads { pad_ids, net_id } => {
            if pad_ids.is_empty() {
                return Err("Choose at least one terminal pad.".into());
            }
            let named_terminal = definition
                .terminals
                .values()
                .any(|terminal_pads| terminal_pads == &pad_ids);
            let standalone_pad = pad_ids.len() == 1
                && definition.pads.iter().any(|pad| {
                    pad.id == pad_ids[0]
                        && pad.plated != Some(false)
                        && !pad.number.is_empty()
                        && !definition
                            .terminals
                            .values()
                            .any(|terminal| terminal.contains(&pad.id))
                });
            if !named_terminal && !standalone_pad {
                return Err("These pads are not an editable terminal or plated pad.".into());
            }
            if let Some(net_id) = net_id.as_deref() {
                let net_is_available = document.nets.iter().any(|net| {
                    net.id == net_id
                        && (board.net_ids.contains(&net.id)
                            || net
                                .pins
                                .iter()
                                .any(|pin| board.part_ids.contains(&pin.part_id)))
                });
                if !net_is_available {
                    return Err("Choose a net available on the selected board.".into());
                }
            }

            let targets = pad_ids
                .iter()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>();
            let mut proposal = document.clone();
            for net in &mut proposal.nets {
                net.pins
                    .retain(|pin| pin.part_id != part_id || !targets.contains(&pin.pad_id));
                if Some(net.id.as_str()) == net_id.as_deref() {
                    net.pins
                        .extend(pad_ids.iter().map(|pad_id| boardstudio_core::model::Pin {
                            part_id: part_id.to_owned(),
                            pad_id: pad_id.clone(),
                        }));
                }
            }
            Ok(proposal)
        }
        PartNetIntent::CreateNet { net_id, name } => {
            let name = name.trim();
            if name.is_empty() {
                return Err("Enter a name for the new net.".into());
            }
            if document.nets.iter().any(|net| net.id == net_id) {
                return Err("The new net identity is already in use.".into());
            }
            let mut proposal = document.clone();
            proposal.nets.push(boardstudio_core::model::Net {
                id: net_id.clone(),
                name: name.to_owned(),
                pins: Vec::new(),
            });
            if let Some(board) = proposal
                .boards
                .iter_mut()
                .find(|board| board.id == board_id)
                && !board.net_ids.contains(&net_id)
            {
                board.net_ids.push(net_id);
            }
            Ok(proposal)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::ProjectDoc;
    use serde_json::json;
    use wasm_bindgen_test::wasm_bindgen_test;

    fn fixture() -> ProjectDoc {
        let mut document = ProjectDoc::empty("project-a", "Project A");
        document.boards = vec![
            serde_json::from_value(json!({
                "id":"left", "name":"Left PCB", "outlineIds":[],
                "partIds":["left-J2", "left-SW1"], "netIds":["net-rx", "net-power"],
                "thickness":1.6
            }))
            .unwrap(),
            serde_json::from_value(json!({
                "id":"right", "name":"Right PCB", "outlineIds":[],
                "partIds":["right-U1"], "netIds":["right-net"], "thickness":1.6
            }))
            .unwrap(),
        ];
        document.definitions = vec![
            serde_json::from_value(json!({
                "id":"trrs", "name":"TRRS connector", "kind":"connector",
                "courtyard":[], "terminals":{"R1":["r1", "r2"], "SL":["sl"]},
                "pads":[
                    {"id":"r1","number":"1","at":{"x":0,"y":0},"size":{"x":1,"y":1},"shape":"circle"},
                    {"id":"r2","number":"2","at":{"x":0,"y":0},"size":{"x":1,"y":1},"shape":"circle"},
                    {"id":"sl","number":"3","at":{"x":0,"y":0},"size":{"x":1,"y":1},"shape":"circle"},
                    {"id":"signal","number":"4","at":{"x":0,"y":0},"size":{"x":1,"y":1},"shape":"circle"},
                    {"id":"npth","number":"5","at":{"x":0,"y":0},"size":{"x":1,"y":1},"shape":"circle","plated":false},
                    {"id":"blank","number":"","at":{"x":0,"y":0},"size":{"x":1,"y":1},"shape":"circle"}
                ]
            }))
            .unwrap(),
            serde_json::from_value(json!({
                "id":"switch", "name":"MX switch", "kind":"switch", "courtyard":[], "pads":[]
            }))
            .unwrap(),
        ];
        document.parts = vec![
            serde_json::from_value(json!({
                "id":"left-J2", "definitionId":"trrs", "reference":"J2",
                "pose":{"at":{"x":0,"y":0},"rotation":0}, "side":"front"
            }))
            .unwrap(),
            serde_json::from_value(json!({
                "id":"left-SW1", "definitionId":"switch", "reference":"SW1",
                "pose":{"at":{"x":0,"y":0},"rotation":0}, "side":"front"
            }))
            .unwrap(),
        ];
        document.nets = vec![
            serde_json::from_value(json!({
                "id":"net-rx", "name":"RX", "pins":[
                    {"partId":"left-J2","padId":"r1"},
                    {"partId":"left-J2","padId":"signal"},
                    {"partId":"left-SW1","padId":"r1"}
                ]
            }))
            .unwrap(),
            serde_json::from_value(json!({
                "id":"net-power", "name":"Power", "pins":[
                    {"partId":"left-J2","padId":"r1"}
                ]
            }))
            .unwrap(),
            serde_json::from_value(json!({
                "id":"right-net", "name":"Right only", "pins":[
                    {"partId":"right-U1","padId":"1"}
                ]
            }))
            .unwrap(),
        ];
        document
    }

    fn pin_pairs(document: &ProjectDoc, net_id: &str) -> Vec<(String, String)> {
        document
            .nets
            .iter()
            .find(|net| net.id == net_id)
            .unwrap()
            .pins
            .iter()
            .map(|pin| (pin.part_id.clone(), pin.pad_id.clone()))
            .collect()
    }

    #[wasm_bindgen_test]
    fn terminal_mapping_moves_only_target_pins_and_preserves_other_boards() {
        let before = fixture();
        let proposal = propose(
            &before,
            "left",
            "left-J2",
            PartNetIntent::AssignPads {
                pad_ids: vec!["r1".into(), "r2".into()],
                net_id: Some("net-power".into()),
            },
        )
        .unwrap();

        assert_eq!(
            pin_pairs(&proposal, "net-rx"),
            vec![
                ("left-J2".into(), "signal".into()),
                ("left-SW1".into(), "r1".into()),
            ]
        );
        assert_eq!(
            pin_pairs(&proposal, "net-power"),
            vec![
                ("left-J2".into(), "r1".into()),
                ("left-J2".into(), "r2".into()),
            ]
        );
        assert_eq!(proposal.boards, before.boards);
        assert_eq!(
            pin_pairs(&proposal, "right-net"),
            pin_pairs(&before, "right-net")
        );
        assert_eq!(
            pin_pairs(&before, "net-rx").len(),
            3,
            "input stays immutable"
        );
    }

    #[wasm_bindgen_test]
    fn creating_named_net_adds_it_only_to_selected_board() {
        let before = fixture();
        let proposal = propose(
            &before,
            "left",
            "left-J2",
            PartNetIntent::CreateNet {
                net_id: "new-net".into(),
                name: "  Audio return  ".into(),
            },
        )
        .unwrap();
        assert_eq!(proposal.nets.last().unwrap().name, "Audio return");
        assert_eq!(proposal.boards[0].net_ids.last().unwrap(), "new-net");
        assert_eq!(proposal.boards[1], before.boards[1]);
        assert_eq!(pin_pairs(&proposal, "net-rx"), pin_pairs(&before, "net-rx"));
    }

    #[wasm_bindgen_test]
    fn mapping_rejects_hidden_or_inherited_targets() {
        let before = fixture();
        for pad_id in ["npth", "blank", "not-a-pad"] {
            assert!(
                propose(
                    &before,
                    "left",
                    "left-J2",
                    PartNetIntent::AssignPads {
                        pad_ids: vec![pad_id.into()],
                        net_id: Some("net-rx".into()),
                    },
                )
                .is_err()
            );
        }
        assert!(
            propose(
                &before,
                "left",
                "left-SW1",
                PartNetIntent::AssignPads {
                    pad_ids: vec!["r1".into()],
                    net_id: Some("net-rx".into()),
                },
            )
            .is_err()
        );
    }

    #[wasm_bindgen_test]
    fn mapping_rejects_net_from_another_board() {
        let before = fixture();
        assert!(
            propose(
                &before,
                "left",
                "left-J2",
                PartNetIntent::AssignPads {
                    pad_ids: vec!["signal".into()],
                    net_id: Some("right-net".into()),
                },
            )
            .is_err()
        );
    }

    #[wasm_bindgen_test]
    fn legacy_net_with_selected_board_part_pin_is_selectable() {
        let mut before = fixture();
        before.boards[0].net_ids.retain(|id| id != "net-power");
        assert!(
            propose(
                &before,
                "left",
                "left-J2",
                PartNetIntent::AssignPads {
                    pad_ids: vec!["signal".into()],
                    net_id: Some("net-power".into()),
                },
            )
            .is_ok()
        );
    }
}

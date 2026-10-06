//! Review and release only manual net assignments that conflict with a current plan.
use boardstudio_core::{
    electrical::ElectricalPlan,
    model::{Pin, ProjectDoc},
};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExistingConnectionNet {
    pub id: String,
    pub name: String,
    pub pins: BTreeSet<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExistingConnectionReview {
    pub nets: Vec<ExistingConnectionNet>,
    pub pin_count: usize,
}

impl ExistingConnectionReview {
    pub fn names(&self) -> Vec<String> {
        self.nets.iter().map(|net| net.name.clone()).collect()
    }
}

pub fn existing_connection_review(
    document: &ProjectDoc,
    plan: &ElectricalPlan,
) -> Option<ExistingConnectionReview> {
    let board_id = plan.board_id.as_deref()?;
    let conflicts = plan
        .diagnostics
        .iter()
        .filter(|finding| finding.code == "manual-net-conflict")
        .filter_map(|finding| finding.key_id.as_deref())
        .collect::<BTreeSet<_>>();
    if conflicts.is_empty() {
        return None;
    }
    let proposed = plan
        .nets
        .iter()
        .flat_map(|net| net.pins.iter())
        .map(|pin| (pin.part_id.as_str(), pin.pad_id.as_str()))
        .collect::<BTreeSet<_>>();
    let matrix_net_prefixes = document
        .matrices
        .iter()
        .filter(|matrix| matrix.board_id.as_deref() == Some(board_id))
        .map(|matrix| format!("matrix/{}/net/", matrix.id))
        .collect::<Vec<_>>();
    let nets = document
        .nets
        .iter()
        .filter(|net| {
            !net.id
                .starts_with(&format!("generated/electrical/{board_id}/"))
                && !matrix_net_prefixes
                    .iter()
                    .any(|prefix| net.id.starts_with(prefix))
        })
        .filter_map(|net| {
            let pins = net
                .pins
                .iter()
                .filter(|pin| {
                    conflicts.contains(pin.part_id.as_str())
                        && proposed.contains(&(pin.part_id.as_str(), pin.pad_id.as_str()))
                })
                .map(|pin| (pin.part_id.clone(), pin.pad_id.clone()))
                .collect::<BTreeSet<_>>();
            (!pins.is_empty()).then(|| ExistingConnectionNet {
                id: net.id.clone(),
                name: net.name.clone(),
                pins,
            })
        })
        .collect::<Vec<_>>();
    let pin_count = nets.iter().map(|net| net.pins.len()).sum();
    (!nets.is_empty()).then_some(ExistingConnectionReview { nets, pin_count })
}

pub fn release_reviewed_connections(
    document: &ProjectDoc,
    review: &ExistingConnectionReview,
) -> ProjectDoc {
    let mut proposal = document.clone();
    for net in &mut proposal.nets {
        let Some(reviewed) = review.nets.iter().find(|reviewed| reviewed.id == net.id) else {
            continue;
        };
        net.pins.retain(|pin| !reviewed_pin(pin, reviewed));
    }
    proposal
}

fn reviewed_pin(pin: &Pin, reviewed: &ExistingConnectionNet) -> bool {
    reviewed
        .pins
        .contains(&(pin.part_id.clone(), pin.pad_id.clone()))
}

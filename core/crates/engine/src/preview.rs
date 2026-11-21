use super::*;

pub(crate) fn changed_ids(a: &ProjectDoc, b: &ProjectDoc) -> Vec<String> {
    let mut ids = BTreeSet::new();
    for part in a.parts.iter().chain(b.parts.iter()) {
        ids.insert(part.id.clone());
    }
    for feature in a.outline.iter().chain(b.outline.iter()) {
        ids.insert(feature.id().to_string());
    }
    ids.into_iter().collect()
}

pub(crate) fn preview_electrical_loss_findings(
    accepted: &ProjectDoc,
    staged: &ProjectDoc,
) -> Vec<Finding> {
    let mut accepted_nets: BTreeMap<&str, &str> = BTreeMap::new();
    let mut accepted_pins = BTreeSet::new();
    for net in &accepted.nets {
        accepted_nets
            .entry(&net.id)
            .and_modify(|name| *name = (*name).min(&net.name))
            .or_insert(&net.name);
        for pin in &net.pins {
            accepted_pins.insert((net.id.as_str(), pin.part_id.as_str(), pin.pad_id.as_str()));
        }
    }

    let staged_nets: BTreeSet<_> = staged.nets.iter().map(|net| net.id.as_str()).collect();
    let staged_pins: BTreeSet<_> = staged
        .nets
        .iter()
        .flat_map(|net| {
            net.pins
                .iter()
                .map(move |pin| (net.id.as_str(), pin.part_id.as_str(), pin.pad_id.as_str()))
        })
        .collect();
    let parts: BTreeMap<&str, &str> =
        accepted
            .parts
            .iter()
            .fold(BTreeMap::new(), |mut parts, part| {
                parts
                    .entry(&part.id)
                    .and_modify(|reference| *reference = (*reference).min(&part.reference))
                    .or_insert(&part.reference);
                parts
            });
    let boards: BTreeMap<&str, &str> =
        accepted
            .boards
            .iter()
            .fold(BTreeMap::new(), |mut boards, board| {
                boards
                    .entry(&board.id)
                    .and_modify(|name| *name = (*name).min(&board.name))
                    .or_insert(&board.name);
                boards
            });
    let staged_registrations: BTreeSet<_> = staged
        .boards
        .iter()
        .flat_map(|board| {
            board
                .net_ids
                .iter()
                .map(move |net_id| (board.id.as_str(), net_id.as_str()))
        })
        .collect();

    let mut findings = BTreeMap::new();
    for (net_id, part_id, pad_id) in accepted_pins.difference(&staged_pins) {
        let net_name = accepted_nets.get(net_id).copied().unwrap_or(net_id);
        let reference = parts.get(part_id).copied().unwrap_or(part_id);
        let finding = Finding {
            id: preview_electrical_notice_id("pin", &[net_id, part_id, pad_id]),
            severity: Severity::Info,
            scope: Scope::Pcb,
            message: format!(
                "Preview removes pad {pad_id} on {reference} ({part_id}) from net {net_name} ({net_id})."
            ),
            target_ids: vec![(*net_id).to_string(), (*part_id).to_string()],
        };
        findings.insert(finding.id.clone(), finding);
    }

    for (net_id, net_name) in &accepted_nets {
        if !staged_nets.contains(net_id) {
            let finding = Finding {
                id: preview_electrical_notice_id("net", &[net_id]),
                severity: Severity::Info,
                scope: Scope::Pcb,
                message: format!("Preview removes net {net_name} ({net_id})."),
                target_ids: vec![(*net_id).to_string()],
            };
            findings.insert(finding.id.clone(), finding);
        }
    }

    let accepted_registrations: BTreeSet<_> = accepted
        .boards
        .iter()
        .flat_map(|board| {
            board
                .net_ids
                .iter()
                .map(move |net_id| (board.id.as_str(), net_id.as_str()))
        })
        .collect();
    for (board_id, net_id) in accepted_registrations.difference(&staged_registrations) {
        let board_name = boards.get(board_id).copied().unwrap_or(board_id);
        let net_name = accepted_nets.get(net_id).copied().unwrap_or(net_id);
        let finding = Finding {
            id: preview_electrical_notice_id("registration", &[board_id, net_id]),
            severity: Severity::Info,
            scope: Scope::Pcb,
            message: format!(
                "Preview removes net {net_name} ({net_id}) from board {board_name} ({board_id})."
            ),
            target_ids: vec![(*board_id).to_string(), (*net_id).to_string()],
        };
        findings.insert(finding.id.clone(), finding);
    }

    findings.into_values().collect()
}

pub(crate) fn preview_electrical_notice_id(kind: &str, identity: &[&str]) -> String {
    let mut id = format!("preview:electrical:{kind}");
    for component in identity {
        id.push(':');
        id.push_str(&component.len().to_string());
        id.push('-');
        for byte in component.as_bytes() {
            use std::fmt::Write as _;
            write!(id, "{byte:02x}").expect("writing to a string cannot fail");
        }
    }
    id
}

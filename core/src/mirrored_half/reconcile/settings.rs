//! Forward projection of keycap settings and plain nets (ticket 147).
//!
//! Creation copies both once. After that they follow their source like every other linked item:
//!
//! - **Keycap settings** are keyed by board, matrix and key (part) id. A linked mirror key or
//!   matrix shows exactly its source's entry, or none when the source has none. A Local item keeps
//!   its own entry. An Omitted item, and the child of an absent matrix or part, has no entry.
//!   Keymap bindings are not keycap settings and are never touched here; legends are copied as
//!   written.
//! - **Plain nets** are the corresponded nets that are not an embedded circuit's own or joined
//!   net (those are projected with their circuit). A linked mirror net takes the source net's
//!   pins on linked parts, mapped through the part correspondences, and keeps the pins the mirror
//!   owns itself (a Local part, a part with no source). A pin on an Omitted part, or on a part that
//!   is not on the mirror, is dropped.
//! - **Private nets.** A mirror net whose source net has no non-controller pin outside an Omitted
//!   item is absent by rule (ADR-0010). Such a net is removed, with its registration, unless a pin
//!   the mirror owns still uses it; it is projected again the moment it stops being private.
//! - **Name.** A mirror net shows its own name (ADR-0010: net names are for the mirror's side,
//!   and a mirror-side rename already travels to the source through `SetNet`). A net projected
//!   again takes the source name with the board token swapped, as at creation.
//! - **Registration.** A mirror net is registered on the mirror board. `SetNet` execution registers
//!   a new net on the document's first board only; a net the mirror needs is not left to that.
//!
//! Nothing here reads anything but the source half and the staged mirror, so a second pass is a
//! no-op.

use crate::model::{
    KeycapConfiguration, MirroredCorrespondence, MirroredExceptionState, MirroredHalves,
    MirroredItemKind, Net, Pin, ProjectDoc,
};
use std::collections::{BTreeMap, BTreeSet};

/// Projects the keycap settings and plain nets of the source half onto the staged mirror.
pub(super) fn project(
    document: &ProjectDoc,
    staged: &mut ProjectDoc,
    halves: &MirroredHalves,
) -> Result<(), String> {
    keycaps(document, staged, halves);
    plain_nets(document, staged, halves);
    Ok(())
}

/// How a correspondence participates: `Some(state)` is an overlay, `None` is linked.
fn overlay(halves: &MirroredHalves, correspondence: &MirroredCorrespondence) -> Option<State> {
    halves
        .overlay_of(correspondence)
        .map(|exception| match exception.state {
            MirroredExceptionState::Local => State::Local,
            MirroredExceptionState::Omitted => State::Omitted,
        })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Local,
    Omitted,
}

/// Sets `key` to `value`, or removes it when the source has none.
fn follow<T>(map: &mut BTreeMap<String, T>, key: &str, value: Option<T>) {
    match value {
        Some(value) => {
            map.insert(key.to_owned(), value);
        }
        None => {
            map.remove(key);
        }
    }
}

fn keycaps(document: &ProjectDoc, staged: &mut ProjectDoc, halves: &MirroredHalves) {
    let empty = KeycapConfiguration::default();
    let source = document.keycaps.as_ref().unwrap_or(&empty);
    let mut next = staged.keycaps.clone().unwrap_or_default();

    // Keycap settings have no exception kind of their own: the board follows the pair's boards.
    follow(
        &mut next.boards,
        &halves.mirror_board_id,
        source.boards.get(&halves.source_board_id).cloned(),
    );

    let matrices: BTreeSet<&str> = staged.matrices.iter().map(|m| m.id.as_str()).collect();
    let parts: BTreeSet<&str> = staged.parts.iter().map(|p| p.id.as_str()).collect();
    for correspondence in &halves.correspondences {
        let (settings, present) = match correspondence.kind {
            MirroredItemKind::Matrix => (
                source.matrices.get(&correspondence.source_id).cloned(),
                matrices.contains(correspondence.mirror_id.as_str()),
            ),
            MirroredItemKind::Part => (None, parts.contains(correspondence.mirror_id.as_str())),
            _ => continue,
        };
        let state = overlay(halves, correspondence);
        if state == Some(State::Local) {
            continue;
        }
        let linked = state.is_none() && present;
        match correspondence.kind {
            MirroredItemKind::Matrix => follow(
                &mut next.matrices,
                &correspondence.mirror_id,
                settings.filter(|_| linked),
            ),
            _ => follow(
                &mut next.keys,
                &correspondence.mirror_id,
                source
                    .keys
                    .get(&correspondence.source_id)
                    .cloned()
                    .filter(|_| linked),
            ),
        }
    }

    if staged.keycaps.is_some() || next != KeycapConfiguration::default() {
        staged.keycaps = Some(next);
    }
}

fn plain_nets(document: &ProjectDoc, staged: &mut ProjectDoc, halves: &MirroredHalves) {
    // An embedded circuit's own and joined nets are projected with the circuit.
    let circuit_nets: BTreeSet<&str> = document
        .embedded_circuits
        .iter()
        .flat_map(|circuit| {
            circuit
                .net_ids
                .iter()
                .chain(circuit.ports.values())
                .map(String::as_str)
        })
        .collect();
    let linked_parts: BTreeMap<&str, &str> = halves
        .correspondences
        .iter()
        .filter(|c| c.kind == MirroredItemKind::Part && overlay(halves, c).is_none())
        .map(|c| (c.source_id.as_str(), c.mirror_id.as_str()))
        .collect();
    let linked_mirror_parts: BTreeSet<&str> = linked_parts.values().copied().collect();

    for correspondence in halves
        .correspondences
        .iter()
        .filter(|c| c.kind == MirroredItemKind::Net && overlay(halves, c).is_none())
    {
        let (source_id, mirror_id) = (&correspondence.source_id, &correspondence.mirror_id);
        if source_id.starts_with("embedded/")
            || mirror_id.starts_with("embedded/")
            || circuit_nets.contains(source_id.as_str())
            || circuit_nets.contains(mirror_id.as_str())
        {
            continue;
        }
        let Some(source) = document.nets.iter().find(|net| net.id == *source_id) else {
            continue;
        };
        let parts: BTreeSet<&str> = staged.parts.iter().map(|p| p.id.as_str()).collect();
        let current = staged.nets.iter().find(|net| net.id == *mirror_id);

        let mut pins: Vec<Pin> = Vec::new();
        for pin in &source.pins {
            let Some(mirror_part) = linked_parts.get(pin.part_id.as_str()) else {
                continue;
            };
            let mapped = Pin {
                part_id: (*mirror_part).to_owned(),
                pad_id: pin.pad_id.clone(),
            };
            if parts.contains(mirror_part) && !pins.contains(&mapped) {
                pins.push(mapped);
            }
        }
        let own: Vec<Pin> = current
            .map(|net| {
                net.pins
                    .iter()
                    .filter(|pin| {
                        parts.contains(pin.part_id.as_str())
                            && !linked_mirror_parts.contains(pin.part_id.as_str())
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        let private = crate::mechanical::halves::private_net_is_absent_with(
            document,
            staged,
            halves,
            source_id,
            &BTreeSet::new(),
        );
        if private && own.is_empty() {
            staged.nets.retain(|net| net.id != *mirror_id);
            for board in &mut staged.boards {
                board.net_ids.retain(|id| id != mirror_id);
            }
            continue;
        }
        for pin in own {
            if !pins.contains(&pin) {
                pins.push(pin);
            }
        }

        // A net the mirror shows keeps its own name (ADR-0010: net names are for the mirror's side);
        // a net projected again takes the source name with the board token swapped, as at creation.
        let name = super::super::mirror_net_name(
            &halves.source_board_id,
            &halves.mirror_board_id,
            &source.name,
        );
        match staged.nets.iter_mut().find(|net| net.id == *mirror_id) {
            Some(net) => {
                // A reorder alone is not a change.
                let same_pins =
                    net.pins.len() == pins.len() && net.pins.iter().all(|pin| pins.contains(pin));
                if !same_pins {
                    net.pins = pins;
                }
            }
            None => staged.nets.push(Net {
                id: mirror_id.clone(),
                name,
                pins,
            }),
        }
        if let Some(board) = staged
            .boards
            .iter_mut()
            .find(|board| board.id == halves.mirror_board_id)
            && !board.net_ids.contains(mirror_id)
        {
            board.net_ids.push(mirror_id.clone());
        }
    }
}

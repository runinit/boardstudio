//! Mirrored-half records of explicit mechanical mirror pairs.
//!
//! A pair with `halves` records which board is the source, which board is its reflection,
//! the counterpart of each item (correspondences) and any per-item exception. Absent `halves`
//! means the pair is unpaired: it links mechanical features and nothing else.

use crate::model::{
    MechanicalMirrorPair, MirroredCompanionMode, MirroredCompanionTweak, MirroredCorrespondence,
    MirroredException, MirroredExceptionState, MirroredHalves, MirroredItemKind, ProjectDoc,
};

impl MirroredHalves {
    /// The correspondence an exception overlays: same kind and mirror item. An exception without
    /// one is a legacy, unassociated record.
    pub(crate) fn overlaid_correspondence(
        &self,
        exception: &MirroredException,
    ) -> Option<&MirroredCorrespondence> {
        self.correspondences.iter().find(|correspondence| {
            correspondence.kind == exception.kind && correspondence.mirror_id == exception.mirror_id
        })
    }

    /// The authored item an overlay belongs to, from its correspondence.
    pub(crate) fn overlay_source(&self, exception: &MirroredException) -> Option<&str> {
        self.overlaid_correspondence(exception)
            .map(|correspondence| correspondence.source_id.as_str())
    }

    /// The state overlay on a correspondence, if the mirror item is Local or Omitted.
    pub(crate) fn overlay_of(
        &self,
        correspondence: &MirroredCorrespondence,
    ) -> Option<&MirroredException> {
        self.exceptions.iter().find(|exception| {
            exception.kind == correspondence.kind && exception.mirror_id == correspondence.mirror_id
        })
    }

    /// The overlay on the item that mirrors `source_id`, when one is stored.
    pub(crate) fn overlay_for_source(
        &self,
        kind: MirroredItemKind,
        source_id: &str,
    ) -> Option<&MirroredException> {
        self.correspondences
            .iter()
            .find(|correspondence| {
                correspondence.kind == kind && correspondence.source_id == source_id
            })
            .and_then(|correspondence| self.overlay_of(correspondence))
    }
}

/// A mirror net that is absent by rule, with nothing stored: every pin of its source net that is
/// not on the controller belongs to an Omitted (absent) mirror part. A net with no such pin is not
/// private to anything.
pub(crate) fn private_net_is_absent(
    document: &ProjectDoc,
    halves: &MirroredHalves,
    source_net_id: &str,
) -> bool {
    private_net_is_absent_with(
        document,
        document,
        halves,
        source_net_id,
        &std::collections::BTreeSet::new(),
    )
}

/// The same rule, reading the source net from `source_document` and the mirror parts from
/// `mirror_document`, with `also_absent` mirror parts counted as absent. Omit uses this to find
/// the nets it is about to leave private, before it removes the parts.
pub(crate) fn private_net_is_absent_with(
    source_document: &ProjectDoc,
    mirror_document: &ProjectDoc,
    halves: &MirroredHalves,
    source_net_id: &str,
    also_absent: &std::collections::BTreeSet<String>,
) -> bool {
    let Some(source_net) = source_document
        .nets
        .iter()
        .find(|net| net.id == source_net_id)
    else {
        return false;
    };
    let controller = source_document
        .hardware
        .as_ref()
        .and_then(|hardware| {
            hardware
                .boards
                .iter()
                .find(|board| board.board_id == halves.source_board_id)
        })
        .and_then(|board| board.controller_part_id.as_deref());
    let mut owned = false;
    for pin in &source_net.pins {
        if Some(pin.part_id.as_str()) == controller {
            continue;
        }
        let Some(mirror) = halves.correspondences.iter().find(|correspondence| {
            correspondence.kind == MirroredItemKind::Part && correspondence.source_id == pin.part_id
        }) else {
            return false;
        };
        let present = !also_absent.contains(&mirror.mirror_id)
            && (mirror_document
                .parts
                .iter()
                .any(|part| part.id == mirror.mirror_id)
                || mirror_document
                    .boards
                    .iter()
                    .any(|board| board.part_ids.contains(&mirror.mirror_id)));
        if present {
            return false;
        }
        owned = true;
    }
    owned
}

/// Dissolves the specifically requested mirrored pair through the existing mechanical edit
/// lifecycle, discovering its normalized clearance template only when Unlink is requested.
/// The source member and pair are resolved before dispatch so a malformed or ambiguous record
/// cannot redirect the operation to another pair.
pub(crate) fn unlink_pair(document: &mut ProjectDoc, pair_id: &str) -> Result<Vec<String>, String> {
    validate_document(document)?;
    let hardware = document
        .hardware
        .as_ref()
        .ok_or_else(|| "Physical instances are unavailable".to_owned())?;
    let mut matches = hardware.pairs.iter().filter(|pair| pair.id == pair_id);
    let pair = matches
        .next()
        .ok_or_else(|| format!("Mechanical pair {pair_id:?} is unavailable."))?;
    if matches.next().is_some() {
        return Err(format!("Mechanical pair {pair_id:?} is ambiguous."));
    }
    let halves = pair
        .halves
        .as_ref()
        .ok_or_else(|| format!("Mechanical pair {pair_id:?} has no mirrored halves record."))?;
    let source_instance_id = pair
        .construction_source
        .as_deref()
        .filter(|source| pair.members.iter().any(|member| member == source))
        .ok_or_else(|| format!("Mechanical pair {pair_id:?} has no valid construction source."))?;
    let source_instances: Vec<_> = hardware
        .instances
        .iter()
        .filter(|instance| instance.id == source_instance_id)
        .collect();
    if source_instances.len() != 1 || source_instances[0].board_id != halves.source_board_id {
        return Err(format!(
            "Mechanical pair {pair_id:?} has no unique construction-source instance on its source board."
        ));
    }
    if hardware
        .pairs
        .iter()
        .filter(|candidate| {
            candidate
                .members
                .iter()
                .any(|member| member == source_instance_id)
        })
        .count()
        != 1
    {
        return Err(format!(
            "Construction-source instance {source_instance_id:?} does not identify exactly one mechanical pair."
        ));
    }

    let mounting_hole = crate::generators::catalogue()?
        .into_iter()
        .find(|definition| {
            definition
                .generator
                .as_ref()
                .is_some_and(|generator| generator.source == "ceoloide/mounting_hole_npth")
        })
        .ok_or_else(|| "The normalized mounting-hole generator is unavailable.".to_owned())?;
    let mut draft = document.clone();
    let changed = crate::mechanical::edit::apply(
        &mut draft,
        &[],
        &crate::mechanical::MechanicalEdit {
            target: crate::mechanical::MechanicalTarget::Instance {
                instance_id: source_instance_id.to_owned(),
            },
            change: crate::mechanical::MechanicalChange::UnlinkMirrorPair {
                instance_id: source_instance_id.to_owned(),
            },
            mounting_hole: Some(Box::new(mounting_hole)),
        },
    )?;
    if draft
        .hardware
        .as_ref()
        .is_some_and(|hardware| hardware.pairs.iter().any(|pair| pair.id == pair_id))
    {
        return Err(format!(
            "Mechanical pair {pair_id:?} remained after its Unlink lifecycle."
        ));
    }
    validate_document(&draft)?;
    *document = draft;
    Ok(changed)
}

/// Shared validation for the creation request and each item's effective placement settings.
pub(crate) fn validate_companion(
    mode: MirroredCompanionMode,
    tweak: Option<MirroredCompanionTweak>,
) -> Result<(), String> {
    if let Some(tweak) = tweak {
        if mode != MirroredCompanionMode::LiteralMirrorWithTweak {
            return Err(
                "companionTweak is only valid with companionMode literal-mirror-with-tweak".into(),
            );
        }
        if !(tweak.x.is_finite() && tweak.y.is_finite() && tweak.rotation.is_finite()) {
            return Err("companionTweak x, y and rotation must be finite".into());
        }
    }
    Ok(())
}

/// Checks the construction source and the mirrored-half record of every pair in the document.
///
/// Returns the first violation, located by pair id and by the record's index. It only reads the
/// document, so a refused record changes nothing. Pairs without `halves` are checked for their
/// construction source only. A record needs a construction source, and its boards are the boards
/// of the two members. A board may be a half of one record only; a record whose source and mirror
/// are the same board (a reused board) is one record. Ticket 18 decides whether one board can be
/// a half of several records.
pub fn validate_document(document: &ProjectDoc) -> Result<(), String> {
    let Some(hardware) = document.hardware.as_ref() else {
        return Ok(());
    };
    // (board id, pair id) for every board a record already claims.
    let mut claimed: Vec<(&str, &str)> = Vec::new();
    for pair in &hardware.pairs {
        if let Some(source) = &pair.construction_source
            && !pair.members.contains(source)
        {
            return Err(format!(
                "mechanical pair {:?} names construction source {source:?}, which is not a pair member",
                pair.id
            ));
        }
        let Some(halves) = &pair.halves else {
            continue;
        };
        validate_halves(document, pair, halves)?;
        let mut boards = vec![halves.source_board_id.as_str()];
        if halves.mirror_board_id != halves.source_board_id {
            boards.push(halves.mirror_board_id.as_str());
        }
        for board_id in boards {
            if let Some((_, other)) = claimed.iter().find(|(board, _)| *board == board_id) {
                return Err(format!(
                    "mechanical pair {:?} halves name board {board_id:?}, which is already a half of pair {other:?}",
                    pair.id
                ));
            }
            claimed.push((board_id, &pair.id));
        }
    }
    Ok(())
}

fn omitted_parent_owns_missing_matrix_child(
    document: &ProjectDoc,
    halves: &MirroredHalves,
    correspondence: &crate::model::MirroredCorrespondence,
) -> bool {
    if correspondence.kind != MirroredItemKind::Part {
        return false;
    }
    let Some((source_parent_id, source_child_id)) = correspondence.source_id.rsplit_once('/')
    else {
        return false;
    };
    let Some((mirror_parent_id, mirror_child_id)) = correspondence.mirror_id.rsplit_once('/')
    else {
        return false;
    };
    if source_child_id.is_empty()
        || source_child_id.contains('/')
        || source_child_id != mirror_child_id
    {
        return false;
    }

    // The matrix keeps its correspondence in every state; a Local overlay does not change that.
    let mut matrix_associations = halves
        .correspondences
        .iter()
        .filter(|record| {
            record.kind == MirroredItemKind::Matrix
                && correspondence
                    .source_id
                    .starts_with(&format!("matrix/{}/", record.source_id))
                && correspondence
                    .mirror_id
                    .starts_with(&format!("matrix/{}/", record.mirror_id))
        })
        .map(|record| (record.source_id.as_str(), record.mirror_id.as_str()));
    let Some((source_matrix_id, mirror_matrix_id)) = matrix_associations.next() else {
        return false;
    };
    if matrix_associations.next().is_some() {
        return false;
    }
    let source_prefix = format!("matrix/{source_matrix_id}/");
    let mirror_prefix = format!("matrix/{mirror_matrix_id}/");
    let Some(source_slot) = source_parent_id.strip_prefix(&source_prefix) else {
        return false;
    };
    let Some(mirror_slot) = mirror_parent_id.strip_prefix(&mirror_prefix) else {
        return false;
    };
    if source_slot != mirror_slot {
        return false;
    }
    let mut source_matrices = document
        .matrices
        .iter()
        .filter(|matrix| matrix.id == source_matrix_id);
    let Some(source_matrix) = source_matrices.next() else {
        return false;
    };
    if source_matrices.next().is_some() {
        return false;
    }
    let mut mirror_matrices = document
        .matrices
        .iter()
        .filter(|matrix| matrix.id == mirror_matrix_id);
    let Some(mirror_matrix) = mirror_matrices.next() else {
        return false;
    };
    if mirror_matrices.next().is_some() {
        return false;
    }
    if source_matrix.board_id.as_deref() != Some(halves.source_board_id.as_str())
        || mirror_matrix.board_id.as_deref() != Some(halves.mirror_board_id.as_str())
    {
        return false;
    }
    let matrix_owns = |matrix: &crate::model::Matrix, parent_id: &str, child_id: &str| {
        matrix.part_ids.iter().any(|id| id == parent_id)
            && matrix.part_ids.iter().any(|id| id == child_id)
            && matrix.cells.iter().any(|cell| {
                !cell.deleted
                    && cell.enabled
                    && crate::matrix::member_id(&matrix.id, cell.row, cell.column) == parent_id
                    && cell
                        .assemblies
                        .iter()
                        .any(|assembly| assembly.id == source_child_id)
            })
    };
    let omitted_target_slot = mirror_matrix
        .cells
        .iter()
        .filter(|cell| {
            cell.deleted
                && !cell.enabled
                && crate::matrix::member_id(&mirror_matrix.id, cell.row, cell.column)
                    == mirror_parent_id
                && cell.row < mirror_matrix.rows
                && cell.column < mirror_matrix.columns
        })
        .count()
        == 1;
    if !matrix_owns(source_matrix, source_parent_id, &correspondence.source_id)
        || !(matrix_owns(mirror_matrix, mirror_parent_id, &correspondence.mirror_id)
            || omitted_target_slot)
    {
        return false;
    }

    let matching_parent_exceptions: Vec<_> = halves
        .exceptions
        .iter()
        .filter(|exception| {
            exception.kind == MirroredItemKind::Part
                && (halves.overlay_source(exception) == Some(source_parent_id)
                    || exception.mirror_id == mirror_parent_id)
        })
        .collect();
    if matching_parent_exceptions.len() != 1
        || halves.overlay_source(matching_parent_exceptions[0]) != Some(source_parent_id)
        || matching_parent_exceptions[0].mirror_id != mirror_parent_id
        || matching_parent_exceptions[0].state != crate::model::MirroredExceptionState::Omitted
        || halves.exceptions.iter().any(|exception| {
            exception.kind == MirroredItemKind::Part
                && exception.mirror_id == correspondence.mirror_id
        })
    {
        return false;
    }

    let exactly_on_board = |id: &str, board_id: &str| {
        document.parts.iter().filter(|part| part.id == id).count() == 1
            && document
                .boards
                .iter()
                .filter(|board| {
                    board.id == board_id && board.part_ids.iter().any(|part_id| part_id == id)
                })
                .count()
                == 1
            && document
                .boards
                .iter()
                .filter(|board| board.part_ids.iter().any(|part_id| part_id == id))
                .count()
                == 1
    };
    let absent_from_document_and_boards = |id: &str| {
        !document.parts.iter().any(|part| part.id == id)
            && !document
                .boards
                .iter()
                .any(|board| board.part_ids.iter().any(|part_id| part_id == id))
    };
    exactly_on_board(source_parent_id, &halves.source_board_id)
        && exactly_on_board(&correspondence.source_id, &halves.source_board_id)
        && absent_from_document_and_boards(mirror_parent_id)
        && absent_from_document_and_boards(&correspondence.mirror_id)
}

/// Absent inherited members keep their exact durable correspondence under an Omitted owner.
/// Namespaces only confirm identities after actual source ownership has been established.
fn omitted_owner_owns_missing_item(
    document: &ProjectDoc,
    halves: &MirroredHalves,
    correspondence: &crate::model::MirroredCorrespondence,
) -> bool {
    use crate::model::{MirroredExceptionState, MirroredItemKind};
    let absent = match correspondence.kind {
        MirroredItemKind::Part => {
            !document
                .parts
                .iter()
                .any(|p| p.id == correspondence.mirror_id)
                && !document
                    .boards
                    .iter()
                    .any(|b| b.part_ids.contains(&correspondence.mirror_id))
        }
        MirroredItemKind::Net => {
            !document
                .nets
                .iter()
                .any(|n| n.id == correspondence.mirror_id)
                && !document
                    .boards
                    .iter()
                    .any(|b| b.net_ids.contains(&correspondence.mirror_id))
        }
        _ => false,
    };
    if !absent
        || halves
            .exceptions
            .iter()
            .any(|e| e.kind == correspondence.kind && e.mirror_id == correspondence.mirror_id)
    {
        return false;
    }
    halves
        .exceptions
        .iter()
        .filter(|owner| owner.state == MirroredExceptionState::Omitted)
        .filter_map(|owner| halves.overlay_source(owner).map(|source| (owner, source)))
        .filter(|(owner, source)| match (owner.kind, correspondence.kind) {
            (MirroredItemKind::Matrix, MirroredItemKind::Part) => {
                !document.matrices.iter().any(|m| m.id == owner.mirror_id)
                    && document
                        .matrices
                        .iter()
                        .filter(|m| {
                            m.id == *source
                                && m.board_id.as_deref() == Some(halves.source_board_id.as_str())
                                && m.part_ids.contains(&correspondence.source_id)
                                && m.cells.iter().any(|cell| {
                                    let key =
                                        crate::matrix::member_id(&m.id, cell.row, cell.column);
                                    cell.enabled
                                        && !cell.deleted
                                        && (key == correspondence.source_id
                                            || cell.assemblies.iter().any(|a| {
                                                crate::matrix::companion_id(&key, &a.id)
                                                    == correspondence.source_id
                                            }))
                                })
                        })
                        .count()
                        == 1
                    && correspondence
                        .source_id
                        .strip_prefix(&format!("matrix/{source}/"))
                        .is_some_and(|suffix| {
                            correspondence.mirror_id
                                == format!("matrix/{}/{suffix}", owner.mirror_id)
                        })
            }
            (MirroredItemKind::EmbeddedCircuit, MirroredItemKind::Part | MirroredItemKind::Net) => {
                !document
                    .embedded_circuits
                    .iter()
                    .any(|c| c.id == owner.mirror_id)
                    && document
                        .embedded_circuits
                        .iter()
                        .filter(|c| {
                            c.id == *source
                                && c.host_board_id == halves.source_board_id
                                && match correspondence.kind {
                                    MirroredItemKind::Part => {
                                        c.part_ids.contains(&correspondence.source_id)
                                    }
                                    _ => c.net_ids.contains(&correspondence.source_id),
                                }
                        })
                        .count()
                        == 1
                    && correspondence
                        .source_id
                        .strip_prefix(&format!("embedded/{source}/"))
                        .is_some_and(|suffix| {
                            correspondence.mirror_id
                                == format!("embedded/{}/{suffix}", owner.mirror_id)
                        })
            }
            _ => false,
        })
        .count()
        == 1
}

fn validate_halves(
    document: &ProjectDoc,
    pair: &MechanicalMirrorPair,
    halves: &MirroredHalves,
) -> Result<(), String> {
    for (field, board_id) in [
        ("sourceBoardId", &halves.source_board_id),
        ("mirrorBoardId", &halves.mirror_board_id),
    ] {
        if !document.boards.iter().any(|board| board.id == *board_id) {
            return Err(format!(
                "mechanical pair {:?} halves.{field} {board_id:?} is not a board in the document",
                pair.id
            ));
        }
    }
    // The record is relative to the construction source, so it needs one, and the source half
    // must sit on the source board. A member that is not an instance is reported by the pair
    // checks, so it is not repeated here.
    let Some(source) = pair.construction_source.as_deref() else {
        return Err(format!(
            "mechanical pair {:?} records halves without a constructionSource",
            pair.id
        ));
    };
    let instance_board = |member: &str| {
        document
            .hardware
            .as_ref()
            .and_then(|hardware| hardware.instances.iter().find(|entry| entry.id == member))
            .map(|entry| entry.board_id.as_str())
    };
    let mirror = pair.members.iter().find(|member| member.as_str() != source);
    for (field, member, board_id) in [
        ("sourceBoardId", Some(source), &halves.source_board_id),
        (
            "mirrorBoardId",
            mirror.map(String::as_str),
            &halves.mirror_board_id,
        ),
    ] {
        if let Some(member) = member
            && let Some(actual) = instance_board(member)
            && actual != board_id
        {
            return Err(format!(
                "mechanical pair {:?} halves.{field} {board_id:?} is not the board of {member:?}, which is on {actual:?}",
                pair.id
            ));
        }
    }
    for (index, correspondence) in halves.correspondences.iter().enumerate() {
        let location = format!(
            "mechanical pair {:?} halves.correspondences[{index}]",
            pair.id
        );
        validate_companion(
            correspondence
                .companion_mode
                .unwrap_or(halves.companion_mode),
            correspondence.companion_tweak,
        )
        .map_err(|error| format!("{location}: {error}"))?;
        let kind = kind_name(correspondence.kind);
        if !item_exists(
            document,
            pair,
            &halves.source_board_id,
            correspondence.kind,
            &correspondence.source_id,
        ) {
            return Err(format!(
                "{location} names {kind} {:?}, which is not on board {:?}",
                correspondence.source_id, halves.source_board_id
            ));
        }
        let overlay = halves.overlay_of(correspondence);
        let omitted_overlay =
            overlay.is_some_and(|exception| exception.state == MirroredExceptionState::Omitted);
        if !item_exists(
            document,
            pair,
            &halves.mirror_board_id,
            correspondence.kind,
            &correspondence.mirror_id,
        ) && !(omitted_overlay
            && mirror_item_is_absent(
                document,
                pair,
                correspondence.kind,
                &correspondence.mirror_id,
            ))
            && !omitted_parent_owns_missing_matrix_child(document, halves, correspondence)
            && !omitted_owner_owns_missing_item(document, halves, correspondence)
            && !(correspondence.kind == MirroredItemKind::Net
                && private_net_is_absent(document, halves, &correspondence.source_id))
        {
            return Err(format!(
                "{location} names {kind} {:?}, which is not on board {:?}",
                correspondence.mirror_id, halves.mirror_board_id
            ));
        }
        let earlier = &halves.correspondences[..index];
        if earlier.iter().any(|other| {
            other.kind == correspondence.kind && other.source_id == correspondence.source_id
        }) {
            return Err(format!(
                "{location} repeats source {kind} {:?}",
                correspondence.source_id
            ));
        }
        if earlier.iter().any(|other| {
            other.kind == correspondence.kind && other.mirror_id == correspondence.mirror_id
        }) {
            return Err(format!(
                "{location} repeats mirror {kind} {:?}",
                correspondence.mirror_id
            ));
        }
    }
    for (index, exception) in halves.exceptions.iter().enumerate() {
        let location = format!("mechanical pair {:?} halves.exceptions[{index}]", pair.id);
        validate_companion(
            exception.companion_mode.unwrap_or(halves.companion_mode),
            exception.companion_tweak,
        )
        .map_err(|error| format!("{location}: {error}"))?;
        let kind = kind_name(exception.kind);
        let earlier = &halves.exceptions[..index];
        if earlier
            .iter()
            .any(|other| other.kind == exception.kind && other.mirror_id == exception.mirror_id)
        {
            return Err(format!(
                "{location} repeats {kind} {:?}",
                exception.mirror_id
            ));
        }
        let mirror_exists = item_exists(
            document,
            pair,
            &halves.mirror_board_id,
            exception.kind,
            &exception.mirror_id,
        );
        let associated = halves.overlaid_correspondence(exception).is_some();
        if let Some(copper) = &exception.removed_copper {
            if !associated || exception.state != MirroredExceptionState::Omitted {
                return Err(format!(
                    "{location} keeps removed copper, which only an Omitted overlay on a correspondence may hold"
                ));
            }
            validate_removed_copper(document, halves, copper)
                .map_err(|error| format!("{location}: {error}"))?;
        }
        if associated {
            // The correspondence owns the placement settings; an overlay is only the state.
            if exception.companion_mode.is_some() || exception.companion_tweak.is_some() {
                return Err(format!(
                    "{location} overlays a correspondence, which owns the companion settings"
                ));
            }
            match exception.state {
                MirroredExceptionState::Local if !mirror_exists => {
                    return Err(format!(
                        "{location} names {kind} {:?}, which is not on board {:?}",
                        exception.mirror_id, halves.mirror_board_id
                    ));
                }
                MirroredExceptionState::Omitted
                    if !mirror_item_is_absent(
                        document,
                        pair,
                        exception.kind,
                        &exception.mirror_id,
                    ) =>
                {
                    return Err(format!(
                        "{location} has Omitted {kind} {:?} that is still materialized or registered",
                        exception.mirror_id
                    ));
                }
                _ => {}
            }
        } else if !mirror_exists {
            // A legacy record names an item that is present; it has no inferred source.
            return Err(format!(
                "{location} names {kind} {:?}, which is not on board {:?}",
                exception.mirror_id, halves.mirror_board_id
            ));
        }
    }
    Ok(())
}

/// Whether the mirror item is not materialized or registered anywhere in the document.
fn mirror_item_is_absent(
    document: &ProjectDoc,
    pair: &MechanicalMirrorPair,
    kind: MirroredItemKind,
    id: &str,
) -> bool {
    match kind {
        MirroredItemKind::Part => {
            !document.parts.iter().any(|p| p.id == id)
                && !document
                    .boards
                    .iter()
                    .any(|b| b.part_ids.iter().any(|part| part == id))
        }
        MirroredItemKind::Net => {
            !document.nets.iter().any(|n| n.id == id)
                && !document
                    .boards
                    .iter()
                    .any(|b| b.net_ids.iter().any(|net| net == id))
        }
        MirroredItemKind::Matrix => !document.matrices.iter().any(|m| m.id == id),
        MirroredItemKind::EmbeddedCircuit => !document.embedded_circuits.iter().any(|c| c.id == id),
        MirroredItemKind::Layout => !document.layouts.iter().any(|l| l.id == id),
        _ => !document
            .boards
            .iter()
            .any(|b| item_exists(document, pair, &b.id, kind, id)),
    }
}

/// Saved copper must not collide with live copper on the mirror board or repeat itself.
fn validate_removed_copper(
    document: &ProjectDoc,
    halves: &MirroredHalves,
    copper: &crate::model::OmittedCopper,
) -> Result<(), String> {
    let board = document
        .boards
        .iter()
        .find(|board| board.id == halves.mirror_board_id)
        .ok_or_else(|| "the mirror board is unavailable".to_owned())?;
    let mut trace_ids = std::collections::BTreeSet::new();
    for trace in &copper.traces {
        if !trace_ids.insert(trace.id.as_str())
            || board.traces.iter().any(|live| live.id == trace.id)
        {
            return Err(format!(
                "removed trace {:?} repeats or collides with live copper",
                trace.id
            ));
        }
    }
    let mut via_ids = std::collections::BTreeSet::new();
    for via in &copper.vias {
        if !via_ids.insert(via.id.as_str()) || board.vias.iter().any(|live| live.id == via.id) {
            return Err(format!(
                "removed via {:?} repeats or collides with live copper",
                via.id
            ));
        }
    }
    for pad in &copper.pads {
        if !(pad.at.x.is_finite() && pad.at.y.is_finite()) {
            return Err(format!(
                "removed pad {:?}/{:?} has a non-finite position",
                pad.part_id, pad.pad_id
            ));
        }
    }
    Ok(())
}

/// Whether an item of `kind` with `id` exists on the board `board_id`. Assemblies are definitions
/// that belong to the document rather than to a board, so the board is not consulted for them.
/// Parts, outlines and nets are listed by the board; a matrix, layout, module or embedded circuit names
/// its board; mechanical features belong to the pair.
fn item_exists(
    document: &ProjectDoc,
    pair: &MechanicalMirrorPair,
    board_id: &str,
    kind: MirroredItemKind,
    id: &str,
) -> bool {
    let board = document.boards.iter().find(|board| board.id == board_id);
    match kind {
        MirroredItemKind::Part => {
            board.is_some_and(|board| board.part_ids.iter().any(|part| part == id))
                && document.parts.iter().any(|part| part.id == id)
        }
        MirroredItemKind::Outline => {
            board.is_some_and(|board| board.outline_ids.iter().any(|outline| outline == id))
        }
        MirroredItemKind::Net => {
            board.is_some_and(|board| board.net_ids.iter().any(|net| net == id))
        }
        // A matrix that names no board is not placed on one, so it matches either half.
        MirroredItemKind::Matrix => document.matrices.iter().any(|matrix| {
            matrix.id == id
                && matrix
                    .board_id
                    .as_deref()
                    .is_none_or(|matrix_board| matrix_board == board_id)
        }),
        MirroredItemKind::Layout => document
            .layouts
            .iter()
            .any(|layout| layout.id == id && layout.board_id == board_id),
        MirroredItemKind::Assembly => document.assemblies.iter().any(|assembly| assembly.id == id),
        MirroredItemKind::Module => document
            .modules
            .iter()
            .any(|module| module.id == id && module.host_board_id == board_id),
        MirroredItemKind::ElectricalBoard => {
            id == board_id
                && document.hardware.as_ref().is_some_and(|hardware| {
                    hardware.boards.iter().any(|entry| entry.board_id == id)
                })
        }
        MirroredItemKind::OutlineVersion => document.board_outlines.iter().any(|outline| {
            outline.board_id == board_id && outline.versions.iter().any(|version| version.id == id)
        }),
        MirroredItemKind::MechanicalFeature => pair.features.iter().any(|feature| feature.id == id),
        MirroredItemKind::EmbeddedCircuit => document
            .embedded_circuits
            .iter()
            .any(|circuit| circuit.id == id && circuit.host_board_id == board_id),
    }
}

/// The name a kind is written under in the document, for messages.
fn kind_name(kind: MirroredItemKind) -> &'static str {
    match kind {
        MirroredItemKind::Matrix => "matrix",
        MirroredItemKind::Layout => "layout",
        MirroredItemKind::Part => "part",
        MirroredItemKind::Outline => "outline",
        MirroredItemKind::Net => "net",
        MirroredItemKind::Assembly => "assembly",
        MirroredItemKind::Module => "module",
        MirroredItemKind::ElectricalBoard => "electrical-board",
        MirroredItemKind::OutlineVersion => "outline-version",
        MirroredItemKind::MechanicalFeature => "mechanical-feature",
        MirroredItemKind::EmbeddedCircuit => "embedded-circuit",
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::validate_document;
    use crate::model::{Board, HardwareConfiguration, MechanicalMirrorPair, Part, ProjectDoc};

    fn construction() -> Value {
        json!({
            "method": "printed", "mount": "tray", "integratedPlateFrame": false,
            "plateThickness": 1.3, "plateFoamThickness": 2.0, "bottomFoamThickness": 2.0,
            "bottomThickness": 3.0, "plateToPcb": 2.2, "wallThickness": 2.0, "clearance": 0.3
        })
    }

    fn pair(extra: Value) -> MechanicalMirrorPair {
        let mut value = json!({
            "id": "pair", "members": ["left", "right"],
            "mapping": { "at": { "x": 200.0, "y": 0.0 }, "rotation": 0.0 },
            "construction": construction(), "features": []
        });
        value
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        serde_json::from_value(value).unwrap()
    }

    fn halves(correspondences: Value, exceptions: Value) -> Value {
        json!({
            "halves": {
                "sourceBoardId": "left-board", "mirrorBoardId": "right-board",
                "correspondences": correspondences, "exceptions": exceptions
            },
            "constructionSource": "left"
        })
    }

    fn board(id: &str, parts: &[&str], outlines: &[&str], nets: &[&str]) -> Board {
        serde_json::from_value(json!({
            "id": id, "name": id, "outlineIds": outlines, "partIds": parts,
            "netIds": nets, "thickness": 1.6
        }))
        .unwrap()
    }

    fn part(id: &str) -> Part {
        serde_json::from_value(json!({
            "id": id, "definitionId": "switch", "reference": id,
            "pose": { "at": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 }, "side": "front"
        }))
        .unwrap()
    }

    fn document(pairs: Vec<MechanicalMirrorPair>) -> ProjectDoc {
        let mut document = ProjectDoc::empty("doc", "Doc");
        document.boards = vec![
            board(
                "left-board",
                &["l-key", "l-extra"],
                &["l-outline"],
                &["l-net"],
            ),
            board(
                "right-board",
                &["r-key", "r-extra"],
                &["r-outline"],
                &["r-net"],
            ),
        ];
        document.parts = ["l-key", "l-extra", "r-key", "r-extra"]
            .iter()
            .map(|id| part(id))
            .collect();
        document.hardware = Some(HardwareConfiguration {
            pairs,
            ..HardwareConfiguration::default()
        });
        document
    }

    fn valid_correspondence() -> Value {
        json!([{ "kind": "part", "sourceId": "l-key", "mirrorId": "r-key" }])
    }

    fn refusal(pair_extra: Value) -> String {
        validate_document(&document(vec![pair(pair_extra)])).unwrap_err()
    }

    #[test]
    fn accepts_a_pair_without_halves_and_a_consistent_record() {
        validate_document(&document(vec![pair(json!({}))])).unwrap();
        validate_document(&document(vec![pair(halves(
            valid_correspondence(),
            json!([{ "kind": "part", "mirrorId": "r-extra", "state": "omitted" }]),
        ))]))
        .unwrap();
        validate_document(&ProjectDoc::empty("doc", "Doc")).unwrap();
    }

    #[test]
    fn refuses_a_construction_source_that_is_not_a_member() {
        let message = refusal(json!({ "constructionSource": "elsewhere" }));
        assert!(message.contains("construction source"), "{message}");
        assert!(message.contains("\"pair\""), "{message}");
        assert!(message.contains("\"elsewhere\""), "{message}");
    }

    #[test]
    fn refuses_a_source_board_that_is_not_in_the_document() {
        let mut value = halves(valid_correspondence(), json!([]));
        value["halves"]["sourceBoardId"] = json!("missing-board");
        let message = refusal(value);
        assert!(message.contains("halves.sourceBoardId"), "{message}");
        assert!(message.contains("\"missing-board\""), "{message}");
    }

    #[test]
    fn refuses_a_correspondence_whose_source_is_on_the_other_board() {
        let message = refusal(halves(
            json!([{ "kind": "part", "sourceId": "r-key", "mirrorId": "r-extra" }]),
            json!([]),
        ));
        assert!(message.contains("correspondences[0]"), "{message}");
        assert!(message.contains("\"r-key\""), "{message}");
        assert!(message.contains("\"left-board\""), "{message}");
    }

    #[test]
    fn refuses_a_correspondence_whose_mirror_is_not_on_the_mirror_board() {
        let message = refusal(halves(
            json!([{ "kind": "part", "sourceId": "l-key", "mirrorId": "ghost" }]),
            json!([]),
        ));
        assert!(message.contains("correspondences[0]"), "{message}");
        assert!(message.contains("\"ghost\""), "{message}");
        assert!(message.contains("\"right-board\""), "{message}");
    }

    #[test]
    fn refuses_a_repeated_correspondence_source_or_mirror() {
        let repeated_source = refusal(halves(
            json!([
                { "kind": "part", "sourceId": "l-key", "mirrorId": "r-key" },
                { "kind": "part", "sourceId": "l-key", "mirrorId": "r-extra" }
            ]),
            json!([]),
        ));
        assert!(
            repeated_source.contains("correspondences[1]"),
            "{repeated_source}"
        );
        assert!(repeated_source.contains("repeats"), "{repeated_source}");

        let repeated_mirror = refusal(halves(
            json!([
                { "kind": "part", "sourceId": "l-key", "mirrorId": "r-key" },
                { "kind": "part", "sourceId": "l-extra", "mirrorId": "r-key" }
            ]),
            json!([]),
        ));
        assert!(
            repeated_mirror.contains("correspondences[1]"),
            "{repeated_mirror}"
        );
        assert!(repeated_mirror.contains("repeats"), "{repeated_mirror}");
    }

    #[test]
    fn refuses_an_exception_that_names_a_source_only_item() {
        let message = refusal(halves(
            valid_correspondence(),
            json!([{ "kind": "part", "mirrorId": "l-extra", "state": "local" }]),
        ));
        assert!(message.contains("exceptions[0]"), "{message}");
        assert!(message.contains("\"right-board\""), "{message}");
    }

    /// The document with `r-extra` removed from the mirror half, as an Omit leaves it.
    fn omitted_mirror_document(pair_extra: Value) -> ProjectDoc {
        let mut document = document(vec![pair(pair_extra)]);
        document.parts.retain(|part| part.id != "r-extra");
        document.boards[1].part_ids.retain(|id| id != "r-extra");
        document
    }

    fn two_associations() -> Value {
        json!([
            { "kind": "part", "sourceId": "l-key", "mirrorId": "r-key" },
            { "kind": "part", "sourceId": "l-extra", "mirrorId": "r-extra" }
        ])
    }

    #[test]
    fn accepts_an_associated_overlay_that_keeps_its_correspondence() {
        // Local keeps its item; Omitted keeps only the correspondence while the item is absent.
        let document = omitted_mirror_document(halves(
            two_associations(),
            json!([
                { "kind": "part", "mirrorId": "r-key", "state": "local" },
                { "kind": "part", "mirrorId": "r-extra", "state": "omitted" }
            ]),
        ));
        validate_document(&document).unwrap();
        // The same record round-trips and still validates.
        let decoded: ProjectDoc =
            serde_json::from_slice(&serde_json::to_vec(&document).unwrap()).unwrap();
        validate_document(&decoded).unwrap();
    }

    #[test]
    fn refuses_an_absent_mirror_item_without_an_omitted_overlay() {
        let document = omitted_mirror_document(halves(two_associations(), json!([])));
        let message = validate_document(&document).unwrap_err();
        assert!(message.contains("correspondences[1]"), "{message}");
        assert!(message.contains("\"r-extra\""), "{message}");
    }

    #[test]
    fn refuses_an_omitted_overlay_whose_mirror_item_is_still_present() {
        let message = refusal(halves(
            valid_correspondence(),
            json!([{ "kind": "part", "mirrorId": "r-key", "state": "omitted" }]),
        ));
        assert!(message.contains("exceptions[0]"), "{message}");
        assert!(message.contains("still materialized"), "{message}");
    }

    #[test]
    fn refuses_a_local_overlay_whose_mirror_item_is_absent() {
        let document = omitted_mirror_document(halves(
            two_associations(),
            json!([{ "kind": "part", "mirrorId": "r-extra", "state": "local" }]),
        ));
        let message = validate_document(&document).unwrap_err();
        assert!(message.contains("correspondences[1]"), "{message}");
        assert!(message.contains("\"right-board\""), "{message}");
    }

    #[test]
    fn refuses_companion_settings_on_an_overlay() {
        let message = refusal(halves(
            valid_correspondence(),
            json!([{ "kind": "part", "mirrorId": "r-key", "state": "local",
                "companionMode": "duplicate-position" }]),
        ));
        assert!(message.contains("exceptions[0]"), "{message}");
        assert!(message.contains("companion settings"), "{message}");
    }

    #[test]
    fn refuses_a_repeated_associated_overlay() {
        let message = refusal(halves(
            valid_correspondence(),
            json!([
                { "kind": "part", "mirrorId": "r-key", "state": "local" },
                { "kind": "part", "mirrorId": "r-key", "state": "local" }
            ]),
        ));
        assert!(message.contains("exceptions[1]"), "{message}");
        assert!(message.contains("repeats"), "{message}");
    }

    fn removed_copper(trace_id: &str) -> Value {
        json!({
            "traces": [{ "id": trace_id, "start": { "x": 0.0, "y": 0.0 },
                "end": { "x": 1.0, "y": 0.0 }, "width": 0.25, "layer": "front", "netId": "r-net" }],
            "vias": [],
            "pads": [{ "partId": "r-extra", "padId": "1", "at": { "x": 5.0, "y": 6.0 } }]
        })
    }

    #[test]
    fn accepts_removed_copper_only_on_an_associated_omitted_overlay() {
        let document = omitted_mirror_document(halves(
            two_associations(),
            json!([{ "kind": "part", "mirrorId": "r-extra", "state": "omitted",
                "removedCopper": removed_copper("saved-trace") }]),
        ));
        validate_document(&document).unwrap();
        let decoded: ProjectDoc =
            serde_json::from_slice(&serde_json::to_vec(&document).unwrap()).unwrap();
        validate_document(&decoded).unwrap();
        let exception = &decoded.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap()
            .exceptions[0];
        assert_eq!(exception.removed_copper.as_ref().unwrap().traces.len(), 1);
    }

    #[test]
    fn refuses_removed_copper_on_a_local_or_unassociated_exception() {
        let local = refusal(halves(
            valid_correspondence(),
            json!([{ "kind": "part", "mirrorId": "r-key", "state": "local",
                "removedCopper": removed_copper("saved-trace") }]),
        ));
        assert!(local.contains("removed copper"), "{local}");
        let legacy = refusal(halves(
            json!([]),
            json!([{ "kind": "part", "mirrorId": "r-extra", "state": "local",
                "removedCopper": removed_copper("saved-trace") }]),
        ));
        assert!(legacy.contains("removed copper"), "{legacy}");
    }

    #[test]
    fn refuses_removed_copper_that_collides_with_live_copper() {
        let mut document = omitted_mirror_document(halves(
            two_associations(),
            json!([{ "kind": "part", "mirrorId": "r-extra", "state": "omitted",
                "removedCopper": removed_copper("live-trace") }]),
        ));
        document.boards[1].traces.push(
            serde_json::from_value(json!({ "id": "live-trace", "start": { "x": 0.0, "y": 0.0 },
                "end": { "x": 2.0, "y": 0.0 }, "width": 0.25, "layer": "front" }))
            .unwrap(),
        );
        let message = validate_document(&document).unwrap_err();
        assert!(message.contains("live-trace"), "{message}");
    }

    #[test]
    fn a_private_net_is_absent_by_rule_only_when_every_non_controller_pin_is_omitted() {
        let net = |pins: Value| -> crate::model::Net {
            serde_json::from_value(json!({ "id": "l-net", "name": "SDA", "pins": pins })).unwrap()
        };
        let correspondences = json!([
            { "kind": "part", "sourceId": "l-key", "mirrorId": "r-key" },
            { "kind": "part", "sourceId": "l-extra", "mirrorId": "r-extra" },
            { "kind": "net", "sourceId": "l-net", "mirrorId": "r-net" }
        ]);
        let overlays = json!([{ "kind": "part", "mirrorId": "r-extra", "state": "omitted" }]);
        let drop_mirror_net = |document: &mut ProjectDoc| {
            document.boards[1].net_ids.retain(|id| id != "r-net");
        };
        // Every pin is on the omitted part: the mirror net is absent with nothing stored.
        let mut private =
            omitted_mirror_document(halves(correspondences.clone(), overlays.clone()));
        private.nets = vec![net(json!([{ "partId": "l-extra", "padId": "1" }]))];
        drop_mirror_net(&mut private);
        validate_document(&private).unwrap();
        // A pin on a part that is still mirrored makes the net shared, so it must be present.
        let mut shared = omitted_mirror_document(halves(correspondences, overlays));
        shared.nets = vec![net(json!([
            { "partId": "l-extra", "padId": "1" },
            { "partId": "l-key", "padId": "1" }
        ]))];
        drop_mirror_net(&mut shared);
        let message = validate_document(&shared).unwrap_err();
        assert!(message.contains("\"r-net\""), "{message}");
    }

    #[test]
    fn refuses_a_repeated_exception() {
        let message = refusal(halves(
            json!([]),
            json!([
                { "kind": "part", "mirrorId": "r-extra", "state": "local" },
                { "kind": "part", "mirrorId": "r-extra", "state": "omitted" }
            ]),
        ));
        assert!(message.contains("exceptions[1]"), "{message}");
        assert!(message.contains("repeats"), "{message}");
    }

    #[test]
    fn refuses_a_board_that_is_a_half_of_two_pairs() {
        let first = pair(halves(valid_correspondence(), json!([])));
        let second = pair(json!({
            "id": "second", "members": ["other-left", "other-right"],
            "constructionSource": "other-left",
            "halves": { "sourceBoardId": "left-board", "mirrorBoardId": "right-board",
                "correspondences": [], "exceptions": [] }
        }));
        let message = validate_document(&document(vec![first, second])).unwrap_err();
        assert!(message.contains("\"left-board\""), "{message}");
        assert!(
            message.contains("\"pair\"") && message.contains("\"second\""),
            "{message}"
        );
    }

    #[test]
    fn outline_and_net_correspondences_must_be_on_their_board() {
        validate_document(&document(vec![pair(halves(
            json!([
                { "kind": "outline", "sourceId": "l-outline", "mirrorId": "r-outline" },
                { "kind": "net", "sourceId": "l-net", "mirrorId": "r-net" }
            ]),
            json!([]),
        ))]))
        .unwrap();
        let message = refusal(halves(
            json!([{ "kind": "net", "sourceId": "l-net", "mirrorId": "l-net" }]),
            json!([]),
        ));
        assert!(message.contains("correspondences[0]"), "{message}");
    }

    fn module(id: &str, host_board_id: &str) -> crate::modules::MountedModule {
        serde_json::from_value(json!({
            "id": id, "definitionId": "reference", "hostBoardId": host_board_id,
            "hostFace": "front", "facingFace": "back", "at": { "x": 0.0, "y": 0.0 },
            "rotation": 0.0, "gap": 0.0, "attachment": "board"
        }))
        .unwrap()
    }

    #[test]
    fn module_correspondences_must_be_hosted_on_their_board() {
        let mut document = document(vec![pair(halves(
            json!([{ "kind": "module", "sourceId": "l-module", "mirrorId": "r-module" }]),
            json!([{ "kind": "module", "mirrorId": "r-extra-module", "state": "local" }]),
        ))]);
        document.modules = vec![
            module("l-module", "left-board"),
            module("r-module", "right-board"),
            module("r-extra-module", "right-board"),
        ];
        validate_document(&document).unwrap();

        // A module hosted on the other board is not an item of this half.
        document.modules[1] = module("r-module", "left-board");
        let message = validate_document(&document).unwrap_err();
        assert!(message.contains("correspondences[0]"), "{message}");
        assert!(message.contains("module \"r-module\""), "{message}");
        assert!(message.contains("\"right-board\""), "{message}");
    }

    fn matrix(id: &str, board_id: Option<&str>) -> crate::model::Matrix {
        serde_json::from_value(json!({
            "id": id, "rows": 1, "columns": 1, "pitch": { "x": 19.0, "y": 19.0 },
            "origin": { "x": 0.0, "y": 0.0 }, "definitionId": "switch", "partIds": [],
            "boardId": board_id
        }))
        .unwrap()
    }

    fn layout(id: &str, board_id: &str) -> crate::model::Layout {
        serde_json::from_value(json!({
            "id": id, "name": id, "boardId": board_id, "matrixIds": ["l-matrix"], "partIds": []
        }))
        .unwrap()
    }

    #[test]
    fn matrix_and_layout_correspondences_must_belong_to_their_board() {
        let correspondences = json!([
            { "kind": "matrix", "sourceId": "l-matrix", "mirrorId": "r-matrix" },
            { "kind": "layout", "sourceId": "l-layout", "mirrorId": "r-layout" }
        ]);
        let mut document = document(vec![pair(halves(correspondences, json!([])))]);
        document.matrices = vec![
            matrix("l-matrix", Some("left-board")),
            matrix("r-matrix", Some("right-board")),
        ];
        document.layouts = vec![
            layout("l-layout", "left-board"),
            layout("r-layout", "right-board"),
        ];
        validate_document(&document).unwrap();

        // A matrix that names no board is not placed on one, so it is not refused for it.
        document.matrices[1] = matrix("r-matrix", None);
        validate_document(&document).unwrap();

        document.matrices[1] = matrix("r-matrix", Some("left-board"));
        let message = validate_document(&document).unwrap_err();
        assert!(message.contains("correspondences[0]"), "{message}");
        assert!(message.contains("matrix \"r-matrix\""), "{message}");

        document.matrices[1] = matrix("r-matrix", Some("right-board"));
        document.layouts[1] = layout("r-layout", "left-board");
        let message = validate_document(&document).unwrap_err();
        assert!(message.contains("correspondences[1]"), "{message}");
        assert!(message.contains("layout \"r-layout\""), "{message}");
    }

    fn instance(id: &str, board_id: &str) -> crate::model::PhysicalBoardInstance {
        serde_json::from_value(json!({
            "id": id, "name": id, "boardId": board_id, "half": id, "role": "controller",
            "flipped": false
        }))
        .unwrap()
    }

    #[test]
    fn halves_need_a_source_whose_board_is_the_source_board() {
        let mut document = document(vec![pair(halves(valid_correspondence(), json!([])))]);
        document.hardware.as_mut().unwrap().instances = vec![
            instance("left", "left-board"),
            instance("right", "right-board"),
        ];
        validate_document(&document).unwrap();

        // The members' boards are the record's boards in the other order.
        document.hardware.as_mut().unwrap().instances = vec![
            instance("left", "right-board"),
            instance("right", "left-board"),
        ];
        let message = validate_document(&document).unwrap_err();
        assert!(message.contains("halves.sourceBoardId"), "{message}");
        assert!(message.contains("\"left\""), "{message}");

        // Reusing one board for both halves is a record of one board, not a mismatch.
        let mut reuse = halves(json!([]), json!([]));
        reuse["halves"]["mirrorBoardId"] = json!("left-board");
        let mut document = document_with(reuse);
        document.hardware.as_mut().unwrap().instances = vec![
            instance("left", "left-board"),
            instance("right", "left-board"),
        ];
        validate_document(&document).unwrap();

        // A record cannot say which member is its source without a construction source.
        let mut missing = halves(json!([]), json!([]));
        missing
            .as_object_mut()
            .unwrap()
            .remove("constructionSource");
        let message = refusal(missing);
        assert!(message.contains("constructionSource"), "{message}");
    }

    fn document_with(extra: Value) -> ProjectDoc {
        document(vec![pair(extra)])
    }
}

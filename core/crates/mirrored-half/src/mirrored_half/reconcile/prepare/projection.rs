//! Pair projection and membership-transition construction for prepared drafts.

pub(super) use super::super::transition_matrix_by_id as matrix_by_id;
use super::*;
use crate::model::{MechanicalMirrorPair, MirroredCorrespondence, MirroredHalves};
use std::cell::OnceCell;

pub(super) struct PairProjection<'a> {
    halves: &'a MirroredHalves,
    mapping: Pose2,
    // A present key with no row represents ambiguous source provenance.
    part_links: OnceCell<BTreeMap<&'a str, Option<&'a MirroredCorrespondence>>>,
}

impl<'a> PairProjection<'a> {
    pub(super) fn new(halves: &'a MirroredHalves, mapping: Pose2) -> Self {
        Self {
            halves,
            mapping,
            part_links: OnceCell::new(),
        }
    }

    pub(super) fn matrix_links(&self) -> impl Iterator<Item = &'a MirroredCorrespondence> {
        self.halves
            .correspondences
            .iter()
            .filter(|link| link.kind == MirroredItemKind::Matrix)
    }

    fn part_link(&self, source_id: &str) -> Result<&'a MirroredCorrespondence, String> {
        // Build once per immutable pair view, only if a matrix needs a cell's link.
        let links = self.part_links.get_or_init(|| {
            let mut links = BTreeMap::new();
            for link in &self.halves.correspondences {
                if link.kind == MirroredItemKind::Part {
                    links
                        .entry(link.source_id.as_str())
                        .and_modify(|entry| *entry = None)
                        .or_insert(Some(link));
                }
            }
            links
        });
        match links.get(source_id) {
            Some(Some(link)) => Ok(link),
            Some(None) => Err(format!(
                "Matrix part {source_id:?} has ambiguous correspondence"
            )),
            None => Err(format!("Matrix part {source_id:?} has no correspondence")),
        }
    }
}

/// Project a matrixless pair before the second constraint solve, at the same point as lifecycle
/// projection. The shared projector still repairs every linked row, including unedited parts
/// and settings. The outer report computes changed IDs from the finished draft.
pub(super) fn project_matrixless_placement(
    accepted: &ProjectDoc,
    draft: &mut ProjectDoc,
    pair: &MechanicalMirrorPair,
    operation: &EditOperation,
) -> Result<bool, String> {
    let Some(halves) = &pair.halves else {
        return Ok(false);
    };
    // Keep hosted inheritance, matrix membership, same-board and legacy layout links with
    // lifecycle. The lifecycle also validates and retires stored constraint records.
    if !matches!(
        operation,
        EditOperation::MoveParts { .. } | EditOperation::SetPartRotation { .. }
    ) || halves.source_board_id == halves.mirror_board_id
        || halves.correspondences.iter().any(|link| match link.kind {
            MirroredItemKind::Matrix | MirroredItemKind::EmbeddedCircuit => true,
            MirroredItemKind::Net => !draft.nets.iter().any(|net| net.id == link.source_id),
            _ => false,
        })
        || halves
            .exceptions
            .iter()
            .any(|exception| halves.overlaid_correspondence(exception).is_none())
        || !accepted.constraints.is_empty()
        || !draft.constraints.is_empty()
        || !stable_placement(accepted, draft, operation)
        || draft.matrices.iter().any(|matrix| {
            matrix.board_id.as_ref().is_some_and(|board| {
                *board == halves.source_board_id || *board == halves.mirror_board_id
            })
        })
    {
        return Ok(false);
    }
    // Authenticate the selected pair and the same materialized ownership the lifecycle needs.
    for document in [accepted, &*draft] {
        let mut matching = document
            .hardware
            .iter()
            .flat_map(|hardware| &hardware.pairs)
            .filter(|row| row.id == pair.id);
        if matching.next() != Some(pair) || matching.next().is_some() {
            return Ok(false);
        }
    }
    if !accepted
        .parts
        .iter()
        .map(|part| &part.id)
        .eq(draft.parts.iter().map(|part| &part.id))
    {
        return Ok(false);
    }
    let mut registrations = BTreeSet::new();
    if !accepted
        .boards
        .iter()
        .flat_map(|board| &board.part_ids)
        .all(|id| registrations.insert(id))
        || !accepted.parts.iter().all(|part| {
            registrations.remove(&part.id)
                && part.pose.at.x.is_finite()
                && part.pose.at.y.is_finite()
                && part.pose.rotation.is_finite()
        })
    {
        return Ok(false);
    }

    // An already absent Omitted target can still have stale references. Apply the same cleanup
    // before projecting the remaining rows, while keeping its correspondence and overlay.
    let omitted: BTreeSet<_> = halves
        .exceptions
        .iter()
        .filter(|exception| {
            exception.kind == MirroredItemKind::Part
                && exception.state == MirroredExceptionState::Omitted
        })
        .map(|exception| exception.mirror_id.clone())
        .collect();
    if !omitted.is_empty() {
        let matrix_members: BTreeSet<_> = accepted
            .matrices
            .iter()
            .flat_map(|matrix| &matrix.part_ids)
            .collect();
        if halves.correspondences.iter().any(|link| {
            link.kind == MirroredItemKind::Part
                && omitted.contains(&link.mirror_id)
                && (matrix_members.contains(&link.source_id)
                    || matrix_members.contains(&link.mirror_id))
        }) {
            return Ok(false);
        }
        crate::matrix::deletion::remove_parts_by_id(draft, &omitted);
    }

    let projection = super::super::projection_for_pair(halves, pair.mapping);
    reconcile_canonical(draft, &pair.id, &projection)?;
    Ok(true)
}

pub(super) fn reconcile_pairs(
    accepted: &ProjectDoc,
    document: &mut ProjectDoc,
    canonical: &Canonical,
) -> Result<(), String> {
    let pair_views = native_pair_views(document);
    for pair in pair_views {
        super::super::validate_local_circuit_targets(document, &pair.halves)?;
        let pair_projection = PairProjection::new(&pair.halves, pair.mapping);
        let mut matrices = pair_projection.matrix_links().peekable();
        if matrices.peek().is_none() {
            if matches!(
                canonical.operation,
                EditOperation::AddPart { .. } | EditOperation::AddMirrorOnlyPart { .. }
            ) {
                let mut projection = super::super::projection_for_pair(&pair.halves, pair.mapping);
                projection.independent_additions =
                    transitions_for(document, canonical, &pair.halves.source_board_id, false).0;
                // Caller-owned draft: no per-pair report. The outer transaction guarantees
                // rollback and owns the single final scoped report, so refusals still leave
                // the accepted document intact while ledger updates stay atomic on success.
                reconcile_canonical(document, &pair.pair_id, &projection)?;
            }
            continue;
        }
        for (index, matrix_link) in matrices.enumerate() {
            if super::super::prepares_matrix_membership(
                &canonical.operation,
                accepted,
                document,
                &matrix_link.source_id,
            ) {
                // Lifecycle preparation has already projected this matrix's changed membership.
                // The fixed-membership reconciler owns only matrices whose membership stayed put.
                continue;
            }
            let projection = projection_for(
                document,
                Some(canonical),
                &pair_projection,
                matrix_link,
                index == 0,
            )?;
            reconcile_canonical(document, &pair.pair_id, &projection)?;
        }
    }
    Ok(())
}

/// The forward projector materializes an independent Add using the selected identities. Commit
/// those identities to the native pair record only after projection succeeds, so the prepared
/// draft is linked for subsequent edits while refusals still leave the accepted document intact.
pub(super) fn record_independent_additions(
    document: &mut ProjectDoc,
    additions: &[inverse::PartAddition],
) -> Result<Vec<String>, String> {
    let mut changed = Vec::new();
    for addition in additions {
        let source_board = super::super::super::board_for_part(document, &addition.source_id)
            .ok_or_else(|| {
                format!(
                    "Independent source Part {:?} has no board owner",
                    addition.source_id
                )
            })?
            .id
            .clone();
        let mirror_board = super::super::super::board_for_part(document, &addition.mirror_id)
            .ok_or_else(|| {
                format!(
                    "Independent mirror Part {:?} has no board owner",
                    addition.mirror_id
                )
            })?
            .id
            .clone();
        let hardware = document
            .hardware
            .as_mut()
            .ok_or("Mirrored hardware is unavailable for independent Add")?;
        let mut matching_pairs = hardware.pairs.iter_mut().filter(|pair| {
            pair.halves.as_ref().is_some_and(|halves| {
                halves.source_board_id == source_board && halves.mirror_board_id == mirror_board
            })
        });
        let pair = matching_pairs.next().ok_or_else(|| {
            format!(
                "No directed mirrored pair owns independent Add {:?} → {:?}",
                addition.source_id, addition.mirror_id
            )
        })?;
        if matching_pairs.next().is_some() {
            return Err(format!(
                "Independent Add {:?} → {:?} has ambiguous pair ownership",
                addition.source_id, addition.mirror_id
            ));
        }
        let halves = pair
            .halves
            .as_mut()
            .ok_or("The independent Add pair lost its halves record")?;
        if halves.correspondences.iter().any(|record| {
            record.kind == MirroredItemKind::Part
                && (record.source_id == addition.source_id
                    || record.mirror_id == addition.mirror_id)
        }) {
            return Err(format!(
                "Independent Add {:?} → {:?} conflicts with existing pair provenance",
                addition.source_id, addition.mirror_id
            ));
        }
        halves.correspondences.push(MirroredCorrespondence {
            kind: MirroredItemKind::Part,
            source_id: addition.source_id.clone(),
            mirror_id: addition.mirror_id.clone(),
            companion_mode: None,
            companion_tweak: None,
        });
        changed.push(pair.id.clone());
    }
    Ok(changed)
}

pub(super) fn projection_for(
    document: &ProjectDoc,
    canonical: Option<&Canonical>,
    pair: &PairProjection<'_>,
    matrix_link: &MirroredCorrespondence,
    include_transitions: bool,
) -> Result<Projection, String> {
    let halves = pair.halves;
    let source_matrix_id = matrix_link.source_id.as_str();
    let mirror_matrix_id = matrix_link.mirror_id.as_str();
    let source_matrix = matrix_by_id(document, source_matrix_id)?;
    let mirror_matrix = match matrix_by_id(document, mirror_matrix_id) {
        Ok(matrix) => Some(matrix),
        Err(_) if is_omitted_matrix_link(halves, source_matrix_id, mirror_matrix_id) => None,
        Err(error) => return Err(error),
    };
    let source_board_id = source_matrix
        .board_id
        .clone()
        .ok_or("The source matrix has no board owner")?;
    let mirror_board_id = mirror_matrix
        .map_or(Some(&halves.mirror_board_id), |matrix| {
            matrix.board_id.as_ref()
        })
        .cloned()
        .ok_or("The mirror matrix has no board owner")?;
    // Members introduced under a detached matrix have no mirror target yet;
    // an explicit Rejoin adopts them. Any other missing correspondence stays
    // a refusal.
    let matrix_detached = halves.overlay_of(matrix_link).is_some_and(|exception| {
        matches!(
            exception.state,
            MirroredExceptionState::Local | MirroredExceptionState::Omitted
        )
    });
    for cell in source_matrix
        .cells
        .iter()
        .filter(|cell| cell.enabled && !cell.deleted)
    {
        let source_id = crate::matrix::member_id(source_matrix_id, cell.row, cell.column);
        let association = match pair.part_link(&source_id) {
            Ok(association) => association,
            Err(_) if matrix_detached => continue,
            Err(error) => return Err(error),
        };
        if association.mirror_id
            != crate::matrix::member_id(mirror_matrix_id, cell.row, cell.column)
        {
            return Err(format!(
                "Matrix part association for {source_id:?} has stale identity"
            ));
        }
    }
    let (independent_additions, hosted_transitions) = match canonical {
        Some(canonical) => {
            transitions_for(document, canonical, &source_board_id, include_transitions)
        }
        None if !include_transitions => (Vec::new(), Vec::new()),
        None => return Err("Edit projection requires canonical intent".into()),
    };
    Ok(Projection {
        matrix: Some(super::super::MatrixLink {
            source_matrix_id: matrix_link.source_id.clone(),
            mirror_matrix_id: matrix_link.mirror_id.clone(),
        }),
        preserve_matrix_geometry: false,
        source_board_id,
        mirror_board_id,
        independent_additions,
        hosted_transitions,
        additions_applied: !include_transitions,
        ..super::super::projection_for_pair(halves, pair.mapping)
    })
}

fn transitions_for(
    document: &ProjectDoc,
    canonical: &Canonical,
    source_board_id: &str,
    include_hosted: bool,
) -> (Vec<PartAddition>, Vec<HostedTransition>) {
    let parts = canonical
        .additions
        .iter()
        .filter(|addition| {
            super::super::super::board_for_part(document, &addition.source_id)
                .is_some_and(|board| board.id == source_board_id)
        })
        .cloned()
        .collect();
    let mut hosted = Vec::new();
    if !include_hosted {
        return (parts, hosted);
    }

    for addition in &canonical.hosted.modules {
        let owner = document
            .modules
            .iter()
            .find(|module| module.id == addition.source_id)
            .map(|module| module.host_board_id.as_str());
        if owner == Some(source_board_id) {
            hosted.push(HostedTransition::AddModule {
                source_id: addition.source_id.clone(),
                mirror_id: addition.mirror_id.clone(),
            });
        }
    }
    for addition in &canonical.hosted.circuits {
        let owner = document
            .embedded_circuits
            .iter()
            .find(|circuit| circuit.id == addition.source_id)
            .map(|circuit| circuit.host_board_id.as_str());
        if owner == Some(source_board_id) {
            hosted.push(HostedTransition::AddCircuit {
                source_id: addition.source_id.clone(),
                mirror_id: addition.mirror_id.clone(),
                member_ids: addition.member_ids.clone(),
                member_references: addition.member_references.clone(),
                definition_ids: addition.definition_ids.clone(),
                net_ids: addition.net_ids.clone(),
            });
        }
    }
    (parts, hosted)
}

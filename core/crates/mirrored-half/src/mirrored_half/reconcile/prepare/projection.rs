//! Pair projection and membership-transition construction for prepared drafts.

use super::*;
use crate::model::{MirroredCorrespondence, MirroredHalves};
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
                let mut projection =
                    super::super::projection_for_matrixless_pair(&pair.halves, pair.mapping);
                projection.independent_part_transitions = transitions_for(
                    accepted,
                    document,
                    canonical,
                    &pair.halves,
                    &pair.halves.source_board_id,
                    &pair.halves.mirror_board_id,
                    false,
                )?
                .0;
                // Caller-owned draft: no per-pair report. The outer transaction guarantees
                // rollback and owns the single final scoped report, so refusals still leave
                // the accepted document intact while ledger updates stay atomic on success.
                reconcile_with_report(document, &pair.pair_id, &projection, None)?;
            }
            continue;
        }
        for (index, matrix_link) in matrices.enumerate() {
            if super::super::matrix_membership_changed(accepted, document, &matrix_link.source_id) {
                // Lifecycle preparation has already projected this matrix's changed membership.
                // The fixed-membership reconciler owns only matrices whose membership stayed put.
                continue;
            }
            let projection = projection_for(
                accepted,
                document,
                Some(canonical),
                &pair_projection,
                matrix_link,
                index == 0,
            )?;
            reconcile_with_report(document, &pair.pair_id, &projection, None)?;
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
    accepted: &ProjectDoc,
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
    let omitted_matrix = is_omitted_matrix_link(halves, source_matrix_id, mirror_matrix_id);
    let absent_omitted_matrix;
    let mirror_matrix = match matrix_by_id(document, mirror_matrix_id) {
        Ok(matrix) => matrix,
        Err(_) if omitted_matrix => {
            absent_omitted_matrix = omitted_mirror_matrix(
                source_matrix,
                mirror_matrix_id,
                &halves.mirror_board_id,
                halves,
            );
            &absent_omitted_matrix
        }
        Err(error) => return Err(error),
    };
    let source_board_id = source_matrix
        .board_id
        .clone()
        .ok_or("The source matrix has no board owner")?;
    let mirror_board_id = mirror_matrix
        .board_id
        .clone()
        .ok_or("The mirror matrix has no board owner")?;
    let mut parts = Vec::new();
    let mut locally_protected_mirror_parts = BTreeSet::new();
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
        let source_id = format!("matrix/{source_matrix_id}/r{}c{}", cell.row, cell.column);
        let association = match pair.part_link(&source_id) {
            Ok(association) => association,
            Err(_) if matrix_detached => continue,
            Err(error) => return Err(error),
        };
        if association.mirror_id
            != format!("matrix/{mirror_matrix_id}/r{}c{}", cell.row, cell.column)
        {
            return Err(format!(
                "Matrix part association for {source_id:?} has stale identity"
            ));
        }
        if halves
            .overlay_of(association)
            .is_some_and(|exception| exception.state == MirroredExceptionState::Local)
        {
            locally_protected_mirror_parts.insert(association.mirror_id.clone());
        } else {
            parts.push(PartProjection {
                source_id,
                mirror_id: association.mirror_id.clone(),
            });
        }
    }
    let mut final_owners = BTreeMap::new();
    for (matrix, owner) in [
        (source_matrix, source_board_id.clone()),
        (mirror_matrix, mirror_board_id.clone()),
    ] {
        for id in &matrix.part_ids {
            final_owners.insert(id.clone(), owner.clone());
        }
    }
    let (independent_part_transitions, hosted_transitions, hosted_removal_baselines) =
        match canonical {
            Some(canonical) => transitions_for(
                accepted,
                document,
                canonical,
                halves,
                &source_board_id,
                &mirror_board_id,
                include_transitions,
            )?,
            None if !include_transitions => (Vec::new(), Vec::new(), BTreeMap::new()),
            None => return Err("Edit projection requires canonical intent".into()),
        };
    let applied_independent_additions = if include_transitions {
        BTreeSet::new()
    } else {
        independent_part_transitions
            .iter()
            .filter_map(|transition| match transition {
                IndependentPartTransition::Add {
                    source_id,
                    mirror_id,
                    mirror_definition_id,
                    mirror_reference,
                } => Some((
                    source_id.clone(),
                    mirror_id.clone(),
                    mirror_definition_id.clone(),
                    mirror_reference.clone(),
                )),
                IndependentPartTransition::Remove { .. } => None,
            })
            .collect()
    };
    Ok(Projection {
        matrix: Some(super::super::MatrixLink {
            source_matrix_id: matrix_link.source_id.clone(),
            mirror_matrix_id: matrix_link.mirror_id.clone(),
        }),
        preserve_matrix_geometry: false,
        source_board_id,
        mirror_board_id,
        mapping: pair.mapping,
        parts,
        independent_part_transitions,
        hosted_transitions,
        applied_independent_additions: std::cell::RefCell::new(applied_independent_additions),
        applied_hosted_operations: std::cell::RefCell::new(BTreeSet::new()),
        applied_hosted_circuit_additions: std::cell::RefCell::new(BTreeMap::new()),
        hosted_removal_baselines: std::cell::RefCell::new(hosted_removal_baselines),
        hosted_omission_baselines: std::cell::RefCell::new(BTreeMap::new()),
        outline_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
        locally_protected_mirror_parts,
        final_owners,
    })
}

pub(super) fn matrix_by_id<'a>(document: &'a ProjectDoc, id: &str) -> Result<&'a Matrix, String> {
    let mut matrices = document.matrices.iter().filter(|matrix| matrix.id == id);
    let matrix = matrices
        .next()
        .ok_or_else(|| format!("Mirrored matrix {id:?} is unavailable"))?;
    if matrices.next().is_some() {
        return Err(format!("Mirrored matrix {id:?} is ambiguous"));
    }
    Ok(matrix)
}

pub(super) type RemovalBaselines = BTreeMap<(String, String, String), ProjectDoc>;

pub(super) fn transitions_for(
    accepted: &ProjectDoc,
    document: &ProjectDoc,
    canonical: &Canonical,
    halves: &crate::model::MirroredHalves,
    source_board_id: &str,
    mirror_board_id: &str,
    include_removals_and_hosted: bool,
) -> Result<
    (
        Vec<IndependentPartTransition>,
        Vec<HostedTransition>,
        RemovalBaselines,
    ),
    String,
> {
    let mut parts = Vec::new();
    for addition in &canonical.additions {
        if super::super::super::board_for_part(document, &addition.source_id)
            .is_some_and(|board| board.id == source_board_id)
        {
            parts.push(IndependentPartTransition::Add {
                source_id: addition.source_id.clone(),
                mirror_id: addition.mirror_id.clone(),
                mirror_definition_id: addition.mirror_definition_id.clone(),
                mirror_reference: addition.mirror_reference.clone(),
            });
        }
    }
    if include_removals_and_hosted && let EditOperation::RemoveParts { ids } = &canonical.operation
    {
        for source_id in ids {
            if let Some(association) = halves.correspondences.iter().find(|association| {
                association.kind == MirroredItemKind::Part && association.source_id == *source_id
            }) {
                parts.push(IndependentPartTransition::Remove {
                    source_id: source_id.clone(),
                    mirror_id: association.mirror_id.clone(),
                });
            }
        }
    }

    let mut hosted = Vec::new();
    let mut removal_baselines = BTreeMap::new();
    if !include_removals_and_hosted {
        return Ok((parts, hosted, removal_baselines));
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
                member_poses: addition.member_poses.clone(),
            });
        }
    }
    match &canonical.operation {
        EditOperation::RemoveMountedModule { id } => {
            if let Some(association) = hosted_association(halves, MirroredItemKind::Module, id)
                && module_owner(accepted, id) == Some(source_board_id)
            {
                hosted.push(HostedTransition::RemoveModule {
                    source_id: id.clone(),
                    mirror_id: association.mirror_id.clone(),
                });
                removal_baselines.insert(
                    (
                        "remove-module".into(),
                        id.clone(),
                        association.mirror_id.clone(),
                    ),
                    accepted.clone(),
                );
            }
        }
        EditOperation::RemoveEmbeddedCircuit { id } => {
            if let Some(association) =
                hosted_association(halves, MirroredItemKind::EmbeddedCircuit, id)
                && circuit_owner(accepted, id) == Some(source_board_id)
            {
                hosted.push(HostedTransition::RemoveCircuit {
                    source_id: id.clone(),
                    mirror_id: association.mirror_id.clone(),
                });
                removal_baselines.insert(
                    (
                        "remove-circuit".into(),
                        id.clone(),
                        association.mirror_id.clone(),
                    ),
                    accepted.clone(),
                );
            }
        }
        _ => {}
    }
    let _ = mirror_board_id;
    Ok((parts, hosted, removal_baselines))
}

pub(super) fn hosted_association<'a>(
    halves: &'a crate::model::MirroredHalves,
    kind: MirroredItemKind,
    source_id: &str,
) -> Option<&'a crate::model::MirroredCorrespondence> {
    halves
        .correspondences
        .iter()
        .find(|association| association.kind == kind && association.source_id == source_id)
}

pub(super) fn module_owner<'a>(document: &'a ProjectDoc, id: &str) -> Option<&'a str> {
    document
        .modules
        .iter()
        .find(|module| module.id == id)
        .map(|module| module.host_board_id.as_str())
}

pub(super) fn circuit_owner<'a>(document: &'a ProjectDoc, id: &str) -> Option<&'a str> {
    document
        .embedded_circuits
        .iter()
        .find(|circuit| circuit.id == id)
        .map(|circuit| circuit.host_board_id.as_str())
}

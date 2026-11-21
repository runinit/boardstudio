//! Correspondence target views for forward reconciliation.

use super::*;
use crate::model::MirroredExceptionState;

/// Three views of the correspondences. `part_targets` and `net_targets` are the linked items,
/// the only ones a projection may write over. `reference_*` also name Local items, so a
/// reference to a Local item (a circuit member, a module connector, a lock) still resolves.
/// `omitted_part_sources` name the items a reference must drop.
pub(super) struct CorrespondenceTargets {
    pub(super) part_targets: BTreeMap<String, String>,
    pub(super) reference_part_targets: BTreeMap<String, String>,
    pub(super) omitted_part_sources: BTreeSet<String>,
    pub(super) net_targets: BTreeMap<String, String>,
    pub(super) reference_net_targets: BTreeMap<String, String>,
}

/// Builds the [`CorrespondenceTargets`] views, composing hosted Add reference targets.
pub(super) fn correspondence_targets(
    document: &ProjectDoc,
    halves: &MirroredHalves,
    circuit_links: &[&MirroredCorrespondence],
    projection: &Projection,
) -> Result<CorrespondenceTargets, String> {
    let mut part_targets = BTreeMap::new();
    let mut reference_part_targets = BTreeMap::new();
    let mut omitted_part_sources = BTreeSet::new();
    let mut net_targets = BTreeMap::new();
    let mut reference_net_targets = BTreeMap::new();
    for link in &halves.correspondences {
        let (linked, references) = match link.kind {
            MirroredItemKind::Part => (&mut part_targets, &mut reference_part_targets),
            MirroredItemKind::Net => (&mut net_targets, &mut reference_net_targets),
            _ => continue,
        };
        let state = super::super::effective_circuit_child_state(
            document,
            halves,
            link,
            circuit_links.iter().copied(),
        );
        if state.is_none() {
            linked.insert(link.source_id.clone(), link.mirror_id.clone());
        }
        if state != Some(MirroredExceptionState::Omitted) {
            references.insert(link.source_id.clone(), link.mirror_id.clone());
        } else if link.kind == MirroredItemKind::Part {
            omitted_part_sources.insert(link.source_id.clone());
        }
    }
    let compose_reference_target = |targets: &mut BTreeMap<String, String>,
                                    source_id: &str,
                                    mirror_id: &str,
                                    kind: MirroredItemKind|
     -> Result<(), String> {
        if targets.iter().any(|(existing_source, existing_mirror)| {
            existing_source != source_id && existing_mirror == mirror_id
        }) {
            return Err(format!(
                "The selected hosted {kind:?} reference target {mirror_id:?} is already in use"
            ));
        }
        match targets.get(source_id) {
            Some(existing) if existing == mirror_id => Ok(()),
            Some(_) => Err(format!(
                "The selected hosted {kind:?} reference conflicts with an existing identity"
            )),
            None => {
                targets.insert(source_id.to_owned(), mirror_id.to_owned());
                Ok(())
            }
        }
    };
    for transition in &projection.hosted_transitions {
        if let HostedTransition::AddCircuit {
            member_ids,
            net_ids,
            ..
        } = transition
        {
            for (source_id, mirror_id) in member_ids {
                compose_reference_target(
                    &mut reference_part_targets,
                    source_id,
                    mirror_id,
                    MirroredItemKind::Part,
                )?;
            }
            for (source_id, mirror_id) in net_ids {
                compose_reference_target(
                    &mut reference_net_targets,
                    source_id,
                    mirror_id,
                    MirroredItemKind::Net,
                )?;
            }
        }
    }
    Ok(CorrespondenceTargets {
        part_targets,
        reference_part_targets,
        omitted_part_sources,
        net_targets,
        reference_net_targets,
    })
}

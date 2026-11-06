//! Inverse compatibility for native mirrored Layout records.

use super::{Link, Links, Target};
use crate::model::{Layout, MirroredExceptionState, MirroredItemKind};
use std::collections::BTreeSet;

const OVERLAP: &str = "A layout cannot use native mirrored-pair and legacy layout mirror links together; unlink one before editing it";
const TARGET_OWNED: &str =
    "Native mirror layout name and local membership keep their authored target semantics";

pub(super) enum Outcome {
    Translated(Layout),
    PassThrough,
    DeliberatelyUnchanged(&'static str),
}

/// Maps a native mirror-side SetLayout to its source layout, accounting for the ordering and
/// retained target entries produced by layout reconciliation.
pub(super) fn translate(links: &Links<'_>, requested: &Layout) -> Result<Outcome, String> {
    let native = links
        .pairs
        .iter()
        .flat_map(|pair| {
            pair.halves
                .correspondences
                .iter()
                .filter(move |correspondence| {
                    correspondence.kind == MirroredItemKind::Layout
                        && (correspondence.source_id == requested.id
                            || correspondence.mirror_id == requested.id)
                })
                .map(move |correspondence| (pair, correspondence))
        })
        .collect::<Vec<_>>();
    if native.is_empty() {
        if introduces_native_legacy_overlap(links, requested) {
            return Err(OVERLAP.into());
        }
        return Ok(Outcome::PassThrough);
    }
    if native.len() != 1 {
        return Err(format!(
            "Layout {:?} belongs to more than one native mirrored pair",
            requested.id
        ));
    }
    let (_, correspondence) = native[0];
    let source = unique_layout(links, &correspondence.source_id)?;
    let mirror = optional_layout(links, &correspondence.mirror_id)?;

    // Removing the stored legacy edge is the existing repair operation. Let that explicit
    // SetLayout run as authored before refusing other edits across an overlap.
    let removes_legacy_link = (requested.id == source.id && is_pure_unlink(requested, source))
        || mirror
            .is_some_and(|layout| requested.id == layout.id && is_pure_unlink(requested, layout));
    if removes_legacy_link {
        return Ok(Outcome::PassThrough);
    }

    if overlaps_legacy_link(links, correspondence, source, mirror) {
        return Err(OVERLAP.into());
    }

    if requested.id == correspondence.source_id {
        if requested.mirror_link != source.mirror_link && requested.mirror_link.is_some() {
            return Err(OVERLAP.into());
        }
        return Ok(Outcome::PassThrough);
    }
    if requested.id != correspondence.mirror_id {
        return Ok(Outcome::PassThrough);
    }
    let link = match links.target(MirroredItemKind::Layout, &requested.id)? {
        Target::NotMirror => return Ok(Outcome::PassThrough),
        Target::Local => return Ok(Outcome::DeliberatelyUnchanged(TARGET_OWNED)),
        Target::Omitted => {
            return Err(format!(
                "Layout {:?} is omitted on its mirror half and cannot be edited there; rejoin it first",
                requested.id
            ));
        }
        Target::Linked(link) => link,
    };
    let mirror = mirror.ok_or_else(|| {
        format!(
            "Native mirror layout {:?} is unavailable",
            correspondence.mirror_id
        )
    })?;
    if link.correspondence.source_id != correspondence.source_id
        || link.correspondence.mirror_id != correspondence.mirror_id
    {
        return Err(format!(
            "Layout {:?} resolves to an inconsistent native correspondence",
            requested.id
        ));
    }

    if requested.board_id != mirror.board_id || requested.matrix_ids != mirror.matrix_ids {
        return Err(format!(
            "The ownership of native mirror layout {:?} cannot be reassigned",
            requested.id
        ));
    }
    if requested.mirror_link != mirror.mirror_link {
        return Err(format!(
            "The legacy mirror link of native layout {:?} cannot be changed through a source SetLayout",
            requested.id
        ));
    }
    if source.board_id != link.pair.halves.source_board_id
        || mirror.board_id != link.pair.halves.mirror_board_id
    {
        return Err(format!(
            "Native layout {:?} has stale source or mirror board ownership",
            requested.id
        ));
    }
    for id in &mirror.matrix_ids {
        let (source_matrix_id, mapping) = links.matrix_frame(id)?;
        if !source.matrix_ids.contains(&source_matrix_id) || mapping != Some(link.pair.mapping) {
            return Err(format!(
                "Native layout {:?} does not follow its paired matrix correspondence",
                requested.id
            ));
        }
    }

    let (current_source_ids, omitted_source_ids) = source_membership(link, source)?;
    let current_projection = project_source_members(link, &current_source_ids)?;
    let current_projection_ids: BTreeSet<_> = current_projection.iter().collect();
    let current_target_members: Vec<_> = mirror
        .part_ids
        .iter()
        .filter(|id| !current_projection_ids.contains(id))
        .cloned()
        .collect();
    let mut requested_source_ids = Vec::with_capacity(requested.part_ids.len());
    let mut requested_projection = Vec::with_capacity(requested.part_ids.len());
    let mut requested_target_members = Vec::new();
    let mut seen_requested = BTreeSet::new();
    for mirror_id in &requested.part_ids {
        if !seen_requested.insert(mirror_id) {
            return Err(format!(
                "Layout membership contains duplicate part {mirror_id:?}"
            ));
        }
        match links.target(MirroredItemKind::Part, mirror_id)? {
            Target::Linked(part_link) => {
                if !same_pair(part_link, link) {
                    return Err(format!(
                        "Part {mirror_id:?} belongs to a different native pair than layout {:?}",
                        requested.id
                    ));
                }
                if link
                    .pair
                    .halves
                    .overlay_of(part_link.correspondence)
                    .is_some_and(|overlay| overlay.state == MirroredExceptionState::Omitted)
                {
                    return Err(format!(
                        "Part {mirror_id:?} is omitted on its mirror half and cannot be added to layout {:?}",
                        requested.id
                    ));
                }
                requested_source_ids.push(part_link.correspondence.source_id.clone());
                requested_projection.push(mirror_id.clone());
            }
            Target::Local => {
                if let Some(source_id) = source_id_for_mirror(link, mirror_id)?
                    && current_source_ids.contains(&source_id)
                {
                    requested_source_ids.push(source_id);
                    requested_projection.push(mirror_id.clone());
                    continue;
                }
                requested_target_members.push(mirror_id.clone());
            }
            Target::NotMirror => requested_target_members.push(mirror_id.clone()),
            Target::Omitted => {
                return Err(format!(
                    "Part {mirror_id:?} is omitted on its mirror half and cannot be added to layout {:?}",
                    requested.id
                ));
            }
        }
    }

    let requested_source_set: BTreeSet<_> = requested_source_ids.iter().collect();
    if requested_source_set.len() != requested_source_ids.len() {
        return Err(format!(
            "Native layout {:?} maps duplicate mirror membership to one source part",
            requested.id
        ));
    }
    for source_id in &current_source_ids {
        let correspondence = source_correspondence(link, source_id)?
            .ok_or_else(|| format!("Source part {source_id:?} has no native correspondence"))?;
        let is_source_local = link
            .pair
            .halves
            .overlay_of(correspondence)
            .is_some_and(|overlay| overlay.state == MirroredExceptionState::Local);
        if is_source_local && !requested_source_set.contains(source_id) {
            return Err(format!(
                "The source-projected Local part {:?} cannot be removed from native layout {:?} through one source SetLayout",
                correspondence.mirror_id, requested.id
            ));
        }
    }

    let has_target_owned_change =
        requested.name != mirror.name || requested_target_members != current_target_members;
    if requested_projection != current_projection && has_target_owned_change {
        return Err(format!(
            "The requested native mirror layout {:?} mixes source-projected membership changes with target-owned changes",
            requested.id
        ));
    }
    let mut source_members = requested_source_ids;
    source_members.extend(omitted_source_ids);
    let projected_after_source = project_source_members(link, &source_members)?;
    // The inverse operation can preserve pre-existing target-owned extras while mapping linked
    // membership changes into source order.
    let mut after_source_reconcile = projected_after_source.clone();
    after_source_reconcile.extend(current_target_members.iter().cloned());

    // When source-projected membership is unchanged, the target layout can carry its own name
    // and unprojected member changes. Reconciliation writes projected members first, then keeps
    // the target entries supplied by this authored operation.
    let mut after_target_edit_reconcile = current_projection.clone();
    after_target_edit_reconcile.extend(requested_target_members.iter().cloned());
    if has_target_owned_change
        && requested_projection == current_projection
        && after_target_edit_reconcile == requested.part_ids
    {
        return Ok(Outcome::DeliberatelyUnchanged(TARGET_OWNED));
    }

    if after_source_reconcile == requested.part_ids {
        if requested.name != mirror.name {
            return Err(format!(
                "The requested native mirror layout {:?} mixes a target-owned name change with a source membership change",
                requested.id
            ));
        }
        let mut canonical = source.clone();
        canonical.part_ids = source_members;
        return Ok(Outcome::Translated(canonical));
    }

    Err(format!(
        "The requested membership of native mirror layout {:?} cannot be reproduced by one source SetLayout",
        requested.id
    ))
}

fn same_pair(left: Link<'_>, right: Link<'_>) -> bool {
    std::ptr::eq(left.pair.halves, right.pair.halves) && left.pair.mapping == right.pair.mapping
}

fn source_membership(
    link: Link<'_>,
    source: &Layout,
) -> Result<(Vec<String>, Vec<String>), String> {
    let mut source_ids = BTreeSet::new();
    let mut omitted = Vec::new();
    for source_id in &source.part_ids {
        if !source_ids.insert(source_id.clone()) {
            return Err(format!(
                "Source layout {:?} contains duplicate part membership {source_id:?}",
                source.id
            ));
        }
        let correspondence = source_correspondence(link, source_id)?.ok_or_else(|| {
            format!(
                "Source layout {:?} contains part {source_id:?} without a native part correspondence",
                source.id
            )
        })?;
        if link
            .pair
            .halves
            .overlay_of(correspondence)
            .is_some_and(|overlay| overlay.state == MirroredExceptionState::Omitted)
        {
            omitted.push(source_id.clone());
        }
    }
    Ok((source.part_ids.clone(), omitted))
}

fn source_correspondence<'a>(
    link: Link<'a>,
    source_id: &str,
) -> Result<Option<&'a crate::model::MirroredCorrespondence>, String> {
    let mut matches = link
        .pair
        .halves
        .correspondences
        .iter()
        .filter(|item| item.kind == MirroredItemKind::Part && item.source_id == source_id);
    let Some(correspondence) = matches.next() else {
        return Ok(None);
    };
    if matches.next().is_some() {
        return Err(format!(
            "Source part {source_id:?} has ambiguous correspondence"
        ));
    }
    Ok(Some(correspondence))
}

fn source_id_for_mirror(link: Link<'_>, mirror_id: &str) -> Result<Option<String>, String> {
    let mut matches = link
        .pair
        .halves
        .correspondences
        .iter()
        .filter(|item| item.kind == MirroredItemKind::Part && item.mirror_id == mirror_id);
    let Some(correspondence) = matches.next() else {
        return Ok(None);
    };
    if matches.next().is_some() {
        return Err(format!(
            "Mirror part {mirror_id:?} has ambiguous correspondence"
        ));
    }
    Ok(Some(correspondence.source_id.clone()))
}

fn project_source_members(link: Link<'_>, source_ids: &[String]) -> Result<Vec<String>, String> {
    let mut projected = Vec::with_capacity(source_ids.len());
    for source_id in source_ids {
        let correspondence = source_correspondence(link, source_id)?
            .ok_or_else(|| format!("Source part {source_id:?} has no native correspondence"))?;
        if link
            .pair
            .halves
            .overlay_of(correspondence)
            .is_some_and(|overlay| overlay.state == MirroredExceptionState::Omitted)
        {
            continue;
        }
        projected.push(correspondence.mirror_id.clone());
    }
    Ok(projected)
}

fn unique_layout<'a>(links: &'a Links<'_>, id: &str) -> Result<&'a Layout, String> {
    optional_layout(links, id)?.ok_or_else(|| format!("Native layout {id:?} is unavailable"))
}

fn is_pure_unlink(requested: &Layout, current: &Layout) -> bool {
    if current.mirror_link.is_none() || requested.mirror_link.is_some() {
        return false;
    }
    let mut expected = current.clone();
    expected.mirror_link = None;
    *requested == expected
}

fn optional_layout<'a>(links: &'a Links<'_>, id: &str) -> Result<Option<&'a Layout>, String> {
    let mut layouts = links
        .document
        .layouts
        .iter()
        .filter(|layout| layout.id == id);
    let layout = layouts.next();
    if layouts.next().is_some() {
        return Err(format!("Native layout {id:?} is ambiguous"));
    }
    Ok(layout)
}

fn overlaps_legacy_link(
    links: &Links<'_>,
    correspondence: &crate::model::MirroredCorrespondence,
    source: &Layout,
    mirror: Option<&Layout>,
) -> bool {
    let ids = [&correspondence.source_id, &correspondence.mirror_id];
    let layout_overlap = links.document.layouts.iter().any(|layout| {
        layout
            .mirror_link
            .as_ref()
            .is_some_and(|legacy| ids.contains(&&layout.id) || ids.contains(&&legacy.source_id))
    });
    layout_overlap
        || source
            .matrix_ids
            .iter()
            .any(|id| crate::matrix::layout::has_link(links.document, id))
        || mirror.is_some_and(|mirror| {
            mirror
                .matrix_ids
                .iter()
                .any(|id| crate::matrix::layout::has_link(links.document, id))
        })
}

fn introduces_native_legacy_overlap(links: &Links<'_>, requested: &Layout) -> bool {
    let Some(legacy) = &requested.mirror_link else {
        return false;
    };
    links.pairs.iter().any(|pair| {
        pair.halves.correspondences.iter().any(|correspondence| {
            (correspondence.kind == MirroredItemKind::Layout
                && [
                    correspondence.source_id.as_str(),
                    correspondence.mirror_id.as_str(),
                ]
                .contains(&legacy.source_id.as_str()))
                || (correspondence.kind == MirroredItemKind::Matrix
                    && [
                        correspondence.source_id.as_str(),
                        correspondence.mirror_id.as_str(),
                    ]
                    .iter()
                    .any(|id| requested.matrix_ids.iter().any(|matrix| matrix == id)))
        })
    })
}

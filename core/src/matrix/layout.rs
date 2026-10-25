use crate::{matrix, mechanical::pair, model::*};
use std::collections::{BTreeMap, BTreeSet};

const MIRRORED_COMPONENT_SOURCE: &str = "boardstudio.mirroredComponentSource";
const MIRRORED_COMPONENT_SUBSTITUTE: &str = "boardstudio.mirroredComponentSubstitute";

pub(crate) fn validate(doc: &ProjectDoc) -> Result<(), String> {
    // Bound every matrix before any walk over its rows and columns below.
    for matrix in &doc.matrices {
        matrix::valid_dimensions(matrix)?;
    }
    if doc.layouts.is_empty() {
        return Ok(());
    }
    let mut ids = BTreeSet::new();
    let mut matrices = BTreeSet::new();
    let mut parts = BTreeSet::new();
    let mut linked = BTreeSet::new();
    for layout in &doc.layouts {
        if layout.id.trim().is_empty() || layout.name.trim().is_empty() || !ids.insert(&layout.id) {
            return Err("Layout names and identities must be present and unique".into());
        }
        let board = doc
            .boards
            .iter()
            .find(|board| board.id == layout.board_id)
            .ok_or("Layout board is missing")?;
        if layout.matrix_ids.is_empty() {
            return Err("Layout matrix membership must be nonempty".into());
        }
        for id in &layout.matrix_ids {
            let matrix = matrix::evaluation::logical_matrix(id, doc)?;
            if !matrices.insert(matrix.id.clone()) || matrix.board_id.as_ref() != Some(&board.id) {
                return Err("Each layout must own distinct matrices on its board".into());
            }
        }
        for id in &layout.part_ids {
            if !parts.insert(id)
                || !board.part_ids.contains(id)
                || !doc.parts.iter().any(|part| &part.id == id)
                || doc
                    .matrices
                    .iter()
                    .any(|matrix| matrix.part_ids.contains(id))
            {
                return Err("Layout components must be independent parts on the same board".into());
            }
        }
        if let Some(link) = &layout.mirror_link {
            let source = doc
                .layouts
                .iter()
                .find(|other| other.id == link.source_id)
                .ok_or("Linked layout is missing")?;
            if !link.axis_x.is_finite()
                || source.id == layout.id
                || source.board_id != layout.board_id
                || source.mirror_link.is_some()
                || !linked.insert(&source.id)
                || !linked.insert(&layout.id)
            {
                return Err("A mirrored pair must link two distinct layouts on one board".into());
            }
            if link.split_gap.is_some_and(|gap| !gap.is_finite()) {
                return Err("Split gap must be finite".into());
            }
            validate_link(doc, source, layout, link)?;
        }
    }
    for part in &doc.parts {
        if primary_member(doc, &part.id)
            .is_some_and(|(matrix, _, _)| partner(doc, &matrix.id).is_some())
            && doc.constraint_driving_part(&part.id)?.is_some()
        {
            return Err("Unlink the halves before constraining a linked key's position".into());
        }
    }
    Ok(())
}

fn owns_member(doc: &ProjectDoc, layout: &Layout, id: &str) -> bool {
    layout.part_ids.iter().any(|member| member == id)
        || layout.matrix_ids.iter().any(|matrix_id| {
            let Ok(matrix) = matrix::evaluation::logical_matrix(matrix_id, doc) else {
                return false;
            };
            let Some(slot) = id.strip_prefix(&format!("matrix/{matrix_id}/r")) else {
                return false;
            };
            let (slot, assembly) = slot
                .split_once('/')
                .map_or((slot, None), |(slot, assembly)| (slot, Some(assembly)));
            let Some((row, column)) = slot.split_once('c') else {
                return false;
            };
            let (Ok(row), Ok(column)) = (row.parse::<u32>(), column.parse::<u32>()) else {
                return false;
            };
            row < matrix.rows
                && column < matrix.columns
                && assembly.is_none_or(|id| {
                    matrix.cells.iter().any(|cell| {
                        cell.row == row
                            && cell.column == column
                            && cell.assemblies.iter().any(|assembly| assembly.id == id)
                    })
                })
        })
}

fn validate_link(
    doc: &ProjectDoc,
    source: &Layout,
    target: &Layout,
    link: &LayoutMirrorLink,
) -> Result<(), String> {
    let mut sources = Vec::new();
    let mut targets = Vec::new();
    for record in &link.correspondences {
        let source_key = (record.kind, record.source_id.as_str());
        let target_key = (record.kind, record.mirror_id.as_str());
        if sources.contains(&source_key) || targets.contains(&target_key) {
            return Err(
                "Layout correspondences must have unique source and mirror identities".into(),
            );
        }
        sources.push(source_key);
        targets.push(target_key);
        let owned = match record.kind {
            MirroredItemKind::Matrix => {
                source.matrix_ids.contains(&record.source_id)
                    && target.matrix_ids.contains(&record.mirror_id)
            }
            MirroredItemKind::Part => {
                owns_member(doc, source, &record.source_id)
                    && owns_member(doc, target, &record.mirror_id)
            }
            _ => false,
        };
        if !owned {
            return Err("Layout correspondence has invalid ownership".into());
        }
    }
    if source
        .matrix_ids
        .iter()
        .any(|id| !sources.contains(&(MirroredItemKind::Matrix, id.as_str())))
    {
        return Err(
            "Linked Layout requires an explicit correspondence for every source matrix".into(),
        );
    }
    let mut overlays = Vec::new();
    for exception in &link.exceptions {
        let key = (exception.kind, exception.mirror_id.as_str());
        if overlays.contains(&key) {
            return Err("Layout exceptions must have unique identities".into());
        }
        overlays.push(key);
        let owned = match exception.kind {
            MirroredItemKind::Matrix => target.matrix_ids.contains(&exception.mirror_id),
            MirroredItemKind::Part => owns_member(doc, target, &exception.mirror_id),
            _ => false,
        };
        if !owned || (!targets.contains(&key) && exception.state != MirroredExceptionState::Local) {
            return Err("Layout exception has invalid ownership or no correspondence".into());
        }
    }
    Ok(())
}

/// The slots both halves still own, paired by slot. A mirror without a physical matrix (an
/// omitted group) pairs by the source shape.
pub(crate) fn member_pairs(
    source: &Matrix,
    mirror_id: &str,
    mirror: Option<&Matrix>,
) -> Vec<(String, String)> {
    let mirror_members = mirror.map(matrix::cell_members);
    matrix::cell_members(source)
        .into_iter()
        .filter_map(|(slot, source_id)| {
            Some((
                source_id,
                match &mirror_members {
                    Some(members) => members.get(&slot).cloned()?,
                    None => matrix::member_id(mirror_id, slot.0, slot.1),
                },
            ))
        })
        .collect()
}

/// Part correspondences for explicit member pairs.
pub(crate) fn part_correspondences(
    pairs: impl IntoIterator<Item = (String, String)>,
) -> impl Iterator<Item = MirroredCorrespondence> {
    pairs
        .into_iter()
        .map(|(source_id, mirror_id)| MirroredCorrespondence {
            kind: MirroredItemKind::Part,
            source_id,
            mirror_id,
            companion_mode: None,
            companion_tweak: None,
        })
}

/// A Local Part exception excuses its mirror member from pairing.
fn is_excepted(exceptions: &[MirroredException], mirror_id: &str) -> bool {
    exceptions.iter().any(|exception| {
        exception.kind == MirroredItemKind::Part && exception.mirror_id == mirror_id
    })
}

pub(crate) fn populate_new_link(doc: &ProjectDoc, target: &mut Layout) -> Result<(), String> {
    let Some(link) = &mut target.mirror_link else {
        return Ok(());
    };
    if !link.correspondences.is_empty() {
        return Ok(());
    }
    let source = doc
        .layouts
        .iter()
        .find(|layout| layout.id == link.source_id)
        .ok_or("Linked layout is missing")?;
    if source.matrix_ids.len() != 1 || target.matrix_ids.len() != 1 {
        return Err("Linking matrix groups requires explicit matrix correspondences".into());
    }
    let source_id = &source.matrix_ids[0];
    let mirror_id = &target.matrix_ids[0];
    link.correspondences.push(MirroredCorrespondence {
        kind: MirroredItemKind::Matrix,
        source_id: source_id.clone(),
        mirror_id: mirror_id.clone(),
        companion_mode: None,
        companion_tweak: None,
    });
    let matrix = matrix::evaluation::logical_matrix(source_id, doc)?;
    let exceptions = link.exceptions.clone();
    link.correspondences.extend(part_correspondences(
        member_pairs(&matrix, mirror_id, None)
            .into_iter()
            .filter(|(_, mirror_id)| !is_excepted(&exceptions, mirror_id)),
    ));
    Ok(())
}

/// Membership edits maintain explicit identities; admission never repairs them.
pub(crate) fn sync_member_pairs(doc: &mut ProjectDoc, matrix_id: &str) -> Result<(), String> {
    for index in 0..doc.layouts.len() {
        let Some(link) = doc.layouts[index].mirror_link.as_ref() else {
            continue;
        };
        let pairs: Vec<_> = link
            .correspondences
            .iter()
            .filter(|record| {
                record.kind == MirroredItemKind::Matrix
                    && (record.source_id == matrix_id || record.mirror_id == matrix_id)
            })
            .cloned()
            .collect();
        if pairs.is_empty() {
            continue;
        }
        let source_index = doc
            .layouts
            .iter()
            .position(|layout| layout.id == link.source_id)
            .ok_or("Linked layout is missing")?;
        let pairs_of_slots: Vec<_> = pairs
            .iter()
            .map(|pair| {
                let source_matrix = matrix::evaluation::logical_matrix(&pair.source_id, doc)?;
                let mirror_matrix = matrix::evaluation::logical_matrix(&pair.mirror_id, doc)?;
                Ok(member_pairs(
                    &source_matrix,
                    &pair.mirror_id,
                    Some(mirror_matrix.as_ref()),
                ))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let Some(mut link) = doc.layouts[index].mirror_link.take() else {
            continue;
        };
        let source = &doc.layouts[source_index];
        let target = &doc.layouts[index];
        link.correspondences.retain(|record| {
            record.kind != MirroredItemKind::Part
                || (owns_member(doc, source, &record.source_id)
                    && owns_member(doc, target, &record.mirror_id))
        });
        link.exceptions.retain(|record| {
            record.kind != MirroredItemKind::Part || owns_member(doc, target, &record.mirror_id)
        });
        for slots in pairs_of_slots {
            for (source_id, mirror_id) in slots {
                if is_excepted(&link.exceptions, &mirror_id)
                    || link.correspondences.iter().any(|record| {
                        record.kind == MirroredItemKind::Part
                            && (record.source_id == source_id || record.mirror_id == mirror_id)
                    })
                {
                    continue;
                }
                link.correspondences
                    .extend(part_correspondences([(source_id, mirror_id)]));
            }
        }
        doc.layouts[index].mirror_link = Some(link);
    }
    Ok(())
}

/// The relationship is stored once, but geometry can be edited from either half.
pub(crate) fn has_link(doc: &ProjectDoc, matrix_id: &str) -> bool {
    doc.layouts
        .iter()
        .filter(|layout| layout.matrix_ids.iter().any(|id| id == matrix_id))
        .any(|owner| {
            owner.mirror_link.is_some()
                || doc.layouts.iter().any(|layout| {
                    layout
                        .mirror_link
                        .as_ref()
                        .is_some_and(|link| link.source_id == owner.id)
                })
        })
}

pub(crate) fn partner(doc: &ProjectDoc, matrix_id: &str) -> Option<(String, f64)> {
    doc.layouts
        .iter()
        .filter_map(|layout| layout.mirror_link.as_ref())
        .find_map(|link| {
            link.correspondences.iter().find_map(|record| {
                if record.kind != MirroredItemKind::Matrix {
                    return None;
                }
                if record.source_id == matrix_id {
                    Some((record.mirror_id.clone(), link.axis_x))
                } else if record.mirror_id == matrix_id {
                    Some((record.source_id.clone(), link.axis_x))
                } else {
                    None
                }
            })
        })
}

/// The pair map of a vertical mirror line at `axis_x`: the reflection `x' = 2 * axis_x - x`.
pub(crate) fn axis_mapping(axis_x: f64) -> Pose2 {
    Pose2 {
        at: Vec2 {
            x: 2.0 * axis_x,
            y: 0.0,
        },
        rotation: 0.0,
    }
}

/// Key-edge to key-edge distance implied by board-coordinate geometry, measured
/// at the half-origin column: the source origin's key edge to its reflection.
/// This is the stored Split gap a legacy pair adopts on its first edit.
pub(crate) fn implied_gap(axis_x: f64, matrix: &Matrix) -> f64 {
    gap_from_origin(axis_x, matrix.origin.x, matrix.pitch.x, matrix.edge_gap)
}

fn gap_from_origin(axis_x: f64, origin_x: f64, pitch_x: f64, edge_gap: Option<Vec2>) -> f64 {
    let edge = edge_gap
        .map(|gap| gap.x)
        .unwrap_or(super::DEFAULT_EDGE_GAP_MM);
    2.0 * (axis_x - origin_x) - (pitch_x - edge)
}

/// The link, axis, stored gap and side of the pair a matrix belongs to: `true`
/// when the matrix lives on the mirror layout. Record-absent mirror members
/// resolve through their correspondence.
fn pair_frame(doc: &ProjectDoc, matrix_id: &str) -> Option<(f64, Option<f64>, bool)> {
    for layout in &doc.layouts {
        if let Some(link) = layout.mirror_link.as_ref()
            && layout.matrix_ids.iter().any(|id| id == matrix_id)
        {
            return Some((link.axis_x, link.split_gap, true));
        }
    }
    for layout in &doc.layouts {
        if layout.mirror_link.is_some() || !layout.matrix_ids.iter().any(|id| id == matrix_id) {
            continue;
        }
        let link = doc
            .layouts
            .iter()
            .filter_map(|other| other.mirror_link.as_ref())
            .find(|link| link.source_id == layout.id)?;
        return Some((link.axis_x, link.split_gap, false));
    }
    for layout in &doc.layouts {
        if let Some(link) = layout.mirror_link.as_ref()
            && link.correspondences.iter().any(|record| {
                record.kind == MirroredItemKind::Matrix && record.mirror_id == matrix_id
            })
        {
            return Some((link.axis_x, link.split_gap, true));
        }
    }
    None
}

/// The stored Split gap of the pair a matrix belongs to, if the pair has one.
pub(crate) fn pair_gap(doc: &ProjectDoc, matrix_id: &str) -> Option<f64> {
    pair_frame(doc, matrix_id).and_then(|(_, gap, _)| gap)
}

/// Board-space position of the half frame a matrix is placed in: the split
/// axis offset by half the stored gap. `None` for unlinked matrices and legacy
/// pairs without a stored gap, whose origins stay board-coordinate.
pub(crate) fn half_offset(doc: &ProjectDoc, matrix_id: &str) -> Option<Vec2> {
    let (axis_x, gap, mirror_side) = pair_frame(doc, matrix_id)?;
    let gap = gap?;
    Some(Vec2 {
        x: if mirror_side {
            axis_x + gap / 2.0
        } else {
            axis_x - gap / 2.0
        },
        y: 0.0,
    })
}

/// Adopt the half frame for every legacy pair (no stored gap) that has an
/// unanchored source matrix: store the implied gap and re-express unanchored
/// World origins as half-frame values. Every evaluated world pose is
/// preserved, so adoption moves nothing. Fully anchored pairs stay legacy;
/// spacing owns their envelopes (ticket 10).
pub(crate) fn adopt_half_frames(doc: &mut ProjectDoc) -> Result<Vec<String>, String> {
    let targets: Vec<String> = doc
        .layouts
        .iter()
        .filter(|layout| {
            layout
                .mirror_link
                .as_ref()
                .is_some_and(|link| link.split_gap.is_none())
        })
        .map(|layout| layout.id.clone())
        .collect();
    let mut changed = Vec::new();
    for target_id in targets {
        let (source_id, axis_x) = {
            let link = doc
                .layouts
                .iter()
                .find(|layout| layout.id == target_id)
                .and_then(|layout| layout.mirror_link.as_ref())
                .ok_or("Linked layout is missing")?;
            (link.source_id.clone(), link.axis_x)
        };
        let Some(reference) = doc
            .layouts
            .iter()
            .find(|layout| layout.id == source_id)
            .map(|layout| layout.matrix_ids.clone())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|id| doc.matrices.iter().find(|matrix| matrix.id == id))
            .find(|matrix| matrix.anchor.is_none())
            .map(|matrix| (matrix.origin.x, matrix.pitch.x, matrix.edge_gap))
        else {
            continue;
        };
        let gap = gap_from_origin(axis_x, reference.0, reference.1, reference.2);
        if !gap.is_finite() {
            return Err("Split gap must be finite".into());
        }
        let members: Vec<(String, bool)> = {
            let source_ids = doc
                .layouts
                .iter()
                .find(|layout| layout.id == source_id)
                .map(|layout| layout.matrix_ids.clone())
                .unwrap_or_default();
            let target_ids = doc
                .layouts
                .iter()
                .find(|layout| layout.id == target_id)
                .map(|layout| layout.matrix_ids.clone())
                .unwrap_or_default();
            source_ids
                .into_iter()
                .map(|id| (id, false))
                .chain(target_ids.into_iter().map(|id| (id, true)))
                .collect()
        };
        for (matrix_id, mirror_side) in members {
            if let Some(matrix) = doc
                .matrices
                .iter_mut()
                .find(|matrix| matrix.id == matrix_id)
                && matrix.anchor.is_none()
            {
                matrix.origin.x -= if mirror_side {
                    axis_x + gap / 2.0
                } else {
                    axis_x - gap / 2.0
                };
                changed.push(matrix_id);
            }
        }
        if let Some(layout) = doc.layouts.iter_mut().find(|layout| layout.id == target_id)
            && let Some(link) = layout.mirror_link.as_mut()
        {
            link.split_gap = Some(gap);
            changed.push(target_id);
        }
    }
    Ok(changed)
}

/// Restore board-coordinate origins for a stored-gap pair (unlink or gap
/// removal), preserving every evaluated world pose. Refuses when a sibling
/// gapped mirror still positions the same source half.
pub(crate) fn unadopt_pair(doc: &mut ProjectDoc, layout_id: &str) -> Result<Vec<String>, String> {
    let (source_id, axis_x, gap) = {
        let layout = doc
            .layouts
            .iter()
            .find(|layout| layout.id == layout_id)
            .ok_or("Layout does not exist")?;
        let link = layout
            .mirror_link
            .as_ref()
            .ok_or("Layout has no mirror link")?;
        let gap = link.split_gap.ok_or("Layout has no stored Split gap")?;
        (link.source_id.clone(), link.axis_x, gap)
    };
    if doc.layouts.iter().any(|layout| {
        layout.id != layout_id
            && layout
                .mirror_link
                .as_ref()
                .is_some_and(|link| link.source_id == source_id && link.split_gap.is_some())
    }) {
        return Err(
            "Unlink one mirrored half at a time; a sibling mirror still shares this source half"
                .into(),
        );
    }
    let mut changed = Vec::new();
    let members: Vec<(String, bool)> = {
        let source_ids = doc
            .layouts
            .iter()
            .find(|layout| layout.id == source_id)
            .map(|layout| layout.matrix_ids.clone())
            .unwrap_or_default();
        let target_ids = doc
            .layouts
            .iter()
            .find(|layout| layout.id == layout_id)
            .map(|layout| layout.matrix_ids.clone())
            .unwrap_or_default();
        source_ids
            .into_iter()
            .map(|id| (id, false))
            .chain(target_ids.into_iter().map(|id| (id, true)))
            .collect()
    };
    for (matrix_id, mirror_side) in members {
        if let Some(matrix) = doc
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == matrix_id)
            && matrix.anchor.is_none()
        {
            matrix.origin.x += if mirror_side {
                axis_x + gap / 2.0
            } else {
                axis_x - gap / 2.0
            };
            changed.push(matrix_id);
        }
    }
    Ok(changed)
}

/// Set the stored Split gap on every link of the pair a layout belongs to:
/// the carrier's own link, or every mirror link pointing at a source layout.
pub(crate) fn set_pair_gap(
    doc: &mut ProjectDoc,
    layout_id: &str,
    gap: f64,
) -> Result<Vec<String>, String> {
    let carriers: Vec<String> = match doc.layouts.iter().find(|layout| layout.id == layout_id) {
        Some(layout) if layout.mirror_link.is_some() => vec![layout_id.to_string()],
        Some(_) => doc
            .layouts
            .iter()
            .filter(|layout| {
                layout
                    .mirror_link
                    .as_ref()
                    .is_some_and(|link| link.source_id == layout_id)
            })
            .map(|layout| layout.id.clone())
            .collect(),
        None => vec![],
    };
    if carriers.is_empty() {
        return Err(format!("Layout {layout_id} is not part of a mirrored pair"));
    }
    for id in &carriers {
        if let Some(layout) = doc.layouts.iter_mut().find(|layout| layout.id == *id)
            && let Some(link) = layout.mirror_link.as_mut()
        {
            link.split_gap = Some(gap);
        }
    }
    Ok(carriers)
}

/// A companion's offset and turn are relative to its part, so the reflection only flips their
/// handedness. The pair rotation is carried once, by the part's own heading.
fn reflected_companion(mut assembly: MatrixAssembly) -> MatrixAssembly {
    assembly.offset.x = -assembly.offset.x;
    assembly.rotation = assembly.rotation.map(|rotation| -rotation);
    assembly
}

/// Reflects `source` through a pair's `mapping` onto `target`, which keeps its identity, board and
/// part ids. Layout records and all key values copy unchanged; the kernel's local mirror
/// reflects the whole local heading once. Companions retain their separate part-local
/// placement rule. Anchored Origin/Rotation are relative, not reflected world points.
///
/// With `half_frame`, both origins are already half-local: the mirror keeps the
/// negated half-frame Origin/Rotation instead of a board-space reflection, so a
/// later Split gap change moves the evaluated halves without rewriting storage.
pub(crate) fn reflected(
    source: &Matrix,
    target: &Matrix,
    mapping: Pose2,
    local_companions: bool,
    half_frame: bool,
) -> Result<Matrix, String> {
    // The pair uses a vertical reflection of the local None/X frames.
    // Switching to a horizontal local frame requires an independent layout.
    if source.mirror == Some(Mirror::Y) {
        return Err("Unlink the halves before changing to a horizontal matrix mirror".into());
    }
    let mut result = target.clone();
    result.rows = source.rows;
    result.columns = source.columns;
    result.pitch = source.pitch;
    result.edge_gap = source.edge_gap;
    if half_frame || source.anchor.is_some() {
        // Half-frame and anchored origins share one rule: relative X negates.
        result.origin = Vec2 {
            x: -source.origin.x,
            y: source.origin.y,
        };
        result.rotation = source.rotation.map(|rotation| -rotation);
    } else {
        result.origin = pair::forward(mapping, source.origin);
        result.rotation = if source.rotation.is_some() || mapping.rotation != 0.0 {
            Some(pair::forward_angle(mapping, source.rotation.unwrap_or(0.0)))
        } else {
            None
        };
    }
    result.mirror = Some(if source.mirror == Some(Mirror::X) {
        Mirror::None
    } else {
        Mirror::X
    });
    result.diode_direction = source.diode_direction;
    result.row_layout = source.row_layout.clone();
    result.column_layout = source.column_layout.clone();
    result.anchor = source.anchor.clone();
    if let Some(anchor) = &mut result.anchor {
        // Relative Origin X and Rotation negate; Y stays. When the receiver
        // already carries a mapped anchor (Omitted logical matrices), keep
        // it; `sync` remaps via correspondences immediately after and
        // overwrites any stale identity there.
        if let Some(mapped) = target.anchor.as_ref().map(|a| a.matrix_id.clone()) {
            anchor.matrix_id = mapped;
        }
        result.origin = Vec2 {
            x: -source.origin.x,
            y: source.origin.y,
        };
        result.rotation = source.rotation.map(|r| -r);
    }
    let source_cells: BTreeMap<_, _> = source
        .cells
        .iter()
        .map(|cell| ((cell.row, cell.column), cell))
        .collect();
    let mut cells: BTreeMap<_, _> = target
        .cells
        .iter()
        .filter(|cell| cell.row < source.rows && cell.column < source.columns)
        .map(|cell| ((cell.row, cell.column), cell.clone()))
        .collect();
    let local_preset = target.cells.iter().find(|cell| {
        cell.assemblies_local == Some(true)
            && cell.definition_id.as_deref() == Some(target.definition_id.as_str())
            && cell
                .variant
                .as_deref()
                .is_some_and(|variant| variant.starts_with("preset/"))
    });
    for (coordinate, source_cell) in &source_cells {
        cells.entry(*coordinate).or_insert_with(|| {
            // New keys inherit this half's saved hardware, not the other half's
            // local companion assignments. Existing per-key overrides stay intact.
            if (source_cell.row >= target.rows || source_cell.column >= target.columns)
                && let Some(template) = local_preset
            {
                return MatrixCell {
                    row: source_cell.row,
                    column: source_cell.column,
                    enabled: true,
                    offset: None,
                    rotation: None,
                    ..template.clone()
                };
            }
            MatrixCell {
                shift: None,
                bend: None,
                deleted: false,
                row: source_cell.row,
                column: source_cell.column,
                enabled: true,

                definition_id: None,
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![],
                assemblies_local: None,
            }
        });
    }
    for (coordinate, cell) in &mut cells {
        let source_cell = source_cells.get(coordinate);
        cell.enabled = source_cell.is_none_or(|cell| cell.enabled);
        cell.deleted = source_cell.is_some_and(|cell| cell.deleted);
        cell.offset = source_cell.and_then(|cell| cell.offset);
        // Cell turns and companion offsets are relative to the matrix frame and to the part, which
        // already carry the pair rotation, so only their handedness flips here.
        cell.rotation = source_cell.and_then(|cell| cell.rotation);
        cell.shift = source_cell.and_then(|cell| cell.shift);
        cell.bend = source_cell.and_then(|cell| cell.bend);
        // A sync keeps the other half's local companions; a new half copies them, since it has none.
        if cell.assemblies_local != Some(true)
            && (local_companions
                || !source_cell.is_some_and(|cell| cell.assemblies_local == Some(true)))
        {
            cell.definition_id = source_cell.and_then(|cell| cell.definition_id.clone());
            cell.variant = source_cell.and_then(|cell| cell.variant.clone());
            cell.assemblies = source_cell
                .map(|cell| {
                    cell.assemblies
                        .iter()
                        .cloned()
                        .map(reflected_companion)
                        .collect()
                })
                .unwrap_or_default();
            cell.assemblies_local = if local_companions {
                source_cell.and_then(|cell| cell.assemblies_local)
            } else {
                None
            };
        }
    }
    result.cells = cells.into_values().collect();
    Ok(result)
}

/// Ticket 31: remap a whole anchor through matrix correspondences without
/// silently retargeting: the single key, or every aggregate part by identity
/// (row/column stay the physical key). Method and Follow cross unchanged; the
/// relative-negate rules apply to the resolved aggregate pose, never per
/// part. A missing counterpart for any part refuses naming the layout.
fn remap_anchor(
    correspondences: &[MirroredCorrespondence],
    anchor: &MatrixAnchor,
    to_mirror: bool,
    layout_id: &str,
) -> Result<MatrixAnchor, String> {
    let mut next = anchor.clone();
    if let Some(aggregate) = &anchor.aggregate {
        let mut parts = Vec::with_capacity(aggregate.parts.len());
        for part in &aggregate.parts {
            parts.push(AnchorPart {
                matrix_id: remap_anchor_target(
                    correspondences,
                    &part.matrix_id,
                    to_mirror,
                    layout_id,
                )?,
                row: part.row,
                column: part.column,
            });
        }
        next.aggregate = Some(AggregateAnchor {
            method: aggregate.method,
            parts,
        });
    } else {
        next.matrix_id =
            remap_anchor_target(correspondences, &anchor.matrix_id, to_mirror, layout_id)?;
    }
    Ok(next)
}

/// Ticket 21: remap an anchor target through matrix correspondences without
/// silently retargeting. `to_mirror` selects source→mirror (true) or
/// mirror→source (false); a missing counterpart refuses naming the layout.
fn remap_anchor_target(
    correspondences: &[MirroredCorrespondence],
    anchor_matrix_id: &str,
    to_mirror: bool,
    layout_id: &str,
) -> Result<String, String> {
    correspondences
        .iter()
        .find(|record| {
            record.kind == MirroredItemKind::Matrix
                && (if to_mirror {
                    record.source_id == anchor_matrix_id
                } else {
                    record.mirror_id == anchor_matrix_id
                })
        })
        .map(|record| {
            if to_mirror {
                record.mirror_id.clone()
            } else {
                record.source_id.clone()
            }
        })
        .ok_or_else(|| {
            format!(
                "Linked counterpart of anchor target {:?} is missing in layout {:?}; name the missing counterpart instead of silently retargeting",
                anchor_matrix_id, layout_id
            )
        })
}

pub(crate) fn sync(doc: &mut ProjectDoc, matrix_id: &str) -> Result<Vec<String>, String> {
    let Some((target_id, axis_x)) = partner(doc, matrix_id) else {
        return Ok(vec![]);
    };
    // Ticket 21: an independent (Local) or absent (Omitted) edited matrix
    // never authors its partner; its own record already holds its state.
    // A Local partner keeps its own dimensions/layout on source resize.
    // An Omitted partner stays live instead: its record follows the source
    // (no parts are ever materialized for it) so anchors through it keep a
    // live logical pose. Neither state blocks the source edit.
    if matches!(
        exception_state(doc, MirroredItemKind::Matrix, matrix_id),
        Some(MirroredExceptionState::Local | MirroredExceptionState::Omitted)
    ) {
        return Ok(vec![]);
    }
    if exception_state(doc, MirroredItemKind::Matrix, &target_id)
        == Some(MirroredExceptionState::Local)
    {
        return Ok(vec![]);
    }
    let target_omitted = exception_state(doc, MirroredItemKind::Matrix, &target_id)
        == Some(MirroredExceptionState::Omitted);
    // Ticket 21: a record-absent Omitted member stays absent; its logical
    // matrix synthesizes from the source during evaluation.
    if target_omitted && !doc.matrices.iter().any(|matrix| matrix.id == target_id) {
        return Ok(vec![]);
    }
    let source = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)
        .ok_or("Linked matrix is missing")?;
    let target = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == target_id)
        .ok_or("Linked matrix is missing")?;
    let source_is_target = doc
        .layouts
        .iter()
        .find(|layout| layout.matrix_ids.iter().any(|id| id == matrix_id))
        .is_some_and(|layout| layout.mirror_link.is_some());
    let sizes: BTreeMap<_, _> = matrix::cell_members(source)
        .into_iter()
        .filter_map(|(cell, id)| {
            doc.parts
                .iter()
                .find(|part| part.id == id)
                .and_then(|part| part.keycap)
                .map(|size| (cell, size))
        })
        .collect();
    let mapping = axis_mapping(axis_x);
    let half_frame = pair_gap(doc, matrix_id).is_some();
    let mut reflected_matrix = reflected(source, target, mapping, false, half_frame)?;
    // Ticket 20: map anchor targets by identity, not list position or the
    // counterpart's stale anchor. Row/column stay the physical key; Follow
    // keeps its subset with source-world transport in the kernel.
    // Ticket 31: aggregate parts map one by one through the same
    // correspondence; the mirror resolves the whole candidate, never a
    // per-part reflection.
    // Ticket 21: an Omitted partner follows source re-anchors when the
    // counterpart exists, keeps its last valid anchor when it does not, and
    // never blocks the source edit while absent; Rejoin restores the
    // current source anchor anyway. `reflected` already preserved the old
    // anchor identity; only a successful remap overwrites it here.
    if let Some(anchor) = &source.anchor {
        let edited_layout = doc
            .layouts
            .iter()
            .find(|layout| layout.matrix_ids.iter().any(|id| id == &source.id))
            .map(|layout| layout.id.clone());
        let target_layout = doc
            .layouts
            .iter()
            .find(|layout| layout.matrix_ids.iter().any(|id| id == &target.id))
            .map(|layout| layout.id.clone());
        if let (Some(edited_layout), Some(target_layout)) = (edited_layout, target_layout) {
            let mut mapped: Option<MatrixAnchor> = None;
            for layout in &doc.layouts {
                let Some(link) = &layout.mirror_link else {
                    continue;
                };
                let edited_is_source =
                    link.source_id == edited_layout && layout.id == target_layout;
                let edited_is_mirror =
                    layout.id == edited_layout && link.source_id == target_layout;
                if !edited_is_source && !edited_is_mirror {
                    continue;
                }
                // Forward wins with a reverse fallback (a self-mirror cannot
                // pass `validate`); the last matching layout still wins.
                if edited_is_source {
                    if let Ok(next) =
                        remap_anchor(&link.correspondences, anchor, true, &target_layout)
                    {
                        mapped = Some(next);
                        continue;
                    }
                    if !edited_is_mirror {
                        continue;
                    }
                }
                if edited_is_mirror
                    && let Ok(next) =
                        remap_anchor(&link.correspondences, anchor, false, &target_layout)
                {
                    mapped = Some(next);
                }
            }
            if let Some(mapped) = mapped {
                if let Some(result_anchor) = reflected_matrix.anchor.as_mut() {
                    result_anchor.matrix_id = mapped.matrix_id;
                    result_anchor.row = mapped.row;
                    result_anchor.column = mapped.column;
                    result_anchor.aggregate = mapped.aggregate;
                }
            } else if target_omitted {
                // No counterpart while absent: keep the last valid anchor
                // (or none) instead of a cross-layout pointer. Rejoin
                // restores the current source anchor and revalidates.
                reflected_matrix.anchor = target.anchor.clone();
            } else {
                return Err(format!(
                    "Linked counterpart of anchor target {:?} is missing in layout {:?}; name the missing counterpart instead of silently retargeting",
                    anchor.matrix_id, target_layout
                ));
            }
        }
    }
    if target_omitted {
        // An absent record must never carry an anchor it cannot resolve:
        // compaction may have diverged its slots from the source side.
        // Keep the newest valid anchor (or none); Rejoin restores the
        // current source anchor and revalidates.
        let valid = |anchor: &MatrixAnchor| {
            matrix::evaluation::logical_matrix(&anchor.matrix_id, doc)
                .ok()
                .is_some_and(|target| anchor.row < target.rows && anchor.column < target.columns)
        };
        if !reflected_matrix.anchor.as_ref().is_some_and(&valid) {
            reflected_matrix.anchor = target.anchor.clone().filter(|anchor| valid(anchor));
        }
    }
    // Ticket 20: Local final-pose edits remain local. Independent key
    // offset/Key rotation stay on their own half; carried Shift/Bend and
    // layout records still sync. Preserve either side's Local slot, defaulting
    // to no independent pose when the counterpart has no explicit record.
    {
        let target_cells: BTreeMap<_, _> = target
            .cells
            .iter()
            .map(|cell| ((cell.row, cell.column), cell))
            .collect();
        for cell in &mut reflected_matrix.cells {
            let source_part = crate::matrix::member_id(&source.id, cell.row, cell.column);
            let target_part = crate::matrix::member_id(&target.id, cell.row, cell.column);
            let local = exception_state(doc, MirroredItemKind::Part, &source_part)
                == Some(MirroredExceptionState::Local)
                || exception_state(doc, MirroredItemKind::Part, &target_part)
                    == Some(MirroredExceptionState::Local);
            if local {
                if let Some(previous) = target_cells.get(&(cell.row, cell.column)) {
                    cell.offset = previous.offset;
                    cell.rotation = previous.rotation;
                } else {
                    cell.offset = None;
                    cell.rotation = None;
                }
            }
        }
    }
    if source_is_target {
        let canonical_cells: BTreeMap<_, _> = target
            .cells
            .iter()
            .map(|cell| ((cell.row, cell.column), cell))
            .collect();
        for cell in &mut reflected_matrix.cells {
            if let Some(canonical) = canonical_cells.get(&(cell.row, cell.column))
                && cell.assemblies_local != Some(true)
            {
                cell.definition_id = canonical.definition_id.clone();
                cell.variant = canonical.variant.clone();
                cell.assemblies = canonical.assemblies.clone();
                cell.assemblies_local = canonical.assemblies_local;
            }
        }
    }
    let mut changed = matrix::set_matrix(doc, &reflected_matrix)?;
    let target = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == target_id)
        .ok_or("Linked matrix is missing")?;
    for (cell, id) in matrix::cell_members(target) {
        if let Some(size) = sizes.get(&cell)
            && let Some(part) = doc.parts.iter_mut().find(|part| part.id == id)
        {
            part.keycap = Some(*size);
        }
    }
    if source_is_target {
        let canonical = doc
            .matrices
            .iter()
            .find(|matrix| matrix.id == target_id)
            .ok_or("Linked matrix is missing")?;
        let mut local = doc
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .ok_or("Linked matrix is missing")?
            .clone();
        let canonical_cells: BTreeMap<_, _> = canonical
            .cells
            .iter()
            .map(|cell| ((cell.row, cell.column), cell))
            .collect();
        for cell in &mut local.cells {
            if cell.assemblies_local == Some(true) {
                continue;
            }
            if let Some(canonical) = canonical_cells.get(&(cell.row, cell.column)) {
                cell.assemblies = canonical
                    .assemblies
                    .iter()
                    .cloned()
                    .map(reflected_companion)
                    .collect();
            } else {
                cell.assemblies.clear();
            }
            cell.assemblies_local = None;
        }
        changed.extend(matrix::set_matrix(doc, &local)?);
    }
    Ok(changed)
}

pub(crate) fn resolve(doc: &mut ProjectDoc) -> Result<(), String> {
    validate(doc)?;
    for matrix in &doc.matrices {
        matrix::valid_projection_matrix(matrix)?;
        matrix::evaluated(matrix, doc)?;
    }
    let sources: Vec<_> = doc
        .layouts
        .iter()
        .filter_map(|layout| {
            let link = layout.mirror_link.as_ref()?;
            doc.layouts
                .iter()
                .find(|source| source.id == link.source_id)
                .map(|source| source.matrix_ids.clone())
        })
        .collect();
    for source in sources.into_iter().flatten() {
        sync(doc, &source)?;
    }
    refresh_existing(doc)?;
    sync_components(doc)?;
    Ok(())
}

pub(crate) fn refresh_existing(doc: &mut ProjectDoc) -> Result<Vec<String>, String> {
    matrix::placement::carrying_authored_companions(
        doc,
        matrix::placement::Reported::PreserveEditReport,
        |doc| {
            for index in matrix::evaluation::order(doc)? {
                let matrix = &doc.matrices[index];
                let evaluated = matrix::evaluated(matrix, doc)?;
                let mut poses = BTreeMap::new();
                for geometry in &evaluated.cells {
                    let id =
                        matrix::member_id(&matrix.id, geometry.row as u32, geometry.column as u32);
                    if exception_state(doc, MirroredItemKind::Part, &id)
                        == Some(MirroredExceptionState::Local)
                    {
                        continue;
                    }
                    poses.insert(id.clone(), geometry.pose);
                    if let Some(cell) = matrix.cells.iter().find(|cell| {
                        cell.row == geometry.row as u32 && cell.column == geometry.column as u32
                    }) {
                        for assembly in &cell.assemblies {
                            let id = matrix::companion_id(&id, &assembly.id);
                            if matches!(companion_mode(doc, &id), MatrixCompanionMode::Follows) {
                                poses.insert(id, matrix::companion_pose(geometry.pose, assembly));
                            }
                        }
                    }
                }
                for part in &mut doc.parts {
                    if let Some(pose) = poses.get(&part.id)
                        && !part.properties.as_ref().is_some_and(|properties| {
                            properties.get(matrix::OVERRIDE) == Some(&serde_json::json!(1))
                        })
                    {
                        part.pose = *pose;
                    }
                }
            }
            Ok(vec![])
        },
    )
}

pub(crate) fn exception_state(
    doc: &ProjectDoc,
    kind: MirroredItemKind,
    id: &str,
) -> Option<MirroredExceptionState> {
    doc.layouts
        .iter()
        .filter_map(|layout| layout.mirror_link.as_ref())
        .flat_map(|link| &link.exceptions)
        .chain(
            doc.hardware
                .iter()
                .flat_map(|hardware| &hardware.pairs)
                .filter_map(|pair| pair.halves.as_ref())
                .flat_map(|halves| &halves.exceptions),
        )
        .find(|exception| exception.kind == kind && exception.mirror_id == id)
        .map(|exception| exception.state)
}

pub(crate) fn omitted(doc: &ProjectDoc, matrix_id: &str, key_id: &str) -> bool {
    // Native reconciliation owns its staged membership plan; filter same-board overlays here.
    let state = |kind, id: &str| {
        doc.layouts
            .iter()
            .filter_map(|layout| layout.mirror_link.as_ref())
            .flat_map(|link| &link.exceptions)
            .find(|exception| exception.kind == kind && exception.mirror_id == id)
            .map(|exception| exception.state)
    };
    state(MirroredItemKind::Part, key_id).or_else(|| state(MirroredItemKind::Matrix, matrix_id))
        == Some(MirroredExceptionState::Omitted)
}

pub(crate) fn sync_components(doc: &mut ProjectDoc) -> Result<Vec<String>, String> {
    let pairs: Vec<_> = doc
        .layouts
        .iter()
        .filter_map(|target| {
            let link = target.mirror_link.as_ref()?;
            let source = doc
                .layouts
                .iter()
                .find(|layout| layout.id == link.source_id)?;
            Some((source.id.clone(), target.id.clone(), link.axis_x))
        })
        .collect();
    let mut changed = Vec::new();
    for (source_layout_id, target_layout_id, axis_x) in pairs {
        adopt_unassigned_components(doc, &source_layout_id, axis_x, &mut changed)?;
        let source_ids = doc
            .layouts
            .iter()
            .find(|layout| layout.id == source_layout_id)
            .ok_or("Linked component source layout is missing")?
            .part_ids
            .clone();
        let target_layout_parts = doc
            .layouts
            .iter()
            .find(|layout| layout.id == target_layout_id)
            .ok_or("Linked component target layout is missing")?
            .part_ids
            .clone();

        let orphaned: Vec<_> = target_layout_parts
            .iter()
            .filter_map(|target_id| {
                let target = doc.parts.iter().find(|part| &part.id == target_id)?;
                let source_id = target
                    .properties
                    .as_ref()?
                    .get(MIRRORED_COMPONENT_SOURCE)?
                    .as_str()?;
                (!source_ids.iter().any(|id| id == source_id)).then(|| {
                    (
                        target_id.clone(),
                        source_id.to_string(),
                        doc.parts.iter().any(|part| part.id == source_id),
                    )
                })
            })
            .collect();
        for (target_id, source_id, source_exists) in orphaned {
            if source_exists {
                if let Some(part) = doc.parts.iter_mut().find(|part| part.id == target_id) {
                    clear_component_link(part);
                    changed.push(target_id);
                }
            } else {
                remove_component(doc, &target_id);
                changed.extend([target_id, source_id]);
            }
        }

        for source_id in source_ids {
            let Some(mut source) = doc.parts.iter().find(|part| part.id == source_id).cloned()
            else {
                continue;
            };
            let target_id = doc.parts.iter().find_map(|target| {
                (target
                    .properties
                    .as_ref()
                    .and_then(|properties| properties.get(MIRRORED_COMPONENT_SOURCE))
                    .and_then(serde_json::Value::as_str)
                    == Some(source_id.as_str()))
                .then(|| target.id.clone())
            });
            if source
                .properties
                .as_ref()
                .and_then(|properties| properties.get(MIRRORED_COMPONENT_SUBSTITUTE))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            {
                changed.push(source_id.clone());
                changed.extend(target_id);
                continue;
            }
            if let Some(target_id) = target_id.as_ref().filter(|id| {
                doc.parts
                    .iter()
                    .find(|part| &part.id == *id)
                    .and_then(|part| part.properties.as_ref())
                    .and_then(|properties| properties.get(MIRRORED_COMPONENT_SUBSTITUTE))
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false)
            }) {
                changed.push(target_id.clone());
                continue;
            }

            let target_id = match target_id {
                Some(id) => id,
                None => {
                    let base = format!("{target_layout_id}::mirror::{source_id}");
                    if doc.parts.iter().any(|part| part.id == base) {
                        let mut suffix = 1;
                        while doc
                            .parts
                            .iter()
                            .any(|part| part.id == format!("{base}::{suffix}"))
                        {
                            suffix += 1;
                        }
                        format!("{base}::{suffix}")
                    } else {
                        base
                    }
                }
            };
            // One rule for every mirrored part heading: controllers keep the plain reflection
            // so both halves share the world rotation; the axis map is `forward_angle = -r`
            // with `mapping.rotation == 0`.
            let is_controller = doc.definitions.iter().any(|definition| {
                definition.id == source.definition_id && definition.kind == PartKind::Controller
            });
            let axis_mapping = crate::model::Pose2 {
                at: Vec2 {
                    x: 2.0 * axis_x,
                    y: 0.0,
                },
                rotation: 0.0,
            };
            let mut target_pose = crate::model::Pose2 {
                at: Vec2 {
                    x: 2.0 * axis_x - source.pose.at.x,
                    y: source.pose.at.y,
                },
                rotation: pair::mirrored_part_heading(
                    is_controller,
                    axis_mapping,
                    source.pose.rotation,
                ),
            };
            let target_exists = doc.parts.iter().any(|part| part.id == target_id);
            if let Some(target) = doc
                .parts
                .iter()
                .find(|part| part.id == target_id)
                .filter(|_| {
                    doc.constraint_driving_part(&target_id)
                        .ok()
                        .flatten()
                        .is_some()
                })
            {
                let source_pose = crate::model::Pose2 {
                    at: Vec2 {
                        x: 2.0 * axis_x - target.pose.at.x,
                        y: target.pose.at.y,
                    },
                    rotation: pair::mirrored_part_heading(
                        is_controller,
                        axis_mapping,
                        target.pose.rotation,
                    ),
                };
                set_component_pose(doc, &source_id, source_pose)?;
                source = doc
                    .parts
                    .iter()
                    .find(|part| part.id == source_id)
                    .cloned()
                    .ok_or("Mirrored component source is missing")?;
                changed.push(source_id.clone());
                target_pose = crate::model::Pose2 {
                    at: Vec2 {
                        x: 2.0 * axis_x - source.pose.at.x,
                        y: source.pose.at.y,
                    },
                    rotation: pair::mirrored_part_heading(
                        is_controller,
                        axis_mapping,
                        source.pose.rotation,
                    ),
                };
            }

            let mut mirrored = source.clone();
            mirrored.id = target_id.clone();
            mirrored.reference = format!("{}_M", source.reference);
            mirrored.pose = target_pose;
            mirrored
                .properties
                .get_or_insert_with(BTreeMap::new)
                .insert(
                    MIRRORED_COMPONENT_SOURCE.into(),
                    serde_json::json!(source_id),
                );
            mirrored
                .properties
                .as_mut()
                .unwrap()
                .remove(MIRRORED_COMPONENT_SUBSTITUTE);
            if let Some(target) = doc.parts.iter_mut().find(|part| part.id == target_id) {
                *target = mirrored;
            } else {
                doc.parts.push(mirrored);
                if let Some(layout) = doc
                    .layouts
                    .iter_mut()
                    .find(|layout| layout.id == target_layout_id)
                {
                    layout.part_ids.push(target_id.clone());
                }
                let board_id = doc
                    .layouts
                    .iter()
                    .find(|layout| layout.id == target_layout_id)
                    .map(|layout| layout.board_id.clone())
                    .ok_or("Linked component target layout is missing")?;
                for board in &mut doc.boards {
                    if board.id == board_id && !board.part_ids.contains(&target_id) {
                        board.part_ids.push(target_id.clone());
                    }
                }
            }
            sync_component_nets(doc, &source_id, &target_id);
            for feature in &mut doc.outline {
                if let crate::model::OutlineFeature::PartEnvelope { part_ids, .. } = feature
                    && part_ids.contains(&source_id)
                    && !part_ids.contains(&target_id)
                {
                    part_ids.push(target_id.clone());
                }
            }
            changed.extend([source_id, target_id]);
            if !target_exists {
                changed.push(target_layout_id.clone());
            }
        }
    }
    Ok(changed)
}

pub(crate) fn move_linked_component(
    doc: &mut ProjectDoc,
    id: &str,
    at: Vec2,
) -> Result<Option<Vec<String>>, String> {
    let Some(target) = doc.parts.iter().find(|part| part.id == id) else {
        return Ok(None);
    };
    let Some(properties) = target.properties.as_ref() else {
        return Ok(None);
    };
    if properties
        .get(MIRRORED_COMPONENT_SUBSTITUTE)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(None);
    }
    let Some(source_id) = properties
        .get(MIRRORED_COMPONENT_SOURCE)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
    else {
        return Ok(None);
    };
    let target_id = target.id.clone();
    let Some(source_layout) = doc
        .layouts
        .iter()
        .find(|layout| layout.part_ids.contains(&source_id))
    else {
        return Ok(None);
    };
    let Some(target_layout) = doc.layouts.iter().find(|layout| {
        layout
            .mirror_link
            .as_ref()
            .is_some_and(|link| link.source_id == source_layout.id)
    }) else {
        return Ok(None);
    };
    let axis_x = target_layout
        .mirror_link
        .as_ref()
        .ok_or("Linked component target layout is missing")?
        .axis_x;
    let is_controller = doc.definitions.iter().any(|definition| {
        definition.id == target.definition_id && definition.kind == PartKind::Controller
    });
    let axis_mapping = crate::model::Pose2 {
        at: Vec2 {
            x: 2.0 * axis_x,
            y: 0.0,
        },
        rotation: 0.0,
    };
    let rotation = pair::mirrored_part_heading(is_controller, axis_mapping, target.pose.rotation);
    set_component_pose(
        doc,
        &source_id,
        crate::model::Pose2 {
            at: Vec2 {
                x: 2.0 * axis_x - at.x,
                y: at.y,
            },
            rotation,
        },
    )?;
    Ok(Some(vec![source_id, target_id]))
}

fn adopt_unassigned_components(
    doc: &mut ProjectDoc,
    source_layout_id: &str,
    axis_x: f64,
    changed: &mut Vec<String>,
) -> Result<(), String> {
    let source_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == source_layout_id)
        .ok_or("Linked component source layout is missing")?;
    let board_id = source_layout.board_id.clone();
    let key_positions: Vec<_> = doc
        .matrices
        .iter()
        .filter(|matrix| source_layout.matrix_ids.contains(&matrix.id))
        .flat_map(matrix::cell_members)
        .filter_map(|(_, id)| doc.parts.iter().find(|part| part.id == id))
        .map(|part| part.pose.at.x)
        .collect();
    if key_positions.is_empty() {
        return Ok(());
    }
    let source_center = key_positions.iter().sum::<f64>() / key_positions.len() as f64;
    if (source_center - axis_x).abs() <= f64::EPSILON {
        return Ok(());
    }
    let source_is_left = source_center < axis_x;

    let mut assigned: BTreeSet<_> = doc
        .layouts
        .iter()
        .flat_map(|layout| layout.part_ids.iter().cloned())
        .collect();
    for matrix in &doc.matrices {
        assigned.extend(matrix.part_ids.iter().cloned());
        assigned.extend(matrix::cell_members(matrix).into_values());
    }
    let board_parts = doc
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .ok_or("Linked component board is missing")?
        .part_ids
        .clone();
    let unassigned: Vec<_> = board_parts
        .into_iter()
        // Case hardware is resolved in board coordinates, independently of layout mirrors.
        .filter(|id| !assigned.contains(id) && !id.starts_with("case-closure/"))
        .filter_map(|id| {
            let part = doc.parts.iter().find(|part| part.id == id)?;
            let on_source_side = if source_is_left {
                part.pose.at.x < axis_x
            } else {
                part.pose.at.x > axis_x
            };
            on_source_side.then_some(id)
        })
        .collect();
    if unassigned.is_empty() {
        return Ok(());
    }
    if let Some(layout) = doc
        .layouts
        .iter_mut()
        .find(|layout| layout.id == source_layout_id)
    {
        for id in unassigned {
            if !layout.part_ids.contains(&id) {
                layout.part_ids.push(id.clone());
                changed.push(id);
            }
        }
    }
    Ok(())
}

fn set_component_pose(
    doc: &mut ProjectDoc,
    id: &str,
    pose: crate::model::Pose2,
) -> Result<(), String> {
    let current = doc
        .parts
        .iter()
        .find(|part| part.id == id)
        .ok_or("Mirrored component source is missing")?
        .pose;
    if let Some(constraint_id) = doc
        .constraint_driving_part(id)?
        .map(crate::model::Constraint::id)
        .map(str::to_owned)
    {
        let constraint = doc
            .constraints
            .iter_mut()
            .find(|constraint| constraint.id() == constraint_id)
            .ok_or_else(|| format!("Constraint {constraint_id} disappeared"))?;
        let delta = Vec2 {
            x: pose.at.x - current.at.x,
            y: pose.at.y - current.at.y,
        };
        let turn = Some(pose.rotation - current.rotation);
        if crate::constraints::retune_offset(&doc.parts, constraint, delta, turn)?
            == crate::constraints::Retune::Mirror
        {
            return Err(format!(
                "Cannot move mirrored component because source {id} is controlled by a mirror constraint"
            ));
        }
    }
    let part = doc
        .parts
        .iter_mut()
        .find(|part| part.id == id)
        .ok_or("Mirrored component source is missing")?;
    part.pose = pose;
    Ok(())
}

fn sync_component_nets(doc: &mut ProjectDoc, source_id: &str, target_id: &str) {
    for net in &mut doc.nets {
        let source_pins: Vec<_> = net
            .pins
            .iter()
            .filter(|pin| pin.part_id == source_id)
            .cloned()
            .collect();
        net.pins.retain(|pin| pin.part_id != target_id);
        net.pins.extend(source_pins.into_iter().map(|mut pin| {
            pin.part_id = target_id.to_string();
            pin
        }));
    }
}

fn clear_component_link(part: &mut Part) {
    if let Some(properties) = part.properties.as_mut() {
        properties.remove(MIRRORED_COMPONENT_SOURCE);
        properties.remove(MIRRORED_COMPONENT_SUBSTITUTE);
        if properties.is_empty() {
            part.properties = None;
        }
    }
}

pub(crate) fn unlink_components(doc: &mut ProjectDoc, layout_id: &str) -> Vec<String> {
    let Some((part_ids, matrix_ids)) = doc
        .layouts
        .iter()
        .find(|layout| layout.id == layout_id)
        .map(|layout| (layout.part_ids.clone(), layout.matrix_ids.clone()))
    else {
        return Vec::new();
    };
    let mut changed = Vec::new();
    for id in part_ids {
        if let Some(part) = doc.parts.iter_mut().find(|part| part.id == id) {
            clear_component_link(part);
            changed.push(id);
        }
    }
    for matrix in doc
        .matrices
        .iter_mut()
        .filter(|matrix| matrix_ids.contains(&matrix.id))
    {
        for cell in &mut matrix.cells {
            if cell.assemblies_local == Some(true) {
                cell.assemblies_local = None;
            }
        }
        changed.push(matrix.id.clone());
    }
    changed
}

fn remove_component(doc: &mut ProjectDoc, id: &str) {
    doc.parts.retain(|part| part.id != id);
    doc.constraints
        .retain(|constraint| constraint.source() != id && constraint.target() != id);
    for board in &mut doc.boards {
        board.part_ids.retain(|part_id| part_id != id);
    }
    for layout in &mut doc.layouts {
        layout.part_ids.retain(|part_id| part_id != id);
    }
    for matrix in &mut doc.matrices {
        matrix.part_ids.retain(|part_id| part_id != id);
    }
    for net in &mut doc.nets {
        net.pins.retain(|pin| pin.part_id != id);
    }
    for feature in &mut doc.outline {
        if let crate::model::OutlineFeature::PartEnvelope { part_ids, .. } = feature {
            part_ids.retain(|part_id| part_id != id);
        }
    }
}

pub(crate) fn mirrored_component_ids(doc: &ProjectDoc, requested: &[String]) -> Vec<String> {
    let mut ids: BTreeSet<_> = requested.iter().cloned().collect();
    for target in &doc.parts {
        let Some(source_id) = target
            .properties
            .as_ref()
            .and_then(|properties| properties.get(MIRRORED_COMPONENT_SOURCE))
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        if ids.contains(&target.id) || ids.contains(source_id) {
            ids.insert(target.id.clone());
            ids.insert(source_id.to_string());
        }
    }
    ids.into_iter().collect()
}

pub(crate) fn remove_matrix(doc: &mut ProjectDoc, matrix_id: &str) {
    let removed: BTreeSet<_> = doc
        .layouts
        .iter()
        .filter(|layout| layout.matrix_ids == [matrix_id])
        .map(|layout| layout.id.clone())
        .collect();
    let detached_parts: Vec<_> = doc
        .layouts
        .iter()
        .filter(|layout| {
            removed.contains(&layout.id)
                || layout
                    .mirror_link
                    .as_ref()
                    .is_some_and(|link| removed.contains(&link.source_id))
        })
        .flat_map(|layout| layout.part_ids.iter().cloned())
        .collect();
    doc.layouts.retain(|layout| !removed.contains(&layout.id));
    for layout in &mut doc.layouts {
        layout.matrix_ids.retain(|id| id != matrix_id);
    }
    for layout in &mut doc.layouts {
        if layout
            .mirror_link
            .as_ref()
            .is_some_and(|link| removed.contains(&link.source_id))
        {
            layout.mirror_link = None;
        }
    }
    for id in detached_parts {
        if let Some(part) = doc.parts.iter_mut().find(|part| part.id == id) {
            clear_component_link(part);
        }
    }
}

/// Whether `id` is a matrix key: the primary member of a cell, never a companion of one.
pub fn is_matrix_key(doc: &ProjectDoc, id: &str) -> bool {
    primary_member(doc, id).is_some()
}

fn primary_member<'a>(doc: &'a ProjectDoc, id: &str) -> Option<(&'a Matrix, u32, u32)> {
    for matrix in &doc.matrices {
        if !matrix.part_ids.iter().any(|part| part == id) {
            continue;
        }
        if let Some(((row, column), _)) = matrix::cell_members(matrix)
            .into_iter()
            .find(|(_, member)| member == id)
        {
            return Some((matrix, row, column));
        }
    }
    None
}

pub(crate) fn has_linked_positions(doc: &ProjectDoc, positions: &[Position]) -> bool {
    let mut affected: BTreeSet<_> = positions
        .iter()
        .map(|position| position.id.as_str())
        .collect();
    loop {
        let previous = affected.len();
        for id in affected.clone() {
            if let Some(constraint) = doc.constraint_driving_part(id).ok().flatten() {
                affected.insert(constraint.target());
            }
        }
        if affected.len() == previous {
            break;
        }
    }
    if affected.iter().any(|id| primary_member(doc, id).is_some()) {
        return true;
    }
    positions.iter().any(|position| {
        primary_member(doc, &position.id).is_some()
            || doc.parts.iter().any(|part| {
                part.id == position.id
                    && part.properties.as_ref().is_some_and(|properties| {
                        properties.contains_key(MIRRORED_COMPONENT_SOURCE)
                    })
            })
            || doc.parts.iter().any(|target| {
                target
                    .properties
                    .as_ref()
                    .and_then(|properties| properties.get(MIRRORED_COMPONENT_SOURCE))
                    .and_then(serde_json::Value::as_str)
                    == Some(position.id.as_str())
            })
            || doc
                .constraint_driving_part(&position.id)
                .ok()
                .flatten()
                .is_some_and(|constraint| constraint.offset_axes().is_some())
    })
}

/// Numeric moves and keyboard nudges must update the same parametric geometry
/// as pointer edits; absolute part overrides would break the next linked edit.
pub(crate) fn move_keys(
    doc: &mut ProjectDoc,
    positions: &[Position],
) -> Result<(Vec<String>, BTreeSet<String>), String> {
    let before: BTreeMap<_, _> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let mut matrices = BTreeMap::new();
    let mut handled = BTreeSet::new();
    let mut unchanged = BTreeSet::new();
    let mut carried = Vec::new();
    let mut constrained = Vec::new();
    for position in positions {
        let Some((source, row, column)) = primary_member(doc, &position.id) else {
            continue;
        };
        let part = doc
            .parts
            .iter()
            .find(|part| part.id == position.id)
            .ok_or("Key is missing")?;
        if part.locked == Some(true) {
            return Err(format!("Part {} is locked", part.id));
        }
        if !position.at.x.is_finite() || !position.at.y.is_finite() {
            return Err("Position must be finite".into());
        }
        if position.at == part.pose.at {
            handled.insert(position.id.clone());
            unchanged.insert(position.id.clone());
            continue;
        }
        if let Some(constraint) = doc.constraint_driving_part(&position.id)? {
            if constraint.is_mirror() {
                return Err(crate::constraints::mirror_drag_refusal(&position.id));
            }
            let mut pose = part.pose;
            pose.at = position.at;
            constrained.push((position.id.clone(), pose));
        }
        let matrix = matrices
            .entry(source.id.clone())
            .or_insert_with(|| source.clone());
        let displacement = Vec2 {
            x: position.at.x - part.pose.at.x,
            y: position.at.y - part.pose.at.y,
        };
        let cell = source
            .cells
            .iter()
            .find(|cell| cell.row == row && cell.column == column);
        if let Some(cell) = cell {
            for assembly in &cell.assemblies {
                let id = matrix::companion_id(&position.id, &assembly.id);
                if positions.iter().any(|position| position.id == id)
                    || doc.constraint_driving_part(&id).ok().flatten().is_some()
                {
                    continue;
                }
                if let Some(companion) = doc.parts.iter().find(|part| part.id == id)
                    && companion.properties.as_ref().is_some_and(|properties| {
                        properties.get(matrix::OVERRIDE) == Some(&serde_json::json!(1))
                    })
                {
                    let mut pose = companion.pose;
                    pose.at.x += displacement.x;
                    pose.at.y += displacement.y;
                    carried.push((id, pose));
                }
            }
        }
        // Older absolute key moves are absorbed into the cell's parametric offset.
        let evaluated = matrix::evaluated(source, doc)?;
        let generated = matrix::evaluation::cell_pose(&evaluated, row, column)?.at;
        let delta = Vec2 {
            x: position.at.x - generated.x,
            y: position.at.y - generated.y,
        };
        let frame = &evaluated
            .cells
            .iter()
            .find(|c| c.row == row as usize && c.column == column as usize)
            .ok_or("Key frame is missing")?
            .offset_frame;
        let local = matrix::evaluation::inverse_offset(frame, delta);
        let index = matrix
            .cells
            .iter()
            .position(|cell| cell.row == row && cell.column == column)
            .unwrap_or_else(|| {
                matrix.cells.push(MatrixCell {
                    shift: None,
                    bend: None,
                    deleted: false,
                    row,
                    column,
                    enabled: true,

                    definition_id: None,
                    variant: None,
                    offset: None,
                    rotation: None,
                    assemblies: vec![],
                    assemblies_local: None,
                });
                matrix.cells.len() - 1
            });
        let cell = &mut matrix.cells[index];
        let offset = cell.offset.unwrap_or_default();
        cell.offset = Some(Vec2 {
            x: offset.x + local.x,
            y: offset.y + local.y,
        });
        handled.insert(position.id.clone());
    }
    // Editing both sides at once would give two competing values for one link.
    if matrices
        .keys()
        .any(|id| partner(doc, id).is_some_and(|(other, _)| matrices.contains_key(&other)))
    {
        return Err("Move one linked half at a time".into());
    }
    let mut changed = vec![];
    for part in &mut doc.parts {
        if handled.contains(&part.id)
            && !unchanged.contains(&part.id)
            && let Some(properties) = &mut part.properties
        {
            properties.remove(matrix::OVERRIDE);
        }
    }
    for (id, pose) in constrained.into_iter().chain(carried) {
        set_component_pose(doc, &id, pose)?;
        changed.push(id);
    }
    for matrix in matrices.values() {
        changed.extend(matrix::set_matrix(doc, matrix)?);
        changed.extend(sync(doc, &matrix.id)?);
    }
    changed.retain(|id| {
        before.get(id).is_none_or(|pose| {
            doc.parts
                .iter()
                .find(|part| &part.id == id)
                .is_none_or(|part| part.pose != *pose)
        })
    });
    Ok((changed, handled))
}

pub(crate) struct AssemblyPlacement {
    pub(crate) parent_id: String,
    pub(crate) local_pose: Pose2,
    pub(crate) authored_pose: Option<Pose2>,
    pub(crate) mode: MatrixCompanionMode,
}

/// Classify a generated companion the same way the solver places it: an explicit
/// constraint wins, then an authored pose, otherwise the companion follows its
/// key. The placement solver and the scene projection both read this mode, so the
/// precedence rule has one source.
pub(crate) fn companion_mode(doc: &ProjectDoc, companion_id: &str) -> MatrixCompanionMode {
    if let Some(constraint) = doc.constraint_driving_part(companion_id).ok().flatten() {
        return MatrixCompanionMode::Constrained {
            constraint_id: constraint.id().to_string(),
            source_part_id: constraint.source().to_string(),
        };
    }
    let authored = doc
        .parts
        .iter()
        .find(|part| part.id == companion_id)
        .is_some_and(|part| {
            part.properties.as_ref().is_some_and(|properties| {
                properties.get(matrix::OVERRIDE) == Some(&serde_json::json!(1))
            })
        });
    if authored {
        MatrixCompanionMode::Authored
    } else {
        MatrixCompanionMode::Follows
    }
}

pub(crate) fn assembly_placements(
    doc: &ProjectDoc,
    explicit: &[Position],
) -> BTreeMap<String, AssemblyPlacement> {
    let parts: BTreeMap<_, _> = doc
        .parts
        .iter()
        .map(|part| (part.id.as_str(), part))
        .collect();
    let mut placements = BTreeMap::new();
    for matrix in &doc.matrices {
        for cell in &matrix.cells {
            if !cell.enabled || cell.deleted {
                continue;
            }
            let parent_id = matrix::member_id(&matrix.id, cell.row, cell.column);
            let Some(parent) = parts.get(parent_id.as_str()) else {
                continue;
            };
            for assembly in &cell.assemblies {
                let id = matrix::companion_id(&parent_id, &assembly.id);
                if explicit.iter().any(|position| position.id == id) {
                    continue;
                }
                let Some(companion) = parts.get(id.as_str()) else {
                    continue;
                };
                let mode = companion_mode(doc, &id);
                let authored = matches!(mode, MatrixCompanionMode::Authored);
                let local_pose = if authored {
                    let (sin, cos) = parent.pose.rotation.to_radians().sin_cos();
                    let dx = companion.pose.at.x - parent.pose.at.x;
                    let dy = companion.pose.at.y - parent.pose.at.y;
                    Pose2 {
                        at: Vec2 {
                            x: dx * cos + dy * sin,
                            y: -dx * sin + dy * cos,
                        },
                        rotation: companion.pose.rotation - parent.pose.rotation,
                    }
                } else {
                    Pose2 {
                        at: assembly.offset,
                        rotation: assembly.rotation.unwrap_or(0.0),
                    }
                };
                placements.insert(
                    id,
                    AssemblyPlacement {
                        parent_id: parent_id.clone(),
                        local_pose,
                        authored_pose: authored.then_some(companion.pose),
                        mode,
                    },
                );
            }
        }
    }
    placements
}

pub(crate) fn reconcile_constraint_keys(
    doc: &mut ProjectDoc,
    moves: &BTreeMap<String, (Pose2, Pose2)>,
    solved: &BTreeMap<String, Pose2>,
) -> Result<Vec<String>, String> {
    let mut matrices = BTreeMap::new();
    for (id, (_, next)) in moves {
        let Some((source, row, column)) = primary_member(doc, id) else {
            continue;
        };
        let evaluated = matrix::evaluated(source, doc)?;
        let geometry = evaluated
            .cells
            .iter()
            .find(|c| c.row == row as usize && c.column == column as usize)
            .ok_or("Key frame is missing")?;
        let local = matrix::evaluation::inverse_offset(
            &geometry.offset_frame,
            Vec2 {
                x: next.at.x - geometry.pose.at.x,
                y: next.at.y - geometry.pose.at.y,
            },
        );
        let matrix = matrices
            .entry(source.id.clone())
            .or_insert_with(|| source.clone());
        let index = matrix
            .cells
            .iter()
            .position(|cell| cell.row == row && cell.column == column)
            .unwrap_or_else(|| {
                matrix.cells.push(MatrixCell {
                    shift: None,
                    bend: None,
                    deleted: false,
                    row,
                    column,
                    enabled: true,
                    definition_id: None,
                    variant: None,
                    offset: None,
                    rotation: None,
                    assemblies: vec![],
                    assemblies_local: None,
                });
                matrix.cells.len() - 1
            });
        let cell = &mut matrix.cells[index];
        let offset = cell.offset.unwrap_or_default();
        cell.offset = Some(Vec2 {
            x: offset.x + local.x,
            y: offset.y + local.y,
        });
        let sign = if matches!(source.mirror, Some(Mirror::X | Mirror::Y)) {
            -1.0
        } else {
            1.0
        };
        cell.rotation = Some(
            cell.rotation.unwrap_or_default() + sign * (next.rotation - geometry.pose.rotation),
        );
    }
    let mut changed = Vec::new();
    for matrix in matrices.values() {
        changed.extend(matrix::set_matrix(doc, matrix)?);
        changed.extend(sync(doc, &matrix.id)?);
    }
    // The combined dependency solver owns final poses, including independently
    // constrained and explicitly positioned companions overwritten by generation.
    for part in &mut doc.parts {
        if let Some(pose) = solved.get(&part.id) {
            part.pose = *pose;
        }
    }
    Ok(changed)
}

/// Ticket 21: same-board exception lifecycle for matrix/key overlays.
///
/// Runs on the accepted layout after it replaces the current one, before
/// `resolve()`. Added Omitted matrices shed their inherited parts here so
/// the scene delta names them; resolve keeps the live record in sync.
/// Making an anchored matrix Local detaches its own anchor keeping its
/// pose. Rejoin and Omitted-to-Local materialize parts from the live
/// record; resolve then adopts the current source layout and anchor. Local
/// keys returning to a shared state reset their independent offset and
/// rotation to the followed cell. Explicit child exceptions are otherwise
/// preserved untouched.
pub(crate) fn apply_exception_transitions(
    doc: &mut ProjectDoc,
    previous: &Layout,
    incoming: &Layout,
) -> Result<Vec<String>, String> {
    let Some(next_link) = incoming.mirror_link.as_ref() else {
        return Ok(vec![]);
    };
    let prev_link = previous.mirror_link.as_ref();
    if prev_link.is_some_and(|link| link.source_id != next_link.source_id) {
        return Ok(vec![]);
    }
    let prev_state = |kind: MirroredItemKind, mirror_id: &str| {
        prev_link
            .and_then(|link| {
                link.exceptions
                    .iter()
                    .find(|exception| exception.kind == kind && exception.mirror_id == mirror_id)
            })
            .map(|exception| exception.state)
    };
    let mut changed = Vec::new();
    // Matrix records and parts first, so key resets below see live cells.
    let matrix_ids: BTreeSet<&str> = incoming
        .matrix_ids
        .iter()
        .chain(previous.matrix_ids.iter())
        .map(String::as_str)
        .collect();
    for mirror_id in matrix_ids {
        let before = prev_state(MirroredItemKind::Matrix, mirror_id);
        let after = next_link
            .exceptions
            .iter()
            .find(|exception| {
                exception.kind == MirroredItemKind::Matrix && exception.mirror_id == mirror_id
            })
            .map(|exception| exception.state);
        if before == after {
            continue;
        }
        let record = doc
            .matrices
            .iter()
            .find(|matrix| matrix.id == mirror_id)
            .cloned();
        match after {
            // Making an anchored matrix Local detaches its own anchor and
            // keeps its pose; a Local record never authors its partner, so
            // the paired sync inside detach is a no-op for it.
            Some(MirroredExceptionState::Local)
                if record
                    .as_ref()
                    .is_some_and(|matrix| matrix.anchor.is_some()) =>
            {
                changed.extend(matrix::anchors::detach(doc, mirror_id)?);
            }
            // An absent matrix sheds (or keeps shed) its inherited parts
            // now so the scene delta names them; resolve keeps the live
            // record following the source.
            Some(MirroredExceptionState::Omitted) => {
                if let Some(record) = record {
                    changed.extend(matrix::set_matrix(doc, &record)?);
                }
            }
            // Omitted-to-Local and Rejoin materialize parts from the live
            // record now; resolve adopts the current source layout/anchor.
            // A record-absent Omitted member synthesizes from its source.
            Some(MirroredExceptionState::Local) | None if before.is_some() => {
                let record = match record {
                    Some(record) => record,
                    None if before == Some(MirroredExceptionState::Omitted) => {
                        synthesize_rejoin_record(doc, incoming, mirror_id)?
                    }
                    None => return Err(format!("Linked matrix {mirror_id} is missing")),
                };
                changed.extend(matrix::set_matrix(doc, &record)?);
            }
            _ => {}
        }
    }
    // Key overlays: an Omitted key needs its correspondence overlay, and a
    // key leaving Local returns to the followed cell instead of freezing
    // its former Local placement.
    let mut part_transitions: Vec<(String, Option<MirroredExceptionState>)> = next_link
        .exceptions
        .iter()
        .filter(|exception| exception.kind == MirroredItemKind::Part)
        .map(|exception| (exception.mirror_id.clone(), Some(exception.state)))
        .collect();
    if let Some(prev_link) = prev_link {
        for exception in &prev_link.exceptions {
            if exception.kind != MirroredItemKind::Part
                || next_link.exceptions.iter().any(|next| {
                    next.kind == MirroredItemKind::Part && next.mirror_id == exception.mirror_id
                })
            {
                continue;
            }
            // A removed overlay rejoins the shared layout.
            part_transitions.push((exception.mirror_id.clone(), None));
        }
    }
    for (mirror_id, after) in part_transitions {
        let before = prev_state(MirroredItemKind::Part, &mirror_id);
        if before == after {
            continue;
        }
        if after == Some(MirroredExceptionState::Omitted) {
            ensure_part_correspondence(doc, &incoming.id, &mirror_id)?;
        }
        if before == Some(MirroredExceptionState::Local) && after != before {
            reset_key_to_followed(doc, &incoming.id, &mirror_id)?;
        }
        // Shed the part when the key becomes absent; materialize it when
        // the key returns from an excepted state. `set_matrix` reports the
        // shed/created part, so the scene delta names it; resolve converges
        // idempotently afterward.
        let shed = after == Some(MirroredExceptionState::Omitted);
        let materialize = before.is_some() && !shed;
        if (shed || materialize)
            && let Some(owner) = owning_matrix(doc, &incoming.id, &mirror_id)
        {
            let record = doc
                .matrices
                .iter()
                .find(|matrix| matrix.id == owner)
                .cloned();
            if let Some(record) = record {
                changed.extend(matrix::set_matrix(doc, &record)?);
            }
        }
    }
    changed.sort();
    changed.dedup();
    Ok(changed)
}

/// Follow-deleted mirror matrices and their member parts for one removal,
/// plus retired absent (Omitted) records that leave no parts behind.
#[derive(Default)]
pub(crate) struct LinkedRemovalFollow {
    pub matrices: Vec<String>,
    pub part_ids: Vec<String>,
    pub retired: Vec<String>,
}

/// Ticket 21: source-removal lifecycle for linked matrix pairs. A Local
/// partner survives complete with its dimensions, layout and parts; only
/// the defunct pair records retire. A linked partner follows the deletion;
/// an Omitted partner stays absent and its record retires. Dependants of
/// retired records detach keeping their frame (an asymmetric detach still
/// refuses naming its Local exception). Unrelated pairs, surviving
/// ownership and anchors are preserved; no replacement is joined.
pub(crate) fn plan_linked_matrix_removal(
    doc: &mut ProjectDoc,
    matrix_id: &str,
    detached: &mut Vec<String>,
) -> Result<LinkedRemovalFollow, String> {
    let pairs: Vec<(String, String)> = doc
        .layouts
        .iter()
        .filter_map(|layout| layout.mirror_link.as_ref())
        .flat_map(|link| link.correspondences.iter())
        .filter(|record| {
            record.kind == MirroredItemKind::Matrix
                && (record.source_id == matrix_id || record.mirror_id == matrix_id)
        })
        .map(|record| (record.source_id.clone(), record.mirror_id.clone()))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut follow = LinkedRemovalFollow::default();
    let mut retired_members: Vec<String> = vec![matrix_id.to_string()];
    for (source_id, mirror_id) in &pairs {
        let removed_is_source = source_id == matrix_id;
        let partner_id = if removed_is_source {
            mirror_id.clone()
        } else {
            source_id.clone()
        };
        if removed_is_source {
            match exception_state(doc, MirroredItemKind::Matrix, &partner_id) {
                // The Local partner survives complete; only the defunct
                // pair records retire below.
                Some(MirroredExceptionState::Local) => {}
                // The absent partner stays absent: detach its dependants
                // keeping their frame, then retire its record.
                Some(MirroredExceptionState::Omitted) => {
                    detached.extend(matrix::anchors::detach_dependants(doc, &partner_id)?);
                    doc.matrices.retain(|matrix| matrix.id != partner_id);
                    follow.retired.push(partner_id.clone());
                    retired_members.push(partner_id.clone());
                }
                // A linked partner follows the deletion with its parts.
                None => {
                    detached.extend(matrix::anchors::detach_dependants(doc, &partner_id)?);
                    if let Some(record) = doc.matrices.iter().find(|matrix| matrix.id == partner_id)
                    {
                        follow.part_ids.extend(record.part_ids.iter().cloned());
                    }
                    doc.matrices.retain(|matrix| matrix.id != partner_id);
                    follow.matrices.push(partner_id.clone());
                    retired_members.push(partner_id.clone());
                }
            }
        }
        retire_pair_records(doc, source_id, mirror_id, &retired_members);
    }
    Ok(follow)
}

/// Retire the matrix/part correspondences and member exceptions for one
/// matrix pair, limited to members of removed records. Surviving members
/// keep their exceptions and ownership.
fn retire_pair_records(
    doc: &mut ProjectDoc,
    source_id: &str,
    mirror_id: &str,
    retired_members: &[String],
) {
    let source_prefix = format!("matrix/{source_id}/");
    let mirror_prefix = format!("matrix/{mirror_id}/");
    for layout in &mut doc.layouts {
        let Some(link) = layout.mirror_link.as_mut() else {
            continue;
        };
        link.correspondences.retain(|record| {
            if record.kind == MirroredItemKind::Matrix
                && ((record.source_id == source_id && record.mirror_id == mirror_id)
                    || (record.source_id == mirror_id && record.mirror_id == source_id))
            {
                return false;
            }
            if record.kind == MirroredItemKind::Part
                && ((record.source_id.starts_with(&source_prefix)
                    && record.mirror_id.starts_with(&mirror_prefix))
                    || (record.source_id.starts_with(&mirror_prefix)
                        && record.mirror_id.starts_with(&source_prefix)))
            {
                return false;
            }
            true
        });
        link.exceptions.retain(|exception| {
            if exception.kind == MirroredItemKind::Matrix && exception.mirror_id == mirror_id {
                return !retired_members.iter().any(|removed| removed == mirror_id);
            }
            if exception.kind == MirroredItemKind::Part {
                for removed in retired_members {
                    if exception
                        .mirror_id
                        .starts_with(&format!("matrix/{removed}/"))
                    {
                        return false;
                    }
                }
            }
            true
        });
    }
}

/// Ticket 21: record a Local matrix exception for a confirmed linked
/// detach. Refuses when the matrix is not a linked mirror member or the
/// overlay already exists, so stale confirmations change nothing.
pub(crate) fn add_local_matrix_exception(
    doc: &mut ProjectDoc,
    mirror_id: &str,
) -> Result<(), String> {
    let layout = doc
        .layouts
        .iter_mut()
        .find(|layout| {
            layout.matrix_ids.iter().any(|id| id == mirror_id) && layout.mirror_link.is_some()
        })
        .ok_or_else(|| format!("Matrix {mirror_id} is not a linked mirror member"))?;
    let link = layout.mirror_link.as_mut().expect("linked mirror");
    if link.exceptions.iter().any(|exception| {
        exception.kind == MirroredItemKind::Matrix && exception.mirror_id == mirror_id
    }) {
        return Err(format!(
            "Confirmed detach is stale: {mirror_id} is already excepted"
        ));
    }
    link.exceptions.push(MirroredException {
        kind: MirroredItemKind::Matrix,
        mirror_id: mirror_id.to_string(),
        state: MirroredExceptionState::Local,
        companion_mode: None,
        companion_tweak: None,
        removed_copper: None,
    });
    Ok(())
}

/// Ticket 21: an Omitted matrix has no independent storage state — it
/// follows its source live and rejoins to adopt it. Refuse direct storage
/// edits naming it (or its absent members) instead of silently diverging
/// the live record or discarding the edit on the next sync.
pub(crate) fn refuse_omitted_storage_edit(
    doc: &ProjectDoc,
    matrix_id: &str,
    cells: &[(u32, u32)],
) -> Result<(), String> {
    if exception_state(doc, MirroredItemKind::Matrix, matrix_id)
        == Some(MirroredExceptionState::Omitted)
    {
        return Err(format!(
            "Matrix {matrix_id} is Omitted and follows its source; Rejoin it to edit"
        ));
    }
    for (row, column) in cells {
        let key_id = crate::matrix::member_id(matrix_id, *row, *column);
        if omitted(doc, matrix_id, &key_id) {
            return Err(format!(
                "Key {key_id} is Omitted and follows its source; Rejoin it to edit"
            ));
        }
    }
    Ok(())
}

/// Whether a matrix edits alone: an independent (Local) or absent (Omitted)
/// matrix neither joins a grouped storage edit nor is joined by one. Its
/// partner follows through `sync` (live for Omitted, skipped for Local).
pub(crate) fn edits_alone(doc: &ProjectDoc, matrix_id: &str) -> bool {
    if matches!(
        exception_state(doc, MirroredItemKind::Matrix, matrix_id),
        Some(MirroredExceptionState::Local | MirroredExceptionState::Omitted)
    ) {
        return true;
    }
    partner(doc, matrix_id).is_some_and(|(partner, _)| {
        matches!(
            exception_state(doc, MirroredItemKind::Matrix, &partner),
            Some(MirroredExceptionState::Local | MirroredExceptionState::Omitted)
        )
    })
}

/// The mirror matrix in `layout_id` owning `mirror_id`, by slot identity.
fn owning_matrix(doc: &ProjectDoc, layout_id: &str, mirror_id: &str) -> Option<String> {
    let layout = doc.layouts.iter().find(|layout| layout.id == layout_id)?;
    layout
        .matrix_ids
        .iter()
        .find(|matrix_id| mirror_id.starts_with(&format!("matrix/{matrix_id}/r")))
        .cloned()
}

/// Parse `matrix/<matrix>/r<row>c<column>` keys; companions resolve to their key.
fn split_key_id(id: &str) -> Option<(String, u32, u32)> {
    let rest = id.strip_prefix("matrix/")?;
    let (matrix_id, slot) = rest.split_once("/r")?;
    let slot = slot.split('/').next()?;
    let (row, column) = slot.split_once('c')?;
    Some((
        matrix_id.to_string(),
        row.parse().ok()?,
        column.parse().ok()?,
    ))
}

/// Ensure an Omitted key overlays a part correspondence, adding it by slot
/// when the source slot exists. Refuses naming the missing counterpart
/// instead of overlaying nothing.
fn ensure_part_correspondence(
    doc: &mut ProjectDoc,
    layout_id: &str,
    mirror_id: &str,
) -> Result<(), String> {
    let layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == layout_id)
        .ok_or("Linked layout is missing")?;
    let link = layout
        .mirror_link
        .as_ref()
        .ok_or("Linked layout is missing")?;
    if link
        .correspondences
        .iter()
        .any(|record| record.kind == MirroredItemKind::Part && record.mirror_id == mirror_id)
    {
        return Ok(());
    }
    let (mirror_matrix, row, column) = split_key_id(mirror_id)
        .ok_or_else(|| format!("Layout exception {mirror_id:?} overlays no correspondence"))?;
    let source_matrix = link
        .correspondences
        .iter()
        .find(|record| record.kind == MirroredItemKind::Matrix && record.mirror_id == mirror_matrix)
        .map(|record| record.source_id.clone())
        .ok_or_else(|| format!("Layout exception {mirror_id:?} overlays no correspondence"))?;
    let source = matrix::evaluation::logical_matrix(&source_matrix, doc)
        .map_err(|_| format!("Layout exception {mirror_id:?} overlays no correspondence"))?;
    if row >= source.rows || column >= source.columns {
        return Err(format!(
            "Layout exception {mirror_id:?} names a missing counterpart in matrix {source_matrix:?}; resize or Rejoin its parent instead of choosing another key"
        ));
    }
    let source_id = matrix::member_id(&source_matrix, row, column);
    if link
        .correspondences
        .iter()
        .any(|record| record.kind == MirroredItemKind::Part && record.source_id == source_id)
    {
        return Err(format!(
            "Layout exception {mirror_id:?} names a missing counterpart in matrix {source_matrix:?}; resize or Rejoin its parent instead of choosing another key"
        ));
    }
    let layout = doc
        .layouts
        .iter_mut()
        .find(|layout| layout.id == layout_id)
        .ok_or("Linked layout is missing")?;
    if let Some(link) = layout.mirror_link.as_mut() {
        link.correspondences.push(MirroredCorrespondence {
            kind: MirroredItemKind::Part,
            source_id,
            mirror_id: mirror_id.to_string(),
            companion_mode: None,
            companion_tweak: None,
        });
    }
    Ok(())
}

/// Ticket 21: materialize a record-absent Omitted member on Rejoin (or
/// Omitted-to-Local): clone its source record through the pair map and
/// retarget its anchor by correspondence. A missing counterpart refuses
/// naming it instead of silently retargeting.
fn synthesize_rejoin_record(
    doc: &ProjectDoc,
    layout: &Layout,
    mirror_id: &str,
) -> Result<Matrix, String> {
    let link = layout
        .mirror_link
        .as_ref()
        .ok_or("Linked layout is missing")?;
    let source_id = link
        .correspondences
        .iter()
        .find(|record| record.kind == MirroredItemKind::Matrix && record.mirror_id == mirror_id)
        .map(|record| record.source_id.clone())
        .ok_or_else(|| format!("Linked matrix {mirror_id} has no source correspondence"))?;
    let source = matrix::evaluation::logical_matrix(&source_id, doc)
        .map_err(|_| format!("Linked matrix {mirror_id} has no source correspondence"))?;
    let owned = source.into_owned();
    let placeholder = Matrix {
        id: mirror_id.to_string(),
        name: None,
        board_id: Some(layout.board_id.clone()),
        part_ids: Vec::new(),
        ..owned.clone()
    };
    let mut synthesized = reflected(
        &owned,
        &placeholder,
        axis_mapping(link.axis_x),
        false,
        link.split_gap.is_some(),
    )?;
    synthesized.board_id = Some(layout.board_id.clone());
    if let Some(anchor) = synthesized.anchor.clone() {
        let mapped =
            remap_anchor_target(&link.correspondences, &anchor.matrix_id, true, &layout.id)?;
        if let Some(result_anchor) = synthesized.anchor.as_mut() {
            result_anchor.matrix_id = mapped;
        }
    }
    Ok(synthesized)
}

/// Return a key leaving Local to the followed cell: copy the counterpart's
/// independent offset/rotation, or clear them when the key follows no
/// counterpart. The next resolve then materializes the shared pose.
fn reset_key_to_followed(
    doc: &mut ProjectDoc,
    layout_id: &str,
    mirror_id: &str,
) -> Result<(), String> {
    let Some((mirror_matrix, row, column)) = split_key_id(mirror_id) else {
        return Ok(());
    };
    let source_cell = doc
        .layouts
        .iter()
        .find(|layout| layout.id == layout_id)
        .and_then(|layout| layout.mirror_link.as_ref())
        .and_then(|link| {
            link.correspondences.iter().find(|record| {
                record.kind == MirroredItemKind::Matrix && record.mirror_id == mirror_matrix
            })
        })
        .and_then(|record| {
            doc.matrices
                .iter()
                .find(|matrix| matrix.id == record.source_id)
        })
        .and_then(|source| {
            source
                .cells
                .iter()
                .find(|cell| cell.row == row && cell.column == column)
        })
        .map(|cell| (cell.offset, cell.rotation));
    let Some(mirror) = doc
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == mirror_matrix)
    else {
        return Ok(());
    };
    if let Some(cell) = mirror
        .cells
        .iter_mut()
        .find(|cell| cell.row == row && cell.column == column)
    {
        let (offset, rotation) = source_cell.unwrap_or((None, None));
        cell.offset = offset;
        cell.rotation = rotation;
    }
    Ok(())
}

#[cfg(test)]
mod case_hardware_tests {
    use super::*;

    #[test]
    fn case_clearance_holes_are_not_adopted_by_linked_layouts() {
        let mut doc = ProjectDoc::empty("case", "Case");
        doc.boards = serde_json::from_value(serde_json::json!([{"id":"board","name":"Board","outlineIds":[],"partIds":["case-closure/board/10/10"],"netIds":[],"thickness":1.6}])).unwrap();
        doc.parts = serde_json::from_value(serde_json::json!([{"id":"case-closure/board/10/10","definitionId":"hole","reference":"MH1","pose":{"at":{"x":10,"y":10},"rotation":0},"side":"front"}])).unwrap();
        doc.matrices = serde_json::from_value(serde_json::json!([{"id":"matrix","boardId":"board","rows":1,"columns":1,"pitch":{"x":19,"y":19},"origin":{"x":0,"y":0},"definitionId":"key","partIds":[]}])).unwrap();
        doc.layouts = serde_json::from_value(serde_json::json!([{"id":"left","name":"Left","boardId":"board","matrixIds":["matrix"],"partIds":[]}])).unwrap();
        adopt_unassigned_components(&mut doc, "left", 100.0, &mut vec![]).unwrap();
        assert!(doc.layouts[0].part_ids.is_empty());
    }

    #[test]
    fn linked_controller_mirror_keeps_world_heading() {
        // Setup "Place controller": one controller source part at rotation 0 on a linked
        // source layout. The derived mirror must share world rotation 0 (plain reflection
        // `-r = 0`), not ADR-0010's `180 - r` half turn.
        let mut doc = ProjectDoc::empty("linked-controller", "Linked controller");
        doc.definitions = serde_json::from_value(serde_json::json!([
            {"id":"mcu","name":"MCU","kind":"controller","courtyard":[],"pads":[]}
        ]))
        .unwrap();
        doc.boards = serde_json::from_value(serde_json::json!([{"id":"board","name":"Board","outlineIds":[],"partIds":["source/U1"],"netIds":[],"thickness":1.6}])).unwrap();
        doc.parts = serde_json::from_value(serde_json::json!([{"id":"source/U1","definitionId":"mcu","reference":"U1","pose":{"at":{"x":10.0,"y":5.0},"rotation":0.0},"side":"front"}])).unwrap();
        doc.matrices = serde_json::from_value(serde_json::json!([
            {"id":"source-matrix","boardId":"board","rows":1,"columns":1,"pitch":{"x":19,"y":19},"origin":{"x":0,"y":0},"definitionId":"key","partIds":[]},
            {"id":"mirror-matrix","boardId":"board","rows":1,"columns":1,"pitch":{"x":19,"y":19},"origin":{"x":0,"y":0},"definitionId":"key","partIds":[]}
        ])).unwrap();
        doc.layouts = serde_json::from_value(serde_json::json!([
            {"id":"source-layout","name":"Source","boardId":"board","matrixIds":["source-matrix"],"partIds":["source/U1"]},
            {"id":"mirror-layout","name":"Mirror","boardId":"board","matrixIds":["mirror-matrix"],"partIds":[],"mirrorLink":{"sourceId":"source-layout","axisX":50.0}}
        ])).unwrap();
        sync_components(&mut doc).unwrap();
        let mirrored = doc
            .parts
            .iter()
            .find(|part| {
                part.properties
                    .as_ref()
                    .and_then(|properties| properties.get(MIRRORED_COMPONENT_SOURCE))
                    .and_then(serde_json::Value::as_str)
                    == Some("source/U1")
            })
            .expect("the linked controller mirror is materialized");
        assert_eq!(mirrored.pose.at, Vec2 { x: 90.0, y: 5.0 });
        assert!(
            (mirrored.pose.rotation - 0.0).abs() < 1e-9,
            "controller mirror keeps world heading 0, got {}",
            mirrored.pose.rotation
        );
        for heading in [90.0f64, 180.0, 270.0] {
            let mut doc = ProjectDoc::empty("linked-controller", "Linked controller");
            doc.definitions = serde_json::from_value(serde_json::json!([
                {"id":"mcu","name":"MCU","kind":"controller","courtyard":[],"pads":[]}
            ]))
            .unwrap();
            doc.boards = serde_json::from_value(serde_json::json!([{"id":"board","name":"Board","outlineIds":[],"partIds":["source/U1"],"netIds":[],"thickness":1.6}])).unwrap();
            doc.parts = serde_json::from_value(serde_json::json!([{"id":"source/U1","definitionId":"mcu","reference":"U1","pose":{"at":{"x":10.0,"y":5.0},"rotation":heading},"side":"front"}])).unwrap();
            doc.matrices = serde_json::from_value(serde_json::json!([
                {"id":"source-matrix","boardId":"board","rows":1,"columns":1,"pitch":{"x":19,"y":19},"origin":{"x":0,"y":0},"definitionId":"key","partIds":[]},
                {"id":"mirror-matrix","boardId":"board","rows":1,"columns":1,"pitch":{"x":19,"y":19},"origin":{"x":0,"y":0},"definitionId":"key","partIds":[]}
            ])).unwrap();
            doc.layouts = serde_json::from_value(serde_json::json!([
                {"id":"source-layout","name":"Source","boardId":"board","matrixIds":["source-matrix"],"partIds":["source/U1"]},
                {"id":"mirror-layout","name":"Mirror","boardId":"board","matrixIds":["mirror-matrix"],"partIds":[],"mirrorLink":{"sourceId":"source-layout","axisX":50.0}}
            ])).unwrap();
            sync_components(&mut doc).unwrap();
            let mirrored = doc
                .parts
                .iter()
                .find(|part| {
                    part.properties
                        .as_ref()
                        .and_then(|properties| properties.get(MIRRORED_COMPONENT_SOURCE))
                        .and_then(serde_json::Value::as_str)
                        == Some("source/U1")
                })
                .expect("the linked controller mirror is materialized");
            let expected = (0.0 - heading).rem_euclid(360.0);
            assert!(
                (mirrored.pose.rotation - expected).abs() < 1e-9,
                "controller heading {heading} mirrors to {expected}, got {}",
                mirrored.pose.rotation
            );
        }
    }

    fn matrix_with_companion() -> Matrix {
        serde_json::from_value(serde_json::json!({
            "id": "source", "rows": 1, "columns": 1, "pitch": {"x": 19, "y": 19},
            "origin": {"x": 10, "y": 5}, "rotation": 20, "definitionId": "key", "partIds": [],
            "cells": [{"row": 0, "column": 0, "enabled": true, "rotation": 15,
                "assemblies": [{"id": "diode", "definitionId": "d", "offset": {"x": 3, "y": 4}, "rotation": 30}]}]
        }))
        .unwrap()
    }

    fn empty_target() -> Matrix {
        let mut target = matrix_with_companion();
        target.id = "target".into();
        target.cells.clear();
        target
    }

    #[test]
    fn a_vertical_axis_reflects_the_origin_turns_and_companions() {
        let reflected = reflected(
            &matrix_with_companion(),
            &empty_target(),
            axis_mapping(50.0),
            true,
            false,
        )
        .unwrap();
        assert_eq!(reflected.origin, Vec2 { x: 90.0, y: 5.0 });
        assert_eq!(reflected.rotation, Some(-20.0));
        assert_eq!(reflected.mirror, Some(Mirror::X));
        let cell = &reflected.cells[0];
        assert_eq!(cell.rotation, Some(15.0));
        assert_eq!(cell.assemblies[0].offset, Vec2 { x: -3.0, y: 4.0 });
        assert_eq!(cell.assemblies[0].rotation, Some(-30.0));
    }

    #[test]
    fn a_rotated_map_turns_the_matrix_heading_once_and_leaves_relative_values_alone() {
        let mapping = Pose2 {
            at: Vec2 { x: 100.0, y: 10.0 },
            rotation: 40.0,
        };
        let reflected = reflected(
            &matrix_with_companion(),
            &empty_target(),
            mapping,
            true,
            false,
        )
        .unwrap();
        // The origin goes through the map and the heading is `rotation - angle`.
        let expected = pair::forward(mapping, Vec2 { x: 10.0, y: 5.0 });
        assert_eq!(reflected.origin, expected);
        assert_eq!(reflected.rotation, Some(20.0));
        // The cell turn and the companion are relative to that frame and to the part, which
        // already carry the pair rotation: only their handedness flips.
        let cell = &reflected.cells[0];
        assert_eq!(cell.rotation, Some(15.0));
        assert_eq!(cell.assemblies[0].offset, Vec2 { x: -3.0, y: 4.0 });
        assert_eq!(cell.assemblies[0].rotation, Some(-30.0));
    }
}

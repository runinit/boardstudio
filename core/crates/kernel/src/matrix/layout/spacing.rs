//! Adaptive spacing between linked halves: envelope extents, gaps and clearance deficits.

use super::{adopt_half_frames, axis_mapping, evaluated_part_pose, implied_gap, reflected};
use crate::{matrix, model::*};
use std::collections::{BTreeMap, BTreeSet};

/// Ticket 10: outline margin (mm) used for the adaptive minimum when the
/// pair's board carries no PartEnvelope outline feature. Matches the
/// prototype's clearance feel; recorded in ADR-0036.
pub const DEFAULT_SPACING_MARGIN_MM: f64 = 3.0;

/// Ticket 11: default Split gap (mm) proposed at mirrored-pair creation. The
/// stored gap is `max(requested, default, minimum)`: an explicit wider
/// placement is kept, anything below the default or the ticket-10 minimum is
/// raised so the placed pair already shows the adapted gap.
pub const DEFAULT_CREATION_GAP_MM: f64 = 40.0;

/// Deficits at or below this (mm) count as clear.
pub(super) const SPACING_EPSILON: f64 = 1e-9;

/// Backstop for the minimum search; the rigid-halves case converges in one step.
pub(super) const MAX_SPACING_PASSES: usize = 50;

/// Ticket 10: bring every stored-gap pair's current spacing to
/// `max(stored gap, minimum)` after a committed edit. The stored gap is never
/// rewritten here; narrowing back stops at the stored floor. Adoptable legacy
/// pairs adopt first (pose-preserving, like the matrix-edit tail), but stay
/// unspaced until spacing is needed: a pair that already carries a stored gap
/// re-spaces on every edit, while a newly adopted pair only gains a spacing
/// when its envelopes touch. Fully anchored pairs stay legacy: with no
/// unanchored matrix there is no implied gap to adopt. `accepted` is the
/// pre-edit document: only pairs already gapped there follow the full rule.
/// Returns the ids whose spacing changed.
pub fn adapt_spacing(doc: &mut ProjectDoc, accepted: &ProjectDoc) -> Result<Vec<String>, String> {
    let pre_gapped: BTreeSet<String> = accepted
        .layouts
        .iter()
        .filter_map(|layout| {
            layout
                .mirror_link
                .as_ref()
                .and_then(|link| link.split_gap.map(|_| layout.id.clone()))
        })
        .collect();
    let mut changed = adopt_half_frames(doc)?;
    let pairs: Vec<(String, String, f64, f64)> = doc
        .layouts
        .iter()
        .filter_map(|layout| {
            let link = layout.mirror_link.as_ref()?;
            let floor = link.split_gap?;
            Some((
                link.source_id.clone(),
                layout.id.clone(),
                link.axis_x,
                floor,
            ))
        })
        .collect();
    for (source_id, target_id, axis_x, floor) in pairs {
        // Newly adopted pairs keep their placement until their envelopes
        // touch; stored-gap pairs follow the minimum on every edit.
        if !pre_gapped.contains(&target_id) {
            let source_left = source_is_left(doc, &source_id, &target_id)?;
            let touching = envelope_clear(doc, &source_id, &target_id, axis_x, source_left)?
                .is_some_and(|clear| clear <= SPACING_EPSILON);
            if !touching {
                continue;
            }
        }
        let board_id = doc
            .layouts
            .iter()
            .find(|layout| layout.id == target_id)
            .map(|layout| layout.board_id.clone())
            .ok_or("Linked layout is missing")?;
        let margin = spacing_margin(doc, &board_id);
        let current = doc
            .layouts
            .iter()
            .find(|layout| layout.id == target_id)
            .and_then(|layout| layout.mirror_link.as_ref())
            .and_then(|link| link.spacing)
            .unwrap_or(floor);
        let Some(minimum) = minimum_gap(doc, &source_id, &target_id, axis_x, margin)? else {
            continue;
        };
        let next_gap = floor.max(minimum);
        if next_gap != current {
            if let Some(layout) = doc.layouts.iter_mut().find(|layout| layout.id == target_id)
                && let Some(link) = layout.mirror_link.as_mut()
            {
                link.spacing = Some(next_gap);
            }
            changed.extend(pair_member_ids(doc, &source_id, &target_id));
        }
    }
    changed.sort();
    changed.dedup();
    Ok(changed)
}

/// Ticket 11: stored gap proposed for a mirrored pair about to be created from
/// `matrix` about `axis_x`, without mutating `doc`: `max(requested placement
/// gap, 40 mm default, ticket-10 minimum)`. The pair is staged in a scratch
/// copy (origins converted exactly like the commit path, mirror reflected,
/// temporary link) so the minimum sees the geometry the committed pair
/// evaluates to at the requested gap. An unsatisfiable minimum (a key
/// spanning the axis, a fixed part pinning an extent) falls back to the
/// default floor instead of widening into the red, like the adaptive pass
/// keeps the current spacing. Anchored matrices stay legacy: the request is
/// returned unchanged and creation stores no gap.
pub fn proposed_creation_gap(
    doc: &ProjectDoc,
    matrix: &Matrix,
    axis_x: f64,
) -> Result<f64, String> {
    let requested = implied_gap(axis_x, matrix);
    if !requested.is_finite() {
        return Err("Split gap must be finite".into());
    }
    if matrix.anchor.is_some() {
        return Ok(requested);
    }
    let board_id = matrix
        .board_id
        .clone()
        .ok_or("A new mirrored pair must be on a board")?;
    let mut scratch = doc.clone();
    let mut source = matrix.clone();
    source.origin.x -= axis_x - requested / 2.0;
    let mut mirror_shell = source.clone();
    mirror_shell.id = format!("{}-creation-mirror", source.id);
    let mirror = reflected(&source, &mirror_shell, axis_mapping(axis_x), false, true)?;
    let source_layout_id = format!("{}-creation-source", source.id);
    let target_layout_id = format!("{}-creation-target", source.id);
    let source_id = source.id.clone();
    let mirror_id = mirror.id.clone();
    scratch.matrices.push(source);
    scratch.matrices.push(mirror);
    scratch.layouts.push(Layout {
        id: source_layout_id.clone(),
        name: "Creation source".into(),
        board_id: board_id.clone(),
        matrix_ids: vec![source_id],
        part_ids: vec![],
        mirror_link: None,
    });
    scratch.layouts.push(Layout {
        id: target_layout_id.clone(),
        name: "Creation target".into(),
        board_id: board_id.clone(),
        matrix_ids: vec![mirror_id],
        part_ids: vec![],
        mirror_link: Some(LayoutMirrorLink {
            source_id: source_layout_id.clone(),
            axis_x,
            split_gap: Some(requested),
            spacing: None,
            correspondences: vec![],
            exceptions: vec![],
        }),
    });
    let margin = spacing_margin(&scratch, &board_id);
    let minimum = minimum_gap(
        &mut scratch,
        &source_layout_id,
        &target_layout_id,
        axis_x,
        margin,
    )?;
    Ok(requested
        .max(DEFAULT_CREATION_GAP_MM)
        .max(minimum.unwrap_or(f64::NEG_INFINITY)))
}

/// The clearance basis for one pair's board: the widest PartEnvelope margin on
/// that board, or the default when the board carries no envelope feature.
pub(super) fn spacing_margin(doc: &ProjectDoc, board_id: &str) -> f64 {
    let owned: BTreeSet<&str> = doc
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .map(|board| board.outline_ids.iter().map(String::as_str).collect())
        .unwrap_or_default();
    doc.outline
        .iter()
        .filter_map(|feature| match feature {
            OutlineFeature::PartEnvelope { id, margin, .. }
                if owned.contains(id.as_str()) && margin.is_finite() && *margin >= 0.0 =>
            {
                Some(*margin)
            }
            _ => None,
        })
        .fold(None, |widest: Option<f64>, margin| {
            Some(widest.map_or(margin, |known| known.max(margin)))
        })
        .unwrap_or(DEFAULT_SPACING_MARGIN_MM)
}

/// Layouts, matrices and member parts of one pair, for the changed-id report.
pub(super) fn pair_member_ids(doc: &ProjectDoc, source_id: &str, target_id: &str) -> Vec<String> {
    let mut ids = vec![source_id.to_string(), target_id.to_string()];
    for layout_id in [source_id, target_id] {
        if let Some(layout) = doc.layouts.iter().find(|layout| layout.id == layout_id) {
            for matrix_id in &layout.matrix_ids {
                ids.push(matrix_id.clone());
                if let Some(matrix) = doc.matrices.iter().find(|matrix| matrix.id == *matrix_id) {
                    ids.extend(matrix.part_ids.iter().cloned());
                }
            }
        }
    }
    ids
}

/// Half-plane x extents of one half: every item's overall min/max plus a
/// presence count for the deficit terms.
#[derive(Clone, Copy)]
pub(super) struct HalfExtents {
    pub(super) min: f64,
    pub(super) max: f64,
    pub(super) center_count: usize,
}

impl HalfExtents {
    pub(super) fn add(&mut self, min: f64, max: f64) {
        self.min = self.min.min(min);
        self.max = self.max.max(max);
        self.center_count += 1;
    }
}

pub(super) fn empty_half() -> HalfExtents {
    HalfExtents {
        min: f64::INFINITY,
        max: f64::NEG_INFINITY,
        center_count: 0,
    }
}

/// Axis-aligned x half-width of a centered rect after a degrees rotation.
pub(super) fn rotated_half_width(size: Vec2, rotation_deg: f64) -> f64 {
    let (sin, cos) = rotation_deg.to_radians().sin_cos();
    (size.x / 2.0 * cos).abs() + (size.y / 2.0 * sin).abs()
}

/// Which layout-owned half sits left of the split axis at the current spacing.
/// The mapping is decided once per adaptive pass and held fixed through the
/// minimum search, so coincident halves at gap zero cannot flip sides
/// mid-search. Falls back to the source-left convention when a half has no
/// keys to compare.
pub(super) fn source_is_left(
    doc: &ProjectDoc,
    source_id: &str,
    target_id: &str,
) -> Result<bool, String> {
    let mut means = Vec::new();
    for layout_id in [source_id, target_id] {
        let Some(layout) = doc.layouts.iter().find(|layout| layout.id == layout_id) else {
            means.push(None);
            continue;
        };
        let mut sum = 0.0;
        let mut count = 0usize;
        for matrix_id in &layout.matrix_ids {
            let Some(matrix) = doc.matrices.iter().find(|matrix| matrix.id == *matrix_id) else {
                continue;
            };
            let evaluated = matrix::evaluated(matrix, doc)?;
            let members = matrix::cell_members(matrix);
            for cell in &evaluated.cells {
                if members.contains_key(&(cell.row as u32, cell.column as u32)) {
                    sum += cell.pose.at.x;
                    count += 1;
                }
            }
        }
        means.push((count > 0).then_some(sum / count as f64));
    }
    Ok(match (means[0], means[1]) {
        (Some(source_mean), Some(mirror_mean)) => source_mean <= mirror_mean,
        _ => true,
    })
}

/// Left/right key-plus-part x extents of one pair at the evaluated geometry.
/// Keys and parts join their half by layout ownership; sides never compare
/// against the axis mid-search, so rigid halves cannot flip. The caller
/// supplies the frozen ownership-to-side mapping.
pub(super) fn collect_sides(
    doc: &ProjectDoc,
    source_id: &str,
    target_id: &str,
    _axis_x: f64,
    source_left: bool,
) -> Result<(HalfExtents, HalfExtents), String> {
    let source = doc
        .layouts
        .iter()
        .find(|layout| layout.id == source_id)
        .ok_or("Linked layout is missing")?;
    let target = doc
        .layouts
        .iter()
        .find(|layout| layout.id == target_id)
        .ok_or("Linked layout is missing")?;
    // Independent parts ride the half frame with their owning layout, exactly
    // like keys: the envelope reads their evaluated poses, so spacing moves
    // them with their half and a local part's overhang at the axis still
    // pushes the other half out. Matrix members and their companions follow
    // their keys through the half above. Only the pair's own layouts count:
    // other layouts sharing the board keep their fixed placement and never
    // pin the pair. Ownership (not board position) keeps each half rigid
    // through the minimum search, so coincident halves cannot flip sides.
    let mut members: BTreeSet<String> = BTreeSet::new();
    for layout in [source, target] {
        for matrix_id in &layout.matrix_ids {
            if let Some(matrix) = doc.matrices.iter().find(|matrix| matrix.id == *matrix_id) {
                members.extend(matrix::cell_members(matrix).values().cloned());
            }
        }
    }
    let definitions: BTreeMap<&str, &PartDefinition> = doc
        .definitions
        .iter()
        .map(|definition| (definition.id.as_str(), definition))
        .collect();
    let mut source_half = empty_half();
    let mut mirror_half = empty_half();
    for (layout, half) in [(source, &mut source_half), (target, &mut mirror_half)] {
        for matrix_id in &layout.matrix_ids {
            let Some(matrix) = doc.matrices.iter().find(|matrix| matrix.id == *matrix_id) else {
                continue;
            };
            let evaluated = matrix::evaluated(matrix, doc)?;
            let members = matrix::cell_members(matrix);
            let edge = matrix.edge_gap.unwrap_or(Vec2 {
                x: super::super::DEFAULT_EDGE_GAP_MM,
                y: super::super::DEFAULT_EDGE_GAP_MM,
            });
            let size = Vec2 {
                x: matrix.pitch.x - edge.x,
                y: matrix.pitch.y - edge.y,
            };
            for cell in &evaluated.cells {
                if !members.contains_key(&(cell.row as u32, cell.column as u32)) {
                    continue;
                }
                let half_width = rotated_half_width(size, cell.pose.rotation);
                half.add(cell.pose.at.x - half_width, cell.pose.at.x + half_width);
            }
        }
        for part_id in &layout.part_ids {
            let Some(part) = doc.parts.iter().find(|part| &part.id == part_id) else {
                continue;
            };
            if members.contains(part.id.as_str())
                || part
                    .id
                    .rsplit_once('/')
                    .is_some_and(|(head, _)| members.contains(head))
            {
                continue;
            }
            let Some(definition) = definitions.get(part.definition_id.as_str()) else {
                continue;
            };
            if definition.kind == PartKind::Utility {
                continue;
            }
            let pose = evaluated_part_pose(doc, part);
            let (min, max) = part_extents_at(&pose, part.side.clone(), part.keycap, definition);
            half.add(min, max);
        }
    }
    // The caller freezes the ownership-to-side mapping before the minimum
    // search, so coincident halves cannot flip sides mid-search.
    let left = if source_left {
        source_half
    } else {
        mirror_half
    };
    let right = if source_left {
        mirror_half
    } else {
        source_half
    };
    Ok((left, right))
}

/// Key-envelope air between the halves at the evaluated geometry (`None`
/// when a half has no keys or parts). Order-free: touching or overlapping
/// reads zero or negative under either side mapping.
pub(super) fn envelope_clear(
    doc: &ProjectDoc,
    source_id: &str,
    target_id: &str,
    axis_x: f64,
    source_left: bool,
) -> Result<Option<f64>, String> {
    let (left, right) = collect_sides(doc, source_id, target_id, axis_x, source_left)?;
    if left.center_count == 0 || right.center_count == 0 {
        return Ok(None);
    }
    Ok(Some(right.min - left.max))
}

/// The prototype ticket-10 deficit at one candidate gap: the largest of the
/// envelope-clearance shortfall against `4 * margin` and each present half's
/// margin overhang past the split axis. Zero or less means the outlines clear
/// by `2 * margin` with each outline on its own side.
pub(super) fn envelope_deficit(
    doc: &ProjectDoc,
    source_id: &str,
    target_id: &str,
    axis_x: f64,
    margin: f64,
    source_left: bool,
) -> Result<f64, String> {
    let (left, right) = collect_sides(doc, source_id, target_id, axis_x, source_left)?;
    let need = 4.0 * margin;
    let mut deficit = f64::NEG_INFINITY;
    if left.center_count > 0 && right.center_count > 0 {
        deficit = deficit.max(need - (right.min - left.max));
    }
    if left.center_count > 0 {
        deficit = deficit.max(2.0 * (left.max + margin - axis_x));
    }
    if right.center_count > 0 {
        deficit = deficit.max(2.0 * (axis_x + margin - right.min));
    }
    if deficit == f64::NEG_INFINITY {
        deficit = 0.0;
    }
    Ok(deficit)
}

/// Board-space x extents of one independent part at its evaluated pose: its
/// courtyard, or its keycap size, or its bare pose when neither is available.
pub fn part_extents_at(
    pose: &Pose2,
    side: Side,
    keycap: Option<Vec2>,
    definition: &PartDefinition,
) -> (f64, f64) {
    let (sin, cos) = pose.rotation.to_radians().sin_cos();
    let project = |point: Vec2| {
        let x = if side == Side::Back {
            -point.x
        } else {
            point.x
        };
        Vec2 {
            x: pose.at.x + x * cos - point.y * sin,
            y: pose.at.y + x * sin + point.y * cos,
        }
    };
    if !definition.courtyard.is_empty() {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for point in &definition.courtyard {
            let placed = project(*point);
            min = min.min(placed.x);
            max = max.max(placed.x);
        }
        return (min, max);
    }
    if let Some(size) = keycap.or(definition.keycap) {
        let half_width = rotated_half_width(size, pose.rotation);
        return (pose.at.x - half_width, pose.at.x + half_width);
    }
    (pose.at.x, pose.at.x)
}

/// Relative x extents of one independent part about its own center, for the
/// split clamp: the evaluated shape shifted so the stored center sits at zero.
pub fn part_extent_offsets(
    pose: &Pose2,
    side: Side,
    keycap: Option<Vec2>,
    definition: &PartDefinition,
) -> (f64, f64) {
    let centered = Pose2 {
        at: Vec2 { x: 0.0, y: 0.0 },
        rotation: pose.rotation,
    };
    part_extents_at(&centered, side, keycap, definition)
}

/// The deficit with the pair evaluated at one candidate gap. Temporarily
/// swaps the stored spacing, then restores it, so callers see no mutation.
pub(super) fn deficit_at(
    doc: &mut ProjectDoc,
    source_id: &str,
    target_id: &str,
    axis_x: f64,
    margin: f64,
    gap: f64,
    source_left: bool,
) -> Result<f64, String> {
    let saved = {
        let layout = doc
            .layouts
            .iter_mut()
            .find(|layout| layout.id == target_id)
            .ok_or("Linked layout is missing")?;
        let link = layout
            .mirror_link
            .as_mut()
            .ok_or("Layout has no mirror link")?;
        link.spacing.replace(gap)
    };
    let result = envelope_deficit(doc, source_id, target_id, axis_x, margin, source_left);
    if let Some(layout) = doc.layouts.iter_mut().find(|layout| layout.id == target_id)
        && let Some(link) = layout.mirror_link.as_mut()
    {
        link.spacing = saved;
    }
    result
}

/// The smallest gap whose deficit clears (`None` when no gap clears: keys
/// that span the axis or fixed parts pinning an extent cannot be separated by
/// symmetric widening, so the caller keeps the current spacing instead of
/// thrashing the halves). Key halves move rigidly with the gap, so the
/// satisfiable case converges in one exact step; the search stops as soon as
/// the deficit stalls or the pass budget runs out.
pub(super) fn minimum_gap(
    doc: &mut ProjectDoc,
    source_id: &str,
    target_id: &str,
    axis_x: f64,
    margin: f64,
) -> Result<Option<f64>, String> {
    let source_left = source_is_left(doc, source_id, target_id)?;
    let mut gap = 0.0;
    let mut previous = f64::INFINITY;
    for _ in 0..MAX_SPACING_PASSES {
        let deficit = deficit_at(doc, source_id, target_id, axis_x, margin, gap, source_left)?;
        if deficit <= SPACING_EPSILON {
            return Ok(Some(gap));
        }
        if previous - deficit <= SPACING_EPSILON {
            return Ok(None);
        }
        previous = deficit;
        gap += deficit;
    }
    Ok(None)
}

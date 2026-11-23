//! Private accepted-document planner for the Layout key-size interaction.
use boardstudio_model::model::{
    KeycapCellOffset, Layout, Matrix, MatrixCell, MatrixScene, Position, ProjectDoc, Vec2,
};
use std::collections::{BTreeMap, BTreeSet};

// Explicit paths: this file also loads natively through lib.rs's `#[path]`
// inline `objects` module, where bare `mod` children would resolve beside
// `objects.rs` instead of in this module's directory.
#[path = "keycap_resize/geometry.rs"]
mod geometry;
#[cfg(test)]
#[path = "keycap_resize/tests.rs"]
mod tests;

use geometry::{
    add_delta, corners, dot, finite, half_extent, near, overlaps_on_axes, reflow_corners, scale,
    set_cell_offset,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeAxis {
    X,
    Y,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeycapPlacement {
    pub id: String,
    pub matrix_id: String,
    pub row: u32,
    pub column: u32,
    pub at: Vec2,
    pub rotation: f64,
    pub size: Vec2,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeycapResizePlan {
    pub document: ProjectDoc,
    pub target_ids: Vec<String>,
    /// Ticket 33: the same resize expressed as a typed Core edit payload.
    pub sizes: BTreeMap<String, Vec2>,
    /// Absolute poses after the reflow, in sorted part-id order.
    pub moves: Vec<Position>,
    /// Cell offsets that changed, in matrix/row/column order.
    pub offsets: Vec<KeycapCellOffset>,
}

/// Returns overlapping oriented keycap-envelope pairs in input order.
/// The accepted placements are preordered by `ProjectDoc.parts`, matching the React workbench.
pub fn overlapping_pairs(placements: &[KeycapPlacement]) -> Vec<(String, String)> {
    let mut bounds: Vec<_> = placements
        .iter()
        .map(|placement| {
            let polygon = corners(placement);
            let min_x = polygon
                .iter()
                .map(|point| point.x)
                .fold(f64::INFINITY, f64::min);
            let max_x = polygon
                .iter()
                .map(|point| point.x)
                .fold(f64::NEG_INFINITY, f64::max);
            let min_y = polygon
                .iter()
                .map(|point| point.y)
                .fold(f64::INFINITY, f64::min);
            let max_y = polygon
                .iter()
                .map(|point| point.y)
                .fold(f64::NEG_INFINITY, f64::max);
            (placement, polygon, min_x, max_x, min_y, max_y)
        })
        .collect();
    // Rust's slice sort is stable; equal min-X bounds retain accepted document-parts order.
    bounds.sort_by(|first, second| {
        first
            .2
            .partial_cmp(&second.2)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut overlaps = Vec::new();
    for index in 0..bounds.len() {
        let first = &bounds[index];
        for second in bounds
            .iter()
            .skip(index + 1)
            .take_while(|second| second.2 < first.3 - 1e-7)
        {
            if first.5 <= second.4 + 1e-7 || second.5 <= first.4 + 1e-7 {
                continue;
            }
            if overlaps_on_axes(&first.1, &second.1, &first.1)
                && overlaps_on_axes(&first.1, &second.1, &second.1)
            {
                overlaps.push((first.0.id.clone(), second.0.id.clone()));
            }
        }
    }
    overlaps
}

pub struct KeycapResizeInput<'a> {
    pub document: &'a ProjectDoc,
    pub matrices: &'a [Matrix],
    pub scenes: &'a [MatrixScene],
    pub layouts: &'a [Layout],
    pub placements: &'a [KeycapPlacement],
    pub selected_ids: &'a BTreeSet<String>,
    pub units: Vec2,
    pub axis: Option<ResizeAxis>,
}

/// Build one immutable document plan. Geometry inputs must come from the accepted active-board projection.
pub fn plan_resize(input: KeycapResizeInput<'_>) -> Option<KeycapResizePlan> {
    let KeycapResizeInput {
        document,
        matrices,
        scenes,
        layouts,
        placements,
        selected_ids,
        units,
        axis,
    } = input;
    if !finite(units.x) || !finite(units.y) || units.x <= 0.0 || units.y <= 0.0 {
        return None;
    }
    let matrix_by_id: BTreeMap<_, _> = matrices
        .iter()
        .map(|matrix| (matrix.id.as_str(), matrix))
        .collect();
    let placement_by_id: BTreeMap<_, _> = placements
        .iter()
        .map(|placement| (placement.id.as_str(), placement))
        .collect();
    let mut resize_by_cell = BTreeMap::<(String, u32, u32), Resize>::new();
    let mut updated_sizes = BTreeMap::<String, Vec2>::new();

    for placement in placements
        .iter()
        .filter(|placement| selected_ids.contains(&placement.id))
    {
        let owner = layouts
            .iter()
            .find(|layout| layout.matrix_ids.contains(&placement.matrix_id));
        let canonical_layout = owner
            .and_then(|owner| {
                owner
                    .mirror_link
                    .as_ref()
                    .and_then(|link| layouts.iter().find(|layout| layout.id == link.source_id))
            })
            .or(owner);
        let matrix_id = owner
            .and_then(|owner| owner.mirror_link.as_ref())
            .and_then(|link| link.partner_matrix(&placement.matrix_id))
            .unwrap_or(placement.matrix_id.as_str());
        let Some(matrix) = matrix_by_id.get(matrix_id).copied() else {
            continue;
        };
        let source_id = scene_member(scenes, matrix_id, placement.row, placement.column)
            .unwrap_or(placement.id.as_str());
        let Some(source) = placement_by_id.get(source_id).copied() else {
            continue;
        };
        if source.matrix_id != matrix_id || !matrix.part_ids.contains(&source.id) {
            continue;
        }
        let gap = matrix.edge_gap.unwrap_or(Vec2 { x: 1.0, y: 1.0 });
        let next_size = Vec2 {
            x: if axis == Some(ResizeAxis::Y) {
                source.size.x
            } else {
                (units.x * matrix.pitch.x - gap.x).max(1.0)
            },
            y: if axis == Some(ResizeAxis::X) {
                source.size.y
            } else {
                (units.y * matrix.pitch.y - gap.y).max(1.0)
            },
        };
        if near(next_size.x, source.size.x) && near(next_size.y, source.size.y) {
            continue;
        }
        let basis = scenes
            .iter()
            .find(|scene| scene.matrix_id == matrix_id)
            .and_then(|scene| {
                scene
                    .columns
                    .iter()
                    .find(|column| column.column == placement.column)
            });
        let angle = source.rotation.to_radians();
        let mut axis_x = Vec2 {
            x: angle.cos(),
            y: angle.sin(),
        };
        let mut axis_y = Vec2 {
            x: -angle.sin(),
            y: angle.cos(),
        };
        if let Some(basis) = basis {
            if dot(axis_x, basis.axis_x) < 0.0 {
                axis_x = scale(axis_x, -1.0);
            }
            if dot(axis_y, basis.axis_y) < 0.0 {
                axis_y = scale(axis_y, -1.0);
            }
        }
        let resize = Resize {
            placement: source.clone(),
            next_size,
            axis_x,
            axis_y,
        };
        resize_by_cell.insert(
            (matrix_id.to_owned(), placement.row, placement.column),
            resize,
        );
        updated_sizes.insert(source.id.clone(), next_size);

        if let Some(canonical_layout) = canonical_layout
            && let Some(paired) = layouts.iter().find(|layout| {
                layout
                    .mirror_link
                    .as_ref()
                    .is_some_and(|link| link.source_id == canonical_layout.id)
            })
            && let Some(paired_matrix_id) = paired
                .mirror_link
                .as_ref()
                .and_then(|link| link.partner_matrix(matrix_id))
            && let Some(paired_id) =
                scene_member(scenes, paired_matrix_id, placement.row, placement.column)
            && placements.iter().any(|candidate| candidate.id == paired_id)
        {
            updated_sizes.insert(paired_id.to_owned(), next_size);
        }
    }
    if updated_sizes.is_empty() {
        return None;
    }

    let resize_values: Vec<_> = resize_by_cell.values().cloned().collect();
    let mut deltas = reflow(&resize_values, placements);
    let mut updated_matrices = BTreeMap::<String, Matrix>::new();
    for (id, delta) in &deltas {
        let Some(placement) = placement_by_id.get(id.as_str()).copied() else {
            continue;
        };
        let Some(matrix) = updated_matrices
            .get(&placement.matrix_id)
            .or_else(|| matrix_by_id.get(placement.matrix_id.as_str()).copied())
        else {
            continue;
        };
        let existing = matrix
            .cells
            .iter()
            .find(|cell| cell.row == placement.row && cell.column == placement.column)
            .and_then(|cell| cell.offset)
            .unwrap_or(Vec2 { x: 0.0, y: 0.0 });
        let frame = &scenes
            .iter()
            .find(|scene| scene.matrix_id == matrix.id)?
            .cells
            .iter()
            .find(|cell| cell.row == placement.row && cell.column == placement.column)?
            .offset_frame;
        let local = frame.inverse_vector(*delta);
        let mut next = matrix.clone();
        set_cell_offset(
            &mut next,
            placement.row,
            placement.column,
            Vec2 {
                x: existing.x + local.x,
                y: existing.y + local.y,
            },
        );
        updated_matrices.insert(next.id.clone(), next);
    }

    // Cell assembly IDs identify the companions owned by each moved switch. Carry
    // the same world-space translation through the immutable document edit, while
    // the cell offset remains the assembly's shared matrix placement authority.
    let companion_deltas: Vec<_> = deltas
        .iter()
        .filter_map(|(id, delta)| {
            let placement = placement_by_id.get(id.as_str())?;
            let matrix = matrix_by_id.get(placement.matrix_id.as_str())?;
            let cell = matrix
                .cells
                .iter()
                .find(|cell| cell.row == placement.row && cell.column == placement.column)?;
            Some(
                cell.assemblies
                    .iter()
                    .map(move |assembly| (format!("{id}/{}", assembly.id), *delta)),
            )
        })
        .flatten()
        .collect();
    deltas.extend(companion_deltas);

    let next_document = ProjectDoc {
        parts: document
            .parts
            .iter()
            .map(|part| {
                let size = updated_sizes.get(&part.id);
                let delta = deltas.get(&part.id);
                if size.is_none() && delta.is_none() {
                    return part.clone();
                }
                let mut next = part.clone();
                if let Some(size) = size {
                    next.keycap = Some(*size);
                }
                if let Some(delta) = delta {
                    next.pose.at.x += delta.x;
                    next.pose.at.y += delta.y;
                }
                next
            })
            .collect(),
        matrices: document
            .matrices
            .iter()
            .map(|matrix| {
                updated_matrices
                    .get(&matrix.id)
                    .cloned()
                    .unwrap_or_else(|| matrix.clone())
            })
            .collect(),
        ..document.clone()
    };
    let mut targets = BTreeSet::new();
    targets.extend(updated_sizes.keys().cloned());
    targets.extend(deltas.keys().cloned());
    targets.extend(updated_matrices.keys().cloned());
    // Ticket 33: expose the same resize as explicit typed-edit payloads so the
    // controller submits SetKeycapSize instead of a document replacement.
    let mut moves = Vec::new();
    for (id, delta) in &deltas {
        let Some(part) = document.parts.iter().find(|part| &part.id == id) else {
            continue;
        };
        moves.push(Position {
            id: id.clone(),
            at: Vec2 {
                x: part.pose.at.x + delta.x,
                y: part.pose.at.y + delta.y,
            },
        });
    }
    let mut offsets = Vec::new();
    for (matrix_id, updated) in &updated_matrices {
        let original = matrix_by_id.get(matrix_id.as_str()).copied();
        for cell in &updated.cells {
            let before = original.and_then(|matrix| {
                matrix
                    .cells
                    .iter()
                    .find(|candidate| candidate.row == cell.row && candidate.column == cell.column)
                    .and_then(|candidate| candidate.offset)
            });
            if cell.offset != before {
                offsets.push(KeycapCellOffset {
                    matrix_id: (*matrix_id).clone(),
                    row: cell.row,
                    column: cell.column,
                    offset: cell.offset,
                });
            }
        }
    }
    Some(KeycapResizePlan {
        document: next_document,
        target_ids: targets.into_iter().collect(),
        sizes: updated_sizes,
        moves,
        offsets,
    })
}

#[derive(Clone)]
struct Resize {
    placement: KeycapPlacement,
    next_size: Vec2,
    axis_x: Vec2,
    axis_y: Vec2,
}

fn reflow(resizes: &[Resize], placements: &[KeycapPlacement]) -> BTreeMap<String, Vec2> {
    let mut deltas = BTreeMap::<String, Vec2>::new();
    let resize_by_id: BTreeMap<_, _> = resizes
        .iter()
        .map(|resize| (resize.placement.id.as_str(), resize))
        .collect();
    for (row_axis, movement_axis) in [(true, true), (false, false)] {
        let mut groups = BTreeMap::<(String, u32), Vec<&Resize>>::new();
        for resize in resizes {
            let index = if row_axis {
                resize.placement.row
            } else {
                resize.placement.column
            };
            groups
                .entry((resize.placement.matrix_id.clone(), index))
                .or_default()
                .push(resize);
        }
        for ((matrix_id, index), group) in groups {
            let mut members: Vec<_> = placements
                .iter()
                .filter(|placement| {
                    placement.matrix_id == matrix_id
                        && if row_axis {
                            placement.row == index
                        } else {
                            placement.column == index
                        }
                })
                .collect();
            members.sort_by_key(|placement| {
                if row_axis {
                    placement.column
                } else {
                    placement.row
                }
            });
            if members.len() < 2 {
                continue;
            }
            let movement = if movement_axis {
                group[0].axis_x
            } else {
                group[0].axis_y
            };
            let old: Vec<_> = members
                .iter()
                .map(|member| dot(member.at, movement))
                .collect();
            let mut next = vec![old[0]];
            for i in 1..members.len() {
                let prev = members[i - 1];
                let current = members[i];
                let prev_size = resize_by_id
                    .get(prev.id.as_str())
                    .map_or(prev.size, |resize| resize.next_size);
                let curr_size = resize_by_id
                    .get(current.id.as_str())
                    .map_or(current.size, |resize| resize.next_size);
                let old_gap = (old[i]
                    - old[i - 1]
                    - half_extent(prev, movement, prev.size)
                    - half_extent(current, movement, current.size))
                .max(0.0);
                next.push(
                    next[i - 1]
                        + half_extent(prev, movement, prev_size)
                        + half_extent(current, movement, curr_size)
                        + old_gap,
                );
            }
            let selected: Vec<_> = members
                .iter()
                .enumerate()
                .filter_map(|(i, member)| {
                    resize_by_id
                        .get(member.id.as_str())
                        .map(|resize| (i, *member, *resize))
                })
                .collect();
            if selected.is_empty() {
                continue;
            }
            let old_min = selected
                .iter()
                .map(|(_, p, _)| dot(p.at, movement) - half_extent(p, movement, p.size))
                .fold(f64::INFINITY, f64::min);
            let old_max = selected
                .iter()
                .map(|(_, p, _)| dot(p.at, movement) + half_extent(p, movement, p.size))
                .fold(f64::NEG_INFINITY, f64::max);
            let new_min = selected
                .iter()
                .map(|(i, p, r)| next[*i] - half_extent(p, movement, r.next_size))
                .fold(f64::INFINITY, f64::min);
            let new_max = selected
                .iter()
                .map(|(i, p, r)| next[*i] + half_extent(p, movement, r.next_size))
                .fold(f64::NEG_INFINITY, f64::max);
            let correction = (old_min + old_max - new_min - new_max) / 2.0;
            for (i, member) in members.iter().enumerate() {
                let amount = next[i] - old[i] + correction;
                if amount != 0.0 {
                    add_delta(&mut deltas, &member.id, scale(movement, amount));
                }
            }
        }
    }
    let before: Vec<_> = resizes
        .iter()
        .flat_map(|resize| reflow_corners(&resize.placement))
        .collect();
    let after: Vec<_> = resizes
        .iter()
        .flat_map(|resize| {
            let mut changed = resize.placement.clone();
            changed.size = resize.next_size;
            let delta = deltas
                .get(&changed.id)
                .copied()
                .unwrap_or(Vec2 { x: 0.0, y: 0.0 });
            changed.at.x += delta.x;
            changed.at.y += delta.y;
            reflow_corners(&changed)
        })
        .collect();
    if !before.is_empty() && !after.is_empty() {
        let center = |points: &[Vec2]| -> Vec2 {
            let min_x = points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
            let max_x = points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
            let min_y = points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
            let max_y = points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
            Vec2 {
                x: (min_x + max_x) / 2.0,
                y: (min_y + max_y) / 2.0,
            }
        };
        let a = center(&before);
        let b = center(&after);
        let correction = Vec2 {
            x: a.x - b.x,
            y: a.y - b.y,
        };
        if correction.x != 0.0 || correction.y != 0.0 {
            for resize in resizes {
                add_delta(&mut deltas, &resize.placement.id, correction);
            }
            for (id, delta) in &mut deltas {
                if !resize_by_id.contains_key(id.as_str()) {
                    delta.x += correction.x;
                    delta.y += correction.y;
                }
            }
        }
    }
    deltas
}

fn scene_member<'a>(
    scenes: &'a [MatrixScene],
    matrix_id: &str,
    row: u32,
    column: u32,
) -> Option<&'a str> {
    scenes
        .iter()
        .find(|scene| scene.matrix_id == matrix_id)?
        .cells
        .iter()
        .find(|cell| cell.row == row && cell.column == column && cell.enabled)?
        .member_id
        .as_deref()
}

//! Private accepted-document planner for the Layout key-size interaction.
use boardstudio_core::model::{
    KeycapCellOffset, Layout, Matrix, MatrixCell, MatrixScene, Position, ProjectDoc, Vec2,
};
use std::collections::{BTreeMap, BTreeSet};

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

fn overlaps_on_axes(first: &[Vec2; 4], second: &[Vec2; 4], source: &[Vec2; 4]) -> bool {
    for index in 0..source.len() {
        let start = source[index];
        let end = source[(index + 1) % source.len()];
        let edge_x = end.x - start.x;
        let edge_y = end.y - start.y;
        let length = edge_x.hypot(edge_y);
        if length == 0.0 {
            continue;
        }
        let axis = Vec2 {
            x: -edge_y / length,
            y: edge_x / length,
        };
        let (first_min, first_max) = project_polygon(first, axis);
        let (second_min, second_max) = project_polygon(second, axis);
        if first_max <= second_min + 1e-7 || second_max <= first_min + 1e-7 {
            return false;
        }
    }
    true
}

fn project_polygon(points: &[Vec2; 4], axis: Vec2) -> (f64, f64) {
    let mut projections = points.iter().map(|point| dot(*point, axis));
    let first = projections.next().unwrap_or_default();
    projections.fold((first, first), |(min, max), value| {
        (min.min(value), max.max(value))
    })
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
        .flat_map(|resize| corners(&resize.placement))
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
            corners(&changed)
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
fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}
fn scale(a: Vec2, factor: f64) -> Vec2 {
    Vec2 {
        x: a.x * factor,
        y: a.y * factor,
    }
}
fn finite(value: f64) -> bool {
    value.is_finite()
}
fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}
fn add_delta(map: &mut BTreeMap<String, Vec2>, id: &str, delta: Vec2) {
    let entry = map.entry(id.to_owned()).or_insert(Vec2 { x: 0.0, y: 0.0 });
    entry.x += delta.x;
    entry.y += delta.y;
}
fn half_extent(placement: &KeycapPlacement, axis: Vec2, size: Vec2) -> f64 {
    let angle = placement.rotation.to_radians();
    let x = Vec2 {
        x: angle.cos(),
        y: angle.sin(),
    };
    let y = Vec2 {
        x: -angle.sin(),
        y: angle.cos(),
    };
    (dot(x, axis).abs() * size.x + dot(y, axis).abs() * size.y) / 2.0
}
fn corners(placement: &KeycapPlacement) -> [Vec2; 4] {
    let angle = placement.rotation.to_radians();
    let (sin, cos) = angle.sin_cos();
    [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)].map(|(x, y)| Vec2 {
        x: placement.at.x + x * placement.size.x * cos - y * placement.size.y * sin,
        y: placement.at.y + x * placement.size.x * sin + y * placement.size.y * cos,
    })
}
fn set_cell_offset(matrix: &mut Matrix, row: u32, column: u32, offset: Vec2) {
    if let Some(cell) = matrix
        .cells
        .iter_mut()
        .find(|cell| cell.row == row && cell.column == column)
    {
        cell.offset = Some(offset);
    } else {
        matrix.cells.push(MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            row,
            column,
            enabled: true,
            definition_id: None,
            variant: None,
            offset: Some(offset),
            rotation: None,
            assemblies: vec![],
            assemblies_local: None,
        });
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::{
        Board, CoreReply, CoreRequest, EditCommand, EditOperation, EditPhase, MatrixCell,
        MatrixColumnBasis, MatrixSceneCell, Mirror, Part, Pose2, Side,
    };

    fn point(x: f64, y: f64) -> Vec2 {
        Vec2 { x, y }
    }

    fn part(id: &str, x: f64) -> Part {
        Part {
            keycap: Some(point(18.0, 18.0)),
            outline: None,
            id: id.into(),
            definition_id: "switch".into(),
            reference: id.into(),
            pose: Pose2 {
                at: point(x, 0.0),
                rotation: 0.0,
            },
            side: Side::Front,
            anchor: None,
            locked: None,
            properties: None,
            generator_parameters: None,
        }
    }

    fn matrix() -> Matrix {
        Matrix {
            id: "m".into(),
            name: Some("Keys".into()),
            rows: 1,
            columns: 3,
            pitch: point(19.0, 19.0),
            origin: point(0.0, 0.0),
            definition_id: "switch".into(),
            part_ids: vec!["a".into(), "b".into(), "c".into()],
            board_id: Some("board".into()),
            mirror: None,
            rotation: None,
            edge_gap: Some(point(1.0, 1.0)),
            diode_direction: None,
            row_layout: vec![],
            column_layout: vec![],
            anchor: None,
            cells: vec![0, 1, 2]
                .into_iter()
                .map(|column| MatrixCell {
                    shift: None,
                    bend: None,
                    deleted: false,
                    row: 0,
                    column,
                    enabled: true,
                    definition_id: None,
                    variant: None,
                    offset: None,
                    rotation: None,
                    assemblies: vec![],
                    assemblies_local: None,
                })
                .collect(),
        }
    }

    fn document(matrix: &Matrix) -> ProjectDoc {
        let mut document = ProjectDoc::empty("project", "Test");
        document.revision = 6;
        document.parts = vec![part("a", 0.0), part("b", 19.0), part("c", 38.0)];
        document.matrices = vec![matrix.clone()];
        document.boards = vec![Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: matrix.part_ids.clone(),
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        }];
        document
    }

    fn scene() -> MatrixScene {
        MatrixScene {
            matrix_id: "m".into(),
            cells: (0..3)
                .map(|column| MatrixSceneCell {
                    shift_frame: Default::default(),
                    offset_frame: Default::default(),
                    row_shift: Default::default(),
                    key_shift: Default::default(),
                    row_bend: 0.0,
                    key_bend: 0.0,
                    cumulative_shift: Default::default(),
                    cumulative_bend: 0.0,
                    row: 0,
                    column,
                    enabled: true,
                    member_id: Some(["a", "b", "c"][column as usize].into()),
                    pose: Pose2 {
                        at: point(column as f64 * 19.0, 0.0),
                        rotation: 0.0,
                    },
                })
                .collect(),
            columns: (0..3)
                .map(|column| MatrixColumnBasis {
                    base: Default::default(),
                    rotation: 0.0,
                    relative_origin: Default::default(),
                    spread_axis: None,
                    column,
                    splay_origin: point(0.0, 0.0),
                    splay_angle: 0.0,
                    custom_origin: false,
                    axis_x: point(1.0, 0.0),
                    axis_y: point(0.0, 1.0),
                    stagger_axis: point(0.0, 1.0),
                })
                .collect(),
            companions: vec![],
        }
    }

    fn placements() -> Vec<KeycapPlacement> {
        ["a", "b", "c"]
            .into_iter()
            .enumerate()
            .map(|(column, id)| KeycapPlacement {
                id: id.into(),
                matrix_id: "m".into(),
                row: 0,
                column: column as u32,
                at: point(column as f64 * 19.0, 0.0),
                rotation: 0.0,
                size: point(18.0, 18.0),
            })
            .collect()
    }

    fn overlap_placement(
        id: &str,
        matrix_id: &str,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        rotation: f64,
    ) -> KeycapPlacement {
        KeycapPlacement {
            id: id.into(),
            matrix_id: matrix_id.into(),
            row: 0,
            column: 0,
            at: point(x, y),
            rotation,
            size: point(width, height),
        }
    }

    #[test]
    fn overlap_query_ignores_edge_contact_and_keeps_cross_matrix_pairs() {
        let touching = vec![
            overlap_placement("a", "m1", 0.0, 0.0, 18.0, 18.0, 0.0),
            overlap_placement("b", "m2", 18.0, 0.0, 18.0, 18.0, 0.0),
        ];
        assert!(overlapping_pairs(&touching).is_empty());

        let crossing = vec![
            overlap_placement("a", "m1", 0.0, 0.0, 18.0, 18.0, 0.0),
            overlap_placement("b", "m2", 17.999, 0.0, 18.0, 18.0, 0.0),
        ];
        assert_eq!(overlapping_pairs(&crossing), vec![("a".into(), "b".into())]);
    }

    #[test]
    fn overlap_query_uses_oriented_rectangles_and_stable_equal_min_x_order() {
        // These thin rotated envelopes have overlapping axis-aligned bounds, but
        // their oriented rectangles are separated along the short axis.
        let separated = vec![
            overlap_placement("upper", "m1", 0.0, 0.0, 18.0, 4.0, 45.0),
            overlap_placement("lower", "m2", -5.656854, 5.656854, 18.0, 4.0, 45.0),
        ];
        assert!(overlapping_pairs(&separated).is_empty());

        let placements = vec![
            overlap_placement("first", "m2", 0.0, 0.0, 18.0, 4.0, 45.0),
            overlap_placement("second", "m1", 0.0, 0.0, 18.0, 4.0, 45.0),
            overlap_placement("third", "m3", 0.0, 0.0, 18.0, 4.0, 45.0),
        ];
        assert_eq!(
            overlapping_pairs(&placements),
            vec![
                ("first".into(), "second".into()),
                ("first".into(), "third".into()),
                ("second".into(), "third".into()),
            ]
        );
    }

    #[test]
    fn widening_selected_key_reflows_row_neighbors_and_preserves_group_center() {
        let matrix = matrix();
        let document = document(&matrix);
        let matrix_scene = scene();
        let placements = placements();
        let selected_ids = BTreeSet::from(["b".into()]);
        let plan = plan_resize(KeycapResizeInput {
            document: &document,
            matrices: &[matrix],
            scenes: &[matrix_scene],
            layouts: &[],
            placements: &placements,
            selected_ids: &selected_ids,
            units: point(2.0, 1.0),
            axis: None,
        })
        .expect("supported accepted selection should make one plan");

        assert_eq!(plan.document.parts[1].keycap, Some(point(37.0, 18.0)));
        assert!((plan.document.parts[0].pose.at.x + 9.5).abs() < 1e-8);
        assert!((plan.document.parts[2].pose.at.x - 47.5).abs() < 1e-8);
        let matrix = plan
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "m")
            .unwrap();
        let changed_cells: Vec<_> = matrix
            .cells
            .iter()
            .filter(|cell| cell.offset.is_some())
            .collect();
        assert_eq!(changed_cells.len(), 2);
        assert_eq!(plan.document.revision, 6);
        assert_eq!(plan.target_ids, vec!["a", "b", "c", "m"]);
    }

    /// Ticket 33: the planner exposes the same resize as typed-edit payloads,
    /// so the controller submits SetKeycapSize instead of ReplaceDocument.
    #[test]
    fn resize_plan_exposes_typed_keycap_size_sizes_moves_and_offsets() {
        let matrix = matrix();
        let document = document(&matrix);
        let matrix_scene = scene();
        let placements = placements();
        let selected_ids = BTreeSet::from(["b".into()]);
        let plan = plan_resize(KeycapResizeInput {
            document: &document,
            matrices: &[matrix],
            scenes: &[matrix_scene],
            layouts: &[],
            placements: &placements,
            selected_ids: &selected_ids,
            units: point(2.0, 1.0),
            axis: None,
        })
        .expect("supported accepted selection should make one plan");

        assert_eq!(
            plan.sizes,
            BTreeMap::from([("b".to_owned(), point(37.0, 18.0))])
        );
        // Absolute poses match the planned document exactly, in sorted id order.
        let expected_moves: Vec<(String, Vec2)> = plan
            .document
            .parts
            .iter()
            .filter(|part| {
                document
                    .parts
                    .iter()
                    .find(|before| before.id == part.id)
                    .is_some_and(|before| before.pose.at != part.pose.at)
            })
            .map(|part| (part.id.clone(), part.pose.at))
            .collect();
        assert_eq!(
            plan.moves
                .iter()
                .map(|position| (position.id.clone(), position.at))
                .collect::<Vec<_>>(),
            expected_moves
        );
        // Every changed cell offset matches the planned matrix exactly.
        let planned: Vec<_> = plan
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "m")
            .unwrap()
            .cells
            .iter()
            .filter(|cell| cell.offset.is_some())
            .map(|cell| (cell.row, cell.column, cell.offset.unwrap()))
            .collect();
        assert_eq!(
            plan.offsets
                .iter()
                .map(|offset| (offset.row, offset.column, offset.offset.unwrap()))
                .collect::<Vec<_>>(),
            planned
        );
        assert!(plan.offsets.iter().all(|offset| offset.matrix_id == "m"));
    }

    #[test]
    fn axis_resize_preserves_other_axis_for_each_mixed_selected_key() {
        let matrix = matrix();
        let mut document = document(&matrix);
        document.parts[0].keycap = Some(point(18.0, 37.0));
        let mut placements = placements();
        placements[0].size = point(18.0, 37.0);
        let matrix_scene = scene();
        let selected_ids = BTreeSet::from(["a".into(), "b".into()]);
        let plan = plan_resize(KeycapResizeInput {
            document: &document,
            matrices: &[matrix],
            scenes: &[matrix_scene],
            layouts: &[],
            placements: &placements,
            selected_ids: &selected_ids,
            units: point(2.0, 4.0),
            axis: Some(ResizeAxis::X),
        })
        .expect("a supported width change should plan");
        assert_eq!(plan.document.parts[0].keycap, Some(point(37.0, 37.0)));
        assert_eq!(plan.document.parts[1].keycap, Some(point(37.0, 18.0)));
    }

    #[test]
    fn invalid_units_and_unchanged_selection_create_no_plan() {
        let matrix = matrix();
        let document = document(&matrix);
        let placements = placements();
        assert!(
            plan_resize(KeycapResizeInput {
                document: &document,
                matrices: std::slice::from_ref(&matrix),
                scenes: &[scene()],
                layouts: &[],
                placements: &placements,
                selected_ids: &BTreeSet::from(["b".into()]),
                units: point(f64::NAN, 1.0),
                axis: None,
            })
            .is_none()
        );
        assert!(
            plan_resize(KeycapResizeInput {
                document: &document,
                matrices: &[matrix],
                scenes: &[scene()],
                layouts: &[],
                placements: &placements,
                selected_ids: &BTreeSet::from(["b".into()]),
                units: point(1.0, 1.0),
                axis: None,
            })
            .is_none()
        );
    }

    #[test]
    fn tall_resize_reflows_same_column_on_projected_y_axis() {
        let mut matrix = matrix();
        matrix.rows = 3;
        matrix.columns = 1;
        matrix.part_ids = vec!["a".into(), "b".into(), "c".into()];
        matrix.cells = (0..3)
            .map(|row| MatrixCell {
                shift: None,
                bend: None,
                deleted: false,
                row,
                column: 0,
                enabled: true,
                definition_id: None,
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![],
                assemblies_local: None,
            })
            .collect();
        let mut document = document(&matrix);
        for (index, part) in document.parts.iter_mut().enumerate() {
            part.pose.at = point(0.0, index as f64 * 19.0);
        }
        document.boards[0].part_ids = matrix.part_ids.clone();
        let scene = MatrixScene {
            matrix_id: "m".into(),
            cells: (0..3)
                .map(|row| MatrixSceneCell {
                    shift_frame: Default::default(),
                    offset_frame: Default::default(),
                    row_shift: Default::default(),
                    key_shift: Default::default(),
                    row_bend: 0.0,
                    key_bend: 0.0,
                    cumulative_shift: Default::default(),
                    cumulative_bend: 0.0,
                    row,
                    column: 0,
                    enabled: true,
                    member_id: Some(["a", "b", "c"][row as usize].into()),
                    pose: Pose2 {
                        at: point(0.0, row as f64 * 19.0),
                        rotation: 0.0,
                    },
                })
                .collect(),
            columns: vec![MatrixColumnBasis {
                base: Default::default(),
                rotation: 0.0,
                relative_origin: Default::default(),
                spread_axis: None,
                column: 0,
                splay_origin: point(0.0, 0.0),
                splay_angle: 0.0,
                custom_origin: false,
                axis_x: point(1.0, 0.0),
                axis_y: point(0.0, 1.0),
                stagger_axis: point(0.0, 1.0),
            }],
            companions: vec![],
        };
        let placements: Vec<_> = ["a", "b", "c"]
            .into_iter()
            .enumerate()
            .map(|(row, id)| KeycapPlacement {
                id: id.into(),
                matrix_id: "m".into(),
                row: row as u32,
                column: 0,
                at: point(0.0, row as f64 * 19.0),
                rotation: 0.0,
                size: point(18.0, 18.0),
            })
            .collect();
        let selected_ids = BTreeSet::from(["b".into()]);
        let plan = plan_resize(KeycapResizeInput {
            document: &document,
            matrices: &[matrix],
            scenes: &[scene],
            layouts: &[],
            placements: &placements,
            selected_ids: &selected_ids,
            units: point(1.0, 2.0),
            axis: None,
        })
        .unwrap();
        assert_eq!(plan.document.parts[1].keycap, Some(point(18.0, 37.0)));
        assert!((plan.document.parts[0].pose.at.y + 9.5).abs() < 1e-8);
        assert!((plan.document.parts[2].pose.at.y - 47.5).abs() < 1e-8);
    }

    #[test]
    fn tall_resize_moves_neighbor_assembly_parts_with_their_switch() {
        let mut matrix = matrix();
        matrix.rows = 3;
        matrix.columns = 1;
        matrix.part_ids = vec!["a".into(), "b".into(), "c".into()];
        matrix.cells = (0..3)
            .map(|row| MatrixCell {
                shift: None,
                bend: None,
                deleted: false,
                row,
                column: 0,
                enabled: true,
                definition_id: None,
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![],
                assemblies_local: None,
            })
            .collect();
        for cell in &mut matrix.cells {
            cell.assemblies = ["diode", "led"]
                .into_iter()
                .map(|id| boardstudio_core::model::MatrixAssembly {
                    id: id.into(),
                    definition_id: "switch".into(),
                    offset: point(3.0, 4.0),
                    rotation: None,
                    side: None,
                })
                .collect();
        }
        let mut document = document(&matrix);
        for (index, part) in document.parts.iter_mut().enumerate() {
            part.pose.at = point(0.0, index as f64 * 19.0);
        }
        for (index, id) in ["a", "b", "c"].into_iter().enumerate() {
            for suffix in ["diode", "led"] {
                let mut companion = part(&format!("{id}/{suffix}"), 3.0);
                companion.pose.at.y = index as f64 * 19.0 + 4.0;
                companion.keycap = None;
                document.parts.push(companion);
            }
        }
        document.boards[0].part_ids = document.parts.iter().map(|part| part.id.clone()).collect();
        let scene = MatrixScene {
            matrix_id: "m".into(),
            cells: (0..3)
                .map(|row| MatrixSceneCell {
                    shift_frame: Default::default(),
                    offset_frame: Default::default(),
                    row_shift: Default::default(),
                    key_shift: Default::default(),
                    row_bend: 0.0,
                    key_bend: 0.0,
                    cumulative_shift: Default::default(),
                    cumulative_bend: 0.0,
                    row,
                    column: 0,
                    enabled: true,
                    member_id: Some(["a", "b", "c"][row as usize].into()),
                    pose: Pose2 {
                        at: point(0.0, row as f64 * 19.0),
                        rotation: 0.0,
                    },
                })
                .collect(),
            columns: vec![MatrixColumnBasis {
                base: Default::default(),
                rotation: 0.0,
                relative_origin: Default::default(),
                spread_axis: None,
                column: 0,
                splay_origin: point(0.0, 0.0),
                splay_angle: 0.0,
                custom_origin: false,
                axis_x: point(1.0, 0.0),
                axis_y: point(0.0, 1.0),
                stagger_axis: point(0.0, 1.0),
            }],
            companions: vec![],
        };
        let placements: Vec<_> = ["a", "b", "c"]
            .into_iter()
            .enumerate()
            .map(|(row, id)| KeycapPlacement {
                id: id.into(),
                matrix_id: "m".into(),
                row: row as u32,
                column: 0,
                at: point(0.0, row as f64 * 19.0),
                rotation: 0.0,
                size: point(18.0, 18.0),
            })
            .collect();
        let selected_ids = BTreeSet::from(["b".into()]);
        let plan = plan_resize(KeycapResizeInput {
            document: &document,
            matrices: &[matrix],
            scenes: &[scene],
            layouts: &[],
            placements: &placements,
            selected_ids: &selected_ids,
            units: point(1.0, 2.0),
            axis: None,
        })
        .unwrap();
        for id in ["a", "b", "c"] {
            let host = plan
                .document
                .parts
                .iter()
                .find(|part| part.id == id)
                .unwrap();
            for suffix in ["diode", "led"] {
                let companion = plan
                    .document
                    .parts
                    .iter()
                    .find(|part| part.id == format!("{id}/{suffix}"))
                    .unwrap();
                assert!((companion.pose.at.x - host.pose.at.x - 3.0).abs() < 1e-8);
                assert!(
                    (companion.pose.at.y - host.pose.at.y - 4.0).abs() < 1e-8,
                    "{id}/{suffix} must follow its switch during key-size reflow"
                );
            }
        }
        assert_eq!(plan.document.parts[1].keycap, Some(point(18.0, 37.0)));
        assert!((plan.document.parts[0].pose.at.y + 9.5).abs() < 1e-8);
        assert!((plan.document.parts[2].pose.at.y - 47.5).abs() < 1e-8);
    }

    #[test]
    fn accepted_rotated_and_mirrored_resize_preserves_assemblies_through_history() {
        let unpack = |reply| match reply {
            CoreReply::Scene {
                document, scene, ..
            } => (*document, scene),
            other => panic!("Core edit did not settle: {other:?}"),
        };
        for mirror in [None, Some(Mirror::X)] {
            let mut engine = boardstudio_core::CoreEngine::new();
            let mut initial = ProjectDoc::empty("project", "Assembly resize");
            initial.boards = document(&matrix()).boards;
            initial.boards[0].part_ids.clear();
            initial.definitions.push(serde_json::from_value(serde_json::json!({
                "id": "switch", "name": "Switch", "kind": "switch",
                "courtyard": [{"x": -3.0, "y": -3.0}, {"x": 3.0, "y": -3.0}, {"x": 3.0, "y": 3.0}],
                "pads": []
            })).unwrap());
            unpack(engine.handle(CoreRequest::Open {
                id: "open".into(),
                document: initial,
            }));
            let mut matrix = matrix();
            matrix.rows = 3;
            matrix.columns = 1;
            matrix.rotation = Some(23.0);
            matrix.mirror = mirror;
            matrix.part_ids.clear();
            matrix.cells = (0..3)
                .map(|row| MatrixCell {
                    shift: None,
                    bend: None,
                    deleted: false,
                    row,
                    column: 0,
                    enabled: true,
                    definition_id: None,
                    variant: None,
                    offset: None,
                    rotation: None,
                    assemblies_local: None,
                    assemblies: ["diode", "led"]
                        .into_iter()
                        .map(|id| boardstudio_core::model::MatrixAssembly {
                            id: id.into(),
                            definition_id: "switch".into(),
                            offset: point(3.0, 4.0),
                            rotation: Some(17.0),
                            side: Some(Side::Back),
                        })
                        .collect(),
                })
                .collect();
            let (accepted, scene) = unpack(engine.handle(CoreRequest::Edit {
                id: "create-matrix".into(),
                command: EditCommand {
                    base_revision: 0,
                    transaction_id: "create".into(),
                    phase: EditPhase::Commit,
                    target_ids: vec![matrix.id.clone()],
                    operation: EditOperation::SetMatrix {
                        matrix,
                        definitions: None,
                    },
                },
            }));
            let placements: Vec<_> = scene.matrix_scenes[0]
                .cells
                .iter()
                .map(|cell| KeycapPlacement {
                    id: cell.member_id.clone().unwrap(),
                    matrix_id: "m".into(),
                    row: cell.row,
                    column: cell.column,
                    at: cell.pose.at,
                    rotation: cell.pose.rotation,
                    size: point(18.0, 18.0),
                })
                .collect();
            let selected = BTreeSet::from([placements[1].id.clone()]);
            let plan = plan_resize(KeycapResizeInput {
                document: &accepted,
                matrices: &accepted.matrices,
                scenes: &scene.matrix_scenes,
                layouts: &accepted.layouts,
                placements: &placements,
                selected_ids: &selected,
                units: point(1.0, 2.0),
                axis: Some(ResizeAxis::Y),
            })
            .unwrap();
            let (saved, saved_scene) = unpack(engine.handle(CoreRequest::Edit {
                id: "resize".into(),
                command: EditCommand {
                    base_revision: accepted.revision,
                    transaction_id: "resize".into(),
                    phase: EditPhase::Commit,
                    // Ticket 33: the planner output commits as the typed edit.
                    target_ids: plan.target_ids.clone(),
                    operation: EditOperation::SetKeycapSize {
                        sizes: plan.sizes.clone(),
                        moves: plan.moves.clone(),
                        offsets: plan.offsets.clone(),
                    },
                },
            }));
            for placement in &placements {
                let original_host = accepted
                    .parts
                    .iter()
                    .find(|part| part.id == placement.id)
                    .unwrap();
                let host = saved
                    .parts
                    .iter()
                    .find(|part| part.id == placement.id)
                    .unwrap();
                let projected = saved_scene.matrix_scenes[0]
                    .cells
                    .iter()
                    .find(|cell| cell.row == placement.row)
                    .unwrap();
                assert!((projected.pose.at.x - host.pose.at.x).abs() < 1e-8);
                assert!((projected.pose.at.y - host.pose.at.y).abs() < 1e-8);
                for suffix in ["diode", "led"] {
                    let id = format!("{}/{suffix}", placement.id);
                    let before = accepted.parts.iter().find(|part| part.id == id).unwrap();
                    let after = saved.parts.iter().find(|part| part.id == id).unwrap();
                    assert!(
                        (after.pose.at.x - host.pose.at.x - before.pose.at.x
                            + original_host.pose.at.x)
                            .abs()
                            < 1e-8
                    );
                    assert!(
                        (after.pose.at.y - host.pose.at.y - before.pose.at.y
                            + original_host.pose.at.y)
                            .abs()
                            < 1e-8,
                        "{id} must retain its assembly offset in the accepted document"
                    );
                    assert_eq!(after.pose.rotation, before.pose.rotation);
                    assert_eq!(after.side, before.side);
                }
            }
            let (undone, _) = unpack(engine.handle(CoreRequest::Undo { id: "undo".into() }));
            assert_eq!(undone.parts, accepted.parts);
            assert_eq!(undone.matrices, accepted.matrices);
            let (redone, _) = unpack(engine.handle(CoreRequest::Redo { id: "redo".into() }));
            assert_eq!(redone.parts, saved.parts);
            assert_eq!(redone.matrices, saved.matrices);
        }
    }

    #[test]
    fn replace_document_reflows_source_and_core_syncs_linked_partner_through_history() {
        use boardstudio_core::model::{LayoutMirrorLink, MirroredCorrespondence, MirroredItemKind};

        let mut source = matrix();
        source.origin = point(-80.0, 10.0);
        source.part_ids = (0..3)
            .map(|column| format!("matrix/m/r0c{column}"))
            .collect();
        let mut partner = source.clone();
        partner.id = "partner".into();
        partner.origin = point(80.0, 10.0);
        partner.mirror = Some(Mirror::X);
        partner.part_ids = (0..3)
            .map(|column| format!("matrix/partner/r0c{column}"))
            .collect();
        let mut initial = document(&source);
        initial.revision = 0;
        initial.parts.clear();
        for matrix in [&source, &partner] {
            for (column, id) in matrix.part_ids.iter().enumerate() {
                let direction = if matrix.id == "m" { 1.0 } else { -1.0 };
                let mut member = part(id, matrix.origin.x + direction * column as f64 * 19.0);
                member.pose.at.y = 10.0;
                initial.parts.push(member);
            }
        }
        initial.boards[0].part_ids = initial.parts.iter().map(|p| p.id.clone()).collect();
        initial.matrices = vec![source, partner];
        initial.definitions.push(
            serde_json::from_value(serde_json::json!({
                "id": "switch", "name": "Switch", "kind": "switch",
                "keycap": {"x": 18.0, "y": 18.0},
                "courtyard": [{"x": -3.0, "y": -3.0}, {"x": 3.0, "y": -3.0}, {"x": 3.0, "y": 3.0}],
                "pads": []
            }))
            .unwrap(),
        );
        initial.layouts = vec![
            Layout {
                id: "left".into(),
                name: "Left".into(),
                board_id: "board".into(),
                matrix_ids: vec!["m".into()],
                part_ids: vec![],
                mirror_link: None,
            },
            Layout {
                id: "right".into(),
                name: "Right".into(),
                board_id: "board".into(),
                matrix_ids: vec!["partner".into()],
                part_ids: vec![],
                mirror_link: Some(LayoutMirrorLink {
                    source_id: "left".into(),
                    axis_x: 0.0,
                    correspondences: vec![MirroredCorrespondence {
                        kind: MirroredItemKind::Matrix,
                        source_id: "m".into(),
                        mirror_id: "partner".into(),
                        companion_mode: None,
                        companion_tweak: None,
                    }],
                    exceptions: vec![],
                    split_gap: None,
                    spacing: None,
                }),
            },
        ];
        let unpack = |reply| match reply {
            CoreReply::Scene {
                document, scene, ..
            } => (*document, scene),
            other => panic!("linked resize did not settle: {other:?}"),
        };
        let mut engine = boardstudio_core::CoreEngine::new();
        let (accepted, scene) = unpack(engine.handle(CoreRequest::Open {
            id: "open".into(),
            document: initial,
        }));

        // Capture all cell identities before editing, not from the resize result.
        let members = |document: &ProjectDoc, scenes: &[MatrixScene]| {
            let mut result = BTreeMap::new();
            for matrix_id in ["m", "partner"] {
                let matrix = document
                    .matrices
                    .iter()
                    .find(|m| m.id == matrix_id)
                    .unwrap();
                let scene = scenes.iter().find(|s| s.matrix_id == matrix_id).unwrap();
                assert_eq!(scene.cells.len(), 3);
                assert_eq!(matrix.part_ids.len(), 3);
                for cell in &scene.cells {
                    assert!(cell.enabled);
                    let id = cell.member_id.clone().expect("enabled cell has a member");
                    assert!(matrix.part_ids.contains(&id));
                    assert_eq!(document.parts.iter().filter(|p| p.id == id).count(), 1);
                    assert!(
                        result
                            .insert((matrix_id.to_owned(), cell.row, cell.column), id)
                            .is_none()
                    );
                }
            }
            assert_eq!(result.len(), 6);
            assert_eq!(result.values().collect::<BTreeSet<_>>().len(), 6);
            result
        };
        let layout_metadata = |document: &ProjectDoc| {
            ["left", "right"].map(|id| {
                assert_eq!(document.layouts.iter().filter(|l| l.id == id).count(), 1);
                let layout = document.layouts.iter().find(|l| l.id == id).unwrap();
                (
                    layout.id.clone(),
                    layout.board_id.clone(),
                    layout.matrix_ids.clone(),
                    layout.part_ids.clone(),
                    layout.mirror_link.clone(),
                )
            })
        };
        let accepted_metadata = layout_metadata(&accepted);
        let axis_x = accepted_metadata[1].4.as_ref().unwrap().axis_x;
        assert!(accepted_metadata[0].4.is_none());
        assert_eq!(accepted_metadata[1].4.as_ref().unwrap().split_gap, None);
        assert_eq!(accepted_metadata[1].4.as_ref().unwrap().spacing, None);
        let mut expected_committed_metadata = accepted_metadata.clone();
        // Legacy half-origin gap: 2 * (0 - (-80)) - (19 - 1) = 142 mm.
        // Adoption preserves placement; these separated halves stay unspaced.
        expected_committed_metadata[1].4.as_mut().unwrap().split_gap = Some(142.0);
        let original_members = members(&accepted, &scene.matrix_scenes);
        let snapshot = |document: &ProjectDoc, scenes: &[MatrixScene]| {
            assert_eq!(members(document, scenes), original_members);
            let member_state = original_members
                .iter()
                .map(|((matrix_id, row, column), id)| {
                    let matrix = document
                        .matrices
                        .iter()
                        .find(|m| &m.id == matrix_id)
                        .unwrap();
                    let cell = matrix
                        .cells
                        .iter()
                        .find(|c| c.row == *row && c.column == *column)
                        .unwrap();
                    let scene_cell = scenes
                        .iter()
                        .find(|s| &s.matrix_id == matrix_id)
                        .unwrap()
                        .cells
                        .iter()
                        .find(|c| c.row == *row && c.column == *column)
                        .unwrap();
                    let part = document.parts.iter().find(|p| &p.id == id).unwrap();
                    assert_eq!(part.pose, scene_cell.pose, "document/scene pose for {id}");
                    (
                        matrix_id.clone(),
                        *row,
                        *column,
                        id.clone(),
                        part.pose,
                        part.keycap,
                        cell.offset,
                    )
                })
                .collect::<Vec<_>>();
            (member_state, layout_metadata(document))
        };
        let before = snapshot(&accepted, &scene.matrix_scenes);
        let mut accepted_placements = Vec::new();
        for matrix in &accepted.matrices {
            let matrix_scene = scene
                .matrix_scenes
                .iter()
                .find(|s| s.matrix_id == matrix.id)
                .unwrap();
            for cell in &matrix_scene.cells {
                let id = cell.member_id.as_ref().unwrap();
                let part = accepted.parts.iter().find(|p| &p.id == id).unwrap();
                let definition = accepted
                    .definitions
                    .iter()
                    .find(|d| d.id == part.definition_id)
                    .unwrap();
                accepted_placements.push(KeycapPlacement {
                    id: id.clone(),
                    matrix_id: matrix.id.clone(),
                    row: cell.row,
                    column: cell.column,
                    at: cell.pose.at,
                    rotation: cell.pose.rotation,
                    size: part.keycap.or(definition.keycap).unwrap(),
                });
            }
        }
        assert_eq!(accepted_placements.len(), 6);
        assert!(
            accepted_placements
                .iter()
                .all(|p| p.size == point(18.0, 18.0))
        );
        let selected_id = original_members[&("m".into(), 0, 1)].clone();
        let plan = plan_resize(KeycapResizeInput {
            document: &accepted,
            matrices: &accepted.matrices,
            scenes: &scene.matrix_scenes,
            layouts: &accepted.layouts,
            placements: &accepted_placements,
            selected_ids: &BTreeSet::from([selected_id]),
            units: point(3.0, 1.0),
            axis: None,
        })
        .expect("linked source selection should make a plan");
        let (saved, saved_scene) = unpack(engine.handle(CoreRequest::Edit {
            id: "resize".into(),
            command: EditCommand {
                base_revision: accepted.revision,
                transaction_id: "keycaps-typed-mirror".into(),
                phase: EditPhase::Commit,
                target_ids: plan.target_ids.clone(),
                operation: EditOperation::SetKeycapSize {
                    sizes: plan.sizes.clone(),
                    moves: plan.moves.clone(),
                    offsets: plan.offsets.clone(),
                },
            },
        }));
        assert_eq!(saved.revision, accepted.revision + 1);
        assert_eq!(
            layout_metadata(&saved),
            expected_committed_metadata,
            "resize preserves ownership and mirror links with expected gap adoption"
        );
        let after = snapshot(&saved, &saved_scene.matrix_scenes);
        assert_ne!(after, before);
        for column in 0..3 {
            let source_id = &original_members[&("m".into(), 0, column)];
            let partner_id = &original_members[&("partner".into(), 0, column)];
            let original = accepted.parts.iter().find(|p| &p.id == source_id).unwrap();
            let source = saved.parts.iter().find(|p| &p.id == source_id).unwrap();
            let partner = saved.parts.iter().find(|p| &p.id == partner_id).unwrap();
            let displacement = [-19.0, 0.0, 19.0][column as usize];
            assert!(
                (source.pose.at.x - original.pose.at.x - displacement).abs() < 1e-6,
                "source column {column} must reflow by {displacement}"
            );
            assert!((source.pose.at.y - original.pose.at.y).abs() < 1e-6);
            assert_eq!(source.pose.rotation, original.pose.rotation);
            let expected_size = Some(point(if column == 1 { 56.0 } else { 18.0 }, 18.0));
            assert_eq!(source.keycap, expected_size);
            assert_eq!(
                partner.keycap, expected_size,
                "linked partner size at column {column}"
            );
            assert!(
                (source.pose.at.x + partner.pose.at.x - 2.0 * axis_x).abs() < 1e-6,
                "linked partner reflection at column {column}"
            );
            assert!((source.pose.at.y - partner.pose.at.y).abs() < 1e-6);
            assert!((source.pose.rotation + partner.pose.rotation).rem_euclid(360.0) < 1e-6);
        }

        let (undone, undone_scene) = unpack(engine.handle(CoreRequest::Undo { id: "undo".into() }));
        assert_eq!(
            snapshot(&undone, &undone_scene.matrix_scenes),
            before,
            "Undo restores both halves"
        );
        let (redone, redone_scene) = unpack(engine.handle(CoreRequest::Redo { id: "redo".into() }));
        assert_eq!(
            snapshot(&redone, &redone_scene.matrix_scenes),
            after,
            "Redo restores both halves"
        );
    }
}

//! Private accepted-document planner for the Layout key-size interaction.
use boardstudio_core::model::{Layout, Matrix, MatrixCell, MatrixScene, Mirror, ProjectDoc, Vec2};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::presentation) enum ResizeAxis {
    X,
    Y,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct KeycapPlacement {
    pub(in crate::presentation) id: String,
    pub(in crate::presentation) matrix_id: String,
    pub(in crate::presentation) row: u32,
    pub(in crate::presentation) column: u32,
    pub(in crate::presentation) at: Vec2,
    pub(in crate::presentation) rotation: f64,
    pub(in crate::presentation) size: Vec2,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct KeycapResizePlan {
    pub(in crate::presentation) document: ProjectDoc,
    pub(in crate::presentation) target_ids: Vec<String>,
}

pub(in crate::presentation) struct KeycapResizeInput<'a> {
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
pub(super) fn plan_resize(input: KeycapResizeInput<'_>) -> Option<KeycapResizePlan> {
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
            .find(|layout| layout.matrix_id == placement.matrix_id);
        let canonical_layout = owner
            .and_then(|owner| {
                owner
                    .mirror_link
                    .as_ref()
                    .and_then(|link| layouts.iter().find(|layout| layout.id == link.source_id))
            })
            .or(owner);
        let matrix_id = canonical_layout.map_or(placement.matrix_id.as_str(), |layout| {
            layout.matrix_id.as_str()
        });
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
            && let Some(paired_id) =
                scene_member(scenes, &paired.matrix_id, placement.row, placement.column)
            && placements.iter().any(|candidate| candidate.id == paired_id)
        {
            updated_sizes.insert(paired_id.to_owned(), next_size);
        }
    }
    if updated_sizes.is_empty() {
        return None;
    }

    let resize_values: Vec<_> = resize_by_cell.values().cloned().collect();
    let deltas = reflow(&resize_values, placements);
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
        let local = local_matrix_delta(matrix, *delta, placement.column as usize);
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
    Some(KeycapResizePlan {
        document: next_document,
        target_ids: targets.into_iter().collect(),
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
fn local_matrix_delta(matrix: &Matrix, delta: Vec2, column: usize) -> Vec2 {
    let angle = -matrix.rotation.unwrap_or_default().to_radians();
    let (sin, cos) = angle.sin_cos();
    let mut x = delta.x * cos - delta.y * sin;
    let mut y = delta.x * sin + delta.y * cos;
    match matrix.mirror {
        Some(Mirror::X) => x = -x,
        Some(Mirror::Y) => y = -y,
        _ => {}
    }
    let splay = -matrix
        .column_splays
        .iter()
        .take(column + 1)
        .sum::<f64>()
        .to_radians();
    let (sin, cos) = splay.sin_cos();
    Vec2 {
        x: x * cos - y * sin,
        y: x * sin + y * cos,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::{
        Board, CoreReply, CoreRequest, EditCommand, EditOperation, EditPhase, MatrixCell,
        MatrixColumnBasis, MatrixSceneCell, Part, Pose2, Side,
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
            row_offsets: vec![],
            column_offsets: vec![],
            column_staggers: vec![],
            column_splays: vec![],
            column_origins: vec![],
            cells: vec![0, 1, 2]
                .into_iter()
                .map(|column| MatrixCell {
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
                    column,
                    splay_origin: point(0.0, 0.0),
                    splay_angle: 0.0,
                    custom_origin: false,
                    axis_x: point(1.0, 0.0),
                    axis_y: point(0.0, 1.0),
                })
                .collect(),
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
                column: 0,
                splay_origin: point(0.0, 0.0),
                splay_angle: 0.0,
                custom_origin: false,
                axis_x: point(1.0, 0.0),
                axis_y: point(0.0, 1.0),
            }],
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

    /// End-to-end Core seam for linked key resize. Set `KEYCAPS_MIRROR_PROJECT_JSON` to the
    /// extracted `project.json` from the pinned Sofle archive to run this opt-in fixture test.
    #[test]
    #[ignore = "requires KEYCAPS_MIRROR_PROJECT_JSON to point at the pinned archive fixture"]
    fn replace_document_reflows_source_and_core_syncs_linked_partner_through_history() {
        let source = std::env::var("KEYCAPS_MIRROR_PROJECT_JSON").unwrap();
        let mut document: ProjectDoc =
            serde_json::from_slice(&std::fs::read(source).unwrap()).unwrap();
        let left = document
            .layouts
            .iter()
            .find(|layout| layout.id == "left-keys-layout")
            .unwrap()
            .clone();
        let right_index = document
            .layouts
            .iter()
            .position(|layout| layout.id == "right-keys-layout")
            .unwrap();
        let right_matrix_id = document.layouts[right_index].matrix_id.clone();
        document.layouts[right_index].board_id = left.board_id.clone();
        document.layouts[right_index].mirror_link =
            Some(boardstudio_core::model::LayoutMirrorLink {
                source_id: left.id.clone(),
                axis_x: 0.0,
            });
        let right_matrix_index = document
            .matrices
            .iter()
            .position(|matrix| matrix.id == right_matrix_id)
            .unwrap();
        document.matrices[right_matrix_index].board_id = Some(left.board_id.clone());
        let right_parts: BTreeSet<_> = document
            .boards
            .iter()
            .find(|board| board.id == "right")
            .unwrap()
            .part_ids
            .iter()
            .cloned()
            .collect();
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "right")
            .unwrap()
            .part_ids
            .retain(|id| !right_parts.contains(id));
        document
            .boards
            .iter_mut()
            .find(|board| board.id == left.board_id)
            .unwrap()
            .part_ids
            .extend(right_parts);

        let mut engine = boardstudio_core::CoreEngine::new();
        let opened = engine.handle(CoreRequest::Open {
            id: "open".into(),
            document,
        });
        let (accepted, scene) = match opened {
            CoreReply::Scene {
                document, scene, ..
            } => (*document, scene),
            other => panic!("linked fixture did not open: {other:?}"),
        };
        let source_matrix_id = accepted
            .layouts
            .iter()
            .find(|layout| layout.id == "left-keys-layout")
            .unwrap()
            .matrix_id
            .clone();
        let source_matrix = accepted
            .matrices
            .iter()
            .find(|matrix| matrix.id == source_matrix_id)
            .unwrap();
        let source_scene = scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == source_matrix_id)
            .unwrap();
        let cell = source_scene
            .cells
            .iter()
            .find(|cell| cell.enabled && cell.row == 1 && cell.column == 2)
            .unwrap();
        let selected_id = cell.member_id.clone().unwrap();
        let mut accepted_placements = Vec::new();
        for matrix in &accepted.matrices {
            let Some(matrix_scene) = scene
                .matrix_scenes
                .iter()
                .find(|scene| scene.matrix_id == matrix.id)
            else {
                continue;
            };
            for cell in matrix_scene.cells.iter().filter(|cell| cell.enabled) {
                let Some(id) = cell.member_id.as_deref() else {
                    continue;
                };
                let Some(part) = accepted.parts.iter().find(|part| part.id == id) else {
                    continue;
                };
                if !matrix.part_ids.contains(&part.id) {
                    continue;
                }
                let definition = accepted
                    .definitions
                    .iter()
                    .find(|definition| definition.id == part.definition_id);
                let gap = matrix.edge_gap.unwrap_or(point(1.0, 1.0));
                let size = part
                    .keycap
                    .or_else(|| definition.and_then(|definition| definition.keycap))
                    .unwrap_or(point(
                        (matrix.pitch.x - gap.x).max(1.0),
                        (matrix.pitch.y - gap.y).max(1.0),
                    ));
                accepted_placements.push(KeycapPlacement {
                    id: id.into(),
                    matrix_id: matrix.id.clone(),
                    row: cell.row,
                    column: cell.column,
                    at: cell.pose.at,
                    rotation: cell.pose.rotation,
                    size,
                });
            }
        }
        let units = point(3.0, 1.0);
        let selected_ids = BTreeSet::from([selected_id.clone()]);
        let plan = plan_resize(KeycapResizeInput {
            document: &accepted,
            matrices: &accepted.matrices,
            scenes: &scene.matrix_scenes,
            layouts: &accepted.layouts,
            placements: &accepted_placements,
            selected_ids: &selected_ids,
            units,
            axis: None,
        })
        .expect("linked source selection should make a plan");
        let command = EditCommand {
            base_revision: accepted.revision,
            transaction_id: "keycaps-replace-document-mirror".into(),
            phase: EditPhase::Commit,
            target_ids: plan.target_ids.clone(),
            operation: EditOperation::ReplaceDocument {
                document: Box::new(plan.document.clone()),
            },
        };
        let committed = engine.handle(CoreRequest::Edit {
            id: "resize".into(),
            command,
        });
        let (saved, saved_scene) = match committed {
            CoreReply::Scene {
                document, scene, ..
            } => (*document, scene),
            other => panic!("ReplaceDocument resize was rejected: {other:?}"),
        };
        assert_eq!(saved.revision, accepted.revision + 1);
        let partner_layout = saved
            .layouts
            .iter()
            .find(|layout| layout.id == "right-keys-layout")
            .unwrap();
        let original_partner_scene = scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == partner_layout.matrix_id)
            .unwrap();
        let partner_scene = saved_scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == partner_layout.matrix_id)
            .unwrap();
        let partner_id = partner_scene
            .cells
            .iter()
            .find(|cell| cell.row == 1 && cell.column == 2)
            .unwrap()
            .member_id
            .as_ref()
            .unwrap();
        let source_part = saved
            .parts
            .iter()
            .find(|part| part.id == selected_id)
            .unwrap();
        let partner_part = saved
            .parts
            .iter()
            .find(|part| &part.id == partner_id)
            .unwrap();
        assert_eq!(
            source_part.keycap,
            Some(point(
                3.0 * source_matrix.pitch.x - source_matrix.edge_gap.unwrap_or(point(1.0, 1.0)).x,
                source_matrix.pitch.y - source_matrix.edge_gap.unwrap_or(point(1.0, 1.0)).y
            ))
        );
        assert_eq!(partner_part.keycap, source_part.keycap);
        let axis_x = partner_layout.mirror_link.as_ref().unwrap().axis_x;
        assert!((source_part.pose.at.x + partner_part.pose.at.x - 2.0 * axis_x).abs() < 1e-6);
        assert!((source_part.pose.at.y - partner_part.pose.at.y).abs() < 1e-6);
        let original_selected_pose = cell.pose.at;
        assert!((source_part.pose.at.x - original_selected_pose.x).abs() < 1e-6);
        assert!((source_part.pose.at.y - original_selected_pose.y).abs() < 1e-6);
        for column in [1, 3] {
            let original_cell = source_scene
                .cells
                .iter()
                .find(|cell| cell.enabled && cell.row == 1 && cell.column == column)
                .unwrap();
            let neighbor_id = original_cell.member_id.as_ref().unwrap();
            let original_neighbor = accepted
                .parts
                .iter()
                .find(|part| &part.id == neighbor_id)
                .unwrap();
            let saved_neighbor = saved
                .parts
                .iter()
                .find(|part| &part.id == neighbor_id)
                .unwrap();
            assert!((saved_neighbor.pose.at.x - original_neighbor.pose.at.x).abs() > 1e-6);
            let original_partner_id = original_partner_scene
                .cells
                .iter()
                .find(|cell| cell.row == 1 && cell.column == column)
                .unwrap()
                .member_id
                .as_ref()
                .unwrap();
            let saved_partner_id = partner_scene
                .cells
                .iter()
                .find(|cell| cell.row == 1 && cell.column == column)
                .unwrap()
                .member_id
                .as_ref()
                .unwrap();
            let saved_partner = saved
                .parts
                .iter()
                .find(|part| &part.id == saved_partner_id)
                .unwrap();
            assert_eq!(
                saved
                    .parts
                    .iter()
                    .find(|part| part.id == *original_partner_id)
                    .unwrap()
                    .id,
                *saved_partner_id
            );
            assert!(
                (saved_neighbor.pose.at.x + saved_partner.pose.at.x - 2.0 * axis_x).abs() < 1e-6
            );
            assert!((saved_neighbor.pose.at.y - saved_partner.pose.at.y).abs() < 1e-6);
        }

        let undone = engine.handle(CoreRequest::Undo { id: "undo".into() });
        let (undone, _) = match undone {
            CoreReply::Scene {
                document, scene, ..
            } => (*document, scene),
            other => panic!("undo failed: {other:?}"),
        };
        assert_eq!(
            undone
                .parts
                .iter()
                .find(|part| part.id == selected_id)
                .unwrap()
                .keycap,
            accepted
                .parts
                .iter()
                .find(|part| part.id == selected_id)
                .unwrap()
                .keycap
        );
        let redone = engine.handle(CoreRequest::Redo { id: "redo".into() });
        let (redone, _) = match redone {
            CoreReply::Scene {
                document, scene, ..
            } => (*document, scene),
            other => panic!("redo failed: {other:?}"),
        };
        assert_eq!(
            redone
                .parts
                .iter()
                .find(|part| part.id == selected_id)
                .unwrap()
                .keycap,
            source_part.keycap
        );
    }
}

//! Accepted-document geometry for the Layout key-size interaction.
//! Overlap queries and reflow math shared by `plan_resize` and its tests.
use super::*;

pub(super) fn overlaps_on_axes(first: &[Vec2; 4], second: &[Vec2; 4], source: &[Vec2; 4]) -> bool {
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

pub(super) fn project_polygon(points: &[Vec2; 4], axis: Vec2) -> (f64, f64) {
    let mut projections = points.iter().map(|point| dot(*point, axis));
    let first = projections.next().unwrap_or_default();
    projections.fold((first, first), |(min, max), value| {
        (min.min(value), max.max(value))
    })
}

pub(super) fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

pub(super) fn scale(a: Vec2, factor: f64) -> Vec2 {
    Vec2 {
        x: a.x * factor,
        y: a.y * factor,
    }
}

pub(super) fn finite(value: f64) -> bool {
    value.is_finite()
}

pub(super) fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

pub(super) fn add_delta(map: &mut BTreeMap<String, Vec2>, id: &str, delta: Vec2) {
    let entry = map.entry(id.to_owned()).or_insert(Vec2 { x: 0.0, y: 0.0 });
    entry.x += delta.x;
    entry.y += delta.y;
}

pub(super) fn half_extent(placement: &KeycapPlacement, axis: Vec2, size: Vec2) -> f64 {
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

pub(super) fn corners(placement: &KeycapPlacement) -> [Vec2; 4] {
    let angle = placement.rotation.to_radians();
    let (sin, cos) = angle.sin_cos();
    [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)].map(|(x, y)| Vec2 {
        x: placement.at.x + x * placement.size.x * cos - y * placement.size.y * sin,
        y: placement.at.y + x * placement.size.x * sin + y * placement.size.y * cos,
    })
}

pub(super) fn set_cell_offset(matrix: &mut Matrix, row: u32, column: u32, offset: Vec2) {
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

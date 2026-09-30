use super::{location, valid_projection_matrix};
use crate::model::{Matrix, MatrixSplayAffect, MatrixSplayChange, Vec2};

fn rotate(point: Vec2, degrees: f64, pivot: Vec2) -> Vec2 {
    let angle = degrees.to_radians();
    let (sin, cos) = angle.sin_cos();
    let x = point.x - pivot.x;
    let y = point.y - pivot.y;
    Vec2 {
        x: pivot.x + x * cos - y * sin,
        y: pivot.y + x * sin + y * cos,
    }
}

fn local_origin(matrix: &Matrix, column: u32) -> Vec2 {
    matrix
        .column_origins
        .get(column as usize)
        .copied()
        .flatten()
        .unwrap_or(Vec2 {
            x: column as f64 * matrix.pitch.x,
            y: matrix
                .column_staggers
                .iter()
                .take(column as usize + 1)
                .sum(),
        })
}

pub(super) fn origin_world(matrix: &Matrix, column: u32) -> Vec2 {
    let mut point = local_origin(matrix, column);
    for index in (0..column as usize).rev() {
        point = rotate(
            point,
            matrix.column_splays.get(index).copied().unwrap_or(0.0),
            local_origin(matrix, index as u32),
        );
    }
    if matches!(matrix.mirror, Some(crate::model::Mirror::X)) {
        point.x = -point.x;
    }
    if matches!(matrix.mirror, Some(crate::model::Mirror::Y)) {
        point.y = -point.y;
    }
    point = rotate(
        point,
        matrix.rotation.unwrap_or(0.0),
        Vec2 { x: 0.0, y: 0.0 },
    );
    Vec2 {
        x: point.x + matrix.origin.x,
        y: point.y + matrix.origin.y,
    }
}

fn inverse_vector(matrix: &Matrix, column: u32, world: Vec2) -> Vec2 {
    let mut point = rotate(
        world,
        -matrix.rotation.unwrap_or(0.0),
        Vec2 { x: 0.0, y: 0.0 },
    );
    if matches!(matrix.mirror, Some(crate::model::Mirror::X)) {
        point.x = -point.x;
    }
    if matches!(matrix.mirror, Some(crate::model::Mirror::Y)) {
        point.y = -point.y;
    }
    rotate(
        point,
        -matrix
            .column_splays
            .iter()
            .take(column as usize + 1)
            .sum::<f64>(),
        Vec2 { x: 0.0, y: 0.0 },
    )
}

fn preserve_columns(
    before: &Matrix,
    mut after: Matrix,
    columns: impl Iterator<Item = u32>,
) -> Matrix {
    after
        .column_offsets
        .resize(after.columns as usize, Vec2 { x: 0.0, y: 0.0 });
    for column in columns {
        let previous = location(before, 0, column, None);
        let next = location(&after, 0, column, None);
        let delta = inverse_vector(
            &after,
            column,
            Vec2 {
                x: previous.x - next.x,
                y: previous.y - next.y,
            },
        );
        after.column_offsets[column as usize].x += delta.x;
        after.column_offsets[column as usize].y += delta.y;
    }
    after
}

pub(crate) fn update(
    matrix: &Matrix,
    column: u32,
    change: &MatrixSplayChange,
) -> Result<Matrix, String> {
    valid_projection_matrix(matrix)?;
    if column >= matrix.columns {
        return Err("Matrix splay column is out of bounds".into());
    }
    let mut after = matrix.clone();
    match change {
        MatrixSplayChange::Origin { world } => {
            let point = world.map(|world| {
                let mut point = Vec2 {
                    x: world.x - matrix.origin.x,
                    y: world.y - matrix.origin.y,
                };
                point = rotate(
                    point,
                    -matrix.rotation.unwrap_or(0.0),
                    Vec2 { x: 0.0, y: 0.0 },
                );
                if matches!(matrix.mirror, Some(crate::model::Mirror::X)) {
                    point.x = -point.x;
                }
                if matches!(matrix.mirror, Some(crate::model::Mirror::Y)) {
                    point.y = -point.y;
                }
                for index in 0..column as usize {
                    point = rotate(
                        point,
                        -matrix.column_splays.get(index).copied().unwrap_or(0.0),
                        local_origin(matrix, index as u32),
                    );
                }
                point
            });
            if world.is_some_and(|v| !v.x.is_finite() || !v.y.is_finite()) {
                return Err("Matrix splay origin is non-finite".into());
            }
            after.column_origins.resize(after.columns as usize, None);
            after.column_origins[column as usize] = point;
            let result = preserve_columns(matrix, after, column..matrix.columns);
            valid_projection_matrix(&result)?;
            Ok(result)
        }
        MatrixSplayChange::Angle { angle, affect } => {
            if !angle.is_finite() {
                return Err("Matrix splay angle is non-finite".into());
            }
            after.column_splays.resize(after.columns as usize, 0.0);
            let delta = *angle - after.column_splays[column as usize];
            after.column_splays[column as usize] = *angle;
            if matches!(affect, MatrixSplayAffect::Following) || column + 1 == matrix.columns {
                valid_projection_matrix(&after)?;
                return Ok(after);
            }
            after.column_splays[column as usize + 1] -= delta;
            let result = preserve_columns(matrix, after, (column + 1)..matrix.columns);
            valid_projection_matrix(&result)?;
            Ok(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Mirror;

    fn matrix() -> Matrix {
        Matrix {
            id: "m".into(),
            name: None,
            rows: 2,
            columns: 3,
            pitch: Vec2 { x: 19.0, y: 19.0 },
            origin: Vec2 { x: 7.0, y: -3.0 },
            definition_id: "d".into(),
            part_ids: vec![],
            board_id: None,
            mirror: Some(Mirror::X),
            rotation: Some(31.0),
            edge_gap: None,

            diode_direction: None,
            row_offsets: vec![],
            column_offsets: vec![],
            column_staggers: vec![0.0, 3.0, -2.0],
            column_splays: vec![9.0, 17.0, -4.0],
            column_origins: vec![],
            cells: vec![],
        }
    }

    fn key(matrix: &Matrix, row: u32, column: u32) -> Vec2 {
        location(matrix, row, column, None)
    }

    #[test]
    fn moving_origin_preserves_all_keys_through_mirror_and_rotation() {
        let before = matrix();
        let target = Vec2 { x: 37.0, y: -43.0 };
        let after = update(
            &before,
            1,
            &MatrixSplayChange::Origin {
                world: Some(target),
            },
        )
        .unwrap();
        let actual = origin_world(&after, 1);
        assert!((actual.x - target.x).abs() < 1e-8);
        assert!((actual.y - target.y).abs() < 1e-8);
        for row in 0..before.rows {
            for column in 0..before.columns {
                let a = key(&before, row, column);
                let b = key(&after, row, column);
                assert!((a.x - b.x).abs() < 1e-8);
                assert!((a.y - b.y).abs() < 1e-8);
            }
        }
    }

    #[test]
    fn column_angle_moves_only_the_selected_column() {
        let mut before = matrix();
        before.mirror = None;
        before.rotation = Some(21.0);
        before.columns = 4;
        before.column_staggers = vec![0.0; 4];
        before.column_splays = vec![12.0, 8.0, 0.0, 0.0];
        let after = update(
            &before,
            1,
            &MatrixSplayChange::Angle {
                angle: 29.0,
                affect: MatrixSplayAffect::Column,
            },
        )
        .unwrap();
        for column in 0..before.columns {
            let a = key(&before, 1, column);
            let b = key(&after, 1, column);
            if column == 1 {
                assert!((a.x - b.x).hypot(a.y - b.y) > 1.0);
            } else {
                assert!((a.x - b.x).abs() < 1e-8, "col {column}: {} {}", a.x, b.x);
                assert!((a.y - b.y).abs() < 1e-8);
            }
        }
    }

    #[test]
    fn custom_origin_is_stationary_during_following_splay() {
        let mut before = matrix();
        before.mirror = None;
        before.rotation = Some(0.0);
        before.origin = Vec2 { x: 0.0, y: 0.0 };
        before.column_staggers.clear();
        before.column_splays.clear();
        let custom = update(
            &before,
            1,
            &MatrixSplayChange::Origin {
                world: Some(Vec2 { x: 0.0, y: -20.0 }),
            },
        )
        .unwrap();
        let after = update(
            &custom,
            1,
            &MatrixSplayChange::Angle {
                angle: 90.0,
                affect: MatrixSplayAffect::Following,
            },
        )
        .unwrap();
        let a = key(&before, 0, 1);
        let b = key(&after, 0, 1);
        assert!((b.x - (-a.y - 20.0)).abs() < 1e-8);
        assert!((b.y - (a.x - 20.0)).abs() < 1e-8);
    }

    #[test]
    fn reset_origin_and_reject_invalid_inputs() {
        let before = matrix();
        let changed = update(
            &before,
            1,
            &MatrixSplayChange::Origin {
                world: Some(Vec2 { x: 2.0, y: 4.0 }),
            },
        )
        .unwrap();
        let reset = update(&changed, 1, &MatrixSplayChange::Origin { world: None }).unwrap();
        assert!(reset.column_origins[1].is_none());
        assert!(
            update(
                &before,
                9,
                &MatrixSplayChange::Angle {
                    angle: 1.0,
                    affect: MatrixSplayAffect::Column
                }
            )
            .is_err()
        );
        assert!(
            update(
                &before,
                1,
                &MatrixSplayChange::Angle {
                    angle: f64::NAN,
                    affect: MatrixSplayAffect::Column
                }
            )
            .is_err()
        );
        assert!(
            update(
                &before,
                1,
                &MatrixSplayChange::Origin {
                    world: Some(Vec2 {
                        x: f64::NAN,
                        y: 0.0
                    })
                }
            )
            .is_err()
        );
    }
}

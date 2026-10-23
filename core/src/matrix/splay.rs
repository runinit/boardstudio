use super::{evaluated, valid_projection_matrix};
use crate::model::{Matrix, MatrixSplayAffect, MatrixSplayChange, ProjectDoc, Vec2};

pub(crate) fn rotate(point: Vec2, degrees: f64, pivot: Vec2) -> Vec2 {
    let angle = degrees.to_radians();
    let (sin, cos) = angle.sin_cos();
    let x = point.x - pivot.x;
    let y = point.y - pivot.y;
    Vec2 {
        x: pivot.x + x * cos - y * sin,
        y: pivot.y + x * sin + y * cos,
    }
}

pub(crate) fn origin_world(matrix: &Matrix, column: u32, doc: &ProjectDoc) -> Result<Vec2, String> {
    evaluated(matrix, doc)?.columns.get(column as usize).map(|c| c.splay_origin).ok_or_else(|| "Matrix column is out of bounds".into())
}

fn compensate(matrix: &Matrix, column: u32, point: Vec2, doc: &ProjectDoc) -> Result<Matrix, String> {
    let mut after = matrix.clone();
    after.column_layout.resize_with((column as usize + 2).min(matrix.columns as usize).max(after.column_layout.len()), Default::default);
    let index = column as usize;
    let values = after.column_layout[index];
    let delta = Vec2 { x: point.x - values.splay_origin.x, y: point.y - values.splay_origin.y };
    after.column_layout[index].splay_origin = point;
    let rotated = rotate(delta, values.splay + values.rotation, Vec2::default());
    let correction = Vec2 { x: rotated.x - delta.x, y: rotated.y - delta.y };
    if column > 0 {
        after.column_layout[index].spread = Some(values.spread.unwrap_or(matrix.pitch.x) + correction.x);
        after.column_layout[index].stagger += correction.y;
    } else {
        let frame = evaluated(matrix, doc)?.frame;
        let reflect = |v: Vec2| match matrix.mirror {
            Some(crate::model::Mirror::X) => Vec2 { x: -v.x, y: v.y },
            Some(crate::model::Mirror::Y) => Vec2 { x: v.x, y: -v.y },
            _ => v,
        };
        let world = rotate(reflect(correction), frame.rotation, Vec2::default());
        let mut x = after.clone(); x.origin.x += 1.0;
        let mut y = after.clone(); y.origin.y += 1.0;
        let base = evaluated(&after, doc)?.frame.origin;
        let px = evaluated(&x, doc)?.frame.origin;
        let py = evaluated(&y, doc)?.frame.origin;
        let change = solve(Vec2 { x: px.x-base.x, y: px.y-base.y }, Vec2 { x: py.x-base.x, y: py.y-base.y }, world)
            .ok_or("Hold-still compensation requires enabling the missing Follow component or explicit Detach")?;
        after.origin.x += change.x;
        after.origin.y += change.y;
    }
    if column + 1 < matrix.columns && values.rotation != 0.0 {
        let rotated = rotate(delta, values.rotation, Vec2::default());
        let next = &mut after.column_layout[index+1];
        next.spread = Some(next.spread.unwrap_or(matrix.pitch.x) + delta.x-rotated.x);
        next.stagger += delta.y-rotated.y;
    }
    Ok(after)
}

fn solve(x: Vec2, y: Vec2, delta: Vec2) -> Option<Vec2> {
    let determinant = x.x*y.y - x.y*y.x;
    if determinant.abs() < 1e-12 {
        if delta.x.abs() < 1e-9 && delta.y.abs() < 1e-9 { return Some(Vec2::default()); }
        return None;
    }
    Some(Vec2 { x: (delta.x*y.y-delta.y*y.x)/determinant, y: (x.x*delta.y-x.y*delta.x)/determinant })
}

pub(crate) fn update(matrix: &Matrix, column: u32, change: &MatrixSplayChange, doc: &ProjectDoc) -> Result<Matrix, String> {
    valid_projection_matrix(matrix)?;
    if column >= matrix.columns { return Err("Matrix splay column is out of bounds".into()); }
    let mut after = matrix.clone();
    after.column_layout.resize_with((column as usize+1).max(after.column_layout.len()), Default::default);
    match change {
        MatrixSplayChange::Angle { angle, affect } => {
            if !angle.is_finite() { return Err("Matrix splay angle is non-finite".into()); }
            let values = &mut after.column_layout[column as usize];
            match affect {
                MatrixSplayAffect::Following => values.splay = *angle,
                MatrixSplayAffect::Column => values.rotation = *angle,
            }
        }
        MatrixSplayChange::Origin { world } => {
            let point = if let Some(target) = world {
                if !target.x.is_finite() || !target.y.is_finite() { return Err("Matrix splay origin is non-finite".into()); }
                let base = compensate(matrix, column, Vec2::default(), doc)?;
                let x = compensate(matrix, column, Vec2 { x: 1.0, y: 0.0 }, doc)?;
                let y = compensate(matrix, column, Vec2 { x: 0.0, y: 1.0 }, doc)?;
                let origin = origin_world(&base, column, doc)?;
                let px = origin_world(&x, column, doc)?;
                let py = origin_world(&y, column, doc)?;
                solve(Vec2 { x: px.x-origin.x, y: px.y-origin.y }, Vec2 { x: py.x-origin.x, y: py.y-origin.y }, Vec2 { x: target.x-origin.x, y: target.y-origin.y })
                    .ok_or("Requested splay origin cannot be represented with these turns and Follow permissions")?
            } else { Vec2::default() };
            after = compensate(matrix, column, point, doc)?;
        }
    }
    evaluated(&after, doc)?;
    Ok(after)
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
            row_layout: vec![],
            column_layout: vec![crate::model::ColumnLayout { splay: 9.0, ..Default::default() }, crate::model::ColumnLayout { stagger: 3.0, splay: 17.0, ..Default::default() }, crate::model::ColumnLayout { stagger: -2.0, splay: -4.0, ..Default::default() }],
            anchor: None,
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

//! Pure carried layout, staged independently of persisted documents and production edits.
//!
//! `evaluate` applies Origin/Rotation and Follow exactly once. Callers resolve the raw
//! final anchor-key pose (including Local exceptions); they never supply a composed frame.
//! Ownership/anchor graphs and materialization filtering belong to callers. Every logical
//! cell is returned in column-major order, including disabled/deleted/Omitted contributors.

use super::splay::rotate;
pub use crate::model::{ColumnLayout, Follow, RowLayout};
use crate::model::{Mirror, Pose2, Vec2};

/// Geometry values only, not a persisted document or an editable parallel matrix model.
/// Short axis lists and missing cells supply defaults. The default is a 2×2 matrix.
pub struct LayoutInput {
    pub column_layout: Vec<ColumnLayout>,
    pub cells: Vec<KeyLayout>,
    pub origin: Vec2,
    pub rotation: f64,
    pub mirror: Mirror,
    pub edge_gap: f64,
    pub row_layout: Vec<RowLayout>,
    pub rows: usize,
    pub columns: usize,
    pub pitch: Vec2,
}

impl Default for LayoutInput {
    fn default() -> Self {
        Self {
            column_layout: vec![],
            cells: vec![],
            origin: Vec2::default(),
            rotation: 0.0,
            mirror: Mirror::None,
            edge_gap: super::DEFAULT_EDGE_GAP_MM,
            row_layout: vec![],
            rows: 2,
            columns: 2,
            pitch: Vec2 { x: 19.05, y: 19.05 },
        }
    }
}

#[derive(Clone, Copy)]
pub enum RawAnchorContext {
    World,
    Key {
        /// Actual final world pose, not a precomposed matrix frame.
        pose: Pose2,
        follow: Follow,
        basis: FollowBasis,
    },
}

#[derive(Clone, Copy, Default)]
pub enum FollowBasis {
    #[default]
    World,
    /// Source-world filter transported by T(p) = R(rotation)·(-p.x,p.y)+translation.
    /// Heading transport is rotation-heading; angles are degrees, never normalized.
    Reflected { translation: Vec2, rotation: f64 },
}

#[derive(Clone, Copy, Default)]
pub struct KeyLayout {
    pub row: usize,
    pub column: usize,
    pub shift: Vec2,
    pub bend: f64,
    pub offset: Vec2,
    pub rotation: f64,
}

pub struct CellLayout {
    pub row: usize,
    pub column: usize,
    pub pose: Pose2,
    pub shift_frame: ComposedFrame,
    pub offset_frame: ComposedFrame,
    pub contributions: CellContributions,
}

/// An evaluated world frame, never a raw input anchor. Axes retain mirror handedness.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposedFrame {
    pub origin: Vec2,
    pub rotation: f64,
    pub axis_x: Vec2,
    pub axis_y: Vec2,
}
impl ComposedFrame {
    /// Invert a displacement through the evaluated, mirror-aware orthonormal axes.
    pub fn inverse_vector(&self, delta: Vec2) -> Vec2 {
        Vec2 {
            x: delta.x * self.axis_x.x + delta.y * self.axis_x.y,
            y: delta.x * self.axis_y.x + delta.y * self.axis_y.y,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StepSource {
    Pitch,
    Explicit,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step {
    pub value: f64,
    pub source: StepSource,
}

/// Numeric facts owned by this (row,column); cumulative values exclude independent edits.
pub struct CellContributions {
    pub padding: Option<Step>,
    pub row_shift: Vec2,
    pub key_shift: Vec2,
    pub row_shift_matrix: Vec2,
    pub key_shift_matrix: Vec2,
    pub row_bend: f64,
    pub key_bend: f64,
    /// Transported into unreflected matrix axes, not a raw sum of stored Shift vectors.
    pub cumulative_shift: Vec2,
    pub cumulative_bend: f64,
    pub row_offset: Vec2,
    pub column_offset: Vec2,
    pub key_offset: Vec2,
    pub key_rotation: f64,
}

pub struct ColumnFrame {
    /// Column 0 still needs its incoming X axis for origin edits, although it has no Spread.
    pub(crate) incoming_axis_x: Vec2,
    /// Base after preceding Splays, before this column's own turns.
    pub base: Vec2,
    pub splay_origin: Vec2,
    pub relative_origin: Vec2,
    pub custom_origin: bool,
    pub frame: ComposedFrame,
    pub spread_axis: Option<Vec2>,
    pub stagger_axis: Vec2,
    pub spread: Option<Step>,
    pub stagger: f64,
    pub cumulative_stagger: f64,
    pub splay: f64,
    pub cumulative_splay: f64,
    pub rotation: f64,
}

pub struct MatrixLayout {
    pub frame: ComposedFrame,
    pub cells: Vec<CellLayout>,
    pub columns: Vec<ColumnFrame>,
}

/// Refuses invalid dimensions/lists/coordinates, axis-zero steps, pitch ≤ edge gap,
/// nonfinite authored values and nonfinite evaluation results (even before Follow).
pub fn evaluate(
    input: &LayoutInput,
    anchor: RawAnchorContext,
) -> Result<MatrixLayout, &'static str> {
    validate(input, anchor)?;
    let matrix_frame = compose_matrix_frame(input, anchor)?;
    let mut cells = Vec::new();
    let mut columns = Vec::new();
    let mut base = Vec2::default();
    let mut prefix = Pose2 {
        at: Vec2::default(),
        rotation: 0.0,
    };
    for column in 0..input.columns {
        let col = input.column_layout.get(column).copied().unwrap_or_default();
        base = add(
            base,
            Vec2 {
                x: if column == 0 {
                    0.0
                } else {
                    col.spread.unwrap_or(input.pitch.x)
                },
                y: col.stagger,
            },
        );
        let origin = transform(prefix, add(base, col.splay_origin));
        let carried_base = transform(prefix, base);
        let mut pose = Pose2 {
            at: add(
                origin,
                rotate(
                    sub(carried_base, origin),
                    col.splay + col.rotation,
                    Vec2::default(),
                ),
            ),
            rotation: prefix.rotation + col.splay + col.rotation,
        };
        let vector_world = |v| {
            rotate(
                reflect(v, input.mirror),
                matrix_frame.rotation,
                Vec2::default(),
            )
        };
        let point_world = |v| transform(matrix_frame, reflect(v, input.mirror));
        let incoming_axis_x = vector_world(rotate(
            Vec2 { x: 1.0, y: 0.0 },
            prefix.rotation,
            Vec2::default(),
        ));
        columns.push(ColumnFrame {
            incoming_axis_x,
            base: point_world(carried_base),
            splay_origin: point_world(origin),
            relative_origin: col.splay_origin,
            custom_origin: col.splay_origin != Vec2::default(),
            frame: world_frame(pose, matrix_frame, input.mirror),
            // Moving the step translates both base and relative origin; own turns cancel.
            spread_axis: (column > 0).then_some(incoming_axis_x),
            stagger_axis: vector_world(rotate(
                Vec2 { x: 0.0, y: 1.0 },
                prefix.rotation,
                Vec2::default(),
            )),
            spread: (column > 0).then(|| step(col.spread, input.pitch.x)),
            stagger: col.stagger,
            cumulative_stagger: base.y,
            splay: col.splay,
            cumulative_splay: prefix.rotation + col.splay,
            rotation: col.rotation,
        });
        prefix.at = add(
            origin,
            rotate(sub(prefix.at, origin), col.splay, Vec2::default()),
        );
        prefix.rotation += col.splay;
        finite_pose(prefix)?;
        finite_pose(pose)?;
        finite_vec(base)?;
        finite_vec(origin)?;
        let mut cumulative_shift = Vec2::default();
        let mut cumulative_bend = 0.0;
        for row in 0..input.rows {
            let values = input.row_layout.get(row).copied().unwrap_or_default();
            let key = input
                .cells
                .iter()
                .find(|c| c.row == row && c.column == column)
                .copied()
                .unwrap_or_default();
            if row > 0 {
                pose.at = transform(
                    pose,
                    Vec2 {
                        x: 0.0,
                        y: values.padding.unwrap_or(input.pitch.y),
                    },
                );
            }
            let shift_frame = world_frame(pose, matrix_frame, input.mirror);
            let row_shift_matrix = rotate(values.shift, pose.rotation, Vec2::default());
            let key_shift_matrix = rotate(key.shift, pose.rotation, Vec2::default());
            cumulative_shift = add(cumulative_shift, add(row_shift_matrix, key_shift_matrix));
            cumulative_bend += values.bend + key.bend;
            pose.at = transform(pose, add(values.shift, key.shift));
            pose.rotation += values.bend + key.bend;
            let local = Pose2 {
                at: transform(pose, add(add(values.offset, col.offset), key.offset)),
                rotation: pose.rotation + key.rotation,
            };
            finite_pose(pose)?;
            finite_pose(local)?;
            finite_vec(cumulative_shift)?;
            finite(cumulative_bend)?;
            cells.push(CellLayout {
                row,
                column,
                shift_frame,
                offset_frame: world_frame(pose, matrix_frame, input.mirror),
                contributions: CellContributions {
                    padding: (row > 0).then(|| step(values.padding, input.pitch.y)),
                    row_shift: values.shift,
                    key_shift: key.shift,
                    row_shift_matrix,
                    key_shift_matrix,
                    row_bend: values.bend,
                    key_bend: key.bend,
                    cumulative_shift,
                    cumulative_bend,
                    row_offset: values.offset,
                    column_offset: col.offset,
                    key_offset: key.offset,
                    key_rotation: key.rotation,
                },
                pose: Pose2 {
                    at: transform(matrix_frame, reflect(local.at, input.mirror)),
                    rotation: matrix_frame.rotation
                        + if matches!(input.mirror, Mirror::None) {
                            local.rotation
                        } else {
                            -local.rotation
                        },
                },
            });
        }
    }
    for cell in &cells {
        finite_pose(cell.pose)?;
        finite_frame(&cell.shift_frame)?;
        finite_frame(&cell.offset_frame)?;
    }
    for column in &columns {
        finite_vec(column.base)?;
        finite_vec(column.splay_origin)?;
        finite_frame(&column.frame)?;
    }
    Ok(MatrixLayout {
        cells,
        columns,
        frame: world_frame(
            Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            },
            matrix_frame,
            Mirror::None,
        ),
    })
}

fn step(value: Option<f64>, pitch: f64) -> Step {
    Step {
        value: value.unwrap_or(pitch),
        source: if value.is_some() {
            StepSource::Explicit
        } else {
            StepSource::Pitch
        },
    }
}

fn world_frame(local: Pose2, matrix: Pose2, mirror: Mirror) -> ComposedFrame {
    ComposedFrame {
        origin: transform(matrix, reflect(local.at, mirror)),
        rotation: matrix.rotation
            + if matches!(mirror, Mirror::None) {
                local.rotation
            } else {
                -local.rotation
            },
        axis_x: rotate(
            reflect(
                rotate(Vec2 { x: 1.0, y: 0.0 }, local.rotation, Vec2::default()),
                mirror,
            ),
            matrix.rotation,
            Vec2::default(),
        ),
        axis_y: rotate(
            reflect(
                rotate(Vec2 { x: 0.0, y: 1.0 }, local.rotation, Vec2::default()),
                mirror,
            ),
            matrix.rotation,
            Vec2::default(),
        ),
    }
}

/// Upstream Ergogen `average` aggregation: the arithmetic mean of x, y **and
/// rotation** over the part poses. The rotation mean is deliberately naive
/// (parity over wraparound purity). An empty part list refuses instead of
/// silently anchoring to the world origin.
pub fn aggregate_average(parts: &[Pose2]) -> Result<Pose2, &'static str> {
    if parts.is_empty() {
        return Err("aggregate anchor needs at least one part");
    }
    let mut x = 0.0;
    let mut y = 0.0;
    let mut rotation = 0.0;
    for part in parts {
        x += part.at.x;
        y += part.at.y;
        rotation += part.rotation;
    }
    let len = parts.len() as f64;
    Ok(Pose2 {
        at: Vec2 {
            x: x / len,
            y: y / len,
        },
        rotation: rotation / len,
    })
}

/// Upstream Ergogen `intersect` aggregation: each part contributes its
/// heading-rotated Y axis as a line; the result is their intersection with
/// rotation forced to 0. Parallel (or coincident) lines refuse, matching
/// upstream's "do not intersect" throw. The caller supplies exactly two
/// poses; arity is the caller's refusal.
pub fn aggregate_intersection(first: Pose2, second: Pose2) -> Result<Pose2, &'static str> {
    let heading = |pose: Pose2| {
        let (sin, cos) = pose.rotation.to_radians().sin_cos();
        Vec2 { x: -sin, y: cos }
    };
    let d1 = heading(first);
    let d2 = heading(second);
    let denom = d1.x * d2.y - d1.y * d2.x;
    // BoardStudio-only refusal threshold per spec §10 (1e-6 compares upstream
    // poses, 1e-9 guards BoardStudio-only numerics): headings closer than
    // ~1e-9 rad meet so far away that the crossing is not a usable anchor.
    if denom.abs() < 1e-9 {
        return Err("aggregate anchor parts do not intersect");
    }
    let dx = second.at.x - first.at.x;
    let dy = second.at.y - first.at.y;
    let t = (dx * d2.y - dy * d2.x) / denom;
    Ok(Pose2 {
        at: Vec2 {
            x: first.at.x + t * d1.x,
            y: first.at.y + t * d1.y,
        },
        rotation: 0.0,
    })
}

fn compose_matrix_frame(
    input: &LayoutInput,
    anchor: RawAnchorContext,
) -> Result<Pose2, &'static str> {
    let relative = Pose2 {
        at: input.origin,
        rotation: input.rotation,
    };
    match anchor {
        RawAnchorContext::World => Ok(relative),
        RawAnchorContext::Key {
            pose,
            follow,
            basis,
        } => {
            let mut candidate = Pose2 {
                at: transform(pose, input.origin),
                rotation: pose.rotation + input.rotation,
            };
            finite_pose(candidate)?;
            if let FollowBasis::Reflected {
                translation,
                rotation,
            } = basis
            {
                candidate.at = reflect(
                    rotate(sub(candidate.at, translation), -rotation, Vec2::default()),
                    Mirror::X,
                );
                candidate.rotation = rotation - candidate.rotation;
            }
            finite_pose(candidate)?;
            if !follow.x {
                candidate.at.x = 0.0;
            }
            if !follow.y {
                candidate.at.y = 0.0;
            }
            if !follow.rotation {
                candidate.rotation = 0.0;
            }
            if let FollowBasis::Reflected {
                translation,
                rotation,
            } = basis
            {
                candidate.at = add(
                    rotate(reflect(candidate.at, Mirror::X), rotation, Vec2::default()),
                    translation,
                );
                candidate.rotation = rotation - candidate.rotation;
            }
            finite_pose(candidate)?;
            Ok(candidate)
        }
    }
}

fn finite(value: f64) -> Result<(), &'static str> {
    if value.is_finite() {
        Ok(())
    } else {
        Err("layout values and evaluated results must be finite")
    }
}

fn finite_vec(value: Vec2) -> Result<(), &'static str> {
    finite(value.x)?;
    finite(value.y)
}

fn finite_pose(value: Pose2) -> Result<(), &'static str> {
    finite_vec(value.at)?;
    finite(value.rotation)
}

fn finite_frame(value: &ComposedFrame) -> Result<(), &'static str> {
    finite_vec(value.origin)?;
    finite(value.rotation)?;
    finite_vec(value.axis_x)?;
    finite_vec(value.axis_y)
}

fn validate(input: &LayoutInput, anchor: RawAnchorContext) -> Result<(), &'static str> {
    // Kept apart from `matrix::valid_dimensions`: this checks the already-parsed `usize` input.
    if input.rows == 0
        || input.columns == 0
        || input
            .rows
            .checked_mul(input.columns)
            .is_none_or(|n| n > super::MAX_MATRIX_PARTS as usize)
    {
        return Err("invalid matrix dimensions");
    }
    if input.column_layout.len() > input.columns || input.row_layout.len() > input.rows {
        return Err("layout list exceeds axis count");
    }
    if input
        .column_layout
        .first()
        .is_some_and(|c| c.spread.is_some())
        || input
            .row_layout
            .first()
            .is_some_and(|r| r.padding.is_some())
    {
        return Err("no Spread or Padding at axis zero");
    }
    finite_vec(input.pitch)?;
    finite(input.edge_gap)?;
    if input.edge_gap < 0.0 || input.pitch.x <= input.edge_gap || input.pitch.y <= input.edge_gap {
        return Err("pitch must exceed nonnegative edge gap");
    }
    finite_vec(input.origin)?;
    finite(input.rotation)?;
    for c in &input.column_layout {
        finite(c.spread.unwrap_or(input.pitch.x))?;
        finite(c.stagger)?;
        finite(c.splay)?;
        finite(c.rotation)?;
        finite_vec(c.splay_origin)?;
        finite_vec(c.offset)?;
    }
    for r in &input.row_layout {
        finite(r.padding.unwrap_or(input.pitch.y))?;
        finite_vec(r.shift)?;
        finite(r.bend)?;
        finite_vec(r.offset)?;
    }
    let mut coordinates = std::collections::BTreeSet::new();
    for c in &input.cells {
        if c.row >= input.rows
            || c.column >= input.columns
            || !coordinates.insert((c.row, c.column))
        {
            return Err("invalid or duplicate cell coordinate");
        }
        finite_vec(c.shift)?;
        finite(c.bend)?;
        finite_vec(c.offset)?;
        finite(c.rotation)?;
    }
    if let RawAnchorContext::Key { pose, basis, .. } = anchor {
        finite_pose(pose)?;
        if let FollowBasis::Reflected {
            translation,
            rotation,
        } = basis
        {
            finite_vec(translation)?;
            finite(rotation)?;
        }
    }
    Ok(())
}

fn reflect(v: Vec2, mirror: Mirror) -> Vec2 {
    match mirror {
        Mirror::None => v,
        Mirror::X => Vec2 { x: -v.x, y: v.y },
        Mirror::Y => Vec2 { x: v.x, y: -v.y },
    }
}

fn transform(pose: Pose2, v: Vec2) -> Vec2 {
    add(pose.at, rotate(v, pose.rotation, Vec2::default()))
}

fn sub(a: Vec2, b: Vec2) -> Vec2 {
    Vec2 {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

fn add(a: Vec2, b: Vec2) -> Vec2 {
    Vec2 {
        x: a.x + b.x,
        y: a.y + b.y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Follow, Pose2, Vec2};

    fn input_with_origin(origin: Vec2) -> LayoutInput {
        LayoutInput {
            origin,
            ..LayoutInput::default()
        }
    }

    #[test]
    fn transported_follow_uses_source_world_basis_for_slanted_maps() {
        // Non-origin, slanted pair map: translation plus 30-degree turn.
        let mapping = Pose2 {
            at: Vec2 { x: 100.0, y: 20.0 },
            rotation: 30.0,
        };
        let anchor = Pose2 {
            at: Vec2 { x: 50.0, y: 0.0 },
            rotation: 10.0,
        };
        let relative = Vec2 { x: 10.0, y: 5.0 };
        let follow = Follow {
            x: false,
            y: true,
            rotation: true,
        };
        let world = evaluate(
            &input_with_origin(relative),
            RawAnchorContext::Key {
                pose: anchor,
                follow,
                basis: FollowBasis::World,
            },
        )
        .expect("world basis evaluates")
        .frame
        .origin;
        // Follow X off zeroes X in world coordinates.
        assert!((world.x).abs() < 1e-9);
        let transported = evaluate(
            &input_with_origin(relative),
            RawAnchorContext::Key {
                pose: anchor,
                follow,
                basis: FollowBasis::Reflected {
                    translation: mapping.at,
                    rotation: mapping.rotation,
                },
            },
        )
        .expect("reflected basis evaluates")
        .frame
        .origin;
        // Transported X is not an independent zero: it is the pair image of
        // the filtered source frame.
        let expected = {
            // Manual T(F(T^-1(candidate))) using the same pair rule the
            // kernel implements, to pin the transport against independent
            // partner-world zeroing.
            let candidate = Vec2 {
                x: anchor.at.x + relative.x * (anchor.rotation.to_radians().cos())
                    - relative.y * (anchor.rotation.to_radians().sin()),
                y: anchor.at.y
                    + relative.x * (anchor.rotation.to_radians().sin())
                    + relative.y * (anchor.rotation.to_radians().cos()),
            };
            // T^-1: remove translation, unrotate, reflect X.
            let dx = candidate.x - mapping.at.x;
            let dy = candidate.y - mapping.at.y;
            let angle = (-mapping.rotation).to_radians();
            let (sin, cos) = angle.sin_cos();
            let rx = dx * cos - dy * sin;
            let ry = dx * sin + dy * cos;
            let pre = Vec2 { x: -rx, y: ry };
            // Follow X off zeroes X in source-world axes.
            let filtered = Vec2 { x: 0.0, y: pre.y };
            // T: reflect, rotate, translate.
            let rx = -filtered.x;
            let ry = filtered.y;
            let angle = mapping.rotation.to_radians();
            let (sin, cos) = angle.sin_cos();
            Vec2 {
                x: rx * cos - ry * sin + mapping.at.x,
                y: rx * sin + ry * cos + mapping.at.y,
            }
        };
        assert!(
            (transported.x - expected.x).abs() < 1e-9 && (transported.y - expected.y).abs() < 1e-9,
            "transported {transported:?} vs expected {expected:?}"
        );
        assert!(
            transported.x.abs() > 1e-6,
            "slanted transport must not be an independent zero: {transported:?}"
        );
    }

    #[test]
    fn unfollowed_rotation_transports_through_slanted_maps() {
        let mapping = Pose2 {
            at: Vec2 { x: 100.0, y: 20.0 },
            rotation: 30.0,
        };
        let anchor = Pose2 {
            at: Vec2 { x: 50.0, y: 0.0 },
            rotation: 10.0,
        };
        let follow = Follow {
            x: true,
            y: true,
            rotation: false,
        };
        let world_rotation = evaluate(
            &input_with_origin(Vec2::default()),
            RawAnchorContext::Key {
                pose: anchor,
                follow,
                basis: FollowBasis::World,
            },
        )
        .expect("world basis evaluates")
        .frame
        .rotation;
        assert!((world_rotation).abs() < 1e-9);
        let transported_rotation = evaluate(
            &input_with_origin(Vec2::default()),
            RawAnchorContext::Key {
                pose: anchor,
                follow,
                basis: FollowBasis::Reflected {
                    translation: mapping.at,
                    rotation: mapping.rotation,
                },
            },
        )
        .expect("reflected basis evaluates")
        .frame
        .rotation;
        // Unfollowed rotation reads the transported basis rotation (30 deg),
        // not an independent zero.
        assert!((transported_rotation - 30.0).abs() < 1e-9);
    }

    fn pose(x: f64, y: f64, rotation: f64) -> Pose2 {
        Pose2 {
            at: Vec2 { x, y },
            rotation,
        }
    }

    fn assert_pose(actual: Pose2, x: f64, y: f64, rotation: f64) {
        assert!(
            (actual.at.x - x).abs() < 1e-9
                && (actual.at.y - y).abs() < 1e-9
                && (actual.rotation - rotation).abs() < 1e-9,
            "expected ({x}, {y}, {rotation}), got {:?}",
            actual
        );
    }

    #[test]
    fn aggregate_average_matches_upstream_naive_mean() {
        // Upstream `o`/`ten` vector: mean of (0,0,0) and (10,10,-90) is
        // (5,5,-45). The rotation mean is naive, not circular.
        assert_pose(
            aggregate_average(&[pose(0.0, 0.0, 0.0), pose(10.0, 10.0, -90.0)]).unwrap(),
            5.0,
            5.0,
            -45.0,
        );
        assert_pose(
            aggregate_average(&[pose(1.0, 2.0, 30.0)]).unwrap(),
            1.0,
            2.0,
            30.0,
        );
    }

    #[test]
    fn aggregate_average_refuses_empty_parts() {
        assert!(aggregate_average(&[]).is_err());
    }

    #[test]
    fn aggregate_intersection_matches_upstream_vectors() {
        // Upstream `o`/`ten`: vertical x=0 meets horizontal y=10.
        assert_pose(
            aggregate_intersection(pose(0.0, 0.0, 0.0), pose(10.0, 10.0, -90.0)).unwrap(),
            0.0,
            10.0,
            0.0,
        );
        // Upstream `o`/`five`: vertical x=0 meets horizontal y=5.
        assert_pose(
            aggregate_intersection(pose(0.0, 0.0, 0.0), pose(5.0, 5.0, 90.0)).unwrap(),
            0.0,
            5.0,
            0.0,
        );
        // Upstream `o`/`rotated_o`: same point, crossed headings.
        assert_pose(
            aggregate_intersection(pose(0.0, 0.0, 0.0), pose(0.0, 0.0, 90.0)).unwrap(),
            0.0,
            0.0,
            0.0,
        );
    }

    #[test]
    fn aggregate_intersection_refuses_parallel_lines() {
        // Upstream `o`/`five_o`: two vertical axes never meet.
        assert!(aggregate_intersection(pose(0.0, 0.0, 0.0), pose(5.0, 0.0, 0.0)).is_err());
        // Upstream `o`/`o_five`: coincident axes also refuse upstream.
        assert!(aggregate_intersection(pose(0.0, 0.0, 0.0), pose(0.0, 5.0, 0.0)).is_err());
    }

    #[test]
    fn aggregate_intersection_near_parallel_guard_uses_local_tolerance() {
        // BoardStudio-only guard per spec §10 (1e-9): headings ~1e-10 rad
        // apart meet absurdly far away, so they refuse like parallels.
        assert!(aggregate_intersection(pose(0.0, 0.0, 0.0), pose(5.0, 0.0, 1e-8)).is_err());
        // Headings ~1e-8 rad apart still resolve: the crossing is finite.
        let crossing = aggregate_intersection(pose(0.0, 0.0, 0.0), pose(5.0, 0.0, 1e-6)).unwrap();
        assert!(crossing.at.x.is_finite() && crossing.at.y.is_finite());
        assert_eq!(crossing.rotation, 0.0);
    }
}

#![allow(clippy::many_single_char_names)]

use super::*;
use crate::Point2;
use crate::filters::StructuringFilter;
use array_macro::array;
use handles::FixedVertexHandle;
use itertools::{Either, Itertools};
use monstertruck_geometry::prelude::ParameterCurve;
use rustc_hash::FxHashMap as HashMap;
use std::{collections::hash_map::Entry, env, iter};
// `std::time::Instant::now()` panics on `wasm32-unknown-unknown`.
// `web_time::Instant` is std-compatible on native and uses
// `performance.now()` in the browser.
use web_time::Instant;

mod boundary;
mod diagnostics;
mod edge_sampling;
mod fallback;
mod mesh;

use boundary::{PolyBoundary, PolyBoundaryPiece};
pub(crate) use diagnostics::classify_face_drop;
pub use diagnostics::{FaceDropReason, face_drop_count, reset_face_drop_count};
use diagnostics::{observe_face_drop, observe_metric_failure};
use edge_sampling::sample_edge;
use fallback::preview_boundary_triangulation;
use mesh::{trimming_tessellation, untrimmed_tessellation};

type SPoint2 = spade::Point2<f64>;
type Cdt = ConstrainedDelaunayTriangulation<SPoint2>;
type MeshedShell = Shell<Point3, PolylineCurve, Option<PolygonMesh>>;
type MeshedCompressedShell = CompressedShell<Point3, PolylineCurve, Option<PolygonMesh>>;
type TrimmedShell<C, S, T> = CompressedTrimmedShell<Point3, C, S, T>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EdgeSamplingError {
    InvalidOptions,
    InvalidSeed,
    NonFinite,
    SingularTangent,
    NoProgress,
    BudgetExceeded,
}

impl EdgeSamplingError {
    const fn face_drop_reason(self) -> FaceDropReason {
        match self {
            Self::InvalidOptions => FaceDropReason::InvalidAngularOptions,
            Self::InvalidSeed => FaceDropReason::InvalidAngularSeed,
            Self::NonFinite => FaceDropReason::NonFiniteAngularSample,
            Self::SingularTangent => FaceDropReason::SingularAngularTangent,
            Self::NoProgress => FaceDropReason::AngularSamplingNoProgress,
            Self::BudgetExceeded => FaceDropReason::AngularSamplingBudgetExceeded,
        }
    }
}

#[derive(Clone, Debug)]
struct EdgeSamplingFailure {
    seed: PolylineCurve,
    reason: EdgeSamplingError,
}

fn edge_sample_parts(
    result: std::result::Result<PolylineCurve, EdgeSamplingFailure>,
) -> (PolylineCurve, Option<EdgeSamplingError>) {
    match result {
        Ok(polyline) => (polyline, None),
        Err(failure) => (failure.seed, Some(failure.reason)),
    }
}

pub(super) struct ShellTessellationResult {
    pub(super) shell: MeshedShell,
    pub(super) metric_failures: Vec<(usize, MetricRefinementError)>,
    pub(super) edge_failures: Vec<(usize, FaceDropReason)>,
}

fn lenient_metric_result(
    result: std::result::Result<PolygonMesh, MetricRefinementError>,
) -> (Option<PolygonMesh>, Option<MetricRefinementError>) {
    match result {
        Ok(mesh) => (Some(mesh), None),
        Err(error) => (None, Some(error)),
    }
}

const MAX_LIFTED_TRIM_SUBDIVISION_DEPTH: usize = 16;

/// Provides exact trim boundaries in surface parameter space.
pub trait ExactTrimBoundary2D {
    /// Samples the trim boundary in surface parameter space.
    fn exact_trim_boundary_2d(&self, tolerance: f64) -> Vec<Point2>;

    /// Projects a point on the spatial boundary back onto this trim boundary.
    fn project_boundary_point(&self, _point: Point3, _hint: Option<f64>) -> Option<(f64, Point2)> {
        None
    }
}

impl<C, S> ExactTrimBoundary2D for ParameterCurve<C, S>
where
    C: ParametricCurve2D + BoundedCurve + ParameterDivision1D<Point = Point2>,
    S: ParametricSurface3D,
    ParameterCurve<C, S>: SearchParameter<CurveParameter, Point = Point3>
        + SearchNearestParameter<CurveParameter, Point = Point3>,
{
    fn exact_trim_boundary_2d(&self, tolerance: f64) -> Vec<Point2> {
        lifted_trim_boundary_2d(self, tolerance)
    }

    fn project_boundary_point(&self, point: Point3, hint: Option<f64>) -> Option<(f64, Point2)> {
        // A boundary is walked from the trim's start. A trim closed in space
        // but open in parameter space matches its start with both ends, and
        // an unseeded search can land on the far one, sending every later
        // projection off the curve. So an absent hint means the start.
        let hint = hint.or(Some(self.curve().range_tuple().0));
        self.search_parameter(point, hint, 100)
            .or_else(|| self.search_nearest_parameter(point, hint, 100))
            .or_else(|| self.search_parameter(point, None, 100))
            .or_else(|| self.search_nearest_parameter(point, None, 100))
            .map(|t| (t, self.curve().subs(t)))
    }
}

fn lifted_trim_boundary_2d<C, S>(trim_curve: &ParameterCurve<C, S>, tolerance: f64) -> Vec<Point2>
where
    C: ParametricCurve2D + BoundedCurve + ParameterDivision1D<Point = Point2>,
    S: ParametricSurface3D,
{
    let (parameters, boundary) = trim_curve
        .curve()
        .parameter_division(trim_curve.curve().range_tuple(), tolerance);
    if parameters.len() != boundary.len() || boundary.len() <= 1 {
        boundary
    } else {
        let mut refined = Vec::with_capacity(boundary.len());
        refined.push(boundary[0]);
        parameters
            .windows(2)
            .zip(boundary.windows(2))
            .for_each(|(parameters, boundary)| {
                append_lifted_trim_segment(
                    trim_curve,
                    (parameters[0], boundary[0]),
                    (parameters[1], boundary[1]),
                    tolerance * tolerance,
                    0,
                    &mut refined,
                );
            });
        refined
    }
}

fn append_lifted_trim_segment<C, S>(
    trim_curve: &ParameterCurve<C, S>,
    front: (f64, Point2),
    back: (f64, Point2),
    tolerance2: f64,
    depth: usize,
    refined: &mut Vec<Point2>,
) where
    C: ParametricCurve2D,
    S: ParametricSurface3D,
{
    let (front_parameter, front_uv) = front;
    let (back_parameter, back_uv) = back;
    if depth >= MAX_LIFTED_TRIM_SUBDIVISION_DEPTH || (back_parameter - front_parameter).so_small() {
        refined.push(back_uv);
    } else {
        let middle_parameter = (front_parameter + back_parameter) * 0.5;
        let middle_uv = trim_curve.curve().subs(middle_parameter);
        let front_point = trim_curve.surface().subs(front_uv.x, front_uv.y);
        let middle_point = trim_curve.surface().subs(middle_uv.x, middle_uv.y);
        let back_point = trim_curve.surface().subs(back_uv.x, back_uv.y);
        if point_segment_distance2(middle_point, front_point, back_point) > tolerance2 {
            append_lifted_trim_segment(
                trim_curve,
                front,
                (middle_parameter, middle_uv),
                tolerance2,
                depth + 1,
                refined,
            );
            append_lifted_trim_segment(
                trim_curve,
                (middle_parameter, middle_uv),
                back,
                tolerance2,
                depth + 1,
                refined,
            );
        } else {
            refined.push(back_uv);
        }
    }
}

fn mesh_trace_enabled() -> bool {
    env::var_os("MT_MESH_TRACE").is_some()
}

fn boundary_tolerance_candidates(tolerance: f64) -> Vec<f64> {
    iter::successors(Some(tolerance), |current| {
        let next = *current * 0.5;
        (next > TOLERANCE).then_some(next)
    })
    .take(5)
    .collect()
}

fn fallback_polyline_curve<C: PolylineableCurve>(
    edge: &CompressedEdge<C>,
    orientation: bool,
    tolerance: f64,
) -> PolylineCurve {
    let curve = PolylineCurve::from_curve(&edge.curve, edge.curve.range_tuple(), tolerance);
    if orientation { curve } else { curve.inverse() }
}

fn densify_curve_parameters(mut parameters: Vec<f64>, min_points: usize) -> Vec<f64> {
    if parameters.len() >= min_points || parameters.len() < 2 || min_points < 2 {
        parameters
    } else {
        let intervals = parameters.len() - 1;
        let extra = min_points - parameters.len();
        parameters = parameters
            .windows(2)
            .enumerate()
            .flat_map(|(interval, window)| {
                let extra_before = interval * extra / intervals;
                let extra_after = (interval + 1) * extra / intervals;
                let splits = 1 + extra_after - extra_before;
                (0..splits).map(move |index| {
                    let weight = index as f64 / splits as f64;
                    window[0] + (window[1] - window[0]) * weight
                })
            })
            .chain(parameters.last().copied())
            .collect();
        parameters
    }
}

fn fallback_polyline_curve_with_min_points<C: PolylineableCurve>(
    edge: &CompressedEdge<C>,
    orientation: bool,
    tolerance: f64,
    min_points: usize,
) -> PolylineCurve {
    let parameters = densify_curve_parameters(
        edge.curve
            .parameter_division(edge.curve.range_tuple(), tolerance)
            .0,
        min_points,
    );
    let curve = parameters
        .into_iter()
        .map(|parameter| edge.curve.subs(parameter))
        .collect::<PolylineCurve>();
    if orientation { curve } else { curve.inverse() }
}

fn untrimmed_isoparametric_curves<S: PreMeshableSurface>(
    surface: &S,
    uv_range: ((f64, f64), (f64, f64)),
    options: IsoparametricCurveOptions,
) -> Vec<Vec<Point3>> {
    if options.samples_per_direction == 0 || options.segments_per_curve == 0 {
        Vec::new()
    } else {
        let ((u_min, u_max), (v_min, v_max)) = uv_range;
        let axis_values = |min: f64, max: f64| {
            (0..options.samples_per_direction)
                .map(move |index| {
                    let parameter = (index + 1) as f64 / (options.samples_per_direction + 1) as f64;
                    min + (max - min) * parameter
                })
                .collect::<Vec<_>>()
        };
        let sample_axis = |constant_axis: usize, constant_value: f64| {
            (0..=options.segments_per_curve)
                .map(|index| {
                    let parameter = index as f64 / options.segments_per_curve as f64;
                    let variable_value = match constant_axis {
                        0 => v_min + (v_max - v_min) * parameter,
                        _ => u_min + (u_max - u_min) * parameter,
                    };
                    match constant_axis {
                        0 => surface.evaluate(constant_value, variable_value),
                        _ => surface.evaluate(variable_value, constant_value),
                    }
                })
                .collect::<Vec<_>>()
        };
        axis_values(u_min, u_max)
            .into_iter()
            .map(|u| sample_axis(0, u))
            .chain(
                axis_values(v_min, v_max)
                    .into_iter()
                    .map(|v| sample_axis(1, v)),
            )
            .collect()
    }
}

fn oriented_edge_indices<'a>(
    wire: &'a [CompressedEdgeIndex],
    face_orientation: bool,
) -> impl Iterator<Item = (usize, bool)> + 'a {
    match face_orientation {
        true => Either::Left(
            wire.iter()
                .map(|edge_idx| (edge_idx.index, edge_idx.orientation)),
        ),
        false => Either::Right(
            wire.iter()
                .rev()
                .map(|edge_idx| (edge_idx.index, !edge_idx.orientation)),
        ),
    }
}

fn oriented_trimmed_edge_uses<'a, T>(
    wire: &'a [CompressedEdgeUse<T>],
    face_orientation: bool,
) -> impl Iterator<Item = (usize, bool, Option<&'a T>)> + 'a {
    match face_orientation {
        true => Either::Left(wire.iter().map(|edge_use| {
            (
                edge_use.index,
                edge_use.orientation,
                edge_use.trim_curve.as_ref(),
            )
        })),
        false => Either::Right(wire.iter().rev().map(|edge_use| {
            (
                edge_use.index,
                !edge_use.orientation,
                edge_use.trim_curve.as_ref(),
            )
        })),
    }
}

fn polyline_from_trim_curve<S, T>(
    surface: &S,
    trim_curve: &T,
    edge_vertices: (usize, usize),
    vertices: &[Point3],
    tolerance: f64,
) -> Option<PolylineCurve>
where
    S: ParametricSurface3D,
    T: ExactTrimBoundary2D,
{
    let mut points = simplify_parameter_boundary(
        surface,
        trim_curve.exact_trim_boundary_2d(tolerance),
        tolerance,
    )
    .into_iter()
    .map(|uv| surface.subs(uv.x, uv.y))
    .collect::<Vec<_>>();
    if points.len() < 2 {
        return None;
    }
    let front = *vertices.get(edge_vertices.0)?;
    let back = *vertices.get(edge_vertices.1)?;
    let start = *points.first()?;
    let end = *points.last()?;
    let direct = start.distance2(front) + end.distance2(back);
    let reversed = start.distance2(back) + end.distance2(front);
    if reversed < direct {
        points.reverse();
    }
    Some(PolylineCurve(points))
}

fn point_segment_distance2(point: Point3, front: Point3, back: Point3) -> f64 {
    let segment = back - front;
    let denom = segment.dot(segment);
    if denom.so_small() {
        point.distance2(front)
    } else {
        let t = ((point - front).dot(segment) / denom).clamp(0.0, 1.0);
        let nearest = front + segment * t;
        point.distance2(nearest)
    }
}

fn simplify_parameter_boundary<S>(surface: &S, boundary: Vec<Point2>, tolerance: f64) -> Vec<Point2>
where
    S: ParametricSurface3D,
{
    let filtered = boundary
        .into_iter()
        .fold(Vec::<Point2>::new(), |mut acc, uv| {
            if acc.last().is_none_or(|last| !last.near(&uv)) {
                acc.push(uv);
            }
            acc
        });
    if filtered.len() <= 2
        || filtered
            .first()
            .zip(filtered.last())
            .is_some_and(|(front, back)| front.near(back))
    {
        filtered
    } else {
        let points = filtered
            .iter()
            .map(|uv| surface.subs(uv.x, uv.y))
            .collect::<Vec<_>>();
        let mut keep = vec![false; filtered.len()];
        keep[0] = true;
        keep[filtered.len() - 1] = true;
        let mut stack = vec![(0usize, filtered.len() - 1)];
        let tolerance2 = tolerance * tolerance;
        while let Some((front, back)) = stack.pop() {
            if back <= front + 1 {
                continue;
            }
            let (index, max_distance2) = ((front + 1)..back)
                .map(|index| {
                    (
                        index,
                        point_segment_distance2(points[index], points[front], points[back]),
                    )
                })
                .max_by(|lhs, rhs| lhs.1.total_cmp(&rhs.1))
                .unwrap_or((front, 0.0));
            if max_distance2 > tolerance2 {
                keep[index] = true;
                stack.push((front, index));
                stack.push((index, back));
            }
        }
        filtered
            .into_iter()
            .enumerate()
            .filter_map(|(index, uv)| keep[index].then_some(uv))
            .collect()
    }
}

fn resample_boundary(boundary: &[Point2], target_len: usize) -> Option<Vec<Point2>> {
    if target_len == 0 || boundary.is_empty() {
        None
    } else if target_len == 1 || boundary.len() == 1 {
        Some(vec![*boundary.first()?; target_len])
    } else {
        let cumulative = iter::once(0.0)
            .chain(boundary.windows(2).scan(0.0, |length, window| {
                *length += window[0].distance(window[1]);
                Some(*length)
            }))
            .collect::<Vec<_>>();
        let total = *cumulative.last()?;
        if total.so_small() {
            Some(vec![*boundary.first()?; target_len])
        } else {
            let last = *boundary.last()?;
            Some(
                (0..target_len)
                    .map(|index| {
                        if index + 1 == target_len {
                            last
                        } else {
                            let distance = total * index as f64 / (target_len - 1) as f64;
                            let upper = cumulative.partition_point(|value| *value < distance);
                            let segment = upper.saturating_sub(1).min(boundary.len() - 2);
                            let front = boundary[segment];
                            let back = boundary[segment + 1];
                            let front_distance = cumulative[segment];
                            let back_distance = cumulative[segment + 1];
                            let weight = if (back_distance - front_distance).so_small() {
                                0.0
                            } else {
                                (distance - front_distance) / (back_distance - front_distance)
                            };
                            Point2::new(
                                front.x + (back.x - front.x) * weight,
                                front.y + (back.y - front.y) * weight,
                            )
                        }
                    })
                    .collect(),
            )
        }
    }
}

fn log_mesh_trace(face_idx: usize, stage: &str, extra: impl AsRef<str>, elapsed: Instant) {
    if mesh_trace_enabled() {
        eprintln!(
            "mesh_trace face={face_idx} stage={stage} elapsed_ms={:.3} {}",
            elapsed.elapsed().as_secs_f64() * 1000.0,
            extra.as_ref(),
        );
    }
}

pub(super) trait SP<S>:
    Fn(&S, Point3, Option<(f64, f64)>) -> Option<(f64, f64)> + Parallelizable
{
}
impl<S, F> SP<S> for F where
    F: Fn(&S, Point3, Option<(f64, f64)>) -> Option<(f64, f64)> + Parallelizable
{
}

pub(super) fn search_parameter_sp<S: MeshableSurface>(trials: usize) -> impl SP<S> {
    move |surface: &S, point: Point3, hint: Option<(f64, f64)>| {
        surface
            .search_parameter(point, hint, trials)
            .or_else(|| surface.search_parameter(point, None, trials))
    }
}

pub(super) fn search_nearest_parameter_sp<S: RobustMeshableSurface>(trials: usize) -> impl SP<S> {
    move |surface: &S, point: Point3, hint: Option<(f64, f64)>| {
        surface
            .search_parameter(point, hint, trials)
            .or_else(|| surface.search_parameter(point, None, trials))
            .or_else(|| surface.search_nearest_parameter(point, hint, trials))
            .or_else(|| surface.search_nearest_parameter(point, None, trials))
    }
}

/// Compatibility wrapper: searches parameter with 100 trials.
#[cfg(test)]
pub(super) fn by_search_parameter<S: MeshableSurface>(
    surface: &S,
    point: Point3,
    hint: Option<(f64, f64)>,
) -> Option<(f64, f64)> {
    search_parameter_sp::<S>(100)(surface, point, hint)
}

/// Tessellates faces.

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn shell_tessellation<'a, C, S>(
    shell: &Shell<Point3, C, S>,
    tolerance: f64,
    sp: impl SP<S>,
    primitive_config: TessellationPrimitiveOptions,
    edge_angular_tolerance: Option<f64>,
) -> MeshedShell
where
    C: PolylineableCurve + 'a,
    S: PreMeshableSurface + 'a,
{
    shell_tessellation_with_failures(
        shell,
        tolerance,
        sp,
        primitive_config,
        edge_angular_tolerance,
    )
    .shell
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn shell_tessellation_with_failures<'a, C, S>(
    shell: &Shell<Point3, C, S>,
    tolerance: f64,
    sp: impl SP<S>,
    primitive_config: TessellationPrimitiveOptions,
    edge_angular_tolerance: Option<f64>,
) -> ShellTessellationResult
where
    C: PolylineableCurve + 'a,
    S: PreMeshableSurface + 'a,
{
    let vmap: HashMap<_, _> = shell
        .vertex_par_iter()
        .map(|v| (v.id(), v.mapped(Point3::clone)))
        .collect();
    let eset: HashMap<_, _> = shell.edge_par_iter().map(move |e| (e.id(), e)).collect();
    let edge_map: HashMap<_, _> = eset
        .into_par_iter()
        .map(move |(id, edge)| {
            // SAFETY: vmap was built from all vertices in the shell.
            let v0 = vmap.get(&edge.absolute_front().id()).unwrap();
            let v1 = vmap.get(&edge.absolute_back().id()).unwrap();
            let curve = edge.curve();
            let (poly, failure) =
                edge_sample_parts(sample_edge(&curve, tolerance, edge_angular_tolerance));
            // Spec 012 U4: `Edge::debug_new` is fallible now and this
            // `par_iter().map()` has no channel; `new_unchecked` is what the
            // release arm of `debug_new` already did.
            (id, (Edge::new_unchecked(v0, v1, poly), failure))
        })
        .collect();
    let create_edge = |edge: &Edge<Point3, C>| -> Edge<_, _> {
        // SAFETY: edge_map was built from all edges in the shell.
        let new_edge = &edge_map.get(&edge.id()).unwrap().0;
        match edge.orientation() {
            true => new_edge.clone(),
            false => new_edge.inverse(),
        }
    };
    let create_boundary =
        |wire: &Wire<Point3, C>| -> Wire<_, _> { wire.edge_iter().map(create_edge).collect() };
    let edge_failures: Vec<_> = shell
        .face_iter()
        .map(|face| {
            face.absolute_boundaries()
                .iter()
                .flat_map(|wire| wire.edge_iter())
                .find_map(|edge| edge_map[&edge.id()].1)
                .or_else(|| {
                    invalid_edge_angular_tolerance(edge_angular_tolerance)
                        .then_some(EdgeSamplingError::InvalidOptions)
                })
                .map(EdgeSamplingError::face_drop_reason)
        })
        .collect();
    let create_face = |(face_idx, face): (usize, &Face<Point3, C, S>)| {
        let wires: Vec<_> = face
            .absolute_boundaries()
            .iter()
            .map(create_boundary)
            .collect();
        if let Some(reason) = edge_failures[face_idx] {
            diagnostics::observe_edge_failure(&face.surface(), face_idx, reason);
            let mut refused = Face::new_unchecked(wires, None);
            if !face.orientation() {
                refused.invert();
            }
            return (refused, None);
        }
        shell_create_polygon(
            &face.surface(),
            wires,
            face.orientation(),
            tolerance,
            &sp,
            primitive_config,
            Some(face_idx),
        )
    };
    let outputs = shell
        .face_par_iter()
        .enumerate()
        .map(create_face)
        .collect::<Vec<_>>();
    let (faces, failures): (Vec<_>, Vec<_>) = outputs.into_iter().enumerate().fold(
        (Vec::new(), Vec::new()),
        |(mut faces, mut failures), (index, (face, error))| {
            faces.push(face);
            if let Some(error) = error {
                failures.push((index, error));
            }
            (faces, failures)
        },
    );
    ShellTessellationResult {
        shell: faces.into(),
        metric_failures: failures,
        edge_failures: edge_failures
            .into_iter()
            .enumerate()
            .filter_map(|(index, reason)| reason.map(|reason| (index, reason)))
            .collect(),
    }
}

/// Tessellates faces.

#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn shell_tessellation_single_thread<'a, C, S>(
    shell: &'a Shell<Point3, C, S>,
    tolerance: f64,
    sp: impl SP<S>,
    primitive_config: TessellationPrimitiveOptions,
    edge_angular_tolerance: Option<f64>,
) -> MeshedShell
where
    C: PolylineableCurve + 'a,
    S: PreMeshableSurface + 'a,
{
    shell_tessellation_single_thread_with_failures(
        shell,
        tolerance,
        sp,
        primitive_config,
        edge_angular_tolerance,
    )
    .shell
}

#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn shell_tessellation_single_thread_with_failures<'a, C, S>(
    shell: &'a Shell<Point3, C, S>,
    tolerance: f64,
    sp: impl SP<S>,
    primitive_config: TessellationPrimitiveOptions,
    edge_angular_tolerance: Option<f64>,
) -> ShellTessellationResult
where
    C: PolylineableCurve + 'a,
    S: PreMeshableSurface + 'a,
{
    let vmap: HashMap<_, _> = shell
        .vertex_iter()
        .map(|v| (v.id(), v.mapped(Point3::clone)))
        .collect();
    let eset: HashMap<_, _> = shell.edge_iter().map(move |e| (e.id(), e)).collect();
    let edge_map: HashMap<_, _> = eset
        .into_iter()
        .map(move |(id, edge)| {
            // SAFETY: vmap was built from all vertices in the shell.
            let v0 = vmap.get(&edge.absolute_front().id()).unwrap();
            let v1 = vmap.get(&edge.absolute_back().id()).unwrap();
            let curve = edge.curve();
            let (poly, failure) =
                edge_sample_parts(sample_edge(&curve, tolerance, edge_angular_tolerance));
            // Spec 012 U4: `Edge::debug_new` is fallible now and this
            // `par_iter().map()` has no channel; `new_unchecked` is what the
            // release arm of `debug_new` already did.
            (id, (Edge::new_unchecked(v0, v1, poly), failure))
        })
        .collect();
    let create_edge = |edge: &Edge<Point3, C>| -> Edge<_, _> {
        // SAFETY: edge_map was built from all edges in the shell.
        let new_edge = &edge_map.get(&edge.id()).unwrap().0;
        match edge.orientation() {
            true => new_edge.clone(),
            false => new_edge.inverse(),
        }
    };
    let create_boundary =
        |wire: &Wire<Point3, C>| -> Wire<_, _> { wire.edge_iter().map(create_edge).collect() };
    let edge_failures: Vec<_> = shell
        .face_iter()
        .map(|face| {
            face.absolute_boundaries()
                .iter()
                .flat_map(|wire| wire.edge_iter())
                .find_map(|edge| edge_map[&edge.id()].1)
                .or_else(|| {
                    invalid_edge_angular_tolerance(edge_angular_tolerance)
                        .then_some(EdgeSamplingError::InvalidOptions)
                })
                .map(EdgeSamplingError::face_drop_reason)
        })
        .collect();
    let create_face = |(face_idx, face): (usize, &Face<Point3, C, S>)| {
        let wires: Vec<_> = face
            .absolute_boundaries()
            .iter()
            .map(create_boundary)
            .collect();
        if let Some(reason) = edge_failures[face_idx] {
            diagnostics::observe_edge_failure(&face.surface(), face_idx, reason);
            let mut refused = Face::new_unchecked(wires, None);
            if !face.orientation() {
                refused.invert();
            }
            return (refused, None);
        }
        shell_create_polygon(
            &face.surface(),
            wires,
            face.orientation(),
            tolerance,
            &sp,
            primitive_config,
            Some(face_idx),
        )
    };
    let outputs = shell
        .face_iter()
        .enumerate()
        .map(create_face)
        .collect::<Vec<_>>();
    let (faces, failures): (Vec<_>, Vec<_>) = outputs.into_iter().enumerate().fold(
        (Vec::new(), Vec::new()),
        |(mut faces, mut failures), (index, (face, error))| {
            faces.push(face);
            if let Some(error) = error {
                failures.push((index, error));
            }
            (faces, failures)
        },
    );
    ShellTessellationResult {
        shell: faces.into(),
        metric_failures: failures,
        edge_failures: edge_failures
            .into_iter()
            .enumerate()
            .filter_map(|(index, reason)| reason.map(|reason| (index, reason)))
            .collect(),
    }
}

fn geometry_closed_axis_metadata<C, S: ParametricSurface3D>(
    shell: &CompressedShell<Point3, C, S>,
) -> Vec<(usize, usize, (f64, f64), Vec<usize>)> {
    shell
        .faces
        .iter()
        .enumerate()
        .filter_map(|(index, face)| {
            if face.boundaries.len() != 1 {
                return None;
            }
            let axis = match face.surface.finite_boundary_equal_axes()? {
                [true, false] => 0,
                [false, true] => 1,
                _ => return None,
            };
            let range = face.surface.try_range_tuple();
            let domain = [range.0, range.1][axis]?;
            if !domain.0.is_finite() || !domain.1.is_finite() || domain.0 >= domain.1 {
                return None;
            }
            let mut uses = HashMap::<usize, Vec<bool>>::default();
            for edge in &face.boundaries[0] {
                shell.edges.get(edge.index)?;
                uses.entry(edge.index).or_default().push(edge.orientation);
            }
            let seams = uses
                .into_iter()
                .filter_map(|(edge, orientations)| {
                    (orientations.len() == 2 && orientations[0] != orientations[1]).then_some(edge)
                })
                .collect::<Vec<_>>();
            (!seams.is_empty()).then_some((index, axis, domain, seams))
        })
        .collect()
}

/// Tessellates faces.
pub(super) fn cshell_tessellation<'a, C, S>(
    shell: &CompressedShell<Point3, C, S>,
    tolerance: f64,
    sp: impl SP<S>,
    primitive_config: TessellationPrimitiveOptions,
    edge_angular_tolerance: Option<f64>,
) -> MeshedCompressedShell
where
    C: PolylineableCurve + ParameterBoundary2D<S> + 'a,
    S: PreMeshableSurface + 'a,
{
    // Finite-seam recovery is pinned by tests/fixtures/choc-face30604.step;
    // tests/fixtures/choc-face27676.step pins existing aligned sample retention.
    // The negative control is
    // proven_closed_boundaries_do_not_turn_an_interior_bridge_into_a_seam.
    let closed_axes = geometry_closed_axis_metadata(shell);
    cshell_tessellation_with_closed_axis_metadata(
        shell,
        tolerance,
        sp,
        primitive_config,
        edge_angular_tolerance,
        &closed_axes,
        true,
    )
}

pub(super) fn cshell_tessellation_with_closed_axis_metadata<'a, C, S>(
    shell: &CompressedShell<Point3, C, S>,
    tolerance: f64,
    sp: impl SP<S>,
    primitive_config: TessellationPrimitiveOptions,
    edge_angular_tolerance: Option<f64>,
    closed_axis_metadata: &[(usize, usize, (f64, f64), Vec<usize>)],
    fallback_from_inferred_chart: bool,
) -> MeshedCompressedShell
where
    C: PolylineableCurve + ParameterBoundary2D<S> + 'a,
    S: PreMeshableSurface + 'a,
{
    cshell_tessellation_with_closed_axis_metadata_report(
        shell,
        tolerance,
        sp,
        primitive_config,
        edge_angular_tolerance,
        closed_axis_metadata,
        fallback_from_inferred_chart,
        false,
    )
    .0
}

/// As [`cshell_tessellation_with_closed_axis_metadata`]. With `preview_fallback`, a face the
/// ordinary route leaves without triangles is meshed from its shared boundary samples instead
/// (see `fallback.rs`), and its index is returned.
pub(super) fn cshell_tessellation_with_closed_axis_metadata_report<'a, C, S>(
    shell: &CompressedShell<Point3, C, S>,
    tolerance: f64,
    sp: impl SP<S>,
    primitive_config: TessellationPrimitiveOptions,
    edge_angular_tolerance: Option<f64>,
    closed_axis_metadata: &[(usize, usize, (f64, f64), Vec<usize>)],
    fallback_from_inferred_chart: bool,
    preview_fallback: bool,
) -> (MeshedCompressedShell, Vec<usize>)
where
    C: PolylineableCurve + ParameterBoundary2D<S> + 'a,
    S: PreMeshableSurface + 'a,
{
    cshell_tessellation_with_closed_axis_metadata_and_repair(
        shell,
        tolerance,
        sp,
        primitive_config,
        edge_angular_tolerance,
        closed_axis_metadata,
        fallback_from_inferred_chart,
        preview_fallback,
        None,
    )
}

pub(super) fn cshell_tessellation_with_closed_axis_metadata_and_repair<'a, C, S>(
    shell: &CompressedShell<Point3, C, S>,
    tolerance: f64,
    sp: impl SP<S>,
    primitive_config: TessellationPrimitiveOptions,
    edge_angular_tolerance: Option<f64>,
    closed_axis_metadata: &[(usize, usize, (f64, f64), Vec<usize>)],
    fallback_from_inferred_chart: bool,
    preview_fallback: bool,
    repair: Option<&repair::Capture>,
) -> (MeshedCompressedShell, Vec<usize>)
where
    C: PolylineableCurve + ParameterBoundary2D<S> + 'a,
    S: PreMeshableSurface + 'a,
{
    let vertices = shell.vertices.clone();
    let tessellate_edge = |edge: &CompressedEdge<C>| {
        let curve = &edge.curve;
        let (curve, failure) =
            edge_sample_parts(sample_edge(curve, tolerance, edge_angular_tolerance));
        (
            CompressedEdge {
                vertices: edge.vertices,
                curve,
            },
            failure,
        )
    };
    #[cfg(not(target_arch = "wasm32"))]
    let (edges, edge_failures): (Vec<_>, Vec<_>) =
        shell.edges.par_iter().map(tessellate_edge).unzip();
    #[cfg(target_arch = "wasm32")]
    let (edges, edge_failures): (Vec<_>, Vec<_>) = shell.edges.iter().map(tessellate_edge).unzip();
    let tessellate_face = |(face_idx, face): (usize, &CompressedFace<S>)| {
        let repair = repair.and_then(|capture| capture.face(face_idx));
        if let Some(trace) = repair {
            trace.unsupported(
                repair::Stage::SourceDomain,
                "generic native finite trim correspondence is unsupported",
            );
            trace.unsupported(
                repair::Stage::Approximation,
                "ordinary tessellation approximation is separate from repair",
            );
            trace.unsupported(
                repair::Stage::Numerical,
                "numerical evaluation error is separate from repair",
            );
        }
        let repair_mark = repair.map(|trace| trace.mark());
        let face_start = Instant::now();
        let boundaries = face.boundaries.clone();
        let surface = &face.surface;
        let face_orientation = face.orientation;
        if let Some(error) = invalid_edge_angular_tolerance(edge_angular_tolerance)
            .then_some(EdgeSamplingError::InvalidOptions)
            .or_else(|| {
                boundaries.iter().flatten().find_map(|edge| {
                    edge_failures
                        .as_slice()
                        .get(edge.index)
                        .copied()
                        .unwrap_or_else(|| {
                            edge_angular_tolerance.map(|_| EdgeSamplingError::InvalidSeed)
                        })
                })
            })
        {
            diagnostics::observe_edge_failure(surface, face_idx, error.face_drop_reason());
            return (
                CompressedFace {
                    boundaries,
                    orientation: face.orientation,
                    surface: None,
                },
                false,
            );
        }

        // Fast path: untrimmed face with bounded surface domain.
        let is_untrimmed = boundaries.iter().all(|wire| wire.is_empty());
        if is_untrimmed && let (Some(urange), Some(vrange)) = surface.try_range_tuple() {
            let polygon =
                untrimmed_tessellation(surface, (urange, vrange), tolerance, primitive_config.mode);
            log_mesh_trace(face_idx, "untrimmed", "ok", face_start);
            observe_face_drop(
                surface,
                Some(face_idx),
                Some(&polygon),
                true,
                boundaries.len(),
                0,
            );
            return (
                CompressedFace {
                    boundaries,
                    orientation: face.orientation,
                    surface: Some(polygon),
                },
                false,
            );
        }

        let create_edge = |(edge_index, edge_orientation): (usize, bool)| match edge_orientation {
            true => Some(edges.get(edge_index)?.curve.clone()),
            false => Some(edges.get(edge_index)?.curve.inverse()),
        };
        let create_boundary = |wire: &Vec<CompressedEdgeIndex>| {
            let exact_edges = |boundary_tolerance| {
                oriented_edge_indices(wire, face_orientation)
                    .filter_map(|(edge_index, edge_orientation)| {
                        let edge = shell.edges.get(edge_index)?;
                        let mut boundary = edge
                            .curve
                            .parameter_boundary_2d(surface, boundary_tolerance);
                        if !edge_orientation {
                            boundary = boundary.map(|mut boundary| {
                                boundary.reverse();
                                boundary
                            });
                        }
                        let polyline = if boundary_tolerance.near(&tolerance) {
                            match edge_orientation {
                                true => edges.get(edge_index)?.curve.clone(),
                                false => edges.get(edge_index)?.curve.inverse(),
                            }
                        } else {
                            fallback_polyline_curve(edge, edge_orientation, boundary_tolerance)
                        };
                        Some((
                            edge_index,
                            edge_orientation,
                            &edge.curve,
                            boundary,
                            polyline,
                        ))
                    })
                    .collect::<Vec<_>>()
            };
            if let Some((_, axis, domain, eligible_edges)) = closed_axis_metadata
                .iter()
                .find(|(index, _, _, _)| *index == face_idx)
            {
                let sampled_edges = exact_edges(tolerance);
                if fallback_from_inferred_chart {
                    // An existing aligned trim retains the caller's raw shared
                    // representatives. Use the inferred chart only to recover
                    // a boundary whose alignment actually failed.
                    let aligned = PolyBoundaryPiece::try_new_from_aligned_exact_with_repair(
                        surface,
                        sampled_edges.iter().map(|(_, _, _, boundary, polyline)| {
                            (boundary.clone(), polyline.clone())
                        }),
                        &sp,
                        tolerance,
                        edge_angular_tolerance.is_some(),
                        repair,
                    );
                    if let Some(aligned) = aligned.filter(|piece| {
                        if edge_angular_tolerance.is_some() {
                            piece.is_cdt_compatible()
                        } else {
                            piece.0.len() >= 3
                        }
                    }) {
                        return Some(aligned);
                    }
                }
                let mut uses = HashMap::<usize, Vec<bool>>::default();
                for (edge_index, orientation, _, _, _) in &sampled_edges {
                    uses.entry(*edge_index).or_default().push(*orientation);
                }
                let repeated_opposite = uses
                    .iter()
                    .filter(|(_, orientations)| {
                        orientations.len() == 2 && orientations[0] != orientations[1]
                    })
                    .map(|(edge_index, _)| *edge_index)
                    .collect::<Vec<_>>();
                if !repeated_opposite.is_empty()
                    && repeated_opposite
                        .iter()
                        .all(|edge| eligible_edges.contains(edge))
                {
                    let ranges = surface.try_range_tuple();
                    if *axis >= 2
                        || boundaries.len() != 1
                        || [ranges.0, ranges.1].get(*axis).copied().flatten() != Some(*domain)
                        || !domain.0.is_finite()
                        || !domain.1.is_finite()
                        || domain.0 >= domain.1
                    {
                        return None;
                    }
                    let input = vec![
                        sampled_edges
                            .iter()
                            .map(|(edge_index, orientation, _, _, polyline)| {
                                (*edge_index, *orientation, polyline.clone())
                            })
                            .collect::<Vec<_>>(),
                    ];
                    let mut axes = [false; 2];
                    axes[*axis] = true;
                    let lifted = PolyBoundaryPiece::try_new_from_closed_axis_edge_samples_with_shared_samples(
                        surface,
                        face_idx,
                        input,
                        axes,
                        &sp,
                        tolerance,
                        edge_angular_tolerance.is_some(),
                    )
                    .and_then(|mut pieces| pieces.pop());
                    if lifted.is_some() || !fallback_from_inferred_chart {
                        return lifted;
                    }
                    // A repeated source edge may be an interior bridge. Inferred
                    // finite-boundary equality does not make that edge a seam.
                    // Preserve the ordinary trim route when the whole chart is
                    // inapplicable; explicitly supplied certificates fail closed.
                }
            }
            if edge_angular_tolerance.is_some() {
                let aligned =
                    oriented_edge_indices(wire, face_orientation).map(|(index, orientation)| {
                        let mut boundary = shell.edges[index]
                            .curve
                            .parameter_boundary_2d(surface, tolerance);
                        if !orientation {
                            if let Some(boundary) = &mut boundary {
                                boundary.reverse();
                            }
                        }
                        let polyline = if orientation {
                            edges[index].curve.clone()
                        } else {
                            edges[index].curve.inverse()
                        };
                        (boundary, polyline)
                    });
                return PolyBoundaryPiece::try_new_from_aligned_exact_with_repair(
                    surface, aligned, &sp, tolerance, true, repair,
                )
                .filter(PolyBoundaryPiece::is_cdt_compatible)
                .or_else(|| {
                    PolyBoundaryPiece::try_new(
                        surface,
                        oriented_edge_indices(wire, face_orientation).filter_map(create_edge),
                        &sp,
                        tolerance,
                    )
                });
            }
            let create_exact_piece = |boundary_tolerance| {
                let exact_edges = exact_edges(boundary_tolerance);
                let direct_piece = PolyBoundaryPiece::try_new_from_exact(
                    surface,
                    exact_edges
                        .iter()
                        .map(|(_, orientation, curve, _, _)| (*orientation, *curve)),
                    boundary_tolerance,
                );
                let aligned_piece = PolyBoundaryPiece::try_new_from_aligned_exact_with_repair(
                    surface,
                    exact_edges
                        .into_iter()
                        .map(|(_, _, _, boundary, polyline)| (boundary, polyline)),
                    &sp,
                    boundary_tolerance,
                    false,
                    repair,
                );
                aligned_piece
                    .filter(|piece| piece.0.len() >= 3)
                    .or(direct_piece)
            };
            let exact_piece = create_exact_piece(tolerance);
            if mesh_trace_enabled() {
                let exact_count = wire
                    .iter()
                    .filter(|edge_idx| {
                        shell
                            .edges
                            .get(edge_idx.index)
                            .and_then(|edge| edge.curve.parameter_boundary_2d(surface, tolerance))
                            .is_some()
                    })
                    .count();
                eprintln!(
                    "mesh_trace face={face_idx} stage=exact-boundary edges={}/{} success={}",
                    exact_count,
                    wire.len(),
                    exact_piece.is_some(),
                );
            }
            exact_piece
                .or_else(|| {
                    boundary_tolerance_candidates(tolerance)
                        .into_iter()
                        .skip(1)
                        .find_map(create_exact_piece)
                })
                .or_else(|| {
                    let wire_iter =
                        oriented_edge_indices(wire, face_orientation).filter_map(create_edge);
                    PolyBoundaryPiece::try_new(surface, wire_iter, &sp, tolerance)
                })
        };
        let boundary_start = Instant::now();
        let preboundary: Option<Vec<_>> = boundaries.iter().map(create_boundary).collect();
        if let Some(preboundary) = &preboundary {
            let point_count = preboundary.iter().map(|piece| piece.0.len()).sum::<usize>();
            log_mesh_trace(
                face_idx,
                "preboundary",
                format!("loops={} points={point_count}", preboundary.len()),
                boundary_start,
            );
        } else {
            log_mesh_trace(face_idx, "preboundary", "failed", boundary_start);
        }
        let mut metric_failure = None;
        let polygon: Option<PolygonMesh> = preboundary.and_then(|preboundary| {
            let close_start = Instant::now();
            let boundary = PolyBoundary::new_with_source_vertices_and_repair(
                preboundary,
                &surface,
                tolerance,
                boundaries.iter().flatten().flat_map(|edge_use| {
                    let source = &shell.edges[edge_use.index];
                    let sampled = &edges[edge_use.index].curve;
                    sampled
                        .first()
                        .zip(sampled.last())
                        .into_iter()
                        .flat_map(|(front, back)| {
                            [
                                (*front, source.vertices.0, shell.vertices[source.vertices.0]),
                                (*back, source.vertices.1, shell.vertices[source.vertices.1]),
                            ]
                        })
                }),
                repair,
            );

            if !boundary.is_valid() {
                return None;
            }
            log_mesh_trace(
                face_idx,
                "polyboundary",
                format!(
                    "closed_loops={} uv_min=({:.6},{:.6}) uv_max=({:.6},{:.6})",
                    boundary.loops.len(),
                    boundary.uv_min.x,
                    boundary.uv_min.y,
                    boundary.uv_max.x,
                    boundary.uv_max.y,
                ),
                close_start,
            );
            let mesh_start = Instant::now();
            let (polygon, error) = lenient_metric_result(trimming_tessellation(
                &surface,
                &boundary,
                tolerance,
                primitive_config,
                face_idx,
            ));
            if let Some(error) = error {
                metric_failure = Some(error);
                return None;
            }
            let Some(polygon) = polygon else {
                return None;
            };
            log_mesh_trace(
                face_idx,
                "trimmed",
                format!(
                    "tris={} quads={}",
                    polygon.tri_faces().len(),
                    polygon.quad_faces().len()
                ),
                mesh_start,
            );
            Some(polygon)
        });
        log_mesh_trace(
            face_idx,
            "face",
            format!("surface={}", polygon.is_some()),
            face_start,
        );
        let mut used_fallback = false;
        let polygon = match polygon {
            Some(mesh) if !mesh.faces().is_empty() => Some(mesh),
            other if preview_fallback => {
                let loops = boundaries
                    .iter()
                    .map(|wire| {
                        oriented_edge_indices(wire, face_orientation)
                            .filter_map(create_edge)
                            .flat_map(|curve| curve.iter().copied().collect::<Vec<_>>())
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                match preview_boundary_triangulation(surface, &loops, tolerance, &sp) {
                    Some(mesh) => {
                        used_fallback = true;
                        if let (Some(trace), Some(mark)) = (repair, repair_mark) {
                            trace.discard_since(mark);
                            trace.unsupported(
                                repair::Stage::Fallback,
                                "fallback native finite-domain correspondence is unsupported",
                            );
                        }
                        log_mesh_trace(
                            face_idx,
                            "preview-fallback",
                            format!("tris={}", mesh.tri_faces().len()),
                            face_start,
                        );
                        Some(mesh)
                    }
                    None => other,
                }
            }
            other => other,
        };
        if let Some(error) = metric_failure.filter(|_| !used_fallback) {
            observe_metric_failure(
                surface,
                Some(face_idx),
                error,
                boundaries.len(),
                boundaries.iter().map(|wire| wire.len()).sum(),
            );
        } else {
            observe_face_drop(
                surface,
                Some(face_idx),
                polygon.as_ref(),
                is_untrimmed,
                boundaries.len(),
                boundaries.iter().map(|wire| wire.len()).sum(),
            );
        }
        (
            CompressedFace {
                boundaries,
                orientation: face.orientation,
                surface: polygon,
            },
            used_fallback,
        )
    };
    #[cfg(not(target_arch = "wasm32"))]
    let (faces, fallback): (Vec<_>, Vec<_>) = shell
        .faces
        .par_iter()
        .enumerate()
        .map(tessellate_face)
        .unzip();
    #[cfg(target_arch = "wasm32")]
    let (faces, fallback): (Vec<_>, Vec<_>) =
        shell.faces.iter().enumerate().map(tessellate_face).unzip();
    let fallback_faces = fallback
        .into_iter()
        .enumerate()
        .filter_map(|(index, used)| used.then_some(index))
        .collect();
    (
        MeshedCompressedShell {
            vertices,
            edges,
            faces,
            vertex_stable_ids: None,
            edge_stable_ids: None,
            face_stable_ids: None,
        },
        fallback_faces,
    )
}

pub(super) fn trimmed_cshell_tessellation<'a, C, S, T>(
    shell: &TrimmedShell<C, S, T>,
    tolerance: f64,
    sp: impl SP<S>,
    primitive_config: TessellationPrimitiveOptions,
    edge_angular_tolerance: Option<f64>,
) -> MeshedCompressedShell
where
    C: PolylineableCurve + ParameterBoundary2D<S> + ExactParameterBoundary2D<S> + 'a,
    S: PreMeshableSurface + 'a,
    T: ExactTrimBoundary2D + Parallelizable + 'a,
    <C as ExactParameterBoundary2D<S>>::BoundaryCurve: ExactTrimBoundary2D,
{
    compressed_trimmed_shell_tessellation_with_isoparams(
        shell,
        tolerance,
        sp,
        primitive_config,
        None,
        edge_angular_tolerance,
    )
    .shell
}

pub(super) fn compressed_trimmed_shell_tessellation_with_isoparams<'a, C, S, T>(
    shell: &TrimmedShell<C, S, T>,
    tolerance: f64,
    sp: impl SP<S>,
    primitive_config: TessellationPrimitiveOptions,
    isoparametric_options: Option<IsoparametricCurveOptions>,
    edge_angular_tolerance: Option<f64>,
) -> CompressedShellTessellation
where
    C: PolylineableCurve + ParameterBoundary2D<S> + ExactParameterBoundary2D<S> + 'a,
    S: PreMeshableSurface + 'a,
    T: ExactTrimBoundary2D + Parallelizable + 'a,
    <C as ExactParameterBoundary2D<S>>::BoundaryCurve: ExactTrimBoundary2D,
{
    let vertices = shell.vertices.clone();
    let exact_edge_polylines = shell
        .faces
        .iter()
        .filter(|_| edge_angular_tolerance.is_none())
        .flat_map(|face| {
            face.boundaries.iter().flat_map(move |wire| {
                wire.iter().filter_map(move |edge_use| {
                    edge_use
                        .trim_curve
                        .as_ref()
                        .map(|trim_curve| (edge_use.index, &face.surface, trim_curve))
                })
            })
        })
        .fold(
            HashMap::<usize, PolylineCurve>::default(),
            |mut acc, (edge_idx, surface, trim_curve)| {
                let edge = &shell.edges[edge_idx];
                let candidate = polyline_from_trim_curve(
                    surface,
                    trim_curve,
                    edge.vertices,
                    &vertices,
                    tolerance,
                )
                .map(|curve| {
                    fallback_polyline_curve_with_min_points(edge, true, tolerance, curve.len())
                })
                .unwrap_or_else(|| fallback_polyline_curve(edge, true, tolerance));
                match acc.entry(edge_idx) {
                    Entry::Occupied(mut entry) => {
                        if candidate.len() > entry.get().len() {
                            entry.insert(candidate);
                        }
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(candidate);
                    }
                }
                acc
            },
        );
    let tessellate_edge = |(edge_idx, edge): (usize, &CompressedEdge<C>)| {
        let curve = &edge.curve;
        if edge_angular_tolerance.is_some() {
            let (curve, failure) =
                edge_sample_parts(sample_edge(curve, tolerance, edge_angular_tolerance));
            return (
                CompressedEdge {
                    vertices: edge.vertices,
                    curve,
                },
                failure,
            );
        }
        (
            CompressedEdge {
                vertices: edge.vertices,
                curve: exact_edge_polylines
                    .get(&edge_idx)
                    .cloned()
                    .unwrap_or_else(|| {
                        PolylineCurve::from_curve(curve, curve.range_tuple(), tolerance)
                    }),
            },
            None,
        )
    };
    #[cfg(not(target_arch = "wasm32"))]
    let (edges, edge_failures): (Vec<_>, Vec<_>) = shell
        .edges
        .par_iter()
        .enumerate()
        .map(tessellate_edge)
        .unzip();
    #[cfg(target_arch = "wasm32")]
    let (edges, edge_failures): (Vec<_>, Vec<_>) =
        shell.edges.iter().enumerate().map(tessellate_edge).unzip();
    let tessellate_face = |(face_idx, face): (usize, &CompressedTrimmedFace<S, T>)| {
        let face_start = Instant::now();
        log_mesh_trace(face_idx, "face-start", "", face_start);
        let boundaries = face
            .boundaries
            .iter()
            .map(|wire| {
                wire.iter()
                    .map(|edge_use| CompressedEdgeIndex {
                        index: edge_use.index,
                        orientation: edge_use.orientation,
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let surface = &face.surface;
        let face_orientation = face.orientation;
        if let Some(error) = invalid_edge_angular_tolerance(edge_angular_tolerance)
            .then_some(EdgeSamplingError::InvalidOptions)
            .or_else(|| {
                boundaries.iter().flatten().find_map(|edge| {
                    edge_failures
                        .as_slice()
                        .get(edge.index)
                        .copied()
                        .unwrap_or_else(|| {
                            edge_angular_tolerance.map(|_| EdgeSamplingError::InvalidSeed)
                        })
                })
            })
        {
            diagnostics::observe_edge_failure(surface, face_idx, error.face_drop_reason());
            return (
                CompressedFace {
                    boundaries,
                    orientation: face.orientation,
                    surface: None,
                },
                Vec::new(),
            );
        }

        let is_untrimmed = boundaries.iter().all(|wire| wire.is_empty());
        if is_untrimmed && let (Some(urange), Some(vrange)) = surface.try_range_tuple() {
            let polygon =
                untrimmed_tessellation(surface, (urange, vrange), tolerance, primitive_config.mode);
            let isoparams = isoparametric_options
                .map(|options| untrimmed_isoparametric_curves(surface, (urange, vrange), options))
                .unwrap_or_default();
            log_mesh_trace(face_idx, "untrimmed", "ok", face_start);
            observe_face_drop(
                surface,
                Some(face_idx),
                Some(&polygon),
                true,
                boundaries.len(),
                0,
            );
            return (
                CompressedFace {
                    boundaries,
                    orientation: face.orientation,
                    surface: Some(polygon),
                },
                isoparams,
            );
        }

        let create_boundary = |wire: &Vec<CompressedEdgeUse<T>>| {
            if edge_angular_tolerance.is_some() {
                let aligned = oriented_trimmed_edge_uses(wire, face_orientation).map(
                    |(index, orientation, trim)| {
                        let polyline = if orientation {
                            edges[index].curve.clone()
                        } else {
                            edges[index].curve.inverse()
                        };
                        (trim, &shell.edges[index].curve, polyline)
                    },
                );
                return PolyBoundaryPiece::try_new_from_aligned_trimmed(
                    surface, aligned, &sp, tolerance, true,
                )
                .filter(PolyBoundaryPiece::is_cdt_compatible);
            }
            let wire_start = Instant::now();
            let all_exact = wire.iter().all(|edge_use| edge_use.trim_curve.is_some());
            let has_shared_edge_polyline = wire
                .iter()
                .any(|edge_use| exact_edge_polylines.contains_key(&edge_use.index));
            let direct_trim_piece = || {
                PolyBoundaryPiece::try_new_from_trimmed(
                    surface,
                    oriented_trimmed_edge_uses(wire, face_orientation).filter_map(
                        |(edge_index, edge_orientation, trim_curve)| {
                            shell
                                .edges
                                .get(edge_index)
                                .map(|edge| (edge_orientation, trim_curve, &edge.curve))
                        },
                    ),
                    tolerance,
                )
            };
            let aligned_trim_piece = || {
                PolyBoundaryPiece::try_new_from_aligned_trimmed(
                    surface,
                    oriented_trimmed_edge_uses(wire, face_orientation).filter_map(
                        |(edge_index, edge_orientation, trim_curve)| {
                            shell.edges.get(edge_index).map(|edge| {
                                let mut polyline = exact_edge_polylines
                                    .get(&edge_index)
                                    .cloned()
                                    .unwrap_or_else(|| {
                                        fallback_polyline_curve(edge, true, tolerance)
                                    });
                                if !edge_orientation {
                                    polyline.invert();
                                }
                                (trim_curve, &edge.curve, polyline)
                            })
                        },
                    ),
                    &sp,
                    tolerance,
                    false,
                )
            };
            let exact_piece = if has_shared_edge_polyline {
                aligned_trim_piece()
                    .filter(PolyBoundaryPiece::is_cdt_compatible)
                    .or_else(direct_trim_piece)
            } else {
                direct_trim_piece().or_else(aligned_trim_piece)
            }
            .or_else(|| {
                if all_exact {
                    None
                } else {
                    boundary_tolerance_candidates(tolerance)
                        .into_iter()
                        .skip(1)
                        .find_map(|boundary_tolerance| {
                            PolyBoundaryPiece::try_new_from_trimmed(
                                surface,
                                oriented_trimmed_edge_uses(wire, face_orientation).filter_map(
                                    |(edge_index, edge_orientation, trim_curve)| {
                                        shell
                                            .edges
                                            .get(edge_index)
                                            .map(|edge| (edge_orientation, trim_curve, &edge.curve))
                                    },
                                ),
                                boundary_tolerance,
                            )
                            .or_else(|| {
                                let wire_iter = oriented_trimmed_edge_uses(wire, face_orientation)
                                    .filter_map(|(edge_index, edge_orientation, _)| {
                                        shell.edges.get(edge_index).map(|edge| {
                                            fallback_polyline_curve(
                                                edge,
                                                edge_orientation,
                                                boundary_tolerance,
                                            )
                                        })
                                    });
                                PolyBoundaryPiece::try_new(surface, wire_iter, &sp, tolerance)
                            })
                        })
                }
            });
            if mesh_trace_enabled() {
                let exact_count = wire
                    .iter()
                    .filter(|edge_use| {
                        edge_use.trim_curve.as_ref().is_some_and(|trim_curve| {
                            !trim_curve.exact_trim_boundary_2d(tolerance).is_empty()
                        }) || shell.edges.get(edge_use.index).is_some_and(|edge| {
                            edge.curve
                                .parameter_boundary_2d(surface, tolerance)
                                .is_some()
                        })
                    })
                    .count();
                eprintln!(
                    "mesh_trace face={face_idx} stage=exact-boundary edges={}/{} success={}",
                    exact_count,
                    wire.len(),
                    exact_piece.is_some(),
                );
            }
            log_mesh_trace(
                face_idx,
                "wire",
                format!("edges={} success={}", wire.len(), exact_piece.is_some()),
                wire_start,
            );
            exact_piece
        };
        let boundary_start = Instant::now();
        let preboundary: Option<Vec<_>> = face.boundaries.iter().map(create_boundary).collect();
        if let Some(preboundary) = &preboundary {
            let point_count = preboundary.iter().map(|piece| piece.0.len()).sum::<usize>();
            log_mesh_trace(
                face_idx,
                "preboundary",
                format!("loops={} points={point_count}", preboundary.len()),
                boundary_start,
            );
        } else {
            log_mesh_trace(face_idx, "preboundary", "failed", boundary_start);
        }
        let mut metric_failure = None;
        let (polygon, isoparams) = preboundary
            .and_then(|preboundary| {
                let close_start = Instant::now();
                let boundary = PolyBoundary::new_with_source_vertices(
                    preboundary,
                    surface,
                    tolerance,
                    boundaries.iter().flatten().flat_map(|edge_use| {
                        let source = &shell.edges[edge_use.index];
                        let sampled = &edges[edge_use.index].curve;
                        sampled
                            .first()
                            .zip(sampled.last())
                            .into_iter()
                            .flat_map(|(front, back)| {
                                [
                                    (*front, source.vertices.0, shell.vertices[source.vertices.0]),
                                    (*back, source.vertices.1, shell.vertices[source.vertices.1]),
                                ]
                            })
                    }),
                );

                if !boundary.is_valid() {
                    return None;
                }
                log_mesh_trace(
                    face_idx,
                    "polyboundary",
                    format!(
                        "closed_loops={} uv_min=({:.6},{:.6}) uv_max=({:.6},{:.6})",
                        boundary.loops.len(),
                        boundary.uv_min.x,
                        boundary.uv_min.y,
                        boundary.uv_max.x,
                        boundary.uv_max.y,
                    ),
                    close_start,
                );
                let isoparams = isoparametric_options
                    .map(|options| boundary.isoparametric_curves(surface, options))
                    .unwrap_or_default();
                let mesh_start = Instant::now();
                let (polygon, error) = lenient_metric_result(trimming_tessellation(
                    surface,
                    &boundary,
                    tolerance,
                    primitive_config,
                    face_idx,
                ));
                if let Some(error) = error {
                    metric_failure = Some(error);
                    return None;
                }
                let Some(polygon) = polygon else {
                    return None;
                };
                log_mesh_trace(
                    face_idx,
                    "trimmed",
                    format!(
                        "tris={} quads={}",
                        polygon.tri_faces().len(),
                        polygon.quad_faces().len()
                    ),
                    mesh_start,
                );
                Some((Some(polygon), isoparams))
            })
            .unwrap_or_default();
        log_mesh_trace(
            face_idx,
            "face",
            format!("surface={}", polygon.is_some()),
            face_start,
        );
        if let Some(error) = metric_failure {
            observe_metric_failure(
                surface,
                Some(face_idx),
                error,
                boundaries.len(),
                boundaries.iter().map(|wire| wire.len()).sum(),
            );
        } else {
            observe_face_drop(
                surface,
                Some(face_idx),
                polygon.as_ref(),
                is_untrimmed,
                boundaries.len(),
                boundaries.iter().map(|wire| wire.len()).sum(),
            );
        }
        (
            CompressedFace {
                boundaries,
                orientation: face.orientation,
                surface: polygon,
            },
            isoparams,
        )
    };
    #[cfg(not(target_arch = "wasm32"))]
    let face_outputs = shell
        .faces
        .par_iter()
        .enumerate()
        .map(tessellate_face)
        .collect::<Vec<_>>();
    #[cfg(target_arch = "wasm32")]
    let face_outputs = shell
        .faces
        .iter()
        .enumerate()
        .map(tessellate_face)
        .collect::<Vec<_>>();
    let (faces, face_isoparams) = face_outputs.into_iter().unzip();
    CompressedShellTessellation {
        shell: MeshedCompressedShell {
            vertices,
            edges,
            faces,
            vertex_stable_ids: None,
            edge_stable_ids: None,
            face_stable_ids: None,
        },
        face_isoparams,
    }
}

fn shell_create_polygon<S: PreMeshableSurface>(
    surface: &S,
    wires: Vec<Wire<Point3, PolylineCurve>>,
    orientation: bool,
    tolerance: f64,
    sp: impl SP<S>,
    primitive_config: TessellationPrimitiveOptions,
    face_idx: Option<usize>,
) -> (
    Face<Point3, PolylineCurve, Option<PolygonMesh>>,
    Option<MetricRefinementError>,
) {
    // Fast path: untrimmed face with bounded surface domain.
    let is_untrimmed = wires.iter().all(|w| w.is_empty());
    let mut metric_failure = None;
    let polygon = if is_untrimmed {
        if let (Some(urange), Some(vrange)) = surface.try_range_tuple() {
            Some(untrimmed_tessellation(
                surface,
                (urange, vrange),
                tolerance,
                primitive_config.mode,
            ))
        } else {
            None
        }
    } else {
        boundary::projection_debug_set_face(face_idx);
        let preboundary = wires
            .iter()
            .map(|wire: &Wire<_, _>| {
                let wire_iter = wire.iter().map(Edge::oriented_curve);
                PolyBoundaryPiece::try_new(surface, wire_iter, &sp, tolerance)
            })
            .collect::<Option<Vec<_>>>();
        preboundary.and_then(|preboundary| {
            let mut source_ids = HashMap::default();
            let mut endpoints = Vec::new();
            for wire in &wires {
                for edge in wire.iter() {
                    let curve = edge.curve();
                    for (point, vertex) in
                        curve
                            .first()
                            .zip(curve.last())
                            .into_iter()
                            .flat_map(|(front, back)| {
                                [
                                    (*front, edge.absolute_front()),
                                    (*back, edge.absolute_back()),
                                ]
                            })
                    {
                        let next = source_ids.len();
                        let id = *source_ids.entry(vertex.id()).or_insert(next);
                        endpoints.push((point, id, vertex.point()));
                    }
                }
            }
            let boundary = PolyBoundary::new_with_source_vertices(
                preboundary,
                &surface,
                tolerance,
                endpoints.iter().copied(),
            );

            if !boundary.is_valid() {
                None
            } else {
                let (polygon, error) = lenient_metric_result(trimming_tessellation(
                    surface,
                    &boundary,
                    tolerance,
                    primitive_config,
                    usize::MAX,
                ));
                metric_failure = error;
                polygon
            }
        })
    };
    let loops = wires.len();
    let edges = wires.iter().map(|wire| wire.len()).sum();
    if let Some(error) = metric_failure {
        observe_metric_failure(surface, face_idx, error, loops, edges);
    } else {
        observe_face_drop(
            surface,
            face_idx,
            polygon.as_ref(),
            is_untrimmed,
            loops,
            edges,
        );
    }
    let mut new_face = Face::new_unchecked(wires, polygon);
    if !orientation {
        new_face.invert();
    }
    (new_face, metric_failure)
}

#[test]
#[ignore]
#[cfg(not(target_arch = "wasm32"))]
fn par_bench() {
    use monstertruck_modeling::*;
    use std::time::Instant;
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../resources/shape/bottle.json"
    );
    let json = std::fs::read_to_string(path).expect("bottle.json resource missing");
    let solid: Solid = serde_json::from_str(&json).unwrap();
    let shell = solid.into_boundaries().pop().unwrap();

    let instant = Instant::now();
    (0..100).for_each(|_| {
        let _shell = shell_tessellation(
            &shell,
            0.01,
            by_search_parameter,
            TessellationPrimitiveOptions::default(),
            None,
        );
    });
    println!("{}ms", instant.elapsed().as_millis());

    let instant = Instant::now();
    (0..100).for_each(|_| {
        let _shell = shell_tessellation_single_thread(
            &shell,
            0.01,
            by_search_parameter,
            TessellationPrimitiveOptions::default(),
            None,
        );
    });
    println!("{}ms", instant.elapsed().as_millis());
}

#[cfg(test)]
mod tests;

//! `PolyBoundaryPiece`: projecting a face's 3D wire into the surface's
//! parameter space, including the restart ladder and the verified-footpoint
//! solver rung.
//!
//! Split out of the module file so the source stays readable; the module path
//! is unchanged.

use super::*;
use mesh::spade_round;
use std::sync::OnceLock;

type SurfaceParameterBounds = (Option<(f64, f64)>, Option<(f64, f64)>);

/// Trials for the verified nearest-parameter rung, matching the 100 the
/// robust path's `search_nearest_parameter_sp` uses by default.
const VERIFIED_FOOTPOINT_TRIALS: usize = 100;

/// `MT_MESH_VERIFIED_FOOTPOINT`: the spec 012 V1 treatment arm, **DEFAULT ON**.
/// See [`verified_footpoint`] for what it does and what it moves. Set the
/// variable to `0` for the pre-`882f9b6f` control arm -- both arms stay in one
/// binary (ledger M13). Read once, so a mid-run change is not observable.
fn verified_footpoint_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| !matches!(env::var("MT_MESH_VERIFIED_FOOTPOINT").as_deref(), Ok("0")))
}

/// The missing solver rung on the `Shell::triangulation` path, with a residual
/// check that ties acceptance to the caller's own tessellation tolerance.
///
/// WHY THIS EXISTS -- measured, spec 012 (`MT_DEBUG_BOUNDARY_PROJECTION=1`).
/// `search_parameter_sp` is CHECKED-ONLY: `search_parameter(hint)` then
/// `search_parameter(None)`. `search_parameter`'s Newton minimises the
/// TANGENTIAL residual, so it lands on the footpoint, and then accepts it only
/// if `surface.evaluate(u, v).near(&point)` -- an ABSOLUTE test at
/// `TOLERANCE = 1e-6`. Boundary polylines are sampled from the EDGE CURVE,
/// which in interchange data does not ride exactly on the face's surface.
///
/// Every boundary vertex that drops a face on the in-repo fixtures fails by
/// that margin and by nothing else. Censused at 1e-1/1e-2/1e-3, hinted
/// nearest-parameter from the previous vertex converges for ALL of them, with
/// residual `1.03e-6 .. 2.51e-6` -- between 1.03x and 2.51x the fixed 1e-6, and
/// three orders of magnitude INSIDE the finest chord ever requested (1e-3):
///
/// * coffy #219 faces 11/21/26/33 (`rational-bspline`, non-periodic, model
///   extent ~3.8e3): residual 1.03e-6 .. 2.51e-6.
/// * ap224 #1727 faces 28-39 and 44-47 (periodic-u revolutions, model extent
///   ~1.2): residual 2.38e-6 and 1.13e-6.
///
/// So the two "families" the drop census suggested are ONE cause. The
/// hint is load-bearing and the unhinted seed is not a substitute: seeded at
/// mid-domain the same solve returns residuals of 0.0625, 1.12 and 64.0, or
/// diverges. This rung is therefore hinted-only.
///
/// WHY IT CANNOT PRODUCE A SILENT PARTIAL. The result is accepted only when
/// `|surface(u, v) - point| <= tolerance`, the very chord the caller asked the
/// tessellator to honour. A boundary vertex admitted here is inside the
/// accuracy the output already advertises; anything worse is refused and the
/// face still drops, loudly, through `observe_face_drop`.
///
/// The free function is used deliberately: it needs only `ParametricSurface`,
/// so the rung is reachable under `PreMeshableSurface` with no bound change and
/// no API break. The `SearchNearestParameter` TRAIT bound -- not the algorithm
/// -- is what kept this path checked-only.
///
/// # `MT_MESH_VERIFIED_FOOTPOINT=0` -- the control arm (spec 019 R1c, task #80)
///
/// **DEFAULT ON.** Setting the variable to `0` selects the pre-`882f9b6f`
/// behaviour -- the rung returns `None` and the face drops as it used to -- so
/// both arms live in ONE binary (ledger M13). This fix shipped without an A/B
/// arm, and its absence is what kept the regression below unattributable for
/// three rounds of analysis.
///
/// **WHY THE ARM IS NOT COSMETIC: this rung changes BOOLEAN OUTPUT, not just
/// mesh output.** The boolean meshes shells internally at the operation's own
/// tolerance (`max(tol, 10*TOLERANCE)` in
/// `integrate::healing::{shell_signed_volume, solid_signed_volume}` and
/// `pipeline::orientation::resolve_material_side`), and carries a per-face
/// `PolygonMesh` into the SSI. A face that this rung rescues is a face those
/// consumers previously did not see, so the rung can move the assembled B-rep.
///
/// Measured on `SW-B3-SLIVER-ODD-ANGLE-Uab-T00-S1-D106-Ga`
/// (`sweep::runner::tests::smoke_sliver_union_near_contact_pass`), one binary,
/// oracle `1.8060271164836628`:
///
/// | arm | result B-rep | hole-free volume | vs oracle |
/// |---|---|---|---|
/// | `MT_MESH_VERIFIED_FOOTPOINT=0` | 24 faces, 88 edges | 1.8058267975602282 | -0.0111% |
/// | default (ON) | 27 faces, 94 edges | 1.7588726252461468 | **-2.611%** |
///
/// The rescued vertex is `(1, 0.995969663, 1)` at residual `3.927e-3` against
/// the operation's `tolerance = 0.01`: far outside the old fixed `1e-6`, well
/// inside what the caller asked for. The rung is doing exactly what it says.
///
/// **The defect it exposes is elsewhere, and reverting the rung is the wrong
/// fix.** `or()` arms the P-D106 certified sub-tolerance face repair ONLY on the
/// ERROR path of a completed first pass (`integrate::mod.rs`, "no currently-green
/// union can move"). With the rung off, the first pass refuses and the repair
/// runs; with it on the first pass CLOSES A MANIFOLD, so the repair never runs
/// and its un-repaired answer is returned. Counted under `MT_BOOL_TRACE`: the
/// control arm executes the pipeline twice (`divide_face` 46, `classify` 4,
/// `build_shells` 2), the default arm once (24, 2, 1). A failure-path-only
/// repair is silently bypassed the moment an unrelated improvement stops the
/// failure -- ledger class C18.
fn verified_footpoint<S: PreMeshableSurface>(
    surface: &S,
    point: Point3,
    hint: (f64, f64),
    tolerance: f64,
) -> Option<(f64, f64)> {
    if !verified_footpoint_enabled() {
        return None;
    }
    let (u, v) =
        algo::surface::search_nearest_parameter(surface, point, hint, VERIFIED_FOOTPOINT_TRIALS)?;
    if !u.is_finite() || !v.is_finite() {
        return None;
    }
    (surface.subs(u, v).distance(point) <= tolerance).then_some((u, v))
}

/// Outcome of matching an actual shared XYZ to a proved finite point boundary.
#[derive(Debug, PartialEq)]
enum FiniteBoundaryProjection {
    Unproved,
    Refused,
    Footpoint((f64, f64)),
}

/// Exact physical anchor identity distinguishes the pole from distinct nearby
/// samples. A proved pole needs one incoming transverse representative; missing
/// or ambiguous roles refuse this restart rather than guessing a chart branch.
fn finite_collapsed_boundary_footpoint<S: PreMeshableSurface>(
    surface: &S,
    point: Point3,
    previous: Option<(f64, f64)>,
) -> FiniteBoundaryProjection {
    use FiniteBoundaryProjection::*;
    let Some(boundaries) = surface.finite_collapsed_boundaries() else {
        return Unproved;
    };
    let (Some(u), Some(v)) = surface.try_range_tuple() else {
        return Refused;
    };
    if ![u, v]
        .iter()
        .all(|(a, b)| a.is_finite() && b.is_finite() && a < b)
    {
        return Refused;
    }
    let ranges = [[u.0, u.1], [v.0, v.1]];
    let mut matched = None;
    for axis in 0..2 {
        for end in 0..2 {
            if boundaries[axis][end] {
                let mut anchor = [u.0, v.0];
                anchor[axis] = ranges[axis][end];
                if surface.evaluate(anchor[0], anchor[1]) == point {
                    if matched.is_some() {
                        return Refused;
                    }
                    matched = Some((axis, end));
                }
            }
        }
    }
    let Some((axis, end)) = matched else {
        return Unproved;
    };
    let Some(previous) = previous else {
        return Refused;
    };
    let mut uv = [previous.0, previous.1];
    let transverse = uv[1 - axis];
    if !transverse.is_finite()
        || transverse < ranges[1 - axis][0]
        || transverse > ranges[1 - axis][1]
    {
        return Refused;
    }
    uv[axis] = ranges[axis][end];
    if surface.evaluate(uv[0], uv[1]).near(&point) {
        Footpoint((uv[0], uv[1]))
    } else {
        Refused
    }
}

fn orient_boundary_to_edge<C, S>(
    surface: &S,
    boundary: &mut [Point2],
    edge_curve: &C,
    orientation: bool,
) where
    C: BoundedCurve<Point = Point3> + ParametricCurve3D,
    S: ParametricSurface3D,
{
    let (edge_front, edge_back) = match orientation {
        true => (edge_curve.front(), edge_curve.back()),
        false => (edge_curve.back(), edge_curve.front()),
    };
    if edge_front.near(&edge_back) {
        return;
    }
    if let (Some(front_uv), Some(back_uv)) = (boundary.first(), boundary.last()) {
        let front_point = surface.evaluate(front_uv.x, front_uv.y);
        let back_point = surface.evaluate(back_uv.x, back_uv.y);
        let direct = front_point.distance2(edge_front) + back_point.distance2(edge_back);
        let reversed = front_point.distance2(edge_back) + back_point.distance2(edge_front);
        if reversed < direct {
            boundary.reverse();
        }
    }
}

#[derive(Debug, Default, Clone)]
pub(in crate::tessellation::triangulation) struct PolyBoundaryPiece(
    pub(in crate::tessellation::triangulation) Vec<SurfacePoint>,
);

impl PolyBoundaryPiece {
    #[cfg(test)]
    pub(in crate::tessellation::triangulation) fn try_new_from_closed_axis_edge_samples<
        S: PreMeshableSurface,
    >(
        surface: &S,
        face_index: usize,
        loops: Vec<Vec<(usize, bool, PolylineCurve)>>,
        closed_axes: [bool; 2],
        sp: &impl SP<S>,
        tolerance: f64,
    ) -> Option<Vec<Self>> {
        Self::try_new_from_closed_axis_edge_samples_with_shared_samples(
            surface,
            face_index,
            loops,
            closed_axes,
            sp,
            tolerance,
            false,
        )
    }

    pub(in crate::tessellation::triangulation) fn try_new_from_closed_axis_edge_samples_with_shared_samples<
        S: PreMeshableSurface,
    >(
        surface: &S,
        face_index: usize,
        loops: Vec<Vec<(usize, bool, PolylineCurve)>>,
        closed_axes: [bool; 2],
        sp: &impl SP<S>,
        tolerance: f64,
        preserve_shared_samples: bool,
    ) -> Option<Vec<Self>> {
        let range = surface.try_range_tuple();
        let (Some(urange), Some(vrange)) = range else {
            return None;
        };
        if closed_axes.into_iter().filter(|closed| *closed).count() != 1
            || loops.len() != 1
            || loops.iter().any(Vec::is_empty)
            || !tolerance.is_finite()
            || tolerance <= 0.0
            || [urange, vrange]
                .into_iter()
                .any(|(min, max)| !min.is_finite() || !max.is_finite() || min >= max)
        {
            return None;
        }
        let bounds = [urange, vrange];
        let inside = |uv: Point2| {
            [uv.x, uv.y]
                .into_iter()
                .zip(bounds)
                .all(|(value, (min, max))| value >= min - 1.0e-8 && value <= max + 1.0e-8)
        };
        // shortcut: finite seam samples reject observed mismatches; certified curve equality is needed for a proof.
        let seam_matches = |axis: usize, from: Point2, to: Point2| {
            [0.0, 0.25, 0.5, 0.75, 1.0].into_iter().all(|fraction| {
                let mut low = from + (to - from) * fraction;
                let mut high = low;
                low[axis] = bounds[axis].0;
                high[axis] = bounds[axis].1;
                surface
                    .evaluate(low.x, low.y)
                    .distance(surface.evaluate(high.x, high.y))
                    <= tolerance.min(1.0e-7)
            })
        };
        let project_candidates =
            |point: Point3, previous: Option<Point2>, pinned_axis: Option<(usize, f64)>| {
                let mut hints = Vec::new();
                if let Some(previous) = previous {
                    hints.push(previous);
                }
                let us = [urange.0, (urange.0 + urange.1) * 0.5, urange.1];
                let vs = [vrange.0, (vrange.0 + vrange.1) * 0.5, vrange.1];
                for u in us {
                    for v in vs {
                        hints.push(Point2::new(u, v));
                    }
                }
                let mut candidates = Vec::<(f64, Point2)>::new();
                let mut in_domain = Vec::<(Point2, f64)>::new();
                for hint in hints {
                    if let Some((u, v)) = sp(surface, point, Some((hint.x, hint.y))) {
                        let uv = Point2::new(u, v);
                        if inside(uv) {
                            let residual = surface.evaluate(u, v).distance(point);
                            if in_domain
                                .iter()
                                .all(|(existing, _)| existing.distance(uv) > 1.0e-9)
                            {
                                in_domain.push((uv, residual));
                            }
                            if residual <= tolerance {
                                let mut uv = uv;
                                if let Some((axis, value)) = pinned_axis {
                                    let mut endpoint = uv;
                                    endpoint[axis] = value;
                                    // Inverse search may canonicalize equal finite boundaries;
                                    // retain the source occurrence's represented endpoint.
                                    if inside(endpoint)
                                        && surface.evaluate(endpoint.x, endpoint.y).distance(point)
                                            <= tolerance
                                    {
                                        uv = endpoint;
                                    }
                                }
                                if candidates
                                    .iter()
                                    .all(|(_, existing)| existing.distance(uv) > 1.0e-9)
                                {
                                    candidates.push((previous.map_or(0.0, |p| p.distance(uv)), uv));
                                }
                            }
                        }
                    }
                }
                candidates.retain(|(_, uv)| {
                    let Some(previous) = previous else {
                        return true;
                    };
                    [previous.x, previous.y]
                        .into_iter()
                        .zip([uv.x, uv.y])
                        .zip(bounds)
                        .enumerate()
                        .all(|(axis, ((from, to), (min, max)))| {
                            if (from - to).abs() <= (max - min) * 0.5 || !closed_axes[axis] {
                                // A bounded nonclosed axis may span its full finite chart
                                // interval across one short 3D edge, especially for a
                                // narrow U range. Both endpoints are already in-domain;
                                // nearest-UV ordering still selects among valid branches.
                                return true;
                            }
                            let midpoint = (min + max) * 0.5;
                            let crosses_chart_cut = (from <= midpoint && to >= midpoint)
                                || (to <= midpoint && from >= midpoint);
                            crosses_chart_cut && seam_matches(axis, previous, *uv)
                        })
                });
                if let Some((axis, value)) = pinned_axis {
                    candidates.retain(|(_, uv)| {
                        (uv[axis] - value).abs() <= 1.0e-7
                            && seam_matches(axis, previous.unwrap_or(*uv), *uv)
                    });
                }
                candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
                if env::var_os("MT_EDGE_CHART_REVERSE_CHOICES").is_some() {
                    candidates.reverse();
                }
                if env::var_os("MT_EDGE_CHART_TRACE").is_some() && face_index == 99 {
                    eprintln!(
                        "[EDGE-CHART-CANDIDATES] face={face_index} point={point:?} previous={previous:?} in_domain_uv_and_residual={in_domain:?} accepted_after_residual={:?}",
                        candidates
                            .iter()
                            .map(|(distance, uv)| (
                                *distance,
                                *uv,
                                surface.evaluate(uv.x, uv.y).distance(point)
                            ))
                            .collect::<Vec<_>>()
                    );
                }
                candidates.into_iter().map(|(_, uv)| uv).collect::<Vec<_>>()
            };

        let trace_all_face99_branches =
            env::var_os("MT_EDGE_CHART_TRACE").is_some() && face_index == 99;
        let mut result = Vec::with_capacity(loops.len());
        for edges in loops {
            let mut selected_ring: Option<Vec<SurfacePoint>> = None;
            let mut visited_states = 0usize;
            let mut repeated_uses = std::collections::BTreeMap::<usize, Vec<bool>>::new();
            for (edge_id, orientation, _) in &edges {
                repeated_uses
                    .entry(*edge_id)
                    .or_default()
                    .push(*orientation);
            }
            let repeated_opposite = repeated_uses
                .into_iter()
                .filter_map(|(id, uses)| (uses.len() == 2 && uses[0] != uses[1]).then_some(id))
                .collect::<std::collections::BTreeSet<_>>();
            if repeated_opposite.is_empty() {
                return None;
            }
            // For a repeated, oppositely oriented source edge, enumerate its two
            // finite seam assignments. Each occurrence stays on one chart boundary;
            // only a unique simple closed trim is accepted.
            for false_use_on_min_axis in [true, false] {
                for start in 0..edges.len() {
                    let ordered = (0..edges.len())
                        .map(|offset| &edges[(start + offset) % edges.len()])
                        .collect::<Vec<_>>();
                    let mut samples = Vec::<Point3>::new();
                    let mut sample_pins = Vec::<Option<(usize, f64)>>::new();
                    let mut valid_joins = true;
                    for (edge_index, (edge_id, orientation, edge)) in ordered.iter().enumerate() {
                        let Some(first) = edge.first().copied() else {
                            valid_joins = false;
                            break;
                        };
                        let pin = if repeated_opposite.contains(edge_id) {
                            let axis = closed_axes.iter().position(|closed| *closed)?;
                            let on_min = *orientation == false_use_on_min_axis;
                            let bounds = [urange, vrange];
                            Some((
                                axis,
                                if on_min {
                                    bounds[axis].0
                                } else {
                                    bounds[axis].1
                                },
                            ))
                        } else {
                            None
                        };
                        if edge_index == 0 {
                            samples.extend(edge.iter().copied());
                            sample_pins.extend(iter::repeat_n(pin, edge.len()));
                        } else if samples
                            .last()
                            .is_some_and(|previous| previous.distance(first) <= tolerance)
                        {
                            if let Some(incoming) = pin {
                                let previous = sample_pins.last_mut()?;
                                if previous.is_some_and(|existing| existing != incoming) {
                                    valid_joins = false;
                                    break;
                                }
                                *previous = Some(incoming);
                            }
                            samples.extend(edge.iter().copied().skip(1));
                            sample_pins.extend(iter::repeat_n(pin, edge.len().saturating_sub(1)));
                        } else {
                            valid_joins = false;
                            break;
                        }
                    }
                    if !valid_joins
                        || samples.len() < 4
                        || samples
                            .first()
                            .zip(samples.last())
                            .is_none_or(|(a, b)| a.distance(*b) > tolerance)
                    {
                        continue;
                    }

                    let first_pin = sample_pins[0];
                    let last_pin = *sample_pins.last()?;
                    if first_pin.is_some() && last_pin.is_some() && first_pin != last_pin {
                        continue;
                    }
                    let closing_pin = first_pin.or(last_pin);
                    sample_pins[0] = closing_pin;
                    *sample_pins.last_mut()? = closing_pin;

                    // Enumerate all bounded, residual-checked UV choices for this
                    // oriented source sample sequence. A branch-state cap is fail-closed.
                    let mut paths = vec![Vec::<Point2>::new()];
                    for (sample_index, point) in samples.iter().copied().enumerate() {
                        let mut next_paths = Vec::<Vec<Point2>>::new();
                        for path in &paths {
                            let previous = path.last().copied();
                            let candidates =
                                project_candidates(point, previous, sample_pins[sample_index]);
                            if trace_all_face99_branches {
                                eprintln!(
                                    "[EDGE-CHART-ENUMERATE] face={face_index} start={start} sample={sample_index}/{} previous={previous:?} in_domain_candidates={:?}",
                                    samples.len(),
                                    candidates
                                        .iter()
                                        .map(|uv| (
                                            *uv,
                                            surface.evaluate(uv.x, uv.y).distance(point)
                                        ))
                                        .collect::<Vec<_>>()
                                );
                            }
                            for uv in candidates {
                                let mut next = path.clone();
                                next.push(uv);
                                next_paths.push(next);
                                visited_states += 1;
                                if visited_states > 16_384 {
                                    if trace_all_face99_branches {
                                        eprintln!(
                                            "[EDGE-CHART-AMBIGUITY] face={face_index} branch state cap exceeded"
                                        );
                                    }
                                    return None;
                                }
                            }
                        }
                        paths = next_paths;
                        if paths.is_empty() {
                            break;
                        }
                    }

                    for path in paths {
                        if path.len() != samples.len()
                            || path
                                .first()
                                .zip(path.last())
                                .is_none_or(|(a, b)| a.distance(*b) > 1.0e-7)
                        {
                            continue;
                        }
                        let mut ring = samples
                            .iter()
                            .copied()
                            .zip(path)
                            .take(samples.len() - 1)
                            .map(|(point, uv)| SurfacePoint::from((uv, point)))
                            .collect::<Vec<_>>();
                        ring =
                            ring.into_iter()
                                .fold(Vec::<SurfacePoint>::new(), |mut acc, point| {
                                    if acc.last().is_none_or(|last| {
                                        (preserve_shared_samples && last.point != point.point)
                                            || last.uv.distance(point.uv) > 1.0e-10
                                    }) {
                                        acc.push(point);
                                    }
                                    acc
                                });
                        let uv_ring = ring.iter().map(|point| point.uv).collect::<Vec<_>>();
                        let simple = if preserve_shared_samples {
                            Self::uv_ring_is_simple_exact(&uv_ring)
                        } else {
                            Self::uv_path_is_simple(&uv_ring, true)
                        };
                        if !simple {
                            if trace_all_face99_branches {
                                eprintln!(
                                    "[EDGE-CHART-REJECT] face={face_index} start={start} reason=non_simple_ring"
                                );
                            }
                            continue;
                        }
                        let area2 = Self::uv_ring_signed_area2(&ring);
                        if area2 == 0.0 {
                            continue;
                        }
                        if trace_all_face99_branches {
                            eprintln!(
                                "[EDGE-CHART-CLOSED-BRANCH] face={face_index} start_edge={start} signed_uv_area={:.12} boundary_samples={} endpoint_residual_mm={:.12}",
                                area2 * 0.5,
                                ring.len(),
                                samples
                                    .first()
                                    .zip(samples.last())
                                    .map_or(f64::INFINITY, |(a, b)| a.distance(*b))
                            );
                        }
                        if let Some(existing) = &selected_ring {
                            if area2.signum() != Self::uv_ring_signed_area2(existing).signum()
                                || !Self::uv_rings_equivalent(
                                    surface,
                                    existing,
                                    &ring,
                                    closed_axes,
                                    [urange, vrange],
                                    tolerance,
                                )
                            {
                                if trace_all_face99_branches {
                                    eprintln!(
                                        "[EDGE-CHART-AMBIGUITY] face={face_index} distinct simple closed trims; fail closed"
                                    );
                                }
                                return None;
                            }
                        } else {
                            if trace_all_face99_branches {
                                eprintln!(
                                    "[EDGE-CHART-SELECTED-TRIM] face={face_index} start_edge={start} signed_uv_area={:.12} boundary_uvs={:?}",
                                    area2 * 0.5,
                                    ring.iter().map(|point| point.uv).collect::<Vec<_>>()
                                );
                            }
                            selected_ring = Some(ring);
                        }
                    }
                }
            }
            result.push(Self::from_surface_points_with_shared_samples(
                selected_ring?,
                preserve_shared_samples,
            )?);
        }
        Some(result)
    }

    fn uv_ring_signed_area2(points: &[SurfacePoint]) -> f64 {
        points
            .iter()
            .enumerate()
            .map(|(index, point)| {
                let next = points[(index + 1) % points.len()];
                point.x * next.y - next.x * point.y
            })
            .sum()
    }

    // UV nearness does not imply shared XYZ identity on a scaled surface.
    fn uv_ring_is_simple_exact(points: &[Point2]) -> bool {
        use num_rational::BigRational as Q;
        use num_traits::Zero;
        let Some(exact) = points
            .iter()
            .map(|p| Some([Q::from_float(p.x)?, Q::from_float(p.y)?]))
            .collect::<Option<Vec<_>>>()
        else {
            return false;
        };
        let n = points.len();
        if n < 3 {
            return false;
        }
        let orient = |a: usize, b: usize, c: usize| {
            let (a, b, c) = (&exact[a], &exact[b], &exact[c]);
            ((&b[0] - &a[0]) * (&c[1] - &a[1]) - (&b[1] - &a[1]) * (&c[0] - &a[0])).cmp(&Q::zero())
        };
        let on_segment = |a: usize, b: usize, c: usize| {
            let (a, b, c) = (points[a], points[b], points[c]);
            c.x >= a.x.min(b.x) && c.x <= a.x.max(b.x) && c.y >= a.y.min(b.y) && c.y <= a.y.max(b.y)
        };
        for a in 0..n {
            let b = (a + 1) % n;
            if points[a] == points[b] {
                return false;
            }
            // Adjacent collinear edges may continue, but may not retrace.
            let c = (b + 1) % n;
            if orient(a, b, c).is_eq() && (on_segment(a, b, c) || on_segment(b, c, a)) {
                return false;
            }
            for c in a + 1..n {
                let d = (c + 1) % n;
                if b == c || d == a {
                    continue;
                }
                let (pa, pb, pc, pd) = (points[a], points[b], points[c], points[d]);
                if pa.x.max(pb.x) < pc.x.min(pd.x)
                    || pc.x.max(pd.x) < pa.x.min(pb.x)
                    || pa.y.max(pb.y) < pc.y.min(pd.y)
                    || pc.y.max(pd.y) < pa.y.min(pb.y)
                {
                    continue;
                }
                let (o1, o2, o3, o4) = (
                    orient(a, b, c),
                    orient(a, b, d),
                    orient(c, d, a),
                    orient(c, d, b),
                );
                if (o1.is_eq() && on_segment(a, b, c))
                    || (o2.is_eq() && on_segment(a, b, d))
                    || (o3.is_eq() && on_segment(c, d, a))
                    || (o4.is_eq() && on_segment(c, d, b))
                    || (!o1.is_eq()
                        && !o2.is_eq()
                        && o1 != o2
                        && !o3.is_eq()
                        && !o4.is_eq()
                        && o3 != o4)
                {
                    return false;
                }
            }
        }
        true
    }

    fn uv_path_is_simple(points: &[Point2], closed: bool) -> bool {
        let count = points.len();
        if count < 3 {
            return false;
        }
        let edge_count = if closed { count } else { count - 1 };
        let orient =
            |a: Point2, b: Point2, c: Point2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
        let on_segment = |a: Point2, b: Point2, p: Point2| {
            p.x >= a.x.min(b.x) - 1.0e-14
                && p.x <= a.x.max(b.x) + 1.0e-14
                && p.y >= a.y.min(b.y) - 1.0e-14
                && p.y <= a.y.max(b.y) + 1.0e-14
        };
        let intersects = |a: Point2, b: Point2, c: Point2, d: Point2| {
            let (o1, o2, o3, o4) = (
                orient(a, b, c),
                orient(a, b, d),
                orient(c, d, a),
                orient(c, d, b),
            );
            (o1 > 0.0 && o2 < 0.0 || o1 < 0.0 && o2 > 0.0)
                && (o3 > 0.0 && o4 < 0.0 || o3 < 0.0 && o4 > 0.0)
                || o1.abs() <= 1.0e-14 && on_segment(a, b, c)
                || o2.abs() <= 1.0e-14 && on_segment(a, b, d)
                || o3.abs() <= 1.0e-14 && on_segment(c, d, a)
                || o4.abs() <= 1.0e-14 && on_segment(c, d, b)
        };
        for first in 0..edge_count {
            let first_next = (first + 1) % count;
            for second in first + 1..edge_count {
                let second_next = (second + 1) % count;
                if first_next == second || (closed && second_next == first) {
                    continue;
                }
                let a = points[first];
                let b = points[first_next];
                let c = points[second];
                let d = points[second_next];
                if a.distance(c) <= 1.0e-10
                    || a.distance(d) <= 1.0e-10
                    || b.distance(c) <= 1.0e-10
                    || b.distance(d) <= 1.0e-10
                    || intersects(a, b, c, d)
                {
                    return false;
                }
            }
        }
        true
    }

    fn uv_rings_equivalent<S: PreMeshableSurface>(
        surface: &S,
        lhs: &[SurfacePoint],
        rhs: &[SurfacePoint],
        closed_axes: [bool; 2],
        ranges: [(f64, f64); 2],
        tolerance: f64,
    ) -> bool {
        if lhs.len() != rhs.len() || lhs.is_empty() {
            return false;
        }
        (0..rhs.len()).any(|offset| {
            (0..lhs.len()).all(|index| {
                let a = lhs[index];
                let b = rhs[(index + offset) % rhs.len()];
                if a.point.distance(b.point) > tolerance {
                    return false;
                }
                if a.uv.distance(b.uv) <= 1.0e-7 {
                    return true;
                }
                (0..2).any(|axis| {
                    if !closed_axes[axis] || (a.uv[1 - axis] - b.uv[1 - axis]).abs() > 1.0e-7 {
                        return false;
                    }
                    let (min, max) = ranges[axis];
                    let opposite_ends = (a.uv[axis] - min).abs() <= 1.0e-7
                        && (b.uv[axis] - max).abs() <= 1.0e-7
                        || (a.uv[axis] - max).abs() <= 1.0e-7 && (b.uv[axis] - min).abs() <= 1.0e-7;
                    opposite_ends
                        && surface
                            .evaluate(a.uv.x, a.uv.y)
                            .distance(surface.evaluate(b.uv.x, b.uv.y))
                            <= 1.0e-7
                })
            })
        })
    }

    fn boundary_samples_match_surface<S: PreMeshableSurface>(
        surface: &S,
        boundary: &[Point2],
        points: &[Point3],
        tolerance: f64,
    ) -> bool {
        let mut min = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut max = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for point in points {
            min.x = min.x.min(point.x);
            min.y = min.y.min(point.y);
            min.z = min.z.min(point.z);
            max.x = max.x.max(point.x);
            max.y = max.y.max(point.y);
            max.z = max.z.max(point.z);
        }
        let extent = (max - min).magnitude();
        // A caller's chord can exceed the whole part, so its absolute value
        // cannot establish correspondence between independently sampled UV
        // and 3D arrays. Preserve the ordinary-size tolerance, but use a
        // relative bound when the part is smaller than the requested chord.
        let pairing_tolerance = if extent.is_finite() && extent < tolerance {
            tolerance.min((extent * 1.0e-6).max(f64::MIN_POSITIVE))
        } else {
            tolerance
        };
        boundary.len() == points.len()
            && boundary
                .iter()
                .zip(points)
                .all(|(uv, point)| surface.subs(uv.x, uv.y).distance(*point) <= pairing_tolerance)
    }

    fn rounded_uv_key(point: SurfacePoint) -> (u64, u64) {
        (
            spade_round(point.x).to_bits(),
            spade_round(point.y).to_bits(),
        )
    }

    pub(in crate::tessellation::triangulation) fn is_cdt_compatible(&self) -> bool {
        let len = self.0.len();
        let mut seen = HashMap::<(u64, u64), usize>::default();
        self.0.iter().copied().enumerate().all(|(index, point)| {
            let key = Self::rounded_uv_key(point);
            if let Some(previous) = seen.insert(key, index) {
                index == previous + 1 || (previous == 0 && index + 1 == len)
            } else {
                true
            }
        })
    }

    fn clamp_near_range(value: f64, (min, max): (f64, f64)) -> f64 {
        if value < min && min - value < TOLERANCE {
            min
        } else if value > max && value - max < TOLERANCE {
            max
        } else {
            value
        }
    }

    fn from_surface_points(vec: Vec<SurfacePoint>) -> Option<Self> {
        Self::from_surface_points_with_shared_samples(vec, false)
    }

    fn from_surface_points_with_shared_samples(
        mut vec: Vec<SurfacePoint>,
        preserve_shared_samples: bool,
    ) -> Option<Self> {
        let (mut uv_min, mut uv_max) = (
            Point2::new(f64::INFINITY, f64::INFINITY),
            Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        );
        for point in &vec {
            uv_min.x = uv_min.x.min(point.uv.x);
            uv_min.y = uv_min.y.min(point.uv.y);
            uv_max.x = uv_max.x.max(point.uv.x);
            uv_max.y = uv_max.y.max(point.uv.y);
        }
        let same_uv = |lhs: Point2, rhs: Point2| {
            let same_axis = |a: f64, b: f64, min: f64, max: f64| {
                let span = max - min;
                if span > 0.0 && span < 1.0e-6 {
                    a == b
                } else {
                    a.near(&b)
                }
            };
            same_axis(lhs.x, rhs.x, uv_min.x, uv_max.x)
                && same_axis(lhs.y, rhs.y, uv_min.y, uv_max.y)
        };
        // UV distance does not bound 3D distance on a scaled surface.
        // Compact a near-UV duplicate in angular mode only when its shared XYZ
        // is identical. Repeated endpoint projections may differ by a few ulps;
        // keeping both introduces near-coincident CDT closure constraints.
        let same_sample = |lhs: SurfacePoint, rhs: SurfacePoint| {
            (!preserve_shared_samples || lhs.point == rhs.point) && same_uv(lhs.uv, rhs.uv)
        };
        let mut retained = Vec::<SurfacePoint>::new();
        for point in vec.drain(..) {
            if retained
                .last()
                .is_none_or(|last| !same_sample(*last, point))
            {
                retained.push(point);
            }
        }
        let mut vec = retained;
        if vec.is_empty() {
            None
        } else {
            if !same_sample(vec[0], vec[vec.len() - 1]) {
                vec.push(vec[0]);
            }
            Some(Self(vec))
        }
    }

    fn from_parameter_boundary<S: PreMeshableSurface>(
        surface: &S,
        boundary: Vec<Point2>,
    ) -> Option<Self> {
        let mut previous = None;
        let vec = boundary
            .into_iter()
            .map(|uv| Self::normalize_uv(surface, (uv.x, uv.y), previous))
            .collect::<Option<Vec<_>>>()?
            .into_iter()
            .map(|(u, v)| {
                previous = Some((u, v));
                SurfacePoint::from((Point2::new(u, v), surface.evaluate(u, v)))
            })
            .collect();
        Self::from_surface_points(vec)
    }

    pub(in crate::tessellation::triangulation) fn try_new_from_trimmed<'a, S, T, C>(
        surface: &S,
        wire: impl Iterator<Item = (bool, Option<&'a T>, &'a C)>,
        tolerance: f64,
    ) -> Option<Self>
    where
        S: PreMeshableSurface,
        T: ExactTrimBoundary2D + 'a,
        C: ParametricCurve3D
            + BoundedCurve<Point = Point3>
            + ParameterBoundary2D<S>
            + ExactParameterBoundary2D<S>
            + 'a,
        <C as ExactParameterBoundary2D<S>>::BoundaryCurve: ExactTrimBoundary2D,
    {
        wire.map(|(orientation, trim_curve, edge_curve)| {
            let boundary = if let Some(trim_curve) = trim_curve {
                let mut boundary = trim_curve.exact_trim_boundary_2d(tolerance);
                orient_boundary_to_edge(surface, &mut boundary, edge_curve, orientation);
                boundary
            } else {
                let mut boundary = edge_curve
                    .exact_parameter_boundary_2d(surface)
                    .map(|trim_curve| trim_curve.exact_trim_boundary_2d(tolerance))
                    .or_else(|| edge_curve.parameter_boundary_2d(surface, tolerance))?;
                if !orientation {
                    boundary.reverse();
                }
                boundary
            };
            let boundary = simplify_parameter_boundary(surface, boundary, tolerance);
            Some(boundary)
        })
        .collect::<Option<Vec<Vec<Point2>>>>()
        .and_then(|boundaries| {
            let concatenated =
                boundaries
                    .into_iter()
                    .fold(Vec::<Point2>::new(), |mut acc, mut boundary| {
                        if !acc.is_empty() && !boundary.is_empty() {
                            boundary.remove(0);
                        }
                        acc.extend(boundary);
                        acc
                    });
            Self::from_parameter_boundary(surface, concatenated)
        })
    }

    pub(in crate::tessellation::triangulation) fn try_new_from_aligned_trimmed<'a, S, T, C>(
        surface: &S,
        wire: impl Iterator<Item = (Option<&'a T>, &'a C, PolylineCurve)>,
        sp: &impl SP<S>,
        tolerance: f64,
        preserve_shared_samples: bool,
    ) -> Option<Self>
    where
        S: PreMeshableSurface,
        T: ExactTrimBoundary2D + 'a,
        C: ParameterBoundary2D<S> + ExactParameterBoundary2D<S> + 'a,
        <C as ExactParameterBoundary2D<S>>::BoundaryCurve: ExactTrimBoundary2D,
    {
        wire.map(|(trim_curve, edge_curve, polyline)| {
            trim_curve
                .and_then(|trim_curve| {
                    let mut previous_t = None;
                    let mut previous_uv = None;
                    polyline
                        .iter()
                        .copied()
                        .map(|point| {
                            let (t, uv) = trim_curve.project_boundary_point(point, previous_t)?;
                            let (u, v) = Self::normalize_uv(surface, (uv.x, uv.y), previous_uv)?;
                            previous_t = Some(t);
                            previous_uv = Some((u, v));
                            Some(SurfacePoint::from((Point2::new(u, v), point)))
                        })
                        .collect::<Option<Vec<_>>>()
                })
                .or_else(|| {
                    let mut previous = None;
                    polyline
                        .iter()
                        .copied()
                        .map(|point| {
                            let (u, v) = sp(surface, point, previous)
                                .and_then(|uv| Self::normalize_uv(surface, uv, previous))?;
                            previous = Some((u, v));
                            Some(SurfacePoint::from((Point2::new(u, v), point)))
                        })
                        .collect::<Option<Vec<_>>>()
                })
                .or_else(|| {
                    if preserve_shared_samples {
                        return None;
                    }
                    edge_curve
                        .parameter_boundary_2d(surface, tolerance)
                        .map(|boundary| simplify_parameter_boundary(surface, boundary, tolerance))
                        .and_then(|boundary| Self::from_parameter_boundary(surface, boundary))
                        .map(|piece| piece.0)
                })
        })
        .collect::<Option<Vec<Vec<SurfacePoint>>>>()
        .and_then(|pieces| {
            let concatenated =
                pieces
                    .into_iter()
                    .fold(Vec::<SurfacePoint>::new(), |mut acc, mut piece| {
                        if !acc.is_empty() && !piece.is_empty() {
                            piece.remove(0);
                        }
                        acc.extend(piece);
                        acc
                    });
            Self::from_surface_points_with_shared_samples(concatenated, preserve_shared_samples)
        })
    }

    pub(in crate::tessellation::triangulation) fn try_new_from_exact<
        'a,
        S: PreMeshableSurface,
        C: ParameterBoundary2D<S> + 'a,
    >(
        surface: &S,
        wire: impl Iterator<Item = (bool, &'a C)>,
        tolerance: f64,
    ) -> Option<Self> {
        wire.map(|(orientation, curve)| {
            let mut boundary = curve.parameter_boundary_2d(surface, tolerance)?;
            if !orientation {
                boundary.reverse();
            }
            Some(boundary)
        })
        .collect::<Option<Vec<Vec<Point2>>>>()
        .and_then(|boundaries| {
            let concatenated =
                boundaries
                    .into_iter()
                    .fold(Vec::<Point2>::new(), |mut acc, mut boundary| {
                        if !acc.is_empty() && !boundary.is_empty() {
                            boundary.remove(0);
                        }
                        acc.extend(boundary);
                        acc
                    });
            Self::from_parameter_boundary(surface, concatenated)
        })
    }

    fn shift_uv_piece(
        piece: &[SurfacePoint],
        shift: Vector2,
        range: SurfaceParameterBounds,
        tolerance: f64,
        evaluate: &impl Fn(f64, f64) -> Point3,
    ) -> Option<Vec<SurfacePoint>> {
        let uv_tolerance = 1.0e-9;
        if shift.x == 0.0 && shift.y == 0.0 {
            return Some(piece.to_vec());
        }
        let shifted = piece
            .iter()
            .copied()
            .map(|mut point| {
                point.uv += shift;
                let inside =
                    [(point.x, range.0), (point.y, range.1)]
                        .into_iter()
                        .all(|(value, bounds)| {
                            bounds.is_none_or(|(min, max)| {
                                value >= min - uv_tolerance && value <= max + uv_tolerance
                            })
                        });
                if !inside || evaluate(point.x, point.y).distance(point.point) > tolerance {
                    None
                } else {
                    Some(point)
                }
            })
            .collect::<Option<Vec<_>>>()?;
        for window in piece.windows(2) {
            let midpoint = (window[0].uv + window[1].uv.to_vec()) * 0.5;
            let shifted_midpoint = midpoint + shift;
            let midpoint_inside = [(shifted_midpoint.x, range.0), (shifted_midpoint.y, range.1)]
                .into_iter()
                .all(|(value, bounds)| {
                    bounds.is_none_or(|(min, max)| {
                        value >= min - uv_tolerance && value <= max + uv_tolerance
                    })
                });
            if !midpoint_inside
                || evaluate(midpoint.x, midpoint.y)
                    .distance(evaluate(shifted_midpoint.x, shifted_midpoint.y))
                    > tolerance
            {
                return None;
            }
        }
        Some(shifted)
    }

    fn branch_candidates(
        piece: &[SurfacePoint],
        range: SurfaceParameterBounds,
        tolerance: f64,
        evaluate: &impl Fn(f64, f64) -> Point3,
    ) -> Vec<(Vector2, Vec<SurfacePoint>)> {
        let axis_shifts = |bounds: Option<(f64, f64)>| {
            bounds
                .map(|(min, max)| {
                    let span = max - min;
                    vec![0.0, span, -span]
                })
                .unwrap_or_else(|| vec![0.0])
        };
        let u_shifts = axis_shifts(range.0);
        let v_shifts = axis_shifts(range.1);
        let mut candidates = Vec::new();
        for du in &u_shifts {
            for dv in &v_shifts {
                let shift = Vector2::new(*du, *dv);
                if let Some(shifted) =
                    Self::shift_uv_piece(piece, shift, range, tolerance, evaluate)
                {
                    candidates.push((shift, shifted));
                }
            }
        }
        candidates
    }

    fn boundary_piece_uv_gap(
        lhs: &[SurfacePoint],
        rhs: &[SurfacePoint],
        tolerance: f64,
    ) -> Option<f64> {
        let end = lhs.last()?;
        let start = rhs.first()?;
        (end.point.distance(start.point) <= tolerance).then_some(end.uv.distance(start.uv))
    }

    /// Selects a UV branch for each edge piece only when a whole-range shift stays
    /// in the bounded surface domain and still evaluates to the paired 3D samples.
    /// Trying every cyclic start lets a closed wire establish its branch from a
    /// neighboring piece without asserting that the surface is periodic.
    fn reconcile_closed_piece_branches<S: PreMeshableSurface>(
        surface: &S,
        pieces: Vec<Vec<SurfacePoint>>,
        tolerance: f64,
    ) -> Option<(Vec<Vec<SurfacePoint>>, bool)> {
        if pieces.is_empty() || pieces.iter().any(Vec::is_empty) {
            return None;
        }
        if pieces.len() < 2 {
            return Some((pieces, false));
        }
        let uv_continuous = |pieces: &[Vec<SurfacePoint>]| {
            (0..pieces.len()).all(|i| {
                Self::boundary_piece_uv_gap(&pieces[i], &pieces[(i + 1) % pieces.len()], tolerance)
                    .is_some_and(|gap| gap <= 1.0e-7)
            })
        };
        if uv_continuous(&pieces) {
            return Some((pieces, false));
        }
        let range = surface.try_range_tuple();
        let full_range_closed_edge = pieces.iter().any(|piece| {
            let (Some(first), Some(last)) = (piece.first(), piece.last()) else {
                return false;
            };
            if first.point.distance(last.point) > tolerance {
                return false;
            }
            [(0usize, range.0, range.1), (1usize, range.1, range.0)]
                .into_iter()
                .any(|(axis, axis_range, _other_range)| {
                    let Some((min, max)) = axis_range else {
                        return false;
                    };
                    let width = max - min;
                    let endpoint_span = (last.uv[axis] - first.uv[axis]).abs();
                    width > 0.0 && (endpoint_span - width).abs() <= 1.0e-6 * width
                })
        });
        let duplicated_reverse_edge = (0..pieces.len()).any(|i| {
            (i + 1..pieces.len()).any(|j| {
                let lhs = &pieces[i];
                let rhs = &pieces[j];
                lhs.len() == rhs.len()
                    && lhs.len() >= 3
                    && lhs
                        .iter()
                        .zip(rhs.iter().rev())
                        .all(|(a, b)| a.point.distance(b.point) <= tolerance)
            })
        });
        if !full_range_closed_edge
            || !duplicated_reverse_edge
            || surface.period_u().is_some()
            || surface.period_v().is_some()
        {
            return Some((pieces, false));
        }
        let evaluate = |u, v| surface.evaluate(u, v);
        let validation_tolerance = tolerance.min(TOLERANCE);
        let mut best: Option<(f64, Vec<Vec<SurfacePoint>>)> = None;
        for start in 0..pieces.len() {
            for (start_shift, start_piece) in
                Self::branch_candidates(&pieces[start], range, validation_tolerance, &evaluate)
            {
                let mut selected = vec![start_piece];
                let mut shift_cost = start_shift.dot(start_shift);
                let mut valid = true;
                for offset in 1..pieces.len() {
                    let next_index = (start + offset) % pieces.len();
                    let previous = selected.last().unwrap();
                    let candidate = Self::branch_candidates(
                        &pieces[next_index],
                        range,
                        validation_tolerance,
                        &evaluate,
                    )
                    .into_iter()
                    .filter_map(|(shift, candidate)| {
                        let gap = Self::boundary_piece_uv_gap(previous, &candidate, tolerance)?;
                        (gap <= 1.0e-7).then_some((shift.dot(shift), candidate))
                    })
                    .min_by(|lhs, rhs| lhs.0.total_cmp(&rhs.0));
                    if let Some((cost, candidate)) = candidate {
                        shift_cost += cost;
                        selected.push(candidate);
                    } else {
                        valid = false;
                        break;
                    }
                }
                if valid
                    && Self::boundary_piece_uv_gap(
                        selected.last().unwrap(),
                        &selected[0],
                        tolerance,
                    )
                    .is_some_and(|gap| gap <= 1.0e-7)
                {
                    let ordered = (0..selected.len())
                        .map(|offset| {
                            selected[(pieces.len() - start + offset) % pieces.len()].clone()
                        })
                        .collect::<Vec<_>>();
                    if best.as_ref().is_none_or(|(cost, _)| shift_cost < *cost) {
                        best = Some((shift_cost, ordered));
                    }
                }
            }
        }
        best.map(|(_, pieces)| (pieces, true))
    }

    fn aligned_boundary_piece<S: PreMeshableSurface>(
        surface: &S,
        boundary: &[Point2],
        polyline: &PolylineCurve,
        previous: Option<SurfacePoint>,
        tolerance: f64,
    ) -> Option<(Vec<SurfacePoint>, Option<SurfacePoint>)> {
        let mut next_previous = previous;
        // Equal lengths do not by themselves establish that independently
        // sampled UV and 3D arrays are paired. Preserve the original samples
        // only when evaluating every UV at the surface reproduces its paired
        // 3D point within the caller's existing geometric tolerance.
        let paired_boundary =
            Self::boundary_samples_match_surface(surface, boundary, polyline, tolerance)
                .then(|| boundary.to_vec());
        let piece = paired_boundary
            .or_else(|| resample_boundary(boundary, polyline.len()))?
            .into_iter()
            .zip(polyline.iter().copied())
            .map(|(uv, point)| {
                let (u, v) = Self::normalize_uv(
                    surface,
                    (uv.x, uv.y),
                    next_previous.as_ref().map(|point| (point.x, point.y)),
                )?;
                let surface_point = SurfacePoint::from((Point2::new(u, v), point));
                next_previous = Some(surface_point);
                Some(surface_point)
            })
            .collect::<Option<Vec<_>>>()?;
        Some((piece, next_previous))
    }

    fn projected_polyline_piece<S: PreMeshableSurface>(
        surface: &S,
        polyline: &PolylineCurve,
        sp: &impl SP<S>,
        previous: Option<SurfacePoint>,
        preserve_shared_samples: bool,
    ) -> Option<(Vec<SurfacePoint>, Option<SurfacePoint>)> {
        let same_point = |a: Point3, b: Point3| {
            if preserve_shared_samples {
                a == b
            } else {
                a.near(&b)
            }
        };
        let mut next_previous = previous;
        let seed = next_previous.as_ref().and_then(|previous_point| {
            polyline
                .first()
                .copied()
                .filter(|point| same_point(*point, previous_point.point))
                .map(|point| {
                    SurfacePoint::from((Point2::new(previous_point.x, previous_point.y), point))
                })
        });
        let prefix = seed.into_iter().collect::<Vec<_>>();
        let suffix = polyline
            .iter()
            .copied()
            .skip(prefix.len())
            .map(|point| {
                let surface_point = if next_previous
                    .as_ref()
                    .is_some_and(|previous_point| same_point(point, previous_point.point))
                {
                    next_previous.as_ref().map(|previous_point| {
                        SurfacePoint::from((Point2::new(previous_point.x, previous_point.y), point))
                    })?
                } else {
                    let (u, v) = sp(
                        surface,
                        point,
                        next_previous.as_ref().map(|point| (point.x, point.y)),
                    )
                    .and_then(|uv| {
                        Self::normalize_uv(
                            surface,
                            uv,
                            next_previous.as_ref().map(|point| (point.x, point.y)),
                        )
                    })?;
                    SurfacePoint::from((Point2::new(u, v), point))
                };
                next_previous = Some(surface_point);
                Some(surface_point)
            })
            .collect::<Option<Vec<_>>>()?;
        Some((prefix.into_iter().chain(suffix).collect(), next_previous))
    }

    pub(in crate::tessellation::triangulation) fn try_new_from_aligned_exact<
        S: PreMeshableSurface,
    >(
        surface: &S,
        wire: impl Iterator<Item = (Option<Vec<Point2>>, PolylineCurve)>,
        sp: &impl SP<S>,
        tolerance: f64,
        preserve_shared_samples: bool,
    ) -> Option<Self> {
        let pieces = wire.collect::<Vec<_>>();
        if pieces.is_empty() {
            None
        } else {
            let piece_count = pieces.len();
            let start = pieces
                .iter()
                .position(|(boundary, _)| boundary.is_some())
                .unwrap_or(0);
            let mut previous = None::<SurfacePoint>;
            let periodic_surface = surface.period_u().is_some() || surface.period_v().is_some();
            pieces
                .into_iter()
                .cycle()
                .skip(start)
                .take(piece_count)
                .map(|(boundary, polyline)| {
                    let boundary_piece = || {
                        boundary.as_ref().and_then(|boundary| {
                            Self::aligned_boundary_piece(
                                surface, boundary, &polyline, previous, tolerance,
                            )
                            .filter(|(piece, _)| {
                                !preserve_shared_samples
                                    || piece.iter().all(|sample| {
                                        let distance = surface
                                            .evaluate(sample.x, sample.y)
                                            .distance(sample.point);
                                        distance.is_finite() && distance <= tolerance.min(TOLERANCE)
                                    })
                            })
                        })
                    };
                    let projected_piece = || {
                        Self::projected_polyline_piece(
                            surface,
                            &polyline,
                            sp,
                            previous,
                            preserve_shared_samples,
                        )
                    };
                    let piece = if periodic_surface {
                        projected_piece().or_else(boundary_piece)
                    } else {
                        boundary_piece().or_else(projected_piece)
                    };
                    piece
                        .map(|(piece, next_previous)| {
                            previous = next_previous;
                            piece
                        })
                        .or_else(|| {
                            if preserve_shared_samples {
                                return None;
                            }
                            boundary.and_then(|boundary| {
                                Self::from_parameter_boundary(surface, boundary).map(|piece| {
                                    previous = piece.0.last().copied();
                                    piece.0
                                })
                            })
                        })
                })
                .collect::<Option<Vec<Vec<SurfacePoint>>>>()
                .and_then(|pieces| {
                    Self::reconcile_closed_piece_branches(surface, pieces, tolerance)
                })
                .and_then(|(pieces, branch_adjusted)| {
                    let concatenated = pieces.into_iter().try_fold(
                        Vec::<SurfacePoint>::new(),
                        |mut acc, mut piece| {
                            if !acc.is_empty() && !piece.is_empty() {
                                let gap = acc.last().unwrap().uv.distance(piece[0].uv);
                                if branch_adjusted && gap > 1.0e-7 {
                                    return None;
                                }
                                piece.remove(0);
                            }
                            acc.extend(piece);
                            Some(acc)
                        },
                    )?;
                    Self::from_surface_points_with_shared_samples(
                        concatenated,
                        preserve_shared_samples,
                    )
                })
        }
    }

    pub(super) fn normalize_axis(
        value: f64,
        previous: Option<f64>,
        period: Option<f64>,
        range: Option<(f64, f64)>,
    ) -> Option<f64> {
        if !value.is_finite() {
            None
        } else if let Some(previous) = previous {
            if let Some(period) = period {
                Some(periodic_min_difference(value, previous, period))
            } else if let Some(range) = range {
                Some(Self::clamp_near_range(value, range))
            } else {
                Some(value)
            }
        } else if let Some((min, max)) = range {
            if let Some(period) = period {
                let span = max - min;
                if span.so_small() {
                    Some(min)
                } else {
                    let mut normalized = value - f64::floor((value - min) / period) * period;
                    if normalized < min {
                        normalized += period;
                    }
                    if normalized > max {
                        normalized -= period;
                    }
                    Some(normalized.clamp(min, max))
                }
            } else {
                Some(Self::clamp_near_range(value, (min, max)))
            }
        } else {
            Some(value)
        }
    }

    fn normalize_uv<S: PreMeshableSurface>(
        surface: &S,
        uv: (f64, f64),
        previous: Option<(f64, f64)>,
    ) -> Option<(f64, f64)> {
        let (urange, vrange) = surface.try_range_tuple();
        let u = Self::normalize_axis(uv.0, previous.map(|(u, _)| u), surface.period_u(), urange)?;
        let v = Self::normalize_axis(uv.1, previous.map(|(_, v)| v), surface.period_v(), vrange)?;
        Some((u, v))
    }

    fn parameter_seed_score<S: PreMeshableSurface>(surface: &S, (u, v): (f64, f64)) -> f64 {
        let uder = surface.derivative_u(u, v);
        let vder = surface.derivative_v(u, v);
        uder.dot(uder) + vder.dot(vder)
    }

    fn project_loop<S: PreMeshableSurface>(
        surface: &S,
        boundary: &[Point3],
        sp: &impl SP<S>,
        start: usize,
        initial_uv: Option<(f64, f64)>,
        tolerance: f64,
        attempt: &str,
    ) -> Option<Vec<SurfacePoint>> {
        let mut initial_uv = initial_uv;
        let mut previous_point = None;
        let collapsed = surface.finite_collapsed_boundaries();
        let ranges = surface.try_range_tuple();
        let certified_boundary = |axis: usize, parameter: f64| {
            let (Some(boundaries), (Some(urange), Some(vrange))) = (collapsed, ranges) else {
                return false;
            };
            let range = [urange, vrange][axis];
            (boundaries[axis][0] && parameter == range.0)
                || (boundaries[axis][1] && parameter == range.1)
        };
        let total = boundary.len();
        boundary
            .iter()
            .copied()
            .cycle()
            .skip(start)
            .take(boundary.len() + 1)
            .enumerate()
            .scan(None, |previous, (offset, pt)| {
                let previous_before = *previous;
                let raw = match finite_collapsed_boundary_footpoint(surface, pt, *previous) {
                    FiniteBoundaryProjection::Footpoint(uv) => Some(uv),
                    FiniteBoundaryProjection::Refused => None,
                    FiniteBoundaryProjection::Unproved => initial_uv
                    .take()
                    .or_else(|| sp(surface, pt, *previous))
                    .or_else(|| {
                        // Supported finite source carriers require a certified
                        // point boundary; sampled analytic guesses supply no proof.
                        if collapsed.is_some() { return None; }
                        // Revolve-pole fallback. At the revolution axis the surface
                        // is singular -- every v maps to the same 3D point -- so the
                        // parameter search (both exact and nearest) can fail to
                        // converge there and return `None`. The pole nonetheless
                        // coincides with a u-endpoint for EVERY v, so reuse the
                        // adjacent vertex's v and let the singular-bridge below
                        // carry the loop across the pole, instead of dropping the
                        // entire face's mesh (the corner-100 in-cube polar cap).
                        //
                        // Gate on v-invariance of the u-endpoint (the exact,
                        // scale-free signature of a revolution pole) so this fires
                        // only at a genuine axis point, never masking a real
                        // projection failure; the vertex only has to sit within a
                        // coarse mesh chord of that pole (a boundary polyline sample
                        // near the pole is a fraction off the exact axis point).
                        let (_, v_prev) = (*previous)?;
                        let (u0, u1) = surface.try_range_tuple().0?;
                        [u0, u1].into_iter().find_map(|u_end| {
                            let pole = surface.subs(u_end, v_prev);
                            let is_pole = pole.near(&surface.subs(u_end, v_prev + 1.0));
                            let at_pole = (pole - pt).magnitude2() < 1.0e-6;
                            (is_pole && at_pole).then_some((u_end, v_prev))
                        })
                    })
                    .or_else(|| {
                        let uv = verified_footpoint(surface, pt, (*previous)?, tolerance)?;
                        if projection_debug_enabled() {
                            eprintln!(
                                "BPROJ_RESCUE face={} attempt={attempt} vertex={}/{total} \
                                 uv=({:.9},{:.9}) residual={:.9} tolerance={tolerance:.9}",
                                projection_debug_face(),
                                if total == 0 {
                                    0
                                } else {
                                    (start + offset) % total
                                },
                                uv.0,
                                uv.1,
                                surface.subs(uv.0, uv.1).distance(pt),
                            );
                        }
                        Some(uv)
                    }),
                };
                let uv = raw
                    .and_then(|uv| Self::normalize_uv(surface, uv, *previous))
                    .and_then(|(u, v)| {
                        let current_uv = Point2::new(u, v);
                        let points = if let Some((u0, v0)) = *previous {
                            let collapsed_u = certified_boundary(0, u0);
                            let collapsed_v = certified_boundary(1, v0);
                            if collapsed_u || collapsed_v {
                                // Derivatives can both vanish at a higher-order
                                // pole. The proof, not derivative order, selects
                                // the fixed axis. Refuse an ambiguous corner.
                                if collapsed_u && collapsed_v && u != u0 && v != v0 {
                                    return None;
                                }
                                let bridge_uv = if collapsed_u {
                                    Point2::new(u0, v)
                                } else {
                                    Point2::new(u, v0)
                                };
                                let bridge_point = previous_point?;
                                if !surface.evaluate(bridge_uv.x, bridge_uv.y).near(&bridge_point) {
                                    return None;
                                }
                                let mut points = Vec::with_capacity(2);
                                if bridge_uv != Point2::new(u0, v0) && bridge_uv != current_uv {
                                    points.push(SurfacePoint::from((bridge_uv, bridge_point)));
                                }
                                points.push(SurfacePoint::from((current_uv, pt)));
                                points
                            } else if !u0.near(&u) && surface.derivative_u(u0, v0).so_small() {
                                if collapsed.is_some() { return None; }
                                vec![
                                    SurfacePoint::from((Point2::new(u, v0), pt)),
                                    SurfacePoint::from((current_uv, pt)),
                                ]
                            } else if !v0.near(&v) && surface.derivative_v(u0, v0).so_small() {
                                if collapsed.is_some() { return None; }
                                vec![
                                    SurfacePoint::from((Point2::new(u0, v), pt)),
                                    SurfacePoint::from((current_uv, pt)),
                                ]
                            } else {
                                vec![SurfacePoint::from((current_uv, pt))]
                            }
                        } else {
                            vec![SurfacePoint::from((current_uv, pt))]
                        };
                        *previous = Some((u, v));
                        previous_point = Some(pt);
                        Some(points)
                    });
                if uv.is_none() && projection_debug_enabled() {
                    report_projection_failure(
                        surface,
                        attempt,
                        start,
                        offset,
                        total,
                        pt,
                        previous_before,
                        raw,
                    );
                }
                Some(uv)
            })
            .collect::<Option<Vec<Vec<SurfacePoint>>>>()
            .map(|chunks| chunks.into_iter().flatten().collect())
    }

    pub(in crate::tessellation::triangulation) fn try_new<S: PreMeshableSurface>(
        surface: &S,
        wire: impl Iterator<Item = PolylineCurve>,
        sp: impl SP<S>,
        tolerance: f64,
    ) -> Option<Self> {
        let (urange, vrange) = surface.try_range_tuple();
        let (bdry3d, candidate_starts) = wire.fold(
            (Vec::<Point3>::new(), Vec::<usize>::new()),
            |(mut boundary, mut starts), poly_edge| {
                let edge_len = poly_edge.len().saturating_sub(1);
                if edge_len != 0 {
                    starts.push(boundary.len());
                    boundary.extend(poly_edge.into_iter().take(edge_len));
                }
                (boundary, starts)
            },
        );
        if bdry3d.is_empty() {
            return None;
        }
        let projected = Self::project_loop(surface, &bdry3d, &sp, 0, None, tolerance, "head")
            .or_else(|| {
                candidate_starts
                    .into_iter()
                    .filter(|start| *start != 0)
                    .filter_map(|start| {
                        let point = bdry3d[start];
                        let uv = sp(surface, point, None)
                            .and_then(|uv| Self::normalize_uv(surface, uv, None))?;
                        Some((Self::parameter_seed_score(surface, uv), start, uv))
                    })
                    .max_by(|lhs, rhs| lhs.0.total_cmp(&rhs.0))
                    .and_then(|(_, start, uv)| {
                        Self::project_loop(
                            surface,
                            &bdry3d,
                            &sp,
                            start,
                            Some(uv),
                            tolerance,
                            "edge-seed",
                        )
                    })
                    .or_else(|| {
                        bdry3d
                            .iter()
                            .copied()
                            .enumerate()
                            .filter(|(start, _)| *start != 0)
                            .filter_map(|(start, point)| {
                                let uv = sp(surface, point, None)
                                    .and_then(|uv| Self::normalize_uv(surface, uv, None))?;
                                Some((Self::parameter_seed_score(surface, uv), start, uv))
                            })
                            .max_by(|lhs, rhs| lhs.0.total_cmp(&rhs.0))
                            .and_then(|(_, start, uv)| {
                                Self::project_loop(
                                    surface,
                                    &bdry3d,
                                    &sp,
                                    start,
                                    Some(uv),
                                    tolerance,
                                    "brute-seed",
                                )
                            })
                    })
            });
        report_projection_outcome("ladder", projected.is_some());
        let mut vec = projected?;
        let grav = vec.iter().fold(Point2::origin(), |g, p| g + p.uv.to_vec()) / vec.len() as f64;
        if let (Some(up), Some((u0, _))) = (surface.period_u(), urange) {
            let quot = f64::floor((grav.x - u0) / up);
            vec.iter_mut().for_each(|p| p.x -= quot * up);
        }
        if let (Some(vp), Some((v0, _))) = (surface.period_v(), vrange) {
            let quot = f64::floor((grav.y - v0) / vp);
            vec.iter_mut().for_each(|p| p.y -= quot * vp);
        }
        // SAFETY: vec is non-empty because it was built from a non-empty boundary.
        let last = *vec.last().unwrap();
        if !vec[0].near(&last) {
            let Point2 { x: u0, y: v0 } = last.uv;
            if surface.derivative_u(u0, v0).so_small() || surface.derivative_v(u0, v0).so_small() {
                vec.push(vec[0]);
            }
        }
        Some(Self(vec))
    }
}

#[cfg(test)]
mod seam_branch_tests {
    use super::*;
    use monstertruck_geometry::prelude::Plane;
    use std::f64::consts::TAU;

    #[test]
    fn co_sample_validation_requires_equal_finite_points_within_caller_tolerance() {
        let plane = Plane::xy();
        let boundary = [0.0, 0.1, 0.4, 0.7, 1.0].map(|u| Point2::new(u, 0.0));
        let points = boundary.map(|uv| plane.evaluate(uv.x, uv.y));
        assert!(PolyBoundaryPiece::boundary_samples_match_surface(
            &plane, &boundary, &points, TOLERANCE
        ));

        let shifted = points.map(|point| point + Vector3::unit_z() * (2.0 * TOLERANCE));
        assert!(!PolyBoundaryPiece::boundary_samples_match_surface(
            &plane, &boundary, &shifted, TOLERANCE
        ));

        let nonfinite = [Point3::new(f64::NAN, 0.0, 0.0); 5];
        assert!(!PolyBoundaryPiece::boundary_samples_match_surface(
            &plane, &boundary, &nonfinite, TOLERANCE
        ));
    }

    #[derive(Clone)]
    struct Cylinder {
        twisted: bool,
        uv_scale: f64,
        model_scale: f64,
    }

    impl ParametricSurface for Cylinder {
        type Point = Point3;
        type Vector = Vector3;

        fn evaluate(&self, u: f64, v: f64) -> Point3 {
            let u = u / self.uv_scale;
            let v = v / self.uv_scale;
            Point3::new(
                u * self.model_scale,
                v.cos() * self.model_scale,
                (v.sin()
                    + if self.twisted {
                        u * (1.0 - u) * (v / TAU)
                    } else {
                        0.0
                    })
                    * self.model_scale,
            )
        }

        fn derivative_u(&self, _u: f64, _v: f64) -> Vector3 {
            Vector3::new(1.0, 0.0, 0.0)
        }
        fn derivative_v(&self, _u: f64, v: f64) -> Vector3 {
            Vector3::new(0.0, -v.sin(), v.cos())
        }
        fn derivative_uu(&self, _u: f64, _v: f64) -> Vector3 {
            Vector3::new(0.0, 0.0, 0.0)
        }
        fn derivative_uv(&self, _u: f64, _v: f64) -> Vector3 {
            Vector3::new(0.0, 0.0, 0.0)
        }
        fn derivative_vv(&self, _u: f64, v: f64) -> Vector3 {
            Vector3::new(0.0, -v.cos(), -v.sin())
        }
        fn derivative_mn(&self, _m: usize, _n: usize, _u: f64, _v: f64) -> Vector3 {
            Vector3::new(0.0, 0.0, 0.0)
        }

        fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
            use std::ops::Bound::Included;
            (
                (Included(0.0), Included(self.uv_scale)),
                (Included(0.0), Included(TAU * self.uv_scale)),
            )
        }
    }

    impl ParametricSurface3D for Cylinder {}

    #[derive(Clone)]
    struct UCylinder(Cylinder);

    impl ParametricSurface for UCylinder {
        type Point = Point3;
        type Vector = Vector3;

        fn evaluate(&self, u: f64, v: f64) -> Point3 {
            self.0.evaluate(v, u)
        }
        fn derivative_u(&self, u: f64, v: f64) -> Vector3 {
            self.0.derivative_v(v, u)
        }
        fn derivative_v(&self, u: f64, v: f64) -> Vector3 {
            self.0.derivative_u(v, u)
        }
        fn derivative_uu(&self, u: f64, v: f64) -> Vector3 {
            self.0.derivative_vv(v, u)
        }
        fn derivative_uv(&self, u: f64, v: f64) -> Vector3 {
            self.0.derivative_uv(v, u)
        }
        fn derivative_vv(&self, u: f64, v: f64) -> Vector3 {
            self.0.derivative_uu(v, u)
        }
        fn derivative_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Vector3 {
            self.0.derivative_mn(n, m, v, u)
        }

        fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
            let (u, v) = self.0.parameter_range();
            (v, u)
        }
    }

    impl ParametricSurface3D for UCylinder {}

    impl ParameterDivision2D for UCylinder {
        fn parameter_division(
            &self,
            _range: SurfaceParameterRange,
            _tolerance: f64,
        ) -> (Vec<f64>, Vec<f64>) {
            (vec![0.0, TAU], vec![0.0, 1.0])
        }
    }

    impl ParameterDivision2D for Cylinder {
        fn parameter_division(
            &self,
            _range: SurfaceParameterRange,
            _tolerance: f64,
        ) -> (Vec<f64>, Vec<f64>) {
            (vec![0.0, self.uv_scale], vec![0.0, TAU * self.uv_scale])
        }
    }

    fn boundary_piece(
        samples: &[(f64, f64)],
        evaluate: impl Fn(f64, f64) -> Point3,
    ) -> Vec<SurfacePoint> {
        samples
            .iter()
            .map(|&(u, v)| SurfacePoint::from((Point2::new(u, v), evaluate(u, v))))
            .collect()
    }

    #[test]
    fn accepts_a_whole_range_shift_only_when_all_paired_samples_match() {
        let cylinder = Cylinder {
            twisted: false,
            uv_scale: 1.0,
            model_scale: 1.0,
        };
        let piece = boundary_piece(&[(0.0, 0.0), (0.5, 0.0), (1.0, 0.0)], |u, v| {
            cylinder.evaluate(u, v)
        });
        let candidates = PolyBoundaryPiece::branch_candidates(
            &piece,
            (Some((0.0, 1.0)), Some((0.0, TAU))),
            TOLERANCE,
            &|u, v| cylinder.evaluate(u, v),
        );
        assert!(
            candidates
                .iter()
                .any(|(shift, _)| { shift.x == 0.0 && (shift.y - TAU).abs() <= 1.0e-12 })
        );
    }

    #[test]
    fn rejects_endpoint_coincidence_when_shifted_interior_is_not_on_surface() {
        let twisted = Cylinder {
            twisted: true,
            uv_scale: 1.0,
            model_scale: 1.0,
        };
        let piece = boundary_piece(&[(0.0, 0.0), (1.0, 0.0)], |u, v| twisted.evaluate(u, v));
        assert!(
            twisted
                .evaluate(0.0, 0.0)
                .distance(twisted.evaluate(0.0, TAU))
                <= TOLERANCE
        );
        assert!(
            twisted
                .evaluate(1.0, 0.0)
                .distance(twisted.evaluate(1.0, TAU))
                <= TOLERANCE
        );
        assert!(
            twisted
                .evaluate(0.5, 0.0)
                .distance(twisted.evaluate(0.5, TAU))
                > TOLERANCE
        );
        let candidates = PolyBoundaryPiece::branch_candidates(
            &piece,
            (Some((0.0, 1.0)), Some((0.0, TAU))),
            TOLERANCE,
            &|u, v| twisted.evaluate(u, v),
        );
        assert!(
            !candidates
                .iter()
                .any(|(shift, _)| { shift.x == 0.0 && (shift.y - TAU).abs() <= 1.0e-12 })
        );
    }

    #[test]
    fn reconciles_a_four_piece_nonperiodic_cylinder_loop_without_dropping_boundary_samples() {
        let surface = Cylinder {
            twisted: false,
            uv_scale: 1.0,
            model_scale: 1.0,
        };
        let evaluate = |u, v| surface.evaluate(u, v);
        let pieces = vec![
            boundary_piece(&[(1.0, 0.0), (0.5, 0.0), (0.0, 0.0)], evaluate),
            boundary_piece(&[(0.0, TAU), (0.0, TAU * 0.5), (0.0, 0.0)], evaluate),
            boundary_piece(&[(0.0, 0.0), (0.5, 0.0), (1.0, 0.0)], evaluate),
            boundary_piece(&[(1.0, 0.0), (1.0, TAU * 0.5), (1.0, TAU)], evaluate),
        ];

        let (reconciled, adjusted) =
            PolyBoundaryPiece::reconcile_closed_piece_branches(&surface, pieces, TOLERANCE)
                .expect("the in-domain periodic branch has matching surface points");

        assert!(adjusted);
        assert_eq!(reconciled.iter().map(Vec::len).sum::<usize>(), 12);
        assert!((0..reconciled.len()).all(|index| {
            let end = reconciled[index].last().unwrap();
            let start = reconciled[(index + 1) % reconciled.len()].first().unwrap();
            end.uv.distance(start.uv) <= 1.0e-7 && end.point.distance(start.point) <= TOLERANCE
        }));
    }

    #[test]
    fn aligned_exact_pipeline_preserves_the_full_four_piece_boundary() {
        let surface = Cylinder {
            twisted: false,
            uv_scale: 1.0,
            model_scale: 1.0,
        };
        let boundaries = [
            vec![(1.0, 0.0), (0.5, 0.0), (0.0, 0.0)],
            vec![(0.0, TAU), (0.0, TAU * 0.5), (0.0, 0.0)],
            vec![(0.0, 0.0), (0.5, 0.0), (1.0, 0.0)],
            vec![(1.0, 0.0), (1.0, TAU * 0.5), (1.0, TAU)],
        ];
        let wire = boundaries.into_iter().map(|boundary| {
            let uv = boundary
                .into_iter()
                .map(|(u, v)| Point2::new(u, v))
                .collect::<Vec<_>>();
            let points = uv
                .iter()
                .map(|point| surface.evaluate(point.x, point.y))
                .collect();
            (Some(uv), PolylineCurve(points))
        });
        let sp = |_surface: &Cylinder, _point: Point3, _hint: Option<(f64, f64)>| None;

        let boundary =
            PolyBoundaryPiece::try_new_from_aligned_exact(&surface, wire, &sp, TOLERANCE, false)
                .expect("all four aligned pieces form a branch-continuous closed boundary");

        assert_eq!(boundary.0.len(), 9);
        assert!((boundary.0.first().unwrap().uv.y - TAU).abs() <= 1.0e-7);
        assert!(
            boundary
                .0
                .first()
                .unwrap()
                .uv
                .near(&boundary.0.last().unwrap().uv)
        );
    }

    #[test]
    fn branch_validation_scales_with_parameterization_and_model_units() {
        let scale = 1_000.0;
        let v_period = TAU * scale;
        let cylinder = Cylinder {
            twisted: false,
            uv_scale: scale,
            model_scale: scale,
        };
        let cylinder_piece =
            boundary_piece(&[(0.0, 0.0), (0.5 * scale, 0.0), (scale, 0.0)], |u, v| {
                cylinder.evaluate(u, v)
            });
        let cylinder_candidates = PolyBoundaryPiece::branch_candidates(
            &cylinder_piece,
            (Some((0.0, scale)), Some((0.0, v_period))),
            TOLERANCE,
            &|u, v| cylinder.evaluate(u, v),
        );
        assert!(
            cylinder_candidates
                .iter()
                .any(|(shift, _)| { shift.x == 0.0 && (shift.y - v_period).abs() <= 1.0e-9 })
        );

        let twisted = Cylinder {
            twisted: true,
            uv_scale: scale,
            model_scale: scale,
        };
        let twisted_piece =
            boundary_piece(&[(0.0, 0.0), (scale, 0.0)], |u, v| twisted.evaluate(u, v));
        assert!(
            twisted
                .evaluate(0.0, 0.0)
                .distance(twisted.evaluate(0.0, v_period))
                <= TOLERANCE
        );
        assert!(
            twisted
                .evaluate(scale, 0.0)
                .distance(twisted.evaluate(scale, v_period))
                <= TOLERANCE
        );
        assert!(
            twisted
                .evaluate(scale * 0.5, 0.0)
                .distance(twisted.evaluate(scale * 0.5, v_period))
                > TOLERANCE
        );
        let twisted_candidates = PolyBoundaryPiece::branch_candidates(
            &twisted_piece,
            (Some((0.0, scale)), Some((0.0, v_period))),
            TOLERANCE,
            &|u, v| twisted.evaluate(u, v),
        );
        assert!(
            !twisted_candidates
                .iter()
                .any(|(shift, _)| { shift.x == 0.0 && (shift.y - v_period).abs() <= 1.0e-9 })
        );
    }

    #[test]
    fn co_sample_validation_does_not_treat_a_chord_larger_than_the_part_as_pairing_evidence() {
        let plane = Plane::xy();
        let boundary = [
            Point2::new(0.0, 0.0),
            Point2::new(1.0e-9, 0.0),
            Point2::new(1.0e-9, 1.0e-9),
        ];
        let points = boundary.map(|uv| plane.evaluate(uv.x, uv.y));
        assert!(PolyBoundaryPiece::boundary_samples_match_surface(
            &plane, &boundary, &points, 0.1
        ));

        let misplaced = points.map(|point| point + Vector3::unit_z() * 1.0e-9);
        assert!(!PolyBoundaryPiece::boundary_samples_match_surface(
            &plane, &boundary, &misplaced, 0.1,
        ));
    }

    #[test]
    fn tiny_parameter_dedup_preserves_close_distinct_vertices() {
        let point = |x| SurfacePoint::from((Point2::new(x, 0.0), Point3::new(x, 0.0, 0.0)));
        let first = 0.0;
        let second = 5.0e-16;
        let third = 1.0e-9;
        let boundary =
            PolyBoundaryPiece::from_surface_points(vec![point(first), point(second), point(third)])
                .expect("nonempty boundary");

        // Includes all three distinct input vertices plus the appended close.
        assert_eq!(boundary.0.len(), 4);
        assert_eq!(boundary.0[0].uv.x, first);
        assert_eq!(boundary.0[1].uv.x, second);
        assert_eq!(boundary.0[2].uv.x, third);
        assert_eq!(boundary.0[3].uv, boundary.0[0].uv);
    }

    #[test]
    fn tiny_parameter_dedup_still_compacts_exact_repeats() {
        let point = |x| SurfacePoint::from((Point2::new(x, 0.0), Point3::new(x, 0.0, 0.0)));
        let boundary = PolyBoundaryPiece::from_surface_points(vec![
            point(0.0),
            point(0.0),
            point(1.0e-9),
            point(0.0),
        ])
        .expect("nonempty boundary");

        assert_eq!(boundary.0.len(), 3);
        assert_eq!(boundary.0[0].uv.x, 0.0);
        assert_eq!(boundary.0[1].uv.x, 1.0e-9);
        assert_eq!(boundary.0[2].uv, boundary.0[0].uv);
    }

    #[test]
    fn identity_repeated_edge_uses_declared_closed_u_chart() {
        let surface = UCylinder(Cylinder {
            twisted: false,
            uv_scale: 1.0,
            model_scale: 1.0,
        });
        let piece =
            |edge, orientation, points: Vec<Point3>| (edge, orientation, PolylineCurve(points));
        let seam = (0..=8)
            .map(|i| surface.evaluate(0.0, i as f64 / 8.0))
            .collect::<Vec<_>>();
        let far = (0..=8)
            .map(|i| surface.evaluate(i as f64 * TAU / 8.0, 1.0))
            .collect::<Vec<_>>();
        let near = (0..=8)
            .rev()
            .map(|i| surface.evaluate(i as f64 * TAU / 8.0, 0.0))
            .collect::<Vec<_>>();
        let result = PolyBoundaryPiece::try_new_from_closed_axis_edge_samples(
            &surface,
            3,
            vec![vec![
                piece(5, true, seam.clone()),
                piece(6, true, far),
                piece(5, false, seam.iter().copied().rev().collect()),
                piece(7, true, near),
            ]],
            [true, false],
            &|_: &UCylinder, point: Point3, hint: Option<(f64, f64)>| {
                let mut u = point.z.atan2(point.y);
                if u < 0.0 {
                    u += TAU;
                }
                if let Some((prev, _)) = hint {
                    if u <= 1e-12 && prev > TAU * 0.5 {
                        u = TAU;
                    }
                }
                Some((u, point.x))
            },
            TOLERANCE,
        )
        .expect(
            "source-declared U closure and repeated opposite edge should form a finite-chart trim",
        )
        .pop()
        .unwrap();
        assert!(
            result.0.iter().all(|p| p.x >= -1e-10
                && p.x <= TAU + 1e-10
                && p.y >= -1e-10
                && p.y <= 1.0 + 1e-10)
        );
        assert!(
            result
                .0
                .iter()
                .all(|p| surface.evaluate(p.x, p.y).distance(p.point) <= TOLERANCE)
        );
    }

    fn closed_chart_test_input(
        surface: &UCylinder,
        seam_steps: usize,
    ) -> Vec<Vec<(usize, bool, PolylineCurve)>> {
        let seam = (0..=seam_steps)
            .map(|i| surface.evaluate(0.0, i as f64 / seam_steps as f64))
            .collect::<Vec<_>>();
        vec![vec![
            (5, true, PolylineCurve(seam.clone())),
            (
                6,
                true,
                PolylineCurve(
                    (0..=8)
                        .map(|i| surface.evaluate(i as f64 * TAU / 8.0, 1.0))
                        .collect(),
                ),
            ),
            (5, false, PolylineCurve(seam.into_iter().rev().collect())),
            (
                7,
                true,
                PolylineCurve(
                    (0..=8)
                        .rev()
                        .map(|i| surface.evaluate(i as f64 * TAU / 8.0, 0.0))
                        .collect(),
                ),
            ),
        ]]
    }

    fn closed_chart_test_search(
        _: &UCylinder,
        point: Point3,
        hint: Option<(f64, f64)>,
    ) -> Option<(f64, f64)> {
        let mut u = point.z.atan2(point.y);
        if u < 0.0 {
            u += TAU;
        }
        if u.abs() < 1e-12 && hint.is_some_and(|(u, _)| u > TAU * 0.5) {
            u = TAU;
        }
        Some((u, point.x))
    }

    #[derive(Clone)]
    struct ClosedNonperiodic(UCylinder);

    impl ParametricSurface for ClosedNonperiodic {
        type Point = Point3;
        type Vector = Vector3;
        fn evaluate(&self, u: f64, v: f64) -> Point3 {
            self.0.evaluate(u, v) + Vector3::unit_z() * (0.01 * u * (u - TAU) * v * (1.0 - v))
        }
        fn derivative_u(&self, u: f64, v: f64) -> Vector3 {
            self.0.derivative_u(u, v) + Vector3::unit_z() * (0.01 * (2.0 * u - TAU) * v * (1.0 - v))
        }
        fn derivative_v(&self, u: f64, v: f64) -> Vector3 {
            self.0.derivative_v(u, v) + Vector3::unit_z() * (0.01 * u * (u - TAU) * (1.0 - 2.0 * v))
        }
        fn derivative_uu(&self, u: f64, v: f64) -> Vector3 {
            self.0.derivative_uu(u, v) + Vector3::unit_z() * (0.02 * v * (1.0 - v))
        }
        fn derivative_uv(&self, u: f64, v: f64) -> Vector3 {
            self.0.derivative_uv(u, v)
                + Vector3::unit_z() * (0.01 * (2.0 * u - TAU) * (1.0 - 2.0 * v))
        }
        fn derivative_vv(&self, u: f64, v: f64) -> Vector3 {
            self.0.derivative_vv(u, v) - Vector3::unit_z() * (0.02 * u * (u - TAU))
        }
        fn derivative_mn(&self, _: usize, _: usize, _: f64, _: f64) -> Vector3 {
            panic!("test search does not use higher derivatives")
        }
        fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
            self.0.parameter_range()
        }
    }
    impl ParametricSurface3D for ClosedNonperiodic {}
    impl ParameterDivision2D for ClosedNonperiodic {
        fn parameter_division(&self, _: SurfaceParameterRange, _: f64) -> (Vec<f64>, Vec<f64>) {
            panic!("projection does not divide the surface")
        }
    }

    #[test]
    fn exact_closed_ring_accepts_narrow_segments_and_rejects_crossings() {
        let ring = |values: &[(f64, f64)]| {
            values
                .iter()
                .map(|&(x, y)| Point2::new(x, y))
                .collect::<Vec<_>>()
        };
        for scale in [1.0e-12, 1.0, 1.0e12] {
            let points = ring(&[(0., 0.), (1., 0.), (1., 5e-11), (1., 1.), (0., 1.)]);
            let mut scaled = points
                .iter()
                .map(|p| Point2::new(p.x * scale, p.y * scale))
                .collect::<Vec<_>>();
            assert!(PolyBoundaryPiece::uv_ring_is_simple_exact(&scaled));
            scaled.reverse();
            assert!(PolyBoundaryPiece::uv_ring_is_simple_exact(&scaled));
        }
        for bad in [
            ring(&[(0., 0.), (1., 1.), (0., 1.), (1., 0.)]),
            ring(&[(0., 0.), (1., 0.), (0.5, 0.), (1., 1.), (0., 1.)]),
            ring(&[(0., 0.), (1., 0.), (1., 1.), (0.5, 0.), (0., 1.)]),
            ring(&[(0., 0.), (1., 0.), (1., 0.), (0., 1.)]),
            ring(&[(0., 0.), (f64::NAN, 0.), (0., 1.)]),
        ] {
            assert!(!PolyBoundaryPiece::uv_ring_is_simple_exact(&bad));
        }
    }

    #[derive(Clone)]
    struct ReviewerAxiallyScaled(UCylinder);
    impl ParametricSurface for ReviewerAxiallyScaled {
        type Point = Point3;
        type Vector = Vector3;
        fn evaluate(&self, u: f64, v: f64) -> Point3 {
            let mut point = self.0.evaluate(u, v);
            point.x *= 1.0e10;
            point
        }
        fn derivative_u(&self, _: f64, _: f64) -> Vector3 {
            panic!("unused by analytic search")
        }
        fn derivative_v(&self, _: f64, _: f64) -> Vector3 {
            panic!("unused by analytic search")
        }
        fn derivative_uu(&self, _: f64, _: f64) -> Vector3 {
            panic!("unused by analytic search")
        }
        fn derivative_uv(&self, _: f64, _: f64) -> Vector3 {
            panic!("unused by analytic search")
        }
        fn derivative_vv(&self, _: f64, _: f64) -> Vector3 {
            panic!("unused by analytic search")
        }
        fn derivative_mn(&self, _: usize, _: usize, _: f64, _: f64) -> Vector3 {
            panic!("unused by analytic search")
        }
        fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
            self.0.parameter_range()
        }
    }
    impl ParametricSurface3D for ReviewerAxiallyScaled {}
    impl ParameterDivision2D for ReviewerAxiallyScaled {
        fn parameter_division(&self, _: SurfaceParameterRange, _: f64) -> (Vec<f64>, Vec<f64>) {
            panic!("helper does not divide surface")
        }
    }

    #[test]
    fn reviewer_combined_closed_chart_keeps_each_distinct_shared_seam_sample() {
        let base = UCylinder(Cylinder {
            twisted: false,
            uv_scale: 1.0,
            model_scale: 1.0,
        });
        let surface = ReviewerAxiallyScaled(base.clone());
        let sample = surface.evaluate(0.0, 5.0e-11);
        let mut input = closed_chart_test_input(&base, 8);
        for (_, _, edge) in &mut input[0] {
            for point in &mut edge.0 {
                point.x *= 1.0e10;
            }
        }
        input[0][0].2.0.insert(1, sample);
        let reversed_index = input[0][2].2.len() - 1;
        input[0][2].2.0.insert(reversed_index, sample);
        let expected_uses = input[0]
            .iter()
            .flat_map(|(_, _, edge)| edge.iter())
            .filter(|point| **point == sample)
            .count();
        assert_eq!(expected_uses, 2);
        assert_eq!(sample.distance(surface.evaluate(0.0, 0.0)), 0.5);
        let result = PolyBoundaryPiece::try_new_from_closed_axis_edge_samples_with_shared_samples(
            &surface,
            3,
            input,
            [true, false],
            &|_: &ReviewerAxiallyScaled, point: Point3, hint: Option<(f64, f64)>| {
                let unscaled = Point3::new(point.x / 1.0e10, point.y, point.z);
                closed_chart_test_search(&base, unscaled, hint)
            },
            TOLERANCE,
            true,
        )
        .expect("the current combined helper accepts this finite closed chart");
        let actual_uses = result[0]
            .0
            .iter()
            .filter(|point| point.point == sample)
            .count();
        eprintln!(
            "shared seam sample={sample:?}, UV gap=5e-11, XYZ gap=0.5, expected_uses={expected_uses}, actual_uses={actual_uses}"
        );
        assert_eq!(
            actual_uses, expected_uses,
            "angular closed chart silently loses a distinct shared source sample use"
        );
    }

    fn reviewer_rotation_source_boundary() -> (
        monstertruck_geometry::prelude::NurbsSurface<Vector4>,
        Vec<Vec<(usize, bool, PolylineCurve)>>,
    ) {
        use monstertruck_geometry::prelude::{BsplineSurface, KnotVector, NurbsSurface};
        // Reduced source #16287/#16333 data, with the actual angular-0.5 shared samples.
        let points = [
            [2.704860298, 0.185803475, 18.805543338],
            [2.704860298, -14.676706185, 18.805543338],
            [2.704860298, -14.676706185, 3.943033678],
            [2.704860298, 0.185803475, 3.943033678],
            [2.704860298, 15.048313135, 3.943033678],
            [2.704860298, 15.048313135, 18.805543338],
            [2.704860298, 0.185803475, 18.805543338],
            [0.901980804, 0.185803475, 19.541472832],
            [0.901980804, -16.148565174, 19.541472832],
            [0.901980804, -16.148565174, 3.207104183],
            [0.901980804, 0.185803475, 3.207104183],
            [0.901980804, 16.520172124, 3.207104183],
            [0.901980804, 16.520172124, 19.541472832],
            [0.901980804, 0.185803475, 19.541472832],
            [-0.90089869, 0.185803475, 19.325144181],
            [-0.90089869, -15.715907872, 19.325144181],
            [-0.90089869, -15.715907872, 3.423432834],
            [-0.90089869, 0.185803475, 3.423432834],
            [-0.90089869, 16.087514822, 3.423432834],
            [-0.90089869, 16.087514822, 19.325144181],
            [-0.90089869, 0.185803475, 19.325144181],
        ];
        let weights = [
            1.0,
            0.333333333,
            0.333333333,
            1.0,
            0.333333333,
            0.333333333,
            1.0,
        ];
        let controls = points
            .chunks_exact(7)
            .map(|row| {
                row.iter()
                    .zip(weights)
                    .map(|(p, w)| Vector4::new(p[0] * w, p[1] * w, p[2] * w, w))
                    .collect()
            })
            .collect();
        let surface = NurbsSurface::new(BsplineSurface::new(
            (
                KnotVector::from(vec![
                    0.064590166,
                    0.064590166,
                    0.064590166,
                    0.627990008,
                    0.627990008,
                    0.627990008,
                ]),
                KnotVector::from(vec![0., 0., 0., 0., 0.5, 0.5, 0.5, 1., 1., 1., 1.]),
            ),
            controls,
        ));
        let edges = vec![
            (
                988,
                false,
                PolylineCurve(vec![
                    Point3::new(-0.900898691, 0.185803475, 19.325144181),
                    Point3::new(0.901980816, 0.185803475, 19.303408294249998),
                    Point3::new(2.704860323, 0.185803475, 18.805543328),
                ]),
            ),
            (
                989,
                true,
                PolylineCurve(vec![
                    Point3::new(2.704860298, 0.18580347500000136, 18.805543338),
                    Point3::new(2.704860298, -2.1105805570009095, 18.441831838321335),
                    Point3::new(2.704860298, -4.182178520101208, 17.38629995500091),
                    Point3::new(2.704860298, -5.82620797200091, 15.74227050310121),
                    Point3::new(2.704860298, -6.881739855321333, 13.670672540000911),
                    Point3::new(2.704860298, -7.245451355, 11.374288508000001),
                ]),
            ),
            (
                990,
                true,
                PolylineCurve(vec![
                    Point3::new(2.704860298, -7.245451355, 11.374288508000001),
                    Point3::new(2.704860298, -6.881739855321334, 9.077904475999091),
                    Point3::new(2.704860298, -5.8262079720009075, 7.006306512898786),
                    Point3::new(2.704860298, -4.182178520101211, 5.36227706099909),
                    Point3::new(2.704860298, -2.110580557000913, 4.306745177678666),
                    Point3::new(2.704860298, 0.18580347499999772, 3.943033677999999),
                    Point3::new(2.704860298, 2.482187507000909, 4.306745177678665),
                    Point3::new(2.704860298, 4.553785470101208, 5.362277060999086),
                    Point3::new(2.704860298, 6.1978149220009096, 7.006306512898788),
                    Point3::new(2.704860298, 7.253346805321333, 9.077904475999086),
                    Point3::new(2.704860298, 7.6170583050000005, 11.374288507999996),
                    Point3::new(2.704860298, 7.253346805321335, 13.670672540000908),
                    Point3::new(2.704860298, 6.197814922000913, 15.742270503101206),
                    Point3::new(2.704860298, 4.553785470101213, 17.386299955000908),
                    Point3::new(2.704860298, 2.4821875070009143, 18.44183183832133),
                    Point3::new(2.704860298, 0.1858034750000032, 18.805543338),
                ]),
            ),
            (
                988,
                true,
                PolylineCurve(vec![
                    Point3::new(2.704860323, 0.185803475, 18.805543328),
                    Point3::new(0.901980816, 0.185803475, 19.303408294249998),
                    Point3::new(-0.900898691, 0.185803475, 19.325144181),
                ]),
            ),
            (
                991,
                true,
                PolylineCurve(vec![
                    Point3::new(-0.90089869, 0.1858034749999995, 19.325144181),
                    Point3::new(-0.90089869, 2.6427529977794593, 18.93600160592894),
                    Point3::new(-0.90089869, 4.859199182695346, 17.80666586727946),
                    Point3::new(-0.90089869, 6.6181808342794595, 16.047684215695348),
                    Point3::new(-0.90089869, 7.747516572928939, 13.83123803077946),
                    Point3::new(-0.90089869, 8.136659148, 11.374288508000001),
                    Point3::new(-0.90089869, 7.74751657292894, 8.91733898522054),
                    Point3::new(-0.90089869, 6.618180834279461, 6.700892800304653),
                    Point3::new(-0.90089869, 4.859199182695348, 4.94191114872054),
                    Point3::new(-0.90089869, 2.642752997779461, 3.8125754100710605),
                    Point3::new(-0.90089869, 0.18580347500000147, 3.423432834999999),
                    Point3::new(-0.90089869, -2.2711460477794514, 3.812575410071058),
                    Point3::new(-0.90089869, -4.487592232695345, 4.941911148720538),
                    Point3::new(-0.90089869, -6.246573884279463, 6.700892800304657),
                    Point3::new(-0.90089869, -7.375909622928939, 8.917338985220539),
                    Point3::new(-0.90089869, -7.765052198, 11.374288507999998),
                ]),
            ),
            (
                992,
                true,
                PolylineCurve(vec![
                    Point3::new(-0.90089869, -7.765052198, 11.374288507999998),
                    Point3::new(-0.90089869, -7.37590962292894, 13.831238030779456),
                    Point3::new(-0.90089869, -6.246573884279461, 16.047684215695345),
                    Point3::new(-0.90089869, -4.487592232695349, 17.806665867279456),
                    Point3::new(-0.90089869, -2.271146047779462, 18.936001605928936),
                    Point3::new(-0.90089869, 0.18580347499999755, 19.325144181),
                ]),
            ),
        ];
        (surface, vec![edges])
    }

    #[test]
    fn reviewer_rotation_source_seam_joins_preserve_one_trim_across_starts() {
        let (surface, source) = reviewer_rotation_source_boundary();
        let search = search_nearest_parameter_sp::<
            monstertruck_geometry::prelude::NurbsSurface<Vector4>,
        >(100);
        for start in 0..6 {
            let mut input = source.clone();
            input[0].rotate_left(start);
            let result = PolyBoundaryPiece::try_new_from_closed_axis_edge_samples_with_shared_samples(
                &surface, 3, input, [false, true], &search, 0.1, true,
            ).expect("incoming and cyclic seam pins must reject the false chart branch, not the source face");
            let area2 = PolyBoundaryPiece::uv_ring_signed_area2(&result[0].0);
            assert!(
                (area2 + 1.126799684065811).abs() < 1e-7,
                "start={start}, area2={area2}"
            );
            let seam_midpoint = source[0][0].2[1];
            assert_eq!(
                result[0]
                    .0
                    .iter()
                    .filter(|p| p.point == seam_midpoint)
                    .count(),
                2
            );
        }
    }

    #[test]
    fn reviewer_closed_chart_conflicting_adjacent_seam_pins_refuse() {
        let surface = UCylinder(Cylinder {
            twisted: false,
            uv_scale: 1.0,
            model_scale: 1.0,
        });
        let mut input = closed_chart_test_input(&surface, 2);
        let seam = input[0][0].clone();
        let reverse = input[0][2].clone();
        input[0] = vec![seam, reverse];
        assert!(
            PolyBoundaryPiece::try_new_from_closed_axis_edge_samples_with_shared_samples(
                &surface,
                3,
                input,
                [true, false],
                &closed_chart_test_search,
                TOLERANCE,
                true,
            )
            .is_none(),
            "opposite finite chart boundaries cannot share one projected join sample"
        );
    }

    #[test]
    fn closed_chart_does_not_require_or_invent_a_period() {
        let base = UCylinder(Cylinder {
            twisted: false,
            uv_scale: 1.0,
            model_scale: 1.0,
        });
        let surface = ClosedNonperiodic(base.clone());
        assert_eq!(surface.period_u(), None);
        assert_eq!(surface.period_v(), None);
        assert!(
            surface
                .evaluate(0.5, 0.5)
                .distance(surface.evaluate(0.5 + TAU, 0.5))
                > 0.01
        );
        assert!(
            surface
                .derivative_u(0.0, 0.5)
                .distance(surface.derivative_u(TAU, 0.5))
                > 0.01
        );
        let result = PolyBoundaryPiece::try_new_from_closed_axis_edge_samples(
            &surface,
            3,
            closed_chart_test_input(&base, 8),
            [true, false],
            &|_: &ClosedNonperiodic, point, hint| closed_chart_test_search(&base, point, hint),
            TOLERANCE,
        )
        .expect("matching finite chart boundaries do not require a periodic evaluator");
        assert!(
            result[0]
                .0
                .iter()
                .all(|p| p.x >= 0.0 && p.x <= TAU && p.y >= 0.0 && p.y <= 1.0)
        );
        assert!(
            result[0]
                .0
                .iter()
                .all(|p| surface.evaluate(p.x, p.y).distance(p.point) <= TOLERANCE)
        );
        assert_eq!(surface.period_u(), None);
    }

    #[test]
    fn closed_chart_defers_multiple_wires() {
        let surface = UCylinder(Cylinder {
            twisted: false,
            uv_scale: 1.0,
            model_scale: 1.0,
        });
        let mut input = closed_chart_test_input(&surface, 8);
        input.push(input[0].clone());
        assert!(
            PolyBoundaryPiece::try_new_from_closed_axis_edge_samples(
                &surface,
                3,
                input,
                [true, false],
                &closed_chart_test_search,
                TOLERANCE,
            )
            .is_none()
        );
    }

    #[test]
    fn closed_chart_rejects_mismatched_seam_interior_between_source_samples() {
        let surface = UCylinder(Cylinder {
            twisted: true,
            uv_scale: 1.0,
            model_scale: 1.0,
        });
        // Only the end vertices agree; the upper seam bows away between them.
        assert!(
            surface
                .evaluate(0.0, 0.5)
                .distance(surface.evaluate(TAU, 0.5))
                > 0.1
        );
        assert!(
            PolyBoundaryPiece::try_new_from_closed_axis_edge_samples(
                &surface,
                3,
                closed_chart_test_input(&surface, 1),
                [true, false],
                &closed_chart_test_search,
                TOLERANCE,
            )
            .is_none()
        );
    }

    #[test]
    fn closed_chart_requires_repeated_source_identity() {
        let surface = UCylinder(Cylinder {
            twisted: false,
            uv_scale: 1.0,
            model_scale: 1.0,
        });
        let mut input = closed_chart_test_input(&surface, 8);
        input[0][2].0 = 8;
        assert!(
            PolyBoundaryPiece::try_new_from_closed_axis_edge_samples(
                &surface,
                3,
                input,
                [true, false],
                &closed_chart_test_search,
                TOLERANCE,
            )
            .is_none()
        );
    }

    #[test]
    fn closed_chart_requires_exactly_one_declared_closed_axis() {
        let surface = UCylinder(Cylinder {
            twisted: false,
            uv_scale: 1.0,
            model_scale: 1.0,
        });
        for axes in [[false, false], [true, true], [false, true]] {
            assert!(
                PolyBoundaryPiece::try_new_from_closed_axis_edge_samples(
                    &surface,
                    3,
                    closed_chart_test_input(&surface, 8),
                    axes,
                    &closed_chart_test_search,
                    TOLERANCE,
                )
                .is_none()
            );
        }
    }

    #[test]
    fn closed_axis_metadata_does_not_pin_twisted_nonperiodic_surface() {
        let surface = UCylinder(Cylinder {
            twisted: true,
            uv_scale: 1.0,
            model_scale: 1.0,
        });
        let piece =
            |edge, orientation, points: Vec<Point3>| (edge, orientation, PolylineCurve(points));
        let seam = (0..=8)
            .map(|i| surface.evaluate(0.0, i as f64 / 8.0))
            .collect::<Vec<_>>();
        let far = (0..=8)
            .map(|i| surface.evaluate(i as f64 * TAU / 8.0, 1.0))
            .collect::<Vec<_>>();
        let near = (0..=8)
            .rev()
            .map(|i| surface.evaluate(i as f64 * TAU / 8.0, 0.0))
            .collect::<Vec<_>>();
        let result = PolyBoundaryPiece::try_new_from_closed_axis_edge_samples(
            &surface,
            3,
            vec![vec![
                piece(5, true, seam.clone()),
                piece(6, true, far),
                piece(5, false, seam.iter().copied().rev().collect()),
                piece(7, true, near),
            ]],
            [true, false],
            &|_: &UCylinder, point: Point3, _: Option<(f64, f64)>| {
                let mut u = point.z.atan2(point.y);
                if u < 0.0 {
                    u += TAU;
                }
                Some((u, point.x))
            },
            TOLERANCE,
        );
        assert!(
            result.is_none(),
            "matching seam endpoints cannot justify a nonperiodic interior branch"
        );
    }
}

#[cfg(test)]
mod angular_closure_tests {
    use super::*;

    #[test]
    fn angular_closure_compacts_near_uv_only_for_identical_xyz() {
        let mut points = [(0., 0.), (1., 0.), (1., 1.), (0., 1.)]
            .into_iter()
            .map(|(x, y)| SurfacePoint::from((Point2::new(x, y), Point3::new(x, y, 0.))))
            .collect::<Vec<_>>();
        // The same source endpoint can project to slightly different UVs.
        // It must not create a second near-coincident CDT closure vertex.
        points.push(SurfacePoint::from((
            Point2::new(1.0e-14, 0.),
            Point3::origin(),
        )));
        let boundary = PolyBoundaryPiece::from_surface_points_with_shared_samples(points, true)
            .expect("closed boundary");
        assert_eq!(
            boundary.0.len(),
            5,
            "identical XYZ must close the loop once"
        );
    }
}

#[cfg(test)]
mod finite_collapsed_boundary_tests {
    use super::*;
    use monstertruck_geometry::prelude::{BsplineSurface, KnotVector};

    fn surface() -> BsplineSurface<Point3> {
        BsplineSurface::new(
            (KnotVector::bezier_knot(1), KnotVector::bezier_knot(1)),
            vec![
                vec![Point3::origin(), Point3::new(1.0, 0.0, 1.0)],
                vec![Point3::origin(), Point3::new(0.0, 1.0, 1.0)],
            ],
        )
    }

    #[test]
    fn completion_requires_exact_proof_and_an_admissible_actual_footpoint() {
        let surface = surface();
        let point = surface.evaluate(0.5, 0.0);
        assert_eq!(
            finite_collapsed_boundary_footpoint(&surface, point, Some((0.25, 0.5))),
            FiniteBoundaryProjection::Footpoint((0.25, 0.0))
        );
        assert_eq!(
            finite_collapsed_boundary_footpoint(&surface, point, Some((1.01, 0.5))),
            FiniteBoundaryProjection::Refused
        );
        assert_eq!(
            finite_collapsed_boundary_footpoint(&surface, point, Some((f64::NAN, 0.5))),
            FiniteBoundaryProjection::Refused
        );
        assert_eq!(
            finite_collapsed_boundary_footpoint(
                &surface,
                point + Vector3::unit_z() * 0.001,
                Some((0.25, 0.5))
            ),
            FiniteBoundaryProjection::Unproved
        );
        let mut controls = surface.control_points().clone();
        controls[1][0].x = 1e-12;
        let near = BsplineSurface::new(surface.knot_vectors().clone(), controls);
        assert_eq!(
            finite_collapsed_boundary_footpoint(&near, point, Some((0.25, 0.5))),
            FiniteBoundaryProjection::Unproved,
            "a sub-tolerance near-collapse is not a boundary certificate"
        );
        let mut transposed = surface.clone();
        transposed.invert();
        assert_eq!(
            finite_collapsed_boundary_footpoint(&transposed, point, Some((0.5, 0.25))),
            FiniteBoundaryProjection::Footpoint((0.0, 0.25))
        );
    }

    #[test]
    fn exact_anchor_matching_keeps_distinct_near_samples_and_refuses_missing_roles() {
        let surface = surface();
        let pole = Point3::origin();
        assert_eq!(
            finite_collapsed_boundary_footpoint(&surface, pole, None),
            FiniteBoundaryProjection::Refused
        );
        assert_eq!(
            finite_collapsed_boundary_footpoint(
                &surface,
                pole + Vector3::unit_z() * 1e-12,
                Some((0.25, 0.5))
            ),
            FiniteBoundaryProjection::Unproved,
            "an exact pole certificate cannot relabel a distinct near-pole shared sample"
        );
        // The source trim U in [.5,1] has this same pole at U=.5.
        // Its separate U=0 collapse must not preempt ordinary source inversion.
        let alias = BsplineSurface::new(
            (KnotVector::bezier_knot(2), KnotVector::bezier_knot(1)),
            vec![
                vec![pole, pole],
                vec![Point3::new(-0.5, 0.0, -0.5), Point3::new(0.0, -0.5, -0.5)],
                vec![Point3::new(1.0, 0.0, 1.0), Point3::new(0.0, 1.0, 1.0)],
            ],
        );
        assert_eq!(alias.evaluate(0.5, 0.5), pole);
        assert_eq!(
            finite_collapsed_boundary_footpoint(&alias, pole, Some((0.75, 0.5))),
            FiniteBoundaryProjection::Unproved
        );
        let folded = BsplineSurface::new(
            (KnotVector::bezier_knot(2), KnotVector::bezier_knot(1)),
            vec![
                vec![pole, pole],
                vec![Point3::new(1.0, 0.0, 1.0), Point3::new(0.0, 1.0, 1.0)],
                vec![pole, pole],
            ],
        );
        assert_eq!(
            folded.finite_collapsed_boundaries(),
            Some([[false, false], [false, false]])
        );
        assert_eq!(
            finite_collapsed_boundary_footpoint(&folded, pole, Some((0.5, 0.25))),
            FiniteBoundaryProjection::Unproved,
            "two source boundaries with the same physical anchor need an explicit trim role"
        );
    }

}

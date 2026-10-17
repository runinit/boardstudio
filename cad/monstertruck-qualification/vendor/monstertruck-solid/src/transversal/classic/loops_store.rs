//! Classic (0.3.2) boolean loops store.
//!
//! Ported verbatim from the published 0.3.2 crate's `transversal::loops_store`,
//! adjusted only for the graft: the marching intersection-curve backend now
//! yields a raw [`IntersectionCurve`] leader (the 0.3.2 parameter wrapper was
//! dropped), so the `.into()` that unwrapped the wrapper is gone.

#![allow(clippy::many_single_char_names)]

use super::intersection_curve;
use monstertruck_core::cgmath64::*;
use monstertruck_geometry::prelude::*;
use monstertruck_meshing::prelude::*;
use monstertruck_topology::{Vertex, *};
use rustc_hash::FxHashMap as HashMap;

type PolylineCurve = monstertruck_meshing::prelude::PolylineCurve<Point3>;
// Retain the parent allocation so its EdgeId cannot be recycled during this event.
type EdgeCuts<P, C> = HashMap<(EdgeId<C>, VertexId<P>), (Edge<P, C>, f64, Wire<P, C>)>;
type IntersectionLoopStorePair<'geometry, 'polygon, C> = (
    &'geometry mut LoopsStore<Point3, C>,
    &'polygon mut LoopsStore<Point3, PolylineCurve>,
);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ShapesOpStatus {
    Unknown,
    And,
    Or,
}

impl ShapesOpStatus {
    fn not(self) -> Self {
        match self {
            Self::Unknown => Self::Unknown,
            Self::And => Self::Or,
            Self::Or => Self::And,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct BoundaryWire<P, C> {
    wire: Wire<P, C>,
    status: ShapesOpStatus,
}

impl<P, C> BoundaryWire<P, C> {
    #[inline(always)]
    pub(super) fn new(wire: Wire<P, C>, status: ShapesOpStatus) -> Self { Self { wire, status } }
    #[inline(always)]
    pub(super) fn status(&self) -> ShapesOpStatus { self.status }
    #[inline(always)]
    pub(super) fn inverse(&self) -> Self {
        Self {
            wire: self.wire.inverse(),
            status: self.status.not(),
        }
    }
}

impl ShapesOpStatus {
    fn from_is_curve<C, S0, S1>(curve: &IntersectionCurve<C, S0, S1>) -> Option<ShapesOpStatus>
    where
        C: ParametricCurve3D + BoundedCurve,
        S0: ParametricSurface3D + SearchNearestParameter<SurfaceParameter, Point = Point3>,
        S1: ParametricSurface3D + SearchNearestParameter<SurfaceParameter, Point = Point3>, {
        let (t0, t1) = curve.range_tuple();
        let t = (t0 + t1) / 2.0;
        let (_, pt0, pt1) = curve.search_triple(t, 100)?;
        let der = curve.leader().derivative(t);
        let normal0 = curve.surface0().normal(pt0[0], pt0[1]);
        let normal1 = curve.surface1().normal(pt1[0], pt1[1]);
        match normal0.cross(der).dot(normal1) > 0.0 {
            true => Some(ShapesOpStatus::Or),
            false => Some(ShapesOpStatus::And),
        }
    }
}

impl<P, C> std::ops::Deref for BoundaryWire<P, C> {
    type Target = Wire<P, C>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target { &self.wire }
}

impl<P, C> std::ops::DerefMut for BoundaryWire<P, C> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.wire }
}

#[derive(Clone, Debug)]
pub(super) struct Loops<P, C>(Vec<BoundaryWire<P, C>>);
#[derive(Clone, Debug)]
pub(super) struct LoopsStore<P, C>(Vec<Loops<P, C>>);

impl<P, C> std::ops::Deref for Loops<P, C> {
    type Target = Vec<BoundaryWire<P, C>>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl<P, C> std::ops::DerefMut for Loops<P, C> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl<P, C> std::ops::Deref for LoopsStore<P, C> {
    type Target = Vec<Loops<P, C>>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl<P, C> std::ops::DerefMut for LoopsStore<P, C> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl<P, C> FromIterator<BoundaryWire<P, C>> for Loops<P, C> {
    #[inline(always)]
    fn from_iter<I: IntoIterator<Item = BoundaryWire<P, C>>>(iter: I) -> Self {
        Self(Vec::from_iter(iter))
    }
}

impl<'a, P, C, S> From<&'a Face<P, C, S>> for Loops<P, C> {
    #[inline(always)]
    fn from(face: &'a Face<P, C, S>) -> Loops<P, C> {
        face.absolute_boundaries()
            .iter()
            .map(|wire| BoundaryWire::new(wire.clone(), ShapesOpStatus::Unknown))
            .collect()
    }
}

impl<'a, P: 'a, C: 'a, S: 'a> FromIterator<&'a Face<P, C, S>> for LoopsStore<P, C> {
    fn from_iter<I: IntoIterator<Item = &'a Face<P, C, S>>>(iter: I) -> Self {
        Self(iter.into_iter().map(Loops::from).collect())
    }
}

impl<'a, P, C> IntoIterator for &'a LoopsStore<P, C> {
    type Item = <&'a Vec<Loops<P, C>> as IntoIterator>::Item;
    type IntoIter = <&'a Vec<Loops<P, C>> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter { self.0.iter() }
}

type MeshBounds = BoundingBox<Point3>;

fn mesh_bounds_from_positions(positions: &[Point3]) -> Option<MeshBounds> {
    if positions.iter().any(|point| {
        !point.x.is_finite() || !point.y.is_finite() || !point.z.is_finite()
    }) {
        return None;
    }
    let bounds: MeshBounds = positions.iter().collect();
    (!bounds.is_empty()).then_some(bounds)
}

fn mesh_bounds(mesh: &PolygonMesh) -> Option<MeshBounds> {
    let positions = mesh.positions();
    // The old collision path indexes every face vertex into `positions`.
    // Unknown indices must keep that path so its failure behavior is retained.
    if mesh
        .faces()
        .face_iter()
        .flatten()
        .any(|vertex| vertex.pos >= positions.len())
    {
        return None;
    }
    // Include unused positions as before; extra extents only reduce pruning.
    mesh_bounds_from_positions(positions)
}

fn should_run_narrow_phase(bounds0: Option<MeshBounds>, bounds1: Option<MeshBounds>) -> bool {
    // `is_empty` uses strict interval inversion, so a zero-width touching
    // intersection is retained for the narrow phase.
    !matches!((bounds0, bounds1), (Some(first), Some(second)) if (first ^ second).is_empty())
}

#[cfg(test)]
mod face_bounds_tests {
    use super::*;

    fn bounds(min: Point3, max: Point3) -> MeshBounds {
        mesh_bounds_from_positions(&[min, max]).expect("finite nonempty bounds")
    }

    fn triangle_mesh(points: [Point3; 3]) -> PolygonMesh {
        PolygonMesh::new(
            StandardAttributes {
                positions: points.to_vec(),
                ..Default::default()
            },
            Faces::from_iter(&[[0, 1, 2]]),
        )
    }

    #[test]
    fn disjoint_face_bounds_match_existing_triangle_rejection() {
        let first = triangle_mesh([
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ]);
        let disjoint = triangle_mesh([
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(3.0, 0.0, 0.0),
            Point3::new(2.0, 1.0, 0.0),
        ]);
        assert!(!should_run_narrow_phase(
            mesh_bounds(&first),
            mesh_bounds(&disjoint),
        ));
        assert!(first.extract_interference(&disjoint).is_empty());
    }

    #[test]
    fn unchecked_invalid_position_index_keeps_narrow_phase() {
        let malformed = PolygonMesh::new_unchecked(
            StandardAttributes {
                positions: vec![
                    Point3::new(0.0, 0.0, 0.0),
                    Point3::new(1.0, 0.0, 0.0),
                    Point3::new(0.0, 1.0, 0.0),
                ],
                ..Default::default()
            },
            Faces::from_iter(&[[0usize, 1, 99]]),
        );
        let disjoint = triangle_mesh([
            Point3::new(3.0, 0.0, 0.0),
            Point3::new(4.0, 0.0, 0.0),
            Point3::new(3.0, 1.0, 0.0),
        ]);
        let malformed_bounds = mesh_bounds(&malformed);
        assert!(malformed_bounds.is_none());
        assert!(should_run_narrow_phase(
            malformed_bounds,
            mesh_bounds(&disjoint),
        ));
    }

    #[test]
    fn unused_finite_position_remains_in_cached_mesh_extent() {
        let mesh_with_unused_position = PolygonMesh::new_unchecked(
            StandardAttributes {
                positions: vec![
                    Point3::new(0.0, 0.0, 0.0),
                    Point3::new(1.0, 0.0, 0.0),
                    Point3::new(0.0, 1.0, 0.0),
                    Point3::new(3.0, 0.25, 0.0),
                ],
                ..Default::default()
            },
            Faces::from_iter(&[[0usize, 1, 2]]),
        );
        let other = triangle_mesh([
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(3.0, 0.0, 0.0),
            Point3::new(2.0, 1.0, 0.0),
        ]);
        assert!(should_run_narrow_phase(
            mesh_bounds(&mesh_with_unused_position),
            mesh_bounds(&other),
        ));
    }

    #[test]
    fn clearly_disjoint_face_mesh_bounds_skip_narrow_phase() {
        let first = bounds(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        let second = bounds(Point3::new(2.0, 0.0, 0.0), Point3::new(3.0, 1.0, 1.0));
        assert!(!should_run_narrow_phase(Some(first), Some(second)));
    }

    #[test]
    fn exact_touch_keeps_narrow_phase() {
        let first = bounds(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        let touching = bounds(Point3::new(1.0, 0.25, 0.25), Point3::new(2.0, 0.75, 0.75));
        assert!(should_run_narrow_phase(Some(first), Some(touching)));
    }

    #[test]
    fn one_ulp_gap_is_disjoint() {
        let first = bounds(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        let next_x = f64::from_bits(1.0_f64.to_bits() + 1);
        let near = bounds(Point3::new(next_x, 0.25, 0.25), Point3::new(2.0, 0.75, 0.75));
        assert!(!should_run_narrow_phase(Some(first), Some(near)));
    }

    #[test]
    fn extreme_finite_overlapping_bounds_keep_narrow_phase() {
        let first = bounds(
            Point3::new(1.0e300, -1.0e300, -1.0e300),
            Point3::new(1.5e300, 1.0e300, 1.0e300),
        );
        let second = bounds(
            Point3::new(1.25e300, -2.0e300, -2.0e300),
            Point3::new(1.75e300, 2.0e300, 2.0e300),
        );
        assert!(should_run_narrow_phase(Some(first), Some(second)));
    }

    #[test]
    fn missing_or_nonfinite_bounds_keep_narrow_phase() {
        let finite = bounds(Point3::origin(), Point3::new(1.0, 1.0, 1.0));
        assert!(should_run_narrow_phase(None, Some(finite)));
        assert!(mesh_bounds_from_positions(&[]).is_none());
        assert!(mesh_bounds_from_positions(&[
            Point3::origin(),
            Point3::new(f64::NAN, 1.0, 1.0),
        ])
        .is_none());
    }
}

#[derive(Clone, Debug, Copy, PartialEq)]
enum ParameterKind {
    Front,
    Back,
    Inner(f64),
}

impl ParameterKind {
    fn try_new(t: f64, (t0, t1): (f64, f64)) -> Option<ParameterKind> {
        if t0.near(&t) {
            Some(ParameterKind::Front)
        } else if t1.near(&t) {
            Some(ParameterKind::Back)
        } else if t0 < t && t < t1 {
            Some(ParameterKind::Inner(t))
        } else {
            None
        }
    }
}

impl<P: Copy + PartialEq, C: Clone> Loops<P, C> {
    fn search_parameter(&self, pt: P) -> Option<(usize, usize, ParameterKind)>
    where C: BoundedCurve<Point = P> + SearchParameter<CurveParameter, Point = P> {
        self.iter()
            .enumerate()
            .flat_map(|(i, wire)| wire.iter().enumerate().map(move |(j, edge)| (i, j, edge)))
            .find_map(|(i, j, edge)| {
                if edge.absolute_front().point() == pt {
                    Some((i, j, ParameterKind::Front))
                } else if edge.absolute_back().point() == pt {
                    Some((i, j, ParameterKind::Back))
                } else {
                    None
                }
            })
            .or_else(|| {
                self.iter()
                    .enumerate()
                    .flat_map(|(i, wire)| {
                        wire.iter().enumerate().map(move |(j, edge)| (i, j, edge))
                    })
                    .find_map(|(i, j, edge)| {
                        let curve = edge.curve();
                        curve.search_parameter(pt, None, 1).and_then(|t| {
                            let kind = ParameterKind::try_new(t, curve.range_tuple())?;
                            Some((i, j, kind))
                        })
                    })
            })
    }

    fn change_vertex(
        &mut self,
        old_vertex: &Vertex<P>,
        new_vertex: &Vertex<P>,
        emap: &mut HashMap<EdgeId<C>, Edge<P, C>>,
    ) {
        self.iter_mut()
            .flat_map(|wire| wire.iter_mut())
            .for_each(|edge| {
                let mut new_edge = if edge.absolute_front() == old_vertex {
                    emap.entry(edge.id()).or_insert_with(|| {
                        Edge::new(new_vertex, edge.absolute_back(), edge.curve())
                    })
                } else if edge.absolute_back() == old_vertex {
                    emap.entry(edge.id()).or_insert_with(|| {
                        Edge::new(edge.absolute_front(), new_vertex, edge.curve())
                    })
                } else {
                    return;
                }
                .clone();
                if !edge.orientation() {
                    new_edge.invert();
                }
                // Remove the edge from the HashMap when it is no longer there because Id reassignment will occur.
                if edge.count() == 1 {
                    emap.remove(&edge.id());
                }
                *edge = new_edge;
            })
    }

    fn swap_edge_into_wire(&mut self, edge_id: EdgeId<C>, new_wire: &Wire<P, C>) {
        self.iter_mut().for_each(|wire| {
            while let Some(idx) = wire.iter().position(|edge| edge.id() == edge_id) {
                if wire[idx].orientation() {
                    wire.swap_edge_into_wire(idx, new_wire.clone());
                } else {
                    wire.swap_edge_into_wire(idx, new_wire.inverse());
                }
            }
        });
    }

    #[inline(always)]
    fn add_independent_loop(&mut self, r#loop: BoundaryWire<P, C>) {
        self.push(r#loop.inverse());
        self.push(r#loop);
    }

    fn edge_attachments(&self, edge0: &Edge<P, C>) -> Option<[Option<(usize, usize)>; 2]> {
        let attachments = |vertex: &Vertex<P>| {
            self.iter()
                .enumerate()
                .flat_map(|(i, wire)| {
                    wire.iter()
                        .enumerate()
                        .filter_map(move |(j, edge)| (edge.front() == vertex).then_some((i, j)))
                })
                .collect::<Vec<_>>()
        };
        let back = attachments(edge0.back());
        let front = attachments(edge0.front());
        let common = back
            .iter()
            .flat_map(|a| {
                front
                    .iter()
                    .filter_map(move |b| (a.0 == b.0).then_some((*a, *b)))
            })
            .collect::<Vec<_>>();
        let (a, b) = match common.as_slice() {
            // Shared contacts can also belong to an already partitioned neighbour.
            // Split the unique common owner instead of merging unrelated first hits.
            [(a, b)] => (Some(*a), Some(*b)),
            [] if back.len() <= 1 && front.len() <= 1 => {
                (back.first().copied(), front.first().copied())
            }
            // Multiple owners need geometric side selection; never guess at material.
            _ => return None,
        };
        Some([a, b])
    }

    fn add_edge(
        &mut self,
        edge: Edge<P, C>,
        status: ShapesOpStatus,
    ) -> Option<[Option<(usize, usize)>; 2]> {
        let attachments = self.edge_attachments(&edge)?;
        Some(self.add_edge_at(edge, status, attachments))
    }

    fn add_edge_at(
        &mut self,
        edge0: Edge<P, C>,
        status: ShapesOpStatus,
        [a, b]: [Option<(usize, usize)>; 2],
    ) -> [Option<(usize, usize)>; 2] {
        if let Some((wire_index0, edge_index0)) = a {
            self[wire_index0].rotate_left(edge_index0);
            self[wire_index0].push_front(edge0.clone());
            self[wire_index0].push_back(edge0.inverse());
        }
        match (a, b) {
            (Some((wire_index0, edge_index0)), Some((wire_index1, edge_index1))) => {
                if wire_index0 == wire_index1 {
                    let len = self[wire_index0].len() - 2;
                    let edge_index1 = (len + edge_index1 - edge_index0) % len + 1;
                    let new_wire = self[wire_index0].split_off(edge_index1);
                    self[wire_index0].status = status;
                    self.push(BoundaryWire::new(new_wire, status.not()));
                } else {
                    let mut new_wire0 = self[wire_index1].clone();
                    let mut new_wire1 = new_wire0.split_off(edge_index1);
                    new_wire0.append(&mut self[wire_index0]);
                    new_wire0.append(&mut new_wire1);
                    self[wire_index0] = new_wire0;
                    self.swap_remove(wire_index1);
                }
            }
            (None, Some((wire_index1, edge_index1))) => {
                self[wire_index1].rotate_left(edge_index1);
                self[wire_index1].push_front(edge0.inverse());
                self[wire_index1].push_back(edge0);
            }
            (None, None) => self.push(BoundaryWire::new(
                vec![edge0.inverse(), edge0].into(),
                ShapesOpStatus::Unknown,
            )),
            _ => {}
        }
        [a, b]
    }
}

impl<P: Copy + PartialEq + Tolerance, C: Clone> LoopsStore<P, C> {
    #[inline(always)]
    fn change_vertex(
        &mut self,
        old_vertex: &Vertex<P>,
        new_vertex: &Vertex<P>,
        emap: &mut HashMap<EdgeId<C>, Edge<P, C>>,
    ) {
        self.iter_mut()
            .for_each(|loops| loops.change_vertex(old_vertex, new_vertex, emap));
    }

    #[inline(always)]
    fn swap_edge_into_wire(&mut self, edge_id: EdgeId<C>, new_wire: &Wire<P, C>) {
        self.iter_mut()
            .for_each(|loops| loops.swap_edge_into_wire(edge_id, new_wire))
    }

    fn add_polygon_vertex(
        &mut self,
        loops_index: usize,
        v: &Vertex<P>,
        emap: &mut HashMap<EdgeId<C>, Edge<P, C>>,
        cuts: &mut EdgeCuts<P, C>,
    ) -> std::result::Result<Option<(usize, usize, ParameterKind)>, ()>
    where
        C: Cut<Point = P> + SearchParameter<CurveParameter, Point = P>,
    {
        let pt = v.point();
        let Some((wire_index, edge_index, kind)) = self[loops_index].search_parameter(pt) else { return Ok(None); };
        match kind {
            ParameterKind::Front => {
                let old_vertex = self[loops_index][wire_index][edge_index]
                    .absolute_front()
                    .clone();
                self.change_vertex(&old_vertex, v, emap);
            }
            ParameterKind::Back => {
                let old_vertex = self[loops_index][wire_index][edge_index]
                    .absolute_back()
                    .clone();
                self.change_vertex(&old_vertex, v, emap);
            }
            ParameterKind::Inner(t) => {
                let edge = self[loops_index][wire_index][edge_index].absolute_clone();
                let edge_id = edge.id();
                let key = (edge_id, v.id());
                let new_wire = if let Some((_parent, parameter, wire)) = cuts.get(&key) {
                    if t != *parameter { return Err(()); }
                    wire.clone()
                } else {
                    let (edge0, edge1) = edge.cut_with_parameter(v, t).ok_or(())?;
                    let wire: Wire<_, _> = vec![edge0, edge1].into();
                    cuts.insert(key, (edge, t, wire.clone()));
                    wire
                };
                self.swap_edge_into_wire(edge_id, &new_wire);
            }
        }
        Ok(Some((wire_index, edge_index, kind)))
    }
}

impl<C> LoopsStore<Point3, C> {
    fn add_geom_vertex<S>(
        &mut self,
        (loops_index, wire_index, edge_index): (usize, usize, usize),
        v: &Vertex<Point3>,
        kind: ParameterKind,
        another_surface: &S,
        emap: &mut HashMap<EdgeId<C>, Edge<Point3, C>>,
        cuts: &mut EdgeCuts<Point3, C>,
    ) -> Option<()>
    where
        C: Cut<Point = Point3, Vector = Vector3>
            + SearchNearestParameter<CurveParameter, Point = Point3>,
        S: ParametricSurface3D + SearchNearestParameter<SurfaceParameter, Point = Point3>,
    {
        match kind {
            ParameterKind::Front => {
                let old_vertex = self[loops_index][wire_index][edge_index]
                    .absolute_front()
                    .clone();
                v.set_point(old_vertex.point());
                self.change_vertex(&old_vertex, v, emap);
            }
            ParameterKind::Back => {
                let old_vertex = self[loops_index][wire_index][edge_index]
                    .absolute_back()
                    .clone();
                v.set_point(old_vertex.point());
                self.change_vertex(&old_vertex, v, emap);
            }
            ParameterKind::Inner(_) => {
                let edge = self[loops_index][wire_index][edge_index].absolute_clone();
                let edge_id = edge.id();
                let curve = edge.curve();
                let (pt, t, _) = curve_surface_projection(&curve, None, another_surface, None, v.point(), 100)?;
                let key = (edge_id, v.id());
                let new_wire = if let Some((_parent, parameter, wire)) = cuts.get(&key) {
                    if t != *parameter { return None; }
                    wire.clone()
                } else {
                    v.set_point(pt);
                    let (edge0, edge1) = edge.cut_with_parameter(v, t)?;
                    let wire: Wire<_, _> = vec![edge0, edge1].into();
                    cuts.insert(key, (edge, t, wire.clone()));
                    wire
                };
                self.swap_edge_into_wire(edge_id, &new_wire);
            }
        }
        Some(())
    }
}

fn curve_surface_projection<C, S>(
    curve: &C,
    curve_hint: Option<f64>,
    surface: &S,
    surface_hint: Option<(f64, f64)>,
    point: Point3,
    trials: usize,
) -> Option<(Point3, f64, Point2)>
where
    C: ParametricCurve3D + BoundedCurve + SearchNearestParameter<CurveParameter, Point = Point3>,
    S: ParametricSurface3D + SearchNearestParameter<SurfaceParameter, Point = Point3>,
{
    if trials == 0 {
        return None;
    }
    let t = curve.search_nearest_parameter(point, curve_hint, 10)?;
    let pt0 = curve.evaluate(t);
    let (u, v) = surface.search_nearest_parameter(point, surface_hint, 10)?;
    let pt1 = surface.evaluate(u, v);
    let tangent = curve.derivative(t);
    let normal = surface.normal(u, v);
    let parameter_step = (pt1 - pt0).dot(normal) / tangent.dot(normal);
    if point.near(&pt0) && point.near(&pt1) && pt0.near(&pt1) {
        // Polish the shared event on its actual curve before using it as a cut vertex.
        let corrected = t + parameter_step;
        let (front, back) = curve.range_tuple();
        if corrected.is_finite() && corrected >= front && corrected <= back {
            let polished = curve.evaluate(corrected);
            if polished.near(&pt0)
                && let Some(uv) = surface.search_nearest_parameter(polished, Some((u, v)), 10)
                && polished.near(&surface.evaluate(uv.0, uv.1))
            {
                return Some((polished, corrected, Point2::new(uv.0, uv.1)));
            }
        }
        Some((pt0, t, Point2::new(u, v)))
    } else {
        curve_surface_projection(
            curve,
            Some(t),
            surface,
            Some((u, v)),
            pt0 + parameter_step * tangent,
            trials - 1,
        )
    }
}

fn create_independent_loop<P, C, D>(mut poly_curve0: C) -> Wire<P, D>
where
    C: Cut<Point = P>,
    D: From<C>,
{
    let (t0, t1) = poly_curve0.range_tuple();
    let t = (t0 + t1) / 2.0;
    let poly_curve1 = poly_curve0.cut(t);
    let v0 = Vertex::new(poly_curve0.front());
    let v1 = Vertex::new(poly_curve1.front());
    let edge0 = Edge::new(&v0, &v1, poly_curve0.into());
    let edge1 = Edge::new(&v1, &v0, poly_curve1.into());
    wire![edge0, edge1]
}

#[allow(dead_code)]
pub(super) struct LoopsStoreQuadruple<C> {
    pub(super) geom_loops_store0: LoopsStore<Point3, C>,
    pub(super) poly_loops_store0: LoopsStore<Point3, PolylineCurve>,
    pub(super) geom_loops_store1: LoopsStore<Point3, C>,
    pub(super) poly_loops_store1: LoopsStore<Point3, PolylineCurve>,
}

fn projected_edge_attachments<S>(
    loops: &Loops<Point3, PolylineCurve>,
    edge: &Edge<Point3, PolylineCurve>,
    surface: &S,
) -> Option<[Option<(usize, usize)>; 2]>
where
    S: ParametricSurface3D + SearchParameter<SurfaceParameter, Point = Point3>,
{
    let curve = edge.oriented_curve();
    let endpoints = [edge.back(), edge.front()];
    let neighbours = [
        curve
            .iter()
            .rev()
            .skip(1)
            .find(|p| **p != edge.back().point())
            .copied()?,
        curve
            .iter()
            .skip(1)
            .find(|p| **p != edge.front().point())
            .copied()?,
    ];
    let project = |point: Point3, hint: Option<(f64, f64)>| -> Option<Point2> {
        let uv = surface.search_parameter(point, hint, 100)?;
        let represented = surface.evaluate(uv.0, uv.1);
        (represented.near(&point) && uv.0.is_finite() && uv.1.is_finite())
            .then_some(Point2::from(uv))
    };
    let direction = |vector: Vector2| -> Option<Vector2> {
        let length = vector.x.hypot(vector.y);
        (length.is_finite() && length > 0.0).then(|| vector / length)
    };
    let mut result = [None; 2];
    for endpoint in 0..2 {
        let candidates: Vec<_> = loops
            .iter()
            .enumerate()
            .flat_map(|(i, wire)| {
                wire.iter().enumerate().filter_map(move |(j, outgoing)| {
                    (outgoing.front() == endpoints[endpoint]).then_some((i, j))
                })
            })
            .collect();
        if candidates.len() < 2 {
            result[endpoint] = candidates.first().copied();
            continue;
        }
        let origin = project(endpoints[endpoint].point(), None)?;
        let reference = direction(project(neighbours[endpoint], Some(origin.into()))? - origin)?;
        let mut best = None;
        let mut tied = false;
        for (i, j) in candidates {
            let outgoing = &loops[i][j];
            let path = outgoing.oriented_curve();
            let point = path
                .iter()
                .skip(1)
                .find(|p| **p != outgoing.front().point())
                .copied()?;
            let tangent = direction(project(point, Some(origin.into()))? - origin)?;
            // The clockwise successor of the reversed incoming direction traces
            // the region on the left in the existing surface parameter chart.
            let cross = reference.x * tangent.y - reference.y * tangent.x;
            let angle = (-cross)
                .atan2(reference.dot(tangent))
                .rem_euclid(std::f64::consts::TAU);
            match best {
                None => {
                    best = Some((angle, (i, j)));
                    tied = false;
                }
                Some((minimum, _)) if angle < minimum => {
                    best = Some((angle, (i, j)));
                    tied = false;
                }
                Some((minimum, _)) if angle == minimum => tied = true,
                _ => {}
            }
        }
        if tied {
            return None;
        }
        result[endpoint] = Some(best?.1);
    }
    Some(result)
}

fn add_paired_intersection_edge<C: Clone, S>(
    polygon: &mut Loops<Point3, PolylineCurve>,
    geometry: &mut Loops<Point3, C>,
    polygon_edge: Edge<Point3, PolylineCurve>,
    geometry_edge: Edge<Point3, C>,
    status: ShapesOpStatus,
    surface: &S,
) -> Option<()>
where
    S: ParametricSurface3D + SearchParameter<SurfaceParameter, Point = Point3>,
{
    if polygon.len() != geometry.len()
        || polygon
            .iter()
            .zip(geometry.iter())
            .any(|(p, g)| p.len() != g.len())
    {
        return None;
    }
    let attachments = match polygon.add_edge(polygon_edge.clone(), status) {
        Some(attachments) => attachments,
        None => {
            let attachments = projected_edge_attachments(polygon, &polygon_edge, surface)?;
            polygon.add_edge_at(polygon_edge, status, attachments)
        }
    };
    geometry.add_edge_at(geometry_edge, status, attachments);
    Some(())
}

fn attach_intersection_vertex<C, S>(
    mut stores: [IntersectionLoopStorePair<'_, '_, C>; 2],
    indices: [usize; 2],
    surfaces: [&S; 2],
    polygon_vertex: &Vertex<Point3>,
    geometry_vertex: &Vertex<Point3>,
) -> Option<()>
where
    C: SearchNearestParameter<CurveParameter, Point = Point3>
        + Cut<Point = Point3, Vector = Vector3>,
    S: ParametricSurface3D + SearchNearestParameter<SurfaceParameter, Point = Point3>,
{
    let mut polygon_replacements = HashMap::default();
    let mut geometry_replacements = HashMap::default();
    let mut polygon_cuts = HashMap::default();
    let mut geometry_cuts = HashMap::default();
    let mut attached = [false; 2];
    for _ in 0..2 {
        for i in 0..2 {
            if attached[i] {
                continue;
            }
            let (geometry, polygon) = &mut stores[i];
            if let Some((wire, edge, kind)) = polygon
                .add_polygon_vertex(
                    indices[i],
                    polygon_vertex,
                    &mut polygon_replacements,
                    &mut polygon_cuts,
                )
                .ok()?
            {
                geometry.add_geom_vertex(
                    (indices[i], wire, edge),
                    geometry_vertex,
                    kind,
                    surfaces[1 - i],
                    &mut geometry_replacements,
                    &mut geometry_cuts,
                )?;
                attached[i] = true;
            }
        }
        if polygon_vertex.point() == geometry_vertex.point() {
            break;
        }
        // Projection on the other operand can reveal a previously missed boundary contact.
        polygon_vertex.set_point(geometry_vertex.point());
    }
    Some(())
}

pub(super) fn create_loops_stores<C, S>(
    geom_shell0: &Shell<Point3, C, S>,
    poly_shell0: &Shell<Point3, PolylineCurve, Option<PolygonMesh>>,
    geom_shell1: &Shell<Point3, C, S>,
    poly_shell1: &Shell<Point3, PolylineCurve, Option<PolygonMesh>>,
) -> Option<LoopsStoreQuadruple<C>>
where
    C: SearchNearestParameter<CurveParameter, Point = Point3>
        + SearchParameter<CurveParameter, Point = Point3>
        + Cut<Point = Point3, Vector = Vector3>
        + From<IntersectionCurve<PolylineCurve, S, S>>,
    S: ParametricSurface3D
        + Clone
        + SearchParameter<SurfaceParameter, Point = Point3>
        + SearchNearestParameter<SurfaceParameter, Point = Point3>,
{
    let mut geom_loops_store0: LoopsStore<_, _> = geom_shell0.face_iter().collect();
    let mut poly_loops_store0: LoopsStore<_, _> = poly_shell0.face_iter().collect();
    let mut geom_loops_store1: LoopsStore<_, _> = geom_shell1.face_iter().collect();
    let mut poly_loops_store1: LoopsStore<_, _> = poly_shell1.face_iter().collect();
    let store0_len = geom_loops_store0.len();
    let store1_len = geom_loops_store1.len();
    let mesh_bounds0 = poly_shell0
        .iter()
        .map(|face| face.surface().as_ref().and_then(mesh_bounds))
        .collect::<Vec<_>>();
    let mesh_bounds1 = poly_shell1
        .iter()
        .map(|face| face.surface().as_ref().and_then(mesh_bounds))
        .collect::<Vec<_>>();
    let pair_result = (0..store0_len)
        .flat_map(move |i| (0..store1_len).map(move |j| (i, j)))
        .try_for_each(|(face_index0, face_index1)| {
            let ori0 = geom_shell0[face_index0].orientation();
            let ori1 = geom_shell1[face_index1].orientation();
            let surface0 = geom_shell0[face_index0].surface();
            let surface1 = geom_shell1[face_index1].surface();
            let polygon0 = poly_shell0[face_index0].surface()?;
            let polygon1 = poly_shell1[face_index1].surface()?;
            if !should_run_narrow_phase(
                mesh_bounds0[face_index0],
                mesh_bounds1[face_index1],
            ) {
                return Some(());
            }
            intersection_curve::intersection_curves(
                surface0.clone(),
                &polygon0,
                surface1.clone(),
                &polygon1,
            )?
            .into_iter()
            .try_for_each(|(mut polyline, closed, mut intersection_curve)| {
                let status = ShapesOpStatus::from_is_curve(&intersection_curve)?;
                let (status0, status1) = match (ori0, ori1) {
                    (true, true) => (status, status.not()),
                    (true, false) => (status.not(), status.not()),
                    (false, true) => (status, status),
                    (false, false) => (status.not(), status),
                };
                if closed {
                    let poly_wire = create_independent_loop(polyline);
                    poly_loops_store0[face_index0]
                        .add_independent_loop(BoundaryWire::new(poly_wire.clone(), status0));
                    poly_loops_store1[face_index1]
                        .add_independent_loop(BoundaryWire::new(poly_wire, status1));
                    let geom_wire = create_independent_loop(intersection_curve);
                    geom_loops_store0[face_index0]
                        .add_independent_loop(BoundaryWire::new(geom_wire.clone(), status0));
                    geom_loops_store1[face_index1]
                        .add_independent_loop(BoundaryWire::new(geom_wire, status1));
                } else {
                    let pv0 = Vertex::new(polyline.front());
                    let pv1 = Vertex::new(polyline.back());
                    let gv0 = Vertex::new(polyline.front());
                    let gv1 = Vertex::new(polyline.back());
                    for (polygon_vertex, geometry_vertex) in [(&pv0, &gv0), (&pv1, &gv1)] {
                        attach_intersection_vertex(
                            [
                                (&mut geom_loops_store0, &mut poly_loops_store0),
                                (&mut geom_loops_store1, &mut poly_loops_store1),
                            ],
                            [face_index0, face_index1],
                            [&surface0, &surface1],
                            polygon_vertex,
                            geometry_vertex,
                        )?;
                    }
                    let leader = intersection_curve.leader_mut();
                    *leader.first_mut()? = gv0.point();
                    *leader.last_mut()? = gv1.point();
                    *polyline.first_mut()? = pv0.point();
                    *polyline.last_mut()? = pv1.point();
                    let pedge = Edge::new(&pv0, &pv1, polyline);
                    let gedge = Edge::new(&gv0, &gv1, intersection_curve.into());
                    add_paired_intersection_edge(
                        &mut poly_loops_store0[face_index0],
                        &mut geom_loops_store0[face_index0],
                        pedge.clone(),
                        gedge.clone(),
                        status0,
                        &surface0,
                    )?;
                    add_paired_intersection_edge(
                        &mut poly_loops_store1[face_index1],
                        &mut geom_loops_store1[face_index1],
                        pedge,
                        gedge,
                        status1,
                        &surface1,
                    )?;
                }
                Some(())
            })
        });
    pair_result?;
    Some(LoopsStoreQuadruple {
        geom_loops_store0,
        poly_loops_store0,
        geom_loops_store1,
        poly_loops_store1,
    })
}

#[cfg(test)]
mod endpoint_priority_regression {
    use super::*;

    #[test]
    fn exact_back_endpoint_does_not_cut_nearby_interior_polyline_sample() {
        let start = Point3::new(0.0, 0.0, 0.0);
        let near_end = Point3::new(1.0, -8.6e-7, 0.0);
        let end = Point3::new(1.0, 0.0, 0.0);
        let front = Vertex::new(start);
        let back = Vertex::new(end);
        let polygon_edge = Edge::new(&front, &back, PolylineCurve(vec![start, near_end, end]));
        let geometry_edge = Edge::new(&front, &back, PolylineCurve(vec![start, end]));
        let polygon_loops: Loops<Point3, PolylineCurve> = vec![BoundaryWire::new(
            vec![polygon_edge].into(),
            ShapesOpStatus::Unknown,
        )]
        .into_iter()
        .collect();
        let mut polygon_store = LoopsStore(vec![polygon_loops]);
        let inserted = polygon_store
            .add_polygon_vertex(0, &Vertex::new(end), &mut HashMap::default(), &mut HashMap::default())
            .expect("endpoint update must succeed").expect("exact endpoint must be found");
        assert_eq!(inserted, (0, 0, ParameterKind::Back));
        assert_eq!(
            polygon_store[0][0].len(),
            1,
            "must not create a zero-length tail edge"
        );
        let repeated = polygon_store
            .add_polygon_vertex(0, &Vertex::new(end), &mut HashMap::default(), &mut HashMap::default())
            .expect("endpoint update must succeed").expect("repeated endpoint must be found");
        assert_eq!(repeated.2, ParameterKind::Back);
        assert_eq!(
            polygon_store[0][0].len(),
            1,
            "repeated endpoint must not append a zero-length edge"
        );

        let inverted =
            Edge::new(&front, &back, PolylineCurve(vec![start, near_end, end])).inverse();
        let inverted_loops: Loops<Point3, PolylineCurve> = vec![BoundaryWire::new(
            vec![inverted].into(),
            ShapesOpStatus::Unknown,
        )]
        .into_iter()
        .collect();
        assert_eq!(
            inverted_loops.search_parameter(end),
            Some((0, 0, ParameterKind::Back))
        );

        let geometry_loops: Loops<Point3, PolylineCurve> = vec![BoundaryWire::new(
            vec![geometry_edge].into(),
            ShapesOpStatus::Unknown,
        )]
        .into_iter()
        .collect();
        let mut geometry_store = LoopsStore(vec![geometry_loops]);
        let plane = Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        geometry_store
            .add_geom_vertex(
                (0, 0, 0),
                &Vertex::new(end),
                inserted.2,
                &plane,
                &mut HashMap::default(),
                &mut HashMap::default(),
            )
            .expect("paired endpoint replacement must not attempt a strict-interior geometry cut");
        assert_eq!(geometry_store[0][0].len(), 1);
        assert_eq!(geometry_store[0][0][0].absolute_back().point(), end);
    }

    #[test]
    fn an_interior_point_near_but_not_equal_to_endpoint_remains_inner() {
        let start = Point3::new(0.0, 0.0, 0.0);
        let interior = Point3::new(1.0, 0.0, 0.0);
        let back = Point3::new(1.0, 8.6e-7, 0.0);
        let front = Vertex::new(start);
        let back_vertex = Vertex::new(back);
        let edge = Edge::new(
            &front,
            &back_vertex,
            PolylineCurve(vec![start, interior, back]),
        );
        let loops: Loops<Point3, PolylineCurve> = vec![BoundaryWire::new(
            vec![edge].into(),
            ShapesOpStatus::Unknown,
        )]
        .into_iter()
        .collect();
        assert!(matches!(
            loops.search_parameter(interior),
            Some((0, 0, ParameterKind::Inner(_)))
        ));
    }
}

#[cfg(test)]
mod shared_split_tests {
    use super::*;

    #[test]
    fn splitting_a_dangling_edge_replaces_both_occurrences() {
        let a = Vertex::new(Point3::origin());
        let b = Vertex::new(Point3::new(1.0, 0.0, 0.0));
        let edge = Edge::new(&a, &b, PolylineCurve::from(vec![a.point(), b.point()]));
        let mut loops = Loops(vec![BoundaryWire::new(vec![edge.clone(), edge.inverse()].into(), ShapesOpStatus::Unknown)]);
        let middle = Vertex::new(Point3::new(0.5, 0.0, 0.0));
        let (left, right) = edge.cut_with_parameter(&middle, 0.5).unwrap();
        loops.swap_edge_into_wire(edge.id(), &vec![left.clone(), right.clone()].into());
        assert_eq!(loops[0].iter().cloned().collect::<Vec<_>>(), vec![left.clone(), right.clone(), right.inverse(), left.inverse()]);
        assert!(loops[0].iter().zip(loops[0].iter().cycle().skip(1)).all(|(a, b)| a.back() == b.front()));
    }

    #[test]
    fn shared_geometric_split_preserves_reversed_child_identity() {
        let a = Vertex::new(Point3::origin());
        let b = Vertex::new(Point3::new(1.0, 0.0, 0.0));
        let edge = Edge::new(&a, &b, Line(a.point(), b.point()));
        let mut first = LoopsStore(vec![Loops(vec![BoundaryWire::new(vec![edge].into(), ShapesOpStatus::Unknown)])]);
        let mut second = first.clone();
        second[0][0] = second[0][0].inverse();
        let middle = Vertex::new(Point3::new(0.5, 0.0, 0.0));
        let plane = Plane::new(Point3::origin(), Point3::new(1.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0));
        let mut replacements = HashMap::default();
        let mut cuts = HashMap::default();
        for store in [&mut first, &mut second] {
            store.add_geom_vertex((0, 0, 0), &middle, ParameterKind::Inner(0.5), &plane,
                &mut replacements, &mut cuts).unwrap();
        }
        assert_eq!(first[0][0].iter().cloned().collect::<Vec<_>>(), second[0][0].iter().rev().map(Edge::inverse).collect::<Vec<_>>());
    }

    #[test]
    fn shared_boundary_split_preserves_child_identity_across_operands() {
        let a = Vertex::new(Point3::origin());
        let b = Vertex::new(Point3::new(1.0, 0.0, 0.0));
        let edge = Edge::new(&a, &b, PolylineCurve::from(vec![a.point(), b.point()]));
        let mut first = LoopsStore(vec![Loops(vec![BoundaryWire::new(vec![edge].into(), ShapesOpStatus::Unknown)])]);
        let mut second = first.clone();
        second[0][0] = second[0][0].inverse();
        let middle = Vertex::new(Point3::new(0.5, 0.0, 0.0));
        let quarter = Vertex::new(Point3::new(0.25, 0.0, 0.0));
        let mut replacements = HashMap::default();
        let mut cuts = HashMap::default();
        let mut quarter_cuts = HashMap::default();
        first.add_polygon_vertex(0, &middle, &mut replacements, &mut cuts).unwrap().unwrap();
        first.add_polygon_vertex(0, &quarter, &mut replacements, &mut quarter_cuts).unwrap().unwrap();
        second.add_polygon_vertex(0, &middle, &mut replacements, &mut cuts).unwrap().unwrap();
        second.add_polygon_vertex(0, &quarter, &mut replacements, &mut quarter_cuts).unwrap().unwrap();
        assert_eq!(first[0][0].len(), 3);
        assert_eq!(first[0][0].iter().cloned().collect::<Vec<_>>(), second[0][0].iter().rev().map(Edge::inverse).collect::<Vec<_>>());
    }
}

#[cfg(test)]
mod conflicting_split_tests {
    use super::*;

    fn retraced_split() -> (Edge<Point3, PolylineCurve>, Vertex<Point3>, EdgeCuts<Point3, PolylineCurve>) {
        let a = Vertex::new(Point3::new(-1.0, 0.0, 0.0));
        let b = Vertex::new(Point3::new(1.0, 0.0, 0.0));
        let curve = PolylineCurve::from(vec![a.point(), b.point(), a.point(), b.point()]);
        let middle = Vertex::new(Point3::origin());
        let chosen = curve.search_parameter(middle.point(), None, 100).unwrap();
        let other = if chosen == 0.5 { 2.5 } else { 0.5 };
        let edge = Edge::new(&a, &b, curve);
        let (left, right) = edge.cut_with_parameter(&middle, other).unwrap();
        let mut cuts = HashMap::default();
        cuts.insert((edge.id(), middle.id()), (edge.clone(), other, vec![left, right].into()));
        (edge, middle, cuts)
    }

    #[test]
    fn polygon_rejects_a_cached_different_parameter_branch_without_mutation() {
        let (edge, middle, mut cuts) = retraced_split();
        let mut store = LoopsStore(vec![Loops(vec![BoundaryWire::new(vec![edge.clone()].into(), ShapesOpStatus::Unknown)])]);
        assert!(store.add_polygon_vertex(0, &middle, &mut HashMap::default(), &mut cuts).is_err());
        assert_eq!(store[0][0].iter().cloned().collect::<Vec<_>>(), vec![edge]);
    }

    #[test]
    fn cached_geometry_still_requires_the_second_surface() {
        let a = Vertex::new(Point3::origin());
        let b = Vertex::new(Point3::new(1.0, 0.0, 0.0));
        let middle = Vertex::new(Point3::new(0.5, 0.0, 0.0));
        let edge = Edge::new(&a, &b, Line(a.point(), b.point()));
        let (left, right) = edge.cut_with_parameter(&middle, 0.5).unwrap();
        let mut cuts = HashMap::default();
        cuts.insert((edge.id(), middle.id()), (edge.clone(), 0.5, vec![left, right].into()));
        let mut store = LoopsStore(vec![Loops(vec![BoundaryWire::new(vec![edge.clone()].into(), ShapesOpStatus::Unknown)])]);
        let plane = Plane::new(Point3::new(0.0, 0.0, 1.0), Point3::new(1.0, 0.0, 1.0), Point3::new(0.0, 1.0, 1.0));
        assert!(store.add_geom_vertex((0, 0, 0), &middle, ParameterKind::Inner(0.5), &plane, &mut HashMap::default(), &mut cuts).is_none());
        assert_eq!(store[0][0].iter().cloned().collect::<Vec<_>>(), vec![edge]);
        assert_eq!(middle.point(), Point3::new(0.5, 0.0, 0.0));
    }

    #[test]
    fn geometry_rejects_a_cached_different_parameter_branch_without_mutation() {
        let (edge, middle, mut cuts) = retraced_split();
        let mut store = LoopsStore(vec![Loops(vec![BoundaryWire::new(vec![edge.clone()].into(), ShapesOpStatus::Unknown)])]);
        let plane = Plane::new(Point3::origin(), Point3::new(1.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0));
        assert!(store.add_geom_vertex((0, 0, 0), &middle, ParameterKind::Inner(0.5), &plane, &mut HashMap::default(), &mut cuts).is_none());
        assert_eq!(store[0][0].iter().cloned().collect::<Vec<_>>(), vec![edge]);
    }
}


#[cfg(test)]
mod shared_event_projection_regression {
    use super::*;

    #[test]
    fn tolerance_near_circle_plane_event_is_polished_on_the_actual_curve() {
        let radius = (6.6_f64.powi(2) + 0.65_f64.powi(2)) / (2.0 * 0.65);
        let center = Point3::new(0.0, 0.0, 7.5 + radius - 0.65);
        let curve = Processor::with_transform(
            TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, 0.21)),
            Matrix4::from_cols(
                Vector4::new(0.0, 0.0, -radius, 0.0),
                Vector4::new(radius, 0.0, 0.0, 0.0),
                Vector4::new(0.0, radius, 0.0, 0.0),
                center.to_homogeneous(),
            ),
        );
        let plane = Plane::new(
            Point3::new(0.0, 0.0, 7.5),
            Point3::new(1.0, 0.0, 7.5),
            Point3::new(0.0, 1.0, 7.5),
        );
        let seed = Point3::new(6.59999896945795, 0.0, 7.5);
        let (point, t, uv) =
            curve_surface_projection(&curve, None, &plane, None, seed, 100).unwrap();
        assert!(
            (point - curve.evaluate(t)).magnitude() < 1e-12,
            "shared event must be on the curve, not merely tolerance near it"
        );
        assert!((point - plane.evaluate(uv.x, uv.y)).magnitude() < 1e-12);
        assert!((point - Point3::new(6.6, 0.0, 7.5)).magnitude() < 1e-11);
    }
}

#[cfg(test)]
mod polished_endpoint_attachment_regression {
    use super::*;
    use monstertruck_modeling::{Curve, Surface, ToSameGeometry};

    #[test]
    fn polishing_the_second_operand_revisits_the_first_outline_and_shared_side_edge() {
        for (offset, reversed) in [(0.0, false), (0.0, true), (-0.01, false), (-0.01, true)] {
            let radius = (6.6_f64.powi(2) + 0.65_f64.powi(2)) / (2.0 * 0.65);
            let center = Point3::new(offset, 0.0, 7.5 + radius - 0.65);
            let meridian = Processor::with_transform(
                TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, 0.21)),
                Matrix4::from_cols(
                    Vector4::new(0.0, 0.0, -radius, 0.0),
                    Vector4::new(radius, 0.0, 0.0, 0.0),
                    Vector4::new(0.0, radius, 0.0, 0.0),
                    center.to_homogeneous(),
                ),
            );
            let plane = Plane::new(
                Point3::new(0.0, 0.0, 7.5),
                Point3::new(1.0, 0.0, 7.5),
                Point3::new(0.0, 1.0, 7.5),
            );
            let sphere = Sphere::new(center, radius);
            let surfaces: [Surface; 2] = [
                plane.into(),
                Surface::SphericalSurface(Processor::new(sphere)),
            ];
            let a = Vertex::new(Point3::new(6.6, -5.8, 7.5));
            let b = Vertex::new(Point3::new(6.6, 5.8, 7.5));
            let geometry_edge = Edge::new(&a, &b, Curve::Line(Line(a.point(), b.point())));
            let polygon_edge = Edge::new(&a, &b, PolylineCurve::from(vec![a.point(), b.point()]));
            let mut geometry0 = LoopsStore(vec![
                Loops(vec![BoundaryWire::new(
                    vec![geometry_edge.clone()].into(),
                    ShapesOpStatus::Unknown,
                )]),
                Loops(vec![BoundaryWire::new(
                    vec![geometry_edge.inverse()].into(),
                    ShapesOpStatus::Unknown,
                )]),
            ]);
            let mut polygon0 = LoopsStore(vec![
                Loops(vec![BoundaryWire::new(
                    vec![polygon_edge.clone()].into(),
                    ShapesOpStatus::Unknown,
                )]),
                Loops(vec![BoundaryWire::new(
                    vec![polygon_edge.inverse()].into(),
                    ShapesOpStatus::Unknown,
                )]),
            ]);
            let c = Vertex::new(meridian.evaluate(0.0));
            let d = Vertex::new(meridian.evaluate(0.21));
            let seed = Point3::new(offset + 6.59999896945795, 0.0, 7.5);
            let mut geometry1 = LoopsStore(vec![Loops(vec![BoundaryWire::new(
                vec![Edge::new(&c, &d, meridian.to_same_geometry())].into(),
                ShapesOpStatus::Unknown,
            )])]);
            let mut polygon1 = LoopsStore(vec![Loops(vec![BoundaryWire::new(
                vec![Edge::new(
                    &c,
                    &d,
                    PolylineCurve::from(vec![c.point(), seed, d.point()]),
                )]
                .into(),
                ShapesOpStatus::Unknown,
            )])]);
            assert!(
                polygon0[0].search_parameter(seed).is_none(),
                "raw leader seed must miss the first boundary"
            );
            let pv = Vertex::new(seed);
            let gv = Vertex::new(seed);
            if reversed {
                attach_intersection_vertex(
                    [
                        (&mut geometry1, &mut polygon1),
                        (&mut geometry0, &mut polygon0),
                    ],
                    [0, 0],
                    [&surfaces[1], &surfaces[0]],
                    &pv,
                    &gv,
                )
                .unwrap();
            } else {
                attach_intersection_vertex(
                    [
                        (&mut geometry0, &mut polygon0),
                        (&mut geometry1, &mut polygon1),
                    ],
                    [0, 0],
                    [&surfaces[0], &surfaces[1]],
                    &pv,
                    &gv,
                )
                .unwrap();
            }
            assert!((gv.point() - Point3::new(offset + 6.6, 0.0, 7.5)).magnitude() < 1e-11);
            let expected = if offset == 0.0 { 2 } else { 1 };
            assert_eq!(
                geometry0[0][0].len(),
                expected,
                "split the polished contact, preserve the genuinely disjoint outline"
            );
            if expected == 2 {
                assert_eq!(geometry0[0][0][0].back(), &gv);
            }
            assert_eq!(
                geometry0[0][0].iter().cloned().collect::<Vec<_>>(),
                geometry0[1][0]
                    .iter()
                    .rev()
                    .map(Edge::inverse)
                    .collect::<Vec<_>>(),
                "side face must reuse reversed child IDs"
            );
            assert_eq!(polygon0[0][0].len(), expected);
            assert_eq!(pv.point(), gv.point());
        }
    }
}

#[cfg(test)]
mod optional_newton_fallback_regression {
    use super::*;
    #[derive(Clone)]
    struct SelectivePlane;
    impl ParametricSurface for SelectivePlane {
        type Point = Point3;
        type Vector = Vector3;
        fn evaluate(&self, u: f64, v: f64) -> Point3 {
            Point3::new(u, v, 0.)
        }
        fn derivative_u(&self, _: f64, _: f64) -> Vector3 {
            Vector3::unit_x()
        }
        fn derivative_v(&self, _: f64, _: f64) -> Vector3 {
            Vector3::unit_y()
        }
        fn derivative_uu(&self, _: f64, _: f64) -> Vector3 {
            Vector3::zero()
        }
        fn derivative_uv(&self, _: f64, _: f64) -> Vector3 {
            Vector3::zero()
        }
        fn derivative_vv(&self, _: f64, _: f64) -> Vector3 {
            Vector3::zero()
        }
        fn derivative_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Vector3 {
            Plane::xy().derivative_mn(m, n, u, v)
        }
    }
    impl ParametricSurface3D for SelectivePlane {
        fn normal(&self, _: f64, _: f64) -> Vector3 {
            Vector3::unit_z()
        }
    }
    impl SearchNearestParameter<SurfaceParameter> for SelectivePlane {
        type Point = Point3;
        fn search_nearest_parameter<H: Into<SearchParameterHint2D>>(
            &self,
            p: Point3,
            _: H,
            _: usize,
        ) -> Option<(f64, f64)> {
            // A valid surface solver may decline a new query instead of converging.
            (p.x <= 3e-7).then_some((p.x, p.y))
        }
    }
    #[test]
    fn optional_inverse_failure_keeps_the_verified_actual_curve_event() {
        let line = Line(
            Point3::new(0.0, 0.0, -4e-7),
            Point3::new(1.0, 0.0, 1.0 - 4e-7),
        );
        let seed = Point3::origin();
        let (point, t, uv) =
            curve_surface_projection(&line, None, &SelectivePlane, None, seed, 100)
                .expect("an optional polish must not discard the verified near event");
        assert_eq!(point, line.evaluate(t));
        assert!(point.near(&seed));
        assert!(point.near(&SelectivePlane.evaluate(uv.x, uv.y)));
        assert!(
            t < 3e-7,
            "fall back to the original curve parameter when corrected inverse declines"
        );
    }
}

#[cfg(test)]
mod contact_wire_partition_regression {
    use super::*;
    use itertools::Itertools;
    use monstertruck_modeling::{Solid, builder};

    fn line(a: &Vertex<Point3>, b: &Vertex<Point3>) -> Edge<Point3, Line<Point3>> {
        Edge::new(a, b, Line(a.point(), b.point()))
    }

    #[test]
    fn shared_contacts_partition_the_same_material_for_every_insertion_orientation() {
        for order in (0..4).permutations(4) {
            for reversed in [false, true] {
                for rotation in 0..8 {
                    for directions in 0..16 {
                        let vertices = Vertex::from_points(&[
                            Point3::new(1.0, 0.0, 0.0),
                            Point3::new(1.0, 1.0, 0.0),
                            Point3::new(0.0, 1.0, 0.0),
                            Point3::new(-1.0, 1.0, 0.0),
                            Point3::new(-1.0, 0.0, 0.0),
                            Point3::new(-1.0, -1.0, 0.0),
                            Point3::new(0.0, -1.0, 0.0),
                            Point3::new(1.0, -1.0, 0.0),
                        ]);
                        let mut boundary: Wire<_, _> = (0..8)
                            .map(|i| line(&vertices[i], &vertices[(i + 1) % 8]))
                            .collect();
                        if reversed {
                            boundary.invert();
                        }
                        boundary.rotate_left(rotation);
                        let mut loops =
                            Loops(vec![BoundaryWire::new(boundary, ShapesOpStatus::Unknown)]);
                        for &quarter in &order {
                            let backward = directions & (1 << quarter) != 0;
                            let mut edge =
                                line(&vertices[2 * quarter], &vertices[(2 * quarter + 2) % 8]);
                            if backward {
                                edge.invert();
                            }
                            let status = if backward == reversed {
                                ShapesOpStatus::And
                            } else {
                                ShapesOpStatus::Or
                            };
                            loops
                                .add_edge(edge, status)
                                .expect("unambiguous wire owner");
                        }
                        assert_eq!(
                            loops.len(),
                            5,
                            "order={order:?} reversed={reversed} rotation={rotation} directions={directions}"
                        );
                        let mut total_area = 0.0;
                        for wire in loops.iter() {
                            assert!(wire.is_closed());
                            let area: f64 = wire
                                .iter()
                                .map(|e| {
                                    let a = e.front().point();
                                    let b = e.back().point();
                                    (a.x * b.y - a.y * b.x) / 2.0
                                })
                                .sum();
                            total_area += area;
                            assert_eq!(area.is_sign_negative(), reversed);
                            if (area.abs() - 2.0).abs() < 1e-12 {
                                assert_eq!(wire.status(), ShapesOpStatus::And);
                            } else {
                                assert!((area.abs() - 0.5).abs() < 1e-12);
                                assert_eq!(wire.status(), ShapesOpStatus::Or);
                            }
                        }
                        assert!((total_area.abs() - 4.0).abs() < 1e-12);
                    }
                }
            }
        }
    }

    #[test]
    fn a_genuine_bridge_still_joins_distinct_boundary_wires() {
        let a = Vertex::from_points(&[
            Point3::new(-2.0, 0.0, 0.0),
            Point3::new(-1.0, 0.0, 0.0),
            Point3::new(-2.0, 1.0, 0.0),
        ]);
        let b = Vertex::from_points(&[
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(2.0, 1.0, 0.0),
        ]);
        let mut loops = Loops(vec![
            BoundaryWire::new(
                (0..3).map(|i| line(&a[i], &a[(i + 1) % 3])).collect(),
                ShapesOpStatus::Unknown,
            ),
            BoundaryWire::new(
                (0..3).map(|i| line(&b[i], &b[(i + 1) % 3])).collect(),
                ShapesOpStatus::Unknown,
            ),
        ]);
        let bridge = line(&a[1], &b[0]);
        let id = bridge.id();
        loops
            .add_edge(bridge, ShapesOpStatus::And)
            .expect("unambiguous wire owner");
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].len(), 8);
        assert!(loops[0].is_closed());
        assert_eq!(loops[0].iter().filter(|e| e.id() == id).count(), 2);
    }

    #[test]
    fn projected_directions_preserve_a_valid_divider_with_two_common_owners() {
        let vertices = Vertex::from_points(&[
            Point3::new(-1.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, -1.0, 0.0),
        ]);
        let edge = |a: usize, b: usize| {
            Edge::new(
                &vertices[a],
                &vertices[b],
                PolylineCurve::from(vec![vertices[a].point(), vertices[b].point()]),
            )
        };
        let mut loops = Loops(vec![
            BoundaryWire::new(
                vec![edge(0, 1), edge(1, 2), edge(2, 0)].into(),
                ShapesOpStatus::And,
            ),
            BoundaryWire::new(
                vec![edge(0, 3), edge(3, 1), edge(1, 0)].into(),
                ShapesOpStatus::Or,
            ),
        ]);
        let before: Vec<Vec<_>> = loops.iter().map(|w| w.iter().cloned().collect()).collect();
        let divider = Edge::new(
            &vertices[0],
            &vertices[1],
            PolylineCurve::from(vec![
                vertices[0].point(),
                Point3::new(0.0, 0.5, 0.0),
                vertices[1].point(),
            ]),
        );
        let mut geometry = loops.clone();
        add_paired_intersection_edge(
            &mut loops,
            &mut geometry,
            divider.clone(),
            divider,
            ShapesOpStatus::And,
            &Plane::xy(),
        )
        .expect("the divider enters the upper region");
        assert_eq!(loops.len(), 3);
        assert_eq!(geometry.len(), 3);
        assert_eq!(before[1], loops[1].iter().cloned().collect::<Vec<_>>());
        let total: f64 = loops
            .iter()
            .map(|w| {
                w.iter()
                    .map(|e| {
                        let a = e.front().point();
                        let b = e.back().point();
                        (a.x * b.y - a.y * b.x) / 2.0
                    })
                    .sum::<f64>()
            })
            .sum();
        assert!((total - 2.0).abs() < 1e-12);
        assert!(loops.iter().all(|w| w.is_closed()));
        assert_eq!(
            loops
                .iter()
                .map(|w| w.iter().cloned().collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            geometry
                .iter()
                .map(|w| w.iter().cloned().collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn unresolved_coincident_branches_refuse_without_changing_material() {
        let vertices = Vertex::from_points(&[
            Point3::new(-1.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ]);
        let edge = |a: usize, b: usize| {
            Edge::new(
                &vertices[a],
                &vertices[b],
                PolylineCurve::from(vec![vertices[a].point(), vertices[b].point()]),
            )
        };
        let wire: Wire<_, _> = vec![edge(0, 1), edge(1, 2), edge(2, 0)].into();
        let mut loops = Loops(vec![
            BoundaryWire::new(wire.clone(), ShapesOpStatus::And),
            BoundaryWire::new(wire, ShapesOpStatus::Or),
        ]);
        let before: Vec<Vec<_>> = loops.iter().map(|w| w.iter().cloned().collect()).collect();
        let mut geometry = loops.clone();
        let divider = Edge::new(
            &vertices[0],
            &vertices[1],
            PolylineCurve::from(vec![
                vertices[0].point(),
                Point3::new(0.0, 0.5, 0.0),
                vertices[1].point(),
            ]),
        );
        assert!(
            add_paired_intersection_edge(
                &mut loops,
                &mut geometry,
                divider.clone(),
                divider,
                ShapesOpStatus::And,
                &Plane::xy()
            )
            .is_none()
        );
        assert_eq!(
            before,
            loops
                .iter()
                .map(|w| w.iter().cloned().collect())
                .collect::<Vec<Vec<_>>>()
        );
        assert_eq!(
            before,
            geometry
                .iter()
                .map(|w| w.iter().cloned().collect())
                .collect::<Vec<Vec<_>>>()
        );
    }

    #[test]
    fn public_difference_preserves_four_corner_regions_at_boundary_contacts() {
        let vertex = builder::vertex(Point3::new(-1.0, -1.0, -1.0));
        let edge = builder::extrude(&vertex, Vector3::new(2.0, 0.0, 0.0));
        let face = builder::extrude(&edge, Vector3::new(0.0, 2.0, 0.0));
        let cube: Solid = builder::extrude(&face, Vector3::new(0.0, 0.0, 2.0));
        // Tapering gives isolated top-plane contacts; a vertical prism would
        // instead create nonmanifold contact lines along the cube side walls.
        let diamond = |z: f64| -> monstertruck_modeling::Wire {
            let vertices = Vertex::from_points(&[
                Point3::new(z, 0.0, z),
                Point3::new(0.0, z, z),
                Point3::new(-z, 0.0, z),
                Point3::new(0.0, -z, z),
            ]);
            (0..4)
                .map(|i| builder::line(&vertices[i], &vertices[(i + 1) % 4]))
                .collect()
        };
        let bottom = diamond(0.5);
        let top = diamond(2.0);
        let mut shell: monstertruck_modeling::Shell = builder::try_wire_homotopy::<
            monstertruck_modeling::Curve,
            monstertruck_modeling::Surface,
        >(&bottom, &top)
        .unwrap();
        shell.push(builder::try_attach_plane(vec![bottom]).unwrap().inverse());
        shell.push(builder::try_attach_plane(vec![top]).unwrap());
        let cutter = Solid::try_new(vec![shell]).unwrap();
        let result = crate::difference(&cube, &cutter, 1e-6)
            .expect("all four corner regions share the correct contact ownership");
        assert!(result.is_geometric_consistent());
        assert!(
            result
                .boundaries()
                .iter()
                .all(|shell| shell.shell_condition() == ShellCondition::Closed)
        );
        let mesh = result.triangulation(0.05).to_polygon();
        // Removed diamond sections have area2*z², integrated fromz=0.5to1.
        assert!((mesh.volume() - (8.0 - 7.0 / 12.0)).abs() < 1e-10);
        assert_eq!(
            result
                .face_iter()
                .filter(|face| face
                    .absolute_boundaries()
                    .iter()
                    .flat_map(|w| w.vertex_iter())
                    .all(|v| (v.point().z - 1.0).abs() < 1e-12))
                .count(),
            4
        );
    }
}

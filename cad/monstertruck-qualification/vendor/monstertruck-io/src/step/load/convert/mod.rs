use super::report::{LossCategory as Cat, LossReason as Why, LossSink};
use super::step_geometry::{
    SurfaceCurve3D as StepSurfaceCurve3D,
    SurfaceCurveAssociatedGeometry as StepSurfaceCurveAssociatedGeometry,
    SurfaceCurveKind as StepSurfaceCurveKind,
};
use super::*;
use monstertruck_topology::compress::*;
use std::collections::HashSet;

/// A compressed shell in the STEP loader's concrete geometry types.
///
/// The `*_reported` conversions (spec 011 T7) return the value paired with a
/// [`ShellLoadReport`], and spelling the four generic arguments inline at every
/// such signature is what tripped `clippy::type_complexity`. Naming the three
/// concrete instantiations once keeps the reported and unreported variants
/// visibly the same type.
pub type StepCompressedShell = CompressedShell<Point3, Curve3D, Surface>;

/// A compressed trimmed shell (exact trim curves preserved) in the STEP
/// loader's concrete geometry types. See [`StepCompressedShell`].
pub type StepCompressedTrimmedShell =
    CompressedTrimmedShell<Point3, Curve3D, Surface, step_geometry::StepParameterCurve>;

/// A compressed solid in the STEP loader's concrete geometry types.
/// See [`StepCompressedShell`].
pub type StepCompressedSolid = CompressedSolid<Point3, Curve3D, Surface>;

type SourceMappedShell = (StepCompressedShell, ShellLoadReport, Vec<Option<u64>>);
type SourceMappedSolid = (
    StepCompressedSolid,
    Vec<ShellLoadReport>,
    Vec<Vec<Option<u64>>>,
);

#[derive(Clone, Copy)]
enum ConnectivitySelection {
    FromSource(Option<u64>),
    Disabled,
}

/// An immutable, lossless source-space shell and its conversion-time face binding.
/// Only checked table conversion can construct this value. Transforming a clone
/// from `shell()` cannot transform or reuse this value's certificates.
#[derive(Debug)]
pub struct SourceBoundStepShell {
    shell: StepCompressedShell,
    source_face_ids: Vec<u64>,
    certified_closed_axes: Vec<Option<[bool; 2]>>,
    source_surfaces: Vec<SurfaceAny>,
    connectivity_attachments: Vec<report::ConnectivityAttachment>,
}
impl SourceBoundStepShell {
    /// Direct source attachments; their discrepancy is separate from mesh error.
    pub fn connectivity_attachments(&self) -> &[report::ConnectivityAttachment] {
        &self.connectivity_attachments
    }
    /// Rechecks every direct attachment on the exact normalized source image.
    /// The result is an attachment budget, separate from tessellation error.
    pub fn connectivity_after_uniform_scale(
        &self,
        scaled: &StepCompressedShell,
        factor: f64,
    ) -> Option<Vec<report::ScaledConnectivityAttachment>> {
        if !factor.is_finite()
            || factor <= 0.0
            || self
                .connectivity_attachments
                .iter()
                .any(|a| a.accuracy.model_mm_per_unit() != factor)
        {
            return None;
        }
        let mut expected = self.shell.clone();
        context::normalization::normalize_shell_to_mm(&mut expected, factor);
        if expected != *scaled {
            return None;
        }
        self.connectivity_attachments
            .iter()
            .map(|attachment| {
                let edge = scaled.edges.get(attachment.edge_index)?;
                let endpoint = if attachment.back {
                    edge.curve.back()
                } else {
                    edge.curve.front()
                };
                let vertex = *scaled.vertices.as_slice().get(attachment.vertex_index)?;
                let mut face_uv = None;
                let (point, attached) = match attachment.kind {
                    report::ConnectivityIncidence::VertexOnEdge => (vertex, endpoint),
                    report::ConnectivityIncidence::VertexOnFace
                    | report::ConnectivityIncidence::EdgeEndpointOnFace => {
                        let face = &scaled.faces.get(attachment.face_index?)?.surface;
                        let (mut u, mut v) = attachment.uv?;
                        if matches!(
                            face,
                            Surface::ElementarySurface(ElementarySurface::Plane(_))
                        ) {
                            u *= factor;
                            v *= factor;
                        }
                        face_uv = Some((u, v));
                        let point =
                            if attachment.kind == report::ConnectivityIncidence::VertexOnFace {
                                vertex
                            } else {
                                endpoint
                            };
                        (point, face.subs(u, v))
                    }
                };
                let discrepancy_mm = point.distance(attached);
                let bound_mm = attachment.accuracy.bound() * factor;
                if !bound_mm.is_finite()
                    || bound_mm <= 0.0
                    || !bound_mm.powi(2).is_finite()
                    || bound_mm.powi(2) == 0.0
                    || !discrepancy_mm.is_finite()
                    || discrepancy_mm > bound_mm
                {
                    return None;
                }
                Some(report::ScaledConnectivityAttachment {
                    source: attachment.clone(),
                    discrepancy_mm,
                    bound_mm,
                    declared_bound_mm: attachment.accuracy.declared_bound() * factor,
                    point_mm: point,
                    witness_mm: attached,
                    face_uv,
                })
            })
            .collect()
    }

    /// Exact converted shell before unit or occurrence transforms.
    pub fn shell(&self) -> &StepCompressedShell {
        &self.shell
    }
    /// Source FACE_SURFACE IDs emitted alongside kept converted faces.
    pub fn source_face_ids(&self) -> &[u64] {
        &self.source_face_ids
    }
    /// Finite boundary equality, not periodicity or trim-domain coverage.
    pub fn certified_closed_axes(&self) -> &[Option<[bool; 2]>] {
        &self.certified_closed_axes
    }
    /// Re-proves closure on an exact uniform-scale image of this source face.
    /// The source and scaled surfaces must both match; a stale certificate is not transported.
    pub fn certified_closed_axes_after_uniform_scale(
        &self,
        index: usize,
        scaled: &Surface,
        factor: f64,
    ) -> Option<[bool; 2]> {
        self.source_surfaces
            .get(index)?
            .certified_closed_axes_after_uniform_scale(
                &self.shell.faces.get(index)?.surface,
                scaled,
                factor,
            )
    }
}
/// Lossless shells retaining the outer/void order, in source coordinates.
#[derive(Debug)]
pub struct SourceBoundStepSolid {
    shells: Vec<SourceBoundStepShell>,
}
impl SourceBoundStepSolid {
    pub fn shells(&self) -> &[SourceBoundStepShell] {
        &self.shells
    }
}

impl Table {
    fn compressed_shell_with_sources_reported(
        &self,
        shell: &ShellHolder,
    ) -> Result<SourceMappedShell, StepConvertingError> {
        self.compressed_shell_in_representation_reported(shell, None)
    }
    fn compressed_shell_in_representation_reported(
        &self,
        shell: &ShellHolder,
        representation: Option<u64>,
    ) -> Result<SourceMappedShell, StepConvertingError> {
        self.compressed_shell_with_connectivity_reported(
            shell,
            ConnectivitySelection::FromSource(representation),
        )
    }
    fn compressed_shell_with_connectivity_reported(
        &self,
        shell: &ShellHolder,
        selection: ConnectivitySelection,
    ) -> Result<SourceMappedShell, StepConvertingError> {
        let sink = LossSink::default();
        let connectivity = match selection {
            ConnectivitySelection::FromSource(representation) => self
                .source_records
                .shell_connectivity(self, shell, representation),
            ConnectivitySelection::Disabled => Ok(None),
        };
        let (vertices, vidx_map) = self.shell_vertices(shell, &sink);
        let (edges, eidx_map) = self.shell_edges(shell, &vidx_map, &sink, &connectivity);
        let (faces, source_ids) =
            self.shell_faces(shell, &eidx_map, &vidx_map, &edges, &sink, &connectivity);
        Ok((
            CompressedShell {
                vertices,
                edges,
                faces,
                vertex_stable_ids: None,
                edge_stable_ids: None,
                face_stable_ids: None,
            },
            sink.into_report(),
            source_ids,
        ))
    }
    fn source_mapped_shell_at(
        &self,
        id: u64,
        selection: ConnectivitySelection,
    ) -> Result<SourceMappedShell, LoadError> {
        if let Some(shell) = self.shell.get(&id) {
            return Ok(self.compressed_shell_with_connectivity_reported(shell, selection)?);
        }
        let oriented = self
            .oriented_shell
            .get(&id)
            .ok_or("failed to reference shell")?;
        let inner = Self::ref_id(&oriented.shell_element).ok_or("failed to reference shell")?;
        let shell = self.shell.get(&inner).ok_or("failed to reference shell")?;
        let (mut converted, report, ids) =
            self.compressed_shell_with_connectivity_reported(shell, selection)?;
        if !oriented.orientation {
            for face in &mut converted.faces {
                face.orientation = !face.orientation;
            }
        }
        Ok((converted, report, ids))
    }
    fn place_holder_edge_any_to_index_and_edge_curve(
        &self,
        edge: &PlaceHolder<EdgeAnyHolder>,
    ) -> Option<(u64, EdgeCurveHolder)> {
        use PlaceHolder::Ref;
        let Ref(Name::Entity(idx)) = edge else {
            return None;
        };
        self.oriented_edge
            .get(idx)
            .and_then(|oriented_edge| {
                Some((
                    oriented_edge.edge_element_idx()?,
                    oriented_edge.edge_element_holder(self)?,
                ))
            })
            .or_else(|| {
                self.edge_curve
                    .get(idx)
                    .map(|edge_curve| (*idx, edge_curve.clone()))
            })
    }
    fn face_any_to_orientation_and_face(
        &self,
        face: Option<FaceAnyHolder>,
    ) -> Option<(bool, FaceSurfaceHolder)> {
        match face? {
            FaceAnyHolder::FaceSurface(face) => Some((true, face)),
            FaceAnyHolder::OrientedFace(oriented_face) => {
                let face_element = oriented_face.face_element_holder(self)?;
                Some((oriented_face.orientation, face_element))
            }
        }
    }

    /// The entity id behind a reference place-holder, when it has one.
    pub(super) fn ref_id<T>(place_holder: &PlaceHolder<T>) -> Option<u64> {
        if let PlaceHolder::Ref(Name::Entity(idx)) = place_holder {
            Some(*idx)
        } else {
            None
        }
    }

    /// Resolve a face bound to the `EDGE_LOOP` the loader can represent, and
    /// REPORT the ones it cannot.
    ///
    /// This is the site of the census's only genuinely silent drop: the three
    /// non-`EDGE_LOOP` outcomes used to be one `?` on an `Option`, so a wire
    /// vanished with no error, no message and no count.
    fn resolved_edge_loop(
        &self,
        bound: &FaceBoundHolder,
        sink: &LossSink,
    ) -> Option<EdgeLoopHolder> {
        match bound.resolve_bound(self) {
            BoundResolution::EdgeLoop(edge_loop) => Some(edge_loop),
            BoundResolution::VertexLoop(id) => {
                sink.lost(Cat::Wire, Why::DegenerateVertexLoop, Some(id), None);
                None
            }
            BoundResolution::Unresolved(id) => {
                // A record that IS in the file but has no arm lands in
                // `Table::dummy` with its name intact, which is a different
                // defect from an id that resolves to nothing at all.
                let unrecognised = id.and_then(|id| self.dummy.get(&id));
                let (why, detail) = match unrecognised {
                    Some(dummy) => (
                        Why::BoundNotALoopWeImplement,
                        Some(dummy.record.chars().take(80).collect::<String>()),
                    ),
                    None => (Why::BoundUnresolved, None),
                };
                sink.lost(Cat::Wire, why, id, detail);
                None
            }
        }
    }

    fn shell_vertices(
        &self,
        shell: &ShellHolder,
        sink: &LossSink,
    ) -> (Vec<Point3>, HashMap<u64, usize>) {
        use PlaceHolder::Ref;
        let mut vidx_map = HashMap::<u64, usize>::new();
        // Distinct ids we have already TRIED, as opposed to ids we succeeded on.
        //
        // The dedup guard used to be `!vidx_map.contains_key(&idx)` with the
        // insert done BEFORE the fallible resolve, so a vertex that failed still
        // consumed an index and every later vertex's `vidx_map` entry pointed one
        // slot past its own point -- silently wrong topology, not merely lossy.
        // Keeping "attempted" separate from "kept" makes the index dense over
        // successes; when nothing fails the numbering is identical to before.
        let mut attempted = HashSet::<u64>::new();
        let vertex_to_point = |v: PlaceHolder<VertexPointHolder>| {
            let Ref(Name::Entity(idx)) = v else {
                sink.listed(Cat::Vertex, 1);
                sink.lost(
                    Cat::Vertex,
                    Why::VertexUnresolved,
                    None,
                    Some("the vertex is not an entity reference".to_owned()),
                );
                return None;
            };
            if !attempted.insert(idx) {
                return None;
            }
            sink.listed(Cat::Vertex, 1);
            match EntityTable::<VertexPointHolder>::get_owned(self, idx) {
                Ok(vertex) => {
                    let index = vidx_map.len();
                    vidx_map.insert(idx, index);
                    sink.kept(Cat::Vertex, 1);
                    Some(Point3::from(&vertex.vertex_geometry))
                }
                Err(e) => {
                    eprintln!("{e}");
                    sink.lost(
                        Cat::Vertex,
                        Why::VertexUnresolved,
                        Some(idx),
                        Some(e.to_string()),
                    );
                    None
                }
            }
        };
        let vertices: Vec<Point3> = shell
            .cfs_faces_holder(self)
            .filter_map(move |face| self.face_any_to_orientation_and_face(face))
            .flat_map(move |(_, face)| face.bounds_holder(self))
            // Wires are counted ONCE, in `shell_faces`/`shell_trimmed_faces`.
            // This walk only needs the edges, so it uses the non-reporting
            // resolver and cannot double-count a lost vertex loop.
            .filter_map(move |(_, bound)| bound?.bound_holder(self))
            .flat_map(move |bound| bound.edge_list)
            .filter_map(move |edge| self.place_holder_edge_any_to_index_and_edge_curve(&edge))
            .flat_map(move |(_, edge)| [edge.edge_start, edge.edge_end])
            .filter_map(vertex_to_point)
            .collect();
        (vertices, vidx_map)
    }

    fn shell_edges(
        &self,
        shell: &ShellHolder,
        vidx_map: &HashMap<u64, usize>,
        sink: &LossSink,
        connectivity: &Result<Option<context::ShellConnectivity>, String>,
    ) -> (Vec<CompressedEdge<Curve3D>>, HashMap<u64, usize>) {
        use PlaceHolder::Ref;
        let mut eidx_map = HashMap::<u64, usize>::new();
        // Same split as `shell_vertices`, for the same reason: a failed edge used
        // to consume an index and shift every later edge's `eidx_map` entry.
        let mut attempted = HashSet::<u64>::new();
        let edge_curve_to_compressed_edge = |(idx, edge): (u64, EdgeCurveHolder)| {
            if !attempted.insert(idx) {
                return None;
            }
            sink.listed(Cat::Edge, 1);
            let edge_curve = match edge.clone().into_owned(self) {
                Ok(edge_curve) => edge_curve,
                Err(e) => {
                    eprintln!("{e}");
                    sink.lost(
                        Cat::Edge,
                        Why::EdgeUnresolved,
                        Some(idx),
                        Some(e.to_string()),
                    );
                    return None;
                }
            };
            let accuracy = match connectivity {
                Ok(policy) => policy
                    .as_ref()
                    .and_then(|p| p.edges.get(&idx))
                    .and_then(Option::as_ref),
                Err(error) => {
                    sink.lost(
                        Cat::Edge,
                        Why::SourceConnectivityRefused,
                        Some(idx),
                        Some(error.clone()),
                    );
                    return None;
                }
            };
            let preview = self.preview_tolerance.is_some();
            let parsed = accuracy.map_or_else(
                || {
                    if preview {
                        edge_curve.parse_curve3d_preview()
                    } else {
                        edge_curve.parse_curve3d()
                    }
                },
                |bound| edge_curve.parse_curve3d_with_connectivity(bound.bound(), preview),
            );
            let curve = match parsed {
                Ok(curve) => curve,
                Err(e) => {
                    eprintln!("{e}");
                    sink.lost(
                        Cat::Edge,
                        Why::EdgeCurveRefused,
                        Some(idx),
                        Some(e.to_string()),
                    );
                    return None;
                }
            };
            if let (CurveAny::SurfaceCurve(source), Curve3D::SurfaceCurve(trimmed)) =
                (&edge_curve.edge_geometry, &curve)
                && source.selected_master() != trimmed.master_representation()
            {
                sink.alternate_master(
                    idx,
                    format!(
                        "selected {:?} does not reach the vertices; attached through {:?}",
                        source.selected_master(),
                        trimmed.master_representation(),
                    ),
                );
            }
            let (Ref(Name::Entity(front_idx)), Ref(Name::Entity(back_idx))) =
                (&edge.edge_start, &edge.edge_end)
            else {
                sink.lost(
                    Cat::Edge,
                    Why::EdgeEndpointUnresolved,
                    Some(idx),
                    Some("edge_start or edge_end is not an entity reference".to_owned()),
                );
                return None;
            };
            let (Some(front), Some(back)) = (vidx_map.get(front_idx), vidx_map.get(back_idx))
            else {
                sink.lost(
                    Cat::Edge,
                    Why::EdgeEndpointUnresolved,
                    Some(idx),
                    Some(format!("no kept vertex for #{front_idx} or #{back_idx}",)),
                );
                return None;
            };
            let index = eidx_map.len();
            if let Some(accuracy) = accuracy {
                for (back, vertex_id, vertex_index, point, endpoint) in [
                    (
                        false,
                        *front_idx,
                        *front,
                        Point3::from(&edge_curve.edge_start.vertex_geometry),
                        curve.front(),
                    ),
                    (
                        true,
                        *back_idx,
                        *back,
                        Point3::from(&edge_curve.edge_end.vertex_geometry),
                        curve.back(),
                    ),
                ] {
                    sink.attachment(report::ConnectivityAttachment {
                        kind: report::ConnectivityIncidence::VertexOnEdge,
                        edge_index: index,
                        vertex_index,
                        face_index: None,
                        back,
                        uv: None,
                        edge_id: idx,
                        vertex_id,
                        face_id: None,
                        discrepancy: endpoint.distance(point),
                        curve_interval: curve.range_tuple(),
                        accuracy: accuracy.clone(),
                    });
                }
            }
            eidx_map.insert(idx, index);
            sink.kept(Cat::Edge, 1);
            Some(CompressedEdge {
                vertices: (*front, *back),
                curve,
            })
        };
        let edges: Vec<CompressedEdge<Curve3D>> = shell
            .cfs_faces_holder(self)
            .filter_map(move |face| self.face_any_to_orientation_and_face(face))
            .flat_map(move |(_, face)| face.bounds_holder(self))
            // Wires are counted ONCE, in `shell_faces`/`shell_trimmed_faces`.
            // This walk only needs the edges, so it uses the non-reporting
            // resolver and cannot double-count a lost vertex loop.
            .filter_map(move |(_, bound)| bound?.bound_holder(self))
            .flat_map(move |bound| bound.edge_list)
            .filter_map(move |edge| self.place_holder_edge_any_to_index_and_edge_curve(&edge))
            .filter_map(edge_curve_to_compressed_edge)
            .collect();
        (edges, eidx_map)
    }
    /// Composes a `FACE_BOUND`'s own orientation with the face's `same_sense`,
    /// so a boundary loop is emitted in the SURFACE's sense and not the FACE's.
    ///
    /// # Why this composition exists (ledger C15, spec 013 V1)
    ///
    /// ISO 10303-42 orients an `ADVANCED_FACE`'s loops about the FACE normal,
    /// and the face normal is the surface normal only when `same_sense` is
    /// `.T.`. `CompressedFace`/`CompressedTrimmedFace` want the other
    /// convention: `CompressDirector::create_cface` stores `Face::boundaries`
    /// -- the ABSOLUTE boundaries, in the surface's own sense -- alongside the
    /// orientation flag, and `CompressedFace::create_face` rebuilds with
    /// `Face::try_new(stored, surface)` followed by `invert()`, which flips the
    /// EFFECTIVE traversal a second time.
    ///
    /// Passing STEP's loops through verbatim therefore traverses every
    /// `same_sense = .F.` face backwards relative to its neighbours. Measured
    /// on `occt-cube.step` (2026-08-01): `#17`, `#237` and `#331` are `.F.`
    /// against `#137`, `#284`, `#338` `.T.`, and the loaded shell came out
    /// `ShellCondition::Regular` -- all twelve edges traversed the SAME way by
    /// both their faces, none of them opposite.
    ///
    /// `normalize_trimmed_shell_orientation` then repaired that topologically,
    /// by 2-colouring from face 0. That is correct as far as it goes and its
    /// doc is explicit that it "does NOT decide global outwardness" -- but face
    /// 0 of the cube is `#17`, a `.F.` face, so the whole shell landed INWARD:
    /// six `false` flags and a divergence-theorem volume of **-1000** on a
    /// 1000-volume cube. That number is what `OperandVolume::trusted` (`> 0`,
    /// `<=` a CERTIFIED bounding box since spec 014 W2 -- the vertex hull it
    /// used before was not one) and `verify_volume_conservation` bound booleans
    /// with.
    ///
    /// The reversal machinery this reuses is the one already here for
    /// `FACE_BOUND.orientation`: flip each use, reverse the loop, invert the
    /// exact trim, and pick the other seam curve. Composing is all that was
    /// missing.
    #[inline]
    fn absolute_bound_orientation(bound: &FaceBoundHolder, face_sense: bool) -> bool {
        bound.orientation == face_sense
    }

    /// A `FACE_BOUND`'s loop, in the SURFACE's own sense.
    ///
    /// `face_sense` is the value that will be stored as the compressed face's
    /// `orientation`, and it belongs here as well as on the face: see
    /// [`Self::absolute_bound_orientation`].
    fn face_bound_to_edges(
        &self,
        bound: FaceBoundHolder,
        face_sense: bool,
        face_surface: &Surface,
        allow_single_sphere_pole: bool,
        eidx_map: &HashMap<u64, usize>,
        sink: &LossSink,
    ) -> Option<Vec<CompressedEdgeIndex>> {
        use PlaceHolder::Ref;
        let ori = Self::absolute_bound_orientation(&bound, face_sense);
        if allow_single_sphere_pole
            && let BoundResolution::VertexLoop(id) = bound.resolve_bound(self)
            && self.vertex_loop_is_spherical_pole(id, face_surface)
        {
            // A VERTEX_LOOP at a full sphere's parameter pole is a legitimate
            // STEP boundary, but has no edge use in the compressed topology.
            sink.listed(Cat::EdgeUse, 0);
            sink.kept(Cat::EdgeUse, 0);
            return Some(Vec::new());
        }
        let bound = self.resolved_edge_loop(&bound, sink)?;
        sink.listed(Cat::EdgeUse, bound.edge_list.len());
        let mut edges: Vec<CompressedEdgeIndex> = bound
            .edge_list
            .into_iter()
            .filter_map(|edge| {
                let Ref(Name::Entity(idx)) = edge else {
                    sink.lost(
                        Cat::EdgeUse,
                        Why::EdgeUseUnresolved,
                        None,
                        Some("the edge use is not an entity reference".to_owned()),
                    );
                    return None;
                };
                let edge_idx = if let Some(oriented_edge) = self.oriented_edge.get(&idx) {
                    let element = oriented_edge
                        .edge_element_idx()
                        .and_then(|element| eidx_map.get(&element).copied());
                    let Some(index) = element else {
                        sink.lost(Cat::EdgeUse, Why::EdgeUseUnresolved, Some(idx), None);
                        return None;
                    };
                    CompressedEdgeIndex {
                        index,
                        orientation: oriented_edge.orientation == ori,
                    }
                } else {
                    let Some(index) = eidx_map.get(&idx).copied() else {
                        sink.lost(Cat::EdgeUse, Why::EdgeUseUnresolved, Some(idx), None);
                        return None;
                    };
                    CompressedEdgeIndex {
                        index,
                        orientation: ori,
                    }
                };
                Some(edge_idx)
            })
            .collect();
        sink.kept(Cat::EdgeUse, edges.len());
        if !ori {
            edges.reverse();
        }
        Some(edges)
    }

    fn exact_trim_curve_on(
        curve: &Curve3D,
        surface: &Surface,
        orientation: bool,
    ) -> Option<step_geometry::StepParameterCurve> {
        let mut trim_curve = match curve {
            Curve3D::SurfaceCurve(surface_curve) => {
                Self::seam_trim_curve_on(surface_curve, surface, orientation).or_else(|| {
                    step_geometry::StepParameterCurve::try_from(step_geometry::CurveTrimRef::new(
                        curve, surface,
                    ))
                    .ok()
                })?
            }
            _ => step_geometry::StepParameterCurve::try_from(step_geometry::CurveTrimRef::new(
                curve, surface,
            ))
            .ok()?,
        };
        if !orientation {
            trim_curve.invert();
        }
        Some(trim_curve)
    }

    fn seam_trim_curve_on(
        curve: &StepSurfaceCurve3D,
        surface: &Surface,
        orientation: bool,
    ) -> Option<step_geometry::StepParameterCurve> {
        (curve.kind() == StepSurfaceCurveKind::SeamCurve)
            .then(|| {
                let curves = curve
                    .associated_geometry()
                    .iter()
                    .filter_map(|geometry| match geometry {
                        StepSurfaceCurveAssociatedGeometry::ParameterCurve(trim_curve)
                            if trim_curve.surface().as_ref() == surface
                                || StepSurfaceCurve3D::same_surface(
                                    trim_curve.surface().as_ref(),
                                    surface,
                                ) =>
                        {
                            Some(trim_curve)
                        }
                        _ => None,
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let index = usize::from(!orientation);
                curves.get(index).or_else(|| curves.first()).cloned()
            })
            .flatten()
    }

    /// The trimmed sibling of [`Self::face_bound_to_edges`], likewise in the
    /// SURFACE's own sense. Reversing a use also inverts its exact trim, which
    /// `exact_trim_curve_on` already does off the composed orientation.
    fn face_bound_to_edge_uses(
        &self,
        bound: FaceBoundHolder,
        face_sense: bool,
        face_surface: &Surface,
        allow_single_sphere_pole: bool,
        eidx_map: &HashMap<u64, usize>,
        sink: &LossSink,
        converted_edges: &[CompressedEdge<Curve3D>],
    ) -> Option<Vec<CompressedEdgeUse<step_geometry::StepParameterCurve>>> {
        use PlaceHolder::Ref;
        let ori = Self::absolute_bound_orientation(&bound, face_sense);
        if allow_single_sphere_pole
            && let BoundResolution::VertexLoop(id) = bound.resolve_bound(self)
            && self.vertex_loop_is_spherical_pole(id, face_surface)
        {
            sink.listed(Cat::EdgeUse, 0);
            sink.kept(Cat::EdgeUse, 0);
            return Some(Vec::new());
        }
        let bound = self.resolved_edge_loop(&bound, sink)?;
        sink.listed(Cat::EdgeUse, bound.edge_list.len());
        let mut edges = bound
            .edge_list
            .into_iter()
            .filter_map(|edge| {
                let Ref(Name::Entity(idx)) = edge else {
                    sink.lost(
                        Cat::EdgeUse,
                        Why::EdgeUseUnresolved,
                        None,
                        Some("the edge use is not an entity reference".to_owned()),
                    );
                    return None;
                };
                let Some((edge_entity_idx, _edge_curve_holder)) =
                    self.place_holder_edge_any_to_index_and_edge_curve(&Ref(Name::Entity(idx)))
                else {
                    sink.lost(Cat::EdgeUse, Why::EdgeUseUnresolved, Some(idx), None);
                    return None;
                };
                let orientation = self
                    .oriented_edge
                    .get(&idx)
                    .map(|oriented_edge| oriented_edge.orientation == ori)
                    .unwrap_or(ori);
                let Some(index) = eidx_map.get(&edge_entity_idx).copied() else {
                    sink.lost(Cat::EdgeUse, Why::EdgeUseUnresolved, Some(idx), None);
                    return None;
                };
                let curve = &converted_edges.get(index)?.curve;
                Some(CompressedEdgeUse {
                    index,
                    orientation,
                    trim_curve: Self::exact_trim_curve_on(curve, face_surface, orientation),
                })
            })
            .collect::<Vec<_>>();
        sink.kept(Cat::EdgeUse, edges.len());
        if !ori {
            edges.reverse();
        }
        Some(edges)
    }

    /// Resolve and convert one face's surface, REPORTING both refusal arms.
    ///
    /// Both arms used to be `.map_err(|e| eprintln!("{e}")).ok()?`, which is how a
    /// typed per-record refusal still reached the caller as `Ok(shell)` minus a
    /// face -- 253 corpus faces, plus every face of a shell whose surface class
    /// the loader refuses.
    fn vertex_loop_is_spherical_pole(&self, id: u64, surface: &Surface) -> bool {
        let Surface::ElementarySurface(ElementarySurface::Sphere(spherical_surface)) = surface
        else {
            return false;
        };
        let source_radius = spherical_surface.0.radius();
        if !source_radius.is_finite() || source_radius <= 0.0 {
            return false;
        }
        let Some(vertex_loop) = self.vertex_loop.get(&id) else {
            return false;
        };
        let PlaceHolder::Ref(Name::Entity(vertex_id)) = &vertex_loop.loop_vertex else {
            return false;
        };
        let Ok(vertex) = EntityTable::<VertexPointHolder>::get_owned(self, *vertex_id) else {
            return false;
        };
        let position = Point3::from(&vertex.vertex_geometry);
        let (Some((u0, u1)), Some((v0, v1))) = surface.try_range_tuple() else {
            return false;
        };
        let poles = [
            surface.evaluate(u0, v0),
            surface.evaluate(u0, v1),
            surface.evaluate(u1, v0),
            surface.evaluate(u1, v1),
        ];
        if poles
            .iter()
            .any(|p| !p.x.is_finite() || !p.y.is_finite() || !p.z.is_finite())
            || !position.x.is_finite()
            || !position.y.is_finite()
            || !position.z.is_finite()
        {
            return false;
        }

        // Estimate transformed center/radius from opposite domain poles. This
        // includes the placement frame used by `surface.evaluate`.
        let mut opposite_poles = None;
        let mut diameter = 0.0f64;
        for i in 0..poles.len() {
            for j in (i + 1)..poles.len() {
                let distance = (poles[i].x - poles[j].x)
                    .hypot(poles[i].y - poles[j].y)
                    .hypot(poles[i].z - poles[j].z);
                if !distance.is_finite() {
                    return false;
                }
                if distance > diameter {
                    diameter = distance;
                    opposite_poles = Some((poles[i], poles[j]));
                }
            }
        }
        let Some((pole_a, pole_b)) = opposite_poles else {
            return false;
        };
        let geometric_radius = diameter * 0.5;
        if !geometric_radius.is_finite() || geometric_radius <= 0.0 {
            return false;
        }
        let center = Point3::new(
            pole_a.x + (pole_b.x - pole_a.x) * 0.5,
            pole_a.y + (pole_b.y - pole_a.y) * 0.5,
            pole_a.z + (pole_b.z - pole_a.z) * 0.5,
        );

        // Use gamma_64 as a scale-aware qualification tolerance, with no
        // unit-sized absolute floor. This models 64 binary64 operations but is
        // not a formal bound for the complete libm evaluation path. If it
        // consumes the radius, pole identity is ambiguous and is rejected.
        let operations = 64.0;
        let gamma = operations * f64::EPSILON / (1.0 - operations * f64::EPSILON);
        let pole_coordinate_scale = poles
            .iter()
            .flat_map(|p| [p.x.abs(), p.y.abs(), p.z.abs()])
            .fold(0.0, f64::max);
        let coordinate_scale = [
            source_radius,
            geometric_radius,
            center.x.abs(),
            center.y.abs(),
            center.z.abs(),
            position.x.abs(),
            position.y.abs(),
            position.z.abs(),
            pole_coordinate_scale,
        ]
        .into_iter()
        .fold(0.0, f64::max);
        let pole_tolerance = gamma * coordinate_scale;
        if !pole_tolerance.is_finite() || pole_tolerance >= geometric_radius {
            return false;
        }
        poles.into_iter().any(|pole| {
            (position.x - pole.x).abs() <= pole_tolerance
                && (position.y - pole.y).abs() <= pole_tolerance
                && (position.z - pole.z).abs() <= pole_tolerance
        })
    }

    fn face_surface_or_report(&self, face: &FaceSurfaceHolder, sink: &LossSink) -> Option<Surface> {
        let id = Self::ref_id(&face.face_geometry);
        let step_surface: SurfaceAny = match face.face_geometry.clone().into_owned(self) {
            Ok(step_surface) => step_surface,
            Err(e) => {
                eprintln!("{e}");
                sink.lost(Cat::Face, Why::SurfaceUnresolved, id, Some(e.to_string()));
                return None;
            }
        };
        match Surface::try_from(&step_surface) {
            Ok(surface) => Some(surface),
            Err(e) => {
                eprintln!("{e}");
                sink.lost(Cat::Face, Why::SurfaceRefused, id, Some(e.to_string()));
                None
            }
        }
    }

    /// The face's boundary loops, with every loop the loader cannot represent
    /// counted against [`Cat::Wire`] rather than dropped in silence.
    fn face_boundaries_or_report<T>(
        &self,
        face: &FaceSurfaceHolder,
        sink: &LossSink,
        mut to_wire: impl FnMut(FaceBoundHolder) -> Option<T>,
    ) -> Vec<T> {
        let bounds = face.bounds_holder(self);
        sink.listed(Cat::Wire, bounds.len());
        bounds
            .into_iter()
            .filter_map(|(id, bound)| {
                let Some(bound) = bound else {
                    sink.lost(Cat::Wire, Why::BoundUnresolved, id, None);
                    return None;
                };
                let wire = to_wire(bound)?;
                sink.kept(Cat::Wire, 1);
                Some(wire)
            })
            .collect()
    }

    /// Every face the shell lists, paired with the orientation, reporting the
    /// ones whose reference resolves to no face record at all.
    fn shell_listed_faces<'a>(
        &'a self,
        shell: &'a ShellHolder,
        sink: &'a LossSink,
    ) -> impl Iterator<Item = (Option<u64>, bool, FaceSurfaceHolder)> + 'a {
        shell
            .cfs_faces
            .iter()
            .zip(shell.cfs_faces_holder(self))
            .filter_map(move |(reference, face)| {
                sink.listed(Cat::Face, 1);
                let resolved = self.face_any_to_orientation_and_face(face);
                if resolved.is_none() {
                    sink.lost(Cat::Face, Why::FaceUnresolved, None, None);
                }
                resolved.map(|(orientation, face)| {
                    let source_id =
                        Self::ref_id(reference).and_then(|id| match self.oriented_face.get(&id) {
                            Some(oriented) => Self::ref_id(&oriented.face_element),
                            None => self.face_surface.contains_key(&id).then_some(id),
                        });
                    (source_id, orientation, face)
                })
            })
    }

    // STEP bounded spline domains are intrinsic; analytic drawing ranges are not.
    // This certificate constrains only established source domains, never a sampling window.
    fn source_attachment_domain(surface: &Surface) -> Result<[Option<(f64, f64)>; 2], String> {
        let profile_domain = |curve: &Curve3D| -> Result<Option<(f64, f64)>, String> {
            match curve {
                Curve3D::BsplineCurve(_) | Curve3D::NurbsCurve(_) | Curve3D::Polyline(_) => {
                    Ok(Some(curve.range_tuple()))
                }
                Curve3D::Line(_) | Curve3D::Conic(_) => Ok(None),
                _ => Err("source attachment domain for this swept profile is not certified".into()),
            }
        };
        match surface {
            Surface::BsplineSurface(_) | Surface::NurbsSurface(_) => {
                let range = surface.try_range_tuple();
                Ok([range.0, range.1])
            }
            Surface::ElementarySurface(_) => Ok([None, None]),
            Surface::SweepSurface(SweepSurface::ExtrusionSurface(surface)) => {
                Ok([profile_domain(surface.entity_curve())?, None])
            }
            Surface::SweepSurface(SweepSurface::RevolutionSurface(surface)) => {
                let domain = profile_domain(surface.entity().entity_curve())?;
                Ok(if surface.orientation() {
                    [domain, None]
                } else {
                    [None, domain]
                })
            }
        }
    }

    fn source_attachment_witness(
        surface: &Surface,
        point: Point3,
        bound: f64,
        preview: bool,
    ) -> Result<((f64, f64), f64), String> {
        let domain = Self::source_attachment_domain(surface)?;
        if domain
            .iter()
            .flatten()
            .any(|(a, b)| !a.is_finite() || !b.is_finite() || a > b)
        {
            return Err("invalid finite face attachment domain".into());
        }
        // `extend` (preview only) keeps a parameter that overshoots a finite domain by at
        // most `PREVIEW_DOMAIN_OVERSHOOT` of its span, so a point on the surface's natural
        // extension is witnessed there instead of at the clamped, farther boundary point.
        const PREVIEW_DOMAIN_OVERSHOOT: f64 = 0.1;
        let check = |mut uv: (f64, f64), extend: bool| {
            if !uv.0.is_finite() || !uv.1.is_finite() {
                return None;
            }
            for (domain, t) in domain.into_iter().zip([&mut uv.0, &mut uv.1]) {
                if let Some((a, b)) = domain {
                    let overshoot = (a - *t).max(*t - b);
                    if !(extend && overshoot <= PREVIEW_DOMAIN_OVERSHOOT * (b - a)) {
                        *t = t.clamp(a, b);
                    }
                }
            }
            let discrepancy = surface.subs(uv.0, uv.1).distance(point);
            (discrepancy.is_finite() && discrepancy <= bound).then_some((uv, discrepancy))
        };
        let nearest = surface.search_nearest_parameter(point, None, 100);
        if let Some(found) = nearest.and_then(|uv| check(uv, false)) {
            return Ok(found);
        }
        let exact = surface.search_parameter(point, None, 100);
        if let Some(found) = exact.and_then(|uv| check(uv, false)) {
            return Ok(found);
        }
        if preview {
            if let Some(found) = nearest
                .into_iter()
                .chain(exact)
                .find_map(|uv| check(uv, true))
            {
                return Ok(found);
            }
        }
        // source-uncertainty-corner8996.step pins this singular endpoint fallback.
        // Endpoint/singular points need no invertible Jacobian. Each candidate
        // is only a direct residual witness, never a sampled whole-face proof.
        let range = surface.try_range_tuple();
        if let (Some((u0, u1)), Some((v0, v1))) = range {
            let us = [u0, u0 + (u1 - u0) * 0.5, u1];
            let vs = [v0, v0 + (v1 - v0) * 0.5, v1];
            for u in us {
                for v in vs {
                    if let Some(found) = check((u, v), false) {
                        return Ok(found);
                    }
                }
            }
            for u in us {
                for v in vs {
                    if u.is_finite() && v.is_finite() {
                        if let Some(found) = surface
                            .search_nearest_parameter(point, Some((u, v)), 100)
                            .and_then(|uv| check(uv, false))
                        {
                            return Ok(found);
                        }
                    }
                }
            }
        }
        Err(format!(
            "no direct finite face attachment witness within source bound {bound}"
        ))
    }

    fn certify_face_attachments(
        &self,
        face: &FaceSurfaceHolder,
        source_id: Option<u64>,
        face_index: usize,
        surface: &Surface,
        eidx_map: &HashMap<u64, usize>,
        vidx_map: &HashMap<u64, usize>,
        converted_edges: &[CompressedEdge<Curve3D>],
        connectivity: &Result<Option<context::ShellConnectivity>, String>,
        sink: &LossSink,
    ) -> Result<(), String> {
        let policy = connectivity.as_ref().map_err(Clone::clone)?;
        let Some(accuracy) = source_id.and_then(|id| policy.as_ref()?.faces.get(&id)?.as_ref())
        else {
            return Ok(());
        };
        if !SurfaceAny::finite_positive_spline_controls_for(surface) {
            return Err(
                "source surface control net must be finite with positive rational weights".into(),
            );
        }
        let mut attachments = Vec::new();
        for (_, bound) in face.bounds_holder(self) {
            let Some(bound) = bound.and_then(|bound| bound.bound_holder(self)) else {
                continue;
            };
            for edge in bound.edge_list {
                let Some((edge_id, holder)) =
                    self.place_holder_edge_any_to_index_and_edge_curve(&edge)
                else {
                    continue;
                };
                let Some(edge_index) = eidx_map.get(&edge_id).copied() else {
                    continue;
                };
                let curve = &converted_edges
                    .get(edge_index)
                    .ok_or("source attachment edge was not retained")?
                    .curve;
                for (back, vertex, endpoint) in [
                    (false, &holder.edge_start, curve.front()),
                    (true, &holder.edge_end, curve.back()),
                ] {
                    let vertex_id =
                        Self::ref_id(vertex).ok_or("source attachment vertex has no identity")?;
                    let vertex_index = *vidx_map
                        .get(&vertex_id)
                        .ok_or("source attachment vertex was not retained")?;
                    let vertex = vertex.clone().into_owned(self).map_err(|e| e.to_string())?;
                    let point = Point3::from(&vertex.vertex_geometry);
                    for (kind, point) in [
                        (report::ConnectivityIncidence::VertexOnFace, point),
                        (report::ConnectivityIncidence::EdgeEndpointOnFace, endpoint),
                    ] {
                        let (uv, discrepancy) =
                            Self::source_attachment_witness(
                                surface,
                                point,
                                accuracy.bound(),
                                self.preview_tolerance.is_some(),
                            )
                                .map_err(|error| format!("vertex #{vertex_id}: {error}"))?;
                        attachments.push(report::ConnectivityAttachment {
                            kind,
                            edge_index,
                            vertex_index,
                            face_index: Some(face_index),
                            back,
                            uv: Some(uv),
                            edge_id,
                            vertex_id,
                            face_id: source_id,
                            discrepancy,
                            curve_interval: curve.range_tuple(),
                            accuracy: accuracy.clone(),
                        });
                    }
                }
            }
        }
        for attachment in attachments {
            sink.attachment(attachment);
        }
        Ok(())
    }

    fn shell_faces(
        &self,
        shell: &ShellHolder,
        eidx_map: &HashMap<u64, usize>,
        vidx_map: &HashMap<u64, usize>,
        converted_edges: &[CompressedEdge<Curve3D>],
        sink: &LossSink,
        connectivity: &Result<Option<context::ShellConnectivity>, String>,
    ) -> (Vec<CompressedFace<Surface>>, Vec<Option<u64>>) {
        let mut source_ids = Vec::new();
        let faces = self
            .shell_listed_faces(shell, sink)
            .filter_map(|(source_id, orientation, face)| {
                let surface = self.face_surface_or_report(&face, sink)?;
                // The stored boundaries and the stored flag are ONE convention,
                // not two: see `absolute_bound_orientation`.
                let face_sense = orientation == face.same_sense;
                let allow_single_sphere_pole = face.bounds.len() == 1;
                let boundaries = self.face_boundaries_or_report(&face, sink, |bound| {
                    self.face_bound_to_edges(
                        bound,
                        face_sense,
                        &surface,
                        allow_single_sphere_pole,
                        eidx_map,
                        sink,
                    )
                });
                if let Err(error) = self.certify_face_attachments(
                    &face,
                    source_id,
                    source_ids.len(),
                    &surface,
                    eidx_map,
                    vidx_map,
                    converted_edges,
                    connectivity,
                    sink,
                ) {
                    sink.lost(
                        Cat::Face,
                        Why::SourceConnectivityRefused,
                        source_id,
                        Some(error),
                    );
                    return None;
                }
                sink.kept(Cat::Face, 1);
                source_ids.push(source_id);
                Some(CompressedFace {
                    surface,
                    boundaries,
                    orientation: face_sense,
                })
            })
            .collect();
        (faces, source_ids)
    }

    fn shell_trimmed_faces(
        &self,
        shell: &ShellHolder,
        eidx_map: &HashMap<u64, usize>,
        vidx_map: &HashMap<u64, usize>,
        converted_edges: &[CompressedEdge<Curve3D>],
        sink: &LossSink,
        connectivity: &Result<Option<context::ShellConnectivity>, String>,
    ) -> Vec<CompressedTrimmedFace<Surface, step_geometry::StepParameterCurve>> {
        self.shell_listed_faces(shell, sink)
            .enumerate()
            .filter_map(|(face_index, (source_id, orientation, face))| {
                let surface = self.face_surface_or_report(&face, sink)?;
                // The stored boundaries and the stored flag are ONE convention,
                // not two: see `absolute_bound_orientation`.
                let face_sense = orientation == face.same_sense;
                let allow_single_sphere_pole = face.bounds.len() == 1;
                let boundaries = self.face_boundaries_or_report(&face, sink, |bound| {
                    self.face_bound_to_edge_uses(
                        bound,
                        face_sense,
                        &surface,
                        allow_single_sphere_pole,
                        eidx_map,
                        sink,
                        converted_edges,
                    )
                });
                if let Err(error) = self.certify_face_attachments(
                    &face,
                    source_id,
                    face_index,
                    &surface,
                    eidx_map,
                    vidx_map,
                    converted_edges,
                    connectivity,
                    sink,
                ) {
                    sink.lost(
                        Cat::Face,
                        Why::SourceConnectivityRefused,
                        source_id,
                        Some(error),
                    );
                    return None;
                }
                sink.kept(Cat::Face, 1);
                Some(CompressedTrimmedFace {
                    surface,
                    boundaries,
                    orientation: face_sense,
                })
            })
            .collect()
    }

    /// Constructs [`CompressedShell`] from a STEP `Shell`.
    ///
    /// # Example
    /// ```
    /// # fn main() -> anyhow::Result<()> {
    /// # use anyhow::anyhow;
    /// use monstertruck_io::step::load::{*, step_geometry::*};
    ///
    /// let step_string = include_str!(concat!(
    ///     env!("CARGO_MANIFEST_DIR"),
    ///     "/../resources/step/occt-cube.step",
    /// ));
    /// let table = Table::from_step(&step_string)?;
    /// let step_shell = table
    ///     .shell
    ///     .values()
    ///     .next()
    ///     .ok_or_else(|| anyhow!("STEP file contains no shell."))?;
    /// let cshell = table.to_compressed_shell(step_shell)?;
    /// assert_eq!(cshell.faces.len(), 6);
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_compressed_shell(
        &self,
        shell: &impl StepShell,
    ) -> Result<CompressedShell<Point3, Curve3D, Surface>, LoadError> {
        Ok(shell.to_compressed_shell(self)?)
    }

    /// [`Self::to_compressed_shell`] plus a truthful answer to "did I get
    /// everything?".
    ///
    /// # Why the un-suffixed method is not enough
    ///
    /// `to_compressed_shell` returns `Ok` for a shell that is short of faces,
    /// wires, edges or vertices, and always has. Measured over the corpus and the
    /// in-repo fixtures (spec 011 Phase 0): 253 faces and 15,444 shell faces lost
    /// corpus-wide, plus wires lost on four in-repo fixtures including the
    /// reference boolean fixture. The evidence was one line on stderr, or -- for a
    /// `VERTEX_LOOP` -- nothing at all.
    ///
    /// The report is ADDITIVE: this method does exactly the same conversion work
    /// and returns exactly the same shell.
    ///
    /// # Example
    /// ```
    /// # fn main() -> anyhow::Result<()> {
    /// # use anyhow::anyhow;
    /// use monstertruck_io::step::load::{*, report::*};
    ///
    /// let step_string = include_str!(concat!(
    ///     env!("CARGO_MANIFEST_DIR"),
    ///     "/../resources/step/occt-cube.step",
    /// ));
    /// let table = Table::from_step(step_string)?;
    /// let step_shell = table
    ///     .shell
    ///     .values()
    ///     .next()
    ///     .ok_or_else(|| anyhow!("STEP file contains no shell."))?;
    /// let (cshell, report) = table.to_compressed_shell_reported(step_shell)?;
    /// assert_eq!(cshell.faces.len(), 6);
    /// // The cube has no degenerate loops and no refused surfaces.
    /// assert!(report.is_lossless(), "{report}");
    /// assert_eq!(report.count(LossCategory::Face).listed, 6);
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_compressed_shell_reported(
        &self,
        shell: &impl StepShell,
    ) -> Result<(CompressedShell<Point3, Curve3D, Surface>, ShellLoadReport), LoadError> {
        Ok(shell.to_compressed_shell_reported(self)?)
    }

    /// Constructs `CompressedTrimmedShell` from `Shell` in STEP file while preserving
    /// exact face-local `ParameterCurve`s when they are present in the STEP data.
    pub fn to_compressed_trimmed_shell(
        &self,
        shell: &impl StepShell,
    ) -> Result<
        CompressedTrimmedShell<Point3, Curve3D, Surface, step_geometry::StepParameterCurve>,
        LoadError,
    > {
        Ok(shell.to_compressed_trimmed_shell(self)?)
    }

    /// [`Self::to_compressed_trimmed_shell`] plus the loss report. See
    /// [`Self::to_compressed_shell_reported`].
    pub fn to_compressed_trimmed_shell_reported(
        &self,
        shell: &impl StepShell,
    ) -> Result<(StepCompressedTrimmedShell, ShellLoadReport), LoadError> {
        Ok(shell.to_compressed_trimmed_shell_reported(self)?)
    }

    /// Constructs `CompressedShell`s from `ShellBasedSurfaceModel` in STEP file
    pub fn to_compressed_shells(
        &self,
        shells: &ShellBasedSurfaceModelHolder,
    ) -> Result<Vec<CompressedShell<Point3, Curve3D, Surface>>, LoadError> {
        let mut res = Vec::new();
        for place_holder in &shells.sbsm_boundary {
            let PlaceHolder::Ref(Name::Entity(idx)) = place_holder else {
                return Err("failed to reference an element of `sbsm_boundary`".into());
            };
            if let Some(shell) = self.shell.get(idx) {
                res.push(self.to_compressed_shell(shell)?);
            } else if let Some(oriented_shell) = self.oriented_shell.get(idx) {
                res.push(self.to_compressed_shell(oriented_shell)?);
            } else {
                return Err("failed to reference an element of `sbsm_boundary`".into());
            }
        }
        Ok(res)
    }

    /// Constructs `CompressedTrimmedShell`s from `ShellBasedSurfaceModel` in STEP file.
    pub fn to_compressed_trimmed_shells(
        &self,
        shells: &ShellBasedSurfaceModelHolder,
    ) -> Result<
        Vec<CompressedTrimmedShell<Point3, Curve3D, Surface, step_geometry::StepParameterCurve>>,
        LoadError,
    > {
        let mut res = Vec::new();
        for place_holder in &shells.sbsm_boundary {
            let PlaceHolder::Ref(Name::Entity(idx)) = place_holder else {
                return Err("failed to reference an element of `sbsm_boundary`".into());
            };
            if let Some(shell) = self.shell.get(idx) {
                res.push(self.to_compressed_trimmed_shell(shell)?);
            } else if let Some(oriented_shell) = self.oriented_shell.get(idx) {
                res.push(self.to_compressed_trimmed_shell(oriented_shell)?);
            } else {
                return Err("failed to reference an element of `sbsm_boundary`".into());
            }
        }
        Ok(res)
    }

    /// Constructs [`CompressedSolid`] from [`ManifoldSolidBrep`] in a STEP file.
    ///
    /// The result keeps the STEP `outer` shell as the first entry of
    /// `boundaries` and appends one entry per `void` (inner cavity).
    ///
    /// `Solid::extract` on the returned [`CompressedSolid`] requires the
    /// outer shell to satisfy [`ShellCondition::Closed`](monstertruck_topology::shell::ShellCondition::Closed). Many real-world
    /// STEP exports produce shells that are topologically *regular* but
    /// not strictly *oriented*; for those, prefer
    /// `to_compressed_trimmed_solid` and downstream meshing/healing.
    ///
    /// # Example
    /// ```
    /// # fn main() -> anyhow::Result<()> {
    /// # use anyhow::anyhow;
    /// use monstertruck_io::step::load::{*, step_geometry::*};
    ///
    /// let step_string = include_str!(concat!(
    ///     env!("CARGO_MANIFEST_DIR"),
    ///     "/../resources/step/occt-cube.step",
    /// ));
    /// let table = Table::from_step(&step_string)?;
    /// let step_solid = table
    ///     .manifold_solid_brep
    ///     .values()
    ///     .next()
    ///     .ok_or_else(|| anyhow!("STEP file contains no manifold solid B-rep."))?;
    /// let csolid = table.to_compressed_solid(step_solid)?;
    /// assert_eq!(csolid.boundaries.len(), 1);
    /// assert_eq!(csolid.boundaries[0].faces.len(), 6);
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_compressed_solid(
        &self,
        solid: &ManifoldSolidBrepHolder,
    ) -> Result<CompressedSolid<Point3, Curve3D, Surface>, LoadError> {
        self.to_compressed_solid_reported(solid)
            .map(|(solid, _)| solid)
    }

    /// [`Self::to_compressed_solid`] plus ONE [`ShellLoadReport`] per entry of
    /// `boundaries`, positionally aligned with it -- so a caller can say which
    /// shell lost what, not merely that the solid lost something.
    ///
    /// See [`Self::to_compressed_shell_reported`] for why this exists.
    pub fn to_compressed_solid_reported(
        &self,
        solid: &ManifoldSolidBrepHolder,
    ) -> Result<(StepCompressedSolid, Vec<ShellLoadReport>), LoadError> {
        self.compressed_solid_with_sources_reported(solid)
            .map(|(solid, reports, _)| (solid, reports))
    }

    fn compressed_solid_with_sources_reported(
        &self,
        solid: &ManifoldSolidBrepHolder,
    ) -> Result<SourceMappedSolid, LoadError> {
        self.compressed_solid_in_representation_reported(solid, None)
    }
    fn compressed_solid_in_representation_reported(
        &self,
        solid: &ManifoldSolidBrepHolder,
        representation: Option<u64>,
    ) -> Result<SourceMappedSolid, LoadError> {
        self.compressed_solid_with_connectivity_reported(
            solid,
            ConnectivitySelection::FromSource(representation),
        )
    }
    fn compressed_solid_with_connectivity_reported(
        &self,
        solid: &ManifoldSolidBrepHolder,
        selection: ConnectivitySelection,
    ) -> Result<SourceMappedSolid, LoadError> {
        let outer_id = Self::ref_id(&solid.outer).ok_or("failed to reference `solid.outer`")?;
        if !self.shell.contains_key(&outer_id) && !self.oriented_shell.contains_key(&outer_id) {
            return Err("failed to reference `solid.outer`".into());
        }
        let (outer, report, ids) = self.source_mapped_shell_at(outer_id, selection)?;
        let mut boundaries = vec![outer];
        let mut reports = vec![report];
        let mut sources = vec![ids];
        for shell in &solid.voids {
            let id =
                Self::ref_id(shell).ok_or("failed to reference an element of `solid.voids`")?;
            if !self.oriented_shell.contains_key(&id) {
                return Err("failed to reference an element of `solid.voids`".into());
            }
            let (shell, report, ids) = self.source_mapped_shell_at(id, selection)?;
            boundaries.push(shell);
            reports.push(report);
            sources.push(ids);
        }
        Ok((
            CompressedSolid {
                boundaries,
                id_allocator: None,
                attributes: None,
            },
            reports,
            sources,
        ))
    }

    /// Converts without losses and binds source identities/certificates to exact
    /// immutable source-space surfaces. Inline faces have no usable entity ID.
    /// This checks conversion completeness, not assembly or whole-shell validity.
    pub fn to_source_bound_solid_checked(
        &self,
        solid: &ManifoldSolidBrepHolder,
    ) -> Result<SourceBoundStepSolid, LoadError> {
        self.source_bound_solid_in_representation(solid, None)
    }

    /// Checked strict geometry with no representation connectivity authority.
    /// This never discovers uncertainty owners or creates attachment witnesses.
    /// Units, assembly validity and whole-shell validity remain caller concerns.
    pub fn to_source_bound_solid_without_connectivity_checked(
        &self,
        solid: &ManifoldSolidBrepHolder,
    ) -> Result<SourceBoundStepSolid, LoadError> {
        self.entity_report.require_empty()?;
        let source = self
            .compressed_solid_with_connectivity_reported(solid, ConnectivitySelection::Disabled)?;
        self.bind_source_solid(source)
    }

    /// Selects the actual geometric owner of a reused solid definition.
    pub fn to_source_bound_solid_in_representation_checked(
        &self,
        solid: &ManifoldSolidBrepHolder,
        representation: u64,
    ) -> Result<SourceBoundStepSolid, LoadError> {
        self.source_bound_solid_in_representation(solid, Some(representation))
    }

    fn source_bound_solid_in_representation(
        &self,
        solid: &ManifoldSolidBrepHolder,
        representation: Option<u64>,
    ) -> Result<SourceBoundStepSolid, LoadError> {
        self.entity_report.require_empty()?;
        let source = self.compressed_solid_in_representation_reported(solid, representation)?;
        self.bind_source_solid(source)
    }

    fn bind_source_solid(
        &self,
        source: SourceMappedSolid,
    ) -> Result<SourceBoundStepSolid, LoadError> {
        let (converted, reports, sources) = source;
        for report in &reports {
            report.require_lossless()?;
        }
        if converted.boundaries.len() != sources.len() || reports.len() != sources.len() {
            return Err("source shell binding count mismatch".into());
        }
        let mut shells = Vec::new();
        for ((shell, ids), report) in converted.boundaries.into_iter().zip(sources).zip(reports) {
            shells.push(self.bind_source_shell(
                shell,
                ids,
                report.connectivity_attachments().to_vec(),
            )?);
        }
        Ok(SourceBoundStepSolid { shells })
    }

    /// Checked source binding for a shell, including an open surface-model shell.
    /// This does not claim that an open shell bounds a solid.
    pub fn to_source_bound_shell_checked(
        &self,
        shell: &ShellHolder,
    ) -> Result<SourceBoundStepShell, LoadError> {
        self.entity_report.require_empty()?;
        let (shell, report, ids) = self.compressed_shell_with_sources_reported(shell)?;
        report.require_lossless()?;
        self.bind_source_shell(shell, ids, report.connectivity_attachments().to_vec())
    }

    /// Checked strict geometry without consulting representation connectivity.
    /// Returns no attachment authority, including for unrecognised declarations.
    /// The caller authenticates units separately; this is not whole-shell validity.
    pub fn to_source_bound_shell_without_connectivity_checked(
        &self,
        shell: &ShellHolder,
    ) -> Result<SourceBoundStepShell, LoadError> {
        self.entity_report.require_empty()?;
        let (shell, report, ids) = self
            .compressed_shell_with_connectivity_reported(shell, ConnectivitySelection::Disabled)?;
        report.require_lossless()?;
        self.bind_source_shell(shell, ids, report.connectivity_attachments().to_vec())
    }

    /// Selects an owning geometric representation explicitly when geometry is reused.
    pub fn to_source_bound_shell_in_representation_checked(
        &self,
        shell: &ShellHolder,
        representation: u64,
    ) -> Result<SourceBoundStepShell, LoadError> {
        self.entity_report.require_empty()?;
        let (shell, report, ids) =
            self.compressed_shell_in_representation_reported(shell, Some(representation))?;
        report.require_lossless()?;
        self.bind_source_shell(shell, ids, report.connectivity_attachments().to_vec())
    }

    /// Diagnostic conversion under an explicit geometric owner. Every refusal
    /// remains in the returned report; callers must require losslessness.
    pub fn to_compressed_shell_in_representation_reported(
        &self,
        shell: &ShellHolder,
        representation: u64,
    ) -> Result<(StepCompressedShell, ShellLoadReport), LoadError> {
        self.compressed_shell_in_representation_reported(shell, Some(representation))
            .map(|(shell, report, _)| (shell, report))
            .map_err(Into::into)
    }

    fn bind_source_shell(
        &self,
        shell: StepCompressedShell,
        ids: Vec<Option<u64>>,
        connectivity_attachments: Vec<report::ConnectivityAttachment>,
    ) -> Result<SourceBoundStepShell, LoadError> {
        if shell.faces.len() != ids.len() {
            return Err("source face binding count mismatch".into());
        }
        let mut source_face_ids = Vec::new();
        let mut certified_closed_axes = Vec::new();
        let mut source_surfaces = Vec::new();
        for (face, id) in shell.faces.iter().zip(ids) {
            let id = id.ok_or("source face has no entity identity")?;
            let raw = self
                .face_surface
                .get(&id)
                .ok_or("source face binding is unresolved")?
                .clone()
                .into_owned(self)?
                .face_geometry;
            if Surface::try_from(&raw)? != face.surface {
                return Err("source face binding surface mismatch".into());
            }
            certified_closed_axes.push(raw.certified_closed_axes_for(&face.surface));
            source_surfaces.push(raw);
            source_face_ids.push(id);
        }
        Ok(SourceBoundStepShell {
            shell,
            source_face_ids,
            certified_closed_axes,
            source_surfaces,
            connectivity_attachments,
        })
    }

    /// Constructs `CompressedTrimmedSolid` from `ManifoldSolidBrep` in STEP file.
    pub fn to_compressed_trimmed_solid(
        &self,
        solid: &ManifoldSolidBrepHolder,
    ) -> Result<
        CompressedTrimmedSolid<Point3, Curve3D, Surface, step_geometry::StepParameterCurve>,
        LoadError,
    > {
        let PlaceHolder::Ref(Name::Entity(outer_idx)) = &solid.outer else {
            return Err("failed to reference `solid.outer`".into());
        };
        let outer_shell = if let Some(step_shell) = self.shell.get(outer_idx) {
            self.to_compressed_trimmed_shell(step_shell)
        } else if let Some(step_shell) = self.oriented_shell.get(outer_idx) {
            self.to_compressed_trimmed_shell(step_shell)
        } else {
            Err("failed to reference `solid.outer`".into())
        }?;
        let mut boundaries = vec![outer_shell];
        for inner in &solid.voids {
            let PlaceHolder::Ref(Name::Entity(inner_idx)) = inner else {
                return Err("failed to reference a member of `solid.voids`".into());
            };
            if let Some(shell) = self.shell.get(inner_idx) {
                boundaries.push(self.to_compressed_trimmed_shell(shell)?);
            } else if let Some(oriented_shell) = self.oriented_shell.get(inner_idx) {
                boundaries.push(self.to_compressed_trimmed_shell(oriented_shell)?);
            } else {
                return Err("failed to reference a member of `solid.voids`".into());
            }
        }
        Ok(CompressedTrimmedSolid { boundaries })
    }

    /// Extracts [`Curve3D`] curves from a [`GeometricCurveSet`].
    ///
    /// Returns `None` if the holder cannot be resolved or yields no curves.
    /// Silently skips points and curves that fail conversion.
    pub fn to_curve3d_set(&self, gcs: &GeometricCurveSetHolder) -> Option<Vec<Curve3D>> {
        let owned = gcs.clone().into_owned(self).ok()?;
        let curves: Vec<Curve3D> = owned
            .elements
            .iter()
            .filter_map(|elem| match elem {
                GeometricSetSelect::Curve(curve) => Curve3D::try_from(curve.as_ref())
                    .map_err(|e| eprintln!("skipping curve in geometric_curve_set: {e}"))
                    .ok(),
                GeometricSetSelect::Point(_) => None,
            })
            .collect();
        if curves.is_empty() {
            None
        } else {
            Some(curves)
        }
    }
}

#[derive(Clone, Debug, derive_more::From)]
pub enum NodeMatrix {
    Identity,
    Transform(Box<ItemDefinedTransformation>),
}

pub use crate::step::common::PartAttributes;

pub type ProductEntity = NodeEntity<Vec<u64>, PartAttributes>;
pub type AssembleEntity = EdgeEntity<NodeMatrix, PartAttributes>;
pub type StepAssembly = Assembly<Vec<u64>, PartAttributes, NodeMatrix, PartAttributes>;

impl TryFrom<&NodeMatrix> for Matrix3 {
    type Error = StepConvertingError;
    fn try_from(value: &NodeMatrix) -> Result<Self, Self::Error> {
        match value {
            NodeMatrix::Identity => Ok(Self::identity()),
            NodeMatrix::Transform(trans) => (&**trans).try_into(),
        }
    }
}

impl TryFrom<&NodeMatrix> for Matrix4 {
    type Error = StepConvertingError;
    fn try_from(value: &NodeMatrix) -> Result<Self, Self::Error> {
        match value {
            NodeMatrix::Identity => Ok(Self::identity()),
            NodeMatrix::Transform(trans) => (&**trans).try_into(),
        }
    }
}

impl Table {
    fn product_node_entity(
        &self,
        pds_idx: u64,
        pd: &ProductDefinitionHolder,
    ) -> Result<ProductEntity, StepConvertingError> {
        let PlaceHolder::Ref(Name::Entity(pdf_idx)) = &pd.formation else {
            return Err("failed to reference `product_definition.formation`".into());
        };
        let Some(pdf) = self.product_definition_formation.get(pdf_idx) else {
            return Err("failed to reference `prouct_definition_formation`".into());
        };
        let PlaceHolder::Ref(Name::Entity(p_idx)) = &pdf.of_product else {
            return Err("failed to reference `product_definition_formation.of_product`".into());
        };
        let Some(product) = self.product.get(p_idx) else {
            return Err("failed to reference `product`".into());
        };
        let attrs = PartAttributes {
            id: product.id.clone(),
            name: product.name.clone(),
            // `PartAttributes::description` is a display field shared with the
            // save side and is a plain `String`, so an UNSET STEP description
            // flattens to `""` here -- the one place the `Option` added for
            // spec 011 T6 loses the unset/empty distinction. Deliberate: keeping
            // it would change what `save` emits for these parts, which is a
            // separate decision from making the load stop dropping the record.
            description: product.description.clone().unwrap_or_default(),
            // Filled in below, once the representation has been RESOLVED. Set
            // here it would be a promise; set there it is a fact.
            shape_representation: None,
        };

        let Some(sdr) = self.shape_definition_representation.values().find(|sdr| {
            let &PlaceHolder::Ref(Name::Entity(idx)) = &sdr.definition else {
                return false;
            };
            pds_idx == idx
        }) else {
            return Err("failed to find `shape_definition_representation` corresp. to `product_definition_shape`".into());
        };
        let PlaceHolder::Ref(Name::Entity(sr_idx)) = &sdr.used_representation else {
            return Err(
                "failed to reference `shape_definition_representation.used_representation`".into(),
            );
        };
        let Some(sr) = self.shape_representation.get(sr_idx) else {
            return Err("failed to reference `shape_representation`".into());
        };
        // The id this whole function already had and used to discard. It is
        // recorded only on the `get` success path above, so
        // `PartAttributes::shape_representation` is `Some(id)` ONLY for an id
        // the table demonstrably holds -- there is no "carried but dangling"
        // state for a caller to have to guard against.
        let attrs = PartAttributes {
            shape_representation: Some(*sr_idx),
            ..attrs
        };
        let Some(shape) = sr
            .items
            .iter()
            .map(|place_holder| {
                if let &PlaceHolder::Ref(Name::Entity(item_idx)) = place_holder {
                    Some(item_idx)
                } else {
                    None
                }
            })
            .collect::<Option<Vec<_>>>()
        else {
            return Err("failed to reference an element of `shape_representation.items`".into());
        };

        Ok(ProductEntity { shape, attrs })
    }

    fn assy_node_entity(
        &self,
        pds_idx: u64,
        next_assy: &NextAssemblyUsageOccurrenceHolder,
    ) -> Result<(AssembleEntity, (u64, u64)), StepConvertingError> {
        let &PlaceHolder::Ref(Name::Entity(parent_idx)) = &next_assy.relating_product_definition
        else {
            return Err("failed to reference the parent node".into());
        };
        let &PlaceHolder::Ref(Name::Entity(child_idx)) = &next_assy.related_product_definition
        else {
            return Err("failed to reference the child node".into());
        };

        let attrs = PartAttributes {
            id: next_assy.id.clone(),
            name: next_assy.name.clone(),
            description: next_assy.description.clone(),
            // An EDGE is a `NEXT_ASSEMBLY_USAGE_OCCURRENCE`, not a product with
            // a shape representation of its own. This is one of the two
            // populations that legitimately carry no id; see
            // `PartAttributes::shape_representation`.
            shape_representation: None,
        };

        let Some(cdsr) = self
            .context_dependent_shape_representation
            .values()
            .find(|cdsr| {
                let &PlaceHolder::Ref(Name::Entity(idx)) = &cdsr.represented_product_relation
                else {
                    return false;
                };
                pds_idx == idx
            })
        else {
            return Err("".into());
        };

        let PlaceHolder::Ref(Name::Entity(srrwt_idx)) = &cdsr.representation_relation else {
            return Err("failed to reference `context_dependent_shape_representation.representation_relation`".into());
        };

        let Some(srrwt) = self
            .shape_representation_relationship_with_transformation
            .get(srrwt_idx)
        else {
            return Err("failed to reference `shape_representation_relationship`".into());
        };
        let idtf = srrwt.transformation_operator.clone().into_owned(self)?;

        let entity = AssembleEntity {
            matrix: NodeMatrix::Transform(idtf.into()),
            attrs,
        };

        Ok((entity, (parent_idx, child_idx)))
    }

    pub fn step_assy(&self) -> Result<StepAssembly, LoadError> {
        let mut product_entities = Vec::<ProductEntity>::new();
        let mut indices_map = HashMap::<u64, usize>::new();
        let mut assy_nodes = Vec::<(AssembleEntity, (u64, u64))>::new();
        for (&pds_idx, pds) in &self.product_definition_shape {
            let &PlaceHolder::Ref(Name::Entity(idx)) = &pds.definition else {
                return Err("failed to reference `product_definition_shape.definition`".into());
            };
            if let Some(pd) = self.product_definition.get(&idx) {
                product_entities.push(self.product_node_entity(pds_idx, pd)?);
                indices_map.insert(idx, product_entities.len() - 1);
            } else if let Some(next_assy) = self.next_assembly_usage_occurrence.get(&idx) {
                assy_nodes.push(self.assy_node_entity(pds_idx, next_assy)?);
            }
        }

        let adjacency = assy_nodes
            .into_iter()
            .map(|(entity, (from, to))| {
                let from = *indices_map.get(&from)?;
                let to = *indices_map.get(&to)?;
                Some((from, to, entity))
            })
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| LoadError::from("failed to reference `product_definition_shape`."))?;

        StepAssembly::try_from_adjacency(product_entities, adjacency)
            .ok_or_else(|| LoadError::from("maybe the assembly graph has a cycle."))
    }

    /// The representation a loaded assembly node came from -- the id it CARRIES
    /// if it has one, and the reverse match otherwise.
    ///
    /// This is what a caller holding a [`ProductEntity`] should ask. Since spec
    /// 012 the node carries the id directly
    /// ([`PartAttributes::shape_representation`]), which is exact by
    /// construction: [`Table::step_assy`] records it on the same success path
    /// that resolved the record. [`Table::shape_representation_of_items`] stays
    /// as the fallback for the nodes that legitimately carry none -- an
    /// assembly this crate did not load, i.e. one an application or the save
    /// side built, where the field is `Default`'s `None`.
    ///
    /// **Measured on the two Scania files (`scania_assembly_graphs_now_load_and_walk`):
    /// the two routes agree node for node, and the id route reaches the same
    /// 832 of 832 and 254 of 254 solids.** A disagreement would be a finding,
    /// not a rounding error, and that pin is where it would surface.
    pub fn shape_representation_of_node(&self, node: &ProductEntity) -> Option<u64> {
        node.attrs
            .shape_representation
            .or_else(|| self.shape_representation_of_items(&node.shape))
    }

    /// The representation whose `items` are EXACTLY `items` -- the bridge from a
    /// [`ProductEntity::shape`] back to the `SHAPE_REPRESENTATION` it came from.
    ///
    /// **Since spec 012 this is the FALLBACK, not the primary route.** A node
    /// loaded by [`Table::step_assy`] carries its representation id in
    /// [`PartAttributes::shape_representation`]; prefer
    /// [`Table::shape_representation_of_node`], which reads the field and only
    /// falls back to this search when there is none. This search remains public
    /// because it answers a question the field cannot: which representation, if
    /// any, holds an item list a caller constructed itself.
    ///
    /// [`Table::step_assy`] fills a node's `shape` from
    /// `shape_definition_representation.used_representation.items`. Before spec
    /// 012 it discarded the representation's own id, so a caller holding an
    /// assembly node could not ask anything else about that representation, and
    /// this inverse was the only bridge.
    ///
    /// Matching is on the whole id list, not on containment, and that is not
    /// caution for its own sake: **measured, `coffy.step` lists `#15756` in BOTH
    /// `#15521` and `#261`**, so a containment rule would answer that item's
    /// owner ambiguously. Whole-list matching is unique on every file measured --
    /// including the two Scania ones, where no representation item is shared at
    /// all (0 of 1,571 and 0 of 1,160). `None` when nothing matches; the FIRST
    /// match in ascending id order if several somehow do.
    ///
    /// `items` that are not all entity references cannot match anything, because
    /// `ProductEntity::shape` only exists when every item was one.
    pub fn shape_representation_of_items(&self, items: &[u64]) -> Option<u64> {
        let mut best: Option<u64> = None;
        for (&id, representation) in &self.shape_representation {
            if representation.items.len() != items.len() {
                continue;
            }
            let same = representation.items.iter().zip(items).all(
                |(held, want)| matches!(held, PlaceHolder::Ref(Name::Entity(held)) if held == want),
            );
            if same && best.is_none_or(|previous| id < previous) {
                best = Some(id);
            }
        }
        best
    }

    /// The solids a part's representation reaches through the **non-transforming**
    /// `SHAPE_REPRESENTATION_RELATIONSHIP` hop, plus what the hop lost.
    ///
    /// # Why this is not [`ProductEntity::shape`]
    ///
    /// `shape` means "the items directly in this node's representation", and on
    /// real assemblies that is honestly what it holds: **measured on
    /// `Scania-8x4.stp` it is `{AXIS2_PLACEMENT_3D: 695, SHELL_BASED_SURFACE_MODEL:
    /// 44}` and on `Scania-Engine-V8-XT-Turbo.step` `{AXIS2_PLACEMENT_3D: 906}` --
    /// zero solids.** The solids live in a second representation, an
    /// `ADVANCED_BREP_SHAPE_REPRESENTATION`, and this hop is what reaches them:
    /// **832 of 832 and 254 of 254**, exactly the 1,086 solids spec 011 T6 found
    /// with no placement.
    ///
    /// Making `shape` chase the relationship silently would make an existing
    /// method's answer depend on a hop the caller cannot see -- and `shape` is
    /// also what the save side round-trips, where the direct meaning is the
    /// right one. So the hop is a separate, visible call. Promoting it to the
    /// default later is easy; un-conflating two meanings after the fact is not.
    ///
    /// # Non-transforming only
    ///
    /// A transforming relationship is a complex record carrying
    /// `REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION`, and it lands in
    /// [`Table::shape_representation_relationship_with_transformation`], a
    /// different map that [`Table::step_assy`] already walks to build the
    /// assembly's edge matrices. Following it here too would apply the same
    /// placement twice. This method reads ONLY
    /// [`Table::shape_representation_relationship`], which by construction holds
    /// simple records exclusively -- a transformation cannot be attached to one.
    ///
    /// Only the `rep_1 == shape_representation -> rep_2` direction is followed.
    /// **Measured: the reverse direction carries zero solids on both Scania
    /// files** (all 188 and 158 relationships have the part's own representation
    /// as `rep_1`), so following it would add no reach and would let one part
    /// claim another's geometry.
    ///
    /// # What the report says
    ///
    /// [`LossCategory::Representation`] counts relationships found versus
    /// relationships whose `rep_2` the table actually holds;
    /// [`LossCategory::Solid`] counts the items of those representations versus
    /// the ones that are solids. Read
    /// [`LossReason::RepresentationItemNotASolid`] before treating a lossy report
    /// as a defect: a representation listing its own `AXIS2_PLACEMENT_3D` beside
    /// its solid is normal, and is the usual reason a file reports items lost
    /// while losing no solid at all.
    ///
    /// Ids come back in ascending relationship id, then representation item
    /// order, and are NOT deduplicated -- `kept` and the returned length always
    /// agree.
    ///
    /// # Example -- how a viewer opts in
    ///
    /// The hop only has work to do when a part's own representation RELATES to
    /// the representation holding the solid; on a file whose parts carry their
    /// solids directly it correctly reaches nothing and reports no loss, which
    /// is what the fixture below shows.
    ///
    /// ```
    /// # fn main() -> anyhow::Result<()> {
    /// use monstertruck_io::step::load::{report::*, Table};
    ///
    /// let bytes = include_bytes!(concat!(
    ///     env!("CARGO_MANIFEST_DIR"),
    ///     "/../resources/step/occt-assy.step",
    /// ));
    /// let table = Table::from_step_bytes(bytes)?;
    /// let assembly = table.step_assy()?;
    ///
    /// let mut solids = Vec::new();
    /// for node in assembly.all_nodes() {
    ///     // `node.shape()` is the items DIRECTLY in this node's representation
    ///     // -- on a real assembly, placements. The hop is the extra call.
    ///     let Some(representation) = table.shape_representation_of_items(node.shape()) else {
    ///         continue;
    ///     };
    ///     let (reached, report) = table.solids_via_shape_relationship(representation);
    ///     // The report says what the hop could NOT deliver, and why.
    ///     assert_eq!(report.lost_for(LossReason::RelatedRepresentationUnresolved), 0);
    ///     solids.extend(reached);
    /// }
    /// // Every relationship this file does have resolved, so nothing was lost.
    /// assert!(solids.iter().all(|id| *id > 0));
    /// # Ok(())
    /// # }
    /// ```
    pub fn solids_via_shape_relationship(
        &self,
        shape_representation: u64,
    ) -> (Vec<u64>, ShellLoadReport) {
        let mut report = ShellLoadReport::default();
        let mut solids = Vec::new();

        let mut relationships: Vec<_> = self
            .shape_representation_relationship
            .iter()
            .filter(|(_, srr)| {
                matches!(&srr.rep_1, PlaceHolder::Ref(Name::Entity(rep_1)) if *rep_1 == shape_representation)
            })
            .collect();
        relationships.sort_unstable_by_key(|(id, _)| **id);

        for (&srr_id, srr) in relationships {
            report.note_listed(Cat::Representation, 1);
            let PlaceHolder::Ref(Name::Entity(rep_2)) = &srr.rep_2 else {
                report.note_lost(
                    Cat::Representation,
                    Why::RelatedRepresentationUnresolved,
                    Some(srr_id),
                    Some("`rep_2` is not an entity reference".to_owned()),
                );
                continue;
            };
            let Some(representation) = self.shape_representation.get(rep_2) else {
                report.note_lost(
                    Cat::Representation,
                    Why::RelatedRepresentationUnresolved,
                    Some(*rep_2),
                    Some(self.describe_unheld(*rep_2)),
                );
                continue;
            };
            report.note_kept(Cat::Representation, 1);

            for item in &representation.items {
                report.note_listed(Cat::Solid, 1);
                let PlaceHolder::Ref(Name::Entity(item_id)) = item else {
                    report.note_lost(
                        Cat::Solid,
                        Why::RepresentationItemNotASolid,
                        Some(*rep_2),
                        Some("the item is not an entity reference".to_owned()),
                    );
                    continue;
                };
                if self.manifold_solid_brep.contains_key(item_id) {
                    report.note_kept(Cat::Solid, 1);
                    solids.push(*item_id);
                } else {
                    report.note_lost(
                        Cat::Solid,
                        Why::RepresentationItemNotASolid,
                        Some(*item_id),
                        Some(self.describe_unheld(*item_id)),
                    );
                }
            }
        }

        (solids, report)
    }

    /// One line naming what `id` actually is, for a loss detail. The entity NAME
    /// is the whole point -- "not a solid" without it sends a reader back to the
    /// file with nothing but an id.
    fn describe_unheld(&self, id: u64) -> String {
        if let Some(dummy) = self.dummy.get(&id) {
            // `Table::dummy` stores `format!("{record:?}")`, whose derived
            // `Debug` puts the entity name in the first quoted field.
            // Diagnostic convenience, not a contract: an unparseable shape
            // degrades to the record text itself.
            let name = dummy.record.split('"').nth(1).unwrap_or(&dummy.record);
            format!("#{id} is `{name}`, an entity type the loader has no arm for")
        } else if self.axis2_placement_3d.contains_key(&id) {
            format!("#{id} is an AXIS2_PLACEMENT_3D")
        } else if self.shell_based_surface_model.contains_key(&id) {
            format!("#{id} is a SHELL_BASED_SURFACE_MODEL")
        } else if self.geometric_curve_set.contains_key(&id) {
            format!("#{id} is a GEOMETRIC_CURVE_SET")
        } else {
            format!("#{id} is in no table map")
        }
    }
}

/// A STEP entity a compressed shell can be built from.
///
/// The `*_reported` methods are the PRIMARY ones: they return the report of what
/// the conversion kept and lost. The un-suffixed methods are provided defaults
/// that discard it, so every existing caller keeps working unchanged.
pub trait StepShell {
    /// Convert, and report what was kept and lost.
    fn to_compressed_shell_reported(
        &self,
        table: &Table,
    ) -> Result<(CompressedShell<Point3, Curve3D, Surface>, ShellLoadReport), StepConvertingError>;

    /// Convert preserving exact trim curves, and report what was kept and lost.
    fn to_compressed_trimmed_shell_reported(
        &self,
        table: &Table,
    ) -> Result<(StepCompressedTrimmedShell, ShellLoadReport), StepConvertingError>;

    /// Convert, DISCARDING the loss report.
    ///
    /// A lossy conversion is indistinguishable from a clean one through this
    /// method -- that is exactly the defect spec 011 T7 exists to end. Prefer
    /// [`Self::to_compressed_shell_reported`] in new code.
    fn to_compressed_shell(
        &self,
        table: &Table,
    ) -> Result<CompressedShell<Point3, Curve3D, Surface>, StepConvertingError> {
        self.to_compressed_shell_reported(table)
            .map(|(shell, _)| shell)
    }

    /// Convert preserving exact trim curves, DISCARDING the loss report.
    /// Prefer [`Self::to_compressed_trimmed_shell_reported`] in new code.
    fn to_compressed_trimmed_shell(
        &self,
        table: &Table,
    ) -> Result<
        CompressedTrimmedShell<Point3, Curve3D, Surface, step_geometry::StepParameterCurve>,
        StepConvertingError,
    > {
        self.to_compressed_trimmed_shell_reported(table)
            .map(|(shell, _)| shell)
    }
}

impl StepShell for ShellHolder {
    fn to_compressed_shell_reported(
        &self,
        table: &Table,
    ) -> Result<(CompressedShell<Point3, Curve3D, Surface>, ShellLoadReport), StepConvertingError>
    {
        table
            .compressed_shell_with_sources_reported(self)
            .map(|(shell, report, _)| (shell, report))
    }

    fn to_compressed_trimmed_shell_reported(
        &self,
        table: &Table,
    ) -> Result<
        (
            CompressedTrimmedShell<Point3, Curve3D, Surface, step_geometry::StepParameterCurve>,
            ShellLoadReport,
        ),
        StepConvertingError,
    > {
        let sink = LossSink::default();
        let connectivity = table.source_records.shell_connectivity(table, self, None);
        let (vertices, vidx_map) = table.shell_vertices(self, &sink);
        let (edges, eidx_map) = table.shell_edges(self, &vidx_map, &sink, &connectivity);
        let faces =
            table.shell_trimmed_faces(self, &eidx_map, &vidx_map, &edges, &sink, &connectivity);
        Ok((
            CompressedTrimmedShell {
                vertices,
                edges,
                faces,
            },
            sink.into_report(),
        ))
    }
}

impl StepShell for OrientedShellHolder {
    fn to_compressed_shell_reported(
        &self,
        table: &Table,
    ) -> Result<(CompressedShell<Point3, Curve3D, Surface>, ShellLoadReport), StepConvertingError>
    {
        let PlaceHolder::Ref(Name::Entity(idx)) = &self.shell_element else {
            return Err("failed to reference shell".into());
        };
        let Some(shell) = table.shell.get(idx) else {
            return Err("failed to reference shell".into());
        };
        let (mut res, report) = shell.to_compressed_shell_reported(table)?;
        if !self.orientation {
            for face in &mut res.faces {
                face.orientation = !face.orientation;
            }
        }
        Ok((res, report))
    }

    fn to_compressed_trimmed_shell_reported(
        &self,
        table: &Table,
    ) -> Result<
        (
            CompressedTrimmedShell<Point3, Curve3D, Surface, step_geometry::StepParameterCurve>,
            ShellLoadReport,
        ),
        StepConvertingError,
    > {
        let PlaceHolder::Ref(Name::Entity(idx)) = &self.shell_element else {
            return Err("failed to reference shell".into());
        };
        let Some(shell) = table.shell.get(idx) else {
            return Err("failed to reference shell".into());
        };
        let (mut res, report) = shell.to_compressed_trimmed_shell_reported(table)?;
        if !self.orientation {
            res.faces.iter_mut().for_each(|face| {
                face.orientation = !face.orientation;
            });
        }
        Ok((res, report))
    }
}

impl StepShell for ShellAnyHolder {
    fn to_compressed_shell_reported(
        &self,
        table: &Table,
    ) -> Result<(CompressedShell<Point3, Curve3D, Surface>, ShellLoadReport), StepConvertingError>
    {
        match self {
            ShellAnyHolder::OrientedShell(shell) => shell.to_compressed_shell_reported(table),
            ShellAnyHolder::Shell(shell) => shell.to_compressed_shell_reported(table),
        }
    }

    fn to_compressed_trimmed_shell_reported(
        &self,
        table: &Table,
    ) -> Result<
        (
            CompressedTrimmedShell<Point3, Curve3D, Surface, step_geometry::StepParameterCurve>,
            ShellLoadReport,
        ),
        StepConvertingError,
    > {
        match self {
            ShellAnyHolder::OrientedShell(shell) => {
                shell.to_compressed_trimmed_shell_reported(table)
            }
            ShellAnyHolder::Shell(shell) => shell.to_compressed_trimmed_shell_reported(table),
        }
    }
}

#[cfg(test)]
mod tests;

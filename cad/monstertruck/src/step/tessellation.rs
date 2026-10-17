//! Checked source-bound STEP tessellation in millimetres.
//!
//! Geometry and all meshing tolerances use the existing millimetre-normalized
//! representation. Closure is re-proved against each exact normalized surface.
//! This returns a meshing attempt, not whole-shell or assembly qualification.
use monstertruck_io::step::load::convert::{SourceBoundStepShell, StepCompressedShell};
use monstertruck_meshing::{
    prelude::*,
    tessellation::{
        TessellationOptions, robust_cshell_triangulation_with_closed_axis_metadata_report,
    },
};
use monstertruck_modeling::{Point3, PolylineCurve as GeometryPolyline};
use monstertruck_topology::compress::CompressedShell;

use super::units;

pub type StepMesh = CompressedShell<Point3, GeometryPolyline<Point3>, Option<PolygonMesh>>;

type ClosedChartMetadata = Vec<(usize, usize, (f64, f64), Vec<usize>)>;

/// Normalizes a private clone using the checked importer's existing unit rules.
/// Certificates are bound to those actual normalized surfaces before meshing.
/// `mm_per_unit` is the factor resolved for this solid's representation; all
/// spatial options are millimetres. The caller applies occurrence placement once.
/// Also returns the indices of faces meshed by the
/// `options.preview_fallback` route from their shared boundary samples.
pub fn triangulate_source_bound_shell_report(
    source: &SourceBoundStepShell,
    mm_per_unit: f64,
    options: TessellationOptions,
) -> Result<(StepMesh, Vec<usize>), String> {
    if !mm_per_unit.is_finite() || mm_per_unit <= 0.0 {
        return Err("source representation unit must be finite and positive".into());
    }
    if !options.tolerance.is_finite()
        || options.tolerance <= 0.0
        || options
            .edge_angular_tolerance
            .is_some_and(|a| !a.is_finite() || a <= 0.0)
    {
        return Err("meshing tolerances must be finite and positive".into());
    }
    let mut normalized = source.shell().clone();
    units::normalize_shell_to_mm(&mut normalized, mm_per_unit)?;
    let finite = |p: &Point3| p.x.is_finite() && p.y.is_finite() && p.z.is_finite();
    if !normalized.vertices.iter().all(finite) {
        return Err("unit normalization produced nonfinite source vertices".into());
    }
    // Raw tuple metadata exists only here, after exact binding and re-certification.
    let metadata = closed_chart_metadata(source, &normalized, mm_per_unit)?;
    let (mesh, fallback_faces) = robust_cshell_triangulation_with_closed_axis_metadata_report(
        &normalized,
        options,
        &metadata,
    );
    if !mesh.vertices.iter().all(finite)
        || !mesh.edges.iter().all(|e| e.curve.iter().all(finite))
        || !mesh.faces.iter().all(|f| {
            f.surface
                .as_ref()
                .is_none_or(|m| m.positions().iter().all(finite))
        })
    {
        return Err("meshing produced nonfinite normalized coordinates".into());
    }
    Ok((mesh, fallback_faces))
}

// Metadata never crosses the checked adapter's public interface.
fn closed_chart_metadata(
    source: &SourceBoundStepShell,
    normalized: &StepCompressedShell,
    mm_per_unit: f64,
) -> Result<ClosedChartMetadata, String> {
    if normalized.faces.len() != source.source_face_ids().len() {
        return Err("unit normalization changed source face mapping".into());
    }
    let mut metadata = Vec::new();
    for (index, face) in normalized.faces.iter().enumerate() {
        if face.boundaries.len() != 1 {
            continue;
        }
        let axis = match source.certified_closed_axes()[index] {
            Some([true, false]) => 0,
            Some([false, true]) => 1,
            _ => continue,
        };
        let rebound = source
            .certified_closed_axes_after_uniform_scale(index, &face.surface, mm_per_unit)
            .ok_or("normalized surface does not match its source-bound uniform scale")?;
        if !rebound[axis] {
            return Err("closed-axis equality did not survive unit normalization".into());
        }
        let range = face.surface.try_range_tuple();
        let domain =
            [range.0, range.1][axis].ok_or("certified closed axis has no finite domain")?;
        let mut uses = std::collections::BTreeMap::<usize, Vec<bool>>::new();
        for edge in face.boundaries.iter().flatten() {
            uses.entry(edge.index).or_default().push(edge.orientation);
        }
        let seams = uses
            .into_iter()
            .filter_map(|(edge, orientations)| {
                (orientations.len() == 2 && orientations[0] != orientations[1]).then_some(edge)
            })
            .collect::<Vec<_>>();
        if !seams.is_empty() {
            metadata.push((index, axis, domain, seams));
        }
    }
    Ok(metadata)
}

#[cfg(test)]
mod collapsed_boundary_tests {
    use super::*;
    use monstertruck_io::step::load::{Table, step_p21::parser};
    use monstertruck_modeling::*;

    #[test]
    fn source_knob_low_collapsed_v_boundary_retains_its_face() {
        let exchange =
            parser::parse(include_str!("../../tests/fixtures/knob-low-face9183.step")).unwrap();
        let table = Table::from_data_section(&exchange.data[0]);
        table.entity_report.require_empty().unwrap();
        let source = table
            .to_source_bound_solid_in_representation_checked(&table.manifold_solid_brep[&14], 12)
            .unwrap();
        let shell = &source.shells()[0];
        assert_eq!(shell.source_face_ids(), &[9183]);
        assert_eq!(shell.certified_closed_axes(), &[Some([false, false])]);
        let (mesh, _) = triangulate_source_bound_shell_report(
            shell,
            1.0,
            TessellationOptions {
                tolerance: 0.1,
                edge_angular_tolerance: Some(0.5),
                ..Default::default()
            },
        )
        .unwrap();
        let face = mesh.faces[0].surface.as_ref().expect(
            "source-certified collapsed V boundary must not drop the positive-area source face",
        );
        assert!(!face.faces().is_empty());
        assert_source_face_geometry(&mesh, &shell.shell().faces[0].surface);
    }

    // A valid generic surface may have no inverse at a singular point. Away
    // from that point this wrapper delegates to the actual source BSP inverse;
    // it does not replace evaluation, coefficients, ranges or source proof.
    use monstertruck_modeling as monstertruck_traits;
    #[derive(Clone, Debug, ParametricSurface3D, ParameterDivision2D)]
    struct PoleInverseUnavailable(BsplineSurface<Point3>);
    impl SearchParameter<SurfaceParameter> for PoleInverseUnavailable {
        type Point = Point3;
        fn search_parameter<H: Into<SearchParameterHint2D>>(
            &self,
            point: Point3,
            hint: H,
            trials: usize,
        ) -> Option<(f64, f64)> {
            if point == Point3::origin() {
                None
            } else {
                self.0.search_parameter(point, hint, trials)
            }
        }
    }
    impl SearchNearestParameter<SurfaceParameter> for PoleInverseUnavailable {
        type Point = Point3;
        fn search_nearest_parameter<H: Into<SearchParameterHint2D>>(
            &self,
            point: Point3,
            hint: H,
            trials: usize,
        ) -> Option<(f64, f64)> {
            if point == Point3::origin() {
                None
            } else {
                self.0.search_nearest_parameter(point, hint, trials)
            }
        }
    }

    #[derive(Clone, Debug, ParametricCurve, BoundedCurve, Invertible)]
    struct SampledLine(Line<Point3>);
    impl<S: ParametricSurface> ParameterBoundary2D<S> for SampledLine {}
    impl ParameterDivision1D for SampledLine {
        type Point = Point3;
        fn parameter_division(&self, range: (f64, f64), _: f64) -> (Vec<f64>, Vec<Point3>) {
            let parameters = (0..=4)
                .map(|i| range.0 + (range.1 - range.0) * (i as f64 / 4.0))
                .collect::<Vec<_>>();
            let points = parameters.iter().map(|t| self.evaluate(*t)).collect();
            (parameters, points)
        }
    }

    #[test]
    fn folded_finite_trim_never_uses_a_sampled_boundary_alias_when_inverse_refuses() {
        use monstertruck_topology::compress::{
            CompressedEdge, CompressedEdgeIndex, CompressedFace, CompressedShell,
        };
        let origin = Point3::origin();
        let p1 = Point3::new(1.0, 0.0, 1.0);
        let p2 = Point3::new(0.0, 1.0, 1.0);
        let folded = BsplineSurface::new(
            (KnotVector::bezier_knot(2), KnotVector::bezier_knot(1)),
            vec![
                vec![origin, origin],
                vec![Point3::new(-0.5, 0.0, -0.5), Point3::new(0.0, -0.5, -0.5)],
                vec![p1, p2],
            ],
        );
        assert_eq!(folded.evaluate(0.5, 0.5), origin);
        assert_eq!(
            folded.finite_collapsed_boundaries(),
            Some([[false, false], [false, false]])
        );
        let support = PoleInverseUnavailable(folded);
        assert_eq!(
            support.search_parameter(origin, Some((0.75, 0.5)), 100),
            None
        );
        assert_eq!(support.search_nearest_parameter(origin, None, 100), None);
        let sampled_line = |a: Point3, b: Point3| SampledLine(Line(a, b));
        let shell = CompressedShell {
            vertices: vec![origin, p1, p2],
            edges: vec![
                CompressedEdge {
                    vertices: (0, 1),
                    curve: sampled_line(origin, p1),
                },
                CompressedEdge {
                    vertices: (1, 2),
                    curve: sampled_line(p1, p2),
                },
                CompressedEdge {
                    vertices: (2, 0),
                    curve: sampled_line(p2, origin),
                },
            ],
            faces: vec![CompressedFace {
                boundaries: vec![vec![
                    CompressedEdgeIndex {
                        index: 0,
                        orientation: true,
                    },
                    CompressedEdgeIndex {
                        index: 1,
                        orientation: true,
                    },
                    CompressedEdgeIndex {
                        index: 2,
                        orientation: true,
                    },
                ]],
                orientation: true,
                surface: support,
            }],
            vertex_stable_ids: None,
            edge_stable_ids: None,
            face_stable_ids: None,
        };
        let mesh=monstertruck_meshing::tessellation::robust_cshell_triangulation_with_closed_axis_metadata(
            &shell,TessellationOptions {tolerance:0.1,edge_angular_tolerance:Some(0.5),..Default::default()},&[],
        );
        // This source trim is U in [.5,1]. An unresolved pole must refuse the
        // face, rather than using sampled U=0 and adding the negative sheet.
        if let Some(face) = mesh.faces[0].surface.as_ref() {
            for (point, uv) in face.positions().iter().zip(face.uv_coords()) {
                assert!(
                    uv.x >= 0.5 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0,
                    "the source trim U in [.5,1] cannot be replaced by another pole branch: uv={uv:?} point={point:?}"
                );
                assert!(
                    shell.faces[0].surface.evaluate(uv.x, uv.y).distance(*point) <= 1e-6,
                    "a retained chart must bind each UV to its actual shared XYZ: uv={uv:?} point={point:?} evaluated={:?}",
                    shell.faces[0].surface.evaluate(uv.x, uv.y)
                );
            }
        }
    }

    // Exact finite-boundary metadata is independent of periodicity and of sampled
    // normals. Even a sub-tolerance change to one coefficient must remove the proof.
    fn finite_collapsed_triangle_surface() -> BsplineSurface<Point3> {
        BsplineSurface::new(
            (KnotVector::bezier_knot(1), KnotVector::bezier_knot(1)),
            vec![
                vec![Point3::origin(), Point3::new(1.0, 0.0, 1.0)],
                vec![Point3::origin(), Point3::new(0.0, 1.0, 1.0)],
            ],
        )
    }

    #[test]
    fn finite_collapsed_boundaries_are_exact_axis_and_endpoint_proofs() {
        let surface = finite_collapsed_triangle_surface();
        assert_eq!(
            surface.finite_collapsed_boundaries(),
            Some([[false, false], [true, false]])
        );
        assert_eq!(surface.period_u(), None);
        assert_eq!(surface.period_v(), None);
        assert_eq!(
            ParametricSurface3D::finite_collapsed_boundaries(&&surface),
            surface.finite_collapsed_boundaries()
        );
        assert_eq!(
            Box::new(surface.clone()).finite_collapsed_boundaries(),
            surface.finite_collapsed_boundaries()
        );
        let mut transposed = surface.clone();
        transposed.invert();
        assert_eq!(
            transposed.finite_collapsed_boundaries(),
            Some([[true, false], [false, false]])
        );
        let upper = BsplineSurface::new(
            surface.knot_vectors().clone(),
            surface
                .control_points()
                .iter()
                .map(|row| row.iter().rev().copied().collect())
                .collect(),
        );
        assert_eq!(
            upper.finite_collapsed_boundaries(),
            Some([[false, false], [false, true]])
        );
        let mut near = surface.control_points().clone();
        near[1][0].x = 1e-12;
        let near = BsplineSurface::new(surface.knot_vectors().clone(), near);
        assert_eq!(
            near.finite_collapsed_boundaries(),
            Some([[false, false], [false, false]])
        );
        let flat = Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        assert_eq!(flat.finite_collapsed_boundaries(), None);
    }

    #[test]
    fn collapsed_boundary_proof_refuses_an_interior_pole_alias() {
        let origin = Point3::origin();
        let p1 = Point3::new(1.0, 0.0, 1.0);
        let p2 = Point3::new(0.0, 1.0, 1.0);
        // S=(2u^2-u)*(1-v,v,1). A valid triangular trim U in [.5,1]
        // has its pole at U=.5; the finite U=0 boundary must not claim it.
        let folded = BsplineSurface::new(
            (KnotVector::bezier_knot(2), KnotVector::bezier_knot(1)),
            vec![
                vec![origin, origin],
                vec![origin + (p1 - origin) * -0.5, origin + (p2 - origin) * -0.5],
                vec![p1, p2],
            ],
        );
        for v in [0.0, 0.25, 0.5, 0.75, 1.0] {
            assert_eq!(folded.evaluate(0.0, v), origin);
            assert_eq!(folded.evaluate(0.5, v), origin);
            assert_eq!(folded.evaluate(1.0, v), Point3::new(1.0 - v, v, 1.0));
        }
        assert_eq!(
            folded.evaluate(0.75, 0.5),
            Point3::new(0.1875, 0.1875, 0.375)
        );
        assert_eq!(
            folded.finite_collapsed_boundaries(),
            Some([[false, false], [false, false]]),
            "collapse alone cannot authorize moving the [.5,1] trim pole to U=0"
        );
    }

    #[test]
    fn collapsed_boundary_proof_refuses_a_finite_flat_band() {
        let origin = Point3::origin();
        let flat_band = BsplineSurface::new(
            (
                KnotVector::from(vec![0.0, 0.0, 0.5, 1.0, 1.0]),
                KnotVector::bezier_knot(1),
            ),
            vec![
                vec![origin, origin],
                vec![origin, origin],
                vec![Point3::new(1.0, 0.0, 1.0), Point3::new(0.0, 1.0, 1.0)],
            ],
        );
        assert_eq!(flat_band.evaluate(0.25, 0.5), origin);
        assert_eq!(
            flat_band.finite_collapsed_boundaries(),
            Some([[false, false], [false, false]]),
            "a pole prefix longer than the normal degree leaves an interior flat band"
        );
    }

    #[test]
    fn finite_collapsed_boundary_refuses_invalid_or_unproved_spline_domains() {
        let surface = finite_collapsed_triangle_surface();
        let padded = BsplineSurface::new(
            (
                KnotVector::from(vec![-1.0, 0.0, 1.0, 2.0]),
                surface.knot_vector_v().clone(),
            ),
            surface.control_points().clone(),
        );
        assert_eq!(padded.finite_collapsed_boundaries(), None);
        for bad_knots in [
            vec![0.0, 0.0, 0.0, 1.0],
            vec![0.0, 0.0, f64::from_bits(1), f64::from_bits(1)],
        ] {
            let bad = BsplineSurface::new_unchecked(
                (surface.knot_vector_u().clone(), KnotVector::from(bad_knots)),
                surface.control_points().clone(),
            );
            assert_eq!(bad.finite_collapsed_boundaries(), None);
        }
        let mut nonfinite = surface.control_points().clone();
        nonfinite[1][1].z = f64::INFINITY;
        let bad = BsplineSurface::new_unchecked(surface.knot_vectors().clone(), nonfinite);
        assert_eq!(bad.finite_collapsed_boundaries(), None);
        let bad = BsplineSurface::<Point3>::new_unchecked(surface.knot_vectors().clone(), vec![]);
        assert_eq!(bad.finite_collapsed_boundaries(), None);
    }

    #[test]
    fn finite_collapsed_boundary_processor_transports_only_safe_instance_axes() {
        let surface = finite_collapsed_triangle_surface();
        let affine = Matrix4::new(
            2.0, 0.0, 0.0, 0.0, 0.5, 3.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 7.0, -2.0, 4.0, 1.0,
        );
        let mut processed = Processor::new(surface.clone());
        processed.transform_by(affine);
        assert_eq!(
            processed.finite_collapsed_boundaries(),
            surface.finite_collapsed_boundaries()
        );
        processed.invert();
        assert_eq!(
            processed.finite_collapsed_boundaries(),
            Some([[true, false], [false, false]])
        );
        for bad in [
            Matrix4::from_scale(0.0),
            Matrix4::new(
                1.0, 0.0, 0.0, 1e-300, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ),
            Matrix4::from_scale(f64::INFINITY),
        ] {
            let mut processed = Processor::new(surface.clone());
            processed.transform_by(bad);
            assert_eq!(processed.finite_collapsed_boundaries(), None);
        }
    }

    #[test]
    fn collapsed_boundary_transport_refuses_exact_singular_affine_cancellation() {
        // Exact integer row3=row1+row2. Floating determinant cancellation
        // reports 8192 here, so a nonzero floating result is not a rank proof.
        let matrix = Matrix4::new(
            9886238.0, 2678639.0, 12564877.0, 0.0, 4667266.0, 7090294.0, 11757560.0, 0.0, 109032.0,
            5708457.0, 5817489.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        );
        let origin = Point3::origin();
        // This exact integer nullvector is a second source preimage of the
        // transformed pole. The untransformed polynomial patch has area.
        let null = Point3::new(25869818333154.0, -56143107147318.0, 57594413242998.0);
        let source = BsplineSurface::new(
            (KnotVector::bezier_knot(1), KnotVector::bezier_knot(1)),
            vec![vec![origin, Point3::new(0.0, 0.0, 1.0)], vec![origin, null]],
        );
        assert!(
            source
                .derivative_u(0.5, 0.5)
                .cross(source.derivative_v(0.5, 0.5))
                .magnitude2()
                > 0.0
        );
        assert_eq!(
            source.finite_collapsed_boundaries(),
            Some([[false, false], [true, false]])
        );
        let processed = Processor::with_transform(source, matrix);
        assert_eq!(
            processed.finite_collapsed_boundaries(),
            None,
            "an exact singular source transform cannot preserve exclusive pole preimages"
        );
        let rotation = Matrix4::from_axis_angle(Vector3::new(1.0, 2.0, 3.0).normalize(), Rad(0.73));
        let rotated = Processor::with_transform(finite_collapsed_triangle_surface(), rotation);
        assert_eq!(
            rotated.finite_collapsed_boundaries(),
            Some([[false, false], [true, false]])
        );
    }

    include!("../../tests/fixtures/knob-low-face9183-witnesses.rs");

    fn distance_to_triangle(point: Point3, [a, b, c]: [Point3; 3]) -> f64 {
        let segment = |a: Point3, b: Point3| {
            let direction = b - a;
            let length = direction.magnitude2();
            if length == 0.0 {
                return point.distance(a);
            }
            point.distance(a + direction * ((point - a).dot(direction) / length).clamp(0.0, 1.0))
        };
        let ab = b - a;
        let ac = c - a;
        let normal = ab.cross(ac);
        let denominator = normal.magnitude2();
        if denominator > 0.0 {
            let offset = point - a;
            let u = (offset.dot(ab) * ac.magnitude2() - offset.dot(ac) * ab.dot(ac)) / denominator;
            let v = (offset.dot(ac) * ab.magnitude2() - offset.dot(ab) * ab.dot(ac)) / denominator;
            if u >= 0.0 && v >= 0.0 && u + v <= 1.0 {
                return offset.dot(normal).abs() / denominator.sqrt();
            }
        }
        segment(a, b).min(segment(b, c)).min(segment(c, a))
    }

    fn assert_source_face_geometry(
        mesh: &StepMesh,
        support: &monstertruck_io::step::load::step_geometry::Surface,
    ) {
        let face = mesh.faces[0]
            .surface
            .as_ref()
            .expect("complete source face");
        assert!(!face.faces().is_empty());
        for (point, uv) in face.positions().iter().zip(face.uv_coords()) {
            assert!(
                support.evaluate(uv.x, uv.y).distance(*point) <= 0.01,
                "UV and authoritative XYZ must remain within declared source precision"
            );
        }
        for edge_use in &mesh.faces[0].boundaries[0] {
            let sampled = &mesh.edges[edge_use.index].curve;
            assert_eq!(sampled.len(), 5);
            for point in &sampled[1..sampled.len() - 1] {
                assert!(
                    face.positions().contains(point),
                    "shared interior edge sample was changed or dropped"
                );
            }
        }
        let triangles = face
            .tri_faces()
            .iter()
            .map(|tri| tri.map(|v| face.positions()[v.pos]))
            .collect::<Vec<_>>();
        for tri in &triangles {
            assert!(
                (tri[1] - tri[0]).cross(tri[2] - tri[0]).magnitude2() > 0.0,
                "source-equivalent pole identities must not emit a zero physical facet"
            );
        }
        let mut maximum_distance: f64 = 0.0;
        for (point, normal) in SOURCE_WITNESSES {
            let point = Point3::from(point);
            let normal = Vector3::from(normal);
            let (distance, tri) = triangles
                .iter()
                .map(|tri| (distance_to_triangle(point, *tri), tri))
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .unwrap();
            maximum_distance = maximum_distance.max(distance);
            assert!(
                distance <= 0.1,
                "independent source interior witness missing at preview tolerance: {distance}"
            );
            let mut facet_normal = (tri[1] - tri[0]).cross(tri[2] - tri[0]);
            if !mesh.faces[0].orientation {
                facet_normal = -facet_normal;
            }
            assert!(
                facet_normal.dot(normal) > 0.0,
                "material orientation disagrees with independent source normal"
            );
        }
        println!(
            "source9183 geometry positions={} triangles={} witnesses=25 maximum_distance_mm={maximum_distance}",
            face.positions().len(),
            triangles.len()
        );
    }

    #[test]
    fn source_knob_low_collapsed_u_reparameterization_preserves_source_geometry() {
        let exchange =
            parser::parse(include_str!("../../tests/fixtures/knob-low-face9183.step")).unwrap();
        let table = Table::from_data_section(&exchange.data[0]);
        let source = table
            .to_source_bound_solid_in_representation_checked(&table.manifold_solid_brep[&14], 12)
            .unwrap();
        let mut shell = source.shells()[0].shell().clone();
        shell.faces[0].surface.invert();
        shell.faces[0].orientation = !shell.faces[0].orientation;
        let mesh = monstertruck_meshing::tessellation::robust_cshell_triangulation_with_closed_axis_metadata(
            &shell, TessellationOptions { tolerance: 0.1, edge_angular_tolerance: Some(0.5), ..Default::default() }, &[],
        );
        assert_source_face_geometry(&mesh, &shell.faces[0].surface);
    }

    #[test]
    fn higher_order_u_pole_bridge_retains_the_proved_axis_and_source_area() {
        use monstertruck_topology::compress::{
            CompressedEdge, CompressedEdgeIndex, CompressedFace, CompressedShell,
        };
        let origin = Point3::origin();
        let p1 = Point3::new(1.0, 0.0, 1.0);
        let p2 = Point3::new(0.0, 1.0, 1.0);
        // Independently S(u,v)=u^2*(1-v,v,1), with both first derivatives zero
        // at its exact U=0 point boundary. Its physical image is a full triangle.
        let support = BsplineSurface::new(
            (KnotVector::bezier_knot(2), KnotVector::bezier_knot(1)),
            vec![vec![origin, origin], vec![origin, origin], vec![p1, p2]],
        );
        assert_eq!(support.evaluate(0.5, 0.5), Point3::new(0.125, 0.125, 0.25));
        assert_eq!(support.derivative_u(0.0, 0.5), Vector3::zero());
        assert_eq!(support.derivative_v(0.0, 0.5), Vector3::zero());
        assert_eq!(
            support.finite_collapsed_boundaries(),
            Some([[true, false], [false, false]])
        );
        let shell = CompressedShell {
            vertices: vec![origin, p1, p2],
            edges: vec![
                CompressedEdge {
                    vertices: (0, 1),
                    curve: Line(origin, p1),
                },
                CompressedEdge {
                    vertices: (1, 2),
                    curve: Line(p1, p2),
                },
                CompressedEdge {
                    vertices: (2, 0),
                    curve: Line(p2, origin),
                },
            ],
            faces: vec![CompressedFace {
                boundaries: vec![vec![
                    CompressedEdgeIndex {
                        index: 0,
                        orientation: true,
                    },
                    CompressedEdgeIndex {
                        index: 1,
                        orientation: true,
                    },
                    CompressedEdgeIndex {
                        index: 2,
                        orientation: true,
                    },
                ]],
                orientation: true,
                surface: support.clone(),
            }],
            vertex_stable_ids: None,
            edge_stable_ids: None,
            face_stable_ids: None,
        };
        let mesh = monstertruck_meshing::tessellation::robust_cshell_triangulation_with_closed_axis_metadata(
            &shell,TessellationOptions {tolerance:0.1,edge_angular_tolerance:Some(0.5),..Default::default()},&[],
        );
        let face = mesh.faces[0]
            .surface
            .as_ref()
            .expect("a zero partial must not select the wrong boundary bridge axis");
        for (point, uv) in face.positions().iter().zip(face.uv_coords()) {
            assert!(
                support.evaluate(uv.x, uv.y).distance(*point) <= 1e-6,
                "bridge UV/XYZ must preserve the actual source point: uv={uv:?} point={point:?} evaluated={:?} residual={}",
                support.evaluate(uv.x, uv.y),
                support.evaluate(uv.x, uv.y).distance(*point)
            );
        }
        let triangles = face
            .tri_faces()
            .iter()
            .map(|tri| tri.map(|v| face.positions()[v.pos]))
            .collect::<Vec<_>>();
        let area: f64 = triangles
            .iter()
            .map(|tri| (tri[1] - tri[0]).cross(tri[2] - tri[0]).magnitude() * 0.5)
            .sum();
        assert!(
            (area - 3.0f64.sqrt() * 0.5).abs() < 1e-10,
            "the source triangular material area must be retained: {area}"
        );
        for u in [0.125, 0.25, 0.5, 0.75, 0.875] {
            for v in [0.125, 0.25, 0.5, 0.75, 0.875] {
                let point = Point3::new(u * u * (1.0 - v), u * u * v, u * u);
                let distance = triangles
                    .iter()
                    .map(|tri| distance_to_triangle(point, *tri))
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    distance <= 1e-6,
                    "source material interior must remain covered: {distance}"
                );
            }
        }
    }
}

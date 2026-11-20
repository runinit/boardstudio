//! Unit tests for the parent module.
//!
//! Split out of the module file so the source stays readable; the module
//! path is unchanged.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use super::{
    boundary::{PolyBoundary, PolyBoundaryPiece, SurfacePoint},
    mesh::trimming_tessellation as try_trimming_tessellation,
    *,
};
use monstertruck_geometry::prelude::{Line, Matrix4, Plane, Processor, RevolutionSurface, Sphere};
use monstertruck_topology::{Edge, Face, Shell, Vertex, Wire};

#[derive(Clone)]
struct SingularMetricPlane;

impl ParametricSurface for SingularMetricPlane {
    type Point = Point3;
    type Vector = Vector3;

    fn evaluate(&self, u: f64, v: f64) -> Point3 {
        Point3::new(u, v, 0.0)
    }
    fn derivative_u(&self, _: f64, _: f64) -> Vector3 {
        Vector3::zero()
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
        match (m, n) {
            (0, 0) => self.evaluate(u, v).to_vec(),
            (1, 0) => self.derivative_u(u, v),
            (0, 1) => self.derivative_v(u, v),
            _ => Vector3::zero(),
        }
    }
}

impl ParametricSurface3D for SingularMetricPlane {}

impl ParameterDivision2D for SingularMetricPlane {
    fn parameter_division(&self, range: ((f64, f64), (f64, f64)), _: f64) -> (Vec<f64>, Vec<f64>) {
        let divide = |(min, max): (f64, f64)| {
            (0..=4)
                .map(|index| min + (max - min) * index as f64 / 4.0)
                .collect()
        };
        (divide(range.0), divide(range.1))
    }
}

impl SearchParameter<SurfaceParameter> for SingularMetricPlane {
    type Point = Point3;

    fn search_parameter<H: Into<SearchParameterHint2D>>(
        &self,
        point: Point3,
        _: H,
        _: usize,
    ) -> Option<(f64, f64)> {
        Some((point.x, point.y))
    }
}

fn singular_metric_shell() -> Shell<Point3, Line<Point3>, SingularMetricPlane> {
    let points = [
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
    ];
    let vertices = Vertex::from_points(points);
    let wire: Wire<Point3, Line<Point3>> = (0..4)
        .map(|index| {
            let next = (index + 1) % 4;
            Edge::new(
                &vertices[index],
                &vertices[next],
                Line(points[index], points[next]),
            )
        })
        .collect();
    vec![Face::new(vec![wire], SingularMetricPlane)].into()
}

// Existing successful-path tests keep their concise call shape while the
// production helper now returns a typed refusal for invalid refinement.
fn trimming_tessellation<S: PreMeshableSurface>(
    surface: &S,
    boundary: &PolyBoundary,
    tolerance: f64,
    primitive_config: TessellationPrimitiveOptions,
    face_idx: usize,
) -> PolygonMesh {
    try_trimming_tessellation(surface, boundary, tolerance, primitive_config, face_idx)
        .expect("fixture CDT metric refinement should succeed")
}

#[derive(Clone, Debug)]
struct SampledTrim {
    samples: Vec<Point2>,
}

impl ExactTrimBoundary2D for SampledTrim {
    fn exact_trim_boundary_2d(&self, _: f64) -> Vec<Point2> {
        self.samples.clone()
    }
}

fn sampled_trim(samples: impl IntoIterator<Item = Point2>) -> SampledTrim {
    SampledTrim {
        samples: samples.into_iter().collect(),
    }
}

fn trimmed_plane_shell(
    loops: Vec<Vec<Point2>>,
) -> CompressedTrimmedShell<Point3, Line<Point3>, Plane, SampledTrim> {
    let surface = Plane::xy();
    let mut vertices = Vec::new();
    let mut edges = Vec::new();
    let mut boundaries = Vec::new();
    for points in loops {
        let segments = if points.len() == 1 {
            vec![(points[0], points[0])]
        } else {
            points
                .iter()
                .copied()
                .circular_tuple_windows()
                .collect::<Vec<_>>()
        };
        let mut wire = Vec::new();
        for (front, back) in segments {
            let front_3d = surface.evaluate(front.x, front.y);
            let back_3d = surface.evaluate(back.x, back.y);
            let front_index = vertices.len();
            vertices.push(front_3d);
            let back_index = vertices.len();
            vertices.push(back_3d);
            let edge_index = edges.len();
            edges.push(CompressedEdge {
                vertices: (front_index, back_index),
                curve: Line(front_3d, back_3d),
            });
            wire.push(CompressedEdgeUse::from((
                edge_index,
                true,
                Some(sampled_trim([front, back])),
            )));
        }
        boundaries.push(wire);
    }
    CompressedTrimmedShell {
        vertices,
        edges,
        faces: vec![CompressedTrimmedFace {
            boundaries,
            orientation: true,
            surface,
        }],
    }
}

#[test]
fn collapsed_outer_loop_must_not_become_the_full_plane() {
    let center = Point2::new(0.5, 0.5);
    let shell = trimmed_plane_shell(vec![vec![center]]);
    let mesh = trimmed_cshell_tessellation(
        &shell,
        0.01,
        |surface: &Plane, point: Point3, _| surface.search_parameter(point, None, 100),
        TessellationPrimitiveOptions::default(),
        None,
    );

    assert!(mesh.faces[0].surface.is_none());
}

#[test]
fn compressed_shell_collapsed_trim_must_not_become_the_full_plane() {
    let surface = Plane::xy();
    let point = surface.evaluate(0.5, 0.5);
    let shell = CompressedShell {
        vertices: vec![point],
        edges: vec![CompressedEdge {
            vertices: (0, 0),
            curve: Line(point, point),
        }],
        faces: vec![CompressedFace {
            boundaries: vec![vec![CompressedEdgeIndex {
                index: 0,
                orientation: true,
            }]],
            orientation: true,
            surface,
        }],
        vertex_stable_ids: None,
        edge_stable_ids: None,
        face_stable_ids: None,
    };
    let mesh = cshell_tessellation(
        &shell,
        0.01,
        |surface: &Plane, point: Point3, _| surface.search_parameter(point, None, 100),
        TessellationPrimitiveOptions::default(),
        None,
    );

    assert!(mesh.faces[0].surface.is_none());
}

#[test]
fn collapsed_hole_must_not_be_silently_filled() {
    let center = Point2::new(0.5, 0.5);
    let shell = trimmed_plane_shell(vec![
        vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ],
        vec![center],
    ]);
    let mesh = trimmed_cshell_tessellation(
        &shell,
        0.01,
        |surface: &Plane, point: Point3, _| surface.search_parameter(point, None, 100),
        TessellationPrimitiveOptions::default(),
        None,
    );

    assert!(mesh.faces[0].surface.is_none());
}

#[test]
fn untrimmed_sphere_keeps_its_bounded_full_surface_path() {
    let sphere = Sphere::new(Point3::origin(), 5.0);
    let (face, metric_failure) = shell_create_polygon(
        &sphere,
        vec![Wire::new()],
        true,
        0.5,
        |_: &Sphere, _: Point3, _: Option<(f64, f64)>| None,
        TessellationPrimitiveOptions::default(),
        Some(0),
    );
    assert_eq!(metric_failure, None);

    let mesh = face
        .surface()
        .expect("untrimmed sphere should still tessellate");
    assert!(!mesh.tri_faces().is_empty());
}

fn square_piece(surface: &Plane, min: f64, max: f64, ccw: bool) -> PolyBoundaryPiece {
    let mut boundary = vec![
        Point2::new(min, min),
        Point2::new(max, min),
        Point2::new(max, max),
        Point2::new(min, max),
    ];
    if !ccw {
        boundary.reverse();
    }
    boundary.push(boundary[0]);
    PolyBoundaryPiece(
        boundary
            .into_iter()
            .map(|uv| SurfacePoint::from((uv, surface.subs(uv.x, uv.y))))
            .collect(),
    )
}

fn cylinder_patch_piece(surface: &RevolutionSurface<Line<Point3>>) -> PolyBoundaryPiece {
    let lower = (0..=16).map(|index| Point2::new(0.0, TAU * index as f64 / 16.0));
    let right = [Point2::new(0.5, TAU), Point2::new(1.0, TAU)];
    let upper = (0..=16)
        .rev()
        .map(|index| Point2::new(1.0, TAU * index as f64 / 16.0));
    let left = [Point2::new(0.5, 0.0), Point2::new(0.0, 0.0)];
    PolyBoundaryPiece(
        lower
            .chain(right)
            .chain(upper)
            .chain(left)
            .map(|uv| SurfacePoint::from((uv, surface.subs(uv.x, uv.y))))
            .collect(),
    )
}

fn cylinder_ring_piece(
    surface: &RevolutionSurface<Line<Point3>>,
    profile_parameter: f64,
    reversed: bool,
) -> PolyBoundaryPiece {
    let mut boundary = (0..=16)
        .map(|index| Point2::new(profile_parameter, TAU * index as f64 / 16.0))
        .chain([Point2::new(profile_parameter, 0.0)])
        .collect::<Vec<_>>();
    if reversed {
        boundary.reverse();
    }
    PolyBoundaryPiece(
        boundary
            .into_iter()
            .map(|uv| SurfacePoint::from((uv, surface.subs(uv.x, uv.y))))
            .collect(),
    )
}

fn shortened_cylinder_ring_piece(
    surface: &RevolutionSurface<Line<Point3>>,
    profile_parameter: f64,
    reversed: bool,
) -> PolyBoundaryPiece {
    let mut boundary = (0..=16)
        .map(|index| Point2::new(profile_parameter, TAU * 0.95 * index as f64 / 16.0))
        .chain([Point2::new(profile_parameter, 0.0)])
        .collect::<Vec<_>>();
    if reversed {
        boundary.reverse();
    }
    PolyBoundaryPiece(
        boundary
            .into_iter()
            .map(|uv| SurfacePoint::from((uv, surface.subs(uv.x, uv.y))))
            .collect(),
    )
}

fn step_cylinder_surface() -> Processor<RevolutionSurface<Line<Point3>>, Matrix4> {
    let profile = Line(Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 0.0, 1.0));
    let mut surface: Processor<_, Matrix4> = Processor::new(RevolutionSurface::by_revolution(
        profile,
        Point3::origin(),
        Vector3::unit_z(),
    ));
    surface.invert();
    surface
}

fn double_lap_step_cylinder_piece(
    surface: &Processor<RevolutionSurface<Line<Point3>>, Matrix4>,
) -> PolyBoundaryPiece {
    let lower = (0..=16).map(|index| Point2::new(-TAU + TAU * index as f64 / 16.0, 0.0));
    let right = [Point2::new(0.0, 1.0), Point2::new(TAU, 1.0)];
    let upper = (0..=16)
        .rev()
        .map(|index| Point2::new(TAU * index as f64 / 16.0, 1.0));
    let left = [Point2::new(0.0, 0.0), Point2::new(-TAU, 0.0)];
    PolyBoundaryPiece(
        lower
            .chain(right)
            .chain(upper)
            .chain(left)
            .map(|uv| SurfacePoint::from((uv, surface.subs(uv.x, uv.y))))
            .collect(),
    )
}

fn shortened_step_cylinder_ring_piece(
    surface: &Processor<RevolutionSurface<Line<Point3>>, Matrix4>,
    height_parameter: f64,
    reversed: bool,
) -> PolyBoundaryPiece {
    let mut boundary = (0..=16)
        .map(|index| Point2::new(TAU * 0.95 * index as f64 / 16.0, height_parameter))
        .chain([Point2::new(0.0, height_parameter)])
        .collect::<Vec<_>>();
    if reversed {
        boundary.reverse();
    }
    PolyBoundaryPiece(
        boundary
            .into_iter()
            .map(|uv| SurfacePoint::from((uv, surface.subs(uv.x, uv.y))))
            .collect(),
    )
}

fn shifted_step_cylinder_ring_piece(
    surface: &Processor<RevolutionSurface<Line<Point3>>, Matrix4>,
    height_parameter: f64,
    offset: f64,
    reversed: bool,
) -> PolyBoundaryPiece {
    let mut boundary = (0..=16)
        .map(|index| Point2::new(offset + TAU * 0.95 * index as f64 / 16.0, height_parameter))
        .chain([Point2::new(offset, height_parameter)])
        .collect::<Vec<_>>();
    if reversed {
        boundary.reverse();
    }
    PolyBoundaryPiece(
        boundary
            .into_iter()
            .map(|uv| SurfacePoint::from((uv, surface.subs(uv.x, uv.y))))
            .collect(),
    )
}

fn seam_crossing_step_cylinder_ring_piece(
    surface: &Processor<RevolutionSurface<Line<Point3>>, Matrix4>,
    height_parameter: f64,
    reversed: bool,
) -> PolyBoundaryPiece {
    seam_crossing_step_cylinder_ring_piece_from(surface, height_parameter, PI, reversed)
}

fn seam_crossing_step_cylinder_ring_piece_from(
    surface: &Processor<RevolutionSurface<Line<Point3>>, Matrix4>,
    height_parameter: f64,
    start_angle: f64,
    reversed: bool,
) -> PolyBoundaryPiece {
    let mut boundary = (0..10)
        .map(|index| {
            Point2::new(
                (start_angle - TAU * index as f64 / 10.0).rem_euclid(TAU),
                height_parameter,
            )
        })
        .chain([Point2::new(start_angle.rem_euclid(TAU), height_parameter)])
        .collect::<Vec<_>>();
    if reversed {
        boundary.reverse();
    }
    PolyBoundaryPiece(
        boundary
            .into_iter()
            .map(|uv| SurfacePoint::from((uv, surface.subs(uv.x, uv.y))))
            .collect(),
    )
}

fn seam_crossing_step_cylinder_patch_piece(
    surface: &Processor<RevolutionSurface<Line<Point3>>, Matrix4>,
) -> PolyBoundaryPiece {
    let upper = (0..=4)
        .map(|index| Point2::new((TAU - FRAC_PI_2 * index as f64 / 4.0).rem_euclid(TAU), 1.0));
    let left = [Point2::new(FRAC_PI_2 * 3.0, 0.0)];
    let lower = (0..=4)
        .rev()
        .map(|index| Point2::new((TAU - FRAC_PI_2 * index as f64 / 4.0).rem_euclid(TAU), 0.0));
    let close = [Point2::new(0.0, 1.0)];
    PolyBoundaryPiece(
        upper
            .chain(left)
            .chain(lower)
            .chain(close)
            .map(|uv| SurfacePoint::from((uv, surface.subs(uv.x, uv.y))))
            .collect(),
    )
}

#[test]
fn parameter_curve_trim_samples_curved_surface_lift() {
    let profile = Line(Point3::new(0.25, 0.0, 0.0), Point3::new(0.25, 0.0, 1.0));
    let surface = RevolutionSurface::by_revolution(profile, Point3::origin(), Vector3::unit_z());
    let trim = ParameterCurve::new(
        Line(Point2::new(0.5, 0.0), Point2::new(0.5, FRAC_PI_2)),
        surface,
    );

    let uv_points = trim.exact_trim_boundary_2d(0.01);
    let lifted = uv_points
        .iter()
        .map(|uv| trim.surface().subs(uv.x, uv.y))
        .collect::<Vec<_>>();
    let length = lifted
        .windows(2)
        .map(|window| window[0].distance(window[1]))
        .sum::<f64>();
    let direct = lifted
        .first()
        .zip(lifted.last())
        .map(|(front, back)| front.distance(*back))
        .unwrap_or(0.0);

    assert!(
        uv_points.len() > 4,
        "curved lifted trims need interior UV samples",
    );
    assert!(length / direct > 1.05);
}

#[test]
fn shared_polyline_preserves_exact_trim_samples() {
    let profile = Line(Point3::new(2.5, 0.0, 0.0), Point3::new(2.5, 0.0, 1.0));
    let mut surface: Processor<_, Matrix4> = Processor::new(RevolutionSurface::by_revolution(
        profile,
        Point3::origin(),
        Vector3::unit_z(),
    ));
    surface.invert();
    let trim = ParameterCurve::new(
        Line(Point2::new(TAU * 0.75, 8.0), Point2::new(-TAU * 0.25, 8.0)),
        surface,
    );
    let trim_points = trim.exact_trim_boundary_2d(0.138564065);
    let polyline = polyline_from_trim_curve(
        trim.surface(),
        &trim,
        (0, 1),
        &[
            trim.surface().subs(TAU * 0.75, 8.0),
            trim.surface().subs(-TAU * 0.25, 8.0),
        ],
        0.138564065,
    )
    .expect("exact trim should produce a shared edge polyline");

    assert_eq!(polyline.len(), trim_points.len());
}

#[test]
fn shared_polyline_densifies_topological_edge_curve() {
    let surface = Plane::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
    );
    let shell = CompressedTrimmedShell {
        vertices: vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)],
        edges: vec![CompressedEdge {
            vertices: (0, 1),
            curve: Line(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)),
        }],
        faces: vec![CompressedTrimmedFace {
            boundaries: vec![vec![CompressedEdgeUse::from((
                0,
                true,
                Some(sampled_trim([
                    Point2::new(0.0, 0.0),
                    Point2::new(0.5, 1.0),
                    Point2::new(1.0, 0.0),
                ])),
            ))]],
            orientation: true,
            surface,
        }],
    };

    let meshed = trimmed_cshell_tessellation(
        &shell,
        0.01,
        |surface: &Plane, point: Point3, _| surface.search_parameter(point, None, 100),
        TessellationPrimitiveOptions::default(),
        None,
    );
    let middle = meshed.edges[0].curve[1];

    assert_eq!(meshed.edges[0].curve.len(), 3);
    assert!((middle.x - 0.5).abs() <= TOLERANCE);
    assert!(middle.y.abs() <= TOLERANCE);
}

#[test]
fn trimmed_cshell_reuses_shared_edge_samples_for_exact_trims() {
    let profile = Line(Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 0.0, 1.0));
    let surface = RevolutionSurface::by_revolution(profile, Point3::origin(), Vector3::unit_z());
    let curve_on_surface = |front, back| ParameterCurve::new(Line(front, back), surface);
    let vertices = vec![
        surface.subs(0.0, 0.0),
        surface.subs(0.0, FRAC_PI_2),
        surface.subs(1.0, FRAC_PI_2),
        surface.subs(1.0, 0.0),
    ];
    let edges = vec![
        CompressedEdge {
            vertices: (0, 1),
            curve: curve_on_surface(Point2::new(0.0, 0.0), Point2::new(0.0, FRAC_PI_2)),
        },
        CompressedEdge {
            vertices: (1, 2),
            curve: curve_on_surface(Point2::new(0.0, FRAC_PI_2), Point2::new(1.0, FRAC_PI_2)),
        },
        CompressedEdge {
            vertices: (2, 3),
            curve: curve_on_surface(Point2::new(1.0, FRAC_PI_2), Point2::new(1.0, 0.0)),
        },
        CompressedEdge {
            vertices: (3, 0),
            curve: curve_on_surface(Point2::new(1.0, 0.0), Point2::new(0.0, 0.0)),
        },
    ];
    let right = sampled_trim([Point2::new(0.0, FRAC_PI_2), Point2::new(1.0, FRAC_PI_2)]);
    let top = sampled_trim([Point2::new(1.0, FRAC_PI_2), Point2::new(1.0, 0.0)]);
    let left = sampled_trim([Point2::new(1.0, 0.0), Point2::new(0.0, 0.0)]);
    let face_with_bottom = |bottom| CompressedTrimmedFace {
        boundaries: vec![vec![
            CompressedEdgeUse::from((0, true, Some(bottom))),
            CompressedEdgeUse::from((1, true, Some(right.clone()))),
            CompressedEdgeUse::from((2, true, Some(top.clone()))),
            CompressedEdgeUse::from((3, true, Some(left.clone()))),
        ]],
        orientation: true,
        surface,
    };
    let shell = CompressedTrimmedShell {
        vertices,
        edges,
        faces: vec![
            face_with_bottom(sampled_trim([
                Point2::new(0.0, 0.0),
                Point2::new(0.0, FRAC_PI_2 * 0.5),
                Point2::new(0.0, FRAC_PI_2),
            ])),
            face_with_bottom(sampled_trim([
                Point2::new(0.0, 0.0),
                Point2::new(0.0, FRAC_PI_2 * 0.25),
                Point2::new(0.0, FRAC_PI_2 * 0.5),
                Point2::new(0.0, FRAC_PI_2 * 0.75),
                Point2::new(0.0, FRAC_PI_2),
            ])),
        ],
    };

    let meshed = trimmed_cshell_tessellation(
        &shell,
        0.01,
        |_: &RevolutionSurface<Line<Point3>>, point: Point3, _| {
            Some((point.z, point.y.atan2(point.x)))
        },
        TessellationPrimitiveOptions::default(),
        None,
    );
    let shared_edge_count = meshed.edges[0].curve.len();
    let second_face = meshed.faces[1]
        .surface
        .as_ref()
        .expect("trimmed face should mesh");
    let bottom_angles = second_face
        .positions()
        .iter()
        .filter(|point| point.z.abs() <= 1.0e-8)
        .map(|point| point.y.atan2(point.x))
        .fold(Vec::<f64>::new(), |mut acc, x| {
            if !acc.iter().any(|known| (known - x).abs() <= 1.0e-8) {
                acc.push(x);
            }
            acc
        });

    assert!(shared_edge_count >= 5);
    assert_eq!(bottom_angles.len(), shared_edge_count);
}

#[test]
fn trimmed_shell_isoparams_stay_inside_trim_domain() {
    let surface = Plane::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
    );
    let mut vertices = Vec::<Point3>::new();
    let mut edges = Vec::<CompressedEdge<Line<Point3>>>::new();
    let mut edge_use = |front: Point2, back: Point2| {
        let front_point = surface.evaluate(front.x, front.y);
        let back_point = surface.evaluate(back.x, back.y);
        let vertex_index = vertices.len();
        let edge_index = edges.len();
        vertices.extend([front_point, back_point]);
        edges.push(CompressedEdge {
            vertices: (vertex_index, vertex_index + 1),
            curve: Line(front_point, back_point),
        });
        CompressedEdgeUse::from((edge_index, true, Some(sampled_trim([front, back]))))
    };
    let outer = vec![
        edge_use(Point2::new(0.0, 0.0), Point2::new(2.0, 0.0)),
        edge_use(Point2::new(2.0, 0.0), Point2::new(2.0, 2.0)),
        edge_use(Point2::new(2.0, 2.0), Point2::new(0.0, 2.0)),
        edge_use(Point2::new(0.0, 2.0), Point2::new(0.0, 0.0)),
    ];
    let hole = vec![
        edge_use(Point2::new(0.75, 0.75), Point2::new(0.75, 1.25)),
        edge_use(Point2::new(0.75, 1.25), Point2::new(1.25, 1.25)),
        edge_use(Point2::new(1.25, 1.25), Point2::new(1.25, 0.75)),
        edge_use(Point2::new(1.25, 0.75), Point2::new(0.75, 0.75)),
    ];
    let shell = CompressedTrimmedShell {
        vertices,
        edges,
        faces: vec![CompressedTrimmedFace {
            boundaries: vec![outer, hole],
            orientation: true,
            surface,
        }],
    };

    let output = compressed_trimmed_shell_tessellation_with_isoparams(
        &shell,
        0.01,
        |surface: &Plane, point: Point3, _| surface.search_parameter(point, None, 100),
        TessellationPrimitiveOptions::default(),
        Some(IsoparametricCurveOptions {
            samples_per_direction: 3,
            segments_per_curve: 32,
        }),
        None,
    );
    let curves = &output.face_isoparams[0];

    assert!(!curves.is_empty());
    assert!(output.shell.faces[0].surface.is_some());
    assert!(curves.iter().flatten().all(|point| {
        let inside_outer = point.x >= -1.0e-6
            && point.x <= 2.0 + 1.0e-6
            && point.y >= -1.0e-6
            && point.y <= 2.0 + 1.0e-6;
        let inside_hole = point.x > 0.75 + 1.0e-5
            && point.x < 1.25 - 1.0e-5
            && point.y > 0.75 + 1.0e-5
            && point.y < 1.25 - 1.0e-5;
        inside_outer && !inside_hole
    }));
}

#[test]
fn tiny_planar_trim_keeps_outer_region_and_hole_distinct_in_cdt_space() {
    let surface = Plane::xy();
    let scale = 1.0e-9;
    let piece = |ring: &[(f64, f64)]| {
        PolyBoundaryPiece(
            ring.iter()
                .map(|&(x, y)| {
                    let uv = Point2::new(x * scale, y * scale);
                    SurfacePoint::from((uv, surface.evaluate(uv.x, uv.y)))
                })
                .collect(),
        )
    };
    let boundary = PolyBoundary::new(
        vec![
            piece(&[(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0), (0.0, 0.0)]),
            piece(&[
                (0.75, 0.75),
                (0.75, 1.25),
                (1.25, 1.25),
                (1.25, 0.75),
                (0.75, 0.75),
            ]),
        ],
        &surface,
        0.1,
    );

    assert!(boundary.is_valid());
    assert!(boundary.include(Point2::new(0.25 * scale, 0.25 * scale)));
    assert!(!boundary.include(Point2::new(1.0 * scale, 1.0 * scale)));

    let mesh = trimming_tessellation(
        &surface,
        &boundary,
        0.1,
        TessellationPrimitiveOptions::default(),
        usize::MAX,
    );
    let positions = mesh.positions();
    assert!(!mesh.faces().is_empty());
    assert!(mesh.faces().face_iter().all(|face| {
        let center = face.iter().fold(Point3::origin(), |sum, vertex| {
            sum + positions[vertex.pos].to_vec()
        }) / face.len() as f64;
        !(center.x > 0.75 * scale
            && center.x < 1.25 * scale
            && center.y > 0.75 * scale
            && center.y < 1.25 * scale)
    }));
}

#[test]
fn trimmed_cylinder_patch_inserts_surface_rows_for_ruled_axis() {
    let profile = Line(Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 0.0, 10.0));
    let surface = RevolutionSurface::by_revolution(profile, Point3::origin(), Vector3::unit_z());
    let boundary = PolyBoundary::new(vec![cylinder_patch_piece(&surface)], &surface, 0.01);

    let mesh = trimming_tessellation(
        &surface,
        &boundary,
        0.01,
        TessellationPrimitiveOptions::default(),
        usize::MAX,
    );

    assert!(
        mesh.uv_coords()
            .iter()
            .any(|uv| uv.x > 0.01 && uv.x < 0.99 && uv.y > 0.01 && uv.y < TAU - 0.01),
        "trimmed cylinder mesh should include interior surface vertices",
    );
}

#[test]
fn periodic_full_rings_pair_into_cylinder_strip() {
    let profile = Line(Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 0.0, 10.0));
    let surface = RevolutionSurface::by_revolution(profile, Point3::origin(), Vector3::unit_z());
    let boundary = PolyBoundary::new(
        vec![
            cylinder_ring_piece(&surface, 0.0, false),
            cylinder_ring_piece(&surface, 1.0, true),
        ],
        &surface,
        0.01,
    );

    assert_eq!(boundary.loops.len(), 1);
    assert!(boundary.include(Point2::new(0.5, TAU * 0.5)));

    let mesh = trimming_tessellation(
        &surface,
        &boundary,
        0.01,
        TessellationPrimitiveOptions::default(),
        usize::MAX,
    );

    assert!(!mesh.tri_faces().is_empty());
}

#[test]
fn periodic_ring_closed_by_wrap_completes_to_full_period() {
    let profile = Line(Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 0.0, 10.0));
    let surface = RevolutionSurface::by_revolution(profile, Point3::origin(), Vector3::unit_z());
    let boundary = PolyBoundary::new(
        vec![
            shortened_cylinder_ring_piece(&surface, 0.0, false),
            shortened_cylinder_ring_piece(&surface, 1.0, true),
        ],
        &surface,
        0.01,
    );

    assert!(
        boundary.uv_max.y - boundary.uv_min.y >= TAU - 0.01,
        "wrapped full rings should open to one complete period",
    );
    assert!(
        boundary.uv_max.y - boundary.uv_min.y <= TAU + 0.01,
        "paired wrapped rings should not span multiple periods",
    );
    let middle_v = (boundary.uv_min.y + boundary.uv_max.y) * 0.5;
    assert!(boundary.include(Point2::new(0.5, middle_v)));

    let mesh = trimming_tessellation(
        &surface,
        &boundary,
        0.01,
        TessellationPrimitiveOptions::default(),
        usize::MAX,
    );

    assert!(mesh.tri_faces().len() > 16);
}

#[test]
fn step_cylinder_ring_pair_compacts_to_one_period() {
    let surface = step_cylinder_surface();
    let boundary = PolyBoundary::new(
        vec![
            shortened_step_cylinder_ring_piece(&surface, 0.0, false),
            shortened_step_cylinder_ring_piece(&surface, 1.0, true),
        ],
        &surface,
        0.01,
    );

    assert!(
        boundary.uv_max.x - boundary.uv_min.x <= TAU + 0.01,
        "STEP-style paired rings should not span multiple angular periods",
    );
    let middle_u = (boundary.uv_min.x + boundary.uv_max.x) * 0.5;
    assert!(boundary.include(Point2::new(middle_u, 0.5)));

    let mesh = trimming_tessellation(
        &surface,
        &boundary,
        0.01,
        TessellationPrimitiveOptions::default(),
        usize::MAX,
    );

    assert!(mesh.tri_faces().len() > 16);
}

#[test]
fn shifted_step_cylinder_ring_pair_compacts_to_one_period() {
    let surface = step_cylinder_surface();
    let boundary = PolyBoundary::new(
        vec![
            shifted_step_cylinder_ring_piece(&surface, 0.0, 0.0, false),
            shifted_step_cylinder_ring_piece(&surface, 1.0, TAU, true),
        ],
        &surface,
        0.01,
    );

    assert!(
        boundary.uv_max.x - boundary.uv_min.x <= TAU + 0.01,
        "shifted STEP-style paired rings should not span multiple angular periods",
    );
    let middle_u = (boundary.uv_min.x + boundary.uv_max.x) * 0.5;
    assert!(boundary.include(Point2::new(middle_u, 0.5)));

    let mesh = trimming_tessellation(
        &surface,
        &boundary,
        0.01,
        TessellationPrimitiveOptions::default(),
        usize::MAX,
    );

    assert!(mesh.tri_faces().len() > 16);
}

#[test]
fn seam_crossing_step_cylinder_ring_pair_compacts_to_one_period() {
    let surface = step_cylinder_surface();
    let boundary = PolyBoundary::new(
        vec![
            seam_crossing_step_cylinder_ring_piece(&surface, 0.0, false),
            seam_crossing_step_cylinder_ring_piece(&surface, 1.0, true),
        ],
        &surface,
        0.01,
    );

    assert!(
        boundary.uv_max.x - boundary.uv_min.x <= TAU + 0.01,
        "seam-crossing STEP-style paired rings should not span multiple angular periods",
    );
    let middle_u = (boundary.uv_min.x + boundary.uv_max.x) * 0.5;
    assert!(boundary.include(Point2::new(middle_u, 0.5)));

    let mesh = trimming_tessellation(
        &surface,
        &boundary,
        0.01,
        TessellationPrimitiveOptions::default(),
        usize::MAX,
    );

    assert!(mesh.tri_faces().len() > 16);
}

#[test]
fn mismatched_seam_step_cylinder_ring_pair_compacts_to_one_period() {
    let surface = step_cylinder_surface();
    let boundary = PolyBoundary::new(
        vec![
            seam_crossing_step_cylinder_ring_piece_from(&surface, 0.0, FRAC_PI_2 * 3.0, false),
            seam_crossing_step_cylinder_ring_piece_from(&surface, 1.0, 0.0, true),
        ],
        &surface,
        0.01,
    );

    assert!(
        boundary.uv_max.x - boundary.uv_min.x <= TAU + 0.01,
        "paired rings with mismatched seam starts should not span multiple angular periods",
    );
    let middle_u = (boundary.uv_min.x + boundary.uv_max.x) * 0.5;
    assert!(boundary.include(Point2::new(middle_u, 0.5)));

    let mesh = trimming_tessellation(
        &surface,
        &boundary,
        0.01,
        TessellationPrimitiveOptions::default(),
        usize::MAX,
    );

    assert!(mesh.tri_faces().len() > 16);
}

#[test]
fn partial_periodic_loop_crossing_seam_compacts_to_short_arc() {
    let surface = step_cylinder_surface();
    let boundary = PolyBoundary::new(
        vec![seam_crossing_step_cylinder_patch_piece(&surface)],
        &surface,
        0.01,
    );

    assert!(
        boundary.uv_max.x - boundary.uv_min.x <= FRAC_PI_2 + 0.01,
        "partial periodic loops crossing the seam should keep the short angular arc",
    );
    let middle_u = (boundary.uv_min.x + boundary.uv_max.x) * 0.5;
    assert!(boundary.include(Point2::new(middle_u, 0.5)));

    let mesh = trimming_tessellation(
        &surface,
        &boundary,
        0.01,
        TessellationPrimitiveOptions::default(),
        usize::MAX,
    );

    assert!(mesh.tri_faces().len() > 16);
}

#[test]
fn periodic_cylinder_loop_spanning_two_laps_collapses_to_one_lap() {
    let surface = step_cylinder_surface();
    let boundary = PolyBoundary::new(
        vec![double_lap_step_cylinder_piece(&surface)],
        &surface,
        0.01,
    );

    assert!(
        boundary.uv_max.x - boundary.uv_min.x <= TAU + 0.01,
        "periodic cylinder trim should not span two angular laps",
    );
    assert!(boundary.include(Point2::new(0.5 * TAU, 0.5)));

    let mesh = trimming_tessellation(
        &surface,
        &boundary,
        0.01,
        TessellationPrimitiveOptions::default(),
        usize::MAX,
    );

    assert!(!mesh.tri_faces().is_empty());
}

#[test]
fn trimming_tessellation_is_invariant_to_annulus_loop_winding() {
    let surface = Plane::xy();
    let canonical = PolyBoundary::new(
        vec![
            square_piece(&surface, 0.0, 1.0, true),
            square_piece(&surface, 0.25, 0.75, false),
        ],
        &surface,
        0.01,
    );
    let reversed = PolyBoundary::new(
        vec![
            square_piece(&surface, 0.0, 1.0, false),
            square_piece(&surface, 0.25, 0.75, true),
        ],
        &surface,
        0.01,
    );

    assert!(canonical.include(Point2::new(0.1, 0.1)));
    assert!(!canonical.include(Point2::new(0.5, 0.5)));
    assert!(reversed.include(Point2::new(0.1, 0.1)));
    assert!(!reversed.include(Point2::new(0.5, 0.5)));

    let canonical_mesh = trimming_tessellation(
        &surface,
        &canonical,
        0.01,
        TessellationPrimitiveOptions::default(),
        usize::MAX,
    );
    let reversed_mesh = trimming_tessellation(
        &surface,
        &reversed,
        0.01,
        TessellationPrimitiveOptions::default(),
        usize::MAX,
    );

    assert_eq!(
        canonical_mesh.tri_faces().len(),
        reversed_mesh.tri_faces().len(),
    );
    assert!(!canonical_mesh.tri_faces().is_empty());
}

#[test]
fn trimming_tessellation_is_invariant_to_single_loop_winding() {
    let surface = Plane::xy();
    let forward = PolyBoundary::new(vec![square_piece(&surface, 0.0, 1.0, true)], &surface, 0.01);
    let reversed = PolyBoundary::new(
        vec![square_piece(&surface, 0.0, 1.0, false)],
        &surface,
        0.01,
    );

    assert!(forward.include(Point2::new(0.5, 0.5)));
    assert!(reversed.include(Point2::new(0.5, 0.5)));

    let forward_mesh = trimming_tessellation(
        &surface,
        &forward,
        0.01,
        TessellationPrimitiveOptions::default(),
        usize::MAX,
    );
    let reversed_mesh = trimming_tessellation(
        &surface,
        &reversed,
        0.01,
        TessellationPrimitiveOptions::default(),
        usize::MAX,
    );

    assert_eq!(
        forward_mesh.tri_faces().len(),
        reversed_mesh.tri_faces().len()
    );
    assert!(!forward_mesh.tri_faces().is_empty());
}

// --- spec 007 D1: loud tessellation face-drop diagnostic ---------------

#[test]
fn classify_face_drop_maps_each_silent_drop() {
    use super::diagnostics::classify_face_drop;

    // `None` mesh on an untrimmed face -> the surface had no bounded domain.
    assert_eq!(
        classify_face_drop(None, true),
        Some(FaceDropReason::UnboundedDomain),
    );
    // `None` mesh on a trimmed face -> a boundary loop would not project
    // (the revolve-pole / periodic-seam / degenerate-trim family).
    assert_eq!(
        classify_face_drop(None, false),
        Some(FaceDropReason::BoundaryProjectionFailed),
    );
    // `Some` but empty -> a silently empty tessellation, on either path.
    let empty = PolygonMesh::default();
    assert_eq!(
        classify_face_drop(Some(&empty), false),
        Some(FaceDropReason::EmptyTessellation),
    );
    assert_eq!(
        classify_face_drop(Some(&empty), true),
        Some(FaceDropReason::EmptyTessellation),
    );

    // A real, non-empty tessellation is not a drop.
    let profile = Line(Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 0.0, 10.0));
    let surface = RevolutionSurface::by_revolution(profile, Point3::origin(), Vector3::unit_z());
    let boundary = PolyBoundary::new(vec![cylinder_patch_piece(&surface)], &surface, 0.01);
    let mesh = trimming_tessellation(
        &surface,
        &boundary,
        0.01,
        TessellationPrimitiveOptions::default(),
        usize::MAX,
    );
    assert!(!mesh.faces().is_empty());
    assert_eq!(classify_face_drop(Some(&mesh), false), None);
}

#[test]
fn metric_refusals_keep_typed_reason_in_strict_and_lenient_paths() {
    for (failure, expected) in [
        (
            MetricRefinementError::InvalidMetric,
            FaceDropReason::InvalidMetric,
        ),
        (
            MetricRefinementError::InvalidTopology,
            FaceDropReason::InvalidTopology,
        ),
        (
            MetricRefinementError::NonConvergent,
            FaceDropReason::NonConvergent,
        ),
    ] {
        let (mesh, reason) = lenient_metric_result(Err(failure));
        assert!(mesh.is_none(), "lenient path must keep its Option contract");
        assert_eq!(reason, Some(failure));
        assert_eq!(failure.face_drop_reason(), expected);
        assert_eq!(expected.tag(), failure.tag());
    }
}

#[test]
fn lenient_metric_success_preserves_the_mesh() {
    let mesh = PolygonMesh::default();
    let (result, failure) = lenient_metric_result(Ok(mesh.clone()));
    assert_eq!(result, Some(mesh));
    assert_eq!(failure, None);
}

#[test]
fn shell_metric_refusal_is_none_lenient_and_typed_strict() {
    let shell = singular_metric_shell();
    let sp = |surface: &SingularMetricPlane, point: Point3, hint| {
        surface.search_parameter(point, hint, 100)
    };
    let options = TessellationPrimitiveOptions::default();

    let parallel = shell_tessellation_with_failures(&shell, 0.1, sp, options, None);
    assert_eq!(
        parallel.metric_failures,
        vec![(0, MetricRefinementError::InvalidMetric)]
    );
    assert!(parallel.shell[0].surface().is_none());

    let serial = shell_tessellation_single_thread_with_failures(&shell, 0.1, sp, options, None);
    assert_eq!(serial.metric_failures, parallel.metric_failures);
    assert!(serial.shell[0].surface().is_none());

    let lenient = shell.triangulation(0.1);
    assert!(lenient[0].surface().is_none());
    assert!(matches!(
        shell_to_polygon_strict(&shell, 0.1),
        Err(TessellationError::FaceDropped {
            face: 0,
            reason: FaceDropReason::InvalidMetric,
            ..
        })
    ));
}

#[test]
fn observe_face_drop_advances_the_loud_metric() {
    use super::diagnostics::observe_face_drop;

    let profile = Line(Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 0.0, 10.0));
    let surface = RevolutionSurface::by_revolution(profile, Point3::origin(), Vector3::unit_z());

    // The metric is process-global and monotonic within a run, so a strict
    // increase across our own dropping call proves the signal fired even if
    // other parallel tests advance it too.
    let before = face_drop_count();
    observe_face_drop(&surface, Some(7), None, false, 1, 4);
    assert!(
        face_drop_count() > before,
        "a boundary-projection drop must advance the face-drop metric",
    );
}

#[test]
fn observe_metric_failure_advances_the_loud_metric() {
    use super::diagnostics::observe_metric_failure;

    let surface = Plane::xy();
    let before = face_drop_count();
    observe_metric_failure(
        &surface,
        Some(8),
        MetricRefinementError::InvalidMetric,
        1,
        4,
    );
    assert!(face_drop_count() > before);
}

#[derive(Clone)]
struct CountedLine {
    line: Line<Point3>,
    interior_seed: Option<f64>,
    singular: bool,
    calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}
impl ParametricCurve for CountedLine {
    type Point = Point3;
    type Vector = Vector3;
    fn evaluate(&self, t: f64) -> Point3 {
        self.line.evaluate(t)
    }
    fn derivative(&self, t: f64) -> Vector3 {
        if self.singular && t == 0.5 {
            Vector3::zero()
        } else {
            self.line.derivative(t)
        }
    }
    fn derivative_2(&self, t: f64) -> Vector3 {
        self.line.derivative_2(t)
    }
    fn derivative_n(&self, n: usize, t: f64) -> Vector3 {
        self.line.derivative_n(n, t)
    }
    fn parameter_range(&self) -> ParameterRange {
        self.line.parameter_range()
    }
}
impl BoundedCurve for CountedLine {}
impl ParameterDivision1D for CountedLine {
    type Point = Point3;
    fn parameter_division(&self, range: (f64, f64), tolerance: f64) -> (Vec<f64>, Vec<Point3>) {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if let Some(fraction) = self.interior_seed {
            let parameters = vec![range.0, range.0 + (range.1 - range.0) * fraction, range.1];
            let points = parameters.iter().map(|t| self.line.evaluate(*t)).collect();
            (parameters, points)
        } else {
            self.line.parameter_division(range, tolerance)
        }
    }
}
impl ParameterBoundary2D<Plane> for CountedLine {}

#[test]
fn angular_shared_edges_sample_once_and_refuse_every_incident_face() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let points = [
        Point3::new(0., 0., 0.),
        Point3::new(1., 0., 0.),
        Point3::new(1., 1., 0.),
        Point3::new(0., 1., 0.),
    ];
    let vertices = Vertex::from_points(points);
    for singular in [false, true] {
        let wire: Wire<_, _> = (0..4)
            .map(|index| {
                let next = (index + 1) % 4;
                Edge::new_unchecked(
                    &vertices[index],
                    &vertices[next],
                    CountedLine {
                        line: Line(points[index], points[next]),
                        interior_seed: None,
                        singular: singular && index == 0,
                        calls: calls.clone(),
                    },
                )
            })
            .collect();
        let face = Face::new_unchecked(vec![wire], Plane::xy());
        let shell: Shell<_, _, _> = vec![face.clone(), face.inverse()].into();
        calls.store(0, Ordering::SeqCst);
        let parallel = shell_tessellation_with_failures(
            &shell,
            0.1,
            by_search_parameter,
            TessellationPrimitiveOptions::default(),
            Some(0.5),
        );
        assert_eq!(
            calls.load(Ordering::SeqCst),
            4,
            "shared edges must be sampled once"
        );
        assert!(
            parallel
                .shell
                .face_iter()
                .all(|f| f.surface().is_none() == singular)
        );
        assert_eq!(
            parallel.shell[0].absolute_boundaries()[0][0].id(),
            parallel.shell[1].absolute_boundaries()[0][0].id()
        );
        let expected = if singular {
            vec![
                (0, FaceDropReason::SingularAngularTangent),
                (1, FaceDropReason::SingularAngularTangent),
            ]
        } else {
            Vec::new()
        };
        assert_eq!(parallel.edge_failures, expected);
        calls.store(0, Ordering::SeqCst);
        let serial = shell_tessellation_single_thread(
            &shell,
            0.1,
            by_search_parameter,
            TessellationPrimitiveOptions::default(),
            Some(0.5),
        );
        assert_eq!(calls.load(Ordering::SeqCst), 4);
        assert!(
            serial
                .face_iter()
                .all(|f| f.surface().is_none() == singular)
        );
        let options = TessellationOptions {
            tolerance: 0.1,
            edge_angular_tolerance: Some(0.5),
            ..Default::default()
        };
        let strict = shell_to_polygon_strict_with(&shell, options);
        if singular {
            assert!(matches!(
                strict,
                Err(TessellationError::FaceDropped {
                    face: 0,
                    reason: FaceDropReason::SingularAngularTangent,
                    ..
                })
            ));
        } else {
            assert!(!strict.unwrap().faces().is_empty());
        }
        calls.store(0, Ordering::SeqCst);
        let compressed = cshell_triangulation_with(&shell.compress(), options);
        assert_eq!(calls.load(Ordering::SeqCst), 4);
        assert!(
            compressed
                .faces
                .iter()
                .all(|f| f.surface.is_none() == singular)
        );
        let invalid = shell_to_polygon_strict_with(
            &shell,
            TessellationOptions {
                edge_angular_tolerance: Some(f64::NAN),
                ..options
            },
        );
        assert!(matches!(
            invalid,
            Err(TessellationError::FaceDropped {
                face: -1,
                reason: FaceDropReason::InvalidAngularOptions,
                ..
            })
        ));
    }
}

#[test]
fn angular_trimmed_routes_refuse_failed_shared_edge_without_coarse_fallback() {
    let mut shell = trimmed_plane_shell(vec![vec![
        Point2::new(0., 0.),
        Point2::new(1., 0.),
        Point2::new(1., 1.),
        Point2::new(0., 1.),
    ]]);
    shell.faces.push(shell.faces[0].clone());
    let options = TessellationOptions {
        tolerance: 0.1,
        edge_angular_tolerance: Some(0.5),
        ..Default::default()
    };
    let valid = trimmed_cshell_triangulation_with(&shell, options);
    assert!(valid.faces.iter().all(|face| face.surface.is_some()));
    shell.edges[0].curve.1 = shell.edges[0].curve.0;
    let refused = trimmed_cshell_triangulation_with(&shell, options);
    assert!(refused.faces.iter().all(|face| face.surface.is_none()));
    let iso = compressed_trimmed_shell_tessellation_with_isoparams(
        &shell,
        0.1,
        by_search_parameter,
        TessellationPrimitiveOptions::default(),
        Some(IsoparametricCurveOptions::default()),
        Some(0.5),
    );
    assert!(iso.shell.faces.iter().all(|face| face.surface.is_none()));
}

#[test]
fn angular_strict_refuses_invalid_options_even_without_edges() {
    let shell = Shell::<Point3, Line<Point3>, Plane>::default();
    for angle in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
        assert!(matches!(
            shell_to_polygon_strict_with(
                &shell,
                TessellationOptions {
                    edge_angular_tolerance: Some(angle),
                    ..Default::default()
                }
            ),
            Err(TessellationError::FaceDropped {
                face: -1,
                reason: FaceDropReason::InvalidAngularOptions,
                ..
            })
        ));
    }
    assert!(shell_to_polygon_strict_with(&shell, TessellationOptions::default()).is_ok());
}

#[test]
fn angular_aligned_trim_cannot_fall_back_to_independent_points() {
    let surface = Plane::xy();
    let curve = ParameterCurve::new(Line(Point2::new(0., 0.), Point2::new(1., 0.)), surface);
    let polyline = PolylineCurve::from(vec![
        Point3::new(0., 0., 0.),
        Point3::new(0.3, 0., 0.),
        Point3::new(1., 0., 0.),
    ]);
    let sp = |_: &Plane, _: Point3, _| None;
    let legacy = PolyBoundaryPiece::try_new_from_aligned_trimmed(
        &surface,
        [(None::<&SampledTrim>, &curve, polyline.clone())].into_iter(),
        &sp,
        0.1,
        false,
    );
    assert!(
        legacy.is_some(),
        "control must exercise the independent-source fallback"
    );
    let strict = PolyBoundaryPiece::try_new_from_aligned_trimmed(
        &surface,
        [(None::<&SampledTrim>, &curve, polyline)].into_iter(),
        &sp,
        0.1,
        true,
    );
    assert!(
        strict.is_none(),
        "angular sampling cannot accept independently sampled coarse geometry"
    );
}

#[test]
fn angular_aligned_exact_does_not_pair_nonuniform_points_with_uniform_uvs() {
    let surface = Plane::xy();
    let polyline = PolylineCurve::from(vec![
        Point3::new(0., 0., 0.),
        Point3::new(0.1, 0., 0.),
        Point3::new(0.9, 0., 0.),
        Point3::new(1., 0., 0.),
    ]);
    let piece = PolyBoundaryPiece::try_new_from_aligned_exact(
        &surface,
        [(
            Some(vec![Point2::new(0., 0.), Point2::new(1., 0.)]),
            polyline,
        )]
        .into_iter(),
        &|_: &Plane, p: Point3, _| Some((p.x, p.y)),
        0.01,
        true,
    )
    .unwrap();
    assert!(
        piece
            .0
            .iter()
            .all(|p| (surface.evaluate(p.x, p.y) - p.point).magnitude() <= 0.01)
    );
}

#[test]
fn angular_invalid_trim_edge_index_refuses_without_panicking() {
    let mut shell = trimmed_plane_shell(vec![vec![
        Point2::new(0., 0.),
        Point2::new(1., 0.),
        Point2::new(1., 1.),
        Point2::new(0., 1.),
    ]]);
    shell.faces[0].boundaries[0][0].index = usize::MAX;
    let output = trimmed_cshell_triangulation_with(
        &shell,
        TessellationOptions {
            edge_angular_tolerance: Some(0.5),
            ..Default::default()
        },
    );
    assert!(output.faces[0].surface.is_none());
}

#[test]
fn angular_invalid_options_refuse_edge_free_faces_in_every_lenient_route() {
    let surface = Plane::xy();
    let shell: Shell<Point3, Line<Point3>, Plane> =
        vec![Face::new_unchecked(Vec::new(), surface)].into();
    let compressed = shell.compress();
    let trimmed: CompressedTrimmedShell<Point3, Line<Point3>, Plane, SampledTrim> =
        CompressedTrimmedShell {
            vertices: Vec::new(),
            edges: Vec::new(),
            faces: vec![CompressedTrimmedFace {
                boundaries: Vec::new(),
                orientation: true,
                surface,
            }],
        };
    let defaults = TessellationPrimitiveOptions::default();
    let valid = shell_tessellation_with_failures(&shell, 0.1, by_search_parameter, defaults, None);
    assert!(
        valid.shell[0].surface().is_some(),
        "control must produce the bounded untrimmed face"
    );
    for angle in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
        let parallel = shell_tessellation_with_failures(
            &shell,
            0.1,
            by_search_parameter,
            defaults,
            Some(angle),
        );
        assert!(
            parallel.shell[0].surface().is_none(),
            "invalid option must not emit an edge-free face"
        );
        assert_eq!(
            parallel.edge_failures,
            vec![(0, FaceDropReason::InvalidAngularOptions)]
        );
        let serial = shell_tessellation_single_thread_with_failures(
            &shell,
            0.1,
            by_search_parameter,
            defaults,
            Some(angle),
        );
        assert!(serial.shell[0].surface().is_none());
        assert_eq!(serial.edge_failures, parallel.edge_failures);
        let options = TessellationOptions {
            tolerance: 0.1,
            edge_angular_tolerance: Some(angle),
            ..Default::default()
        };
        assert!(
            cshell_triangulation_with(&compressed, options).faces[0]
                .surface
                .is_none()
        );
        assert!(
            trimmed_cshell_triangulation_with(&trimmed, options).faces[0]
                .surface
                .is_none()
        );
        let iso = compressed_trimmed_shell_tessellation_with_isoparams(
            &trimmed,
            0.1,
            by_search_parameter,
            defaults,
            Some(IsoparametricCurveOptions::default()),
            Some(angle),
        );
        assert!(iso.shell.faces[0].surface.is_none());
    }
}

#[test]
fn angular_preserve_close_uv_shared_sample() {
    let surface = Plane::new(
        Point3::origin(),
        Point3::new(1.0e6, 0., 0.),
        Point3::new(0., 1.0e6, 0.),
    );
    let uvs = vec![
        Point2::new(0., 0.),
        Point2::new(5.0e-7, 0.),
        Point2::new(1., 0.),
        Point2::new(1., 1.),
        Point2::new(0., 1.),
        Point2::new(0., 0.),
    ];
    let points: Vec<_> = uvs.iter().map(|p| surface.evaluate(p.x, p.y)).collect();
    let original = points[1];
    let polyline = PolylineCurve::from(points);
    let sp = |_: &Plane, p: Point3, _| Some((p.x / 1.0e6, p.y / 1.0e6));
    let curve = ParameterCurve::new(Line(Point2::new(0., 0.), Point2::new(1., 0.)), surface);
    let exact = PolyBoundaryPiece::try_new_from_aligned_exact(
        &surface,
        [(Some(uvs), polyline.clone())].into_iter(),
        &sp,
        0.01,
        true,
    );
    let trimmed = PolyBoundaryPiece::try_new_from_aligned_trimmed(
        &surface,
        [(None::<&SampledTrim>, &curve, polyline)].into_iter(),
        &sp,
        0.01,
        true,
    );
    for piece in [exact, trimmed] {
        let piece = piece.expect("aligned chart must emit");
        assert!(
            piece.0.iter().any(|p| p.point == original),
            "distinct shared source point (0.5,0,0) lost in accepted angular chart: {:?}",
            piece.0
        );
    }
}

#[test]
fn angular_public_compressed_preserve_shared_sample() {
    for scale in [1.0, 1.0e6] {
        let surface = Plane::new(
            Point3::origin(),
            Point3::new(scale, 0., 0.),
            Point3::new(0., scale, 0.),
        );
        let points = [
            Point3::origin(),
            Point3::new(scale, 0., 0.),
            Point3::new(scale, scale, 0.),
            Point3::new(0., scale, 0.),
        ];
        let vertices = Vertex::from_points(points);
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let wire: Wire<_, _> = (0..4)
            .map(|i| {
                let j = (i + 1) % 4;
                Edge::new_unchecked(
                    &vertices[i],
                    &vertices[j],
                    CountedLine {
                        line: Line(points[i], points[j]),
                        interior_seed: Some(5.0e-7),
                        singular: false,
                        calls: calls.clone(),
                    },
                )
            })
            .collect();
        let face = Face::new_unchecked(vec![wire], surface);
        let shell: Shell<_, _, _> = vec![face.clone(), face.inverse()].into();
        let result = cshell_triangulation_with(
            &shell.compress(),
            TessellationOptions {
                tolerance: 0.01,
                edge_angular_tolerance: Some(0.5),
                ..Default::default()
            },
        );
        let original = Point3::new(scale * 5.0e-7, 0., 0.);
        assert!(
            result
                .edges
                .iter()
                .any(|edge| edge.curve.iter().any(|p| *p == original)),
            "source edge seed exists"
        );
        assert!(
            result.faces.iter().all(|face| face.surface.is_some()),
            "faces emit"
        );
        assert!(
            result.faces.iter().all(|face| {
                let mesh = face.surface.as_ref().unwrap();
                mesh.faces()
                    .face_iter()
                    .flatten()
                    .any(|vertex| mesh.positions()[vertex.pos] == original)
            }),
            "emitted incident face lost shared seed {original:?} at scale {scale}"
        );
    }
}

#[test]
fn proven_closed_boundaries_do_not_turn_an_interior_bridge_into_a_seam() {
    use monstertruck_geometry::prelude::{BsplineSurface, KnotVector};
    let row = |x, z| vec![Point3::new(x, 0.0, z), Point3::new(x, 1.0, z)];
    let surface = BsplineSurface::new(
        (
            KnotVector::from(vec![0., 0., 0., 0., 0.875, 0.875, 0.875, 1., 1., 1., 1.]),
            KnotVector::bezier_knot(1),
        ),
        vec![
            row(0., 0.),
            row(0.2916666666666667, 0.),
            row(0.5833333333333334, 0.),
            row(0.875, 0.),
            row(0.5833333333333334, -128.),
            row(0.2916666666666667, 128.),
            row(0., 0.),
        ],
    );
    assert_eq!(surface.finite_boundary_equal_axes(), Some([true, false]));
    // Global boundary equality still does not authorize a local endpoint flip:
    // the correct source segment is affine, while the alias hides a 36mm bow.
    assert_eq!(surface.evaluate(0., 0.), surface.evaluate(1., 0.));
    assert!(
        surface
            .evaluate(0.9375, 0.)
            .distance(Point3::new(0.4375, 0., 0.))
            < 1e-12
    );
    assert!(
        (surface
            .evaluate(0.96875, 0.)
            .distance(Point3::new(0.21875, 0., 0.))
            - 36.)
            .abs()
            < 1e-9
    );
    let uv = [(0., 0.), (0.875, 0.), (0.875, 1.), (0., 1.), (0.25, 0.5)];
    let vertices = uv.map(|(u, v)| surface.evaluate(u, v)).to_vec();
    let pairs = [(0, 1), (1, 2), (2, 3), (3, 0), (0, 4)];
    let edges = pairs
        .map(|(a, b)| CompressedEdge {
            vertices: (a, b),
            curve: Line(vertices[a], vertices[b]),
        })
        .to_vec();
    let shell = CompressedShell {
        vertices,
        edges,
        faces: vec![CompressedFace {
            boundaries: vec![
                [
                    (0, true),
                    (1, true),
                    (2, true),
                    (3, true),
                    (4, true),
                    (4, false),
                ]
                .map(|(index, orientation)| CompressedEdgeIndex { index, orientation })
                .to_vec(),
            ],
            orientation: true,
            surface,
        }],
        vertex_stable_ids: None,
        edge_stable_ids: None,
        face_stable_ids: None,
    };
    let mesh = cshell_tessellation(
        &shell,
        0.0005,
        |_surface: &BsplineSurface<Point3>, point: Point3, _| Some((point.x, point.y)),
        TessellationPrimitiveOptions::default(),
        None,
    );
    let face = mesh.faces[0]
        .surface
        .as_ref()
        .expect("a repeated source bridge is not a finite-boundary seam");
    let area = face
        .faces()
        .triangle_iter()
        .map(|triangle| {
            let p = triangle.map(|vertex| face.positions()[vertex.pos]);
            assert!(p.iter().all(|point| point.z.abs() < 1e-12));
            (p[1] - p[0]).cross(p[2] - p[0]).z * 0.5
        })
        .sum::<f64>();
    assert!((area - 0.875).abs() < 1e-10);
}

//! Private observation of the actual analysis emission; no source certificate.
//!
//! Coordinates are body-local, including authored elevation/source placement.
//! No physical row transform is accepted or applied here. The hook observes the
//! existing polygon fan after orientation and render rejection, without another
//! tessellation. UVs are attributes, not evidence of evaluation or trim coverage.
//! In particular the mesher's preview fallback has placeholder UVs, and its
//! periodic lifting decisions are not exposed. Never infer an unwrap or an
//! angular/height bound from the requested meshing settings.

use super::*;
use monstertruck_meshing::rexport_polymesh::{PolygonMesh, StandardVertex};
use monstertruck_modeling::{EuclideanSpace, Face};

mod cylinder_proof;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UvUnknown {
    Missing,
    InvalidIndex,
    NonFinite,
    DegenerateOrPlaceholder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChartObservation {
    NonperiodicPlane,
    UnknownPeriodicUnwrap,
    UnknownUnsupportedSupport,
}

fn chart_observation(surface: &Surface) -> ChartObservation {
    match surface {
        Surface::Plane(_) => ChartObservation::NonperiodicPlane,
        Surface::RevolutionSurface(_) => ChartObservation::UnknownPeriodicUnwrap,
        _ => ChartObservation::UnknownUnsupportedSupport,
    }
}

// Ok preserves raw attributes only; it proves neither surface evaluation nor coverage.
fn triangle_uv(
    poly: &PolygonMesh,
    vertices: [StandardVertex; 3],
) -> Result<[[f64; 2]; 3], UvUnknown> {
    let mut result = [[0.; 2]; 3];
    for (corner, vertex) in vertices.into_iter().enumerate() {
        let uv = poly
            .uv_coords()
            .get(vertex.uv.ok_or(UvUnknown::Missing)?)
            .ok_or(UvUnknown::InvalidIndex)?;
        if !uv.x.is_finite() || !uv.y.is_finite() {
            return Err(UvUnknown::NonFinite);
        }
        result[corner] = [uv.x, uv.y];
    }
    let [a, b, c] = result;
    let area = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]);
    if !area.is_finite() || area == 0. {
        return Err(UvUnknown::DegenerateOrPlaceholder);
    }
    Ok(result)
}

struct RetainedFace {
    source: GeneratedSourceFace,
    surface: Surface,
    boundaries: Vec<Wire>,
    mesh_orientation: bool,
    chart: ChartObservation,
    polygon: PolygonMesh,
}

struct FinalTriangle {
    occurrence_index: usize,
    vertices: [StandardVertex; 3],
    uv: Result<[[f64; 2]; 3], UvUnknown>,
    positions_mm_f64: [f64; 9],
    rendered_positions_f32: [f32; 9],
}

#[derive(Default)]
pub(super) struct AnalysisTrace {
    faces: Vec<RetainedFace>,
    triangles: Vec<FinalTriangle>,
}

impl AnalysisTrace {
    pub(super) fn record_face(
        &mut self,
        source: &GeneratedSourceFace,
        face: &Face,
        mesh_orientation: bool,
        polygon: &PolygonMesh,
    ) {
        let surface = face.surface();
        self.faces.push(RetainedFace {
            source: source.clone(),
            chart: chart_observation(&surface),
            surface,
            boundaries: face.boundaries(),
            mesh_orientation,
            polygon: polygon.clone(),
        });
    }

    pub(super) fn record_triangle(
        &mut self,
        occurrence_index: usize,
        poly: &PolygonMesh,
        vertices: [StandardVertex; 3],
        positions: &[f64],
        rendered: &[f32],
    ) {
        self.triangles.push(FinalTriangle {
            occurrence_index,
            vertices,
            uv: triangle_uv(poly, vertices),
            positions_mm_f64: positions.try_into().expect("just emitted one f64 triangle"),
            rendered_positions_f32: rendered.try_into().expect("just emitted one f32 triangle"),
        });
    }
}

fn plate() -> ExactModel {
    build_profile(ProfileInput {
        kind: ProfileKind::Plate,
        base_z: 0.0,
        thickness: 2.0,
        wall_height: 0.0,
        regions: vec![ProfileRegion {
            outer: [[0., 0.], [20., 0.], [20., 10.], [0., 10.]]
                .map(|[x, y]| Vec2 { x, y })
                .to_vec(),
            holes: vec![],
        }],
    })
    .unwrap()
}

// The oracle is the public entry point and final buffers from the SAME emission
// routine, plus independent authored bounds/census. No shadow mesher or UV search.
fn checked_trace(
    model: &ExactModel,
    target: Option<f64>,
) -> (GeneratedAnalysisMesh, AnalysisTrace) {
    let expected = mesh_profile_for_analysis(model, target).unwrap();
    let mut trace = AnalysisTrace::default();
    let mesh = mesh_profile_for_analysis_impl(model, target, Some(&mut trace)).unwrap();
    assert_eq!(mesh, expected);
    assert_eq!(
        mesh.positions_mm_f64
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>(),
        expected
            .positions_mm_f64
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        mesh.mesh
            .positions
            .iter()
            .chain(&mesh.mesh.normals)
            .map(|v| v.to_bits())
            .collect::<Vec<_>>(),
        expected
            .mesh
            .positions
            .iter()
            .chain(&expected.mesh.normals)
            .map(|v| v.to_bits())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        trace.triangles.len(),
        mesh.positions_mm_f64.len() / 9,
        "every final emitted triangle must have its actual UV/source trace"
    );
    assert_eq!(trace.faces.len(), mesh.source_faces.len());
    assert_eq!(mesh.mesh.positions.len(), mesh.positions_mm_f64.len());
    let mut seen = vec![false; trace.faces.len()];
    let mut next_triangle = vec![0; trace.faces.len()];
    for (i, tri) in trace.triangles.iter().enumerate() {
        assert_eq!(
            tri.occurrence_index,
            mesh.triangle_face_occurrence_indices[i]
        );
        let face = &trace.faces[tri.occurrence_index];
        assert_eq!(face.source, mesh.source_faces[tri.occurrence_index]);
        seen[tri.occurrence_index] = true;
        let expected_vertices = face
            .polygon
            .faces()
            .triangle_iter()
            .nth(next_triangle[tri.occurrence_index])
            .unwrap();
        next_triangle[tri.occurrence_index] += 1;
        assert_eq!(tri.vertices, expected_vertices);
        for (corner, vertex) in expected_vertices.iter().enumerate() {
            let point = face.polygon.positions()[vertex.pos];
            assert_eq!(
                tri.positions_mm_f64[corner * 3..corner * 3 + 3],
                [point.x, point.y, point.z]
            );
            if let Ok(uv) = tri.uv {
                let actual = face.polygon.uv_coords()[vertex.uv.unwrap()];
                assert_eq!(
                    uv[corner].map(f64::to_bits),
                    [actual.x.to_bits(), actual.y.to_bits()]
                );
            }
        }
        for j in 0..9 {
            assert_eq!(
                tri.positions_mm_f64[j].to_bits(),
                mesh.positions_mm_f64[i * 9 + j].to_bits()
            );
            assert_eq!(
                tri.rendered_positions_f32[j].to_bits(),
                mesh.mesh.positions[i * 9 + j].to_bits()
            );
            assert_eq!(
                (tri.positions_mm_f64[j] as f32).to_bits(),
                tri.rendered_positions_f32[j].to_bits()
            );
        }
    }
    assert!(seen.into_iter().all(|v| v));
    let mut face_index = 0;
    for (si, solid) in model.solids.iter().enumerate() {
        for (shi, shell) in solid.boundaries().iter().enumerate() {
            for (fi, native) in shell.face_iter().enumerate() {
                let face = &trace.faces[face_index];
                assert_eq!(
                    (
                        face.source.solid_index,
                        face.source.shell_index,
                        face.source.face_index
                    ),
                    (si, shi, fi)
                );
                assert_eq!(
                    serde_json::to_value(&face.surface).unwrap(),
                    serde_json::to_value(native.surface()).unwrap()
                );
                assert_eq!(face.boundaries, native.boundaries());
                assert_eq!(face.mesh_orientation, native.orientation());
                assert_eq!(face.chart, chart_observation(&native.surface()));
                assert_eq!(
                    next_triangle[face_index],
                    face.polygon.faces().triangle_iter().count()
                );
                face_index += 1;
            }
        }
    }
    assert_eq!(face_index, trace.faces.len());
    assert_eq!(mesh.analysis_error_bound_mm, None);
    (mesh, trace)
}

#[test]
fn final_trace_covers_actual_plate_emission() {
    let (mesh, trace) = checked_trace(&plate(), None);
    assert_eq!(mesh.bounds, [[0., 0., 0.], [20., 10., 2.]]);
    assert_eq!(trace.faces.len(), 6);
    assert_eq!(trace.triangles.len(), 12);
    assert!(trace.triangles.iter().all(|t| t.uv.is_ok()));
    assert!(
        trace
            .faces
            .iter()
            .all(|f| f.chart == ChartObservation::NonperiodicPlane)
    );
}

#[test]
fn final_trace_retains_authored_pcb_circle_and_drill_trims() {
    use serde_json::json;
    let corners = [[0., 0.], [20., 0.], [20., 10.], [0., 10.]];
    let points = corners.map(|[x, y]| Vec2 { x, y }).to_vec();
    let lines = (0..4).map(|i| json!({
        "sourceId": format!("edge-{i}"), "segmentIndex": 0,
        "geometry": {"kind":"line", "start": points[i], "end": points[(i+1)%4], "width":null}
    })).collect::<Vec<_>>();
    let input: boardstudio_contracts::PreparedCaseIR = serde_json::from_value(json!({
        "revision":1,
        "body": {"id":"trace-pcb", "name":"trace-pcb", "boardId":"board", "kind":"plate",
            "thickness":2., "clearance":0.,
            "mounts":[{"id":"drill", "kind":"hole", "at":{"x":4.,"y":5.}, "holeDiameter":1.}],
            "pcb_source":{"contours":[
                {"id":"outer", "outerId":null, "primitives":lines},
                {"id":"circle", "outerId":"outer", "primitives":[{
                    "sourceId":"authored-circle", "segmentIndex":0,
                    "geometry":{"kind":"circle", "center":{"x":10.,"y":5.}, "end":{"x":11.,"y":5.}, "width":null},
                    "curveSupport":{"center":{"x":10.,"y":5.}, "radiusMm":1.,
                        "startDirection":{"x":1.,"y":0.}, "sweepRadians":std::f64::consts::TAU}
                }]}
            ], "slots":[]}},
        "regions":[{"outer":points, "holes":[], "cavities":[], "gaskets":[], "mounts":[]}]
    })).unwrap();
    let model = crate::build_case(&input).unwrap();
    for target in [None, Some(0.01)] {
        let (mesh, trace) = checked_trace(&model, target);
        assert_eq!(mesh.bounds, [[0., 0., 0.], [20., 10., 2.]]);
        assert_eq!(
            trace
                .faces
                .iter()
                .filter(|f| f.boundaries.len() == 3)
                .count(),
            2,
            "both caps retain outer, authored circular trim, and drilled trim"
        );
        assert_eq!(
            trace
                .faces
                .iter()
                .filter(|f| matches!(f.surface, Surface::RevolutionSurface(_)))
                .count(),
            2
        );
        assert!(trace.triangles.iter().all(|t| t.uv.is_ok()));
        assert!(
            trace
                .faces
                .iter()
                .filter(|f| matches!(f.surface, Surface::RevolutionSurface(_)))
                .all(|f| f.chart == ChartObservation::UnknownPeriodicUnwrap)
        );
    }
}

fn prepared_plate() -> boardstudio_contracts::PreparedCaseIR {
    serde_json::from_str(include_str!("../../tests/fixtures/layered/pcb-simple.json")).unwrap()
}

#[test]
fn final_trace_covers_native_boss_and_countersink_without_qualifying_curves() {
    use boardstudio_contracts::{CaseFeature, CaseKind, MountKind};
    let mut boss = prepared_plate();
    boss.body.kind = CaseKind::Tray;
    boss.body.thickness = 2.;
    boss.body.wall_height = Some(4.);
    boss.regions[0].cavities = vec![
        [[2., 2.], [18., 2.], [18., 14.], [2., 14.]]
            .map(|[x, y]| Vec2 { x, y })
            .to_vec(),
    ];
    let mount = &mut boss.regions[0].mounts[0];
    mount.kind = MountKind::Boss;
    mount.hole_diameter = 1.;
    mount.boss_diameter = Some(4.);
    mount.height = Some(3.);
    let mut cone = prepared_plate();
    cone.body.thickness = 4.;
    cone.regions[0].mounts.clear();
    cone.body.features = Some(vec![CaseFeature::ConicalSeat {
        id: "countersink".into(),
        at: Vec2 { x: 10., y: 8. },
        z: 2.,
        height: 2.,
        diameter: 4.,
        end_diameter: 2.,
    }]);
    for (input, expected_curves) in [(boss, 2), (cone, 1)] {
        let model = crate::build_case(&input).unwrap();
        for target in [None, Some(0.01)] {
            let (_, trace) = checked_trace(&model, target);
            let curved = trace
                .faces
                .iter()
                .filter(|f| matches!(f.surface, Surface::RevolutionSurface(_)))
                .collect::<Vec<_>>();
            assert_eq!(curved.len(), expected_curves);
            assert!(
                curved
                    .iter()
                    .all(|f| f.chart == ChartObservation::UnknownPeriodicUnwrap)
            );
            assert!(trace.triangles.iter().all(|t| t.uv.is_ok()));
        }
    }
}

#[test]
fn final_trace_is_body_local_with_elevation_and_source_placement_once() {
    use monstertruck_modeling::{Matrix4, Rad};
    let mut input = prepared_plate();
    input.body.z = Some(7.);
    input.regions[0].mounts.clear();
    let elevated = crate::build_case(&input).unwrap();
    let (mesh, _) = checked_trace(&elevated, None);
    assert_eq!(mesh.bounds, [[0., 0., 7.], [20., 16., 8.6]]);
    let translated = ExactModel {
        solids: elevated
            .solids
            .iter()
            .map(|s| builder::translated(s, Vector3::new(32., -16., 0.)))
            .collect(),
    };
    let (mesh, _) = checked_trace(&translated, None);
    assert_eq!(mesh.bounds, [[32., -16., 7.], [52., 0., 8.6]]);
    let rotated = ExactModel {
        solids: plate()
            .solids
            .iter()
            .map(|s| builder::rotated(s, Point3::origin(), Vector3::unit_z(), Rad(0.3)))
            .collect(),
    };
    checked_trace(&rotated, None);
    // Reflection reverses winding to preserve the solid; no physical row is applied.
    let reflected = ExactModel {
        solids: plate()
            .solids
            .iter()
            .map(|s| {
                let mut s = builder::transformed(s, Matrix4::from_nonuniform_scale(-1., 1., 1.));
                s.not();
                s
            })
            .collect(),
    };
    let (mesh, _) = checked_trace(&reflected, None);
    assert_eq!(mesh.bounds, [[-20., 0., 0.], [0., 10., 2.]]);
}

#[test]
fn final_trace_missing_malformed_and_placeholder_uv_never_panics() {
    use monstertruck_modeling::Vector2;
    let mut poly = PolygonMesh::default();
    poly.extend_uv_coords([
        Vector2::new(0., 0.),
        Vector2::new(1., 0.),
        Vector2::new(0., 1.),
    ]);
    let vertices = [0, 1, 2].map(|pos| StandardVertex {
        pos,
        uv: Some(pos),
        nor: None,
    });
    assert_eq!(
        triangle_uv(&poly, vertices),
        Ok([[0., 0.], [1., 0.], [0., 1.]])
    );
    for corner in 0..3 {
        for (uv, reason) in [
            (None, UvUnknown::Missing),
            (Some(99), UvUnknown::InvalidIndex),
        ] {
            let mut invalid = vertices;
            invalid[corner].uv = uv;
            assert_eq!(triangle_uv(&poly, invalid), Err(reason));
        }
    }
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        poly.uv_coords_mut()[0].x = value;
        assert_eq!(triangle_uv(&poly, vertices), Err(UvUnknown::NonFinite));
    }
    poly.uv_coords_mut().fill(Vector2::new(0., 0.));
    assert_eq!(
        triangle_uv(&poly, vertices),
        Err(UvUnknown::DegenerateOrPlaceholder)
    );
}

#[test]
fn final_trace_keeps_crossed_periodic_uv_raw_and_unwrap_unknown() {
    use monstertruck_modeling::Vector2;
    let model = crate::build_case(&prepared_plate()).unwrap();
    let (_, trace) = checked_trace(&model, None);
    let face = trace
        .faces
        .iter()
        .find(|f| matches!(f.surface, Surface::RevolutionSurface(_)))
        .unwrap();
    let mut poly = PolygonMesh::default();
    poly.extend_uv_coords([
        Vector2::new(0., 3.),
        Vector2::new(0., -3.),
        Vector2::new(1., -3.),
    ]);
    let vertices = [0, 1, 2].map(|pos| StandardVertex {
        pos,
        uv: Some(pos),
        nor: None,
    });
    assert_eq!(
        triangle_uv(&poly, vertices),
        Ok([[0., 3.], [0., -3.], [1., -3.]])
    );
    assert_eq!(face.chart, ChartObservation::UnknownPeriodicUnwrap);
    // A short-looking span is also unqualified: no caller or requested setting
    // can supply the missing mesher chart/period correspondence.
    poly.uv_coords_mut()[0].y = -2.9;
    assert!(triangle_uv(&poly, vertices).is_ok());
    assert_eq!(
        chart_observation(&face.surface),
        ChartObservation::UnknownPeriodicUnwrap
    );
}

#[test]
fn final_trace_preserves_fan_and_inverted_uv_corner_order() {
    use monstertruck_meshing::rexport_polymesh::{Faces, StandardAttributes};
    use monstertruck_modeling::Vector2;
    let vertices = [0, 1, 2, 3, 4].map(|pos| StandardVertex {
        pos,
        uv: Some(4 - pos),
        nor: None,
    });
    let mut poly = PolygonMesh::new(
        StandardAttributes {
            positions: vec![Point3::origin(); 5],
            uv_coords: (0..5)
                .map(|i| Vector2::new(f64::from(i), f64::from(i * i)))
                .collect(),
            ..Default::default()
        },
        Faces::from_iter([vertices]),
    );
    for inverted in [false, true] {
        if inverted {
            poly.invert();
        }
        let triangles = poly.faces().triangle_iter().collect::<Vec<_>>();
        let order = if inverted {
            [4, 3, 2, 1, 0]
        } else {
            [0, 1, 2, 3, 4]
        };
        assert_eq!(triangles.len(), 3);
        for (i, tri) in triangles.iter().enumerate() {
            assert_eq!(tri.map(|v| v.pos), [order[0], order[i + 1], order[i + 2]]);
            let expected = tri.map(|v| {
                let uv = poly.uv_coords()[4 - v.pos];
                [uv.x, uv.y]
            });
            assert_eq!(triangle_uv(&poly, *tri), Ok(expected));
        }
    }
}

#[test]
fn final_trace_does_not_retain_triangles_rejected_by_real_emission() {
    use monstertruck_modeling::Matrix4;
    let model = ExactModel {
        solids: plate()
            .solids
            .iter()
            .map(|s| builder::transformed(s, Matrix4::from_translation(Vector3::new(1e10, 0., 0.))))
            .collect(),
    };
    let mut trace = AnalysisTrace::default();
    let error = mesh_profile_for_analysis_impl(&model, None, Some(&mut trace)).unwrap_err();
    assert_eq!(error, mesh_profile_for_analysis(&model, None).unwrap_err());
    assert!(error.contains("f32 render precision"), "{error}");
    assert!(trace.triangles.is_empty());
}

#[test]
fn final_trace_unsupported_support_has_explicit_unknown_chart() {
    use monstertruck_modeling::BsplineSurface;
    let surface = Surface::BsplineSurface(BsplineSurface::debug_new(
        (vec![0., 0., 1., 1.].into(), vec![0., 0., 1., 1.].into()),
        vec![
            vec![Point3::new(0., 0., 0.), Point3::new(0., 1., 0.)],
            vec![Point3::new(1., 0., 0.), Point3::new(1., 1., 0.1)],
        ],
    ));
    assert_eq!(
        chart_observation(&surface),
        ChartObservation::UnknownUnsupportedSupport
    );
}

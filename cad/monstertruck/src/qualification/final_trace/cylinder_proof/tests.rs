use super::*;
use boardstudio_contracts::Vec2;
use monstertruck_modeling::{Invertible, builder};
fn bound(model: &ExactModel, mesh: &GeneratedAnalysisMesh, trace: &AnalysisTrace) -> f64 {
    match qualify(model, mesh, trace) {
        GeneratedQualification::Bounded {
            direct_bound_mm,
            source_precision_mm,
            total_bound_mm,
            ..
        } => {
            assert_eq!(source_precision_mm, 0.);
            assert_eq!(direct_bound_mm, total_bound_mm);
            direct_bound_mm
        }
        other => panic!("expected complete symmetric generated proof, got {other:?}"),
    }
}

#[test]
fn drilled_plate_has_a_complete_symmetric_source_bound() {
    let mut input = prepared_plate();
    input.body.thickness = 2.;
    let model = crate::build_case(&input).unwrap();
    let (mesh, trace) = checked_trace(&model, None);
    let bound = bound(&model, &mesh, &trace);
    assert!(bound > 0. && bound < 0.2, "{bound}");
}

fn drilled_input(radius: f64, thickness: f64) -> boardstudio_contracts::PreparedCaseIR {
    let mut input = prepared_plate();
    input.body.thickness = thickness;
    input.regions[0].outer = [[0., 0.], [20., 0.], [20., 10.], [0., 10.]]
        .map(|[x, y]| Vec2 { x, y })
        .to_vec();
    input.regions[0].mounts[0].at = Vec2 { x: 10., y: 5. };
    input.regions[0].mounts[0].hole_diameter = 2. * radius;
    input
}

#[test]
fn cylinder_proof_authored_radius_height_and_actual_refinement_controls() {
    for (radius, height) in [(0.5, 2.), (1., 2.), (1., 4.)] {
        let model = crate::build_case(&drilled_input(radius, height)).unwrap();
        let (coarse, trace) = checked_trace(&model, None);
        let eb = bound(&model, &coarse, &trace);
        let (fine, trace) = checked_trace(&model, Some(0.01));
        let ef = bound(&model, &fine, &trace);
        eprintln!("authored R={radius} H={height}: coarse={eb:.17} refined={ef:.17}");
        assert!(ef < eb && ef < 0.04 && eb < 0.4);
        assert!(fine.positions_mm_f64.len() > coarse.positions_mm_f64.len());
        assert_eq!(fine.bounds, [[0., 0., 0.], [20., 10., height]]);
        // Independent chord-midpoint observation only checks the certificate's
        // lower sanity band; the certificate itself never samples an error.
        let mut witnessed: f64 = 0.;
        for tri in fine.positions_mm_f64.as_chunks::<9>().0 {
            let a = [tri[0] - 10., tri[1] - 5.];
            let b = [tri[3] - 10., tri[4] - 5.];
            if (a[0].hypot(a[1]) - radius).abs() < 1e-12
                && (b[0].hypot(b[1]) - radius).abs() < 1e-12
            {
                witnessed =
                    witnessed.max((radius - ((a[0] + b[0]) / 2.).hypot((a[1] + b[1]) / 2.)).abs());
            }
        }
        assert!(witnessed > 0. && ef >= witnessed);
        assert_eq!(fine.analysis_error_bound_mm, None);
    }
}

#[test]
fn cylinder_proof_authored_circle_pcb_with_real_drill_is_bounded() {
    let input = circular_drilled_input(2., 0.);
    let model = crate::build_case(&input).unwrap();
    let (mesh, trace) = checked_trace(&model, Some(0.01));
    assert_eq!(
        trace
            .faces
            .iter()
            .filter(|f| matches!(f.surface, Surface::RevolutionSurface(_)))
            .count(),
        2
    );
    assert!(bound(&model, &mesh, &trace) < 0.04);
}

fn circular_drilled_input(thickness: f64, elevation: f64) -> boardstudio_contracts::PreparedCaseIR {
    use boardstudio_contracts::{Mount, MountKind};
    let mut input: boardstudio_contracts::PreparedCaseIR = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/pcb-source/circle-tracer.json"
    ))
    .unwrap();
    input.body.thickness = thickness;
    input.body.z = Some(elevation);
    input.body.mounts = Some(vec![Mount {
        id: "authored-drill".into(),
        kind: MountKind::Hole,
        at: Vec2 { x: 10., y: 8. },
        hole_diameter: 1.,
        boss_diameter: None,
        height: None,
    }]);
    input
}

#[test]
fn cylinder_proof_ordinary_1_6_source_pcb_has_a_complete_height_enclosure() {
    for input in [drilled_input(1., 1.6), circular_drilled_input(1.6, 0.)] {
        let model = crate::build_case(&input).unwrap();
        let (mesh, trace) = checked_trace(&model, Some(0.01));
        assert!(bind(&model, &mesh, &trace).is_ok());
        assert_eq!(mesh.bounds[0][2], 0.);
        let error = bound(&model, &mesh, &trace);
        assert!(error < 0.04);
        assert!((q(mesh.bounds[1][2]).unwrap() - q(1.6).unwrap()).abs() <= q(error).unwrap());
        let expected_xy = if input.body.pcb_source.is_some() {
            [[8., 6.], [12., 10.]] // Authored center (10,8), radius 2.
        } else {
            [[0., 0.], [20., 10.]]
        };
        for (actual, expected) in mesh.bounds.iter().zip(expected_xy) {
            for axis in 0..2 {
                assert!(
                    (q(actual[axis]).unwrap() - q(expected[axis]).unwrap()).abs()
                        <= q(error).unwrap()
                );
            }
        }
        let groups = bind(&model, &mesh, &trace).unwrap();
        let mut radii = BTreeSet::new();
        for (face, tris) in trace.faces.iter().zip(&groups) {
            if matches!(face.surface, Surface::RevolutionSurface(_)) {
                let (c, _) = cylinder(face, tris).unwrap();
                assert_eq!(c.z, [n(0), q(1.6).unwrap()]);
                assert_eq!(
                    c.center,
                    if input.body.pcb_source.is_some() {
                        [n(10), n(8)]
                    } else {
                        [n(10), n(5)]
                    }
                );
                radii.insert(c.radius);
            }
        }
        assert_eq!(
            radii,
            if input.body.pcb_source.is_some() {
                BTreeSet::from([n(2), n(1) / n(2)])
            } else {
                BTreeSet::from([n(1)])
            }
        );
        let (ci, wi) = upper_circular_trim(&trace);
        let (circle, _) = cylinder(&trace.faces[ci], &groups[ci]).unwrap();
        let trim = curve_ring(
            &trace.faces[ci].boundaries[wi],
            &circle.center,
            &circle.radius,
            &q(1.6).unwrap(),
        )
        .unwrap();
        assert!(
            trim.height > n(0),
            "exercise the real retained nonplanarity"
        );
        assert!(
            circle.mesh_height > n(0),
            "exercise final evaluation rounding"
        );
        let (coarse, coarse_trace) = checked_trace(&model, None);
        let coarse_error = bound(&model, &coarse, &coarse_trace);
        assert!(coarse_error > error && coarse_error < 0.2);
        eprintln!(
            "1.6 source-backed={}: default={coarse_error:.17} tight={error:.17}",
            input.body.pcb_source.is_some()
        );
        assert_eq!(mesh.analysis_error_bound_mm, None);
    }
}

fn upper_circular_trim(trace: &AnalysisTrace) -> (usize, usize) {
    let (fi, face) = trace
        .faces
        .iter()
        .enumerate()
        .find(|(_, face)| matches!(face.surface, Surface::RevolutionSurface(_)))
        .unwrap();
    let (wi, _) = face
        .boundaries
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            a.front_vertex()
                .unwrap()
                .point()
                .z
                .total_cmp(&b.front_vertex().unwrap().point().z)
        })
        .unwrap();
    (fi, wi)
}

#[test]
fn cylinder_proof_height_distortion_grows_and_encloses_an_independent_source_witness() {
    let mut previous = n(0);
    for delta in [0., 0.03125, 0.125, 0.25, 1.] {
        let model = crate::build_case(&drilled_input(1., 1.6)).unwrap();
        let (mesh, trace) = checked_trace(&model, Some(0.01));
        let (ci, wi) = upper_circular_trim(&trace);
        let edge = trace.faces[ci].boundaries[wi].edge_iter().next().unwrap();
        let Curve::NurbsCurve(mut raw) = edge.curve() else {
            unreachable!()
        };
        raw.control_point_mut(1).z += delta;
        // Independent closed-form B-spline basis at t=1/8 in the actual first
        // span [0,1/4]: (N0,N1,N2)=(2,5,1)/8. No extraction/evaluation helper.
        assert_eq!(&raw.knot_vector()[..5], &[0., 0., 0., 0.25, 0.5]);
        let mut zz = n(0);
        let mut ww = n(0);
        for (i, weight) in [2, 5, 1].into_iter().enumerate() {
            zz += n(weight) * q(raw.control_point(i).z).unwrap();
            ww += n(weight) * q(raw.control_point(i).w).unwrap();
        }
        let witness_z = zz / ww;
        let final_max_z = mesh
            .positions_mm_f64
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| q(p[2]).unwrap())
            .max()
            .unwrap();
        edge.set_curve(Curve::NurbsCurve(raw));
        assert!(bind(&model, &mesh, &trace).is_ok());
        if delta == 1. {
            unknown(&model, &mesh, &trace); // height slabs no longer separated
            continue;
        }
        let error = q(bound(&model, &mesh, &trace)).unwrap();
        assert!(error > previous);
        assert!(
            error >= &witness_z - final_max_z,
            "retained upper source point lies above every final triangle"
        );
        previous = error;
        let groups = bind(&model, &mesh, &trace).unwrap();
        let (circle, radial) = cylinder(&trace.faces[ci], &groups[ci]).unwrap();
        let ids = wire_ids(&trace.faces[ci].boundaries[wi]);
        let cap = trace
            .faces
            .iter()
            .position(|face| {
                matches!(face.surface, Surface::Plane(_))
                    && face.boundaries.iter().any(|w| wire_ids(w) == ids)
            })
            .unwrap();
        let local_cap = planar(&trace.faces[cap], &groups[cap], &[circle], &radial, &n(0)).unwrap();
        assert!(
            local_cap >= (&witness_z - q(1.6).unwrap()).abs(),
            "cap cannot discard its own trim height contribution"
        );
        eprintln!(
            "height coefficient delta={delta}: bound={:.17}",
            previous.to_f64().unwrap()
        );
    }
}

#[test]
fn cylinder_proof_1_6_malformed_trim_domains_and_denominators_are_unknown() {
    use monstertruck_modeling::{BsplineCurve, NurbsCurve};
    for mutation in 0..4 {
        let model = crate::build_case(&drilled_input(1., 1.6)).unwrap();
        let (mesh, trace) = checked_trace(&model, Some(0.01));
        let (ci, wi) = upper_circular_trim(&trace);
        let edge = trace.faces[ci].boundaries[wi].edge_iter().next().unwrap();
        let Curve::NurbsCurve(mut raw) = edge.curve() else {
            unreachable!()
        };
        match mutation {
            0 => raw.control_point_mut(1).w = 0.,
            1 => raw.control_point_mut(1).w = -0.25,
            2 => {
                // Retain only a first subarc, with the real end vertex intact.
                assert!(raw.control_points().len() > 3);
                raw = NurbsCurve::new(BsplineCurve::new(
                    vec![0., 0., 0., 1., 1., 1.].into(),
                    raw.control_points()[..3].to_vec(),
                ));
            }
            _ => {
                let mut knots = raw.knot_vector().iter().copied().collect::<Vec<_>>();
                let mut controls = raw.control_points().clone();
                knots.insert(4, 0.5);
                knots.insert(4, 0.5);
                controls.insert(3, controls[3]);
                controls.insert(3, controls[3]);
                raw = NurbsCurve::new(BsplineCurve::new(knots.into(), controls));
            }
        }
        edge.set_curve(Curve::NurbsCurve(raw));
        assert!(bind(&model, &mesh, &trace).is_ok());
        unknown(&model, &mesh, &trace);
    }
}

#[test]
fn cylinder_proof_1_6_height_bands_enter_narrow_hole_separation() {
    let mut input = drilled_input(1., 1.6);
    input.regions[0].mounts[0].at.x = 1.03125;
    let model = crate::build_case(&input).unwrap();
    let (mesh, trace) = checked_trace(&model, Some(0.01));
    assert!(bound(&model, &mesh, &trace) < 0.03125);
    let (ci, wi) = upper_circular_trim(&trace);
    let edge = trace.faces[ci].boundaries[wi].edge_iter().next().unwrap();
    let Curve::NurbsCurve(mut raw) = edge.curve() else {
        unreachable!()
    };
    raw.control_point_mut(1).z += 0.03125;
    edge.set_curve(Curve::NurbsCurve(raw));
    assert!(bind(&model, &mesh, &trace).is_ok());
    match qualify(&model, &mesh, &trace) {
        GeneratedQualification::Unknown { reasons } => assert!(
            reasons[0].contains("trim band reaches exterior"),
            "{reasons:?}"
        ),
        other => panic!("narrow domain must refuse inflated trim slab: {other:?}"),
    }
}

#[test]
fn cylinder_proof_1_6_source_plane_displacement_is_charged() {
    let model = crate::build_case(&drilled_input(1., 1.6)).unwrap();
    let (mesh, mut trace) = checked_trace(&model, Some(0.01));
    let original = q(bound(&model, &mesh, &trace)).unwrap();
    let cap = trace.faces.iter().position(|face| {
        matches!(&face.surface, Surface::Plane(plane) if plane_frame(plane).unwrap().0 == 2 && plane_frame(plane).unwrap().1 > n(0))
    }).unwrap();
    let Surface::Plane(plane) = &trace.faces[cap].surface else {
        unreachable!()
    };
    let mut value = serde_json::to_value(plane).unwrap();
    value["transform"]["w"]["z"] = serde_json::json!(0.125);
    let surface = Surface::Plane(serde_json::from_value(value).unwrap());
    model.solids[0].boundaries()[0]
        .face_iter()
        .nth(cap)
        .unwrap()
        .set_surface(surface.clone());
    trace.faces[cap].surface = surface;
    assert!(bind(&model, &mesh, &trace).is_ok());
    let displaced = q(bound(&model, &mesh, &trace)).unwrap();
    assert!(displaced > original && displaced >= n(1) / n(8));
}

#[test]
fn cylinder_proof_1_6_final_cap_and_end_height_displacement_is_charged() {
    for delta in [0.125, 2.] {
        let model = crate::build_case(&drilled_input(1., 1.6)).unwrap();
        let (mut mesh, mut trace) = checked_trace(&model, Some(0.01));
        let original = q(bound(&model, &mesh, &trace)).unwrap();
        // An independent raw positive-weight hull bounds every source trim Z.
        let mut source_max_z = q(1.6).unwrap();
        for face in &trace.faces {
            for wire in &face.boundaries {
                for edge in wire.edge_iter() {
                    if let Curve::NurbsCurve(raw) = edge.curve() {
                        for p in raw.control_points() {
                            source_max_z = source_max_z.max(q(p.z).unwrap() / q(p.w).unwrap());
                        }
                    }
                }
            }
        }
        // Move the actual shared hole boundary on cap and cylinder; outer
        // rectangle/side boundaries stay put. This leaves a sloping final cap.
        for face in &mut trace.faces {
            for p in face.polygon.positions_mut() {
                let dx = q(p.x).unwrap() - n(10);
                let dy = q(p.y).unwrap() - n(5);
                if p.z > 0. && &dx * &dx + &dy * &dy < n(2) {
                    p.z += delta;
                }
            }
        }
        for (i, tri) in trace.triangles.iter_mut().enumerate() {
            for (j, vertex) in tri.vertices.iter().enumerate() {
                let p = trace.faces[tri.occurrence_index].polygon.positions()[vertex.pos];
                for (axis, coordinate) in [p.x, p.y, p.z].into_iter().enumerate() {
                    tri.positions_mm_f64[j * 3 + axis] = coordinate;
                    tri.rendered_positions_f32[j * 3 + axis] = coordinate as f32;
                    mesh.positions_mm_f64[i * 9 + j * 3 + axis] = coordinate;
                    mesh.mesh.positions[i * 9 + j * 3 + axis] = coordinate as f32;
                }
            }
        }
        assert!(bind(&model, &mesh, &trace).is_ok());
        if delta == 2. {
            unknown(&model, &mesh, &trace);
        } else {
            let error = q(bound(&model, &mesh, &trace)).unwrap();
            let final_max_z = mesh
                .positions_mm_f64
                .as_chunks::<3>()
                .0
                .iter()
                .map(|p| q(p[2]).unwrap())
                .max()
                .unwrap();
            assert!(error > original && error >= final_max_z - source_max_z);
        }
    }
}

#[test]
fn cylinder_proof_1_6_elevation_translation_and_reflection_are_applied_once() {
    use monstertruck_modeling::Matrix4;
    for thickness in [1.6, 2.] {
        let mut input = drilled_input(1., thickness);
        input.body.z = Some(7.);
        let model = crate::build_case(&input).unwrap();
        let translated = ExactModel {
            circles: Default::default(),
            solids: model
                .solids
                .iter()
                .map(|s| builder::translated(s, Vector3::new(32., -16., 8.)))
                .collect(),
        };
        let (mesh, trace) = checked_trace(&translated, Some(0.01));
        let error = q(bound(&translated, &mesh, &trace)).unwrap();
        assert!(error < n(1) / n(25));
        let expected = [[32., -16., 15.], [52., -6., 15. + thickness]];
        for (actual, expected) in mesh.bounds.iter().zip(expected) {
            for axis in 0..3 {
                assert!((q(actual[axis]).unwrap() - q(expected[axis]).unwrap()).abs() <= error);
            }
        }
        let reflected = ExactModel {
            circles: Default::default(),
            solids: translated
                .solids
                .iter()
                .map(|s| {
                    let mut s =
                        builder::transformed(s, Matrix4::from_nonuniform_scale(-1., 1., 1.));
                    s.not();
                    s
                })
                .collect(),
        };
        let (mesh, trace) = checked_trace(&reflected, Some(0.01));
        assert_eq!(mesh.bounds[0][0], -52.);
        assert_eq!(mesh.bounds[1][0], -32.);
        assert!(bind(&reflected, &mesh, &trace).is_ok());
        // Reflected native processors are outside this canonical-carrier class.
        match qualify(&reflected, &mesh, &trace) {
            GeneratedQualification::Unknown { reasons } => {
                assert!(reasons[0].contains("affine transform"), "{reasons:?}")
            }
            other => panic!("unqualified reflected carrier must refuse: {other:?}"),
        }
    }
}

#[test]
fn cylinder_proof_native_boss_caps_and_drilled_wall_are_bounded() {
    use boardstudio_contracts::{CaseKind, MountKind};
    let mut input = prepared_plate();
    input.body.kind = CaseKind::Tray;
    input.body.thickness = 2.;
    input.body.wall_height = Some(4.);
    input.regions[0].cavities = vec![
        [[2., 2.], [18., 2.], [18., 14.], [2., 14.]]
            .map(|[x, y]| Vec2 { x, y })
            .to_vec(),
    ];
    let mount = &mut input.regions[0].mounts[0];
    mount.kind = MountKind::Boss;
    mount.hole_diameter = 1.;
    mount.boss_diameter = Some(4.);
    mount.height = Some(3.);
    let model = crate::build_case(&input).unwrap();
    let (mesh, trace) = checked_trace(&model, Some(0.01));
    assert!(bound(&model, &mesh, &trace) < 0.04);
}

#[test]
fn cylinder_proof_clearance_is_independent_of_requested_deflection() {
    let mut input = drilled_input(1., 2.);
    input.regions[0].mounts[0].at.x = 1.03125;
    let model = crate::build_case(&input).unwrap();
    let (coarse, trace) = checked_trace(&model, None);
    assert!(matches!(
        qualify(&model, &coarse, &trace),
        GeneratedQualification::Unknown { .. }
    ));
    let (fine, trace) = checked_trace(&model, Some(0.01));
    assert!(bound(&model, &fine, &trace) < 0.03125);
}

fn unknown(model: &ExactModel, mesh: &GeneratedAnalysisMesh, trace: &AnalysisTrace) {
    match qualify(model, mesh, trace) {
        GeneratedQualification::Unknown { reasons } => assert!(!reasons.is_empty()),
        other => panic!("expected Unknown, got {other:?}"),
    }
}

// Mutation-only helper: keep malformed delivered buffers/trace consistently
// bound, so coverage tests must fail the proof rather than a stale-data check.
fn mutate_face_triangle(
    mesh: &mut GeneratedAnalysisMesh,
    trace: &mut AnalysisTrace,
    face_index: usize,
    duplicate: bool,
) {
    use monstertruck_meshing::rexport_polymesh::Faces;
    let poly = &mut trace.faces[face_index].polygon;
    let mut triangles = poly.faces().triangle_iter().collect::<Vec<_>>();
    let ti = trace
        .triangles
        .iter()
        .position(|t| t.occurrence_index == face_index)
        .unwrap();
    if duplicate {
        let copy = triangles[0];
        triangles.push(copy);
        let old = &trace.triangles[ti];
        let new = FinalTriangle {
            occurrence_index: face_index,
            vertices: old.vertices,
            uv: old.uv,
            positions_mm_f64: old.positions_mm_f64,
            rendered_positions_f32: old.rendered_positions_f32,
        };
        mesh.positions_mm_f64.extend(new.positions_mm_f64);
        mesh.mesh.positions.extend(new.rendered_positions_f32);
        mesh.mesh.normals.extend_from_within(ti * 9..ti * 9 + 9);
        mesh.triangle_face_occurrence_indices.push(face_index);
        trace.triangles.push(new);
    } else {
        triangles.remove(0);
        trace.triangles.remove(ti);
        mesh.positions_mm_f64.drain(ti * 9..ti * 9 + 9);
        mesh.mesh.positions.drain(ti * 9..ti * 9 + 9);
        mesh.mesh.normals.drain(ti * 9..ti * 9 + 9);
        mesh.triangle_face_occurrence_indices.remove(ti);
    }
    *poly = PolygonMesh::new(poly.attributes().clone(), Faces::from_iter(triangles));
}

#[test]
fn cylinder_proof_missing_duplicate_and_seam_sliver_fail_after_binding() {
    let model = crate::build_case(&drilled_input(1., 2.)).unwrap();
    for curved in [false, true] {
        for duplicate in [false, true] {
            let (mut mesh, mut trace) = checked_trace(&model, None);
            let fi = trace
                .faces
                .iter()
                .position(|f| matches!(f.surface, Surface::RevolutionSurface(_)) == curved)
                .unwrap();
            mutate_face_triangle(&mut mesh, &mut trace, fi, duplicate);
            assert!(bind(&model, &mesh, &trace).is_ok());
            unknown(&model, &mesh, &trace);
        }
    }
}

#[test]
fn cylinder_proof_missing_uv_face_and_occurrence_are_unknown() {
    let model = crate::build_case(&drilled_input(1., 2.)).unwrap();
    for mutation in 0..5 {
        let (mut mesh, mut trace) = checked_trace(&model, None);
        match mutation {
            0 => {
                trace.triangles[0].uv = Err(UvUnknown::Missing);
            }
            1 => {
                trace.faces.pop();
            }
            2 => {
                mesh.source_faces.pop();
            }
            3 => {
                mesh.triangle_face_occurrence_indices[0] = usize::MAX;
            }
            _ => {
                trace.faces[0]
                    .polygon
                    .uv_coords_mut()
                    .fill(monstertruck_modeling::Vector2::new(0., 0.));
            }
        }
        unknown(&model, &mesh, &trace);
    }
}

#[test]
fn cylinder_proof_rejects_bad_span_retrace_and_normalization() {
    let r = n(1);
    let center = [n(0), n(0)];
    let p = |x, y| [n(x), n(y), n(0)];
    assert!(turn(&[p(1, 0), p(-1, 1), p(-1, -1)], &center).is_err());
    assert!(turn(&[p(1, 0), p(0, 1), p(1, 0), p(0, -1)], &center).is_err());
    assert!(triangle_radial_error(&[p(1, 0), p(-1, 1), p(-1, -1)], &center, &r).is_err());
    let model = crate::build_case(&drilled_input(1., 2.)).unwrap();
    let (_, mut trace) = checked_trace(&model, None);
    let fi = trace
        .faces
        .iter()
        .position(|f| matches!(f.surface, Surface::RevolutionSurface(_)))
        .unwrap();
    let tris = bind(
        &model,
        &mesh_profile_for_analysis(&model, None).unwrap(),
        &trace,
    )
    .unwrap();
    let Surface::RevolutionSurface(s) = &trace.faces[fi].surface else {
        unreachable!()
    };
    let mut value = serde_json::to_value(s).unwrap();
    value["entity"]["revolution"]["axis"]["z"] = serde_json::json!(2.);
    let bad = serde_json::from_value(value).unwrap();
    trace.faces[fi].surface = Surface::RevolutionSurface(bad);
    assert!(cylinder(&trace.faces[fi], &tris[fi]).is_err());
}

#[test]
fn cylinder_proof_cone_remains_unknown() {
    use boardstudio_contracts::CaseFeature;
    let mut input = drilled_input(1., 4.);
    input.regions[0].mounts.clear();
    input.body.features = Some(vec![CaseFeature::ConicalSeat {
        id: "cone".into(),
        at: Vec2 { x: 10., y: 5. },
        z: 2.,
        height: 2.,
        diameter: 4.,
        end_diameter: 2.,
    }]);
    let model = crate::build_case(&input).unwrap();
    let (mesh, trace) = checked_trace(&model, None);
    unknown(&model, &mesh, &trace);
}

#[test]
fn cylinder_proof_source_translation_and_large_coordinate_controls() {
    let model = crate::build_case(&drilled_input(1., 2.)).unwrap();
    let (mesh, trace) = checked_trace(&model, None);
    let e = bound(&model, &mesh, &trace);
    let translated = ExactModel {
        circles: Default::default(),
        solids: model
            .solids
            .iter()
            .map(|s| builder::translated(s, Vector3::new(32., 16., 8.)))
            .collect(),
    };
    let (mesh, trace) = checked_trace(&translated, None);
    let moved = bound(&translated, &mesh, &trace);
    assert!((moved - e).abs() < 1e-12); // test observation only; each proof bounds its own final rounding.
    assert!(q(1e20).is_err());
    let mut huge = mesh;
    huge.positions_mm_f64[0] = 1e20;
    unknown(&translated, &huge, &trace);
}

#[test]
fn cylinder_proof_source_coefficients_have_a_continuous_numeric_contribution() {
    let model = crate::build_case(&drilled_input(1., 2.)).unwrap();
    let (mesh, trace) = checked_trace(&model, None);
    let fi = trace
        .faces
        .iter()
        .position(|f| matches!(f.surface, Surface::RevolutionSurface(_)))
        .unwrap();
    let groups = bind(&model, &mesh, &trace).unwrap();
    let (original, _) = cylinder(&trace.faces[fi], &groups[fi]).unwrap();
    assert_eq!(original.source_error, n(0));
    let edge = trace.faces[fi].boundaries[0]
        .edge_iter()
        .next()
        .unwrap()
        .clone();
    let Curve::NurbsCurve(mut curve) = edge.curve() else {
        unreachable!()
    };
    curve.control_point_mut(1).x = curve.control_point(1).x.next_up();
    edge.set_curve(Curve::NurbsCurve(curve));
    assert!(bind(&model, &mesh, &trace).is_ok());
    let (changed, _) = cylinder(&trace.faces[fi], &groups[fi]).unwrap();
    assert!(changed.source_error > n(0));
    assert!(prove(&model, &mesh, &trace).unwrap() > original.source_error);
    let Curve::NurbsCurve(mut curve) = edge.curve() else {
        unreachable!()
    };
    curve.control_point_mut(1).z += 0.25;
    edge.set_curve(Curve::NurbsCurve(curve));
    let (height_changed, _) = cylinder(&trace.faces[fi], &groups[fi]).unwrap();
    assert!(height_changed.source_error > changed.source_error);
    assert!(prove(&model, &mesh, &trace).unwrap() > n(1) / n(4));
}

#[test]
fn cylinder_proof_consistently_bound_cap_sliver_and_real_open_seam_are_unknown() {
    let model = crate::build_case(&drilled_input(1., 2.)).unwrap();
    let (mut mesh, mut trace) = checked_trace(&model, None);
    let face = 0;
    let vertex = trace
        .triangles
        .iter()
        .find(|t| t.occurrence_index == face)
        .unwrap()
        .vertices[0]
        .pos;
    trace.faces[face].polygon.positions_mut()[vertex].x += 1. / 1024.;
    for (i, tri) in trace
        .triangles
        .iter_mut()
        .enumerate()
        .filter(|(_, t)| t.occurrence_index == face)
    {
        for (j, v) in tri.vertices.iter().enumerate() {
            let p = trace.faces[face].polygon.positions()[v.pos];
            for (axis, value) in [p.x, p.y, p.z].into_iter().enumerate() {
                tri.positions_mm_f64[j * 3 + axis] = value;
                tri.rendered_positions_f32[j * 3 + axis] = value as f32;
                mesh.positions_mm_f64[i * 9 + j * 3 + axis] = value;
                mesh.mesh.positions[i * 9 + j * 3 + axis] = value as f32;
            }
        }
    }
    assert!(bind(&model, &mesh, &trace).is_ok());
    unknown(&model, &mesh, &trace);
    let model = crate::build_case(&drilled_input(2., 4.)).unwrap();
    let (mesh, trace) = checked_trace(&model, None);
    match qualify(&model, &mesh, &trace) {
        GeneratedQualification::Unknown { reasons } => {
            assert!(reasons.iter().any(|r| r.contains("seam")), "{reasons:?}")
        }
        other => panic!("unproven real seam must stay Unknown: {other:?}"),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test::wasm_bindgen_test]
fn generated_cylinder_proof_actual_wasm() {
    drilled_plate_has_a_complete_symmetric_source_bound();
    cylinder_proof_authored_radius_height_and_actual_refinement_controls();
    cylinder_proof_authored_circle_pcb_with_real_drill_is_bounded();
    cylinder_proof_native_boss_caps_and_drilled_wall_are_bounded();
    cylinder_proof_clearance_is_independent_of_requested_deflection();
    cylinder_proof_missing_duplicate_and_seam_sliver_fail_after_binding();
    cylinder_proof_missing_uv_face_and_occurrence_are_unknown();
    cylinder_proof_rejects_bad_span_retrace_and_normalization();
    cylinder_proof_cone_remains_unknown();
    cylinder_proof_ordinary_1_6_source_pcb_has_a_complete_height_enclosure();
    cylinder_proof_height_distortion_grows_and_encloses_an_independent_source_witness();
    cylinder_proof_1_6_malformed_trim_domains_and_denominators_are_unknown();
    cylinder_proof_1_6_height_bands_enter_narrow_hole_separation();
    cylinder_proof_1_6_source_plane_displacement_is_charged();
    cylinder_proof_1_6_final_cap_and_end_height_displacement_is_charged();
    cylinder_proof_1_6_elevation_translation_and_reflection_are_applied_once();
    cylinder_proof_source_translation_and_large_coordinate_controls();
    cylinder_proof_source_coefficients_have_a_continuous_numeric_contribution();
    cylinder_proof_consistently_bound_cap_sliver_and_real_open_seam_are_unknown();
    cylinder_proof_discontinuous_trim_knots_are_unknown();
    p1_asymmetric_shared_trim_cannot_publish_a_smaller_surface_bound();
    p1_raw_orientation_preserves_locus_when_reflected_knots_collide();
    p1_cap_trim_error_is_returned_without_a_cylinder_error_allowance();
    p1_cap_trim_error_participates_in_nesting_separation();
}

#[test]
fn cylinder_proof_discontinuous_trim_knots_are_unknown() {
    use monstertruck_modeling::{BsplineCurve, NurbsCurve};
    let model = crate::build_case(&drilled_input(1., 2.)).unwrap();
    let (mesh, trace) = checked_trace(&model, None);
    let face = trace
        .faces
        .iter()
        .find(|f| matches!(f.surface, Surface::RevolutionSurface(_)))
        .unwrap();
    let edge = face.boundaries[0].edge_iter().next().unwrap();
    let Curve::NurbsCurve(curve) = edge.curve() else {
        unreachable!()
    };
    let mut knots = curve.knot_vector().iter().copied().collect::<Vec<_>>();
    knots.insert(4, 0.5);
    knots.insert(4, 0.5);
    let mut controls = curve.control_points().clone();
    controls.insert(3, controls[3]);
    controls.insert(3, controls[3]);
    edge.set_curve(Curve::NurbsCurve(NurbsCurve::new(BsplineCurve::new(
        knots.into(),
        controls,
    ))));
    assert!(bind(&model, &mesh, &trace).is_ok());
    match qualify(&model, &mesh, &trace) {
        GeneratedQualification::Unknown { reasons } => {
            assert!(reasons[0].contains("trim knots"), "{reasons:?}")
        }
        other => panic!("discontinuous domain must refuse: {other:?}"),
    }
}

// Adversarial source mutation only: retain the real emitted fine mesh, and
// replace one actual shared native trim used backward on its cylinder and
// forward on its cap. The retained source, not a newly meshed replacement, is
// what a source/final certificate must bound.
struct AsymmetricTrim {
    model: ExactModel,
    mesh: GeneratedAnalysisMesh,
    trace: AnalysisTrace,
    side: (usize, usize),
    cap: (usize, usize),
    witness: P,
}

fn asymmetric_shared_trim() -> AsymmetricTrim {
    asymmetric_shared_trim_with_left(0.)
}

fn asymmetric_shared_trim_with_left(left: f64) -> AsymmetricTrim {
    use monstertruck_modeling::{BsplineCurve, NurbsCurve, Vector4};
    let mut input = drilled_input(1., 2.);
    input.regions[0].outer[0].x = left;
    input.regions[0].outer[3].x = left;
    let model = crate::build_case(&input).unwrap();
    let (mesh, trace) = checked_trace(&model, Some(0.01));
    let ci = trace
        .faces
        .iter()
        .position(|f| matches!(f.surface, Surface::RevolutionSurface(_)))
        .unwrap();
    let (wi, edge) = trace.faces[ci]
        .boundaries
        .iter()
        .enumerate()
        .find_map(|(wi, wire)| wire.edge_iter().find(|e| !e.orientation()).map(|e| (wi, e)))
        .unwrap();
    let (cap, cap_wire) = trace
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| matches!(f.surface, Surface::Plane(_)))
        .find_map(|(fi, f)| {
            f.boundaries.iter().enumerate().find_map(|(wi, w)| {
                w.edge_iter()
                    .any(|e| e.id() == edge.id() && e.orientation())
                    .then_some((fi, wi))
            })
        })
        .unwrap();
    let Curve::NurbsCurve(raw) = edge.curve() else {
        unreachable!()
    };
    let start = edge.absolute_front().point();
    let sx = (start.x - 10.).signum();
    let control = raw.control_point(1);
    let sy = (control.y / control.w - 5.).signum();
    let z = start.z;
    let w = 0.5_f64.sqrt();
    let controls = [
        (sx, 0., 1.),
        (sx * w, sy * w, w),
        (0., sy, 1.),
        (-sx * w, sy * w, w),
        (-sx, 0., 1.),
    ]
    .map(|(x, y, weight)| Vector4::new(10. * weight + x, 5. * weight + y, z * weight, weight))
    .to_vec();
    let curve = NurbsCurve::new(BsplineCurve::new(
        vec![0., 0., 0., 0.00008, 0.00015, 1e12, 1e12, 1e12].into(),
        controls,
    ));
    edge.set_curve(Curve::NurbsCurve(curve));
    let witness = [
        q(10.).unwrap() + q(sx).unwrap() * n(47) / n(64),
        q(5.).unwrap() + q(sy).unwrap() * n(47) / n(64),
        q(z).unwrap(),
    ];
    assert!(
        bind(&model, &mesh, &trace).is_ok(),
        "shared source mutation must retain binding"
    );
    AsymmetricTrim {
        model,
        mesh,
        trace,
        side: (ci, wi),
        cap: (cap, cap_wire),
        witness,
    }
}

// Positive homogeneous control hulls provide an independent continuous oracle:
// each dyadic subcurve and its endpoint chord stay outside the witness ball.
// Their homotopy therefore preserves winding about that witness.
fn trim_distance_and_winding(wire: &Wire, witness: &P, rho: &Q) -> i32 {
    let mut polygon = Vec::new();
    for edge in wire.edge_iter() {
        let Curve::NurbsCurve(raw) = edge.curve() else {
            unreachable!()
        };
        let knots = raw
            .knot_vector()
            .iter()
            .map(|v| q(*v).unwrap())
            .collect::<Vec<_>>();
        let control = raw
            .control_points()
            .iter()
            .map(|p| {
                [
                    q(p.x).unwrap(),
                    q(p.y).unwrap(),
                    q(p.z).unwrap(),
                    q(p.w).unwrap(),
                ]
            })
            .collect::<Vec<_>>();
        let mut points = Vec::new();
        for span in knots.windows(2).filter(|p| p[0] < p[1]) {
            for i in 0..64 {
                let a = &span[0] + (&span[1] - &span[0]) * n(i) / n(64);
                let b = &span[0] + (&span[1] - &span[0]) * n(i + 1) / n(64);
                let left = spline_value(&knots, &control, &a);
                let right = spline_value(&knots, &control, &b);
                let mid = spline_value(&knots, &control, &((&a + &b) / n(2)));
                let middle: [Q; 4] =
                    std::array::from_fn(|k| n(2) * &mid[k] - (&left[k] + &right[k]) / n(2));
                let hull = [left, middle, right].map(|p| {
                    assert!(p[3] > n(0));
                    std::array::from_fn::<_, 3, _>(|k| &p[k] / &p[3])
                });
                let distance2: Q = (0..3)
                    .map(|k| {
                        let lo = hull.iter().map(|p| &p[k]).min().unwrap();
                        let hi = hull.iter().map(|p| &p[k]).max().unwrap();
                        let d = if witness[k] < *lo {
                            lo - &witness[k]
                        } else if witness[k] > *hi {
                            &witness[k] - hi
                        } else {
                            n(0)
                        };
                        &d * &d
                    })
                    .sum();
                assert!(
                    distance2 > rho * rho,
                    "entire raw source subcurve must avoid witness ball"
                );
                if points.is_empty() {
                    points.push(hull[0].clone());
                }
                points.push(hull[2].clone());
            }
        }
        if !edge.orientation() {
            points.reverse();
        }
        if let Some(last) = polygon.last() {
            assert_eq!(last, &points[0]);
        }
        polygon.extend(points.into_iter().skip(usize::from(!polygon.is_empty())));
    }
    assert_eq!(polygon.first(), polygon.last());
    polygon.pop();
    let center = [witness[0].clone(), witness[1].clone()];
    let mut winding = 0;
    for (a, b) in edges(&polygon) {
        let a = centered(a, &center);
        let b = centered(b, &center);
        let d = det(&a, &b);
        if a[1] <= n(0) && b[1] > n(0) && d > n(0) {
            winding += 1;
        }
        if b[1] <= n(0) && a[1] > n(0) && d < n(0) {
            winding -= 1;
        }
    }
    winding
}

#[test]
fn p1_asymmetric_shared_trim_cannot_publish_a_smaller_surface_bound() {
    let AsymmetricTrim {
        model,
        mesh,
        trace,
        side: (ci, wi),
        cap: (cap, cap_wire),
        witness: p,
    } = asymmetric_shared_trim();
    let groups = bind(&model, &mesh, &trace).unwrap();
    assert_eq!(
        trace
            .faces
            .iter()
            .filter(|f| matches!(f.surface, Surface::RevolutionSurface(_)))
            .count(),
        1
    );
    let (circle, _) = cylinder(&trace.faces[ci], &groups[ci]).unwrap();
    let cap_error = curve_ring(
        &trace.faces[cap].boundaries[cap_wire],
        &circle.center,
        &circle.radius,
        &p[2],
    )
    .unwrap()
    .total();
    let rho = n(1) / n(32);
    assert!(cap_error > n(1) / n(4));
    assert!(cap_wire > 0, "witness winding is in a source hole");
    for (fi, face) in trace.faces.iter().enumerate() {
        if fi != cap
            && let Surface::Plane(plane) = &face.surface
        {
            let (axis, level, _) = plane_frame(plane).unwrap();
            assert!(
                (&p[axis] - level).abs() > rho,
                "every other source plane is also farther than 1/32"
            );
        }
    }
    assert_eq!(
        trim_distance_and_winding(&trace.faces[cap].boundaries[cap_wire], &p, &rho).abs(),
        1
    );
    let radial = centered(&p, &circle.center);
    assert!(
        dot2(&radial, &radial) > (&circle.radius + &rho) * (&circle.radius + &rho),
        "source cylinder is also farther than 1/32"
    );
    assert!(
        groups[cap].iter().any(|tri| {
            let signs = edges(tri)
                .map(|(a, b)| det(&sub(b, a), &sub(&p, a)))
                .collect::<Vec<_>>();
            tri.iter().all(|v| v[2] == p[2])
                && (signs.iter().all(|d| d >= &n(0)) || signs.iter().all(|d| d <= &n(0)))
        }),
        "witness lies on an actual final cap triangle"
    );
    assert!(trace.faces[cap].boundaries[cap_wire].edge_iter().any(|e| {
        e.orientation()
            && trace.faces[ci].boundaries[wi]
                .edge_iter()
                .any(|other| !other.orientation() && other.id() == e.id())
    }));
    match qualify(&model, &mesh, &trace) {
        GeneratedQualification::Bounded {
            direct_bound_mm, ..
        } => {
            eprintln!(
                "P1 corrected bound={direct_bound_mm:.17}; certified final/source distance >1/32"
            );
            assert!(
                q(direct_bound_mm).unwrap() >= rho,
                "P1: reported {direct_bound_mm}, but final cap witness is >1/32 from the source surface"
            );
        }
        GeneratedQualification::Unknown { .. } => (),
    }
}

#[test]
fn p1_raw_orientation_preserves_locus_when_reflected_knots_collide() {
    let case = asymmetric_shared_trim();
    let wire = &case.trace.faces[case.cap.0].boundaries[case.cap.1];
    let edge = wire
        .edge_iter()
        .find(|e| {
            e.orientation()
                && case.trace.faces[case.side.0].boundaries[case.side.1]
                    .edge_iter()
                    .any(|other| other.id() == e.id() && !other.orientation())
        })
        .unwrap();
    let Curve::NurbsCurve(raw) = edge.curve() else {
        unreachable!()
    };
    assert_ne!(raw.knot(3), raw.knot(4));
    let end = q(raw.knot(7)).unwrap();
    assert_ne!(
        &end - q(raw.knot(3)).unwrap(),
        &end - q(raw.knot(4)).unwrap()
    );
    // Exhibit the library's lossy f64 inversion only as a regression control.
    let mut rounded = raw;
    rounded.invert();
    assert_eq!(rounded.knot(3).to_bits(), rounded.knot(4).to_bits());
    let reversed: Wire = wire.edge_iter().rev().map(|edge| edge.inverse()).collect();
    let forward = curve_ring(wire, &[n(10), n(5)], &n(1), &case.witness[2]).unwrap();
    let backward = curve_ring(&reversed, &[n(10), n(5)], &n(1), &case.witness[2]).unwrap();
    assert!(forward.total() > n(1) / n(4));
    assert_eq!(
        forward.total(),
        backward.total(),
        "raw source enclosure must be orientation invariant"
    );
    assert_eq!(forward.sign, -backward.sign);
}

#[test]
fn p1_cap_trim_error_is_returned_without_a_cylinder_error_allowance() {
    let case = asymmetric_shared_trim();
    let groups = bind(&case.model, &case.mesh, &case.trace).unwrap();
    let (circle, mesh_error) =
        cylinder(&case.trace.faces[case.side.0], &groups[case.side.0]).unwrap();
    let cap = &case.trace.faces[case.cap.0];
    let error = curve_ring(
        &cap.boundaries[case.cap.1],
        &circle.center,
        &circle.radius,
        &case.witness[2],
    )
    .unwrap()
    .total();
    // Authenticate the cap independently, with no allowance inherited from a
    // cylinder pass. Its own raw trim error must still enter the returned band.
    let cap_band = planar(cap, &groups[case.cap.0], &[circle], &mesh_error, &n(0)).unwrap();
    assert_eq!(cap_band, mesh_error + error);
    assert!(cap_band > n(1) / n(4));
}

#[test]
fn p1_cap_trim_error_participates_in_nesting_separation() {
    let case = asymmetric_shared_trim_with_left(8.75);
    let groups = bind(&case.model, &case.mesh, &case.trace).unwrap();
    let (circle, mesh_error) =
        cylinder(&case.trace.faces[case.side.0], &groups[case.side.0]).unwrap();
    let cap = &case.trace.faces[case.cap.0];
    assert!(mesh_error < n(1) / n(4));
    assert_eq!(
        planar(cap, &groups[case.cap.0], &[circle], &mesh_error, &n(0)),
        Err("trim band reaches exterior"),
        "cap-only error must enlarge the domain separation band"
    );
}

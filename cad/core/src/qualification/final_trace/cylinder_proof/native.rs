//! Bind the generated B-rep and the delivered analysis triangles directly.

use super::*;
use crate::{
    ExactModel,
    layered::brep::{Face, Ring, Support, Topology},
    profile::{GeneratedAnalysisMesh, closed_oriented_shell_triangles},
};

pub(crate) fn qualify(model: &ExactModel, mesh: &GeneratedAnalysisMesh) -> GeneratedQualification {
    qualification(prove(model, mesh))
}

fn prove(model: &ExactModel, mesh: &GeneratedAnalysisMesh) -> Proof<Q> {
    let count = mesh.triangle_face_occurrence_indices.len();
    if count == 0
        || count > 10_000
        || mesh.source_faces.len() > count
        || mesh.positions_mm_f64.len() != count * 9
        || mesh.mesh.positions.len() != count * 9
        || mesh.mesh.normals.len() != count * 9
        || !mesh.mesh_closed_oriented
        || !mesh.deflection_mm.is_finite()
        || mesh.deflection_mm <= 0.
        || !mesh.angular_rad.is_finite()
        || mesh.angular_rad <= 0.
    {
        return Err("invalid final analysis buffer or settings");
    }

    let mut groups = vec![Vec::new(); mesh.source_faces.len()];
    for (i, &tag) in mesh.triangle_face_occurrence_indices.iter().enumerate() {
        let points = &mesh.positions_mm_f64[i * 9..][..9];
        let rendered = &mesh.mesh.positions[i * 9..][..9];
        if points.iter().zip(rendered).any(|(&value, &rendered)| {
            !rendered.is_finite() || (value as f32).to_bits() != rendered.to_bits()
        }) {
            return Err("final render/filter binding mismatch");
        }
        let triangle = std::array::from_fn(|j| {
            Point3::new(points[j * 3], points[j * 3 + 1], points[j * 3 + 2])
        });
        crate::mesh::triangle_normal(triangle).map_err(|_| "invalid final triangle")?;
        groups
            .get_mut(tag)
            .ok_or("invalid occurrence")?
            .push(triangle);
    }

    let mut bound = n(0);
    let mut offset = 0;
    for (solid_index, (topology, shell)) in model.shells().enumerate() {
        let end = offset + shell.len();
        let sources = mesh
            .source_faces
            .get(offset..end)
            .ok_or("missing native face")?;
        let groups = &groups[offset..end];
        for (face_index, ((face, source), tris)) in
            shell.iter().zip(sources).zip(groups).enumerate()
        {
            let support = match face.support {
                Support::Plane { .. } => "PLANE",
                Support::Round { .. } => "REVOLUTION_SURFACE",
            };
            if (source.solid_index, source.shell_index, source.face_index)
                != (solid_index, 0, face_index)
                || source.support_kind != support
                || tris.is_empty()
            {
                return Err("native support/trim census binding mismatch");
            }
        }
        let all = groups.iter().flatten().copied().collect::<Vec<_>>();
        if !closed_oriented_shell_triangles(&all) {
            return Err("incomplete final oriented shell");
        }
        let groups = groups
            .iter()
            .map(|tris| {
                tris.iter()
                    .map(|&[a, b, c]| Ok([point(a)?, point(b)?, point(c)?]))
                    .collect::<Proof<Vec<_>>>()
            })
            .collect::<Proof<Vec<_>>>()?;
        bound = bound.max(shell_bound(topology, shell, &groups)?);
        offset = end;
    }
    if offset != mesh.source_faces.len() || offset == 0 {
        return Err("extra or missing native source faces");
    }
    Ok(bound)
}

fn shell_bound(topology: &Topology, shell: &[Face], groups: &[Vec<[P; 3]>]) -> Proof<Q> {
    let mut circles = Vec::new();
    let mut mesh_error = n(0);
    let mut source_error = n(0);
    let mut bound = n(0);
    for (face, tris) in shell.iter().zip(groups) {
        if let Support::Round {
            center,
            lower,
            upper,
            outward,
        } = face.support
        {
            if lower[1] != upper[1] || lower[1] <= 0. || q(lower[0])? >= q(upper[0])? {
                return Err("unsupported conical or invalid round support");
            }
            let center = [q(center[0])?, q(center[1])?];
            let radius = q(lower[1])?;
            let sense = if outward == face.forward { 1 } else { -1 };
            let (circle, radial) = cylinder(topology, face, center, radius, sense, tris)?;
            bound = bound.max(&radial + &circle.source_error + &circle.mesh_height);
            source_error = source_error.max(circle.source_error.clone());
            mesh_error = mesh_error.max(radial);
            circles.push(circle);
        }
    }
    for (face, tris) in shell.iter().zip(groups) {
        if matches!(face.support, Support::Plane { .. }) {
            bound = bound.max(plane(
                topology,
                face,
                tris,
                &circles,
                &mesh_error,
                &source_error,
            )?);
        }
    }
    if circles.iter().any(|circle| bound >= circle.radius) {
        return Err("radial band reaches axis");
    }
    Ok(bound)
}

fn ring_ids(ring: &Ring) -> BTreeSet<usize> {
    ring.iter().map(|edge| edge.index).collect()
}

fn cylinder(
    topology: &Topology,
    face: &Face,
    center: [Q; 2],
    radius: Q,
    sense: i32,
    tris: &[[P; 3]],
) -> Proof<(Circle<BTreeSet<usize>>, Q)> {
    let [first, last] = face.rings.as_slice() else {
        return Err("incomplete cylindrical trim domain");
    };
    let (a, a_sign, a_error) = circle_ring(topology, first, &center, &radius, face.forward)?;
    let (b, b_sign, b_error) = circle_ring(topology, last, &center, &radius, face.forward)?;
    if a == b || a_sign != if a < b { sense } else { -sense } || b_sign != -a_sign {
        return Err("source cylinder end orientation/height");
    }

    // A merged face's carrier can be seeded from any of its axial strips.
    // The actual native trim rings supply the complete axial domain.
    let mut circle = Circle {
        center,
        radius,
        z: [a.clone().min(b.clone()), a.clone().max(b.clone())],
        source_error: a_error.max(b_error),
        source_edges: vec![ring_ids(first), ring_ids(last)],
        source_levels: vec![a, b],
        mesh_rings: vec![],
        mesh_levels: vec![],
        mesh_height: n(0),
    };
    let mesh_error = cylinder_mesh(&mut circle, tris, Some(sense))?;
    Ok((circle, mesh_error))
}

// The retained arc is the oriented interval between endpoint rays on its authored
// circle. The sweep magnitude is a sampling input. For proof use its sign and
// exact endpoint directions, including diametrically opposite endpoints.
fn circle_ring(
    topology: &Topology,
    ring: &Ring,
    center: &[Q; 2],
    radius: &Q,
    forward: bool,
) -> Proof<(Q, i32, Q)> {
    if !(2..=128).contains(&ring.len()) {
        return Err("missing or oversized circular trim");
    }
    let mut z = None;
    let mut sign = 0;
    let mut winding = 0;
    let mut error = n(0);
    let radius2 = radius * radius;
    for (edge, next) in ring.iter().zip(ring.iter().cycle().skip(1)) {
        let [a, b] = edge.vertices(topology);
        if a == b || b != next.vertices(topology)[0] {
            return Err("disconnected source arc");
        }
        let a = point(topology.vertices[a])?;
        let b = point(topology.vertices[b])?;
        let arc = topology.edges[edge.index]
            .circle
            .ok_or("missing native circular carrier")?;
        let level = z.get_or_insert_with(|| a[2].clone());
        if [q(arc.center.x)?, q(arc.center.y)?] != *center
            || q(arc.center.z)? != *level
            || a[2] != *level
            || b[2] != *level
            || q(arc.radius)? != *radius
            || !arc.sweep.is_finite()
            || arc.sweep == 0.
            || arc.sweep.abs() > std::f64::consts::PI + 1e-10
        {
            return Err("unbound native circular trim");
        }
        let mut step = if arc.sweep > 0. { 1 } else { -1 };
        if edge.forward != forward {
            step = -step;
        }
        if sign != 0 && sign != step {
            return Err("source angular retrace");
        }
        sign = step;
        let (a, b) = if forward { (a, b) } else { (b, a) };
        let a = centered(&a, center);
        let b = centered(&b, center);
        for p in [&a, &b] {
            let r2 = dot2(p, p);
            if r2.is_zero() {
                return Err("source trim endpoint reaches axis");
            }
            error = error.max((r2 - &radius2).abs() / radius);
        }
        if dot2(&a, &b) < n(0) {
            // The rotated ray splits a near-half turn into two positively
            // oriented sectors, including a semicircle with exact opposite rays.
            let mid = [
                -n(i64::from(sign)) * &a[1],
                n(i64::from(sign)) * &a[0],
                n(0),
            ];
            winding += signed_crossing(&a, &mid, sign)?;
            winding += signed_crossing(&mid, &b, sign)?;
        } else {
            winding += signed_crossing(&a, &b, sign)?;
        }
    }
    if winding != sign {
        return Err("incomplete source circular turn");
    }
    Ok((z.ok_or("missing source trim level")?, sign, error))
}

fn signed_crossing(a: &P, b: &P, sign: i32) -> Proof<i32> {
    if det(a, b) * n(i64::from(sign)) <= n(0) {
        return Err("invalid source angular span");
    }
    Ok(if sign > 0 && a[1] <= n(0) && b[1] > n(0) {
        1
    } else if sign < 0 && b[1] <= n(0) && a[1] > n(0) {
        -1
    } else {
        0
    })
}

fn plane(
    topology: &Topology,
    face: &Face,
    tris: &[[P; 3]],
    circles: &[Circle<BTreeSet<usize>>],
    mesh_error: &Q,
    source_error: &Q,
) -> Proof<Q> {
    let Support::Plane { origin, u, v } = face.support else {
        return Err("unsupported plane");
    };
    let normal = cross(&[q(u.x)?, q(u.y)?, q(u.z)?], &[q(v.x)?, q(v.y)?, q(v.z)?]);
    let frame = axis_frame(
        &point(origin)?,
        if face.forward {
            normal
        } else {
            normal.map(|v| -v)
        },
    )?;
    let (axis, level, _) = &frame;
    let trims = face
        .rings
        .iter()
        .map(|ring| {
            if let Some((circle, end)) = circle_end(circles, &ring_ids(ring)) {
                if *axis != 2 {
                    return Err("nonhorizontal circular cap");
                }
                let c = &circles[circle];
                let (z, sign, error) =
                    circle_ring(topology, ring, &c.center, &c.radius, face.forward)?;
                if z != *level {
                    return Err("native circular cap leaves its plane");
                }
                Ok(PlanarTrim::Circular {
                    circle,
                    end,
                    sign,
                    error,
                })
            } else {
                let mut points = Vec::new();
                for (edge, next) in ring.iter().zip(ring.iter().cycle().skip(1)) {
                    let [a, b] = edge.vertices(topology);
                    if a == b
                        || b != next.vertices(topology)[0]
                        || topology.edges[edge.index].circle.is_some()
                    {
                        return Err("unsupported or disconnected native plane trim");
                    }
                    let p = point(topology.vertices[a])?;
                    if p[*axis] != *level {
                        return Err("native line trim leaves its plane");
                    }
                    points.push(p);
                }
                if !face.forward {
                    points.reverse();
                }
                let (points, error) = rectangle(points, *axis, level)?;
                Ok(PlanarTrim::Rectangle { points, error })
            }
        })
        .collect::<Proof<Vec<_>>>()?;
    if tris.iter().flatten().any(|p| p[*axis] != *level) {
        return Err("final triangles leave their native plane");
    }
    planar_domain(frame, trims, tris, circles, mesh_error, source_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        layered::brep::EdgeUse,
        mesh_profile_for_analysis,
        qualification::{
            Limits, PreparedAnalysisMesh, PreparedAnalysisView, SourceQualification,
            WorldSourceBound, prepare_generated_analysis, verify_analysis_pair,
        },
    };
    use boardstudio_contracts::{CaseFeature, MountKind, PreparedCaseIR, Vec2};

    fn prepared_plate() -> PreparedCaseIR {
        serde_json::from_str(include_str!(
            "../../../../tests/fixtures/layered/pcb-simple.json"
        ))
        .unwrap()
    }

    fn circular_drilled_input(thickness: f64, elevation: f64) -> PreparedCaseIR {
        let mut input: PreparedCaseIR = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/pcb-source/circle-tracer.json"
        ))
        .unwrap();
        input.body.thickness = thickness;
        input.body.z = Some(elevation);
        input.body.mounts = Some(vec![boardstudio_contracts::Mount {
            id: "authored-drill".into(),
            kind: MountKind::Hole,
            at: Vec2 { x: 10., y: 8. },
            hole_diameter: 1.,
            boss_diameter: None,
            height: None,
        }]);
        input
    }

    fn drilled_plate() -> PreparedCaseIR {
        let mut input = prepared_plate();
        input.body.thickness = 2.;
        let drill = &mut input.regions[0].mounts[0];
        drill.kind = MountKind::Hole;
        drill.hole_diameter = 2.;
        drill.at = Vec2 { x: 10., y: 8. };
        input
    }

    fn bounded(model: &ExactModel, target: Option<f64>) -> f64 {
        let prepared = prepare_generated_analysis(model, target).unwrap();
        let source = prepared.source_qualification();
        source
            .total_bound_mm()
            .unwrap_or_else(|| panic!("{source:?}"))
    }

    fn emission(prepared: &PreparedAnalysisMesh) -> &GeneratedAnalysisMesh {
        let PreparedAnalysisView::Generated(view) = prepared.export_view() else {
            panic!("expected native generated analysis");
        };
        view.emission
    }

    fn face_triangles(
        mesh: &GeneratedAnalysisMesh,
        face: usize,
    ) -> impl Iterator<Item = &[[f64; 3]; 3]> {
        mesh.positions_mm_f64
            .as_chunks::<9>()
            .0
            .iter()
            .zip(&mesh.triangle_face_occurrence_indices)
            .filter_map(move |(tri, &tag)| (tag == face).then_some(tri.as_chunks::<3>().0))
            .map(|tri| tri.try_into().unwrap())
    }

    fn assert_refuses(model: &ExactModel, mesh: &GeneratedAnalysisMesh, reason: &str) {
        match qualify(model, mesh) {
            GeneratedQualification::Unknown { reasons } => assert_eq!(reasons, [reason]),
            report => panic!("expected {reason}, got {report:?}"),
        }
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn native_cylinder_encloses_the_whole_final_chord() {
        let input = drilled_plate();
        let model = crate::build_case(&input).unwrap();
        let mut bounds = Vec::new();
        for target in [None, Some(0.01)] {
            let mesh = mesh_profile_for_analysis(&model, target).unwrap();
            let bound = bounded(&model, target);
            // Midpoints of all final cylinder chord edges at z=1 are actual
            // final surface points. The unit cylinder is the nearest source
            // face there (caps are 1 mm away and the rectangle is farther).
            let mut observed = 0.0_f64;
            for (positions, &tag) in mesh
                .positions_mm_f64
                .as_chunks::<9>()
                .0
                .iter()
                .zip(&mesh.triangle_face_occurrence_indices)
            {
                if mesh.source_faces[tag].support_kind != "REVOLUTION_SURFACE" {
                    continue;
                }
                let [a, b, c] = std::array::from_fn(|i| {
                    let i = i * 3;
                    [positions[i], positions[i + 1], positions[i + 2]]
                });
                for (a, b) in [(a, b), (b, c), (c, a)] {
                    if a[2] == b[2] {
                        continue;
                    }
                    let r = ((a[0] + b[0]) / 2. - 10.).hypot((a[1] + b[1]) / 2. - 8.);
                    observed = observed.max((1. - r).abs());
                }
            }
            assert!(observed > 0. && bound >= observed, "{bound} < {observed}");
            bounds.push(bound);
        }
        assert!((0.01..=0.015386).contains(&bounds[1]), "{bounds:?}");
        assert!(bounds[1] < bounds[0] && bounds[0] <= 0.058825, "{bounds:?}");
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn native_pcb_circular_exterior_and_drill_preserve_authored_elevation() {
        let input = circular_drilled_input(1.6, 7.);
        let model = crate::build_case(&input).unwrap();
        let mesh = mesh_profile_for_analysis(&model, None).unwrap();
        assert_eq!([mesh.bounds[0][2], mesh.bounds[1][2]], [7., 7. + 1.6]);
        let coarse = bounded(&model, None);
        // An angular partition may miss a cardinal point on the source rim.
        for (actual, authored) in
            mesh.bounds
                .into_iter()
                .flatten()
                .zip([8., 6., 7., 12., 10., 7. + 1.6])
        {
            assert!((actual - authored).abs() <= coarse);
        }
        let fine = bounded(&model, Some(0.01));
        assert!((0.01..=0.017979).contains(&fine));
        assert!(fine < coarse && coarse <= 0.117648);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn native_final_triangle_census_and_geometry_fail_closed() {
        let model = crate::build_case(&drilled_plate()).unwrap();
        let mesh = mesh_profile_for_analysis(&model, None).unwrap();
        assert!(matches!(
            qualify(&model, &mesh),
            GeneratedQualification::Bounded { .. }
        ));
        let assert_unknown = |mesh| {
            assert!(matches!(
                qualify(&model, &mesh),
                GeneratedQualification::Unknown { .. }
            ));
        };

        let mut bad = mesh.clone();
        bad.source_faces[1].face_index = 0;
        assert_unknown(bad);
        let mut bad = mesh.clone();
        bad.triangle_face_occurrence_indices[0] = bad.source_faces.len();
        assert_unknown(bad);
        let mut bad = mesh.clone();
        bad.mesh.positions[0] = bad.mesh.positions[0].next_up();
        assert_unknown(bad);
        let mut bad = mesh.clone();
        let i = bad
            .triangle_face_occurrence_indices
            .iter()
            .position(|&tag| bad.source_faces[tag].support_kind == "REVOLUTION_SURFACE")
            .unwrap();
        let z = (i * 9..(i + 1) * 9)
            .skip(2)
            .step_by(3)
            .find(|&j| bad.positions_mm_f64[j] == 2.)
            .expect("cylinder triangle reaches the top");
        bad.positions_mm_f64[z] = bad.positions_mm_f64[z].next_up();
        assert_eq!(bad.positions_mm_f64[z] as f32, bad.mesh.positions[z]);
        assert_unknown(bad);

        // Move every occurrence of the top elevation, so the mutated shell
        // still closes exactly and all rendered positions are byte-identical.
        // The plane evidence must detect the new f64 surface.
        let mut bad = mesh.clone();
        for z in bad.positions_mm_f64.iter_mut().skip(2).step_by(3) {
            if *z == 2. {
                *z = z.next_up();
            }
        }
        let tris = bad
            .positions_mm_f64
            .as_chunks::<9>()
            .0
            .iter()
            .map(|tri| {
                std::array::from_fn(|j| Point3::new(tri[j * 3], tri[j * 3 + 1], tri[j * 3 + 2]))
            })
            .collect::<Vec<_>>();
        assert!(closed_oriented_shell_triangles(&tris));
        assert!(
            bad.positions_mm_f64
                .iter()
                .zip(&bad.mesh.positions)
                .all(|(&f64, &f32)| (f64 as f32).to_bits() == f32.to_bits())
        );
        assert!(matches!(
            prove(&model, &bad),
            Err("final triangles leave their native plane" | "uncovered rectangular cap")
        ));

        let mut bad = mesh.clone();
        let tag = bad.triangle_face_occurrence_indices[i];
        bad.triangle_face_occurrence_indices.push(tag);
        bad.positions_mm_f64.extend_from_within(i * 9..(i + 1) * 9);
        bad.mesh.positions.extend_from_within(i * 9..(i + 1) * 9);
        bad.mesh.normals.extend_from_within(i * 9..(i + 1) * 9);
        assert_unknown(bad);
        let mut bad = mesh;
        bad.triangle_face_occurrence_indices.remove(i);
        bad.positions_mm_f64.drain(i * 9..(i + 1) * 9);
        bad.mesh.positions.drain(i * 9..(i + 1) * 9);
        bad.mesh.normals.drain(i * 9..(i + 1) * 9);
        assert_unknown(bad);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn native_same_support_sub_ulp_break_retessellates_but_shallow_cut_refuses() {
        let mut input = drilled_plate();
        let original = crate::build_case(&input).unwrap();
        let original_mesh = mesh_profile_for_analysis(&original, None).unwrap();
        let top = 2.0_f64;
        let breaks = [top.next_down(), top.next_down().next_down()];
        assert!(breaks.iter().all(|&z| z < top && z as f32 == top as f32));
        let cut = |x, z| CaseFeature::RoundSeat {
            id: format!("sub-ulp-seat-{z}"),
            at: Vec2 { x, y: 5. },
            z,
            height: top - z,
            diameter: 2.,
        };
        // Two outside cutters introduce consecutive collapsed strips without
        // changing material. Both rectangle walls and the bore traverse them.
        input.body.features = Some(breaks.map(|z| cut(40., z)).into());
        let split = crate::build_case(&input).unwrap();
        let (topology, faces) = split.shells().next().unwrap();
        for round in [true, false] {
            assert!(
                faces
                    .iter()
                    .filter(|face| matches!(face.support, Support::Round { .. }) == round)
                    .any(
                        |face| breaks.iter().all(|&z| face.strips.iter().any(|strip| {
                            topology.vertices[strip.lower.vertices(topology)[0]].z == z
                        }))
                    )
            );
        }
        let mesh = mesh_profile_for_analysis(&split, None).unwrap();
        assert!(
            !mesh
                .positions_mm_f64
                .iter()
                .skip(2)
                .step_by(3)
                .any(|z| breaks.contains(z))
        );
        assert!(mesh.mesh_closed_oriented);
        assert_eq!(mesh.bounds, original_mesh.bounds);
        assert_eq!(
            mesh.positions_mm_f64.len(),
            original_mesh.positions_mm_f64.len()
        );
        assert_eq!(bounded(&split, None), bounded(&original, None));

        // Bringing the cutter inside adds a real floor and a cylindrical wall
        // of positive f64 height. The delivery has no valid f32 triangles for
        // that wall, so it must fail before a source certificate can be issued.
        input.body.features = Some(vec![cut(5., breaks[0])]);
        let deformed = crate::build_case(&input).unwrap();
        let error = prepare_generated_analysis(&deformed, None)
            .err()
            .expect("shallow cut must refuse");
        assert!(error.contains("loses its orientation at f32"), "{error}");
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn native_boss_counterbore_and_annular_caps_enclose_the_final_chords() {
        let mut input = drilled_plate();
        let mount = &mut input.regions[0].mounts[0];
        mount.kind = MountKind::Boss;
        mount.boss_diameter = Some(4.);
        mount.height = Some(5.);
        mount.hole_diameter = 1.;
        input.body.features = Some(vec![CaseFeature::RoundSeat {
            id: "counterbore".into(),
            at: mount.at,
            z: 5.,
            height: 2.,
            diameter: 2.,
        }]);
        let model = crate::build_case(&input).unwrap();
        let (topology, faces) = model.shells().next().unwrap();
        let prepared = prepare_generated_analysis(&model, Some(0.01)).unwrap();
        let bound = prepared.source_qualification().total_bound_mm().unwrap();
        let mesh = emission(&prepared);
        let radial = |x: f64, y: f64| (x - 10.).hypot(y - 8.);

        let walls = [
            (false, 0., 5., 0.5),
            (false, 5., 7., 1.),
            (true, 2., 7., 2.),
        ];
        assert_eq!(
            faces
                .iter()
                .filter(|f| matches!(f.support, Support::Round { .. }))
                .count(),
            walls.len()
        );
        for (exterior, low, high, radius) in walls {
            let (face, native) = faces.iter().enumerate().find(|(_, face)| {
                matches!(face.support, Support::Round { center: [10., 8.], lower, upper, outward }
                    if lower[1] == radius && upper[1] == radius && (outward == face.forward) == exterior)
            }).unwrap();
            let mut end_levels = native
                .rings
                .iter()
                .map(|ring| topology.vertices[ring[0].vertices(topology)[0]].z)
                .collect::<Vec<_>>();
            end_levels.sort_by(f64::total_cmp);
            assert_eq!(end_levels, [low, high]);

            // Each wall spans the mid-height even when it has internal strips.
            // Intersect its delivered triangles with that plane and take each
            // section segment's midpoint. The other cylinder radii and all cap
            // levels are at least 0.5 mm away from this witness's nominal rim.
            let mid_z = (low + high) / 2.;
            let mut witnessed = 0.0_f64;
            for tri in face_triangles(mesh, face) {
                let section: Vec<_> = [(0, 1), (1, 2), (2, 0)]
                    .into_iter()
                    .filter_map(|(i, j)| {
                        let (a, b) = (tri[i], tri[j]);
                        if (a[2] < mid_z) == (b[2] < mid_z) {
                            return None;
                        }
                        let t = (mid_z - a[2]) / (b[2] - a[2]);
                        Some([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])])
                    })
                    .collect();
                if let [a, b] = section.as_slice() {
                    let r = radial((a[0] + b[0]) / 2., (a[1] + b[1]) / 2.);
                    witnessed = witnessed.max((radius - r).abs());
                }
            }
            assert!(
                witnessed > 0.001 && bound >= witnessed,
                "{radius}: {bound} < {witnessed}"
            );
        }
        assert!(bound < 0.025, "{bound}");

        for (z, inner, outer) in [(5., 0.5, 1.), (7., 1., 2.)] {
            let (tag, face) = faces
                .iter()
                .enumerate()
                .find(|(_, face)| {
                    matches!(face.support, Support::Plane { origin, u, v }
                    if origin.z == z && u.z == 0. && v.z == 0.)
                })
                .unwrap();
            assert_eq!(face.rings.len(), 2);
            assert!(
                face.rings
                    .iter()
                    .flatten()
                    .all(|edge| topology.edges[edge.index].circle.is_some())
            );
            let mut area = 0.;
            let mut witnessed = 0.0_f64;
            for &[a, b, c] in face_triangles(mesh, tag) {
                assert_eq!([a[2], b[2], c[2]], [z; 3]);
                let twice_area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
                assert!(twice_area > 0.);
                area += twice_area / 2.;
                for (a, b) in [(a, b), (b, c), (c, a)] {
                    // The inner chord midpoint lies inside the source hole.
                    // Its nearest source point is on the inner rim at this Z.
                    if [a, b]
                        .iter()
                        .all(|p| (radial(p[0], p[1]) - inner).abs() < 1e-12)
                    {
                        let r = radial((a[0] + b[0]) / 2., (a[1] + b[1]) / 2.);
                        witnessed = witnessed.max(inner - r);
                    }
                }
            }
            assert!(
                witnessed > 0.001 && bound >= witnessed,
                "{z}: {bound} < {witnessed}"
            );
            let annular_area = std::f64::consts::PI * (outer * outer - inner * inner);
            assert!((area / annular_area - 1.).abs() < 0.04, "{z}: {area}");
        }
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn native_narrow_hole_refuses_coarse_band_and_publishes_refined_bound() {
        let mut input = drilled_plate();
        input.regions[0].mounts[0].at.x = 1.03125;
        let model = crate::build_case(&input).unwrap();
        let coarse = prepare_generated_analysis(&model, None).unwrap();
        assert!(emission(&coarse).mesh_closed_oriented);
        assert!(matches!(
            coarse.source_qualification(),
            SourceQualification::Unknown { reasons } if reasons == &["trim band reaches exterior"]
        ));
        let fine = bounded(&model, Some(0.01));
        assert!((0.01..0.03125).contains(&fine), "{fine}");
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn native_xy_change_charges_a_witness_on_the_closed_final_surface() {
        let model = crate::build_case(&drilled_plate()).unwrap();
        let prepared = prepare_generated_analysis(&model, None).unwrap();
        let original = prepared.source_qualification().total_bound_mm().unwrap();
        let mut mesh = emission(&prepared).clone();
        let mut heights = BTreeSet::new();
        for xyz in mesh.positions_mm_f64.as_chunks_mut::<3>().0 {
            if [xyz[0], xyz[1]] == [11., 8.] {
                xyz[0] -= 0.25;
                heights.insert(xyz[2].to_bits());
            }
        }
        mesh.mesh = crate::MeshBuffers::default();
        for tri in mesh.positions_mm_f64.as_chunks::<9>().0 {
            let p =
                std::array::from_fn(|i| Point3::new(tri[i * 3], tri[i * 3 + 1], tri[i * 3 + 2]));
            crate::mesh::push_triangle(&mut mesh.mesh, p).unwrap();
        }
        assert_eq!(heights, [0.0_f64.to_bits(), 2.0_f64.to_bits()].into());
        // Updating every use on both rims preserves the vertical seam. Its
        // midpoint (10.75, 8, 1) lies 0.25 mm from the unit cylinder, with
        // both source caps and all rectangular walls farther away.
        assert!(mesh.positions_mm_f64.as_chunks::<9>().0.iter().any(|tri| {
            let points = tri.as_chunks::<3>().0;
            points.contains(&[10.75, 8., 0.]) && points.contains(&[10.75, 8., 2.])
        }));
        let GeneratedQualification::Bounded { total_bound_mm, .. } = qualify(&model, &mesh) else {
            panic!("coherently perturbed cylinder and caps must qualify");
        };
        assert!(
            original < 0.25 && total_bound_mm >= 0.25,
            "{original} -> {total_bound_mm}"
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn native_same_support_tag_swap_and_closed_flag_are_independently_checked() {
        let mut input = drilled_plate();
        input.regions[0].mounts.clear();
        input.body.features = Some(
            [5., 15.]
                .map(|x| CaseFeature::RoundSeat {
                    id: format!("seat-{x}"),
                    at: Vec2 { x, y: 8. },
                    z: 1.,
                    height: 1.,
                    diameter: 2.,
                })
                .into(),
        );
        let model = crate::build_case(&input).unwrap();
        let prepared = prepare_generated_analysis(&model, None).unwrap();
        assert!(prepared.source_qualification().total_bound_mm().is_some());
        let (topology, faces) = model.shells().next().unwrap();
        let floors = faces
            .iter()
            .enumerate()
            .filter(|(_, face)| {
                matches!(face.support, Support::Plane { origin, u, v }
                    if origin.z == 1. && u.z == 0. && v.z == 0.)
                    && face
                        .rings
                        .iter()
                        .flatten()
                        .all(|edge| topology.edges[edge.index].circle.is_some())
            })
            .map(|(tag, _)| tag)
            .collect::<Vec<_>>();
        let &[left, right] = floors.as_slice() else {
            panic!("expected two circular seat floors: {floors:?}");
        };
        let mesh = emission(&prepared);
        let mut bad = mesh.clone();
        // Swap the complete coplanar cap groups. Whole-shell incidence, plane
        // coordinates/orientation and group census survive, but the final
        // cap edges no longer match their own cylinder end edges.
        for tag in &mut bad.triangle_face_occurrence_indices {
            if *tag == left {
                *tag = right;
            } else if *tag == right {
                *tag = left;
            }
        }
        assert_refuses(&model, &bad, "cap/cylinder boundary mismatch");

        let mut bad = mesh.clone();
        bad.mesh_closed_oriented = false;
        assert_refuses(&model, &bad, "invalid final analysis buffer or settings");

        let mut bad = mesh.clone();
        let i = mesh
            .triangle_face_occurrence_indices
            .iter()
            .position(|&tag| tag == left)
            .unwrap();
        assert!(face_triangles(mesh, left).count() > 1);
        bad.triangle_face_occurrence_indices.remove(i);
        for positions in [&mut bad.mesh.positions, &mut bad.mesh.normals] {
            positions.drain(i * 9..(i + 1) * 9);
        }
        bad.positions_mm_f64.drain(i * 9..(i + 1) * 9);
        assert!(bad.mesh_closed_oriented);
        assert_refuses(&model, &bad, "incomplete final oriented shell");
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn native_retraced_unmatched_and_split_source_rings_refuse_closed_mesh() {
        let model = || crate::build_case(&drilled_plate()).unwrap();
        let baseline = model();
        let prepared = prepare_generated_analysis(&baseline, None).unwrap();
        assert!(prepared.source_qualification().total_bound_mm().is_some());
        let mesh = emission(&prepared);

        let mut retraced = model();
        let round = retraced.regions[0].shells[0]
            .iter_mut()
            .find(|face| matches!(face.support, Support::Round { .. }))
            .unwrap();
        let first = round.rings[0][0];
        round.rings[0].splice(0..0, [first, first.inverse()]);
        assert_refuses(&retraced, mesh, "source angular retrace");

        let mut split = model();
        let round = split.regions[0].shells[0]
            .iter_mut()
            .find(|face| matches!(face.support, Support::Round { .. }))
            .unwrap();
        let half_ring = round.rings[0].split_off(1);
        assert!(!half_ring.is_empty());
        round.rings.push(half_ring);
        assert_refuses(&split, mesh, "incomplete cylindrical trim domain");

        let mut unmatched = model();
        let region = &mut unmatched.regions[0];
        let (cap, ring) = region.shells[0]
            .iter()
            .enumerate()
            .filter(|(_, face)| matches!(face.support, Support::Plane { .. }))
            .find_map(|(cap, face)| {
                face.rings
                    .iter()
                    .position(|ring| region.topology.edges[ring[0].index].circle.is_some())
                    .map(|ring| (cap, ring))
            })
            .unwrap();
        let cap_ring = &mut region.shells[0][cap].rings[ring];
        let edge = region.topology.edges[cap_ring[0].index];
        let arc = edge.circle.unwrap();
        let center = [q(arc.center.x).unwrap(), q(arc.center.y).unwrap()];
        let radius = q(arc.radius).unwrap();
        let ring_before = circle_ring(&region.topology, cap_ring, &center, &radius, true).unwrap();
        cap_ring[0].index = region.topology.edges.len();
        region.topology.edges.push(edge);
        assert_refuses(
            &unmatched,
            mesh,
            "unsupported or disconnected native plane trim",
        );

        // Split the cap's semicircle into quarter arcs on the same authored
        // circle. The coincident locus alone cannot replace the shared edge IDs.
        let region = &mut unmatched.regions[0];
        let cap_ring = &mut region.shells[0][cap].rings[ring];
        let [a, b] = edge.vertices;
        let start = region.topology.vertices[a] - arc.center;
        let sign = arc.sweep.signum();
        let midpoint =
            arc.center + crate::layered::brep::Vector3::new(-sign * start.y, sign * start.x, 0.);
        let m = region.topology.vertices.len();
        region.topology.vertices.push(midpoint);
        let first = region.topology.edges.len();
        region
            .topology
            .edges
            .extend([[a, m], [m, b]].map(|vertices| crate::layered::brep::Edge {
                vertices,
                circle: Some(crate::layered::brep::Circle {
                    sweep: arc.sweep / 2.,
                    ..arc
                }),
            }));
        let forward = cap_ring[0].forward;
        let mut split = [first, first + 1].map(|index| EdgeUse { index, forward });
        if !forward {
            split.reverse();
        }
        cap_ring.splice(0..1, split);
        assert_eq!(
            circle_ring(&region.topology, cap_ring, &center, &radius, true).unwrap(),
            ring_before
        );
        assert_refuses(
            &unmatched,
            mesh,
            "unsupported or disconnected native plane trim",
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn native_disjoint_planar_shells_publish_zero_and_cone_refuses() {
        let mut input = drilled_plate();
        input.regions[0].mounts.clear();
        let mut other = input.regions[0].clone();
        for p in &mut other.outer {
            p.x += 32.;
        }
        input.regions.push(other);
        let model = crate::build_case(&input).unwrap();
        assert_eq!(model.shells().count(), 2);
        let prepared = prepare_generated_analysis(&model, None).unwrap();
        assert_eq!(prepared.source_qualification().total_bound_mm(), Some(0.));
        let mesh = emission(&prepared);
        for solid in 0..2 {
            assert!(
                mesh.triangle_face_occurrence_indices
                    .iter()
                    .any(|&tag| mesh.source_faces[tag].solid_index == solid)
            );
        }

        input.regions.pop();
        input.body.features = Some(vec![CaseFeature::ConicalSeat {
            id: "countersink".into(),
            at: Vec2 { x: 10., y: 8. },
            z: 1.,
            height: 1.,
            diameter: 4.,
            end_diameter: 2.,
        }]);
        let cone = crate::build_case(&input).unwrap();
        let prepared = prepare_generated_analysis(&cone, None).unwrap();
        assert!(emission(&prepared).mesh_closed_oriented);
        assert!(matches!(
            prepared.source_qualification(),
            SourceQualification::Unknown { reasons } if reasons == &["unsupported conical or invalid round support"]
        ));
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn native_nonzero_source_bound_scales_in_reflected_translated_world_row() {
        let mut input = drilled_plate();
        input.body.z = Some(7.);
        let model = crate::build_case(&input).unwrap();
        let prepared = prepare_generated_analysis(&model, Some(0.01)).unwrap();
        let local = prepared.source_qualification().total_bound_mm().unwrap();
        assert!(local > 0.);
        let identity = [
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
        ];
        let physical = [
            -2., 0., 0., 0., 0., 2., 0., 0., 0., 0., 2., 0., 128., 0., 16., 1.,
        ];
        let evidence = verify_analysis_pair(
            &prepared,
            &prepared,
            [identity, physical],
            Limits::default(),
        );
        let [base, world] = &evidence.source_bounds;
        assert!(
            matches!(base, WorldSourceBound::Bounded { total_bound_mm, .. } if *total_bound_mm == local)
        );
        let WorldSourceBound::Bounded {
            mesh_identity,
            norm_upper,
            direct_bound_mm,
            source_precision_mm,
            transform_rounding_mm,
            total_bound_mm,
            ..
        } = world
        else {
            panic!("{world:?}");
        };
        assert_eq!(*mesh_identity, prepared.analysis_mesh().mesh_identity);
        assert_eq!(
            (*norm_upper, *direct_bound_mm, *source_precision_mm),
            (2., 2. * local, 0.)
        );
        let mut rounding = n(0);
        for (p, actual) in emission(&prepared)
            .positions_mm_f64
            .as_chunks::<3>()
            .0
            .iter()
            .zip(evidence.world_positions()[1].as_chunks::<3>().0)
        {
            assert_eq!([actual[1], actual[2]], [2. * p[1], 2. * p[2] + 16.]);
            rounding =
                rounding.max((q(actual[0]).unwrap() - (n(128) - n(2) * q(p[0]).unwrap())).abs());
        }
        assert_eq!(
            evidence.world_positions()[1].len(),
            emission(&prepared).positions_mm_f64.len()
        );
        assert!(rounding > n(0));
        assert!(q(*transform_rounding_mm).unwrap() >= rounding);
        let exact_total = q(2. * local).unwrap() + q(*transform_rounding_mm).unwrap();
        assert!(q(*total_bound_mm).unwrap() >= exact_total);
        assert!(q(total_bound_mm.next_down()).unwrap() < exact_total);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn native_mixed_arc_rectangle_is_unqualified() {
        let input = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/pcb-source/line-arc-tracer.json"
        ))
        .unwrap();
        let prepared =
            prepare_generated_analysis(&crate::build_case(&input).unwrap(), None).unwrap();
        assert!(matches!(
            prepared.source_qualification(),
            SourceQualification::Unknown { .. }
        ));
    }
}

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
        mesh_profile_for_analysis,
        qualification::{SourceQualification, prepare_generated_analysis},
    };
    use boardstudio_contracts::{CaseFeature, MountKind, PreparedCaseIR, Vec2};

    fn drilled_plate() -> PreparedCaseIR {
        let mut input = super::super::super::prepared_plate();
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

    #[test]
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
                .chunks_exact(9)
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

    #[test]
    fn native_pcb_circular_exterior_and_drill_preserve_authored_elevation() {
        let input = super::super::tests::circular_drilled_input(1.6, 7.);
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

    #[test]
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

    #[test]
    fn native_same_support_sub_ulp_break_retessellates_but_shallow_cut_refuses() {
        let mut input = drilled_plate();
        let original = crate::build_case(&input).unwrap();
        let original_mesh = mesh_profile_for_analysis(&original, None).unwrap();
        let top = 2.0_f64;
        let break_z = top.next_down();
        assert_ne!(break_z, top);
        assert_eq!(break_z as f32, top as f32);
        let cut = |x| CaseFeature::RoundSeat {
            id: "sub-ulp-seat".into(),
            at: Vec2 { x, y: 5. },
            z: break_z,
            height: top - break_z,
            diameter: 2.,
        };
        // An outside cutter introduces the layer break without changing any
        // material. Both the rectangle walls and through-bore traverse it.
        input.body.features = Some(vec![cut(40.)]);
        let split = crate::build_case(&input).unwrap();
        let (topology, faces) = split.shells().next().unwrap();
        for round in [true, false] {
            assert!(
                faces
                    .iter()
                    .filter(|face| { matches!(face.support, Support::Round { .. }) == round })
                    .flat_map(|face| &face.strips)
                    .any(|strip| {
                        topology.vertices[strip.lower.vertices(topology)[0]].z == break_z
                    })
            );
        }
        let mesh = mesh_profile_for_analysis(&split, None).unwrap();
        assert!(
            !mesh
                .positions_mm_f64
                .iter()
                .skip(2)
                .step_by(3)
                .any(|&z| z == break_z)
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
        input.body.features = Some(vec![cut(5.)]);
        let deformed = crate::build_case(&input).unwrap();
        let error = prepare_generated_analysis(&deformed, None)
            .err()
            .expect("shallow cut must refuse");
        assert!(error.contains("loses its orientation at f32"), "{error}");
    }

    #[test]
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

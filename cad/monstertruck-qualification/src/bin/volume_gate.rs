use monstertruck_meshing::prelude::*;
use monstertruck_modeling::{
    BoundedCurve, Curve, Edge, Face, ParametricSurface, Point2, Point3, Shell, Solid, Surface,
    Vector3, builder,
};
use std::f64::consts::PI;

const GX: [f64; 5] = [
    0.046_910_077_030_668,
    0.230_765_344_947_158,
    0.5,
    0.769_234_655_052_842,
    0.953_089_922_969_332,
];
const GW: [f64; 5] = [
    0.118_463_442_528_095,
    0.239_314_335_249_683,
    0.284_444_444_444_444,
    0.239_314_335_249_683,
    0.118_463_442_528_095,
];

fn cylinder(radius: f64, height: f64, offset: [f64; 3]) -> Solid {
    let vertex = builder::vertex(Point3::new(
        offset[0],
        offset[1] - height / 2.0,
        offset[2] + radius,
    ));
    let circle = builder::revolve(
        &vertex,
        Point3::new(offset[0], offset[1], offset[2]),
        Vector3::unit_y(),
        builder::SweepAngle::Closed,
        2,
    );
    let disk = builder::try_attach_plane(&[circle]).expect("circle has a planar cap");
    builder::extrude(&disk, Vector3::new(0.0, height, 0.0))
}

fn inward_frustum_baseline(r0: f64, r1: f64, height: f64, offset: [f64; 3]) -> Solid {
    let p = [
        Point3::new(offset[0], offset[1], offset[2]),
        Point3::new(offset[0] + r0, offset[1], offset[2]),
        Point3::new(offset[0] + r1, offset[1], offset[2] + height),
        Point3::new(offset[0], offset[1], offset[2] + height),
    ];
    let v = p.map(builder::vertex);
    let wire: monstertruck_modeling::Wire = vec![
        builder::line(&v[0], &v[1]),
        builder::line(&v[1], &v[2]),
        builder::line(&v[2], &v[3]),
    ]
    .into();
    let shell = builder::revolve_wire(
        &wire,
        Point3::new(offset[0], offset[1], offset[2]),
        Vector3::unit_z(),
        builder::SweepAngle::Closed,
        4,
    );
    Solid::new(vec![shell])
}

fn corrected_frustum(r0: f64, r1: f64, height: f64, offset: [f64; 3]) -> Solid {
    // Matches the corrected qualification-gate profile traversal:
    // axis-top, outer-top, outer-bottom, axis-bottom.
    let p = [
        Point3::new(offset[0], offset[1], offset[2] + height),
        Point3::new(offset[0] + r1, offset[1], offset[2] + height),
        Point3::new(offset[0] + r0, offset[1], offset[2]),
        Point3::new(offset[0], offset[1], offset[2]),
    ];
    let v = p.map(builder::vertex);
    let wire: monstertruck_modeling::Wire = vec![
        builder::line(&v[0], &v[1]),
        builder::line(&v[1], &v[2]),
        builder::line(&v[2], &v[3]),
    ]
    .into();
    let shell = builder::revolve_wire(
        &wire,
        Point3::new(offset[0], offset[1], offset[2]),
        Vector3::unit_z(),
        builder::SweepAngle::Closed,
        4,
    );
    Solid::new(vec![shell])
}

fn circle_loop(radius: f64, offset: [f64; 3], reverse: bool) -> monstertruck_modeling::Wire {
    let vertex = builder::vertex(Point3::new(offset[0] + radius, offset[1], offset[2]));
    let loop_wire = builder::revolve(
        &vertex,
        Point3::new(offset[0], offset[1], offset[2]),
        Vector3::unit_z(),
        builder::SweepAngle::Closed,
        2,
    );
    if reverse {
        loop_wire.inverse()
    } else {
        loop_wire
    }
}

fn annular_prism(offset: [f64; 3]) -> Solid {
    let outer = circle_loop(5.0, offset, false);
    let inner = circle_loop(2.0, offset, true);
    monstertruck_modeling::profile::solid_from_planar_profile(
        vec![outer, inner],
        Vector3::new(0.0, 0.0, 3.0),
    )
    .expect("two-circle annular profile extrudes")
}

fn square_loop(x0: f64, y0: f64, size: f64, z: f64, reverse: bool) -> monstertruck_modeling::Wire {
    let mut points = vec![
        Point3::new(x0, y0, z),
        Point3::new(x0 + size, y0, z),
        Point3::new(x0 + size, y0 + size, z),
        Point3::new(x0, y0 + size, z),
    ];
    if reverse {
        points.reverse();
    }
    let vertices: Vec<_> = points.into_iter().map(builder::vertex).collect();
    let edges = (0..vertices.len())
        .map(|i| builder::line(&vertices[i], &vertices[(i + 1) % vertices.len()]))
        .collect::<Vec<_>>();
    edges.into()
}

fn square_tube(offset: [f64; 3]) -> Solid {
    let outer = square_loop(offset[0], offset[1], 10.0, offset[2], false);
    let inner = square_loop(offset[0] + 3.0, offset[1] + 3.0, 4.0, offset[2], true);
    monstertruck_modeling::profile::solid_from_planar_profile(
        vec![outer, inner],
        Vector3::new(0.0, 0.0, 3.0),
    )
    .expect("square profile with one hole extrudes")
}

fn tri_integral(
    surface: &Surface,
    [a, b, c]: [Point2; 3],
    reference: Point3,
) -> Result<f64, String> {
    if !finite3(reference) || ![a, b, c].iter().all(|p| p.x.is_finite() && p.y.is_finite()) {
        return Err("non-finite triangle coordinate or reference".to_string());
    }
    let (ab, ac) = (b - a, c - a);
    let jac = ab.x * ac.y - ab.y * ac.x;
    if !jac.is_finite() {
        return Err("non-finite UV triangle Jacobian".to_string());
    }
    if jac == 0.0 {
        return Err("degenerate UV triangle".to_string());
    }
    let mut sum = 0.0;
    for (i, s) in GX.iter().copied().enumerate() {
        for (j, t) in GX.iter().copied().enumerate() {
            let uv = a + ab * s + ac * ((1.0 - s) * t);
            let x = surface.evaluate(uv.x, uv.y);
            let normal_area = surface
                .derivative_u(uv.x, uv.y)
                .cross(surface.derivative_v(uv.x, uv.y));
            let f = (x - reference).dot(normal_area) / 3.0;
            if !finite3(x)
                || !normal_area.x.is_finite()
                || !normal_area.y.is_finite()
                || !normal_area.z.is_finite()
                || !f.is_finite()
            {
                return Err("non-finite surface value in UV triangle integral".to_string());
            }
            sum += GW[i] * GW[j] * f * jac * (1.0 - s);
            if !sum.is_finite() {
                return Err("non-finite UV triangle accumulation".to_string());
            }
        }
    }
    Ok(sum)
}

fn finite3(p: Point3) -> bool {
    p.x.is_finite() && p.y.is_finite() && p.z.is_finite()
}

fn validate_solid(solid: &Solid) -> Result<(), String> {
    if solid.boundaries().is_empty() {
        return Err("solid has no shells".to_string());
    }
    for (shell_index, shell) in solid.boundaries().iter().enumerate() {
        let mut uses: Vec<(Edge, usize, usize)> = Vec::new();
        let mut face_count = 0;
        for face in shell.face_iter() {
            face_count += 1;
            let boundaries = face.boundaries();
            if boundaries.is_empty() || boundaries.iter().any(|wire| wire.is_empty()) {
                return Err(format!(
                    "shell {shell_index} face has no non-empty boundary loops"
                ));
            }
            for wire in &boundaries {
                if !wire.is_closed() {
                    return Err(format!("shell {shell_index} has an open boundary loop"));
                }
                for edge in wire.edge_iter() {
                    let curve = edge.oriented_curve();
                    let (t0, t1) = curve
                        .try_range_tuple()
                        .ok_or_else(|| format!("shell {shell_index} edge has unbounded curve"))?;
                    if !t0.is_finite() || !t1.is_finite() || t0 >= t1 {
                        return Err(format!(
                            "shell {shell_index} edge has invalid parameter interval"
                        ));
                    }
                    let p0 = curve.evaluate(t0);
                    let p1 = curve.evaluate(t1);
                    let v0 = edge.front().point();
                    let v1 = edge.back().point();
                    if !finite3(p0)
                        || !finite3(p1)
                        || !finite3(v0)
                        || !finite3(v1)
                        || (p0 - v0).magnitude() > 1e-6
                        || (p1 - v1).magnitude() > 1e-6
                    {
                        return Err(format!(
                            "shell {shell_index} edge curve does not meet its oriented vertices"
                        ));
                    }
                    if let Some((_, forward, reverse)) =
                        uses.iter_mut().find(|(seen, _, _)| seen.is_same(edge))
                    {
                        if edge.orientation() {
                            *forward += 1;
                        } else {
                            *reverse += 1;
                        }
                    } else if edge.orientation() {
                        uses.push((edge.clone(), 1, 0));
                    } else {
                        uses.push((edge.clone(), 0, 1));
                    }
                }
            }
        }
        if face_count == 0 {
            return Err(format!("shell {shell_index} has no faces"));
        }
        if let Some((_, forward, reverse)) = uses.iter().find(|(_, f, r)| *f != 1 || *r != 1) {
            return Err(format!(
                "shell {shell_index} is not closed/oriented at an edge use ({forward} forward, {reverse} reverse)"
            ));
        }
    }
    Ok(())
}

fn exact_plane_edge_area(
    edge: &monstertruck_modeling::Edge,
    plane: &monstertruck_modeling::Plane,
) -> Result<f64, String> {
    let curve = edge.oriented_curve();
    if !matches!(curve, Curve::Line(_) | Curve::NurbsCurve(_)) {
        return Err("unsupported plane edge variant".to_string());
    }
    let (t0, t1) = curve.range_tuple();
    if !t0.is_finite() || !t1.is_finite() || t0 >= t1 {
        return Err(format!("invalid edge parameter domain [{t0}, {t1}]"));
    }
    let mut integral = 0.0;
    // Composite Gauss quadrature remains on the original curve parameter domain.
    // Subdivision controls empirical convergence; it is not an error certificate.
    let intervals = std::env::var("PLANE_EDGE_INTERVALS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(32);
    if intervals == 0 {
        return Err("plane edge interval count must be positive".to_string());
    }
    if std::env::var_os("PLANE_TRACE_EDGES").is_some() {
        let kind = match &curve {
            Curve::Line(_) => "Line",
            Curve::NurbsCurve(_) => "NurbsCurve",
            _ => unreachable!("variants rejected above"),
        };
        println!("plane_edge kind={kind} domain=[{t0:.17},{t1:.17}] subdivisions={intervals}");
    }
    for segment in 0..intervals {
        let a = t0 + (t1 - t0) * segment as f64 / intervals as f64;
        let b = t0 + (t1 - t0) * (segment + 1) as f64 / intervals as f64;
        for (node, weight) in GX.iter().copied().zip(GW.iter().copied()) {
            let t = a + (b - a) * node;
            let p = curve.evaluate(t);
            let dp = curve.derivative(t);
            if !finite3(p) || !dp.x.is_finite() || !dp.y.is_finite() || !dp.z.is_finite() {
                return Err("non-finite plane boundary curve value".to_string());
            }
            let prm = plane.parameter(p);
            let dprm = plane.parameter(p + dp) - prm;
            let reconstructed = plane.origin()
                + prm.x * plane.axis_u()
                + prm.y * plane.axis_v()
                + prm.z * plane.normal();
            let reconstruction_error = (reconstructed - p).magnitude();
            if !reconstruction_error.is_finite() || reconstruction_error > 1e-9 {
                return Err(format!(
                    "plane reconstruction error {reconstruction_error:e}"
                ));
            }
            if prm.z.abs() > 1e-9 {
                return Err(format!("edge leaves plane by {:e}", prm.z.abs()));
            }
            integral += 0.5 * weight * (b - a) * (prm.x * dprm.y - prm.y * dprm.x);
            if !integral.is_finite() {
                return Err("non-finite plane boundary accumulation".to_string());
            }
        }
    }
    Ok(integral)
}

fn exact_plane_face_volume(face: &Face, reference: Point3) -> Result<f64, String> {
    let Surface::Plane(plane) = face.surface() else {
        return Err("not a plane face".to_string());
    };
    let boundaries = face.boundaries();
    if boundaries.is_empty() || boundaries.iter().any(|wire| wire.is_empty()) {
        return Err("plane face has an empty boundary loop".to_string());
    }
    let signed_area_uv = boundaries
        .iter()
        .flat_map(|wire| wire.edge_iter())
        .try_fold(0.0, |sum, edge| {
            exact_plane_edge_area(edge, &plane).map(|area| sum + area)
        })?;
    let flux_density = (plane.origin() - reference).dot(plane.axis_u().cross(plane.axis_v())) / 3.0;
    let contribution = flux_density * signed_area_uv;
    if !contribution.is_finite() {
        return Err("non-finite plane face contribution".to_string());
    }
    Ok(contribution)
}

fn volume(solid: &Solid, mesh_tol: f64) -> Result<(f64, usize, usize), String> {
    if !mesh_tol.is_finite() || mesh_tol <= 0.0 {
        return Err("mesh tolerance must be finite and positive".to_string());
    }
    validate_solid(solid)?;
    let reference = solid
        .vertex_iter()
        .next()
        .ok_or_else(|| "solid has no vertex for local integration origin".to_string())?
        .point();
    let mut total = 0.0;
    let mut faces_seen = 0;
    let mut tris_seen = 0;
    for shell in solid.boundaries() {
        for face in shell.face_iter() {
            faces_seen += 1;
            let surface = face.surface();
            let orientation = if face.orientation() { 1.0 } else { -1.0 };
            let contribution = match surface {
                Surface::Plane(_) => {
                    let value = exact_plane_face_volume(face, reference)?;
                    total += value;
                    value
                }
                Surface::RevolutionSurface(_) | Surface::NurbsSurface(_) => {
                    // Triangulate a one-face shell, avoiding any assumption that whole-shell
                    // tessellation preserves face iteration order or identity.
                    let single_face: Shell = std::iter::once(face.clone()).collect();
                    let meshed_face_shell = single_face.triangulation(mesh_tol);
                    let mut mesh_faces = meshed_face_shell.face_iter();
                    let mesh_face = mesh_faces.next().ok_or_else(|| {
                        format!(
                            "face {} dropped at mesh tolerance {mesh_tol:e}",
                            faces_seen - 1
                        )
                    })?;
                    if mesh_faces.next().is_some() {
                        return Err("one-face triangulation returned multiple faces".to_string());
                    }
                    let poly = mesh_face
                        .surface()
                        .ok_or_else(|| format!("face {} has no polygon mesh", faces_seen - 1))?;
                    let mut face_total = 0.0;
                    let mut face_triangles = 0;
                    for tri in poly.faces().triangle_iter() {
                        let mut uv = [Point2::new(0.0, 0.0); 3];
                        for i in 0..3 {
                            let idx = tri[i]
                                .uv
                                .ok_or_else(|| "triangle missing uv index".to_string())?;
                            let p = poly
                                .uv_coords()
                                .get(idx)
                                .ok_or_else(|| format!("UV index {idx} is out of range"))?;
                            uv[i] = Point2::new(p.x, p.y);
                        }
                        face_total += tri_integral(&surface, uv, reference)?;
                        if !face_total.is_finite() {
                            return Err("non-finite curved face accumulation".to_string());
                        }
                        face_triangles += 1;
                        tris_seen += 1;
                    }
                    if face_triangles == 0 {
                        return Err(format!(
                            "curved face {} has no UV triangles",
                            faces_seen - 1
                        ));
                    }
                    let value = orientation * face_total;
                    if std::env::var_os("UV_TRACE_FACES").is_some() && mesh_tol >= 1e-3 {
                        let kind = if matches!(surface, Surface::NurbsSurface(_)) {
                            "nurbs"
                        } else {
                            "revolution"
                        };
                        println!(
                            "face={} kind={kind} orientation={} raw_uv_volume={:.15} signed_face_volume={:.15} tris={}",
                            faces_seen - 1,
                            orientation,
                            face_total,
                            value,
                            face_triangles
                        );
                    }
                    total += value;
                    value
                }
                _ => {
                    return Err(format!(
                        "unsupported surface kind on face {}",
                        faces_seen - 1
                    ));
                }
            };
            if !contribution.is_finite() || !total.is_finite() {
                return Err("non-finite solid volume accumulation".to_string());
            }
            if matches!(surface, Surface::Plane(_)) && std::env::var_os("UV_TRACE_FACES").is_some()
            {
                println!(
                    "face={} kind=plane_boundary orientation={} signed_face_volume={:.15}",
                    faces_seen - 1,
                    orientation,
                    contribution
                );
            }
        }
    }
    Ok((total, faces_seen, tris_seen))
}

fn main() {
    let cyl_exact = PI * 4.0 * 5.0;
    let cone_exact = PI * 4.0 * (2.25 + 1.125 + 0.5625) / 3.0;
    let tube_exact = (100.0 - 16.0) * 3.0;
    let annulus_exact = PI * (25.0 - 4.0) * 3.0;
    for (name, solid, physical_exact, signed_expected) in [
        (
            "cylinder-origin",
            cylinder(2.0, 5.0, [0.0; 3]),
            cyl_exact,
            cyl_exact,
        ),
        (
            "cylinder-translated",
            cylinder(2.0, 5.0, [1000.0, -2000.0, 3000.0]),
            cyl_exact,
            cyl_exact,
        ),
        (
            "square-hole-origin",
            square_tube([0.0; 3]),
            tube_exact,
            tube_exact,
        ),
        (
            "square-hole-translated",
            square_tube([1000.0, -2000.0, 3000.0]),
            tube_exact,
            tube_exact,
        ),
        (
            "annular-hole-origin",
            annular_prism([0.0; 3]),
            annulus_exact,
            annulus_exact,
        ),
        (
            "annular-hole-translated",
            annular_prism([1000.0, -2000.0, 3000.0]),
            annulus_exact,
            annulus_exact,
        ),
        // Preserve the inward-oriented pre-correction recipe as a negative baseline.
        (
            "frustum-inward-baseline-origin",
            inward_frustum_baseline(1.5, 0.75, 4.0, [0.0; 3]),
            cone_exact,
            -cone_exact,
        ),
        (
            "frustum-inward-baseline-translated",
            inward_frustum_baseline(1.5, 0.75, 4.0, [1000.0, -2000.0, 3000.0]),
            cone_exact,
            -cone_exact,
        ),
        // Matches the corrected profile order now used by accuracy_gate.rs.
        (
            "frustum-corrected-origin",
            corrected_frustum(1.5, 0.75, 4.0, [0.0; 3]),
            cone_exact,
            cone_exact,
        ),
        (
            "frustum-corrected-translated",
            corrected_frustum(1.5, 0.75, 4.0, [1000.0, -2000.0, 3000.0]),
            cone_exact,
            cone_exact,
        ),
    ] {
        let mut previous: Option<f64> = None;
        for tol in [1e-3, 1e-4] {
            match volume(&solid, tol) {
                Ok((measured, faces, tris)) => {
                    let error = (measured - signed_expected).abs();
                    assert!(
                        measured.is_finite() && error <= 1e-6,
                        "{name} failed signed analytic oracle: measured={measured}, expected={signed_expected}, error={error:e}"
                    );
                    if let Some(previous) = previous {
                        assert!(
                            (measured - previous).abs() <= 1e-6,
                            "{name} failed empirical mesh-tolerance convergence: {previous} vs {measured}"
                        );
                    }
                    previous = Some(measured);
                    println!(
                        "{name} mesh_tol={tol:.0e} signed_uv_volume={measured:.15} physical_analytic={physical_exact:.15} signed_expected={signed_expected:.15} signed_abs_error={error:.6e} faces={faces} triangles={tris}"
                    );
                }
                Err(error) => panic!("{name} mesh_tol={tol:.0e} failed closed: {error}"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offset_plane() -> Surface {
        Surface::Plane(monstertruck_modeling::Plane::new(
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(1.0, 0.0, 1.0),
            Point3::new(0.0, 1.0, 1.0),
        ))
    }

    #[test]
    fn triangle_rejects_exact_degeneracy_and_nonfinite_uv() {
        let plane = offset_plane();
        let p = Point2::new(0.0, 0.0);
        assert!(
            tri_integral(
                &plane,
                [p, p, Point2::new(1.0, 0.0)],
                Point3::new(0.0, 0.0, 0.0)
            )
            .unwrap_err()
            .contains("degenerate")
        );
        assert!(
            tri_integral(
                &plane,
                [p, Point2::new(f64::NAN, 1.0), Point2::new(0.0, 1.0)],
                Point3::new(0.0, 0.0, 0.0)
            )
            .unwrap_err()
            .contains("non-finite")
        );
    }

    #[test]
    fn triangle_rejects_nonfinite_surface_integrand() {
        let inf = f64::INFINITY;
        let plane = Surface::Plane(monstertruck_modeling::Plane::new(
            Point3::new(0.0, 0.0, inf),
            Point3::new(1.0, 0.0, inf),
            Point3::new(0.0, 1.0, inf),
        ));
        let tri = [
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        ];
        assert!(
            tri_integral(&plane, tri, Point3::new(0.0, 0.0, 0.0))
                .unwrap_err()
                .contains("non-finite")
        );
    }

    #[test]
    fn valid_skinny_positive_uv_triangle_is_not_discarded() {
        let plane = offset_plane();
        let d = 1e-10;
        let result = tri_integral(
            &plane,
            [
                Point2::new(0.0, 0.0),
                Point2::new(d, 0.0),
                Point2::new(0.0, d),
            ],
            Point3::new(0.0, 0.0, 0.0),
        )
        .unwrap();
        let expected = d * d / 6.0;
        assert!(
            result > 0.0,
            "valid positive-area skinny triangle was lost: {result:e}"
        );
        assert!(
            (result - expected).abs() <= expected * 1e-12,
            "{result:e} != {expected:e}"
        );
    }

    #[test]
    fn rejects_open_shell_and_open_wire() {
        let mut open_shell = cylinder(2.0, 5.0, [0.0; 3]).boundaries()[0].clone();
        open_shell.remove(0);
        let open_solid = Solid::new_unchecked(vec![open_shell]);
        assert!(
            validate_solid(&open_solid)
                .unwrap_err()
                .contains("closed/oriented")
        );

        let a = builder::vertex(Point3::new(0.0, 0.0, 0.0));
        let b = builder::vertex(Point3::new(1.0, 0.0, 0.0));
        let c = builder::vertex(Point3::new(1.0, 1.0, 0.0));
        let open_wire: monstertruck_modeling::Wire =
            vec![builder::line(&a, &b), builder::line(&b, &c)].into();
        let open_face = Face::new_unchecked(
            vec![open_wire],
            Surface::Plane(monstertruck_modeling::Plane::xy()),
        );
        let open_wire_shell: Shell = std::iter::once(open_face).collect();
        let open_wire_solid = Solid::new_unchecked(vec![open_wire_shell]);
        assert!(
            validate_solid(&open_wire_solid)
                .unwrap_err()
                .contains("open boundary loop")
        );
    }

    #[test]
    fn rejects_nonfinite_integral_tolerance() {
        let solid = square_tube([0.0; 3]);
        assert!(volume(&solid, f64::NAN).unwrap_err().contains("tolerance"));
    }
}

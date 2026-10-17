use monstertruck_io::step::load::{
    step_geometry::{ElementarySurface, Surface},
    *,
};
use monstertruck_meshing::{analyzers::CalcVolume, tessellation::RobustMeshableShape};
use monstertruck_modeling::InnerSpace;
use std::f64::consts::PI;

fn main() {
    let mut args = std::env::args().skip(1);
    let input = args
        .next()
        .expect("usage: sphere_deflection_probe <step> <tolerance>");
    let tolerance: f64 = args
        .next()
        .expect("tolerance")
        .parse()
        .expect("valid tolerance");
    assert!(tolerance.is_finite() && tolerance > 0.0);
    let bytes = std::fs::read(&input).expect("read STEP fixture");
    let table = Table::from_step_bytes(&bytes).expect("parse STEP");
    let id = *table.manifold_solid_brep.keys().next().expect("solid");
    let (solid, reports) = table
        .to_compressed_solid_reported(&table.manifold_solid_brep[&id])
        .expect("convert");
    for report in &reports {
        report.require_lossless().expect("lossless conversion");
    }
    assert_eq!(solid.boundaries.len(), 1);
    let shell = solid.boundaries.into_iter().next().unwrap();
    assert_eq!(shell.faces.len(), 1);
    let (center, radius) = match &shell.faces[0].surface {
        Surface::ElementarySurface(ElementarySurface::Sphere(sphere)) => {
            (sphere.entity().0.center(), sphere.entity().0.radius())
        }
        other => panic!("expected analytic sphere surface, got {other:?}"),
    };
    let meshed = shell.robust_triangulation(tolerance);
    assert_eq!(meshed.faces.len(), 1);
    let face = &meshed.faces[0];
    let mesh = face.surface.as_ref().expect("sphere mesh");
    let triangles = mesh.faces().triangle_iter().collect::<Vec<_>>();
    let mut max_plane_deficit = 0.0f64;
    let mut min_plane_offset = f64::INFINITY;
    let mut max_vertex_radius_error = 0.0f64;
    let mut inward = 0usize;
    let mut degenerate = 0usize;
    let mut max_sampled_facet_normal_angle_deg = 0.0f64;
    for position in mesh.positions() {
        let radial = *position - center;
        max_vertex_radius_error = max_vertex_radius_error.max((radial.magnitude() - radius).abs());
    }
    for triangle in &triangles {
        let points = triangle.map(|vertex| mesh.positions()[vertex.pos]);
        let mut normal = (points[1] - points[0]).cross(points[2] - points[0]);
        let normal_len = normal.magnitude();
        if normal_len <= 1.0e-18 {
            degenerate += 1;
            continue;
        }
        if !face.orientation {
            normal = -normal;
        }
        normal = normal / normal_len;
        let offset = normal.dot(points[0] - center);
        for point in points {
            let radial = point - center;
            let cosine = (normal.dot(radial) / radial.magnitude()).clamp(-1.0, 1.0);
            max_sampled_facet_normal_angle_deg =
                max_sampled_facet_normal_angle_deg.max(cosine.acos().to_degrees());
        }
        min_plane_offset = min_plane_offset.min(offset);
        if offset <= 0.0 {
            inward += 1;
        }
        max_plane_deficit = max_plane_deficit.max(radius - offset);
    }
    let signed_volume = if face.orientation {
        mesh.volume()
    } else {
        -mesh.volume()
    };
    let expected_volume = 4.0 * PI * radius.powi(3) / 3.0;
    let within = max_plane_deficit <= tolerance * (1.0 + 1.0e-10);
    println!(
        "file={input} radius={radius:.12} center=({:.12},{:.12},{:.12}) tolerance={tolerance:.12} triangles={} max_plane_deficit={max_plane_deficit:.12} tolerance_ratio={:.9} max_vertex_radius_error={max_vertex_radius_error:.12e} min_plane_offset={min_plane_offset:.12} inward_triangles={inward} degenerate_triangles={degenerate} max_sampled_facet_normal_angle_deg={max_sampled_facet_normal_angle_deg:.9} signed_volume={signed_volume:.12} expected_volume={expected_volume:.12} volume_relative_error={:.9} deflection_pass={within}",
        center.x,
        center.y,
        center.z,
        triangles.len(),
        max_plane_deficit / tolerance,
        (signed_volume - expected_volume).abs() / expected_volume
    );
    assert!(
        within,
        "sphere facet plane deficit exceeds requested tolerance"
    );
    assert_eq!(inward, 0, "inward triangles");
    assert_eq!(degenerate, 0, "degenerate triangles");
}

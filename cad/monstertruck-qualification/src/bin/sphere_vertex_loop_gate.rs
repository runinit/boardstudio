use monstertruck_io::step::load::{convert::*, *};
use monstertruck_meshing::{analyzers::CalcVolume, prelude::*};
use monstertruck_modeling::{Point3, Vector3};
use std::f64::consts::PI;

fn main() {
    let input = std::env::args().nth(1).expect("fixture path");
    let bytes = std::fs::read(&input).expect("read STEP fixture");
    let table = Table::from_step_bytes(&bytes).expect("parse sphere STEP");
    let id = *table.manifold_solid_brep.keys().next().expect("sphere solid");
    let (solid, reports) = table
        .to_compressed_solid_reported(&table.manifold_solid_brep[&id])
        .expect("convert sphere solid");
    for report in &reports {
        report.require_lossless().expect("lossless sphere face conversion");
    }
    assert_eq!(solid.boundaries.len(), 1, "one shell");
    let shell = solid.boundaries.into_iter().next().unwrap();
    assert_eq!(shell.faces.len(), 1, "one complete sphere face");
    assert!(matches!(
        shell.faces[0].surface,
        monstertruck_io::step::load::step_geometry::Surface::ElementarySurface(
            monstertruck_io::step::load::step_geometry::ElementarySurface::Sphere(_)
        )
    ), "preserve the analytic spherical face");
    assert_eq!(shell.faces[0].boundaries.len(), 1, "retain degenerate pole bound");
    assert!(shell.faces[0].boundaries[0].is_empty(), "pole bound has no edge uses");

    let meshed = shell.robust_triangulation(0.0005);
    assert_eq!(meshed.faces.len(), 1, "full sphere tessellated as one face");
    let face = &meshed.faces[0];
    let mesh = face.surface.as_ref().expect("sphere face mesh");
    let triangles = mesh.faces().triangle_iter().collect::<Vec<_>>();
    assert!(triangles.len() > 40_000, "full sphere coverage, got {} triangles", triangles.len());
    let mut bad_normals = 0usize;
    let mut inward_triangles = 0usize;
    let mut nondegenerate_triangles = 0usize;
    let mut min_radius = f64::INFINITY;
    let mut max_radius = 0.0f64;
    for position in mesh.positions() {
        let radius = (position.x * position.x + position.y * position.y + position.z * position.z).sqrt();
        min_radius = min_radius.min(radius);
        max_radius = max_radius.max(radius);
    }
    for triangle in &triangles {
        let points = triangle.map(|v| mesh.positions()[v.pos]);
        let edge_a = points[1] - points[0];
        let edge_b = points[2] - points[0];
        let mut normal = edge_a.cross(edge_b);
        if !face.orientation { normal = -normal; }
        let centroid = Point3::from_vec((points[0].to_vec() + points[1].to_vec() + points[2].to_vec()) / 3.0);
        if normal.magnitude2() > 1.0e-20 {
            nondegenerate_triangles += 1;
            if normal.dot(centroid.to_vec()) <= 0.0 { inward_triangles += 1; }
        }
        for vertex in triangle {
            let Some(index) = vertex.nor else { bad_normals += 1; continue; };
            let mut n = mesh.normals()[index];
            if !face.orientation { n = -n; }
            let p = mesh.positions()[vertex.pos];
            if !n.x.is_finite() || !n.y.is_finite() || !n.z.is_finite() || n.magnitude2() < 0.9 || n.dot(p.to_vec()) <= 0.0 {
                bad_normals += 1;
            }
        }
    }
    let signed_volume = if face.orientation { mesh.volume() } else { -mesh.volume() };
    let expected_volume = 4.0 * PI * 125.0 / 3.0;
    assert!((min_radius - 5.0).abs() < 0.02 && (max_radius - 5.0).abs() < 1.0e-8, "sphere radii [{min_radius},{max_radius}]");
    assert!(nondegenerate_triangles > 40_000, "sphere area coverage");
    assert_eq!(inward_triangles, 0, "outward triangle winding");
    assert_eq!(bad_normals, 0, "finite outward vertex normals");
    assert!((signed_volume - expected_volume).abs() / expected_volume < 0.01, "sphere signed volume {signed_volume}, expected {expected_volume}");
    println!("sphere_vertex_loop=PASS; face_count=1; triangles={}; nondegenerate_triangles={nondegenerate_triangles}; radius_min={min_radius:.9}; radius_max={max_radius:.9}; signed_volume={signed_volume:.6}; expected_volume={expected_volume:.6}; inward_triangles={inward_triangles}; bad_normals={bad_normals}", triangles.len());
}

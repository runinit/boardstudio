//! Qualification-only profile and extrusion gate using sampled production inputs.
//!
//! This executable starts from already accepted planar contours. It does not
//! replace BoardStudio's user-input validation or qualify Boolean operations.

use monstertruck_io::step::{
    load::Table,
    save::{CompleteStepDisplay, StepModel},
};
use monstertruck_meshing::{prelude::*, tessellation::shell_to_polygon_strict};
use monstertruck_modeling::{BoundedCurve, Point3, Solid, Vector3, Wire, builder};
use serde_json::Value;
use std::{fs, path::Path};
use ttf_parser::{Face, OutlineBuilder};

const EXTRUSION_MM: f64 = 2.0;
const MESH_TOLERANCE_MM: f64 = 0.05;

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let concave = read_json(root.join("fixtures/profile-topology/rotated-concave.json"));
    let multi = read_json(root.join("fixtures/profile-topology/multi-hole-region.json"));
    let split = read_json(root.join("fixtures/profile-topology/split-regions.json"));

    let concave_outer = points(&concave["outer"]);
    let concave_volume = ring_area(&concave_outer) * EXTRUSION_MM;
    check_profile(
        "production_rotated_concave",
        std::slice::from_ref(&concave_outer),
        concave_volume,
    );
    let mut reversed = concave_outer.clone();
    reversed.reverse();
    check_profile(
        "production_rotated_concave_reversed",
        &[reversed],
        concave_volume,
    );

    let outer = points(&multi["outer"]);
    let holes: Vec<_> = multi["holes"]
        .as_array()
        .unwrap()
        .iter()
        .map(points)
        .collect();
    assert_eq!(holes.len(), 35, "production plate sample has 35 holes");
    let multi_volume =
        (ring_area(&outer) - holes.iter().map(|r| ring_area(r)).sum::<f64>()) * EXTRUSION_MM;
    let mut multi_rings = vec![outer.clone()];
    multi_rings.extend(holes.iter().cloned());
    check_profile("production_plate_35_holes", &multi_rings, multi_volume);
    let mut reversed_multi = multi_rings.clone();
    for ring in &mut reversed_multi {
        ring.reverse();
    }
    check_profile(
        "production_plate_35_holes_reversed",
        &reversed_multi,
        multi_volume,
    );

    let islands: Vec<_> = split["islands"]
        .as_array()
        .unwrap()
        .iter()
        .map(points)
        .collect();
    assert_eq!(islands.len(), 2, "production split profile has two regions");
    let mut island_volumes = Vec::new();
    for (index, island) in islands.iter().enumerate() {
        let expected = ring_area(island) * EXTRUSION_MM;
        check_profile(
            &format!("production_disconnected_region_{index}"),
            std::slice::from_ref(island),
            expected,
        );
        island_volumes.push(expected);
    }
    assert_eq!(
        island_volumes.len(),
        2,
        "keep disconnected regions as solids"
    );

    check_letter_b(root.join("../wasm/assets/DejaVuSans.ttf"));
    println!(
        "profile_topology_gate=PASS; source=production-sampled-contours; concave=1; multi_hole=35; reversed_winding=pass; disconnected_regions=2; lettering=B; extrusion_mm={EXTRUSION_MM}; mesh_tolerance_mm={MESH_TOLERANCE_MM}"
    );
}

fn read_json(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).expect("read profile fixture"))
        .expect("parse profile fixture")
}

fn points(value: &Value) -> Vec<(f64, f64)> {
    value
        .as_array()
        .expect("ring array")
        .iter()
        .map(|point| {
            let pair = point.as_array().expect("2D point");
            assert_eq!(pair.len(), 2, "2D coordinate pair");
            (pair[0].as_f64().unwrap(), pair[1].as_f64().unwrap())
        })
        .collect()
}

fn ring_area(points: &[(f64, f64)]) -> f64 {
    assert!(
        points.len() >= 3,
        "profile ring must have at least three points"
    );
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(&(x0, y0), &(x1, y1))| x0 * y1 - x1 * y0)
        .sum::<f64>()
        .abs()
        * 0.5
}

fn wire(points: &[(f64, f64)], z: f64) -> Wire {
    assert!(
        points.len() >= 3,
        "profile ring must have at least three points"
    );
    let vertices: Vec<_> = points
        .iter()
        .map(|&(x, y)| builder::vertex(Point3::new(x, y, z)))
        .collect();
    (0..vertices.len())
        .map(|i| builder::line(&vertices[i], &vertices[(i + 1) % vertices.len()]))
        .collect::<Vec<_>>()
        .into()
}

fn check_profile(label: &str, rings: &[Vec<(f64, f64)>], expected_volume: f64) {
    let solid = monstertruck_modeling::profile::solid_from_planar_profile(
        rings.iter().map(|ring| wire(ring, 0.0)).collect(),
        Vector3::unit_z() * EXTRUSION_MM,
    )
    .unwrap_or_else(|error| panic!("{label}: profile extrusion failed: {error:?}"));
    assert!(
        solid.is_geometric_consistent(),
        "{label}: source B-rep inconsistent"
    );
    check_solid(label, &solid, expected_volume, rings);
    step_roundtrip(label, &solid, expected_volume, rings);
}

fn check_solid<C, S>(
    label: &str,
    solid: &monstertruck_topology::Solid<Point3, C, S>,
    expected_volume: f64,
    rings: &[Vec<(f64, f64)>],
) where
    C: PolylineableCurve + BoundedCurve<Point = Point3> + Clone,
    S: MeshableSurface,
{
    assert_eq!(
        solid.boundaries().len(),
        1,
        "{label}: one connected profile shell"
    );
    let expected_faces = rings.iter().map(Vec::len).sum::<usize>() + 2;
    let actual_faces = solid.boundaries()[0].face_iter().count();
    assert_eq!(
        actual_faces, expected_faces,
        "{label}: preserve walls, top and bottom"
    );

    let mesh = shell_to_polygon_strict(&solid.boundaries()[0], MESH_TOLERANCE_MM)
        .unwrap_or_else(|error| panic!("{label}: strict tessellation failed: {error:?}"));
    assert_mesh_closed(&mesh, label);
    let mesh_volume = mesh.volume().abs();
    let volume_tolerance = expected_volume.abs().max(1.0) * 1.0e-8;
    assert!(
        (mesh_volume - expected_volume).abs() <= volume_tolerance,
        "{label}: strict mesh volume {mesh_volume}, analytic profile volume {expected_volume}, tolerance {volume_tolerance}"
    );

    let (expected_min, expected_max) = bounds(rings, EXTRUSION_MM);
    let mut actual_min = [f64::INFINITY; 3];
    let mut actual_max = [f64::NEG_INFINITY; 3];
    for point in mesh.positions() {
        let coords = [point.x, point.y, point.z];
        for axis in 0..3 {
            actual_min[axis] = actual_min[axis].min(coords[axis]);
            actual_max[axis] = actual_max[axis].max(coords[axis]);
        }
    }
    for axis in 0..3 {
        assert!(
            (actual_min[axis] - expected_min[axis]).abs() < 1.0e-7,
            "{label}: min bound axis {axis}: {} vs {}",
            actual_min[axis],
            expected_min[axis]
        );
        assert!(
            (actual_max[axis] - expected_max[axis]).abs() < 1.0e-7,
            "{label}: max bound axis {axis}: {} vs {}",
            actual_max[axis],
            expected_max[axis]
        );
    }
    println!(
        "{label}=PASS; faces={actual_faces}; strict_mesh_triangles={}; volume_mm3={mesh_volume:.9}; bounds={actual_min:?}..{actual_max:?}",
        mesh.faces().triangle_iter().count()
    );
}

fn bounds(rings: &[Vec<(f64, f64)>], height: f64) -> ([f64; 3], [f64; 3]) {
    let mut min = [f64::INFINITY, f64::INFINITY, 0.0];
    let mut max = [f64::NEG_INFINITY, f64::NEG_INFINITY, height];
    for &(x, y) in rings.iter().flatten() {
        min[0] = min[0].min(x);
        min[1] = min[1].min(y);
        max[0] = max[0].max(x);
        max[1] = max[1].max(y);
    }
    (min, max)
}

fn assert_mesh_closed(mesh: &PolygonMesh, label: &str) {
    use std::collections::HashMap;
    let key = |vertex: monstertruck_meshing::rexport_polymesh::StandardVertex| {
        let point = mesh.positions()[vertex.pos];
        [point.x, point.y, point.z].map(|coordinate| (coordinate / 1.0e-7).round() as i64)
    };
    let mut edge_uses = HashMap::<([i64; 3], [i64; 3]), usize>::new();
    for triangle in mesh.faces().triangle_iter() {
        for i in 0..3 {
            let a = key(triangle[i]);
            let b = key(triangle[(i + 1) % 3]);
            let edge = if a <= b { (a, b) } else { (b, a) };
            *edge_uses.entry(edge).or_default() += 1;
        }
    }
    let nonmanifold = edge_uses.values().filter(|&&uses| uses != 2).count();
    assert_eq!(
        nonmanifold, 0,
        "{label}: strict mesh has {nonmanifold} nonmanifold edges"
    );
}

fn step_roundtrip(label: &str, solid: &Solid, expected_volume: f64, rings: &[Vec<(f64, f64)>]) {
    let compressed = solid.compress();
    let exchange =
        CompleteStepDisplay::new(StepModel::from(&compressed), Default::default()).to_string();
    let table = Table::from_step(&exchange)
        .unwrap_or_else(|error| panic!("{label}: STEP parse failed: {error}"));
    assert_eq!(
        table.manifold_solid_brep.len(),
        1,
        "{label}: one STEP solid"
    );
    let holder = table.manifold_solid_brep.values().next().unwrap();
    let (loaded, reports) = table
        .to_compressed_solid_reported(holder)
        .unwrap_or_else(|error| panic!("{label}: STEP conversion failed: {error}"));
    for report in &reports {
        report
            .require_lossless()
            .unwrap_or_else(|error| panic!("{label}: STEP roundtrip loss: {error}"));
    }
    let loaded: monstertruck_topology::Solid<
        Point3,
        monstertruck_io::step::load::step_geometry::Curve3D,
        monstertruck_io::step::load::step_geometry::Surface,
    > = monstertruck_topology::Solid::extract(loaded)
        .unwrap_or_else(|error| panic!("{label}: extracted STEP solid invalid: {error}"));
    check_solid(
        &format!("{label}_step_reloaded"),
        &loaded,
        expected_volume,
        rings,
    );
}

#[derive(Default)]
struct FlattenedOutline {
    paths: Vec<Vec<(f64, f64)>>,
    current: Vec<(f64, f64)>,
}

impl OutlineBuilder for FlattenedOutline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.close();
        self.current.push((f64::from(x), f64::from(y)));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.current.push((f64::from(x), f64::from(y)));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (a, b) = *self.current.last().unwrap();
        for i in 1..=12 {
            let t = f64::from(i) / 12.0;
            let u = 1.0 - t;
            self.current.push((
                u * u * a + 2.0 * u * t * f64::from(x1) + t * t * f64::from(x),
                u * u * b + 2.0 * u * t * f64::from(y1) + t * t * f64::from(y),
            ));
        }
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (a, b) = *self.current.last().unwrap();
        for i in 1..=16 {
            let t = f64::from(i) / 16.0;
            let u = 1.0 - t;
            self.current.push((
                u * u * u * a
                    + 3.0 * u * u * t * f64::from(x1)
                    + 3.0 * u * t * t * f64::from(x2)
                    + t * t * t * f64::from(x),
                u * u * u * b
                    + 3.0 * u * u * t * f64::from(y1)
                    + 3.0 * u * t * t * f64::from(y2)
                    + t * t * t * f64::from(y),
            ));
        }
    }
    fn close(&mut self) {
        if self.current.len() > 2 {
            let mut path = std::mem::take(&mut self.current);
            if path.first() == path.last() {
                path.pop();
            }
            self.paths.push(path);
        } else {
            self.current.clear();
        }
    }
}

fn inside(point: (f64, f64), polygon: &[(f64, f64)]) -> bool {
    let mut yes = false;
    for (a, b) in polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .take(polygon.len())
    {
        if (a.1 > point.1) != (b.1 > point.1)
            && point.0 < (b.0 - a.0) * (point.1 - a.1) / (b.1 - a.1) + a.0
        {
            yes = !yes;
        }
    }
    yes
}

fn check_letter_b(font_path: impl AsRef<Path>) {
    let font_data = fs::read(font_path).expect("read production bundled font");
    let face = Face::parse(&font_data, 0).expect("parse production bundled font");
    let glyph = face.glyph_index('B').expect("font includes B");
    let mut outline = FlattenedOutline::default();
    face.outline_glyph(glyph, &mut outline)
        .expect("B has outline");
    outline.close();
    let paths = outline.paths;
    assert_eq!(
        paths.len(),
        3,
        "sampled production B has outer plus two counters"
    );
    let nesting: Vec<_> = paths
        .iter()
        .enumerate()
        .map(|(i, path)| {
            paths
                .iter()
                .enumerate()
                .filter(|(j, other)| i != *j && inside(path[0], other))
                .count()
        })
        .collect();
    assert_eq!(
        nesting.iter().filter(|&&depth| depth == 0).count(),
        1,
        "B has one outer contour"
    );
    assert_eq!(
        nesting.iter().filter(|&&depth| depth == 1).count(),
        2,
        "B has two counter contours"
    );

    let bounds = paths.iter().flatten().fold(
        (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ),
        |(xmin, xmax, ymin, ymax), &(x, y)| (xmin.min(x), xmax.max(x), ymin.min(y), ymax.max(y)),
    );
    let scale = (18.0 / (bounds.1 - bounds.0))
        .min(8.0 / (bounds.3 - bounds.2))
        .min(4.0 / (bounds.3 - bounds.2));
    let xmid = (bounds.0 + bounds.1) * 0.5;
    let ymid = (bounds.2 + bounds.3) * 0.5;
    let transformed: Vec<Vec<_>> = paths
        .iter()
        .map(|path| {
            path.iter()
                .map(|&(x, y)| ((x - xmid) * scale, (y - ymid) * scale))
                .collect()
        })
        .collect();
    let mut total_profiles = 0;
    for (i, path) in transformed
        .iter()
        .enumerate()
        .filter(|(i, _)| nesting[*i] % 2 == 0)
    {
        let mut rings = vec![path.clone()];
        rings.extend(
            transformed
                .iter()
                .enumerate()
                .filter(|(j, hole)| nesting[*j] == nesting[i] + 1 && inside(hole[0], path))
                .map(|(_, ring)| ring.clone()),
        );
        let area = ring_area(&rings[0])
            - rings
                .iter()
                .skip(1)
                .map(|ring| ring_area(ring))
                .sum::<f64>();
        check_profile(
            &format!("production_letter_B_region_{total_profiles}"),
            &rings,
            area * EXTRUSION_MM,
        );
        total_profiles += 1;
    }
    assert_eq!(total_profiles, 1, "B is one independent solid profile");
}

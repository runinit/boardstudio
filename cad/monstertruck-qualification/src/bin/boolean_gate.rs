use monstertruck_meshing::{prelude::*, tessellation::shell_to_polygon_strict};
use monstertruck_modeling::*;
use std::collections::BTreeMap;
use std::io::Write;
use std::time::Instant;

const BOOLEAN_TOLERANCE: f64 = 1.0e-6;
const MESH_TOLERANCE: f64 = 0.1;
const BODY_WIDTH: f64 = 100.0;
const BODY_HEIGHT: f64 = 70.0;
const BODY_THICKNESS: f64 = 2.0;
const GASKET_WIDTH: f64 = 12.0;
const GASKET_HEIGHT: f64 = 4.0;
const MOUNT_DIAMETER: f64 = 2.2;
const REGRESSION_Z: f64 = -6.6;
const REGRESSION_THICKNESS: f64 = 13.3;
const REGRESSION_MOUNT_X: f64 = 42.93354797363281;
const REGRESSION_MOUNT_Y: f64 = -11.1;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let run_start = Instant::now();
    println!("monstertruck_version=0.4.1; published_crate_source");
    println!("upstream_source_reference=d2ad1ed7aa420b7f32cd2c1e3ad319b46d593652");
    println!(
        "profile_fixture=internal-gasket-v1/rectangle.json (100x70mm board); boolean_fixture=live-gasket-bottom-regression.json (region 0, mount 0)"
    );
    println!(
        "profile_depth={BODY_THICKNESS}mm; gasket_hole={GASKET_WIDTH}x{GASKET_HEIGHT}mm; regression_z={REGRESSION_Z}; regression_thickness={REGRESSION_THICKNESS}; mount_diameter={MOUNT_DIAMETER}mm"
    );
    println!("boolean_tolerance={BOOLEAN_TOLERANCE}; mesh_tolerance={MESH_TOLERANCE}");

    // Harness control: a centered cube difference must remove one eighth of the volume.
    let control_start = Instant::now();
    let cube = unit_cube(Point3::origin());
    let overlap = unit_cube(Point3::new(0.5, 0.5, 0.5));
    progress("box_control difference_start", run_start);
    let control_result = monstertruck_solid::difference(&cube, &overlap, 0.05);
    progress("box_control difference_returned", run_start);
    let control = control_result?;
    let control_mesh = strict_mesh(&control);
    let control_volume = control_mesh.volume();
    assert!(
        (control_volume - 0.875).abs() < 1.0e-3,
        "box control volume {control_volume}, expected 0.875"
    );
    assert_mesh_closed(&control_mesh, "box control");
    println!(
        "control=pass; volume={control_volume:.9}; elapsed_ms={}",
        control_start.elapsed().as_millis()
    );

    // This uses BoardStudio's 100x70 mm gasket fixture and the M2.2 mounting-hole
    // diameter. The inner rectangular loop is its 12x4 mm gasket profile.
    let plate = profile_with_hole();
    let expected_before_mount =
        BODY_WIDTH * BODY_HEIGHT * BODY_THICKNESS - GASKET_WIDTH * GASKET_HEIGHT * BODY_THICKNESS;
    let before_mount_mesh = strict_mesh(&plate);
    assert_mesh_closed(&before_mount_mesh, "profile hole extrusion");
    let before_mount_volume = before_mount_mesh.volume();
    assert!(
        (before_mount_volume - expected_before_mount).abs() < 0.05,
        "profile extrusion volume {before_mount_volume}, expected {expected_before_mount}"
    );
    println!(
        "profile_hole_extrusion=pass; mesh_volume={before_mount_volume:.6}; expected={expected_before_mount:.6}"
    );

    if std::env::args().any(|arg| arg == "--simple-round-only") {
        simple_round_cut();
        return Ok(());
    }

    // Reproduce the recorded first cut: exact live-gasket regression region 0,
    // mount 0, at its authored z/thickness and with the original two-arc cutter.
    let exact_base = regression_region_zero();
    let hole_origin = Point3::new(REGRESSION_MOUNT_X, REGRESSION_MOUNT_Y, REGRESSION_Z);
    let hole_radius = MOUNT_DIAMETER / 2.0;
    let hole = round_cutter(hole_origin, hole_radius, REGRESSION_THICKNESS);
    let subtract_start = Instant::now();
    progress("regression_mount difference_start", run_start);
    let result = monstertruck_solid::difference(&exact_base, &hole, BOOLEAN_TOLERANCE);
    progress("regression_mount difference_returned", run_start);
    let result = result?;
    let elapsed = subtract_start.elapsed();
    let mesh = strict_mesh(&result);
    assert_mesh_closed(&mesh, "mounting-hole cut");
    let volume = mesh.volume();
    let body_area = polygon_area(&regression_region_zero_points());
    let expected = body_area * REGRESSION_THICKNESS
        - std::f64::consts::PI * hole_radius.powi(2) * REGRESSION_THICKNESS;
    let chord_volume_bound = std::f64::consts::PI
        * (2.0 * hole_radius * MESH_TOLERANCE - MESH_TOLERANCE.powi(2))
        * REGRESSION_THICKNESS;
    assert!(
        (volume - expected).abs() <= chord_volume_bound,
        "mounting-hole mesh volume {volume}, analytic volume {expected}, conservative 0.1mm chord bound {chord_volume_bound}"
    );
    println!(
        "mounting_hole=pass; tessellated_volume={volume:.6}; analytic_expected={expected:.6}; conservative_mesh_error_bound={chord_volume_bound:.6}; elapsed_ms={}",
        elapsed.as_millis()
    );
    Ok(())
}

fn unit_cube(origin: Point3) -> Solid {
    let vertex = builder::vertex(origin);
    let edge = builder::extrude(&vertex, Vector3::unit_x());
    let face = builder::extrude(&edge, Vector3::unit_y());
    builder::extrude(&face, Vector3::unit_z())
}

fn strict_mesh(solid: &Solid) -> PolygonMesh {
    assert_eq!(
        solid.boundaries().len(),
        1,
        "expected one closed boundary shell"
    );
    shell_to_polygon_strict(&solid.boundaries()[0], MESH_TOLERANCE)
        .expect("strict tessellation must not drop a boundary face")
}

fn simple_round_cut() {
    let size = 10.0;
    let thickness = 2.0;
    let plate = profile::solid_from_planar_profile(
        vec![primitive::rect(
            BoundingBox::from_iter([Point2::new(0.0, 0.0), Point2::new(size, size)]),
            Plane::xy(),
        )],
        Vector3::unit_z() * thickness,
    )
    .expect("simple control plate should extrude");
    let origin = Point3::new(size / 2.0, size / 2.0, 0.0);
    let radius = MOUNT_DIAMETER / 2.0;
    let cutter = round_cutter(origin, radius, thickness);
    let start = Instant::now();
    progress("simple_round_hole difference_start", start);
    let cut_result = monstertruck_solid::difference(&plate, &cutter, BOOLEAN_TOLERANCE);
    progress("simple_round_hole difference_returned", start);
    let cut = cut_result.expect("simple round hole boolean should succeed");
    let mesh = strict_mesh(&cut);
    assert_mesh_closed(&mesh, "simple round hole");
    let volume = mesh.volume();
    let expected = size * size * thickness - std::f64::consts::PI * radius.powi(2) * thickness;
    let chord_volume_bound =
        std::f64::consts::PI * (2.0 * radius * MESH_TOLERANCE - MESH_TOLERANCE.powi(2)) * thickness;
    assert!(
        (volume - expected).abs() <= chord_volume_bound,
        "simple round hole mesh volume {volume}, analytic volume {expected}, conservative mesh error bound {chord_volume_bound}"
    );
    println!(
        "simple_round_hole=pass; tessellated_volume={volume:.6}; analytic_expected={expected:.6}; conservative_mesh_error_bound={chord_volume_bound:.6}; elapsed_ms={}",
        start.elapsed().as_millis()
    );
}

fn progress(stage: &str, started: Instant) {
    println!(
        "stage={stage}; elapsed_ms={}",
        started.elapsed().as_millis()
    );
    std::io::stdout().flush().expect("flush stage marker");
}

fn assert_mesh_closed(mesh: &PolygonMesh, label: &str) {
    type PointKey = [i64; 3];
    let point_key = |vertex: monstertruck_meshing::rexport_polymesh::StandardVertex| {
        let point = mesh.positions()[vertex.pos];
        [point.x, point.y, point.z].map(|coordinate| (coordinate / 1.0e-7).round() as i64)
    };
    let mut edges = BTreeMap::<[PointKey; 2], (usize, i32)>::new();
    let mut triangles = 0;
    for face in mesh.face_iter() {
        assert_eq!(face.len(), 3, "{label}: non-triangle face in strict mesh");
    }
    for triangle in mesh.faces().triangle_iter() {
        triangles += 1;
        for index in 0..3 {
            let start = point_key(triangle[index]);
            let end = point_key(triangle[(index + 1) % 3]);
            assert_ne!(start, end, "{label}: zero-length welded mesh edge");
            let (key, orientation) = if start < end {
                ([start, end], 1)
            } else {
                ([end, start], -1)
            };
            let entry = edges.entry(key).or_default();
            entry.0 += 1;
            entry.1 += orientation;
        }
    }
    assert!(triangles > 0, "{label}: no mesh triangles");
    let open = edges
        .values()
        .filter(|&&(uses, winding)| uses != 2 || winding != 0)
        .count();
    assert_eq!(
        open, 0,
        "{label}: {open} unpaired or inconsistently directed welded edges"
    );
}

fn profile_with_hole() -> Solid {
    let rectangle = |width: f64, height: f64, x: f64, y: f64| {
        primitive::rect(
            BoundingBox::from_iter([Point2::new(x, y), Point2::new(x + width, y + height)]),
            Plane::xy(),
        )
    };
    let outer = rectangle(BODY_WIDTH, BODY_HEIGHT, 0.0, 0.0);
    let inner = rectangle(GASKET_WIDTH, GASKET_HEIGHT, 44.0, 33.0);
    profile::solid_from_planar_profile(vec![outer, inner], Vector3::unit_z() * BODY_THICKNESS)
        .expect("fixture rectangle profile should extrude")
}

fn regression_region_zero() -> Solid {
    let points = regression_region_zero_points();
    let vertices = builder::vertices(points.iter().map(|&(x, y)| Point3::new(x, y, REGRESSION_Z)));
    let wire: Wire = (0..vertices.len())
        .map(|i| builder::line(&vertices[i], &vertices[(i + 1) % vertices.len()]))
        .collect();
    let face: Face =
        builder::try_attach_plane(vec![wire]).expect("regression polygon should be planar");
    builder::extrude(&face, Vector3::unit_z() * REGRESSION_THICKNESS)
}

fn regression_region_zero_points() -> Vec<(f64, f64)> {
    vec![
        (-15.599999880790705, 48.00000007692723),
        (-15.599999880790705, 34.000000076927236),
        (-4.599999880790705, 34.000000076927236),
        (-4.599999880790705, -4.599999827705332),
        (35.933547997474676, -4.599999827705332),
        (35.933547997474676, -15.599999827705332),
        (49.93354799747467, -15.599999827705332),
        (49.93354799747467, -4.599999827705332),
        (120.00000002384186, -4.599999827705332),
        (120.00000002384186, -15.599999827705332),
        (134.00000002384186, -15.599999827705332),
        (134.00000002384186, -4.599999827705332),
        (149.59999992847443, -4.599999827705332),
        (149.59999992847443, 109.5999999815598),
        (146.00000002384186, 109.5999999815598),
        (146.00000002384186, 120.5999999815598),
        (132.00000002384186, 120.5999999815598),
        (132.00000002384186, 109.5999999815598),
        (111.00000002384186, 109.5999999815598),
        (111.00000002384186, 120.5999999815598),
        (97.00000002384186, 120.5999999815598),
        (97.00000002384186, 109.5999999815598),
        (25.000000023841864, 109.5999999815598),
        (25.000000023841864, 120.5999999815598),
        (11.000000023841864, 120.5999999815598),
        (11.000000023841864, 109.5999999815598),
        (-4.599999880790705, 109.5999999815598),
        (-4.599999880790705, 48.00000007692723),
    ]
}

fn polygon_area(points: &[(f64, f64)]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(&(x0, y0), &(x1, y1))| x0 * y1 - x1 * y0)
        .sum::<f64>()
        .abs()
        / 2.0
}

fn round_cutter(origin: Point3, radius: f64, height: f64) -> Solid {
    let right = builder::vertex(Point3::new(origin.x + radius, origin.y, origin.z));
    let left = builder::vertex(Point3::new(origin.x - radius, origin.y, origin.z));
    let wire: Wire = vec![
        builder::circle_arc(
            &right,
            &left,
            Point3::new(origin.x, origin.y + radius, origin.z),
        ),
        builder::circle_arc(
            &left,
            &right,
            Point3::new(origin.x, origin.y - radius, origin.z),
        ),
    ]
    .into();
    let face: Face =
        builder::try_attach_plane(vec![wire]).expect("round cutter profile should be planar");
    builder::extrude(&face, Vector3::unit_z() * height)
}

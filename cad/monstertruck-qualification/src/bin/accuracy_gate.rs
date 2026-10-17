use i_overlay::{
    core::{fill_rule::FillRule, overlay_rule::OverlayRule},
    float::single::SingleFloatOverlay,
};
use monstertruck_meshing::analyzers::CalcVolume;
use monstertruck_meshing::tessellation::shell_to_polygon_strict;
use monstertruck_modeling::{
    Point3, Solid, Vector3, bounding::certified_solid_bounding_box, builder,
};
use std::f64::consts::PI;

const MESH_TOLERANCE_MM: f64 = 0.001;

fn cylinder(radius: f64, height: f64) -> Solid {
    let vertex = builder::vertex(Point3::new(0.0, -height / 2.0, radius));
    let circle = builder::revolve(
        &vertex,
        Point3::new(0.0, 0.0, 0.0),
        Vector3::unit_y(),
        builder::SweepAngle::Closed,
        2,
    );
    let disk = builder::try_attach_plane(&[circle]).expect("circle has a planar cap");
    builder::extrude(&disk, Vector3::new(0.0, height, 0.0))
}

fn conical_frustum(r0: f64, r1: f64, height: f64) -> Solid {
    let p = [
        Point3::new(0.0, 0.0, height),
        Point3::new(r1, 0.0, height),
        Point3::new(r0, 0.0, 0.0),
        Point3::new(0.0, 0.0, 0.0),
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
        Point3::new(0.0, 0.0, 0.0),
        Vector3::unit_z(),
        builder::SweepAngle::Closed,
        4,
    );
    Solid::new(vec![shell])
}

fn strict_mesh_volume(solid: &Solid) -> f64 {
    solid
        .boundaries()
        .iter()
        .map(|shell| {
            shell_to_polygon_strict(shell, MESH_TOLERANCE_MM)
                .expect("fixture tessellates without dropping faces")
                .volume()
        })
        .sum()
}

fn assert_bounds(solid: &Solid, expected_min: [f64; 3], expected_max: [f64; 3], label: &str) {
    let bounds = certified_solid_bounding_box(solid).expect("curved B-rep bound is certified");
    let actual_min = bounds.min();
    let actual_max = bounds.max();
    let maximum_side_excess = (0..3)
        .map(|axis| {
            (expected_min[axis] - actual_min[axis])
                .max(actual_max[axis] - expected_max[axis])
                .max(0.0)
        })
        .fold(0.0_f64, f64::max);
    println!(
        "{label} certified_bounds_min={:?} max={:?} maximum_side_excess_mm={maximum_side_excess:.9} enclosure_only=true",
        actual_min, actual_max,
    );
    for axis in 0..3 {
        assert!(
            actual_min[axis] <= expected_min[axis] + 1e-9
                && actual_max[axis] >= expected_max[axis] - 1e-9,
            "{label} bound axis {axis} does not enclose the analytic extremum: [{}, {}] vs [{}, {}]",
            actual_min[axis],
            actual_max[axis],
            expected_min[axis],
            expected_max[axis]
        );
    }
}

fn accuracy_fixtures() {
    let (radius, height) = (2.0, 5.0);
    let cyl = cylinder(radius, height);
    assert_bounds(
        &cyl,
        [-radius, -height / 2.0, -radius],
        [radius, height / 2.0, radius],
        "cylinder",
    );
    let expected_cylinder = PI * radius * radius * height;
    let measured_cylinder = strict_mesh_volume(&cyl).abs();
    let cylinder_relative_error = (measured_cylinder - expected_cylinder).abs() / expected_cylinder;

    let (r0, r1, frustum_height) = (1.5, 0.75, 4.0);
    let frustum = conical_frustum(r0, r1, frustum_height);
    assert_bounds(
        &frustum,
        [-r0, -r0, 0.0],
        [r0, r0, frustum_height],
        "frustum",
    );
    let expected_frustum = PI * frustum_height * (r0 * r0 + r0 * r1 + r1 * r1) / 3.0;
    let signed_frustum_mesh_volume = strict_mesh_volume(&frustum);
    assert!(
        signed_frustum_mesh_volume > 0.0,
        "frustum shell orientation must yield positive signed volume, got {signed_frustum_mesh_volume}"
    );
    let frustum_relative_error =
        (signed_frustum_mesh_volume - expected_frustum).abs() / expected_frustum;

    println!("cylinder exact formula volume_mm3={expected_cylinder:.12}");
    println!(
        "cylinder strict_mesh_volume_mm3={measured_cylinder:.12} relative_error={cylinder_relative_error:.3e}"
    );
    println!("frustum exact formula volume_mm3={expected_frustum:.12}");
    println!(
        "frustum strict_mesh_signed_volume_mm3={signed_frustum_mesh_volume:.12} relative_error={frustum_relative_error:.3e}"
    );
    println!("curved cylinder and frustum analytic extrema enclosed by certified bounds");
    println!(
        "exact B-rep volume API: unsupported by pinned dependencies; measured volumes are tessellated mesh estimates"
    );
}

fn signed_area(contour: &[[f64; 2]]) -> f64 {
    contour
        .iter()
        .zip(contour.iter().cycle().skip(1))
        .take(contour.len())
        .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
        .sum::<f64>()
        / 2.0
}

fn support_grid_counterexample() -> bool {
    const GRID_MM: f64 = 0.001;
    const EDGE_MM: f64 = 10.0;
    const SUPPORT_WIDTH_MM: f64 = 10.0;
    const PRISM_HEIGHT_MM: f64 = 1.0;
    const VALIDATOR_THRESHOLD_MM3: f64 = 1e-8;
    // Direct Cadrum 0.8.20 + OCCT 8.0.1 results from the matching prism fixture;
    // its source, command, and artifact provenance are recorded in cadrum_baseline.md.
    const CADRUM_BREP_BASELINE: [(f64, f64); 7] = [
        (5.000_000_413_701_855e-10, 0.0),
        (5.000_000_413_701_855e-9, 0.0),
        (1.999_999_987_845_058e-8, 0.0),
        (5.000_000_058_430_487e-8, 0.0),
        (9.999_999_939_225_29e-8, 0.0),
        (9.999_999_992_515_995e-7, 9.999_999_992_515_994e-6),
        (4.000_000_000_008_441e-4, 4.000_000_000_008_441e-3),
    ];

    let quantize = |x: f64| (x / GRID_MM).round() * GRID_MM;
    let exterior = vec![[0.0, 0.0], [EDGE_MM, 0.0], [EDGE_MM, 20.0], [0.0, 20.0]];
    let mut recognized_outside = false;
    let mut parity_mismatch = false;
    for (index, protrusion) in [5e-10, 5e-9, 2e-8, 5e-8, 1e-7, 1e-6, 4e-4]
        .into_iter()
        .enumerate()
    {
        let footprint = vec![
            [EDGE_MM - 1.0, 1.0],
            [EDGE_MM + protrusion, 1.0],
            [EDGE_MM + protrusion, 1.0 + SUPPORT_WIDTH_MM],
            [EDGE_MM - 1.0, 1.0 + SUPPORT_WIDTH_MM],
        ];
        let outside = footprint.overlay(&exterior, OverlayRule::Difference, FillRule::EvenOdd);
        let actual_protrusion = footprint[1][0] - EDGE_MM;
        let exact_outside_volume = actual_protrusion * SUPPORT_WIDTH_MM * PRISM_HEIGHT_MM;
        let planar_outside_area = outside
            .iter()
            .map(|shape| {
                shape
                    .iter()
                    .map(|contour| signed_area(contour))
                    .sum::<f64>()
                    .abs()
            })
            .sum::<f64>();
        let planar_outside_volume = planar_outside_area * PRISM_HEIGHT_MM;
        let area_threshold_rejects = planar_outside_volume >= VALIDATOR_THRESHOLD_MM3;
        let (cadrum_actual_protrusion, cadrum_outside_volume) = CADRUM_BREP_BASELINE[index];
        assert_eq!(
            actual_protrusion, cadrum_actual_protrusion,
            "Cadrum baseline coordinate at sample {index}"
        );
        let cadrum_accepts = cadrum_outside_volume < VALIDATOR_THRESHOLD_MM3;
        let overlay_accepts = outside.is_empty();
        println!(
            "support nominal_protrusion_mm={protrusion:.1e} actual_protrusion_mm={actual_protrusion:.17e} analytic_outside_area_mm2={exact_outside_volume:.17e} analytic_outside_volume_mm3={exact_outside_volume:.17e} source_threshold_rejects={} i_overlay_outside_area_mm2={planar_outside_area:.17e} area_threshold_rejects={area_threshold_rejects} cadrum_brep_outside_mm3={cadrum_outside_volume:.17e} cadrum_accepts={cadrum_accepts} i_overlay_accepts={overlay_accepts}",
            exact_outside_volume >= VALIDATOR_THRESHOLD_MM3,
        );
        recognized_outside |= !overlay_accepts && !cadrum_accepts;
        parity_mismatch |= cadrum_accepts != overlay_accepts;
    }
    assert!(
        recognized_outside,
        "float overlay and Cadrum reject the clearly protruding fixture"
    );
    assert_eq!(quantize(EDGE_MM), quantize(EDGE_MM + 4e-4));
    println!(
        "prepared-region 0.001 mm quantization collapses a 0.0004 mm boundary sliver; measured overlay/Cadrum parity is reported per fixture"
    );
    parity_mismatch
}

fn main() {
    accuracy_fixtures();
    let support_parity_mismatch = support_grid_counterexample();
    // CalcVolume is implemented for tessellated PolygonMesh solids, not analytic B-reps.
    let exact_brep_volume_available = false;
    println!(
        "accuracy_gate_verdict={}",
        if support_parity_mismatch || !exact_brep_volume_available {
            "FAIL: direct Cadrum planar parity differs at sampled slivers; exact B-rep volume and tight-bounds parity remain unqualified"
        } else {
            "PASS"
        }
    );
    if support_parity_mismatch || !exact_brep_volume_available {
        std::process::exit(2);
    }
}

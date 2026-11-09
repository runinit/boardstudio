use boardstudio_cad::{build_case, export_step, mesh_profile};
use boardstudio_contracts::PreparedCaseIR;

#[test]
fn prepared_pcb_round_drill_uses_the_public_case_entrypoint() {
    let input: PreparedCaseIR =
        serde_json::from_str(include_str!("fixtures/layered/pcb-simple.json")).unwrap();
    let model = build_case(&input).expect("prepared PCB with authored round drill");
    let mesh = mesh_profile(&model).expect("complete production mesh");
    assert!(!mesh.positions.is_empty());
    assert_eq!(mesh.positions.len(), mesh.normals.len());
    assert_eq!(mesh.positions.len() % 9, 0);
    assert!(
        mesh.positions
            .iter()
            .chain(&mesh.normals)
            .all(|v| v.is_finite())
    );
    let step = String::from_utf8(export_step(&model).unwrap()).unwrap();
    assert_eq!(step.matches("CYLINDRICAL_SURFACE(").count(), 1);
}

#[test]
fn adoption_cone_checks_only_the_height_band_intersecting_material() {
    let input: PreparedCaseIR =
        serde_json::from_str(include_str!("fixtures/adoption/cone-clipped-height.json")).unwrap();
    let model = build_case(&input).expect("the cone radius is 4 to 2 inside the plate, not 6");
    let analysis = boardstudio_cad::mesh_profile_for_analysis(&model, Some(1e-4)).unwrap();
    assert_eq!(model.cache_complexity().0, 1);
    assert_eq!(analysis.bounds, [[0., 0., 0.], [20., 16., 4.]]);
    let volume: f64 = analysis
        .positions_mm_f64
        .as_chunks::<9>()
        .0
        .iter()
        .map(|t| {
            let [a, b, c] = [&t[0..3], &t[3..6], &t[6..9]];
            (a[0] * (b[1] * c[2] - b[2] * c[1])
                + a[1] * (b[2] * c[0] - b[0] * c[2])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.
        })
        .sum();
    // Plate minus the intersecting frustum: pi*4*(4^2 + 4*2 + 2^2)/3.
    assert!((volume - (1280. - 112. * std::f64::consts::PI / 3.)).abs() < 0.002);
    let step = String::from_utf8(export_step(&model).unwrap()).unwrap();
    assert_eq!(step.matches("MANIFOLD_SOLID_BREP(").count(), 1);
    assert_eq!(step.matches("RATIONAL_B_SPLINE_SURFACE(").count(), 1);
}

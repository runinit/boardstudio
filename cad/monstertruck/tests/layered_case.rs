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

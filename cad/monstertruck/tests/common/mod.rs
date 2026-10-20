use boardstudio_contracts::{CaseBody, CaseKind, PreparedCaseIR, PreparedCaseRegion, Vec2};

/// A prepared plate with one region, the shape Core hands the kernel.
pub fn plate(thickness: f64, outer: Vec<Vec2>, holes: Vec<Vec<Vec2>>) -> PreparedCaseIR {
    PreparedCaseIR {
        revision: 1,
        body: CaseBody {
            id: "plate".into(),
            name: "Plate".into(),
            board_id: "board".into(),
            kind: CaseKind::Plate,
            thickness,
            clearance: 0.,
            material_id: None,
            z: None,
            wall_height: None,
            wall_thickness: None,
            mounts: None,
            pcb_source: None,
            gasket: None,
            features: None,
            openings: None,
        },
        regions: vec![PreparedCaseRegion {
            outer,
            holes,
            cavities: vec![],
            gaskets: vec![],
            mounts: vec![],
        }],
    }
}

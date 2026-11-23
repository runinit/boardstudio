//! Fixtures that build prepared cases the way Core hands them to the kernel, so kernel
//! tests exercise `build_case` and not a second builder.
use crate::{ExactModel, build_case};
use boardstudio_contracts::{CaseBody, CaseKind, PreparedCaseIR, PreparedCaseRegion, Vec2};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileKind {
    Plate,
    Tray,
    Lid,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProfileRegion {
    pub outer: Vec<Vec2>,
    pub holes: Vec<Vec<Vec2>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProfileInput {
    pub kind: ProfileKind,
    pub base_z: f64,
    pub thickness: f64,
    pub wall_height: f64,
    pub regions: Vec<ProfileRegion>,
}

/// Builds the plain extrusion of each region through `build_case`.
pub fn build_profile(input: ProfileInput) -> Result<ExactModel, String> {
    let kind = match input.kind {
        ProfileKind::Plate => CaseKind::Plate,
        ProfileKind::Tray => CaseKind::Tray,
        ProfileKind::Lid => CaseKind::Lid,
    };
    let plate = input.kind == ProfileKind::Plate;
    let case = PreparedCaseIR {
        revision: 1,
        body: CaseBody {
            id: "fixture".into(),
            name: "Fixture".into(),
            board_id: "board".into(),
            kind,
            thickness: input.thickness,
            clearance: 0.,
            material_id: None,
            z: Some(input.base_z),
            wall_height: (!plate).then_some(input.wall_height),
            wall_thickness: None,
            mounts: None,
            pcb_source: None,
            gasket: None,
            features: None,
            openings: None,
        },
        regions: input
            .regions
            .into_iter()
            .map(|region| PreparedCaseRegion {
                outer: region.outer,
                holes: region.holes,
                cavities: vec![],
                gaskets: vec![],
                mounts: vec![],
            })
            .collect(),
    };
    build_case(&case)
}

/// Axis-aligned bounds of 3D points, shared by the mesh test assertions.
pub fn bounds_of(points: impl IntoIterator<Item = [f64; 3]>) -> [[f64; 3]; 2] {
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for point in points {
        for axis in 0..3 {
            bounds[0][axis] = bounds[0][axis].min(point[axis]);
            bounds[1][axis] = bounds[1][axis].max(point[axis]);
        }
    }
    bounds
}

/// Asserts a builder rejects its input with an error containing `fragment`.
/// `context` names the rejected case for the failure message.
pub fn assert_rejected<T>(result: Result<T, String>, fragment: &str, context: &str) {
    match result {
        Ok(_) => panic!("{context}"),
        Err(error) => assert!(error.contains(fragment), "{error}"),
    }
}

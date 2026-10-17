use crate::ExactModel;
use monstertruck_assembly::assy::{Assembly, NodeEntity};
use monstertruck_io::step::{
    common::PartAttributes,
    save::{CompleteStepDisplay, StepDesign, StepModel},
};
use monstertruck_modeling::{Curve, Matrix4, Point3, Surface};
use std::fmt::Write;

/// One prepared body's identity and exact geometry, already in world coordinates.
pub struct StepBody<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub model: &'a ExactModel,
}

/// Writes every exact solid, in model order, as one millimetre STEP product.
pub fn export_step(model: &ExactModel) -> Result<Vec<u8>, String> {
    export_step_bodies(&[StepBody {
        id: "body",
        name: "Body",
        model,
    }])
}

/// Writes one STEP product per body, retaining every exact solid in body order.
/// Placement is already baked into geometry; no additional transform is applied.
pub fn export_step_bodies(bodies: &[StepBody<'_>]) -> Result<Vec<u8>, String> {
    if bodies.is_empty() || bodies.iter().any(|body| body.model.solids.is_empty()) {
        return Err("STEP export requires at least one exact solid per body".into());
    }
    let solids = bodies
        .iter()
        .map(|body| {
            body.model
                .solids
                .iter()
                .map(|solid| solid.compress())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    type Shape<'a> = StepModel<'a, Point3, Curve, Surface>;
    let mut assembly: Assembly<Vec<Shape<'_>>, PartAttributes, Matrix4, PartAttributes> =
        Assembly::new();
    for (body, solids) in bodies.iter().zip(&solids) {
        assembly.create_node(NodeEntity {
            shape: solids.iter().map(StepModel::from).collect(),
            attrs: PartAttributes {
                id: step_string(body.id),
                name: step_string(body.name),
                ..Default::default()
            },
        });
    }
    let design = StepDesign::<Shape<'_>, _, Matrix4>::new(assembly);
    let mut step = String::new();
    write!(
        &mut step,
        "{}",
        CompleteStepDisplay::new(design, Default::default())
    )
    .map_err(|_| "Monstertruck could not serialize exact STEP geometry".to_string())?;
    Ok(step.into_bytes())
}

// Part21 string escaping preserves caller identity when StepDesign interpolates attributes.
fn step_string(value: &str) -> String {
    let mut encoded = String::new();
    for c in value.chars() {
        match c {
            '\'' => encoded.push_str("''"),
            '\\' => encoded.push_str("\\\\"),
            ' '..='~' => encoded.push(c),
            _ if c.is_ascii() => write!(&mut encoded, "\\X\\{:02X}", c as u32).unwrap(),
            _ => write!(&mut encoded, "\\X4\\{:08X}\\X0\\", c as u32).unwrap(),
        }
    }
    encoded
}

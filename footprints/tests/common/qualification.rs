//! Shared render and S-expression fixture helpers for footprint qualification.

use boardstudio_footprints::{
    GeneratorError, bundled,
    nets::NetAllocator,
    sexpr::{self, Expr},
    types::{GeneratorRef, PartRef, Pose2, Side, Vec2},
};
use serde_json::Value;

pub fn generator(source: &str, parameters: Value) -> GeneratorRef {
    GeneratorRef {
        source: source.into(),
        version: "bundled-1".into(),
        parameters: serde_json::from_value(parameters).expect("fixture parameters"),
    }
}

pub fn part(id: &str, reference: &str, side: Side, at: Vec2, rotation: f64) -> PartRef {
    PartRef {
        id: id.into(),
        reference: reference.into(),
        pose: Pose2 { at, rotation },
        side,
        generator_parameters: None,
    }
}

pub fn render(
    definition: &str,
    generator: &GeneratorRef,
    part: Option<&PartRef>,
) -> Result<Vec<Expr>, GeneratorError> {
    bundled().render(definition, generator, part, &mut NetAllocator::new(&[], 1))
}

#[track_caller]
pub fn field<'a>(items: &'a [Expr], name: &str) -> &'a [Expr] {
    sexpr::child(items, name).unwrap_or_else(|| panic!("missing {name} in {items:?}"))
}

pub fn footprint(forms: &[Expr]) -> &[Expr] {
    field(forms, "footprint")
}

#[track_caller]
pub fn scalar(items: &[Expr], index: usize) -> &str {
    items[index].value().expect("scalar S-expression value")
}

#[track_caller]
pub fn number(items: &[Expr], index: usize) -> f64 {
    scalar(items, index)
        .parse()
        .expect("numeric S-expression value")
}

pub fn numbers_in_field(items: &[Expr], name: &str) -> Vec<f64> {
    let field = field(items, name);
    (1..field.len()).map(|index| number(field, index)).collect()
}

pub fn xyz(items: &[Expr], name: &str) -> [f64; 3] {
    numbers_in_field(field(items, name), "xyz")
        .try_into()
        .unwrap_or_else(|values| panic!("expected three {name} coordinates, got {values:?}"))
}

pub fn layers(pad: &[Expr]) -> Vec<&str> {
    let layers = field(pad, "layers");
    (1..layers.len())
        .map(|index| scalar(layers, index))
        .collect()
}

pub fn net_name(pad: &[Expr]) -> Option<&str> {
    sexpr::child(pad, "net").map(|net| scalar(net, 2))
}

pub fn smd_pad<'a>(footprint: &'a [Expr], number: &str) -> &'a [Expr] {
    sexpr::children(footprint, "pad")
        .find(|pad| scalar(pad, 1) == number && scalar(pad, 2) == "smd")
        .unwrap_or_else(|| panic!("missing SMD pad {number}"))
}

#[track_caller]
pub fn near(actual: f64, expected: f64, label: &str) {
    let epsilon = 1e-6;
    assert!(
        (actual - expected).abs() <= epsilon,
        "{label}: expected {expected}, found {actual} (epsilon {epsilon})"
    );
}

pub fn kicad_world_delta(footprint: &[Expr], [x, y]: [f64; 2]) -> [f64; 2] {
    // KiCad uses a down-Y frame. The expected vectors in each fixture are
    // independently derived from drawing dimensions.
    let (sin, cos) = number(field(footprint, "at"), 3).to_radians().sin_cos();
    [x * cos + y * sin, -x * sin + y * cos]
}

pub fn socket_model_rotation(side: &str) -> [f64; 3] {
    if side == "F" {
        [0.0, 180.0, 0.0]
    } else {
        [180.0, 0.0, 0.0]
    }
}

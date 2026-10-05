//! `ceoloide/utility_logo`: a silkscreen logo polygon, optionally on both sides.
//!
//! Ported from `utility_ergogen_logo.js` of ceoloide/ergogen-footprints.
//! Copyright (c) 2023 Marco Massarelli
//! SPDX-License-Identifier: MIT
//! Author: @dieseltravis + @ceoloide improvements
//!
//! The source was named `utility_ergogen_logo`; the plan renames it so that no
//! persisted identifier contains `ergogen`.
use crate::Result;
use crate::context::RenderContext;
use crate::number::{js_number as n, js_to_fixed};
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 5] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "LOGO"),
    ParamSpec::string("layer", "SilkS"),
    ParamSpec::boolean("reversible", false),
    ParamSpec::number("scale", 1.0),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "ceoloide/utility_logo",
    display_name: "utility logo",
    kind: PartKind::Utility,
    matrix_terminals: None,
    keycap_parameters: None,
    parameters: &PARAMS,
    license: License {
        spdx: "MIT",
        author: "@dieseltravis + @ceoloide improvements",
    },
    body,
};

/// Polygon outline in the logo's own units.
const POINTS: [(f64, f64); 56] = [
    (2.501231, 0.0),
    (2.501231, 2.501231),
    (0.0, 2.501231),
    (-2.50123, 2.501231),
    (-2.50123, 1.013088),
    (-1.738355, 1.013088),
    (-0.021885, 1.009917),
    (1.694584, 1.006746),
    (1.697905, 0.662827),
    (1.701225, 0.318907),
    (1.52891, 0.490867),
    (1.356594, 0.662827),
    (-0.19088, 0.662827),
    (-1.738355, 0.662827),
    (-1.738355, 0.837957),
    (-1.738355, 1.013088),
    (-2.50123, 1.013088),
    (-2.50123, 0.150074),
    (-1.394101, 0.150074),
    (-0.637478, 0.150074),
    (0.119144, 0.150074),
    (0.293895, -0.025012),
    (0.468646, -0.200098),
    (-0.287976, -0.200098),
    (-1.044599, -0.200098),
    (-1.21935, -0.025012),
    (-1.394101, 0.150074),
    (-2.50123, 0.150074),
    (-2.50123, 0.0),
    (-2.50123, -1.063023),
    (-1.738355, -1.063023),
    (-1.738355, -0.887937),
    (-1.738355, -0.71285),
    (-0.190545, -0.71285),
    (1.357266, -0.71285),
    (1.525751, -0.544017),
    (1.578342, -0.491483),
    (1.624575, -0.445614),
    (1.661679, -0.409133),
    (1.686885, -0.384763),
    (1.697422, -0.375226),
    (1.697537, -0.375184),
    (1.698399, -0.387158),
    (1.699177, -0.420952),
    (1.69984, -0.473372),
    (1.700359, -0.541225),
    (1.700701, -0.62132),
    (1.700836, -0.710463),
    (1.700837, -0.719103),
    (1.700837, -1.063023),
    (-0.018759, -1.063023),
    (-1.738355, -1.063023),
    (-2.50123, -1.063023),
    (-2.50123, -2.50123),
    (0.0, -2.50123),
    (2.501231, -2.50123),
];

fn polygon(side: &str, layer: &str, scale: f64, mirrored: bool) -> String {
    let points: Vec<String> = POINTS
        .iter()
        .map(|&(x, y)| {
            let scaled_x = x * scale * if mirrored { -1.0 } else { 1.0 };
            let scaled_y = y * scale;
            format!(
                "(xy {} {})",
                js_to_fixed(scaled_x, 6),
                js_to_fixed(scaled_y, 6)
            )
        })
        .collect();
    format!(
        "(fp_poly (pts {}) (stroke (width 0.01) (type solid)) (fill solid) (layer \"{side}.{layer}\"))\n",
        points.join(" ")
    )
}

fn body(p: &RenderContext<'_>) -> Result<String> {
    let (side, layer, scale) = (p.side(), p.text("layer"), p.number("scale"));
    let mut out = format!(
        "(footprint \"ceoloide:utility_logo\"
  (layer \"{side}.Cu\")
  {at}
  (property \"Reference\" \"{reference}\"
    (at {label_x} 0 {rotation})
    (layer \"{side}.Fab\")
    {hide}
    (effects (font (size 1 1) (thickness 0.15)))
  )
  (attr exclude_from_pos_files exclude_from_bom)
",
        at = p.at(),
        reference = p.reference(),
        label_x = n(scale * 4.572),
        rotation = n(p.rotation()),
        hide = p.ref_hide(),
    );
    if p.flag("reversible") {
        out.push_str(&polygon("F", layer, scale, false));
        out.push_str(&polygon("B", layer, scale, true));
    } else {
        out.push_str(&polygon(side, layer, scale, side == "B"));
    }
    out.push_str(")\n");
    Ok(out)
}

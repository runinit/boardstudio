//! `ceoloide/mounting_hole_plated`: a plated mounting hole, round or oval.
//!
//! Ported from `mounting_hole_plated.js` of ceoloide/ergogen-footprints.
//! Copyright (c) 2023 Marco Massarelli
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0
//! Author: @infused-kim + @ceoloide improvements
use crate::Expr;
use crate::Result;
use crate::build::{at, atom, effects, form, layer, num, nums, placement, string};
use crate::context::RenderContext;
use crate::number::js_number;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 6] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "H"),
    ParamSpec::number("outline", 0.8),
    ParamSpec::number("drill", 2.2),
    ParamSpec::number("drill_y", 0.0),
    ParamSpec::boolean("include_courtyard", true),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "ceoloide/mounting_hole_plated",
    display_name: "mounting hole plated",
    kind: PartKind::Custom,
    matrix_terminals: None,
    keycap_parameters: None,
    parameters: &PARAMS,
    license: License {
        spdx: "CC-BY-NC-SA-4.0",
        author: "@infused-kim + @ceoloide improvements",
    },
    body: crate::registry::Body::Forms(body),
};

fn body(p: &RenderContext<'_>) -> Result<Vec<Expr>> {
    let drill = p.number("drill");
    let drill_y = if p.number("drill_y") == 0.0 {
        drill
    } else {
        p.number("drill_y")
    };
    let size_x = drill + p.number("outline") * 2.0;
    let size_y = drill_y + p.number("outline") * 2.0;
    let courtyard_offset = 0.25;
    let courtyard_x = size_x / 2.0 + courtyard_offset;
    let courtyard_y = size_y / 2.0 + courtyard_offset;
    let size = nums("size", &[size_x, size_y]);
    let layers = form("layers", [string("*.Cu"), string("*.Mask")]);
    let mut items = vec![
        string("ceoloide:mounting_hole_plated"),
        layer(p, "Cu"),
        placement(p),
        form(
            "property",
            [
                string("Reference"),
                string(p.reference()),
                at(0.0, 3.0, p.rotation()),
                layer(p, "SilkS"),
                effects(1.0, 0.15),
            ],
        ),
    ];
    if size_x == size_y {
        items.push(form(
            "pad",
            [
                string(""),
                atom("thru_hole"),
                atom("circle"),
                at(0.0, 0.0, p.rotation()),
                size,
                nums("drill", &[drill]),
                layers,
            ],
        ));
        for side in ["F", "B"] {
            items.push(form(
                "fp_circle",
                [
                    nums("center", &[0.0, 0.0]),
                    form("end", [negated(courtyard_x), num(0.0)]),
                    courtyard_layer(side),
                    stroke(),
                    form("fill", [atom("none")]),
                ],
            ));
        }
    } else {
        items.push(form(
            "pad",
            [
                string(""),
                atom("thru_hole"),
                atom("oval"),
                at(0.0, 0.0, p.rotation()),
                size,
                form("drill", [atom("oval"), num(drill), num(drill_y)]),
                layers,
            ],
        ));
        // (start x, start y, end x, end y) signs, in the order the generator draws them.
        let edges = [
            ("F", [1.0, -1.0, 1.0, 1.0]),
            ("F", [-1.0, -1.0, -1.0, 1.0]),
            ("F", [-1.0, 1.0, 1.0, 1.0]),
            ("F", [-1.0, -1.0, 1.0, -1.0]),
            ("B", [-1.0, 1.0, 1.0, 1.0]),
            ("B", [-1.0, 1.0, -1.0, -1.0]),
            ("B", [-1.0, -1.0, 1.0, -1.0]),
            ("B", [1.0, 1.0, 1.0, -1.0]),
        ];
        for (side, [sx, sy, ex, ey]) in edges {
            items.push(form(
                "fp_line",
                [
                    form("start", [signed(sx, courtyard_x), signed(sy, courtyard_y)]),
                    form("end", [signed(ex, courtyard_x), signed(ey, courtyard_y)]),
                    courtyard_layer(side),
                    stroke(),
                ],
            ));
        }
    }
    Ok(vec![form("footprint", items)])
}

fn courtyard_layer(side: &str) -> Expr {
    form("layer", [string(&format!("{side}.CrtYd"))])
}

fn stroke() -> Expr {
    form(
        "stroke",
        [nums("width", &[0.05]), form("type", [atom("solid")])],
    )
}

/// A leading minus on the spelled value, so a zero edge stays `-0` as before.
fn negated(value: f64) -> Expr {
    Expr::Atom(format!("-{}", js_number(value)))
}

fn signed(sign: f64, value: f64) -> Expr {
    if sign < 0.0 {
        negated(value)
    } else {
        num(value)
    }
}

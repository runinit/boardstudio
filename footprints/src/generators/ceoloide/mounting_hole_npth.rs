//! `ceoloide/mounting_hole_npth`: a non-plated mounting hole.
//!
//! Ported from `mounting_hole_npth.js` of ceoloide/ergogen-footprints.
//! Copyright (c) 2023 Marco Massarelli
//! SPDX-License-Identifier: MIT
//! Author: @ceoloide
//!
//! `hole_size` and `hole_drill` are numbers; the source declared them as strings.
use crate::Expr;
use crate::Result;
use crate::build::{at, atom, effects, form, layer, nums, placement, string};
use crate::context::RenderContext;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 4] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "MH"),
    ParamSpec::number("hole_size", 2.2),
    ParamSpec::number("hole_drill", 2.2),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "ceoloide/mounting_hole_npth",
    display_name: "mounting hole npth",
    kind: PartKind::Custom,
    matrix_terminals: None,
    keycap_parameters: None,
    parameters: &PARAMS,
    license: License {
        spdx: "MIT",
        author: "@ceoloide",
    },
    body: crate::registry::Body::Forms(body),
};

fn body(p: &RenderContext<'_>) -> Result<Vec<Expr>> {
    let size = p.number("hole_size");
    Ok(vec![form(
        "footprint",
        [
            string("ceoloide:mounting_hole_npth"),
            layer(p, "Cu"),
            placement(p),
            form(
                "property",
                [
                    string("Reference"),
                    string(p.reference()),
                    at(0.0, 2.55, p.rotation()),
                    layer(p, "SilkS"),
                    effects(1.0, 0.15),
                ],
            ),
            form(
                "pad",
                [
                    string(""),
                    atom("np_thru_hole"),
                    atom("circle"),
                    at(0.0, 0.0, p.rotation()),
                    nums("size", &[size, size]),
                    nums("drill", &[p.number("hole_drill")]),
                    form("layers", [string("*.Cu"), string("*.Mask")]),
                ],
            ),
        ],
    )])
}

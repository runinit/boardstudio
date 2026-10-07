//! `ceoloide/mounting_hole_npth`: a non-plated mounting hole.
//!
//! Ported from `mounting_hole_npth.js` of ceoloide/ergogen-footprints.
//! Copyright (c) 2023 Marco Massarelli
//! SPDX-License-Identifier: MIT
//! Author: @ceoloide
//!
//! `hole_size` and `hole_drill` are numbers; the source declared them as strings.
use crate::Result;
use crate::context::RenderContext;
use crate::number::js_number as n;
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
    body,
};

fn body(p: &RenderContext<'_>) -> Result<String> {
    let side = p.side();
    let size = n(p.number("hole_size"));
    Ok(format!(
        r#"
  (footprint "ceoloide:mounting_hole_npth"
    (layer "{side}.Cu")
    {at}
    (property "Reference" "{reference}"
      (at 0 2.55 {rotation})
      (layer "{side}.SilkS")
      {hide}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (pad "" np_thru_hole circle
      (at 0 0 {rotation})
      (size {size} {size})
      (drill {drill})
      (layers "*.Cu" "*.Mask")
    )
  )
"#,
        at = p.at(),
        reference = p.reference(),
        rotation = n(p.rotation()),
        hide = p.ref_hide(),
        drill = n(p.number("hole_drill")),
    ))
}

//! `ceoloide/mounting_hole_plated`: a plated mounting hole, round or oval.
//!
//! Ported from `mounting_hole_plated.js` of ceoloide/ergogen-footprints.
//! Copyright (c) 2023 Marco Massarelli
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0
//! Author: @infused-kim + @ceoloide improvements
use crate::Result;
use crate::context::RenderContext;
use crate::number::js_number as n;
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
    body,
};

fn body(p: &RenderContext<'_>) -> Result<String> {
    let side = p.side();
    let drill = p.number("drill");
    let drill_y = if p.number("drill_y") == 0.0 {
        drill
    } else {
        p.number("drill_y")
    };
    let size_x = drill + p.number("outline") * 2.0;
    let size_y = drill_y + p.number("outline") * 2.0;
    let courtyard_offset = 0.25;
    let courtyard_x = n(size_x / 2.0 + courtyard_offset);
    let courtyard_y = n(size_y / 2.0 + courtyard_offset);
    let (size_x_text, size_y_text) = (n(size_x), n(size_y));
    let mut out = format!(
        r#"
  (footprint "ceoloide:mounting_hole_plated"
    (layer "{side}.Cu")
    {at}
    (property "Reference" "{reference}"
      (at 0 3 {rotation})
      (layer "{side}.SilkS")
      {hide}
      (effects (font (size 1 1) (thickness 0.15)))
    )
"#,
        at = p.at(),
        reference = p.reference(),
        rotation = n(p.rotation()),
        hide = p.ref_hide(),
    );
    if size_x == size_y {
        out.push_str(&format!(
            r#"
    (pad "" thru_hole circle (at 0 0 {rotation}) (size {size_x_text} {size_y_text}) (drill {drill}) (layers "*.Cu" "*.Mask"))
    (fp_circle (center 0 0) (end -{courtyard_x} 0) (layer "F.CrtYd") (stroke (width 0.05) (type solid)) (fill none))
    (fp_circle (center 0 0) (end -{courtyard_x} 0) (layer "B.CrtYd") (stroke (width 0.05) (type solid)) (fill none))
"#,
            rotation = n(p.rotation()),
            drill = n(drill),
        ));
    } else {
        out.push_str(&format!(
            r#"
    (pad "" thru_hole oval (at 0 0 {rotation}) (size {size_x_text} {size_y_text}) (drill oval {drill} {drill_y_text}) (layers "*.Cu" "*.Mask"))
    (fp_line (start {courtyard_x} -{courtyard_y}) (end {courtyard_x} {courtyard_y}) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -{courtyard_x} -{courtyard_y}) (end -{courtyard_x} {courtyard_y}) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -{courtyard_x} {courtyard_y}) (end {courtyard_x} {courtyard_y}) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -{courtyard_x} -{courtyard_y}) (end {courtyard_x} -{courtyard_y}) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -{courtyard_x} {courtyard_y}) (end {courtyard_x} {courtyard_y}) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -{courtyard_x} {courtyard_y}) (end -{courtyard_x} -{courtyard_y}) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -{courtyard_x} -{courtyard_y}) (end {courtyard_x} -{courtyard_y}) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start {courtyard_x} {courtyard_y}) (end {courtyard_x} -{courtyard_y}) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
"#,
            rotation = n(p.rotation()),
            drill = n(drill),
            drill_y_text = n(drill_y),
        ));
    }
    out.push_str("  )\n");
    Ok(out)
}

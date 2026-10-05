//! `infused-kim/mounting_hole`: a plated mounting hole, round or oval.
//!
//! Ported from `mounting_hole.js` of infused-kim/kb_ergogen_fp.
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0 (see the vendored LICENSE)
//! Author: @infused-kim
use crate::Result;
use crate::context::RenderContext;
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 5] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "H"),
    ParamSpec::number("outline", 0.8),
    ParamSpec::number("drill", 2.2),
    ParamSpec::number("drill_y", 0.0),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "infused-kim/mounting_hole",
    display_name: "mounting hole",
    kind: PartKind::Custom,
    matrix_terminals: None,
    keycap_parameters: None,
    parameters: &PARAMS,
    license: License {
        spdx: "CC-BY-NC-SA-4.0",
        author: "@infused-kim",
    },
    body,
};

fn body(p: &RenderContext<'_>) -> Result<String> {
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
            (module mounting_hole (layer F.Cu) (tedit 64B5A986)
                {at}
                (fp_text reference "{reference}" (at 0 3) (layer F.SilkS) {hide}
                    (effects (font (size 1 1) (thickness 0.15)))
                )
        "#,
        at = p.at(),
        reference = p.reference(),
        hide = p.ref_hide(),
    );
    if size_x == size_y {
        out.push_str(&format!(
            r#"
                (pad "" thru_hole circle (at 0 0 180) (size {size_x_text} {size_y_text}) (drill {drill}) (layers *.Cu *.Mask))
                (fp_circle (center 0 0) (end -{courtyard_x} 0) (layer F.CrtYd) (width 0.05))
                (fp_circle (center 0 0) (end -{courtyard_x} 0) (layer B.CrtYd) (width 0.05))
        "#,
            drill = n(drill),
        ));
    } else {
        out.push_str(&format!(
            r#"
                (pad "" thru_hole oval (at 0 0 180) (size {size_x_text} {size_y_text}) (drill oval {drill} {drill_y}) (layers *.Cu *.Mask))
                (fp_line (start {courtyard_x} -{courtyard_y}) (end {courtyard_x} {courtyard_y}) (layer F.CrtYd) (width 0.05))
                (fp_line (start -{courtyard_x} -{courtyard_y}) (end -{courtyard_x} {courtyard_y}) (layer F.CrtYd) (width 0.05))
                (fp_line (start -{courtyard_x} {courtyard_y}) (end {courtyard_x} {courtyard_y}) (layer F.CrtYd) (width 0.05))
                (fp_line (start -{courtyard_x} -{courtyard_y}) (end {courtyard_x} -{courtyard_y}) (layer F.CrtYd) (width 0.05))
                (fp_line (start -{courtyard_x} {courtyard_y}) (end {courtyard_x} {courtyard_y}) (layer B.CrtYd) (width 0.05))
                (fp_line (start -{courtyard_x} {courtyard_y}) (end -{courtyard_x} -{courtyard_y}) (layer B.CrtYd) (width 0.05))
                (fp_line (start -{courtyard_x} -{courtyard_y}) (end {courtyard_x} -{courtyard_y}) (layer B.CrtYd) (width 0.05))
                (fp_line (start {courtyard_x} {courtyard_y}) (end {courtyard_x} -{courtyard_y}) (layer B.CrtYd) (width 0.05))
        "#,
            drill = n(drill),
            drill_y = n(drill_y),
        ));
    }
    out.push_str("\n            )\n        ");
    Ok(out)
}

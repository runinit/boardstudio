//! `ceoloide/reset_switch_tht_top`
//!
//! Ported from `reset_switch_tht_top.js` of ceoloide/ergogen-footprints.
//! Copyright (c) 2023 Marco Massarelli
//! SPDX-License-Identifier: MIT
//! Author: @ceoloide
use crate::Result;
use crate::context::RenderContext;
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 6] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "RST"),
    ParamSpec::boolean("reversible", false),
    ParamSpec::boolean("include_silkscreen", true),
    ParamSpec::net_default("from", "GND"),
    ParamSpec::net_default("to", "RST"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "ceoloide/reset_switch_tht_top",
    display_name: "reset switch tht top",
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
    // Match the PTS636 THT drawing and preserve the 0.35 mm annular ring.
    let pin_spacing = 6.4_f64;
    let pin_drill = 1.2_f64;
    let pin_pad = 1.9_f64;
    let common_start = format!(
        r####"
  (footprint "ceoloide:reset_switch_tht_top"
    (layer "{e0}.Cu")
    {e1}
    (property "Reference" "{e2}"
      (at 0 2.55 {e3})
      (layer "{e0}.SilkS")
      {e4}
      (effects (font (size 1 1) (thickness 0.15)))
    )
        "####,
        e0 = p.side(),
        e1 = p.at(),
        e2 = p.reference(),
        e3 = n(90.0 + p.rotation()),
        e4 = p.ref_hide(),
    );

    let silkscreen_front = format!(
        r####"
    (fp_text user "RST" (at 0 0 {e0}) (layer "F.SilkS") (effects (font (size 1 1) (thickness 0.15))))
    (fp_line (start -3 1.75) (end 3 1.75) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 3 1.75) (end 3 1.5) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start -3 1.75) (end -3 1.5) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start -3 -1.75) (end -3 -1.5) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start -3 -1.75) (end 3 -1.75) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 3 -1.75) (end 3 -1.5) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
        "####,
        e0 = n(p.rotation()),
    );

    let silkscreen_back = format!(
        r####"
    (fp_text user "RST" (at 0 0 {e0}) (layer "B.SilkS") (effects (font (size 1 1) (thickness 0.15)) (justify mirror)))
    (fp_line (start 3 1.5) (end 3 1.75) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 3 1.75) (end -3 1.75) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start -3 1.75) (end -3 1.5) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start -3 -1.5) (end -3 -1.75) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start -3 -1.75) (end 3 -1.75) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 3 -1.75) (end 3 -1.5) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
        "####,
        e0 = n(p.rotation()),
    );

    let common_end = format!(
        r####"
    (pad "2" thru_hole circle (at {e0} 0 {e1}) (size {e2} {e2}) (drill {e3}) (layers "*.Cu" "*.Mask") {e4})
    (pad "1" thru_hole circle (at {e5} 0 {e1}) (size {e2} {e2}) (drill {e3}) (layers "*.Cu" "*.Mask") {e6})
  )
        "####,
        e0 = n(-pin_spacing / 2.0),
        e1 = n(p.rotation()),
        e2 = n(pin_pad),
        e3 = n(pin_drill),
        e4 = p.net("from"),
        e5 = n(pin_spacing / 2.0),
        e6 = p.net("to"),
    );

    let (side, reversible) = (p.side(), p.flag("reversible"));
    let mut out = common_start;
    if (side == "F" || reversible) && p.flag("include_silkscreen") {
        out += &silkscreen_front;
    }
    if (side == "B" || reversible) && p.flag("include_silkscreen") {
        out += &silkscreen_back;
    }
    out += &common_end;
    Ok(out)
}

//! `ceoloide/battery_connector_molex_pico_ezmate_1x02`
//!
//! Ported from `battery_connector_molex_pico_ezmate_1x02.js` of ceoloide/ergogen-footprints.
//! Copyright (c) 2023 Marco Massarelli
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0
//! Author: @infused-kim + @ceoloide improvements
use crate::Result;
use crate::context::RenderContext;
use crate::generators::util::{join_numbers, xyz};
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 16] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "CONN"),
    ParamSpec::boolean("reversible", false),
    ParamSpec::boolean("include_silkscreen", true),
    ParamSpec::boolean("include_fabrication", true),
    ParamSpec::boolean("include_courtyard", true),
    ParamSpec::string(
        "socket_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/Molex_Ezmate_Pico_Socket_2pin.step",
    ),
    ParamSpec::array("socket_3dmodel_xyz_scale", "[1,1,1]"),
    ParamSpec::array("socket_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("socket_3dmodel_xyz_offset", "[0,0,1.4]"),
    ParamSpec::string(
        "cable_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/Molex_Ezmate_Pico_Cable_2pin.step",
    ),
    ParamSpec::array("cable_3dmodel_xyz_scale", "[1,1,1]"),
    ParamSpec::array("cable_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("cable_3dmodel_xyz_offset", "[0,0,0.8]"),
    ParamSpec::net_default("BAT_P", "BAT_P"),
    ParamSpec::net_default("BAT_N", "GND"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "ceoloide/battery_connector_molex_pico_ezmate_1x02",
    display_name: "battery connector molex pico ezmate 1x02",
    kind: PartKind::Connector,
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
    // KiCad flips a back-side model around X; turn it to match the mirrored pins.
    let model = |name: &str, rotation: [f64; 3]| -> Result<String> {
        let path = p.text(&format!("{name}_3dmodel_filename"));
        if path.is_empty() {
            return Ok(String::new());
        }
        let angles = p
            .vec3(&format!("{name}_3dmodel_xyz_rotation"))?
            .unwrap_or(rotation);
        Ok(format!(
            "\n    (model \"{path}\"\n      (offset (xyz {}))\n      (scale (xyz {}))\n      (rotate (xyz {})))",
            join_numbers(p.list(&format!("{name}_3dmodel_xyz_offset"))),
            join_numbers(p.list(&format!("{name}_3dmodel_xyz_scale"))),
            xyz(angles),
        ))
    };
    let top = format!(
        r####"
  (footprint "ceoloide:battery_connector_molex_pico_ezmate_1x02"
    (layer "{e0}.Cu")
    {e1}
    (property "Reference" "{e2}"
      (at 0.1 3.9 {e3})
      (layer "{e0}.SilkS")
      {e4}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (attr smd)
    "####,
        e0 = p.side(),
        e1 = p.at(),
        e2 = p.reference(),
        e3 = n(p.rotation()),
        e4 = p.ref_hide()
    );

    let front_silkscreen = String::from(
        r####"
    (fp_line (start 0.5 3.85) (end 1.5 3.85) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 1 4.35) (end 1 3.35) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -0.5 3.85) (end -1.5 3.85) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 0.64 2.63) (end 1.14 2.63) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 0.34 2.13) (end 0.64 2.63) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -0.34 2.13) (end 0.34 2.13) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -0.64 2.63) (end -0.34 2.13) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -1.16 -2.09) (end -1.16 -2.3) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -2.21 -2.09) (end -1.16 -2.09) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -2.21 1.24) (end -2.21 -2.09) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -1.14 2.63) (end -0.64 2.63) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 2.21 -2.09) (end 1.16 -2.09) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 2.21 1.24) (end 2.21 -2.09) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    "####,
    );

    let front_fabrication = String::from(
        r####"
    (fp_line (start -0.45 2.02) (end 0.45 2.02) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -0.75 2.52) (end -0.45 2.02) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -2.1 2.52) (end -0.75 2.52) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -2.1 -1.98) (end 2.1 -1.98) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -0.6 -1.272893) (end -0.1 -1.98) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -1.1 -1.98) (end -0.6 -1.272893) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.1 -1.98) (end 2.1 2.52) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -2.1 -1.98) (end -2.1 2.52) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 0.75 2.52) (end 2.1 2.52) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 0.45 2.02) (end 0.75 2.52) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    "####,
    );

    let front_courtyard = String::from(
        r####"
    (fp_line (start 2.6 -2.8) (end -2.6 -2.8) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -2.6 -2.8) (end -2.6 6.75) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 2.6 6.75) (end 2.6 -2.8) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -2.6 6.75) (end 2.6 6.75) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    "####,
    );

    let front_pads = format!(
        r####"
    (pad "" smd roundrect (at 1.75 1.9 {e0}) (size 0.7 0.8) (layers "F.Cu" "F.Paste" "F.Mask") (roundrect_rratio 0.25))
    (pad "" smd roundrect (at -1.75 1.9 {e0}) (size 0.7 0.8) (layers "F.Cu" "F.Paste" "F.Mask") (roundrect_rratio 0.25))
    (pad "2" smd roundrect (at 0.6 -1.875 {e0}) (size 0.6 0.85) (layers "F.Cu" "F.Paste" "F.Mask") (roundrect_rratio 0.25) {e1})
    (pad "1" smd roundrect (at -0.6 -1.875 {e0}) (size 0.6 0.85) (layers "F.Cu" "F.Paste" "F.Mask") (roundrect_rratio 0.25) {e2})
    "####,
        e0 = n(p.rotation()),
        e1 = p.net("BAT_P"),
        e2 = p.net("BAT_N")
    );

    let back_silkscreen = String::from(
        r####"
    (fp_line (start 0.5 3.85) (end 1.5 3.85) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -0.5 3.85) (end -1.5 3.85) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -1 4.35) (end -1 3.35) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -0.34 2.13) (end -0.64 2.63) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -0.64 2.63) (end -1.14 2.63) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 1.16 -2.09) (end 1.16 -2.3) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 2.21 -2.09) (end 1.16 -2.09) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 2.21 1.24) (end 2.21 -2.09) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -2.21 -2.09) (end -1.16 -2.09) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -2.21 1.24) (end -2.21 -2.09) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 0.64 2.63) (end 0.34 2.13) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 1.14 2.63) (end 0.64 2.63) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 0.34 2.13) (end -0.34 2.13) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    "####,
    );

    let back_fabrication = String::from(
        r####"
    (fp_line (start 2.1 -1.98) (end -2.1 -1.98) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 0.6 -1.272893) (end 0.1 -1.98) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 1.1 -1.98) (end 0.6 -1.272893) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -2.1 -1.98) (end -2.1 2.52) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.1 -1.98) (end 2.1 2.52) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -0.75 2.52) (end -2.1 2.52) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -0.45 2.02) (end -0.75 2.52) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 0.45 2.02) (end -0.45 2.02) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 0.75 2.52) (end 0.45 2.02) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.1 2.52) (end 0.75 2.52) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    "####,
    );

    let back_courtyard = String::from(
        r####"
    (fp_line (start -2.6 6.75) (end -2.6 -2.8) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 2.6 6.75) (end -2.6 6.75) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -2.6 -2.8) (end 2.6 -2.8) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 2.6 -2.8) (end 2.6 6.75) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    "####,
    );

    let back_pads = format!(
        r####"
    (pad "" smd roundrect (at 1.75 1.9 {e0}) (size 0.7 0.8) (layers "B.Cu" "B.Paste" "B.Mask") (roundrect_rratio 0.25))
    (pad "" smd roundrect (at -1.75 1.9 {e0}) (size 0.7 0.8) (layers "B.Cu" "B.Paste" "B.Mask") (roundrect_rratio 0.25))
    (pad "1" smd roundrect (at 0.6 -1.875 {e0}) (size 0.6 0.85) (layers "B.Cu" "B.Paste" "B.Mask") (roundrect_rratio 0.25) {e1})
    (pad "2" smd roundrect (at -0.6 -1.875 {e0}) (size 0.6 0.85) (layers "B.Cu" "B.Paste" "B.Mask") (roundrect_rratio 0.25) {e2})
    "####,
        e0 = n(180.0 + p.rotation()),
        e1 = p.net("BAT_N"),
        e2 = p.net("BAT_P")
    );

    let bottom = String::from(
        r####"
  )
    "####,
    );

    let (side, reversible) = (p.side(), p.flag("reversible"));
    let mut out = top;
    if side == "F" || reversible {
        out += &front_pads;
        if p.flag("include_silkscreen") {
            out += &front_silkscreen;
        }
        if p.flag("include_courtyard") {
            out += &front_courtyard;
        }
        if p.flag("include_fabrication") {
            out += &front_fabrication;
        }
    }
    if side == "B" || reversible {
        out += &back_pads;
        if p.flag("include_silkscreen") {
            out += &back_silkscreen;
        }
        if p.flag("include_courtyard") {
            out += &back_courtyard;
        }
        if p.flag("include_fabrication") {
            out += &back_fabrication;
        }
    }
    let turn = if side == "B" { 180.0 } else { 0.0 };
    out += &model("socket", [-90.0, 0.0, turn])?;
    out += &model("cable", [0.0, 0.0, turn])?;
    out += &bottom;
    Ok(out)
}

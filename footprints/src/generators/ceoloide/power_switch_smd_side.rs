//! `ceoloide/power_switch_smd_side`
//!
//! Ported from `power_switch_smd_side.js` of ceoloide/ergogen-footprints.
//! Copyright (c) 2023 Marco Massarelli
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0
//! Authors: @infused-kim + @ceoloide improvements
use crate::Result;
use crate::context::RenderContext;
use crate::generators::util::json_string;
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 12] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "PWR"),
    ParamSpec::boolean("reversible", false),
    ParamSpec::boolean("invert_behavior", false),
    ParamSpec::boolean("include_silkscreen", true),
    ParamSpec::boolean("include_courtyard", false),
    ParamSpec::string(
        "switch_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/Switch_Power.step",
    ),
    ParamSpec::array("switch_3dmodel_xyz_offset", "[0,0,0]"),
    ParamSpec::array("switch_3dmodel_xyz_rotation", "[-90,0,-90]"),
    ParamSpec::array("switch_3dmodel_xyz_scale", "[1,1,1]"),
    ParamSpec::net_default("from", "BAT_P"),
    ParamSpec::net_default("to", "RAW"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "ceoloide/power_switch_smd_side",
    display_name: "power switch smd side",
    kind: PartKind::Passive,
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
    for name in [
        "switch_3dmodel_xyz_offset",
        "switch_3dmodel_xyz_scale",
        "switch_3dmodel_xyz_rotation",
    ] {
        p.vec3(name)?;
    }
    let common_start = format!(
        r####"
  (footprint "ceoloide:power_switch_smd_side"
    (layer "{e0}.Cu")
    {e1}
    (property "Reference" "{e2}"
      (at -3.6 0 {e3})
      (layer "{e0}.SilkS")
      {e4}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (attr smd)
    "####,
        e0 = p.side(),
        e1 = p.at(),
        e2 = p.reference(),
        e3 = n(-90.0 + p.rotation()),
        e4 = p.ref_hide(),
    );

    let silkscreen_front = format!(
        r####"
    (fp_text user "ON" (at 0 {e0}5 {e1}) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (fp_text user "OFF" (at 0 {e2}5 {e1}) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (fp_line (start 0.415 -3.45) (end -0.375 -3.45) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -0.375 3.45) (end 0.415 3.45) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -1.425 1.6) (end -1.425 -0.1) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 1.425 2.85) (end 1.425 -2.85) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -1.425 -1.4) (end -1.425 -1.6) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    "####,
        e0 = if p.flag("invert_behavior") { "-" } else { "" },
        e1 = n(p.rotation()),
        e2 = if p.flag("invert_behavior") { "" } else { "-" },
    );

    let silkscreen_back = format!(
        r####"
    (fp_text user "{e0}" (at -3.5 0 {e1}) (layer "B.SilkS") {e2}
      (effects (font (size 1 1) (thickness 0.15)) (justify mirror))
    )
    (fp_text user "ON" (at 0 {e3}5 {e4}) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify mirror))
    )
    (fp_text user "OFF" (at 0 {e5}5 {e4}) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify mirror))
    )
    (fp_line (start -1.425 1.4) (end -1.425 1.6) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 0.415 3.45) (end -0.375 3.45) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -0.375 -3.45) (end 0.415 -3.45) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -1.425 -1.6) (end -1.425 0.1) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 1.425 -2.85) (end 1.425 2.85) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    "####,
        e0 = p.reference(),
        e1 = n(90.0 + p.rotation()),
        e2 = p.ref_hide(),
        e3 = if p.flag("invert_behavior") { "-" } else { "" },
        e4 = n(p.rotation()),
        e5 = if p.flag("invert_behavior") { "" } else { "-" },
    );

    let courtyard_front = String::from(
        r####"
    (fp_line (start 1.795 4.4) (end -2.755 4.4) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 1.795 1.65) (end 1.795 4.4) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 3.095 1.65) (end 1.795 1.65) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 3.095 -1.65) (end 3.095 1.65) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 1.795 -1.65) (end 3.095 -1.65) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 1.795 -4.4) (end 1.795 -1.65) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -2.755 -4.4) (end 1.795 -4.4) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -2.755 4.4) (end -2.755 -4.4) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    "####,
    );

    let courtyard_back = String::from(
        r####"
    (fp_line (start -2.755 -4.4) (end -2.755 4.4) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 3.095 1.65) (end 3.095 -1.65) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 1.795 1.65) (end 3.095 1.65) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 1.795 -4.4) (end -2.755 -4.4) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 1.795 -1.65) (end 1.795 -4.4) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 3.095 -1.65) (end 1.795 -1.65) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 1.795 4.4) (end 1.795 1.65) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -2.755 4.4) (end 1.795 4.4) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    "####,
    );

    let pads_front = format!(
        r####"
    (fp_line (start -1.305 -3.35) (end -1.305 3.35) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 1.295 -3.35) (end -1.305 -3.35) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 1.295 3.35) (end 1.295 -3.35) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -1.305 3.35) (end 1.295 3.35) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.595 0.1) (end 1.295 0.1) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.645 0.15) (end 2.595 0.1) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.845 0.35) (end 2.645 0.15) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.845 1.2) (end 2.845 0.35) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.645 1.4) (end 2.845 1.2) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 1.345 1.4) (end 2.645 1.4) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (pad "" smd rect (at 1.125 -3.65 {e0}) (size 1 0.8) (layers "F.Cu" "F.Paste" "F.Mask"))
    (pad "" smd rect (at -1.085 -3.65 {e0}) (size 1 0.8) (layers "F.Cu" "F.Paste" "F.Mask"))
    (pad "" smd rect (at -1.085 3.65 {e0}) (size 1 0.8) (layers "F.Cu" "F.Paste" "F.Mask"))
    (pad "" smd rect (at 1.125 3.65 {e0}) (size 1 0.8) (layers "F.Cu" "F.Paste" "F.Mask"))
    (pad "1" smd rect (at -1.735 2.25 {e0}) (size 0.7 1.5) (layers "F.Cu" "F.Paste" "F.Mask") {e1})
    (pad "2" smd rect (at -1.735 -0.75 {e0}) (size 0.7 1.5) (layers "F.Cu" "F.Paste" "F.Mask") {e2})
    (pad "3" smd rect (at -1.735 -2.25 {e0}) (size 0.7 1.5) (layers "F.Cu" "F.Paste" "F.Mask") {e3})
    "####,
        e0 = n(90.0 + p.rotation()),
        e1 = if p.flag("invert_behavior") {
            String::new()
        } else {
            p.net("from").to_string()
        },
        e2 = p.net("to"),
        e3 = if p.flag("invert_behavior") {
            p.net("from").to_string()
        } else {
            String::new()
        },
    );

    let pads_back = format!(
        r####"
    (fp_line (start 2.595 -0.1) (end 1.295 -0.1) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -1.305 3.35) (end -1.305 -3.35) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.645 -0.15) (end 2.595 -0.1) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.845 -1.2) (end 2.845 -0.35) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 1.345 -1.4) (end 2.645 -1.4) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.845 -0.35) (end 2.645 -0.15) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.645 -1.4) (end 2.845 -1.2) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 1.295 -3.35) (end 1.295 3.35) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 1.295 3.35) (end -1.305 3.35) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -1.305 -3.35) (end 1.295 -3.35) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (pad "" smd rect (at -1.085 -3.65 {e0}) (size 1 0.8) (layers "B.Cu" "B.Paste" "B.Mask"))
    (pad "" smd rect (at 1.125 -3.65 {e0}) (size 1 0.8) (layers "B.Cu" "B.Paste" "B.Mask"))
    (pad "" smd rect (at -1.085 3.65 {e0}) (size 1 0.8) (layers "B.Cu" "B.Paste" "B.Mask"))
    (pad "" smd rect (at 1.125 3.65 {e0}) (size 1 0.8) (layers "B.Cu" "B.Paste" "B.Mask"))
    (pad "1" smd rect (at -1.735 -2.25 {e0}) (size 0.7 1.5) (layers "B.Cu" "B.Paste" "B.Mask") {e1})
    (pad "2" smd rect (at -1.735 0.75 {e0}) (size 0.7 1.5) (layers "B.Cu" "B.Paste" "B.Mask") {e2})
    (pad "3" smd rect (at -1.735 2.25 {e0}) (size 0.7 1.5) (layers "B.Cu" "B.Paste" "B.Mask") {e3})
    "####,
        e0 = n(270.0 + p.rotation()),
        e1 = if p.flag("invert_behavior") {
            p.net("from").to_string()
        } else {
            String::new()
        },
        e2 = p.net("to"),
        e3 = if p.flag("invert_behavior") {
            String::new()
        } else {
            p.net("from").to_string()
        },
    );

    let common_end = format!(
        r####"
    (pad "" np_thru_hole circle (at 0.025 -1.5 {e0}) (size 0.9 0.9) (drill 0.9) (layers "*.Cu" "*.Mask"))
    (pad "" np_thru_hole circle (at 0.025 1.5 {e0}) (size 0.9 0.9) (drill 0.9) (layers "*.Cu" "*.Mask"))
  )
    "####,
        e0 = n(90.0 + p.rotation()),
    );

    let switch_3dmodel = format!(
        r####"
    (model {e0}
      (offset (xyz {e1} {e2} {e3}))
      (scale (xyz {e4} {e5} {e6}))
      (rotate (xyz {e7} {e8} {e9}))
    )
    "####,
        e0 = json_string(p.text("switch_3dmodel_filename")),
        e1 = n(p.component("switch_3dmodel_xyz_offset", 0)),
        e2 = n(p.component("switch_3dmodel_xyz_offset", 1)),
        e3 = n(p.component("switch_3dmodel_xyz_offset", 2)),
        e4 = n(p.component("switch_3dmodel_xyz_scale", 0)),
        e5 = n(p.component("switch_3dmodel_xyz_scale", 1)),
        e6 = n(p.component("switch_3dmodel_xyz_scale", 2)),
        e7 = n(p.component("switch_3dmodel_xyz_rotation", 0)),
        e8 = n(p.component("switch_3dmodel_xyz_rotation", 1)),
        e9 = n(p.component("switch_3dmodel_xyz_rotation", 2)),
    );

    let (side, reversible) = (p.side(), p.flag("reversible"));
    let mut out = common_start;
    if side == "F" || reversible {
        out += &pads_front;
        if p.flag("include_silkscreen") {
            out += &silkscreen_front;
        }
        if p.flag("include_courtyard") {
            out += &courtyard_front;
        }
    }
    if side == "B" || reversible {
        out += &pads_back;
        if p.flag("include_silkscreen") {
            out += &silkscreen_back;
        }
        if p.flag("include_courtyard") {
            out += &courtyard_back;
        }
    }
    if !p.text("switch_3dmodel_filename").is_empty() {
        out += &switch_3dmodel;
    }
    out += &common_end;
    Ok(out)
}

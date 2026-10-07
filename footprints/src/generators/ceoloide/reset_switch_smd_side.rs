//! `ceoloide/reset_switch_smd_side`
//!
//! Ported from `reset_switch_smd_side.js` of ceoloide/ergogen-footprints.
//! Copyright (c) 2023 Marco Massarelli
//! SPDX-License-Identifier: MIT
//! Author: @ceoloide
use crate::Result;
use crate::context::RenderContext;
use crate::generators::util::json_string;
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 12] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "RST"),
    ParamSpec::boolean("reversible", false),
    ParamSpec::boolean("include_bosses", false),
    ParamSpec::boolean("include_silkscreen", true),
    ParamSpec::boolean("include_courtyard", false),
    ParamSpec::string(
        "reset_switch_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/kicad/Panasonic_EVQPUJ_EVQPUA.step",
    ),
    ParamSpec::array("reset_switch_3dmodel_xyz_offset", "[0,0,0]"),
    ParamSpec::array("reset_switch_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("reset_switch_3dmodel_xyz_scale", "[1,1,1]"),
    ParamSpec::net_default("from", "GND"),
    ParamSpec::net_default("to", "RST"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "ceoloide/reset_switch_smd_side",
    display_name: "reset switch smd side",
    kind: PartKind::Passive,
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
    // The bundled boss-compatible switch replaces the default model when bosses are on.
    const DEFAULT_MODEL: &str = "${KIPRJMOD}/models/boardstudio/kicad/Panasonic_EVQPUJ_EVQPUA.step";
    const BOSS_MODEL: &str = "${KIPRJMOD}/models/boardstudio/kicad/Panasonic_EVQPUL_EVQPUC.step";
    let saved_model = p.text("reset_switch_3dmodel_filename");
    let model_filename = if p.flag("include_bosses") && saved_model == DEFAULT_MODEL {
        BOSS_MODEL
    } else {
        saved_model
    };
    // Use the selected side's model frame; an explicit rotation wins.
    let model_rotation = p.vec3("reset_switch_3dmodel_xyz_rotation")?.unwrap_or([
        0.0,
        0.0,
        if p.side() == "B" { 180.0 } else { 0.0 },
    ]);
    for name in [
        "reset_switch_3dmodel_xyz_offset",
        "reset_switch_3dmodel_xyz_scale",
    ] {
        p.vec3(name)?;
    }

    let common_start = format!(
        r####"
  (footprint "ceoloide:reset_switch_smd_side"
    (layer "{e0}.Cu")
    {e1}
    (property "Reference" "{e2}"
      (at 0 0 {e3})
      (layer "{e0}.SilkS")
      {e4}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (attr smd)
    (fp_line (start -2.35 -1.75) (end -2.35 1.75) (stroke (width 0.1) (type solid)) (layer "Dwgs.User"))
    (fp_line (start -2.35 -1.75) (end 2.35 -1.75) (stroke (width 0.1) (type solid)) (layer "Dwgs.User"))
    (fp_line (start -2.35 1.75) (end 2.35 1.75) (stroke (width 0.1) (type solid)) (layer "Dwgs.User"))
    (fp_line (start -1.3 -2.75) (end -1.3 -1.75) (stroke (width 0.1) (type solid)) (layer "Dwgs.User"))
    (fp_line (start -1.3 -2.75) (end 1.3 -2.75) (stroke (width 0.1) (type solid)) (layer "Dwgs.User"))
    (fp_line (start 1.3 -2.75) (end 1.3 -1.75) (stroke (width 0.1) (type solid)) (layer "Dwgs.User"))
    (fp_line (start 2.35 -1.75) (end 2.35 1.75) (stroke (width 0.1) (type solid)) (layer "Dwgs.User"))
    "####,
        e0 = p.side(),
        e1 = p.at(),
        e2 = p.reference(),
        e3 = n(p.rotation()),
        e4 = p.ref_hide(),
    );

    let silkscreen_front = String::from(
        r####"
    (fp_line (start -2.35 -1.5) (end -2.35 -1.75) (stroke (width 0.1) (type solid)) (layer "F.SilkS"))
    (fp_line (start -2.35 1.5) (end -2.35 1.75) (stroke (width 0.1) (type solid)) (layer "F.SilkS"))
    (fp_line (start -2.1 -1.75) (end -2.35 -1.75) (stroke (width 0.1) (type solid)) (layer "F.SilkS"))
    (fp_line (start -2.1 1.75) (end -2.35 1.75) (stroke (width 0.1) (type solid)) (layer "F.SilkS"))
    (fp_line (start 2.1 -1.75) (end 2.35 -1.75) (stroke (width 0.1) (type solid)) (layer "F.SilkS"))
    (fp_line (start 2.1 1.75) (end 2.35 1.75) (stroke (width 0.1) (type solid)) (layer "F.SilkS"))
    (fp_line (start 2.35 -1.5) (end 2.35 -1.75) (stroke (width 0.1) (type solid)) (layer "F.SilkS"))
    (fp_line (start 2.35 1.5) (end 2.35 1.75) (stroke (width 0.1) (type solid)) (layer "F.SilkS"))
    "####,
    );

    let silkscreen_back = String::from(
        r####"
    (fp_line (start -2.35 -1.5) (end -2.35 -1.75) (stroke (width 0.1) (type solid)) (layer "B.SilkS"))
    (fp_line (start -2.35 1.5) (end -2.35 1.75) (stroke (width 0.1) (type solid)) (layer "B.SilkS"))
    (fp_line (start -2.1 -1.75) (end -2.35 -1.75) (stroke (width 0.1) (type solid)) (layer "B.SilkS"))
    (fp_line (start -2.1 1.75) (end -2.35 1.75) (stroke (width 0.1) (type solid)) (layer "B.SilkS"))
    (fp_line (start 2.1 -1.75) (end 2.35 -1.75) (stroke (width 0.1) (type solid)) (layer "B.SilkS"))
    (fp_line (start 2.1 1.75) (end 2.35 1.75) (stroke (width 0.1) (type solid)) (layer "B.SilkS"))
    (fp_line (start 2.35 -1.5) (end 2.35 -1.75) (stroke (width 0.1) (type solid)) (layer "B.SilkS"))
    (fp_line (start 2.35 1.5) (end 2.35 1.75) (stroke (width 0.1) (type solid)) (layer "B.SilkS"))
    "####,
    );

    let pads_front = format!(
        r####"
    (pad "1" smd rect (at 2.625 -0.85 {e0}) (size 1.55 1) (layers "F.Cu" "F.Paste" "F.Mask") {e1})
    (pad "2" smd rect (at 2.625 0.85 {e0}) (size 1.55 1) (layers "F.Cu" "F.Paste" "F.Mask") {e2})
    (pad "3" smd rect (at -2.625 -0.85 {e0}) (size 1.55 1) (layers "F.Cu" "F.Paste" "F.Mask") {e1})
    (pad "4" smd rect (at -2.625 0.85 {e0}) (size 1.55 1) (layers "F.Cu" "F.Paste" "F.Mask") {e2})
    "####,
        e0 = n(180.0 + p.rotation()),
        e1 = p.net("from"),
        e2 = p.net("to"),
    );

    let pads_back = format!(
        r####"
    (pad "1" smd rect (at -2.625 -0.85 {e0}) (size 1.55 1) (layers "B.Cu" "B.Paste" "B.Mask") {e1})
    (pad "2" smd rect (at -2.625 0.85 {e0}) (size 1.55 1) (layers "B.Cu" "B.Paste" "B.Mask") {e2})
    (pad "3" smd rect (at 2.625 -0.85 {e0}) (size 1.55 1) (layers "B.Cu" "B.Paste" "B.Mask") {e1})
    (pad "4" smd rect (at 2.625 0.85 {e0}) (size 1.55 1) (layers "B.Cu" "B.Paste" "B.Mask") {e2})
    "####,
        e0 = n(180.0 + p.rotation()),
        e1 = p.net("from"),
        e2 = p.net("to"),
    );

    let courtyard_front = String::from(
        r####"
    (fp_rect (start 2.36 1.75) (end -2.36 -1.75) (stroke (width 0.05) (type solid)) (fill none) (layer "F.CrtYd"))
    "####,
    );

    let courtyard_back = String::from(
        r####"
    (fp_rect (start 2.36 1.75) (end -2.36 -1.75) (stroke (width 0.05) (type solid)) (fill none) (layer "B.CrtYd"))
    "####,
    );

    let bosses = format!(
        r####"
    (pad "" np_thru_hole circle (at 0 -1.375 {e0}) (size 0.75 0.75) (drill 0.75) (layers "*.Cu" "*.Mask"))
    (pad "" np_thru_hole circle (at 0 1.375 {e0}) (size 0.75 0.75) (drill 0.75) (layers "*.Cu" "*.Mask"))
    "####,
        e0 = n(180.0 + p.rotation()),
    );

    let reset_switch_3dmodel = format!(
        r####"
    (model {e0}
      (offset (xyz {e1} {e2} {e3}))
      (scale (xyz {e4} {e5} {e6}))
      (rotate (xyz {e7} {e8} {e9}))
    )
    "####,
        e0 = json_string(model_filename),
        e1 = n(p.component("reset_switch_3dmodel_xyz_offset", 0)),
        e2 = n(p.component("reset_switch_3dmodel_xyz_offset", 1)),
        e3 = n(p.component("reset_switch_3dmodel_xyz_offset", 2)),
        e4 = n(p.component("reset_switch_3dmodel_xyz_scale", 0)),
        e5 = n(p.component("reset_switch_3dmodel_xyz_scale", 1)),
        e6 = n(p.component("reset_switch_3dmodel_xyz_scale", 2)),
        e7 = n(model_rotation[0]),
        e8 = n(model_rotation[1]),
        e9 = n(model_rotation[2]),
    );

    let common_end = String::from(
        r####"
  )
    "####,
    );

    let (side, reversible) = (p.side(), p.flag("reversible"));
    let mut out = common_start;
    if p.flag("include_bosses") {
        out += &bosses;
    }
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
    if !model_filename.is_empty() {
        out += &reset_switch_3dmodel;
    }
    out += &common_end;
    Ok(out)
}

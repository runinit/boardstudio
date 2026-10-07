//! `infused-kim/switch_power`
//!
//! Ported from `switch_power.js` of infused-kim/kb_ergogen_fp.
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0 (see the vendored LICENSE)
//! Author: @infused-kim
use crate::Result;
use crate::context::RenderContext;
use crate::generators::util::{ModelDefaults, ModelParams, infused_model};
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 10] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "SW"),
    ParamSpec::boolean("reverse", false),
    ParamSpec::net_default("from", "BAT_P"),
    ParamSpec::net_default("to", "RAW"),
    ParamSpec::string(
        "switch_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/Switch_Power.step",
    ),
    ParamSpec::string("switch_3dmodel_side", ""),
    ParamSpec::array("switch_3dmodel_xyz_scale", "[]"),
    ParamSpec::array("switch_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("switch_3dmodel_xyz_offset", "[]"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "infused-kim/switch_power",
    display_name: "switch power",
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
    let shared_1 = format!(
        r####"
        (module power_switch (layer F.Cu) (tedit 644556E6)
          {e0}
          (attr smd)

      "####,
        e0 = p.at()
    );

    let front_switch = format!(
        r####"
          (fp_text reference "{e0}" (at -3.6 0 {e1}) (layer F.SilkS) {e2}
            (effects (font (size 1 1) (thickness 0.15)))
          )

          (fp_line (start 0.415 -3.45) (end -0.375 -3.45) (layer F.SilkS) (width 0.12))
          (fp_line (start -0.375 3.45) (end 0.415 3.45) (layer F.SilkS) (width 0.12))
          (fp_line (start -1.425 1.6) (end -1.425 -0.1) (layer F.SilkS) (width 0.12))
          (fp_line (start 1.425 2.85) (end 1.425 -2.85) (layer F.SilkS) (width 0.12))
          (fp_line (start 1.795 4.4) (end -2.755 4.4) (layer F.CrtYd) (width 0.05))
          (fp_line (start 1.795 1.65) (end 1.795 4.4) (layer F.CrtYd) (width 0.05))
          (fp_line (start 3.095 1.65) (end 1.795 1.65) (layer F.CrtYd) (width 0.05))
          (fp_line (start 3.095 -1.65) (end 3.095 1.65) (layer F.CrtYd) (width 0.05))
          (fp_line (start 1.795 -1.65) (end 3.095 -1.65) (layer F.CrtYd) (width 0.05))
          (fp_line (start 1.795 -4.4) (end 1.795 -1.65) (layer F.CrtYd) (width 0.05))
          (fp_line (start -2.755 -4.4) (end 1.795 -4.4) (layer F.CrtYd) (width 0.05))
          (fp_line (start -2.755 4.4) (end -2.755 -4.4) (layer F.CrtYd) (width 0.05))
          (fp_line (start -1.425 -1.4) (end -1.425 -1.6) (layer F.SilkS) (width 0.12))
          (fp_line (start -1.305 -3.35) (end -1.305 3.35) (layer F.Fab) (width 0.1))
          (fp_line (start 1.295 -3.35) (end -1.305 -3.35) (layer F.Fab) (width 0.1))
          (fp_line (start 1.295 3.35) (end 1.295 -3.35) (layer F.Fab) (width 0.1))
          (fp_line (start -1.305 3.35) (end 1.295 3.35) (layer F.Fab) (width 0.1))
          (fp_line (start 2.595 0.1) (end 1.295 0.1) (layer F.Fab) (width 0.1))
          (fp_line (start 2.645 0.15) (end 2.595 0.1) (layer F.Fab) (width 0.1))
          (fp_line (start 2.845 0.35) (end 2.645 0.15) (layer F.Fab) (width 0.1))
          (fp_line (start 2.845 1.2) (end 2.845 0.35) (layer F.Fab) (width 0.1))
          (fp_line (start 2.645 1.4) (end 2.845 1.2) (layer F.Fab) (width 0.1))
          (fp_line (start 1.345 1.4) (end 2.645 1.4) (layer F.Fab) (width 0.1))

          (pad "" smd rect (at 1.125 -3.65 {e3}) (size 1 0.8) (layers F.Cu F.Paste F.Mask))
          (pad "" smd rect (at -1.085 -3.65 {e3}) (size 1 0.8) (layers F.Cu F.Paste F.Mask))
          (pad "" smd rect (at -1.085 3.65 {e3}) (size 1 0.8) (layers F.Cu F.Paste F.Mask))
          (pad 1 smd rect (at -1.735 2.25 {e3}) (size 0.7 1.5) (layers F.Cu F.Paste F.Mask))
          (pad 2 smd rect (at -1.735 -0.75 {e3}) (size 0.7 1.5) (layers F.Cu F.Paste F.Mask) {e4})
          (pad 3 smd rect (at -1.735 -2.25 {e3}) (size 0.7 1.5) (layers F.Cu F.Paste F.Mask) {e5})
          (pad "" smd rect (at 1.125 3.65 {e3}) (size 1 0.8) (layers F.Cu F.Paste F.Mask))

      "####,
        e0 = p.reference(),
        e1 = n(-90.0 + p.rotation()),
        e2 = p.ref_hide(),
        e3 = n(90.0 + p.rotation()),
        e4 = p.net("from"),
        e5 = p.net("to")
    );

    let back_switch = format!(
        r####"
        {e0}
        (fp_text user "{e1}" (at -3.5 0 {e2}) (layer B.SilkS) {e3}
        (effects (font (size 1 1) (thickness 0.15)) (justify mirror))
        )
        (fp_line (start 2.595 -0.1) (end 1.295 -0.1) (layer B.Fab) (width 0.1))
        (fp_line (start -1.305 3.35) (end -1.305 -3.35) (layer B.Fab) (width 0.1))
        (fp_line (start 2.645 -0.15) (end 2.595 -0.1) (layer B.Fab) (width 0.1))
        (fp_line (start -1.425 1.4) (end -1.425 1.6) (layer B.SilkS) (width 0.12))
        (fp_line (start 0.415 3.45) (end -0.375 3.45) (layer B.SilkS) (width 0.12))
        (fp_line (start -0.375 -3.45) (end 0.415 -3.45) (layer B.SilkS) (width 0.12))
        (fp_line (start -1.425 -1.6) (end -1.425 0.1) (layer B.SilkS) (width 0.12))
        (fp_line (start 1.425 -2.85) (end 1.425 2.85) (layer B.SilkS) (width 0.12))
        (fp_line (start 1.795 4.4) (end 1.795 1.65) (layer B.CrtYd) (width 0.05))
        (fp_line (start -2.755 4.4) (end 1.795 4.4) (layer B.CrtYd) (width 0.05))
        (fp_line (start 2.845 -1.2) (end 2.845 -0.35) (layer B.Fab) (width 0.1))
        (fp_line (start 1.345 -1.4) (end 2.645 -1.4) (layer B.Fab) (width 0.1))
        (fp_line (start 1.795 -4.4) (end -2.755 -4.4) (layer B.CrtYd) (width 0.05))
        (fp_line (start 1.795 -1.65) (end 1.795 -4.4) (layer B.CrtYd) (width 0.05))
        (fp_line (start 3.095 -1.65) (end 1.795 -1.65) (layer B.CrtYd) (width 0.05))
        (fp_line (start 2.845 -0.35) (end 2.645 -0.15) (layer B.Fab) (width 0.1))
        (fp_line (start 2.645 -1.4) (end 2.845 -1.2) (layer B.Fab) (width 0.1))
        (fp_line (start 1.295 -3.35) (end 1.295 3.35) (layer B.Fab) (width 0.1))
        (fp_line (start 1.295 3.35) (end -1.305 3.35) (layer B.Fab) (width 0.1))
        (fp_line (start -1.305 -3.35) (end 1.295 -3.35) (layer B.Fab) (width 0.1))
        (fp_line (start -2.755 -4.4) (end -2.755 4.4) (layer B.CrtYd) (width 0.05))
        (fp_line (start 3.095 1.65) (end 3.095 -1.65) (layer B.CrtYd) (width 0.05))
        (fp_line (start 1.795 1.65) (end 3.095 1.65) (layer B.CrtYd) (width 0.05))
        (pad "" smd rect (at -1.085 -3.65 {e4}) (size 1 0.8) (layers B.Cu B.Paste B.Mask))
        (pad "" smd rect (at 1.125 -3.65 {e4}) (size 1 0.8) (layers B.Cu B.Paste B.Mask))
        (pad 4 smd rect (at -1.735 2.25 {e4}) (size 0.7 1.5) (layers B.Cu B.Paste B.Mask))
        (pad "" smd rect (at -1.085 3.65 {e4}) (size 1 0.8) (layers B.Cu B.Paste B.Mask))
        (pad 5 smd rect (at -1.735 0.75 {e4}) (size 0.7 1.5) (layers B.Cu B.Paste B.Mask) {e5})
        (pad 6 smd rect (at -1.735 -2.25 {e4}) (size 0.7 1.5) (layers B.Cu B.Paste B.Mask) {e6})
        (pad "" smd rect (at 1.125 3.65 {e4}) (size 1 0.8) (layers B.Cu B.Paste B.Mask))
        "####,
        e0 = "",
        e1 = p.reference(),
        e2 = n(90.0 + p.rotation()),
        e3 = p.ref_hide(),
        e4 = n(270.0 + p.rotation()),
        e5 = p.net("from"),
        e6 = p.net("to")
    );

    let shared_2 = format!(
        r####"
          (pad "" np_thru_hole circle (at 0.025 -1.5 {e0}) (size 0.9 0.9) (drill 0.9) (layers *.Cu *.Mask))
          (pad "" np_thru_hole circle (at 0.025 1.5 {e0}) (size 0.9 0.9) (drill 0.9) (layers *.Cu *.Mask))
        "####,
        e0 = n(90.0 + p.rotation())
    );

    let final_add0 = String::from(
        r####"
          )
        "####,
    );

    let (side, reverse) = (p.side(), p.flag("reverse"));
    let mut out = shared_1;
    if side == "F" || reverse {
        out += &front_switch;
    }
    if side == "B" || reverse {
        out += &back_switch;
    }
    out += &shared_2;
    out += &infused_model(
        p,
        ModelParams {
            filename: "switch_3dmodel_filename",
            scale: "switch_3dmodel_xyz_scale",
            rotation: "switch_3dmodel_xyz_rotation",
            offset: "switch_3dmodel_xyz_offset",
            side: "switch_3dmodel_side",
        },
        ModelDefaults {
            default_side: "B",
            rotation_f: [-90.0, 0.0, -90.0],
            offset_f: [0.0, 0.0, 0.0],
            rotation_b: [90.0, 0.0, 90.0],
            offset_b: [0.0, 0.0, -1.6],
        },
    )?;
    out += &final_add0;
    Ok(out)
}

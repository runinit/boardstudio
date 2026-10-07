//! `infused-kim/switch_reset`
//!
//! Ported from `switch_reset.js` of infused-kim/kb_ergogen_fp.
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
    ParamSpec::net_default("from", "GND"),
    ParamSpec::net_default("to", "RST"),
    ParamSpec::string(
        "switch_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/Switch_Reset.step",
    ),
    ParamSpec::string("switch_3dmodel_side", ""),
    ParamSpec::array("switch_3dmodel_xyz_scale", "[]"),
    ParamSpec::array("switch_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("switch_3dmodel_xyz_offset", "[]"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "infused-kim/switch_reset",
    display_name: "switch reset",
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
    let top = format!(
        r####"
        (module sw_reset_side (layer F.Cu) (tedit 64473C6F)
          {e0}
          (attr smd)

          (fp_text reference "{e1}" (at 0 3.5 {e2}) (layer {e3}.SilkS) {e4}
            (effects (font (size 1 1) (thickness 0.15)))
          )
      "####,
        e0 = p.at(),
        e1 = p.reference(),
        e2 = n(p.rotation()),
        e3 = p.side(),
        e4 = p.ref_hide()
    );

    let front = format!(
        r####"
          (fp_line (start 1.7 2.75) (end -1.7 2.75) (layer F.CrtYd) (width 0.05))
          (fp_line (start -1.7 2.75) (end -1.7 -2.75) (layer F.CrtYd) (width 0.05))
          (fp_line (start 2.1 0.85) (end 2.1 -0.85) (layer F.Fab) (width 0.1))
          (fp_line (start 1.7 -1.1) (end 2.35 -1.1) (layer F.CrtYd) (width 0.05))
          (fp_line (start -1.7 -2.75) (end 1.7 -2.75) (layer F.CrtYd) (width 0.05))
          (fp_line (start 1.45 -1.75) (end 1.45 1.75) (layer F.Fab) (width 0.1))
          (fp_line (start 1.7 1.1) (end 1.7 2.75) (layer F.CrtYd) (width 0.05))
          (fp_line (start 2.35 1.1) (end 1.7 1.1) (layer F.CrtYd) (width 0.05))
          (fp_line (start 1.7 -2.75) (end 1.7 -1.1) (layer F.CrtYd) (width 0.05))
          (fp_line (start 1.55 -1.75) (end 1.55 1.75) (layer F.SilkS) (width 0.12))
          (fp_line (start 2.1 -0.85) (end 1.45 -0.85) (layer F.Fab) (width 0.1))
          (fp_line (start 2.35 -1.1) (end 2.35 1.1) (layer F.CrtYd) (width 0.05))
          (fp_line (start 2.1 0.85) (end 1.45 0.85) (layer F.Fab) (width 0.1))
          (fp_line (start -1.55 1.75) (end -1.55 -1.75) (layer F.SilkS) (width 0.12))
          (fp_line (start 1.45 1.75) (end -1.4 1.75) (layer F.Fab) (width 0.1))
          (fp_line (start -1.45 1.75) (end -1.45 -1.75) (layer F.Fab) (width 0.1))
          (fp_line (start -1.45 -1.75) (end 1.45 -1.75) (layer F.Fab) (width 0.1))

          (pad 1 smd rect (at -0.72 -1.8 {e0}) (size 1.4 1.05) (layers F.Cu F.Paste F.Mask) {e1})

          (pad 1 smd rect (at -0.72 1.8 {e0}) (size 1.4 1.05) (layers F.Cu F.Paste F.Mask) {e1})
          (pad 2 smd rect (at 0.72 -1.8 {e0}) (size 1.4 1.05) (layers F.Cu F.Paste F.Mask) {e2})
          (pad 2 smd rect (at 0.72 1.8 {e0}) (size 1.4 1.05) (layers F.Cu F.Paste F.Mask) {e2})
      "####,
        e0 = n(90.0 + p.rotation()),
        e1 = p.net("from"),
        e2 = p.net("to")
    );

    let back = format!(
        r####"
      (fp_line (start -1.45 1.75) (end 1.45 1.75) (layer B.Fab) (width 0.1))
      (fp_line (start 1.45 1.75) (end 1.45 -1.75) (layer B.Fab) (width 0.1))
      (fp_line (start 1.7 -1.1) (end 1.7 -2.75) (layer B.CrtYd) (width 0.05))
      (fp_line (start 2.35 -1.1) (end 1.7 -1.1) (layer B.CrtYd) (width 0.05))
      (fp_line (start 1.7 2.75) (end 1.7 1.1) (layer B.CrtYd) (width 0.05))
      (fp_line (start 1.55 1.75) (end 1.55 -1.75) (layer B.SilkS) (width 0.12))
      (fp_line (start 2.1 0.85) (end 1.45 0.85) (layer B.Fab) (width 0.1))
      (fp_line (start 2.35 1.1) (end 2.35 -1.1) (layer B.CrtYd) (width 0.05))
      (fp_line (start 2.1 -0.85) (end 1.45 -0.85) (layer B.Fab) (width 0.1))
      (fp_line (start -1.55 -1.75) (end -1.55 1.75) (layer B.SilkS) (width 0.12))
      (fp_line (start 1.45 -1.75) (end -1.4 -1.75) (layer B.Fab) (width 0.1))
      (fp_line (start -1.45 -1.75) (end -1.45 1.75) (layer B.Fab) (width 0.1))
      (fp_line (start 1.7 -2.75) (end -1.7 -2.75) (layer B.CrtYd) (width 0.05))
      (fp_line (start -1.7 -2.75) (end -1.7 2.75) (layer B.CrtYd) (width 0.05))
      (fp_line (start 2.1 -0.85) (end 2.1 0.85) (layer B.Fab) (width 0.1))
      (fp_line (start 1.7 1.1) (end 2.35 1.1) (layer B.CrtYd) (width 0.05))
      (fp_line (start -1.7 2.75) (end 1.7 2.75) (layer B.CrtYd) (width 0.05))
      (pad 1 smd rect (at -0.72 -1.8 {e0}) (size 1.4 1.05) (layers B.Cu B.Paste B.Mask) {e1})
      (pad 2 smd rect (at 0.72 1.8 {e0}) (size 1.4 1.05) (layers B.Cu B.Paste B.Mask) {e2})
      (pad 2 smd rect (at 0.72 -1.8 {e0}) (size 1.4 1.05) (layers B.Cu B.Paste B.Mask) {e2})
      (pad 1 smd rect (at -0.72 1.8 {e0}) (size 1.4 1.05) (layers B.Cu B.Paste B.Mask) {e1})
      (fp_text user {e3} (at 0 3.5 {e4}) (layer B.SilkS) {e5}
        (effects (font (size 1 1) (thickness 0.15)) (justify mirror))
      )
      "####,
        e0 = n(270.0 + p.rotation()),
        e1 = p.net("from"),
        e2 = p.net("to"),
        e3 = p.reference(),
        e4 = n(p.rotation()),
        e5 = p.ref_hide()
    );

    let bottom = String::from(
        r####"
        )
      "####,
    );

    let (side, reverse) = (p.side(), p.flag("reverse"));
    let mut out = top;
    if side == "F" || reverse {
        out += &front;
    }
    if side == "B" || reverse {
        out += &back;
    }
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
    out += &bottom;
    Ok(out)
}

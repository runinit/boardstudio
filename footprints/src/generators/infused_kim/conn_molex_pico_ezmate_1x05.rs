//! `infused-kim/conn_molex_pico_ezmate_1x05`
//!
//! Ported from `conn_molex_pico_ezmate_1x05.js` of infused-kim/kb_ergogen_fp.
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0 (see the vendored LICENSE)
//! Author: @infused-kim
use crate::Result;
use crate::context::RenderContext;
use crate::generators::util::{ModelDefaults, ModelParams, infused_model};
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 18] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "CONN"),
    ParamSpec::boolean("reverse", false),
    ParamSpec::net_default("pad_1", "CONN_1"),
    ParamSpec::net_default("pad_2", "CONN_2"),
    ParamSpec::net_default("pad_3", "CONN_3"),
    ParamSpec::net_default("pad_4", "CONN_4"),
    ParamSpec::net_default("pad_5", "CONN_5"),
    ParamSpec::string(
        "cable_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/Molex_Ezmate_Pico_Cable_5pin.step",
    ),
    ParamSpec::string("cable_3dmodel_side", ""),
    ParamSpec::array("cable_3dmodel_xyz_scale", "[]"),
    ParamSpec::array("cable_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("cable_3dmodel_xyz_offset", "[]"),
    ParamSpec::string(
        "socket_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/Molex_Ezmate_Pico_Socket_5pin.step",
    ),
    ParamSpec::string("socket_3dmodel_side", ""),
    ParamSpec::array("socket_3dmodel_xyz_scale", "[]"),
    ParamSpec::array("socket_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("socket_3dmodel_xyz_offset", "[]"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "infused-kim/conn_molex_pico_ezmate_1x05",
    display_name: "conn molex pico ezmate 1x05",
    kind: PartKind::Connector,
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
      (module conn_molex_pico_ezmate_1x05 (layer F.Cu) (tedit 644602FB)
        {e0}
        (attr smd)

      "####,
        e0 = p.at()
    );

    let front = format!(
        r####"
        (fp_text reference "{e0}" (at 0 0.25 {e1}) (layer F.SilkS) {e2}
          (effects (font (size 1 1) (thickness 0.15)))
        )
        (fp_line (start -2.96 -2.09) (end -2.96 -2.3) (layer F.SilkS) (width 0.12))
        (fp_line (start 4.01 1.24) (end 4.01 -2.09) (layer F.SilkS) (width 0.12))
        (fp_line (start 4.01 -2.09) (end 2.96 -2.09) (layer F.SilkS) (width 0.12))
        (fp_line (start -2.94 2.63) (end -2.44 2.63) (layer F.SilkS) (width 0.12))
        (fp_line (start -2.44 2.63) (end -2.14 2.13) (layer F.SilkS) (width 0.12))
        (fp_line (start -2.14 2.13) (end 2.14 2.13) (layer F.SilkS) (width 0.12))
        (fp_line (start 2.14 2.13) (end 2.44 2.63) (layer F.SilkS) (width 0.12))
        (fp_line (start 2.44 2.63) (end 2.94 2.63) (layer F.SilkS) (width 0.12))
        (fp_line (start -3.9 2.52) (end -2.55 2.52) (layer F.Fab) (width 0.1))
        (fp_line (start -2.55 2.52) (end -2.25 2.02) (layer F.Fab) (width 0.1))
        (fp_line (start -2.25 2.02) (end 2.25 2.02) (layer F.Fab) (width 0.1))
        (fp_line (start 2.25 2.02) (end 2.55 2.52) (layer F.Fab) (width 0.1))
        (fp_line (start 2.55 2.52) (end 3.9 2.52) (layer F.Fab) (width 0.1))
        (fp_line (start -3.9 -1.98) (end -3.9 2.52) (layer F.Fab) (width 0.1))
        (fp_line (start 3.9 -1.98) (end 3.9 2.52) (layer F.Fab) (width 0.1))
        (fp_line (start -4.4 -2.8) (end -4.4 3.02) (layer F.CrtYd) (width 0.05))
        (fp_line (start -4.4 3.02) (end 4.4 3.02) (layer F.CrtYd) (width 0.05))
        (fp_line (start 4.4 3.02) (end 4.4 -2.8) (layer F.CrtYd) (width 0.05))
        (fp_line (start 4.4 -2.8) (end -4.4 -2.8) (layer F.CrtYd) (width 0.05))
        (fp_line (start -2.9 -1.98) (end -2.4 -1.272893) (layer F.Fab) (width 0.1))
        (fp_line (start -2.4 -1.272893) (end -1.9 -1.98) (layer F.Fab) (width 0.1))
        (fp_line (start -3.9 -1.98) (end 3.9 -1.98) (layer F.Fab) (width 0.1))
        (fp_line (start -4.01 1.24) (end -4.01 -2.09) (layer F.SilkS) (width 0.12))
        (fp_line (start -4.01 -2.09) (end -2.96 -2.09) (layer F.SilkS) (width 0.12))
        (fp_text user %R (at 0 0.27 {e1}) (layer F.Fab)
          (effects (font (size 1 1) (thickness 0.15)))
        )
        (pad 1 smd roundrect (at -2.4 -1.875 {e1}) (size 0.6 0.85) (layers F.Cu F.Paste F.Mask) (roundrect_rratio 0.25) {e3})
        (pad MP smd roundrect (at 3.55 1.9 {e1}) (size 0.7 0.8) (layers F.Cu F.Paste F.Mask) (roundrect_rratio 0.25))
        (pad MP smd roundrect (at -3.55 1.9 {e1}) (size 0.7 0.8) (layers F.Cu F.Paste F.Mask) (roundrect_rratio 0.25))
        (pad 2 smd roundrect (at -1.2 -1.875 {e1}) (size 0.6 0.85) (layers F.Cu F.Paste F.Mask) (roundrect_rratio 0.25) {e4})
        (pad 4 smd roundrect (at 1.2 -1.875 {e1}) (size 0.6 0.85) (layers F.Cu F.Paste F.Mask) (roundrect_rratio 0.25) {e5})
        (pad 5 smd roundrect (at 2.4 -1.875 {e1}) (size 0.6 0.85) (layers F.Cu F.Paste F.Mask) (roundrect_rratio 0.25) {e6})
        (pad 3 smd roundrect (at 0 -1.875 {e1}) (size 0.6 0.85) (layers F.Cu F.Paste F.Mask) (roundrect_rratio 0.25) {e7})
      "####,
        e0 = p.reference(),
        e1 = n(p.rotation()),
        e2 = p.ref_hide(),
        e3 = p.net("pad_1"),
        e4 = p.net("pad_2"),
        e5 = p.net("pad_4"),
        e6 = p.net("pad_5"),
        e7 = p.net("pad_3")
    );

    let back = format!(
        r####"
        (fp_text user %R (at 0 0.27 {e0}) (layer B.Fab)
          (effects (font (size 1 1) (thickness 0.15)) (justify mirror))
        )
        (fp_line (start 2.94 2.63) (end 2.44 2.63) (layer B.SilkS) (width 0.12))
        (fp_line (start -2.14 2.13) (end -2.44 2.63) (layer B.SilkS) (width 0.12))
        (fp_line (start -2.44 2.63) (end -2.94 2.63) (layer B.SilkS) (width 0.12))
        (fp_line (start 3.9 2.52) (end 2.55 2.52) (layer B.Fab) (width 0.1))
        (fp_line (start 2.55 2.52) (end 2.25 2.02) (layer B.Fab) (width 0.1))
        (fp_line (start 2.25 2.02) (end -2.25 2.02) (layer B.Fab) (width 0.1))
        (fp_line (start -2.25 2.02) (end -2.55 2.52) (layer B.Fab) (width 0.1))
        (fp_line (start 2.44 2.63) (end 2.14 2.13) (layer B.SilkS) (width 0.12))
        (fp_line (start 2.14 2.13) (end -2.14 2.13) (layer B.SilkS) (width 0.12))
        (fp_line (start 2.96 -2.09) (end 2.96 -2.3) (layer B.SilkS) (width 0.12))
        (fp_line (start -4.01 -2.09) (end -2.96 -2.09) (layer B.SilkS) (width 0.12))
        (fp_line (start -2.55 2.52) (end -3.9 2.52) (layer B.Fab) (width 0.1))
        (fp_line (start 3.9 -1.98) (end 3.9 2.52) (layer B.Fab) (width 0.1))
        (fp_line (start -3.9 -1.98) (end -3.9 2.52) (layer B.Fab) (width 0.1))
        (fp_line (start 4.4 -2.8) (end 4.4 3.02) (layer B.CrtYd) (width 0.05))
        (fp_line (start 4.4 3.02) (end -4.4 3.02) (layer B.CrtYd) (width 0.05))
        (fp_line (start -4.4 3.02) (end -4.4 -2.8) (layer B.CrtYd) (width 0.05))
        (fp_line (start -4.4 -2.8) (end 4.4 -2.8) (layer B.CrtYd) (width 0.05))
        (fp_line (start 2.9 -1.98) (end 2.4 -1.272893) (layer B.Fab) (width 0.1))
        (fp_line (start 2.4 -1.272893) (end 1.9 -1.98) (layer B.Fab) (width 0.1))
        (fp_line (start 3.9 -1.98) (end -3.9 -1.98) (layer B.Fab) (width 0.1))
        (fp_line (start 4.01 1.24) (end 4.01 -2.09) (layer B.SilkS) (width 0.12))
        (fp_line (start 4.01 -2.09) (end 2.96 -2.09) (layer B.SilkS) (width 0.12))
        (fp_line (start -4.01 1.24) (end -4.01 -2.09) (layer B.SilkS) (width 0.12))
        (pad 3 smd roundrect (at 0 -1.875 {e0}) (size 0.6 0.85) (layers B.Cu B.Paste B.Mask) (roundrect_rratio 0.25) {e1})
        (pad MP smd roundrect (at 3.55 1.9 {e0}) (size 0.7 0.8) (layers B.Cu B.Paste B.Mask) (roundrect_rratio 0.25))
        (pad 4 smd roundrect (at -1.2 -1.875 {e0}) (size 0.6 0.85) (layers B.Cu B.Paste B.Mask) (roundrect_rratio 0.25) {e2})
        (pad 5 smd roundrect (at -2.4 -1.875 {e0}) (size 0.6 0.85) (layers B.Cu B.Paste B.Mask) (roundrect_rratio 0.25) {e3})
        (pad 2 smd roundrect (at 1.2 -1.875 {e0}) (size 0.6 0.85) (layers B.Cu B.Paste B.Mask) (roundrect_rratio 0.25) {e4})
        (pad 1 smd roundrect (at 2.4 -1.875 {e0}) (size 0.6 0.85) (layers B.Cu B.Paste B.Mask) (roundrect_rratio 0.25) {e5})
        (pad MP smd roundrect (at -3.55 1.9 {e0}) (size 0.7 0.8) (layers B.Cu B.Paste B.Mask) (roundrect_rratio 0.25))
      "####,
        e0 = n(180.0 + p.rotation()),
        e1 = p.net("pad_3"),
        e2 = p.net("pad_4"),
        e3 = p.net("pad_5"),
        e4 = p.net("pad_2"),
        e5 = p.net("pad_1")
    );

    let all_3d_models = format!(
        r####"
          {e0}
          {e1}
      "####,
        e0 = infused_model(
            p,
            ModelParams {
                filename: "cable_3dmodel_filename",
                scale: "cable_3dmodel_xyz_scale",
                rotation: "cable_3dmodel_xyz_rotation",
                offset: "cable_3dmodel_xyz_offset",
                side: "cable_3dmodel_side"
            },
            ModelDefaults {
                default_side: "F",
                rotation_f: [0.0, 0.0, 0.0],
                offset_f: [0.0, 0.0, 0.8],
                rotation_b: [0.0, 180.0, 0.0],
                offset_b: [0.0, 0.0, -(1.6 + 0.8)]
            }
        )?,
        e1 = infused_model(
            p,
            ModelParams {
                filename: "socket_3dmodel_filename",
                scale: "socket_3dmodel_xyz_scale",
                rotation: "socket_3dmodel_xyz_rotation",
                offset: "socket_3dmodel_xyz_offset",
                side: "socket_3dmodel_side"
            },
            ModelDefaults {
                default_side: "F",
                rotation_f: [-90.0, 0.0, 0.0],
                offset_f: [0.0, 0.0, 1.4],
                rotation_b: [-90.0, 180.0, 0.0],
                offset_b: [0.0, 0.0, -3.0]
            }
        )?
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
    out += &all_3d_models;
    out += &bottom;
    Ok(out)
}

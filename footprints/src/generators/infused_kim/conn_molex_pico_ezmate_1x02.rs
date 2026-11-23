//! `infused-kim/conn_molex_pico_ezmate_1x02`
//!
//! Ported from `conn_molex_pico_ezmate_1x02.js` of infused-kim/kb_ergogen_fp.
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0 (see the vendored LICENSE)
//! Author: @infused-kim
use super::molex;
use crate::Result;
use crate::context::RenderContext;
use crate::generators::util::atom_or_string;
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 15] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "CONN"),
    ParamSpec::boolean("reverse", false),
    ParamSpec::net_default("pad_1", "RAW"),
    ParamSpec::net_default("pad_2", "GND"),
    ParamSpec::string(
        "cable_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/Molex_Ezmate_Pico_Cable_2pin.step",
    ),
    ParamSpec::string("cable_3dmodel_side", ""),
    ParamSpec::array("cable_3dmodel_xyz_scale", "[]"),
    ParamSpec::array("cable_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("cable_3dmodel_xyz_offset", "[]"),
    ParamSpec::string(
        "socket_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/Molex_Ezmate_Pico_Socket_2pin.step",
    ),
    ParamSpec::string("socket_3dmodel_side", ""),
    ParamSpec::array("socket_3dmodel_xyz_scale", "[]"),
    ParamSpec::array("socket_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("socket_3dmodel_xyz_offset", "[]"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "infused-kim/conn_molex_pico_ezmate_1x02",
    display_name: "conn molex pico ezmate 1x02",
    kind: PartKind::Connector,
    matrix_terminals: None,
    keycap_parameters: None,
    parameters: &PARAMS,
    license: License {
        spdx: "CC-BY-NC-SA-4.0",
        author: "@infused-kim",
    },
    body: crate::registry::Body::Text(body),
};

fn body(p: &RenderContext<'_>) -> Result<String> {
    let top = format!(
        r####"
        (module conn_molex_pico_ezmate_1x02 (layer F.Cu) (tedit 6445F610)
          {e0}
          (attr smd)

      "####,
        e0 = p.at()
    );

    let front = format!(
        r####"
        (fp_text reference {e0} (at 0.1 3.9 {e1}) (layer F.SilkS) {e2}
          (effects (font (size 1 1) (thickness 0.15)))
        )
        (fp_line (start 0.64 2.63) (end 1.14 2.63) (layer F.SilkS) (width 0.12))
        (fp_line (start 0.34 2.13) (end 0.64 2.63) (layer F.SilkS) (width 0.12))
        (fp_line (start -0.34 2.13) (end 0.34 2.13) (layer F.SilkS) (width 0.12))
        (fp_line (start -0.64 2.63) (end -0.34 2.13) (layer F.SilkS) (width 0.12))
        (fp_line (start -0.45 2.02) (end 0.45 2.02) (layer F.Fab) (width 0.1))
        (fp_line (start -0.75 2.52) (end -0.45 2.02) (layer F.Fab) (width 0.1))
        (fp_line (start -2.1 2.52) (end -0.75 2.52) (layer F.Fab) (width 0.1))
        (fp_line (start -1.16 -2.09) (end -1.16 -2.3) (layer F.SilkS) (width 0.12))
        (fp_line (start -2.21 -2.09) (end -1.16 -2.09) (layer F.SilkS) (width 0.12))
        (fp_line (start -2.21 1.24) (end -2.21 -2.09) (layer F.SilkS) (width 0.12))
        (fp_line (start -2.1 -1.98) (end 2.1 -1.98) (layer F.Fab) (width 0.1))
        (fp_line (start -1.14 2.63) (end -0.64 2.63) (layer F.SilkS) (width 0.12))
        (fp_line (start 2.21 -2.09) (end 1.16 -2.09) (layer F.SilkS) (width 0.12))
        (fp_line (start 2.21 1.24) (end 2.21 -2.09) (layer F.SilkS) (width 0.12))
        (fp_line (start -0.6 -1.272893) (end -0.1 -1.98) (layer F.Fab) (width 0.1))
        (fp_line (start -1.1 -1.98) (end -0.6 -1.272893) (layer F.Fab) (width 0.1))
        (fp_line (start 2.6 -2.8) (end -2.6 -2.8) (layer F.CrtYd) (width 0.05))
        (fp_line (start -2.6 -2.8) (end -2.6 3.02) (layer F.CrtYd) (width 0.05))
        (fp_line (start 2.1 -1.98) (end 2.1 2.52) (layer F.Fab) (width 0.1))
        (fp_line (start -2.1 -1.98) (end -2.1 2.52) (layer F.Fab) (width 0.1))
        (fp_line (start 0.75 2.52) (end 2.1 2.52) (layer F.Fab) (width 0.1))
        (fp_line (start 0.45 2.02) (end 0.75 2.52) (layer F.Fab) (width 0.1))
        (fp_line (start 2.6 3.02) (end 2.6 -2.8) (layer F.CrtYd) (width 0.05))
        (fp_line (start -2.6 3.02) (end 2.6 3.02) (layer F.CrtYd) (width 0.05))
        (pad MP smd roundrect (at 1.75 1.9 {e1}) (size 0.7 0.8) (layers F.Cu F.Paste F.Mask) (roundrect_rratio 0.25))
        (pad MP smd roundrect (at -1.75 1.9 {e1}) (size 0.7 0.8) (layers F.Cu F.Paste F.Mask) (roundrect_rratio 0.25))
        (pad 2 smd roundrect (at 0.6 -1.875 {e1}) (size 0.6 0.85) (layers F.Cu F.Paste F.Mask) (roundrect_rratio 0.25) {e3})
        (pad 1 smd roundrect (at -0.6 -1.875 {e1}) (size 0.6 0.85) (layers F.Cu F.Paste F.Mask) (roundrect_rratio 0.25) {e4})
      "####,
        e0 = atom_or_string(p.reference()),
        e1 = n(p.rotation()),
        e2 = p.ref_hide(),
        e3 = p.net("pad_2"),
        e4 = p.net("pad_1")
    );

    let back = format!(
        r####"
        (fp_line (start -0.34 2.13) (end -0.64 2.63) (layer B.SilkS) (width 0.12))
        (fp_line (start -2.6 3.02) (end -2.6 -2.8) (layer B.CrtYd) (width 0.05))
        (fp_line (start 2.6 3.02) (end -2.6 3.02) (layer B.CrtYd) (width 0.05))
        (fp_line (start -0.64 2.63) (end -1.14 2.63) (layer B.SilkS) (width 0.12))
        (fp_line (start 1.16 -2.09) (end 1.16 -2.3) (layer B.SilkS) (width 0.12))
        (fp_line (start 2.21 -2.09) (end 1.16 -2.09) (layer B.SilkS) (width 0.12))
        (fp_line (start 2.21 1.24) (end 2.21 -2.09) (layer B.SilkS) (width 0.12))
        (fp_line (start 2.1 -1.98) (end -2.1 -1.98) (layer B.Fab) (width 0.1))
        (fp_line (start 1.14 2.63) (end 0.64 2.63) (layer B.SilkS) (width 0.12))
        (fp_line (start -2.21 -2.09) (end -1.16 -2.09) (layer B.SilkS) (width 0.12))
        (fp_line (start -2.21 1.24) (end -2.21 -2.09) (layer B.SilkS) (width 0.12))
        (fp_line (start 0.6 -1.272893) (end 0.1 -1.98) (layer B.Fab) (width 0.1))
        (fp_line (start 1.1 -1.98) (end 0.6 -1.272893) (layer B.Fab) (width 0.1))
        (fp_line (start -2.6 -2.8) (end 2.6 -2.8) (layer B.CrtYd) (width 0.05))
        (fp_line (start 2.6 -2.8) (end 2.6 3.02) (layer B.CrtYd) (width 0.05))
        (fp_line (start -2.1 -1.98) (end -2.1 2.52) (layer B.Fab) (width 0.1))
        (fp_line (start 2.1 -1.98) (end 2.1 2.52) (layer B.Fab) (width 0.1))
        (fp_line (start -0.75 2.52) (end -2.1 2.52) (layer B.Fab) (width 0.1))
        (fp_line (start -0.45 2.02) (end -0.75 2.52) (layer B.Fab) (width 0.1))
        (fp_line (start 0.64 2.63) (end 0.34 2.13) (layer B.SilkS) (width 0.12))
        (fp_line (start 0.45 2.02) (end -0.45 2.02) (layer B.Fab) (width 0.1))
        (fp_line (start 0.75 2.52) (end 0.45 2.02) (layer B.Fab) (width 0.1))
        (fp_line (start 2.1 2.52) (end 0.75 2.52) (layer B.Fab) (width 0.1))
        (fp_line (start 0.34 2.13) (end -0.34 2.13) (layer B.SilkS) (width 0.12))
        (pad MP smd roundrect (at 1.75 1.9 {e0}) (size 0.7 0.8) (layers B.Cu B.Paste B.Mask) (roundrect_rratio 0.25))
        (pad 2 smd roundrect (at -0.6 -1.875 {e0}) (size 0.6 0.85) (layers B.Cu B.Paste B.Mask) (roundrect_rratio 0.25) {e1})
        (pad 1 smd roundrect (at 0.6 -1.875 {e0}) (size 0.6 0.85) (layers B.Cu B.Paste B.Mask) (roundrect_rratio 0.25) {e2})
        (pad MP smd roundrect (at -1.75 1.9 {e0}) (size 0.7 0.8) (layers B.Cu B.Paste B.Mask) (roundrect_rratio 0.25))
      "####,
        e0 = n(180.0 + p.rotation()),
        e1 = p.net("pad_2"),
        e2 = p.net("pad_1")
    );

    let (cable, socket) = molex::molex_models(p)?;
    let all_3d_models = format!(
        r####"
        {e0}
        {e1}
    "####,
        e0 = cable,
        e1 = socket
    );

    Ok(molex::assemble_sides(p, top, front, back, all_3d_models))
}

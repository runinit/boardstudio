//! `infused-kim/nice_view`
//!
//! Ported from `nice_view.js` of infused-kim/kb_ergogen_fp.
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0 (see the vendored LICENSE)
//! Author: @infused-kim
use crate::Result;
use crate::context::RenderContext;
use crate::generators::util::{ModelDefaults, ModelParams, infused_model};
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 24] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "DISP"),
    ParamSpec::boolean("reverse", false),
    ParamSpec::number("pcb_thickness", 1.6),
    ParamSpec::net_default("MOSI", "MOSI"),
    ParamSpec::net_default("SCK", "SCK"),
    ParamSpec::net_default("VCC", "VCC"),
    ParamSpec::net_default("GND", "GND"),
    ParamSpec::net_default("CS", "CS"),
    ParamSpec::boolean("show_labels", true),
    ParamSpec::boolean("jumpers_at_bottom", false),
    ParamSpec::string("display_3dmodel_side", ""),
    ParamSpec::string(
        "display_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/Nice_View.step",
    ),
    ParamSpec::array("display_3dmodel_xyz_scale", "[]"),
    ParamSpec::array("display_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("display_3dmodel_xyz_offset", "[]"),
    ParamSpec::string(
        "header_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/PinHeader_2.54mm_1x-5.step",
    ),
    ParamSpec::array("header_3dmodel_xyz_scale", "[]"),
    ParamSpec::array("header_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("header_3dmodel_xyz_offset", "[]"),
    ParamSpec::string(
        "socket_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/kicad/PinSocket_1x05_P2.54mm_Vertical.step",
    ),
    ParamSpec::array("socket_3dmodel_xyz_scale", "[]"),
    ParamSpec::array("socket_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("socket_3dmodel_xyz_offset", "[]"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "infused-kim/nice_view",
    display_name: "nice view",
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
    let (reverse, side) = (p.flag("reverse"), p.side());
    let dst_nets: Vec<String> = ["MOSI", "SCK", "VCC", "GND", "CS"]
        .iter()
        .map(|name| p.net(name).to_string())
        .collect();
    let local_nets: Vec<String> = vec![
        p.local_net("1")?.to_string(),
        p.local_net("2")?.to_string(),
        p.net("VCC").to_string(),
        p.local_net("4")?.to_string(),
        p.local_net("5")?.to_string(),
    ];
    let socket_nets: Vec<String> = if reverse {
        local_nets.clone()
    } else if side == "B" {
        dst_nets.iter().rev().cloned().collect()
    } else {
        dst_nets.clone()
    };

    let mut jumpers_offset = 0.0;
    let mut labels_offset = 0.0;
    let mut label_vcc_offset = 0.0;
    let mut jumpers_front_top = dst_nets.clone();
    let mut jumpers_front_bottom = local_nets.clone();
    let mut jumpers_back_top = dst_nets.clone();
    let mut jumpers_back_bottom: Vec<String> = local_nets.iter().rev().cloned().collect();
    if p.flag("jumpers_at_bottom") {
        jumpers_offset = 5.7;
        labels_offset = jumpers_offset + 2.0 + 1.0 + 0.1;
        label_vcc_offset = 4.85;
        jumpers_front_top = local_nets.clone();
        jumpers_front_bottom = dst_nets.clone();
        jumpers_back_top = local_nets.iter().rev().cloned().collect();
        jumpers_back_bottom = dst_nets.clone();
    }
    // Match the standard 8.5 mm socket while retaining the model-side override.
    let socket_height = 8.5;
    let display_height = socket_height + 1.8;
    let header_height = socket_height - 2.0;
    let top = format!(
        r####"
      (module nice!view (layer F.Cu) (tedit 6448AF5B)
        {e0}
        (attr virtual)
        (fp_text reference "{e1}" (at 0 20 {e2}) (layer {e3}.SilkS) {e4}
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
        (fp_line (start -6.5 -18) (end 6.5 -18) (layer F.Fab) (width 0.15))
        (fp_line (start 6.5 18) (end -6.5 18) (layer F.Fab) (width 0.15))
        (fp_line (start -7 17.5) (end -7 -17.5) (layer F.Fab) (width 0.15))
        (fp_line (start 7 17.5) (end 7 -17.5) (layer F.Fab) (width 0.15))
        (fp_line (start -6.41 15.37) (end -6.41 18.03) (layer F.SilkS) (width 0.12))
        (fp_line (start 6.41 18.03) (end -6.41 18.03) (layer F.SilkS) (width 0.12))
        (fp_line (start 6.88 14.9) (end 6.88 18.45) (layer F.CrtYd) (width 0.15))
        (fp_line (start 6.88 18.45) (end -6.82 18.45) (layer F.CrtYd) (width 0.15))
        (fp_line (start -6.82 18.45) (end -6.82 14.9) (layer F.CrtYd) (width 0.15))
        (fp_line (start -6.82 14.9) (end 6.88 14.9) (layer F.CrtYd) (width 0.15))
        (fp_line (start 6.41 15.37) (end 6.41 18.03) (layer F.SilkS) (width 0.12))
        (fp_line (start 6.41 15.37) (end -6.41 15.37) (layer F.SilkS) (width 0.12))
        (fp_arc (start -6.5 17.5) (end -7 17.5) (angle -90) (layer F.Fab) (width 0.15))
        (fp_arc (start 6.5 17.5) (end 6.5 18) (angle -90) (layer F.Fab) (width 0.15))
        (fp_arc (start 6.5 -17.5) (end 6.5 -18) (angle 90) (layer F.Fab) (width 0.15))
        (fp_arc (start -6.5 -17.5) (end -6.5 -18) (angle -90) (layer F.Fab) (width 0.15))
        (fp_text user %R (at 0 20 {e0}) (layer F.Fab)
          (effects (font (size 1 1) (thickness 0.15)))
        )

    "####,
        e0 = n(p.rotation())
    );

    let front_jumpers = format!(
        r####"
        (fp_line (start 5.93 {e0}) (end 5.93 {e1}) (layer F.Fab) (width 0.15))
        (fp_line (start -5.93 {e1}) (end -5.93 {e0}) (layer F.Fab) (width 0.15))
        (fp_line (start -5.93 {e0}) (end -4.23 {e0}) (layer F.Fab) (width 0.15))
        (fp_line (start -4.23 {e1}) (end -5.93 {e1}) (layer F.Fab) (width 0.15))
        (fp_line (start -4.23 {e0}) (end -4.23 {e1}) (layer F.Fab) (width 0.15))
        (fp_line (start -3.39 {e1}) (end -3.39 {e0}) (layer F.Fab) (width 0.15))
        (fp_line (start -3.39 {e0}) (end -1.69 {e0}) (layer F.Fab) (width 0.15))
        (fp_line (start -1.69 {e1}) (end -3.39 {e1}) (layer F.Fab) (width 0.15))
        (fp_line (start -1.69 {e0}) (end -1.69 {e1}) (layer F.Fab) (width 0.15))
        (fp_line (start 3.39 {e0}) (end 3.39 {e1}) (layer F.Fab) (width 0.15))
        (fp_line (start 3.39 {e1}) (end 1.69 {e1}) (layer F.Fab) (width 0.15))
        (fp_line (start 1.69 {e1}) (end 1.69 {e0}) (layer F.Fab) (width 0.15))
        (fp_line (start 1.69 {e0}) (end 3.39 {e0}) (layer F.Fab) (width 0.15))
        (fp_line (start 5.93 {e1}) (end 4.23 {e1}) (layer F.Fab) (width 0.15))
        (fp_line (start 4.23 {e1}) (end 4.23 {e0}) (layer F.Fab) (width 0.15))
        (fp_line (start 4.23 {e0}) (end 5.93 {e0}) (layer F.Fab) (width 0.15))

        (pad 14 smd rect (at -5.08 {e2} {e3}) (size 0.6 1.2) (layers F.Cu F.Mask) {e4})
        (pad 15 smd rect (at -2.54 {e2} {e3}) (size 0.6 1.2) (layers F.Cu F.Mask) {e5})
        (pad 16 smd rect (at 2.54 {e2} {e3}) (size 0.6 1.2) (layers F.Cu F.Mask) {e6})
        (pad 17 smd rect (at 5.08 {e2} {e3}) (size 0.6 1.2) (layers F.Cu F.Mask) {e7})

        (pad 10 smd rect (at -5.08 {e8} {e3}) (size 0.6 1.2) (layers F.Cu F.Mask) {e9})
        (pad 11 smd rect (at -2.54 {e8} {e3}) (size 0.6 1.2) (layers F.Cu F.Mask) {e10})
        (pad 12 smd rect (at 2.54 {e8} {e3}) (size 0.6 1.2) (layers F.Cu F.Mask) {e11})
        (pad 13 smd rect (at 5.08 {e8} {e3}) (size 0.6 1.2) (layers F.Cu F.Mask) {e12})
    "####,
        e0 = n(12.9 + jumpers_offset),
        e1 = n(14.9 + jumpers_offset),
        e2 = n(13.45 + jumpers_offset),
        e3 = n(90.0 + p.rotation()),
        e4 = jumpers_front_top[0],
        e5 = jumpers_front_top[1],
        e6 = jumpers_front_top[3],
        e7 = jumpers_front_top[4],
        e8 = n(14.35 + jumpers_offset),
        e9 = jumpers_front_bottom[0],
        e10 = jumpers_front_bottom[1],
        e11 = jumpers_front_bottom[3],
        e12 = jumpers_front_bottom[4]
    );

    let back = String::from(
        r####"
        (fp_line (start 6.41 15.37) (end 6.41 18.03) (layer B.SilkS) (width 0.12))
        (fp_line (start 6.41 15.37) (end -6.41 15.37) (layer B.SilkS) (width 0.12))
        (fp_line (start 6.41 18.03) (end -6.41 18.03) (layer B.SilkS) (width 0.12))
        (fp_line (start 6.88 14.9) (end 6.88 18.45) (layer B.CrtYd) (width 0.15))
        (fp_line (start 6.88 18.45) (end -6.82 18.45) (layer B.CrtYd) (width 0.15))
        (fp_line (start -6.82 18.45) (end -6.82 14.9) (layer B.CrtYd) (width 0.15))
        (fp_line (start -6.82 14.9) (end 6.88 14.9) (layer B.CrtYd) (width 0.15))
        (fp_line (start -6.41 15.37) (end -6.41 18.03) (layer B.SilkS) (width 0.12))
        (fp_line (start -6.5 18) (end 6.5 18) (layer B.Fab) (width 0.15))
        (fp_line (start 7 -17.5) (end 7 17.5) (layer B.Fab) (width 0.15))
        (fp_line (start 6.5 -18) (end -6.5 -18) (layer B.Fab) (width 0.15))
        (fp_line (start -7 -17.5) (end -7 17.5) (layer B.Fab) (width 0.15))
        (fp_arc (start -6.5 -17.5) (end -7 -17.5) (angle 90) (layer B.Fab) (width 0.15))
        (fp_arc (start 6.5 -17.5) (end 6.5 -18) (angle 90) (layer B.Fab) (width 0.15))
        (fp_arc (start 6.5 17.5) (end 6.5 18) (angle -90) (layer B.Fab) (width 0.15))
        (fp_arc (start -6.5 17.5) (end -6.5 18) (angle 90) (layer B.Fab) (width 0.15))
    "####,
    );

    let back_jumpers = format!(
        r####"
        (fp_line (start -5.93 {e0}) (end -5.93 {e1}) (layer B.Fab) (width 0.15))
        (fp_line (start -5.93 {e1}) (end -4.23 {e1}) (layer B.Fab) (width 0.15))
        (fp_line (start -4.23 {e0}) (end -5.93 {e0}) (layer B.Fab) (width 0.15))
        (fp_line (start -4.23 {e1}) (end -4.23 {e0}) (layer B.Fab) (width 0.15))
        (fp_line (start -3.39 {e1}) (end -1.69 {e1}) (layer B.Fab) (width 0.15))
        (fp_line (start -1.69 {e0}) (end -3.39 {e0}) (layer B.Fab) (width 0.15))
        (fp_line (start 4.23 {e1}) (end 5.93 {e1}) (layer B.Fab) (width 0.15))
        (fp_line (start 5.93 {e1}) (end 5.93 {e0}) (layer B.Fab) (width 0.15))
        (fp_line (start 3.39 {e0}) (end 1.69 {e0}) (layer B.Fab) (width 0.15))
        (fp_line (start -1.69 {e1}) (end -1.69 {e0}) (layer B.Fab) (width 0.15))
        (fp_line (start -3.39 {e0}) (end -3.39 {e1}) (layer B.Fab) (width 0.15))
        (fp_line (start 1.69 {e0}) (end 1.69 {e1}) (layer B.Fab) (width 0.15))
        (fp_line (start 1.69 {e1}) (end 3.39 {e1}) (layer B.Fab) (width 0.15))
        (fp_line (start 3.39 {e1}) (end 3.39 {e0}) (layer B.Fab) (width 0.15))
        (fp_line (start 5.93 {e0}) (end 4.23 {e0}) (layer B.Fab) (width 0.15))
        (fp_line (start 4.23 {e0}) (end 4.23 {e1}) (layer B.Fab) (width 0.15))

        (pad 24 smd rect (at 5.08 {e2} {e3}) (size 0.6 1.2) (layers B.Cu B.Mask) {e4})
        (pad 25 smd rect (at 2.54 {e2} {e3}) (size 0.6 1.2) (layers B.Cu B.Mask) {e5})
        (pad 26 smd rect (at -2.54 {e2} {e3}) (size 0.6 1.2) (layers B.Cu B.Mask) {e6})
        (pad 27 smd rect (at -5.08 {e2} {e3}) (size 0.6 1.2) (layers B.Cu B.Mask) {e7})

        (pad 20 smd rect (at 5.08 {e8} {e3}) (size 0.6 1.2) (layers B.Cu B.Mask) {e9})
        (pad 21 smd rect (at 2.54 {e8} {e3}) (size 0.6 1.2) (layers B.Cu B.Mask) {e10})
        (pad 22 smd rect (at -2.54 {e8} {e3}) (size 0.6 1.2) (layers B.Cu B.Mask) {e11})
        (pad 23 smd rect (at -5.08 {e8} {e3}) (size 0.6 1.2) (layers B.Cu B.Mask) {e12})
    "####,
        e0 = n(12.9 + jumpers_offset),
        e1 = n(14.9 + jumpers_offset),
        e2 = n(13.45 + jumpers_offset),
        e3 = n(270.0 + p.rotation()),
        e4 = jumpers_back_top[0],
        e5 = jumpers_back_top[1],
        e6 = jumpers_back_top[3],
        e7 = jumpers_back_top[4],
        e8 = n(14.35 + jumpers_offset),
        e9 = jumpers_back_bottom[0],
        e10 = jumpers_back_bottom[1],
        e11 = jumpers_back_bottom[3],
        e12 = jumpers_back_bottom[4]
    );

    let labels = format!(
        r####"
        (fp_text user DA (at -5.08 {e0} {e1}) (layer F.SilkS)
          (effects (font (size 1 0.7) (thickness 0.1)))
        )
        (fp_text user CS (at 5.12 {e0} {e1}) (layer F.SilkS)
          (effects (font (size 1 0.7) (thickness 0.1)))
        )
        (fp_text user GND (at 2.62 {e0} {e1}) (layer F.SilkS)
          (effects (font (size 1 0.7) (thickness 0.1)))
        )
        (fp_text user VCC (at 0.15 {e2} {e1}) (layer F.SilkS)
          (effects (font (size 1 0.7) (thickness 0.1)))
        )
        (fp_text user CL (at -2.48 {e0} {e1}) (layer F.SilkS)
          (effects (font (size 1 0.7) (thickness 0.1)))
        )
        (fp_text user CS (at -4.98 {e0} {e1}) (layer B.SilkS)
          (effects (font (size 1 0.7) (thickness 0.1)) (justify mirror))
        )
        (fp_text user VCC (at 0.15 {e2} {e1}) (layer B.SilkS)
          (effects (font (size 1 0.7) (thickness 0.1)) (justify mirror))
        )
        (fp_text user DA (at 5.22 {e0} {e1}) (layer B.SilkS)
          (effects (font (size 1 0.7) (thickness 0.1)) (justify mirror))
        )
        (fp_text user CL (at 2.72 {e0} {e1}) (layer B.SilkS)
          (effects (font (size 1 0.7) (thickness 0.1)) (justify mirror))
        )
        (fp_text user GND (at -2.38 {e0} {e1}) (layer B.SilkS)
          (effects (font (size 1 0.7) (thickness 0.1)) (justify mirror))
        )
    "####,
        e0 = n(12.5 + labels_offset),
        e1 = n(p.rotation()),
        e2 = n(14.4 + label_vcc_offset)
    );

    let bottom = format!(
        r####"
      (pad 1 thru_hole oval (at -5.08 16.7 {e0}) (size 1.7 1.7) (drill 1) (layers *.Cu *.Mask) {e1})
      (pad 2 thru_hole oval (at -2.54 16.7 {e0}) (size 1.7 1.7) (drill 1) (layers *.Cu *.Mask) {e2})
      (pad 3 thru_hole oval (at 0 16.7 {e0}) (size 1.7 1.7) (drill 1) (layers *.Cu *.Mask) {e3})
      (pad 4 thru_hole oval (at 2.54 16.7 {e0}) (size 1.7 1.7) (drill 1) (layers *.Cu *.Mask) {e4})
      (pad 5 thru_hole circle (at 5.08 16.7 {e0}) (size 1.7 1.7) (drill 1) (layers *.Cu *.Mask) {e5})

      (fp_line (start 5.4 13.4) (end 5.4 -11.9) (layer Dwgs.User) (width 0.15))
      (fp_line (start -5.4 13.4) (end -5.4 -11.9) (layer Dwgs.User) (width 0.15))
      (fp_line (start 5.4 -11.9) (end -5.4 -11.9) (layer Dwgs.User) (width 0.15))
      (fp_line (start -5.4 13.4) (end 5.4 13.4) (layer Dwgs.User) (width 0.15))
    )
    "####,
        e0 = n(270.0 + p.rotation()),
        e1 = socket_nets[0],
        e2 = socket_nets[1],
        e3 = socket_nets[2],
        e4 = socket_nets[3],
        e5 = socket_nets[4]
    );

    let final_add0 = format!(
        r####"
      {e0}
      {e1}
      {e2}
    "####,
        e0 = infused_model(
            p,
            ModelParams {
                filename: "display_3dmodel_filename",
                scale: "display_3dmodel_xyz_scale",
                rotation: "display_3dmodel_xyz_rotation",
                offset: "display_3dmodel_xyz_offset",
                side: "display_3dmodel_side"
            },
            ModelDefaults {
                default_side: "F",
                rotation_f: [0.0, 0.0, 0.0],
                offset_f: [-7.0, -18.0, display_height],
                rotation_b: [0.0, 180.0, 0.0],
                offset_b: [7.0, -18.0, -p.number("pcb_thickness") - display_height]
            }
        )?,
        e1 = infused_model(
            p,
            ModelParams {
                filename: "header_3dmodel_filename",
                scale: "header_3dmodel_xyz_scale",
                rotation: "header_3dmodel_xyz_rotation",
                offset: "header_3dmodel_xyz_offset",
                side: "display_3dmodel_side"
            },
            ModelDefaults {
                default_side: "F",
                rotation_f: [0.0, 0.0, -90.0],
                offset_f: [0.0124, -16.732, header_height],
                rotation_b: [180.0, 0.0, -90.0],
                offset_b: [-0.0124, -16.732, -p.number("pcb_thickness") - header_height]
            }
        )?,
        e2 = infused_model(
            p,
            ModelParams {
                filename: "socket_3dmodel_filename",
                scale: "socket_3dmodel_xyz_scale",
                rotation: "socket_3dmodel_xyz_rotation",
                offset: "socket_3dmodel_xyz_offset",
                side: "display_3dmodel_side"
            },
            ModelDefaults {
                default_side: "F",
                rotation_f: [0.0, 0.0, -90.0],
                offset_f: [-5.08, -16.7, 0.0],
                rotation_b: [180.0, 0.0, -90.0],
                offset_b: [5.08, -16.7, -p.number("pcb_thickness")]
            }
        )?
    );

    let mut out = top;
    if side == "F" || reverse {
        out += &front;
    }
    if side == "B" || reverse {
        out += &back;
    }
    if reverse {
        out += &front_jumpers;
        out += &back_jumpers;
        if p.flag("show_labels") {
            out += &labels;
        }
    }
    out += &final_add0;
    out += &bottom;
    Ok(out)
}

//! `ceoloide/display_nice_view`
//!
//! Ported from `display_nice_view.js` of ceoloide/ergogen-footprints.
//! Copyright (c) 2023 Marco Massarelli
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0
//! Author: @infused-kim + @ceoloide improvements
use crate::Result;
use crate::context::NetRef;
use crate::context::RenderContext;
use crate::generators::util::json_string;
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 28] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "DISP"),
    ParamSpec::boolean("reversible", false),
    ParamSpec::boolean("include_traces", true),
    ParamSpec::number("gnd_trace_width", 0.25),
    ParamSpec::number("signal_trace_width", 0.25),
    ParamSpec::boolean("invert_jumpers_position", false),
    ParamSpec::boolean("invert_labels_position", false),
    ParamSpec::boolean("include_silkscreen", true),
    ParamSpec::boolean("include_labels", true),
    ParamSpec::boolean("include_courtyard", true),
    ParamSpec::string(
        "niceview_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/Nice_View.step",
    ),
    ParamSpec::array("niceview_3dmodel_xyz_offset", "[]"),
    ParamSpec::array("niceview_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("niceview_3dmodel_xyz_scale", "[1,1,1]"),
    ParamSpec::string(
        "pin_socket_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/kicad/PinSocket_1x05_P2.54mm_Vertical.step",
    ),
    ParamSpec::array("pin_socket_3dmodel_xyz_offset", "[]"),
    ParamSpec::array("pin_socket_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("pin_socket_3dmodel_xyz_scale", "[1,1,1]"),
    ParamSpec::string(
        "pin_header_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/PinHeader_2.54mm_1x-5.step",
    ),
    ParamSpec::array("pin_header_3dmodel_xyz_offset", "[]"),
    ParamSpec::array("pin_header_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("pin_header_3dmodel_xyz_scale", "[1,1,1]"),
    ParamSpec::net_default("MOSI", "MOSI"),
    ParamSpec::net_default("SCK", "SCK"),
    ParamSpec::net_default("VCC", "VCC"),
    ParamSpec::net_default("GND", "GND"),
    ParamSpec::net_default("CS", "CS"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "ceoloide/display_nice_view",
    display_name: "display nice view",
    kind: PartKind::Custom,
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
    let (reversible, side) = (p.flag("reversible"), p.side());
    let mut dst_nets: Vec<NetRef> = ["MOSI", "SCK", "VCC", "GND", "CS"]
        .iter()
        .map(|name| p.net(name).clone())
        .collect();
    let local_nets: Vec<NetRef> = vec![
        p.local_net("1")?,
        p.local_net("2")?,
        p.net("VCC").clone(),
        p.local_net("4")?,
        p.local_net("5")?,
    ];
    if reversible || side == "B" {
        dst_nets.reverse();
    }
    let socket_nets = if reversible {
        local_nets.clone()
    } else {
        dst_nets.clone()
    };

    let mut jumpers_offset = 0.0;
    let mut labels_offset = 3.75;
    let mut label_vcc_offset = 3.75;
    let mut jumpers_front_top = dst_nets.clone();
    let mut jumpers_front_bottom = local_nets.clone();
    let mut jumpers_back_top = dst_nets.clone();
    let mut jumpers_back_bottom: Vec<NetRef> = local_nets.iter().rev().cloned().collect();
    if p.flag("invert_labels_position") {
        if reversible && !p.flag("invert_jumpers_position") {
            label_vcc_offset = 0.0;
            labels_offset = -1.62;
        } else {
            label_vcc_offset = 0.0;
            labels_offset = label_vcc_offset;
        }
    } else if reversible && p.flag("invert_jumpers_position") {
        labels_offset = 1.62 + label_vcc_offset;
    }
    if p.flag("invert_jumpers_position") {
        jumpers_offset = 4.4;
        jumpers_front_top = local_nets.clone();
        jumpers_front_bottom = dst_nets.clone();
        jumpers_back_top = local_nets.iter().rev().cloned().collect();
        jumpers_back_bottom = dst_nets.clone();
    }

    // Place the independently authored models on their shared five-pin row.
    let socket_height = 8.5;
    let display_height = socket_height + 1.8;
    let header_height = socket_height - 2.0;
    let back = side == "B";
    for name in [
        "niceview_3dmodel_xyz_scale",
        "pin_socket_3dmodel_xyz_scale",
        "pin_header_3dmodel_xyz_scale",
    ] {
        p.vec3(name)?;
    }
    let display_offset = p.vec3("niceview_3dmodel_xyz_offset")?.unwrap_or(if back {
        [7.0, 18.0, display_height]
    } else {
        [-7.0, -18.0, display_height]
    });
    let display_rotation = p.vec3("niceview_3dmodel_xyz_rotation")?.unwrap_or([
        0.0,
        0.0,
        if back { 180.0 } else { 0.0 },
    ]);
    let header_offset = p.vec3("pin_header_3dmodel_xyz_offset")?.unwrap_or(if back {
        [-0.0124, 16.732, header_height]
    } else {
        [0.0124, -16.732, header_height]
    });
    let header_rotation = p.vec3("pin_header_3dmodel_xyz_rotation")?.unwrap_or([
        0.0,
        0.0,
        if back { 90.0 } else { -90.0 },
    ]);
    let socket_offset = p.vec3("pin_socket_3dmodel_xyz_offset")?.unwrap_or(if back {
        [5.08, 16.7, 0.0]
    } else {
        [-5.08, -16.7, 0.0]
    });
    let socket_rotation = p.vec3("pin_socket_3dmodel_xyz_rotation")?.unwrap_or([
        0.0,
        0.0,
        if back { 90.0 } else { -90.0 },
    ]);

    let top = format!(
        r####"
  (footprint "ceoloide:display_nice_view"
    (layer {e0}.Cu)
    {e1}
    (property "Reference" "{e2}"
      (at 0 20 {e3})
      (layer "{e0}.SilkS")
      {e4}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (attr exclude_from_pos_files exclude_from_bom)
    "####,
        e0 = p.side(),
        e1 = p.at(),
        e2 = p.reference(),
        e3 = n(p.rotation()),
        e4 = p.ref_hide()
    );

    let front_silkscreen = String::from(
        r####"
    (fp_line (start -6.41 15.37) (end -6.41 18.03) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 6.41 18.03) (end -6.41 18.03) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 6.41 15.37) (end 6.41 18.03) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 6.41 15.37) (end -6.41 15.37) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    "####,
    );

    let front_courtyard = String::from(
        r####"
    (fp_line (start 6.88 14.9) (end 6.88 18.45) (layer "F.CrtYd") (stroke (width 0.15) (type solid)))
    (fp_line (start 6.88 18.45) (end -6.82 18.45) (layer "F.CrtYd") (stroke (width 0.15) (type solid)))
    (fp_line (start -6.82 18.45) (end -6.82 14.9) (layer "F.CrtYd") (stroke (width 0.15) (type solid)))
    (fp_line (start -6.82 14.9) (end 6.88 14.9) (layer "F.CrtYd") (stroke (width 0.15) (type solid)))
    "####,
    );

    let front_jumpers = format!(
        r####"
    (pad "14" smd rect (at -5.08 {e0} {e1}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") {e2})
    (pad "15" smd rect (at -2.54 {e0} {e1}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") {e3})
    (pad "16" smd rect (at 2.54 {e0} {e1}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") {e4})
    (pad "17" smd rect (at 5.08 {e0} {e1}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") {e5})

    (pad "10" smd rect (at -5.08 {e6} {e1}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") {e7})
    (pad "11" smd rect (at -2.54 {e6} {e1}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") {e8})
    (pad "12" smd rect (at 2.54 {e6} {e1}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") {e9})
    (pad "13" smd rect (at 5.08 {e6} {e1}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") {e10})
    "####,
        e0 = n(14.05 + jumpers_offset),
        e1 = n(90.0 + p.rotation()),
        e2 = jumpers_front_top[0],
        e3 = jumpers_front_top[1],
        e4 = jumpers_front_top[3],
        e5 = jumpers_front_top[4],
        e6 = n(14.95 + jumpers_offset),
        e7 = jumpers_front_bottom[0],
        e8 = jumpers_front_bottom[1],
        e9 = jumpers_front_bottom[3],
        e10 = jumpers_front_bottom[4]
    );

    let back_silkscreen = String::from(
        r####"
    (fp_line (start 6.41 15.37) (end 6.41 18.03) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 6.41 15.37) (end -6.41 15.37) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 6.41 18.03) (end -6.41 18.03) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -6.41 15.37) (end -6.41 18.03) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    "####,
    );

    let back_courtyard = String::from(
        r####"
    (fp_line (start 6.88 14.9) (end 6.88 18.45) (layer "B.CrtYd") (stroke (width 0.15) (type solid)))
    (fp_line (start 6.88 18.45) (end -6.82 18.45) (layer "B.CrtYd") (stroke (width 0.15) (type solid)))
    (fp_line (start -6.82 18.45) (end -6.82 14.9) (layer "B.CrtYd") (stroke (width 0.15) (type solid)))
    (fp_line (start -6.82 14.9) (end 6.88 14.9) (layer "B.CrtYd") (stroke (width 0.15) (type solid)))
    "####,
    );

    let back_jumpers = format!(
        r####"
    (pad "24" smd rect (at 5.08 {e0} {e1}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") {e2})
    (pad "25" smd rect (at 2.54 {e0} {e1}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") {e3})
    (pad "26" smd rect (at -2.54 {e0} {e1}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") {e4})
    (pad "27" smd rect (at -5.08 {e0} {e1}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") {e5})

    (pad "20" smd rect (at 5.08 {e6} {e1}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") {e7})
    (pad "21" smd rect (at 2.54 {e6} {e1}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") {e8})
    (pad "22" smd rect (at -2.54 {e6} {e1}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") {e9})
    (pad "23" smd rect (at -5.08 {e6} {e1}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") {e10})
    "####,
        e0 = n(14.05 + jumpers_offset),
        e1 = n(270.0 + p.rotation()),
        e2 = jumpers_back_top[0],
        e3 = jumpers_back_top[1],
        e4 = jumpers_back_top[3],
        e5 = jumpers_back_top[4],
        e6 = n(14.95 + jumpers_offset),
        e7 = jumpers_back_bottom[0],
        e8 = jumpers_back_bottom[1],
        e9 = jumpers_back_bottom[3],
        e10 = jumpers_back_bottom[4]
    );

    let silkscreen_labels_front = format!(
        r####"
    (fp_text user "{e0}" (at -5.08 {e1} {e2}) (unlocked yes) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify {e3}))
    )
    (fp_text user "{e4}" (at -2.48 {e1} {e2}) (unlocked yes) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify {e5}))
    )
    (fp_text user "{e6}" (at 0.15 {e7} {e2}) (unlocked yes) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify {e5}))
    )
    (fp_text user "{e8}" (at 2.62 {e1} {e2}) (unlocked yes) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify {e5}))
    )
    (fp_text user "{e9}" (at 5.12 {e1} {e2}) (unlocked yes) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify {e5}))
    )
    "####,
        e0 = dst_nets[0].name,
        e1 = n(14.75 + labels_offset),
        e2 = n(90.0 + p.rotation()),
        e3 = if !(p.flag("invert_labels_position")) {
            "right".to_string()
        } else {
            "left".to_string()
        },
        e4 = dst_nets[1].name,
        e5 = if !(p.flag("invert_labels_position")) {
            "right".to_string()
        } else {
            "left".to_string()
        },
        e6 = dst_nets[2].name,
        e7 = n(14.75 + label_vcc_offset),
        e8 = dst_nets[3].name,
        e9 = dst_nets[4].name
    );

    let silkscreen_labels_back = format!(
        r####"
    (fp_text user "{e0}" (at 5.22 {e1} {e2}) (unlocked yes) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify {e3} mirror))
    )
    (fp_text user "{e4}" (at 2.72 {e1} {e2}) (unlocked yes) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify {e3} mirror))
    )
    (fp_text user "{e5}" (at 0.15 {e6} {e2}) (unlocked yes) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify {e3} mirror))
    )
    (fp_text user "{e7}" (at -2.38 {e1} {e2}) (unlocked yes) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify {e3} mirror))
    )
    (fp_text user "{e8}" (at -4.98 {e1} {e2}) (unlocked yes) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify {e3} mirror))
    )
    "####,
        e0 = if reversible {
            dst_nets[0].name.clone()
        } else {
            dst_nets[4].name.clone()
        },
        e1 = n(14.75 + labels_offset),
        e2 = n(90.0 + p.rotation()),
        e3 = if !(p.flag("invert_labels_position")) {
            "left".to_string()
        } else {
            "right".to_string()
        },
        e4 = if p.flag("reversible") {
            dst_nets[1].name.clone()
        } else {
            dst_nets[3].name.clone()
        },
        e5 = if p.flag("reversible") {
            dst_nets[2].name.clone()
        } else {
            dst_nets[2].name.clone()
        },
        e6 = n(14.75 + label_vcc_offset),
        e7 = if p.flag("reversible") {
            dst_nets[3].name.clone()
        } else {
            dst_nets[1].name.clone()
        },
        e8 = if p.flag("reversible") {
            dst_nets[4].name.clone()
        } else {
            dst_nets[0].name.clone()
        }
    );

    let niceview_3dmodel = format!(
        r####"
    (model {e0}
      (offset (xyz {e1} {e2} {e3}))
      (scale (xyz {e4} {e5} {e6}))
      (rotate (xyz {e7} {e8} {e9}))
    )
    "####,
        e0 = json_string(p.text("niceview_3dmodel_filename")),
        e1 = n(display_offset[0]),
        e2 = n(display_offset[1]),
        e3 = n(display_offset[2]),
        e4 = n(p.component("niceview_3dmodel_xyz_scale", 0)),
        e5 = n(p.component("niceview_3dmodel_xyz_scale", 1)),
        e6 = n(p.component("niceview_3dmodel_xyz_scale", 2)),
        e7 = n(display_rotation[0]),
        e8 = n(display_rotation[1]),
        e9 = n(display_rotation[2])
    );

    let pin_socket_3dmodel = format!(
        r####"
    (model {e0}
      (offset (xyz {e1} {e2} {e3}))
      (scale (xyz {e4} {e5} {e6}))
      (rotate (xyz {e7} {e8} {e9}))
    )
    "####,
        e0 = json_string(p.text("pin_socket_3dmodel_filename")),
        e1 = n(socket_offset[0]),
        e2 = n(socket_offset[1]),
        e3 = n(socket_offset[2]),
        e4 = n(p.component("pin_socket_3dmodel_xyz_scale", 0)),
        e5 = n(p.component("pin_socket_3dmodel_xyz_scale", 1)),
        e6 = n(p.component("pin_socket_3dmodel_xyz_scale", 2)),
        e7 = n(socket_rotation[0]),
        e8 = n(socket_rotation[1]),
        e9 = n(socket_rotation[2])
    );

    let pin_header_3dmodel = format!(
        r####"
    (model {e0}
      (offset (xyz {e1} {e2} {e3}))
      (scale (xyz {e4} {e5} {e6}))
      (rotate (xyz {e7} {e8} {e9}))
    )
    "####,
        e0 = json_string(p.text("pin_header_3dmodel_filename")),
        e1 = n(header_offset[0]),
        e2 = n(header_offset[1]),
        e3 = n(header_offset[2]),
        e4 = n(p.component("pin_header_3dmodel_xyz_scale", 0)),
        e5 = n(p.component("pin_header_3dmodel_xyz_scale", 1)),
        e6 = n(p.component("pin_header_3dmodel_xyz_scale", 2)),
        e7 = n(header_rotation[0]),
        e8 = n(header_rotation[1]),
        e9 = n(header_rotation[2])
    );

    let bottom = format!(
        r####"
    (pad "1" thru_hole oval (at -5.08 16.7 {e0}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") {e1})
    (pad "2" thru_hole oval (at -2.54 16.7 {e0}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") {e2})
    (pad "3" thru_hole oval (at 0 16.7 {e0}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") {e3})
    (pad "4" thru_hole oval (at 2.54 16.7 {e0}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") {e4})
    (pad "5" thru_hole circle (at 5.08 16.7 {e0}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") {e5})

    (fp_line (start 5.4 13.4) (end 5.4 -11.9) (layer Dwgs.User) (stroke (width 0.15) (type solid)))
    (fp_line (start -5.4 13.4) (end -5.4 -11.9) (layer Dwgs.User) (stroke (width 0.15) (type solid)))
    (fp_line (start 5.4 -11.9) (end -5.4 -11.9) (layer Dwgs.User) (stroke (width 0.15) (type solid)))
    (fp_line (start -5.4 13.4) (end 5.4 13.4) (layer Dwgs.User) (stroke (width 0.15) (type solid)))

    (fp_line (start -7 -18) (end 7 -18) (layer Dwgs.User) (stroke (width 0.15) (type solid)))
    (fp_line (start 7 18) (end -7 18) (layer Dwgs.User) (stroke (width 0.15) (type solid)))
    (fp_line (start -7 18) (end -7 -18) (layer Dwgs.User) (stroke (width 0.15) (type solid)))
    (fp_line (start 7 18) (end 7 -18) (layer Dwgs.User) (stroke (width 0.15) (type solid)))
  )
    "####,
        e0 = n(270.0 + p.rotation()),
        e1 = socket_nets[0],
        e2 = socket_nets[1],
        e3 = socket_nets[2],
        e4 = socket_nets[3],
        e5 = socket_nets[4]
    );

    let traces_bottom = format!(
        r####"
  (segment (start {e0}) (end {e1}) (width {e2}) (layer "F.Cu") (net {e3}))
  (segment (start {e4}) (end {e5}) (width {e6}) (layer "F.Cu") (net {e7}))
  (segment (start {e8}) (end {e9}) (width {e2}) (layer "F.Cu") (net {e10}))
  (segment (start {e11}) (end {e12}) (width {e2}) (layer "F.Cu") (net {e13}))
  (segment (start {e0}) (end {e1}) (width {e2}) (layer "B.Cu") (net {e3}))
  (segment (start {e4}) (end {e5}) (width {e2}) (layer "B.Cu") (net {e7}))
  (segment (start {e8}) (end {e9}) (width {e6}) (layer "B.Cu") (net {e10}))
  (segment (start {e11}) (end {e12}) (width {e2}) (layer "B.Cu") (net {e13}))
    "####,
        e0 = p.eaxy(-5.08, 16.7),
        e1 = p.eaxy(-5.08, 18.45),
        e2 = n(p.number("signal_trace_width")),
        e3 = socket_nets[0].index,
        e4 = p.eaxy(-2.54, 16.7),
        e5 = p.eaxy(-2.54, 18.45),
        e6 = n(p.number("gnd_trace_width")),
        e7 = socket_nets[1].index,
        e8 = p.eaxy(2.54, 16.7),
        e9 = p.eaxy(2.54, 18.45),
        e10 = socket_nets[3].index,
        e11 = p.eaxy(5.08, 16.7),
        e12 = p.eaxy(5.08, 18.45),
        e13 = socket_nets[4].index
    );

    let traces_top = format!(
        r####"
  (segment (start {e0}) (end {e1}) (width {e2}) (layer "F.Cu") (net {e3}))
  (segment (start {e4}) (end {e5}) (width {e6}) (layer "F.Cu") (net {e7}))
  (segment (start {e8}) (end {e9}) (width {e2}) (layer "F.Cu") (net {e10}))
  (segment (start {e11}) (end {e12}) (width {e2}) (layer "F.Cu") (net {e13}))
  (segment (start {e0}) (end {e1}) (width {e2}) (layer "B.Cu") (net {e3}))
  (segment (start {e4}) (end {e5}) (width {e2}) (layer "B.Cu") (net {e7}))
  (segment (start {e8}) (end {e9}) (width {e6}) (layer "B.Cu") (net {e10}))
  (segment (start {e11}) (end {e12}) (width {e2}) (layer "B.Cu") (net {e13}))
    "####,
        e0 = p.eaxy(-5.08, 16.7),
        e1 = p.eaxy(-5.08, 14.95),
        e2 = n(p.number("signal_trace_width")),
        e3 = socket_nets[0].index,
        e4 = p.eaxy(-2.54, 16.7),
        e5 = p.eaxy(-2.54, 14.95),
        e6 = n(p.number("gnd_trace_width")),
        e7 = socket_nets[1].index,
        e8 = p.eaxy(2.54, 16.7),
        e9 = p.eaxy(2.54, 14.95),
        e10 = socket_nets[3].index,
        e11 = p.eaxy(5.08, 16.7),
        e12 = p.eaxy(5.08, 14.95),
        e13 = socket_nets[4].index
    );

    let mut out = top;
    if side == "F" || reversible {
        if p.flag("include_silkscreen") {
            out += &front_silkscreen;
            if p.flag("include_labels") {
                out += &silkscreen_labels_front;
            }
        }
        if p.flag("include_courtyard") {
            out += &front_courtyard;
        }
    }
    if side == "B" || reversible {
        if p.flag("include_silkscreen") {
            out += &back_silkscreen;
            if p.flag("include_labels") {
                out += &silkscreen_labels_back;
            }
        }
        if p.flag("include_courtyard") {
            out += &back_courtyard;
        }
    }
    if reversible {
        out += &front_jumpers;
        out += &back_jumpers;
    }
    if !p.text("niceview_3dmodel_filename").is_empty() {
        out += &niceview_3dmodel;
    }
    if !p.text("pin_socket_3dmodel_filename").is_empty() {
        out += &pin_socket_3dmodel;
    }
    if !p.text("pin_header_3dmodel_filename").is_empty() {
        out += &pin_header_3dmodel;
    }
    out += &bottom;
    if p.flag("include_traces") && reversible {
        out += if p.flag("invert_jumpers_position") {
            &traces_bottom
        } else {
            &traces_top
        };
    }
    Ok(out)
}

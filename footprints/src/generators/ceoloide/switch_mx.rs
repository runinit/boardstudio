//! `ceoloide/switch_mx`
//!
//! Ported from `switch_mx.js` of ceoloide/ergogen-footprints.
//! Copyright (c) 2023 Marco Massarelli
//! SPDX-License-Identifier: MIT
//! Author: @ceoloide
use crate::Result;
use crate::context::RenderContext;
use crate::generators::util::json_string;
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, KeycapParameters, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 41] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "S"),
    ParamSpec::number("pcb_thickness", 1.6),
    ParamSpec::boolean("reversible", false),
    ParamSpec::boolean("hotswap", true),
    ParamSpec::boolean("hotswap_pads_same_side", false),
    ParamSpec::boolean("include_traces_vias", true),
    ParamSpec::number("trace_width", 0.2),
    ParamSpec::number("via_size", 0.6),
    ParamSpec::number("via_drill", 0.3),
    ParamSpec::boolean("locked_traces_vias", false),
    ParamSpec::boolean("include_plated_holes", false),
    ParamSpec::boolean("include_stabilizer_nets", false),
    ParamSpec::boolean("include_centerhole_net", false),
    ParamSpec::boolean("solder", false),
    ParamSpec::number("outer_pad_width_front", 2.6),
    ParamSpec::number("outer_pad_width_back", 2.6),
    ParamSpec::number("outer_pad_height", 2.5),
    ParamSpec::number("stabilizers_diameter", 1.9),
    ParamSpec::boolean("include_keycap", true),
    ParamSpec::number("keycap_width", 18.0),
    ParamSpec::number("keycap_height", 18.0),
    ParamSpec::boolean("include_corner_marks", false),
    ParamSpec::boolean("include_silkscreen", true),
    ParamSpec::string(
        "switch_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/kiswitch/SW_Cherry_MX_PCB.stp",
    ),
    ParamSpec::array("switch_3dmodel_xyz_offset", "[]"),
    ParamSpec::array("switch_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("switch_3dmodel_xyz_scale", "[1,1,1]"),
    ParamSpec::string(
        "hotswap_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/kiswitch/SW_Hotswap_Kailh_MX.stp",
    ),
    ParamSpec::array("hotswap_3dmodel_xyz_offset", "[]"),
    ParamSpec::array("hotswap_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("hotswap_3dmodel_xyz_scale", "[1,1,1]"),
    ParamSpec::string("keycap_3dmodel_filename", ""),
    ParamSpec::array("keycap_3dmodel_xyz_offset", "[0,0,0]"),
    ParamSpec::array("keycap_3dmodel_xyz_rotation", "[0,0,0]"),
    ParamSpec::array("keycap_3dmodel_xyz_scale", "[1,1,1]"),
    ParamSpec::net("from"),
    ParamSpec::net("to"),
    ParamSpec::net_default("CENTERHOLE", "GND"),
    ParamSpec::net_default("LEFTSTAB", "D1"),
    ParamSpec::net_default("RIGHTSTAB", "D2"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "ceoloide/switch_mx",
    display_name: "switch mx",
    kind: PartKind::Switch,
    matrix_terminals: Some(("from", "to")),
    keycap_parameters: Some(KeycapParameters {
        width: "keycap_width",
        height: "keycap_height",
    }),
    parameters: &PARAMS,
    license: License {
        spdx: "MIT",
        author: "@ceoloide",
    },
    body,
};

fn body(p: &RenderContext<'_>) -> Result<String> {
    let keycap_xo = 0.5 * p.number("keycap_width");
    let keycap_yo = 0.5 * p.number("keycap_height");
    for name in [
        "switch_3dmodel_xyz_scale",
        "hotswap_3dmodel_xyz_scale",
        "keycap_3dmodel_xyz_offset",
        "keycap_3dmodel_xyz_scale",
        "keycap_3dmodel_xyz_rotation",
    ] {
        p.vec3(name)?;
    }
    // Socket side is opposite the switch; correct the downloaded housing datum.
    const SWITCH_XY_DATUM: f64 = 0.00709;
    const SWITCH_Z_DATUM: f64 = 0.24252;
    const SOCKET_Z_DATUM: f64 = 0.27094;
    let back = p.side() == "B";
    let pcb_thickness = p.number("pcb_thickness");
    let model_rotation = if back {
        [180.0, 0.0, 0.0]
    } else {
        [0.0, 180.0, 0.0]
    };
    let switch_rotation = p
        .vec3("switch_3dmodel_xyz_rotation")?
        .unwrap_or(model_rotation);
    let switch_offset = p.vec3("switch_3dmodel_xyz_offset")?.unwrap_or([
        if back {
            SWITCH_XY_DATUM
        } else {
            -SWITCH_XY_DATUM
        },
        if back {
            -SWITCH_XY_DATUM
        } else {
            SWITCH_XY_DATUM
        },
        -pcb_thickness + SWITCH_Z_DATUM,
    ]);
    let hotswap_rotation = p
        .vec3("hotswap_3dmodel_xyz_rotation")?
        .unwrap_or(model_rotation);
    let hotswap_offset = p.vec3("hotswap_3dmodel_xyz_offset")?.unwrap_or([
        0.0,
        0.0,
        -pcb_thickness - SOCKET_Z_DATUM,
    ]);

    let common_top = format!(
        r####"
  (footprint "ceoloide:switch_mx"
    (layer "{e0}.Cu")
    {e1}
    (property "Reference" "{e2}"
      (at 0 -7.5 180)
      (layer "{e0}.SilkS")
      {e3}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    
    (pad "" {e4} circle 
      (at 0 0 {e5})
      (size {e6})
      (drill 4.1)
      (layers "*.Cu" "*.Mask")
      {e7}
    )
    (pad "" {e4} circle 
      (at 5.08 0 {e5})
      (size {e8} {e8})
      (drill {e9})
      (layers "*.Cu" "*.Mask")
      {e10}
    )
    (pad "" {e4} circle 
      (at -5.08 0 {e5})
      (size {e8} {e8})
      (drill {e9})
      (layers "*.Cu" "*.Mask")
      {e11}
    )
    "####,
        e0 = p.side(),
        e1 = p.at(),
        e2 = p.reference(),
        e3 = p.ref_hide(),
        e4 = if !(p.flag("include_plated_holes")) {
            "np_thru_hole".to_string()
        } else {
            "thru_hole".to_string()
        },
        e5 = n(p.rotation()),
        e6 = if p.flag("include_plated_holes") {
            "4.4 4.4".to_string()
        } else {
            "4.1 4.1".to_string()
        },
        e7 = if p.flag("include_plated_holes") && p.flag("include_centerhole_net") {
            p.net("CENTERHOLE").to_string()
        } else {
            "".to_string()
        },
        e8 = n(p.number("stabilizers_diameter")
            + (if p.flag("include_plated_holes") {
                0.3
            } else {
                0.0
            })),
        e9 = n(p.number("stabilizers_diameter")),
        e10 = if p.flag("include_plated_holes") && p.flag("include_stabilizer_nets") {
            p.net("RIGHTSTAB").to_string()
        } else {
            "".to_string()
        },
        e11 = if p.flag("include_plated_holes") && p.flag("include_stabilizer_nets") {
            p.net("LEFTSTAB").to_string()
        } else {
            "".to_string()
        },
    );

    let corner_marks = String::from(
        r####"
    (fp_line (start -7 -6) (end -7 -7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -7 7) (end -6 7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -6 -7) (end -7 -7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -7 7) (end -7 6) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start 7 6) (end 7 7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start 7 -7) (end 6 -7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start 6 7) (end 7 7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start 7 -7) (end 7 -6) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    "####,
    );

    let keycap_marks = format!(
        r####"
    (fp_rect (start {e0} {e1}) (end {e2} {e3}) (layer "Dwgs.User") (stroke (width 0.15) (type solid)) (fill none))
    "####,
        e0 = n(keycap_xo),
        e1 = n(keycap_yo),
        e2 = n(-keycap_xo),
        e3 = n(-keycap_yo),
    );

    let hotswap_front = format!(
        r####"
		(pad "" np_thru_hole circle (at -2.54 -5.08 180) (size 3 3) (drill 3) (layers "F&B.Cu" "*.Mask"))
		(pad "" np_thru_hole circle (at 3.81 -2.54 180) (size 3 3) (drill 3) (layers "F&B.Cu" "*.Mask"))
		(pad "1" smd rect (at {e0} -2.54 {e1}) (size {e2} {e3}) (layers "F.Cu" "F.Paste" "F.Mask") {e4})
		(pad "2" smd {e5}
      (at -5.842 -5.08 {e1})
      (size 2.55 2.5)
      (layers "F.Cu" "F.Paste" "F.Mask"){e6}
      {e7}
    )
    "####,
        e0 = n(5.81 + p.number("outer_pad_width_front") / 2.0),
        e1 = n(p.rotation()),
        e2 = n(p.number("outer_pad_width_front")),
        e3 = n(p.number("outer_pad_height")),
        e4 = p.net("from"),
        e5 = if p.flag("reversible") {
            "roundrect".to_string()
        } else {
            "rect".to_string()
        },
        e6 = if p.flag("reversible") {
            "\n\t\t\t(roundrect_rratio 0)\n\t\t\t(chamfer_ratio 0.2)\n\t\t\t(chamfer bottom_right)"
                .to_string()
        } else {
            "".to_string()
        },
        e7 = p.net("to"),
    );

    let hotswap_back = format!(
        r####"
		(pad "" np_thru_hole circle (at 2.54 -5.08 180) (size 3 3) (drill 3) (layers "F&B.Cu" "*.Mask"))
		(pad "" np_thru_hole circle (at -3.81 -2.54 180) (size 3 3) (drill 3) (layers "F&B.Cu" "*.Mask"))
		(pad "1" smd rect
      (at {e0} -2.54 {e1})
      (size {e2} {e3})
      (layers "B.Cu" "B.Paste" "B.Mask")
      {e4}
    )
		(pad "2" smd {e5}
      (at 5.842 -5.08 {e1})
      (size 2.55 2.5)
      (layers "B.Cu" "B.Paste" "B.Mask"){e6}
      {e7}
    )
    "####,
        e0 = n(-5.81 - p.number("outer_pad_width_back") / 2.0),
        e1 = n(p.rotation()),
        e2 = n(p.number("outer_pad_width_back")),
        e3 = n(p.number("outer_pad_height")),
        e4 = if p.flag("hotswap_pads_same_side") {
            p.net("to").to_string()
        } else {
            p.net("from").to_string()
        },
        e5 = if p.flag("reversible") {
            "roundrect".to_string()
        } else {
            "rect".to_string()
        },
        e6 = if p.flag("reversible") {
            "\n\t\t\t(roundrect_rratio 0)\n\t\t\t(chamfer_ratio 0.2)\n\t\t\t(chamfer bottom_left)"
                .to_string()
        } else {
            "".to_string()
        },
        e7 = if p.flag("hotswap_pads_same_side") {
            p.net("from").to_string()
        } else {
            p.net("to").to_string()
        },
    );

    let hotswap_silkscreen_back = String::from(
        r####"
		(fp_poly
			(pts
				(xy -3.6 -6.5) (xy -3.8 -6.5) (xy -4.1 -6.45) (xy -4.4 -6.35) (xy -4.6 -6.25) (xy -4.75 -6.15) (xy -4.95 -6)
				(xy -5.1 -5.85) (xy -5.25 -5.65) (xy -5.4 -5.4) (xy -5.5 -5) (xy -5.5 -4.6) (xy -5.35 -4.5) (xy -5.2 -4.4)
				(xy -4.75 -4.65) (xy -4.5 -4.75) (xy -4.05 -4.85) (xy -3.55 -4.85) (xy -2.95 -4.7) (xy -2.45 -4.4) (xy -2.15 -4.15)
        (xy -1.75 -3.6) (xy -1.55 -3.05) (xy -1.5 -2.6) (xy -1.25 -2.8) (xy -0.9 -2.9) (xy -0.4 -2.95) (xy 1.65 -2.95)
        (xy 1.2 -3.2) (xy 0.95 -3.4) (xy 0.65 -3.75) (xy 0.5 -4) (xy 0.35 -4.35) (xy 0.25 -4.75) (xy 0.25 -5.05)
        (xy 0.25 -5.4) (xy 0.3 -5.65) (xy 0.45 -6.05) (xy 0.75 -6.5)
			)
			(stroke (width 0.4) (type solid))
			(fill solid)
			(layer "B.SilkS")
		)
    "####,
    );

    let hotswap_silkscreen_front = String::from(
        r####"
		(fp_poly
			(pts
				(xy 3.6 -6.5) (xy 3.8 -6.5) (xy 4.1 -6.45) (xy 4.4 -6.35) (xy 4.6 -6.25) (xy 4.75 -6.15) (xy 4.95 -6)
				(xy 5.1 -5.85) (xy 5.25 -5.65) (xy 5.4 -5.4) (xy 5.5 -5) (xy 5.5 -4.6) (xy 5.35 -4.5) (xy 5.2 -4.4)
				(xy 4.75 -4.65) (xy 4.5 -4.75) (xy 4.05 -4.85) (xy 3.55 -4.85) (xy 2.95 -4.7) (xy 2.45 -4.4) (xy 2.15 -4.15)
				(xy 1.75 -3.6) (xy 1.55 -3.05) (xy 1.5 -2.6) (xy 1.25 -2.8) (xy 0.9 -2.9) (xy 0.4 -2.95) (xy -1.65 -2.95)
				(xy -1.2 -3.2) (xy -0.95 -3.4) (xy -0.65 -3.75) (xy -0.5 -4) (xy -0.35 -4.35) (xy -0.25 -4.75) (xy -0.25 -5.05)
				(xy -0.25 -5.4) (xy -0.3 -5.65) (xy -0.45 -6.05) (xy -0.75 -6.5)
			)
			(stroke (width 0.4) (type solid))
			(fill solid)
			(layer "F.SilkS")
		)
    "####,
    );

    let hotswap_silkscreen_reversible = String::from(
        r####"
		(fp_line (start 1.22 -3.77) (end 0 -2.52) (stroke (width 0.1) (type default)) (layer "B.SilkS"))
		(fp_line (start 0 -2.52) (end -1.88 -2.52) (stroke (width 0.1) (type default)) (layer "B.SilkS"))
		(fp_line (start -1.22 -3.77) (end 0 -2.52) (stroke (width 0.1) (type default)) (layer "F.SilkS"))
		(fp_line (start 0 -2.52) (end 1.88 -2.52) (stroke (width 0.1) (type default)) (layer "F.SilkS"))
    "####,
    );

    let solder_front = format!(
        r####"
    (pad "1" thru_hole circle (at {e0}2.54 {e0}5.08) (size 2.286 2.286) (drill 1.4986) (layers "F&B.Cu" "*.Mask") {e1})
    (pad "2" thru_hole circle (at {e2}3.81 {e0}2.54) (size 2.286 2.286) (drill 1.4986) (layers "F&B.Cu" "*.Mask") {e3})
    "####,
        e0 = if p.flag("solder") && p.flag("hotswap") {
            "".to_string()
        } else {
            "-".to_string()
        },
        e1 = p.net("from"),
        e2 = if p.flag("solder") && p.flag("hotswap") {
            "-".to_string()
        } else {
            "".to_string()
        },
        e3 = p.net("to"),
    );

    let solder_back = format!(
        r####"
    (pad "1" thru_hole circle (at {e0}2.54 {e1}5.08) (size 2.286 2.286) (drill 1.4986) (layers "F&B.Cu" "*.Mask") {e2})
    (pad "2" thru_hole circle (at {e1}3.81 {e1}2.54) (size 2.286 2.286) (drill 1.4986) (layers "F&B.Cu" "*.Mask") {e3})
    "####,
        e0 = if p.flag("solder") && p.flag("hotswap") {
            "-".to_string()
        } else {
            "".to_string()
        },
        e1 = if p.flag("solder") && p.flag("hotswap") {
            "".to_string()
        } else {
            "-".to_string()
        },
        e2 = p.net("from"),
        e3 = p.net("to"),
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
        e1 = n(switch_offset[0]),
        e2 = n(switch_offset[1]),
        e3 = n(switch_offset[2]),
        e4 = n(p.component("switch_3dmodel_xyz_scale", 0)),
        e5 = n(p.component("switch_3dmodel_xyz_scale", 1)),
        e6 = n(p.component("switch_3dmodel_xyz_scale", 2)),
        e7 = n(switch_rotation[0]),
        e8 = n(switch_rotation[1]),
        e9 = n(switch_rotation[2]),
    );

    let hotswap_3dmodel = format!(
        r####"
    (model {e0}
      (offset (xyz {e1} {e2} {e3}))
      (scale (xyz {e4} {e5} {e6}))
      (rotate (xyz {e7} {e8} {e9}))
    )
	  "####,
        e0 = json_string(p.text("hotswap_3dmodel_filename")),
        e1 = n(hotswap_offset[0]),
        e2 = n(hotswap_offset[1]),
        e3 = n(hotswap_offset[2]),
        e4 = n(p.component("hotswap_3dmodel_xyz_scale", 0)),
        e5 = n(p.component("hotswap_3dmodel_xyz_scale", 1)),
        e6 = n(p.component("hotswap_3dmodel_xyz_scale", 2)),
        e7 = n(hotswap_rotation[0]),
        e8 = n(hotswap_rotation[1]),
        e9 = n(hotswap_rotation[2]),
    );

    let keycap_3dmodel = format!(
        r####"
    (model {e0}
      (offset (xyz {e1} {e2} {e3}))
      (scale (xyz {e4} {e5} {e6}))
      (rotate (xyz {e7} {e8} {e9}))
    )
	  "####,
        e0 = json_string(p.text("keycap_3dmodel_filename")),
        e1 = n(p.component("keycap_3dmodel_xyz_offset", 0)),
        e2 = n(p.component("keycap_3dmodel_xyz_offset", 1)),
        e3 = n(p.component("keycap_3dmodel_xyz_offset", 2)),
        e4 = n(p.component("keycap_3dmodel_xyz_scale", 0)),
        e5 = n(p.component("keycap_3dmodel_xyz_scale", 1)),
        e6 = n(p.component("keycap_3dmodel_xyz_scale", 2)),
        e7 = n(p.component("keycap_3dmodel_xyz_rotation", 0)),
        e8 = n(p.component("keycap_3dmodel_xyz_rotation", 1)),
        e9 = n(p.component("keycap_3dmodel_xyz_rotation", 2)),
    );

    let common_bottom = String::from(
        r####"
  )
    "####,
    );

    let hotswap_routes_unplated = format!(
        r####"
	(segment
		(start {e0})
		(end {e1})
		(width {e2})
    (locked {e3})
		(layer "F.Cu")
		(net {e4})
	)
	(segment
		(start {e1})
		(end {e5})
		(width {e2})
    (locked {e3})
		(layer "F.Cu")
		(net {e4})
	)
	(via
		(at {e5})
		(size {e6})
    (drill {e7})
		(layers "F.Cu" "B.Cu")
    (locked {e3})
		(net {e4})
	)
	(segment
		(start {e5})
		(end {e8})
		(width {e2})
    (locked {e3})
		(layer "B.Cu")
		(net {e4})
	)
	(segment
		(start {e9})
		(end {e10})
		(width {e2})
    (locked {e3})
		(layer "B.Cu")
		(net {e4})
  )
	(segment
    (start {e11})
    (end {e12})
    (width {e2})
    (locked {e3})
    (layer "F.Cu")
    (net {e13})
  )
  (segment
    (start {e14})
    (end {e15})
    (width {e2})
    (locked {e3})
    (layer "F.Cu")
    (net {e13})
  )
  (segment
    (start {e15})
    (end {e16})
    (width {e2})
    (locked {e3})
    (layer "F.Cu")
    (net {e13})
  )
  (segment
    (start {e12})
    (end {e14})
    (width {e2})
    (locked {e3})
    (layer "F.Cu")
    (net {e13})
  )
  (via
    (at {e11})
		(size {e6})
    (drill {e7})
    (layers "F.Cu" "B.Cu")
    (locked {e3})
    (net {e13})
  )
  (segment
    (start {e17})
    (end {e18})
    (width {e2})
    (locked {e3})
    (layer "B.Cu")
    (net {e13})
  )
  (segment
    (start {e19})
    (end {e20})
    (width {e2})
    (locked {e3})
    (layer "B.Cu")
    (net {e13})
  )
  (segment
    (start {e20})
    (end {e17})
    (width {e2})
    (locked {e3})
    (layer "B.Cu")
    (net {e13})
  )
  (segment
    (start {e18})
    (end {e11})
    (width {e2})
    (locked {e3})
    (layer "B.Cu")
    (net {e13})
  )
    "####,
        e0 = p.eaxy(-5.842, -5.08),
        e1 = p.eaxy(-3.963, -6.959),
        e2 = n(p.number("trace_width")),
        e3 = if p.flag("locked_traces_vias") {
            "yes".to_string()
        } else {
            "no".to_string()
        },
        e4 = p.net("to").index,
        e5 = p.eaxy(0.0, -6.959),
        e6 = n(p.number("via_size")),
        e7 = n(p.number("via_drill")),
        e8 = p.eaxy(3.963, -6.9595),
        e9 = p.eaxy(3.963, -6.959),
        e10 = p.eaxy(5.842, -5.08),
        e11 = p.eaxy(0.0, -5.93),
        e12 = p.eaxy(1.029, -6.959),
        e13 = p.net("from").index,
        e14 = p.eaxy(4.166, -6.959),
        e15 = p.eaxy(7.085, -4.04),
        e16 = p.eaxy(7.085, -2.54),
        e17 = p.eaxy(-4.166, -6.959),
        e18 = p.eaxy(-1.029, -6.959),
        e19 = p.eaxy(-7.085, -2.54),
        e20 = p.eaxy(-7.085, -4.04),
    );

    let hotswap_routes_same_side = format!(
        r####"
	(segment
		(start {e0})
		(end {e1})
		(width {e2})
    (locked {e3})
		(layer "F.Cu")
		(net {e4})
	)
	(segment
		(start {e5})
		(end {e0})
		(width {e2})
    (locked {e3})
		(layer "F.Cu")
		(net {e4})
	)
	(via
		(at {e5})
		(size {e6})
    (drill {e7})
		(layers "F.Cu" "B.Cu")
    (locked {e3})
		(net {e4})
	)
	(segment
		(start {e8})
		(end {e5})
		(width {e2})
    (locked {e3})
		(layer "B.Cu")
		(net {e4})
	)

	(segment
    (start {e9})
    (end {e10})
    (width {e2})
    (locked {e3})
    (layer "B.Cu")
    (net {e11})
  )
  (segment
    (start {e12})
    (end {e9})
    (width {e2})
    (locked {e3})
    (layer "B.Cu")
    (net {e11})
  )
  (via
    (at {e12})
		(size {e6})
    (drill {e7})
    (layers "F.Cu" "B.Cu")
    (locked {e3})
    (net {e11})
  )
  (segment
    (start {e13})
    (end {e12})
    (width {e2})
    (locked {e3})
    (layer "F.Cu")
    (net {e11})
  )
    "####,
        e0 = p.eaxy(7.085, -4.415),
        e1 = p.eaxy(7.085, -2.54),
        e2 = n(p.number("trace_width")),
        e3 = if p.flag("locked_traces_vias") {
            "yes".to_string()
        } else {
            "no".to_string()
        },
        e4 = p.net("from").index,
        e5 = p.eaxy(7.75, -5.08),
        e6 = n(p.number("via_size")),
        e7 = n(p.number("via_drill")),
        e8 = p.eaxy(5.842, -5.08),
        e9 = p.eaxy(-7.085, -4.415),
        e10 = p.eaxy(-7.085, -2.54),
        e11 = p.net("to").index,
        e12 = p.eaxy(-7.75, -5.08),
        e13 = p.eaxy(-5.842, -5.08),
    );

    let (side, reversible, hotswap) = (p.side(), p.flag("reversible"), p.flag("hotswap"));
    let mut out = common_top;
    if p.flag("include_corner_marks") {
        out += &corner_marks;
    }
    if p.flag("include_keycap") {
        out += &keycap_marks;
    }
    if hotswap {
        if reversible || side == "F" {
            out += &hotswap_front;
            if p.flag("include_silkscreen") && !reversible {
                out += &hotswap_silkscreen_front;
            }
        }
        if reversible || side == "B" {
            out += &hotswap_back;
            if p.flag("include_silkscreen") && !reversible {
                out += &hotswap_silkscreen_back;
            }
        }
        if !p.text("hotswap_3dmodel_filename").is_empty() {
            out += &hotswap_3dmodel;
        }
        if p.flag("include_silkscreen") && reversible {
            out += &hotswap_silkscreen_reversible;
        }
    }
    if p.flag("solder") {
        if reversible || side == "F" {
            out += &solder_front;
        }
        if reversible || side == "B" {
            out += &solder_back;
        }
    }
    if !p.text("switch_3dmodel_filename").is_empty() {
        out += &switch_3dmodel;
    }
    if !p.text("keycap_3dmodel_filename").is_empty() {
        out += &keycap_3dmodel;
    }
    out += &common_bottom;
    if reversible && hotswap && p.flag("include_traces_vias") && !p.flag("include_plated_holes") {
        out += if p.flag("hotswap_pads_same_side") {
            &hotswap_routes_same_side
        } else {
            &hotswap_routes_unplated
        };
    }
    Ok(out)
}

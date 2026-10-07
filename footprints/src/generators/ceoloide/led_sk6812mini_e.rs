//! `ceoloide/led_sk6812mini-e`
//!
//! Ported from `led_sk6812mini-e.js` of ceoloide/ergogen-footprints.
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

static PARAMS: [ParamSpec; 20] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "LED"),
    ParamSpec::boolean("reversible", false),
    ParamSpec::boolean("reverse_mount", true),
    ParamSpec::boolean("include_traces_vias", true),
    ParamSpec::number("signal_trace_width", 0.25),
    ParamSpec::number("gnd_trace_width", 0.25),
    ParamSpec::number("vcc_trace_width", 0.25),
    ParamSpec::number("via_size", 0.8),
    ParamSpec::number("via_drill", 0.4),
    ParamSpec::boolean("include_courtyard", true),
    ParamSpec::boolean("include_keepout", false),
    ParamSpec::string(
        "led_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/keebio/SK6812MINI-E v1.step",
    ),
    ParamSpec::array("led_3dmodel_xyz_offset", "[0,0,0]"),
    ParamSpec::array("led_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("led_3dmodel_xyz_scale", "[1,1,1]"),
    ParamSpec::net_default("P1", "VCC"),
    ParamSpec::net("P2"),
    ParamSpec::net_default("P3", "GND"),
    ParamSpec::net("P4"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "ceoloide/led_sk6812mini-e",
    display_name: "led sk6812mini e",
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
    // Default orientation follows the mounting variant; an explicit rotation wins.
    let model_rotation = p.vec3("led_3dmodel_xyz_rotation")?.unwrap_or([
        if p.flag("reverse_mount") { -90.0 } else { 90.0 },
        0.0,
        180.0,
    ]);
    for name in ["led_3dmodel_xyz_offset", "led_3dmodel_xyz_scale"] {
        p.vec3(name)?;
    }

    let standard_opening = format!(
        r####"
  (footprint "ceoloide:led_SK6812mini-e ({e0}{e1})" 
    (layer "{e2}.Cu")
    {e3}
    (property "Reference" "{e4}"
      (at -4.75 0 {e5})
      (layer "{e2}.SilkS")
      {e6}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (attr smd)

    (fp_line (start -1.6 -1.4) (end 1.6 -1.4) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start -1.6 1.4) (end 1.6 1.4) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start -1.6 -1.4) (end -1.6 1.4) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start 1.6 -1.4) (end 1.6 1.4) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start -1.6 -1.05) (end -2.94 -1.05) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start -2.94 -1.05) (end -2.94 -0.37) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start -2.94 -0.37) (end -1.6 -0.37) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start -1.6 0.35) (end -2.94 0.35) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start -2.94 1.03) (end -1.6 1.03) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start -2.94 0.35) (end -2.94 1.03) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start 1.6 1.03) (end 2.94 1.03) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start 2.94 0.35) (end 1.6 0.35) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start 2.94 1.03) (end 2.94 0.35) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start 1.6 -0.37) (end 2.94 -0.37) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start 2.94 -1.05) (end 1.6 -1.05) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start 2.94 -0.37) (end 2.94 -1.05) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    "####,
        e0 = if p.flag("reverse_mount") {
            "per-key"
        } else {
            "underglow"
        },
        e1 = if p.flag("reversible") {
            ", reversible"
        } else {
            "single-side"
        },
        e2 = p.side(),
        e3 = p.at(),
        e4 = p.reference(),
        e5 = n(90.0 + p.rotation()),
        e6 = p.ref_hide(),
    );

    let marks_reversed = String::from(
        r####"
    (fp_line (start -0.8 -1.4) (end -0.8 1.4) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start 0.8 -1.4) (end 0.8 1.4) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start -1 -1.4) (end -1 1.4) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start 1 -1.4) (end 1 1.4) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    "####,
    );

    let marks_straight = String::from(
        r####"
    (fp_line (start -1.6 -0.7) (end -0.8 -1.4) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    "####,
    );

    let front_reversed = format!(
        r####"
    (fp_line (start -3.8 1.6) (end -2.2 1.6) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -3.8 0) (end -3.8 1.6) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (pad "4" smd rect (at -2.7 -0.7 {e0}) (size 1.4 1) (layers "F.Cu" "F.Paste" "F.Mask") {e1})
    (pad "3" smd rect (at -2.7 0.7 {e0}) (size 1.4 1) (layers "F.Cu" "F.Paste" "F.Mask") {e2})
    (pad "1" smd rect (at 2.7 -0.7 {e0}) (size 1.4 1) (layers "F.Cu" "F.Paste" "F.Mask") {e3})
    (pad "2" smd rect (at 2.7 0.7 {e0}) (size 1.4 1) (layers "F.Cu" "F.Paste" "F.Mask") {e4})
    "####,
        e0 = n(p.rotation()),
        e1 = p.net("P4"),
        e2 = p.net("P3"),
        e3 = p.net("P1"),
        e4 = p.net("P2"),
    );

    let front = format!(
        r####"
    (fp_line (start -3.8 -1.6) (end -2.2 -1.6) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -3.8 0) (end -3.8 -1.6) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (pad "4" smd rect (at -2.70 0.7 {e0}) (size 1.4 1) (layers "F.Cu" "F.Paste" "F.Mask") {e1})
    (pad "3" smd rect (at -2.70 -0.7 {e0}) (size 1.4 1) (layers "F.Cu" "F.Paste" "F.Mask") {e2})
    (pad "1" smd rect (at 2.70 0.7 {e0}) (size 1.4 1) (layers "F.Cu" "F.Paste" "F.Mask") {e3})
    (pad "2" smd rect (at 2.70 -0.7 {e0}) (size 1.4 1) (layers "F.Cu" "F.Paste" "F.Mask") {e4})
    "####,
        e0 = n(p.rotation()),
        e1 = p.net("P4"),
        e2 = p.net("P3"),
        e3 = p.net("P1"),
        e4 = p.net("P2"),
    );

    let back_reversed = format!(
        r####"
    (fp_line (start -3.8 -1.6) (end -2.2 -1.6) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -3.8 0) (end -3.8 -1.6) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (pad "2" smd rect (at 2.70 -0.7 {e0}) (size 1.4 1) (layers "B.Cu" "B.Paste" "B.Mask") {e1})
    (pad "1" smd rect (at 2.70 0.7 {e0}) (size 1.4 1) (layers "B.Cu" "B.Paste" "B.Mask") {e2})
    (pad "3" smd rect (at -2.70 -0.7 {e0}) (size 1.4 1) (layers "B.Cu" "B.Paste" "B.Mask") {e3})
    (pad "4" smd rect (at -2.70 0.7 {e0}) (size 1.4 1) (layers "B.Cu" "B.Paste" "B.Mask") {e4})
    "####,
        e0 = n(p.rotation()),
        e1 = p.net("P2"),
        e2 = p.net("P1"),
        e3 = p.net("P3"),
        e4 = p.net("P4"),
    );

    let back = format!(
        r####"
    (fp_line (start -3.8 1.6) (end -2.2 1.6) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -3.8 0) (end -3.8 1.6) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (pad "2" smd rect (at 2.70 0.7 {e0}) (size 1.4 1) (layers "B.Cu" "B.Paste" "B.Mask") {e1})
    (pad "1" smd rect (at 2.70 -0.7 {e0}) (size 1.4 1) (layers "B.Cu" "B.Paste" "B.Mask") {e2})
    (pad "3" smd rect (at -2.70 0.7 {e0}) (size 1.4 1) (layers "B.Cu" "B.Paste" "B.Mask") {e3})
    (pad "4" smd rect (at -2.70 -0.7 {e0}) (size 1.4 1) (layers "B.Cu" "B.Paste" "B.Mask") {e4})
    "####,
        e0 = n(p.rotation()),
        e1 = p.net("P2"),
        e2 = p.net("P1"),
        e3 = p.net("P3"),
        e4 = p.net("P4"),
    );

    let standard_closing = String::from(
        r####"
    (fp_rect (start -1.8 -1.55) (end 1.8 1.55) (layer "Edge.Cuts") (stroke (width 0.12) (type solid)) (fill none))
  )
    "####,
    );

    let traces_vias_reversed = format!(
        r####"
  {e0}
  (segment (start {e1}) (end {e2}) (width {e3}) (layer "F.Cu") (net {e4}))
  (segment (start {e2}) (end {e5}) (width {e3}) (layer "F.Cu") (net {e4}))
  (segment (start {e6}) (end {e1}) (width {e3}) (layer "F.Cu") (net {e4}))
  (via (at {e5}) (size {e7}) (drill {e8}) (layers "F.Cu" "B.Cu") (net {e4}))
  (segment (start {e9}) (end {e5}) (width {e3}) (layer "B.Cu") (net {e4}))
  {e10}
  (segment (start {e11}) (end {e6}) (width {e12}) (layer "B.Cu") (net {e13}))
  (via (at {e11}) (size {e7}) (drill {e8}) (layers "F.Cu" "B.Cu") (net {e13}))
  (segment (start {e9}) (end {e14}) (width {e12}) (layer "F.Cu") (net {e13}))
  (segment (start {e14}) (end {e15}) (width {e12}) (layer "F.Cu") (net {e13}))
  (segment (start {e16}) (end {e11}) (width {e12}) (layer "F.Cu") (net {e13}))
  (segment (start {e15}) (end {e16}) (width {e12}) (layer "F.Cu") (net {e13}))
  {e17}
  (segment (start {e18}) (end {e19}) (width {e20}) (layer "B.Cu") (net {e21}))
  (segment (start {e19}) (end {e22}) (width {e20}) (layer "B.Cu") (net {e21}))
  (segment (start {e23}) (end {e18}) (width {e20}) (layer "B.Cu") (net {e21}))
  (via (at {e22}) (size {e7}) (drill {e8}) (layers "F.Cu" "B.Cu") (net {e21}))
  (segment (start {e24}) (end {e22}) (width {e20}) (layer "F.Cu") (net {e21}))
  {e25}
  (segment (start {e26}) (end {e23}) (width {e12}) (layer "F.Cu") (net {e27}))
  (via (at {e26}) (size {e7}) (drill {e8}) (layers "F.Cu" "B.Cu") (net {e27}))
  (segment (start {e24}) (end {e28}) (width {e12}) (layer "B.Cu") (net {e27}))
  (segment (start {e28}) (end {e29}) (width {e12}) (layer "B.Cu") (net {e27}))
  (segment (start {e30}) (end {e26}) (width {e12}) (layer "B.Cu") (net {e27}))
  (segment (start {e29}) (end {e30}) (width {e12}) (layer "B.Cu") (net {e27}))
    "####,
        e0 = "",
        e1 = p.eaxy(3.4, -0.7),
        e2 = p.eaxy(4.06, -0.105916),
        e3 = n(p.number("vcc_trace_width")),
        e4 = p.net("P1").index,
        e5 = p.eaxy(4.06, 0.7),
        e6 = p.eaxy(2.7, -0.7),
        e7 = n(p.number("via_size")),
        e8 = n(p.number("via_drill")),
        e9 = p.eaxy(2.7, 0.7),
        e10 = "",
        e11 = p.eaxy(4.95, -0.7),
        e12 = n(p.number("signal_trace_width")),
        e13 = p.net("P2").index,
        e14 = p.eaxy(3.481, 1.485),
        e15 = p.eaxy(4.529, 1.485),
        e16 = p.eaxy(4.95, 1.06),
        e17 = "",
        e18 = p.eaxy(-3.4, -0.7),
        e19 = p.eaxy(-4.06, -0.105916),
        e20 = n(p.number("gnd_trace_width")),
        e21 = p.net("P3").index,
        e22 = p.eaxy(-4.06, 0.7),
        e23 = p.eaxy(-2.7, -0.7),
        e24 = p.eaxy(-2.7, 0.7),
        e25 = "",
        e26 = p.eaxy(-4.95, -0.7),
        e27 = p.net("P4").index,
        e28 = p.eaxy(-3.481, 1.485),
        e29 = p.eaxy(-4.529, 1.485),
        e30 = p.eaxy(-4.95, 1.06),
    );

    let traces_vias_straight = format!(
        r####"
  {e0}
  (segment (start {e1}) (end {e2}) (width {e3}) (layer "B.Cu") (net {e4}))
  (segment (start {e2}) (end {e5}) (width {e3}) (layer "B.Cu") (net {e4}))
  (segment (start {e6}) (end {e1}) (width {e3}) (layer "B.Cu") (net {e4}))
  (via (at {e5}) (size {e7}) (drill {e8}) (layers "F.Cu" "B.Cu") (net {e4}))
  (segment (start {e9}) (end {e5}) (width {e3}) (layer "F.Cu") (net {e4}))
  {e10}
  (segment (start {e11}) (end {e6}) (width {e12}) (layer "F.Cu") (net {e13}))
  (via (at {e11}) (size {e7}) (drill {e8}) (layers "F.Cu" "B.Cu") (net {e13}))
  (segment (start {e9}) (end {e14}) (width {e12}) (layer "B.Cu") (net {e13}))
  (segment (start {e14}) (end {e15}) (width {e12}) (layer "B.Cu") (net {e13}))
  (segment (start {e16}) (end {e11}) (width {e12}) (layer "B.Cu") (net {e13}))
  (segment (start {e15}) (end {e16}) (width {e12}) (layer "B.Cu") (net {e13}))
  {e17}
  (segment (start {e18}) (end {e19}) (width {e20}) (layer "F.Cu") (net {e21}))
  (segment (start {e19}) (end {e22}) (width {e20}) (layer "F.Cu") (net {e21}))
  (segment (start {e23}) (end {e18}) (width {e20}) (layer "F.Cu") (net {e21}))
  (via (at {e22}) (size {e7}) (drill {e8}) (layers "F.Cu" "B.Cu") (net {e21}))
  (segment (start {e24}) (end {e22}) (width {e20}) (layer "B.Cu") (net {e21}))
  {e25}
  (segment (start {e26}) (end {e23}) (width {e12}) (layer "B.Cu") (net {e27}))
  (via (at {e26}) (size {e7}) (drill {e8}) (layers "F.Cu" "B.Cu") (net {e27}))
  (segment (start {e24}) (end {e28}) (width {e12}) (layer "F.Cu") (net {e27}))
  (segment (start {e28}) (end {e29}) (width {e12}) (layer "F.Cu") (net {e27}))
  (segment (start {e30}) (end {e26}) (width {e12}) (layer "F.Cu") (net {e27}))
  (segment (start {e29}) (end {e30}) (width {e12}) (layer "F.Cu") (net {e27}))
    "####,
        e0 = "",
        e1 = p.eaxy(3.4, -0.7),
        e2 = p.eaxy(4.06, -0.105916),
        e3 = n(p.number("vcc_trace_width")),
        e4 = p.net("P1").index,
        e5 = p.eaxy(4.06, 0.7),
        e6 = p.eaxy(2.7, -0.7),
        e7 = n(p.number("via_size")),
        e8 = n(p.number("via_drill")),
        e9 = p.eaxy(2.7, 0.7),
        e10 = "",
        e11 = p.eaxy(4.95, -0.7),
        e12 = n(p.number("signal_trace_width")),
        e13 = p.net("P2").index,
        e14 = p.eaxy(3.481, 1.485),
        e15 = p.eaxy(4.529, 1.485),
        e16 = p.eaxy(4.95, 1.06),
        e17 = "",
        e18 = p.eaxy(-3.4, -0.7),
        e19 = p.eaxy(-4.06, -0.105916),
        e20 = n(p.number("gnd_trace_width")),
        e21 = p.net("P3").index,
        e22 = p.eaxy(-4.06, 0.7),
        e23 = p.eaxy(-2.7, -0.7),
        e24 = p.eaxy(-2.7, 0.7),
        e25 = "",
        e26 = p.eaxy(-4.95, -0.7),
        e27 = p.net("P4").index,
        e28 = p.eaxy(-3.481, 1.485),
        e29 = p.eaxy(-4.529, 1.485),
        e30 = p.eaxy(-4.95, 1.06),
    );

    let courtyard_front = String::from(
        r####"
    (fp_poly
      (pts
        (xy 1.6 -1.05)
        (xy 2.94 -1.05)
        (xy 2.94 -0.37)
        (xy 1.6 -0.37)
        (xy 1.6 0.35)
        (xy 2.94 0.35)
        (xy 2.94 1.03)
        (xy 1.6 1.03)
        (xy 1.6 1.4)
        (xy -1.6 1.4)
        (xy -1.6 1.03)
        (xy -2.94 1.03)
        (xy -2.94 0.35)
        (xy -1.6 0.35)
        (xy -1.6 -0.37)
        (xy -2.94 -0.37)
        (xy -2.94 -1.05)
        (xy -1.6 -1.05)
        (xy -1.6 -1.4)
        (xy 1.6 -1.4)
      )
			(stroke
				(width 0.1)
				(type solid)
			)
      (layer "B.CrtYd")
    )
    "####,
    );

    let courtyard_back = String::from(
        r####"
    (fp_poly
      (pts
        (xy 1.6 -1.05)
        (xy 2.94 -1.05)
        (xy 2.94 -0.37)
        (xy 1.6 -0.37)
        (xy 1.6 0.35)
        (xy 2.94 0.35)
        (xy 2.94 1.03)
        (xy 1.6 1.03)
        (xy 1.6 1.4)
        (xy -1.6 1.4)
        (xy -1.6 1.03)
        (xy -2.94 1.03)
        (xy -2.94 0.35)
        (xy -1.6 0.35)
        (xy -1.6 -0.37)
        (xy -2.94 -0.37)
        (xy -2.94 -1.05)
        (xy -1.6 -1.05)
        (xy -1.6 -1.4)
        (xy 1.6 -1.4)
      )
			(stroke
				(width 0.1)
				(type solid)
			)
      (layer "B.CrtYd")
    )
    "####,
    );

    let keepout = format!(
        r####"
  (zone
    (net 0)
    (net_name "")
    (layers "F&B.Cu")
    (hatch edge 0.3)
    (connect_pads (clearance 0))
    (min_thickness 0.25)
		(filled_areas_thickness no)
		(keepout
			(tracks not_allowed)
			(vias not_allowed)
			(pads allowed)
			(copperpour not_allowed)
			(footprints allowed)
		)
		(fill
			(thermal_gap 0.5)
			(thermal_bridge_width 0.5)
		)
    (polygon
      (pts
        (xy {e0})
        (xy {e1})
        (xy {e2})
        (xy {e3})
      )
    )
  )
    "####,
        e0 = p.eaxy(-2.00, -1.85),
        e1 = p.eaxy(2.00, -1.85),
        e2 = p.eaxy(2.00, 1.85),
        e3 = p.eaxy(-2.00, 1.85),
    );

    let led_3dmodel = format!(
        r####"
    (model {e0}
      (offset (xyz {e1} {e2} {e3}))
      (scale (xyz {e4} {e5} {e6}))
      (rotate (xyz {e7} {e8} {e9}))
    )
      "####,
        e0 = json_string(p.text("led_3dmodel_filename")),
        e1 = n(p.component("led_3dmodel_xyz_offset", 0)),
        e2 = n(p.component("led_3dmodel_xyz_offset", 1)),
        e3 = n(p.component("led_3dmodel_xyz_offset", 2)),
        e4 = n(p.component("led_3dmodel_xyz_scale", 0)),
        e5 = n(p.component("led_3dmodel_xyz_scale", 1)),
        e6 = n(p.component("led_3dmodel_xyz_scale", 2)),
        e7 = n(model_rotation[0]),
        e8 = n(model_rotation[1]),
        e9 = n(model_rotation[2]),
    );

    let (side, reversible, reverse_mount) =
        (p.side(), p.flag("reversible"), p.flag("reverse_mount"));
    let mut out = standard_opening;
    if side == "F" || reversible {
        if reverse_mount {
            out += &marks_reversed;
            out += &front_reversed;
        } else {
            out += &marks_straight;
            out += &front;
        }
        if p.flag("include_courtyard") {
            out += &courtyard_front;
        }
    }
    if side == "B" || reversible {
        if reverse_mount {
            out += &back_reversed;
            out += &marks_reversed;
        } else {
            out += &marks_straight;
            out += &back;
        }
        if p.flag("include_courtyard") {
            out += &courtyard_back;
        }
    }
    if !p.text("led_3dmodel_filename").is_empty() {
        out += &led_3dmodel;
    }
    out += &standard_closing;
    if p.flag("include_keepout") {
        out += &keepout;
    }
    if reversible && p.flag("include_traces_vias") {
        out += if reverse_mount {
            &traces_vias_reversed
        } else {
            &traces_vias_straight
        };
    }
    Ok(out)
}

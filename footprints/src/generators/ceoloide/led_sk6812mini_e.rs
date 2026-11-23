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
use std::fmt::Write as _;

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
    body: crate::registry::Body::Text(body),
};

fn push_led_segment(out: &mut String, start: &str, end: &str, width: &str, layer: &str, net: u32) {
    let _ = write!(
        out,
        "\n  (segment (start {start}) (end {end}) (width {width}) (layer \"{layer}.Cu\") (net {net}))"
    );
}

fn push_led_via(out: &mut String, at: &str, size: &str, drill: &str, net: u32) {
    let _ = write!(
        out,
        "\n  (via (at {at}) (size {size}) (drill {drill}) (layers \"F.Cu\" \"B.Cu\") (net {net}))"
    );
}

fn led_traces(p: &RenderContext<'_>, reverse_mount: bool) -> String {
    let (top, bottom) = if reverse_mount {
        ("F", "B")
    } else {
        ("B", "F")
    };
    let vcc = n(p.number("vcc_trace_width"));
    let signal = n(p.number("signal_trace_width"));
    let gnd = n(p.number("gnd_trace_width"));
    let size = n(p.number("via_size"));
    let drill = n(p.number("via_drill"));
    let (p1, p2, p3, p4) = (
        p.net("P1").index,
        p.net("P2").index,
        p.net("P3").index,
        p.net("P4").index,
    );
    let mut out = String::from("\n  \n");
    for (s, e) in [
        ((3.4, -0.7), (4.06, -0.105916)),
        ((4.06, -0.105916), (4.06, 0.7)),
        ((2.7, -0.7), (3.4, -0.7)),
    ] {
        push_led_segment(
            &mut out,
            &p.eaxy(s.0, s.1),
            &p.eaxy(e.0, e.1),
            &vcc,
            top,
            p1,
        );
    }
    push_led_via(&mut out, &p.eaxy(4.06, 0.7), &size, &drill, p1);
    push_led_segment(
        &mut out,
        &p.eaxy(2.7, 0.7),
        &p.eaxy(4.06, 0.7),
        &vcc,
        bottom,
        p1,
    );
    out.push_str("\n  \n");
    push_led_segment(
        &mut out,
        &p.eaxy(4.95, -0.7),
        &p.eaxy(2.7, -0.7),
        &signal,
        bottom,
        p2,
    );
    push_led_via(&mut out, &p.eaxy(4.95, -0.7), &size, &drill, p2);
    for (s, e) in [
        ((2.7, 0.7), (3.481, 1.485)),
        ((3.481, 1.485), (4.529, 1.485)),
        ((4.95, 1.06), (4.95, -0.7)),
        ((4.529, 1.485), (4.95, 1.06)),
    ] {
        push_led_segment(
            &mut out,
            &p.eaxy(s.0, s.1),
            &p.eaxy(e.0, e.1),
            &signal,
            top,
            p2,
        );
    }
    out.push_str("\n  \n");
    for (s, e) in [
        ((-3.4, -0.7), (-4.06, -0.105916)),
        ((-4.06, -0.105916), (-4.06, 0.7)),
        ((-2.7, -0.7), (-3.4, -0.7)),
    ] {
        push_led_segment(
            &mut out,
            &p.eaxy(s.0, s.1),
            &p.eaxy(e.0, e.1),
            &gnd,
            bottom,
            p3,
        );
    }
    push_led_via(&mut out, &p.eaxy(-4.06, 0.7), &size, &drill, p3);
    push_led_segment(
        &mut out,
        &p.eaxy(-2.7, 0.7),
        &p.eaxy(-4.06, 0.7),
        &gnd,
        top,
        p3,
    );
    out.push_str("\n  \n");
    push_led_segment(
        &mut out,
        &p.eaxy(-4.95, -0.7),
        &p.eaxy(-2.7, -0.7),
        &signal,
        top,
        p4,
    );
    push_led_via(&mut out, &p.eaxy(-4.95, -0.7), &size, &drill, p4);
    for (s, e) in [
        ((-2.7, 0.7), (-3.481, 1.485)),
        ((-3.481, 1.485), (-4.529, 1.485)),
        ((-4.95, 1.06), (-4.95, -0.7)),
        ((-4.529, 1.485), (-4.95, 1.06)),
    ] {
        push_led_segment(
            &mut out,
            &p.eaxy(s.0, s.1),
            &p.eaxy(e.0, e.1),
            &signal,
            bottom,
            p4,
        );
    }
    out.push_str("\n    ");
    out
}

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
    (property "Reference" {e4}
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
        e4 = json_string(p.reference()),
        e5 = n(90.0 + p.rotation()),
        e6 = p.ref_hide(),
    );

    const MARKS_REVERSED: &str = r####"
    (fp_line (start -0.8 -1.4) (end -0.8 1.4) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start 0.8 -1.4) (end 0.8 1.4) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start -1 -1.4) (end -1 1.4) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start 1 -1.4) (end 1 1.4) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    "####;

    const MARKS_STRAIGHT: &str = r####"
    (fp_line (start -1.6 -0.7) (end -0.8 -1.4) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    "####;

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

    const STANDARD_CLOSING: &str = r####"
    (fp_rect (start -1.8 -1.55) (end 1.8 1.55) (layer "Edge.Cuts") (stroke (width 0.12) (type solid)) (fill none))
  )
    "####;

    const COURTYARD: &str = r####"
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
    "####;

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
            out += MARKS_REVERSED;
            out += &front_reversed;
        } else {
            out += MARKS_STRAIGHT;
            out += &front;
        }
        if p.flag("include_courtyard") {
            out += COURTYARD;
        }
    }
    if side == "B" || reversible {
        if reverse_mount {
            out += &back_reversed;
            out += MARKS_REVERSED;
        } else {
            out += MARKS_STRAIGHT;
            out += &back;
        }
        if p.flag("include_courtyard") {
            out += COURTYARD;
        }
    }
    if !p.text("led_3dmodel_filename").is_empty() {
        out += &led_3dmodel;
    }
    out += STANDARD_CLOSING;
    if p.flag("include_keepout") {
        out += &keepout;
    }
    if reversible && p.flag("include_traces_vias") {
        out += &led_traces(p, reverse_mount);
    }
    Ok(out)
}

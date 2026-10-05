//! `ceoloide/diode_tht_sod123`
//!
//! Ported from `diode_tht_sod123.js` of ceoloide/ergogen-footprints.
//! Copyright (c) 2023 Marco Massarelli
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0
//! Authors: @ergogen + (@infused-kim, @ceoloide, @achamian, @im-AMS improvements)
use crate::Result;
use crate::context::RenderContext;
use crate::error::GeneratorError;
use crate::generators::util::json_string;
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 16] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "D"),
    ParamSpec::boolean("reversible", false),
    ParamSpec::boolean("include_traces_vias", false),
    ParamSpec::number("trace_distance", 1.2),
    ParamSpec::number("trace_width", 0.25),
    ParamSpec::number("via_size", 0.6),
    ParamSpec::number("via_drill", 0.3),
    ParamSpec::boolean("include_tht", false),
    ParamSpec::boolean("include_thru_hole_smd_pads", false),
    ParamSpec::string(
        "diode_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/Diode_1N4148W.step",
    ),
    ParamSpec::array("diode_3dmodel_xyz_offset", "[0,0,0.7]"),
    ParamSpec::array("diode_3dmodel_xyz_rotation", "[-90,0,0]"),
    ParamSpec::array("diode_3dmodel_xyz_scale", "[1,1,1]"),
    ParamSpec::net("from"),
    ParamSpec::net("to"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "ceoloide/diode_tht_sod123",
    display_name: "diode tht sod123",
    kind: PartKind::Passive,
    matrix_terminals: None,
    keycap_parameters: None,
    parameters: &PARAMS,
    license: License {
        spdx: "CC-BY-NC-SA-4.0",
        author: "@ergogen + (@infused-kim, @ceoloide, @achamian, @im-AMS improvements)",
    },
    body,
};

fn body(p: &RenderContext<'_>) -> Result<String> {
    if p.flag("include_thru_hole_smd_pads") && !p.flag("reversible") {
        return Err(GeneratorError::rejected(
            SPEC.source,
            Some("include_thru_hole_smd_pads"),
            "diode_tht_sod123: include_thru_hole_smd_pads requires reversible: true",
        ));
    }

    let standard_opening = format!(
        r####"
    (footprint "ceoloide:diode_tht_sod123"
        (layer "{e0}.Cu")
        {e1}
        (property "Reference" "{e2}"
            (at 0 0 {e3})
            (layer "{e0}.SilkS")
            {e4}
            (effects (font (size 1 1) (thickness 0.15)))
        )
        "####,
        e0 = if p.flag("reversible") { "F" } else { p.side() },
        e1 = p.at(),
        e2 = p.reference(),
        e3 = n(p.rotation()),
        e4 = p.ref_hide(),
    );

    let thru_hole_smd_pads_description = String::from(
        r####"
      (property "Description" "Thru-hole SMD pads, *NOT* via-in-pad (do not plug or tent)."
        (at 0 0 0)
        (unlocked yes)
        (layer "F.Fab")
        (hide yes)
        (effects
          (font
            (size 1.27 1.27)
          )
        )
      )
    "####,
    );

    let front_silk = String::from(
        r####"
      (fp_line (start 0.25 0) (end 0.75 0) (layer "F.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start 0.25 0.4) (end -0.35 0) (layer "F.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start 0.25 -0.4) (end 0.25 0.4) (layer "F.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start -0.35 0) (end 0.25 -0.4) (layer "F.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start -0.35 0) (end -0.35 0.55) (layer "F.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start -0.35 0) (end -0.35 -0.55) (layer "F.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start -0.75 0) (end -0.35 0) (layer "F.SilkS") (stroke (width 0.1) (type solid)))
        "####,
    );

    let front_smd_pads = format!(
        r####"
      (pad "1" smd rect (at -1.65 0 {e0}) (size 0.9 1.2) (layers "F.Cu" "F.Paste" "F.Mask") {e1})
      (pad "2" smd rect (at 1.65 0 {e0}) (size 0.9 1.2) (layers "F.Cu" "F.Paste" "F.Mask") {e2})
        "####,
        e0 = n(p.rotation()),
        e1 = p.net("to"),
        e2 = p.net("from"),
    );

    let back_silk = String::from(
        r####"
      (fp_line (start 0.25 0) (end 0.75 0) (layer "B.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start 0.25 0.4) (end -0.35 0) (layer "B.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start 0.25 -0.4) (end 0.25 0.4) (layer "B.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start -0.35 0) (end 0.25 -0.4) (layer "B.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start -0.35 0) (end -0.35 0.55) (layer "B.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start -0.35 0) (end -0.35 -0.55) (layer "B.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start -0.75 0) (end -0.35 0) (layer "B.SilkS") (stroke (width 0.1) (type solid)))
        "####,
    );

    let back_smd_pads = format!(
        r####"
      (pad "1" smd rect (at -1.65 0 {e0}) (size 0.9 1.2) (layers "B.Cu" "B.Paste" "B.Mask") {e1})
      (pad "2" smd rect (at 1.65 0 {e0}) (size 0.9 1.2) (layers "B.Cu" "B.Paste" "B.Mask") {e2})
        "####,
        e0 = n(p.rotation()),
        e1 = p.net("to"),
        e2 = p.net("from"),
    );

    let reversible_tht_pads = format!(
        r####"
      (pad "1" thru_hole rect (at -1.65 0 {e0}) (size 0.9 1.2) (drill 0.3) (layers "*.Cu" "*.Paste" "*.Mask") {e1})
      (pad "2" thru_hole rect (at 1.65 0 {e0}) (size 0.9 1.2) (drill 0.3) (layers "*.Cu" "*.Paste" "*.Mask") {e2})
        "####,
        e0 = n(p.rotation()),
        e1 = p.net("to"),
        e2 = p.net("from"),
    );

    let tht = format!(
        r####"
      (pad "1" thru_hole rect (at -3.81 0 {e0}) (size 1.778 1.778) (drill 0.9906) (layers "*.Cu" "*.Mask") {e1})
      (pad "2" thru_hole circle (at 3.81 0 {e0}) (size 1.905 1.905) (drill 0.9906) (layers "*.Cu" "*.Mask") {e2})
        "####,
        e0 = n(p.rotation()),
        e1 = p.net("to"),
        e2 = p.net("from"),
    );

    let diode_3dmodel = format!(
        r####"
      (model {e0}
          (offset (xyz {e1} {e2} {e3}))
          (scale (xyz {e4} {e5} {e6}))
          (rotate (xyz {e7} {e8} {e9})))
        "####,
        e0 = json_string(p.text("diode_3dmodel_filename")),
        e1 = n(p.component("diode_3dmodel_xyz_offset", 0)),
        e2 = n(p.component("diode_3dmodel_xyz_offset", 1)),
        e3 = n(p.component("diode_3dmodel_xyz_offset", 2)),
        e4 = n(p.component("diode_3dmodel_xyz_scale", 0)),
        e5 = n(p.component("diode_3dmodel_xyz_scale", 1)),
        e6 = n(p.component("diode_3dmodel_xyz_scale", 2)),
        e7 = n(p.component("diode_3dmodel_xyz_rotation", 0)),
        e8 = n(p.component("diode_3dmodel_xyz_rotation", 1)),
        e9 = n(p.component("diode_3dmodel_xyz_rotation", 2)),
    );

    let standard_closing = String::from(
        r####"
    )
        "####,
    );

    let tht_traces = format!(
        r####"
    (segment
      (start {e0})
      (end {e1})
      (width {e2})
      (layer "F.Cu")
      (net {e3})
    )
    (segment
      (start {e0})
      (end {e1})
      (width {e2})
      (layer "B.Cu")
      (net {e3})
    )
    (segment
      (start {e4})
      (end {e5})
      (width {e2})
      (layer "F.Cu")
      (net {e6})
    )
    (segment
      (start {e4})
      (end {e5})
      (width {e2})
      (layer "B.Cu")
      (net {e6})
    )
    "####,
        e0 = p.eaxy(3.81, 0.0),
        e1 = p.eaxy(1.65, 0.0),
        e2 = n(p.number("trace_width")),
        e3 = p.net("from").index,
        e4 = p.eaxy(-1.65, 0.0),
        e5 = p.eaxy(-3.81, 0.0),
        e6 = p.net("to").index,
    );

    let smd_pad_traces = format!(
        r####"
    (segment
      (start {e0})
      (end {e1})
      (width {e2})
      (layer "F.Cu")
      (net {e3})
    )
    (via
      (at {e1})
      (size {e4})
      (drill {e5})
      (layers "F.Cu" "B.Cu")
      (net {e3})
    )
    (segment
      (start {e1})
      (end {e0})
      (width {e2})
      (layer "B.Cu")
      (net {e3})
    )
    (segment
      (start {e6})
      (end {e7})
      (width {e2})
      (layer "F.Cu")
      (net {e8})
    )
    (via
      (at {e7})
      (size {e4})
      (drill {e5})
      (layers "F.Cu" "B.Cu")
      (net {e8})
    )
    (segment
      (start {e7})
      (end {e6})
      (width {e2})
      (layer "B.Cu")
      (net {e8})
    )
    "####,
        e0 = p.eaxy(1.65, 0.0),
        e1 = p.eaxy(1.65 + 1.0 * p.number("trace_distance"), 0.0),
        e2 = n(p.number("trace_width")),
        e3 = p.net("from").index,
        e4 = n(p.number("via_size")),
        e5 = n(p.number("via_drill")),
        e6 = p.eaxy(-1.65, 0.0),
        e7 = p.eaxy(-1.65 - 1.0 * p.number("trace_distance"), 0.0),
        e8 = p.net("to").index,
    );

    let (side, reversible) = (p.side(), p.flag("reversible"));
    let hole_pads = p.flag("include_thru_hole_smd_pads");
    let mut out = standard_opening;
    if side == "F" || reversible {
        out += &front_silk;
        if !hole_pads {
            out += &front_smd_pads;
        }
    }
    if side == "B" || reversible {
        out += &back_silk;
        if !hole_pads {
            out += &back_smd_pads;
        }
    }
    if p.flag("include_tht") {
        out += &tht;
    }
    if reversible && hole_pads {
        out += &thru_hole_smd_pads_description;
        out += &reversible_tht_pads;
    }
    if !p.text("diode_3dmodel_filename").is_empty() {
        for name in [
            "diode_3dmodel_xyz_offset",
            "diode_3dmodel_xyz_scale",
            "diode_3dmodel_xyz_rotation",
        ] {
            p.vec3(name)?;
        }
        out += &diode_3dmodel;
    }
    out += &standard_closing;
    if reversible && p.flag("include_traces_vias") {
        if p.flag("include_tht") {
            out += &tht_traces;
        } else if !hole_pads {
            out += &smd_pad_traces;
        }
    }
    Ok(out)
}

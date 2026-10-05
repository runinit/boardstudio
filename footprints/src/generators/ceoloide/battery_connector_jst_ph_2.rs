//! `ceoloide/battery_connector_jst_ph_2`
//!
//! Ported from `battery_connector_jst_ph_2.js` of ceoloide/ergogen-footprints.
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

static PARAMS: [ParamSpec; 14] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "JST"),
    ParamSpec::boolean("reversible", false),
    ParamSpec::boolean("include_traces", true),
    ParamSpec::number("trace_width", 0.25),
    ParamSpec::boolean("include_silkscreen", true),
    ParamSpec::boolean("include_fabrication", true),
    ParamSpec::boolean("include_courtyard", true),
    ParamSpec::string(
        "battery_connector_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/kicad/JST_PH_S2B-PH-K_1x02_P2.00mm_Horizontal.step",
    ),
    ParamSpec::array("battery_connector_3dmodel_xyz_offset", "[]"),
    ParamSpec::array("battery_connector_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("battery_connector_3dmodel_xyz_scale", "[1,1,1]"),
    ParamSpec::net_default("BAT_P", "BAT_P"),
    ParamSpec::net_default("BAT_N", "GND"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "ceoloide/battery_connector_jst_ph_2",
    display_name: "battery connector jst ph 2",
    kind: PartKind::Connector,
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
    let local_nets = [p.local_net("1")?, p.local_net("2")?];
    let back = p.side() == "B";
    p.vec3("battery_connector_3dmodel_xyz_scale")?;
    let model_offset = p.vec3("battery_connector_3dmodel_xyz_offset")?.unwrap_or([
        if back { 1.0 } else { -1.0 },
        0.0,
        0.0,
    ]);
    let model_rotation = p
        .vec3("battery_connector_3dmodel_xyz_rotation")?
        .unwrap_or([0.0, 0.0, if back { 180.0 } else { 0.0 }]);
    let standard_opening = format!(
        r####"
    (footprint "ceoloide:battery_connector_jst_ph_2"
        (layer "{e0}.Cu")
        {e1}
        (property "Reference" "{e2}"
            (at 0 4.8 {e3})
            (layer "{e0}.SilkS")
            {e4}
            (effects (font (size 1 1) (thickness 0.15)))
        )
        "####,
        e0 = p.side(),
        e1 = p.at(),
        e2 = p.reference(),
        e3 = n(p.rotation()),
        e4 = p.ref_hide()
    );

    let front_fabrication = String::from(
        r####"
        (fp_line (start -2.95 -1.35) (end -2.95 6.25) (stroke (width 0.1) (type solid)) (layer "F.Fab"))
        (fp_line (start -2.95 6.25) (end 2.95 6.25) (stroke (width 0.1) (type solid)) (layer "F.Fab"))
        (fp_line (start -2.25 -1.35) (end -2.95 -1.35) (stroke (width 0.1) (type solid)) (layer "F.Fab"))
        (fp_line (start -2.25 0.25) (end -2.25 -1.35) (stroke (width 0.1) (type solid)) (layer "F.Fab"))
        (fp_line (start 2.25 -1.35) (end 2.25 0.25) (stroke (width 0.1) (type solid)) (layer "F.Fab"))
        (fp_line (start 2.25 0.25) (end -2.25 0.25) (stroke (width 0.1) (type solid)) (layer "F.Fab"))
        (fp_line (start 2.95 -1.35) (end 2.25 -1.35) (stroke (width 0.1) (type solid)) (layer "F.Fab"))
        (fp_line (start 2.95 6.25) (end 2.95 -1.35) (stroke (width 0.1) (type solid)) (layer "F.Fab"))
        "####,
    );

    let front_courtyard = String::from(
        r####"
        (fp_line (start -3.45 -1.85) (end -3.45 10.5) (stroke (width 0.05) (type solid)) (layer "F.CrtYd"))
        (fp_line (start -3.45 10.5) (end 3.45 10.5) (stroke (width 0.05) (type solid)) (layer "F.CrtYd"))
        (fp_line (start 3.45 -1.85) (end -3.45 -1.85) (stroke (width 0.05) (type solid)) (layer "F.CrtYd"))
        (fp_line (start 3.45 10.5) (end 3.45 -1.85) (stroke (width 0.05) (type solid)) (layer "F.CrtYd"))
        "####,
    );

    let front_silkscreen = String::from(
        r####"
        (fp_line (start -1.5 7.40) (end -0.5 7.40) (stroke (width 0.1) (type solid)) (layer "F.SilkS"))
        (fp_line (start 1.5 7.40) (end 0.5 7.40) (stroke (width 0.1) (type solid)) (layer "F.SilkS"))
        (fp_line (start 1 6.90) (end 1 7.90) (stroke (width 0.1) (type solid)) (layer "F.SilkS"))
        (fp_line (start -2.06 -1.46) (end -3.06 -1.46) (stroke (width 0.12) (type solid)) (layer "F.SilkS"))
        (fp_line (start -3.06 -1.46) (end -3.06 -0.46) (stroke (width 0.12) (type solid)) (layer "F.SilkS"))
        (fp_line (start 2.14 -1.46) (end 3.06 -1.46) (stroke (width 0.12) (type solid)) (layer "F.SilkS"))
        (fp_line (start 3.06 -1.46) (end 3.06 -0.46) (stroke (width 0.12) (type solid)) (layer "F.SilkS"))
        (fp_line (start -2.14 6.36) (end -3.06 6.36) (stroke (width 0.12) (type solid)) (layer "F.SilkS"))
        (fp_line (start -3.06 6.36) (end -3.06 5.36) (stroke (width 0.12) (type solid)) (layer "F.SilkS"))
        (fp_line (start 2.14 6.36) (end 3.06 6.36) (stroke (width 0.12) (type solid)) (layer "F.SilkS"))
        (fp_line (start 3.06 6.36) (end 3.06 5.36) (stroke (width 0.12) (type solid)) (layer "F.SilkS"))
        "####,
    );

    let back_fabrication = String::from(
        r####"
        (fp_line (start -2.95 -1.35) (end -2.25 -1.35) (stroke (width 0.1) (type solid)) (layer "B.Fab"))
        (fp_line (start -2.95 6.25) (end -2.95 -1.35) (stroke (width 0.1) (type solid)) (layer "B.Fab"))
        (fp_line (start -2.25 -1.35) (end -2.25 0.25) (stroke (width 0.1) (type solid)) (layer "B.Fab"))
        (fp_line (start -2.25 0.25) (end 2.25 0.25) (stroke (width 0.1) (type solid)) (layer "B.Fab"))
        (fp_line (start 2.25 -1.35) (end 2.95 -1.35) (stroke (width 0.1) (type solid)) (layer "B.Fab"))
        (fp_line (start 2.25 0.25) (end 2.25 -1.35) (stroke (width 0.1) (type solid)) (layer "B.Fab"))
        (fp_line (start 2.95 -1.35) (end 2.95 6.25) (stroke (width 0.1) (type solid)) (layer "B.Fab"))
        (fp_line (start 2.95 6.25) (end -2.95 6.25) (stroke (width 0.1) (type solid)) (layer "B.Fab"))
        "####,
    );

    let back_courtyard = String::from(
        r####"
        (fp_line (start -3.45 -1.85) (end -3.45 10.5) (stroke (width 0.05) (type solid)) (layer "B.CrtYd"))
        (fp_line (start -3.45 10.5) (end 3.45 10.5) (stroke (width 0.05) (type solid)) (layer "B.CrtYd"))
        (fp_line (start 3.45 -1.85) (end -3.45 -1.85) (stroke (width 0.05) (type solid)) (layer "B.CrtYd"))
        (fp_line (start 3.45 10.5) (end 3.45 -1.85) (stroke (width 0.05) (type solid)) (layer "B.CrtYd"))
        "####,
    );

    let back_silkscreen = String::from(
        r####"
        (fp_line (start 1.5 7.40) (end 0.5 7.40) (stroke (width 0.1) (type solid)) (layer "B.SilkS"))
        (fp_line (start -1.5 7.40) (end -0.5 7.40) (stroke (width 0.1) (type solid)) (layer "B.SilkS"))
        (fp_line (start -1 6.90) (end -1 7.90) (stroke (width 0.1) (type solid)) (layer "B.SilkS"))
        (fp_line (start -2.06 -1.46) (end -3.06 -1.46) (stroke (width 0.12) (type solid)) (layer "B.SilkS"))
        (fp_line (start -3.06 -1.46) (end -3.06 -0.46) (stroke (width 0.12) (type solid)) (layer "B.SilkS"))
        (fp_line (start 2.14 -1.46) (end 3.06 -1.46) (stroke (width 0.12) (type solid)) (layer "B.SilkS"))
        (fp_line (start 3.06 -1.46) (end 3.06 -0.46) (stroke (width 0.12) (type solid)) (layer "B.SilkS"))
        (fp_line (start -2.14 6.36) (end -3.06 6.36) (stroke (width 0.12) (type solid)) (layer "B.SilkS"))
        (fp_line (start -3.06 6.36) (end -3.06 5.36) (stroke (width 0.12) (type solid)) (layer "B.SilkS"))
        (fp_line (start 2.14 6.36) (end 3.06 6.36) (stroke (width 0.12) (type solid)) (layer "B.SilkS"))
        (fp_line (start 3.06 6.36) (end 3.06 5.36) (stroke (width 0.12) (type solid)) (layer "B.SilkS"))
        "####,
    );

    let front_pads = format!(
        r####"
        (pad "1" thru_hole roundrect (at -1 0 {e0}) (size 1.2 1.75) (drill 0.75) (layers "*.Cu" "*.Mask") (roundrect_rratio 0.20) {e1})
        (pad "2" thru_hole oval (at 1 0 {e0}) (size 1.2 1.75) (drill 0.75) (layers "*.Cu" "*.Mask") {e2})
        "####,
        e0 = n(p.rotation()),
        e1 = p.net("BAT_N"),
        e2 = p.net("BAT_P")
    );

    let back_pads = format!(
        r####"
        (pad "1" thru_hole roundrect (at 1 0 {e0}) (size 1.2 1.75) (drill 0.75) (layers "*.Cu" "*.Mask") (roundrect_rratio 0.20) {e1})
        (pad "2" thru_hole oval (at -1 0 {e0}) (size 1.2 1.75) (drill 0.75) (layers "*.Cu" "*.Mask") {e2})
        "####,
        e0 = n(p.rotation()),
        e1 = p.net("BAT_N"),
        e2 = p.net("BAT_P")
    );

    let reversible_pads = format!(
        r####"
        (pad "11" thru_hole oval (at -1 0 {e0}) (size 1.2 1.75) (drill 0.75) (layers "*.Cu" "*.Mask") {e1})
        (pad "12" thru_hole oval (at 1 0 {e0}) (size 1.2 1.75) (drill 0.75) (layers "*.Cu" "*.Mask") {e2})
        (pad "21" smd custom (at -1 1.8 {e3}) (size 0.1 0.1) (layers "F.Cu" "F.Mask" "F.Paste")
            (clearance 0.1) (zone_connect 0)
            (options (clearance outline) (anchor rect))
            (primitives
                (gr_poly
                    (pts
                        (xy 0.6 0.4)
                        (xy -0.6 0.4)
                        (xy -0.6 0.2)
                        (xy 0 -0.4)
                        (xy 0.6 0.2)
                    )   
                    (width 0)
                    (fill yes)
                )
            )
            {e4}
        )
        (pad "31" smd custom (at -1 1.8 {e3}) (size 0.1 0.1) (layers "B.Cu" "B.Mask" "B.Paste")
            (clearance 0.1) (zone_connect 0)
            (options (clearance outline) (anchor rect))
            (primitives
                (gr_poly
                    (pts
                        (xy 0.6 0.4)
                        (xy -0.6 0.4)
                        (xy -0.6 0.2)
                        (xy 0 -0.4)
                        (xy 0.6 0.2)
                    )
                    (width 0)
                    (fill yes)
                )
            )
            {e4}
        )
        (pad "22" smd custom (at 1 1.8 {e3}) (size 0.1 0.1) (layers "F.Cu" "F.Mask" "F.Paste")
            (clearance 0.1) (zone_connect 0)
            (options (clearance outline) (anchor rect))
            (primitives
                (gr_poly
                    (pts
                        (xy 0.6 0.4)
                        (xy -0.6 0.4)
                        (xy -0.6 0.2)
                        (xy 0 -0.4)
                        (xy 0.6 0.2)
                    )
                    (width 0)
                    (fill yes)
                )
            )
            {e5}
        )
        (pad "32" smd custom (at 1 1.8 {e3}) (size 0.1 0.1) (layers "B.Cu" "B.Mask" "B.Paste")
            (clearance 0.1) (zone_connect 0)
            (options (clearance outline) (anchor rect))
            (primitives
                (gr_poly
                    (pts
                        (xy 0.6 0.4)
                        (xy -0.6 0.4)
                        (xy -0.6 0.2)
                        (xy 0 -0.4)
                        (xy 0.6 0.2)
                    )
                    (width 0)
                    (fill yes)
                )
            )
            {e5}
        )
        (pad "1" smd custom (at -1 2.816 {e3}) (size 1.2 0.5) (layers "F.Cu" "F.Mask" "F.Paste") {e6}
            (clearance 0.1) (zone_connect 0)
            (options (clearance outline) (anchor rect))
            (primitives
                (gr_poly
                    (pts
                        (xy 0.6 0)
                        (xy -0.6 0)
                        (xy -0.6 1)
                        (xy 0 0.4)
                        (xy 0.6 1)
                    )
                    (width 0)
                    (fill yes)
                )
            )
        )
        (pad "1" smd custom (at 1 2.816 {e3}) (size 1.2 0.5) (layers "B.Cu" "B.Mask" "B.Paste") {e6}
            (clearance 0.1) (zone_connect 0)
            (options (clearance outline) (anchor rect))
            (primitives
                (gr_poly
                    (pts
                        (xy 0.6 0)
                        (xy -0.6 0)
                        (xy -0.6 1)
                        (xy 0 0.4)
                        (xy 0.6 1)
                    )
                    (width 0)
                    (fill yes)
                )
            )
        )
        (pad "2" smd custom (at -1 2.816 {e3}) (size 1.2 0.5) (layers "B.Cu" "B.Mask" "B.Paste") {e7}
            (clearance 0.1) (zone_connect 0)
            (options (clearance outline) (anchor rect))
            (primitives
                (gr_poly
                    (pts
                        (xy 0.6 0)
                        (xy -0.6 0)
                        (xy -0.6 1)
                        (xy 0 0.4)
                        (xy 0.6 1)
                    )
                    (width 0)
                    (fill yes)
                )
            )
        )
        (pad "2" smd custom (at 1 2.816 {e3}) (size 1.2 0.5) (layers "F.Cu" "F.Mask" "F.Paste") {e7}
            (clearance 0.1) (zone_connect 0)
            (options (clearance outline) (anchor rect))
            (primitives
                (gr_poly
                    (pts
                        (xy 0.6 0)
                        (xy -0.6 0)
                        (xy -0.6 1)
                        (xy 0 0.4)
                        (xy 0.6 1)
                    )
                    (width 0)
                    (fill yes)
                )
            ) 
        )
        "####,
        e0 = n(p.rotation()),
        e1 = local_nets[0],
        e2 = local_nets[1],
        e3 = n(180.0 + p.rotation()),
        e4 = local_nets[0],
        e5 = local_nets[1],
        e6 = p.net("BAT_P"),
        e7 = p.net("BAT_N")
    );

    let standard_closing = String::from(
        r####"
    )
        "####,
    );

    let reversible_traces = format!(
        r####" 
    (segment (start {e0}) (end {e1}) (width {e2}) (layer "F.Cu") (net {e3}))
    (segment (start {e0}) (end {e1}) (width {e2}) (layer "B.Cu") (net {e3}))
    (segment (start {e4}) (end {e5}) (width {e2}) (layer "F.Cu") (net {e6}))
    (segment (start {e4}) (end {e5}) (width {e2}) (layer "B.Cu") (net {e6}))
        "####,
        e0 = p.eaxy(-1.0, 1.8),
        e1 = p.eaxy(-1.0, 0.0),
        e2 = n(p.number("trace_width")),
        e3 = local_nets[0].index,
        e4 = p.eaxy(1.0, 1.8),
        e5 = p.eaxy(1.0, 0.0),
        e6 = local_nets[1].index
    );

    let battery_connector_3dmodel = format!(
        r####"
    (model {e0}
      (offset (xyz {e1} {e2} {e3}))
      (scale (xyz {e4} {e5} {e6}))
      (rotate (xyz {e7} {e8} {e9}))
    )
    "####,
        e0 = json_string(p.text("battery_connector_3dmodel_filename")),
        e1 = n(model_offset[0]),
        e2 = n(model_offset[1]),
        e3 = n(model_offset[2]),
        e4 = n(p.component("battery_connector_3dmodel_xyz_scale", 0)),
        e5 = n(p.component("battery_connector_3dmodel_xyz_scale", 1)),
        e6 = n(p.component("battery_connector_3dmodel_xyz_scale", 2)),
        e7 = n(model_rotation[0]),
        e8 = n(model_rotation[1]),
        e9 = n(model_rotation[2])
    );

    let (side, reversible) = (p.side(), p.flag("reversible"));
    let mut out = standard_opening;
    if side == "F" || reversible {
        if p.flag("include_fabrication") {
            out += &front_fabrication;
        }
        if p.flag("include_courtyard") {
            out += &front_courtyard;
        }
        if p.flag("include_silkscreen") {
            out += &front_silkscreen;
        }
    }
    if side == "B" || reversible {
        if p.flag("include_fabrication") {
            out += &back_fabrication;
        }
        if p.flag("include_courtyard") {
            out += &back_courtyard;
        }
        if p.flag("include_silkscreen") {
            out += &back_silkscreen;
        }
    }
    if reversible {
        out += &reversible_pads;
    } else if side == "F" {
        out += &front_pads;
    } else {
        out += &back_pads;
    }
    if !p.text("battery_connector_3dmodel_filename").is_empty() {
        out += &battery_connector_3dmodel;
    }
    out += &standard_closing;
    if reversible && p.flag("include_traces") {
        out += &reversible_traces;
    }
    Ok(out)
}

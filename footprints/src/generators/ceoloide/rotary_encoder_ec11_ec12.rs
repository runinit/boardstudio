//! `ceoloide/rotary_encoder_ec11_ec12`
//!
//! Ported from `rotary_encoder_ec11_ec12.js` of ceoloide/ergogen-footprints.
//! Copyright (c) 2023 Marco Massarelli
//! SPDX-License-Identifier: MIT
//! Author: @ceoloide
use crate::Result;
use crate::context::RenderContext;
use crate::error::GeneratorError;
use crate::generators::util::json_string;
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 22] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "RE"),
    ParamSpec::boolean("reversible", false),
    ParamSpec::boolean("include_momentary_switch_pads", true),
    ParamSpec::boolean("include_plate_hole_marking", false),
    ParamSpec::boolean("include_silkscreen", true),
    ParamSpec::boolean("include_plated_mounting_holes", true),
    ParamSpec::number("mounting_holes_position", 5.6),
    ParamSpec::number("mounting_holes_height", 2.3),
    ParamSpec::number("mounting_holes_width", 1.5),
    ParamSpec::number("encoder_pads_position", 7.5),
    ParamSpec::number("signal_hole_width", 1.0),
    ParamSpec::number("signal_hole_height", 0.5),
    ParamSpec::string("encoder_3dmodel_filename", ""),
    ParamSpec::array("encoder_3dmodel_xyz_offset", "[0,0,0]"),
    ParamSpec::array("encoder_3dmodel_xyz_rotation", "[0,0,0]"),
    ParamSpec::array("encoder_3dmodel_xyz_scale", "[1,1,1]"),
    ParamSpec::net_default("S1", ""),
    ParamSpec::net_default("S2", ""),
    ParamSpec::net_default("A", "RE_A"),
    ParamSpec::net_default("B", "GND"),
    ParamSpec::net_default("C", "RE_C"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "ceoloide/rotary_encoder_ec11_ec12",
    display_name: "rotary encoder ec11 ec12",
    kind: PartKind::Encoder,
    matrix_terminals: Some(("S1", "S2")),
    keycap_parameters: None,
    parameters: &PARAMS,
    license: License {
        spdx: "MIT",
        author: "@ceoloide",
    },
    body,
};

fn body(p: &RenderContext<'_>) -> Result<String> {
    const SIGNAL_PAD_MARGIN: f64 = 0.6;
    let (hole_width, hole_height) = (
        p.number("signal_hole_width"),
        p.number("signal_hole_height"),
    );
    if [hole_width, hole_height]
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(GeneratorError::rejected(
            SPEC.source,
            Some("signal_hole_width"),
            "Encoder signal hole dimensions must be positive numbers",
        ));
    }
    p.vec3("encoder_3dmodel_xyz_offset")?;
    p.vec3("encoder_3dmodel_xyz_rotation")?;
    p.vec3("encoder_3dmodel_xyz_scale")?;
    // Equal dimensions emit a round drill for part-specific mounting patterns.
    let signal_drill = if hole_width == hole_height {
        n(hole_width)
    } else {
        format!("oval {} {}", n(hole_width), n(hole_height))
    };
    let signal_size = format!(
        r####"{e0} {e1}"####,
        e0 = n(hole_width + SIGNAL_PAD_MARGIN),
        e1 = n(hole_height + SIGNAL_PAD_MARGIN)
    );

    let common_top = format!(
        r####"
  (footprint "ceoloide:rotary_encoder_ec11_ec12"
    (layer "{e0}.Cu")
    {e1}
    (property "Reference" "{e2}"
      (at 0 -8.5)
      (layer "{e0}.{e3}")
      {e4}
      (effects (font (size 1 1) (thickness 0.15)){e5})
    ){e6}
    (attr through_hole)
    (pad "A" thru_hole oval (at 2.5 {e7} {e8}) (size {e9}) (drill {e10}) (layers "*.Cu" "*.Mask") {e11})
    (pad "B" thru_hole oval (at 0 {e7} {e8}) (size {e9}) (drill {e10}) (layers "*.Cu" "*.Mask") {e12})
    (pad "C" thru_hole oval (at -2.5 {e7} {e8}) (size {e9}) (drill {e10}) (layers "*.Cu" "*.Mask") {e13})
    "####,
        e0 = p.side(),
        e1 = p.at(),
        e2 = p.reference(),
        e3 = if p.flag("include_silkscreen") {
            "SilkS".to_string()
        } else {
            "Fab".to_string()
        },
        e4 = p.ref_hide(),
        e5 = if p.side() == "B" {
            " (justify mirror)".to_string()
        } else {
            "".to_string()
        },
        e6 = if p.flag("reversible") && !(!p.ref_hide().is_empty()) {
            format!(
                r####"
    (fp_text user "{e0}"
      (at 0 -8.5)
      (layer "{e1}.{e2}")
      (effects (font (size 1 1) (thickness 0.15)){e3})
    )"####,
                e0 = p.reference(),
                e1 = if p.side() == "B" {
                    "F".to_string()
                } else {
                    "B".to_string()
                },
                e2 = if p.flag("include_silkscreen") {
                    "SilkS".to_string()
                } else {
                    "Fab".to_string()
                },
                e3 = if p.side() == "F" {
                    " (justify mirror)".to_string()
                } else {
                    "".to_string()
                }
            )
        } else {
            "".to_string()
        },
        e7 = n(p.number("encoder_pads_position")),
        e8 = n(p.rotation()),
        e9 = signal_size.clone(),
        e10 = signal_drill,
        e11 = p.net("A"),
        e12 = p.net("B"),
        e13 = p.net("C")
    );

    let momentary_switch_pads = format!(
        r####"
    (pad "S1" thru_hole oval (at -2.5 -7 {e0}) (size {e1}) (drill {e2}) (layers "*.Cu" "*.Mask") {e3})
    (pad "S2" thru_hole oval (at 2.5 -7 {e0}) (size {e1}) (drill {e2}) (layers "*.Cu" "*.Mask") {e4})
    "####,
        e0 = n(p.rotation()),
        e1 = signal_size.clone(),
        e2 = signal_drill,
        e3 = p.net("S1"),
        e4 = p.net("S2")
    );

    let plated_mp = format!(
        r####"
    (pad "" thru_hole roundrect
      (at -{e0} 0 {e1})
      (size {e2} {e3})
      (drill oval {e4} {e5})
      (layers "*.Cu" "*.Mask")
      (roundrect_rratio 0)
      (chamfer_ratio 0.2)
      (chamfer top_right top_left)
    )
    (pad "" thru_hole roundrect
      (at {e0} 0 {e1})
      (size {e2} {e3})
      (drill oval {e4} {e5})
      (layers "*.Cu" "*.Mask")
      (roundrect_rratio 0)
      (chamfer_ratio 0.2)
      (chamfer bottom_right bottom_left)
    )
    "####,
        e0 = n(p.number("mounting_holes_position")),
        e1 = n(p.rotation() - 90.0),
        e2 = n(p.number("mounting_holes_height") + 0.3),
        e3 = n(p.number("mounting_holes_width") + 0.3),
        e4 = n(p.number("mounting_holes_height")),
        e5 = n(p.number("mounting_holes_width"))
    );

    let npth_mp = format!(
        r####"
    (pad "" np_thru_hole oval
      (at -{e0} 0 {e1})
      (size {e2} {e3})
      (drill oval {e2} {e3})
      (layers "*.Cu" "*.Mask")
    )    
    (pad "" np_thru_hole oval
      (at {e0} 0 {e1})
      (size {e2} {e3})
      (drill oval {e2} {e3})
      (layers "*.Cu" "*.Mask")
    )
    "####,
        e0 = n(p.number("mounting_holes_position")),
        e1 = n(p.rotation() - 90.0),
        e2 = n(p.number("mounting_holes_height")),
        e3 = n(p.number("mounting_holes_width"))
    );

    let plate_hole = String::from(
        r####"
    (fp_rect (start -5.9 -6) (end 5.9 6) (layer "Dwgs.User") (stroke (width 0.15) (type solid)) (fill none))
    "####,
    );

    let encoder_3dmodel = format!(
        r####"
    (model {e0}
      (offset (xyz {e1} {e2} {e3}))
      (scale (xyz {e4} {e5} {e6}))
      (rotate (xyz {e7} {e8} {e9}))
    )
    "####,
        e0 = json_string(p.text("encoder_3dmodel_filename")),
        e1 = n(p.component("encoder_3dmodel_xyz_offset", 0)),
        e2 = n(p.component("encoder_3dmodel_xyz_offset", 1)),
        e3 = n(p.component("encoder_3dmodel_xyz_offset", 2)),
        e4 = n(p.component("encoder_3dmodel_xyz_scale", 0)),
        e5 = n(p.component("encoder_3dmodel_xyz_scale", 1)),
        e6 = n(p.component("encoder_3dmodel_xyz_scale", 2)),
        e7 = n(p.component("encoder_3dmodel_xyz_rotation", 0)),
        e8 = n(p.component("encoder_3dmodel_xyz_rotation", 1)),
        e9 = n(p.component("encoder_3dmodel_xyz_rotation", 2))
    );

    let common_bottom = String::from(
        r####"
  )
    "####,
    );

    let mut out = common_top;
    if p.flag("include_momentary_switch_pads") {
        out += &momentary_switch_pads;
    }
    out += if p.flag("include_plated_mounting_holes") {
        &plated_mp
    } else {
        &npth_mp
    };
    if p.flag("include_plate_hole_marking") {
        out += &plate_hole;
    }
    if !p.text("encoder_3dmodel_filename").is_empty() {
        out += &encoder_3dmodel;
    }
    out += &common_bottom;
    Ok(out)
}

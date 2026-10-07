//! `ceoloide/trrs_pj320a`
//!
//! Ported from `trrs_pj320a.js` of ceoloide/ergogen-footprints.
//! Copyright (c) 2023 Marco Massarelli
//! SPDX-License-Identifier: MIT
//! Authors: @ergogen + @ceoloide improvements
use crate::Result;
use crate::context::RenderContext;
use crate::generators::util::{join_numbers, json_string, xyz};
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 12] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "TRRS"),
    ParamSpec::boolean("reversible", false),
    ParamSpec::boolean("symmetric", false),
    ParamSpec::string(
        "trrs_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/keebio/PJ-320A.step",
    ),
    ParamSpec::array("trrs_3dmodel_xyz_scale", "[1,1,1]"),
    ParamSpec::array("trrs_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("trrs_3dmodel_xyz_offset", "[]"),
    ParamSpec::net_default("TP", "TP"),
    ParamSpec::net_default("R1", "R1"),
    ParamSpec::net_default("R2", "R2"),
    ParamSpec::net_default("SL", "SL"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "ceoloide/trrs_pj320a",
    display_name: "trrs pj320a",
    kind: PartKind::Connector,
    matrix_terminals: None,
    keycap_parameters: None,
    parameters: &PARAMS,
    license: License {
        spdx: "MIT",
        author: "@ergogen + @ceoloide improvements",
    },
    body,
};

fn body(p: &RenderContext<'_>) -> Result<String> {
    let (reversible, symmetric) = (p.flag("reversible"), p.flag("symmetric"));
    let mut footprint_name = String::from("trrs_pj320a");
    if reversible {
        footprint_name += if symmetric {
            " (reversible, symmetric)"
        } else {
            " (reversible)"
        };
    }
    let standard_opening = format!(
        r####"
  (footprint "ceoloide:{e0}"
    (layer "{e1}.Cu")
    {e2}
    (property "Reference" "{e3}"
      (at 0 14.2 {e4})
      (layer "{e1}.SilkS")
      {e5}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    "####,
        e0 = footprint_name,
        e1 = p.side(),
        e2 = p.at(),
        e3 = p.reference(),
        e4 = n(p.rotation()),
        e5 = p.ref_hide()
    );

    let rotation_text = n(p.rotation());
    let corner_marks = |offset_x: f64| {
        format!(
            r#"
    (fp_line (start {a} -2) (end {b} -2) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start {b} 0) (end {b} -2) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start {a} 0) (end {a} -2) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start {c} 0) (end {c} 12.1) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start {d} 0) (end {d} 12.1) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start {d} 12.1) (end {c} 12.1) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start {d} 0) (end {c} 0) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
      "#,
            a = n(2.8 + offset_x),
            b = n(-2.8 + offset_x),
            c = n(-3.05 + offset_x),
            d = n(3.05 + offset_x),
        )
    };
    let stabilizers = |position: f64| {
        format!(
            r#"
    (pad "" np_thru_hole circle (at {position} 8.6 {rotation_text}) (size 1.5 1.5) (drill 1.5) (layers "*.Cu" "*.Mask"))
    (pad "" np_thru_hole circle (at {position} 1.6 {rotation_text}) (size 1.5 1.5) (drill 1.5) (layers "*.Cu" "*.Mask"))
      "#,
            position = n(position),
        )
    };
    let pins = |negative: f64, positive: f64| {
        let (negative, positive) = (n(negative), n(positive));
        if symmetric && reversible {
            format!(
                r#"
    (pad "2" thru_hole oval (at {positive} 3.2 {rotation_text}) (size 1.6 2.2) (drill oval 0.9 1.5) (layers "*.Cu" "*.Mask") {sl})
    (pad "3" thru_hole oval (at {positive} 6.2 {rotation_text}) (size 1.6 2.2) (drill oval 0.9 1.5) (layers "*.Cu" "*.Mask") {r2})
    (pad "4" thru_hole oval (at {positive} 10.75 {rotation_text}) (size 1.6 3.3) (drill oval 0.9 2.6) (layers "*.Cu" "*.Mask") {tp})
        "#,
                sl = p.net("SL"),
                r2 = p.net("R2"),
                tp = p.net("TP"),
            )
        } else {
            format!(
                r#"
    (pad "2" thru_hole oval (at {positive} 3.2 {rotation_text}) (size 1.6 2.2) (drill oval 0.9 1.5) (layers "*.Cu" "*.Mask") {sl})
    (pad "3" thru_hole oval (at {positive} 6.2 {rotation_text}) (size 1.6 2.2) (drill oval 0.9 1.5) (layers "*.Cu" "*.Mask") {r2})
    (pad "4" thru_hole oval (at {positive} 10.2 {rotation_text}) (size 1.6 2.2) (drill oval 0.9 1.5) (layers "*.Cu" "*.Mask") {tp})
    (pad "5" thru_hole oval (at {negative} 11.3 {rotation_text}) (size 1.6 2.2) (drill oval 0.9 1.5) (layers "*.Cu" "*.Mask") {r1})
        "#,
                sl = p.net("SL"),
                r2 = p.net("R2"),
                tp = p.net("TP"),
                r1 = p.net("R1"),
            )
        }
    };
    // The staggered reversible layout shifts the connector center by half a pin pitch.
    let front = p.side() == "F";
    let center = if reversible && !symmetric {
        if front { 1.15 } else { -1.15 }
    } else {
        0.0
    };
    p.vec3("trrs_3dmodel_xyz_scale")?;
    let rotation = p.vec3("trrs_3dmodel_xyz_rotation")?.unwrap_or([
        -90.0,
        0.0,
        if front { 180.0 } else { 0.0 },
    ]);
    let offset = p
        .vec3("trrs_3dmodel_xyz_offset")?
        .unwrap_or([center, 0.0, 0.0]);
    let model = if p.text("trrs_3dmodel_filename").is_empty() {
        String::new()
    } else {
        format!(
            "\n    (model {}\n      (offset (xyz {}))\n      (scale (xyz {}))\n      (rotate (xyz {})))",
            json_string(p.text("trrs_3dmodel_filename")),
            xyz(offset),
            join_numbers(p.list("trrs_3dmodel_xyz_scale")),
            xyz(rotation),
        )
    };
    let mut out = standard_opening;
    if reversible && symmetric {
        out += &corner_marks(0.0);
        out += &stabilizers(0.0);
        out += &pins(2.3, -2.3);
        out += &pins(-2.3, 2.3);
    } else if reversible {
        out += &corner_marks(1.15);
        out += &stabilizers(-1.15);
        out += &stabilizers(1.15);
        out += &pins(-1.15, 3.45);
        out += &pins(1.15, -3.45);
    } else {
        out += &corner_marks(0.0);
        out += &stabilizers(0.0);
        out += &if front {
            pins(-2.3, 2.3)
        } else {
            pins(2.3, -2.3)
        };
    }
    out += &model;
    out += "\n  )\n";
    Ok(out)
}

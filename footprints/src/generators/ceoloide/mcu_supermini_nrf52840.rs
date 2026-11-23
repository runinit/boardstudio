//! `ceoloide/mcu_supermini_nrf52840`
//!
//! Ported from `mcu_supermini_nrf52840.js` of ceoloide/ergogen-footprints.
//! Copyright (c) 2023 Marco Massarelli
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0
//! Author: @infused-kim + @ceoloide improvements
use super::display_shared;
use super::mcu_promicro;
use crate::Result;
use crate::context::RenderContext;
use crate::generators::util::json_string;
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 70] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "MCU"),
    ParamSpec::boolean("reversible", false),
    ParamSpec::boolean("reverse_mount", false),
    ParamSpec::boolean("include_traces", true),
    ParamSpec::boolean("include_extra_pins", false),
    ParamSpec::boolean("invert_jumpers_position", false),
    ParamSpec::boolean("only_required_jumpers", false),
    ParamSpec::boolean("use_rectangular_jumpers", false),
    ParamSpec::number("pcb_thickness", 1.6),
    ParamSpec::number("via_size", 0.8),
    ParamSpec::number("via_drill", 0.4),
    ParamSpec::boolean("show_instructions", true),
    ParamSpec::boolean("show_silk_labels", true),
    ParamSpec::boolean("show_silk_labels_on_both_sides", false),
    ParamSpec::boolean("show_via_labels", true),
    ParamSpec::string(
        "mcu_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/tsuki/nrf52840.step",
    ),
    ParamSpec::array("mcu_3dmodel_xyz_offset", "[]"),
    ParamSpec::array("mcu_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("mcu_3dmodel_xyz_scale", "[1,1,1]"),
    ParamSpec::string("RAW_label", ""),
    ParamSpec::string("GND_label", ""),
    ParamSpec::string("RST_label", ""),
    ParamSpec::string("VCC_label", ""),
    ParamSpec::string("P21_label", ""),
    ParamSpec::string("P20_label", ""),
    ParamSpec::string("P19_label", ""),
    ParamSpec::string("P18_label", ""),
    ParamSpec::string("P15_label", ""),
    ParamSpec::string("P14_label", ""),
    ParamSpec::string("P16_label", ""),
    ParamSpec::string("P10_label", ""),
    ParamSpec::string("P1_label", ""),
    ParamSpec::string("P0_label", ""),
    ParamSpec::string("P2_label", ""),
    ParamSpec::string("P3_label", ""),
    ParamSpec::string("P4_label", ""),
    ParamSpec::string("P5_label", ""),
    ParamSpec::string("P6_label", ""),
    ParamSpec::string("P7_label", ""),
    ParamSpec::string("P8_label", ""),
    ParamSpec::string("P9_label", ""),
    ParamSpec::string("P101_label", ""),
    ParamSpec::string("P102_label", ""),
    ParamSpec::string("P107_label", ""),
    ParamSpec::net_default("RAW", "RAW"),
    ParamSpec::net_default("GND", "GND"),
    ParamSpec::net_default("RST", "RST"),
    ParamSpec::net_default("VCC", "VCC"),
    ParamSpec::net_default("P21", "P21"),
    ParamSpec::net_default("P20", "P20"),
    ParamSpec::net_default("P19", "P19"),
    ParamSpec::net_default("P18", "P18"),
    ParamSpec::net_default("P15", "P15"),
    ParamSpec::net_default("P14", "P14"),
    ParamSpec::net_default("P16", "P16"),
    ParamSpec::net_default("P10", "P10"),
    ParamSpec::net_default("P1", "P1"),
    ParamSpec::net_default("P0", "P0"),
    ParamSpec::net_default("P2", "P2"),
    ParamSpec::net_default("P3", "P3"),
    ParamSpec::net_default("P4", "P4"),
    ParamSpec::net_default("P5", "P5"),
    ParamSpec::net_default("P6", "P6"),
    ParamSpec::net_default("P7", "P7"),
    ParamSpec::net_default("P8", "P8"),
    ParamSpec::net_default("P9", "P9"),
    ParamSpec::net_default("P101", "P101"),
    ParamSpec::net_default("P102", "P102"),
    ParamSpec::net_default("P107", "P107"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "ceoloide/mcu_supermini_nrf52840",
    display_name: "mcu supermini nrf52840",
    kind: PartKind::Controller,
    matrix_terminals: None,
    keycap_parameters: None,
    parameters: &PARAMS,
    license: License {
        spdx: "CC-BY-NC-SA-4.0",
        author: "@infused-kim + @ceoloide improvements",
    },
    body: crate::registry::Body::Text(body),
};

/// The row whose label is hidden when extra pins take its place.
const EXTRA_PIN_ROW: usize = 10;

fn body(p: &RenderContext<'_>) -> Result<String> {
    mcu_promicro::reject_inverted_jumpers(SPEC.source, p)?;
    p.vec3("mcu_3dmodel_xyz_scale")?;
    let reversible = p.flag("reversible");
    let invert_pins = mcu_promicro::invert_pins(p);

    let common_top = format!(
        r####"
  (footprint "ceoloide:mcu_supermini_nrf52840"
    (layer "{e0}.Cu")
    {e1}
    (property "Reference" {e2}
      (at 0 -15 {e3})
      (layer "{e0}.SilkS")
      {e4}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (attr exclude_from_pos_files exclude_from_bom)

    {e5}
    (fp_line (start 3.556 -17.20) (end 3.556 -16.51) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -3.81 -16.51) (end -3.81 -17.20) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -3.81 -17.20) (end 3.556 -17.20) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))


  {e6}
    (fp_line (start -8.89 -16.51) (end 8.89 -16.51) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -8.89 -16.51) (end -8.89 16.57) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start 8.89 -16.51) (end 8.89 16.57) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -8.89 16.57) (end 8.89 16.57) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    "####,
        e0 = p.side(),
        e1 = p.at(),
        e2 = json_string(p.reference()),
        e3 = n(p.rotation()),
        e4 = p.ref_hide(),
        e5 = "",
        e6 = ""
    );
    let instructions = mcu_promicro::instructions_text(p);
    let socket_rows = mcu_promicro::socket_rows(
        p,
        EXTRA_PIN_ROW,
        invert_pins,
        p.flag("show_via_labels"),
        p.flag("show_silk_labels"),
    )?;
    let traces = mcu_promicro::traces(p, invert_pins)?;
    let extra_pins = format!(
        r####"
    (pad "25" thru_hole circle (at {e0}4.54 {e1} {e2}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") {e3})
    (pad "26" thru_hole circle (at {e0}2.00 {e1} {e2}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") {e4})
    (pad "27" thru_hole circle (at {e5}0.54 {e1} {e2}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") {e6})
    "####,
        e0 = if invert_pins {
            "".to_string()
        } else {
            "-".to_string()
        },
        e1 = n(-13.2548 + 25.4),
        e2 = n(p.rotation()),
        e3 = p.net("P101"),
        e4 = p.net("P102"),
        e5 = if invert_pins {
            "-".to_string()
        } else {
            "".to_string()
        },
        e6 = p.net("P107")
    );
    let extra_pins_reversible = format!(
        r####"
    (pad "25" thru_hole circle (at {e0}4.54 {e1} {e2}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") {e3})
    (pad "26" thru_hole circle (at {e0}2.00 {e1} {e2}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") {e4})
    (pad "28" thru_hole circle (at {e5}4.54 {e1} {e2}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") {e3})
    (pad "29" thru_hole circle (at {e5}2.00 {e1} {e2}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") {e4})
    "####,
        e0 = if invert_pins {
            "".to_string()
        } else {
            "-".to_string()
        },
        e1 = n(-13.2548 + 25.4),
        e2 = n(p.rotation()),
        e3 = p.net("P101"),
        e4 = p.net("P102"),
        e5 = if invert_pins {
            "-".to_string()
        } else {
            "".to_string()
        }
    );

    // Reversible pin rows require back mounting, regardless of footprint layer.
    let socket_height = 5.0;
    let module_thickness = 1.6;
    let module_center_x = 16.5;
    let module_center_y = 8.89;
    let flip_layer = p.flag("reversible") && p.side() == "F";
    let mounting_back = p.flag("reversible") || p.side() == "B";
    let model_height = socket_height
        + if p.flag("reverse_mount") {
            0.0
        } else {
            module_thickness
        };
    let model_offset = p.vec3("mcu_3dmodel_xyz_offset")?.unwrap_or([
        (if mounting_back { 1.0 } else { -1.0 })
            * if p.flag("reverse_mount") {
                -module_center_y
            } else {
                module_center_y
            },
        (if p.side() == "B" { -1.0 } else { 1.0 }) * module_center_x,
        if flip_layer {
            -p.number("pcb_thickness") - model_height
        } else {
            model_height
        },
    ]);
    let model_rotation = p.vec3("mcu_3dmodel_xyz_rotation")?.unwrap_or([
        if p.flag("reverse_mount") != flip_layer {
            180.0
        } else {
            0.0
        },
        0.0,
        if p.side() == "B" { -90.0 } else { 90.0 },
    ]);

    let mcu_3dmodel = display_shared::ceoloide_model(
        p.text("mcu_3dmodel_filename"),
        model_offset,
        [
            p.component("mcu_3dmodel_xyz_scale", 0),
            p.component("mcu_3dmodel_xyz_scale", 1),
            p.component("mcu_3dmodel_xyz_scale", 2),
        ],
        model_rotation,
    );

    let mut out = common_top + &socket_rows;
    if p.flag("include_extra_pins") && !p.flag("reversible") {
        out += &extra_pins;
    }
    if p.flag("include_extra_pins") && reversible && p.flag("only_required_jumpers") {
        out += &extra_pins_reversible;
    }
    if reversible && p.flag("show_instructions") {
        out += &instructions;
    }
    if !p.text("mcu_3dmodel_filename").is_empty() {
        out += &mcu_3dmodel;
    }
    out += "\n  )\n";
    if reversible && p.flag("include_traces") {
        out += &traces;
    }
    Ok(out)
}

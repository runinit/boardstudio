//! `ceoloide/mcu_supermini_nrf52840`
//!
//! Ported from `mcu_supermini_nrf52840.js` of ceoloide/ergogen-footprints.
//! Copyright (c) 2023 Marco Massarelli
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0
//! Author: @infused-kim + @ceoloide improvements
use crate::Result;
use crate::context::RenderContext;
use crate::error::GeneratorError;
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
    body,
};

const PIN_NAMES: [[&str; 2]; 12] = [
    ["P1", "RAW"],
    ["P0", "GND"],
    ["GND", "RST"],
    ["GND", "VCC"],
    ["P2", "P21"],
    ["P3", "P20"],
    ["P4", "P19"],
    ["P5", "P18"],
    ["P6", "P15"],
    ["P7", "P14"],
    ["P8", "P16"],
    ["P9", "P10"],
];

/// The row whose label is hidden when extra pins take its place.
const EXTRA_PIN_ROW: usize = 10;

fn pin_label(p: &RenderContext<'_>, pin: &str) -> String {
    let label = p.text(&format!("{pin}_label"));
    if label.is_empty() {
        p.net(pin).name.clone()
    } else {
        label.to_owned()
    }
}

fn traces_row(p: &RenderContext<'_>, row: usize, invert_pins: bool) -> Result<String> {
    let row_num = row as f64;
    let net_left = p.net(PIN_NAMES[row][usize::from(invert_pins)]).index;
    let net_right = p.net(PIN_NAMES[row][usize::from(!invert_pins)]).index;
    Ok(format!(
        r####"
  (segment (start {e0}) (end {e1}) (width 0.25) (layer "F.Cu") (net {e2}))
  (segment (start {e3}) (end {e4}) (width 0.25) (layer "F.Cu") (net {e5}))

  (segment (start {e6}) (end {e7}) (width 0.25) (layer "F.Cu") (net {e8}))
  (segment (start {e6}) (end {e7}) (width 0.25) (layer "B.Cu") (net {e8}))
  (segment (start {e9}) (end {e10}) (width 0.25) (layer "F.Cu") (net {e11}))
  (segment (start {e10}) (end {e9}) (width 0.25) (layer "B.Cu") (net {e11}))

  (segment (start {e12}) (end {e13}) (width 0.25) (layer "B.Cu") (net {e2}))
  (segment (start {e14}) (end {e15}) (width 0.25) (layer "B.Cu") (net {e2}))
  (segment (start {e16}) (end {e17}) (width 0.25) (layer "B.Cu") (net {e2}))
  (segment (start {e15}) (end {e16}) (width 0.25) (layer "B.Cu") (net {e2}))
  (segment (start {e17}) (end {e12}) (width 0.25) (layer "B.Cu") (net {e2}))

  (segment (start {e18}) (end {e19}) (width 0.25) (layer "B.Cu") (net {e5}))
  (segment (start {e20}) (end {e21}) (width 0.25) (layer "B.Cu") (net {e5}))
  (segment (start {e19}) (end {e22}) (width 0.25) (layer "B.Cu") (net {e5}))
  (segment (start {e22}) (end {e23}) (width 0.25) (layer "B.Cu") (net {e5}))
  (segment (start {e23}) (end {e20}) (width 0.25) (layer "B.Cu") (net {e5}))
        "####,
        e0 = p.eaxy(
            if p.flag("use_rectangular_jumpers") {
                4.58
            } else {
                4.775
            },
            -12.7 + (row_num * 2.54)
        ),
        e1 = p.eaxy(3.4, -12.7 + (row_num * 2.54)),
        e2 = net_right,
        e3 = p.eaxy(
            if p.flag("use_rectangular_jumpers") {
                -4.58
            } else {
                -4.775
            },
            -12.7 + (row_num * 2.54)
        ),
        e4 = p.eaxy(-3.4, -12.7 + (row_num * 2.54)),
        e5 = net_left,
        e6 = p.eaxy(-7.62, -12.7 + (row_num * 2.54)),
        e7 = p.eaxy(-5.5, -12.7 + (row_num * 2.54)),
        e8 = p.local_net(&n(24.0 - row_num))?.index,
        e9 = p.eaxy(5.5, -12.7 + (row_num * 2.54)),
        e10 = p.eaxy(7.62, -12.7 + (row_num * 2.54)),
        e11 = p.local_net(&n(1.0 + row_num))?.index,
        e12 = p.eaxy(-2.604695, 0.23 + (row_num * 2.54) - 12.7),
        e13 = p.eaxy(3.17, 0.23 + (row_num * 2.54) - 12.7),
        e14 = p.eaxy(-4.775, 0.0 + (row_num * 2.54) - 12.7),
        e15 = p.eaxy(-4.425305, 0.0 + (row_num * 2.54) - 12.7),
        e16 = p.eaxy(-3.700305, 0.725 + (row_num * 2.54) - 12.7),
        e17 = p.eaxy(-3.099695, 0.725 + (row_num * 2.54) - 12.7),
        e18 = p.eaxy(4.775, 0.0 + (row_num * 2.54) - 12.7),
        e19 = p.eaxy(4.425305, 0.0 + (row_num * 2.54) - 12.7),
        e20 = p.eaxy(2.594695, -0.22 + (row_num * 2.54) - 12.7),
        e21 = p.eaxy(-3.18, -0.22 + (row_num * 2.54) - 12.7),
        e22 = p.eaxy(3.700305, -0.725 + (row_num * 2.54) - 12.7),
        e23 = p.eaxy(3.099695, -0.725 + (row_num * 2.54) - 12.7)
    ))
}

fn traces(p: &RenderContext<'_>, invert_pins: bool) -> Result<String> {
    let mut out = String::new();
    for row in 0..12 {
        if row < 4 || !p.flag("only_required_jumpers") {
            out += &traces_row(p, row, invert_pins)?;
        }
    }
    Ok(out)
}

fn socket_row(
    p: &RenderContext<'_>,
    row: usize,
    left: &str,
    right: &str,
    invert_pins: bool,
    show_via_labels: bool,
    show_silk_labels: bool,
) -> Result<String> {
    let row_num = row as f64;
    let row_offset_y = 2.54 * row_num;
    let socket_hole_num_left = 24.0 - row_num;
    let socket_hole_num_right = 1.0 + row_num;
    let via_num_left = 124.0 - row_num;
    let via_num_right = 101.0 + row_num;
    let net_left = p.net(left).to_string();
    let net_right = p.net(right).to_string();
    let via_label_left = pin_label(p, left);
    let via_label_right = pin_label(p, right);
    // Labels printed on the PCB; a reversible footprint aligns them with the
    // pins on the side opposite the controller.
    let net_silk_front_left = via_label_left.clone();
    let net_silk_front_right = via_label_right.clone();
    let net_silk_back_left = via_label_right.clone();
    let net_silk_back_right = via_label_left.clone();
    let reversible = p.flag("reversible");
    let jumpers_here = reversible && (row_num < 4.0 || !p.flag("only_required_jumpers"));
    let (extra, only_required) = (
        p.flag("include_extra_pins"),
        p.flag("only_required_jumpers"),
    );

    let socket_row_base = format!(
        r####"
    {e0}
    (pad "{e1}" thru_hole circle (at -7.62 {e2} {e3}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") {e4})
    (pad "{e5}" thru_hole circle (at 7.62 {e2} {e3}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") {e6})
      "####,
        e0 = "",
        e1 = n(socket_hole_num_left),
        e2 = n(-12.7 + row_offset_y),
        e3 = n(p.rotation()),
        e4 = if jumpers_here {
            p.local_net(&n(socket_hole_num_left))?.to_string()
        } else {
            net_left.clone()
        },
        e5 = n(socket_hole_num_right),
        e6 = if jumpers_here {
            p.local_net(&n(socket_hole_num_right))?.to_string()
        } else {
            net_right.clone()
        }
    );
    let socket_row_vias = format!(
        r####"
    {e0}
    (pad "{e1}" thru_hole circle (at -3.4 {e2} {e3}) (size {e4} {e4}) (drill {e5}) (layers "*.Cu" "*.Mask") {e6})
    (pad "{e7}" thru_hole circle (at 3.4 {e2} {e3}) (size {e4} {e4}) (drill {e5}) (layers "*.Cu" "*.Mask") {e8})
      "####,
        e0 = "",
        e1 = n(via_num_left),
        e2 = n(-12.7 + row_offset_y),
        e3 = n(p.rotation()),
        e4 = n(p.number("via_size")),
        e5 = n(p.number("via_drill")),
        e6 = net_left,
        e7 = n(via_num_right),
        e8 = net_right
    );
    let socket_row_rectangular_jumpers = format!(
        r####"
    {e0}
    (pad "{e1}" smd rect (at -5.48 {e2} {e3}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") {e4})
    (pad "{e5}" smd rect (at -4.58 {e2} {e3}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") {e6})

    {e7}
    (pad "{e8}" smd rect (at 4.58 {e2} {e3}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") {e9})
    (pad "{e10}" smd rect (at 5.48 {e2} {e3}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") {e11})

    {e12}
    (pad "{e1}" smd rect (at -5.48 {e2} {e3}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") {e4})
    (pad "{e8}" smd rect (at -4.58 {e2} {e3}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") {e9})

    {e13}
    (pad "{e5}" smd rect (at 4.58 {e2} {e3}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") {e6})
    (pad "{e10}" smd rect (at 5.48 {e2} {e3}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") {e11})
        "####,
        e0 = "",
        e1 = n(socket_hole_num_left),
        e2 = n(-12.7 + row_offset_y),
        e3 = n(p.rotation()),
        e4 = p.local_net(&n(socket_hole_num_left))?,
        e5 = n(via_num_left),
        e6 = net_left,
        e7 = "",
        e8 = n(via_num_right),
        e9 = net_right,
        e10 = n(socket_hole_num_right),
        e11 = p.local_net(&n(socket_hole_num_right))?,
        e12 = "",
        e13 = ""
    );
    let socket_row_chevron_jumpers = format!(
        r####"
    {e0}
    (pad "{e1}" smd custom (at -5.5 {e2} {e3}) (size 0.2 0.2) (layers "F.Cu" "F.Paste" "F.Mask") {e4}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.5 -0.625) (xy -0.25 -0.625) (xy 0.25 0) (xy -0.25 0.625) (xy -0.5 0.625)
      ) (width 0) (fill yes))
    ))
    (pad "{e5}" smd custom (at -4.775 {e2} {e3}) (size 0.2 0.2) (layers "F.Cu" "F.Paste" "F.Mask") {e6}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.65 -0.625) (xy 0.5 -0.625) (xy 0.5 0.625) (xy -0.65 0.625) (xy -0.15 0)
      ) (width 0) (fill yes))
    ))

    {e7}
    (pad "{e8}" smd custom (at 4.775 {e2} {e9}) (size 0.2 0.2) (layers "F.Cu" "F.Paste" "F.Mask") {e10}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.65 -0.625) (xy 0.5 -0.625) (xy 0.5 0.625) (xy -0.65 0.625) (xy -0.15 0)
      ) (width 0) (fill yes))
    ))
    (pad "{e11}" smd custom (at 5.5 {e2} {e9}) (size 0.2 0.2) (layers "F.Cu" "F.Paste" "F.Mask") {e12}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.5 -0.625) (xy -0.25 -0.625) (xy 0.25 0) (xy -0.25 0.625) (xy -0.5 0.625)
      ) (width 0) (fill yes))
    ))

    {e13}
    (pad "{e1}" smd custom (at -5.5 {e2} {e3}) (size 0.2 0.2) (layers "B.Cu" "B.Paste" "B.Mask") {e4}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.5 0.625) (xy -0.25 0.625) (xy 0.25 0) (xy -0.25 -0.625) (xy -0.5 -0.625)
      ) (width 0) (fill yes))
    ))

    (pad "{e8}" smd custom (at -4.775 {e2} {e3}) (size 0.2 0.2) (layers "B.Cu" "B.Paste" "B.Mask") {e10}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.65 0.625) (xy 0.5 0.625) (xy 0.5 -0.625) (xy -0.65 -0.625) (xy -0.15 0)
      ) (width 0) (fill yes))
    ))

    {e14}
    (pad "{e5}" smd custom (at 4.775 {e2} {e9}) (size 0.2 0.2) (layers "B.Cu" "B.Paste" "B.Mask") {e6}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.65 0.625) (xy 0.5 0.625) (xy 0.5 -0.625) (xy -0.65 -0.625) (xy -0.15 0)
      ) (width 0) (fill yes))
    ))
    (pad "{e11}" smd custom (at 5.5 {e2} {e9}) (size 0.2 0.2) (layers "B.Cu" "B.Paste" "B.Mask") {e12}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.5 0.625) (xy -0.25 0.625) (xy 0.25 0) (xy -0.25 -0.625) (xy -0.5 -0.625)
      ) (width 0) (fill yes))
    ))
        "####,
        e0 = "",
        e1 = n(socket_hole_num_left),
        e2 = n(-12.7 + row_offset_y),
        e3 = n(p.rotation()),
        e4 = p.local_net(&n(socket_hole_num_left))?,
        e5 = n(via_num_left),
        e6 = net_left,
        e7 = "",
        e8 = n(via_num_right),
        e9 = n(180.0 + p.rotation()),
        e10 = net_right,
        e11 = n(socket_hole_num_right),
        e12 = p.local_net(&n(socket_hole_num_right))?,
        e13 = "",
        e14 = ""
    );
    let mut socket_row = socket_row_base;
    if jumpers_here {
        socket_row += &socket_row_vias;
        socket_row += if p.flag("use_rectangular_jumpers") {
            &socket_row_rectangular_jumpers
        } else {
            &socket_row_chevron_jumpers
        };
    }
    if show_silk_labels {
        let hide_extra = row != EXTRA_PIN_ROW;
        if reversible || p.flag("show_silk_labels_on_both_sides") || p.side() == "F" {
            if hide_extra
                || !extra
                || (extra && invert_pins && !reversible)
                || (extra && !only_required && reversible)
            {
                socket_row += &format!(
                    r####"
    (fp_text user "{e0}" (at -{e1} {e2} {e3}) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)))
    )
            "####,
                    e0 = net_silk_front_left,
                    e1 = n(if jumpers_here {
                        if net_silk_front_left.encode_utf16().count() > 2 {
                            1.45
                        } else {
                            2.04
                        }
                    } else {
                        4.47
                    }),
                    e2 = n(-12.7 + row_offset_y),
                    e3 = n(p.rotation())
                );
            }
            if hide_extra
                || !extra
                || (extra && !invert_pins && !reversible)
                || (extra && !only_required && reversible)
            {
                socket_row += &format!(
                    r####"
    (fp_text user "{e0}" (at {e1} {e2} {e3}) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)))
    )
            "####,
                    e0 = net_silk_front_right,
                    e1 = n(if jumpers_here {
                        if net_silk_front_right.encode_utf16().count() > 2 {
                            1.45
                        } else {
                            2.04
                        }
                    } else {
                        4.47
                    }),
                    e2 = n(-12.7 + row_offset_y),
                    e3 = n(p.rotation())
                );
            }
        }
        if reversible || p.flag("show_silk_labels_on_both_sides") || p.side() == "B" {
            if hide_extra
                || !extra
                || (extra && !invert_pins && !reversible)
                || (extra && !only_required && reversible)
            {
                socket_row += &format!(
                    r####"
    (fp_text user "{e0}" (at {e1}{e2} {e3} {e4}) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify mirror))
    )
            "####,
                    e0 = net_silk_back_left,
                    e1 = if p.flag("reversible") {
                        "-".to_string()
                    } else {
                        "".to_string()
                    },
                    e2 = n(if jumpers_here {
                        if net_silk_back_left.encode_utf16().count() > 2 {
                            1.45
                        } else {
                            2.04
                        }
                    } else {
                        4.47
                    }),
                    e3 = n(-12.7 + row_offset_y),
                    e4 = n(p.rotation())
                );
            }
            if hide_extra
                || !extra
                || (extra && invert_pins && !reversible)
                || (extra && !only_required && reversible)
            {
                socket_row += &format!(
                    r####"
    (fp_text user "{e0}" (at {e1}{e2} {e3} {e4}) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify mirror))
    )
            "####,
                    e0 = net_silk_back_right,
                    e1 = if p.flag("reversible") {
                        "".to_string()
                    } else {
                        "-".to_string()
                    },
                    e2 = n(if jumpers_here {
                        if net_silk_back_right.encode_utf16().count() > 2 {
                            1.45
                        } else {
                            2.04
                        }
                    } else {
                        4.47
                    }),
                    e3 = n(-12.7 + row_offset_y),
                    e4 = n(p.rotation())
                );
            }
        }
    }
    if show_via_labels && jumpers_here {
        socket_row += &format!(
            r####"
    {e0}
    (fp_text user "{e1}" (at -3.262 {e2} {e3}) (layer "F.Fab")
      (effects (font (size 0.5 0.5) (thickness 0.08)))
    )
    (fp_text user "{e4}" (at 3.262 {e2} {e3}) (layer "F.Fab")
      (effects (font (size 0.5 0.5) (thickness 0.08)))
    )

    {e5}
    (fp_text user "{e1}" (at -3.262 {e2} {e6}) (layer "B.Fab")
      (effects (font (size 0.5 0.5) (thickness 0.08)) (justify mirror))
    )
    (fp_text user "{e4}" (at 3.262 {e2} {e6}) (layer "B.Fab")
      (effects (font (size 0.5 0.5) (thickness 0.08)) (justify mirror))
    )
          "####,
            e0 = "",
            e1 = via_label_left,
            e2 = n(-13.5 + row_offset_y),
            e3 = n(p.rotation()),
            e4 = via_label_right,
            e5 = "",
            e6 = n(180.0 + p.rotation())
        );
    }
    Ok(socket_row)
}

fn socket_rows(
    p: &RenderContext<'_>,
    invert_pins: bool,
    show_via_labels: bool,
    show_silk_labels: bool,
) -> Result<String> {
    let reversible = p.flag("reversible");
    let mut out = String::new();
    for (row, pins) in PIN_NAMES.iter().enumerate() {
        let left = pins[usize::from(invert_pins)];
        let right = pins[usize::from(!invert_pins)];
        out += &socket_row(
            p,
            row,
            left,
            right,
            invert_pins,
            show_via_labels,
            show_silk_labels,
        )?;
    }
    // Socket silkscreen; the first pin is marked according to orientation.
    if show_silk_labels {
        if reversible || p.flag("show_silk_labels_on_both_sides") || p.side() == "F" {
            out += &format!(
                r####"
    (fp_line (start 6.29 -14.03) (end 8.95 -14.03) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 6.29 -14.03) (end 6.29 16.57) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 6.29 16.57) (end 8.95 16.57) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -6.29 -14.03) (end -6.29 16.57) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 8.95 -14.03) (end 8.95 16.57) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -8.95 -14.03) (end -6.29 -14.03) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -8.95 -14.03) (end -8.95 16.57) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -8.95 16.57) (end -6.29 16.57) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start {e0}6.29 -11.43) (end {e0}8.95 -11.43) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
            "####,
                e0 = if invert_pins {
                    "".to_string()
                } else {
                    "-".to_string()
                }
            );
        }
        if reversible || p.flag("show_silk_labels_on_both_sides") || p.side() == "B" {
            out += &format!(
                r####"
    (fp_line (start -6.29 -14.03) (end -8.95 -14.03) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -6.29 -14.03) (end -6.29 16.57) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -6.29 16.57) (end -8.95 16.57) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -8.95 -14.03) (end -8.95 16.57) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 8.95 -14.03) (end 6.29 -14.03) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 8.95 -14.03) (end 8.95 16.57) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 8.95 16.57) (end 6.29 16.57) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 6.29 -14.03) (end 6.29 16.57) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start {e0}8.95 -11.43) (end {e0}6.29 -11.43) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
          "####,
                e0 = if invert_pins {
                    if reversible { "-" } else { "" }
                } else if reversible {
                    ""
                } else {
                    "-"
                }
            );
        }
    }
    Ok(out)
}

fn body(p: &RenderContext<'_>) -> Result<String> {
    if p.flag("invert_jumpers_position") {
        return Err(GeneratorError::rejected(
            SPEC.source,
            Some("invert_jumpers_position"),
            "invert_jumpers_position is unsupported; set it to false and use the documented jumper assembly.",
        ));
    }
    p.vec3("mcu_3dmodel_xyz_scale")?;
    let (side, reversible, reverse_mount) =
        (p.side(), p.flag("reversible"), p.flag("reverse_mount"));
    let invert_pins = (side == "B" && !reverse_mount && !reversible)
        || (side == "F" && reverse_mount && !reversible)
        || (!reverse_mount && reversible);

    let common_top = format!(
        r####"
  (footprint "ceoloide:mcu_supermini_nrf52840"
    (layer "{e0}.Cu")
    {e1}
    (property "Reference" "{e2}"
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
        e2 = p.reference(),
        e3 = n(p.rotation()),
        e4 = p.ref_hide(),
        e5 = "",
        e6 = ""
    );
    let instructions = format!(
        r####"
    (fp_text user "R hand back side (M{e0})" (at 0 -15.245 {e1}) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (fp_text user "L hand back side (M{e0})" (at 0 -15.245 {e1}) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify mirror))
    )
    "####,
        e0 = if !(p.flag("reverse_mount")) {
            "↑".to_string()
        } else {
            "↓".to_string()
        },
        e1 = n(p.rotation())
    );
    let socket_rows = socket_rows(
        p,
        invert_pins,
        p.flag("show_via_labels"),
        p.flag("show_silk_labels"),
    )?;
    let traces = traces(p, invert_pins)?;
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

    let mcu_3dmodel = format!(
        r####"
    (model {e0}
      (offset (xyz {e1} {e2} {e3}))
      (scale (xyz {e4} {e5} {e6}))
      (rotate (xyz {e7} {e8} {e9}))
    )
    "####,
        e0 = json_string(p.text("mcu_3dmodel_filename")),
        e1 = n(model_offset[0]),
        e2 = n(model_offset[1]),
        e3 = n(model_offset[2]),
        e4 = n(p.component("mcu_3dmodel_xyz_scale", 0)),
        e5 = n(p.component("mcu_3dmodel_xyz_scale", 1)),
        e6 = n(p.component("mcu_3dmodel_xyz_scale", 2)),
        e7 = n(model_rotation[0]),
        e8 = n(model_rotation[1]),
        e9 = n(model_rotation[2])
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

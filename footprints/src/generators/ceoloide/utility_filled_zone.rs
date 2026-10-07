//! `ceoloide/utility_filled_zone`: a copper zone over a polygon.
//!
//! Ported from `utility_filled_zone.js` of ceoloide/ergogen-footprints.
//! Copyright (c) 2023 Marco Massarelli
//! SPDX-License-Identifier: MIT
//! Author: @ceoloide
//!
//! The source tested `p.prority` (a typo), so `priority` was never written to
//! the zone. That behaviour is kept until it is decided separately.
use crate::Result;
use crate::context::RenderContext;
use crate::generators::util::{points_text, yes_no};
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 21] = [
    ParamSpec::SIDE,
    ParamSpec::net_default("net", "GND"),
    ParamSpec::string("name", ""),
    ParamSpec::number("priority", 0.0),
    ParamSpec::boolean("locked", false),
    ParamSpec::string("corner_smoothing", "chamfer"),
    ParamSpec::number("smoothing_radius", 0.5),
    ParamSpec::string("connect_pads", ""),
    ParamSpec::number("pad_clearance", 0.508),
    ParamSpec::number("min_thickness", 0.25),
    ParamSpec::number("thermal_gap", 0.5),
    ParamSpec::number("thermal_bridge_width", 0.5),
    ParamSpec::string("remove_islands", "never"),
    ParamSpec::number("min_island_size", 5.0),
    ParamSpec::string("fill_type", "solid"),
    ParamSpec::number("hatch_thickness", 1.0),
    ParamSpec::number("hatch_gap", 1.5),
    ParamSpec::number("hatch_orientation", 0.0),
    ParamSpec::number("hatch_smoothing_level", 0.0),
    ParamSpec::number("hatch_smoothing_value", 0.1),
    ParamSpec::array("points", "[[0,0],[420,0],[420,297],[0,297]]"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "ceoloide/utility_filled_zone",
    display_name: "utility filled zone",
    kind: PartKind::Utility,
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
    let polygon_pts = points_text("ceoloide/utility_filled_zone", p, "points")?;
    let net = p.net("net");
    let name = p.text("name");
    let fill_type = p.text("fill_type");
    let solid = fill_type == "solid";
    let smoothing = p.text("corner_smoothing");
    let islands = p.text("remove_islands");
    let hatch_smoothing_level = p.number("hatch_smoothing_level");
    let hatch_smoothing = solid || hatch_smoothing_level < 1.0;
    let when = |condition: bool, text: String| if condition { text } else { String::new() };
    Ok(format!(
        r#"
  (zone
    (net {index})
    (net_name "{net_name}")
    (locked {locked})
    (layers "{side}.Cu")
    {name_form}
    (hatch edge 0.5)
    {priority}
    (connect_pads {connect_pads}
      (clearance {pad_clearance})
    )
    (min_thickness {min_thickness})
    (filled_areas_thickness no)
    (fill
      {mode}
      (thermal_gap {thermal_gap})
      (thermal_bridge_width {thermal_bridge_width})
      {smoothing_form}
      {radius_form}
      {island_mode}
      {island_area}
      {hatch_thickness}
      {hatch_gap}
      {hatch_orientation}
      {hatch_smoothing_level_form}
      {hatch_smoothing_value_form}
      {hatch_border}
      {hatch_hole}
    )
    (polygon
      (pts
        {polygon_pts}
      )
    )
  )
"#,
        index = net.index,
        net_name = net.name,
        locked = yes_no(p.flag("locked")),
        side = p.side(),
        name_form = when(!name.is_empty(), format!("(name \"{name}\")")),
        // `p.prority` never exists in the source, so priority is never written.
        priority = "",
        connect_pads = p.text("connect_pads"),
        pad_clearance = n(p.number("pad_clearance")),
        min_thickness = n(p.number("min_thickness")),
        mode = if solid {
            "yes".to_owned()
        } else {
            format!("(mode {fill_type})")
        },
        thermal_gap = n(p.number("thermal_gap")),
        thermal_bridge_width = n(p.number("thermal_bridge_width")),
        smoothing_form = when(!smoothing.is_empty(), format!("(smoothing {smoothing})")),
        radius_form = when(
            !smoothing.is_empty(),
            format!("(radius {})", n(p.number("smoothing_radius")))
        ),
        island_mode = when(
            islands != "always",
            format!(
                "(island_removal_mode {})",
                if islands == "never" { 1 } else { 2 }
            )
        ),
        island_area = when(
            islands != "always",
            format!("(island_area_min {})", n(p.number("min_island_size")))
        ),
        hatch_thickness = when(
            !solid,
            format!("(hatch_thickness {})", n(p.number("hatch_thickness")))
        ),
        hatch_gap = when(!solid, format!("(hatch_gap {})", n(p.number("hatch_gap")))),
        hatch_orientation = when(
            !solid,
            format!("(hatch_orientation {})", n(p.number("hatch_orientation")))
        ),
        hatch_smoothing_level_form = when(
            !hatch_smoothing,
            format!("(hatch_smoothing_level {})", n(hatch_smoothing_level))
        ),
        hatch_smoothing_value_form = when(
            !hatch_smoothing,
            format!(
                "(hatch_smoothing_value {})",
                n(p.number("hatch_smoothing_value"))
            )
        ),
        hatch_border = when(
            !solid,
            "(hatch_border_algorithm hatch_thickness)".to_owned()
        ),
        hatch_hole = when(!solid, "(hatch_min_hole_area 0.3)".to_owned()),
    ))
}

//! `ceoloide/utility_keepout_zone`: a keepout area over a polygon.
//!
//! Ported from `utility_keepout_zone.js` of ceoloide/ergogen-footprints.
//! Copyright (c) 2023 Marco Massarelli
//! SPDX-License-Identifier: MIT
//! Author: @ceoloide
use crate::Result;
use crate::context::RenderContext;
use crate::error::GeneratorError;
use crate::generators::util::{points_text, yes_no};
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 11] = [
    ParamSpec::SIDE,
    ParamSpec::string("name", ""),
    ParamSpec::boolean("locked", false),
    ParamSpec::boolean("tracks_allowed", true),
    ParamSpec::boolean("vias_allowed", true),
    ParamSpec::boolean("pads_allowed", true),
    ParamSpec::boolean("copperpour_allowed", true),
    ParamSpec::boolean("footprints_allowed", true),
    ParamSpec::string("outline_type", "edge"),
    ParamSpec::number("hatch_pitch", 1.0),
    ParamSpec::array("points", "[[0,0],[420,0],[420,297],[0,297]]"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "ceoloide/utility_keepout_zone",
    display_name: "utility keepout zone",
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
    let pitch = p.number("hatch_pitch");
    if pitch < 0.0 || pitch > 2.0 {
        return Err(GeneratorError::rejected(
            SPEC.source,
            Some("hatch_pitch"),
            format!(
                "Parameter hatch_pitch must be a positive number below 2mm to avoid KiCad issues. Current value: {}mm",
                n(pitch)
            ),
        ));
    }
    let polygon_pts = points_text(SPEC.source, p, "points")?;
    let name = p.text("name");
    let allowed = |parameter: &str| {
        if p.flag(parameter) {
            "allowed"
        } else {
            "not_allowed"
        }
    };
    Ok(format!(
        r#"
  (zone
    (net 0)
    (net_name "")
    (locked {locked})
    (layers "{side}.Cu")
    {name_form}
    (hatch {outline_type} {pitch})
    (connect_pads
      (clearance 0)
    )
    (min_thickness 0.25)
    (filled_areas_thickness no)
    (keepout
      (tracks {tracks})
      (vias {vias})
      (pads {pads})
      (copperpour {copperpour})
      (footprints {footprints})
    )
    (fill
      (thermal_gap 0.5)
      (thermal_bridge_width 0.5)
    )
    (polygon
      (pts
        {polygon_pts}
      )
    )
  )
"#,
        locked = yes_no(p.flag("locked")),
        side = p.side(),
        name_form = if name.is_empty() {
            String::new()
        } else {
            format!("(name \"{name}\")")
        },
        outline_type = p.text("outline_type"),
        pitch = n(pitch),
        tracks = allowed("tracks_allowed"),
        vias = allowed("vias_allowed"),
        pads = allowed("pads_allowed"),
        copperpour = allowed("copperpour_allowed"),
        footprints = allowed("footprints_allowed"),
    ))
}

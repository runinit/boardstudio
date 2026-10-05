//! `ceoloide/utility_point_debugger`: a crosshair marking the part's origin.
//!
//! Ported from `utility_point_debugger.js` of ceoloide/ergogen-footprints.
//! Copyright (c) 2023 Marco Massarelli
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0
//! Author: @infused-kim + @ceoloide improvements
//!
//! The source labelled the crosshair with a point name that never existed and
//! printed "undefined"; the label is now empty.
use crate::Result;
use crate::context::RenderContext;
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 3] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "P"),
    ParamSpec::boolean("enabled", true),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "ceoloide/utility_point_debugger",
    display_name: "utility point debugger",
    kind: PartKind::Utility,
    matrix_terminals: None,
    keycap_parameters: None,
    parameters: &PARAMS,
    license: License {
        spdx: "CC-BY-NC-SA-4.0",
        author: "@infused-kim + @ceoloide improvements",
    },
    body,
};

fn body(p: &RenderContext<'_>) -> Result<String> {
    if !p.flag("enabled") {
        return Ok(String::new());
    }
    Ok(format!(
        r#"
  (footprint "ceoloide:utility_point_debugger"
    (layer "F.Cu")
    {at}
    (property "Reference" "{reference}"
      (at 0 2)
      (layer "Dwgs.User")
      {hide}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (fp_line (start -0.6 0) (end 0.6 0) (layer "Dwgs.User") (stroke (width 0.05) (type solid)))
    (fp_line (start 0 -0.6) (end 0 0.6) (layer "Dwgs.User") (stroke (width 0.05) (type solid)))
    (fp_line (start 0.6 0) (end 0.5 -0.1) (layer "Dwgs.User") (stroke (width 0.05) (type solid)))
    (fp_line (start 0.6 0) (end 0.5 0.1) (layer "Dwgs.User") (stroke (width 0.05) (type solid)))
    (fp_line (start 0 -0.6) (end 0.1 -0.5) (layer "Dwgs.User") (stroke (width 0.05) (type solid)))
    (fp_line (start 0 -0.6) (end -0.1 -0.5) (layer "Dwgs.User") (stroke (width 0.05) (type solid)))
    (fp_text user ""
      (at -0.3 -0.05 {rotation})
      (layer "Dwgs.User")
      (effects (font (size 0.0254 0.0254) (thickness 0.001)))
    )
  )
"#,
        at = p.at(),
        reference = p.reference(),
        hide = p.ref_hide(),
        rotation = n(p.rotation()),
    ))
}

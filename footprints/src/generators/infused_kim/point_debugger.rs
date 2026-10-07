//! `infused-kim/point_debugger`: a crosshair marking the part's origin.
//!
//! Ported from `point_debugger.js` of infused-kim/kb_ergogen_fp.
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0 (see the vendored LICENSE)
//! Author: @infused-kim
//!
//! The source labelled the crosshair with a point name that never existed and
//! printed `undefined`; the label is now an empty string.
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
    source: "infused-kim/point_debugger",
    display_name: "point debugger",
    kind: PartKind::Utility,
    matrix_terminals: None,
    keycap_parameters: None,
    parameters: &PARAMS,
    license: License {
        spdx: "CC-BY-NC-SA-4.0",
        author: "@infused-kim",
    },
    body,
};

fn body(p: &RenderContext<'_>) -> Result<String> {
    if !p.flag("enabled") {
        return Ok(String::new());
    }
    Ok(format!(
        r#"
            (module point_debugger (layer F.Cu) (tedit 64B42FA5)
                {at}
                (fp_text reference {reference} (at 0 2) (layer F.SilkS) {hide}
                    (effects (font (size 1 1) (thickness 0.15)))
                )
                (fp_line (start -0.6 0) (end 0.6 0) (layer Dwgs.User) (width 0.05))
                (fp_line (start 0 -0.6) (end 0 0.6) (layer Dwgs.User) (width 0.05))
                (fp_line (start 0.6 0) (end 0.5 -0.1) (layer Dwgs.User) (width 0.05))
                (fp_line (start 0.6 0) (end 0.5 0.1) (layer Dwgs.User) (width 0.05))
                (fp_line (start 0 -0.6) (end 0.1 -0.5) (layer Dwgs.User) (width 0.05))
                (fp_line (start 0 -0.6) (end -0.1 -0.5) (layer Dwgs.User) (width 0.05))
                    (fp_text user "" (at -0.3 -0.05 {rotation}) (layer Dwgs.User)
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

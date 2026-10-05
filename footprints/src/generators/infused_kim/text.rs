//! `infused-kim/text`: silkscreen text on the front, back or both sides.
//!
//! Ported from `text.js` of infused-kim/kb_ergogen_fp.
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0 (see the vendored LICENSE)
//! Author: @infused-kim
use crate::Result;
use crate::context::RenderContext;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 4] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "TXT"),
    ParamSpec::boolean("reverse", false),
    ParamSpec::string("text", "Awesomeness"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "infused-kim/text",
    display_name: "text",
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
    let (text, at) = (p.text("text"), p.at());
    let mut out = String::new();
    if p.side() == "F" || p.flag("reverse") {
        out.push_str(&format!(
            "\n      (gr_text \"{text}\" {at} (layer F.SilkS)\n        (effects (font (size 1 1) (thickness 0.15)))\n      )\n"
        ));
    }
    if p.side() == "B" || p.flag("reverse") {
        out.push_str(&format!(
            "\n      (gr_text \"{text}\" {at} (layer B.SilkS)\n        (effects (font (size 1 1) (thickness 0.15)) (justify mirror))\n      )\n"
        ));
    }
    Ok(out)
}

//! `ceoloide/utility_text`: board text, optionally mirrored onto both sides.
//!
//! Ported from `utility_text.js` of ceoloide/ergogen-footprints.
//! Copyright (c) 2023 Marco Massarelli
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0
//! Authors: @infused-kim + @ceoloide & @dieseltravis improvements
use crate::Result;
use crate::context::RenderContext;
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 14] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "TXT"),
    ParamSpec::string("layer", "SilkS"),
    ParamSpec::boolean("reversible", false),
    ParamSpec::number("thickness", 0.15),
    ParamSpec::number("height", 1.0),
    ParamSpec::number("width", 1.0),
    ParamSpec::boolean("mirrored", false),
    ParamSpec::boolean("knockout", false),
    ParamSpec::boolean("bold", false),
    ParamSpec::boolean("italic", false),
    ParamSpec::string("align", ""),
    ParamSpec::string("face", ""),
    ParamSpec::string("text", ""),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "ceoloide/utility_text",
    display_name: "utility text",
    kind: PartKind::Utility,
    matrix_terminals: None,
    keycap_parameters: None,
    parameters: &PARAMS,
    license: License {
        spdx: "CC-BY-NC-SA-4.0",
        author: "@infused-kim + @ceoloide & @dieseltravis improvements",
    },
    body,
};

fn text_on(p: &RenderContext<'_>, side: &str, align: &str, mirrored: bool) -> String {
    let face = p.text("face");
    let justify = format!("(justify {align} {})", if mirrored { "mirror" } else { "" });
    format!(
        r#"
  (gr_text "{text}"
    {at}
    (layer "{side}.{layer}" {knockout})
    (effects
      (font {face}
        (size {height} {width})
        (thickness {thickness})
        {bold}
        {italic}
      )
      {justify}
    )
  )
"#,
        text = p.text("text"),
        at = p.at(),
        layer = p.text("layer"),
        knockout = if p.flag("knockout") { "knockout" } else { "" },
        face = if face.is_empty() {
            String::new()
        } else {
            format!("(face \"{face}\")")
        },
        height = n(p.number("height")),
        width = n(p.number("width")),
        thickness = n(p.number("thickness")),
        bold = if p.flag("bold") { "(bold yes)" } else { "" },
        italic = if p.flag("italic") { "(italic yes)" } else { "" },
        justify = if !align.is_empty() || mirrored {
            justify
        } else {
            String::new()
        },
    )
}

fn body(p: &RenderContext<'_>) -> Result<String> {
    let align = p.text("align");
    if p.flag("reversible") {
        Ok(text_on(p, "F", align, false) + &text_on(p, "B", align, true))
    } else {
        Ok(text_on(p, p.side(), align, p.flag("mirrored")))
    }
}

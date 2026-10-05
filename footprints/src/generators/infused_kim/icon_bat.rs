//! `infused-kim/icon_bat`: a battery polarity icon on the silkscreen.
//!
//! Ported from `icon_bat.js` of infused-kim/kb_ergogen_fp.
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0 (see the vendored LICENSE)
//! Author: @infused-kim
use crate::Result;
use crate::context::RenderContext;
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 4] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "ICON"),
    ParamSpec::boolean("reverse", false),
    ParamSpec::number("spacing", 1.0),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "infused-kim/icon_bat",
    display_name: "icon bat",
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
    let adjust = p.number("spacing") / 2.0;
    let rotation = n(p.rotation());
    let mut out = format!(
        "\n        (module icon_bat (layer F.Cu) (tedit 64461058)\n          {}\n          (attr virtual)\n\n      ",
        p.at()
    );
    if p.side() == "F" || p.flag("reverse") {
        out.push_str(&format!(
            r#"
          (fp_text reference "{reference}" (at 0 3 {rotation}) (layer F.SilkS) {hide}
            (effects (font (size 1 1) (thickness 0.15)))
          )
          (fp_circle (center {a} 0) (end {b} 0) (layer F.SilkS) (width 0.1))
          (fp_line (start {a} -0.3) (end {a} 0.3) (layer F.SilkS) (width 0.1))
          (fp_line (start {c} 0) (end {d} 0) (layer F.SilkS) (width 0.1))

          (fp_circle (center {e} 0) (end {f}  0) (layer F.SilkS) (width 0.1))
          (fp_line (start {g} 0) (end {h} 0) (layer F.SilkS) (width 0.1))
      "#,
            reference = p.reference(),
            hide = p.ref_hide(),
            a = n(-0.55 - adjust),
            b = n(-0.05 - adjust),
            c = n(-0.85 - adjust),
            d = n(-0.25 - adjust),
            e = n(0.55 + adjust),
            f = n(1.05 + adjust),
            g = n(0.25 + adjust),
            h = n(0.85 + adjust),
        ));
    }
    if p.side() == "B" || p.flag("reverse") {
        out.push_str(&format!(
            r#"
          (fp_circle (center {a} 0) (end {b}  0) (layer B.SilkS) (width 0.1))
          (fp_line (start {c} 0) (end {d} 0) (layer B.SilkS) (width 0.1))

          (fp_circle (center {e} 0) (end {f} 0) (layer B.SilkS) (width 0.1))
          (fp_line (start {e} -0.3) (end {e} 0.3) (layer B.SilkS) (width 0.1))
          (fp_line (start {g} 0) (end {h} 0) (layer B.SilkS) (width 0.1))
      "#,
            a = n(-0.55 - adjust),
            b = n(-1.05 - adjust),
            c = n(-0.25 - adjust),
            d = n(-0.85 - adjust),
            e = n(0.55 + adjust),
            f = n(0.05 + adjust),
            g = n(0.85 + adjust),
            h = n(0.25 + adjust),
        ));
    }
    out.push_str("\n      )\n      ");
    Ok(out)
}

//! `infused-kim/pads`: a row of surface-mount pads with optional labels.
//!
//! Ported from `pads.js` of infused-kim/kb_ergogen_fp.
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0 (see the vendored LICENSE)
//! Author: @infused-kim
use crate::Result;
use crate::context::RenderContext;
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 21] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "PAD"),
    ParamSpec::boolean("reverse", true),
    ParamSpec::number("width", 1.25),
    ParamSpec::number("height", 2.5),
    ParamSpec::number("space", 2.0),
    ParamSpec::boolean("mirror", true),
    ParamSpec::number("pads", 2.0),
    ParamSpec::net_default("net_1", "PAD_1"),
    ParamSpec::net_default("net_2", "PAD_2"),
    ParamSpec::net_default("net_3", "PAD_3"),
    ParamSpec::net_default("net_4", "PAD_4"),
    ParamSpec::net_default("net_5", "PAD_5"),
    ParamSpec::net_default("net_6", "PAD_6"),
    ParamSpec::string("label_1", ""),
    ParamSpec::string("label_2", ""),
    ParamSpec::string("label_3", ""),
    ParamSpec::string("label_4", ""),
    ParamSpec::string("label_5", ""),
    ParamSpec::string("label_6", ""),
    ParamSpec::boolean("label_at_bottom", false),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "infused-kim/pads",
    display_name: "pads",
    kind: PartKind::Custom,
    matrix_terminals: None,
    keycap_parameters: None,
    parameters: &PARAMS,
    license: License {
        spdx: "CC-BY-NC-SA-4.0",
        author: "@infused-kim",
    },
    body,
};

struct Layout {
    width: f64,
    height: f64,
    space: f64,
    rotation: f64,
    label_at_bottom: bool,
}

fn pad(layout: &Layout, index: usize, count: usize, net: &str, label: &str, layer: &str) -> String {
    let Layout {
        width,
        height,
        space,
        rotation,
        label_at_bottom,
    } = *layout;
    let position_raw = (width + space) * index as f64;
    let position = position_raw - (width + space) * (count as f64 - 1.0) / 2.0;
    let mut label_y = -(height / 2.0 + 0.2);
    if label_at_bottom {
        label_y *= -1.0;
    }
    let turned = (rotation > 0.0 && rotation <= 180.0) || rotation <= -180.0;
    let direction = if !label_at_bottom || layer == "B" {
        if turned { "right" } else { "left" }
    } else if turned {
        "left"
    } else {
        "right"
    };
    let mirror = if layer == "B" { "mirror" } else { "" };
    let mut out = format!(
        "\n                (pad {} smd rect (at {} 0 {}) (size {} {}) (layers {layer}.Cu {layer}.Paste {layer}.Mask) {net})\n            ",
        index + 1,
        n(position),
        n(rotation),
        n(width),
        n(height),
    );
    if !label.is_empty() {
        out.push_str(&format!(
            "\n                (fp_text user \"{label}\" (at {} {} {}) (layer {layer}.SilkS)\n                  (effects (font (size 1 1) (thickness 0.1)) (justify {direction} {mirror}))\n                )\n              ",
            n(position),
            n(label_y),
            n(90.0 + rotation),
        ));
    }
    out
}

fn pads(layout: &Layout, nets: &[(String, String)], layer: &str, mirror: bool) -> String {
    let mut ordered: Vec<&(String, String)> = nets.iter().collect();
    if mirror {
        ordered.reverse();
    }
    ordered
        .iter()
        .enumerate()
        .map(|(index, (net, label))| pad(layout, index, nets.len(), net, label, layer))
        .collect()
}

fn body(p: &RenderContext<'_>) -> Result<String> {
    let requested = p.number("pads");
    let available = 6.0;
    let count = if requested > available {
        available
    } else {
        requested
    };
    let mut nets = Vec::new();
    let mut i = 0usize;
    while (i as f64) < count {
        nets.push((
            p.net(&format!("net_{}", i + 1)).to_string(),
            p.text(&format!("label_{}", i + 1)).to_owned(),
        ));
        i += 1;
    }
    let layout = Layout {
        width: p.number("width"),
        height: p.number("height"),
        space: p.number("space"),
        rotation: p.rotation(),
        label_at_bottom: p.flag("label_at_bottom"),
    };
    let front = if p.side() == "F" || p.flag("reverse") {
        pads(&layout, &nets, "F", false)
    } else {
        String::new()
    };
    let back = if p.side() == "B" || p.flag("reverse") {
        pads(&layout, &nets, "B", p.flag("mirror"))
    } else {
        String::new()
    };
    Ok(format!(
        r#"
          (module pads (layer F.Cu) (tedit 6446BF3D)
            {at}
            (attr smd)

            (fp_text reference "{reference}" (at 0 2.2) (layer F.SilkS) {hide}
              (effects (font (size 1 1) (thickness 0.15)))
            )
            {front}
            {back}
          )
        "#,
        at = p.at(),
        reference = p.reference(),
        hide = p.ref_hide(),
    ))
}

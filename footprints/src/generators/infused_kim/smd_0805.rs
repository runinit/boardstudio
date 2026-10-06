//! `infused-kim/smd_0805`
//!
//! Ported from `smd_0805.js` of infused-kim/kb_ergogen_fp.
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0 (see the vendored LICENSE)
//! Author: @infused-kim
use crate::Result;
use crate::context::RenderContext;
use crate::generators::util::{ModelDefaults, ModelParams, infused_model, model_side};
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 51] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "SMD"),
    ParamSpec::boolean("reverse", true),
    ParamSpec::number("space", 2.0),
    ParamSpec::boolean("mirror", true),
    ParamSpec::boolean("swap_pad_direction", false),
    ParamSpec::number("components", 2.0),
    ParamSpec::net_default("net_1_from", "SMD_1_F"),
    ParamSpec::net_default("net_1_to", "SMD_1_T"),
    ParamSpec::net_default("net_2_from", "SMD_2_F"),
    ParamSpec::net_default("net_2_to", "SMD_2_T"),
    ParamSpec::net_default("net_3_from", "SMD_3_F"),
    ParamSpec::net_default("net_3_to", "SMD_3_T"),
    ParamSpec::net_default("net_4_from", "SMD_4_F"),
    ParamSpec::net_default("net_4_to", "SMD_4_T"),
    ParamSpec::net_default("net_5_from", "SMD_5_F"),
    ParamSpec::net_default("net_5_to", "SMD_5_T"),
    ParamSpec::net_default("net_6_from", "SMD_6_F"),
    ParamSpec::net_default("net_6_to", "SMD_6_T"),
    ParamSpec::string("label_1", ""),
    ParamSpec::string("label_2", ""),
    ParamSpec::string("label_3", ""),
    ParamSpec::string("label_4", ""),
    ParamSpec::string("label_5", ""),
    ParamSpec::string("label_6", ""),
    ParamSpec::boolean("label_at_bottom", false),
    ParamSpec::string("component_3dmodel_side", ""),
    ParamSpec::string(
        "component_1_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/SMD_0805_Resistor.step",
    ),
    ParamSpec::array("component_1_3dmodel_xyz_scale", "[]"),
    ParamSpec::array("component_1_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("component_1_3dmodel_xyz_offset", "[]"),
    ParamSpec::string(
        "component_2_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/SMD_0805_Resistor.step",
    ),
    ParamSpec::array("component_2_3dmodel_xyz_scale", "[]"),
    ParamSpec::array("component_2_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("component_2_3dmodel_xyz_offset", "[]"),
    ParamSpec::string(
        "component_3_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/SMD_0805_Resistor.step",
    ),
    ParamSpec::array("component_3_3dmodel_xyz_scale", "[]"),
    ParamSpec::array("component_3_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("component_3_3dmodel_xyz_offset", "[]"),
    ParamSpec::string(
        "component_4_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/SMD_0805_Resistor.step",
    ),
    ParamSpec::array("component_4_3dmodel_xyz_scale", "[]"),
    ParamSpec::array("component_4_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("component_4_3dmodel_xyz_offset", "[]"),
    ParamSpec::string(
        "component_5_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/SMD_0805_Resistor.step",
    ),
    ParamSpec::array("component_5_3dmodel_xyz_scale", "[]"),
    ParamSpec::array("component_5_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("component_5_3dmodel_xyz_offset", "[]"),
    ParamSpec::string(
        "component_6_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/SMD_0805_Resistor.step",
    ),
    ParamSpec::array("component_6_3dmodel_xyz_scale", "[]"),
    ParamSpec::array("component_6_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("component_6_3dmodel_xyz_offset", "[]"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "infused-kim/smd_0805",
    display_name: "smd 0805",
    kind: PartKind::Passive,
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
    space: f64,
    rotation: f64,
    label_at_bottom: bool,
}

/// One component's 3D model, placed at the pad it belongs to.
fn model_for_net(p: &RenderContext<'_>, pad_idx: usize, pos_x: f64) -> Result<String> {
    let base = format!("component_{}", pad_idx + 1);
    if p.text(&format!("{base}_3dmodel_filename")).is_empty() {
        return Ok(String::new());
    }
    infused_model(
        p,
        ModelParams {
            filename: &format!("{base}_3dmodel_filename"),
            scale: &format!("{base}_3dmodel_xyz_scale"),
            rotation: &format!("{base}_3dmodel_xyz_rotation"),
            offset: &format!("{base}_3dmodel_xyz_offset"),
            side: "component_3dmodel_side",
        },
        ModelDefaults {
            default_side: "F",
            rotation_f: [0.0, 0.0, 0.0],
            offset_f: [pos_x, 0.0, 0.0],
            rotation_b: [0.0, 180.0, 0.0],
            offset_b: [if p.flag("mirror") { -pos_x } else { pos_x }, 0.0, -1.6],
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn gen_pad(
    p: &RenderContext<'_>,
    pad_idx: usize,
    pad_cnt: usize,
    net_from: &str,
    net_to: &str,
    net_label: &str,
    layout: &Layout,
    layer: &str,
) -> Result<String> {
    let Layout {
        space,
        rotation: rot,
        label_at_bottom,
    } = *layout;
    let width = 1.025;
    let height = 3.36;
    // Pad position from the center, so the row is centered.
    let pos_x_raw = (width + space) * pad_idx as f64;
    let pos_x = pos_x_raw - (width + space) * (pad_cnt as f64 - 1.0) / 2.0;
    let mut label_pos_y = -(height / 2.0 + 0.2);
    if label_at_bottom {
        label_pos_y *= -1.0;
    }
    let turned = (rot > 0.0 && rot <= 180.0) || rot <= -180.0;
    let direction = if !label_at_bottom || layer == "B" {
        if turned { "right" } else { "left" }
    } else if turned {
        "left"
    } else {
        "right"
    };
    let justify_mirror = if layer == "B" { "mirror" } else { "" };
    let label_justify = format!("(justify {direction} {justify_mirror})");
    let label_fab_justify = if justify_mirror.is_empty() {
        String::new()
    } else {
        format!("(justify {justify_mirror})")
    };
    let pad_num = (pad_idx * 2 + 1) as f64;
    let mut pad = format!(
        r####"
                (fp_line (start {e0} -1) (end {e0} 1) (layer {e1}.Fab) (width 0.1))
                (fp_line (start {e2} -1) (end {e0} -1) (layer {e1}.Fab) (width 0.1))
                (fp_line (start {e2} 1) (end {e2} -1) (layer {e1}.Fab) (width 0.1))
                (fp_line (start {e0} 1) (end {e2} 1) (layer {e1}.Fab) (width 0.1))

                (fp_line (start {e3} -1.68) (end {e3} 1.68) (layer {e1}.CrtYd) (width 0.05))
                (fp_line (start {e4} -1.68) (end {e3} -1.68) (layer {e1}.CrtYd) (width 0.05))
                (fp_line (start {e4} 1.68) (end {e4} -1.68) (layer {e1}.CrtYd) (width 0.05))
                (fp_line (start {e3} 1.68) (end {e4} 1.68) (layer {e1}.CrtYd) (width 0.05))

                (fp_line (start {e5} 0.227064) (end {e5} -0.227064) (layer {e1}.SilkS) (width 0.12))
                (fp_line (start {e6} 0.227064) (end {e6} -0.227064) (layer {e1}.SilkS) (width 0.12))

                (pad {e7} smd roundrect (at {e8} 0.9125 {e9}) (size 1.025 1.4) (layers {e1}.Cu {e1}.Paste {e1}.Mask) (roundrect_rratio 0.243902) {e10})
                (pad {e11} smd roundrect (at {e8} -0.9125 {e9}) (size 1.025 1.4) (layers {e1}.Cu {e1}.Paste {e1}.Mask) (roundrect_rratio 0.243902) {e12})
            "####,
        e0 = n(0.625 + pos_x),
        e1 = layer,
        e2 = n(-0.625 + pos_x),
        e3 = n(0.95 + pos_x),
        e4 = n(-0.95 + pos_x),
        e5 = n(0.735 + pos_x),
        e6 = n(-0.735 + pos_x),
        e7 = n(pad_num),
        e8 = n(0.0 + pos_x),
        e9 = n(90.0 + rot),
        e10 = net_from,
        e11 = n(pad_num + 1.0),
        e12 = net_to
    );
    if !net_label.is_empty() {
        pad += &format!(
            r####"
              (fp_text user "{e0}" (at {e1} 0 {e2}) (layer {e3}.Fab)
                (effects (font (size 0.5 0.5) (thickness 0.08)) {e4})
              )
              (fp_text user "{e0}" (at {e5} {e6} {e2}) (layer {e3}.SilkS)
                  (effects (font (size 1 1) (thickness 0.1)) {e7})
                )
              "####,
            e0 = net_label,
            e1 = n(0.0 + pos_x),
            e2 = n(90.0 + rot),
            e3 = layer,
            e4 = label_fab_justify,
            e5 = n(pos_x),
            e6 = n(label_pos_y),
            e7 = label_justify
        );
    }
    if layer == model_side(p, p.text("component_3dmodel_side"), "F") {
        pad += &model_for_net(p, pad_idx, pos_x)?;
    }
    Ok(pad)
}

fn gen_pads(
    p: &RenderContext<'_>,
    nets: &[[String; 3]],
    layout: &Layout,
    layer: &str,
    mirror: bool,
    swap: bool,
) -> Result<String> {
    let mut ordered: Vec<&[String; 3]> = nets.iter().collect();
    if mirror {
        ordered.reverse();
    }
    let mut pads = String::new();
    for (index, net) in ordered.iter().enumerate() {
        let (from, to) = if swap {
            (&net[1], &net[0])
        } else {
            (&net[0], &net[1])
        };
        pads += &gen_pad(p, index, ordered.len(), from, to, &net[2], layout, layer)?;
    }
    Ok(pads)
}

fn body(p: &RenderContext<'_>) -> Result<String> {
    let mut nets: Vec<[String; 3]> = Vec::new();
    let requested = p.number("components");
    let count = if requested > 6.0 { 6.0 } else { requested };
    let mut i = 0usize;
    while (i as f64) < count {
        nets.push([
            p.net(&format!("net_{}_from", i + 1)).to_string(),
            p.net(&format!("net_{}_to", i + 1)).to_string(),
            p.text(&format!("label_{}", i + 1)).to_owned(),
        ]);
        i += 1;
    }
    let layout = Layout {
        space: p.number("space"),
        rotation: p.rotation(),
        label_at_bottom: p.flag("label_at_bottom"),
    };
    let (side, reverse, swap) = (p.side(), p.flag("reverse"), p.flag("swap_pad_direction"));
    let pads_front = if side == "F" || reverse {
        gen_pads(p, &nets, &layout, "F", false, swap)?
    } else {
        String::new()
    };
    let pads_back = if side == "B" || reverse {
        gen_pads(p, &nets, &layout, "B", p.flag("mirror"), swap)?
    } else {
        String::new()
    };
    Ok(format!(
        r####"
          (module smd_805 (layer F.Cu) (tedit 6446BF3D)
            {e0}
            (attr smd)

            (fp_text reference "{e1}" (at 0 3) (layer F.SilkS) {e2}
              (effects (font (size 1 1) (thickness 0.15)))
            )
            {e3}
            {e4}
          )
        "####,
        e0 = p.at(),
        e1 = p.reference(),
        e2 = p.ref_hide(),
        e3 = pads_front,
        e4 = pads_back
    ))
}

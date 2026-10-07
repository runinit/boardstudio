//! `ceoloide/switch_choc_v1_v2`
//!
//! Ported from `switch_choc_v1_v2.js` of ceoloide/ergogen-footprints.
//! Copyright (c) 2023 Marco Massarelli
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0
//! Authors: @ergogen + @infused-kim, @ceoloide, @grazfather, @nxtk improvements
use crate::Result;
use crate::context::RenderContext;
use crate::error::GeneratorError;
use crate::generators::util::json_string;
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, KeycapParameters, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 47] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "S"),
    ParamSpec::number("pcb_thickness", 1.6),
    ParamSpec::boolean("reversible", false),
    ParamSpec::boolean("hotswap_pads_same_side", false),
    ParamSpec::boolean("include_traces_vias", true),
    ParamSpec::number("trace_width", 0.2),
    ParamSpec::number("via_size", 0.6),
    ParamSpec::number("via_drill", 0.3),
    ParamSpec::boolean("locked_traces_vias", false),
    ParamSpec::boolean("hotswap", true),
    ParamSpec::boolean("include_plated_holes", false),
    ParamSpec::boolean("include_stabilizer_nets", false),
    ParamSpec::boolean("include_centerhole_net", false),
    ParamSpec::boolean("solder", false),
    ParamSpec::number("outer_pad_width_front", 2.6),
    ParamSpec::number("outer_pad_width_back", 2.6),
    ParamSpec::boolean("include_keycap", true),
    ParamSpec::number("keycap_width", 18.0),
    ParamSpec::number("keycap_height", 18.0),
    ParamSpec::boolean("include_corner_marks", false),
    ParamSpec::boolean("include_choc_v1_led_cutout_marks", false),
    ParamSpec::boolean("include_choc_v2_led_cutout_marks", false),
    ParamSpec::boolean("include_stabilizer_pad", true),
    ParamSpec::boolean("oval_stabilizer_pad", false),
    ParamSpec::boolean("choc_v1_support", true),
    ParamSpec::boolean("choc_v2_support", true),
    ParamSpec::number("choc_v1_stabilizers_diameter", 1.9),
    ParamSpec::number("center_hole_diameter", 0.0),
    ParamSpec::boolean("allow_soldermask_bridges", true),
    ParamSpec::string(
        "switch_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/Choc_V1_Switch.step",
    ),
    ParamSpec::array("switch_3dmodel_xyz_offset", "[]"),
    ParamSpec::array("switch_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("switch_3dmodel_xyz_scale", "[1,1,1]"),
    ParamSpec::string(
        "hotswap_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/Choc_V1_Hotswap.step",
    ),
    ParamSpec::array("hotswap_3dmodel_xyz_offset", "[]"),
    ParamSpec::array("hotswap_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("hotswap_3dmodel_xyz_scale", "[1,1,1]"),
    ParamSpec::string(
        "keycap_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/Choc_V1_Keycap_MBK_Black_1u.step",
    ),
    ParamSpec::array("keycap_3dmodel_xyz_offset", "[]"),
    ParamSpec::array("keycap_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("keycap_3dmodel_xyz_scale", "[1,1,1]"),
    ParamSpec::net("from"),
    ParamSpec::net("to"),
    ParamSpec::net_default("CENTERHOLE", "GND"),
    ParamSpec::net_default("LEFTSTAB", "D1"),
    ParamSpec::net_default("RIGHTSTAB", "D2"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "ceoloide/switch_choc_v1_v2",
    display_name: "switch choc v1 v2",
    kind: PartKind::Switch,
    matrix_terminals: Some(("from", "to")),
    keycap_parameters: Some(KeycapParameters {
        width: "keycap_width",
        height: "keycap_height",
    }),
    parameters: &PARAMS,
    license: License {
        spdx: "CC-BY-NC-SA-4.0",
        author: "@ergogen + @infused-kim, @ceoloide, @grazfather, @nxtk improvements",
    },
    body,
};

fn body(p: &RenderContext<'_>) -> Result<String> {
    const DEFAULT_SWITCH: &str = "${KIPRJMOD}/models/boardstudio/infused-kim/Choc_V1_Switch.step";
    const DEFAULT_HOTSWAP: &str = "${KIPRJMOD}/models/boardstudio/infused-kim/Choc_V1_Hotswap.step";
    const DEFAULT_KEYCAP: &str =
        "${KIPRJMOD}/models/boardstudio/infused-kim/Choc_V1_Keycap_MBK_Black_1u.step";
    const CHOC_V2_SWITCH: &str = "${KIPRJMOD}/models/boardstudio/koktoh/Choc_V2_Red.step";
    for name in [
        "switch_3dmodel_xyz_scale",
        "hotswap_3dmodel_xyz_scale",
        "keycap_3dmodel_xyz_scale",
    ] {
        p.vec3(name)?;
    }

    // Without Choc V1 support the bundled V1 models no longer match the footprint.
    let mut switch_filename = p.text("switch_3dmodel_filename");
    let mut hotswap_filename = p.text("hotswap_3dmodel_filename");
    let mut keycap_filename = p.text("keycap_3dmodel_filename");
    let mut switch_rotation_param = p.vec3("switch_3dmodel_xyz_rotation")?;
    let mut switch_offset_param = p.vec3("switch_3dmodel_xyz_offset")?;
    if !p.flag("choc_v1_support") {
        if !p.flag("choc_v2_support") {
            for filename in [&mut switch_filename, &mut hotswap_filename] {
                if *filename == DEFAULT_SWITCH || *filename == DEFAULT_HOTSWAP {
                    *filename = "";
                }
            }
            if keycap_filename == DEFAULT_KEYCAP {
                keycap_filename = "";
            }
        } else {
            if switch_filename == DEFAULT_SWITCH {
                let automatic = switch_rotation_param.is_none()
                    && switch_offset_param.is_none()
                    && p.list("switch_3dmodel_xyz_scale")
                        .iter()
                        .all(|value| value.as_f64() == Some(1.0));
                let post_diameter = 4.8;
                let center_drill = p.number("center_hole_diameter");
                if automatic
                    && (!p.flag("include_stabilizer_pad")
                        || p.flag("oval_stabilizer_pad")
                        || (center_drill > 0.0 && center_drill < post_diameter))
                {
                    return Err(GeneratorError::rejected(
                        SPEC.source,
                        Some("switch_3dmodel_filename"),
                        "The bundled Choc V2 model requires the round stabilizer hole and a center drill of at least 4.8 mm. Use a matching model or transform for modified hardware.",
                    ));
                }
                switch_filename = CHOC_V2_SWITCH;
                if switch_rotation_param.is_none() {
                    switch_rotation_param = Some(if p.side() == "F" {
                        [180.0, 0.0, 0.0]
                    } else {
                        [0.0, 180.0, 0.0]
                    });
                }
                if switch_offset_param.is_none() {
                    switch_offset_param = Some([0.0, 0.0, -p.number("pcb_thickness")]);
                }
            }
            if keycap_filename == DEFAULT_KEYCAP {
                keycap_filename = "";
            }
        }
    }

    let center_hole_diameter = if p.number("center_hole_diameter") > 0.0 {
        p.number("center_hole_diameter")
    } else if p.flag("choc_v2_support") {
        5.0
    } else {
        3.4
    };
    let keycap_xo = 0.5 * p.number("keycap_width");
    let keycap_yo = 0.5 * p.number("keycap_height");

    // If both hotswap and solder are enabled, move the solder holes "down" to
    // the opposite side of the switch.
    let swapped = p.flag("hotswap") && p.flag("solder");
    let (
        solder_offset_x_front,
        solder_offset_x_back,
        solder_offset_y,
        stab_offset_x_front,
        stab_offset_x_back,
        stab_offset_y,
    ) = if swapped {
        ("", "-", "", "-", "", "")
    } else {
        ("-", "", "-", "", "-", "")
    };

    // `side` selects the socket; the switch and cap sit across the PCB.
    let keycap_height = 6.6;
    let pcb_thickness = p.number("pcb_thickness");
    let model_rotation = if p.side() == "B" {
        [180.0, 0.0, 0.0]
    } else {
        [0.0, 180.0, 0.0]
    };
    let model_offset = [0.0, 0.0, -pcb_thickness];
    let cap_offset = [0.0, 0.0, -pcb_thickness - keycap_height];
    let switch_rotation = switch_rotation_param.unwrap_or(model_rotation);
    let switch_offset = switch_offset_param.unwrap_or(model_offset);
    let hotswap_rotation = p
        .vec3("hotswap_3dmodel_xyz_rotation")?
        .unwrap_or(model_rotation);
    let hotswap_offset = p
        .vec3("hotswap_3dmodel_xyz_offset")?
        .unwrap_or(model_offset);
    let keycap_rotation = p
        .vec3("keycap_3dmodel_xyz_rotation")?
        .unwrap_or(model_rotation);
    let keycap_offset = p.vec3("keycap_3dmodel_xyz_offset")?.unwrap_or(cap_offset);

    let common_top = format!(
        r####"
  (footprint "ceoloide:switch_choc_v1_v2"
    (layer "{e0}.Cu")
    {e1}
    (property "Reference" "{e2}"
      (at 0 8.8 {e3})
      (layer "{e0}.SilkS")
      {e4}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (attr exclude_from_pos_files exclude_from_bom{e5})

    {e6}
    {e7}
    "####,
        e0 = p.side(),
        e1 = p.at(),
        e2 = p.reference(),
        e3 = n(p.rotation()),
        e4 = p.ref_hide(),
        e5 = if p.flag("allow_soldermask_bridges") {
            " allow_soldermask_bridges".to_string()
        } else {
            "".to_string()
        },
        e6 = "",
        e7 = if p.flag("include_plated_holes") {
            format!(
                r####"
    (pad "" thru_hole circle (at 0 0 {e0}) (size {e1} {e1}) (drill {e2}) (layers "*.Cu" "*.Mask") {e3})
    "####,
                e0 = n(p.rotation()),
                e1 = n(center_hole_diameter + 0.3),
                e2 = n(center_hole_diameter),
                e3 = if p.flag("include_centerhole_net") {
                    p.net("CENTERHOLE").to_string()
                } else {
                    "".to_string()
                }
            )
        } else {
            format!(
                r####"
    (pad "" np_thru_hole circle (at 0 0 {e0}) (size {e1} {e1}) (drill {e1}) (layers "*.Cu" "*.Mask"))
    "####,
                e0 = n(p.rotation()),
                e1 = n(center_hole_diameter)
            )
        }
    );

    let choc_v1_stabilizers = format!(
        r####"
    {e0}
    "####,
        e0 = if p.flag("include_plated_holes") {
            format!(
                r####"
    (pad "" thru_hole circle (at 5.5 0 {e0}) (size {e1} {e1}) (drill {e2}) (layers "*.Cu" "*.Mask"))
    (pad "" thru_hole circle (at -5.5 0 {e0}) (size {e1} {e1}) (drill {e2}) (layers "*.Cu" "*.Mask"))
    "####,
                e0 = n(p.rotation()),
                e1 = n(p.number("choc_v1_stabilizers_diameter") + 0.3),
                e2 = n(p.number("choc_v1_stabilizers_diameter"))
            )
        } else {
            format!(
                r####"
    (pad "" np_thru_hole circle (at 5.5 0 {e0}) (size {e1} {e1}) (drill {e1}) (layers "*.Cu" "*.Mask"))
    (pad "" np_thru_hole circle (at -5.5 0 {e0}) (size {e1} {e1}) (drill {e1}) (layers "*.Cu" "*.Mask"))
    "####,
                e0 = n(p.rotation()),
                e1 = n(p.number("choc_v1_stabilizers_diameter"))
            )
        }
    );

    let corner_marks = format!(
        r####"
    {e0}
    (fp_line (start -7 -6) (end -7 -7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -7 7) (end -6 7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -6 -7) (end -7 -7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -7 7) (end -7 6) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start 7 6) (end 7 7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start 7 -7) (end 6 -7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start 6 7) (end 7 7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start 7 -7) (end 7 -6) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    "####,
        e0 = ""
    );

    let keycap_marks = format!(
        r####"
    (fp_rect (start {e0} {e1}) (end {e2} {e3}) (layer "Dwgs.User") (stroke (width 0.15) (type solid)) (fill none))
    "####,
        e0 = n(keycap_xo),
        e1 = n(keycap_yo),
        e2 = n(-keycap_xo),
        e3 = n(-keycap_yo)
    );

    let choc_v1_led_cutout_marks = String::from(
        r####"
    (fp_rect (start -2.65 6.325) (end 2.65 3.075) (layer "Dwgs.User") (width 0.15) (stroke (width 0.15) (type solid)) (fill none))
    "####,
    );

    let choc_v2_led_cutout_marks = String::from(
        r####"
    (fp_rect (start -2.75 6.405) (end 2.75 3.455) (layer "Dwgs.User") (width 0.15) (stroke (width 0.15) (type solid)) (fill none))
    "####,
    );

    let hotswap_common = format!(
        r####"
    {e0}
    {e1}
    "####,
        e0 = "",
        e1 = if p.flag("include_plated_holes") {
            format!(
                r####"
    (pad {e0} thru_hole circle (at 0 -5.95 {e1}) (size 3.3 3.3) (drill 3) (layers "*.Cu" "*.Mask") {e2})
    "####,
                e0 = if p.flag("reversible") {
                    "\"\"".to_string()
                } else {
                    "\"1\"".to_string()
                },
                e1 = n(p.rotation()),
                e2 = if p.flag("reversible") {
                    "".to_string()
                } else {
                    p.net("from").to_string()
                }
            )
        } else {
            format!(
                r####"
    (pad "" np_thru_hole circle (at 0 -5.95 {e0}) (size 3 3) (drill 3) (layers "*.Cu" "*.Mask"))
    "####,
                e0 = n(p.rotation())
            )
        }
    );

    let hotswap_back_pads_plated = format!(
        r####"
    (pad "1" smd roundrect (at -2.648 -5.95 {e0}) (size 3.8 2.15) (layers "B.Cu") (roundrect_rratio 0.1) {e1})
    (pad "" smd roundrect (at -3.248 -5.95 {e0}) (size 2.6 2.15) (layers "B.Paste" "B.Mask") (roundrect_rratio 0.1))
    (pad "2" smd roundrect (at {e2} -3.75 {e0}) (size {e3} 2.15) (layers "B.Cu") (roundrect_rratio 0.1) {e4})
    (pad "" smd roundrect (at {e5} -3.75 {e0}) (size {e6} 2.15) (layers "B.Paste" "B.Mask") (roundrect_rratio {e7}))
    "####,
        e0 = n(p.rotation()),
        e1 = p.net("from"),
        e2 = n(7.6475 - (2.6 - p.number("outer_pad_width_back")) / 2.0),
        e3 = n(p.number("outer_pad_width_back") + 1.2),
        e4 = p.net("to"),
        e5 = n(8.2475 - (2.6 - p.number("outer_pad_width_back")) / 2.0),
        e6 = n(p.number("outer_pad_width_back")),
        e7 = if (2.15 / p.number("outer_pad_width_back")) <= (1.0) {
            n(0.1)
        } else {
            n(0.1 * (2.15 / p.number("outer_pad_width_back")))
        }
    );

    let hotswap_front_pads_plated = format!(
        r####"
    (pad "1" smd roundrect (at 2.648 -5.95 {e0}) (size 3.8 2.15) (layers "F.Cu") (roundrect_rratio 0.1) {e1})
    (pad "" smd roundrect (at 3.248 -5.95 {e0}) (size 2.6 2.15) (layers "F.Paste" "F.Mask") (roundrect_rratio 0.1)) 
    (pad "2" smd roundrect (at {e2} -3.75 {e0}) (size {e3} 2.15) (layers "F.Cu") (roundrect_rratio 0.1) {e4})
    (pad "" smd roundrect (at {e5} -3.75 {e0}) (size {e6} 2.15) (layers "F.Paste" "F.Mask") (roundrect_rratio {e7}))
    "####,
        e0 = n(p.rotation()),
        e1 = p.net("from"),
        e2 = n(-7.6475 + (2.6 - p.number("outer_pad_width_front")) / 2.0),
        e3 = n(p.number("outer_pad_width_front") + 1.2),
        e4 = p.net("to"),
        e5 = n(-8.2475 + (2.6 - p.number("outer_pad_width_front")) / 2.0),
        e6 = n(p.number("outer_pad_width_front")),
        e7 = if (2.15 / p.number("outer_pad_width_front")) <= (1.0) {
            n(0.1)
        } else {
            n(0.1 * (2.15 / p.number("outer_pad_width_front")))
        }
    );

    let hotswap_back_pads_plated_reversible = format!(
        r####"
    (pad "1" smd roundrect (at -3.245 -5.95 {e0}) (size 2.65 2.15) (layers "B.Cu" "B.Paste" "B.Mask") (roundrect_rratio 0.1) {e1})
    (pad "2" smd roundrect (at {e2} -3.75 {e0}) (size {e3} 2.15) (layers "B.Cu") (roundrect_rratio 0.1) {e4})
    (pad "" smd roundrect (at {e5} -3.75 {e0}) (size {e6} 2.15) (layers "B.Paste" "B.Mask") (roundrect_rratio {e7}))
    "####,
        e0 = n(p.rotation()),
        e1 = p.net("from"),
        e2 = n(7.6475 - (2.6 - p.number("outer_pad_width_back")) / 2.0),
        e3 = n(p.number("outer_pad_width_back") + 1.2),
        e4 = p.net("to"),
        e5 = n(8.2475 - (2.6 - p.number("outer_pad_width_back")) / 2.0),
        e6 = n(p.number("outer_pad_width_back")),
        e7 = if (2.15 / p.number("outer_pad_width_back")) <= (1.0) {
            n(0.1)
        } else {
            n(0.1 * (2.15 / p.number("outer_pad_width_back")))
        }
    );

    let hotswap_front_pads_plated_reversible = format!(
        r####"
    (pad "2" smd roundrect (at 3.245 -5.95 {e0}) (size 2.65 2.15) (layers "F.Cu" "F.Paste" "F.Mask") (roundrect_rratio 0.1) {e1})
    (pad "1" smd roundrect (at {e2} -3.75 {e0}) (size {e3} 2.15) (layers "F.Cu") (roundrect_rratio 0.1) {e4})
    (pad "" smd roundrect (at {e5} -3.75 {e0}) (size {e6} 2.15) (layers "F.Paste" "F.Mask") (roundrect_rratio {e7}))
    "####,
        e0 = n(p.rotation()),
        e1 = p.net("to"),
        e2 = n(-7.6475 + (2.6 - p.number("outer_pad_width_front")) / 2.0),
        e3 = n(p.number("outer_pad_width_front") + 1.2),
        e4 = p.net("from"),
        e5 = n(-8.2475 + (2.6 - p.number("outer_pad_width_front")) / 2.0),
        e6 = n(p.number("outer_pad_width_front")),
        e7 = if (2.15 / p.number("outer_pad_width_front")) <= (1.0) {
            n(0.1)
        } else {
            n(0.1 * (2.15 / p.number("outer_pad_width_front")))
        }
    );

    let hotswap_front_pad_cutoff = format!(
        r####"
    (pad "1" smd roundrect
      (at 3.275 -5.95 {e0})
      (size 2.6 2.6)
      (layers "F.Cu" "F.Paste" "F.Mask")
      (roundrect_rratio 0)
			(chamfer_ratio 0.455)
			(chamfer bottom_right)
      {e1}
    )
    "####,
        e0 = n(p.rotation()),
        e1 = p.net("from")
    );

    let hotswap_front_pad_full = format!(
        r####"
    (pad "1" smd rect (at 3.275 -5.95 {e0}) (size 2.6 2.6) (layers "F.Cu" "F.Paste" "F.Mask") {e1})
    "####,
        e0 = n(p.rotation()),
        e1 = p.net("from")
    );

    let hotswap_back_pad_cutoff = format!(
        r####"
    (pad "1" smd roundrect
      (at -3.275 -5.95 {e0})
      (size 2.6 2.6)
      (layers "B.Cu" "B.Paste" "B.Mask")
      (roundrect_rratio 0)
			(chamfer_ratio 0.455)
			(chamfer bottom_left)
      {e1}
    )
    "####,
        e0 = n(p.rotation()),
        e1 = if p.flag("hotswap_pads_same_side") {
            p.net("to").to_string()
        } else {
            p.net("from").to_string()
        }
    );

    let hotswap_back_pad_full = format!(
        r####"
    (pad "1" smd rect (at -3.275 -5.95 {e0}) (size 2.6 2.6) (layers "B.Cu" "B.Paste" "B.Mask") {e1})
    "####,
        e0 = n(p.rotation()),
        e1 = if p.flag("hotswap_pads_same_side") {
            p.net("to").to_string()
        } else {
            p.net("from").to_string()
        }
    );

    let hotswap_back = format!(
        r####"
    {e0}
    {e1}
    (fp_line (start -1.5 -8.2) (end -2 -7.7) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 1.5 -8.2) (end -1.5 -8.2) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 2 -7.7) (end 1.5 -8.2) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 2 -7.7) (end 2 -6.78) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 2.52 -6.2) (end 7 -6.2) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 7 -6.2) (end 7 -5.6) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_arc (start 2.52 -6.2) (mid 2.139878 -6.382304) (end 2 -6.78) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
  
    {e2}
    (fp_line (start -1.5 -3.7) (end -2 -4.2) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 0.8 -3.7) (end -1.5 -3.7) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 2.5 -1.5) (end 2.5 -2.2) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 7 -1.5) (end 2.5 -1.5) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 7 -2) (end 7 -1.5) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_arc (start 0.8 -3.7) (mid 1.956518 -3.312082) (end 2.5 -2.22) (layer "B.SilkS") (stroke (width 0.15) (type solid)))

    {e3}
    "####,
        e0 = "",
        e1 = "",
        e2 = "",
        e3 = if p.flag("include_plated_holes") {
            format!(
                r####"
    {e0}
      {e1}
    "####,
                e0 = "",
                e1 = if p.flag("reversible") {
                    format!(
                        r####"
      (pad "2" thru_hole circle (at 5 -3.75 {e0}) (size 3.3 3.3) (drill 3) (layers "*.Cu" "*.Mask") {e1})
      {e2}
      {e3}
      "####,
                        e0 = n(195.0 + p.rotation()),
                        e1 = p.net("to"),
                        e2 = "",
                        e3 = hotswap_back_pads_plated_reversible.clone()
                    )
                } else {
                    format!(
                        r####"
      (pad "2" thru_hole circle (at 5 -3.75 {e0}) (size 3.3 3.3) (drill 3) (layers "*.Cu" "*.Mask") {e1})
      {e2}
      {e3}
      "####,
                        e0 = n(195.0 + p.rotation()),
                        e1 = p.net("to"),
                        e2 = "",
                        e3 = hotswap_back_pads_plated.clone()
                    )
                }
            )
        } else {
            format!(
                r####"
    {e0}
    {e1}

    {e0}
    (pad "2" smd rect (at {e2} -3.75 {e3}) (size {e4} 2.6) (layers "B.Cu" "B.Paste" "B.Mask") {e5})

    {e0}
    (pad "" np_thru_hole circle (at 5 -3.75 {e6}) (size 3 3) (drill 3) (layers "*.Cu" "*.Mask"))
    "####,
                e0 = "",
                e1 = if p.flag("reversible") {
                    hotswap_back_pad_cutoff.clone()
                } else {
                    hotswap_back_pad_full.clone()
                },
                e2 = n(8.275 - (2.6 - p.number("outer_pad_width_back")) / 2.0),
                e3 = n(p.rotation()),
                e4 = n(p.number("outer_pad_width_back")),
                e5 = if p.flag("hotswap_pads_same_side") {
                    p.net("from").to_string()
                } else {
                    p.net("to").to_string()
                },
                e6 = n(195.0 + p.rotation())
            )
        }
    );

    let hotswap_front = format!(
        r####"
    {e0}
    {e1}
    (fp_line (start -7 -5.6) (end -7 -6.2) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start -7 -6.2) (end -2.52 -6.2) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start -2 -6.78) (end -2 -7.7) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start -1.5 -8.2) (end -2 -7.7) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 1.5 -8.2) (end -1.5 -8.2) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 2 -7.7) (end 1.5 -8.2) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_arc (start -2 -6.78) (mid -2.139878 -6.382304) (end -2.52 -6.2) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
  
    {e2}
    (fp_line (start -7 -1.5) (end -7 -2) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start -2.5 -1.5) (end -7 -1.5) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start -2.5 -2.2) (end -2.5 -1.5) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 1.5 -3.7) (end -0.8 -3.7) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 2 -4.2) (end 1.5 -3.7) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_arc (start -2.5 -2.22) (mid -1.956518 -3.312082) (end -0.8 -3.7) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    
    {e3}
    "####,
        e0 = "",
        e1 = "",
        e2 = "",
        e3 = if p.flag("include_plated_holes") {
            format!(
                r####"
    {e0}
    {e1}
    "####,
                e0 = "",
                e1 = if p.flag("reversible") {
                    format!(
                        r####"
    (pad "1" thru_hole circle (at -5 -3.75 {e0}) (size 3.3 3.3) (drill 3) (layers "*.Cu" "*.Mask") {e1})
    {e2}
    {e3}
    "####,
                        e0 = n(195.0 + p.rotation()),
                        e1 = p.net("from"),
                        e2 = "",
                        e3 = hotswap_front_pads_plated_reversible.clone()
                    )
                } else {
                    format!(
                        r####"
    (pad "2" thru_hole circle (at -5 -3.75 {e0}) (size 3.3 3.3) (drill 3) (layers "*.Cu" "*.Mask") {e1})
    {e2}
    {e3}
    "####,
                        e0 = n(195.0 + p.rotation()),
                        e1 = p.net("to"),
                        e2 = "",
                        e3 = hotswap_front_pads_plated.clone()
                    )
                }
            )
        } else {
            format!(
                r####"
    {e0}
    (pad "" np_thru_hole circle (at -5 -3.75 {e1}) (size 3 3) (drill 3) (layers "*.Cu" "*.Mask"))

    {e0}
    {e2}

    {e0}
    (pad "2" smd rect (at {e3} -3.75 {e4}) (size {e5} 2.6) (layers "F.Cu" "F.Paste" "F.Mask") {e6})
    "####,
                e0 = "",
                e1 = n(195.0 + p.rotation()),
                e2 = if p.flag("reversible") {
                    hotswap_front_pad_cutoff.clone()
                } else {
                    hotswap_front_pad_full.clone()
                },
                e3 = n(-8.275 + (2.6 - p.number("outer_pad_width_front")) / 2.0),
                e4 = n(p.rotation()),
                e5 = n(p.number("outer_pad_width_front")),
                e6 = p.net("to")
            )
        }
    );

    let solder_common = format!(
        r####"
    (pad "2" thru_hole circle (at 0 {e0}5.9 {e1}) (size 2.032 2.032) (drill 1.27) (layers "*.Cu" "*.Mask") {e2})
    "####,
        e0 = solder_offset_y,
        e1 = n(195.0 + p.rotation()),
        e2 = p.net("from")
    );

    let solder_front = format!(
        r####"
    (pad "1" thru_hole circle (at {e0}5 {e1}3.8 {e2}) (size 2.032 2.032) (drill 1.27) (layers "*.Cu" "*.Mask") {e3})
    "####,
        e0 = solder_offset_x_front,
        e1 = solder_offset_y,
        e2 = n(195.0 + p.rotation()),
        e3 = p.net("to")
    );

    let solder_back = format!(
        r####"
    (pad "1" thru_hole circle (at {e0}5 {e1}3.8 {e2}) (size 2.032 2.032) (drill 1.27) (layers "*.Cu" "*.Mask") {e3})
    "####,
        e0 = solder_offset_x_back,
        e1 = solder_offset_y,
        e2 = n(195.0 + p.rotation()),
        e3 = p.net("to")
    );

    let oval_corner_stab_front = format!(
        r####"
    (pad "" thru_hole oval (at {e0}5 {e1}5.15 {e2}) (size 2.4 1.2) (drill oval 1.6 0.4) (layers "*.Cu" "*.Mask") {e3})
    "####,
        e0 = stab_offset_x_front,
        e1 = stab_offset_y,
        e2 = n(p.rotation()),
        e3 = if swapped {
            p.net("to").to_string()
        } else if p.flag("include_stabilizer_nets") {
            p.net("RIGHTSTAB").to_string()
        } else {
            String::new()
        }
    );

    let oval_corner_stab_back = format!(
        r####"
    (pad "" thru_hole oval (at {e0}5 {e1}5.15 {e2}) (size 2.4 1.2) (drill oval 1.6 0.4) (layers "*.Cu" "*.Mask") {e3})
    "####,
        e0 = stab_offset_x_back,
        e1 = stab_offset_y,
        e2 = n(p.rotation()),
        e3 = if swapped {
            p.net("to").to_string()
        } else if p.flag("include_stabilizer_nets") {
            p.net("LEFTSTAB").to_string()
        } else {
            String::new()
        }
    );

    let round_corner_stab_front = format!(
        r####"
    (pad "" thru_hole circle (at {e0}5.00 {e1}5.15 {e2}) (size 1.9 1.9) (drill 1.6) (layers "*.Cu" "*.Mask") {e3})
    "####,
        e0 = stab_offset_x_front,
        e1 = stab_offset_y,
        e2 = n(p.rotation()),
        e3 = if swapped {
            p.net("to").to_string()
        } else if p.flag("include_stabilizer_nets") {
            p.net("RIGHTSTAB").to_string()
        } else {
            String::new()
        }
    );

    let round_corner_stab_back = format!(
        r####"
    (pad "" thru_hole circle (at {e0}5.00 {e1}5.15 {e2}) (size 1.9 1.9) (drill 1.6) (layers "*.Cu" "*.Mask") {e3})
    "####,
        e0 = stab_offset_x_back,
        e1 = stab_offset_y,
        e2 = n(p.rotation()),
        e3 = if swapped {
            p.net("to").to_string()
        } else if p.flag("include_stabilizer_nets") {
            p.net("LEFTSTAB").to_string()
        } else {
            String::new()
        }
    );

    let switch_3dmodel = format!(
        r####"
    (model {e0}
      (offset (xyz {e1} {e2} {e3}))
      (scale (xyz {e4} {e5} {e6}))
      (rotate (xyz {e7} {e8} {e9}))
    )
    "####,
        e0 = json_string(switch_filename),
        e1 = n(switch_offset[0]),
        e2 = n(switch_offset[1]),
        e3 = n(switch_offset[2]),
        e4 = n(p.component("switch_3dmodel_xyz_scale", 0)),
        e5 = n(p.component("switch_3dmodel_xyz_scale", 1)),
        e6 = n(p.component("switch_3dmodel_xyz_scale", 2)),
        e7 = n(switch_rotation[0]),
        e8 = n(switch_rotation[1]),
        e9 = n(switch_rotation[2])
    );

    let hotswap_3dmodel = format!(
        r####"
    (model {e0}
      (offset (xyz {e1} {e2} {e3}))
      (scale (xyz {e4} {e5} {e6}))
      (rotate (xyz {e7} {e8} {e9}))
    )
	  "####,
        e0 = json_string(hotswap_filename),
        e1 = n(hotswap_offset[0]),
        e2 = n(hotswap_offset[1]),
        e3 = n(hotswap_offset[2]),
        e4 = n(p.component("hotswap_3dmodel_xyz_scale", 0)),
        e5 = n(p.component("hotswap_3dmodel_xyz_scale", 1)),
        e6 = n(p.component("hotswap_3dmodel_xyz_scale", 2)),
        e7 = n(hotswap_rotation[0]),
        e8 = n(hotswap_rotation[1]),
        e9 = n(hotswap_rotation[2])
    );

    let keycap_3dmodel = format!(
        r####"
    (model {e0}
      (offset (xyz {e1} {e2} {e3}))
      (scale (xyz {e4} {e5} {e6}))
      (rotate (xyz {e7} {e8} {e9}))
    )
	  "####,
        e0 = json_string(keycap_filename),
        e1 = n(keycap_offset[0]),
        e2 = n(keycap_offset[1]),
        e3 = n(keycap_offset[2]),
        e4 = n(p.component("keycap_3dmodel_xyz_scale", 0)),
        e5 = n(p.component("keycap_3dmodel_xyz_scale", 1)),
        e6 = n(p.component("keycap_3dmodel_xyz_scale", 2)),
        e7 = n(keycap_rotation[0]),
        e8 = n(keycap_rotation[1]),
        e9 = n(keycap_rotation[2])
    );

    let common_bottom = String::from(
        r####"
  )
    "####,
    );

    let hotswap_routes_unplated = format!(
        r####"
	(segment
		(start {e0})
		(end {e1})
		(width {e2})
    (locked {e3})
		(layer "F.Cu")
		(net {e4})
	)
	(segment
		(start {e1})
		(end {e5})
		(width {e2})
    (locked {e3})
		(layer "F.Cu")
		(net {e4})
	)
	(via
		(at {e5})
		(size {e6})
    (drill {e7})
		(layers "F.Cu" "B.Cu")
    (locked {e3})
		(net {e4})
	)
	(segment
		(start {e8})
		(end {e5})
		(width {e2})
    (locked {e3})
		(layer "B.Cu")
		(net {e4})
	)
	(segment
		(start {e9})
		(end {e8})
		(width {e2})
    (locked {e3})
		(layer "B.Cu")
		(net {e4})
	)
	(segment
		(start {e10})
		(end {e11})
		(width {e2})
    (locked {e3})
		(layer "F.Cu")
		(net {e12})
	)
	(segment
		(start {e13})
		(end {e14})
		(width {e2})
    (locked {e3})
		(layer "F.Cu")
		(net {e12})
	)
	(segment
		(start {e15})
		(end {e10})
		(width {e2})
    (locked {e3})
		(layer "F.Cu")
		(net {e12})
	)
	(segment
		(start {e11})
		(end {e13})
		(width {e2})
    (locked {e3})
		(layer "F.Cu")
		(net {e12})
	)
	(via
		(at {e14})
		(size {e6})
    (drill {e7})
		(layers "F.Cu" "B.Cu")
    (locked {e3})
		(net {e12})
	)
	(segment
		(start {e16})
		(end {e17})
		(width {e2})
    (locked {e3})
		(layer "B.Cu")
		(net {e12})
	)
	(segment
		(start {e18})
		(end {e16})
		(width {e2})
    (locked {e3})
		(layer "B.Cu")
		(net {e12})
	)
	(segment
		(start {e17})
		(end {e14})
		(width {e2})
    (locked {e3})
		(layer "B.Cu")
		(net {e12})
	)
	(segment
		(start {e19})
		(end {e18})
		(width {e2})
    (locked {e3})
		(layer "B.Cu")
		(net {e12})
	)
    "####,
        e0 = p.eaxy(3.275, -5.95),
        e1 = p.eaxy(1.2, -3.875),
        e2 = n(p.number("trace_width")),
        e3 = if p.flag("locked_traces_vias") {
            "yes".to_string()
        } else {
            "no".to_string()
        },
        e4 = p.net("from").index,
        e5 = p.eaxy(0.0, -3.875),
        e6 = n(p.number("via_size")),
        e7 = n(p.number("via_drill")),
        e8 = p.eaxy(-1.2, -3.875),
        e9 = p.eaxy(-3.275, -5.95),
        e10 = p.eaxy(-6.421, -1.896),
        e11 = p.eaxy(-2.154, -1.896),
        e12 = p.net("to").index,
        e13 = p.eaxy(-0.975, -3.075),
        e14 = p.eaxy(0.0, -3.075),
        e15 = p.eaxy(-8.275, -3.75),
        e16 = p.eaxy(2.140166, -1.896),
        e17 = p.eaxy(0.961166, -3.075),
        e18 = p.eaxy(6.421, -1.896),
        e19 = p.eaxy(8.275, -3.75)
    );

    let hotswap_routes_same_side = format!(
        r####"
  (segment
		(start {e0})
		(end {e1})
		(width {e2})
    (locked {e3})
		(layer "F.Cu")
		(net {e4})
	)
	(via
		(at {e1})
		(size {e5})
    (drill {e6})
		(layers "F.Cu" "B.Cu")
    (locked {e3})
		(net {e4})
	)
	(segment
		(start {e7})
		(end {e1})
		(width {e2})
    (locked {e3})
		(layer "B.Cu")
		(net {e4})
	)
	(segment
		(start {e8})
		(end {e9})
		(width {e2})
    (locked {e3})
		(layer "F.Cu")
		(net {e10})
	)
	(via
		(at {e9})
		(size {e5})
    (drill {e6})
		(layers "F.Cu" "B.Cu")
    (locked {e3})
		(net {e10})
	)
	(segment
		(start {e11})
		(end {e9})
		(width {e2})
    (locked {e3})
		(layer "B.Cu")
		(net {e10})
	)
    "####,
        e0 = p.eaxy(3.275, -5.95),
        e1 = p.eaxy(7.775, -5.95),
        e2 = n(p.number("trace_width")),
        e3 = if p.flag("locked_traces_vias") {
            "yes".to_string()
        } else {
            "no".to_string()
        },
        e4 = p.net("from").index,
        e5 = n(p.number("via_size")),
        e6 = n(p.number("via_drill")),
        e7 = p.eaxy(7.775, -3.75),
        e8 = p.eaxy(-7.775, -3.75),
        e9 = p.eaxy(-7.775, -5.95),
        e10 = p.net("to").index,
        e11 = p.eaxy(-3.275, -5.95)
    );

    let (side, reversible, hotswap) = (p.side(), p.flag("reversible"), p.flag("hotswap"));
    let mut out = common_top;
    if p.flag("choc_v1_support") {
        out += &choc_v1_stabilizers;
    }
    if p.flag("include_corner_marks") {
        out += &corner_marks;
    }
    if p.flag("include_keycap") {
        out += &keycap_marks;
    }
    if p.flag("include_stabilizer_pad") && p.flag("choc_v2_support") {
        // Combined mounting needs both stabilizer positions, even on one side.
        let both_stabilizers = reversible || swapped;
        let oval = p.flag("oval_stabilizer_pad");
        if both_stabilizers || side == "F" {
            out += if oval {
                &oval_corner_stab_front
            } else {
                &round_corner_stab_front
            };
        }
        if both_stabilizers || side == "B" {
            out += if oval {
                &oval_corner_stab_back
            } else {
                &round_corner_stab_back
            };
        }
    }
    if p.flag("include_choc_v1_led_cutout_marks") {
        out += &choc_v1_led_cutout_marks;
    }
    if p.flag("include_choc_v2_led_cutout_marks") {
        out += &choc_v2_led_cutout_marks;
    }
    if hotswap {
        out += &hotswap_common;
        if reversible || side == "F" {
            out += &hotswap_front;
        }
        if reversible || side == "B" {
            out += &hotswap_back;
        }
        if !hotswap_filename.is_empty() {
            out += &hotswap_3dmodel;
        }
    }
    if p.flag("solder") {
        out += &solder_common;
        if reversible || side == "F" {
            out += &solder_front;
        }
        if reversible || side == "B" {
            out += &solder_back;
        }
    }
    if !switch_filename.is_empty() {
        out += &switch_3dmodel;
    }
    if !keycap_filename.is_empty() {
        out += &keycap_3dmodel;
    }
    out += &common_bottom;
    if reversible && hotswap && p.flag("include_traces_vias") && !p.flag("include_plated_holes") {
        out += if p.flag("hotswap_pads_same_side") {
            &hotswap_routes_same_side
        } else {
            &hotswap_routes_unplated
        };
    }
    Ok(out)
}

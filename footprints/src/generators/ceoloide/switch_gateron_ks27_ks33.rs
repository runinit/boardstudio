//! `ceoloide/switch_gateron_ks27_ks33`
//!
//! Ported from `switch_gateron_ks27_ks33.js` of ceoloide/ergogen-footprints.
//! Copyright (c) 2023 Marco Massarelli
//! SPDX-License-Identifier: MIT
//! Authors: @nxtk
use crate::Result;
use crate::context::RenderContext;
use crate::error::GeneratorError;
use crate::generators::util::json_string;
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, KeycapParameters, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 34] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "S"),
    ParamSpec::boolean("reversible", false),
    ParamSpec::boolean("solder", true),
    ParamSpec::boolean("hotswap", true),
    ParamSpec::number("keycap_width", 18.0),
    ParamSpec::number("keycap_height", 18.0),
    ParamSpec::boolean("include_corner_marks", false),
    ParamSpec::boolean("include_centerhole_net", false),
    ParamSpec::boolean("include_keycap", true),
    ParamSpec::boolean("include_stem_outline", false),
    ParamSpec::boolean("include_led_outline", false),
    ParamSpec::boolean("include_socket_silks", false),
    ParamSpec::boolean("include_socket_fabs", false),
    ParamSpec::boolean("include_custom_solder_pads", false),
    ParamSpec::boolean("allow_soldermask_bridges", true),
    ParamSpec::number("outer_pad_width_front", 2.6),
    ParamSpec::number("outer_pad_width_back", 2.6),
    ParamSpec::string(
        "switch_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/gdek/KS33.stp",
    ),
    ParamSpec::number("pcb_thickness", 1.6),
    ParamSpec::array("switch_3dmodel_xyz_offset", "[]"),
    ParamSpec::array("switch_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("switch_3dmodel_xyz_scale", "[1,1,1]"),
    ParamSpec::string("hotswap_3dmodel_filename", ""),
    ParamSpec::array("hotswap_3dmodel_xyz_offset", "[0,0,0]"),
    ParamSpec::array("hotswap_3dmodel_xyz_rotation", "[0,0,0]"),
    ParamSpec::array("hotswap_3dmodel_xyz_scale", "[1,1,1]"),
    ParamSpec::string("keycap_3dmodel_filename", ""),
    ParamSpec::array("keycap_3dmodel_xyz_offset", "[0,0,0]"),
    ParamSpec::array("keycap_3dmodel_xyz_rotation", "[0,0,0]"),
    ParamSpec::array("keycap_3dmodel_xyz_scale", "[1,1,1]"),
    ParamSpec::net("from"),
    ParamSpec::net("to"),
    ParamSpec::net_default("CENTERHOLE", "GND"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "ceoloide/switch_gateron_ks27_ks33",
    display_name: "switch gateron ks27 ks33",
    kind: PartKind::Switch,
    matrix_terminals: Some(("from", "to")),
    keycap_parameters: Some(KeycapParameters {
        width: "keycap_width",
        height: "keycap_height",
    }),
    parameters: &PARAMS,
    license: License {
        spdx: "MIT",
        author: "@nxtk",
    },
    body,
};

fn body(p: &RenderContext<'_>) -> Result<String> {
    if p.flag("hotswap") && p.flag("reversible") {
        return Err(GeneratorError::rejected(
            SPEC.source,
            Some("reversible"),
            "switch_gateron_ks27_ks33: reversible hotswap has overlapping 3 mm drills; use reversible: false or hotswap: false with solder: true",
        ));
    }
    for name in [
        "switch_3dmodel_xyz_scale",
        "hotswap_3dmodel_xyz_offset",
        "hotswap_3dmodel_xyz_scale",
        "hotswap_3dmodel_xyz_rotation",
        "keycap_3dmodel_xyz_offset",
        "keycap_3dmodel_xyz_scale",
        "keycap_3dmodel_xyz_rotation",
    ] {
        p.vec3(name)?;
    }
    let keycap_xo = 0.5 * p.number("keycap_width");
    let keycap_yo = 0.5 * p.number("keycap_height");
    let hotswap = p.flag("hotswap");
    // Combined hotswap and solder moves the solder holes to the other side.
    let solder_offset_y = if hotswap && p.flag("solder") { "-" } else { "" };

    // KS-33 feet define the mounting plane; hotswap mounts opposite the footprint.
    let model_center_x = 60.0;
    let model_mount_z = 3.25;
    let model_back = p.side() == "B";
    let model_rotation = p
        .vec3("switch_3dmodel_xyz_rotation")?
        .unwrap_or(if hotswap {
            if model_back {
                [180.0, 0.0, 0.0]
            } else {
                [0.0, 180.0, 0.0]
            }
        } else if model_back {
            [0.0, 0.0, 180.0]
        } else {
            [0.0, 0.0, 0.0]
        });
    let model_offset = p.vec3("switch_3dmodel_xyz_offset")?.unwrap_or([
        if model_back == hotswap {
            -model_center_x
        } else {
            model_center_x
        },
        0.0,
        if hotswap {
            model_mount_z - p.number("pcb_thickness")
        } else {
            -model_mount_z
        },
    ]);

    let common_top = format!(
        r####"
  (footprint "ceoloide:switch_gateron_ks27_ks33"
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
    (pad "" thru_hole circle (at 0 0 {e3}) (size 5.6 5.6) (drill 5.1) (layers "*.Cu" "*.Mask") {e7})
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
        e7 = if p.flag("include_centerhole_net") {
            p.net("CENTERHOLE").to_string()
        } else {
            "".to_string()
        }
    );

    let corner_marks = format!(
        r####"
    {e0}
    (fp_line (start -7 -6) (end -7 -7) (stroke (width 0.15) (type solid)) (layer "Dwgs.User"))
    (fp_line (start -7 7) (end -7 6) (stroke (width 0.15) (type solid)) (layer "Dwgs.User"))
    (fp_line (start -7 7) (end -6 7) (stroke (width 0.15) (type solid)) (layer "Dwgs.User"))
    (fp_line (start -6 -7) (end -7 -7) (stroke (width 0.15) (type solid)) (layer "Dwgs.User"))
    (fp_line (start 6 7) (end 7 7) (stroke (width 0.15) (type solid)) (layer "Dwgs.User"))
    (fp_line (start 7 -7) (end 6 -7) (stroke (width 0.15) (type solid)) (layer "Dwgs.User"))
    (fp_line (start 7 -7) (end 7 -6) (stroke (width 0.15) (type solid)) (layer "Dwgs.User"))
    (fp_line (start 7 6) (end 7 7) (stroke (width 0.15) (type solid)) (layer "Dwgs.User"))
    "####,
        e0 = ""
    );

    let led_outline = format!(
        r####"
    {e0}
    {e1}
    "####,
        e0 = "",
        e1 = if p.side() == "B" {
            "\r\n    (fp_rect (start -3.2 -6.3) (end 1.8 -4.05) (stroke (width 0.15) (type solid)) (fill none) (layer \"Dwgs.User\"))\r\n    ".to_string()
        } else {
            "\r\n    (fp_rect (start -1.8 -6.3) (end 3.2 -4.05) (stroke (width 0.15) (type solid)) (fill none) (layer \"Dwgs.User\"))\r\n    ".to_string()
        }
    );

    let stem_outline = format!(
        r####"
    {e0}
    (fp_poly (pts (xy -0.525791 -3.207186) (xy -0.869467 -3.131537) (xy -1.202949 -3.019174) (xy -1.522327 -2.871414) (xy -1.823858 -2.689989) (xy -2.104005 -2.477027) (xy -2.359485 -2.235023) (xy -2.389234 -2.2) (xy -4.7 -2.2) (xy -4.7 2.2) (xy -2.389234 2.2) (xy -2.359485 2.235023) (xy -2.104005 2.477027) (xy -1.823858 2.689989) (xy -1.522327 2.871414) (xy -1.202949 3.019174) (xy -0.869467 3.131537) (xy -0.525791 3.207186) (xy -0.175951 3.245234) (xy 0 3.245234) (xy 0 2.845178) (xy -0.165713 2.845178) (xy -0.494897 2.806702) (xy -0.817389 2.73027) (xy -1.128827 2.616916) (xy -1.425 2.468172) (xy -1.701902 2.286051) (xy -1.955789 2.073015) (xy -2.183227 1.831945) (xy -2.38114 1.566101) (xy -2.546853 1.279078) (xy -2.678124 0.974757) (xy -2.773178 0.657255) (xy -2.830729 0.330865) (xy -2.85 0) (xy -2.830729 -0.330865) (xy -2.773178 -0.657255) (xy -2.678124 -0.974757) (xy -2.546853 -1.279078) (xy -2.38114 -1.566101) (xy -2.183227 -1.831945) (xy -1.955789 -2.073015) (xy -1.701902 -2.286051) (xy -1.425 -2.468172) (xy -1.128827 -2.616916) (xy -0.817389 -2.73027) (xy -0.494897 -2.806702) (xy -0.165713 -2.845178) (xy 0 -2.845178) (xy 0 -3.245234) (xy -0.175951 -3.245234)) (stroke (width 0.001) (type solid)) (fill solid) (layer "Dwgs.User"))
    (fp_poly (pts (xy 0.525791 -3.207186) (xy 0.869467 -3.131537) (xy 1.202949 -3.019174) (xy 1.522327 -2.871414) (xy 1.823858 -2.689989) (xy 2.104005 -2.477027) (xy 2.359485 -2.235023) (xy 2.389234 -2.2) (xy 4.7 -2.2) (xy 4.7 2.2) (xy 2.389234 2.2) (xy 2.359485 2.235023) (xy 2.104005 2.477027) (xy 1.823858 2.689989) (xy 1.522327 2.871414) (xy 1.202949 3.019174) (xy 0.869467 3.131537) (xy 0.525791 3.207186) (xy 0.175951 3.245234) (xy 0 3.245234) (xy 0 2.845178) (xy 0.165713 2.845178) (xy 0.494897 2.806702) (xy 0.817389 2.73027) (xy 1.128827 2.616916) (xy 1.425 2.468172) (xy 1.701902 2.286051) (xy 1.955789 2.073015) (xy 2.183227 1.831945) (xy 2.38114 1.566101) (xy 2.546853 1.279078) (xy 2.678124 0.974757) (xy 2.773178 0.657255) (xy 2.830729 0.330865) (xy 2.85 0) (xy 2.830729 -0.330865) (xy 2.773178 -0.657255) (xy 2.678124 -0.974757) (xy 2.546853 -1.279078) (xy 2.38114 -1.566101) (xy 2.183227 -1.831945) (xy 1.955789 -2.073015) (xy 1.701902 -2.286051) (xy 1.425 -2.468172) (xy 1.128827 -2.616916) (xy 0.817389 -2.73027) (xy 0.494897 -2.806702) (xy 0.165713 -2.845178) (xy 0 -2.845178) (xy 0 -3.245234) (xy 0.175951 -3.245234)) (stroke (width 0.001) (type solid)) (fill solid) (layer "Dwgs.User"))
    "####,
        e0 = ""
    );

    let stem_cross_outline = format!(
        r####"
    {e0}
    (fp_poly (pts (xy -0.55 -0.55) (xy -0.55 -2) (xy 0.55 -2) (xy 0.55 -0.55) (xy 2 -0.55) (xy 2 0.55) (xy 0.55 0.55) (xy 0.55 2) (xy -0.55 2) (xy -0.55 0.55) (xy -2 0.55) (xy -2 -0.55)) (stroke (width 0) (type solid)) (fill solid) (layer "Dwgs.User"))
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

    let hotswap_fab_front = String::from(
        r####"
    (fp_line (start -6.65 6.525) (end -6.65 4.975) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_line (start -6.55 4.875) (end -5.025 4.875) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_line (start -5.025 4.875) (end -5.025 3.675) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_line (start -5.025 4.875) (end -5.025 6.625) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_line (start -5.025 6.625) (end -6.55 6.625) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_line (start -5.025 6.625) (end -5.025 7.825) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_line (start -4.925 3.575) (end 0.788397 3.575) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_line (start -4.925 7.925) (end -0.775 7.925) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_line (start -0.675 7.825) (end -0.675 7.325) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_line (start -0.475 7.125) (end 0.625 7.125) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_line (start 0.825 7.325) (end 0.825 7.825) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_line (start 0.925 7.925) (end 1.022371 7.925) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_line (start 2.642949 2.658975) (end 1.288397 3.441026) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_line (start 2.742949 7.008975) (end 1.272371 7.858013) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_line (start 6.725 2.525) (end 3.142949 2.525) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_line (start 6.725 6.875) (end 3.242949 6.875) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_line (start 6.825 3.825) (end 6.825 2.625) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_line (start 6.825 3.825) (end 8.35 3.825) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_line (start 6.825 5.575) (end 6.825 3.825) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_line (start 6.825 6.775) (end 6.825 5.575) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_line (start 8.35 5.575) (end 6.825 5.575) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_line (start 8.45 3.925) (end 8.45 5.475) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_arc (start -6.65 4.975) (mid -6.620711 4.904289) (end -6.55 4.875) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_arc (start -6.55 6.625) (mid -6.620711 6.595711) (end -6.65 6.525) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_arc (start -5.025 3.675) (mid -4.995711 3.604289) (end -4.925 3.575) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_arc (start -4.925 7.925) (mid -4.995711 7.895711) (end -5.025 7.825) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_arc (start -0.675 7.325) (mid -0.616421 7.183579) (end -0.475 7.125) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_arc (start -0.675 7.825) (mid -0.704289 7.895711) (end -0.775 7.925) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_arc (start 0.625 7.125) (mid 0.76642 7.183579) (end 0.825 7.325) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_arc (start 0.925 7.925) (mid 0.854288 7.895711) (end 0.825 7.825) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_arc (start 1.272371 7.858013) (mid 1.151778 7.907947) (end 1.022371 7.925) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_arc (start 1.288397 3.441026) (mid 1.047216 3.540926) (end 0.788397 3.575) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_arc (start 2.642949 2.658975) (mid 2.884134 2.559088) (end 3.142949 2.525) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_arc (start 2.742949 7.008975) (mid 2.984134 6.909088) (end 3.242949 6.875) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_arc (start 6.725 2.525) (mid 6.795709 2.55429) (end 6.825 2.625) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_arc (start 6.825 6.775) (mid 6.795711 6.845711) (end 6.725 6.875) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_arc (start 8.35 3.825) (mid 8.420709 3.85429) (end 8.45 3.925) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_arc (start 8.45 5.475) (mid 8.420711 5.545711) (end 8.35 5.575) (stroke (width 0.001) (type solid)) (layer "F.Fab"))
    (fp_circle (center -2.6 5.75) (end -1.1 5.75) (stroke (width 0.001) (type solid)) (fill none) (layer "F.Fab"))
    (fp_circle (center 4.4 4.7) (end 5.9 4.7) (stroke (width 0.001) (type solid)) (fill none) (layer "F.Fab"))
    "####,
    );

    let hotswap_fab_back = String::from(
        r####"
    (fp_line (start -8.45 5.475) (end -8.45 3.925) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_line (start -8.35 3.825) (end -6.825 3.825) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_line (start -6.825 2.625) (end -6.825 3.825) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_line (start -6.825 3.825) (end -6.825 5.575) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_line (start -6.825 5.575) (end -8.35 5.575) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_line (start -6.825 5.575) (end -6.825 6.775) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_line (start -3.242949 6.875) (end -6.725 6.875) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_line (start -3.142949 2.525) (end -6.725 2.525) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_line (start -1.288397 3.441026) (end -2.642949 2.658975) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_line (start -1.272371 7.858013) (end -2.742949 7.008975) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_line (start -1.022371 7.925) (end -0.925 7.925) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_line (start -0.825 7.825) (end -0.825 7.325) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_line (start -0.788397 3.575) (end 4.925 3.575) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_line (start -0.625 7.125) (end 0.475 7.125) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_line (start 0.675 7.325) (end 0.675 7.825) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_line (start 0.775 7.925) (end 4.925 7.925) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_line (start 5.025 3.675) (end 5.025 4.875) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_line (start 5.025 4.875) (end 6.55 4.875) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_line (start 5.025 6.625) (end 5.025 4.875) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_line (start 5.025 7.825) (end 5.025 6.625) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_line (start 6.55 6.625) (end 5.025 6.625) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_line (start 6.65 4.975) (end 6.65 6.525) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_arc (start -8.45 3.925) (mid -8.420711 3.854289) (end -8.35 3.825) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_arc (start -8.35 5.575) (mid -8.420711 5.545711) (end -8.45 5.475) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_arc (start -6.825 2.625) (mid -6.795711 2.554289) (end -6.725 2.525) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_arc (start -6.725 6.875) (mid -6.795711 6.845711) (end -6.825 6.775) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_arc (start -3.242949 6.875) (mid -2.98413 6.909077) (end -2.742949 7.008975) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_arc (start -3.142949 2.525) (mid -2.88413 2.559077) (end -2.642949 2.658975) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_arc (start -1.022371 7.925) (mid -1.15178 7.907962) (end -1.272371 7.858013) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_arc (start -0.825 7.325) (mid -0.766421 7.183579) (end -0.625 7.125) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_arc (start -0.825 7.825) (mid -0.854289 7.895711) (end -0.925 7.925) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_arc (start -0.788397 3.575) (mid -1.047216 3.540927) (end -1.288397 3.441026) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_arc (start 0.475 7.125) (mid 0.616421 7.183579) (end 0.675 7.325) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_arc (start 0.775 7.925) (mid 0.704289 7.895711) (end 0.675 7.825) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_arc (start 4.925 3.575) (mid 4.995711 3.604289) (end 5.025 3.675) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_arc (start 5.025 7.825) (mid 4.995711 7.895711) (end 4.925 7.925) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_arc (start 6.55 4.875) (mid 6.620711 4.904289) (end 6.65 4.975) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_arc (start 6.65 6.525) (mid 6.620711 6.595711) (end 6.55 6.625) (stroke (width 0.001) (type solid)) (layer "B.Fab"))
    (fp_circle (center -4.4 4.7) (end -2.85 4.7) (stroke (width 0.001) (type solid)) (fill none) (layer "B.Fab"))
    (fp_circle (center 2.6 5.75) (end 4.15 5.75) (stroke (width 0.001) (type solid)) (fill none) (layer "B.Fab"))
    "####,
    );

    let hotswap_silk_front = format!(
        r####"
    (fp_line (start -5.025 7.825) (end -5.025 7.225) (stroke (width 0.15) (type solid)) (layer "F.SilkS"))
    (fp_line (start -4.325 7.925) (end -4.925 7.925) (stroke (width 0.15) (type solid)) (layer "F.SilkS"))
    (fp_line (start 0.788397 3.575) (end -0.75 3.575) (stroke (width 0.15) (type solid)) (layer "F.SilkS"))
    (fp_line (start 2.642949 2.658975) (end 1.288397 3.441026) (stroke (width 0.15) (type solid)) (layer "F.SilkS"))
    (fp_line (start 4.681346 2.525) (end 3.142949 2.525) (stroke (width 0.15) (type solid)) (layer "F.SilkS"))
    (fp_line (start 6.725 2.525) (end 6.125 2.525) (stroke (width 0.15) (type solid)) (layer "F.SilkS"))
    (fp_line (start 6.725 6.875) (end 6.125 6.875) (stroke (width 0.15) (type solid)) (layer "F.SilkS"))
    (fp_line (start 6.825 3.225) (end 6.825 2.625) (stroke (width 0.15) (type solid)) (layer "F.SilkS"))
    (fp_line (start 6.825 6.775) (end 6.825 6.175) (stroke (width 0.15) (type solid)) (layer "F.SilkS"))
    (fp_arc (start -4.925 7.925) (mid -4.995711 7.895711) (end -5.025 7.825) (stroke (width 0.15) (type solid)) (layer "F.SilkS"))
    (fp_arc (start 1.288397 3.441026) (mid 1.047216 3.540926) (end 0.788397 3.575) (stroke (width 0.15) (type solid)) (layer "F.SilkS"))
    (fp_arc (start 2.642949 2.658975) (mid 2.884131 2.559086) (end 3.142949 2.525) (stroke (width 0.15) (type solid)) (layer "F.SilkS"))
    (fp_arc (start 6.725 2.525) (mid 6.795711 2.554289) (end 6.825 2.625) (stroke (width 0.15) (type solid)) (layer "F.SilkS"))
    (fp_arc (start 6.825 6.775) (mid 6.795711 6.845711) (end 6.725 6.875) (stroke (width 0.15) (type solid)) (layer "F.SilkS"))
    
    {e0}
    "####,
        e0 = if p.flag("reversible") {
            "".to_string()
        } else {
            "\r\n    (fp_arc (start -5.025 3.675) (mid -4.995711 3.604289) (end -4.925 3.575) (stroke (width 0.15) (type solid)) (layer \"F.SilkS\"))\r\n    (fp_line (start -5.025 4.275) (end -5.025 3.675) (stroke (width 0.15) (type solid)) (layer \"F.SilkS\"))\r\n    (fp_line (start -4.325 3.575) (end -4.925 3.575) (stroke (width 0.15) (type solid)) (layer \"F.SilkS\"))\r\n    ".to_string()
        }
    );

    let hotswap_silk_back = format!(
        r####"
    (fp_line (start -6.825 2.625) (end -6.825 3.225) (stroke (width 0.15) (type solid)) (layer "B.SilkS"))
    (fp_line (start -6.825 6.175) (end -6.825 6.775) (stroke (width 0.15) (type solid)) (layer "B.SilkS"))
    (fp_line (start -6.125 2.525) (end -6.725 2.525) (stroke (width 0.15) (type solid)) (layer "B.SilkS"))
    (fp_line (start -6.125 6.875) (end -6.725 6.875) (stroke (width 0.15) (type solid)) (layer "B.SilkS"))
    (fp_line (start -3.142949 2.525) (end -4.681346 2.525) (stroke (width 0.15) (type solid)) (layer "B.SilkS"))
    (fp_line (start -1.288397 3.441026) (end -2.642949 2.658975) (stroke (width 0.15) (type solid)) (layer "B.SilkS"))
    (fp_line (start 0.75 3.575) (end -0.788397 3.575) (stroke (width 0.15) (type solid)) (layer "B.SilkS"))
    (fp_line (start 4.925 7.925) (end 4.325 7.925) (stroke (width 0.15) (type solid)) (layer "B.SilkS"))
    (fp_line (start 5.025 7.225) (end 5.025 7.825) (stroke (width 0.15) (type solid)) (layer "B.SilkS"))
    (fp_arc (start -6.825 2.625) (mid -6.795711 2.554289) (end -6.725 2.525) (stroke (width 0.15) (type solid)) (layer "B.SilkS"))
    (fp_arc (start -6.725 6.875) (mid -6.795711 6.845711) (end -6.825 6.775) (stroke (width 0.15) (type solid)) (layer "B.SilkS"))
    (fp_arc (start -3.142949 2.525) (mid -2.884135 2.559092) (end -2.642949 2.658975) (stroke (width 0.15) (type solid)) (layer "B.SilkS"))
    (fp_arc (start -0.788397 3.575) (mid -1.047216 3.540926) (end -1.288397 3.441026) (stroke (width 0.15) (type solid)) (layer "B.SilkS"))
    (fp_arc (start 5.025 7.825) (mid 4.995711 7.895711) (end 4.925 7.925) (stroke (width 0.15) (type solid)) (layer "B.SilkS"))
    
    {e0}
    "####,
        e0 = if p.flag("reversible") {
            "".to_string()
        } else {
            "\r\n    (fp_arc (start 4.925 3.575) (mid 4.995711 3.604289) (end 5.025 3.675) (stroke (width 0.15) (type solid)) (layer \"B.SilkS\"))\r\n    (fp_line (start 5.025 3.675) (end 5.025 4.275) (stroke (width 0.15) (type solid)) (layer \"B.SilkS\"))\r\n    (fp_line (start 4.925 3.575) (end 4.325 3.575) (stroke (width 0.15) (type solid)) (layer \"B.SilkS\"))\r\n    ".to_string()
        }
    );

    let hotswap_front_full = format!(
        r####"
    (pad "1" smd roundrect (at -5.55 5.75 {e0}) (size 4 2.5) (layers "F.Cu") (roundrect_rratio 0.1) {e1})
    (pad "" smd roundrect (at -6.25 5.75 {e0}) (size 2.6 2.5) (layers "F.Paste" "F.Mask") (roundrect_rratio 0.1))
    "####,
        e0 = n(p.rotation()),
        e1 = p.net("from")
    );

    let hotswap_back_full = format!(
        r####"
    (pad "2" smd roundrect (at 5.55 5.75 {e0}) (size 4 2.5) (layers "B.Cu") (roundrect_rratio 0.1) {e1})
    (pad "" smd roundrect (at 6.25 5.75 {e0}) (size 2.6 2.5) (layers "B.Paste" "B.Mask") (roundrect_rratio 0.1))
    "####,
        e0 = n(p.rotation()),
        e1 = p.net("to")
    );

    let hotswap_front = format!(
        r####"
    (pad {e0} thru_hole circle (at -2.6 5.75 {e1}) (size 3.5 3.5) (drill 3) (layers "*.Cu" "*.Mask") {e2})
    (pad "2" thru_hole circle (at 4.4 4.7 {e1}) (size 3.5 3.5) (drill 3) (layers "*.Cu" "*.Mask") {e3})

    (pad "2" smd roundrect (at {e4} 4.7 {e1}) (size {e5} 2.5) (layers "F.Cu") (roundrect_rratio 0.1) {e3})
    (pad "" smd roundrect (at {e6} 4.7 {e1}) (size {e7} 2.5) (layers "F.Paste" "F.Mask") (roundrect_rratio 0.1))

    {e8}
    "####,
        e0 = if p.flag("reversible") {
            "\"\"".to_string()
        } else {
            "\"1\"".to_string()
        },
        e1 = n(p.rotation()),
        e2 = p.net("from"),
        e3 = p.net("to"),
        e4 = n(7.35 - (2.6 - p.number("outer_pad_width_front")) / 2.0),
        e5 = n(p.number("outer_pad_width_front") + 1.4),
        e6 = n(8.05 - (2.6 - p.number("outer_pad_width_front")) / 2.0),
        e7 = n(p.number("outer_pad_width_front")),
        e8 = if !(p.flag("reversible")) {
            hotswap_front_full.clone()
        } else {
            format!(
                r####"(pad "1" smd roundrect (at -6.25 5.75 {e0}) (size 2.6 2.5) (layers "F.Cu" "F.Paste" "F.Mask") (roundrect_rratio 0.1) {e1})"####,
                e0 = n(p.rotation()),
                e1 = p.net("from")
            )
        }
    );

    let hotswap_back = format!(
        r####"
    (pad "1" thru_hole circle (at -4.4 4.7 {e0}) (size 3.5 3.5) (drill 3) (layers "*.Cu" "*.Mask") {e1})
    (pad {e2} thru_hole circle (at 2.6 5.75 {e0}) (size 3.5 3.5) (drill 3) (layers "*.Cu" "*.Mask") {e3})

    
    (pad "1" smd roundrect (at {e4} 4.7 {e0}) (size {e5} 2.5) (layers "B.Cu") (roundrect_rratio 0.1) {e1})
    (pad "" smd roundrect (at {e6} 4.7 {e0}) (size {e7} 2.5) (layers "B.Paste" "B.Mask") (roundrect_rratio {e8}))
    
    {e9}
    "####,
        e0 = n(p.rotation()),
        e1 = p.net("from"),
        e2 = if p.flag("reversible") {
            "\"\"".to_string()
        } else {
            "\"2\"".to_string()
        },
        e3 = p.net("to"),
        e4 = n(-7.35 + (2.6 - p.number("outer_pad_width_back")) / 2.0),
        e5 = n(p.number("outer_pad_width_back") + 1.4),
        e6 = n(-8.05 + (2.6 - p.number("outer_pad_width_back")) / 2.0),
        e7 = n(p.number("outer_pad_width_back")),
        e8 = if (2.5 / p.number("outer_pad_width_back")) <= (1.0) {
            n(0.1)
        } else {
            n(0.1 * (2.5 / p.number("outer_pad_width_back")))
        },
        e9 = if !(p.flag("reversible")) {
            hotswap_back_full.clone()
        } else {
            format!(
                r####"(pad "2" smd roundrect (at 6.25 5.75 {e0}) (size 2.6 2.5) (layers "B.Cu" "B.Paste" "B.Mask") (roundrect_rratio 0.1) {e1})"####,
                e0 = n(p.rotation()),
                e1 = p.net("to")
            )
        }
    );

    let solder_back = format!(
        r####"
    (pad "1" thru_hole circle (at -2.6 {e0}5.75 {e1}) (size 2.1 2.1) (drill 1.25) (layers "*.Cu" "*.Mask") {e2})
    (pad "2" thru_hole circle (at 4.4 {e0}4.7 {e1}) (size 2.1 2.1) (drill 1.25) (layers "*.Cu" "*.Mask") {e3})
    "####,
        e0 = solder_offset_y,
        e1 = n(p.rotation()),
        e2 = p.net("from"),
        e3 = p.net("to")
    );

    let solder_front = format!(
        r####"
    (pad "1" thru_hole circle (at -4.4 {e0}4.7 {e1}) (size 2.1 2.1) (drill 1.25) (layers "*.Cu" "*.Mask") {e2})
    (pad "2" thru_hole circle (at 2.6 {e0}5.75 {e1}) (size 2.1 2.1) (drill 1.25) (layers "*.Cu" "*.Mask") {e3})
    "####,
        e0 = solder_offset_y,
        e1 = n(p.rotation()),
        e2 = p.net("from"),
        e3 = p.net("to")
    );

    let solder_custom_reversible_top = format!(
        r####"
    (pad "" thru_hole circle (at -4.4 -4.7) (size 1.8 1.8) (drill 1.25) (layers "*.Cu" "*.Mask") {e0})
    (pad "" thru_hole circle (at -2.6 -5.75) (size 1.8 1.8) (drill 1.25) (layers "*.Cu" "*.Mask") {e0})
    (pad "1" smd custom (at -2.6 -5.75 {e1}) (size 1 1) (layers "F.Cu") (thermal_bridge_angle 90) (options (clearance outline) (anchor circle))
      (primitives (gr_poly (pts (xy -0.19509 -0.980785) (xy -0.382683 -0.92388) (xy -0.55557 -0.83147) (xy -2.35557 0.21853) (xy -2.507107 0.342893) (xy -2.63147 0.49443) (xy -2.72388 0.667317) (xy -2.780785 0.85491) (xy -2.8 1.05) (xy -2.780785 1.24509) (xy -2.72388 1.432683) (xy -2.63147 1.60557) (xy -2.507107 1.757107) (xy -2.35557 1.88147) (xy -2.182683 1.97388) (xy -1.99509 2.030785) (xy -1.8 2.05) (xy -1.60491 2.030785) (xy -1.417317 1.97388) (xy -1.24443 1.88147) (xy 0.55557 0.83147) (xy 0.707107 0.707107) (xy 0.83147 0.55557) (xy 0.92388 0.382683) (xy 0.980785 0.19509) (xy 1 0) (xy 0.980785 -0.19509) (xy 0.92388 -0.382683) (xy 0.83147 -0.55557) (xy 0.707107 -0.707107) (xy 0.55557 -0.83147) (xy 0.382683 -0.92388) (xy 0.19509 -0.980785) (xy 0 -1)) (width 0.1) (fill yes)))
      {e0})
    (pad "1" smd custom (at -2.6 -5.75 {e1}) (size 1 1) (layers "B.Cu") (thermal_bridge_angle 90) (options (clearance outline) (anchor circle))
      (primitives (gr_poly (pts (xy -0.19509 -0.980785) (xy -0.382683 -0.92388) (xy -0.55557 -0.83147) (xy -2.35557 0.21853) (xy -2.507107 0.342893) (xy -2.63147 0.49443) (xy -2.72388 0.667317) (xy -2.780785 0.85491) (xy -2.8 1.05) (xy -2.780785 1.24509) (xy -2.72388 1.432683) (xy -2.63147 1.60557) (xy -2.507107 1.757107) (xy -2.35557 1.88147) (xy -2.182683 1.97388) (xy -1.99509 2.030785) (xy -1.8 2.05) (xy -1.60491 2.030785) (xy -1.417317 1.97388) (xy -1.24443 1.88147) (xy 0.55557 0.83147) (xy 0.707107 0.707107) (xy 0.83147 0.55557) (xy 0.92388 0.382683) (xy 0.980785 0.19509) (xy 1 0) (xy 0.980785 -0.19509) (xy 0.92388 -0.382683) (xy 0.83147 -0.55557) (xy 0.707107 -0.707107) (xy 0.55557 -0.83147) (xy 0.382683 -0.92388) (xy 0.19509 -0.980785) (xy 0 -1)) (width 0.1) (fill yes)))
      {e0})

    (pad "" thru_hole circle (at 2.6 -5.75) (size 1.8 1.8) (drill 1.25) (layers "*.Cu" "*.Mask") {e2})
    (pad "" thru_hole circle (at 4.4 -4.7) (size 1.8 1.8) (drill 1.25) (layers "*.Cu" "*.Mask") {e2})
    (pad "2" smd custom (at 2.6 -5.75 {e1}) (size 1 1) (layers "F.Cu") (thermal_bridge_angle 90) (options (clearance outline) (anchor circle))
      (primitives (gr_poly (pts (xy 0.19509 -0.980785) (xy 0.382683 -0.92388) (xy 0.55557 -0.83147) (xy 2.35557 0.21853) (xy 2.507107 0.342893) (xy 2.63147 0.49443) (xy 2.72388 0.667317) (xy 2.780785 0.85491) (xy 2.8 1.05) (xy 2.780785 1.24509) (xy 2.72388 1.432683) (xy 2.63147 1.60557) (xy 2.507107 1.757107) (xy 2.35557 1.88147) (xy 2.182683 1.97388) (xy 1.99509 2.030785) (xy 1.8 2.05) (xy 1.60491 2.030785) (xy 1.417317 1.97388) (xy 1.24443 1.88147) (xy -0.55557 0.83147) (xy -0.707107 0.707107) (xy -0.83147 0.55557) (xy -0.92388 0.382683) (xy -0.980785 0.19509) (xy -1 0) (xy -0.980785 -0.19509) (xy -0.92388 -0.382683) (xy -0.83147 -0.55557) (xy -0.707107 -0.707107) (xy -0.55557 -0.83147) (xy -0.382683 -0.92388) (xy -0.19509 -0.980785) (xy 0 -1)) (width 0.1) (fill yes)))
      {e2})
    (pad "2" smd custom (at 2.6 -5.75 {e1}) (size 1 1) (layers "B.Cu") (thermal_bridge_angle 90) (options (clearance outline) (anchor circle))
      (primitives (gr_poly (pts (xy 0.19509 -0.980785) (xy 0.382683 -0.92388) (xy 0.55557 -0.83147) (xy 2.35557 0.21853) (xy 2.507107 0.342893) (xy 2.63147 0.49443) (xy 2.72388 0.667317) (xy 2.780785 0.85491) (xy 2.8 1.05) (xy 2.780785 1.24509) (xy 2.72388 1.432683) (xy 2.63147 1.60557) (xy 2.507107 1.757107) (xy 2.35557 1.88147) (xy 2.182683 1.97388) (xy 1.99509 2.030785) (xy 1.8 2.05) (xy 1.60491 2.030785) (xy 1.417317 1.97388) (xy 1.24443 1.88147) (xy -0.55557 0.83147) (xy -0.707107 0.707107) (xy -0.83147 0.55557) (xy -0.92388 0.382683) (xy -0.980785 0.19509) (xy -1 0) (xy -0.980785 -0.19509) (xy -0.92388 -0.382683) (xy -0.83147 -0.55557) (xy -0.707107 -0.707107) (xy -0.55557 -0.83147) (xy -0.382683 -0.92388) (xy -0.19509 -0.980785) (xy 0 -1)) (width 0.1) (fill yes)))
      {e2})

    "####,
        e0 = p.net("from"),
        e1 = n(p.rotation()),
        e2 = p.net("to")
    );

    let solder_custom_reversible_bottom = format!(
        r####"
    (pad "" thru_hole circle (at -4.4 4.7) (size 1.8 1.8) (drill 1.25) (layers "*.Cu" "*.Mask") {e0})
    (pad "" thru_hole circle (at -2.6 5.75) (size 1.8 1.8) (drill 1.25) (layers "*.Cu" "*.Mask") {e0})
    (pad "1" smd custom (at -4.4 4.7 {e1}) (size 1 1) (layers "F.Cu") (thermal_bridge_angle 90) (options (clearance outline) (anchor circle)) 
      (primitives (gr_poly (pts (xy 0.19509 -0.980785) (xy 0.382683 -0.92388) (xy 0.55557 -0.83147) (xy 2.35557 0.21853) (xy 2.507107 0.342893) (xy 2.63147 0.49443) (xy 2.72388 0.667317) (xy 2.780785 0.85491) (xy 2.8 1.05) (xy 2.780785 1.24509) (xy 2.72388 1.432683) (xy 2.63147 1.60557) (xy 2.507107 1.757107) (xy 2.35557 1.88147) (xy 2.182683 1.97388) (xy 1.99509 2.030785) (xy 1.8 2.05) (xy 1.60491 2.030785) (xy 1.417317 1.97388) (xy 1.24443 1.88147) (xy -0.55557 0.83147) (xy -0.707107 0.707107) (xy -0.83147 0.55557) (xy -0.92388 0.382683) (xy -0.980785 0.19509) (xy -1 0) (xy -0.980785 -0.19509) (xy -0.92388 -0.382683) (xy -0.83147 -0.55557) (xy -0.707107 -0.707107) (xy -0.55557 -0.83147) (xy -0.382683 -0.92388) (xy -0.19509 -0.980785) (xy 0 -1)) (width 0.1) (fill yes)))
       {e0})
    (pad "1" smd custom (at -4.4 4.7 {e1}) (size 1 1) (layers "B.Cu") (thermal_bridge_angle 90) (options (clearance outline) (anchor circle)) 
      (primitives (gr_poly (pts (xy 0.19509 -0.980785) (xy 0.382683 -0.92388) (xy 0.55557 -0.83147) (xy 2.35557 0.21853) (xy 2.507107 0.342893) (xy 2.63147 0.49443) (xy 2.72388 0.667317) (xy 2.780785 0.85491) (xy 2.8 1.05) (xy 2.780785 1.24509) (xy 2.72388 1.432683) (xy 2.63147 1.60557) (xy 2.507107 1.757107) (xy 2.35557 1.88147) (xy 2.182683 1.97388) (xy 1.99509 2.030785) (xy 1.8 2.05) (xy 1.60491 2.030785) (xy 1.417317 1.97388) (xy 1.24443 1.88147) (xy -0.55557 0.83147) (xy -0.707107 0.707107) (xy -0.83147 0.55557) (xy -0.92388 0.382683) (xy -0.980785 0.19509) (xy -1 0) (xy -0.980785 -0.19509) (xy -0.92388 -0.382683) (xy -0.83147 -0.55557) (xy -0.707107 -0.707107) (xy -0.55557 -0.83147) (xy -0.382683 -0.92388) (xy -0.19509 -0.980785) (xy 0 -1)) (width 0.1) (fill yes)))
       {e0})

    (pad "" thru_hole circle (at 2.6 5.75) (size 1.8 1.8) (drill 1.25) (layers "*.Cu" "*.Mask") {e2})
    (pad "" thru_hole circle (at 4.4 4.7) (size 1.8 1.8) (drill 1.25) (layers "*.Cu" "*.Mask") {e2})
    (pad "2" smd custom (at 4.4 4.7 {e1}) (size 1 1) (layers "F.Cu") (thermal_bridge_angle 90) (options (clearance outline) (anchor circle))
      (primitives (gr_poly (pts (xy -0.19509 -0.980785) (xy -0.382683 -0.92388) (xy -0.55557 -0.83147) (xy -2.35557 0.21853) (xy -2.507107 0.342893) (xy -2.63147 0.49443) (xy -2.72388 0.667317) (xy -2.780785 0.85491) (xy -2.8 1.05) (xy -2.780785 1.24509) (xy -2.72388 1.432683) (xy -2.63147 1.60557) (xy -2.507107 1.757107) (xy -2.35557 1.88147) (xy -2.182683 1.97388) (xy -1.99509 2.030785) (xy -1.8 2.05) (xy -1.60491 2.030785) (xy -1.417317 1.97388) (xy -1.24443 1.88147) (xy 0.55557 0.83147) (xy 0.707107 0.707107) (xy 0.83147 0.55557) (xy 0.92388 0.382683) (xy 0.980785 0.19509) (xy 1 0) (xy 0.980785 -0.19509) (xy 0.92388 -0.382683) (xy 0.83147 -0.55557) (xy 0.707107 -0.707107) (xy 0.55557 -0.83147) (xy 0.382683 -0.92388) (xy 0.19509 -0.980785) (xy 0 -1)) (width 0.1) (fill yes)))
      {e2})
    (pad "2" smd custom (at 4.4 4.7 {e1}) (size 1 1) (layers "B.Cu") (thermal_bridge_angle 90) (options (clearance outline) (anchor circle)) 
      (primitives (gr_poly (pts (xy -0.19509 -0.980785) (xy -0.382683 -0.92388) (xy -0.55557 -0.83147) (xy -2.35557 0.21853) (xy -2.507107 0.342893) (xy -2.63147 0.49443) (xy -2.72388 0.667317) (xy -2.780785 0.85491) (xy -2.8 1.05) (xy -2.780785 1.24509) (xy -2.72388 1.432683) (xy -2.63147 1.60557) (xy -2.507107 1.757107) (xy -2.35557 1.88147) (xy -2.182683 1.97388) (xy -1.99509 2.030785) (xy -1.8 2.05) (xy -1.60491 2.030785) (xy -1.417317 1.97388) (xy -1.24443 1.88147) (xy 0.55557 0.83147) (xy 0.707107 0.707107) (xy 0.83147 0.55557) (xy 0.92388 0.382683) (xy 0.980785 0.19509) (xy 1 0) (xy 0.980785 -0.19509) (xy 0.92388 -0.382683) (xy 0.83147 -0.55557) (xy 0.707107 -0.707107) (xy 0.55557 -0.83147) (xy 0.382683 -0.92388) (xy 0.19509 -0.980785) (xy 0 -1)) (width 0.1) (fill yes)))
      {e2})
    "####,
        e0 = p.net("from"),
        e1 = n(p.rotation()),
        e2 = p.net("to")
    );

    let switch_3dmodel = format!(
        r####"
    (model {e0}
      (offset (xyz {e1} {e2} {e3}))
      (scale (xyz {e4} {e5} {e6}))
      (rotate (xyz {e7} {e8} {e9}))
    )
    "####,
        e0 = json_string(p.text("switch_3dmodel_filename")),
        e1 = n(model_offset[0]),
        e2 = n(model_offset[1]),
        e3 = n(model_offset[2]),
        e4 = n(p.component("switch_3dmodel_xyz_scale", 0)),
        e5 = n(p.component("switch_3dmodel_xyz_scale", 1)),
        e6 = n(p.component("switch_3dmodel_xyz_scale", 2)),
        e7 = n(model_rotation[0]),
        e8 = n(model_rotation[1]),
        e9 = n(model_rotation[2])
    );

    let hotswap_3dmodel = format!(
        r####"
    (model {e0}
      (offset (xyz {e1} {e2} {e3}))
      (scale (xyz {e4} {e5} {e6}))
      (rotate (xyz {e7} {e8} {e9}))
    )
	  "####,
        e0 = json_string(p.text("hotswap_3dmodel_filename")),
        e1 = n(p.component("hotswap_3dmodel_xyz_offset", 0)),
        e2 = n(p.component("hotswap_3dmodel_xyz_offset", 1)),
        e3 = n(p.component("hotswap_3dmodel_xyz_offset", 2)),
        e4 = n(p.component("hotswap_3dmodel_xyz_scale", 0)),
        e5 = n(p.component("hotswap_3dmodel_xyz_scale", 1)),
        e6 = n(p.component("hotswap_3dmodel_xyz_scale", 2)),
        e7 = n(p.component("hotswap_3dmodel_xyz_rotation", 0)),
        e8 = n(p.component("hotswap_3dmodel_xyz_rotation", 1)),
        e9 = n(p.component("hotswap_3dmodel_xyz_rotation", 2))
    );

    let keycap_3dmodel = format!(
        r####"
    (model {e0}
      (offset (xyz {e1} {e2} {e3}))
      (scale (xyz {e4} {e5} {e6}))
      (rotate (xyz {e7} {e8} {e9}))
    )
	  "####,
        e0 = json_string(p.text("keycap_3dmodel_filename")),
        e1 = n(p.component("keycap_3dmodel_xyz_offset", 0)),
        e2 = n(p.component("keycap_3dmodel_xyz_offset", 1)),
        e3 = n(p.component("keycap_3dmodel_xyz_offset", 2)),
        e4 = n(p.component("keycap_3dmodel_xyz_scale", 0)),
        e5 = n(p.component("keycap_3dmodel_xyz_scale", 1)),
        e6 = n(p.component("keycap_3dmodel_xyz_scale", 2)),
        e7 = n(p.component("keycap_3dmodel_xyz_rotation", 0)),
        e8 = n(p.component("keycap_3dmodel_xyz_rotation", 1)),
        e9 = n(p.component("keycap_3dmodel_xyz_rotation", 2))
    );

    let common_bottom = String::from(
        r####"
  )
    "####,
    );

    let (side, reversible) = (p.side(), p.flag("reversible"));
    let mut out = common_top;
    if p.flag("include_corner_marks") {
        out += &corner_marks;
    }
    if p.flag("include_keycap") {
        out += &keycap_marks;
    }
    if p.flag("include_led_outline") {
        out += &led_outline;
    }
    if p.flag("include_stem_outline") {
        out += &stem_outline;
        out += &stem_cross_outline;
    }
    if hotswap {
        if reversible || side == "F" {
            out += &hotswap_front;
            if p.flag("include_socket_silks") {
                out += &hotswap_silk_front;
            }
            if p.flag("include_socket_fabs") {
                out += &hotswap_fab_front;
            }
        }
        if reversible || side == "B" {
            out += &hotswap_back;
            if p.flag("include_socket_silks") {
                out += &hotswap_silk_back;
            }
            if p.flag("include_socket_fabs") {
                out += &hotswap_fab_back;
            }
        }
        if !p.text("hotswap_3dmodel_filename").is_empty() {
            out += &hotswap_3dmodel;
        }
    }
    if p.flag("solder") {
        if reversible && p.flag("include_custom_solder_pads") {
            out += if hotswap {
                &solder_custom_reversible_top
            } else {
                &solder_custom_reversible_bottom
            };
        } else {
            if reversible || side == "F" {
                out += &solder_front;
            }
            if reversible || side == "B" {
                out += &solder_back;
            }
        }
    }
    if !p.text("switch_3dmodel_filename").is_empty() {
        out += &switch_3dmodel;
    }
    if !p.text("keycap_3dmodel_filename").is_empty() {
        out += &keycap_3dmodel;
    }
    out += &common_bottom;
    Ok(out)
}

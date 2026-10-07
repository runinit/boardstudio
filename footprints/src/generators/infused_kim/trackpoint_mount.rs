//! `infused-kim/trackpoint_mount`
//!
//! Ported from `trackpoint_mount.js` of infused-kim/kb_ergogen_fp.
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0 (see the vendored LICENSE)
//! Author: @infused-kim
use crate::Result;
use crate::context::RenderContext;
use crate::error::GeneratorError;
use crate::generators::util::{ModelDefaults, ModelParams, infused_model};
use crate::number::js_number as n;
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 22] = [
    ParamSpec::SIDE,
    ParamSpec::string("designator", "TP"),
    ParamSpec::boolean("reverse", false),
    ParamSpec::number("drill", 5.5),
    ParamSpec::number("outline", 0.25),
    ParamSpec::boolean("show_outline_t430", false),
    ParamSpec::boolean("show_outline_x240", false),
    ParamSpec::boolean("show_outline_t460s", false),
    ParamSpec::boolean("show_board", false),
    ParamSpec::string("tp_3dmodel_side", ""),
    ParamSpec::string(
        "tp_cap_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/trackpoint/TP_Cap_Red_T460S.step",
    ),
    ParamSpec::array("tp_cap_3dmodel_xyz_scale", "[]"),
    ParamSpec::array("tp_cap_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("tp_cap_3dmodel_xyz_offset", "[]"),
    ParamSpec::string(
        "tp_extension_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/trackpoint/TP_Extension_Red_T460S_h10.5_md0.0_pcb1.6.step",
    ),
    ParamSpec::array("tp_extension_3dmodel_xyz_scale", "[]"),
    ParamSpec::array("tp_extension_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("tp_extension_3dmodel_xyz_offset", "[]"),
    ParamSpec::string(
        "tp_3dmodel_filename",
        "${KIPRJMOD}/models/boardstudio/infused-kim/trackpoint/TP_Red_T460S_platform_z_offset_+0.0_pcb_offset_-2.0.step",
    ),
    ParamSpec::array("tp_3dmodel_xyz_scale", "[]"),
    ParamSpec::array("tp_3dmodel_xyz_rotation", "[]"),
    ParamSpec::array("tp_3dmodel_xyz_offset", "[]"),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "infused-kim/trackpoint_mount",
    display_name: "trackpoint mount",
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

fn body(p: &RenderContext<'_>) -> Result<String> {
    // The bundled extension crosses the PCB with a 5 mm outer diameter.
    const EXTENSION_DIAMETER: f64 = 5.0;
    const DEFAULT_EXTENSION: &str = "${KIPRJMOD}/models/boardstudio/infused-kim/trackpoint/TP_Extension_Red_T460S_h10.5_md0.0_pcb1.6.step";
    if p.number("drill") < EXTENSION_DIAMETER
        && p.text("tp_extension_3dmodel_filename") == DEFAULT_EXTENSION
        && p.vec3("tp_extension_3dmodel_xyz_scale")?.is_none()
        && p.vec3("tp_extension_3dmodel_xyz_rotation")?.is_none()
        && p.vec3("tp_extension_3dmodel_xyz_offset")?.is_none()
    {
        return Err(GeneratorError::rejected(
            SPEC.source,
            Some("drill"),
            "The bundled trackpoint extension requires a center drill of at least 5 mm. Select a compatible extension model or drill size.",
        ));
    }
    let size = p.number("drill") + p.number("outline") * 2.0;
    let top = format!(
        r####"
      (module trackpoint_mount_t430 (layer F.Cu) (tedit 6449FFC5)
        {e0}
        (attr virtual)

        (fp_text reference "{e1}" (at 0 0) (layer {e2}.SilkS) {e3}
          (effects (font (size 1 1) (thickness 0.15)))
        )
    "####,
        e0 = p.at(),
        e1 = p.reference(),
        e2 = p.side(),
        e3 = p.ref_hide()
    );

    let front = String::from(
        r####"
        (fp_circle (center 0 -9.75) (end -2.15 -9.75) (layer F.CrtYd) (width 0.05))
        (fp_circle (center 0 -9.75) (end -1.9 -9.75) (layer Cmts.User) (width 0.15))
        (fp_circle (center 0 9.75) (end -2.15 9.75) (layer F.CrtYd) (width 0.05))
        (fp_circle (center 0 9.75) (end -1.9 9.75) (layer Cmts.User) (width 0.15))
        (fp_circle (center 0 0) (end -3.95 0) (layer F.CrtYd) (width 0.05))
        (fp_circle (center 0 0) (end -3.7 0) (layer Cmts.User) (width 0.15))

        (fp_text user %R (at 0 0 180) (layer F.Fab)
          (effects (font (size 1 1) (thickness 0.15)))
        )
    "####,
    );

    let back = String::from(
        r####"
        (fp_circle (center 0 0) (end -3.95 0) (layer B.CrtYd) (width 0.05))
        (fp_circle (center 0 0) (end -3.7 0) (layer Cmts.User) (width 0.15))
        (fp_circle (center 0 9.75) (end -2.15 9.75) (layer B.CrtYd) (width 0.05))
        (fp_circle (center 0 -9.75) (end -2.15 -9.75) (layer B.CrtYd) (width 0.05))
    "####,
    );

    let outline_t430_front = String::from(
        r####"
        (fp_line (start -4.5 -12.75) (end -9.5 -7.25) (layer F.Fab) (width 0.2))
        (fp_line (start -9.5 7.25) (end -4.5 12.75) (layer F.Fab) (width 0.2))
        (fp_line (start 6.5 8) (end 6.5 -8) (layer F.Fab) (width 0.2))
        (fp_line (start 9.5 -8) (end 9.5 -12.75) (layer F.Fab) (width 0.2))
        (fp_line (start -9.5 7.25) (end -9.5 -7.25) (layer F.Fab) (width 0.2))
        (fp_line (start 9.5 -12.75) (end -4.5 -12.75) (layer F.Fab) (width 0.2))
        (fp_line (start 9.5 12.75) (end -4.5 12.75) (layer F.Fab) (width 0.2))
        (fp_line (start 9.5 -8) (end 6.5 -8) (layer F.Fab) (width 0.2))
        (fp_line (start 9.5 8) (end 9.5 12.75) (layer F.Fab) (width 0.2))
        (fp_line (start 9.5 8) (end 6.5 8) (layer F.Fab) (width 0.2))
        (fp_line (start 8.5 5.5) (end 8.5 -5.5) (layer F.Fab) (width 0.2))
        (fp_line (start 8.5 -5.5) (end 6.5 -5.5) (layer F.Fab) (width 0.2))
        (fp_line (start 8.5 5.5) (end 6.5 5.5) (layer F.Fab) (width 0.2))
    "####,
    );

    let outline_t430_back = String::from(
        r####"
        (fp_line (start -4.5 12.75) (end -9.5 7.25) (layer B.Fab) (width 0.2))
        (fp_line (start 9.5 -8) (end 9.5 -12.75) (layer B.Fab) (width 0.12))
        (fp_line (start 9.5 8) (end 9.5 12.75) (layer B.Fab) (width 0.2))
        (fp_line (start 6.5 -8) (end 6.5 8) (layer B.Fab) (width 0.2))
        (fp_line (start 9.5 -12.75) (end -4.5 -12.75) (layer B.Fab) (width 0.2))
        (fp_line (start -9.5 -7.25) (end -4.5 -12.75) (layer B.Fab) (width 0.2))
        (fp_line (start 9.5 -8) (end 6.5 -8) (layer B.Fab) (width 0.12))
        (fp_line (start 9.5 8) (end 6.5 8) (layer B.Fab) (width 0.2))
        (fp_line (start -9.5 -7.25) (end -9.5 7.25) (layer B.Fab) (width 0.2))
        (fp_line (start 9.5 12.75) (end -4.5 12.75) (layer B.Fab) (width 0.2))
        (fp_line (start 8.5 -5.5) (end 8.5 5.5) (layer B.Fab) (width 0.2))
        (fp_line (start 8.5 -5.5) (end 6.5 -5.5) (layer B.Fab) (width 0.2))
        (fp_line (start 8.5 5.5) (end 6.5 5.5) (layer B.Fab) (width 0.2))
    "####,
    );

    let outline_x240_front = String::from(
        r####"
        (fp_line (start 12.25 -6.5) (end 6.75 -6.5) (layer F.Fab) (width 0.2))
        (fp_line (start 12.25 6.5) (end 6.75 6.5) (layer F.Fab) (width 0.2))
        (fp_line (start 12.25 6.5) (end 12.25 -6.5) (layer F.Fab) (width 0.2))
        (fp_line (start 6.75 11.5) (end -6.75 11.5) (layer F.Fab) (width 0.2))
        (fp_line (start 6.75 -11.5) (end -6.75 -11.5) (layer F.Fab) (width 0.2))
        (fp_line (start -6.75 11.5) (end -6.75 -11.5) (layer F.Fab) (width 0.2))
        (fp_line (start 6.75 11.5) (end 6.75 -11.5) (layer F.Fab) (width 0.2))
    "####,
    );

    let outline_x240_back = String::from(
        r####"
        (fp_line (start 12.25 -6.5) (end 6.75 -6.5) (layer B.Fab) (width 0.2))
        (fp_line (start 12.25 -6.5) (end 12.25 6.5) (layer B.Fab) (width 0.2))
        (fp_line (start 6.75 -11.5) (end -6.75 -11.5) (layer B.Fab) (width 0.2))
        (fp_line (start 6.75 11.5) (end -6.75 11.5) (layer B.Fab) (width 0.2))
        (fp_line (start -6.75 -11.5) (end -6.75 11.5) (layer B.Fab) (width 0.2))
        (fp_line (start 6.75 -11.5) (end 6.75 11.5) (layer B.Fab) (width 0.2))
        (fp_line (start 12.25 6.5) (end 6.75 6.5) (layer B.Fab) (width 0.2))
    "####,
    );

    let outline_x240_board = String::from(
        r####"
        (fp_line (start 39.25 12) (end 23.25 12) (layer Dwgs.User) (width 0.2))
        (fp_line (start 23.25 5.5) (end 23.25 12) (layer Dwgs.User) (width 0.2))
        (fp_line (start 23.25 -5.5) (end 23.25 5.5) (layer Dwgs.User) (width 0.2))
        (fp_line (start 23.25 5.5) (end 12.25 5.5) (layer Dwgs.User) (width 0.2))
        (fp_line (start 23.25 -5.5) (end 12.25 -5.5) (layer Dwgs.User) (width 0.2))
        (fp_line (start 39.25 -22) (end 39.25 12) (layer Dwgs.User) (width 0.2))
        (fp_line (start 39.25 -22) (end 23.25 -22) (layer Dwgs.User) (width 0.2))
        (fp_line (start 23.25 -22) (end 23.25 -5.5) (layer Dwgs.User) (width 0.2))
        (fp_line (start 12.25 -5.5) (end 12.25 5.5) (layer Dwgs.User) (width 0.2))
    "####,
    );

    let outline_t460s_front = String::from(
        r####"
        (fp_line (start 2.75 6.5) (end 6.25 3) (layer F.Fab) (width 0.2))
        (fp_line (start 2.75 11.5) (end -2.75 11.5) (layer F.Fab) (width 0.2))
        (fp_line (start -6.25 3) (end -6.25 -3) (layer F.Fab) (width 0.2))
        (fp_line (start 6.25 3) (end 6.25 -3) (layer F.Fab) (width 0.2))
        (fp_line (start 2.75 -11.5) (end -2.75 -11.5) (layer F.Fab) (width 0.2))
        (fp_line (start 2.75 6.5) (end 2.75 11.5) (layer F.Fab) (width 0.2))
        (fp_line (start -2.75 6.5) (end -2.75 11.5) (layer F.Fab) (width 0.2))
        (fp_line (start -2.75 -11.5) (end -2.75 -6.5) (layer F.Fab) (width 0.2))
        (fp_line (start 2.75 -11.5) (end 2.75 -6.5) (layer F.Fab) (width 0.2))
        (fp_line (start -2.75 6.5) (end -6.25 3) (layer F.Fab) (width 0.2))
        (fp_line (start 6.25 -3) (end 2.75 -6.5) (layer F.Fab) (width 0.2))
        (fp_line (start -6.25 -3) (end -2.75 -6.5) (layer F.Fab) (width 0.2))
    "####,
    );

    let outline_t460s_back = String::from(
        r####"
        (fp_line (start -6.25 -3) (end -2.75 -6.5) (layer B.Fab) (width 0.2))
        (fp_line (start 6.25 -3) (end 2.75 -6.5) (layer B.Fab) (width 0.2))

        (fp_line (start 2.75 6.5) (end 6.25 3) (layer B.Fab) (width 0.2))
        (fp_line (start -2.75 6.5) (end -6.25 3) (layer B.Fab) (width 0.2))

        (fp_line (start 6.25 3) (end 6.25 -3) (layer B.Fab) (width 0.2))
        (fp_line (start 2.75 11.5) (end -2.75 11.5) (layer B.Fab) (width 0.2))
        (fp_line (start -6.25 3) (end -6.25 -3) (layer B.Fab) (width 0.2))
        (fp_line (start 2.75 -11.5) (end -2.75 -11.5) (layer B.Fab) (width 0.2))
        (fp_line (start -2.75 6.5) (end -2.75 11.5) (layer B.Fab) (width 0.2))
        (fp_line (start 2.75 6.5) (end 2.75 11.5) (layer B.Fab) (width 0.2))
        (fp_line (start -2.75 -11.5) (end -2.75 -6.5) (layer B.Fab) (width 0.2))
        (fp_line (start 2.75 -11.5) (end 2.75 -6.5) (layer B.Fab) (width 0.2))
    "####,
    );

    let outline_t460s_board = String::from(
        r####"
        (fp_line (start 38.25 12.25) (end 22.25 12.25) (layer Dwgs.User) (width 0.2))
        (fp_line (start 22.25 2.75) (end 22.25 12.25) (layer Dwgs.User) (width 0.2))
        (fp_line (start 22.25 -2.75) (end 22.25 2.75) (layer Dwgs.User) (width 0.2))
        (fp_line (start 22.25 2.75) (end 6.25 2.75) (layer Dwgs.User) (width 0.2))
        (fp_line (start 22.25 -2.75) (end 6.25 -2.75) (layer Dwgs.User) (width 0.2))
        (fp_line (start 38.25 -22.25) (end 38.25 12.25) (layer Dwgs.User) (width 0.2))
        (fp_line (start 38.25 -22.25) (end 22.25 -22.25) (layer Dwgs.User) (width 0.2))
        (fp_line (start 22.25 -22.25) (end 22.25 -2.75) (layer Dwgs.User) (width 0.2))
        (fp_line (start 6.25 -2.75) (end 6.25 2.75) (layer Dwgs.User) (width 0.2))
    "####,
    );

    let bottom = format!(
        r####"
        (pad "" thru_hole circle (at 0 -9.75 180) (size 3.8 3.8) (drill 2.2) (layers *.Cu *.Mask))
        (pad 1 np_thru_hole circle (at 0 0 180) (size {e0} {e0}) (drill {e1}) (layers *.Cu *.Mask))
        (pad "" thru_hole circle (at 0 9.75 180) (size 3.8 3.8) (drill 2.2) (layers *.Cu *.Mask))
      )
    "####,
        e0 = n(size),
        e1 = n(p.number("drill"))
    );

    let final_add0 = format!(
        r####"
      {e0}

      {e1}

      {e2}
    "####,
        e0 = infused_model(
            p,
            ModelParams {
                filename: "tp_cap_3dmodel_filename",
                scale: "tp_cap_3dmodel_xyz_scale",
                rotation: "tp_cap_3dmodel_xyz_rotation",
                offset: "tp_cap_3dmodel_xyz_offset",
                side: "tp_3dmodel_side"
            },
            ModelDefaults {
                default_side: "F",
                rotation_f: [0.0, 0.0, 0.0],
                offset_f: [0.0, 0.0, 10.5],
                rotation_b: [0.0, 180.0, 0.0],
                offset_b: [0.0, 0.0, -(10.5 + 1.6)]
            }
        )?,
        e1 = infused_model(
            p,
            ModelParams {
                filename: "tp_extension_3dmodel_filename",
                scale: "tp_extension_3dmodel_xyz_scale",
                rotation: "tp_extension_3dmodel_xyz_rotation",
                offset: "tp_extension_3dmodel_xyz_offset",
                side: "tp_3dmodel_side"
            },
            ModelDefaults {
                default_side: "F",
                rotation_f: [0.0, 0.0, 0.0],
                offset_f: [0.0, 0.0, 0.0],
                rotation_b: [0.0, 180.0, 0.0],
                offset_b: [0.0, 0.0, -1.6]
            }
        )?,
        e2 = infused_model(
            p,
            ModelParams {
                filename: "tp_3dmodel_filename",
                scale: "tp_3dmodel_xyz_scale",
                rotation: "tp_3dmodel_xyz_rotation",
                offset: "tp_3dmodel_xyz_offset",
                side: "tp_3dmodel_side"
            },
            ModelDefaults {
                default_side: "F",
                rotation_f: [0.0, 0.0, 180.0],
                offset_f: [0.0, 0.0, 0.0],
                rotation_b: [0.0, 0.0, 0.0],
                offset_b: [0.0, 0.0, 0.0]
            }
        )?
    );

    let (side, reverse) = (p.side(), p.flag("reverse"));
    let (t430, x240, t460s) = (
        p.flag("show_outline_t430"),
        p.flag("show_outline_x240"),
        p.flag("show_outline_t460s"),
    );
    let mut out = top;
    if side == "F" || reverse {
        out += &front;
        if t430 {
            out += &outline_t430_front;
        }
        if x240 {
            out += &outline_x240_front;
        }
        if t460s {
            out += &outline_t460s_front;
        }
    }
    if side == "B" || reverse {
        out += &back;
        if t430 {
            out += &outline_t430_back;
        }
        if x240 {
            out += &outline_x240_back;
        }
        if t460s {
            out += &outline_t460s_back;
        }
    }
    if p.flag("show_board") {
        if x240 {
            out += &outline_x240_board;
        }
        if t460s {
            out += &outline_t460s_board;
        }
    }
    out += &final_add0;
    out += &bottom;
    Ok(out)
}

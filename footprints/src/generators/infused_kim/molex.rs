//! Shared Molex Pico-EzMate connector rendering: 3D-model pair and assembly.
//!
//! Used by `conn_molex_pico_ezmate_1x02` and `conn_molex_pico_ezmate_1x05`;
//! the per-pin front/back drawings stay in each generator.
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0 (see the vendored LICENSE)
//! Author: @infused-kim
use crate::Result;
use crate::context::RenderContext;
use crate::generators::util::{ModelDefaults, ModelParams, infused_model};

/// The cable and socket `(model ...)` blocks; both connectors share the same
/// defaults and differ only in their model filenames.
pub(crate) fn molex_models(p: &RenderContext<'_>) -> Result<(String, String)> {
    Ok((
        infused_model(
            p,
            ModelParams {
                filename: "cable_3dmodel_filename",
                scale: "cable_3dmodel_xyz_scale",
                rotation: "cable_3dmodel_xyz_rotation",
                offset: "cable_3dmodel_xyz_offset",
                side: "cable_3dmodel_side",
            },
            ModelDefaults {
                default_side: "F",
                rotation_f: [0.0, 0.0, 0.0],
                offset_f: [0.0, 0.0, 0.8],
                rotation_b: [0.0, 180.0, 0.0],
                offset_b: [0.0, 0.0, -(1.6 + 0.8)],
            },
        )?,
        infused_model(
            p,
            ModelParams {
                filename: "socket_3dmodel_filename",
                scale: "socket_3dmodel_xyz_scale",
                rotation: "socket_3dmodel_xyz_rotation",
                offset: "socket_3dmodel_xyz_offset",
                side: "socket_3dmodel_side",
            },
            ModelDefaults {
                default_side: "F",
                rotation_f: [-90.0, 0.0, 0.0],
                offset_f: [0.0, 0.0, 1.4],
                rotation_b: [-90.0, 180.0, 0.0],
                offset_b: [0.0, 0.0, -3.0],
            },
        )?,
    ))
}

/// The shared module trailer both connectors close with.
pub(crate) fn molex_bottom() -> String {
    String::from(
        r####"
      )
      "####,
    )
}

/// Front/back side assembly both connectors share.
pub(crate) fn assemble_sides(
    p: &RenderContext<'_>,
    top: String,
    front: String,
    back: String,
    models: String,
) -> String {
    let (side, reverse) = (p.side(), p.flag("reverse"));
    let mut out = top;
    if side == "F" || reverse {
        out += &front;
    }
    if side == "B" || reverse {
        out += &back;
    }
    out += &models;
    out += &molex_bottom();
    out
}

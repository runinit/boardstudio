//! Shared ceoloide rendering: 3D-model blocks and display jumper/net setup.
//!
//! Used by the CC-BY-NC-SA-4.0 display and ProMicro-family MCU generators;
//! MIT-licensed ceoloide generators keep their own copies so the two
//! licence families never mix.
//! SPDX-License-Identifier: CC-BY-NC-SA-4.0
//! Author: @infused-kim + @ceoloide improvements
use crate::context::{NetRef, RenderContext};
use crate::generators::util::json_string;
use crate::number::js_number as n;

/// One `(model ...)` block as the ceoloide generators write it.
pub(crate) fn ceoloide_model(
    filename: &str,
    offset: [f64; 3],
    scale: [f64; 3],
    rotation: [f64; 3],
) -> String {
    format!(
        r####"
    (model {e0}
      (offset (xyz {e1} {e2} {e3}))
      (scale (xyz {e4} {e5} {e6}))
      (rotate (xyz {e7} {e8} {e9}))
    )
    "####,
        e0 = json_string(filename),
        e1 = n(offset[0]),
        e2 = n(offset[1]),
        e3 = n(offset[2]),
        e4 = n(scale[0]),
        e5 = n(scale[1]),
        e6 = n(scale[2]),
        e7 = n(rotation[0]),
        e8 = n(rotation[1]),
        e9 = n(rotation[2])
    )
}

/// The net lists a reversible display derives from its pin and local nets:
/// destination order, jumper placement and silk offsets follow the
/// `invert_labels_position` / `invert_jumpers_position` flags.
pub(crate) struct DisplayJumperNets {
    pub dst_nets: Vec<NetRef>,
    pub socket_nets: Vec<NetRef>,
    pub jumpers_offset: f64,
    pub labels_offset: f64,
    pub jumpers_front_top: Vec<NetRef>,
    pub jumpers_front_bottom: Vec<NetRef>,
    pub jumpers_back_top: Vec<NetRef>,
    pub jumpers_back_bottom: Vec<NetRef>,
}

pub(crate) fn display_jumper_nets(
    p: &RenderContext<'_>,
    mut dst_nets: Vec<NetRef>,
    local_nets: Vec<NetRef>,
) -> DisplayJumperNets {
    let reversible = p.flag("reversible");
    if reversible || p.side() == "B" {
        dst_nets.reverse();
    }
    let socket_nets = if reversible {
        local_nets.clone()
    } else {
        dst_nets.clone()
    };

    let mut jumpers_offset = 0.0;
    let mut labels_offset = 3.75;
    let mut jumpers_front_top = dst_nets.clone();
    let mut jumpers_front_bottom = local_nets.clone();
    let mut jumpers_back_top = dst_nets.clone();
    let mut jumpers_back_bottom: Vec<NetRef> = local_nets.iter().rev().cloned().collect();
    if p.flag("invert_labels_position") {
        labels_offset = if reversible && !p.flag("invert_jumpers_position") {
            -1.62
        } else {
            0.0
        };
    } else if reversible && p.flag("invert_jumpers_position") {
        labels_offset = 1.62 + 3.75;
    }
    if p.flag("invert_jumpers_position") {
        jumpers_offset = 4.4;
        jumpers_front_top = local_nets.clone();
        jumpers_front_bottom = dst_nets.clone();
        jumpers_back_top = local_nets.iter().rev().cloned().collect();
        jumpers_back_bottom = dst_nets.clone();
    }
    DisplayJumperNets {
        dst_nets,
        socket_nets,
        jumpers_offset,
        labels_offset,
        jumpers_front_top,
        jumpers_front_bottom,
        jumpers_back_top,
        jumpers_back_bottom,
    }
}

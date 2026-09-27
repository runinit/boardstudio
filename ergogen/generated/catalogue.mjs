// Generated from the unchanged ergogen/library sources.
export default {
  "ceoloide/battery_connector_jst_ph_2": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Copyright (c) 2023 Marco Massarelli
//
// SPDX-License-Identifier: MIT
//
// To view a copy of this license, visit https://opensource.org/license/mit/
//
// Author: @ceoloide
//
// Description:
//  A reversible JST PH 2.0mm footprint with support for solder jumpers and traces. This is
//  the same part sold at Typeractive.xyz and LCSC.
//
//  Note that the footprint's courtyard includes the space required for the male connector
//  and its cables. Make sure to leave enough room in front of the connector. The silkscreen
//  includes a handy reference for positive and negative terminals that remains visible
//  after the connector is soldered, to ensure wire polarity is correct.
//
// Datasheet:
//  https://cdn.shopify.com/s/files/1/0618/5674/3655/files/JST-S2B-PH-K.pdf?v=1670451309
//
// Params
//    side: default is F for Front
//      the side on which to place the single-side footprint and designator, either F or B
//    reversible: default is false
//      if true, the footprint will be placed on both sides so that the PCB can be
//      reversible
//    include_traces: default is true
//      if true it will include traces that connect the jumper pads to the connector pins
//    trace_width: default is 0.250mm
//      allows to override the trace width that connects the jumper pads to the connector
//      pins. Not recommended to go below 0.25mm.
//    include_silkscreen: default is true
//      if true it will include the silkscreen. Recommended to be true to ensure connector
//      polarity is not reversed, which can lead to shorting and damage to the MCU
//    include_fabrication: default is true
//      if true it will include the outline of the connector in the fabrication layer
//    include_courtyard: default is true
//      if true it will include a courtyard outline around the connector and in front of it
//      to also account for the male connector plug and the wires. Recommended to be true
//      at least once in the development of a board to confirm sufficient clearance for the
//      connector and wires.
//    battery_connector_3dmodel_filename: default is ''
//      Allows you to specify the path to a 3D model STEP or WRL file to be
//      used when rendering the PCB. Use the ${VAR_NAME} syntax to point to
//      a KiCad configured path.
//    battery_connector_3dmodel_xyz_offset: default is [0, 0, 0]
//      xyz offset (in mm), used to adjust the position of the 3d model
//      relative the footprint.
//    battery_connector_3dmodel_xyz_scale: default is [1, 1, 1]
//      xyz scale, used to adjust the size of the 3d model relative to its
//      original size.
//    battery_connector_3dmodel_xyz_rotation: default is [0, 0, 0]
//      xyz rotation (in degrees), used to adjust the orientation of the 3d
//      model relative the footprint.

module.exports = {
  params: {
    designator: 'JST',
    side: 'F',
    reversible: false,
    include_traces: true,
    trace_width: 0.250,
    include_silkscreen: true,
    include_fabrication: true,
    include_courtyard: true,
    battery_connector_3dmodel_filename: '',
    battery_connector_3dmodel_xyz_offset: [0, 0, 0],
    battery_connector_3dmodel_xyz_rotation: [0, 0, 0],
    battery_connector_3dmodel_xyz_scale: [1, 1, 1],
    BAT_P: { type: 'net', value: 'BAT_P' },
    BAT_N: { type: 'net', value: 'GND' },
  },
  body: p => {
    let local_nets = [
      p.local_net("1"),
      p.local_net("2"),
    ];

    const standard_opening = `
    (footprint "ceoloide:battery_connector_jst_ph_2"
        (layer "${p.side}.Cu")
        ${p.at}
        (property "Reference" "${p.ref}"
            (at 0 4.8 ${p.r})
            (layer "${p.side}.SilkS")
            ${p.ref_hide}
            (effects (font (size 1 1) (thickness 0.15)))
        )
        `
    const front_fabrication = `
        (fp_line (start -2.95 -1.35) (end -2.95 6.25) (stroke (width 0.1) (type solid)) (layer "F.Fab"))
        (fp_line (start -2.95 6.25) (end 2.95 6.25) (stroke (width 0.1) (type solid)) (layer "F.Fab"))
        (fp_line (start -2.25 -1.35) (end -2.95 -1.35) (stroke (width 0.1) (type solid)) (layer "F.Fab"))
        (fp_line (start -2.25 0.25) (end -2.25 -1.35) (stroke (width 0.1) (type solid)) (layer "F.Fab"))
        (fp_line (start 2.25 -1.35) (end 2.25 0.25) (stroke (width 0.1) (type solid)) (layer "F.Fab"))
        (fp_line (start 2.25 0.25) (end -2.25 0.25) (stroke (width 0.1) (type solid)) (layer "F.Fab"))
        (fp_line (start 2.95 -1.35) (end 2.25 -1.35) (stroke (width 0.1) (type solid)) (layer "F.Fab"))
        (fp_line (start 2.95 6.25) (end 2.95 -1.35) (stroke (width 0.1) (type solid)) (layer "F.Fab"))
        `
    const front_courtyard = `
        (fp_line (start -3.45 -1.85) (end -3.45 10.5) (stroke (width 0.05) (type solid)) (layer "F.CrtYd"))
        (fp_line (start -3.45 10.5) (end 3.45 10.5) (stroke (width 0.05) (type solid)) (layer "F.CrtYd"))
        (fp_line (start 3.45 -1.85) (end -3.45 -1.85) (stroke (width 0.05) (type solid)) (layer "F.CrtYd"))
        (fp_line (start 3.45 10.5) (end 3.45 -1.85) (stroke (width 0.05) (type solid)) (layer "F.CrtYd"))
        `
    const front_silkscreen = `
        (fp_line (start -1.5 7.40) (end -0.5 7.40) (stroke (width 0.1) (type solid)) (layer "F.SilkS"))
        (fp_line (start 1.5 7.40) (end 0.5 7.40) (stroke (width 0.1) (type solid)) (layer "F.SilkS"))
        (fp_line (start 1 6.90) (end 1 7.90) (stroke (width 0.1) (type solid)) (layer "F.SilkS"))
        (fp_line (start -2.06 -1.46) (end -3.06 -1.46) (stroke (width 0.12) (type solid)) (layer "F.SilkS"))
        (fp_line (start -3.06 -1.46) (end -3.06 -0.46) (stroke (width 0.12) (type solid)) (layer "F.SilkS"))
        (fp_line (start 2.14 -1.46) (end 3.06 -1.46) (stroke (width 0.12) (type solid)) (layer "F.SilkS"))
        (fp_line (start 3.06 -1.46) (end 3.06 -0.46) (stroke (width 0.12) (type solid)) (layer "F.SilkS"))
        (fp_line (start -2.14 6.36) (end -3.06 6.36) (stroke (width 0.12) (type solid)) (layer "F.SilkS"))
        (fp_line (start -3.06 6.36) (end -3.06 5.36) (stroke (width 0.12) (type solid)) (layer "F.SilkS"))
        (fp_line (start 2.14 6.36) (end 3.06 6.36) (stroke (width 0.12) (type solid)) (layer "F.SilkS"))
        (fp_line (start 3.06 6.36) (end 3.06 5.36) (stroke (width 0.12) (type solid)) (layer "F.SilkS"))
        `
    const back_fabrication = `
        (fp_line (start -2.95 -1.35) (end -2.25 -1.35) (stroke (width 0.1) (type solid)) (layer "B.Fab"))
        (fp_line (start -2.95 6.25) (end -2.95 -1.35) (stroke (width 0.1) (type solid)) (layer "B.Fab"))
        (fp_line (start -2.25 -1.35) (end -2.25 0.25) (stroke (width 0.1) (type solid)) (layer "B.Fab"))
        (fp_line (start -2.25 0.25) (end 2.25 0.25) (stroke (width 0.1) (type solid)) (layer "B.Fab"))
        (fp_line (start 2.25 -1.35) (end 2.95 -1.35) (stroke (width 0.1) (type solid)) (layer "B.Fab"))
        (fp_line (start 2.25 0.25) (end 2.25 -1.35) (stroke (width 0.1) (type solid)) (layer "B.Fab"))
        (fp_line (start 2.95 -1.35) (end 2.95 6.25) (stroke (width 0.1) (type solid)) (layer "B.Fab"))
        (fp_line (start 2.95 6.25) (end -2.95 6.25) (stroke (width 0.1) (type solid)) (layer "B.Fab"))
        `
    const back_courtyard = `
        (fp_line (start -3.45 -1.85) (end -3.45 10.5) (stroke (width 0.05) (type solid)) (layer "B.CrtYd"))
        (fp_line (start -3.45 10.5) (end 3.45 10.5) (stroke (width 0.05) (type solid)) (layer "B.CrtYd"))
        (fp_line (start 3.45 -1.85) (end -3.45 -1.85) (stroke (width 0.05) (type solid)) (layer "B.CrtYd"))
        (fp_line (start 3.45 10.5) (end 3.45 -1.85) (stroke (width 0.05) (type solid)) (layer "B.CrtYd"))
        `
    const back_silkscreen = `
        (fp_line (start 1.5 7.40) (end 0.5 7.40) (stroke (width 0.1) (type solid)) (layer "B.SilkS"))
        (fp_line (start -1.5 7.40) (end -0.5 7.40) (stroke (width 0.1) (type solid)) (layer "B.SilkS"))
        (fp_line (start -1 6.90) (end -1 7.90) (stroke (width 0.1) (type solid)) (layer "B.SilkS"))
        (fp_line (start -2.06 -1.46) (end -3.06 -1.46) (stroke (width 0.12) (type solid)) (layer "B.SilkS"))
        (fp_line (start -3.06 -1.46) (end -3.06 -0.46) (stroke (width 0.12) (type solid)) (layer "B.SilkS"))
        (fp_line (start 2.14 -1.46) (end 3.06 -1.46) (stroke (width 0.12) (type solid)) (layer "B.SilkS"))
        (fp_line (start 3.06 -1.46) (end 3.06 -0.46) (stroke (width 0.12) (type solid)) (layer "B.SilkS"))
        (fp_line (start -2.14 6.36) (end -3.06 6.36) (stroke (width 0.12) (type solid)) (layer "B.SilkS"))
        (fp_line (start -3.06 6.36) (end -3.06 5.36) (stroke (width 0.12) (type solid)) (layer "B.SilkS"))
        (fp_line (start 2.14 6.36) (end 3.06 6.36) (stroke (width 0.12) (type solid)) (layer "B.SilkS"))
        (fp_line (start 3.06 6.36) (end 3.06 5.36) (stroke (width 0.12) (type solid)) (layer "B.SilkS"))
        `
    const front_pads = `
        (pad "1" thru_hole roundrect (at -1 0 ${p.r}) (size 1.2 1.75) (drill 0.75) (layers "*.Cu" "*.Mask") (roundrect_rratio 0.20) ${p.BAT_N.str})
        (pad "2" thru_hole oval (at 1 0 ${p.r}) (size 1.2 1.75) (drill 0.75) (layers "*.Cu" "*.Mask") ${p.BAT_P.str})
        `
    const back_pads = `
        (pad "1" thru_hole roundrect (at 1 0 ${p.r}) (size 1.2 1.75) (drill 0.75) (layers "*.Cu" "*.Mask") (roundrect_rratio 0.20) ${p.BAT_N.str})
        (pad "2" thru_hole oval (at -1 0 ${p.r}) (size 1.2 1.75) (drill 0.75) (layers "*.Cu" "*.Mask") ${p.BAT_P.str})
        `
    const reversible_pads = `
        (pad "11" thru_hole oval (at -1 0 ${p.r}) (size 1.2 1.75) (drill 0.75) (layers "*.Cu" "*.Mask") ${local_nets[0].str})
        (pad "12" thru_hole oval (at 1 0 ${p.r}) (size 1.2 1.75) (drill 0.75) (layers "*.Cu" "*.Mask") ${local_nets[1].str})
        (pad "21" smd custom (at -1 1.8 ${180 + p.r}) (size 0.1 0.1) (layers "F.Cu" "F.Mask" "F.Paste")
            (clearance 0.1) (zone_connect 0)
            (options (clearance outline) (anchor rect))
            (primitives
                (gr_poly
                    (pts
                        (xy 0.6 0.4)
                        (xy -0.6 0.4)
                        (xy -0.6 0.2)
                        (xy 0 -0.4)
                        (xy 0.6 0.2)
                    )   
                    (width 0)
                    (fill yes)
                )
            )
            ${local_nets[0]}
        )
        (pad "31" smd custom (at -1 1.8 ${180 + p.r}) (size 0.1 0.1) (layers "B.Cu" "B.Mask" "B.Paste")
            (clearance 0.1) (zone_connect 0)
            (options (clearance outline) (anchor rect))
            (primitives
                (gr_poly
                    (pts
                        (xy 0.6 0.4)
                        (xy -0.6 0.4)
                        (xy -0.6 0.2)
                        (xy 0 -0.4)
                        (xy 0.6 0.2)
                    )
                    (width 0)
                    (fill yes)
                )
            )
            ${local_nets[0]}
        )
        (pad "22" smd custom (at 1 1.8 ${180 + p.r}) (size 0.1 0.1) (layers "F.Cu" "F.Mask" "F.Paste")
            (clearance 0.1) (zone_connect 0)
            (options (clearance outline) (anchor rect))
            (primitives
                (gr_poly
                    (pts
                        (xy 0.6 0.4)
                        (xy -0.6 0.4)
                        (xy -0.6 0.2)
                        (xy 0 -0.4)
                        (xy 0.6 0.2)
                    )
                    (width 0)
                    (fill yes)
                )
            )
            ${local_nets[1]}
        )
        (pad "32" smd custom (at 1 1.8 ${180 + p.r}) (size 0.1 0.1) (layers "B.Cu" "B.Mask" "B.Paste")
            (clearance 0.1) (zone_connect 0)
            (options (clearance outline) (anchor rect))
            (primitives
                (gr_poly
                    (pts
                        (xy 0.6 0.4)
                        (xy -0.6 0.4)
                        (xy -0.6 0.2)
                        (xy 0 -0.4)
                        (xy 0.6 0.2)
                    )
                    (width 0)
                    (fill yes)
                )
            )
            ${local_nets[1]}
        )
        (pad "1" smd custom (at -1 2.816 ${180 + p.r}) (size 1.2 0.5) (layers "F.Cu" "F.Mask" "F.Paste") ${p.BAT_P.str}
            (clearance 0.1) (zone_connect 0)
            (options (clearance outline) (anchor rect))
            (primitives
                (gr_poly
                    (pts
                        (xy 0.6 0)
                        (xy -0.6 0)
                        (xy -0.6 1)
                        (xy 0 0.4)
                        (xy 0.6 1)
                    )
                    (width 0)
                    (fill yes)
                )
            )
        )
        (pad "1" smd custom (at 1 2.816 ${180 + p.r}) (size 1.2 0.5) (layers "B.Cu" "B.Mask" "B.Paste") ${p.BAT_P.str}
            (clearance 0.1) (zone_connect 0)
            (options (clearance outline) (anchor rect))
            (primitives
                (gr_poly
                    (pts
                        (xy 0.6 0)
                        (xy -0.6 0)
                        (xy -0.6 1)
                        (xy 0 0.4)
                        (xy 0.6 1)
                    )
                    (width 0)
                    (fill yes)
                )
            )
        )
        (pad "2" smd custom (at -1 2.816 ${180 + p.r}) (size 1.2 0.5) (layers "B.Cu" "B.Mask" "B.Paste") ${p.BAT_N.str}
            (clearance 0.1) (zone_connect 0)
            (options (clearance outline) (anchor rect))
            (primitives
                (gr_poly
                    (pts
                        (xy 0.6 0)
                        (xy -0.6 0)
                        (xy -0.6 1)
                        (xy 0 0.4)
                        (xy 0.6 1)
                    )
                    (width 0)
                    (fill yes)
                )
            )
        )
        (pad "2" smd custom (at 1 2.816 ${180 + p.r}) (size 1.2 0.5) (layers "F.Cu" "F.Mask" "F.Paste") ${p.BAT_N.str}
            (clearance 0.1) (zone_connect 0)
            (options (clearance outline) (anchor rect))
            (primitives
                (gr_poly
                    (pts
                        (xy 0.6 0)
                        (xy -0.6 0)
                        (xy -0.6 1)
                        (xy 0 0.4)
                        (xy 0.6 1)
                    )
                    (width 0)
                    (fill yes)
                )
            ) 
        )
        `
    const standard_closing = `
    )
        `

    const reversible_traces = ` 
    (segment (start ${p.eaxy(-1, 1.8)}) (end ${p.eaxy(-1, 0)}) (width ${p.trace_width}) (layer "F.Cu") (net ${local_nets[0].index}))
    (segment (start ${p.eaxy(-1, 1.8)}) (end ${p.eaxy(-1, 0)}) (width ${p.trace_width}) (layer "B.Cu") (net ${local_nets[0].index}))
    (segment (start ${p.eaxy(1, 1.8)}) (end ${p.eaxy(1, 0)}) (width ${p.trace_width}) (layer "F.Cu") (net ${local_nets[1].index}))
    (segment (start ${p.eaxy(1, 1.8)}) (end ${p.eaxy(1, 0)}) (width ${p.trace_width}) (layer "B.Cu") (net ${local_nets[1].index}))
        `

    // Center the pin-1-origin model on the selected mounting side.
    const model_offset = p.battery_connector_3dmodel_xyz_offset || [p.side === 'B' ? 1 : -1, 0, 0];
    const model_rotation = p.battery_connector_3dmodel_xyz_rotation || [0, 0, p.side === 'B' ? 180 : 0];
    const battery_connector_3dmodel = `
    (model ${JSON.stringify(p.battery_connector_3dmodel_filename)}
      (offset (xyz ${model_offset[0]} ${model_offset[1]} ${model_offset[2]}))
      (scale (xyz ${p.battery_connector_3dmodel_xyz_scale[0]} ${p.battery_connector_3dmodel_xyz_scale[1]} ${p.battery_connector_3dmodel_xyz_scale[2]}))
      (rotate (xyz ${model_rotation[0]} ${model_rotation[1]} ${model_rotation[2]}))
    )
    `

    let final = standard_opening;

    if (p.side == "F" || p.reversible) {
      if (p.include_fabrication) {
        final += front_fabrication;
      }
      if (p.include_courtyard) {
        final += front_courtyard;
      }
      if (p.include_silkscreen) {
        final += front_silkscreen;
      }
    }
    if (p.side == "B" || p.reversible) {
      if (p.include_fabrication) {
        final += back_fabrication;
      }
      if (p.include_courtyard) {
        final += back_courtyard;
      }
      if (p.include_silkscreen) {
        final += back_silkscreen;
      }
    }
    if (p.reversible) {
      final += reversible_pads;
    } else if (p.side == "F") {
      final += front_pads;
    } else if (p.side == "B") {
      final += back_pads;
    }
    if (p.battery_connector_3dmodel_filename) {
      final += battery_connector_3dmodel
    }
    final += standard_closing;
    if (p.reversible && p.include_traces) {
      final += reversible_traces;
    }
    return final;
  }
}


;module.exports = ((original) => {
    const defaults = {"battery_connector_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/kicad/JST_PH_S2B-PH-K_1x02_P2.00mm_Horizontal.step","battery_connector_3dmodel_xyz_offset":"","battery_connector_3dmodel_xyz_rotation":""};
    const bossVariant = null;
    const chocV2 = "${KIPRJMOD}/models/boardstudio/koktoh/Choc_V2_Red.step";
    return {
      ...original,
      params: {...original.params, ...defaults},
      body: p => {
        // The bundled extension crosses the PCB with a 5 mm outer diameter.
        const extensionDiameter = 5;
        if (false && p.drill < extensionDiameter
          && p.tp_extension_3dmodel_filename === defaults.tp_extension_3dmodel_filename
          && !p.tp_extension_3dmodel_xyz_scale
          && !p.tp_extension_3dmodel_xyz_rotation
          && !p.tp_extension_3dmodel_xyz_offset) {
          throw new Error('The bundled trackpoint extension requires a center drill of at least 5 mm. Select a compatible extension model or drill size.');
        }
        if (false && !p.choc_v1_support) {
          p = {...p};
          if (!p.choc_v2_support) {
            for (const key of Object.keys(defaults)) {
              if (p[key] === defaults[key]) { p[key] = ''; }
            }
          } else {
            if (p.switch_3dmodel_filename === defaults.switch_3dmodel_filename) {
              const automatic = !p.switch_3dmodel_xyz_rotation && !p.switch_3dmodel_xyz_offset
                && p.switch_3dmodel_xyz_scale.every(value => value === 1);
              const postDiameter = 4.8;
              if (automatic && (!p.include_stabilizer_pad || p.oval_stabilizer_pad
                || (p.center_hole_diameter > 0 && p.center_hole_diameter < postDiameter))) {
                throw new Error('The bundled Choc V2 model requires the round stabilizer hole and a center drill of at least 4.8 mm. Use a matching model or transform for modified hardware.');
              }
              p.switch_3dmodel_filename = chocV2;
              p.switch_3dmodel_xyz_rotation ||= p.side === 'F' ? [180, 0, 0] : [0, 180, 0];
              p.switch_3dmodel_xyz_offset ||= [0, 0, -p.pcb_thickness];
            }
            if (p.keycap_3dmodel_filename === defaults.keycap_3dmodel_filename) {
              p.keycap_3dmodel_filename = '';
            }
          }
        }
        if (bossVariant && p.include_bosses && p.reset_switch_3dmodel_filename === defaults.reset_switch_3dmodel_filename) {
          p = {...p, reset_switch_3dmodel_filename: bossVariant};
        }
        return original.body(p);
      }
    };
  })(module.exports);

return module.exports; })(),
  "ceoloide/battery_connector_molex_pico_ezmate_1x02": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
/*
Copyright (c) 2023 Marco Massarelli

SPDX-License-Identifier: CC-BY-NC-SA-4.0

To view a copy of this license, visit http://creativecommons.org/licenses/by-nc-sa/4.0/

Author: @infused-kim + @ceoloide improvements

Description:
  PCB footprint for for molex pico ezmate connector with 2 pins. Ideal for
  battery connections.

  This connector was chosen over the more common JST connector, because it
  has a mated profile height of only 1.65 mm. This is lower than the Kailh
  Choc hotswap sockets.

  It should also be compatible with the JST ACH connector (which is almost the
  same).

  One downside is that there are almost no batteries that ship with this
  connector. The one exception is the Nintendo Joycon 500 mAh battery.

  If you want to use the common 301230 battery, you will either need to crimp
  the connector yourself or buy a pre-crimped connector that you attach to
  the battery wires (available on digikey).

Params:
  side: default is F for Front
      the side on which to place the single-side footprint and designator, either F or B
  reversible: default is false
    if true, it will include pads on both Front and Back to make the footprint reversible
  include_silkscreen: default is true
    if true it will include the silkscreen. Recommended to be true to ensure connector
    polarity is not reversed, which can lead to shorting and damage to the MCU
  include_fabrication: default is true
    if true it will include the outline of the connector in the fabrication layer
  include_courtyard: default is true
    if true it will include a courtyard outline around the connector and in front of it
    to also account for the male connector plug and the wires. Recommended to be true
    at least once in the development of a board to confirm sufficient clearance for the
    connector and wires.

@ceoloide's improvements:
  - Add parameters to control silkscreen, fabrication, and courtyard
  - Expand courtyard to include space for the connector cable
  - Add polarity silkscreen
  - Upgrade to KiCad 8
*/

module.exports = {
  params: {
    designator: 'CONN',
    side: 'F',
    reversible: false,
    include_silkscreen: true,
    include_fabrication: true,
    include_courtyard: true,
    socket_3dmodel_filename: '',
    socket_3dmodel_xyz_scale: [1, 1, 1],
    socket_3dmodel_xyz_rotation: '',
    socket_3dmodel_xyz_offset: [0, 0, 1.4],
    cable_3dmodel_filename: '',
    cable_3dmodel_xyz_scale: [1, 1, 1],
    cable_3dmodel_xyz_rotation: '',
    cable_3dmodel_xyz_offset: [0, 0, 0.8],
    BAT_P: { type: 'net', value: 'BAT_P' },
    BAT_N: { type: 'net', value: 'GND' },
  },
  body: p => {
    const top = `
  (footprint "ceoloide:battery_connector_molex_pico_ezmate_1x02"
    (layer "${p.side}.Cu")
    ${p.at}
    (property "Reference" "${p.ref}"
      (at 0.1 3.9 ${p.r})
      (layer "${p.side}.SilkS")
      ${p.ref_hide}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (attr smd)
    `;

    const front_silkscreen = `
    (fp_line (start 0.5 3.85) (end 1.5 3.85) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 1 4.35) (end 1 3.35) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -0.5 3.85) (end -1.5 3.85) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 0.64 2.63) (end 1.14 2.63) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 0.34 2.13) (end 0.64 2.63) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -0.34 2.13) (end 0.34 2.13) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -0.64 2.63) (end -0.34 2.13) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -1.16 -2.09) (end -1.16 -2.3) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -2.21 -2.09) (end -1.16 -2.09) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -2.21 1.24) (end -2.21 -2.09) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -1.14 2.63) (end -0.64 2.63) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 2.21 -2.09) (end 1.16 -2.09) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 2.21 1.24) (end 2.21 -2.09) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    `
    const front_fabrication = `
    (fp_line (start -0.45 2.02) (end 0.45 2.02) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -0.75 2.52) (end -0.45 2.02) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -2.1 2.52) (end -0.75 2.52) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -2.1 -1.98) (end 2.1 -1.98) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -0.6 -1.272893) (end -0.1 -1.98) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -1.1 -1.98) (end -0.6 -1.272893) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.1 -1.98) (end 2.1 2.52) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -2.1 -1.98) (end -2.1 2.52) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 0.75 2.52) (end 2.1 2.52) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 0.45 2.02) (end 0.75 2.52) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    `
    const front_courtyard = `
    (fp_line (start 2.6 -2.8) (end -2.6 -2.8) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -2.6 -2.8) (end -2.6 6.75) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 2.6 6.75) (end 2.6 -2.8) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -2.6 6.75) (end 2.6 6.75) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    `
    const front_pads = `
    (pad "" smd roundrect (at 1.75 1.9 ${p.r}) (size 0.7 0.8) (layers "F.Cu" "F.Paste" "F.Mask") (roundrect_rratio 0.25))
    (pad "" smd roundrect (at -1.75 1.9 ${p.r}) (size 0.7 0.8) (layers "F.Cu" "F.Paste" "F.Mask") (roundrect_rratio 0.25))
    (pad "2" smd roundrect (at 0.6 -1.875 ${p.r}) (size 0.6 0.85) (layers "F.Cu" "F.Paste" "F.Mask") (roundrect_rratio 0.25) ${p.BAT_P.str})
    (pad "1" smd roundrect (at -0.6 -1.875 ${p.r}) (size 0.6 0.85) (layers "F.Cu" "F.Paste" "F.Mask") (roundrect_rratio 0.25) ${p.BAT_N.str})
    `
    const back_silkscreen = `
    (fp_line (start 0.5 3.85) (end 1.5 3.85) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -0.5 3.85) (end -1.5 3.85) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -1 4.35) (end -1 3.35) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -0.34 2.13) (end -0.64 2.63) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -0.64 2.63) (end -1.14 2.63) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 1.16 -2.09) (end 1.16 -2.3) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 2.21 -2.09) (end 1.16 -2.09) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 2.21 1.24) (end 2.21 -2.09) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -2.21 -2.09) (end -1.16 -2.09) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -2.21 1.24) (end -2.21 -2.09) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 0.64 2.63) (end 0.34 2.13) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 1.14 2.63) (end 0.64 2.63) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 0.34 2.13) (end -0.34 2.13) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    `
    const back_fabrication = `
    (fp_line (start 2.1 -1.98) (end -2.1 -1.98) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 0.6 -1.272893) (end 0.1 -1.98) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 1.1 -1.98) (end 0.6 -1.272893) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -2.1 -1.98) (end -2.1 2.52) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.1 -1.98) (end 2.1 2.52) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -0.75 2.52) (end -2.1 2.52) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -0.45 2.02) (end -0.75 2.52) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 0.45 2.02) (end -0.45 2.02) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 0.75 2.52) (end 0.45 2.02) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.1 2.52) (end 0.75 2.52) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    `
    const back_courtyard = `
    (fp_line (start -2.6 6.75) (end -2.6 -2.8) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 2.6 6.75) (end -2.6 6.75) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -2.6 -2.8) (end 2.6 -2.8) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 2.6 -2.8) (end 2.6 6.75) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    `
    const back_pads = `
    (pad "" smd roundrect (at 1.75 1.9 ${180 + p.r}) (size 0.7 0.8) (layers "B.Cu" "B.Paste" "B.Mask") (roundrect_rratio 0.25))
    (pad "" smd roundrect (at -1.75 1.9 ${180 + p.r}) (size 0.7 0.8) (layers "B.Cu" "B.Paste" "B.Mask") (roundrect_rratio 0.25))
    (pad "1" smd roundrect (at 0.6 -1.875 ${180 + p.r}) (size 0.6 0.85) (layers "B.Cu" "B.Paste" "B.Mask") (roundrect_rratio 0.25) ${p.BAT_N.str})
    (pad "2" smd roundrect (at -0.6 -1.875 ${180 + p.r}) (size 0.6 0.85) (layers "B.Cu" "B.Paste" "B.Mask") (roundrect_rratio 0.25) ${p.BAT_P.str})
    `
    const bottom = `
  )
    `
    let final = top;
    if (p.side == "F" || p.reversible) {
      final += front_pads;
      if(p.include_silkscreen) final += front_silkscreen;
      if(p.include_courtyard) final += front_courtyard;
      if(p.include_fabrication) final += front_fabrication;
    }
    if (p.side == "B" || p.reversible) {
      final += back_pads;
      if(p.include_silkscreen) final += back_silkscreen;
      if(p.include_courtyard) final += back_courtyard;
      if(p.include_fabrication) final += back_fabrication;
    }
    // KiCad flips a back-side model around X; turn it to match the mirrored pins.
    const model = (name, rotation) => {
      const path = p[`${name}_3dmodel_filename`];
      if (!path) {
        return '';
      }
      const angles = p[`${name}_3dmodel_xyz_rotation`] || rotation;
      return `
    (model "${path}"
      (offset (xyz ${p[`${name}_3dmodel_xyz_offset`].join(' ')}))
      (scale (xyz ${p[`${name}_3dmodel_xyz_scale`].join(' ')}))
      (rotate (xyz ${angles.join(' ')})))`;
    };
    const turn = p.side === 'B' ? 180 : 0;
    final += model('socket', [-90, 0, turn]);
    final += model('cable', [0, 0, turn]);
    final += bottom;
    return final;
  }
}
;module.exports = ((original) => {
    const defaults = {"socket_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/Molex_Ezmate_Pico_Socket_2pin.step","cable_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/Molex_Ezmate_Pico_Cable_2pin.step"};
    const bossVariant = null;
    const chocV2 = "${KIPRJMOD}/models/boardstudio/koktoh/Choc_V2_Red.step";
    return {
      ...original,
      params: {...original.params, ...defaults},
      body: p => {
        // The bundled extension crosses the PCB with a 5 mm outer diameter.
        const extensionDiameter = 5;
        if (false && p.drill < extensionDiameter
          && p.tp_extension_3dmodel_filename === defaults.tp_extension_3dmodel_filename
          && !p.tp_extension_3dmodel_xyz_scale
          && !p.tp_extension_3dmodel_xyz_rotation
          && !p.tp_extension_3dmodel_xyz_offset) {
          throw new Error('The bundled trackpoint extension requires a center drill of at least 5 mm. Select a compatible extension model or drill size.');
        }
        if (false && !p.choc_v1_support) {
          p = {...p};
          if (!p.choc_v2_support) {
            for (const key of Object.keys(defaults)) {
              if (p[key] === defaults[key]) { p[key] = ''; }
            }
          } else {
            if (p.switch_3dmodel_filename === defaults.switch_3dmodel_filename) {
              const automatic = !p.switch_3dmodel_xyz_rotation && !p.switch_3dmodel_xyz_offset
                && p.switch_3dmodel_xyz_scale.every(value => value === 1);
              const postDiameter = 4.8;
              if (automatic && (!p.include_stabilizer_pad || p.oval_stabilizer_pad
                || (p.center_hole_diameter > 0 && p.center_hole_diameter < postDiameter))) {
                throw new Error('The bundled Choc V2 model requires the round stabilizer hole and a center drill of at least 4.8 mm. Use a matching model or transform for modified hardware.');
              }
              p.switch_3dmodel_filename = chocV2;
              p.switch_3dmodel_xyz_rotation ||= p.side === 'F' ? [180, 0, 0] : [0, 180, 0];
              p.switch_3dmodel_xyz_offset ||= [0, 0, -p.pcb_thickness];
            }
            if (p.keycap_3dmodel_filename === defaults.keycap_3dmodel_filename) {
              p.keycap_3dmodel_filename = '';
            }
          }
        }
        if (bossVariant && p.include_bosses && p.reset_switch_3dmodel_filename === defaults.reset_switch_3dmodel_filename) {
          p = {...p, reset_switch_3dmodel_filename: bossVariant};
        }
        return original.body(p);
      }
    };
  })(module.exports);

return module.exports; })(),
  "ceoloide/diode_tht_sod123": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Copyright (c) 2023 Marco Massarelli
//
// SPDX-License-Identifier: CC-BY-NC-SA-4.0
//
// To view a copy of this license, visit http://creativecommons.org/licenses/by-nc-sa/4.0/
//
// Authors: @ergogen + (@infused-kim, @ceoloide, @achamian, @im-AMS improvements)
//
// Description:
//  Combined Thru-Hole and SMD diode footprint for SOD-123 package, like the Semtech 1N4148W
//  component sold by Typeractive.xyz or LCSC.
//
// Datasheet:
//  https://cdn.shopify.com/s/files/1/0618/5674/3655/files/Semtech-1N4148W.pdf?v=1670451309
//
// Params:
//    side: default is B for Back
//      the side on which to place the single-side footprint and designator, either F or B
//    reversible: default is false
//      if true, the footprint will be placed on both sides so that the PCB can be
//      reversible
//    include_thru_hole_smd_pads: default is false
//      if true, the reversible pads will include a thru-hole on the SMD pad to connect
//      front to back, for ease of routing. This may make SMD soldering slightly harder.
//    include_traces_vias: default is false
//      if true it will include traces and vias when reversible is true, and include_tht if false,
//      using this can make routing simpler with reversible PCBs. In the other cases it's simply
//      not needed.
//    trace_distance: default is 1.2
//      this is the extra distance the trace moves from the pad, and creates a via. By default it goes outward.
//      you could set this to -1.1, and the trace moves inward and creates the via under the diode.
//    trace_width: default is 0.250mm
//      allows to override the trace width that connects the pads. Not recommended
//      to go below 0.15mm (JLCPC min is 0.127mm).
//    via_size: default is 0.6
//      allows to define the size of the via. Not recommended below 0.56 (JLCPCB minimum),
//      or above 0.8 (KiCad default), to avoid overlap or DRC errors
//    via_drill: default is 0.3
//      allows to define the size of the drill. Not recommended below 0.3 (JLCPCB minimum),
//      or above 0.4 (KiCad default), to avoid overlap or DRC errors
//    include_tht: default is false
//      if true it includes through-hole pads alongside SMD ones
//    diode_3dmodel_filename: default is ''
//      Allows you to specify the path to a 3D model STEP or WRL file to be
//      used when rendering the PCB. Use the ${VAR_NAME} syntax to point to
//      a KiCad configured path.
//    diode_3dmodel_xyz_offset: default is [0, 0, 0]
//      xyz offset (in mm), used to adjust the position of the 3d model
//      relative the footprint.
//    diode_3dmodel_xyz_scale: default is [1, 1, 1]
//      xyz scale, used to adjust the size of the 3d model relative to its
//      original size.
//    diode_3dmodel_xyz_rotation: default is [0, 0, 0]
//      xyz rotation (in degrees), used to adjust the orientation of the 3d
//      model relative the footprint.
//
// @infused-kim's improvements:
//  - Add option to hide thru-holes
//  - Add virtual attribute to silence DRC error
//
// @ceoloide's improvements:
//  - Add single side support
//  - Upgrade to KiCad 8
//  - Add traces for THT pads
//
// @grazfather's improvements:
//  - Add support for switch 3D model
//
// @achamian's improvements:
//  - Add thru-hole SMD pads for ease of routing
//
// @im-AMS's improvements:
//  - Add traces and vias to connect reversible SMD pads when no THT are present

module.exports = {
  params: {
    designator: 'D',
    side: 'B',
    reversible: false,
    include_traces_vias: false,
    trace_distance: { type: 'number', value: 1.2 },
    trace_width: 0.25,
    via_size: 0.6,
    via_drill: 0.3,
    include_tht: false,
    include_thru_hole_smd_pads: false,
    diode_3dmodel_filename: '',
    diode_3dmodel_xyz_offset: [0, 0, 0],
    diode_3dmodel_xyz_rotation: [0, 0, 0],
    diode_3dmodel_xyz_scale: [1, 1, 1],
    from: { type: 'net', value: undefined },
    to: { type: 'net', value: undefined }
  },
  body: p => {
    if (p.include_thru_hole_smd_pads && !p.reversible) {
      throw new Error("diode_tht_sod123: include_thru_hole_smd_pads requires reversible: true");
    }

    const standard_opening = `
    (footprint "ceoloide:diode_tht_sod123"
        (layer "${p.reversible ? 'F' : p.side}.Cu")
        ${p.at}
        (property "Reference" "${p.ref}"
            (at 0 0 ${p.r})
            (layer "${p.reversible ? 'F' : p.side}.SilkS")
            ${p.ref_hide}
            (effects (font (size 1 1) (thickness 0.15)))
        )
        `
    // This can be useful to avoid confusion from the fab, since via-in-pads are usually premium
    const thru_hole_smd_pads_description = `
      (property "Description" "Thru-hole SMD pads, *NOT* via-in-pad (do not plug or tent)."
        (at 0 0 0)
        (unlocked yes)
        (layer "F.Fab")
        (hide yes)
        (effects
          (font
            (size 1.27 1.27)
          )
        )
      )
    `
    const front_silk = `
      (fp_line (start 0.25 0) (end 0.75 0) (layer "F.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start 0.25 0.4) (end -0.35 0) (layer "F.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start 0.25 -0.4) (end 0.25 0.4) (layer "F.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start -0.35 0) (end 0.25 -0.4) (layer "F.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start -0.35 0) (end -0.35 0.55) (layer "F.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start -0.35 0) (end -0.35 -0.55) (layer "F.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start -0.75 0) (end -0.35 0) (layer "F.SilkS") (stroke (width 0.1) (type solid)))
        `

    const front_smd_pads = `
      (pad "1" smd rect (at -1.65 0 ${p.r}) (size 0.9 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${p.to.str})
      (pad "2" smd rect (at 1.65 0 ${p.r}) (size 0.9 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${p.from.str})
        `
    const back_silk = `
      (fp_line (start 0.25 0) (end 0.75 0) (layer "B.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start 0.25 0.4) (end -0.35 0) (layer "B.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start 0.25 -0.4) (end 0.25 0.4) (layer "B.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start -0.35 0) (end 0.25 -0.4) (layer "B.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start -0.35 0) (end -0.35 0.55) (layer "B.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start -0.35 0) (end -0.35 -0.55) (layer "B.SilkS") (stroke (width 0.1) (type solid)))
      (fp_line (start -0.75 0) (end -0.35 0) (layer "B.SilkS") (stroke (width 0.1) (type solid)))
        `
    const back_smd_pads = `
      (pad "1" smd rect (at -1.65 0 ${p.r}) (size 0.9 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${p.to.str})
      (pad "2" smd rect (at 1.65 0 ${p.r}) (size 0.9 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${p.from.str})
        `
      
    const reversible_tht_pads = `
      (pad "1" thru_hole rect (at -1.65 0 ${p.r}) (size 0.9 1.2) (drill 0.3) (layers "*.Cu" "*.Paste" "*.Mask") ${p.to.str})
      (pad "2" thru_hole rect (at 1.65 0 ${p.r}) (size 0.9 1.2) (drill 0.3) (layers "*.Cu" "*.Paste" "*.Mask") ${p.from.str})
        `

    const tht = `
      (pad "1" thru_hole rect (at -3.81 0 ${p.r}) (size 1.778 1.778) (drill 0.9906) (layers "*.Cu" "*.Mask") ${p.to.str})
      (pad "2" thru_hole circle (at 3.81 0 ${p.r}) (size 1.905 1.905) (drill 0.9906) (layers "*.Cu" "*.Mask") ${p.from.str})
        `

    const diode_3dmodel = `
      (model ${JSON.stringify(p.diode_3dmodel_filename)}
          (offset (xyz ${p.diode_3dmodel_xyz_offset[0]} ${p.diode_3dmodel_xyz_offset[1]} ${p.diode_3dmodel_xyz_offset[2]}))
          (scale (xyz ${p.diode_3dmodel_xyz_scale[0]} ${p.diode_3dmodel_xyz_scale[1]} ${p.diode_3dmodel_xyz_scale[2]}))
          (rotate (xyz ${p.diode_3dmodel_xyz_rotation[0]} ${p.diode_3dmodel_xyz_rotation[1]} ${p.diode_3dmodel_xyz_rotation[2]})))
        `
    const standard_closing = `
    )
        `
    const tht_traces = `
    (segment
      (start ${p.eaxy(3.81, 0)})
      (end ${p.eaxy(1.65, 0)})
      (width ${p.trace_width})
      (layer "F.Cu")
      (net ${p.from.index})
    )
    (segment
      (start ${p.eaxy(3.81, 0)})
      (end ${p.eaxy(1.65, 0)})
      (width ${p.trace_width})
      (layer "B.Cu")
      (net ${p.from.index})
    )
    (segment
      (start ${p.eaxy(-1.65, 0)})
      (end ${p.eaxy(-3.81, 0)})
      (width ${p.trace_width})
      (layer "F.Cu")
      (net ${p.to.index})
    )
    (segment
      (start ${p.eaxy(-1.65, 0)})
      (end ${p.eaxy(-3.81, 0)})
      (width ${p.trace_width})
      (layer "B.Cu")
      (net ${p.to.index})
    )
    `

    const smd_pad_traces = `
    (segment
      (start ${p.eaxy(1.65, 0)})
      (end ${p.eaxy(1.65 + 1*p.trace_distance, 0)})
      (width ${p.trace_width})
      (layer "F.Cu")
      (net ${p.from.index})
    )
    (via
      (at ${p.eaxy(1.65 + 1*p.trace_distance, 0)})
      (size ${p.via_size})
      (drill ${p.via_drill})
      (layers "F.Cu" "B.Cu")
      (net ${p.from.index})
    )
    (segment
      (start ${p.eaxy(1.65 + 1*p.trace_distance, 0)})
      (end ${p.eaxy(1.65, 0)})
      (width ${p.trace_width})
      (layer "B.Cu")
      (net ${p.from.index})
    )
    (segment
      (start ${p.eaxy(-1.65, 0)})
      (end ${p.eaxy(-1.65 - 1*p.trace_distance, 0)})
      (width ${p.trace_width})
      (layer "F.Cu")
      (net ${p.to.index})
    )
    (via
      (at ${p.eaxy(-1.65 - 1*p.trace_distance, 0)})
      (size ${p.via_size})
      (drill ${p.via_drill})
      (layers "F.Cu" "B.Cu")
      (net ${p.to.index})
    )
    (segment
      (start ${p.eaxy(-1.65 - 1*p.trace_distance, 0)})
      (end ${p.eaxy(-1.65, 0)})
      (width ${p.trace_width})
      (layer "B.Cu")
      (net ${p.to.index})
    )
    `

    let final = standard_opening;

    if (p.side == "F" || p.reversible) {
      final += front_silk;
      if(!p.include_thru_hole_smd_pads) {
        final += front_smd_pads;
      }
    }
    if (p.side == "B" || p.reversible) {
      final += back_silk;
      if(!p.include_thru_hole_smd_pads) {
        final += back_smd_pads;
      }
    }
    if (p.include_tht) {
      final += tht;
    }
    if (p.reversible && p.include_thru_hole_smd_pads) {
      final += thru_hole_smd_pads_description;
      final += reversible_tht_pads;
    }

    if (p.diode_3dmodel_filename) {
      final += diode_3dmodel;
    }

    final += standard_closing;

    if (p.reversible && p.include_traces_vias) {
      if(p.include_tht) {
        final += tht_traces;
      } else if (!p.include_tht && !p.include_thru_hole_smd_pads) {
        final += smd_pad_traces;
      }
    }

    return final;
  }
}


;module.exports = ((original) => {
    const defaults = {"diode_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/Diode_1N4148W.step","diode_3dmodel_xyz_rotation":[-90,0,0],"diode_3dmodel_xyz_offset":[0,0,0.7]};
    const bossVariant = null;
    const chocV2 = "${KIPRJMOD}/models/boardstudio/koktoh/Choc_V2_Red.step";
    return {
      ...original,
      params: {...original.params, ...defaults},
      body: p => {
        // The bundled extension crosses the PCB with a 5 mm outer diameter.
        const extensionDiameter = 5;
        if (false && p.drill < extensionDiameter
          && p.tp_extension_3dmodel_filename === defaults.tp_extension_3dmodel_filename
          && !p.tp_extension_3dmodel_xyz_scale
          && !p.tp_extension_3dmodel_xyz_rotation
          && !p.tp_extension_3dmodel_xyz_offset) {
          throw new Error('The bundled trackpoint extension requires a center drill of at least 5 mm. Select a compatible extension model or drill size.');
        }
        if (false && !p.choc_v1_support) {
          p = {...p};
          if (!p.choc_v2_support) {
            for (const key of Object.keys(defaults)) {
              if (p[key] === defaults[key]) { p[key] = ''; }
            }
          } else {
            if (p.switch_3dmodel_filename === defaults.switch_3dmodel_filename) {
              const automatic = !p.switch_3dmodel_xyz_rotation && !p.switch_3dmodel_xyz_offset
                && p.switch_3dmodel_xyz_scale.every(value => value === 1);
              const postDiameter = 4.8;
              if (automatic && (!p.include_stabilizer_pad || p.oval_stabilizer_pad
                || (p.center_hole_diameter > 0 && p.center_hole_diameter < postDiameter))) {
                throw new Error('The bundled Choc V2 model requires the round stabilizer hole and a center drill of at least 4.8 mm. Use a matching model or transform for modified hardware.');
              }
              p.switch_3dmodel_filename = chocV2;
              p.switch_3dmodel_xyz_rotation ||= p.side === 'F' ? [180, 0, 0] : [0, 180, 0];
              p.switch_3dmodel_xyz_offset ||= [0, 0, -p.pcb_thickness];
            }
            if (p.keycap_3dmodel_filename === defaults.keycap_3dmodel_filename) {
              p.keycap_3dmodel_filename = '';
            }
          }
        }
        if (bossVariant && p.include_bosses && p.reset_switch_3dmodel_filename === defaults.reset_switch_3dmodel_filename) {
          p = {...p, reset_switch_3dmodel_filename: bossVariant};
        }
        return original.body(p);
      }
    };
  })(module.exports);

return module.exports; })(),
  "ceoloide/display_nice_view": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Copyright (c) 2023 Marco Massarelli
//
// SPDX-License-Identifier: CC-BY-NC-SA-4.0
//
// To view a copy of this license, visit http://creativecommons.org/licenses/by-nc-sa/4.0/
//
// Author: @infused-kim + @ceoloide improvements
//
// Description:
//  Reversible footprint for nice!view display. Includes an outline of the
//  display to make positioning easier.
//
//  Note that because the center pin is VCC on both sides, there is no associated jumper pad
//  in the reversible footprint.
//
//  In its default configuration, jumper pads are positioned above the pins, when the
//  component is oriented verically and pointing upwards, or left of the pins, when oriented
//  horizontally and oriented leftward. Jumper pads position can be inverted with a parameter.
//
//  In its default configuration, labels are positioned below the pins, when the
//  component is oriented verically and pointing upwards, or right of the pins, when oriented
//  horizontally and oriented leftward. Labels position can be inverted with a parameter.
//
// Pinout and schematics:
//  https://nicekeyboards.com/docs/nice-view/pinout-schematic
//
// Params:
//    side: default is F for Front
//      the side on which to place the single-side footprint and designator, either F or B
//    reversible: default is false
//      if true, the footprint will be placed on both sides so that the PCB can be
//      reversible
//    include_traces: default is true
//      if true it will include traces that connect the jumper pads to the vias
//      and the through-holes for the MCU
//    gnd_trace_width: default is 0.250mm
//      allows to override the GND trace width. Not recommended to go below 0.25mm (JLCPC
//      min is 0.127mm).
//    signal_trace_width: default is 0.250mm
//      allows to override the trace width that connects the jumper pads to the MOSI, SCK,
//      and CS pins. Not recommended to go below 0.15mm (JLCPC min is 0.127mm).
//    invert_jumpers_position default is false
//      allows to change the position of the jumper pads, from their default to the opposite
//      side of the pins. See the description above for more details.
//    invert_labels_position default is false
//      allows to change the position of the labels, from their default to the north
//      side of the pins. See the description above for more details.
//    include_silkscreen: default is true
//      if true it will include the silkscreen layer.
//    include_labels default is true
//      if true and Silkscreen layer is included, it will include the pin labels. The labels
//      will match the *opposite* side of the board when the footprint is set to be reversible, 
//      since they are meant to match the solder jumpers behavior and aid testing.
//    include_courtyard: default is true
//      if true it will include a courtyard outline around the pin header.
//    niceview_3dmodel_filename: default is ''
//      Allows you to specify the path to a 3D model STEP or WRL file to be
//      used when rendering the PCB for the nice!view display. Use the ${VAR_NAME} syntax to point to
//      a KiCad configured path.
//    niceview_3dmodel_xyz_offset: default is [0, 0, 0]
//      xyz offset (in mm), used to adjust the position of the nice!view 3d model
//      relative to the footprint.
//    niceview_3dmodel_xyz_scale: default is [1, 1, 1]
//      xyz scale, used to adjust the size of the nice!view 3d model relative to its
//      original size.
//    niceview_3dmodel_xyz_rotation: default is [0, 0, 0]
//      xyz rotation (in degrees), used to adjust the orientation of the nice!view 3d
//      model relative to the footprint.
//    pin_socket_3dmodel_filename: default is ''
//      Allows you to specify the path to a 3D model STEP or WRL file for pin socket to be
//      used when rendering the PCB. Use the ${VAR_NAME} syntax to point to
//      a KiCad configured path.
//    pin_socket_3dmodel_xyz_offset: default is [0, 0, 0]
//      xyz offset (in mm), used to adjust the position of the pin socket 3d model
//      relative to the footprint.
//    pin_socket_3dmodel_xyz_scale: default is [1, 1, 1]
//      xyz scale, used to adjust the size of the pin socket 3d model relative to its
//      original size.
//    pin_socket_3dmodel_xyz_rotation: default is [0, 0, 0]
//      xyz rotation (in degrees), used to adjust the orientation of the pin socket 3d
//      model relative to the footprint.
//    pin_header_3dmodel_filename: default is ''
//      Allows you to specify the path to a 3D model STEP or WRL file for pin header to be
//      used when rendering the PCB. Use the ${VAR_NAME} syntax to point to
//      a KiCad configured path.
//    pin_header_3dmodel_xyz_offset: default is [0, 0, 0]
//      xyz offset (in mm), used to adjust the position of the pin header 3d model
//      relative to the footprint.
//    pin_header_3dmodel_xyz_scale: default is [1, 1, 1]
//      xyz scale, used to adjust the size of the pin header 3d model relative to its
//      original size.
//    pin_header_3dmodel_xyz_rotation: default is [0, 0, 0]
//      xyz rotation (in degrees), used to adjust the orientation of the pin header 3d
//      model relative to the footprint.
//
// @ceoloide's improvements:
//  - Added support for traces
//  - Upgraded to KiCad 8 format
//  - Make silkscreen and courtyard optional
//  - Added ability to move labels and jumpers independently
//
// # Placement and soldering of jumpers
//
// The reversible footprint is meant to be used with jumpers on the
// OPPOSITE side of where the nice!view is installed. The silkscreen
// labels will also match the board when read on the opposite side.
// This is to have all jumpers and components to solder on the same
// side, and be able to read the correct labels of the display to do
// tests with a multimeter.
//
// @nidhishs' improvements:
// - Added 3D model support

module.exports = {
  params: {
    designator: 'DISP',
    side: 'F',
    reversible: false,
    include_traces: true,
    gnd_trace_width: 0.25,
    signal_trace_width: 0.25,
    invert_jumpers_position: false,
    invert_labels_position: false,
    include_silkscreen: true,
    include_labels: true,
    include_courtyard: true,
    niceview_3dmodel_filename: '',
    niceview_3dmodel_xyz_offset: [0, 0, 0],
    niceview_3dmodel_xyz_rotation: [0, 0, 0],
    niceview_3dmodel_xyz_scale: [1, 1, 1],
    pin_socket_3dmodel_filename: '',
    pin_socket_3dmodel_xyz_offset: [0, 0, 0],
    pin_socket_3dmodel_xyz_rotation: [0, 0, 0],
    pin_socket_3dmodel_xyz_scale: [1, 1, 1],
    pin_header_3dmodel_filename: '',
    pin_header_3dmodel_xyz_offset: [0, 0, 0],
    pin_header_3dmodel_xyz_rotation: [0, 0, 0],
    pin_header_3dmodel_xyz_scale: [1, 1, 1],
    MOSI: { type: 'net', value: 'MOSI' },
    SCK: { type: 'net', value: 'SCK' },
    VCC: { type: 'net', value: 'VCC' },
    GND: { type: 'net', value: 'GND' },
    CS: { type: 'net', value: 'CS' },
  },
  body: p => {
    let dst_nets = [
      p.MOSI,
      p.SCK,
      p.VCC,
      p.GND,
      p.CS,
    ];

    let local_nets = [
      p.local_net("1"),
      p.local_net("2"),
      p.VCC,
      p.local_net("4"),
      p.local_net("5"),
    ];

    if (p.reversible || p.side == "B") {
      dst_nets = dst_nets.slice().reverse();
    }
    let socket_nets = p.reversible ? local_nets : dst_nets;

    let jumpers_offset = 0;
    let labels_offset = 3.75;
    let label_vcc_offset = 3.75;

    let jumpers_front_top = dst_nets;
    let jumpers_front_bottom = local_nets;
    let jumpers_back_top = dst_nets;
    let jumpers_back_bottom = local_nets.slice().reverse();

    if (p.invert_labels_position) {
      if(p.reversible && !p.invert_jumpers_position) {
          label_vcc_offset = 0;
          labels_offset = -1.62;
      } else {
        label_vcc_offset = 0;
        labels_offset = label_vcc_offset;
      }
    } else {
      if(p.reversible && p.invert_jumpers_position) {
        labels_offset = 1.62 + label_vcc_offset;
      }
    }
    if (p.invert_jumpers_position) {
      jumpers_offset = 4.4;

      jumpers_front_top = local_nets;
      jumpers_front_bottom = dst_nets;
      jumpers_back_top = local_nets.slice().reverse();
      jumpers_back_bottom = dst_nets;
    }

    const top = `
  (footprint "ceoloide:display_nice_view"
    (layer ${p.side}.Cu)
    ${p.at /* parametric position */}
    (property "Reference" "${p.ref}"
      (at 0 20 ${p.r})
      (layer "${p.side}.SilkS")
      ${p.ref_hide}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (attr exclude_from_pos_files exclude_from_bom)
    `
    const front_silkscreen = `
    (fp_line (start -6.41 15.37) (end -6.41 18.03) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 6.41 18.03) (end -6.41 18.03) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 6.41 15.37) (end 6.41 18.03) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 6.41 15.37) (end -6.41 15.37) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    `
    const front_courtyard = `
    (fp_line (start 6.88 14.9) (end 6.88 18.45) (layer "F.CrtYd") (stroke (width 0.15) (type solid)))
    (fp_line (start 6.88 18.45) (end -6.82 18.45) (layer "F.CrtYd") (stroke (width 0.15) (type solid)))
    (fp_line (start -6.82 18.45) (end -6.82 14.9) (layer "F.CrtYd") (stroke (width 0.15) (type solid)))
    (fp_line (start -6.82 14.9) (end 6.88 14.9) (layer "F.CrtYd") (stroke (width 0.15) (type solid)))
    `

    const front_jumpers = `
    (pad "14" smd rect (at -5.08 ${14.05 + jumpers_offset} ${90 + p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${jumpers_front_top[0].str})
    (pad "15" smd rect (at -2.54 ${14.05 + jumpers_offset} ${90 + p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${jumpers_front_top[1].str})
    (pad "16" smd rect (at 2.54 ${14.05 + jumpers_offset} ${90 + p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${jumpers_front_top[3].str})
    (pad "17" smd rect (at 5.08 ${14.05 + jumpers_offset} ${90 + p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${jumpers_front_top[4].str})

    (pad "10" smd rect (at -5.08 ${14.95 + jumpers_offset} ${90 + p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${jumpers_front_bottom[0].str})
    (pad "11" smd rect (at -2.54 ${14.95 + jumpers_offset} ${90 + p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${jumpers_front_bottom[1].str})
    (pad "12" smd rect (at 2.54 ${14.95 + jumpers_offset} ${90 + p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${jumpers_front_bottom[3].str})
    (pad "13" smd rect (at 5.08 ${14.95 + jumpers_offset} ${90 + p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${jumpers_front_bottom[4].str})
    `

    const back_silkscreen = `
    (fp_line (start 6.41 15.37) (end 6.41 18.03) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 6.41 15.37) (end -6.41 15.37) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 6.41 18.03) (end -6.41 18.03) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -6.41 15.37) (end -6.41 18.03) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    `

    const back_courtyard = `
    (fp_line (start 6.88 14.9) (end 6.88 18.45) (layer "B.CrtYd") (stroke (width 0.15) (type solid)))
    (fp_line (start 6.88 18.45) (end -6.82 18.45) (layer "B.CrtYd") (stroke (width 0.15) (type solid)))
    (fp_line (start -6.82 18.45) (end -6.82 14.9) (layer "B.CrtYd") (stroke (width 0.15) (type solid)))
    (fp_line (start -6.82 14.9) (end 6.88 14.9) (layer "B.CrtYd") (stroke (width 0.15) (type solid)))
    `

    const back_jumpers = `
    (pad "24" smd rect (at 5.08 ${14.05 + jumpers_offset} ${270 + p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${jumpers_back_top[0].str})
    (pad "25" smd rect (at 2.54 ${14.05 + jumpers_offset} ${270 + p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${jumpers_back_top[1].str})
    (pad "26" smd rect (at -2.54 ${14.05 + jumpers_offset} ${270 + p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${jumpers_back_top[3].str})
    (pad "27" smd rect (at -5.08 ${14.05 + jumpers_offset} ${270 + p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${jumpers_back_top[4].str})

    (pad "20" smd rect (at 5.08 ${14.95 + jumpers_offset} ${270 + p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${jumpers_back_bottom[0].str})
    (pad "21" smd rect (at 2.54 ${14.95 + jumpers_offset} ${270 + p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${jumpers_back_bottom[1].str})
    (pad "22" smd rect (at -2.54 ${14.95 + jumpers_offset} ${270 + p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${jumpers_back_bottom[3].str})
    (pad "23" smd rect (at -5.08 ${14.95 + jumpers_offset} ${270 + p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${jumpers_back_bottom[4].str})
    `

    const silkscreen_labels_front = `
    (fp_text user "${dst_nets[0].name}" (at -5.08 ${14.75 + labels_offset} ${90 + p.r}) (unlocked yes) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify ${!p.invert_labels_position  ? "right" : "left"}))
    )
    (fp_text user "${dst_nets[1].name}" (at -2.48 ${14.75 + labels_offset} ${90 + p.r}) (unlocked yes) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify ${!p.invert_labels_position ? "right" : "left"}))
    )
    (fp_text user "${dst_nets[2].name}" (at 0.15 ${14.75 + label_vcc_offset} ${90 + p.r}) (unlocked yes) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify ${!p.invert_labels_position ? "right" : "left"}))
    )
    (fp_text user "${dst_nets[3].name}" (at 2.62 ${14.75 + labels_offset} ${90 + p.r}) (unlocked yes) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify ${!p.invert_labels_position ? "right" : "left"}))
    )
    (fp_text user "${dst_nets[4].name}" (at 5.12 ${14.75 + labels_offset} ${90 + p.r}) (unlocked yes) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify ${!p.invert_labels_position ? "right" : "left"}))
    )
    `

    const silkscreen_labels_back = `
    (fp_text user "${p.reversible ? dst_nets[0].name : dst_nets[4].name}" (at 5.22 ${14.75 + labels_offset} ${90 + p.r}) (unlocked yes) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify ${!p.invert_labels_position ? "left" : "right"} mirror))
    )
    (fp_text user "${p.reversible ? dst_nets[1].name : dst_nets[3].name}" (at 2.72 ${14.75 + labels_offset} ${90 + p.r}) (unlocked yes) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify ${!p.invert_labels_position ? "left" : "right"} mirror))
    )
    (fp_text user "${p.reversible ? dst_nets[2].name : dst_nets[2].name}" (at 0.15 ${14.75 + label_vcc_offset} ${90 + p.r}) (unlocked yes) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify ${!p.invert_labels_position ? "left" : "right"} mirror))
    )
    (fp_text user "${p.reversible ? dst_nets[3].name : dst_nets[1].name}" (at -2.38 ${14.75 + labels_offset} ${90 + p.r}) (unlocked yes) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify ${!p.invert_labels_position ? "left" : "right"} mirror))
    )
    (fp_text user "${p.reversible ? dst_nets[4].name : dst_nets[0].name}" (at -4.98 ${14.75 + labels_offset} ${90 + p.r}) (unlocked yes) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify ${!p.invert_labels_position ? "left" : "right"} mirror))
    )
    `

    // Place the independently authored models on their shared five-pin row.
    const socket_height = 8.5;
    const display_height = socket_height + 1.8;
    const header_height = socket_height - 2;
    const back = p.side === 'B';
    const display_offset = p.niceview_3dmodel_xyz_offset || (back ? [7, 18, display_height] : [-7, -18, display_height]);
    const display_rotation = p.niceview_3dmodel_xyz_rotation || [0, 0, back ? 180 : 0];
    const header_offset = p.pin_header_3dmodel_xyz_offset || (back ? [-0.0124, 16.732, header_height] : [0.0124, -16.732, header_height]);
    const header_rotation = p.pin_header_3dmodel_xyz_rotation || [0, 0, back ? 90 : -90];
    const socket_offset = p.pin_socket_3dmodel_xyz_offset || (back ? [5.08, 16.7, 0] : [-5.08, -16.7, 0]);
    const socket_rotation = p.pin_socket_3dmodel_xyz_rotation || [0, 0, back ? 90 : -90];
    const niceview_3dmodel = `
    (model ${JSON.stringify(p.niceview_3dmodel_filename)}
      (offset (xyz ${display_offset[0]} ${display_offset[1]} ${display_offset[2]}))
      (scale (xyz ${p.niceview_3dmodel_xyz_scale[0]} ${p.niceview_3dmodel_xyz_scale[1]} ${p.niceview_3dmodel_xyz_scale[2]}))
      (rotate (xyz ${display_rotation[0]} ${display_rotation[1]} ${display_rotation[2]}))
    )
    `

    const pin_socket_3dmodel = `
    (model ${JSON.stringify(p.pin_socket_3dmodel_filename)}
      (offset (xyz ${socket_offset[0]} ${socket_offset[1]} ${socket_offset[2]}))
      (scale (xyz ${p.pin_socket_3dmodel_xyz_scale[0]} ${p.pin_socket_3dmodel_xyz_scale[1]} ${p.pin_socket_3dmodel_xyz_scale[2]}))
      (rotate (xyz ${socket_rotation[0]} ${socket_rotation[1]} ${socket_rotation[2]}))
    )
    `

    const pin_header_3dmodel = `
    (model ${JSON.stringify(p.pin_header_3dmodel_filename)}
      (offset (xyz ${header_offset[0]} ${header_offset[1]} ${header_offset[2]}))
      (scale (xyz ${p.pin_header_3dmodel_xyz_scale[0]} ${p.pin_header_3dmodel_xyz_scale[1]} ${p.pin_header_3dmodel_xyz_scale[2]}))
      (rotate (xyz ${header_rotation[0]} ${header_rotation[1]} ${header_rotation[2]}))
    )
    `

    const bottom = `
    (pad "1" thru_hole oval (at -5.08 16.7 ${270 + p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${socket_nets[0].str})
    (pad "2" thru_hole oval (at -2.54 16.7 ${270 + p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${socket_nets[1].str})
    (pad "3" thru_hole oval (at 0 16.7 ${270 + p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${socket_nets[2].str})
    (pad "4" thru_hole oval (at 2.54 16.7 ${270 + p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${socket_nets[3].str})
    (pad "5" thru_hole circle (at 5.08 16.7 ${270 + p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${socket_nets[4].str})

    (fp_line (start 5.4 13.4) (end 5.4 -11.9) (layer Dwgs.User) (stroke (width 0.15) (type solid)))
    (fp_line (start -5.4 13.4) (end -5.4 -11.9) (layer Dwgs.User) (stroke (width 0.15) (type solid)))
    (fp_line (start 5.4 -11.9) (end -5.4 -11.9) (layer Dwgs.User) (stroke (width 0.15) (type solid)))
    (fp_line (start -5.4 13.4) (end 5.4 13.4) (layer Dwgs.User) (stroke (width 0.15) (type solid)))

    (fp_line (start -7 -18) (end 7 -18) (layer Dwgs.User) (stroke (width 0.15) (type solid)))
    (fp_line (start 7 18) (end -7 18) (layer Dwgs.User) (stroke (width 0.15) (type solid)))
    (fp_line (start -7 18) (end -7 -18) (layer Dwgs.User) (stroke (width 0.15) (type solid)))
    (fp_line (start 7 18) (end 7 -18) (layer Dwgs.User) (stroke (width 0.15) (type solid)))
  )
    `

    const traces_bottom = `
  (segment (start ${p.eaxy(-5.08, 16.7)}) (end ${p.eaxy(-5.08, 18.45)}) (width ${p.signal_trace_width}) (layer "F.Cu") (net ${socket_nets[0].index}))
  (segment (start ${p.eaxy(-2.54, 16.7)}) (end ${p.eaxy(-2.54, 18.45)}) (width ${p.gnd_trace_width}) (layer "F.Cu") (net ${socket_nets[1].index}))
  (segment (start ${p.eaxy(2.54, 16.7)}) (end ${p.eaxy(2.54, 18.45)}) (width ${p.signal_trace_width}) (layer "F.Cu") (net ${socket_nets[3].index}))
  (segment (start ${p.eaxy(5.08, 16.7)}) (end ${p.eaxy(5.08, 18.45)}) (width ${p.signal_trace_width}) (layer "F.Cu") (net ${socket_nets[4].index}))
  (segment (start ${p.eaxy(-5.08, 16.7)}) (end ${p.eaxy(-5.08, 18.45)}) (width ${p.signal_trace_width}) (layer "B.Cu") (net ${socket_nets[0].index}))
  (segment (start ${p.eaxy(-2.54, 16.7)}) (end ${p.eaxy(-2.54, 18.45)}) (width ${p.signal_trace_width}) (layer "B.Cu") (net ${socket_nets[1].index}))
  (segment (start ${p.eaxy(2.54, 16.7)}) (end ${p.eaxy(2.54, 18.45)}) (width ${p.gnd_trace_width}) (layer "B.Cu") (net ${socket_nets[3].index}))
  (segment (start ${p.eaxy(5.08, 16.7)}) (end ${p.eaxy(5.08, 18.45)}) (width ${p.signal_trace_width}) (layer "B.Cu") (net ${socket_nets[4].index}))
    `

    const traces_top = `
  (segment (start ${p.eaxy(-5.08, 16.7)}) (end ${p.eaxy(-5.08, 14.95)}) (width ${p.signal_trace_width}) (layer "F.Cu") (net ${socket_nets[0].index}))
  (segment (start ${p.eaxy(-2.54, 16.7)}) (end ${p.eaxy(-2.54, 14.95)}) (width ${p.gnd_trace_width}) (layer "F.Cu") (net ${socket_nets[1].index}))
  (segment (start ${p.eaxy(2.54, 16.7)}) (end ${p.eaxy(2.54, 14.95)}) (width ${p.signal_trace_width}) (layer "F.Cu") (net ${socket_nets[3].index}))
  (segment (start ${p.eaxy(5.08, 16.7)}) (end ${p.eaxy(5.08, 14.95)}) (width ${p.signal_trace_width}) (layer "F.Cu") (net ${socket_nets[4].index}))
  (segment (start ${p.eaxy(-5.08, 16.7)}) (end ${p.eaxy(-5.08, 14.95)}) (width ${p.signal_trace_width}) (layer "B.Cu") (net ${socket_nets[0].index}))
  (segment (start ${p.eaxy(-2.54, 16.7)}) (end ${p.eaxy(-2.54, 14.95)}) (width ${p.signal_trace_width}) (layer "B.Cu") (net ${socket_nets[1].index}))
  (segment (start ${p.eaxy(2.54, 16.7)}) (end ${p.eaxy(2.54, 14.95)}) (width ${p.gnd_trace_width}) (layer "B.Cu") (net ${socket_nets[3].index}))
  (segment (start ${p.eaxy(5.08, 16.7)}) (end ${p.eaxy(5.08, 14.95)}) (width ${p.signal_trace_width}) (layer "B.Cu") (net ${socket_nets[4].index}))
    `

    let final = top;

    if (p.side == "F" || p.reversible) {
      if (p.include_silkscreen) {
        final += front_silkscreen;
        if (p.include_labels) final += silkscreen_labels_front;
      }
      if (p.include_courtyard) final += front_courtyard;
    }
    if (p.side == "B" || p.reversible) {
      if (p.include_silkscreen) {
        final += back_silkscreen;
        if (p.include_labels) final += silkscreen_labels_back;
      }
      if (p.include_courtyard) final += back_courtyard;
    }
    if (p.reversible) {
      final += front_jumpers;
      final += back_jumpers;
    }
    if (p.niceview_3dmodel_filename) {
      final += niceview_3dmodel;
    }
    if (p.pin_socket_3dmodel_filename) {
      final += pin_socket_3dmodel;
    }
    if (p.pin_header_3dmodel_filename) {
      final += pin_header_3dmodel;
    }
    final += bottom;
    if (p.include_traces && p.reversible) {
      if (p.invert_jumpers_position) {
        final += traces_bottom;
      } else {
        final += traces_top;
      }
    }
    return final;
  }
}
;module.exports = ((original) => {
    const defaults = {"niceview_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/Nice_View.step","pin_header_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/PinHeader_2.54mm_1x-5.step","pin_socket_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/kicad/PinSocket_1x05_P2.54mm_Vertical.step","niceview_3dmodel_xyz_offset":"","niceview_3dmodel_xyz_rotation":"","pin_header_3dmodel_xyz_offset":"","pin_header_3dmodel_xyz_rotation":"","pin_socket_3dmodel_xyz_offset":"","pin_socket_3dmodel_xyz_rotation":""};
    const bossVariant = null;
    const chocV2 = "${KIPRJMOD}/models/boardstudio/koktoh/Choc_V2_Red.step";
    return {
      ...original,
      params: {...original.params, ...defaults},
      body: p => {
        // The bundled extension crosses the PCB with a 5 mm outer diameter.
        const extensionDiameter = 5;
        if (false && p.drill < extensionDiameter
          && p.tp_extension_3dmodel_filename === defaults.tp_extension_3dmodel_filename
          && !p.tp_extension_3dmodel_xyz_scale
          && !p.tp_extension_3dmodel_xyz_rotation
          && !p.tp_extension_3dmodel_xyz_offset) {
          throw new Error('The bundled trackpoint extension requires a center drill of at least 5 mm. Select a compatible extension model or drill size.');
        }
        if (false && !p.choc_v1_support) {
          p = {...p};
          if (!p.choc_v2_support) {
            for (const key of Object.keys(defaults)) {
              if (p[key] === defaults[key]) { p[key] = ''; }
            }
          } else {
            if (p.switch_3dmodel_filename === defaults.switch_3dmodel_filename) {
              const automatic = !p.switch_3dmodel_xyz_rotation && !p.switch_3dmodel_xyz_offset
                && p.switch_3dmodel_xyz_scale.every(value => value === 1);
              const postDiameter = 4.8;
              if (automatic && (!p.include_stabilizer_pad || p.oval_stabilizer_pad
                || (p.center_hole_diameter > 0 && p.center_hole_diameter < postDiameter))) {
                throw new Error('The bundled Choc V2 model requires the round stabilizer hole and a center drill of at least 4.8 mm. Use a matching model or transform for modified hardware.');
              }
              p.switch_3dmodel_filename = chocV2;
              p.switch_3dmodel_xyz_rotation ||= p.side === 'F' ? [180, 0, 0] : [0, 180, 0];
              p.switch_3dmodel_xyz_offset ||= [0, 0, -p.pcb_thickness];
            }
            if (p.keycap_3dmodel_filename === defaults.keycap_3dmodel_filename) {
              p.keycap_3dmodel_filename = '';
            }
          }
        }
        if (bossVariant && p.include_bosses && p.reset_switch_3dmodel_filename === defaults.reset_switch_3dmodel_filename) {
          p = {...p, reset_switch_3dmodel_filename: bossVariant};
        }
        return original.body(p);
      }
    };
  })(module.exports);

return module.exports; })(),
  "ceoloide/display_ssd1306": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Copyright (c) 2023 Marco Massarelli
//
// SPDX-License-Identifier: CC-BY-NC-SA-4.0
//
// To view a copy of this license, visit http://creativecommons.org/licenses/by-nc-sa/4.0/
//
// Author: @ceoloide
//
// Description:
//  Reversible footprint for SSD1306 OLED display. Includes an outline of the
//  display to make positioning easier.
//
//  In its default configuration, jumper pads are positioned above the pins, when the
//  component is oriented verically and pointing upwards, or left of the pins, when oriented
//  horizontally and oriented leftward. Jumper pads position can be inverted with a parameter.
//
//  In its default configuration, labels are positioned below the pins, when the
//  component is oriented verically and pointing upwards, or right of the pins, when oriented
//  horizontally and oriented leftward. Labels position can be inverted with a parameter.
//
// Params:
//    side: default is F for Front
//      the side on which to place the single-side footprint and designator, either F or B
//    reversible: default is false
//      if true, the footprint will be placed on both sides so that the PCB can be
//      reversible
//    include_traces: default is true
//      if true it will include traces that connect the jumper pads to the vias
//      and the through-holes for the MCU
//    gnd_trace_width: default is 0.250mm
//      allows to override the GND trace width. Not recommended to go below 0.25mm (JLCPC
//      min is 0.127mm).
//    signal_trace_width: default is 0.250mm
//      allows to override the trace width that connects the jumper pads to the SDA, and SCL.
//      Not recommended to go below 0.15mm (JLCPC min is 0.127mm).
//    invert_jumpers_position default is false
//      allows to change the position of the jumper pads, from their default to the opposite
//      side of the pins. See the description above for more details.
//    invert_labels_position default is false
//      allows to change the position of the labels, from their default to the north
//      side of the pins. See the description above for more details.
//    include_silkscreen: default is true
//      if true it will include the silkscreen layer.
//    include_labels default is true
//      if true and Silkscreen layer is included, it will include the pin labels. The labels
//      will match the *opposite* side of the board when the footprint is set to be reversible, 
//      since they are meant to match the solder jumpers behavior and aid testing.
//    include_courtyard: default is true
//      if true it will include a courtyard outline around the pin header.
//
// # Placement and soldering of jumpers
//
// The reversible footprint is meant to be used with jumpers on the
// OPPOSITE side of where the display is installed. The silkscreen
// labels will also match the board when read on the opposite side.
// This is to have all jumpers and components to solder on the same
// side, and be able to read the correct labels of the display to do
// tests with a multimeter.

module.exports = {
  params: {
    designator: 'DISP',
    side: 'F',
    reversible: false,
    include_traces: true,
    gnd_trace_width: 0.25,
    signal_trace_width: 0.25,
    invert_jumpers_position: false,
    invert_labels_position: false,
    include_silkscreen: true,
    include_labels: true,
    include_courtyard: true,
    display_3dmodel_filename: '',
    display_3dmodel_xyz_scale: [1, 1, 1],
    display_3dmodel_xyz_rotation: '',
    display_3dmodel_xyz_offset: '',
    SDA: { type: 'net', value: 'SDA' },
    SCL: { type: 'net', value: 'SCL' },
    VCC: { type: 'net', value: 'VCC' },
    GND: { type: 'net', value: 'GND' },
  },
  body: p => {
    let dst_nets = [
      p.SDA,
      p.SCL,
      p.VCC,
      p.GND,
    ];

    let local_nets = [
      p.local_net("1"),
      p.local_net("2"),
      p.local_net("3"),
      p.local_net("4"),
    ];

    if (p.reversible || p.side == "B") {
      dst_nets = dst_nets.slice().reverse();
    }
    let socket_nets = p.reversible ? local_nets : dst_nets;

    let jumpers_offset = 0;
    let labels_offset = 3.75;

    let jumpers_front_top = dst_nets;
    let jumpers_front_bottom = local_nets;
    let jumpers_back_top = dst_nets;
    let jumpers_back_bottom = local_nets.slice().reverse();

    if (p.invert_labels_position) {
      if(p.reversible && !p.invert_jumpers_position) {
          labels_offset = -1.62;
      } else {
        labels_offset = 0;
      }
    } else {
      if(p.reversible && p.invert_jumpers_position) {
        labels_offset = 1.62 + 3.75;
      }
    }
    if (p.invert_jumpers_position) {
      jumpers_offset = 4.4;

      jumpers_front_top = local_nets;
      jumpers_front_bottom = dst_nets;
      jumpers_back_top = local_nets.slice().reverse();
      jumpers_back_bottom = dst_nets;
    }

    const top = `
  (footprint "ceoloide:display_ssd1306"
    (layer "${p.side}.Cu")
    ${p.at}
    (property "Reference" "${p.ref}"
      (at 0 20 ${p.r})
      (layer "${p.side}.SilkS")
      ${p.ref_hide}
      (effects (font (size 1 1) (thickness 0.15)))
    )
		(attr exclude_from_pos_files exclude_from_bom)
    `
    
    const front_silkscreen = `
    (fp_line (start 5.14 15.37) (end 5.14 18.03) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 5.14 15.37) (end -5.14 15.37) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 5.14 18.03) (end -5.14 18.03) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -5.14 15.37) (end -5.14 18.03) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    `
    
    const front_courtyard = `
    (fp_line (start 5.61 14.9) (end 5.61 18.45) (layer "F.CrtYd") (stroke (width 0.15) (type solid)))
    (fp_line (start 5.61 18.45) (end -5.61 18.45) (layer "F.CrtYd") (stroke (width 0.15) (type solid)))
    (fp_line (start -5.61 18.45) (end -5.61 14.9) (layer "F.CrtYd") (stroke (width 0.15) (type solid)))
    (fp_line (start -5.61 14.9) (end 5.61 14.9) (layer "F.CrtYd") (stroke (width 0.15) (type solid)))
    `
    
    const front_fab = `
    (fp_line (start -3.77 -11.14) (end -3.77 11.24) (layer "F.Fab") (stroke (width 0.15) (type solid)))
    (fp_line (start 1.75 -11.14) (end 1.75 11.24) (layer "F.Fab") (stroke (width 0.15) (type solid)))
    (fp_line (start -3.77 -11.14) (end 1.75 -11.14) (layer "F.Fab") (stroke (width 0.15) (type solid)))
    (fp_line (start -3.77 11.24) (end 1.75 11.24) (layer "F.Fab") (stroke (width 0.15) (type solid)))
    `

    const front_jumpers = `
    (pad "14" smd rect (at -3.81 ${14.05 + jumpers_offset} ${90 + p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${jumpers_front_top[0].str})
    (pad "15" smd rect (at -1.27 ${14.05 + jumpers_offset} ${90 + p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${jumpers_front_top[1].str})
    (pad "16" smd rect (at 1.27 ${14.05 + jumpers_offset} ${90 + p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${jumpers_front_top[2].str})
    (pad "17" smd rect (at 3.81 ${14.05 + jumpers_offset} ${90 + p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${jumpers_front_top[3].str})

    (pad "10" smd rect (at -3.81 ${14.95 + jumpers_offset} ${90 + p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${jumpers_front_bottom[0].str})
    (pad "11" smd rect (at -1.27 ${14.95 + jumpers_offset} ${90 + p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${jumpers_front_bottom[1].str})
    (pad "12" smd rect (at 1.27 ${14.95 + jumpers_offset} ${90 + p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${jumpers_front_bottom[2].str})
    (pad "13" smd rect (at 3.81 ${14.95 + jumpers_offset} ${90 + p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${jumpers_front_bottom[3].str})
    `

    const back_silkscreen = `
    (fp_line (start 5.14 15.37) (end 5.14 18.03) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 5.14 15.37) (end -5.14 15.37) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 5.14 18.03) (end -5.14 18.03) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -5.14 15.37) (end -5.14 18.03) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    `

    const back_courtyard = `
    (fp_line (start 5.61 14.9) (end 5.61 18.45) (layer "B.CrtYd") (stroke (width 0.15) (type solid)))
    (fp_line (start 5.61 18.45) (end -5.61 18.45) (layer "B.CrtYd") (stroke (width 0.15) (type solid)))
    (fp_line (start -5.61 18.45) (end -5.61 14.9) (layer "B.CrtYd") (stroke (width 0.15) (type solid)))
    (fp_line (start -5.61 14.9) (end 5.61 14.9) (layer "B.CrtYd") (stroke (width 0.15) (type solid)))
    `
    
    const back_fab = `
    (fp_line (start 3.77 -11.14) (end 3.77 11.24) (layer "B.Fab") (stroke (width 0.15) (type solid)))
    (fp_line (start -1.75 -11.14) (end -1.75 11.24) (layer "B.Fab") (stroke (width 0.15) (type solid)))
    (fp_line (start 3.77 -11.14) (end -1.75 -11.14) (layer "B.Fab") (stroke (width 0.15) (type solid)))
    (fp_line (start 3.77 11.24) (end -1.75 11.24) (layer "B.Fab") (stroke (width 0.15) (type solid)))
    `

    const back_jumpers = `
    (pad "24" smd rect (at 3.81 ${14.05 + jumpers_offset} ${270 + p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${jumpers_back_top[0].str})
    (pad "25" smd rect (at 1.27 ${14.05 + jumpers_offset} ${270 + p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${jumpers_back_top[1].str})
    (pad "26" smd rect (at -1.27 ${14.05 + jumpers_offset} ${270 + p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${jumpers_back_top[2].str})
    (pad "27" smd rect (at -3.81 ${14.05 + jumpers_offset} ${270 + p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${jumpers_back_top[3].str})

    (pad "20" smd rect (at 3.81 ${14.95 + jumpers_offset} ${270 + p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${jumpers_back_bottom[0].str})
    (pad "21" smd rect (at 1.27 ${14.95 + jumpers_offset} ${270 + p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${jumpers_back_bottom[1].str})
    (pad "22" smd rect (at -1.27 ${14.95 + jumpers_offset} ${270 + p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${jumpers_back_bottom[2].str})
    (pad "23" smd rect (at -3.81 ${14.95 + jumpers_offset} ${270 + p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${jumpers_back_bottom[3].str})
    `

    const silkscreen_labels_front = `
    (fp_text user "${dst_nets[0].name}" (at -3.81 ${14.75 + labels_offset} ${90 + p.r}) (unlocked yes) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify ${!p.invert_labels_position  ? "right" : "left"}))
    )
    (fp_text user "${dst_nets[1].name}" (at -1.27 ${14.75 + labels_offset} ${90 + p.r}) (unlocked yes) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify ${!p.invert_labels_position ? "right" : "left"}))
    )
    (fp_text user "${dst_nets[2].name}" (at 1.27 ${14.75 + labels_offset} ${90 + p.r}) (unlocked yes) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify ${!p.invert_labels_position ? "right" : "left"}))
    )
    (fp_text user "${dst_nets[3].name}" (at 3.81 ${14.75 + labels_offset} ${90 + p.r}) (unlocked yes) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify ${!p.invert_labels_position ? "right" : "left"}))
    )
    `
    const silkscreen_labels_back = `
    (fp_text user "${p.reversible ? dst_nets[0].name : dst_nets[3].name}" (at 3.81 ${14.75 + labels_offset} ${90 + p.r}) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify ${!p.invert_labels_position ? "left" : "right"} mirror))
    )
    (fp_text user "${p.reversible ? dst_nets[1].name : dst_nets[2].name}" (at 1.27 ${14.75 + labels_offset} ${90 + p.r}) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify ${!p.invert_labels_position ? "left" : "right"} mirror))
    )
    (fp_text user "${p.reversible ? dst_nets[2].name : dst_nets[1].name}" (at -1.27 ${14.75 + labels_offset} ${90 + p.r}) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify ${!p.invert_labels_position ? "left" : "right"} mirror))
    )
    (fp_text user "${p.reversible ? dst_nets[3].name : dst_nets[0].name}" (at -3.81 ${14.75 + labels_offset} ${90 + p.r}) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify ${!p.invert_labels_position ? "left" : "right"} mirror))
    )
    `

    const bottom = `
    (pad "1" thru_hole oval (at -3.81 16.7 ${270 + p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${socket_nets[0].str})
    (pad "2" thru_hole oval (at -1.27 16.7 ${270 + p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${socket_nets[1].str})
    (pad "3" thru_hole oval (at 1.27 16.7 ${270 + p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${socket_nets[2].str})
    (pad "4" thru_hole oval (at 3.81 16.7 ${270 + p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${socket_nets[3].str})

    (fp_line (start -6.00 -19.70) (end -6.00 18.30) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start 6.00 -19.70) (end 6.00 18.30) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -6.00 -19.70) (end 6.00 -19.70) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -6.00 -17.45) (end 6.00 -17.45) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -6.00 18.30) (end 6.00 18.30) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -6.00 12.55) (end 6.00 12.55) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
  )
    `

    const traces_bottom = `
  (segment (start ${p.eaxy(-3.81, 16.7)}) (end ${p.eaxy(-3.81, 18.45)}) (width ${p.gnd_trace_width}) (layer "F.Cu") (net ${socket_nets[0].index}))
  (segment (start ${p.eaxy(-1.27, 16.7)}) (end ${p.eaxy(-1.27, 18.45)}) (width ${p.signal_trace_width}) (layer "F.Cu") (net ${socket_nets[1].index}))
  (segment (start ${p.eaxy(1.27, 16.7)}) (end ${p.eaxy(1.27, 18.45)}) (width ${p.signal_trace_width}) (layer "F.Cu") (net ${socket_nets[2].index}))
  (segment (start ${p.eaxy(3.81, 16.7)}) (end ${p.eaxy(3.81, 18.45)}) (width ${p.signal_trace_width}) (layer "F.Cu") (net ${socket_nets[3].index}))
  (segment (start ${p.eaxy(-3.81, 16.7)}) (end ${p.eaxy(-3.81, 18.45)}) (width ${p.signal_trace_width}) (layer "B.Cu") (net ${socket_nets[0].index}))
  (segment (start ${p.eaxy(-1.27, 16.7)}) (end ${p.eaxy(-1.27, 18.45)}) (width ${p.signal_trace_width}) (layer "B.Cu") (net ${socket_nets[1].index}))
  (segment (start ${p.eaxy(1.27, 16.7)}) (end ${p.eaxy(1.27, 18.45)}) (width ${p.signal_trace_width}) (layer "B.Cu") (net ${socket_nets[2].index}))
  (segment (start ${p.eaxy(3.81, 16.7)}) (end ${p.eaxy(3.81, 18.45)}) (width ${p.gnd_trace_width}) (layer "B.Cu") (net ${socket_nets[3].index}))
    `

    const traces_top = `
  (segment (start ${p.eaxy(-3.81, 16.7)}) (end ${p.eaxy(-3.81, 14.95)}) (width ${p.gnd_trace_width}) (layer "F.Cu") (net ${socket_nets[0].index}))
  (segment (start ${p.eaxy(-1.27, 16.7)}) (end ${p.eaxy(-1.27, 14.95)}) (width ${p.signal_trace_width}) (layer "F.Cu") (net ${socket_nets[1].index}))
  (segment (start ${p.eaxy(1.27, 16.7)}) (end ${p.eaxy(1.27, 14.95)}) (width ${p.signal_trace_width}) (layer "F.Cu") (net ${socket_nets[2].index}))
  (segment (start ${p.eaxy(3.81, 16.7)}) (end ${p.eaxy(3.81, 14.95)}) (width ${p.signal_trace_width}) (layer "F.Cu") (net ${socket_nets[3].index}))
  (segment (start ${p.eaxy(-3.81, 16.7)}) (end ${p.eaxy(-3.81, 14.95)}) (width ${p.signal_trace_width}) (layer "B.Cu") (net ${socket_nets[0].index}))
  (segment (start ${p.eaxy(-1.27, 16.7)}) (end ${p.eaxy(-1.27, 14.95)}) (width ${p.signal_trace_width}) (layer "B.Cu") (net ${socket_nets[1].index}))
  (segment (start ${p.eaxy(1.27, 16.7)}) (end ${p.eaxy(1.27, 14.95)}) (width ${p.signal_trace_width}) (layer "B.Cu") (net ${socket_nets[2].index}))
  (segment (start ${p.eaxy(3.81, 16.7)}) (end ${p.eaxy(3.81, 14.95)}) (width ${p.gnd_trace_width}) (layer "B.Cu") (net ${socket_nets[3].index}))
    `

    let final = top;

    if (p.side == "F" || p.reversible) {
      final += front_fab;
      if (p.include_silkscreen) {
        final += front_silkscreen;
        if (p.include_labels) final += silkscreen_labels_front;
      }
      if (p.include_courtyard) final += front_courtyard;
    }
    if (p.side == "B" || p.reversible) {
      final += back_fab;
      if (p.include_silkscreen) {
        final += back_silkscreen;
        if (p.include_labels) final += silkscreen_labels_back;
      }
      if (p.include_courtyard) final += back_courtyard;
    }
    if (p.reversible) {
      final += front_jumpers;
      final += back_jumpers;
    }
    // The model origin is its header; the footprint origin is the display center.
    if (p.display_3dmodel_filename) {
      const rotation = p.display_3dmodel_xyz_rotation || [0, 0, p.side === 'B' ? 180 : 0];
      const offset = p.display_3dmodel_xyz_offset || [0, p.side === 'B' ? 16.7 : -16.7, 0];
      final += `
    (model ${JSON.stringify(p.display_3dmodel_filename)}
      (offset (xyz ${offset.join(' ')}))
      (scale (xyz ${p.display_3dmodel_xyz_scale.join(' ')}))
      (rotate (xyz ${rotation.join(' ')})))`;
    }
    final += bottom;
    if (p.include_traces && p.reversible) {
      if (p.invert_jumpers_position) {
        final += traces_bottom;
      } else {
        final += traces_top;
      }
    }
    return final;
  }
}
;module.exports = ((original) => {
    const defaults = {"display_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/foostan/OLED-Module-with-Pins.step"};
    const bossVariant = null;
    const chocV2 = "${KIPRJMOD}/models/boardstudio/koktoh/Choc_V2_Red.step";
    return {
      ...original,
      params: {...original.params, ...defaults},
      body: p => {
        // The bundled extension crosses the PCB with a 5 mm outer diameter.
        const extensionDiameter = 5;
        if (false && p.drill < extensionDiameter
          && p.tp_extension_3dmodel_filename === defaults.tp_extension_3dmodel_filename
          && !p.tp_extension_3dmodel_xyz_scale
          && !p.tp_extension_3dmodel_xyz_rotation
          && !p.tp_extension_3dmodel_xyz_offset) {
          throw new Error('The bundled trackpoint extension requires a center drill of at least 5 mm. Select a compatible extension model or drill size.');
        }
        if (false && !p.choc_v1_support) {
          p = {...p};
          if (!p.choc_v2_support) {
            for (const key of Object.keys(defaults)) {
              if (p[key] === defaults[key]) { p[key] = ''; }
            }
          } else {
            if (p.switch_3dmodel_filename === defaults.switch_3dmodel_filename) {
              const automatic = !p.switch_3dmodel_xyz_rotation && !p.switch_3dmodel_xyz_offset
                && p.switch_3dmodel_xyz_scale.every(value => value === 1);
              const postDiameter = 4.8;
              if (automatic && (!p.include_stabilizer_pad || p.oval_stabilizer_pad
                || (p.center_hole_diameter > 0 && p.center_hole_diameter < postDiameter))) {
                throw new Error('The bundled Choc V2 model requires the round stabilizer hole and a center drill of at least 4.8 mm. Use a matching model or transform for modified hardware.');
              }
              p.switch_3dmodel_filename = chocV2;
              p.switch_3dmodel_xyz_rotation ||= p.side === 'F' ? [180, 0, 0] : [0, 180, 0];
              p.switch_3dmodel_xyz_offset ||= [0, 0, -p.pcb_thickness];
            }
            if (p.keycap_3dmodel_filename === defaults.keycap_3dmodel_filename) {
              p.keycap_3dmodel_filename = '';
            }
          }
        }
        if (bossVariant && p.include_bosses && p.reset_switch_3dmodel_filename === defaults.reset_switch_3dmodel_filename) {
          p = {...p, reset_switch_3dmodel_filename: bossVariant};
        }
        return original.body(p);
      }
    };
  })(module.exports);

return module.exports; })(),
  "ceoloide/led_sk6812mini-e": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Copyright (c) 2023 Marco Massarelli
//
// SPDX-License-Identifier: MIT
//
// To view a copy of this license, visit https://opensource.org/license/mit/
//
// Author: @ceoloide
//
// Description:
//  Reversible footprint for "YS-SK6812mini-e" LEDs, to be used either as per-key lightning or
//  underglow. The footprint allows many customizations, including pre-defined traces to
//  simplify routing.
//
//  These LEDs are very tolerant of undervoltage, and are easy to solder thanks to the side
//  legs.
//
// Datasheet:
//  https://datasheet.lcsc.com/lcsc/2305101623_OPSCO-Optoelectronics-SK6812MINI-E_C5149201.pdf
//
// Nets:
//    P1: corresponds to VCC pin
//    P2: corresponds to Data-Out pin
//    P3: corresponds to GND pin
//    P4: corresponds to Data-In pin
//
// Params:
//    side: default is B for Back
//      the side on which to place the single-side footprint and designator, either F or B
//    reversible: default is false
//      if true, the footprint will be placed on both sides so that the PCB can be
//      reversible
//    reverse_mount: default is true (per-key LED)
//      if true, the pads will be oriented so that the LED shines through ther PCB, i.e.
//      when used for per-key LEDs. When set to false, the pads will match the datasheet
//      and assume the LED shines away from the PCB, i.e. when used as underglow. Note that
//      automated PCB assembly may not support both options depending on the component reel
//    include_traces_vias: default is true
//      if true it will include traces and vias to simplify routing when the footprint is
//      made reversible
//    signal_trace_width: default is 0.250mm
//      allows to override the trace width that connects the DIN / DOUT pads. Not recommended
//      to go below 0.15mm (JLCPC min is 0.127mm)
//    gnd_trace_width: default is 0.250mm
//      allows to override the GND trace width. Not recommended to go below 0.25mm (JLCPC
//      min is 0.127mm). Do not exceed 0.8mm to avoid clearance errors
//    vcc_trace_width: default is 0.250mm
//      allows to override the VCC trace width. Not recommended to go below 0.25mm (JLCPC
//      min is 0.127mm). Do not exceed 0.8mm to avoid clearance errors
//    via_size: default is 0.8
//      allows to define the size of the via. Not recommended below 0.56 (JLCPCB minimum),
//      or above 0.8 (KiCad default), to avoid overlap or DRC errors
//    via_drill: default is 0.4
//      allows to define the size of the drill. Not recommended below 0.3 (JLCPCB minimum),
//      or above 0.4 (KiCad default), to avoid overlap or DRC errors 
//    include_courtyard: default is true
//      if true it will include the part courtyard
//    include_keepout: default is false
//      if true it will include the part keepout area
//    led_3dmodel_filename: default is ''
//      Allows you to specify the path to a 3D model STEP or WRL file to be
//      used when rendering the PCB. Use the ${VAR_NAME} syntax to point to
//      a KiCad configured path.
//    led_3dmodel_xyz_offset: default is [0, 0, 0]
//      xyz offset (in mm), used to adjust the position of the 3d model
//      relative the footprint.
//    led_3dmodel_xyz_scale: default is [1, 1, 1]
//      xyz scale, used to adjust the size of the 3d model relative to its
//      original size.
//    led_3dmodel_xyz_rotation: default is [0, 0, 0]
//      xyz rotation (in degrees), used to adjust the orientation of the 3d
//      model relative the footprint.

module.exports = {
  params: {
    designator: 'LED',
    side: 'B',
    reversible: false,
    reverse_mount: true,
    include_traces_vias: true,
    signal_trace_width: 0.25,
    gnd_trace_width: 0.25,
    vcc_trace_width: 0.25,
    via_size: 0.8,
    via_drill: 0.4,
    include_courtyard: true,
    include_keepout: false,
    led_3dmodel_filename: '',
    led_3dmodel_xyz_offset: [0, 0, 0],
    led_3dmodel_xyz_rotation: [0, 0, 0],
    led_3dmodel_xyz_scale: [1, 1, 1],
    P1: { type: 'net', value: 'VCC' },
    P2: undefined,
    P3: { type: 'net', value: 'GND' },
    P4: undefined,
  },
  body: p => {
    const standard_opening = `
  (footprint "ceoloide:led_SK6812mini-e (${p.reverse_mount ? "per-key" : "underglow"}${p.reversible ? ", reversible" : "single-side"})" 
    (layer "${p.side}.Cu")
    ${p.at}
    (property "Reference" "${p.ref}"
      (at -4.75 0 ${90 + p.r})
      (layer "${p.side}.SilkS")
      ${p.ref_hide}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (attr smd)

    (fp_line (start -1.6 -1.4) (end 1.6 -1.4) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start -1.6 1.4) (end 1.6 1.4) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start -1.6 -1.4) (end -1.6 1.4) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start 1.6 -1.4) (end 1.6 1.4) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start -1.6 -1.05) (end -2.94 -1.05) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start -2.94 -1.05) (end -2.94 -0.37) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start -2.94 -0.37) (end -1.6 -0.37) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start -1.6 0.35) (end -2.94 0.35) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start -2.94 1.03) (end -1.6 1.03) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start -2.94 0.35) (end -2.94 1.03) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start 1.6 1.03) (end 2.94 1.03) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start 2.94 0.35) (end 1.6 0.35) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start 2.94 1.03) (end 2.94 0.35) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start 1.6 -0.37) (end 2.94 -0.37) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start 2.94 -1.05) (end 1.6 -1.05) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start 2.94 -0.37) (end 2.94 -1.05) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    `
    const marks_reversed = `
    (fp_line (start -0.8 -1.4) (end -0.8 1.4) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start 0.8 -1.4) (end 0.8 1.4) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start -1 -1.4) (end -1 1.4) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    (fp_line (start 1 -1.4) (end 1 1.4) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    `
    const marks_straight = `
    (fp_line (start -1.6 -0.7) (end -0.8 -1.4) (layer "Dwgs.User") (stroke (width 0.12) (type solid)))
    `
    const front_reversed = `
    (fp_line (start -3.8 1.6) (end -2.2 1.6) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -3.8 0) (end -3.8 1.6) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (pad "4" smd rect (at -2.7 -0.7 ${p.r}) (size 1.4 1) (layers "F.Cu" "F.Paste" "F.Mask") ${p.P4.str})
    (pad "3" smd rect (at -2.7 0.7 ${p.r}) (size 1.4 1) (layers "F.Cu" "F.Paste" "F.Mask") ${p.P3.str})
    (pad "1" smd rect (at 2.7 -0.7 ${p.r}) (size 1.4 1) (layers "F.Cu" "F.Paste" "F.Mask") ${p.P1.str})
    (pad "2" smd rect (at 2.7 0.7 ${p.r}) (size 1.4 1) (layers "F.Cu" "F.Paste" "F.Mask") ${p.P2.str})
    `
    const front = `
    (fp_line (start -3.8 -1.6) (end -2.2 -1.6) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -3.8 0) (end -3.8 -1.6) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (pad "4" smd rect (at -2.70 0.7 ${p.r}) (size 1.4 1) (layers "F.Cu" "F.Paste" "F.Mask") ${p.P4.str})
    (pad "3" smd rect (at -2.70 -0.7 ${p.r}) (size 1.4 1) (layers "F.Cu" "F.Paste" "F.Mask") ${p.P3.str})
    (pad "1" smd rect (at 2.70 0.7 ${p.r}) (size 1.4 1) (layers "F.Cu" "F.Paste" "F.Mask") ${p.P1.str})
    (pad "2" smd rect (at 2.70 -0.7 ${p.r}) (size 1.4 1) (layers "F.Cu" "F.Paste" "F.Mask") ${p.P2.str})
    `
    const back_reversed = `
    (fp_line (start -3.8 -1.6) (end -2.2 -1.6) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -3.8 0) (end -3.8 -1.6) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (pad "2" smd rect (at 2.70 -0.7 ${p.r}) (size 1.4 1) (layers "B.Cu" "B.Paste" "B.Mask") ${p.P2.str})
    (pad "1" smd rect (at 2.70 0.7 ${p.r}) (size 1.4 1) (layers "B.Cu" "B.Paste" "B.Mask") ${p.P1.str})
    (pad "3" smd rect (at -2.70 -0.7 ${p.r}) (size 1.4 1) (layers "B.Cu" "B.Paste" "B.Mask") ${p.P3.str})
    (pad "4" smd rect (at -2.70 0.7 ${p.r}) (size 1.4 1) (layers "B.Cu" "B.Paste" "B.Mask") ${p.P4.str})
    `
    const back = `
    (fp_line (start -3.8 1.6) (end -2.2 1.6) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -3.8 0) (end -3.8 1.6) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (pad "2" smd rect (at 2.70 0.7 ${p.r}) (size 1.4 1) (layers "B.Cu" "B.Paste" "B.Mask") ${p.P2.str})
    (pad "1" smd rect (at 2.70 -0.7 ${p.r}) (size 1.4 1) (layers "B.Cu" "B.Paste" "B.Mask") ${p.P1.str})
    (pad "3" smd rect (at -2.70 0.7 ${p.r}) (size 1.4 1) (layers "B.Cu" "B.Paste" "B.Mask") ${p.P3.str})
    (pad "4" smd rect (at -2.70 -0.7 ${p.r}) (size 1.4 1) (layers "B.Cu" "B.Paste" "B.Mask") ${p.P4.str})
    `
    const standard_closing = `
    (fp_rect (start -1.8 -1.55) (end 1.8 1.55) (layer "Edge.Cuts") (stroke (width 0.12) (type solid)) (fill none))
  )
    `

    const traces_vias_reversed = `
  ${'' /* VCC Trace */}
  (segment (start ${p.eaxy(3.4, -0.7)}) (end ${p.eaxy(4.06, -0.105916)}) (width ${p.vcc_trace_width}) (layer "F.Cu") (net ${p.P1.index}))
  (segment (start ${p.eaxy(4.06, -0.105916)}) (end ${p.eaxy(4.06, 0.7)}) (width ${p.vcc_trace_width}) (layer "F.Cu") (net ${p.P1.index}))
  (segment (start ${p.eaxy(2.7, -0.7)}) (end ${p.eaxy(3.4, -0.7)}) (width ${p.vcc_trace_width}) (layer "F.Cu") (net ${p.P1.index}))
  (via (at ${p.eaxy(4.06, 0.7)}) (size ${p.via_size}) (drill ${p.via_drill}) (layers "F.Cu" "B.Cu") (net ${p.P1.index}))
  (segment (start ${p.eaxy(2.7, 0.7)}) (end ${p.eaxy(4.06, 0.7)}) (width ${p.vcc_trace_width}) (layer "B.Cu") (net ${p.P1.index}))
  ${'' /* Data signal out trace */}
  (segment (start ${p.eaxy(4.95, -0.7)}) (end ${p.eaxy(2.7, -0.7)}) (width ${p.signal_trace_width}) (layer "B.Cu") (net ${p.P2.index}))
  (via (at ${p.eaxy(4.95, -0.7)}) (size ${p.via_size}) (drill ${p.via_drill}) (layers "F.Cu" "B.Cu") (net ${p.P2.index}))
  (segment (start ${p.eaxy(2.7, 0.7)}) (end ${p.eaxy(3.481, 1.485)}) (width ${p.signal_trace_width}) (layer "F.Cu") (net ${p.P2.index}))
  (segment (start ${p.eaxy(3.481, 1.485)}) (end ${p.eaxy(4.529, 1.485)}) (width ${p.signal_trace_width}) (layer "F.Cu") (net ${p.P2.index}))
  (segment (start ${p.eaxy(4.95, 1.06)}) (end ${p.eaxy(4.95, -0.7)}) (width ${p.signal_trace_width}) (layer "F.Cu") (net ${p.P2.index}))
  (segment (start ${p.eaxy(4.529, 1.485)}) (end ${p.eaxy(4.95, 1.06)}) (width ${p.signal_trace_width}) (layer "F.Cu") (net ${p.P2.index}))
  ${'' /* GND Trace */}
  (segment (start ${p.eaxy(-3.4, -0.7)}) (end ${p.eaxy(-4.06, -0.105916)}) (width ${p.gnd_trace_width}) (layer "B.Cu") (net ${p.P3.index}))
  (segment (start ${p.eaxy(-4.06, -0.105916)}) (end ${p.eaxy(-4.06, 0.7)}) (width ${p.gnd_trace_width}) (layer "B.Cu") (net ${p.P3.index}))
  (segment (start ${p.eaxy(-2.7, -0.7)}) (end ${p.eaxy(-3.4, -0.7)}) (width ${p.gnd_trace_width}) (layer "B.Cu") (net ${p.P3.index}))
  (via (at ${p.eaxy(-4.06, 0.7)}) (size ${p.via_size}) (drill ${p.via_drill}) (layers "F.Cu" "B.Cu") (net ${p.P3.index}))
  (segment (start ${p.eaxy(-2.7, 0.7)}) (end ${p.eaxy(-4.06, 0.7)}) (width ${p.gnd_trace_width}) (layer "F.Cu") (net ${p.P3.index}))
  ${'' /* Data signal in trace */}
  (segment (start ${p.eaxy(-4.95, -0.7)}) (end ${p.eaxy(-2.7, -0.7)}) (width ${p.signal_trace_width}) (layer "F.Cu") (net ${p.P4.index}))
  (via (at ${p.eaxy(-4.95, -0.7)}) (size ${p.via_size}) (drill ${p.via_drill}) (layers "F.Cu" "B.Cu") (net ${p.P4.index}))
  (segment (start ${p.eaxy(-2.7, 0.7)}) (end ${p.eaxy(-3.481, 1.485)}) (width ${p.signal_trace_width}) (layer "B.Cu") (net ${p.P4.index}))
  (segment (start ${p.eaxy(-3.481, 1.485)}) (end ${p.eaxy(-4.529, 1.485)}) (width ${p.signal_trace_width}) (layer "B.Cu") (net ${p.P4.index}))
  (segment (start ${p.eaxy(-4.95, 1.06)}) (end ${p.eaxy(-4.95, -0.7)}) (width ${p.signal_trace_width}) (layer "B.Cu") (net ${p.P4.index}))
  (segment (start ${p.eaxy(-4.529, 1.485)}) (end ${p.eaxy(-4.95, 1.06)}) (width ${p.signal_trace_width}) (layer "B.Cu") (net ${p.P4.index}))
    `

    const traces_vias_straight = `
  ${'' /* VCC Trace */}
  (segment (start ${p.eaxy(3.4, -0.7)}) (end ${p.eaxy(4.06, -0.105916)}) (width ${p.vcc_trace_width}) (layer "B.Cu") (net ${p.P1.index}))
  (segment (start ${p.eaxy(4.06, -0.105916)}) (end ${p.eaxy(4.06, 0.7)}) (width ${p.vcc_trace_width}) (layer "B.Cu") (net ${p.P1.index}))
  (segment (start ${p.eaxy(2.7, -0.7)}) (end ${p.eaxy(3.4, -0.7)}) (width ${p.vcc_trace_width}) (layer "B.Cu") (net ${p.P1.index}))
  (via (at ${p.eaxy(4.06, 0.7)}) (size ${p.via_size}) (drill ${p.via_drill}) (layers "F.Cu" "B.Cu") (net ${p.P1.index}))
  (segment (start ${p.eaxy(2.7, 0.7)}) (end ${p.eaxy(4.06, 0.7)}) (width ${p.vcc_trace_width}) (layer "F.Cu") (net ${p.P1.index}))
  ${'' /* Data signal out trace */}
  (segment (start ${p.eaxy(4.95, -0.7)}) (end ${p.eaxy(2.7, -0.7)}) (width ${p.signal_trace_width}) (layer "F.Cu") (net ${p.P2.index}))
  (via (at ${p.eaxy(4.95, -0.7)}) (size ${p.via_size}) (drill ${p.via_drill}) (layers "F.Cu" "B.Cu") (net ${p.P2.index}))
  (segment (start ${p.eaxy(2.7, 0.7)}) (end ${p.eaxy(3.481, 1.485)}) (width ${p.signal_trace_width}) (layer "B.Cu") (net ${p.P2.index}))
  (segment (start ${p.eaxy(3.481, 1.485)}) (end ${p.eaxy(4.529, 1.485)}) (width ${p.signal_trace_width}) (layer "B.Cu") (net ${p.P2.index}))
  (segment (start ${p.eaxy(4.95, 1.06)}) (end ${p.eaxy(4.95, -0.7)}) (width ${p.signal_trace_width}) (layer "B.Cu") (net ${p.P2.index}))
  (segment (start ${p.eaxy(4.529, 1.485)}) (end ${p.eaxy(4.95, 1.06)}) (width ${p.signal_trace_width}) (layer "B.Cu") (net ${p.P2.index}))
  ${'' /* GND Trace */}
  (segment (start ${p.eaxy(-3.4, -0.7)}) (end ${p.eaxy(-4.06, -0.105916)}) (width ${p.gnd_trace_width}) (layer "F.Cu") (net ${p.P3.index}))
  (segment (start ${p.eaxy(-4.06, -0.105916)}) (end ${p.eaxy(-4.06, 0.7)}) (width ${p.gnd_trace_width}) (layer "F.Cu") (net ${p.P3.index}))
  (segment (start ${p.eaxy(-2.7, -0.7)}) (end ${p.eaxy(-3.4, -0.7)}) (width ${p.gnd_trace_width}) (layer "F.Cu") (net ${p.P3.index}))
  (via (at ${p.eaxy(-4.06, 0.7)}) (size ${p.via_size}) (drill ${p.via_drill}) (layers "F.Cu" "B.Cu") (net ${p.P3.index}))
  (segment (start ${p.eaxy(-2.7, 0.7)}) (end ${p.eaxy(-4.06, 0.7)}) (width ${p.gnd_trace_width}) (layer "B.Cu") (net ${p.P3.index}))
  ${'' /* Data signal in trace */}
  (segment (start ${p.eaxy(-4.95, -0.7)}) (end ${p.eaxy(-2.7, -0.7)}) (width ${p.signal_trace_width}) (layer "B.Cu") (net ${p.P4.index}))
  (via (at ${p.eaxy(-4.95, -0.7)}) (size ${p.via_size}) (drill ${p.via_drill}) (layers "F.Cu" "B.Cu") (net ${p.P4.index}))
  (segment (start ${p.eaxy(-2.7, 0.7)}) (end ${p.eaxy(-3.481, 1.485)}) (width ${p.signal_trace_width}) (layer "F.Cu") (net ${p.P4.index}))
  (segment (start ${p.eaxy(-3.481, 1.485)}) (end ${p.eaxy(-4.529, 1.485)}) (width ${p.signal_trace_width}) (layer "F.Cu") (net ${p.P4.index}))
  (segment (start ${p.eaxy(-4.95, 1.06)}) (end ${p.eaxy(-4.95, -0.7)}) (width ${p.signal_trace_width}) (layer "F.Cu") (net ${p.P4.index}))
  (segment (start ${p.eaxy(-4.529, 1.485)}) (end ${p.eaxy(-4.95, 1.06)}) (width ${p.signal_trace_width}) (layer "F.Cu") (net ${p.P4.index}))
    `

    const courtyard_front = `
    (fp_poly
      (pts
        (xy 1.6 -1.05)
        (xy 2.94 -1.05)
        (xy 2.94 -0.37)
        (xy 1.6 -0.37)
        (xy 1.6 0.35)
        (xy 2.94 0.35)
        (xy 2.94 1.03)
        (xy 1.6 1.03)
        (xy 1.6 1.4)
        (xy -1.6 1.4)
        (xy -1.6 1.03)
        (xy -2.94 1.03)
        (xy -2.94 0.35)
        (xy -1.6 0.35)
        (xy -1.6 -0.37)
        (xy -2.94 -0.37)
        (xy -2.94 -1.05)
        (xy -1.6 -1.05)
        (xy -1.6 -1.4)
        (xy 1.6 -1.4)
      )
			(stroke
				(width 0.1)
				(type solid)
			)
      (layer "B.CrtYd")
    )
    `

    const courtyard_back = `
    (fp_poly
      (pts
        (xy 1.6 -1.05)
        (xy 2.94 -1.05)
        (xy 2.94 -0.37)
        (xy 1.6 -0.37)
        (xy 1.6 0.35)
        (xy 2.94 0.35)
        (xy 2.94 1.03)
        (xy 1.6 1.03)
        (xy 1.6 1.4)
        (xy -1.6 1.4)
        (xy -1.6 1.03)
        (xy -2.94 1.03)
        (xy -2.94 0.35)
        (xy -1.6 0.35)
        (xy -1.6 -0.37)
        (xy -2.94 -0.37)
        (xy -2.94 -1.05)
        (xy -1.6 -1.05)
        (xy -1.6 -1.4)
        (xy 1.6 -1.4)
      )
			(stroke
				(width 0.1)
				(type solid)
			)
      (layer "B.CrtYd")
    )
    `

    const keepout = `
  (zone
    (net 0)
    (net_name "")
    (layers "F&B.Cu")
    (hatch edge 0.3)
    (connect_pads (clearance 0))
    (min_thickness 0.25)
		(filled_areas_thickness no)
		(keepout
			(tracks not_allowed)
			(vias not_allowed)
			(pads allowed)
			(copperpour not_allowed)
			(footprints allowed)
		)
		(fill
			(thermal_gap 0.5)
			(thermal_bridge_width 0.5)
		)
    (polygon
      (pts
        (xy ${p.eaxy(-2.00, -1.85)})
        (xy ${p.eaxy(2.00, -1.85)})
        (xy ${p.eaxy(2.00, 1.85)})
        (xy ${p.eaxy(-2.00, 1.85)})
      )
    )
  )
    `

    // Default orientation follows the mounting variant; explicit rotations still win.
    const model_rotation = p.led_3dmodel_xyz_rotation || [p.reverse_mount ? -90 : 90, 0, 180];
    const led_3dmodel = `
    (model ${JSON.stringify(p.led_3dmodel_filename)}
      (offset (xyz ${p.led_3dmodel_xyz_offset[0]} ${p.led_3dmodel_xyz_offset[1]} ${p.led_3dmodel_xyz_offset[2]}))
      (scale (xyz ${p.led_3dmodel_xyz_scale[0]} ${p.led_3dmodel_xyz_scale[1]} ${p.led_3dmodel_xyz_scale[2]}))
      (rotate (xyz ${model_rotation[0]} ${model_rotation[1]} ${model_rotation[2]}))
    )
      `

    let final = standard_opening;

    if (p.side == "F" || p.reversible) {
      if (p.reverse_mount) {
        final += marks_reversed;
        final += front_reversed;
      } else {
        final += marks_straight;
        final += front;
      }
      if (p.include_courtyard) {
        final += courtyard_front;
      }
    }
    if (p.side == "B" || p.reversible) {
      if (p.reverse_mount) {
        final += back_reversed;
        final += marks_reversed;
      } else {
        final += marks_straight;
        final += back;
      }
      if (p.include_courtyard) {
        final += courtyard_back;
      }
    }

    if (p.led_3dmodel_filename) {
      final += led_3dmodel
    }

    final += standard_closing;
    if (p.include_keepout) {
      final += keepout;
    }
    if (p.reversible && p.include_traces_vias) {
      if (p.reverse_mount) {
        final += traces_vias_reversed;
      } else {
        final += traces_vias_straight;
      }
    }

    return final;
  }
}

;module.exports = ((original) => {
    const defaults = {"led_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/keebio/SK6812MINI-E v1.step","led_3dmodel_xyz_rotation":""};
    const bossVariant = null;
    const chocV2 = "${KIPRJMOD}/models/boardstudio/koktoh/Choc_V2_Red.step";
    return {
      ...original,
      params: {...original.params, ...defaults},
      body: p => {
        // The bundled extension crosses the PCB with a 5 mm outer diameter.
        const extensionDiameter = 5;
        if (false && p.drill < extensionDiameter
          && p.tp_extension_3dmodel_filename === defaults.tp_extension_3dmodel_filename
          && !p.tp_extension_3dmodel_xyz_scale
          && !p.tp_extension_3dmodel_xyz_rotation
          && !p.tp_extension_3dmodel_xyz_offset) {
          throw new Error('The bundled trackpoint extension requires a center drill of at least 5 mm. Select a compatible extension model or drill size.');
        }
        if (false && !p.choc_v1_support) {
          p = {...p};
          if (!p.choc_v2_support) {
            for (const key of Object.keys(defaults)) {
              if (p[key] === defaults[key]) { p[key] = ''; }
            }
          } else {
            if (p.switch_3dmodel_filename === defaults.switch_3dmodel_filename) {
              const automatic = !p.switch_3dmodel_xyz_rotation && !p.switch_3dmodel_xyz_offset
                && p.switch_3dmodel_xyz_scale.every(value => value === 1);
              const postDiameter = 4.8;
              if (automatic && (!p.include_stabilizer_pad || p.oval_stabilizer_pad
                || (p.center_hole_diameter > 0 && p.center_hole_diameter < postDiameter))) {
                throw new Error('The bundled Choc V2 model requires the round stabilizer hole and a center drill of at least 4.8 mm. Use a matching model or transform for modified hardware.');
              }
              p.switch_3dmodel_filename = chocV2;
              p.switch_3dmodel_xyz_rotation ||= p.side === 'F' ? [180, 0, 0] : [0, 180, 0];
              p.switch_3dmodel_xyz_offset ||= [0, 0, -p.pcb_thickness];
            }
            if (p.keycap_3dmodel_filename === defaults.keycap_3dmodel_filename) {
              p.keycap_3dmodel_filename = '';
            }
          }
        }
        if (bossVariant && p.include_bosses && p.reset_switch_3dmodel_filename === defaults.reset_switch_3dmodel_filename) {
          p = {...p, reset_switch_3dmodel_filename: bossVariant};
        }
        return original.body(p);
      }
    };
  })(module.exports);

return module.exports; })(),
  "ceoloide/mcu_nice_nano": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Copyright (c) 2023 Marco Massarelli
//
// SPDX-License-Identifier: CC-BY-NC-SA-4.0
//
// To view a copy of this license, visit http://creativecommons.org/licenses/by-nc-sa/4.0/
//
// Author: @infused-kim + @ceoloide improvements
//
// Description:
//  A single-side or reversible footprint for the nice!nano (or any pro-micro compatible
//  controller) that uses jumpers instead of two socket rows to be reversible.
//
//  Note that the extra pins are *ONLY* compatible with nice!nano boards and not with
//  clones like the Supermini, which has pins in a slightly different position.
//
//  This is a re-implementation of the promicro_pretty footprint made popular
//  by @benvallack.
//
// Pinout and schematics:
//  https://nicekeyboards.com/docs/nice-nano/pinout-schematic
//
// Params:
//    side: default is F for Front
//      the side on which to place the single-side footprint and designator, either F or B
//    reversible: default is false
//      if true, the footprint will be placed on both sides so that the PCB can be
//      reversible
//    reverse_mount: default is false (MCU facing away from the PCB)
//      if true, the sockets will be oriented so that the MCU faces the PCB (RAW / B+ is the
//      top left pin). This is the most common mounting option for the nice!nano.
//      When set to false, the pads will match the datasheet and assume the MCU faces away
//      from the PCB (RAW / B+ is the top right pin).
//    include_traces: default is true
//      if true it will include traces that connect the jumper pads to the vias
//      and the through-holes for the MCU
//    include_extra_pins: default is false
//      if true and if not reversible it will include nice!nano extra pin sockets (P1.01,
//      P1.02, P1.07)
//    only_required_jumpers: default is false
//      if true, it will only place jumpers on the first 4 rows of pins, which can't be
//      reversed in firmware, i.e. RAW and P1, GND and P0, GND and RST, GND and VCC.
//    use_rectangular_jumpers: default is false
//      if true, it will replace chevron-style jumpers with rectangual pads
//    via_size: default is 0.8
//      allows to define the size of the via. Not recommended below 0.56 (JLCPCB minimum),
//      or above 0.8 (KiCad default), to avoid overlap or DRC errors.
//    via_drill: default is 0.4
//      allows to define the size of the drill. Not recommended below 0.3 (JLCPCB minimum),
//      or above 0.4 (KiCad default), to avoid overlap or DRC errors. 
//    Pxx_label, VCC_label, RAW_label, GND_label, RST_label: default is ''
//      allows to override the label for each pin
//    mcu_3dmodel_filename: default is ''
//      Allows you to specify the path to a 3D model STEP or WRL file to be
//      used when rendering the PCB. Use the ${VAR_NAME} syntax to point to
//      a KiCad configured path.
//    mcu_3dmodel_xyz_offset: default is [0, 0, 0]
//      xyz offset (in mm), used to adjust the position of the 3d model
//      relative the footprint.
//    mcu_3dmodel_xyz_scale: default is [1, 1, 1]
//      xyz scale, used to adjust the size of the 3d model relative to its
//      original size.
//    mcu_3dmodel_xyz_rotation: default is [0, 0, 0]
//      xyz rotation (in degrees), used to adjust the orientation of the 3d
//      model relative the footprint.
//
// @infused-kim's improvements:
//  - Use real traces instead of pads, which gets rid of hundreds of DRC errors.
//  - Leave more space between the vias to allow easier routing through the middle
//    of the footprint
//
// @ceoloide's improvements:
//  - Move vias closer to the pads to clear up more space for silkscreen
//  - Add ability to use rectangular jumpers instead of chevron-style
//  - Add ability to control via size, to free up space for routing if needed
//  - Add ability to only have required jumpers and let the rest be handled in firmware
//  - Add single side (non-reversible) support
//  - Add ability to mount with MCU facing towards or away from PCB
//  - Add ability to show silkscreen labels on both sides for single side footprint
//  - Add extra pins (P1.01, P1.02, P1.07) when footprint is single-side or reversible
//    (only required jumpers)
//  - Upgrade to KiCad 8
//
// # Placement and soldering of jumpers
//
// The reversible footprint is meant to be used with jumpers on the
// OPPOSITE side of where the nice!nano (or pro-micro compatible board) is
// installed. The silkscreen labels will also match the board when read on
// the opposite side. This is to have all jumpers and components to solder on
// the same side, and be able to read the correct labels of the MCU to do
// tests with a multimeter.
//
// # Further credits
//
// The original footprint was created from scratch by @infused-kim, but was based on the ideas from
// these other footprints:
//
// https://github.com/Albert-IV/ergogen-contrib/blob/main/src/footprints/promicro_pretty.js
// https://github.com/50an6xy06r6n/keyboard_reversible.pretty

module.exports = {
  params: {
    designator: 'MCU',
    side: 'F',
    reversible: false,
    reverse_mount: false,
    include_traces: true,
    include_extra_pins: false,
    // Inverted jumper placement is unsupported; true is rejected during generation.
    invert_jumpers_position: false,
    only_required_jumpers: false,
    use_rectangular_jumpers: false,
    via_size: 0.8, // JLCPC min is 0.56 for 1-2 layer boards, KiCad defaults to 0.8
    via_drill: 0.4, // JLCPC min is 0.3 for 1-2 layer boards, KiCad defaults to 0.4

    show_instructions: true,
    show_silk_labels: true,
    show_silk_labels_on_both_sides: false,
    show_via_labels: true,

    mcu_3dmodel_filename: '',
    mcu_3dmodel_xyz_offset: [0, 0, 0],
    mcu_3dmodel_xyz_rotation: [0, 0, 0],
    mcu_3dmodel_xyz_scale: [1, 1, 1],

    RAW_label: '',
    GND_label: '',
    RST_label: '',
    VCC_label: '',
    P21_label: '',
    P20_label: '',
    P19_label: '',
    P18_label: '',
    P15_label: '',
    P14_label: '',
    P16_label: '',
    P10_label: '',

    P1_label: '',
    P0_label: '',
    P2_label: '',
    P3_label: '',
    P4_label: '',
    P5_label: '',
    P6_label: '',
    P7_label: '',
    P8_label: '',
    P9_label: '',

    P101_label: '',
    P102_label: '',
    P107_label: '',

    RAW: { type: 'net', value: 'RAW' },
    GND: { type: 'net', value: 'GND' },
    RST: { type: 'net', value: 'RST' },
    VCC: { type: 'net', value: 'VCC' },
    P21: { type: 'net', value: 'P21' },
    P20: { type: 'net', value: 'P20' },
    P19: { type: 'net', value: 'P19' },
    P18: { type: 'net', value: 'P18' },
    P15: { type: 'net', value: 'P15' },
    P14: { type: 'net', value: 'P14' },
    P16: { type: 'net', value: 'P16' },
    P10: { type: 'net', value: 'P10' },

    P1: { type: 'net', value: 'P1' },
    P0: { type: 'net', value: 'P0' },
    P2: { type: 'net', value: 'P2' },
    P3: { type: 'net', value: 'P3' },
    P4: { type: 'net', value: 'P4' },
    P5: { type: 'net', value: 'P5' },
    P6: { type: 'net', value: 'P6' },
    P7: { type: 'net', value: 'P7' },
    P8: { type: 'net', value: 'P8' },
    P9: { type: 'net', value: 'P9' },

    P101: { type: 'net', value: 'P101' },
    P102: { type: 'net', value: 'P102' },
    P107: { type: 'net', value: 'P107' },
  },
  body: p => {
    if (p.invert_jumpers_position) {
      throw new Error('invert_jumpers_position is unsupported; set it to false and use the documented jumper assembly.');
    }
    const get_pin_net_name = (p, pin_name) => {
      return p[pin_name].name;
    };

    const get_pin_net_str = (p, pin_name) => {
      return p[pin_name].str;
    };

    const get_pin_label_override = (p, pin_name) => {
      let prop_name = `${pin_name}_label`;
      return p[prop_name];
    };

    const get_pin_label = (p, pin_name) => {
      let label = get_pin_label_override(p, pin_name);
      if (label == '') {
        label = get_pin_net_name(p, pin_name);
      }

      if (label === undefined) {
        label = '""';
      }

      return label;
    };

    const pin_names = [
      // The pin matrix below assumes PCB is mounted with the MCU
      // facing away from the PCB (reverse_mount = false) on the
      // Front side. It should be inverted for reverse_mount = true
      // or when mounted on teh Back
      ['P1', 'RAW'],
      ['P0', 'GND'],
      ['GND', 'RST'],
      ['GND', 'VCC'],
      ['P2', 'P21'],
      ['P3', 'P20'],
      ['P4', 'P19'],
      ['P5', 'P18'],
      ['P6', 'P15'],
      ['P7', 'P14'],
      ['P8', 'P16'],
      ['P9', 'P10'],
    ]

    const gen_traces_row = (row_num) => {
      const net_left = p[pin_names[row_num][invert_pins ? 1 : 0]].index
      const net_right = p[pin_names[row_num][invert_pins ? 0 : 1]].index
      const traces = `
  (segment (start ${p.eaxy((p.use_rectangular_jumpers ? 4.58 : 4.775), -12.7 + (row_num * 2.54))}) (end ${p.eaxy(3.4, -12.7 + (row_num * 2.54))}) (width 0.25) (layer "F.Cu") (net ${net_right}))
  (segment (start ${p.eaxy((p.use_rectangular_jumpers ? -4.58 : -4.775), -12.7 + (row_num * 2.54))}) (end ${p.eaxy(-3.4, -12.7 + (row_num * 2.54))}) (width 0.25) (layer "F.Cu") (net ${net_left}))

  (segment (start ${p.eaxy(-7.62, -12.7 + (row_num * 2.54))}) (end ${p.eaxy(-5.5, -12.7 + (row_num * 2.54))}) (width 0.25) (layer "F.Cu") (net ${p.local_net(24 - row_num).index}))
  (segment (start ${p.eaxy(-7.62, -12.7 + (row_num * 2.54))}) (end ${p.eaxy(-5.5, -12.7 + (row_num * 2.54))}) (width 0.25) (layer "B.Cu") (net ${p.local_net(24 - row_num).index}))
  (segment (start ${p.eaxy(5.5, -12.7 + (row_num * 2.54))}) (end ${p.eaxy(7.62, -12.7 + (row_num * 2.54))}) (width 0.25) (layer "F.Cu") (net ${p.local_net(1 + row_num).index}))
  (segment (start ${p.eaxy(7.62, -12.7 + (row_num * 2.54))}) (end ${p.eaxy(5.5, -12.7 + (row_num * 2.54))}) (width 0.25) (layer "B.Cu") (net ${p.local_net(1 + row_num).index}))

  (segment (start ${p.eaxy(-2.604695, 0.23 + (row_num * 2.54) - 12.7)}) (end ${p.eaxy(3.17, 0.23 + (row_num * 2.54) - 12.7)}) (width 0.25) (layer "B.Cu") (net ${net_right}))
  (segment (start ${p.eaxy(-4.775, 0 + (row_num * 2.54) - 12.7)}) (end ${p.eaxy(-4.425305, 0 + (row_num * 2.54) - 12.7)}) (width 0.25) (layer "B.Cu") (net ${net_right}))
  (segment (start ${p.eaxy(-3.700305, 0.725 + (row_num * 2.54) - 12.7)}) (end ${p.eaxy(-3.099695, 0.725 + (row_num * 2.54) - 12.7)}) (width 0.25) (layer "B.Cu") (net ${net_right}))
  (segment (start ${p.eaxy(-4.425305, 0 + (row_num * 2.54) - 12.7)}) (end ${p.eaxy(-3.700305, 0.725 + (row_num * 2.54) - 12.7)}) (width 0.25) (layer "B.Cu") (net ${net_right}))
  (segment (start ${p.eaxy(-3.099695, 0.725 + (row_num * 2.54) - 12.7)}) (end ${p.eaxy(-2.604695, 0.23 + (row_num * 2.54) - 12.7)}) (width 0.25) (layer "B.Cu") (net ${net_right}))

  (segment (start ${p.eaxy(4.775, 0 + (row_num * 2.54) - 12.7)}) (end ${p.eaxy(4.425305, 0 + (row_num * 2.54) - 12.7)}) (width 0.25) (layer "B.Cu") (net ${net_left}))
  (segment (start ${p.eaxy(2.594695, -0.22 + (row_num * 2.54) - 12.7)}) (end ${p.eaxy(-3.18, -0.22 + (row_num * 2.54) - 12.7)}) (width 0.25) (layer "B.Cu") (net ${net_left}))
  (segment (start ${p.eaxy(4.425305, 0 + (row_num * 2.54) - 12.7)}) (end ${p.eaxy(3.700305, -0.725 + (row_num * 2.54) - 12.7)}) (width 0.25) (layer "B.Cu") (net ${net_left}))
  (segment (start ${p.eaxy(3.700305, -0.725 + (row_num * 2.54) - 12.7)}) (end ${p.eaxy(3.099695, -0.725 + (row_num * 2.54) - 12.7)}) (width 0.25) (layer "B.Cu") (net ${net_left}))
  (segment (start ${p.eaxy(3.099695, -0.725 + (row_num * 2.54) - 12.7)}) (end ${p.eaxy(2.594695, -0.22 + (row_num * 2.54) - 12.7)}) (width 0.25) (layer "B.Cu") (net ${net_left}))
        `

      return traces
    }

    const gen_traces = () => {
      let traces = '';
      for (let i = 0; i < 12; i++) {
        if (i < 4 || !p.only_required_jumpers) {
          let row_traces = gen_traces_row(i)
          traces += row_traces
        }
      }

      return traces
    }

    const invert_pins = (p.side == 'B' && !p.reverse_mount && !p.reversible) || (p.side == 'F' && p.reverse_mount && !p.reversible) || (!p.reverse_mount && p.reversible)

    const gen_socket_row = (row_num, pin_name_left, pin_name_right, show_via_labels, show_silk_labels) => {
      const row_offset_y = 2.54 * row_num

      const socket_hole_num_left = 24 - row_num
      const socket_hole_num_right = 1 + row_num
      const via_num_left = 124 - row_num
      const via_num_right = 101 + row_num

      const net_left = get_pin_net_str(p, pin_name_left)
      const net_right = get_pin_net_str(p, pin_name_right)
      const via_label_left = get_pin_label(p, pin_name_left)
      const via_label_right = get_pin_label(p, pin_name_right)

      // These are the silkscreen labels that will be printed on the PCB.
      // If the footprint is reversible, they will be aligned with the pins
      // on the opposite side of where the MCU board is mounted.
      const net_silk_front_left = via_label_left
      const net_silk_front_right = via_label_right
      const net_silk_back_left = via_label_right
      const net_silk_back_right = via_label_left

      let socket_row_base = `
    ${''/* Socket Holes */}
    (pad "${socket_hole_num_left}" thru_hole circle (at -7.62 ${-12.7 + row_offset_y} ${p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${p.reversible && (row_num < 4 || !p.only_required_jumpers) ? p.local_net(socket_hole_num_left).str : net_left})
    (pad "${socket_hole_num_right}" thru_hole circle (at 7.62 ${-12.7 + row_offset_y} ${p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${p.reversible && (row_num < 4 || !p.only_required_jumpers) ? p.local_net(socket_hole_num_right).str : net_right})
      `
      let socket_row_vias = `
    ${''/* Inside VIAS */}
    (pad "${via_num_left}" thru_hole circle (at -3.4 ${-12.7 + row_offset_y} ${p.r}) (size ${p.via_size} ${p.via_size}) (drill ${p.via_drill}) (layers "*.Cu" "*.Mask") ${net_left})
    (pad "${via_num_right}" thru_hole circle (at 3.4 ${-12.7 + row_offset_y} ${p.r}) (size ${p.via_size} ${p.via_size}) (drill ${p.via_drill}) (layers "*.Cu" "*.Mask") ${net_right})
      `

      let socket_row_rectangular_jumpers = `
    ${''/* Jumper Pads - Front Left */}
    (pad "${socket_hole_num_left}" smd rect (at -5.48 ${-12.7 + row_offset_y} ${p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${p.local_net(socket_hole_num_left).str})
    (pad "${via_num_left}" smd rect (at -4.58 ${-12.7 + row_offset_y} ${p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${net_left})

    ${''/* Jumper Pads - Front Right */}
    (pad "${via_num_right}" smd rect (at 4.58 ${-12.7 + row_offset_y} ${p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${net_right})
    (pad "${socket_hole_num_right}" smd rect (at 5.48 ${-12.7 + row_offset_y} ${p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${p.local_net(socket_hole_num_right).str})

    ${''/* Jumper Pads - Back Left */}
    (pad "${socket_hole_num_left}" smd rect (at -5.48 ${-12.7 + row_offset_y} ${p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${p.local_net(socket_hole_num_left).str})
    (pad "${via_num_right}" smd rect (at -4.58 ${-12.7 + row_offset_y} ${p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${net_right})

    ${''/* Jumper Pads - Back Right */}
    (pad "${via_num_left}" smd rect (at 4.58 ${-12.7 + row_offset_y} ${p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${net_left})
    (pad "${socket_hole_num_right}" smd rect (at 5.48 ${-12.7 + row_offset_y} ${p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${p.local_net(socket_hole_num_right).str})
        `

      let socket_row_chevron_jumpers = `
    ${''/* Jumper Pads - Front Left */}
    (pad "${socket_hole_num_left}" smd custom (at -5.5 ${-12.7 + row_offset_y} ${p.r}) (size 0.2 0.2) (layers "F.Cu" "F.Paste" "F.Mask") ${p.local_net(socket_hole_num_left).str}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.5 -0.625) (xy -0.25 -0.625) (xy 0.25 0) (xy -0.25 0.625) (xy -0.5 0.625)
      ) (width 0) (fill yes))
    ))
    (pad "${via_num_left}" smd custom (at -4.775 ${-12.7 + row_offset_y} ${p.r}) (size 0.2 0.2) (layers "F.Cu" "F.Paste" "F.Mask") ${net_left}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.65 -0.625) (xy 0.5 -0.625) (xy 0.5 0.625) (xy -0.65 0.625) (xy -0.15 0)
      ) (width 0) (fill yes))
    ))

    ${''/* Jumper Pads - Front Right */}
    (pad "${via_num_right}" smd custom (at 4.775 ${-12.7 + row_offset_y} ${180 + p.r}) (size 0.2 0.2) (layers "F.Cu" "F.Paste" "F.Mask") ${net_right}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.65 -0.625) (xy 0.5 -0.625) (xy 0.5 0.625) (xy -0.65 0.625) (xy -0.15 0)
      ) (width 0) (fill yes))
    ))
    (pad "${socket_hole_num_right}" smd custom (at 5.5 ${-12.7 + row_offset_y} ${180 + p.r}) (size 0.2 0.2) (layers "F.Cu" "F.Paste" "F.Mask") ${p.local_net(socket_hole_num_right).str}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.5 -0.625) (xy -0.25 -0.625) (xy 0.25 0) (xy -0.25 0.625) (xy -0.5 0.625)
      ) (width 0) (fill yes))
    ))

    ${''/* Jumper Pads - Back Left */}
    (pad "${socket_hole_num_left}" smd custom (at -5.5 ${-12.7 + row_offset_y} ${p.r}) (size 0.2 0.2) (layers "B.Cu" "B.Paste" "B.Mask") ${p.local_net(socket_hole_num_left).str}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.5 0.625) (xy -0.25 0.625) (xy 0.25 0) (xy -0.25 -0.625) (xy -0.5 -0.625)
      ) (width 0) (fill yes))
    ))

    (pad "${via_num_right}" smd custom (at -4.775 ${-12.7 + row_offset_y} ${p.r}) (size 0.2 0.2) (layers "B.Cu" "B.Paste" "B.Mask") ${net_right}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.65 0.625) (xy 0.5 0.625) (xy 0.5 -0.625) (xy -0.65 -0.625) (xy -0.15 0)
      ) (width 0) (fill yes))
    ))

    ${''/* Jumper Pads - Back Right */}
    (pad "${via_num_left}" smd custom (at 4.775 ${-12.7 + row_offset_y} ${180 + p.r}) (size 0.2 0.2) (layers "B.Cu" "B.Paste" "B.Mask") ${net_left}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.65 0.625) (xy 0.5 0.625) (xy 0.5 -0.625) (xy -0.65 -0.625) (xy -0.15 0)
      ) (width 0) (fill yes))
    ))
    (pad "${socket_hole_num_right}" smd custom (at 5.5 ${-12.7 + row_offset_y} ${180 + p.r}) (size 0.2 0.2) (layers "B.Cu" "B.Paste" "B.Mask") ${p.local_net(socket_hole_num_right).str}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.5 0.625) (xy -0.25 0.625) (xy 0.25 0) (xy -0.25 -0.625) (xy -0.5 -0.625)
      ) (width 0) (fill yes))
    ))
        `
      let socket_row = socket_row_base;
      if (p.reversible && (row_num < 4 || !p.only_required_jumpers)) {
        socket_row += socket_row_vias;
        if (p.use_rectangular_jumpers) {
          socket_row += socket_row_rectangular_jumpers
        } else {
          socket_row += socket_row_chevron_jumpers
        }
      }
      if (show_silk_labels == true) {
        if (p.reversible || p.show_silk_labels_on_both_sides || p.side == 'F') {
          // Silkscreen labels - front
          if (row_num != 9
            || !p.include_extra_pins
            || (p.include_extra_pins && invert_pins && !p.reversible) 
            || (p.include_extra_pins && !p.only_required_jumpers && p.reversible)
          ) {
            socket_row += `
    (fp_text user "${net_silk_front_left}" (at -${p.reversible && (row_num < 4 || !p.only_required_jumpers) ? (net_silk_front_left.length > 2 ? 1.45 : 2.04) : 4.47} ${-12.7 + row_offset_y} ${p.r}) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)))
    )
            `
          }
          if (row_num != 9 
            || !p.include_extra_pins
            || (p.include_extra_pins && !invert_pins && !p.reversible)
            || (p.include_extra_pins && !p.only_required_jumpers && p.reversible)
          ) {
            socket_row += `
    (fp_text user "${net_silk_front_right}" (at ${p.reversible && (row_num < 4 || !p.only_required_jumpers) ? (net_silk_front_right.length > 2 ? 1.45 : 2.04) : 4.47} ${-12.7 + row_offset_y} ${p.r}) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)))
    )
            `
          }
        }
        if (p.reversible|| p.show_silk_labels_on_both_sides || p.side == 'B') {
          // Silkscreen labels - back
          if (row_num != 9 
            || !p.include_extra_pins
            || (p.include_extra_pins && !invert_pins && !p.reversible) 
            || (p.include_extra_pins && !p.only_required_jumpers && p.reversible)
          ) {
            socket_row += `
    (fp_text user "${net_silk_back_left}" (at ${p.reversible ? '-' : ''}${p.reversible && (row_num < 4 || !p.only_required_jumpers) ? (net_silk_back_left.length > 2 ? 1.45 : 2.04) : 4.47} ${-12.7 + row_offset_y} ${p.r}) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify mirror))
    )
            `
          }
          if (row_num != 9 
            || !p.include_extra_pins
            || (p.include_extra_pins && invert_pins && !p.reversible) 
            || (p.include_extra_pins && !p.only_required_jumpers && p.reversible)
          ) {
            socket_row += `
    (fp_text user "${net_silk_back_right}" (at ${p.reversible ? '' : '-'}${p.reversible && (row_num < 4 || !p.only_required_jumpers) ? (net_silk_back_right.length > 2 ? 1.45 : 2.04) : 4.47} ${-12.7 + row_offset_y} ${p.r}) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify mirror))
    )
            `
          }
        }
      }

      if (show_via_labels && (p.reversible && (row_num < 4 || !p.only_required_jumpers))) {
        socket_row += `
    ${''/* Via Labels - Front */}
    (fp_text user "${via_label_left}" (at -3.262 ${-13.5 + row_offset_y} ${p.r}) (layer "F.Fab")
      (effects (font (size 0.5 0.5) (thickness 0.08)))
    )
    (fp_text user "${via_label_right}" (at 3.262 ${-13.5 + row_offset_y} ${p.r}) (layer "F.Fab")
      (effects (font (size 0.5 0.5) (thickness 0.08)))
    )

    ${''/* Via Labels - Back */}
    (fp_text user "${via_label_left}" (at -3.262 ${-13.5 + row_offset_y} ${180 + p.r}) (layer "B.Fab")
      (effects (font (size 0.5 0.5) (thickness 0.08)) (justify mirror))
    )
    (fp_text user "${via_label_right}" (at 3.262 ${-13.5 + row_offset_y} ${180 + p.r}) (layer "B.Fab")
      (effects (font (size 0.5 0.5) (thickness 0.08)) (justify mirror))
    )
          `
      }

      return socket_row
    }
    const gen_socket_rows = (show_via_labels, show_silk_labels) => {

      let socket_rows = '';
      for (let i = 0; i < pin_names.length; i++) {
        let pin_name_left = pin_names[i][invert_pins ? 1 : 0]
        let pin_name_right = pin_names[i][invert_pins ? 0 : 1]

        const socket_row = gen_socket_row(
          i, pin_name_left, pin_name_right,
          show_via_labels, show_silk_labels
        )

        socket_rows += socket_row
      }
      // Socket silkscreen
      // P1 / D1 / P0.06 is marked according to orientation
      if (show_silk_labels == true) {
        if (p.reversible || p.show_silk_labels_on_both_sides || p.side == 'F') {
          socket_rows += `
    (fp_line (start 6.29 -14.03) (end 8.95 -14.03) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 6.29 -14.03) (end 6.29 16.57) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 6.29 16.57) (end 8.95 16.57) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -6.29 -14.03) (end -6.29 16.57) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 8.95 -14.03) (end 8.95 16.57) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -8.95 -14.03) (end -6.29 -14.03) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -8.95 -14.03) (end -8.95 16.57) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -8.95 16.57) (end -6.29 16.57) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start ${invert_pins ? '' : '-'}6.29 -11.43) (end ${invert_pins ? '' : '-'}8.95 -11.43) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
            `
        }
        if (p.reversible || p.show_silk_labels_on_both_sides || p.side == 'B') {
          socket_rows += `
    (fp_line (start -6.29 -14.03) (end -8.95 -14.03) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -6.29 -14.03) (end -6.29 16.57) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -6.29 16.57) (end -8.95 16.57) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -8.95 -14.03) (end -8.95 16.57) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 8.95 -14.03) (end 6.29 -14.03) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 8.95 -14.03) (end 8.95 16.57) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 8.95 16.57) (end 6.29 16.57) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 6.29 -14.03) (end 6.29 16.57) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start ${invert_pins ? (p.reversible ? '-' : '') : (p.reversible ? '' : '-')}8.95 -11.43) (end ${invert_pins ? (p.reversible ? '-' : '') : (p.reversible ? '' : '-')}6.29 -11.43) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
          `
        }
      }
      return socket_rows
    }

    const common_top = `
  (footprint "ceoloide:mcu_nice_nano"
    (layer "${p.side}.Cu")
    ${p.at}
    (property "Reference" "${p.ref}"
      (at 0 -15 ${p.r})
      (layer "${p.side}.SilkS")
      ${p.ref_hide}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (attr exclude_from_pos_files exclude_from_bom)

    ${''/* USB socket outline */}
    (fp_line (start 3.556 -18.034) (end 3.556 -16.51) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -3.81 -16.51) (end -3.81 -18.034) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -3.81 -18.034) (end 3.556 -18.034) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))


  ${''/* Controller outline */}
    (fp_line (start -8.89 -16.51) (end 8.89 -16.51) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -8.89 -16.51) (end -8.89 16.57) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start 8.89 -16.51) (end 8.89 16.57) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -8.89 16.57) (end 8.89 16.57) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    `;

    const instructions = `
    (fp_text user "R hand back side (M${!p.reverse_mount ? '↑' : '↓'})" (at 0 -15.245 ${p.r}) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (fp_text user "L hand back side (M${!p.reverse_mount ? '↑' : '↓'})" (at 0 -15.245 ${p.r}) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify mirror))
    )
    `

    const socket_rows = gen_socket_rows(
      p.show_via_labels, p.show_silk_labels
    )
    const traces = gen_traces()

    const extra_pins = `
    (pad "25" thru_hole circle (at ${invert_pins ? '' : '-'}5.08 10.16 ${p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${p.P101})
    (pad "26" thru_hole circle (at ${invert_pins ? '' : '-'}2.54 10.16 ${p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${p.P102})
    (pad "27" thru_hole circle (at 0 10.16 ${p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${p.P107})
    `
    const extra_pins_reversible = `
    (pad "28" thru_hole circle (at ${invert_pins ? '-' : ''}5.08 10.16 ${p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${p.P101})
    (pad "29" thru_hole circle (at ${invert_pins ? '-' : ''}2.54 10.16 ${p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${p.P102})
    `

    // Match the bundled 5 mm socket stack and the model's board datum.
    const socket_height = 5;
    const module_thickness = 1.6;
    const model_row_offset = 0.01002;
    const model_offset = p.mcu_3dmodel_xyz_offset || [0, p.side === 'B' ? model_row_offset : -model_row_offset, socket_height + (p.reverse_mount ? 0 : module_thickness)];
    const model_rotation = p.mcu_3dmodel_xyz_rotation || [0, p.reverse_mount ? 0 : 180, p.side === 'B' ? 180 : 0];
    const mcu_3dmodel = `
    (model ${JSON.stringify(p.mcu_3dmodel_filename)}
      (offset (xyz ${model_offset[0]} ${model_offset[1]} ${model_offset[2]}))
      (scale (xyz ${p.mcu_3dmodel_xyz_scale[0]} ${p.mcu_3dmodel_xyz_scale[1]} ${p.mcu_3dmodel_xyz_scale[2]}))
      (rotate (xyz ${model_rotation[0]} ${model_rotation[1]} ${model_rotation[2]}))
    )
    `

    return `
    ${''/* Controller*/}
    ${common_top}
    ${socket_rows}
    ${p.include_extra_pins && (!p.reversible || (p.reversible && p.only_required_jumpers)) ? extra_pins : ''}
    ${p.include_extra_pins && p.reversible && p.only_required_jumpers ? extra_pins_reversible : ''}
    ${p.reversible && p.show_instructions ? instructions : ''}
    ${p.mcu_3dmodel_filename ? mcu_3dmodel : ''}
  )

  ${''/* Traces */}
  ${p.reversible && p.include_traces ? traces : ''}
    `;
  }
}
;module.exports = ((original) => {
    const defaults = {"mcu_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/Nice_Nano_V2.step","mcu_3dmodel_xyz_offset":"","mcu_3dmodel_xyz_rotation":""};
    const bossVariant = null;
    const chocV2 = "${KIPRJMOD}/models/boardstudio/koktoh/Choc_V2_Red.step";
    return {
      ...original,
      params: {...original.params, ...defaults},
      body: p => {
        // The bundled extension crosses the PCB with a 5 mm outer diameter.
        const extensionDiameter = 5;
        if (false && p.drill < extensionDiameter
          && p.tp_extension_3dmodel_filename === defaults.tp_extension_3dmodel_filename
          && !p.tp_extension_3dmodel_xyz_scale
          && !p.tp_extension_3dmodel_xyz_rotation
          && !p.tp_extension_3dmodel_xyz_offset) {
          throw new Error('The bundled trackpoint extension requires a center drill of at least 5 mm. Select a compatible extension model or drill size.');
        }
        if (false && !p.choc_v1_support) {
          p = {...p};
          if (!p.choc_v2_support) {
            for (const key of Object.keys(defaults)) {
              if (p[key] === defaults[key]) { p[key] = ''; }
            }
          } else {
            if (p.switch_3dmodel_filename === defaults.switch_3dmodel_filename) {
              const automatic = !p.switch_3dmodel_xyz_rotation && !p.switch_3dmodel_xyz_offset
                && p.switch_3dmodel_xyz_scale.every(value => value === 1);
              const postDiameter = 4.8;
              if (automatic && (!p.include_stabilizer_pad || p.oval_stabilizer_pad
                || (p.center_hole_diameter > 0 && p.center_hole_diameter < postDiameter))) {
                throw new Error('The bundled Choc V2 model requires the round stabilizer hole and a center drill of at least 4.8 mm. Use a matching model or transform for modified hardware.');
              }
              p.switch_3dmodel_filename = chocV2;
              p.switch_3dmodel_xyz_rotation ||= p.side === 'F' ? [180, 0, 0] : [0, 180, 0];
              p.switch_3dmodel_xyz_offset ||= [0, 0, -p.pcb_thickness];
            }
            if (p.keycap_3dmodel_filename === defaults.keycap_3dmodel_filename) {
              p.keycap_3dmodel_filename = '';
            }
          }
        }
        if (bossVariant && p.include_bosses && p.reset_switch_3dmodel_filename === defaults.reset_switch_3dmodel_filename) {
          p = {...p, reset_switch_3dmodel_filename: bossVariant};
        }
        return original.body(p);
      }
    };
  })(module.exports);

return module.exports; })(),
  "ceoloide/mcu_supermini_nrf52840": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Copyright (c) 2023 Marco Massarelli
//
// SPDX-License-Identifier: CC-BY-NC-SA-4.0
//
// To view a copy of this license, visit http://creativecommons.org/licenses/by-nc-sa/4.0/
//
// Author: @infused-kim + @ceoloide improvements
//
// Description:
//  A single-side or reversible footprint for the SuperMini NRF52840 (or any
//  pro-micro compatible controller) that uses jumpers instead of two socket
//  rows to be reversible.
//
//  Note that the extra pins are *ONLY* compatible with SuperMini NRF52840
//  board and not with the Nice!Nano, which has pins in a slightly different position.
//
//  This is a re-implementation of the promicro_pretty footprint made popular
//  by @benvallack, adapted to match the SuperMini extra pin position.
//
// Pinout and schematics:
//  https://wiki.icbbuy.com/doku.php?id=developmentboard:nrf52840
//
// Params:
//    side: default is F for Front
//      the side on which to place the single-side footprint and designator, either F or B
//    reversible: default is false
//      if true, the footprint will be placed on both sides so that the PCB can be
//      reversible
//    reverse_mount: default is false (MCU facing away from the PCB)
//      if true, the sockets will be oriented so that the MCU faces the PCB (RAW / B+ is the
//      top left pin). This is the most common mounting option for the SuperMini.
//      When set to false, the pads will match the datasheet and assume the MCU faces away
//      from the PCB (RAW / B+ is the top right pin).
//    include_traces: default is true
//      if true it will include traces that connect the jumper pads to the vias
//      and the through-holes for the MCU
//    include_extra_pins: default is false
//      if true and if not reversible it will include SuperMini extra pin sockets (P1.01,
//      P1.02, P1.07). If the board is reversible, extra pins will be added only if the
//      flag `only_required_jumpers` is set to true, and only P1.01 and P1.02 will be
//      added to avoid overlap of P1.07.
//    only_required_jumpers: default is false
//      if true, it will only place jumpers on the first 4 rows of pins, which can't be
//      reversed in firmware, i.e. RAW and P1, GND and P0, GND and RST, GND and VCC.
//    use_rectangular_jumpers: default is false
//      if true, it will replace chevron-style jumpers with rectangual pads
//    via_size: default is 0.8
//      allows to define the size of the via. Not recommended below 0.56 (JLCPCB minimum),
//      or above 0.8 (KiCad default), to avoid overlap or DRC errors.
//    via_drill: default is 0.4
//      allows to define the size of the drill. Not recommended below 0.3 (JLCPCB minimum),
//      or above 0.4 (KiCad default), to avoid overlap or DRC errors. 
//    Pxx_label, VCC_label, RAW_label, GND_label, RST_label: default is ''
//      allows to override the label for each pin
//    mcu_3dmodel_filename: default is ''
//      Allows you to specify the path to a 3D model STEP or WRL file to be
//      used when rendering the PCB. Use the ${VAR_NAME} syntax to point to
//      a KiCad configured path.
//    mcu_3dmodel_xyz_offset: default is [0, 0, 0]
//      xyz offset (in mm), used to adjust the position of the 3d model
//      relative the footprint.
//    mcu_3dmodel_xyz_scale: default is [1, 1, 1]
//      xyz scale, used to adjust the size of the 3d model relative to its
//      original size.
//    mcu_3dmodel_xyz_rotation: default is [0, 0, 0]
//      xyz rotation (in degrees), used to adjust the orientation of the 3d
//      model relative the footprint.
//
// @infused-kim's improvements:
//  - Use real traces instead of pads, which gets rid of hundreds of DRC errors.
//  - Leave more space between the vias to allow easier routing through the middle
//    of the footprint
//
// @ceoloide's improvements:
//  - Move vias closer to the pads to clear up more space for silkscreen
//  - Add ability to use rectangular jumpers instead of chevron-style
//  - Add ability to control via size, to free up space for routing if needed
//  - Add ability to only have required jumpers and let the rest be handled in firmware
//  - Add single side (non-reversible) support
//  - Add ability to mount with MCU facing towards or away from PCB
//  - Add ability to show silkscreen labels on both sides for single side footprint
//  - Add extra pins (P1.01, P1.02, P1.07) when footprint is single-side or reversible
//    (only required jumpers)
//  - Upgrade to KiCad 8
//  - Adapted to match the SuperMini extra pin positions
//
// # Placement and soldering of jumpers
//
// The reversible footprint is meant to be used with jumpers on the
// OPPOSITE side of where the SuperMini (or pro-micro compatible board) is
// installed. The silkscreen labels will also match the board when read on
// the opposite side. This is to have all jumpers and components to solder on
// the same side, and be able to read the correct labels of the MCU to do
// tests with a multimeter.
//
// # Further credits
//
// The original footprint was created from scratch by @infused-kim, but was based on the ideas from
// these other footprints:
//
// https://github.com/Albert-IV/ergogen-contrib/blob/main/src/footprints/promicro_pretty.js
// https://github.com/50an6xy06r6n/keyboard_reversible.pretty

module.exports = {
  params: {
    designator: 'MCU',
    side: 'F',
    reversible: false,
    reverse_mount: false,
    include_traces: true,
    include_extra_pins: false,
    // Inverted jumper placement is unsupported; true is rejected during generation.
    invert_jumpers_position: false,
    only_required_jumpers: false,
    use_rectangular_jumpers: false,
    pcb_thickness: 1.6,
    via_size: 0.8, // JLCPC min is 0.56 for 1-2 layer boards, KiCad defaults to 0.8
    via_drill: 0.4, // JLCPC min is 0.3 for 1-2 layer boards, KiCad defaults to 0.4

    show_instructions: true,
    show_silk_labels: true,
    show_silk_labels_on_both_sides: false,
    show_via_labels: true,

    mcu_3dmodel_filename: '',
    mcu_3dmodel_xyz_offset: [0, 0, 0],
    mcu_3dmodel_xyz_rotation: [0, 0, 0],
    mcu_3dmodel_xyz_scale: [1, 1, 1],

    RAW_label: '',
    GND_label: '',
    RST_label: '',
    VCC_label: '',
    P21_label: '',
    P20_label: '',
    P19_label: '',
    P18_label: '',
    P15_label: '',
    P14_label: '',
    P16_label: '',
    P10_label: '',

    P1_label: '',
    P0_label: '',
    P2_label: '',
    P3_label: '',
    P4_label: '',
    P5_label: '',
    P6_label: '',
    P7_label: '',
    P8_label: '',
    P9_label: '',

    P101_label: '',
    P102_label: '',
    P107_label: '',

    RAW: { type: 'net', value: 'RAW' },
    GND: { type: 'net', value: 'GND' },
    RST: { type: 'net', value: 'RST' },
    VCC: { type: 'net', value: 'VCC' },
    P21: { type: 'net', value: 'P21' },
    P20: { type: 'net', value: 'P20' },
    P19: { type: 'net', value: 'P19' },
    P18: { type: 'net', value: 'P18' },
    P15: { type: 'net', value: 'P15' },
    P14: { type: 'net', value: 'P14' },
    P16: { type: 'net', value: 'P16' },
    P10: { type: 'net', value: 'P10' },

    P1: { type: 'net', value: 'P1' },
    P0: { type: 'net', value: 'P0' },
    P2: { type: 'net', value: 'P2' },
    P3: { type: 'net', value: 'P3' },
    P4: { type: 'net', value: 'P4' },
    P5: { type: 'net', value: 'P5' },
    P6: { type: 'net', value: 'P6' },
    P7: { type: 'net', value: 'P7' },
    P8: { type: 'net', value: 'P8' },
    P9: { type: 'net', value: 'P9' },

    P101: { type: 'net', value: 'P101' },
    P102: { type: 'net', value: 'P102' },
    P107: { type: 'net', value: 'P107' },
  },
  body: p => {
    if (p.invert_jumpers_position) {
      throw new Error('invert_jumpers_position is unsupported; set it to false and use the documented jumper assembly.');
    }
    const get_pin_net_name = (p, pin_name) => {
      return p[pin_name].name;
    };

    const get_pin_net_str = (p, pin_name) => {
      return p[pin_name].str;
    };

    const get_pin_label_override = (p, pin_name) => {
      let prop_name = `${pin_name}_label`;
      return p[prop_name];
    };

    const get_pin_label = (p, pin_name) => {
      let label = get_pin_label_override(p, pin_name);
      if (label == '') {
        label = get_pin_net_name(p, pin_name);
      }

      if (label === undefined) {
        label = '""';
      }

      return label;
    };

    const pin_names = [
      // The pin matrix below assumes PCB is mounted with the MCU
      // facing away from the PCB (reverse_mount = false) on the
      // Front side. It should be inverted for reverse_mount = true
      // or when mounted on teh Back
      ['P1', 'RAW'],
      ['P0', 'GND'],
      ['GND', 'RST'],
      ['GND', 'VCC'],
      ['P2', 'P21'],
      ['P3', 'P20'],
      ['P4', 'P19'],
      ['P5', 'P18'],
      ['P6', 'P15'],
      ['P7', 'P14'],
      ['P8', 'P16'],
      ['P9', 'P10'],
    ]

    const gen_traces_row = (row_num) => {
      const net_left = p[pin_names[row_num][invert_pins ? 1 : 0]].index
      const net_right = p[pin_names[row_num][invert_pins ? 0 : 1]].index
      const traces = `
  (segment (start ${p.eaxy((p.use_rectangular_jumpers ? 4.58 : 4.775), -12.7 + (row_num * 2.54))}) (end ${p.eaxy(3.4, -12.7 + (row_num * 2.54))}) (width 0.25) (layer "F.Cu") (net ${net_right}))
  (segment (start ${p.eaxy((p.use_rectangular_jumpers ? -4.58 : -4.775), -12.7 + (row_num * 2.54))}) (end ${p.eaxy(-3.4, -12.7 + (row_num * 2.54))}) (width 0.25) (layer "F.Cu") (net ${net_left}))

  (segment (start ${p.eaxy(-7.62, -12.7 + (row_num * 2.54))}) (end ${p.eaxy(-5.5, -12.7 + (row_num * 2.54))}) (width 0.25) (layer "F.Cu") (net ${p.local_net(24 - row_num).index}))
  (segment (start ${p.eaxy(-7.62, -12.7 + (row_num * 2.54))}) (end ${p.eaxy(-5.5, -12.7 + (row_num * 2.54))}) (width 0.25) (layer "B.Cu") (net ${p.local_net(24 - row_num).index}))
  (segment (start ${p.eaxy(5.5, -12.7 + (row_num * 2.54))}) (end ${p.eaxy(7.62, -12.7 + (row_num * 2.54))}) (width 0.25) (layer "F.Cu") (net ${p.local_net(1 + row_num).index}))
  (segment (start ${p.eaxy(7.62, -12.7 + (row_num * 2.54))}) (end ${p.eaxy(5.5, -12.7 + (row_num * 2.54))}) (width 0.25) (layer "B.Cu") (net ${p.local_net(1 + row_num).index}))

  (segment (start ${p.eaxy(-2.604695, 0.23 + (row_num * 2.54) - 12.7)}) (end ${p.eaxy(3.17, 0.23 + (row_num * 2.54) - 12.7)}) (width 0.25) (layer "B.Cu") (net ${net_right}))
  (segment (start ${p.eaxy(-4.775, 0 + (row_num * 2.54) - 12.7)}) (end ${p.eaxy(-4.425305, 0 + (row_num * 2.54) - 12.7)}) (width 0.25) (layer "B.Cu") (net ${net_right}))
  (segment (start ${p.eaxy(-3.700305, 0.725 + (row_num * 2.54) - 12.7)}) (end ${p.eaxy(-3.099695, 0.725 + (row_num * 2.54) - 12.7)}) (width 0.25) (layer "B.Cu") (net ${net_right}))
  (segment (start ${p.eaxy(-4.425305, 0 + (row_num * 2.54) - 12.7)}) (end ${p.eaxy(-3.700305, 0.725 + (row_num * 2.54) - 12.7)}) (width 0.25) (layer "B.Cu") (net ${net_right}))
  (segment (start ${p.eaxy(-3.099695, 0.725 + (row_num * 2.54) - 12.7)}) (end ${p.eaxy(-2.604695, 0.23 + (row_num * 2.54) - 12.7)}) (width 0.25) (layer "B.Cu") (net ${net_right}))

  (segment (start ${p.eaxy(4.775, 0 + (row_num * 2.54) - 12.7)}) (end ${p.eaxy(4.425305, 0 + (row_num * 2.54) - 12.7)}) (width 0.25) (layer "B.Cu") (net ${net_left}))
  (segment (start ${p.eaxy(2.594695, -0.22 + (row_num * 2.54) - 12.7)}) (end ${p.eaxy(-3.18, -0.22 + (row_num * 2.54) - 12.7)}) (width 0.25) (layer "B.Cu") (net ${net_left}))
  (segment (start ${p.eaxy(4.425305, 0 + (row_num * 2.54) - 12.7)}) (end ${p.eaxy(3.700305, -0.725 + (row_num * 2.54) - 12.7)}) (width 0.25) (layer "B.Cu") (net ${net_left}))
  (segment (start ${p.eaxy(3.700305, -0.725 + (row_num * 2.54) - 12.7)}) (end ${p.eaxy(3.099695, -0.725 + (row_num * 2.54) - 12.7)}) (width 0.25) (layer "B.Cu") (net ${net_left}))
  (segment (start ${p.eaxy(3.099695, -0.725 + (row_num * 2.54) - 12.7)}) (end ${p.eaxy(2.594695, -0.22 + (row_num * 2.54) - 12.7)}) (width 0.25) (layer "B.Cu") (net ${net_left}))
        `

      return traces
    }

    const gen_traces = () => {
      let traces = '';
      for (let i = 0; i < 12; i++) {
        if (i < 4 || !p.only_required_jumpers) {
          let row_traces = gen_traces_row(i)
          traces += row_traces
        }
      }

      return traces
    }

    const invert_pins = (p.side == 'B' && !p.reverse_mount && !p.reversible) || (p.side == 'F' && p.reverse_mount && !p.reversible) || (!p.reverse_mount && p.reversible)

    const gen_socket_row = (row_num, pin_name_left, pin_name_right, show_via_labels, show_silk_labels) => {
      const row_offset_y = 2.54 * row_num

      const socket_hole_num_left = 24 - row_num
      const socket_hole_num_right = 1 + row_num
      const via_num_left = 124 - row_num
      const via_num_right = 101 + row_num

      const net_left = get_pin_net_str(p, pin_name_left)
      const net_right = get_pin_net_str(p, pin_name_right)
      const via_label_left = get_pin_label(p, pin_name_left)
      const via_label_right = get_pin_label(p, pin_name_right)

      // These are the silkscreen labels that will be printed on the PCB.
      // If the footprint is reversible, they will be aligned with the pins
      // on the opposite side of where the MCU board is mounted.
      const net_silk_front_left = via_label_left
      const net_silk_front_right = via_label_right
      const net_silk_back_left = via_label_right
      const net_silk_back_right = via_label_left

      let socket_row_base = `
    ${''/* Socket Holes */}
    (pad "${socket_hole_num_left}" thru_hole circle (at -7.62 ${-12.7 + row_offset_y} ${p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${p.reversible && (row_num < 4 || !p.only_required_jumpers) ? p.local_net(socket_hole_num_left).str : net_left})
    (pad "${socket_hole_num_right}" thru_hole circle (at 7.62 ${-12.7 + row_offset_y} ${p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${p.reversible && (row_num < 4 || !p.only_required_jumpers) ? p.local_net(socket_hole_num_right).str : net_right})
      `
      let socket_row_vias = `
    ${''/* Inside VIAS */}
    (pad "${via_num_left}" thru_hole circle (at -3.4 ${-12.7 + row_offset_y} ${p.r}) (size ${p.via_size} ${p.via_size}) (drill ${p.via_drill}) (layers "*.Cu" "*.Mask") ${net_left})
    (pad "${via_num_right}" thru_hole circle (at 3.4 ${-12.7 + row_offset_y} ${p.r}) (size ${p.via_size} ${p.via_size}) (drill ${p.via_drill}) (layers "*.Cu" "*.Mask") ${net_right})
      `

      let socket_row_rectangular_jumpers = `
    ${''/* Jumper Pads - Front Left */}
    (pad "${socket_hole_num_left}" smd rect (at -5.48 ${-12.7 + row_offset_y} ${p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${p.local_net(socket_hole_num_left).str})
    (pad "${via_num_left}" smd rect (at -4.58 ${-12.7 + row_offset_y} ${p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${net_left})

    ${''/* Jumper Pads - Front Right */}
    (pad "${via_num_right}" smd rect (at 4.58 ${-12.7 + row_offset_y} ${p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${net_right})
    (pad "${socket_hole_num_right}" smd rect (at 5.48 ${-12.7 + row_offset_y} ${p.r}) (size 0.6 1.2) (layers "F.Cu" "F.Paste" "F.Mask") ${p.local_net(socket_hole_num_right).str})

    ${''/* Jumper Pads - Back Left */}
    (pad "${socket_hole_num_left}" smd rect (at -5.48 ${-12.7 + row_offset_y} ${p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${p.local_net(socket_hole_num_left).str})
    (pad "${via_num_right}" smd rect (at -4.58 ${-12.7 + row_offset_y} ${p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${net_right})

    ${''/* Jumper Pads - Back Right */}
    (pad "${via_num_left}" smd rect (at 4.58 ${-12.7 + row_offset_y} ${p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${net_left})
    (pad "${socket_hole_num_right}" smd rect (at 5.48 ${-12.7 + row_offset_y} ${p.r}) (size 0.6 1.2) (layers "B.Cu" "B.Paste" "B.Mask") ${p.local_net(socket_hole_num_right).str})
        `

      let socket_row_chevron_jumpers = `
    ${''/* Jumper Pads - Front Left */}
    (pad "${socket_hole_num_left}" smd custom (at -5.5 ${-12.7 + row_offset_y} ${p.r}) (size 0.2 0.2) (layers "F.Cu" "F.Paste" "F.Mask") ${p.local_net(socket_hole_num_left).str}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.5 -0.625) (xy -0.25 -0.625) (xy 0.25 0) (xy -0.25 0.625) (xy -0.5 0.625)
      ) (width 0) (fill yes))
    ))
    (pad "${via_num_left}" smd custom (at -4.775 ${-12.7 + row_offset_y} ${p.r}) (size 0.2 0.2) (layers "F.Cu" "F.Paste" "F.Mask") ${net_left}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.65 -0.625) (xy 0.5 -0.625) (xy 0.5 0.625) (xy -0.65 0.625) (xy -0.15 0)
      ) (width 0) (fill yes))
    ))

    ${''/* Jumper Pads - Front Right */}
    (pad "${via_num_right}" smd custom (at 4.775 ${-12.7 + row_offset_y} ${180 + p.r}) (size 0.2 0.2) (layers "F.Cu" "F.Paste" "F.Mask") ${net_right}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.65 -0.625) (xy 0.5 -0.625) (xy 0.5 0.625) (xy -0.65 0.625) (xy -0.15 0)
      ) (width 0) (fill yes))
    ))
    (pad "${socket_hole_num_right}" smd custom (at 5.5 ${-12.7 + row_offset_y} ${180 + p.r}) (size 0.2 0.2) (layers "F.Cu" "F.Paste" "F.Mask") ${p.local_net(socket_hole_num_right).str}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.5 -0.625) (xy -0.25 -0.625) (xy 0.25 0) (xy -0.25 0.625) (xy -0.5 0.625)
      ) (width 0) (fill yes))
    ))

    ${''/* Jumper Pads - Back Left */}
    (pad "${socket_hole_num_left}" smd custom (at -5.5 ${-12.7 + row_offset_y} ${p.r}) (size 0.2 0.2) (layers "B.Cu" "B.Paste" "B.Mask") ${p.local_net(socket_hole_num_left).str}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.5 0.625) (xy -0.25 0.625) (xy 0.25 0) (xy -0.25 -0.625) (xy -0.5 -0.625)
      ) (width 0) (fill yes))
    ))

    (pad "${via_num_right}" smd custom (at -4.775 ${-12.7 + row_offset_y} ${p.r}) (size 0.2 0.2) (layers "B.Cu" "B.Paste" "B.Mask") ${net_right}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.65 0.625) (xy 0.5 0.625) (xy 0.5 -0.625) (xy -0.65 -0.625) (xy -0.15 0)
      ) (width 0) (fill yes))
    ))

    ${''/* Jumper Pads - Back Right */}
    (pad "${via_num_left}" smd custom (at 4.775 ${-12.7 + row_offset_y} ${180 + p.r}) (size 0.2 0.2) (layers "B.Cu" "B.Paste" "B.Mask") ${net_left}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.65 0.625) (xy 0.5 0.625) (xy 0.5 -0.625) (xy -0.65 -0.625) (xy -0.15 0)
      ) (width 0) (fill yes))
    ))
    (pad "${socket_hole_num_right}" smd custom (at 5.5 ${-12.7 + row_offset_y} ${180 + p.r}) (size 0.2 0.2) (layers "B.Cu" "B.Paste" "B.Mask") ${p.local_net(socket_hole_num_right).str}
      (zone_connect 2)
      (options (clearance outline) (anchor rect))
      (primitives
        (gr_poly (pts
          (xy -0.5 0.625) (xy -0.25 0.625) (xy 0.25 0) (xy -0.25 -0.625) (xy -0.5 -0.625)
      ) (width 0) (fill yes))
    ))
        `
      let socket_row = socket_row_base;
      if (p.reversible && (row_num < 4 || !p.only_required_jumpers)) {
        socket_row += socket_row_vias;
        if (p.use_rectangular_jumpers) {
          socket_row += socket_row_rectangular_jumpers
        } else {
          socket_row += socket_row_chevron_jumpers
        }
      }
      if (show_silk_labels == true) {
        if (p.reversible || p.show_silk_labels_on_both_sides || p.side == 'F') {
          // Silkscreen labels - front
          if (row_num != 10 
            || !p.include_extra_pins
            || (p.include_extra_pins && invert_pins && !p.reversible) 
            || (p.include_extra_pins && !p.only_required_jumpers && p.reversible)
          ) {
            socket_row += `
    (fp_text user "${net_silk_front_left}" (at -${p.reversible && (row_num < 4 || !p.only_required_jumpers) ? (net_silk_front_left.length > 2 ? 1.45 : 2.04) : 4.47} ${-12.7 + row_offset_y} ${p.r}) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)))
    )
            `
          }
          if (row_num != 10 
            || !p.include_extra_pins
            || (p.include_extra_pins && !invert_pins && !p.reversible) 
            || (p.include_extra_pins && !p.only_required_jumpers && p.reversible)
          ) {
            socket_row += `
    (fp_text user "${net_silk_front_right}" (at ${p.reversible && (row_num < 4 || !p.only_required_jumpers) ? (net_silk_front_right.length > 2 ? 1.45 : 2.04) : 4.47} ${-12.7 + row_offset_y} ${p.r}) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)))
    )
            `
          }
        }
        if (p.reversible|| p.show_silk_labels_on_both_sides || p.side == 'B') {
          // Silkscreen labels - back
          if (row_num != 10
            || !p.include_extra_pins
            || (p.include_extra_pins && !invert_pins && !p.reversible) 
            || (p.include_extra_pins && !p.only_required_jumpers && p.reversible)
          ) {
            socket_row += `
    (fp_text user "${net_silk_back_left}" (at ${p.reversible ? '-' : ''}${p.reversible && (row_num < 4 || !p.only_required_jumpers) ? (net_silk_back_left.length > 2 ? 1.45 : 2.04) : 4.47} ${-12.7 + row_offset_y} ${p.r}) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify mirror))
    )
            `
          }
          if (row_num != 10
            || !p.include_extra_pins
            || (p.include_extra_pins && invert_pins && !p.reversible) 
            || (p.include_extra_pins && !p.only_required_jumpers && p.reversible)
          ) {
            socket_row += `
    (fp_text user "${net_silk_back_right}" (at ${p.reversible ? '' : '-'}${p.reversible && (row_num < 4 || !p.only_required_jumpers) ? (net_silk_back_right.length > 2 ? 1.45 : 2.04) : 4.47} ${-12.7 + row_offset_y} ${p.r}) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify mirror))
    )
            `
          }
        }
      }

      if (show_via_labels && (p.reversible && (row_num < 4 || !p.only_required_jumpers))) {
        socket_row += `
    ${''/* Via Labels - Front */}
    (fp_text user "${via_label_left}" (at -3.262 ${-13.5 + row_offset_y} ${p.r}) (layer "F.Fab")
      (effects (font (size 0.5 0.5) (thickness 0.08)))
    )
    (fp_text user "${via_label_right}" (at 3.262 ${-13.5 + row_offset_y} ${p.r}) (layer "F.Fab")
      (effects (font (size 0.5 0.5) (thickness 0.08)))
    )

    ${''/* Via Labels - Back */}
    (fp_text user "${via_label_left}" (at -3.262 ${-13.5 + row_offset_y} ${180 + p.r}) (layer "B.Fab")
      (effects (font (size 0.5 0.5) (thickness 0.08)) (justify mirror))
    )
    (fp_text user "${via_label_right}" (at 3.262 ${-13.5 + row_offset_y} ${180 + p.r}) (layer "B.Fab")
      (effects (font (size 0.5 0.5) (thickness 0.08)) (justify mirror))
    )
          `
      }

      return socket_row
    }
    const gen_socket_rows = (show_via_labels, show_silk_labels) => {

      let socket_rows = '';
      for (let i = 0; i < pin_names.length; i++) {
        let pin_name_left = pin_names[i][invert_pins ? 1 : 0]
        let pin_name_right = pin_names[i][invert_pins ? 0 : 1]

        const socket_row = gen_socket_row(
          i, pin_name_left, pin_name_right,
          show_via_labels, show_silk_labels
        )

        socket_rows += socket_row
      }
      // Socket silkscreen
      // P1 / D1 / P0.06 is marked according to orientation
      if (show_silk_labels == true) {
        if (p.reversible || p.show_silk_labels_on_both_sides || p.side == 'F') {
          socket_rows += `
    (fp_line (start 6.29 -14.03) (end 8.95 -14.03) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 6.29 -14.03) (end 6.29 16.57) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 6.29 16.57) (end 8.95 16.57) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -6.29 -14.03) (end -6.29 16.57) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 8.95 -14.03) (end 8.95 16.57) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -8.95 -14.03) (end -6.29 -14.03) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -8.95 -14.03) (end -8.95 16.57) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -8.95 16.57) (end -6.29 16.57) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start ${invert_pins ? '' : '-'}6.29 -11.43) (end ${invert_pins ? '' : '-'}8.95 -11.43) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
            `
        }
        if (p.reversible || p.show_silk_labels_on_both_sides || p.side == 'B') {
          socket_rows += `
    (fp_line (start -6.29 -14.03) (end -8.95 -14.03) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -6.29 -14.03) (end -6.29 16.57) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -6.29 16.57) (end -8.95 16.57) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -8.95 -14.03) (end -8.95 16.57) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 8.95 -14.03) (end 6.29 -14.03) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 8.95 -14.03) (end 8.95 16.57) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 8.95 16.57) (end 6.29 16.57) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 6.29 -14.03) (end 6.29 16.57) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start ${invert_pins ? (p.reversible ? '-' : '') : (p.reversible ? '' : '-')}8.95 -11.43) (end ${invert_pins ? (p.reversible ? '-' : '') : (p.reversible ? '' : '-')}6.29 -11.43) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
          `
        }
      }
      return socket_rows
    }

    const common_top = `
  (footprint "ceoloide:mcu_supermini_nrf52840"
    (layer "${p.side}.Cu")
    ${p.at}
    (property "Reference" "${p.ref}"
      (at 0 -15 ${p.r})
      (layer "${p.side}.SilkS")
      ${p.ref_hide}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (attr exclude_from_pos_files exclude_from_bom)

    ${''/* USB socket outline */}
    (fp_line (start 3.556 -17.20) (end 3.556 -16.51) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -3.81 -16.51) (end -3.81 -17.20) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -3.81 -17.20) (end 3.556 -17.20) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))


  ${''/* Controller outline */}
    (fp_line (start -8.89 -16.51) (end 8.89 -16.51) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -8.89 -16.51) (end -8.89 16.57) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start 8.89 -16.51) (end 8.89 16.57) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -8.89 16.57) (end 8.89 16.57) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    `;

    const instructions = `
    (fp_text user "R hand back side (M${!p.reverse_mount ? '↑' : '↓'})" (at 0 -15.245 ${p.r}) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (fp_text user "L hand back side (M${!p.reverse_mount ? '↑' : '↓'})" (at 0 -15.245 ${p.r}) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify mirror))
    )
    `

    const socket_rows = gen_socket_rows(
      p.show_via_labels, p.show_silk_labels
    )
    const traces = gen_traces()

    const extra_pins = `
    (pad "25" thru_hole circle (at ${invert_pins ? '' : '-'}4.54 ${-13.2548 + 25.4} ${p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${p.P101})
    (pad "26" thru_hole circle (at ${invert_pins ? '' : '-'}2.00 ${-13.2548 + 25.4} ${p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${p.P102})
    (pad "27" thru_hole circle (at ${invert_pins ? '-' : ''}0.54 ${-13.2548 + 25.4} ${p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${p.P107})
    `
    const extra_pins_reversible = `
    (pad "25" thru_hole circle (at ${invert_pins ? '' : '-'}4.54 ${-13.2548 + 25.4} ${p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${p.P101})
    (pad "26" thru_hole circle (at ${invert_pins ? '' : '-'}2.00 ${-13.2548 + 25.4} ${p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${p.P102})
    (pad "28" thru_hole circle (at ${invert_pins ? '-' : ''}4.54 ${-13.2548 + 25.4} ${p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${p.P101})
    (pad "29" thru_hole circle (at ${invert_pins ? '-' : ''}2.00 ${-13.2548 + 25.4} ${p.r}) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") ${p.P102})
    `

    // Reversible pin rows require back mounting, regardless of footprint layer.
    const socket_height = 5;
    const module_thickness = 1.6;
    const module_center_x = 16.5;
    const module_center_y = 8.89;
    const flip_layer = p.reversible && p.side === 'F';
    const mounting_back = p.reversible || p.side === 'B';
    const model_height = socket_height + (p.reverse_mount ? 0 : module_thickness);
    const model_offset = p.mcu_3dmodel_xyz_offset || [
      (mounting_back ? 1 : -1) * (p.reverse_mount ? -module_center_y : module_center_y),
      (p.side === 'B' ? -1 : 1) * module_center_x,
      flip_layer ? -p.pcb_thickness - model_height : model_height,
    ];
    const model_rotation = p.mcu_3dmodel_xyz_rotation || [
      (p.reverse_mount !== flip_layer) ? 180 : 0,
      0,
      p.side === 'B' ? -90 : 90,
    ];
    const mcu_3dmodel = `
    (model ${JSON.stringify(p.mcu_3dmodel_filename)}
      (offset (xyz ${model_offset[0]} ${model_offset[1]} ${model_offset[2]}))
      (scale (xyz ${p.mcu_3dmodel_xyz_scale[0]} ${p.mcu_3dmodel_xyz_scale[1]} ${p.mcu_3dmodel_xyz_scale[2]}))
      (rotate (xyz ${model_rotation[0]} ${model_rotation[1]} ${model_rotation[2]}))
    )
    `

    return `
    ${''/* Controller*/}
    ${common_top}
    ${socket_rows}
    ${p.include_extra_pins && !p.reversible ? extra_pins : ''}
    ${p.include_extra_pins && p.reversible && p.only_required_jumpers ? extra_pins_reversible : ''}
    ${p.reversible && p.show_instructions ? instructions : ''}
    ${p.mcu_3dmodel_filename ? mcu_3dmodel : ''}
  )

  ${''/* Traces */}
  ${p.reversible && p.include_traces ? traces : ''}
    `;
  }
}

;module.exports = ((original) => {
    const defaults = {"mcu_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/tsuki/nrf52840.step","mcu_3dmodel_xyz_offset":"","mcu_3dmodel_xyz_rotation":""};
    const bossVariant = null;
    const chocV2 = "${KIPRJMOD}/models/boardstudio/koktoh/Choc_V2_Red.step";
    return {
      ...original,
      params: {...original.params, ...defaults},
      body: p => {
        // The bundled extension crosses the PCB with a 5 mm outer diameter.
        const extensionDiameter = 5;
        if (false && p.drill < extensionDiameter
          && p.tp_extension_3dmodel_filename === defaults.tp_extension_3dmodel_filename
          && !p.tp_extension_3dmodel_xyz_scale
          && !p.tp_extension_3dmodel_xyz_rotation
          && !p.tp_extension_3dmodel_xyz_offset) {
          throw new Error('The bundled trackpoint extension requires a center drill of at least 5 mm. Select a compatible extension model or drill size.');
        }
        if (false && !p.choc_v1_support) {
          p = {...p};
          if (!p.choc_v2_support) {
            for (const key of Object.keys(defaults)) {
              if (p[key] === defaults[key]) { p[key] = ''; }
            }
          } else {
            if (p.switch_3dmodel_filename === defaults.switch_3dmodel_filename) {
              const automatic = !p.switch_3dmodel_xyz_rotation && !p.switch_3dmodel_xyz_offset
                && p.switch_3dmodel_xyz_scale.every(value => value === 1);
              const postDiameter = 4.8;
              if (automatic && (!p.include_stabilizer_pad || p.oval_stabilizer_pad
                || (p.center_hole_diameter > 0 && p.center_hole_diameter < postDiameter))) {
                throw new Error('The bundled Choc V2 model requires the round stabilizer hole and a center drill of at least 4.8 mm. Use a matching model or transform for modified hardware.');
              }
              p.switch_3dmodel_filename = chocV2;
              p.switch_3dmodel_xyz_rotation ||= p.side === 'F' ? [180, 0, 0] : [0, 180, 0];
              p.switch_3dmodel_xyz_offset ||= [0, 0, -p.pcb_thickness];
            }
            if (p.keycap_3dmodel_filename === defaults.keycap_3dmodel_filename) {
              p.keycap_3dmodel_filename = '';
            }
          }
        }
        if (bossVariant && p.include_bosses && p.reset_switch_3dmodel_filename === defaults.reset_switch_3dmodel_filename) {
          p = {...p, reset_switch_3dmodel_filename: bossVariant};
        }
        return original.body(p);
      }
    };
  })(module.exports);

return module.exports; })(),
  "ceoloide/mounting_hole_npth": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Copyright (c) 2023 Marco Massarelli
//
// SPDX-License-Identifier: MIT
//
// To view a copy of this license, visit https://opensource.org/license/mit/
//
// Author: @ceoloide
//
// Description:
//  A non-plated, mechanical through-hole to be used for screws, standoffs or
//  other mounting options. Both the drill size and pad size can be independently
//  defined.
//
// Params:
//    side: default is F for Front
//      the side on which to place the footprint and designator, either F or B
//    hole_size: default is 2.2mm for M2 screws
//      the size of the pad around the hole
//    hole_drill: default is 2.2mm for M2 screws
//      the size of the hole to drill

module.exports = {
  params: {
    designator: 'MH',
    side: 'F',
    hole_size: '2.2',
    hole_drill: '2.2',
  },
  body: p => `
  (footprint "ceoloide:mounting_hole_npth"
    (layer "${p.side}.Cu")
    ${p.at}
    (property "Reference" "${p.ref}"
      (at 0 2.55 ${p.r})
      (layer "${p.side}.SilkS")
      ${p.ref_hide}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (pad "" np_thru_hole circle
      (at 0 0 ${p.r})
      (size ${p.hole_size} ${p.hole_size})
      (drill ${p.hole_drill})
      (layers "*.Cu" "*.Mask")
    )
  )
  `
}

return module.exports; })(),
  "ceoloide/mounting_hole_plated": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Copyright (c) 2023 Marco Massarelli
//
// SPDX-License-Identifier: CC-BY-NC-SA-4.0
//
// To view a copy of this license, visit http://creativecommons.org/licenses/by-nc-sa/4.0/
//
// Author: @infused-kim + @ceoloide improvements
//
// Description:
//  A simple mounting hole with plated rim.
//
//  Note that some fine details may be lost depending on scale and fab capabilities.
//
// Params:
//    side: default is F for Front
//      The side on which to place the mounting hole.
//    outline: default is 0.8 (mm)
//      The width of the gold rim around the hole, in mm.
//    drill: default is 2.2 (mm / M2 screw)
//      The actual size for the hole. If drill_y is defined, it represents the
//      horizontal width of an oval hole, in mm.
//    drill_y: default is 0 (mm)
//      The vertical height of an oval hole, in mm.
//    include_courtyard: default is true
//      if true it will include the part courtyard
//
//  @ceoloide's improvements:
//    - Upgrade to KiCad 8
//    - Minor footprint restructure

module.exports = {
  params: {
    designator: 'H',
    side: 'F',
    outline: 0.8,
    drill: 2.2,
    drill_y: 0,
    include_courtyard: true,
  },
  body: p => {
    if (p.drill_y == 0) {
      p.drill_y = p.drill
    }

    const size_x = p.drill + p.outline * 2;
    const size_y = p.drill_y + p.outline * 2;

    const courtyard_offset = 0.25;
    const courtyard_x = size_x / 2 + courtyard_offset;
    const courtyard_y = size_y / 2 + courtyard_offset;

    const top = `
  (footprint "ceoloide:mounting_hole_plated"
    (layer "${p.side}.Cu")
    ${p.at}
    (property "Reference" "${p.ref}"
      (at 0 3 ${p.r})
      (layer "${p.side}.SilkS")
      ${p.ref_hide}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    `

    const pad_circle = `
    (pad "" thru_hole circle (at 0 0 ${p.r}) (size ${size_x} ${size_y}) (drill ${p.drill}) (layers "*.Cu" "*.Mask"))
    `
    const courtyard_circle = `
    (fp_circle (center 0 0) (end -${courtyard_x} 0) (layer "F.CrtYd") (stroke (width 0.05) (type solid)) (fill none))
    (fp_circle (center 0 0) (end -${courtyard_x} 0) (layer "B.CrtYd") (stroke (width 0.05) (type solid)) (fill none))
    `
    const pad_oval = `
    (pad "" thru_hole oval (at 0 0 ${p.r}) (size ${size_x} ${size_y}) (drill oval ${p.drill} ${p.drill_y}) (layers "*.Cu" "*.Mask"))
    `
    const courtyard_oval = `
    (fp_line (start ${courtyard_x} -${courtyard_y}) (end ${courtyard_x} ${courtyard_y}) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -${courtyard_x} -${courtyard_y}) (end -${courtyard_x} ${courtyard_y}) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -${courtyard_x} ${courtyard_y}) (end ${courtyard_x} ${courtyard_y}) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -${courtyard_x} -${courtyard_y}) (end ${courtyard_x} -${courtyard_y}) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -${courtyard_x} ${courtyard_y}) (end ${courtyard_x} ${courtyard_y}) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -${courtyard_x} ${courtyard_y}) (end -${courtyard_x} -${courtyard_y}) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -${courtyard_x} -${courtyard_y}) (end ${courtyard_x} -${courtyard_y}) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start ${courtyard_x} ${courtyard_y}) (end ${courtyard_x} -${courtyard_y}) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    `

    const bottom = `
  )
    `

    let final = top
    if (size_x == size_y) {
      final += pad_circle
      final += courtyard_circle
    } else {
      final += pad_oval
      final += courtyard_oval
    }

    final += bottom

    return final
  }
}
return module.exports; })(),
  "ceoloide/power_switch_smd_side": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Copyright (c) 2023 Marco Massarelli
//
// SPDX-License-Identifier: CC-BY-NC-SA-4.0
//
// To view a copy of this license, visit http://creativecommons.org/licenses/by-nc-sa/4.0/
//
// Authors: @infused-kim + @ceoloide improvements
//
// Description:
//  SMD side-operated on-off switch, compatible with Alps SSSS811101 as sold on
//  Typeractive.xyz and LCSC. These switches are shorter than the height of hotswap sockets,
//  so they can be mounted on the same side.
//
//  Should be compatible with:
//    - G-Switch MK-12C02-G015 (untested)
//    - PCM12SMTR (untested)
//
// Datasheet:
//   https://cdn.shopify.com/s/files/1/0618/5674/3655/files/ALPS-SSSS811101.pdf?v=1670451309
//
// Nets:
//    from: corresponds to pin 1 on the Front and 3 on the back
//    to: corresponds to pin 2 on both sides
//
// Params:
//    side: default is F for Front
//      the side on which to place the single-side footprint and designator, either F (Front)
//      or B (Back)
//    reversible: default is false
//      if true, it will include pads on both Front and Back to make the footprint reversible
//    invert_behavior: default is false
//      if true, pin 3 will connect to the "from" net, and if false it will connect to pin 1,
//      effectively inverting the behavior of the switch.
//    include_silkscreen: default is true
//      if true it will include silkscreen markings, which is recommended to know which side
//      connects Bat+ to RAW.
//    include_courtyard: default is false
//      if true it will include the courtyard around the component
//    switch_3dmodel_filename: default is ''
//      Allows you to specify the path to a 3D model STEP or WRL file to be
//      used when rendering the PCB. Use the ${VAR_NAME} syntax to point to
//      a KiCad configured path.
//    switch_3dmodel_xyz_offset: default is [0, 0, 0]
//      xyz offset (in mm), used to adjust the position of the 3d model
//      relative the footprint.
//    switch_3dmodel_xyz_scale: default is [1, 1, 1]
//      xyz scale, used to adjust the size of the 3d model relative to its
//      original size.
//    switch_3dmodel_xyz_rotation: default is [0, 0, 0]
//      xyz rotation (in degrees), used to adjust the orientation of the 3d
//      model relative the footprint.
//
// @ceoloide's improvements:
//  - Add ability to set text on both sides
//  - Add ability to adjust font thickness and size
//  - Add ability to invert switch behavior / pin connections
//  - Invert behavior on opposite layer to maintain consistency
//  - Add on/off silkscreen to aid operation
//  - Upgrade to KiCad 8

module.exports = {
  params: {
    designator: 'PWR',
    side: 'F',
    reversible: false,
    invert_behavior: false,
    include_silkscreen: true,
    include_courtyard: false,
    switch_3dmodel_filename: '',
    switch_3dmodel_xyz_offset: [0, 0, 0],
    switch_3dmodel_xyz_rotation: [0, 0, 0],
    switch_3dmodel_xyz_scale: [1, 1, 1],
    from: { type: 'net', value: 'BAT_P' },
    to: { type: 'net', value: 'RAW' },
  },
  body: p => {
    const common_start = `
  (footprint "ceoloide:power_switch_smd_side"
    (layer "${p.side}.Cu")
    ${p.at}
    (property "Reference" "${p.ref}"
      (at -3.6 0 ${-90 + p.r})
      (layer "${p.side}.SilkS")
      ${p.ref_hide}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (attr smd)
    `
    const silkscreen_front = `
    (fp_text user "ON" (at 0 ${p.invert_behavior ? '-' : ''}5 ${p.r}) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (fp_text user "OFF" (at 0 ${p.invert_behavior ? '' : '-'}5 ${p.r}) (layer "F.SilkS")
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (fp_line (start 0.415 -3.45) (end -0.375 -3.45) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -0.375 3.45) (end 0.415 3.45) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -1.425 1.6) (end -1.425 -0.1) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 1.425 2.85) (end 1.425 -2.85) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -1.425 -1.4) (end -1.425 -1.6) (layer "F.SilkS") (stroke (width 0.12) (type solid)))
    `
    const silkscreen_back = `
    (fp_text user "${p.ref}" (at -3.5 0 ${90 + p.r}) (layer "B.SilkS") ${p.ref_hide}
      (effects (font (size 1 1) (thickness 0.15)) (justify mirror))
    )
    (fp_text user "ON" (at 0 ${p.invert_behavior ? '-' : ''}5 ${p.r}) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify mirror))
    )
    (fp_text user "OFF" (at 0 ${p.invert_behavior ? '' : '-'}5 ${p.r}) (layer "B.SilkS")
      (effects (font (size 1 1) (thickness 0.15)) (justify mirror))
    )
    (fp_line (start -1.425 1.4) (end -1.425 1.6) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 0.415 3.45) (end -0.375 3.45) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -0.375 -3.45) (end 0.415 -3.45) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start -1.425 -1.6) (end -1.425 0.1) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    (fp_line (start 1.425 -2.85) (end 1.425 2.85) (layer "B.SilkS") (stroke (width 0.12) (type solid)))
    `
    const courtyard_front = `
    (fp_line (start 1.795 4.4) (end -2.755 4.4) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 1.795 1.65) (end 1.795 4.4) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 3.095 1.65) (end 1.795 1.65) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 3.095 -1.65) (end 3.095 1.65) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 1.795 -1.65) (end 3.095 -1.65) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 1.795 -4.4) (end 1.795 -1.65) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -2.755 -4.4) (end 1.795 -4.4) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -2.755 4.4) (end -2.755 -4.4) (layer "F.CrtYd") (stroke (width 0.05) (type solid)))
    `
    const courtyard_back = `
    (fp_line (start -2.755 -4.4) (end -2.755 4.4) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 3.095 1.65) (end 3.095 -1.65) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 1.795 1.65) (end 3.095 1.65) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 1.795 -4.4) (end -2.755 -4.4) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 1.795 -1.65) (end 1.795 -4.4) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 3.095 -1.65) (end 1.795 -1.65) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start 1.795 4.4) (end 1.795 1.65) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    (fp_line (start -2.755 4.4) (end 1.795 4.4) (layer "B.CrtYd") (stroke (width 0.05) (type solid)))
    `

    const pads_front = `
    (fp_line (start -1.305 -3.35) (end -1.305 3.35) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 1.295 -3.35) (end -1.305 -3.35) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 1.295 3.35) (end 1.295 -3.35) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -1.305 3.35) (end 1.295 3.35) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.595 0.1) (end 1.295 0.1) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.645 0.15) (end 2.595 0.1) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.845 0.35) (end 2.645 0.15) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.845 1.2) (end 2.845 0.35) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.645 1.4) (end 2.845 1.2) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 1.345 1.4) (end 2.645 1.4) (layer "F.Fab") (stroke (width 0.1) (type solid)))
    (pad "" smd rect (at 1.125 -3.65 ${90 + p.r}) (size 1 0.8) (layers "F.Cu" "F.Paste" "F.Mask"))
    (pad "" smd rect (at -1.085 -3.65 ${90 + p.r}) (size 1 0.8) (layers "F.Cu" "F.Paste" "F.Mask"))
    (pad "" smd rect (at -1.085 3.65 ${90 + p.r}) (size 1 0.8) (layers "F.Cu" "F.Paste" "F.Mask"))
    (pad "" smd rect (at 1.125 3.65 ${90 + p.r}) (size 1 0.8) (layers "F.Cu" "F.Paste" "F.Mask"))
    (pad "1" smd rect (at -1.735 2.25 ${90 + p.r}) (size 0.7 1.5) (layers "F.Cu" "F.Paste" "F.Mask") ${p.invert_behavior ? '' : p.from.str})
    (pad "2" smd rect (at -1.735 -0.75 ${90 + p.r}) (size 0.7 1.5) (layers "F.Cu" "F.Paste" "F.Mask") ${p.to.str})
    (pad "3" smd rect (at -1.735 -2.25 ${90 + p.r}) (size 0.7 1.5) (layers "F.Cu" "F.Paste" "F.Mask") ${p.invert_behavior ? p.from.str : ''})
    `
    const pads_back = `
    (fp_line (start 2.595 -0.1) (end 1.295 -0.1) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -1.305 3.35) (end -1.305 -3.35) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.645 -0.15) (end 2.595 -0.1) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.845 -1.2) (end 2.845 -0.35) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 1.345 -1.4) (end 2.645 -1.4) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.845 -0.35) (end 2.645 -0.15) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 2.645 -1.4) (end 2.845 -1.2) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 1.295 -3.35) (end 1.295 3.35) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start 1.295 3.35) (end -1.305 3.35) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (fp_line (start -1.305 -3.35) (end 1.295 -3.35) (layer "B.Fab") (stroke (width 0.1) (type solid)))
    (pad "" smd rect (at -1.085 -3.65 ${270 + p.r}) (size 1 0.8) (layers "B.Cu" "B.Paste" "B.Mask"))
    (pad "" smd rect (at 1.125 -3.65 ${270 + p.r}) (size 1 0.8) (layers "B.Cu" "B.Paste" "B.Mask"))
    (pad "" smd rect (at -1.085 3.65 ${270 + p.r}) (size 1 0.8) (layers "B.Cu" "B.Paste" "B.Mask"))
    (pad "" smd rect (at 1.125 3.65 ${270 + p.r}) (size 1 0.8) (layers "B.Cu" "B.Paste" "B.Mask"))
    (pad "1" smd rect (at -1.735 -2.25 ${270 + p.r}) (size 0.7 1.5) (layers "B.Cu" "B.Paste" "B.Mask") ${p.invert_behavior ? p.from.str : ''})
    (pad "2" smd rect (at -1.735 0.75 ${270 + p.r}) (size 0.7 1.5) (layers "B.Cu" "B.Paste" "B.Mask") ${p.to.str})
    (pad "3" smd rect (at -1.735 2.25 ${270 + p.r}) (size 0.7 1.5) (layers "B.Cu" "B.Paste" "B.Mask") ${p.invert_behavior ? '' : p.from.str})
    `
    const common_end = `
    (pad "" np_thru_hole circle (at 0.025 -1.5 ${90 + p.r}) (size 0.9 0.9) (drill 0.9) (layers "*.Cu" "*.Mask"))
    (pad "" np_thru_hole circle (at 0.025 1.5 ${90 + p.r}) (size 0.9 0.9) (drill 0.9) (layers "*.Cu" "*.Mask"))
  )
    `

    const switch_3dmodel = `
    (model ${JSON.stringify(p.switch_3dmodel_filename)}
      (offset (xyz ${p.switch_3dmodel_xyz_offset[0]} ${p.switch_3dmodel_xyz_offset[1]} ${p.switch_3dmodel_xyz_offset[2]}))
      (scale (xyz ${p.switch_3dmodel_xyz_scale[0]} ${p.switch_3dmodel_xyz_scale[1]} ${p.switch_3dmodel_xyz_scale[2]}))
      (rotate (xyz ${p.switch_3dmodel_xyz_rotation[0]} ${p.switch_3dmodel_xyz_rotation[1]} ${p.switch_3dmodel_xyz_rotation[2]}))
    )
    `

    let final = common_start;
    if (p.side == "F" || p.reversible) {
      final += pads_front
      if (p.include_silkscreen) {
        final += silkscreen_front
      }
      if (p.include_courtyard) {
        final += courtyard_front
      }
    }
    if (p.side == "B" || p.reversible) {
      final += pads_back
      if (p.include_silkscreen) {
        final += silkscreen_back
      }
      if (p.include_courtyard) {
        final += courtyard_back
      }
    }

    if (p.switch_3dmodel_filename) {
      final += switch_3dmodel
    }

    final += common_end;
    return final;
  }
}

;module.exports = ((original) => {
    const defaults = {"switch_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/Switch_Power.step","switch_3dmodel_xyz_rotation":[-90,0,-90]};
    const bossVariant = null;
    const chocV2 = "${KIPRJMOD}/models/boardstudio/koktoh/Choc_V2_Red.step";
    return {
      ...original,
      params: {...original.params, ...defaults},
      body: p => {
        // The bundled extension crosses the PCB with a 5 mm outer diameter.
        const extensionDiameter = 5;
        if (false && p.drill < extensionDiameter
          && p.tp_extension_3dmodel_filename === defaults.tp_extension_3dmodel_filename
          && !p.tp_extension_3dmodel_xyz_scale
          && !p.tp_extension_3dmodel_xyz_rotation
          && !p.tp_extension_3dmodel_xyz_offset) {
          throw new Error('The bundled trackpoint extension requires a center drill of at least 5 mm. Select a compatible extension model or drill size.');
        }
        if (false && !p.choc_v1_support) {
          p = {...p};
          if (!p.choc_v2_support) {
            for (const key of Object.keys(defaults)) {
              if (p[key] === defaults[key]) { p[key] = ''; }
            }
          } else {
            if (p.switch_3dmodel_filename === defaults.switch_3dmodel_filename) {
              const automatic = !p.switch_3dmodel_xyz_rotation && !p.switch_3dmodel_xyz_offset
                && p.switch_3dmodel_xyz_scale.every(value => value === 1);
              const postDiameter = 4.8;
              if (automatic && (!p.include_stabilizer_pad || p.oval_stabilizer_pad
                || (p.center_hole_diameter > 0 && p.center_hole_diameter < postDiameter))) {
                throw new Error('The bundled Choc V2 model requires the round stabilizer hole and a center drill of at least 4.8 mm. Use a matching model or transform for modified hardware.');
              }
              p.switch_3dmodel_filename = chocV2;
              p.switch_3dmodel_xyz_rotation ||= p.side === 'F' ? [180, 0, 0] : [0, 180, 0];
              p.switch_3dmodel_xyz_offset ||= [0, 0, -p.pcb_thickness];
            }
            if (p.keycap_3dmodel_filename === defaults.keycap_3dmodel_filename) {
              p.keycap_3dmodel_filename = '';
            }
          }
        }
        if (bossVariant && p.include_bosses && p.reset_switch_3dmodel_filename === defaults.reset_switch_3dmodel_filename) {
          p = {...p, reset_switch_3dmodel_filename: bossVariant};
        }
        return original.body(p);
      }
    };
  })(module.exports);

return module.exports; })(),
  "ceoloide/reset_switch_smd_side": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Copyright (c) 2023 Marco Massarelli
//
// SPDX-License-Identifier: MIT
//
// To view a copy of this license, visit https://opensource.org/license/mit/
//
// Author: @ceoloide
//
// Description:
//   SMD side-operated momentary switch, compatible with Panasonic EVQ-PU[A|C|J|L]02K
//   as sold on Typeractive.xyz https://typeractive.xyz/products/reset-button
//   keebd.com https://keebd.com/products/reset-button-panasonic
//   and LCSC. These switches are shorter than the height of hotswap
//   sockets, so they can be mounted on the same side.
//
// Datasheet:
//   https://cdn.shopify.com/s/files/1/0618/5674/3655/files/PANASONIC-EVQPUC02K.pdf?v=1670451309
//
// Nets:
//    from: corresponds to pin 1 and 3
//    to: corresponds to pin 2 and 4
//
// Params:
//    side: default is F for Front
//      the side on which to place the single-side footprint and designator, either F or B
//    reversible: default is false
//      if true, it will include pads on both Front and Back to make the footprint reversible
//    include_bosses: default is false
//      if true it will include two mechanical NPTH for the switches that have them (EVQ-PUC02K
//      and EVQ-PUL02K)
//    include_silkscreen: default is true
//      if true it will include silkscreen markings
//    include_courtyard: default is false
//      if true it will include the part courtyard
//    reset_switch_3dmodel_filename: default is ''
//      Allows you to specify the path to a 3D model STEP or WRL file to be
//      used when rendering the PCB. Use the ${VAR_NAME} syntax to point to
//      a KiCad configured path.
//    reset_switch_3dmodel_xyz_offset: default is [0, 0, 0]
//      xyz offset (in mm), used to adjust the position of the 3d model
//      relative the footprint.
//    reset_switch_3dmodel_xyz_scale: default is [1, 1, 1]
//      xyz scale, used to adjust the size of the 3d model relative to its
//      original size.
//    reset_switch_3dmodel_xyz_rotation: default is [0, 0, 0]
//      xyz rotation (in degrees), used to adjust the orientation of the 3d
//      model relative the footprint.

module.exports = {
  params: {
    designator: 'RST',
    side: 'F',
    reversible: false,
    include_bosses: false,
    include_silkscreen: true,
    include_courtyard: false,
    reset_switch_3dmodel_filename: '',
    reset_switch_3dmodel_xyz_offset: [0, 0, 0],
    reset_switch_3dmodel_xyz_rotation: [0, 0, 0],
    reset_switch_3dmodel_xyz_scale: [1, 1, 1],
    from: { type: 'net', value: 'GND' },
    to: { type: 'net', value: 'RST' },
  },
  body: p => {
    const common_start = `
  (footprint "ceoloide:reset_switch_smd_side"
    (layer "${p.side}.Cu")
    ${p.at}
    (property "Reference" "${p.ref}"
      (at 0 0 ${p.r})
      (layer "${p.side}.SilkS")
      ${p.ref_hide}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (attr smd)
    (fp_line (start -2.35 -1.75) (end -2.35 1.75) (stroke (width 0.1) (type solid)) (layer "Dwgs.User"))
    (fp_line (start -2.35 -1.75) (end 2.35 -1.75) (stroke (width 0.1) (type solid)) (layer "Dwgs.User"))
    (fp_line (start -2.35 1.75) (end 2.35 1.75) (stroke (width 0.1) (type solid)) (layer "Dwgs.User"))
    (fp_line (start -1.3 -2.75) (end -1.3 -1.75) (stroke (width 0.1) (type solid)) (layer "Dwgs.User"))
    (fp_line (start -1.3 -2.75) (end 1.3 -2.75) (stroke (width 0.1) (type solid)) (layer "Dwgs.User"))
    (fp_line (start 1.3 -2.75) (end 1.3 -1.75) (stroke (width 0.1) (type solid)) (layer "Dwgs.User"))
    (fp_line (start 2.35 -1.75) (end 2.35 1.75) (stroke (width 0.1) (type solid)) (layer "Dwgs.User"))
    `
    const silkscreen_front = `
    (fp_line (start -2.35 -1.5) (end -2.35 -1.75) (stroke (width 0.1) (type solid)) (layer "F.SilkS"))
    (fp_line (start -2.35 1.5) (end -2.35 1.75) (stroke (width 0.1) (type solid)) (layer "F.SilkS"))
    (fp_line (start -2.1 -1.75) (end -2.35 -1.75) (stroke (width 0.1) (type solid)) (layer "F.SilkS"))
    (fp_line (start -2.1 1.75) (end -2.35 1.75) (stroke (width 0.1) (type solid)) (layer "F.SilkS"))
    (fp_line (start 2.1 -1.75) (end 2.35 -1.75) (stroke (width 0.1) (type solid)) (layer "F.SilkS"))
    (fp_line (start 2.1 1.75) (end 2.35 1.75) (stroke (width 0.1) (type solid)) (layer "F.SilkS"))
    (fp_line (start 2.35 -1.5) (end 2.35 -1.75) (stroke (width 0.1) (type solid)) (layer "F.SilkS"))
    (fp_line (start 2.35 1.5) (end 2.35 1.75) (stroke (width 0.1) (type solid)) (layer "F.SilkS"))
    `
    const silkscreen_back = `
    (fp_line (start -2.35 -1.5) (end -2.35 -1.75) (stroke (width 0.1) (type solid)) (layer "B.SilkS"))
    (fp_line (start -2.35 1.5) (end -2.35 1.75) (stroke (width 0.1) (type solid)) (layer "B.SilkS"))
    (fp_line (start -2.1 -1.75) (end -2.35 -1.75) (stroke (width 0.1) (type solid)) (layer "B.SilkS"))
    (fp_line (start -2.1 1.75) (end -2.35 1.75) (stroke (width 0.1) (type solid)) (layer "B.SilkS"))
    (fp_line (start 2.1 -1.75) (end 2.35 -1.75) (stroke (width 0.1) (type solid)) (layer "B.SilkS"))
    (fp_line (start 2.1 1.75) (end 2.35 1.75) (stroke (width 0.1) (type solid)) (layer "B.SilkS"))
    (fp_line (start 2.35 -1.5) (end 2.35 -1.75) (stroke (width 0.1) (type solid)) (layer "B.SilkS"))
    (fp_line (start 2.35 1.5) (end 2.35 1.75) (stroke (width 0.1) (type solid)) (layer "B.SilkS"))
    `
    const pads_front = `
    (pad "1" smd rect (at 2.625 -0.85 ${180 + p.r}) (size 1.55 1) (layers "F.Cu" "F.Paste" "F.Mask") ${p.from.str})
    (pad "2" smd rect (at 2.625 0.85 ${180 + p.r}) (size 1.55 1) (layers "F.Cu" "F.Paste" "F.Mask") ${p.to.str})
    (pad "3" smd rect (at -2.625 -0.85 ${180 + p.r}) (size 1.55 1) (layers "F.Cu" "F.Paste" "F.Mask") ${p.from.str})
    (pad "4" smd rect (at -2.625 0.85 ${180 + p.r}) (size 1.55 1) (layers "F.Cu" "F.Paste" "F.Mask") ${p.to.str})
    `
    const pads_back = `
    (pad "1" smd rect (at -2.625 -0.85 ${180 + p.r}) (size 1.55 1) (layers "B.Cu" "B.Paste" "B.Mask") ${p.from.str})
    (pad "2" smd rect (at -2.625 0.85 ${180 + p.r}) (size 1.55 1) (layers "B.Cu" "B.Paste" "B.Mask") ${p.to.str})
    (pad "3" smd rect (at 2.625 -0.85 ${180 + p.r}) (size 1.55 1) (layers "B.Cu" "B.Paste" "B.Mask") ${p.from.str})
    (pad "4" smd rect (at 2.625 0.85 ${180 + p.r}) (size 1.55 1) (layers "B.Cu" "B.Paste" "B.Mask") ${p.to.str})
    `
    const courtyard_front = `
    (fp_rect (start 2.36 1.75) (end -2.36 -1.75) (stroke (width 0.05) (type solid)) (fill none) (layer "F.CrtYd"))
    `
    const courtyard_back = `
    (fp_rect (start 2.36 1.75) (end -2.36 -1.75) (stroke (width 0.05) (type solid)) (fill none) (layer "B.CrtYd"))
    `
    const bosses = `
    (pad "" np_thru_hole circle (at 0 -1.375 ${180 + p.r}) (size 0.75 0.75) (drill 0.75) (layers "*.Cu" "*.Mask"))
    (pad "" np_thru_hole circle (at 0 1.375 ${180 + p.r}) (size 0.75 0.75) (drill 0.75) (layers "*.Cu" "*.Mask"))
    `

    // Use the selected side's model frame; explicit transforms remain authoritative.
    const model_rotation = p.reset_switch_3dmodel_xyz_rotation || [0, 0, p.side === 'B' ? 180 : 0];
    const reset_switch_3dmodel = `
    (model ${JSON.stringify(p.reset_switch_3dmodel_filename)}
      (offset (xyz ${p.reset_switch_3dmodel_xyz_offset[0]} ${p.reset_switch_3dmodel_xyz_offset[1]} ${p.reset_switch_3dmodel_xyz_offset[2]}))
      (scale (xyz ${p.reset_switch_3dmodel_xyz_scale[0]} ${p.reset_switch_3dmodel_xyz_scale[1]} ${p.reset_switch_3dmodel_xyz_scale[2]}))
      (rotate (xyz ${model_rotation[0]} ${model_rotation[1]} ${model_rotation[2]}))
    )
    `

    const common_end = `
  )
    `

    let final = common_start;
    if (p.include_bosses) {
      final += bosses
    }
    if (p.side == "F" || p.reversible) {
      final += pads_front
      if (p.include_silkscreen) {
        final += silkscreen_front
      }
      if (p.include_courtyard) {
        final += courtyard_front
      }
    }
    if (p.side == "B" || p.reversible) {
      final += pads_back
      if (p.include_silkscreen) {
        final += silkscreen_back
      }
      if (p.include_courtyard) {
        final += courtyard_back
      }
    }
    if (p.reset_switch_3dmodel_filename) {
      final += reset_switch_3dmodel
    }

    final += common_end;

    return final;
  }
}

;module.exports = ((original) => {
    const defaults = {"reset_switch_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/kicad/Panasonic_EVQPUJ_EVQPUA.step","reset_switch_3dmodel_xyz_rotation":""};
    const bossVariant = "${KIPRJMOD}/models/boardstudio/kicad/Panasonic_EVQPUL_EVQPUC.step";
    const chocV2 = "${KIPRJMOD}/models/boardstudio/koktoh/Choc_V2_Red.step";
    return {
      ...original,
      params: {...original.params, ...defaults},
      body: p => {
        // The bundled extension crosses the PCB with a 5 mm outer diameter.
        const extensionDiameter = 5;
        if (false && p.drill < extensionDiameter
          && p.tp_extension_3dmodel_filename === defaults.tp_extension_3dmodel_filename
          && !p.tp_extension_3dmodel_xyz_scale
          && !p.tp_extension_3dmodel_xyz_rotation
          && !p.tp_extension_3dmodel_xyz_offset) {
          throw new Error('The bundled trackpoint extension requires a center drill of at least 5 mm. Select a compatible extension model or drill size.');
        }
        if (false && !p.choc_v1_support) {
          p = {...p};
          if (!p.choc_v2_support) {
            for (const key of Object.keys(defaults)) {
              if (p[key] === defaults[key]) { p[key] = ''; }
            }
          } else {
            if (p.switch_3dmodel_filename === defaults.switch_3dmodel_filename) {
              const automatic = !p.switch_3dmodel_xyz_rotation && !p.switch_3dmodel_xyz_offset
                && p.switch_3dmodel_xyz_scale.every(value => value === 1);
              const postDiameter = 4.8;
              if (automatic && (!p.include_stabilizer_pad || p.oval_stabilizer_pad
                || (p.center_hole_diameter > 0 && p.center_hole_diameter < postDiameter))) {
                throw new Error('The bundled Choc V2 model requires the round stabilizer hole and a center drill of at least 4.8 mm. Use a matching model or transform for modified hardware.');
              }
              p.switch_3dmodel_filename = chocV2;
              p.switch_3dmodel_xyz_rotation ||= p.side === 'F' ? [180, 0, 0] : [0, 180, 0];
              p.switch_3dmodel_xyz_offset ||= [0, 0, -p.pcb_thickness];
            }
            if (p.keycap_3dmodel_filename === defaults.keycap_3dmodel_filename) {
              p.keycap_3dmodel_filename = '';
            }
          }
        }
        if (bossVariant && p.include_bosses && p.reset_switch_3dmodel_filename === defaults.reset_switch_3dmodel_filename) {
          p = {...p, reset_switch_3dmodel_filename: bossVariant};
        }
        return original.body(p);
      }
    };
  })(module.exports);

return module.exports; })(),
  "ceoloide/reset_switch_tht_top": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Copyright (c) 2023 Marco Massarelli
//
// SPDX-License-Identifier: MIT
//
// To view a copy of this license, visit https://opensource.org/license/mit/
//
// Author: @ceoloide
//
// Description:
//  A through-hole top-actuated momentary switch, the same used by the Corne keyboard and
//  compatible with "PTS636 S[L|M]43 LFS" tactile switches sold on LCSC.
//
// Datasheet:
//  https://datasheet.lcsc.com/lcsc/2110271930_C-K-PTS636SM43LFS_C2689636.pdf
//
// Params:
//    side: default is F for Front
//      the side on which to place the single-side footprint and designator, either F or B
//    reversible: default is false
//      if true, it will include silkscreen on both Front and Back, if silkscreen is included.
//      because the footprint is through-hole and because it's only connecting RST to GND, the
//      pads are reversible in any case.
//    include_silkscreen: default is true
//      if true it will include silkscreen markings

module.exports = {
  params: {
    designator: 'RST', // for Button
    side: 'F',
    reversible: false,
    include_silkscreen: true,
    from: { type: 'net', value: 'GND' },
    to: { type: 'net', value: 'RST' },
  },
  body: p => {
    // Match the PTS636 THT drawing and preserve the 0.35 mm annular ring.
    const pin_spacing = 6.4;
    const pin_drill = 1.2;
    const pin_pad = 1.9;
    const common_start = `
  (footprint "ceoloide:reset_switch_tht_top"
    (layer "${p.side}.Cu")
    ${p.at}
    (property "Reference" "${p.ref}"
      (at 0 2.55 ${90 + p.r})
      (layer "${p.side}.SilkS")
      ${p.ref_hide}
      (effects (font (size 1 1) (thickness 0.15)))
    )
        `
    const silkscreen_front = `
    (fp_text user "RST" (at 0 0 ${p.r}) (layer "F.SilkS") (effects (font (size 1 1) (thickness 0.15))))
    (fp_line (start -3 1.75) (end 3 1.75) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 3 1.75) (end 3 1.5) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start -3 1.75) (end -3 1.5) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start -3 -1.75) (end -3 -1.5) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start -3 -1.75) (end 3 -1.75) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 3 -1.75) (end 3 -1.5) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
        `
    const silkscreen_back = `
    (fp_text user "RST" (at 0 0 ${p.r}) (layer "B.SilkS") (effects (font (size 1 1) (thickness 0.15)) (justify mirror)))
    (fp_line (start 3 1.5) (end 3 1.75) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 3 1.75) (end -3 1.75) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start -3 1.75) (end -3 1.5) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start -3 -1.5) (end -3 -1.75) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start -3 -1.75) (end 3 -1.75) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 3 -1.75) (end 3 -1.5) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
        `
    const common_end = `
    (pad "2" thru_hole circle (at ${-pin_spacing / 2} 0 ${p.r}) (size ${pin_pad} ${pin_pad}) (drill ${pin_drill}) (layers "*.Cu" "*.Mask") ${p.from.str})
    (pad "1" thru_hole circle (at ${pin_spacing / 2} 0 ${p.r}) (size ${pin_pad} ${pin_pad}) (drill ${pin_drill}) (layers "*.Cu" "*.Mask") ${p.to.str})
  )
        `
    let final = common_start;
    if (p.side == "F" || p.reversible) {
      if (p.include_silkscreen) {
        final += silkscreen_front
      }
    }
    if (p.side == "B" || p.reversible) {
      if (p.include_silkscreen) {
        final += silkscreen_back
      }
    }
    final += common_end;
    return final;
  }
}

return module.exports; })(),
  "ceoloide/rotary_encoder_ec11_ec12": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
/*
Copyright (c) 2023 Marco Massarelli

SPDX-License-Identifier: MIT

To view a copy of this license, visit https://opensource.org/license/mit/

Author: @ceoloide

Description:
  Reversible Alps EC11 / EC12 rotary encoder footprint with customization options to make it
  compatible with Cherry MX or Choc v1 / v2 footprints, when co-located.
  
  The Alps EC12 (EC12E2440301) has a short through-shaft model that is well suited for low
  profile keyboards, although it doesn't include a momentary switch.

Notes:
- The footprint can be co-located with a Cherry MX hotswap footprint, as long as the outermost
  hotswap pads are at most 2 mm tall, so that they won't overlap with the plate mount legs
  of the rotary encoder. The encoder mounting pads can be positioned 7.5mm instead of 5.6mm to
  avoid overlap with mounting pins.
- The footprint can be co-located with a Choc v1 / v2 hotswap footprint, as long as the encoder
  pads are positioned at least 8.254 mm. If Choc v1 is co-located with the encoder, then mounting
  pads can be positioned 8.00mm instead of 5.6mm to avoid overlap with mounting pins. Choc v2
  don't have those pins, so wouldn't need the encoder mounting pads moved.
- The footprint is inherently reversible, no solder jumper needed. Make sure to invert the pin
  assignment for pad A and C in firmware, to ensure the encoder rotation is handled consistently.

Datasheet:
  https://www.soundwell.hk/ec12-rotary-encoder.html

Params:
  side: default is F for Front
    the side on which to place the single-side footprint and designator, either F or B
  reversible: default is false
    if true, the footprint silkscreen will be placed on both sides so that the PCB can be
    reversible. Pads are all THT, and only A and C need to be reversed in software.
  include_momentary_switch_pads: default is true
    if true, the footprint will include 2 extra pads to accommodate for momentary switch pins,
    normally found in EC11-compatible rotary encoders.
  include_plate_hole_marking: default is false
    if true, will add a shape to indicate plate hole size and position for the encoder. If the
    encoder is co-located with a switch, then the regular 14mm plate hole will be slightly
    larger and work well enough.
  include_silkscreen: default is true
    if true it will include the silkscreen.
  include_plated_mounting_holes: default is true
    If true, it will include plated mounting holes and their pads, instead of just mechanical
    NPTH for the mounting pins. The advantage of NPTH is that they can be more easily co-located
    with Cherry MX mounting pins, allowing the Cherry MX hotswap pads to be 2.3 mm tall instead of
    2.0 mmm.
  mounting_holes_position: default is 5.6 (mm)
    The distance in mm from the center of the footprint where to position the side mounting
    pads. The datasheet calls for 5.7mm, but wider position can be accommodated by bending
    legs slightly, while clearing up space for switch mounting pins (i.e. 7.5mm for Cherry,
    8mm for Choc v1, see notes).
  mounting_hole_width: default is 1.5 (mm)
    The width of the mounting holes, in mm. The datasheet calls for 1.4mm rectangular, so an
    extra mm is added for the oval shape. Note that the plated pad is made 0.3mm larger.
  mounting_hole_height: default is 2.3 (mm)
    The height of the mounting holes, in mm. The datasheet calls for 2.1mm rectangular, but users
    reported a very tight fit with 2.1 mm oval holes, so the default is slightly larger. If the
    footprint is not co-located with hotswap sockets, then 2.8 mm is a safe default, as it is the
    same value used in the Sofle and other keyboards. Note that the plated pad is made 0.3mm larger.
  encoder_pads_position: default is 7.5 (mm)
    The position of the encoder pads, in mm. The datasheet calls for 7.5mm, but this makes it
    impossible for the encoder to be co-located wtih Choc v1 / v2 hotswap pads, as it would overlap.
    Increasing the value will require the encoder pins to be slightly bent. For Choc v1 / v2 use 
    8.254 as a minimum.
  signal_hole_width: default is 1 (mm)
  signal_hole_height: default is 0.5 (mm)
    Signal drill dimensions; equal dimensions produce round holes.
    Set both to 1 for the Alps EC11E15244G1 signal-hole drawing.
    Mounting slots require a separate part-specific configuration.
  encoder_3dmodel_filename: default is ''
    Allows you to specify the path to a 3D model STEP or WRL file to be
    used when rendering the PCB. Use the ${VAR_NAME} syntax to point to
    a KiCad configured path.
  encoder_3dmodel_xyz_offset: default is [0, 0, 0]
    xyz offset (in mm), used to adjust the position of the 3d model
    relative the footprint.
  encoder_3dmodel_xyz_scale: default is [1, 1, 1]
    xyz scale, used to adjust the size of the 3d model relative to its
    original size.
  encoder_3dmodel_xyz_rotation: default is [0, 0, 0]
    xyz rotation (in degrees), used to adjust the orientation of the 3d
    model relative the footprint.
*/

const SIGNAL_PAD_MARGIN = 0.6;

module.exports = {
  params: {
    designator: 'RE',
    side: 'F',
    reversible: false,
    include_momentary_switch_pads: true,
    include_plate_hole_marking: false,
    include_silkscreen: true,
    include_plated_mounting_holes: true,
    mounting_holes_position: 5.6,
    mounting_holes_height: 2.3,
    mounting_holes_width: 1.5,
    encoder_pads_position: 7.5,
    signal_hole_width: 1,
    signal_hole_height: 0.5,
    encoder_3dmodel_filename: '',
    encoder_3dmodel_xyz_offset: [0, 0, 0],
    encoder_3dmodel_xyz_rotation: [0, 0, 0],
    encoder_3dmodel_xyz_scale: [1, 1, 1],
    S1: { type: 'net', value: '' },
    S2: { type: 'net', value: '' },
    A: { type: 'net', value: 'RE_A' },
    B: { type: 'net', value: 'GND' },
    C: { type: 'net', value: 'RE_C' },
  },
  body: p => {
    const dimensions = [p.signal_hole_width, p.signal_hole_height];
    if (dimensions.some(value => !Number.isFinite(value) || value <= 0)) {
      throw new Error('Encoder signal hole dimensions must be positive numbers');
    }

    // Equal dimensions emit a round drill for part-specific mounting patterns.
    const signal_drill = p.signal_hole_width === p.signal_hole_height
      ? `${p.signal_hole_width}`
      : `oval ${p.signal_hole_width} ${p.signal_hole_height}`;
    const signal_size = `${p.signal_hole_width + SIGNAL_PAD_MARGIN} ${p.signal_hole_height + SIGNAL_PAD_MARGIN}`;

    const common_top = `
  (footprint "ceoloide:rotary_encoder_ec11_ec12"
    (layer "${p.side}.Cu")
    ${p.at}
    (property "Reference" "${p.ref}"
      (at 0 -8.5)
      (layer "${p.side}.${p.include_silkscreen ? 'SilkS' : 'Fab'}")
      ${p.ref_hide}
      (effects (font (size 1 1) (thickness 0.15))${p.side == 'B' ? ' (justify mirror)' : ''})
    )${p.reversible && !p.ref_hide ? `
    (fp_text user "${p.ref}"
      (at 0 -8.5)
      (layer "${p.side == 'B' ? 'F' : 'B'}.${p.include_silkscreen ? 'SilkS' : 'Fab'}")
      (effects (font (size 1 1) (thickness 0.15))${p.side == 'F' ? ' (justify mirror)' : ''})
    )`:''}
    (attr through_hole)
    (pad "A" thru_hole oval (at 2.5 ${p.encoder_pads_position} ${p.r}) (size ${signal_size}) (drill ${signal_drill}) (layers "*.Cu" "*.Mask") ${p.A})
    (pad "B" thru_hole oval (at 0 ${p.encoder_pads_position} ${p.r}) (size ${signal_size}) (drill ${signal_drill}) (layers "*.Cu" "*.Mask") ${p.B})
    (pad "C" thru_hole oval (at -2.5 ${p.encoder_pads_position} ${p.r}) (size ${signal_size}) (drill ${signal_drill}) (layers "*.Cu" "*.Mask") ${p.C})
    `
    const momentary_switch_pads = `
    (pad "S1" thru_hole oval (at -2.5 -7 ${p.r}) (size ${signal_size}) (drill ${signal_drill}) (layers "*.Cu" "*.Mask") ${p.S1})
    (pad "S2" thru_hole oval (at 2.5 -7 ${p.r}) (size ${signal_size}) (drill ${signal_drill}) (layers "*.Cu" "*.Mask") ${p.S2})
    `
    const plated_mp = `
    (pad "" thru_hole roundrect
      (at -${p.mounting_holes_position} 0 ${p.r - 90})
      (size ${p.mounting_holes_height+0.3} ${p.mounting_holes_width+0.3})
      (drill oval ${p.mounting_holes_height} ${p.mounting_holes_width})
      (layers "*.Cu" "*.Mask")
      (roundrect_rratio 0)
      (chamfer_ratio 0.2)
      (chamfer top_right top_left)
    )
    (pad "" thru_hole roundrect
      (at ${p.mounting_holes_position} 0 ${p.r - 90})
      (size ${p.mounting_holes_height+0.3} ${p.mounting_holes_width+0.3})
      (drill oval ${p.mounting_holes_height} ${p.mounting_holes_width})
      (layers "*.Cu" "*.Mask")
      (roundrect_rratio 0)
      (chamfer_ratio 0.2)
      (chamfer bottom_right bottom_left)
    )
    `
    const npth_mp = `
    (pad "" np_thru_hole oval
      (at -${p.mounting_holes_position} 0 ${p.r - 90})
      (size ${p.mounting_holes_height} ${p.mounting_holes_width})
      (drill oval ${p.mounting_holes_height} ${p.mounting_holes_width})
      (layers "*.Cu" "*.Mask")
    )    
    (pad "" np_thru_hole oval
      (at ${p.mounting_holes_position} 0 ${p.r - 90})
      (size ${p.mounting_holes_height} ${p.mounting_holes_width})
      (drill oval ${p.mounting_holes_height} ${p.mounting_holes_width})
      (layers "*.Cu" "*.Mask")
    )
    `
    const plate_hole = `
    (fp_rect (start -5.9 -6) (end 5.9 6) (layer "Dwgs.User") (stroke (width 0.15) (type solid)) (fill none))
    `
    const encoder_3dmodel = `
    (model ${JSON.stringify(p.encoder_3dmodel_filename)}
      (offset (xyz ${p.encoder_3dmodel_xyz_offset[0]} ${p.encoder_3dmodel_xyz_offset[1]} ${p.encoder_3dmodel_xyz_offset[2]}))
      (scale (xyz ${p.encoder_3dmodel_xyz_scale[0]} ${p.encoder_3dmodel_xyz_scale[1]} ${p.encoder_3dmodel_xyz_scale[2]}))
      (rotate (xyz ${p.encoder_3dmodel_xyz_rotation[0]} ${p.encoder_3dmodel_xyz_rotation[1]} ${p.encoder_3dmodel_xyz_rotation[2]}))
    )
    `
    const common_bottom = `
  )
    `

    let final = common_top;
    if (p.include_momentary_switch_pads) {
      final += momentary_switch_pads;
    }
    if (p.include_plated_mounting_holes) {
      final += plated_mp;
    } else {
      final += npth_mp;
    }
    if (p.include_plate_hole_marking) {
      final += plate_hole;
    }
    if (p.encoder_3dmodel_filename) {
      final += encoder_3dmodel
    }
    final += common_bottom

    return final
  }
}

return module.exports; })(),
  "ceoloide/switch_choc_v1_v2": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Copyright (c) 2023 Marco Massarelli
//
// SPDX-License-Identifier: CC-BY-NC-SA-4.0
//
// To view a copy of this license, visit http://creativecommons.org/licenses/by-nc-sa/4.0/
//
// Authors: @ergogen + @infused-kim, @ceoloide, @grazfather, @nxtk improvements
//
// Description:
//    Kailh Choc PG1350 (v1) + Kailh Choc PG1353 (v2) reversible and hotswappable footprint.
//    This includes support for LOFREE low profile POM switches (Ghost, Phantom, Wizard)
//
//    With the set defaults it will include support for choc v1 and v2 hotswap, single side
//    (Back).
//
// Nets:
//    from: corresponds to pin 1
//    to: corresponds to pin 2
//
// Params:
//    side: default is B for Back
//      the side on which to place the single-side footprint and designator, either F or B
//    pcb_thickness: default is 1.6 mm
//      Must match the PCB thickness when using bundled model transforms.
//    reversible: default is false
//      if true, the footprint will be placed on both sides so that the PCB can be
//      reversible
//    hotswap: default is true
//      if true, will include holes and pads for Kailh choc hotswap sockets
//    solder: default is false
//      if true, will include holes to solder switches (works with hotswap too)
//    hotswap_pads_same_side: false
//      if true, and if not using plated holes, it will position the pads so that nets are on
//      the same side to simplify routing.
//    include_traces_vias: default is true
//      if true it will include traces and vias when hotswap is true, footprint is reversible
//      and when no plated holes are used, to simplify routing. In the other cases it's simply
//      not needed.
//    trace_width: default is 0.200mm
//      allows to override the trace width that connects the pads. Not recommended
//      to go below 0.15mm (JLCPC min is 0.127mm), or above 0.200mm to avoid DRC errors.
//    via_size: default is 0.6
//      allows to define the size of the via. Not recommended below 0.56 (JLCPCB minimum),
//      or above 0.8 (KiCad default), to avoid overlap or DRC errors
//    via_drill: default is 0.3
//      allows to define the size of the drill. Not recommended below 0.3 (JLCPCB minimum),
//      or above 0.4 (KiCad default), to avoid overlap or DRC errors 
//    locked_traces_vias: false
//      sets the traces and vias as locked in KiCad. Locked objects may not be manipulated
//      or moved, and cannot be selected unless the Locked Items option is enabled in the
//      Selection Filter panel in KiCad. Useful for a faster workflow. If using autorouting
//      solutions like Freerouting, locking can prevent the traces and vias from being
//      replaced.
//    include_plated_holes: default is false
//      Alternate version of the footprint compatible with side, reversible, hotswap, solder options in any combination.
//      Pretty, allows for connecting ground fill zones via center hole, 
//      allows for setting nets to Choc v2 stabilizers and them for your routing needs.
//    include_stabilizer_nets: default is false
//      if true, will add adjustable nets to choc v2 plated stabilizer holes, 
//      LEFTSTAB: default is "D1"
//      RIGHTSTAB: default is "D2"
//    include_centerhole_net: default is false
//      if true, will add adjustable net to the center hole
//      CENTERHOLE: default is "GND"
//    outer_pad_width_front: default 2.6
//    outer_pad_width_back: default 2.6
//      Allows you to make the outer hotswap pads smaller to silence DRC
//      warnings when the sockets are too close to the edge cuts. It's not
//      recommended to go below 1.6mm to ensure the hotswap socket can be
//      properly soldered.
//    include_keycap: default is false
//      if true, will add mx sized keycap box around the footprint (18mm)
//    include_corner_marks: default is false
//      if true, will add corner marks to indicate plate hole size and position
//    include_choc_v1_led_cutout_marks: default is false
//      if true, will add marks for the led cutout in choc v1 switch body
//    include_choc_v2_led_cutout_marks: default is false
//      if true, will add marks for the led cutout in choc v2 switch body
//    include_stabilizer_pad: default is true
//      if true, will add a corner pad for the stabilizer leg present in some
//      Choc switches, unless choc_v2_support is false.
//    oval_stabilizer_pad: default is false
//      if false, will add an oval pad for the stabilizer leg, and a round one
//      if true. Note that the datasheet calls for a round one.
//    choc_v1_stabilizers_diameter: default is 1.9 (mm)
//      Allows you to narrow Choc v1 stabilizer / boss holes diameter for tighter fit, not recommended to set below 1.7
//    center_hole_diameter: default is 3.4 mm for choc v1 or 5.0 mm for choc v2
//      The diameter of the center hole under the switch.
//    choc_v1_support: default is true
//      if true, will add lateral stabilizer holes that are required for
//      Choc v1 footprints.
//    choc_v2_support: default is true
//      if true, will make the central hole bigger to as required for
//      Choc v2 footprints. If false it will also disable the corner stabilizer
//      pad even if include_stabilizer_pad is true.
//    keycap_height: default is 18
//      Allows you to adjust the width of the keycap outline. For example,
//      to show a 1.5u outline for easier aligning.
//    keycap_width: default is 18
//      Allows you to adjust the height of the keycap outline.
//    allow_soldermask_bridges: default is true
//      Disables 'solder mask aperture bridges items with different nets' DRC check when set to true
//      setting this option to false may be useful for debugging purposes, (applied locally to this footprint only)
//      for global setting see `allow_soldermask_bridges_in_footprints` in a kicad template
//    switch_3dmodel_filename: default is ''
//      Allows you to specify the path to a 3D model STEP or WRL file to be
//      used when rendering the PCB. Use the ${VAR_NAME} syntax to point to
//      a KiCad configured path.
//    switch_3dmodel_xyz_offset: default is [0, 0, 0]
//      xyz offset (in mm), used to adjust the position of the 3d model
//      relative the footprint.
//    switch_3dmodel_xyz_scale: default is [1, 1, 1]
//      xyz scale, used to adjust the size of the 3d model relative to its
//      original size.
//    switch_3dmodel_xyz_rotation: default is [0, 0, 0]
//      xyz rotation (in degrees), used to adjust the orientation of the 3d
//      model relative the footprint.
//    hotswap_3dmodel_filename: default is ''
//      Allows you to specify the path to a 3D model to be used when rendering
//      the PCB. Allows for paths using a configured path by using the
//      ${VAR_NAME} syntax.
//    hotswap_3dmodel_xyz_offset: default is [0, 0, 0]
//      xyz offset (in mm), used to adjust the position of the 3d model
//      relative the footprint.
//    hotswap_3dmodel_xyz_scale: default is [1, 1, 1]
//      xyz scale, used to adjust the size of the 3d model relative its
//      original size.
//    hotswap_3dmodel_xyz_rotation: default is [0, 0, 0]
//      xyz rotation (in degrees), used to adjust the orientation of the 3d
//      model relative the footprint.
//    keycap_3dmodel_filename: default is ''
//      Allows you to specify the path to a 3D model STEP or WRL file to be
//      used when rendering the PCB. Use the ${VAR_NAME} syntax to point to
//      a KiCad configured path.
//    keycap_3dmodel_xyz_offset: default is [0, 0, 0]
//      xyz offset (in mm), used to adjust the position of the 3d model
//      relative the footprint.
//    keycap_3dmodel_xyz_scale: default is [1, 1, 1]
//      xyz scale, used to adjust the size of the 3d model relative to its
//      original size.
//    keycap_3dmodel_xyz_rotation: default is [0, 0, 0]
//      xyz rotation (in degrees), used to adjust the orientation of the 3d
//      model relative the footprint.
//
// Notes:
// - Hotswap and solder can be used together. The solder holes will then be
//   added above the hotswap holes.
//
// @infused-kim's improvements:
//  - Add hotswap socket outlines
//  - Move switch corner marks from user layer to silk screen
//  - Add option to adjust keycap size outlines (to show 1.5u outline)
//  - Add option to add hotswap sockets and direct soldering holes at the
//    same time
//  - Make hotswap pads not overlap holes to fix DRC errors
//  - Fixed DRC errors "Drilled holes co-located"
//
// @ceoloide's improvements:
//  - Adjusted footprint to be Choc PG1353 (v2) compatible
//  - Add option to hide corner marks, as they interfere with hotswap silkscreen
//  - Add ability to specify board side
//  - Add ability to include stabilizer pad
//  - Add ability to use an oval stabilizer pad
//  - Add option to add routes between pads, and have pads on the same side
//  - Add option to lock routes between pads
//  - Upgrade to KiCad 8
//
// @grazfather's improvements:
//  - Add support for switch 3D model
//
// @nxtk's improvements:
//  - Add plated version, inspired by @daprice and @ssbb
//  - Add ability to adjust v1 stabilizer / boss holes
//  - Add ability to assign nets to v2 stabilizer / boss holes (useful for routing diode in place of backlight led)
//  - Add ability to set a net to central hole (useful for connecting ground fill zones)
//  - Add opposite stabilizer / boss holes when (choc_v2_support & solder & hotswap) options enabled together
//  - Change v2 stabilizer / boss holes to plated
//  - Add allow_soldermask_bridges option, which disables 'solder mask aperture bridges items with different nets' DRC check
//
// @mlilley's improvements:
//  - Add options to include marks for the led cutout in choc v1 and v2 switch bodies

module.exports = {
  params: {
    designator: 'S',
    side: 'B',
    pcb_thickness: 1.6,
    reversible: false,
    hotswap_pads_same_side: false,
    include_traces_vias: true,
    trace_width: 0.2,
    via_size: 0.6,
    via_drill: 0.3,
    locked_traces_vias: false,
    hotswap: true,
    include_plated_holes: false,
    include_stabilizer_nets: false,
    include_centerhole_net: false,
    solder: false,
    outer_pad_width_front: 2.6,
    outer_pad_width_back: 2.6,
    include_keycap: false,
    keycap_width: 18,
    keycap_height: 18,
    include_corner_marks: false,
    include_choc_v1_led_cutout_marks: false,
    include_choc_v2_led_cutout_marks: false,
    include_stabilizer_pad: true,
    oval_stabilizer_pad: false,
    choc_v1_support: true,
    choc_v2_support: true,
    choc_v1_stabilizers_diameter: 1.9,
    center_hole_diameter: 0.0,
    allow_soldermask_bridges: true,
    switch_3dmodel_filename: '',
    switch_3dmodel_xyz_offset: [0, 0, 0],
    switch_3dmodel_xyz_rotation: [0, 0, 0],
    switch_3dmodel_xyz_scale: [1, 1, 1],
    hotswap_3dmodel_filename: '',
    hotswap_3dmodel_xyz_offset: [0, 0, 0],
    hotswap_3dmodel_xyz_rotation: [0, 0, 0],
    hotswap_3dmodel_xyz_scale: [1, 1, 1],
    keycap_3dmodel_filename: '',
    keycap_3dmodel_xyz_offset: [0, 0, 0],
    keycap_3dmodel_xyz_rotation: [0, 0, 0],
    keycap_3dmodel_xyz_scale: [1, 1, 1],
    from: undefined,
    to: undefined,
    CENTERHOLE: { type: 'net', value: 'GND'},
    LEFTSTAB: { type: 'net', value: 'D1' },
    RIGHTSTAB: { type: 'net', value: 'D2' }
  },
  body: p => {
    const center_hole_diameter = p.center_hole_diameter > 0 ? p.center_hole_diameter : (p.choc_v2_support ? 5 : 3.4);
    const common_top = `
  (footprint "ceoloide:switch_choc_v1_v2"
    (layer "${p.side}.Cu")
    ${p.at}
    (property "Reference" "${p.ref}"
      (at 0 8.8 ${p.r})
      (layer "${p.side}.SilkS")
      ${p.ref_hide}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (attr exclude_from_pos_files exclude_from_bom${p.allow_soldermask_bridges ? ' allow_soldermask_bridges' : ''})

    ${''/* middle shaft hole */}
    ${p.include_plated_holes ? `
    (pad "" thru_hole circle (at 0 0 ${p.r}) (size ${center_hole_diameter + 0.3} ${center_hole_diameter + 0.3}) (drill ${center_hole_diameter}) (layers "*.Cu" "*.Mask") ${p.include_centerhole_net ? p.CENTERHOLE : ''})
    `: `
    (pad "" np_thru_hole circle (at 0 0 ${p.r}) (size ${center_hole_diameter} ${center_hole_diameter}) (drill ${center_hole_diameter}) (layers "*.Cu" "*.Mask"))
    `}
    `

    const choc_v1_stabilizers = `
    ${p.include_plated_holes ? `
    (pad "" thru_hole circle (at 5.5 0 ${p.r}) (size ${p.choc_v1_stabilizers_diameter + 0.3} ${p.choc_v1_stabilizers_diameter + 0.3}) (drill ${p.choc_v1_stabilizers_diameter}) (layers "*.Cu" "*.Mask"))
    (pad "" thru_hole circle (at -5.5 0 ${p.r}) (size ${p.choc_v1_stabilizers_diameter + 0.3} ${p.choc_v1_stabilizers_diameter + 0.3}) (drill ${p.choc_v1_stabilizers_diameter}) (layers "*.Cu" "*.Mask"))
    `: `
    (pad "" np_thru_hole circle (at 5.5 0 ${p.r}) (size ${p.choc_v1_stabilizers_diameter} ${p.choc_v1_stabilizers_diameter}) (drill ${p.choc_v1_stabilizers_diameter}) (layers "*.Cu" "*.Mask"))
    (pad "" np_thru_hole circle (at -5.5 0 ${p.r}) (size ${p.choc_v1_stabilizers_diameter} ${p.choc_v1_stabilizers_diameter}) (drill ${p.choc_v1_stabilizers_diameter}) (layers "*.Cu" "*.Mask"))
    `}
    `

    const corner_marks = `
    ${''/* corner marks - front */}
    (fp_line (start -7 -6) (end -7 -7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -7 7) (end -6 7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -6 -7) (end -7 -7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -7 7) (end -7 6) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start 7 6) (end 7 7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start 7 -7) (end 6 -7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start 6 7) (end 7 7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start 7 -7) (end 7 -6) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    `

    const keycap_xo = 0.5 * p.keycap_width
    const keycap_yo = 0.5 * p.keycap_height
    const keycap_marks = `
    (fp_rect (start ${keycap_xo} ${keycap_yo}) (end ${-keycap_xo} ${-keycap_yo}) (layer "Dwgs.User") (stroke (width 0.15) (type solid)) (fill none))
    `

    const choc_v1_led_cutout_marks = `
    (fp_rect (start -2.65 6.325) (end 2.65 3.075) (layer "Dwgs.User") (width 0.15) (stroke (width 0.15) (type solid)) (fill none))
    `

    const choc_v2_led_cutout_marks = `
    (fp_rect (start -2.75 6.405) (end 2.75 3.455) (layer "Dwgs.User") (width 0.15) (stroke (width 0.15) (type solid)) (fill none))
    `

    const hotswap_common = `
    ${'' /* Middle Hole */}
    ${p.include_plated_holes ? `
    (pad ${p.reversible ? '""' : '"1"'} thru_hole circle (at 0 -5.95 ${p.r}) (size 3.3 3.3) (drill 3) (layers "*.Cu" "*.Mask") ${p.reversible ? '' : p.from.str})
    `: `
    (pad "" np_thru_hole circle (at 0 -5.95 ${p.r}) (size 3 3) (drill 3) (layers "*.Cu" "*.Mask"))
    `}
    `

    const hotswap_back_pads_plated = `
    (pad "1" smd roundrect (at -2.648 -5.95 ${p.r}) (size 3.8 2.15) (layers "B.Cu") (roundrect_rratio 0.1) ${p.from.str})
    (pad "" smd roundrect (at -3.248 -5.95 ${p.r}) (size 2.6 2.15) (layers "B.Paste" "B.Mask") (roundrect_rratio 0.1))
    (pad "2" smd roundrect (at ${7.6475 - (2.6 - p.outer_pad_width_back) / 2} -3.75 ${p.r}) (size ${p.outer_pad_width_back + 1.2} 2.15) (layers "B.Cu") (roundrect_rratio 0.1) ${p.to.str})
    (pad "" smd roundrect (at ${8.2475 - (2.6 - p.outer_pad_width_back) / 2} -3.75 ${p.r}) (size ${p.outer_pad_width_back} 2.15) (layers "B.Paste" "B.Mask") (roundrect_rratio ${(2.15 / p.outer_pad_width_back) <= 1 ? 0.1 : 0.1 * (2.15 / p.outer_pad_width_back)}))
    `

    const hotswap_front_pads_plated = `
    (pad "1" smd roundrect (at 2.648 -5.95 ${p.r}) (size 3.8 2.15) (layers "F.Cu") (roundrect_rratio 0.1) ${p.from.str})
    (pad "" smd roundrect (at 3.248 -5.95 ${p.r}) (size 2.6 2.15) (layers "F.Paste" "F.Mask") (roundrect_rratio 0.1)) 
    (pad "2" smd roundrect (at ${-7.6475 + (2.6 - p.outer_pad_width_front) / 2} -3.75 ${p.r}) (size ${p.outer_pad_width_front + 1.2} 2.15) (layers "F.Cu") (roundrect_rratio 0.1) ${p.to.str})
    (pad "" smd roundrect (at ${-8.2475 + (2.6 - p.outer_pad_width_front) / 2} -3.75 ${p.r}) (size ${p.outer_pad_width_front} 2.15) (layers "F.Paste" "F.Mask") (roundrect_rratio ${(2.15 / p.outer_pad_width_front) <= 1 ? 0.1 : 0.1 * (2.15 / p.outer_pad_width_front)}))
    `

    const hotswap_back_pads_plated_reversible = `
    (pad "1" smd roundrect (at -3.245 -5.95 ${p.r}) (size 2.65 2.15) (layers "B.Cu" "B.Paste" "B.Mask") (roundrect_rratio 0.1) ${p.from.str})
    (pad "2" smd roundrect (at ${7.6475 - (2.6 - p.outer_pad_width_back) / 2} -3.75 ${p.r}) (size ${p.outer_pad_width_back + 1.2} 2.15) (layers "B.Cu") (roundrect_rratio 0.1) ${p.to.str})
    (pad "" smd roundrect (at ${8.2475 - (2.6 - p.outer_pad_width_back) / 2} -3.75 ${p.r}) (size ${p.outer_pad_width_back} 2.15) (layers "B.Paste" "B.Mask") (roundrect_rratio ${(2.15 / p.outer_pad_width_back) <= 1 ? 0.1 : 0.1 * (2.15 / p.outer_pad_width_back)}))
    `

    const hotswap_front_pads_plated_reversible = `
    (pad "2" smd roundrect (at 3.245 -5.95 ${p.r}) (size 2.65 2.15) (layers "F.Cu" "F.Paste" "F.Mask") (roundrect_rratio 0.1) ${p.to.str})
    (pad "1" smd roundrect (at ${-7.6475 + (2.6 - p.outer_pad_width_front) / 2} -3.75 ${p.r}) (size ${p.outer_pad_width_front + 1.2} 2.15) (layers "F.Cu") (roundrect_rratio 0.1) ${p.from.str})
    (pad "" smd roundrect (at ${-8.2475 + (2.6 - p.outer_pad_width_front) / 2} -3.75 ${p.r}) (size ${p.outer_pad_width_front} 2.15) (layers "F.Paste" "F.Mask") (roundrect_rratio ${(2.15 / p.outer_pad_width_front) <= 1 ? 0.1 : 0.1 * (2.15 / p.outer_pad_width_front)}))
    `

    const hotswap_front_pad_cutoff = `
    (pad "1" smd roundrect
      (at 3.275 -5.95 ${p.r})
      (size 2.6 2.6)
      (layers "F.Cu" "F.Paste" "F.Mask")
      (roundrect_rratio 0)
			(chamfer_ratio 0.455)
			(chamfer bottom_right)
      ${p.from.str}
    )
    `

    const hotswap_front_pad_full = `
    (pad "1" smd rect (at 3.275 -5.95 ${p.r}) (size 2.6 2.6) (layers "F.Cu" "F.Paste" "F.Mask") ${p.from.str})
    `

    const hotswap_back_pad_cutoff = `
    (pad "1" smd roundrect
      (at -3.275 -5.95 ${p.r})
      (size 2.6 2.6)
      (layers "B.Cu" "B.Paste" "B.Mask")
      (roundrect_rratio 0)
			(chamfer_ratio 0.455)
			(chamfer bottom_left)
      ${p.hotswap_pads_same_side ? p.to.str : p.from.str}
    )
    `

    const hotswap_back_pad_full = `
    (pad "1" smd rect (at -3.275 -5.95 ${p.r}) (size 2.6 2.6) (layers "B.Cu" "B.Paste" "B.Mask") ${p.hotswap_pads_same_side ? p.to.str : p.from.str})
    `

    const hotswap_back = `
    ${'' /* Silkscreen outline */}
    ${'' /* back top */}
    (fp_line (start -1.5 -8.2) (end -2 -7.7) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 1.5 -8.2) (end -1.5 -8.2) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 2 -7.7) (end 1.5 -8.2) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 2 -7.7) (end 2 -6.78) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 2.52 -6.2) (end 7 -6.2) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 7 -6.2) (end 7 -5.6) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_arc (start 2.52 -6.2) (mid 2.139878 -6.382304) (end 2 -6.78) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
  
    ${'' /* back bottom */}
    (fp_line (start -1.5 -3.7) (end -2 -4.2) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 0.8 -3.7) (end -1.5 -3.7) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 2.5 -1.5) (end 2.5 -2.2) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 7 -1.5) (end 2.5 -1.5) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 7 -2) (end 7 -1.5) (layer "B.SilkS") (stroke (width 0.15) (type solid)))
    (fp_arc (start 0.8 -3.7) (mid 1.956518 -3.312082) (end 2.5 -2.22) (layer "B.SilkS") (stroke (width 0.15) (type solid)))

    ${p.include_plated_holes ? `
    ${'' /* Side Hole */}
      ${p.reversible ? `
      (pad "2" thru_hole circle (at 5 -3.75 ${195 + p.r}) (size 3.3 3.3) (drill 3) (layers "*.Cu" "*.Mask") ${p.to.str})
      ${'' /* Pads */}
      ${hotswap_back_pads_plated_reversible}
      `:`
      (pad "2" thru_hole circle (at 5 -3.75 ${195 + p.r}) (size 3.3 3.3) (drill 3) (layers "*.Cu" "*.Mask") ${p.to.str})
      ${'' /* Pads */}
      ${hotswap_back_pads_plated}
      `}
    `: `
    ${'' /* Left Pad*/}
    ${p.reversible ? hotswap_back_pad_cutoff : hotswap_back_pad_full}

    ${'' /* Right Pad (not cut off) */}
    (pad "2" smd rect (at ${8.275 - (2.6 - p.outer_pad_width_back) / 2} -3.75 ${p.r}) (size ${p.outer_pad_width_back} 2.6) (layers "B.Cu" "B.Paste" "B.Mask") ${p.hotswap_pads_same_side ? p.from.str : p.to.str})

    ${'' /* Side Hole */}
    (pad "" np_thru_hole circle (at 5 -3.75 ${195 + p.r}) (size 3 3) (drill 3) (layers "*.Cu" "*.Mask"))
    `}
    `

    const hotswap_front = `
    ${'' /* Silkscreen outline */}
    ${'' /* front top */}
    (fp_line (start -7 -5.6) (end -7 -6.2) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start -7 -6.2) (end -2.52 -6.2) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start -2 -6.78) (end -2 -7.7) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start -1.5 -8.2) (end -2 -7.7) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 1.5 -8.2) (end -1.5 -8.2) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 2 -7.7) (end 1.5 -8.2) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_arc (start -2 -6.78) (mid -2.139878 -6.382304) (end -2.52 -6.2) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
  
    ${'' /* front bottom */}
    (fp_line (start -7 -1.5) (end -7 -2) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start -2.5 -1.5) (end -7 -1.5) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start -2.5 -2.2) (end -2.5 -1.5) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 1.5 -3.7) (end -0.8 -3.7) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_line (start 2 -4.2) (end 1.5 -3.7) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    (fp_arc (start -2.5 -2.22) (mid -1.956518 -3.312082) (end -0.8 -3.7) (layer "F.SilkS") (stroke (width 0.15) (type solid)))
    
    ${p.include_plated_holes ? `
    ${'' /* Side Hole */}
    ${p.reversible ? `
    (pad "1" thru_hole circle (at -5 -3.75 ${195 + p.r}) (size 3.3 3.3) (drill 3) (layers "*.Cu" "*.Mask") ${p.from.str})
    ${'' /* Pads */}
    ${hotswap_front_pads_plated_reversible}
    `:`
    (pad "2" thru_hole circle (at -5 -3.75 ${195 + p.r}) (size 3.3 3.3) (drill 3) (layers "*.Cu" "*.Mask") ${p.to.str})
    ${'' /* Pads */}
    ${hotswap_front_pads_plated}
    `}
    `: `
    ${'' /* Side Hole */}
    (pad "" np_thru_hole circle (at -5 -3.75 ${195 + p.r}) (size 3 3) (drill 3) (layers "*.Cu" "*.Mask"))

    ${'' /* Right Pad (cut off) */}
    ${p.reversible ? hotswap_front_pad_cutoff : hotswap_front_pad_full}

    ${'' /* Left Pad (not cut off) */}
    (pad "2" smd rect (at ${-8.275 + (2.6 - p.outer_pad_width_front) / 2} -3.75 ${p.r}) (size ${p.outer_pad_width_front} 2.6) (layers "F.Cu" "F.Paste" "F.Mask") ${p.to.str})
    `}
    `

    // If both hotswap and solder are enabled, move the solder holes
    // "down" to the opposite side of the switch.
    // Since switches can be rotated by 90 degrees, this won't be a
    // problem as long as we switch the side the holes are on.
    let solder_offset_x_front = '-'
    let solder_offset_x_back = ''
    let solder_offset_y = '-'
    let stab_offset_x_front = ''
    let stab_offset_x_back = '-'
    let stab_offset_y = ''
    if (p.hotswap && p.solder) {
      solder_offset_x_front = ''
      solder_offset_x_back = '-'
      solder_offset_y = ''
      stab_offset_x_front = '-'
      stab_offset_x_back = ''
      stab_offset_y = ''
    }

    const solder_common = `
    (pad "2" thru_hole circle (at 0 ${solder_offset_y}5.9 ${195 + p.r}) (size 2.032 2.032) (drill 1.27) (layers "*.Cu" "*.Mask") ${p.from.str})
    `

    const solder_front = `
    (pad "1" thru_hole circle (at ${solder_offset_x_front}5 ${solder_offset_y}3.8 ${195 + p.r}) (size 2.032 2.032) (drill 1.27) (layers "*.Cu" "*.Mask") ${p.to.str})
    `
    
    const solder_back = `
    (pad "1" thru_hole circle (at ${solder_offset_x_back}5 ${solder_offset_y}3.8 ${195 + p.r}) (size 2.032 2.032) (drill 1.27) (layers "*.Cu" "*.Mask") ${p.to.str})
    `

    const oval_corner_stab_front = `
    (pad "" thru_hole oval (at ${stab_offset_x_front}5 ${stab_offset_y}5.15 ${p.r}) (size 2.4 1.2) (drill oval 1.6 0.4) (layers "*.Cu" "*.Mask") ${p.solder && p.hotswap ? p.to.str : p.include_stabilizer_nets ? p.RIGHTSTAB : ''})
    `

    const oval_corner_stab_back = `
    (pad "" thru_hole oval (at ${stab_offset_x_back}5 ${stab_offset_y}5.15 ${p.r}) (size 2.4 1.2) (drill oval 1.6 0.4) (layers "*.Cu" "*.Mask") ${p.solder && p.hotswap ? p.to.str : p.include_stabilizer_nets ? p.LEFTSTAB : ''})
    `

    const round_corner_stab_front = `
    (pad "" thru_hole circle (at ${stab_offset_x_front}5.00 ${stab_offset_y}5.15 ${p.r}) (size 1.9 1.9) (drill 1.6) (layers "*.Cu" "*.Mask") ${p.solder && p.hotswap ? p.to.str : p.include_stabilizer_nets ? p.RIGHTSTAB : ''})
    `

    const round_corner_stab_back = `
    (pad "" thru_hole circle (at ${stab_offset_x_back}5.00 ${stab_offset_y}5.15 ${p.r}) (size 1.9 1.9) (drill 1.6) (layers "*.Cu" "*.Mask") ${p.solder && p.hotswap ? p.to.str : p.include_stabilizer_nets ? p.LEFTSTAB : ''})
    `

    // `side` selects the socket; the switch and cap sit across the PCB.
    const keycap_height = 6.6;
    const model_rotation = p.side === 'B' ? [180, 0, 0] : [0, 180, 0];
    const model_offset = [0, 0, -p.pcb_thickness];
    const cap_offset = [0, 0, -p.pcb_thickness - keycap_height];
    const switch_rotation = p.switch_3dmodel_xyz_rotation || model_rotation;
    const switch_offset = p.switch_3dmodel_xyz_offset || model_offset;
    const hotswap_rotation = p.hotswap_3dmodel_xyz_rotation || model_rotation;
    const hotswap_offset = p.hotswap_3dmodel_xyz_offset || model_offset;
    const keycap_rotation = p.keycap_3dmodel_xyz_rotation || model_rotation;
    const keycap_offset = p.keycap_3dmodel_xyz_offset || cap_offset;
    const switch_3dmodel = `
    (model ${JSON.stringify(p.switch_3dmodel_filename)}
      (offset (xyz ${switch_offset[0]} ${switch_offset[1]} ${switch_offset[2]}))
      (scale (xyz ${p.switch_3dmodel_xyz_scale[0]} ${p.switch_3dmodel_xyz_scale[1]} ${p.switch_3dmodel_xyz_scale[2]}))
      (rotate (xyz ${switch_rotation[0]} ${switch_rotation[1]} ${switch_rotation[2]}))
    )
    `

    const hotswap_3dmodel = `
    (model ${JSON.stringify(p.hotswap_3dmodel_filename)}
      (offset (xyz ${hotswap_offset[0]} ${hotswap_offset[1]} ${hotswap_offset[2]}))
      (scale (xyz ${p.hotswap_3dmodel_xyz_scale[0]} ${p.hotswap_3dmodel_xyz_scale[1]} ${p.hotswap_3dmodel_xyz_scale[2]}))
      (rotate (xyz ${hotswap_rotation[0]} ${hotswap_rotation[1]} ${hotswap_rotation[2]}))
    )
	  `

    const keycap_3dmodel = `
    (model ${JSON.stringify(p.keycap_3dmodel_filename)}
      (offset (xyz ${keycap_offset[0]} ${keycap_offset[1]} ${keycap_offset[2]}))
      (scale (xyz ${p.keycap_3dmodel_xyz_scale[0]} ${p.keycap_3dmodel_xyz_scale[1]} ${p.keycap_3dmodel_xyz_scale[2]}))
      (rotate (xyz ${keycap_rotation[0]} ${keycap_rotation[1]} ${keycap_rotation[2]}))
    )
	  `

    const common_bottom = `
  )
    `

    const hotswap_routes_unplated = `
	(segment
		(start ${p.eaxy(3.275, -5.95)})
		(end ${p.eaxy(1.2, -3.875)})
		(width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(layer "F.Cu")
		(net ${p.from.index})
	)
	(segment
		(start ${p.eaxy(1.2, -3.875)})
		(end ${p.eaxy(0, -3.875)})
		(width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(layer "F.Cu")
		(net ${p.from.index})
	)
	(via
		(at ${p.eaxy(0, -3.875)})
		(size ${p.via_size})
    (drill ${p.via_drill})
		(layers "F.Cu" "B.Cu")
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(net ${p.from.index})
	)
	(segment
		(start ${p.eaxy(-1.2, -3.875)})
		(end ${p.eaxy(0, -3.875)})
		(width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(layer "B.Cu")
		(net ${p.from.index})
	)
	(segment
		(start ${p.eaxy(-3.275, -5.95)})
		(end ${p.eaxy(-1.2, -3.875)})
		(width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(layer "B.Cu")
		(net ${p.from.index})
	)
	(segment
		(start ${p.eaxy(-6.421, -1.896)})
		(end ${p.eaxy(-2.154, -1.896)})
		(width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(layer "F.Cu")
		(net ${p.to.index})
	)
	(segment
		(start ${p.eaxy(-0.975, -3.075)})
		(end ${p.eaxy(0, -3.075)})
		(width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(layer "F.Cu")
		(net ${p.to.index})
	)
	(segment
		(start ${p.eaxy(-8.275, -3.75)})
		(end ${p.eaxy(-6.421, -1.896)})
		(width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(layer "F.Cu")
		(net ${p.to.index})
	)
	(segment
		(start ${p.eaxy(-2.154, -1.896)})
		(end ${p.eaxy(-0.975, -3.075)})
		(width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(layer "F.Cu")
		(net ${p.to.index})
	)
	(via
		(at ${p.eaxy(0, -3.075)})
		(size ${p.via_size})
    (drill ${p.via_drill})
		(layers "F.Cu" "B.Cu")
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(net ${p.to.index})
	)
	(segment
		(start ${p.eaxy(2.140166, -1.896)})
		(end ${p.eaxy(0.961166, -3.075)})
		(width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(layer "B.Cu")
		(net ${p.to.index})
	)
	(segment
		(start ${p.eaxy(6.421, -1.896)})
		(end ${p.eaxy(2.140166, -1.896)})
		(width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(layer "B.Cu")
		(net ${p.to.index})
	)
	(segment
		(start ${p.eaxy(0.961166, -3.075)})
		(end ${p.eaxy(0, -3.075)})
		(width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(layer "B.Cu")
		(net ${p.to.index})
	)
	(segment
		(start ${p.eaxy(8.275, -3.75)})
		(end ${p.eaxy(6.421, -1.896)})
		(width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(layer "B.Cu")
		(net ${p.to.index})
	)
    `

    const hotswap_routes_same_side = `
  (segment
		(start ${p.eaxy(3.275, -5.95)})
		(end ${p.eaxy(7.775, -5.95)})
		(width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(layer "F.Cu")
		(net ${p.from.index})
	)
	(via
		(at ${p.eaxy(7.775, -5.95)})
		(size ${p.via_size})
    (drill ${p.via_drill})
		(layers "F.Cu" "B.Cu")
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(net ${p.from.index})
	)
	(segment
		(start ${p.eaxy(7.775, -3.75)})
		(end ${p.eaxy(7.775, -5.95)})
		(width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(layer "B.Cu")
		(net ${p.from.index})
	)
	(segment
		(start ${p.eaxy(-7.775, -3.75)})
		(end ${p.eaxy(-7.775, -5.95)})
		(width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(layer "F.Cu")
		(net ${p.to.index})
	)
	(via
		(at ${p.eaxy(-7.775, -5.95)})
		(size ${p.via_size})
    (drill ${p.via_drill})
		(layers "F.Cu" "B.Cu")
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(net ${p.to.index})
	)
	(segment
		(start ${p.eaxy(-3.275, -5.95)})
		(end ${p.eaxy(-7.775, -5.95)})
		(width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(layer "B.Cu")
		(net ${p.to.index})
	)
    `

    let final = common_top
    if (p.choc_v1_support) {
      final += choc_v1_stabilizers
    }
    if (p.include_corner_marks) {
      final += corner_marks;
    }
    if (p.include_keycap) {
      final += keycap_marks
    }
    if (p.include_stabilizer_pad && p.choc_v2_support) {
      // Combined mounting needs both stabilizer positions, even on one side.
      const both_stabilizers = p.reversible || (p.hotswap && p.solder);
      if (both_stabilizers || p.side == "F") {
        if (p.oval_stabilizer_pad) {
          final += oval_corner_stab_front
        } else {
          final += round_corner_stab_front
        }
      }
      if (both_stabilizers || p.side == "B") {
        if (p.oval_stabilizer_pad) {
          final += oval_corner_stab_back
        } else {
          final += round_corner_stab_back
        }
      }
    }
    if (p.include_choc_v1_led_cutout_marks) {
      final += choc_v1_led_cutout_marks
    }
    if (p.include_choc_v2_led_cutout_marks) {
      final += choc_v2_led_cutout_marks
    }
    if (p.hotswap) {
      final += hotswap_common
      if (p.reversible || p.side == "F") {
        final += hotswap_front
      }
      if (p.reversible || p.side == "B") {
        final += hotswap_back
      }
      if (p.hotswap_3dmodel_filename) {
        final += hotswap_3dmodel
      }
    }
    if (p.solder) {
      final += solder_common
      if (p.reversible || p.side == "F") {
        final += solder_front
      }
      if (p.reversible || p.side == "B") {
        final += solder_back
      }
    }

    if (p.switch_3dmodel_filename) {
      final += switch_3dmodel
    }

    if (p.keycap_3dmodel_filename) {
      final += keycap_3dmodel
    }

    final += common_bottom

    if (p.reversible && p.hotswap && p.include_traces_vias && !p.include_plated_holes) {
      if(p.hotswap_pads_same_side){
        final += hotswap_routes_same_side
      } else {
        final += hotswap_routes_unplated
      }
    }

    return final;
  }
}

;module.exports = ((original) => {
    const defaults = {"switch_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/Choc_V1_Switch.step","hotswap_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/Choc_V1_Hotswap.step","keycap_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/Choc_V1_Keycap_MBK_Black_1u.step","switch_3dmodel_xyz_offset":"","switch_3dmodel_xyz_rotation":"","hotswap_3dmodel_xyz_offset":"","hotswap_3dmodel_xyz_rotation":"","keycap_3dmodel_xyz_offset":"","keycap_3dmodel_xyz_rotation":""};
    const bossVariant = null;
    const chocV2 = "${KIPRJMOD}/models/boardstudio/koktoh/Choc_V2_Red.step";
    return {
      ...original,
      params: {...original.params, ...defaults},
      body: p => {
        // The bundled extension crosses the PCB with a 5 mm outer diameter.
        const extensionDiameter = 5;
        if (false && p.drill < extensionDiameter
          && p.tp_extension_3dmodel_filename === defaults.tp_extension_3dmodel_filename
          && !p.tp_extension_3dmodel_xyz_scale
          && !p.tp_extension_3dmodel_xyz_rotation
          && !p.tp_extension_3dmodel_xyz_offset) {
          throw new Error('The bundled trackpoint extension requires a center drill of at least 5 mm. Select a compatible extension model or drill size.');
        }
        if (true && !p.choc_v1_support) {
          p = {...p};
          if (!p.choc_v2_support) {
            for (const key of Object.keys(defaults)) {
              if (p[key] === defaults[key]) { p[key] = ''; }
            }
          } else {
            if (p.switch_3dmodel_filename === defaults.switch_3dmodel_filename) {
              const automatic = !p.switch_3dmodel_xyz_rotation && !p.switch_3dmodel_xyz_offset
                && p.switch_3dmodel_xyz_scale.every(value => value === 1);
              const postDiameter = 4.8;
              if (automatic && (!p.include_stabilizer_pad || p.oval_stabilizer_pad
                || (p.center_hole_diameter > 0 && p.center_hole_diameter < postDiameter))) {
                throw new Error('The bundled Choc V2 model requires the round stabilizer hole and a center drill of at least 4.8 mm. Use a matching model or transform for modified hardware.');
              }
              p.switch_3dmodel_filename = chocV2;
              p.switch_3dmodel_xyz_rotation ||= p.side === 'F' ? [180, 0, 0] : [0, 180, 0];
              p.switch_3dmodel_xyz_offset ||= [0, 0, -p.pcb_thickness];
            }
            if (p.keycap_3dmodel_filename === defaults.keycap_3dmodel_filename) {
              p.keycap_3dmodel_filename = '';
            }
          }
        }
        if (bossVariant && p.include_bosses && p.reset_switch_3dmodel_filename === defaults.reset_switch_3dmodel_filename) {
          p = {...p, reset_switch_3dmodel_filename: bossVariant};
        }
        return original.body(p);
      }
    };
  })(module.exports);

return module.exports; })(),
  "ceoloide/switch_gateron_ks27_ks33": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Copyright (c) 2023 Marco Massarelli
//
// SPDX-License-Identifier: MIT
//
// To view a copy of this license, visit https://spdx.org/licenses/MIT.html, https://opensource.org/license/mit
//
// Authors: @nxtk
//
// Description:
//    Gateron LP KS27 (v1) P\N: KS-27H10B050NN + Gateron LP KS33 (v2) P\N: KS-33H10B050NN reversible and hotswappable footprint.
//    This includes support for NuPhy low profile switches Wisteria (T55), Aloe (L37), Daisy (L48), Cowberry, Moss
//    Hotswap socket: Gateron LP HS 2.0 P\N KS-2P02B01-02
//
//    With the set defaults it will include support for hotswap, single side (Back).
//
// Notes:
//   - Hotswap and solder can be used together. The solder holes will then be
//     added above the hotswap holes.
//   - Soldered reversible mode has alternate custom pads enabled by `include_custom_solder_pads` option.
//     Compatible with hotswap mode.
//
// Params:
//    side: default is B for Back
//      the side on which to place the single-side footprint and designator, either F or B
//    reversible: default is false
//      if true, the footprint will be placed on both sides so that the PCB can be
//      reversible
//    hotswap: default is true
//      if true, will include holes and pads for Kailh choc hotswap sockets
//    solder: default is false
//      if true, will include holes to solder switches (works with hotswap too)
//    outer_pad_width_front: default 2.6
//    outer_pad_width_back: default 2.6
//      Allows you to make the outer hotswap pads smaller to silence DRC
//      warnings when the sockets are too close to the edge cuts. It's not
//      recommended to go below 1.6mm to ensure the hotswap socket can be
//      properly soldered.
//    include_keycap: default is false
//      if true, will add mx sized keycap box around the footprint (18mm)
//    include_corner_marks: default is false
//      if true, will add corner marks to indicate plate hole size and position
//    include_stem_outline: default is false
//      if true, will add switch stem outline
//    include_led_outline: default is false
//      if true, will add switch led cutout outline (changes alignment based on the `side` option)
//    include_socket_silks: default is false
//      if true, will add hotswap sockets silkscreens (follows `reversible` and `side` options)
//    include_socket_fabs: default is false
//      if true, will add hotswap socket outlines to *.Fab layers which might be helful for new users
//    include_custom_solder_pads: default is false
//      if true, replaces reversible solder pads with alternate version
//      (disabled in non reversible mode)
//    include_centerhole_net: default is false
//      if true, will add adjustable net to the center hole
//      CENTERHOLE: default is "GND"
//    keycap_height: default is 18
//      Allows you to adjust the width of the keycap outline. For example,
//      to show a 1.5u outline for easier aligning.
//    keycap_width: default is 18
//      Allows you to adjust the height of the keycap outline.
//    allow_soldermask_bridges: default is true
//      Disables 'solder mask aperture bridges items with different nets' DRC check when set to true
//      setting this option to false may be useful for debugging purposes, (applied locally to this footprint only)
//      for global setting see `allow_soldermask_bridges_in_footprints` in a kicad template
//    switch_3dmodel_filename: default is ''
//      Allows you to specify the path to a 3D model STEP or WRL file to be
//      used when rendering the PCB. Use the ${VAR_NAME} syntax to point to
//      a KiCad configured path.
//    switch_3dmodel_xyz_offset: default is [0, 0, 0]
//      xyz offset (in mm), used to adjust the position of the 3d model
//      relative the footprint.
//    switch_3dmodel_xyz_scale: default is [1, 1, 1]
//      xyz scale, used to adjust the size of the 3d model relative to its
//      original size.
//    switch_3dmodel_xyz_rotation: default is [0, 0, 0]
//      xyz rotation (in degrees), used to adjust the orientation of the 3d
//      model relative the footprint.
//    hotswap_3dmodel_filename: default is ''
//      Allows you to specify the path to a 3D model to be used when rendering
//      the PCB. Allows for paths using a configured path by using the
//      ${VAR_NAME} syntax.
//    hotswap_3dmodel_xyz_offset: default is [0, 0, 0]
//      xyz offset (in mm), used to adjust the position of the 3d model
//      relative the footprint.
//    hotswap_3dmodel_xyz_scale: default is [1, 1, 1]
//      xyz scale, used to adjust the size of the 3d model relative its
//      original size.
//    hotswap_3dmodel_xyz_rotation: default is [0, 0, 0]
//      xyz rotation (in degrees), used to adjust the orientation of the 3d
//      model relative the footprint.
//    keycap_3dmodel_filename: default is ''
//      Allows you to specify the path to a 3D model STEP or WRL file to be
//      used when rendering the PCB. Use the ${VAR_NAME} syntax to point to
//      a KiCad configured path.
//    keycap_3dmodel_xyz_offset: default is [0, 0, 0]
//      xyz offset (in mm), used to adjust the position of the 3d model
//      relative the footprint.
//    keycap_3dmodel_xyz_scale: default is [1, 1, 1]
//      xyz scale, used to adjust the size of the 3d model relative to its
//      original size.
//    keycap_3dmodel_xyz_rotation: default is [0, 0, 0]
//      xyz rotation (in degrees), used to adjust the orientation of the 3d
//      model relative the footprint.

module.exports = {
  params: {
    designator: 'S',
    side: 'B',
    reversible: false,
    solder: true,
    hotswap: true,
    keycap_width: 18,
    keycap_height: 18,
    include_corner_marks: false,
    include_centerhole_net: false,
    include_keycap: false,
    include_stem_outline: false,
    include_led_outline: false,
    include_socket_silks: false,
    include_socket_fabs: false,
    include_custom_solder_pads: false,
    allow_soldermask_bridges: true,
    outer_pad_width_front: 2.6,
    outer_pad_width_back: 2.6,
    switch_3dmodel_filename: '',
    pcb_thickness: 1.6,
    switch_3dmodel_xyz_offset: '',
    switch_3dmodel_xyz_rotation: '',
    switch_3dmodel_xyz_scale: [1, 1, 1],
    hotswap_3dmodel_filename: '',
    hotswap_3dmodel_xyz_offset: [0, 0, 0],
    hotswap_3dmodel_xyz_rotation: [0, 0, 0],
    hotswap_3dmodel_xyz_scale: [1, 1, 1],
    keycap_3dmodel_filename: '',
    keycap_3dmodel_xyz_offset: [0, 0, 0],
    keycap_3dmodel_xyz_rotation: [0, 0, 0],
    keycap_3dmodel_xyz_scale: [1, 1, 1],
    from: undefined,
    to: undefined,
    CENTERHOLE: { type: 'net', value: 'GND' },
  },
  body: p => {
    if (p.hotswap && p.reversible) {
      throw new Error("switch_gateron_ks27_ks33: reversible hotswap has overlapping 3 mm drills; use reversible: false or hotswap: false with solder: true");
    }

    const common_top = `
  (footprint "ceoloide:switch_gateron_ks27_ks33"
    (layer "${p.side}.Cu")
    ${p.at}
    (property "Reference" "${p.ref}"
      (at 0 8.8 ${p.r})
      (layer "${p.side}.SilkS")
      ${p.ref_hide}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (attr exclude_from_pos_files exclude_from_bom${p.allow_soldermask_bridges ? ' allow_soldermask_bridges' : ''})

    ${''/* middle shaft hole */}
    (pad "" thru_hole circle (at 0 0 ${p.r}) (size 5.6 5.6) (drill 5.1) (layers "*.Cu" "*.Mask") ${p.include_centerhole_net ? p.CENTERHOLE : ''})
    `

    const corner_marks = `
    ${''/* corner marks - front */}
    (fp_line (start -7 -6) (end -7 -7) (stroke (width 0.15) (type solid)) (layer "Dwgs.User"))
    (fp_line (start -7 7) (end -7 6) (stroke (width 0.15) (type solid)) (layer "Dwgs.User"))
    (fp_line (start -7 7) (end -6 7) (stroke (width 0.15) (type solid)) (layer "Dwgs.User"))
    (fp_line (start -6 -7) (end -7 -7) (stroke (width 0.15) (type solid)) (layer "Dwgs.User"))
    (fp_line (start 6 7) (end 7 7) (stroke (width 0.15) (type solid)) (layer "Dwgs.User"))
    (fp_line (start 7 -7) (end 6 -7) (stroke (width 0.15) (type solid)) (layer "Dwgs.User"))
    (fp_line (start 7 -7) (end 7 -6) (stroke (width 0.15) (type solid)) (layer "Dwgs.User"))
    (fp_line (start 7 6) (end 7 7) (stroke (width 0.15) (type solid)) (layer "Dwgs.User"))
    `

    const led_outline = `
    ${''/* led outline - front */}
    ${p.side == 'B' ? `
    (fp_rect (start -3.2 -6.3) (end 1.8 -4.05) (stroke (width 0.15) (type solid)) (fill none) (layer "Dwgs.User"))
    `:`
    (fp_rect (start -1.8 -6.3) (end 3.2 -4.05) (stroke (width 0.15) (type solid)) (fill none) (layer "Dwgs.User"))
    `}
    `

    const stem_outline = `
    ${''/* stem outline - front */}
    (fp_poly (pts (xy -0.525791 -3.207186) (xy -0.869467 -3.131537) (xy -1.202949 -3.019174) (xy -1.522327 -2.871414) (xy -1.823858 -2.689989) (xy -2.104005 -2.477027) (xy -2.359485 -2.235023) (xy -2.389234 -2.2) (xy -4.7 -2.2) (xy -4.7 2.2) (xy -2.389234 2.2) (xy -2.359485 2.235023) (xy -2.104005 2.477027) (xy -1.823858 2.689989) (xy -1.522327 2.871414) (xy -1.202949 3.019174) (xy -0.869467 3.131537) (xy -0.525791 3.207186) (xy -0.175951 3.245234) (xy 0 3.245234) (xy 0 2.845178) (xy -0.165713 2.845178) (xy -0.494897 2.806702) (xy -0.817389 2.73027) (xy -1.128827 2.616916) (xy -1.425 2.468172) (xy -1.701902 2.286051) (xy -1.955789 2.073015) (xy -2.183227 1.831945) (xy -2.38114 1.566101) (xy -2.546853 1.279078) (xy -2.678124 0.974757) (xy -2.773178 0.657255) (xy -2.830729 0.330865) (xy -2.85 0) (xy -2.830729 -0.330865) (xy -2.773178 -0.657255) (xy -2.678124 -0.974757) (xy -2.546853 -1.279078) (xy -2.38114 -1.566101) (xy -2.183227 -1.831945) (xy -1.955789 -2.073015) (xy -1.701902 -2.286051) (xy -1.425 -2.468172) (xy -1.128827 -2.616916) (xy -0.817389 -2.73027) (xy -0.494897 -2.806702) (xy -0.165713 -2.845178) (xy 0 -2.845178) (xy 0 -3.245234) (xy -0.175951 -3.245234)) (stroke (width 0.001) (type solid)) (fill solid) (layer "Dwgs.User"))
    (fp_poly (pts (xy 0.525791 -3.207186) (xy 0.869467 -3.131537) (xy 1.202949 -3.019174) (xy 1.522327 -2.871414) (xy 1.823858 -2.689989) (xy 2.104005 -2.477027) (xy 2.359485 -2.235023) (xy 2.389234 -2.2) (xy 4.7 -2.2) (xy 4.7 2.2) (xy 2.389234 2.2) (xy 2.359485 2.235023) (xy 2.104005 2.477027) (xy 1.823858 2.689989) (xy 1.522327 2.871414) (xy 1.202949 3.019174) (xy 0.869467 3.131537) (xy 0.525791 3.207186) (xy 0.175951 3.245234) (xy 0 3.245234) (xy 0 2.845178) (xy 0.165713 2.845178) (xy 0.494897 2.806702) (xy 0.817389 2.73027) (xy 1.128827 2.616916) (xy 1.425 2.468172) (xy 1.701902 2.286051) (xy 1.955789 2.073015) (xy 2.183227 1.831945) (xy 2.38114 1.566101) (xy 2.546853 1.279078) (xy 2.678124 0.974757) (xy 2.773178 0.657255) (xy 2.830729 0.330865) (xy 2.85 0) (xy 2.830729 -0.330865) (xy 2.773178 -0.657255) (xy 2.678124 -0.974757) (xy 2.546853 -1.279078) (xy 2.38114 -1.566101) (xy 2.183227 -1.831945) (xy 1.955789 -2.073015) (xy 1.701902 -2.286051) (xy 1.425 -2.468172) (xy 1.128827 -2.616916) (xy 0.817389 -2.73027) (xy 0.494897 -2.806702) (xy 0.165713 -2.845178) (xy 0 -2.845178) (xy 0 -3.245234) (xy 0.175951 -3.245234)) (stroke (width 0.001) (type solid)) (fill solid) (layer "Dwgs.User"))
    `

    const stem_cross_outline = `
    ${''/* stem outline cross - front */}
    (fp_poly (pts (xy -0.55 -0.55) (xy -0.55 -2) (xy 0.55 -2) (xy 0.55 -0.55) (xy 2 -0.55) (xy 2 0.55) (xy 0.55 0.55) (xy 0.55 2) (xy -0.55 2) (xy -0.55 0.55) (xy -2 0.55) (xy -2 -0.55)) (stroke (width 0) (type solid)) (fill solid) (layer "Dwgs.User"))
    `


    const keycap_xo = 0.5 * p.keycap_width
    const keycap_yo = 0.5 * p.keycap_height
    const keycap_marks = `
    (fp_rect (start ${keycap_xo} ${keycap_yo}) (end ${-keycap_xo} ${-keycap_yo}) (layer "Dwgs.User") (stroke (width 0.15) (type solid)) (fill none))
    `

    const hotswap_fab_front = `
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
    `

    const hotswap_fab_back = `
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
    `

    const hotswap_silk_front = `
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
    
    ${p.reversible ? `` : `
    (fp_arc (start -5.025 3.675) (mid -4.995711 3.604289) (end -4.925 3.575) (stroke (width 0.15) (type solid)) (layer "F.SilkS"))
    (fp_line (start -5.025 4.275) (end -5.025 3.675) (stroke (width 0.15) (type solid)) (layer "F.SilkS"))
    (fp_line (start -4.325 3.575) (end -4.925 3.575) (stroke (width 0.15) (type solid)) (layer "F.SilkS"))
    `}
    `

    const hotswap_silk_back = `
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
    
    ${p.reversible ? `` : `
    (fp_arc (start 4.925 3.575) (mid 4.995711 3.604289) (end 5.025 3.675) (stroke (width 0.15) (type solid)) (layer "B.SilkS"))
    (fp_line (start 5.025 3.675) (end 5.025 4.275) (stroke (width 0.15) (type solid)) (layer "B.SilkS"))
    (fp_line (start 4.925 3.575) (end 4.325 3.575) (stroke (width 0.15) (type solid)) (layer "B.SilkS"))
    `}
    `

    const hotswap_front_full = `
    (pad "1" smd roundrect (at -5.55 5.75 ${p.r}) (size 4 2.5) (layers "F.Cu") (roundrect_rratio 0.1) ${p.from.str})
    (pad "" smd roundrect (at -6.25 5.75 ${p.r}) (size 2.6 2.5) (layers "F.Paste" "F.Mask") (roundrect_rratio 0.1))
    `

    const hotswap_back_full = `
    (pad "2" smd roundrect (at 5.55 5.75 ${p.r}) (size 4 2.5) (layers "B.Cu") (roundrect_rratio 0.1) ${p.to.str})
    (pad "" smd roundrect (at 6.25 5.75 ${p.r}) (size 2.6 2.5) (layers "B.Paste" "B.Mask") (roundrect_rratio 0.1))
    `

    const hotswap_front = `
    (pad ${p.reversible ? '""' : '"1"'} thru_hole circle (at -2.6 5.75 ${p.r}) (size 3.5 3.5) (drill 3) (layers "*.Cu" "*.Mask") ${p.from.str})
    (pad "2" thru_hole circle (at 4.4 4.7 ${p.r}) (size 3.5 3.5) (drill 3) (layers "*.Cu" "*.Mask") ${p.to.str})

    (pad "2" smd roundrect (at ${7.35 - (2.6 - p.outer_pad_width_front) / 2} 4.7 ${p.r}) (size ${p.outer_pad_width_front + 1.4} 2.5) (layers "F.Cu") (roundrect_rratio 0.1) ${p.to.str})
    (pad "" smd roundrect (at ${8.05 - (2.6 - p.outer_pad_width_front) / 2} 4.7 ${p.r}) (size ${p.outer_pad_width_front} 2.5) (layers "F.Paste" "F.Mask") (roundrect_rratio 0.1))

    ${!p.reversible ? hotswap_front_full
        : `(pad "1" smd roundrect (at -6.25 5.75 ${p.r}) (size 2.6 2.5) (layers "F.Cu" "F.Paste" "F.Mask") (roundrect_rratio 0.1) ${p.from.str})`}
    `

    const hotswap_back = `
    (pad "1" thru_hole circle (at -4.4 4.7 ${p.r}) (size 3.5 3.5) (drill 3) (layers "*.Cu" "*.Mask") ${p.from.str})
    (pad ${p.reversible ? '""' : '"2"'} thru_hole circle (at 2.6 5.75 ${p.r}) (size 3.5 3.5) (drill 3) (layers "*.Cu" "*.Mask") ${p.to.str})

    
    (pad "1" smd roundrect (at ${-7.35 + (2.6 - p.outer_pad_width_back) / 2} 4.7 ${p.r}) (size ${p.outer_pad_width_back + 1.4} 2.5) (layers "B.Cu") (roundrect_rratio 0.1) ${p.from.str})
    (pad "" smd roundrect (at ${-8.05 + (2.6 - p.outer_pad_width_back) / 2} 4.7 ${p.r}) (size ${p.outer_pad_width_back} 2.5) (layers "B.Paste" "B.Mask") (roundrect_rratio ${(2.5 / p.outer_pad_width_back) <= 1 ? 0.1 : 0.1 * (2.5 / p.outer_pad_width_back)}))
    
    ${!p.reversible ? hotswap_back_full
        : `(pad "2" smd roundrect (at 6.25 5.75 ${p.r}) (size 2.6 2.5) (layers "B.Cu" "B.Paste" "B.Mask") (roundrect_rratio 0.1) ${p.to.str})`}
    `


    // If both hotswap and solder are enabled, move the solder holes
    // "down" to the opposite side of the switch.
    // Since switches can be r3otated by 90 degrees, this won't be a
    // problem as long as we switch the side the holes are on.
    //let solder_offset_x_front = ''
    //let solder_offset_x_back = ''
    let solder_offset_y = ''
    if (p.hotswap && p.solder) {
      //solder_offset_x_front = ''
      //solder_offset_x_back = ''
      solder_offset_y = '-'
    }

    const solder_back = `
    (pad "1" thru_hole circle (at -2.6 ${solder_offset_y}5.75 ${p.r}) (size 2.1 2.1) (drill 1.25) (layers "*.Cu" "*.Mask") ${p.from.str})
    (pad "2" thru_hole circle (at 4.4 ${solder_offset_y}4.7 ${p.r}) (size 2.1 2.1) (drill 1.25) (layers "*.Cu" "*.Mask") ${p.to.str})
    `

    const solder_front = `
    (pad "1" thru_hole circle (at -4.4 ${solder_offset_y}4.7 ${p.r}) (size 2.1 2.1) (drill 1.25) (layers "*.Cu" "*.Mask") ${p.from.str})
    (pad "2" thru_hole circle (at 2.6 ${solder_offset_y}5.75 ${p.r}) (size 2.1 2.1) (drill 1.25) (layers "*.Cu" "*.Mask") ${p.to.str})
    `
    
    const solder_custom_reversible_top = `
    (pad "" thru_hole circle (at -4.4 -4.7) (size 1.8 1.8) (drill 1.25) (layers "*.Cu" "*.Mask") ${p.from.str})
    (pad "" thru_hole circle (at -2.6 -5.75) (size 1.8 1.8) (drill 1.25) (layers "*.Cu" "*.Mask") ${p.from.str})
    (pad "1" smd custom (at -2.6 -5.75 ${p.r}) (size 1 1) (layers "F.Cu") (thermal_bridge_angle 90) (options (clearance outline) (anchor circle))
      (primitives (gr_poly (pts (xy -0.19509 -0.980785) (xy -0.382683 -0.92388) (xy -0.55557 -0.83147) (xy -2.35557 0.21853) (xy -2.507107 0.342893) (xy -2.63147 0.49443) (xy -2.72388 0.667317) (xy -2.780785 0.85491) (xy -2.8 1.05) (xy -2.780785 1.24509) (xy -2.72388 1.432683) (xy -2.63147 1.60557) (xy -2.507107 1.757107) (xy -2.35557 1.88147) (xy -2.182683 1.97388) (xy -1.99509 2.030785) (xy -1.8 2.05) (xy -1.60491 2.030785) (xy -1.417317 1.97388) (xy -1.24443 1.88147) (xy 0.55557 0.83147) (xy 0.707107 0.707107) (xy 0.83147 0.55557) (xy 0.92388 0.382683) (xy 0.980785 0.19509) (xy 1 0) (xy 0.980785 -0.19509) (xy 0.92388 -0.382683) (xy 0.83147 -0.55557) (xy 0.707107 -0.707107) (xy 0.55557 -0.83147) (xy 0.382683 -0.92388) (xy 0.19509 -0.980785) (xy 0 -1)) (width 0.1) (fill yes)))
      ${p.from.str})
    (pad "1" smd custom (at -2.6 -5.75 ${p.r}) (size 1 1) (layers "B.Cu") (thermal_bridge_angle 90) (options (clearance outline) (anchor circle))
      (primitives (gr_poly (pts (xy -0.19509 -0.980785) (xy -0.382683 -0.92388) (xy -0.55557 -0.83147) (xy -2.35557 0.21853) (xy -2.507107 0.342893) (xy -2.63147 0.49443) (xy -2.72388 0.667317) (xy -2.780785 0.85491) (xy -2.8 1.05) (xy -2.780785 1.24509) (xy -2.72388 1.432683) (xy -2.63147 1.60557) (xy -2.507107 1.757107) (xy -2.35557 1.88147) (xy -2.182683 1.97388) (xy -1.99509 2.030785) (xy -1.8 2.05) (xy -1.60491 2.030785) (xy -1.417317 1.97388) (xy -1.24443 1.88147) (xy 0.55557 0.83147) (xy 0.707107 0.707107) (xy 0.83147 0.55557) (xy 0.92388 0.382683) (xy 0.980785 0.19509) (xy 1 0) (xy 0.980785 -0.19509) (xy 0.92388 -0.382683) (xy 0.83147 -0.55557) (xy 0.707107 -0.707107) (xy 0.55557 -0.83147) (xy 0.382683 -0.92388) (xy 0.19509 -0.980785) (xy 0 -1)) (width 0.1) (fill yes)))
      ${p.from.str})

    (pad "" thru_hole circle (at 2.6 -5.75) (size 1.8 1.8) (drill 1.25) (layers "*.Cu" "*.Mask") ${p.to.str})
    (pad "" thru_hole circle (at 4.4 -4.7) (size 1.8 1.8) (drill 1.25) (layers "*.Cu" "*.Mask") ${p.to.str})
    (pad "2" smd custom (at 2.6 -5.75 ${p.r}) (size 1 1) (layers "F.Cu") (thermal_bridge_angle 90) (options (clearance outline) (anchor circle))
      (primitives (gr_poly (pts (xy 0.19509 -0.980785) (xy 0.382683 -0.92388) (xy 0.55557 -0.83147) (xy 2.35557 0.21853) (xy 2.507107 0.342893) (xy 2.63147 0.49443) (xy 2.72388 0.667317) (xy 2.780785 0.85491) (xy 2.8 1.05) (xy 2.780785 1.24509) (xy 2.72388 1.432683) (xy 2.63147 1.60557) (xy 2.507107 1.757107) (xy 2.35557 1.88147) (xy 2.182683 1.97388) (xy 1.99509 2.030785) (xy 1.8 2.05) (xy 1.60491 2.030785) (xy 1.417317 1.97388) (xy 1.24443 1.88147) (xy -0.55557 0.83147) (xy -0.707107 0.707107) (xy -0.83147 0.55557) (xy -0.92388 0.382683) (xy -0.980785 0.19509) (xy -1 0) (xy -0.980785 -0.19509) (xy -0.92388 -0.382683) (xy -0.83147 -0.55557) (xy -0.707107 -0.707107) (xy -0.55557 -0.83147) (xy -0.382683 -0.92388) (xy -0.19509 -0.980785) (xy 0 -1)) (width 0.1) (fill yes)))
      ${p.to.str})
    (pad "2" smd custom (at 2.6 -5.75 ${p.r}) (size 1 1) (layers "B.Cu") (thermal_bridge_angle 90) (options (clearance outline) (anchor circle))
      (primitives (gr_poly (pts (xy 0.19509 -0.980785) (xy 0.382683 -0.92388) (xy 0.55557 -0.83147) (xy 2.35557 0.21853) (xy 2.507107 0.342893) (xy 2.63147 0.49443) (xy 2.72388 0.667317) (xy 2.780785 0.85491) (xy 2.8 1.05) (xy 2.780785 1.24509) (xy 2.72388 1.432683) (xy 2.63147 1.60557) (xy 2.507107 1.757107) (xy 2.35557 1.88147) (xy 2.182683 1.97388) (xy 1.99509 2.030785) (xy 1.8 2.05) (xy 1.60491 2.030785) (xy 1.417317 1.97388) (xy 1.24443 1.88147) (xy -0.55557 0.83147) (xy -0.707107 0.707107) (xy -0.83147 0.55557) (xy -0.92388 0.382683) (xy -0.980785 0.19509) (xy -1 0) (xy -0.980785 -0.19509) (xy -0.92388 -0.382683) (xy -0.83147 -0.55557) (xy -0.707107 -0.707107) (xy -0.55557 -0.83147) (xy -0.382683 -0.92388) (xy -0.19509 -0.980785) (xy 0 -1)) (width 0.1) (fill yes)))
      ${p.to.str})

    `

    const solder_custom_reversible_bottom = `
    (pad "" thru_hole circle (at -4.4 4.7) (size 1.8 1.8) (drill 1.25) (layers "*.Cu" "*.Mask") ${p.from.str})
    (pad "" thru_hole circle (at -2.6 5.75) (size 1.8 1.8) (drill 1.25) (layers "*.Cu" "*.Mask") ${p.from.str})
    (pad "1" smd custom (at -4.4 4.7 ${p.r}) (size 1 1) (layers "F.Cu") (thermal_bridge_angle 90) (options (clearance outline) (anchor circle)) 
      (primitives (gr_poly (pts (xy 0.19509 -0.980785) (xy 0.382683 -0.92388) (xy 0.55557 -0.83147) (xy 2.35557 0.21853) (xy 2.507107 0.342893) (xy 2.63147 0.49443) (xy 2.72388 0.667317) (xy 2.780785 0.85491) (xy 2.8 1.05) (xy 2.780785 1.24509) (xy 2.72388 1.432683) (xy 2.63147 1.60557) (xy 2.507107 1.757107) (xy 2.35557 1.88147) (xy 2.182683 1.97388) (xy 1.99509 2.030785) (xy 1.8 2.05) (xy 1.60491 2.030785) (xy 1.417317 1.97388) (xy 1.24443 1.88147) (xy -0.55557 0.83147) (xy -0.707107 0.707107) (xy -0.83147 0.55557) (xy -0.92388 0.382683) (xy -0.980785 0.19509) (xy -1 0) (xy -0.980785 -0.19509) (xy -0.92388 -0.382683) (xy -0.83147 -0.55557) (xy -0.707107 -0.707107) (xy -0.55557 -0.83147) (xy -0.382683 -0.92388) (xy -0.19509 -0.980785) (xy 0 -1)) (width 0.1) (fill yes)))
       ${p.from.str})
    (pad "1" smd custom (at -4.4 4.7 ${p.r}) (size 1 1) (layers "B.Cu") (thermal_bridge_angle 90) (options (clearance outline) (anchor circle)) 
      (primitives (gr_poly (pts (xy 0.19509 -0.980785) (xy 0.382683 -0.92388) (xy 0.55557 -0.83147) (xy 2.35557 0.21853) (xy 2.507107 0.342893) (xy 2.63147 0.49443) (xy 2.72388 0.667317) (xy 2.780785 0.85491) (xy 2.8 1.05) (xy 2.780785 1.24509) (xy 2.72388 1.432683) (xy 2.63147 1.60557) (xy 2.507107 1.757107) (xy 2.35557 1.88147) (xy 2.182683 1.97388) (xy 1.99509 2.030785) (xy 1.8 2.05) (xy 1.60491 2.030785) (xy 1.417317 1.97388) (xy 1.24443 1.88147) (xy -0.55557 0.83147) (xy -0.707107 0.707107) (xy -0.83147 0.55557) (xy -0.92388 0.382683) (xy -0.980785 0.19509) (xy -1 0) (xy -0.980785 -0.19509) (xy -0.92388 -0.382683) (xy -0.83147 -0.55557) (xy -0.707107 -0.707107) (xy -0.55557 -0.83147) (xy -0.382683 -0.92388) (xy -0.19509 -0.980785) (xy 0 -1)) (width 0.1) (fill yes)))
       ${p.from.str})

    (pad "" thru_hole circle (at 2.6 5.75) (size 1.8 1.8) (drill 1.25) (layers "*.Cu" "*.Mask") ${p.to.str})
    (pad "" thru_hole circle (at 4.4 4.7) (size 1.8 1.8) (drill 1.25) (layers "*.Cu" "*.Mask") ${p.to.str})
    (pad "2" smd custom (at 4.4 4.7 ${p.r}) (size 1 1) (layers "F.Cu") (thermal_bridge_angle 90) (options (clearance outline) (anchor circle))
      (primitives (gr_poly (pts (xy -0.19509 -0.980785) (xy -0.382683 -0.92388) (xy -0.55557 -0.83147) (xy -2.35557 0.21853) (xy -2.507107 0.342893) (xy -2.63147 0.49443) (xy -2.72388 0.667317) (xy -2.780785 0.85491) (xy -2.8 1.05) (xy -2.780785 1.24509) (xy -2.72388 1.432683) (xy -2.63147 1.60557) (xy -2.507107 1.757107) (xy -2.35557 1.88147) (xy -2.182683 1.97388) (xy -1.99509 2.030785) (xy -1.8 2.05) (xy -1.60491 2.030785) (xy -1.417317 1.97388) (xy -1.24443 1.88147) (xy 0.55557 0.83147) (xy 0.707107 0.707107) (xy 0.83147 0.55557) (xy 0.92388 0.382683) (xy 0.980785 0.19509) (xy 1 0) (xy 0.980785 -0.19509) (xy 0.92388 -0.382683) (xy 0.83147 -0.55557) (xy 0.707107 -0.707107) (xy 0.55557 -0.83147) (xy 0.382683 -0.92388) (xy 0.19509 -0.980785) (xy 0 -1)) (width 0.1) (fill yes)))
      ${p.to.str})
    (pad "2" smd custom (at 4.4 4.7 ${p.r}) (size 1 1) (layers "B.Cu") (thermal_bridge_angle 90) (options (clearance outline) (anchor circle)) 
      (primitives (gr_poly (pts (xy -0.19509 -0.980785) (xy -0.382683 -0.92388) (xy -0.55557 -0.83147) (xy -2.35557 0.21853) (xy -2.507107 0.342893) (xy -2.63147 0.49443) (xy -2.72388 0.667317) (xy -2.780785 0.85491) (xy -2.8 1.05) (xy -2.780785 1.24509) (xy -2.72388 1.432683) (xy -2.63147 1.60557) (xy -2.507107 1.757107) (xy -2.35557 1.88147) (xy -2.182683 1.97388) (xy -1.99509 2.030785) (xy -1.8 2.05) (xy -1.60491 2.030785) (xy -1.417317 1.97388) (xy -1.24443 1.88147) (xy 0.55557 0.83147) (xy 0.707107 0.707107) (xy 0.83147 0.55557) (xy 0.92388 0.382683) (xy 0.980785 0.19509) (xy 1 0) (xy 0.980785 -0.19509) (xy 0.92388 -0.382683) (xy 0.83147 -0.55557) (xy 0.707107 -0.707107) (xy 0.55557 -0.83147) (xy 0.382683 -0.92388) (xy 0.19509 -0.980785) (xy 0 -1)) (width 0.1) (fill yes)))
      ${p.to.str})
    `

    // KS-33 feet define the mounting plane; hotswap mounts opposite the footprint.
    const model_center_x = 60;
    const model_mount_z = 3.25;
    const model_back = p.side === 'B';
    const model_rotation = p.switch_3dmodel_xyz_rotation || (p.hotswap
      ? (model_back ? [180, 0, 0] : [0, 180, 0])
      : (model_back ? [0, 0, 180] : [0, 0, 0]));
    const model_offset = p.switch_3dmodel_xyz_offset || [
      model_back === p.hotswap ? -model_center_x : model_center_x,
      0,
      p.hotswap ? model_mount_z - p.pcb_thickness : -model_mount_z,
    ];
    const switch_3dmodel = `
    (model ${JSON.stringify(p.switch_3dmodel_filename)}
      (offset (xyz ${model_offset[0]} ${model_offset[1]} ${model_offset[2]}))
      (scale (xyz ${p.switch_3dmodel_xyz_scale[0]} ${p.switch_3dmodel_xyz_scale[1]} ${p.switch_3dmodel_xyz_scale[2]}))
      (rotate (xyz ${model_rotation[0]} ${model_rotation[1]} ${model_rotation[2]}))
    )
    `

    const hotswap_3dmodel = `
    (model ${JSON.stringify(p.hotswap_3dmodel_filename)}
      (offset (xyz ${p.hotswap_3dmodel_xyz_offset[0]} ${p.hotswap_3dmodel_xyz_offset[1]} ${p.hotswap_3dmodel_xyz_offset[2]}))
      (scale (xyz ${p.hotswap_3dmodel_xyz_scale[0]} ${p.hotswap_3dmodel_xyz_scale[1]} ${p.hotswap_3dmodel_xyz_scale[2]}))
      (rotate (xyz ${p.hotswap_3dmodel_xyz_rotation[0]} ${p.hotswap_3dmodel_xyz_rotation[1]} ${p.hotswap_3dmodel_xyz_rotation[2]}))
    )
	  `

    const keycap_3dmodel = `
    (model ${JSON.stringify(p.keycap_3dmodel_filename)}
      (offset (xyz ${p.keycap_3dmodel_xyz_offset[0]} ${p.keycap_3dmodel_xyz_offset[1]} ${p.keycap_3dmodel_xyz_offset[2]}))
      (scale (xyz ${p.keycap_3dmodel_xyz_scale[0]} ${p.keycap_3dmodel_xyz_scale[1]} ${p.keycap_3dmodel_xyz_scale[2]}))
      (rotate (xyz ${p.keycap_3dmodel_xyz_rotation[0]} ${p.keycap_3dmodel_xyz_rotation[1]} ${p.keycap_3dmodel_xyz_rotation[2]}))
    )
	  `

    const common_bottom = `
  )
    `
    
    let final = common_top

    if (p.include_corner_marks) {
      final += corner_marks;
    }

    if (p.include_keycap) {
      final += keycap_marks
    }

    if (p.include_led_outline) {
      final += led_outline
    }

    if (p.include_stem_outline) {
      final += stem_outline
      final += stem_cross_outline
    }

    if (p.hotswap) {
      if (p.reversible || p.side == "F") {
        final += hotswap_front

        if (p.include_socket_silks) {
          final += hotswap_silk_front
        }

        if (p.include_socket_fabs) {
          final += hotswap_fab_front
        }
      }

      if (p.reversible || p.side == "B") {
        final += hotswap_back

        if (p.include_socket_silks) {
          final += hotswap_silk_back
        }

        if (p.include_socket_fabs) {
          final += hotswap_fab_back
        }
      }

      if (p.hotswap_3dmodel_filename) {
        final += hotswap_3dmodel
      }
    }
      
    if (p.solder) {
      if (p.reversible && p.include_custom_solder_pads) {
        if (p.hotswap) {
          final += solder_custom_reversible_top
        } else {
          final += solder_custom_reversible_bottom
        }
      } else {
        if (p.reversible || p.side == "F") {
          final += solder_front
        }

        if (p.reversible || p.side == "B") {
          final += solder_back
        }
      }
    }
    
    if (p.switch_3dmodel_filename) {
      final += switch_3dmodel
    }

    if (p.keycap_3dmodel_filename) {
      final += keycap_3dmodel
    }

    final += common_bottom
    return final
  }
}

;module.exports = ((original) => {
    const defaults = {"switch_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/gdek/KS33.stp","switch_3dmodel_xyz_offset":"","switch_3dmodel_xyz_rotation":""};
    const bossVariant = null;
    const chocV2 = "${KIPRJMOD}/models/boardstudio/koktoh/Choc_V2_Red.step";
    return {
      ...original,
      params: {...original.params, ...defaults},
      body: p => {
        // The bundled extension crosses the PCB with a 5 mm outer diameter.
        const extensionDiameter = 5;
        if (false && p.drill < extensionDiameter
          && p.tp_extension_3dmodel_filename === defaults.tp_extension_3dmodel_filename
          && !p.tp_extension_3dmodel_xyz_scale
          && !p.tp_extension_3dmodel_xyz_rotation
          && !p.tp_extension_3dmodel_xyz_offset) {
          throw new Error('The bundled trackpoint extension requires a center drill of at least 5 mm. Select a compatible extension model or drill size.');
        }
        if (false && !p.choc_v1_support) {
          p = {...p};
          if (!p.choc_v2_support) {
            for (const key of Object.keys(defaults)) {
              if (p[key] === defaults[key]) { p[key] = ''; }
            }
          } else {
            if (p.switch_3dmodel_filename === defaults.switch_3dmodel_filename) {
              const automatic = !p.switch_3dmodel_xyz_rotation && !p.switch_3dmodel_xyz_offset
                && p.switch_3dmodel_xyz_scale.every(value => value === 1);
              const postDiameter = 4.8;
              if (automatic && (!p.include_stabilizer_pad || p.oval_stabilizer_pad
                || (p.center_hole_diameter > 0 && p.center_hole_diameter < postDiameter))) {
                throw new Error('The bundled Choc V2 model requires the round stabilizer hole and a center drill of at least 4.8 mm. Use a matching model or transform for modified hardware.');
              }
              p.switch_3dmodel_filename = chocV2;
              p.switch_3dmodel_xyz_rotation ||= p.side === 'F' ? [180, 0, 0] : [0, 180, 0];
              p.switch_3dmodel_xyz_offset ||= [0, 0, -p.pcb_thickness];
            }
            if (p.keycap_3dmodel_filename === defaults.keycap_3dmodel_filename) {
              p.keycap_3dmodel_filename = '';
            }
          }
        }
        if (bossVariant && p.include_bosses && p.reset_switch_3dmodel_filename === defaults.reset_switch_3dmodel_filename) {
          p = {...p, reset_switch_3dmodel_filename: bossVariant};
        }
        return original.body(p);
      }
    };
  })(module.exports);

return module.exports; })(),
  "ceoloide/switch_mx": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
/*
Copyright (c) 2023 Marco Massarelli

SPDX-License-Identifier: MIT

To view a copy of this license, visit https://opensource.org/license/mit/

Author: @ceoloide

Description:
   Cherry MX compatible reversible and hotswappable footprint.

Notes:
- Hotswap and solder can be used together. The solder holes will then be
  added above the hotswap holes.

Nets:
   from: corresponds to pin 1
   to: corresponds to pin 2

Params:
  pcb_thickness: default is 1.6 mm
    Must match the board thickness for bundled model placement.
  side: default is B for Back
    the side on which to place the single-side footprint and designator, either F or B
  reversible: default is false
    if true, the footprint will be placed on both sides so that the PCB can be
    reversible
  hotswap: default is true
    if true, will include holes and pads for Kailh choc hotswap sockets
  hotswap_pads_same_side: false
    if true, and if not using plated holes, it will position the pads so that nets are on
    the same side to simplify routing.
  include_traces_vias: default is true
    if true it will include traces and vias when hotswap is true, footprint is reversible
    and when no plated holes are used, to simplify routing. In the other cases it's simply
    not needed.
  trace_width: default is 0.200mm
    allows to override the trace width that connects the pads. Not recommended
    to go below 0.15mm (JLCPC min is 0.127mm), or above 0.200mm to avoid DRC errors.
  via_size: default is 0.6
    allows to define the size of the via. Not recommended below 0.56 (JLCPCB minimum),
    or above 0.8 (KiCad default), to avoid overlap or DRC errors
  via_drill: default is 0.3
    allows to define the size of the drill. Not recommended below 0.3 (JLCPCB minimum),
    or above 0.4 (KiCad default), to avoid overlap or DRC errors 
  locked_traces_vias: false
    sets the traces and vias as locked in KiCad. Locked objects may not be manipulated
    or moved, and cannot be selected unless the Locked Items option is enabled in the
    Selection Filter panel in KiCad. Useful for a faster workflow. If using autorouting
    solutions like Freerouting, locking can prevent the traces and vias from being
    replaced.
  solder: default is false
    if true, will include holes to solder switches (works with hotswap too)
  include_plated_holes: default is false
    Alternate version of the footprint compatible with side, reversible, hotswap, solder
    options in any combination. Pretty, allows for connecting ground fill zones via
    center hole, and allows for setting nets on stabilizers for your routing needs.
  include_stabilizer_nets: default is false
    if true, will add adjustable nets to plated stabilizer holes, 
    LEFTSTAB: default is "D1"
    RIGHTSTAB: default is "D2"
  include_centerhole_net: default is false
    if true, will add adjustable net to the center hole
    CENTERHOLE: default is "GND"
  outer_pad_width_front: default 2.6
  outer_pad_width_back: default 2.6
    Allows you to make the outer hotswap pads smaller to silence DRC
    warnings when the sockets are too close to the edge cuts. It's not
    recommended to go below 1.6mm to ensure the hotswap socket can be
    properly soldered.
  outer_pad_height: default 2.5 (mm)
    Allows you to make the outer hotswap pad vertically shorter, e.g. to make room for 
    encoder pads to be colocated with the switch (use 2.0mm for EC12).  
   stabilizers_diameter: default is 1.9 (mm)
     Allows you to narrow stabilizer / boss holes diameter for tighter fit, not
     recommended to set below 1.7mm.
  include_keycap: default is false
    if true, will add mx sized keycap box around the footprint (18mm)
  keycap_width: default is 18 (mm - default MX size)
    Allows you to adjust the height of the keycap outline.
  keycap_height: default is 18 (mm - default MX size)
    Allows you to adjust the width of the keycap outline. For example,
    to show a 1.5u outline for easier aligning.
  include_corner_marks: default is false
    if true, will add corner marks to indicate plate hole size and position
  include_silkscreen: default is true
    if true it will include the silkscreen. Recommended to be true to ensure connector
    polarity is not reversed, which can lead to shorting and damage to the MCU
  switch_3dmodel_filename: default is ''
    Allows you to specify the path to a 3D model STEP or WRL file to be
    used when rendering the PCB. Use the ${VAR_NAME} syntax to point to
    a KiCad configured path.
  switch_3dmodel_xyz_offset: default is [0, 0, 0]
    xyz offset (in mm), used to adjust the position of the 3d model
    relative the footprint.
  switch_3dmodel_xyz_scale: default is [1, 1, 1]
    xyz scale, used to adjust the size of the 3d model relative to its
    original size.
  switch_3dmodel_xyz_rotation: default is [0, 0, 0]
    xyz rotation (in degrees), used to adjust the orientation of the 3d
    model relative the footprint.
  hotswap_3dmodel_filename: default is ''
    Allows you to specify the path to a 3D model to be used when rendering
    the PCB. Allows for paths using a configured path by using the
    ${VAR_NAME} syntax.
  hotswap_3dmodel_xyz_offset: default is [0, 0, 0]
    xyz offset (in mm), used to adjust the position of the 3d model
    relative the footprint.
  hotswap_3dmodel_xyz_scale: default is [1, 1, 1]
    xyz scale, used to adjust the size of the 3d model relative its
    original size.
  hotswap_3dmodel_xyz_rotation: default is [0, 0, 0]
    xyz rotation (in degrees), used to adjust the orientation of the 3d
    model relative the footprint.
  keycap_3dmodel_filename: default is ''
    Allows you to specify the path to a 3D model STEP or WRL file to be
    used when rendering the PCB. Use the ${VAR_NAME} syntax to point to
    a KiCad configured path.
  keycap_3dmodel_xyz_offset: default is [0, 0, 0]
    xyz offset (in mm), used to adjust the position of the 3d model
    relative the footprint.
  keycap_3dmodel_xyz_scale: default is [1, 1, 1]
    xyz scale, used to adjust the size of the 3d model relative to its
    original size.
  keycap_3dmodel_xyz_rotation: default is [0, 0, 0]
    xyz rotation (in degrees), used to adjust the orientation of the 3d
    model relative the footprint.
*/

module.exports = {
  params: {
    designator: 'S',
    side: 'B',
    pcb_thickness: 1.6,
    reversible: false,
    hotswap: true,
    hotswap_pads_same_side: false,
    include_traces_vias: true,
    trace_width: 0.2,
    via_size: 0.6,
    via_drill: 0.3,
    locked_traces_vias: false,
    include_plated_holes: false,
    include_stabilizer_nets: false,
    include_centerhole_net: false,
    solder: false,
    outer_pad_width_front: 2.6,
    outer_pad_width_back: 2.6,
    outer_pad_height: 2.5,
    stabilizers_diameter: 1.9,
    include_keycap: false,
    keycap_width: 18,
    keycap_height: 18,
    include_corner_marks: false,
    include_silkscreen: true,
    switch_3dmodel_filename: '',
    switch_3dmodel_xyz_offset: [0, 0, 0],
    switch_3dmodel_xyz_rotation: [0, 0, 0],
    switch_3dmodel_xyz_scale: [1, 1, 1],
    hotswap_3dmodel_filename: '',
    hotswap_3dmodel_xyz_offset: [0, 0, 0],
    hotswap_3dmodel_xyz_rotation: [0, 0, 0],
    hotswap_3dmodel_xyz_scale: [1, 1, 1],
    keycap_3dmodel_filename: '',
    keycap_3dmodel_xyz_offset: [0, 0, 0],
    keycap_3dmodel_xyz_rotation: [0, 0, 0],
    keycap_3dmodel_xyz_scale: [1, 1, 1],
    from: undefined,
    to: undefined,
    CENTERHOLE: { type: 'net', value: 'GND'},
    LEFTSTAB: { type: 'net', value: 'D1' },
    RIGHTSTAB: { type: 'net', value: 'D2' }
  },
  body: p => {
    const common_top = `
  (footprint "ceoloide:switch_mx"
    (layer "${p.side}.Cu")
    ${p.at}
    (property "Reference" "${p.ref}"
      (at 0 -7.5 180)
      (layer "${p.side}.SilkS")
      ${p.ref_hide}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    
    (pad "" ${!p.include_plated_holes ? `np_thru_hole` : `thru_hole`} circle 
      (at 0 0 ${p.r})
      (size ${p.include_plated_holes ? `4.4 4.4` : `4.1 4.1`})
      (drill 4.1)
      (layers "*.Cu" "*.Mask")
      ${p.include_plated_holes && p.include_centerhole_net ? p.CENTERHOLE : ''}
    )
    (pad "" ${!p.include_plated_holes ? `np_thru_hole` : `thru_hole`} circle 
      (at 5.08 0 ${p.r})
      (size ${p.stabilizers_diameter + (p.include_plated_holes ? 0.3 : 0)} ${p.stabilizers_diameter + (p.include_plated_holes ? 0.3 : 0)})
      (drill ${p.stabilizers_diameter})
      (layers "*.Cu" "*.Mask")
      ${p.include_plated_holes && p.include_stabilizer_nets ? p.RIGHTSTAB : ''}
    )
    (pad "" ${!p.include_plated_holes ? `np_thru_hole` : `thru_hole`} circle 
      (at -5.08 0 ${p.r})
      (size ${p.stabilizers_diameter + (p.include_plated_holes ? 0.3 : 0)} ${p.stabilizers_diameter + (p.include_plated_holes ? 0.3 : 0)})
      (drill ${p.stabilizers_diameter})
      (layers "*.Cu" "*.Mask")
      ${p.include_plated_holes && p.include_stabilizer_nets ? p.LEFTSTAB : ''}
    )
    `
    const corner_marks = `
    (fp_line (start -7 -6) (end -7 -7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -7 7) (end -6 7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -6 -7) (end -7 -7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start -7 7) (end -7 6) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start 7 6) (end 7 7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start 7 -7) (end 6 -7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start 6 7) (end 7 7) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start 7 -7) (end 7 -6) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    `
    const keycap_xo = 0.5 * p.keycap_width
    const keycap_yo = 0.5 * p.keycap_height
    const keycap_marks = `
    (fp_rect (start ${keycap_xo} ${keycap_yo}) (end ${-keycap_xo} ${-keycap_yo}) (layer "Dwgs.User") (stroke (width 0.15) (type solid)) (fill none))
    `

    const hotswap_front = `
		(pad "" np_thru_hole circle (at -2.54 -5.08 180) (size 3 3) (drill 3) (layers "F&B.Cu" "*.Mask"))
		(pad "" np_thru_hole circle (at 3.81 -2.54 180) (size 3 3) (drill 3) (layers "F&B.Cu" "*.Mask"))
		(pad "1" smd rect (at ${5.81 + p.outer_pad_width_front / 2} -2.54 ${p.r}) (size ${p.outer_pad_width_front} ${p.outer_pad_height}) (layers "F.Cu" "F.Paste" "F.Mask") ${p.from})
		(pad "2" smd ${p.reversible ? 'roundrect' : 'rect'}
      (at -5.842 -5.08 ${p.r})
      (size 2.55 2.5)
      (layers "F.Cu" "F.Paste" "F.Mask")${p.reversible ? `
			(roundrect_rratio 0)
			(chamfer_ratio 0.2)
			(chamfer bottom_right)` : ''}
      ${p.to}
    )
    `

    const hotswap_back = `
		(pad "" np_thru_hole circle (at 2.54 -5.08 180) (size 3 3) (drill 3) (layers "F&B.Cu" "*.Mask"))
		(pad "" np_thru_hole circle (at -3.81 -2.54 180) (size 3 3) (drill 3) (layers "F&B.Cu" "*.Mask"))
		(pad "1" smd rect
      (at ${-5.81 - p.outer_pad_width_back / 2} -2.54 ${p.r})
      (size ${p.outer_pad_width_back} ${p.outer_pad_height})
      (layers "B.Cu" "B.Paste" "B.Mask")
      ${p.hotswap_pads_same_side ? p.to : p.from}
    )
		(pad "2" smd ${p.reversible ? 'roundrect' : 'rect'}
      (at 5.842 -5.08 ${p.r})
      (size 2.55 2.5)
      (layers "B.Cu" "B.Paste" "B.Mask")${p.reversible ? `
			(roundrect_rratio 0)
			(chamfer_ratio 0.2)
			(chamfer bottom_left)` : ''}
      ${p.hotswap_pads_same_side ? p.from : p.to}
    )
    `

    const hotswap_silkscreen_back = `
		(fp_poly
			(pts
				(xy -3.6 -6.5) (xy -3.8 -6.5) (xy -4.1 -6.45) (xy -4.4 -6.35) (xy -4.6 -6.25) (xy -4.75 -6.15) (xy -4.95 -6)
				(xy -5.1 -5.85) (xy -5.25 -5.65) (xy -5.4 -5.4) (xy -5.5 -5) (xy -5.5 -4.6) (xy -5.35 -4.5) (xy -5.2 -4.4)
				(xy -4.75 -4.65) (xy -4.5 -4.75) (xy -4.05 -4.85) (xy -3.55 -4.85) (xy -2.95 -4.7) (xy -2.45 -4.4) (xy -2.15 -4.15)
        (xy -1.75 -3.6) (xy -1.55 -3.05) (xy -1.5 -2.6) (xy -1.25 -2.8) (xy -0.9 -2.9) (xy -0.4 -2.95) (xy 1.65 -2.95)
        (xy 1.2 -3.2) (xy 0.95 -3.4) (xy 0.65 -3.75) (xy 0.5 -4) (xy 0.35 -4.35) (xy 0.25 -4.75) (xy 0.25 -5.05)
        (xy 0.25 -5.4) (xy 0.3 -5.65) (xy 0.45 -6.05) (xy 0.75 -6.5)
			)
			(stroke (width 0.4) (type solid))
			(fill solid)
			(layer "B.SilkS")
		)
    `
    const hotswap_silkscreen_front = `
		(fp_poly
			(pts
				(xy 3.6 -6.5) (xy 3.8 -6.5) (xy 4.1 -6.45) (xy 4.4 -6.35) (xy 4.6 -6.25) (xy 4.75 -6.15) (xy 4.95 -6)
				(xy 5.1 -5.85) (xy 5.25 -5.65) (xy 5.4 -5.4) (xy 5.5 -5) (xy 5.5 -4.6) (xy 5.35 -4.5) (xy 5.2 -4.4)
				(xy 4.75 -4.65) (xy 4.5 -4.75) (xy 4.05 -4.85) (xy 3.55 -4.85) (xy 2.95 -4.7) (xy 2.45 -4.4) (xy 2.15 -4.15)
				(xy 1.75 -3.6) (xy 1.55 -3.05) (xy 1.5 -2.6) (xy 1.25 -2.8) (xy 0.9 -2.9) (xy 0.4 -2.95) (xy -1.65 -2.95)
				(xy -1.2 -3.2) (xy -0.95 -3.4) (xy -0.65 -3.75) (xy -0.5 -4) (xy -0.35 -4.35) (xy -0.25 -4.75) (xy -0.25 -5.05)
				(xy -0.25 -5.4) (xy -0.3 -5.65) (xy -0.45 -6.05) (xy -0.75 -6.5)
			)
			(stroke (width 0.4) (type solid))
			(fill solid)
			(layer "F.SilkS")
		)
    `
    const hotswap_silkscreen_reversible = `
		(fp_line (start 1.22 -3.77) (end 0 -2.52) (stroke (width 0.1) (type default)) (layer "B.SilkS"))
		(fp_line (start 0 -2.52) (end -1.88 -2.52) (stroke (width 0.1) (type default)) (layer "B.SilkS"))
		(fp_line (start -1.22 -3.77) (end 0 -2.52) (stroke (width 0.1) (type default)) (layer "F.SilkS"))
		(fp_line (start 0 -2.52) (end 1.88 -2.52) (stroke (width 0.1) (type default)) (layer "F.SilkS"))
    `

    const solder_front = `
    (pad "1" thru_hole circle (at ${p.solder && p.hotswap ? '' : '-'}2.54 ${p.solder && p.hotswap ? '' : '-'}5.08) (size 2.286 2.286) (drill 1.4986) (layers "F&B.Cu" "*.Mask") ${p.from})
    (pad "2" thru_hole circle (at ${p.solder && p.hotswap ? '-' : ''}3.81 ${p.solder && p.hotswap ? '' : '-'}2.54) (size 2.286 2.286) (drill 1.4986) (layers "F&B.Cu" "*.Mask") ${p.to})
    `
    const solder_back = `
    (pad "1" thru_hole circle (at ${p.solder && p.hotswap ? '-' : ''}2.54 ${p.solder && p.hotswap ? '' : '-'}5.08) (size 2.286 2.286) (drill 1.4986) (layers "F&B.Cu" "*.Mask") ${p.from})
    (pad "2" thru_hole circle (at ${p.solder && p.hotswap ? '' : '-'}3.81 ${p.solder && p.hotswap ? '' : '-'}2.54) (size 2.286 2.286) (drill 1.4986) (layers "F&B.Cu" "*.Mask") ${p.to})
    `

    // Socket side is opposite the switch; correct the downloaded housing datum.
    const switch_xy_datum = 0.00709;
    const switch_z_datum = 0.24252;
    const socket_z_datum = 0.27094;
    const model_rotation = p.side === 'B' ? [180, 0, 0] : [0, 180, 0];
    const switch_rotation = p.switch_3dmodel_xyz_rotation || model_rotation;
    const switch_offset = p.switch_3dmodel_xyz_offset || [p.side === 'B' ? switch_xy_datum : -switch_xy_datum, p.side === 'B' ? -switch_xy_datum : switch_xy_datum, -p.pcb_thickness + switch_z_datum];
    const hotswap_rotation = p.hotswap_3dmodel_xyz_rotation || model_rotation;
    const hotswap_offset = p.hotswap_3dmodel_xyz_offset || [0, 0, -p.pcb_thickness - socket_z_datum];
    const switch_3dmodel = `
    (model ${JSON.stringify(p.switch_3dmodel_filename)}
      (offset (xyz ${switch_offset[0]} ${switch_offset[1]} ${switch_offset[2]}))
      (scale (xyz ${p.switch_3dmodel_xyz_scale[0]} ${p.switch_3dmodel_xyz_scale[1]} ${p.switch_3dmodel_xyz_scale[2]}))
      (rotate (xyz ${switch_rotation[0]} ${switch_rotation[1]} ${switch_rotation[2]}))
    )
    `

    const hotswap_3dmodel = `
    (model ${JSON.stringify(p.hotswap_3dmodel_filename)}
      (offset (xyz ${hotswap_offset[0]} ${hotswap_offset[1]} ${hotswap_offset[2]}))
      (scale (xyz ${p.hotswap_3dmodel_xyz_scale[0]} ${p.hotswap_3dmodel_xyz_scale[1]} ${p.hotswap_3dmodel_xyz_scale[2]}))
      (rotate (xyz ${hotswap_rotation[0]} ${hotswap_rotation[1]} ${hotswap_rotation[2]}))
    )
	  `

    const keycap_3dmodel = `
    (model ${JSON.stringify(p.keycap_3dmodel_filename)}
      (offset (xyz ${p.keycap_3dmodel_xyz_offset[0]} ${p.keycap_3dmodel_xyz_offset[1]} ${p.keycap_3dmodel_xyz_offset[2]}))
      (scale (xyz ${p.keycap_3dmodel_xyz_scale[0]} ${p.keycap_3dmodel_xyz_scale[1]} ${p.keycap_3dmodel_xyz_scale[2]}))
      (rotate (xyz ${p.keycap_3dmodel_xyz_rotation[0]} ${p.keycap_3dmodel_xyz_rotation[1]} ${p.keycap_3dmodel_xyz_rotation[2]}))
    )
	  `
    const common_bottom = `
  )
    `

    const hotswap_routes_unplated = `
	(segment
		(start ${p.eaxy(-5.842, -5.08)})
		(end ${p.eaxy(-3.963, -6.959)})
		(width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(layer "F.Cu")
		(net ${p.to.index})
	)
	(segment
		(start ${p.eaxy(-3.963, -6.959)})
		(end ${p.eaxy(0, -6.959)})
		(width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(layer "F.Cu")
		(net ${p.to.index})
	)
	(via
		(at ${p.eaxy(0, -6.959)})
		(size ${p.via_size})
    (drill ${p.via_drill})
		(layers "F.Cu" "B.Cu")
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(net ${p.to.index})
	)
	(segment
		(start ${p.eaxy(0, -6.959)})
		(end ${p.eaxy(3.963, -6.9595)})
		(width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(layer "B.Cu")
		(net ${p.to.index})
	)
	(segment
		(start ${p.eaxy(3.963, -6.959)})
		(end ${p.eaxy(5.842, -5.08)})
		(width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(layer "B.Cu")
		(net ${p.to.index})
  )
	(segment
    (start ${p.eaxy(0, -5.93)})
    (end ${p.eaxy(1.029, -6.959)})
    (width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
    (layer "F.Cu")
    (net ${p.from.index})
  )
  (segment
    (start ${p.eaxy(4.166, -6.959)})
    (end ${p.eaxy(7.085, -4.04)})
    (width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
    (layer "F.Cu")
    (net ${p.from.index})
  )
  (segment
    (start ${p.eaxy(7.085, -4.04)})
    (end ${p.eaxy(7.085, -2.54)})
    (width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
    (layer "F.Cu")
    (net ${p.from.index})
  )
  (segment
    (start ${p.eaxy(1.029, -6.959)})
    (end ${p.eaxy(4.166, -6.959)})
    (width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
    (layer "F.Cu")
    (net ${p.from.index})
  )
  (via
    (at ${p.eaxy(0, -5.93)})
		(size ${p.via_size})
    (drill ${p.via_drill})
    (layers "F.Cu" "B.Cu")
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
    (net ${p.from.index})
  )
  (segment
    (start ${p.eaxy(-4.166, -6.959)})
    (end ${p.eaxy(-1.029, -6.959)})
    (width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
    (layer "B.Cu")
    (net ${p.from.index})
  )
  (segment
    (start ${p.eaxy(-7.085, -2.54)})
    (end ${p.eaxy(-7.085, -4.04)})
    (width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
    (layer "B.Cu")
    (net ${p.from.index})
  )
  (segment
    (start ${p.eaxy(-7.085, -4.04)})
    (end ${p.eaxy(-4.166, -6.959)})
    (width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
    (layer "B.Cu")
    (net ${p.from.index})
  )
  (segment
    (start ${p.eaxy(-1.029, -6.959)})
    (end ${p.eaxy(0, -5.93)})
    (width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
    (layer "B.Cu")
    (net ${p.from.index})
  )
    `
    const hotswap_routes_same_side = `
	(segment
		(start ${p.eaxy(7.085, -4.415)})
		(end ${p.eaxy(7.085, -2.54)})
		(width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(layer "F.Cu")
		(net ${p.from.index})
	)
	(segment
		(start ${p.eaxy(7.75, -5.08)})
		(end ${p.eaxy(7.085, -4.415)})
		(width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(layer "F.Cu")
		(net ${p.from.index})
	)
	(via
		(at ${p.eaxy(7.75, -5.08)})
		(size ${p.via_size})
    (drill ${p.via_drill})
		(layers "F.Cu" "B.Cu")
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(net ${p.from.index})
	)
	(segment
		(start ${p.eaxy(5.842, -5.08)})
		(end ${p.eaxy(7.75, -5.08)})
		(width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
		(layer "B.Cu")
		(net ${p.from.index})
	)

	(segment
    (start ${p.eaxy(-7.085, -4.415)})
    (end ${p.eaxy(-7.085, -2.54)})
    (width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
    (layer "B.Cu")
    (net ${p.to.index})
  )
  (segment
    (start ${p.eaxy(-7.75, -5.08)})
    (end ${p.eaxy(-7.085, -4.415)})
    (width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
    (layer "B.Cu")
    (net ${p.to.index})
  )
  (via
    (at ${p.eaxy(-7.75, -5.08)})
		(size ${p.via_size})
    (drill ${p.via_drill})
    (layers "F.Cu" "B.Cu")
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
    (net ${p.to.index})
  )
  (segment
    (start ${p.eaxy(-5.842, -5.08)})
    (end ${p.eaxy(-7.75, -5.08)})
    (width ${p.trace_width})
    (locked ${p.locked_traces_vias ? 'yes' : 'no'})
    (layer "F.Cu")
    (net ${p.to.index})
  )
    `

    let final = common_top;
    if (p.include_corner_marks) {
      final += corner_marks;
    }
    if (p.include_keycap) {
      final += keycap_marks;
    }
    if (p.hotswap) {
      if (p.reversible || p.side == "F") {
        final += hotswap_front;
        if (p.include_silkscreen && !p.reversible) {
          final += hotswap_silkscreen_front;
        }
      }
      if (p.reversible || p.side == "B") {
        final += hotswap_back;
        if (p.include_silkscreen && !p.reversible) {
          final += hotswap_silkscreen_back;
        }
      }
      if (p.hotswap_3dmodel_filename) {
        final += hotswap_3dmodel;
      }
      if (p.include_silkscreen && p.reversible) {
        final += hotswap_silkscreen_reversible;
      }
    }
    if (p.solder) {
      if (p.reversible || p.side == "F") {
        final += solder_front;
      }
      if (p.reversible || p.side == "B") {
        final += solder_back;
      }
    }

    if (p.switch_3dmodel_filename) {
      final += switch_3dmodel
    }

    if (p.keycap_3dmodel_filename) {
      final += keycap_3dmodel
    }

    final += common_bottom

    if (p.reversible && p.hotswap && p.include_traces_vias && !p.include_plated_holes) {
      if(p.hotswap_pads_same_side){
        final += hotswap_routes_same_side
      } else {
        final += hotswap_routes_unplated
      }
    }

    return final
  }
}

;module.exports = ((original) => {
    const defaults = {"switch_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/kiswitch/SW_Cherry_MX_PCB.stp","hotswap_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/kiswitch/SW_Hotswap_Kailh_MX.stp","switch_3dmodel_xyz_offset":"","switch_3dmodel_xyz_rotation":"","hotswap_3dmodel_xyz_offset":"","hotswap_3dmodel_xyz_rotation":""};
    const bossVariant = null;
    const chocV2 = "${KIPRJMOD}/models/boardstudio/koktoh/Choc_V2_Red.step";
    return {
      ...original,
      params: {...original.params, ...defaults},
      body: p => {
        // The bundled extension crosses the PCB with a 5 mm outer diameter.
        const extensionDiameter = 5;
        if (false && p.drill < extensionDiameter
          && p.tp_extension_3dmodel_filename === defaults.tp_extension_3dmodel_filename
          && !p.tp_extension_3dmodel_xyz_scale
          && !p.tp_extension_3dmodel_xyz_rotation
          && !p.tp_extension_3dmodel_xyz_offset) {
          throw new Error('The bundled trackpoint extension requires a center drill of at least 5 mm. Select a compatible extension model or drill size.');
        }
        if (false && !p.choc_v1_support) {
          p = {...p};
          if (!p.choc_v2_support) {
            for (const key of Object.keys(defaults)) {
              if (p[key] === defaults[key]) { p[key] = ''; }
            }
          } else {
            if (p.switch_3dmodel_filename === defaults.switch_3dmodel_filename) {
              const automatic = !p.switch_3dmodel_xyz_rotation && !p.switch_3dmodel_xyz_offset
                && p.switch_3dmodel_xyz_scale.every(value => value === 1);
              const postDiameter = 4.8;
              if (automatic && (!p.include_stabilizer_pad || p.oval_stabilizer_pad
                || (p.center_hole_diameter > 0 && p.center_hole_diameter < postDiameter))) {
                throw new Error('The bundled Choc V2 model requires the round stabilizer hole and a center drill of at least 4.8 mm. Use a matching model or transform for modified hardware.');
              }
              p.switch_3dmodel_filename = chocV2;
              p.switch_3dmodel_xyz_rotation ||= p.side === 'F' ? [180, 0, 0] : [0, 180, 0];
              p.switch_3dmodel_xyz_offset ||= [0, 0, -p.pcb_thickness];
            }
            if (p.keycap_3dmodel_filename === defaults.keycap_3dmodel_filename) {
              p.keycap_3dmodel_filename = '';
            }
          }
        }
        if (bossVariant && p.include_bosses && p.reset_switch_3dmodel_filename === defaults.reset_switch_3dmodel_filename) {
          p = {...p, reset_switch_3dmodel_filename: bossVariant};
        }
        return original.body(p);
      }
    };
  })(module.exports);

return module.exports; })(),
  "ceoloide/trrs_pj320a": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Copyright (c) 2023 Marco Massarelli
//
// SPDX-License-Identifier: MIT
//
// To view a copy of this license, visit https://opensource.org/license/mit/
//
// Authors: @ergogen + @ceoloide improvements
//
// Description:
//  A reversible "PJ-320A" TRRS footprint similar to the one used on
//  the Corne keyboard, and available at LCSC. The footprint offers many
//  customization options.
//
//  Normal / single side
//     ____________________
//    |                   |_
//    | (R1)              | |
//    |                   |_|
//    |___(TP)___(R2)_(SL)|
// 
//  Reversible
//     ____________________
//    |   (TP)   (R2) (SL)|_
//    | (R1)              | |
//    | (R1)              |_|
//    |___(TP)___(R2)_(SL)|
//
// Reversible & symmetrical
//     ___________________
//    | ( TP) (R2)   (SL)|_
//    |                  |_|
//    |_( TP)_(R2)___(SL)|
//
// Datasheet:
//  https://datasheet.lcsc.com/lcsc/2311241628_Hong-Cheng-HC-PJ-320A_C7501806.pdf
//
// Nets:
//    SL: corresponds to pin 2 (Sleeve)
//    R2: corresponds to pin 3 (Ring 2)
//    R1: corresponds to pin 1 (Ring 1)
//    TP: corresponds to pin 4 (Tip)
//
// Warning:
//    TRRS cables should never be hotswapped (removed or inserted when the MCU is turned on).
//    To minimize the chance of damaging the MCU, connect VCC to the tip (TP) and GND on the
//    sleeve (SL).
//
// Params:
//    side: default is F for Front
//      the side on which to place the single-side footprint and designator, either F or B
//    reversible: default is false
//      if true, the footprint will be placed on both sides so that the PCB can be
//      reversible
//    symmetric: default is false
//      if true, it will only work if reversible is also true. This will cause the
//      footprint to be symmetrical on each half, however reducing the footprint
//      to three pins: TP, R2, and SL
//
// @ceoloide's improvements:
//  - Add oval pad when symmetrical
//  - Adjust positioning to be symmetrical
//  - Revamp pinout and nets
//  - Upgrade to KiCad 8

module.exports = {
  params: {
    designator: 'TRRS',
    side: 'F',
    reversible: false,
    symmetric: false,
    trrs_3dmodel_filename: '',
    trrs_3dmodel_xyz_scale: [1, 1, 1],
    trrs_3dmodel_xyz_rotation: '',
    trrs_3dmodel_xyz_offset: '',
    TP: { type: 'net', value: 'TP' },
    R1: { type: 'net', value: 'R1' },
    R2: { type: 'net', value: 'R2' },
    SL: { type: 'net', value: 'SL' },
  },
  body: p => {

    let footprint_name = "trrs_pj320a"
    if (p.reversible) {
      if (p.symmetric) {
        footprint_name += " (reversible, symmetric)"
      } else {
        footprint_name += " (reversible)"
      }
    }

    const standard_opening = `
  (footprint "ceoloide:${footprint_name}"
    (layer "${p.side}.Cu")
    ${p.at}
    (property "Reference" "${p.ref}"
      (at 0 14.2 ${p.r})
      (layer "${p.side}.SilkS")
      ${p.ref_hide}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    `
    function corner_marks(offset_x) {
      return `
    (fp_line (start ${2.8 + offset_x} -2) (end ${-2.8 + offset_x} -2) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start ${-2.8 + offset_x} 0) (end ${-2.8 + offset_x} -2) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start ${2.8 + offset_x} 0) (end ${2.8 + offset_x} -2) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start ${-3.05 + offset_x} 0) (end ${-3.05 + offset_x} 12.1) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start ${3.05 + offset_x} 0) (end ${3.05 + offset_x} 12.1) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start ${3.05 + offset_x} 12.1) (end ${-3.05 + offset_x} 12.1) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
    (fp_line (start ${3.05 + offset_x} 0) (end ${-3.05 + offset_x} 0) (layer "Dwgs.User") (stroke (width 0.15) (type solid)))
      `
    }
    function stabilizers(def_pos) {
      return `
    (pad "" np_thru_hole circle (at ${def_pos} 8.6 ${p.r}) (size 1.5 1.5) (drill 1.5) (layers "*.Cu" "*.Mask"))
    (pad "" np_thru_hole circle (at ${def_pos} 1.6 ${p.r}) (size 1.5 1.5) (drill 1.5) (layers "*.Cu" "*.Mask"))
      `
    }
    function pins(def_neg, def_pos) {
      if (p.symmetric && p.reversible) {
        return `
    (pad "2" thru_hole oval (at ${def_pos} 3.2 ${p.r}) (size 1.6 2.2) (drill oval 0.9 1.5) (layers "*.Cu" "*.Mask") ${p.SL.str})
    (pad "3" thru_hole oval (at ${def_pos} 6.2 ${p.r}) (size 1.6 2.2) (drill oval 0.9 1.5) (layers "*.Cu" "*.Mask") ${p.R2.str})
    (pad "4" thru_hole oval (at ${def_pos} 10.75 ${p.r}) (size 1.6 3.3) (drill oval 0.9 2.6) (layers "*.Cu" "*.Mask") ${p.TP.str})
        `
      } else {
        return `
    (pad "2" thru_hole oval (at ${def_pos} 3.2 ${p.r}) (size 1.6 2.2) (drill oval 0.9 1.5) (layers "*.Cu" "*.Mask") ${p.SL.str})
    (pad "3" thru_hole oval (at ${def_pos} 6.2 ${p.r}) (size 1.6 2.2) (drill oval 0.9 1.5) (layers "*.Cu" "*.Mask") ${p.R2.str})
    (pad "4" thru_hole oval (at ${def_pos} 10.2 ${p.r}) (size 1.6 2.2) (drill oval 0.9 1.5) (layers "*.Cu" "*.Mask") ${p.TP.str})
    (pad "5" thru_hole oval (at ${def_neg} 11.3 ${p.r}) (size 1.6 2.2) (drill oval 0.9 1.5) (layers "*.Cu" "*.Mask") ${p.R1.str})
        `
      }
    }
    // The staggered reversible layout shifts the connector center by half a pin pitch.
    const center = p.reversible && !p.symmetric ? (p.side === 'F' ? 1.15 : -1.15) : 0;
    const rotation = p.trrs_3dmodel_xyz_rotation || [-90, 0, p.side === 'F' ? 180 : 0];
    const offset = p.trrs_3dmodel_xyz_offset || [center, 0, 0];
    const model = p.trrs_3dmodel_filename ? `
    (model ${JSON.stringify(p.trrs_3dmodel_filename)}
      (offset (xyz ${offset.join(' ')}))
      (scale (xyz ${p.trrs_3dmodel_xyz_scale.join(' ')}))
      (rotate (xyz ${rotation.join(' ')})))` : '';
    if (p.reversible & p.symmetric) {
      return `
    ${standard_opening}
    ${corner_marks(0)}
    ${stabilizers(0)}
    ${pins(2.3, -2.3)}
    ${pins(-2.3, 2.3)}
    ${model}
  )
      `
    } else if (p.reversible) {
      return `
    ${standard_opening}
    ${corner_marks(1.15)}
    ${stabilizers(-1.15)}
    ${stabilizers(1.15)}
    ${pins(-1.15, 3.45)}
    ${pins(1.15, -3.45)}
    ${model}
  )
      `
    } else {
      return `
    ${standard_opening}
    ${corner_marks(0)}
    ${stabilizers(0)}
    ${p.side == 'F' ? pins(-2.3, 2.3) : pins(2.3, -2.3) }
    ${model}
  )
    `
    }
  }
}

;module.exports = ((original) => {
    const defaults = {"trrs_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/keebio/PJ-320A.step"};
    const bossVariant = null;
    const chocV2 = "${KIPRJMOD}/models/boardstudio/koktoh/Choc_V2_Red.step";
    return {
      ...original,
      params: {...original.params, ...defaults},
      body: p => {
        // The bundled extension crosses the PCB with a 5 mm outer diameter.
        const extensionDiameter = 5;
        if (false && p.drill < extensionDiameter
          && p.tp_extension_3dmodel_filename === defaults.tp_extension_3dmodel_filename
          && !p.tp_extension_3dmodel_xyz_scale
          && !p.tp_extension_3dmodel_xyz_rotation
          && !p.tp_extension_3dmodel_xyz_offset) {
          throw new Error('The bundled trackpoint extension requires a center drill of at least 5 mm. Select a compatible extension model or drill size.');
        }
        if (false && !p.choc_v1_support) {
          p = {...p};
          if (!p.choc_v2_support) {
            for (const key of Object.keys(defaults)) {
              if (p[key] === defaults[key]) { p[key] = ''; }
            }
          } else {
            if (p.switch_3dmodel_filename === defaults.switch_3dmodel_filename) {
              const automatic = !p.switch_3dmodel_xyz_rotation && !p.switch_3dmodel_xyz_offset
                && p.switch_3dmodel_xyz_scale.every(value => value === 1);
              const postDiameter = 4.8;
              if (automatic && (!p.include_stabilizer_pad || p.oval_stabilizer_pad
                || (p.center_hole_diameter > 0 && p.center_hole_diameter < postDiameter))) {
                throw new Error('The bundled Choc V2 model requires the round stabilizer hole and a center drill of at least 4.8 mm. Use a matching model or transform for modified hardware.');
              }
              p.switch_3dmodel_filename = chocV2;
              p.switch_3dmodel_xyz_rotation ||= p.side === 'F' ? [180, 0, 0] : [0, 180, 0];
              p.switch_3dmodel_xyz_offset ||= [0, 0, -p.pcb_thickness];
            }
            if (p.keycap_3dmodel_filename === defaults.keycap_3dmodel_filename) {
              p.keycap_3dmodel_filename = '';
            }
          }
        }
        if (bossVariant && p.include_bosses && p.reset_switch_3dmodel_filename === defaults.reset_switch_3dmodel_filename) {
          p = {...p, reset_switch_3dmodel_filename: bossVariant};
        }
        return original.body(p);
      }
    };
  })(module.exports);

return module.exports; })(),
  "ceoloide/utility_ergogen_logo": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Copyright (c) 2023 Marco Massarelli
//
// SPDX-License-Identifier: MIT
//
// To view a copy of this license, visit https://opensource.org/license/mit/
//
// Author: @dieseltravis + @ceoloide improvements
//
// Description:
//  A 5mm x 5mmm Ergogen logo that can be scaled and assigned to any layer of your board.
//  Make sure to add it to your board and spread the love <3
//
//  Note that some fine details may be lost depending on scale and fab capabilities.
//
// Params:
//    side: default is F for Front
//      the side on which to place the logo. When the backside is selected, the logo will
//      be mirrored automatically
//    layer: default is 'SilkS' (Silkscreen layer)
//      the layer where the logo will be placed, useful to have copper + soldermask texts
//    reversible: default is false
//      adds the logo on both sides, taking care of mirroring the backside
//    scale: default is 1.0 (100%)
//      the scale ratio to apply to the logo, to make it bigger or smaller
//
// @ceoloide's improvements:
//  - Mirror the logo when added to the back layer
//  - Add reversible option to add the logo on both layers
//  - Ensure numbers have at most 6 decimals (KiCad max precision)
//  - Upgrade to KiCad 8

module.exports = {
  params: {
    designator: 'LOGO',
    side: 'F',
    layer: 'SilkS',
    reversible: false,
    scale: 1.0,
  },
  body: p => {
    const scaled_point = (x, y, scale, mirrored) => {
      let scaled_x = x * scale * (mirrored ? -1.0 : 1.0)
      let scaled_y = y * scale
      return `(xy ${scaled_x.toFixed(6)} ${scaled_y.toFixed(6)})`
    }
    const fp_poly = (side, layer, scale, mirrored) => {
      const s = scale
      const m = mirrored
      return `
    (fp_poly 
      (pts
          ${scaled_point(2.501231, 0, s, m)} ${scaled_point(2.501231, 2.501231, s, m)} ${scaled_point(0, 2.501231, s, m)} ${scaled_point(-2.50123, 2.501231, s, m)} ${scaled_point(-2.50123, 1.013088, s, m)}
          ${scaled_point(-1.738355, 1.013088, s, m)} ${scaled_point(-0.021885, 1.009917, s, m)} ${scaled_point(1.694584, 1.006746, s, m)} ${scaled_point(1.697905, 0.662827, s, m)} ${scaled_point(1.701225, 0.318907, s, m)}
          ${scaled_point(1.52891, 0.490867, s, m)} ${scaled_point(1.356594, 0.662827, s, m)} ${scaled_point(-0.19088, 0.662827, s, m)} ${scaled_point(-1.738355, 0.662827, s, m)} ${scaled_point(-1.738355, 0.837957, s, m)}
          ${scaled_point(-1.738355, 1.013088, s, m)} ${scaled_point(-2.50123, 1.013088, s, m)} ${scaled_point(-2.50123, 0.150074, s, m)} ${scaled_point(-1.394101, 0.150074, s, m)} ${scaled_point(-0.637478, 0.150074, s, m)}
          ${scaled_point(0.119144, 0.150074, s, m)} ${scaled_point(0.293895, -0.025012, s, m)} ${scaled_point(0.468646, -0.200098, s, m)} ${scaled_point(-0.287976, -0.200098, s, m)} ${scaled_point(-1.044599, -0.200098, s, m)}
          ${scaled_point(-1.21935, -0.025012, s, m)} ${scaled_point(-1.394101, 0.150074, s, m)} ${scaled_point(-2.50123, 0.150074, s, m)} ${scaled_point(-2.50123, 0, s, m)} ${scaled_point(-2.50123, -1.063023, s, m)}
          ${scaled_point(-1.738355, -1.063023, s, m)} ${scaled_point(-1.738355, -0.887937, s, m)} ${scaled_point(-1.738355, -0.71285, s, m)} ${scaled_point(-0.190545, -0.71285, s, m)} ${scaled_point(1.357266, -0.71285, s, m)}
          ${scaled_point(1.525751, -0.544017, s, m)} ${scaled_point(1.578342, -0.491483, s, m)} ${scaled_point(1.624575, -0.445614, s, m)} ${scaled_point(1.661679, -0.409133, s, m)} ${scaled_point(1.686885, -0.384763, s, m)}
          ${scaled_point(1.697422, -0.375226, s, m)} ${scaled_point(1.697537, -0.375184, s, m)} ${scaled_point(1.698399, -0.387158, s, m)} ${scaled_point(1.699177, -0.420952, s, m)} ${scaled_point(1.69984, -0.473372, s, m)}
          ${scaled_point(1.700359, -0.541225, s, m)} ${scaled_point(1.700701, -0.62132, s, m)} ${scaled_point(1.700836, -0.710463, s, m)} ${scaled_point(1.700837, -0.719103, s, m)} ${scaled_point(1.700837, -1.063023, s, m)}
          ${scaled_point(-0.018759, -1.063023, s, m)} ${scaled_point(-1.738355, -1.063023, s, m)} ${scaled_point(-2.50123, -1.063023, s, m)} ${scaled_point(-2.50123, -2.50123, s, m)} ${scaled_point(0, -2.50123, s, m)}
          ${scaled_point(2.501231, -2.50123, s, m)}
      )
			(stroke
				(width 0.01)
				(type solid)
			)
			(fill solid)
      (layer "${side}.${layer}")
    )
      `
    }
    const common_top = `
  (footprint "ceoloide:utility_ergogen_logo"
    (layer "${p.side}.Cu")
    ${p.at}
    (property "Reference" "${p.ref}"
      (at ${p.scale * 4.572} 0 ${p.r})
      (layer "${p.side}.Fab")
      ${p.ref_hide}
      (effects (font (size 1 1) (thickness 0.15)))
    )
		(attr exclude_from_pos_files exclude_from_bom)
    `
    const common_bottom = `
  )
    `
    let ergogen_log_fp = common_top
    if (p.reversible) {
      ergogen_log_fp += fp_poly('F', p.layer, p.scale, false)
      ergogen_log_fp += fp_poly('B', p.layer, p.scale, true)
    } else {
      ergogen_log_fp += fp_poly(p.side, p.layer, p.scale, p.side == 'B')
    }
    ergogen_log_fp += common_bottom
    return ergogen_log_fp
  }
};

return module.exports; })(),
  "ceoloide/utility_filled_zone": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Copyright (c) 2023 Marco Massarelli
//
// SPDX-License-Identifier: MIT
//
// To view a copy of this license, visit https://opensource.org/license/mit/
//
// Author: @ceoloide
//
// Description:
//  A KiCad native filled zone, for example a copper fill or hatch pattern.
//
// NOTE: Filled zones need to be calculated at least once or they'll result
//       empty.
//
// Params:
//    side: default is 'F&B' for both Front and Back
//      the side on which to place the filled zone, either 'F' for Front, 'B'
//      for Back, or 'F&B' for both.
//    name: default is '' (no name)
//      an optional name to give to the zone.
//    priority: default is 0
//      an optional priority to give to the zone
//    locked: default is false
//      if true it will lock the resulting fill.
//    corner_smoothing: default is chamfer
//      allows to specify the type of corner smoothing ('none', 'chamfer',
//      'fillet').
//    smoothing_radius: default is 0.5
//      the radius of the corner smoothing, valid when 'chamfer' or
//      'fillet' are selected.
//    net: default is GND
//      the net connected to this filled zone, for example GND
//    pad_clearance: default is 0.508
//      the electrical clearance to be applied (in mm). KiCad default
//      is 0.508.
//    min_thickness: default is 0.25
//      the minimum thickness of the zone areas (in mm). KiCad default
//      is 0.25 to match default net properties. It shouldn't be lowered
//      below 0.127 (the min width JLCPCB handles).
//    connect_pads: default is ''
//      whether pads should be connected, one of '' (thermal reliefs), 
//      'yes' (solid connection), 'thru_hole_only', or 'no'.
//    thermal_gap: default is 0.5
//      the thermal relief gap (in mm), with KiCad default being 0.5
//    thermal_bridge_width: default is 0.5
//      the thermal relief spoke width (in mm), with KiCad default being 0.5
//    remove_islands: default is 'never'
//      whether to remove islands, one of 'never', 'always', or 'below_area_limit'
//    min_island_size: default is 5
//      the min island size in mm^2 to be kept if island should be removed below
//      a given area limit.
//    fill_type: default is 'solid'
//      the type of fill, either 'solid' or 'hatch'
//    hatch_thickness: default is 1
//      the thickness of the hatch pattern (in mm)
//    hatch_gap: default is 1.5
//      the hatch gap size (in mm)
//    hatch_orientation: default is 0
//      the orientation of the hatch pattern (in degrees)
//    hatch_smoothing_level: default is 0
//      the level of smoothing to apply to the hatch pattern algorithm,
//      between 0 and 3
//    hatch_smoothing_value: default is 0.1
//      the smoothing value used by the hatch smoothing algorithm,
//      between 0.0 and 1.0
//    points: default is [[0,0],[420,0],[420,297],[0,297]]
//      an array containing the polygon points of the filled area, in
//      xy coordinates relative to the PCB. The default is a square area of
//      420x297mm^2 located at (0,0) xy coordinates, essentially filling the
//      entire "PCB sheet" area.

module.exports = {
  params: {
    side: 'F',
    net: { type: 'net', value: 'GND' },
    name: '',
    priority: 0,
    locked: false,
    corner_smoothing: 'chamfer',
    smoothing_radius: 0.5,
    connect_pads: '',
    pad_clearance: 0.508,
    min_thickness: 0.25,
    thermal_gap: 0.5,
    thermal_bridge_width: 0.5,
    remove_islands: 'never',
    min_island_size: 5,
    fill_type: 'solid',
    hatch_thickness: 1,
    hatch_gap: 1.5,
    hatch_orientation: 0,
    hatch_smoothing_level: 0,
    hatch_smoothing_value: 0.1,
    points: [[0, 0], [420, 0], [420, 297], [0, 297]],
  },
  body: p => {
    let polygon_pts = ''
    for (let i = 0; i < p.points.length; i++) {
      polygon_pts += `(xy ${p.points[i][0]} ${p.points[i][1]}) `
    }
    return `
  (zone
    (net ${p.net.index})
    (net_name "${p.net.name}")
    (locked ${p.locked ? 'yes' : 'no'})
    (layers "${p.side}.Cu")
    ${p.name ? '(name "' + p.name + '")' : ''}
    (hatch edge 0.5)
    ${p.prority > 0 ? '(priority ' + p.priority + ')' : ''}
    (connect_pads ${p.connect_pads}
      (clearance ${p.pad_clearance})
    )
    (min_thickness ${p.min_thickness})
    (filled_areas_thickness no)
    (fill 
      ${p.fill_type == 'solid' ? 'yes' : '(mode ' + p.fill_type + ')'}
      (thermal_gap ${p.thermal_gap})
      (thermal_bridge_width ${p.thermal_bridge_width})
      ${p.corner_smoothing != '' ? '(smoothing ' + p.corner_smoothing + ')' : ''}
      ${p.corner_smoothing != '' ? '(radius ' + p.smoothing_radius + ')' : ''}
      ${p.remove_islands == 'always' ? '' : '(island_removal_mode ' + (p.remove_islands == 'never' ? 1 : 2) + ')'}
      ${p.remove_islands == 'always' ? '' : '(island_area_min ' + p.min_island_size + ')'}
      ${p.fill_type == 'solid' ? '' : '(hatch_thickness ' + p.hatch_thickness + ')'}
      ${p.fill_type == 'solid' ? '' : '(hatch_gap ' + p.hatch_gap + ')'}
      ${p.fill_type == 'solid' ? '' : '(hatch_orientation ' + p.hatch_orientation + ')'}
      ${p.fill_type == 'solid' || p.hatch_smoothing_level < 1 ? '' : '(hatch_smoothing_level ' + p.hatch_smoothing_level + ')'}
      ${p.fill_type == 'solid' || p.hatch_smoothing_level < 1 ? '' : '(hatch_smoothing_value ' + p.hatch_smoothing_value + ')'}
      ${p.fill_type == 'solid' ? '' : '(hatch_border_algorithm hatch_thickness)'}
      ${p.fill_type == 'solid' ? '' : '(hatch_min_hole_area 0.3)'}
    )
    (polygon
      (pts
        ${polygon_pts}
      )
    )
  )
    `
  }
}

return module.exports; })(),
  "ceoloide/utility_keepout_zone": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Copyright (c) 2023 Marco Massarelli
//
// SPDX-License-Identifier: MIT
//
// To view a copy of this license, visit https://opensource.org/license/mit/
//
// Author: @ceoloide
//
// Description:
//  A KiCad native keepout zone.
//
// Params:
//    side: default is 'F&B' for both Front and Back
//      the side on which to place the keepout zone, either 'F' for Front, 'B'
//      for Back, or 'F&B' for both.
//    name: default is '' (no name)
//      an optional name to give to the zone.
//    locked: default is false
//      if true it will lock the resulting keepout zone.
//    tracks_allowed: default is true
//      if true, tracks are allowed in the keepout zone.
//    vias_allowed: default is true
//      if true, vias are allowed in the keepout zone.
//    pads_allowed: default is true
//      if true, pads are allowed in the keepout zone.
//    copperpour_allowed: default is true
//      if true, copper pours are allowed in the keepout zone.
//    footprints_allowed: default is true
//      if true, footprints are allowed in the keepout zone.
//    outline_type: default is 'edge'
//      the type of outline to use for the keepout zone, either 'none' to just
//      display a line, 'edge' to display a hatch pattern at the edge, or `full`
//      to display a full hatch pattern.
//    hatch_pitch: default is 1
//      the pitch (distance between display lines) of the hatch pattern (in mm).
//      Must be below 2mm.
//    points: default is [[0,0],[420,0],[420,297],[0,297]]
//      an array containing the polygon points of the filled area, in
//      xy coordinates relative to the PCB. The default is a square area of
//      420x297mm^2 located at (0,0) xy coordinates, essentially filling the
//      entire "PCB sheet" area.

module.exports = {
  params: {
    side: 'F&B',
    name: '',
    locked: false,
    tracks_allowed: true,
    vias_allowed: true,
    pads_allowed: true,
    copperpour_allowed: true,
    footprints_allowed: true,
    outline_type: 'edge',
    hatch_pitch: 1,
    points: [[0, 0], [420, 0], [420, 297], [0, 297]],
  },
  body: p => {
    if (p.hatch_pitch < 0 || p.hatch_pitch > 2) {
      throw new Error(
        'Parameter hatch_pitch must be a positive number below 2mm to avoid KiCad issues. Current value: ' + p.hatch_pitch + 'mm'
      );
    }
    let polygon_pts = ''
    for (let i = 0; i < p.points.length; i++) {
      polygon_pts += `(xy ${p.points[i][0]} ${p.points[i][1]}) `
    }
    return `
  (zone
    (net 0)
    (net_name "")
    (locked ${p.locked ? 'yes' : 'no'})
    (layers "${p.side}.Cu")
    ${p.name ? '(name "' + p.name + '")' : ''}
		(hatch ${p.outline_type} ${p.hatch_pitch})
    (connect_pads
      (clearance 0)
    )
    (min_thickness 0.25)
    (filled_areas_thickness no)
		(keepout
			(tracks ${p.tracks_allowed ? 'allowed' : 'not_allowed'})
			(vias ${p.vias_allowed ? 'allowed' : 'not_allowed'})
			(pads ${p.pads_allowed ? 'allowed' : 'not_allowed'})
			(copperpour ${p.copperpour_allowed ? 'allowed' : 'not_allowed'})
			(footprints ${p.footprints_allowed ? 'allowed' : 'not_allowed'})
		)
		(fill
			(thermal_gap 0.5)
			(thermal_bridge_width 0.5)
		)
    (polygon
      (pts
        ${polygon_pts}
      )
    )
  )
    `
  }
}

return module.exports; })(),
  "ceoloide/utility_point_debugger": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
/*
Copyright (c) 2023 Marco Massarelli

SPDX-License-Identifier: CC-BY-NC-SA-4.0

To view a copy of this license, visit http://creativecommons.org/licenses/by-nc-sa/4.0/

Author: @infused-kim + @ceoloide improvements

Description:
  Adds tiny x and y arrows at the origin of points to help with the debugging
  of complex layouts.

  This will only show arrows for actual points you define. So if you draw an
  outline without defining points, then they won't show up.

Usage:
  You can make enabling and disabling easy with ergogen's preprocessor:

  ```js
  settings:
    point_debugger:
      enabled: false
  [...]
  pcbs:
    your_keyboard:
      footprints:
        point_debugger:
          what: infused-kim/point_debugger
          where: true
          params:
            $extends: settings.point_debugger
  ```

Showing the point name:
  The footprint supports showing the name of the point when you zoom in, but
  the latest version of ergogen (4.1.0) does not make the name available to
  the footprint.

  Until this gets merged, you can use the ergogen fork from this PR:
  https://github.com/ergogen/ergogen/pull/103
  
Params:
  enabled: default is true
    Whether to include or not the point debugger footprint

@ceoloide' improvements
  - Upgrade to KiCad 8 
*/
module.exports = {
  params: {
    designator: 'P',
    enabled: true,
  },
  body: p => {
    if (p.enabled == false) {
      return '';
    }

    const top = `
  (footprint "ceoloide:utility_point_debugger"
    (layer "F.Cu")
    ${p.at}
    (property "Reference" "${p.ref}"
      (at 0 2)
      (layer "Dwgs.User")
      ${p.ref_hide}
      (effects (font (size 1 1) (thickness 0.15)))
    )
    (fp_line (start -0.6 0) (end 0.6 0) (layer "Dwgs.User") (stroke (width 0.05) (type solid)))
    (fp_line (start 0 -0.6) (end 0 0.6) (layer "Dwgs.User") (stroke (width 0.05) (type solid)))
    (fp_line (start 0.6 0) (end 0.5 -0.1) (layer "Dwgs.User") (stroke (width 0.05) (type solid)))
    (fp_line (start 0.6 0) (end 0.5 0.1) (layer "Dwgs.User") (stroke (width 0.05) (type solid)))
    (fp_line (start 0 -0.6) (end 0.1 -0.5) (layer "Dwgs.User") (stroke (width 0.05) (type solid)))
    (fp_line (start 0 -0.6) (end -0.1 -0.5) (layer "Dwgs.User") (stroke (width 0.05) (type solid)))
      `

    const bottom = `
  )
      `

    let final = top;

    if ('point' in p) { // Only available from commit #852c100
      final += ` 
    (fp_text user "${p.point.meta.name}"
      (at -0.3 -0.05 ${p.r})
      (layer "Dwgs.User")
      (effects (font (size 0.0254 0.0254) (thickness 0.001)))
    )
      `;
    }

    final += bottom;

    return final;
  }
}
return module.exports; })(),
  "ceoloide/utility_router": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Copyright (c) 2025 Marco Massarelli
//
// SPDX-License-Identifier: MIT
//
// To view a copy of this license, visit https://opensource.org/license/mit/
//
// Author: @yanshay + @ceoloide improvements
//
// Description:
//   This footprint provides basic routing capabilities including laying routes (traces) and vias.
//
//   It is useful especially since there are tens of keys per keyboard which have exact same routing
//   and whenever a change needs to be made to Ergogen config, rerouting is required. This footprint
//   allows compact routing declaration in Ergogen directly.
//
//   For a more complete description, consult the original documentation at: https://github.com/yanshay/ergogen-stuff/blob/main/docs/router.md
//
//   A KiCad Plugin exists in order to facilitate the configuration of this footprint: 
//  
// Example:
//  row_route:
//    what: ceoloide/utility_router
//    where: true
//    params:
//    net: "{{row_net}}"
//    route: "f(-8.275,5.1)(-8.275,7.26)"
//
//  row_route2:
//    what: ceoloide/utility_router
//    where: true
//    params:
//    net: "{{row_net}}"
//    route: "f(-8.275 5.1) (-8.275 7.26)"
//
// Params:
//    net: default is no net
//      allows specifying a net for all routes in this footprint. To route multiple different nets,
//      you would need several different footprint configurations. However, if this parameter is left
//      to the default value then KiCad will fill in the missing nets when a file is opened in KiCad.
//      Support for multiple nets is included, but depends on an unmerged Ergogen PR. For details see
//      https://github.com/ergogen/ergogen/pull/109
//    width: default 0.250mm
//      the default trace width to be used.  Not recommended to go below 0.15mm (JLCPC min is 0.127mm).
//    via_size: default is 0.6
//      allows to define the size of the via. Not recommended below 0.56 (JLCPCB minimum),
//      or above 0.8 (KiCad default), to avoid overlap or DRC errors
//    via_drill: default is 0.3
//      allows to define the size of the drill. Not recommended below 0.3 (JLCPCB minimum),
//      or above 0.4 (KiCad default), to avoid overlap or DRC errors 
//    locked: default is false
//      sets the traces and vias as locked in KiCad. Locked objects may not be manipulated
//      or moved, and cannot be selected unless the Locked Items option is enabled in the
//      Selection Filter panel in KiCad. Useful for a faster workflow. If using autorouting
//      solutions like Freerouting, locking can prevent the traces and vias from being
//      replaced.
//    routes: default empty / no routes
//      an array of routes based on the syntax described below, each stands by its own except they all
//      share other params (net, ...)
//    route: default empty / no route
//      allows adding a single route using the syntax described below, but on a single row and in a more
//      concise format
//
// Route syntax:
//  A route is a string that describe how to route using one letter commands and positions. It follows to
//  some extent KiCad key presses to make it easy to remember. Valid commands are as follows:
//    
//    b - route on the back layer - there is no default layer to avoid mistakes
//    f - route on the front layer
//    v - place a via and switch layer
//    x or | - start a new route (if layer is set, stays on the same layer, just like in KiCad)
//    (x_pos,y_pos) - route to the given position (relative to the Ergogen point). If it is the first
//      occurrence in the route or if after x command then it places the cursor in the specific point.
//    <net_name> - the name of a net to use for the following segment. Currently unsupported in mainline
//      Ergogen, until https://github.com/ergogen/ergogen/pull/109 is merged.
//
// @morrijr's improvements:
//  - ',' between x and y, is now optional
//  - Spaces between coordinates are now allowed
// @ceoloide's improvements:
//  - Replace `get_at_coordinates` and `adjust_point` with native Ergogen `eaxy()`
//  - Refresh `via` and `segment` syntax to align with KiCad 8
//
// Special credit to @infused-kim for the adjust_point() function in the original code.
 
module.exports = {
  params: {
    net: { type: "net", value: "" },
    width: { type: "number", value: 0.25 },
    via_size: { type: "number", value: 0.6 },
    via_drill: { type: "number", value: 0.3 },
    locked: false,
    routes: { type: "array", value: [] },
    route: { type: "string", value: "" },
  },

  body: (p) => {
    /*
    Reference (KiCad 8):
    
    (segment
      (start 108.8 108)
      (end 109.7 108)
      (width 0.2)
      (locked yes)
      (layer "F.Cu")
      (net 0)
    )
      */
    const get_segment = (start, end, layer, net) => {
      if (!layer) {
        throw new Error(
          "Can't place segment before layer is set, use 'f' or 'b', to set starting layer"
        )
      }
      return `
  (segment
    (start ${p.eaxy(start[0], start[1])}) 
    (end ${p.eaxy(end[0], end[1])})
    (width ${p.width})
    (locked ${p.locked ? 'yes' : 'no'})
    (layer ${layer})
    (net ${net})
  )`
    }

    /*
    Reference (KiCad 8):

    (via
      (at -7.775 -5.95)
      (size 0.6)
      (drill 0.3)
      (layers "F.Cu" "B.Cu")
      (locked yes)
      (net 2)
    )
    */
    const get_via = (pos, net) => {
      if (!pos) {
        throw new Error(
          "Can't place via when position is not set, use (x,y) to set position"
        )
      }
      return `
  (via
    (at ${p.eaxy(pos[0], pos[1])})
    (size ${p.via_size})
    (drill ${p.via_drill})
    (layers "F.Cu" "B.Cu")
    (locked ${p.locked ? 'yes' : 'no'})
    (net ${net})
  )`
    }

    const parse_tuple = (t) => {
      let str_tuple = JSON.parse(
        t.replace(/ /g, ',') // replace spaces with commas
        .replace(/[,]+/g, ',') // replace multiple commas with a single comma
        .replace(/\([,]*/g, "[") // replace opening parenthesis (and any leading comma's) with a bracket
        .replace(/[,]*\)/g, "]")) // replace closing parenthesis (and any trailing comma's) with a bracket
      let num_tuple = str_tuple.map((v) => Number(v))
      if (isNaN(num_tuple[0] || isNaN(num_tuple[1]))) {
        throw new Error(`Invalid position encountered: ${str_tuple}`)
      }
      return num_tuple
    }

    const get_traces = (route, net) => {
      let traces = ""
      let layer = undefined
      let start = undefined // [x, y]

      for (let i = 0; i < route.length; i++) {
        let command = route[i].toLowerCase()
        switch (command) {
          case "f":
            layer = "F.Cu"
            break
          case "b":
            layer = "B.Cu"
            break
          case "v":
            traces = traces + get_via(start, net) + "\n"
            switch (layer) {
              case "F.Cu":
                layer = "B.Cu"
                break
              case "B.Cu":
                layer = "F.Cu"
                break
            }
            break
          case "(":
            let tuple_str = "("
            let parenthesis_idx = i
            for (i = i + 1; i < route.length; i++) {
              let ch = route[i]
              tuple_str += ch
              if (route[i] == ")") {
                break
              }
              if (i > route.length) {
                throw new Error(
                  `Unclosed position parenthesis in ${route} at character position ${parenthesis_idx}`
                )
              }
            }
            let pos = parse_tuple(tuple_str)
            if (start) {
              traces = traces + get_segment(start, pos, layer, net) + "\n"
            }
            start = pos
            break
          case "<":
            if(typeof p.global_net !== 'Function') {
              throw new Error(
                `Global nets are not yet supported (character position ${i}). See https://github.com/ergogen/ergogen/pull/109`
              )
            }
            let net_name = ""
            let lt_idx = i
            for (i = i + 1; i < route.length; i++) {
              let ch = route[i]
              if (route[i] == ">") {
                break
              }
              net_name += ch
              if (i > route.length) {
                throw new Error(
                  `Unclosed net parenthesis in ${route} at character position ${lt_idx}`
                )
              }
            }
            net = p.global_net(net_name)
          case "x":
          case "|":
            start = undefined
            break
          case " ":
            break // ignore spaces between coordinates
          default:
            throw new Error(`Unsupported character '${command}' at position ${i}.`)
        }
      }

      return traces
    }

    const get_routes_traces = (routes, net) => {
      let routes_traces = routes.reduce((acc_traces, route) => {
        return acc_traces + get_traces(route, net)
      }, "")
      return routes_traces
    }

    let combined_traces = ""
    if (p.route) {
      combined_traces += get_traces(p.route, p.net.index)
    }
    if (p.routes) {
      combined_traces += get_routes_traces(p.routes, p.net.index)
    }

    return combined_traces
  },
}

return module.exports; })(),
  "ceoloide/utility_text": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Copyright (c) 2023 Marco Massarelli
//
// SPDX-License-Identifier: CC-BY-NC-SA-4.0
//
// To view a copy of this license, visit http://creativecommons.org/licenses/by-nc-sa/4.0/
//
// Authors: @infused-kim + @ceoloide & @dieseltravis improvements
//
// Description:
//  Allows you to place text on the PCB's Silkscreen
//  layer, and optionally make it reversible on the
//  opposite side.
//
// Params:
//    side: default is F for Front
//      the side on which to place the single-side footprint and designator, either F or B
//    layer: default is 'SilkS' (Silkscreen layer)
//      the layer where the text will be placed, useful to have copper + soldermask texts
//    reversible: default is false
//      adds a mirrored text on the opposite side of the board with the same style and text
//    thickness: default is 0.15
//      set the thickness of the stroke for the text (only applicable to the default font)
//    width: default is 1
//      set the text width
//    height: default is 1
//      set the text height
//    mirrored: default is false
//      mirror the text, useful when text is added to the back. A reversible text is mirrored
//      by default on the backside.
//    knockout: default is false
//      add the knockout effect to the text
//    bold: default is false
//      adds bold effect to the text
//    italic: default is false
//      adds italics effect to the text
//    align: default is ''
//      control the alignment of the text (e.g. top left)
//    face: default is '' (KiCad Default)
//      control the font face applied to the text
//    text:
//      The text to display
//
// @ceoloide's improvements:
//  - Add ability to set text on both sides
//  - Add ability to adjust font thickness and size
//  - Add mirrored and knockout effects
//
// @diseltravis's improvements:
//  - Add option to customizer the font face
//  - Add option to set bold, italic font styles
//  - Add option to chose the layer where the text is added to

module.exports = {
  params: {
    designator: 'TXT',
    side: 'F',
    layer: 'SilkS',
    reversible: false,
    thickness: 0.15,
    height: 1,
    width: 1,
    mirrored: false,
    knockout: false,
    bold: false,
    italic: false,
    align: '',
    face: '',
    text: ''
  },
  body: p => {
    const generate_text = (side, layer, align, mirrored, thickness, height, width, text, face, bold, italic, knockout) => {
      let justify = `(justify ${align} ${mirrored ? 'mirror' : ''})`;
      const gr_text = `
  (gr_text "${text}"
    ${p.at}
    (layer "${side}.${layer}" ${knockout ? 'knockout' : ''})
    (effects
      (font ${face != '' ? '(face "' + face + '")' : ''}
        (size ${height} ${width})
        (thickness ${thickness})
        ${bold ? '(bold yes)' : ''}
        ${italic ? '(italic yes)' : ''}
      )
      ${align != '' || mirrored ? justify : ''}
    )
  )
      `;
      return gr_text;
    };

    let final = '';
    if (p.reversible) {
      final += generate_text('F', p.layer, p.align, false, p.thickness, p.height, p.width, p.text, p.face, p.bold, p.italic, p.knockout);
      final += generate_text('B', p.layer, p.align, true, p.thickness, p.height, p.width, p.text, p.face, p.bold, p.italic, p.knockout);
    } else {
      final += generate_text(p.side, p.layer, p.align, p.mirrored, p.thickness, p.height, p.width, p.text, p.face, p.bold, p.italic, p.knockout);
    }
    return final;
  }
};
return module.exports; })(),
  "infused-kim/conn_molex_pico_ezmate_1x02": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Author: @infused-kim
//
// Description:
// PCB footprint for for molex pico ezmate connector with 2 pins. Ideal for
// battery connections.
//
// This connector was chosen over the more common JST connector, because it
// has a mated profile height of only 1.65 mm. This is lower than the Kailh
// Choc hotswap sockets.
//
// It should also be compatible with the JST ACH connector (which is almost the
// same).
//
// One downside is that there are almost no batteries that ship with this
// connector. The one exception is the Nintendo Joycon 500 mAh battery.
//
// If you want to use the common 301230 battery, you will either need to crimp
// the connector yourself or buy a pre-crimped connector that you attach to
// the battery wires (available on digikey).

module.exports = {
    params: {
      designator: 'CONN',
      side: 'F',
      reverse: false,
      pad_1: {type: 'net', value: 'RAW'},
      pad_2: {type: 'net', value: 'GND'},

      cable_3dmodel_filename: '${EG_INFUSED_KIM_3D_MODELS}/Molex_Ezmate_Pico_Cable_2pin.step',
      cable_3dmodel_side: '',
      cable_3dmodel_xyz_scale: '',
      cable_3dmodel_xyz_rotation: '',
      cable_3dmodel_xyz_offset: '',

      socket_3dmodel_filename: '${EG_INFUSED_KIM_3D_MODELS}/Molex_Ezmate_Pico_Socket_2pin.step',
      socket_3dmodel_side: '',
      socket_3dmodel_xyz_scale: '',
      socket_3dmodel_xyz_rotation: '',
      socket_3dmodel_xyz_offset: '',

    },
    body: p => {

      const gen_3d_model = (filename, scale, rotation, offset, side, {
        default_side =  'F',
        scale_f =       [1, 1, 1],
        rotation_f =    [0, 0, 0],
        offset_f =      [0, 0, 0],
        scale_b =       [1, 1, 1],
        rotation_b =    [0, 0, 0],
        offset_b =      [0, 0, 0]
      } = {}) => {

        if(filename == '') {
          return '';
        }

        const get_3d_model_side = (side, default_side) => {

            if(side == '') {
                if(p.reverse == true) {
                    side = default_side;
                } else {
                    side = p.side;
                }
            }

            if(side == 'F' || side == 'B') {
                return side;
            } else {
                return default_side;
            }
        }

        const final_side = get_3d_model_side(side, default_side, p);
        const is_front = final_side === 'F';

        // Determine the actual values to use
        const final_scale = scale || (is_front ? scale_f : scale_b);
        const final_rotation = rotation || (is_front ? rotation_f : rotation_b);
        let final_offset = offset || (is_front ? offset_f : offset_b);

        // Fix bug that seems to happen during the upgrade from KiCad 5 to
        // 8. All offset values seem to be multiplied by 25.4. So here we
        // divide them so that the upgrade KiCad file ends up with the
        // correct value.
        const offset_divisor = 25.4;
        final_offset = final_offset.map(value => value / offset_divisor);

        return  `
          (model ${JSON.stringify(filename)}
            (at (xyz ${final_offset[0]} ${final_offset[1]} ${final_offset[2]}))
            (scale (xyz ${final_scale[0]} ${final_scale[1]} ${final_scale[2]}))
            (rotate (xyz ${final_rotation[0]} ${final_rotation[1]} ${final_rotation[2]}))
          )
        `;
      };

      const top = `
        (module conn_molex_pico_ezmate_1x02 (layer F.Cu) (tedit 6445F610)
          ${p.at /* parametric position */}
          (attr smd)

      `;

      const front = `
        (fp_text reference ${p.ref} (at 0.1 3.9 ${p.rot}) (layer F.SilkS) ${p.ref_hide}
          (effects (font (size 1 1) (thickness 0.15)))
        )
        (fp_line (start 0.64 2.63) (end 1.14 2.63) (layer F.SilkS) (width 0.12))
        (fp_line (start 0.34 2.13) (end 0.64 2.63) (layer F.SilkS) (width 0.12))
        (fp_line (start -0.34 2.13) (end 0.34 2.13) (layer F.SilkS) (width 0.12))
        (fp_line (start -0.64 2.63) (end -0.34 2.13) (layer F.SilkS) (width 0.12))
        (fp_line (start -0.45 2.02) (end 0.45 2.02) (layer F.Fab) (width 0.1))
        (fp_line (start -0.75 2.52) (end -0.45 2.02) (layer F.Fab) (width 0.1))
        (fp_line (start -2.1 2.52) (end -0.75 2.52) (layer F.Fab) (width 0.1))
        (fp_line (start -1.16 -2.09) (end -1.16 -2.3) (layer F.SilkS) (width 0.12))
        (fp_line (start -2.21 -2.09) (end -1.16 -2.09) (layer F.SilkS) (width 0.12))
        (fp_line (start -2.21 1.24) (end -2.21 -2.09) (layer F.SilkS) (width 0.12))
        (fp_line (start -2.1 -1.98) (end 2.1 -1.98) (layer F.Fab) (width 0.1))
        (fp_line (start -1.14 2.63) (end -0.64 2.63) (layer F.SilkS) (width 0.12))
        (fp_line (start 2.21 -2.09) (end 1.16 -2.09) (layer F.SilkS) (width 0.12))
        (fp_line (start 2.21 1.24) (end 2.21 -2.09) (layer F.SilkS) (width 0.12))
        (fp_line (start -0.6 -1.272893) (end -0.1 -1.98) (layer F.Fab) (width 0.1))
        (fp_line (start -1.1 -1.98) (end -0.6 -1.272893) (layer F.Fab) (width 0.1))
        (fp_line (start 2.6 -2.8) (end -2.6 -2.8) (layer F.CrtYd) (width 0.05))
        (fp_line (start -2.6 -2.8) (end -2.6 3.02) (layer F.CrtYd) (width 0.05))
        (fp_line (start 2.1 -1.98) (end 2.1 2.52) (layer F.Fab) (width 0.1))
        (fp_line (start -2.1 -1.98) (end -2.1 2.52) (layer F.Fab) (width 0.1))
        (fp_line (start 0.75 2.52) (end 2.1 2.52) (layer F.Fab) (width 0.1))
        (fp_line (start 0.45 2.02) (end 0.75 2.52) (layer F.Fab) (width 0.1))
        (fp_line (start 2.6 3.02) (end 2.6 -2.8) (layer F.CrtYd) (width 0.05))
        (fp_line (start -2.6 3.02) (end 2.6 3.02) (layer F.CrtYd) (width 0.05))
        (pad MP smd roundrect (at 1.75 1.9 ${p.rot}) (size 0.7 0.8) (layers F.Cu F.Paste F.Mask) (roundrect_rratio 0.25))
        (pad MP smd roundrect (at -1.75 1.9 ${p.rot}) (size 0.7 0.8) (layers F.Cu F.Paste F.Mask) (roundrect_rratio 0.25))
        (pad 2 smd roundrect (at 0.6 -1.875 ${p.rot}) (size 0.6 0.85) (layers F.Cu F.Paste F.Mask) (roundrect_rratio 0.25) ${p.pad_2.str})
        (pad 1 smd roundrect (at -0.6 -1.875 ${p.rot}) (size 0.6 0.85) (layers F.Cu F.Paste F.Mask) (roundrect_rratio 0.25) ${p.pad_1.str})
      `
      const back = `
        (fp_line (start -0.34 2.13) (end -0.64 2.63) (layer B.SilkS) (width 0.12))
        (fp_line (start -2.6 3.02) (end -2.6 -2.8) (layer B.CrtYd) (width 0.05))
        (fp_line (start 2.6 3.02) (end -2.6 3.02) (layer B.CrtYd) (width 0.05))
        (fp_line (start -0.64 2.63) (end -1.14 2.63) (layer B.SilkS) (width 0.12))
        (fp_line (start 1.16 -2.09) (end 1.16 -2.3) (layer B.SilkS) (width 0.12))
        (fp_line (start 2.21 -2.09) (end 1.16 -2.09) (layer B.SilkS) (width 0.12))
        (fp_line (start 2.21 1.24) (end 2.21 -2.09) (layer B.SilkS) (width 0.12))
        (fp_line (start 2.1 -1.98) (end -2.1 -1.98) (layer B.Fab) (width 0.1))
        (fp_line (start 1.14 2.63) (end 0.64 2.63) (layer B.SilkS) (width 0.12))
        (fp_line (start -2.21 -2.09) (end -1.16 -2.09) (layer B.SilkS) (width 0.12))
        (fp_line (start -2.21 1.24) (end -2.21 -2.09) (layer B.SilkS) (width 0.12))
        (fp_line (start 0.6 -1.272893) (end 0.1 -1.98) (layer B.Fab) (width 0.1))
        (fp_line (start 1.1 -1.98) (end 0.6 -1.272893) (layer B.Fab) (width 0.1))
        (fp_line (start -2.6 -2.8) (end 2.6 -2.8) (layer B.CrtYd) (width 0.05))
        (fp_line (start 2.6 -2.8) (end 2.6 3.02) (layer B.CrtYd) (width 0.05))
        (fp_line (start -2.1 -1.98) (end -2.1 2.52) (layer B.Fab) (width 0.1))
        (fp_line (start 2.1 -1.98) (end 2.1 2.52) (layer B.Fab) (width 0.1))
        (fp_line (start -0.75 2.52) (end -2.1 2.52) (layer B.Fab) (width 0.1))
        (fp_line (start -0.45 2.02) (end -0.75 2.52) (layer B.Fab) (width 0.1))
        (fp_line (start 0.64 2.63) (end 0.34 2.13) (layer B.SilkS) (width 0.12))
        (fp_line (start 0.45 2.02) (end -0.45 2.02) (layer B.Fab) (width 0.1))
        (fp_line (start 0.75 2.52) (end 0.45 2.02) (layer B.Fab) (width 0.1))
        (fp_line (start 2.1 2.52) (end 0.75 2.52) (layer B.Fab) (width 0.1))
        (fp_line (start 0.34 2.13) (end -0.34 2.13) (layer B.SilkS) (width 0.12))
        (pad MP smd roundrect (at 1.75 1.9 ${180 + p.rot}) (size 0.7 0.8) (layers B.Cu B.Paste B.Mask) (roundrect_rratio 0.25))
        (pad 2 smd roundrect (at -0.6 -1.875 ${180 + p.rot}) (size 0.6 0.85) (layers B.Cu B.Paste B.Mask) (roundrect_rratio 0.25) ${p.pad_2.str})
        (pad 1 smd roundrect (at 0.6 -1.875 ${180 + p.rot}) (size 0.6 0.85) (layers B.Cu B.Paste B.Mask) (roundrect_rratio 0.25) ${p.pad_1.str})
        (pad MP smd roundrect (at -1.75 1.9 ${180 + p.rot}) (size 0.7 0.8) (layers B.Cu B.Paste B.Mask) (roundrect_rratio 0.25))
      `

      const all_3d_models = `
        ${ gen_3d_model(
              p.cable_3dmodel_filename,
              p.cable_3dmodel_xyz_scale,
              p.cable_3dmodel_xyz_rotation,
              p.cable_3dmodel_xyz_offset,
              p.cable_3dmodel_side,
              {
                rotation_f: [0, 0, 0],
                offset_f: [0, 0, 0.8],

                rotation_b: [0, 180, 0],
                offset_b: [0, 0, -(1.6 + 0.8)],
              },
          )
        }
        ${ gen_3d_model(
              p.socket_3dmodel_filename,
              p.socket_3dmodel_xyz_scale,
              p.socket_3dmodel_xyz_rotation,
              p.socket_3dmodel_xyz_offset,
              p.socket_3dmodel_side,
              {
                rotation_f: [-90, 0, 0],
                offset_f: [0, 0, 1.4],

                rotation_b: [-90, 180, 0],
                offset_b: [0, 0, -3],
              },
          )
        }
    `

      const bottom = `
      )
      `

      let final = top;

      if(p.side == "F" || p.reverse) {
        final += front;
      }
      if(p.side == "B" || p.reverse) {
        final += back;
      }

      final += all_3d_models;
      final += bottom;

      return final;
    }
}

;module.exports = ((original) => {
    const defaults = {"socket_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/Molex_Ezmate_Pico_Socket_2pin.step","cable_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/Molex_Ezmate_Pico_Cable_2pin.step"};
    const bossVariant = null;
    const chocV2 = "${KIPRJMOD}/models/boardstudio/koktoh/Choc_V2_Red.step";
    return {
      ...original,
      params: {...original.params, ...defaults},
      body: p => {
        // The bundled extension crosses the PCB with a 5 mm outer diameter.
        const extensionDiameter = 5;
        if (false && p.drill < extensionDiameter
          && p.tp_extension_3dmodel_filename === defaults.tp_extension_3dmodel_filename
          && !p.tp_extension_3dmodel_xyz_scale
          && !p.tp_extension_3dmodel_xyz_rotation
          && !p.tp_extension_3dmodel_xyz_offset) {
          throw new Error('The bundled trackpoint extension requires a center drill of at least 5 mm. Select a compatible extension model or drill size.');
        }
        if (false && !p.choc_v1_support) {
          p = {...p};
          if (!p.choc_v2_support) {
            for (const key of Object.keys(defaults)) {
              if (p[key] === defaults[key]) { p[key] = ''; }
            }
          } else {
            if (p.switch_3dmodel_filename === defaults.switch_3dmodel_filename) {
              const automatic = !p.switch_3dmodel_xyz_rotation && !p.switch_3dmodel_xyz_offset
                && p.switch_3dmodel_xyz_scale.every(value => value === 1);
              const postDiameter = 4.8;
              if (automatic && (!p.include_stabilizer_pad || p.oval_stabilizer_pad
                || (p.center_hole_diameter > 0 && p.center_hole_diameter < postDiameter))) {
                throw new Error('The bundled Choc V2 model requires the round stabilizer hole and a center drill of at least 4.8 mm. Use a matching model or transform for modified hardware.');
              }
              p.switch_3dmodel_filename = chocV2;
              p.switch_3dmodel_xyz_rotation ||= p.side === 'F' ? [180, 0, 0] : [0, 180, 0];
              p.switch_3dmodel_xyz_offset ||= [0, 0, -p.pcb_thickness];
            }
            if (p.keycap_3dmodel_filename === defaults.keycap_3dmodel_filename) {
              p.keycap_3dmodel_filename = '';
            }
          }
        }
        if (bossVariant && p.include_bosses && p.reset_switch_3dmodel_filename === defaults.reset_switch_3dmodel_filename) {
          p = {...p, reset_switch_3dmodel_filename: bossVariant};
        }
        return original.body(p);
      }
    };
  })(module.exports);

return module.exports; })(),
  "infused-kim/conn_molex_pico_ezmate_1x05": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Author: @infused-kim
//
// Description:
// PCB footprint for for molex pico ezmate connector with 5 pins. Used to
// connect a trackpoint to the PCB on my keyboards.
//
// This connector was chosen over the more common JST connector, because it
// has a mated profile height of only 1.65 mm. This is lower than the Kailh
// Choc hotswap sockets.
//
// It should also be compatible with the JST ACH connector (which is almost the
// same).

module.exports = {
    params: {
      designator: 'CONN',
      side: 'F',
      reverse: false,
      pad_1: {type: 'net', value: 'CONN_1'},
      pad_2: {type: 'net', value: 'CONN_2'},
      pad_3: {type: 'net', value: 'CONN_3'},
      pad_4: {type: 'net', value: 'CONN_4'},
      pad_5: {type: 'net', value: 'CONN_5'},

      cable_3dmodel_filename: '${EG_INFUSED_KIM_3D_MODELS}/Molex_Ezmate_Pico_Cable_5pin.step',
      cable_3dmodel_side: '',
      cable_3dmodel_xyz_scale: '',
      cable_3dmodel_xyz_rotation: '',
      cable_3dmodel_xyz_offset: '',

      socket_3dmodel_filename: '${EG_INFUSED_KIM_3D_MODELS}/Molex_Ezmate_Pico_Socket_5pin.step',
      socket_3dmodel_side: '',
      socket_3dmodel_xyz_scale: '',
      socket_3dmodel_xyz_rotation: '',
      socket_3dmodel_xyz_offset: '',
    },
    body: p => {

      const gen_3d_model = (filename, scale, rotation, offset, side, {
        default_side =  'F',
        scale_f =       [1, 1, 1],
        rotation_f =    [0, 0, 0],
        offset_f =      [0, 0, 0],
        scale_b =       [1, 1, 1],
        rotation_b =    [0, 0, 0],
        offset_b =      [0, 0, 0]
      } = {}) => {

        if(filename == '') {
          return '';
        }

        const get_3d_model_side = (side, default_side) => {

            if(side == '') {
                if(p.reverse == true) {
                    side = default_side;
                } else {
                    side = p.side;
                }
            }

            if(side == 'F' || side == 'B') {
                return side;
            } else {
                return default_side;
            }
        }

        const final_side = get_3d_model_side(side, default_side, p);
        const is_front = final_side === 'F';

        // Determine the actual values to use
        const final_scale = scale || (is_front ? scale_f : scale_b);
        const final_rotation = rotation || (is_front ? rotation_f : rotation_b);
        let final_offset = offset || (is_front ? offset_f : offset_b);

        // Fix bug that seems to happen during the upgrade from KiCad 5 to
        // 8. All offset values seem to be multiplied by 25.4. So here we
        // divide them so that the upgrade KiCad file ends up with the
        // correct value.
        const offset_divisor = 25.4;
        final_offset = final_offset.map(value => value / offset_divisor);

        return  `
          (model ${JSON.stringify(filename)}
            (at (xyz ${final_offset[0]} ${final_offset[1]} ${final_offset[2]}))
            (scale (xyz ${final_scale[0]} ${final_scale[1]} ${final_scale[2]}))
            (rotate (xyz ${final_rotation[0]} ${final_rotation[1]} ${final_rotation[2]}))
          )
        `;
      };

      const top = `
      (module conn_molex_pico_ezmate_1x05 (layer F.Cu) (tedit 644602FB)
        ${p.at /* parametric position */}
        (attr smd)

      `;

      const front = `
        (fp_text reference "${p.ref}" (at 0 0.25 ${p.rot}) (layer F.SilkS) ${p.ref_hide}
          (effects (font (size 1 1) (thickness 0.15)))
        )
        (fp_line (start -2.96 -2.09) (end -2.96 -2.3) (layer F.SilkS) (width 0.12))
        (fp_line (start 4.01 1.24) (end 4.01 -2.09) (layer F.SilkS) (width 0.12))
        (fp_line (start 4.01 -2.09) (end 2.96 -2.09) (layer F.SilkS) (width 0.12))
        (fp_line (start -2.94 2.63) (end -2.44 2.63) (layer F.SilkS) (width 0.12))
        (fp_line (start -2.44 2.63) (end -2.14 2.13) (layer F.SilkS) (width 0.12))
        (fp_line (start -2.14 2.13) (end 2.14 2.13) (layer F.SilkS) (width 0.12))
        (fp_line (start 2.14 2.13) (end 2.44 2.63) (layer F.SilkS) (width 0.12))
        (fp_line (start 2.44 2.63) (end 2.94 2.63) (layer F.SilkS) (width 0.12))
        (fp_line (start -3.9 2.52) (end -2.55 2.52) (layer F.Fab) (width 0.1))
        (fp_line (start -2.55 2.52) (end -2.25 2.02) (layer F.Fab) (width 0.1))
        (fp_line (start -2.25 2.02) (end 2.25 2.02) (layer F.Fab) (width 0.1))
        (fp_line (start 2.25 2.02) (end 2.55 2.52) (layer F.Fab) (width 0.1))
        (fp_line (start 2.55 2.52) (end 3.9 2.52) (layer F.Fab) (width 0.1))
        (fp_line (start -3.9 -1.98) (end -3.9 2.52) (layer F.Fab) (width 0.1))
        (fp_line (start 3.9 -1.98) (end 3.9 2.52) (layer F.Fab) (width 0.1))
        (fp_line (start -4.4 -2.8) (end -4.4 3.02) (layer F.CrtYd) (width 0.05))
        (fp_line (start -4.4 3.02) (end 4.4 3.02) (layer F.CrtYd) (width 0.05))
        (fp_line (start 4.4 3.02) (end 4.4 -2.8) (layer F.CrtYd) (width 0.05))
        (fp_line (start 4.4 -2.8) (end -4.4 -2.8) (layer F.CrtYd) (width 0.05))
        (fp_line (start -2.9 -1.98) (end -2.4 -1.272893) (layer F.Fab) (width 0.1))
        (fp_line (start -2.4 -1.272893) (end -1.9 -1.98) (layer F.Fab) (width 0.1))
        (fp_line (start -3.9 -1.98) (end 3.9 -1.98) (layer F.Fab) (width 0.1))
        (fp_line (start -4.01 1.24) (end -4.01 -2.09) (layer F.SilkS) (width 0.12))
        (fp_line (start -4.01 -2.09) (end -2.96 -2.09) (layer F.SilkS) (width 0.12))
        (fp_text user %R (at 0 0.27 ${p.rot}) (layer F.Fab)
          (effects (font (size 1 1) (thickness 0.15)))
        )
        (pad 1 smd roundrect (at -2.4 -1.875 ${p.rot}) (size 0.6 0.85) (layers F.Cu F.Paste F.Mask) (roundrect_rratio 0.25) ${p.pad_1.str})
        (pad MP smd roundrect (at 3.55 1.9 ${p.rot}) (size 0.7 0.8) (layers F.Cu F.Paste F.Mask) (roundrect_rratio 0.25))
        (pad MP smd roundrect (at -3.55 1.9 ${p.rot}) (size 0.7 0.8) (layers F.Cu F.Paste F.Mask) (roundrect_rratio 0.25))
        (pad 2 smd roundrect (at -1.2 -1.875 ${p.rot}) (size 0.6 0.85) (layers F.Cu F.Paste F.Mask) (roundrect_rratio 0.25) ${p.pad_2.str})
        (pad 4 smd roundrect (at 1.2 -1.875 ${p.rot}) (size 0.6 0.85) (layers F.Cu F.Paste F.Mask) (roundrect_rratio 0.25) ${p.pad_4.str})
        (pad 5 smd roundrect (at 2.4 -1.875 ${p.rot}) (size 0.6 0.85) (layers F.Cu F.Paste F.Mask) (roundrect_rratio 0.25) ${p.pad_5.str})
        (pad 3 smd roundrect (at 0 -1.875 ${p.rot}) (size 0.6 0.85) (layers F.Cu F.Paste F.Mask) (roundrect_rratio 0.25) ${p.pad_3.str})
      `
      const back = `
        (fp_text user %R (at 0 0.27 ${180 + p.rot}) (layer B.Fab)
          (effects (font (size 1 1) (thickness 0.15)) (justify mirror))
        )
        (fp_line (start 2.94 2.63) (end 2.44 2.63) (layer B.SilkS) (width 0.12))
        (fp_line (start -2.14 2.13) (end -2.44 2.63) (layer B.SilkS) (width 0.12))
        (fp_line (start -2.44 2.63) (end -2.94 2.63) (layer B.SilkS) (width 0.12))
        (fp_line (start 3.9 2.52) (end 2.55 2.52) (layer B.Fab) (width 0.1))
        (fp_line (start 2.55 2.52) (end 2.25 2.02) (layer B.Fab) (width 0.1))
        (fp_line (start 2.25 2.02) (end -2.25 2.02) (layer B.Fab) (width 0.1))
        (fp_line (start -2.25 2.02) (end -2.55 2.52) (layer B.Fab) (width 0.1))
        (fp_line (start 2.44 2.63) (end 2.14 2.13) (layer B.SilkS) (width 0.12))
        (fp_line (start 2.14 2.13) (end -2.14 2.13) (layer B.SilkS) (width 0.12))
        (fp_line (start 2.96 -2.09) (end 2.96 -2.3) (layer B.SilkS) (width 0.12))
        (fp_line (start -4.01 -2.09) (end -2.96 -2.09) (layer B.SilkS) (width 0.12))
        (fp_line (start -2.55 2.52) (end -3.9 2.52) (layer B.Fab) (width 0.1))
        (fp_line (start 3.9 -1.98) (end 3.9 2.52) (layer B.Fab) (width 0.1))
        (fp_line (start -3.9 -1.98) (end -3.9 2.52) (layer B.Fab) (width 0.1))
        (fp_line (start 4.4 -2.8) (end 4.4 3.02) (layer B.CrtYd) (width 0.05))
        (fp_line (start 4.4 3.02) (end -4.4 3.02) (layer B.CrtYd) (width 0.05))
        (fp_line (start -4.4 3.02) (end -4.4 -2.8) (layer B.CrtYd) (width 0.05))
        (fp_line (start -4.4 -2.8) (end 4.4 -2.8) (layer B.CrtYd) (width 0.05))
        (fp_line (start 2.9 -1.98) (end 2.4 -1.272893) (layer B.Fab) (width 0.1))
        (fp_line (start 2.4 -1.272893) (end 1.9 -1.98) (layer B.Fab) (width 0.1))
        (fp_line (start 3.9 -1.98) (end -3.9 -1.98) (layer B.Fab) (width 0.1))
        (fp_line (start 4.01 1.24) (end 4.01 -2.09) (layer B.SilkS) (width 0.12))
        (fp_line (start 4.01 -2.09) (end 2.96 -2.09) (layer B.SilkS) (width 0.12))
        (fp_line (start -4.01 1.24) (end -4.01 -2.09) (layer B.SilkS) (width 0.12))
        (pad 3 smd roundrect (at 0 -1.875 ${180 + p.rot}) (size 0.6 0.85) (layers B.Cu B.Paste B.Mask) (roundrect_rratio 0.25) ${p.pad_3.str})
        (pad MP smd roundrect (at 3.55 1.9 ${180 + p.rot}) (size 0.7 0.8) (layers B.Cu B.Paste B.Mask) (roundrect_rratio 0.25))
        (pad 4 smd roundrect (at -1.2 -1.875 ${180 + p.rot}) (size 0.6 0.85) (layers B.Cu B.Paste B.Mask) (roundrect_rratio 0.25) ${p.pad_4.str})
        (pad 5 smd roundrect (at -2.4 -1.875 ${180 + p.rot}) (size 0.6 0.85) (layers B.Cu B.Paste B.Mask) (roundrect_rratio 0.25) ${p.pad_5.str})
        (pad 2 smd roundrect (at 1.2 -1.875 ${180 + p.rot}) (size 0.6 0.85) (layers B.Cu B.Paste B.Mask) (roundrect_rratio 0.25) ${p.pad_2.str})
        (pad 1 smd roundrect (at 2.4 -1.875 ${180 + p.rot}) (size 0.6 0.85) (layers B.Cu B.Paste B.Mask) (roundrect_rratio 0.25) ${p.pad_1.str})
        (pad MP smd roundrect (at -3.55 1.9 ${180 + p.rot}) (size 0.7 0.8) (layers B.Cu B.Paste B.Mask) (roundrect_rratio 0.25))
      `


        const all_3d_models = `
          ${ gen_3d_model(
                p.cable_3dmodel_filename,
                p.cable_3dmodel_xyz_scale,
                p.cable_3dmodel_xyz_rotation,
                p.cable_3dmodel_xyz_offset,
                p.cable_3dmodel_side,
                {
                  rotation_f: [0, 0, 0],
                  offset_f: [0, 0, 0.8],

                  rotation_b: [0, 180, 0],
                  offset_b: [0, 0, -(1.6 + 0.8)],
                },
            )
          }
          ${ gen_3d_model(
                p.socket_3dmodel_filename,
                p.socket_3dmodel_xyz_scale,
                p.socket_3dmodel_xyz_rotation,
                p.socket_3dmodel_xyz_offset,
                p.socket_3dmodel_side,
                {
                  rotation_f: [-90, 0, 0],
                  offset_f: [0, 0, 1.4],

                  rotation_b: [-90, 180, 0],
                  offset_b: [0, 0, -3],
                },
            )
          }
      `

      const bottom = `
      )
      `

      let final = top;

      if(p.side == "F" || p.reverse) {
        final += front;
      }
      if(p.side == "B" || p.reverse) {
        final += back;
      }

      final += all_3d_models;
      final += bottom;

      return final;
    }
}

;module.exports = ((original) => {
    const defaults = {"socket_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/Molex_Ezmate_Pico_Socket_5pin.step","cable_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/Molex_Ezmate_Pico_Cable_5pin.step"};
    const bossVariant = null;
    const chocV2 = "${KIPRJMOD}/models/boardstudio/koktoh/Choc_V2_Red.step";
    return {
      ...original,
      params: {...original.params, ...defaults},
      body: p => {
        // The bundled extension crosses the PCB with a 5 mm outer diameter.
        const extensionDiameter = 5;
        if (false && p.drill < extensionDiameter
          && p.tp_extension_3dmodel_filename === defaults.tp_extension_3dmodel_filename
          && !p.tp_extension_3dmodel_xyz_scale
          && !p.tp_extension_3dmodel_xyz_rotation
          && !p.tp_extension_3dmodel_xyz_offset) {
          throw new Error('The bundled trackpoint extension requires a center drill of at least 5 mm. Select a compatible extension model or drill size.');
        }
        if (false && !p.choc_v1_support) {
          p = {...p};
          if (!p.choc_v2_support) {
            for (const key of Object.keys(defaults)) {
              if (p[key] === defaults[key]) { p[key] = ''; }
            }
          } else {
            if (p.switch_3dmodel_filename === defaults.switch_3dmodel_filename) {
              const automatic = !p.switch_3dmodel_xyz_rotation && !p.switch_3dmodel_xyz_offset
                && p.switch_3dmodel_xyz_scale.every(value => value === 1);
              const postDiameter = 4.8;
              if (automatic && (!p.include_stabilizer_pad || p.oval_stabilizer_pad
                || (p.center_hole_diameter > 0 && p.center_hole_diameter < postDiameter))) {
                throw new Error('The bundled Choc V2 model requires the round stabilizer hole and a center drill of at least 4.8 mm. Use a matching model or transform for modified hardware.');
              }
              p.switch_3dmodel_filename = chocV2;
              p.switch_3dmodel_xyz_rotation ||= p.side === 'F' ? [180, 0, 0] : [0, 180, 0];
              p.switch_3dmodel_xyz_offset ||= [0, 0, -p.pcb_thickness];
            }
            if (p.keycap_3dmodel_filename === defaults.keycap_3dmodel_filename) {
              p.keycap_3dmodel_filename = '';
            }
          }
        }
        if (bossVariant && p.include_bosses && p.reset_switch_3dmodel_filename === defaults.reset_switch_3dmodel_filename) {
          p = {...p, reset_switch_3dmodel_filename: bossVariant};
        }
        return original.body(p);
      }
    };
  })(module.exports);

return module.exports; })(),
  "infused-kim/icon_bat": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Author: @infused-kim
//
// Description:
// Shows battery icons on the PCB silkscreen.

module.exports = {
    params: {
      designator: 'ICON',
      side: 'F',
      reverse: false,
      spacing: 1
    },
    body: p => {
      const spacing_adj = p.spacing / 2;

      const top = `
        (module icon_bat (layer F.Cu) (tedit 64461058)
          ${p.at /* parametric position */}
          (attr virtual)

      `;

      const front = `
          (fp_text reference "${p.ref}" (at 0 3 ${p.rot}) (layer F.SilkS) ${p.ref_hide}
            (effects (font (size 1 1) (thickness 0.15)))
          )
          (fp_circle (center ${-0.55 - spacing_adj} 0) (end ${-0.05 - spacing_adj} 0) (layer F.SilkS) (width 0.1))
          (fp_line (start ${-0.55 - spacing_adj} -0.3) (end ${-0.55 - spacing_adj} 0.3) (layer F.SilkS) (width 0.1))
          (fp_line (start ${-0.85 - spacing_adj} 0) (end ${-0.25 - spacing_adj} 0) (layer F.SilkS) (width 0.1))

          (fp_circle (center ${0.55 + spacing_adj} 0) (end ${1.05 + spacing_adj}  0) (layer F.SilkS) (width 0.1))
          (fp_line (start ${0.25 + spacing_adj} 0) (end ${0.85 + spacing_adj} 0) (layer F.SilkS) (width 0.1))
      `

      const back = `
          (fp_circle (center ${-0.55 - spacing_adj} 0) (end ${-1.05 - spacing_adj}  0) (layer B.SilkS) (width 0.1))
          (fp_line (start ${-0.25 - spacing_adj} 0) (end ${-0.85 - spacing_adj} 0) (layer B.SilkS) (width 0.1))

          (fp_circle (center ${0.55 + spacing_adj} 0) (end ${0.05 + spacing_adj} 0) (layer B.SilkS) (width 0.1))
          (fp_line (start ${0.55 + spacing_adj} -0.3) (end ${0.55 + spacing_adj} 0.3) (layer B.SilkS) (width 0.1))
          (fp_line (start ${0.85 + spacing_adj} 0) (end ${0.25 + spacing_adj} 0) (layer B.SilkS) (width 0.1))
      `

      const bottom = `
      )
      `

      let final = top;

      if(p.side == "F" || p.reverse) {
        final += front;
      }
      if(p.side == "B" || p.reverse) {
        final += back;
      }

      final += bottom;

      return final;
    }
}

return module.exports; })(),
  "infused-kim/mounting_hole": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Author: @infused-kim
//
// Description:
// Simple mounting hole with gold rim.
//
// Parameters:
//   - outline: The width of the gold rim around the hole (Default: 0.8mm)
//   - drill: The actual size for the hole (Default 2.2mm - for m2 screws)
//   - drill_y: The height if you want an oval hole (Default: off)

module.exports = {
    params: {
      designator: 'H',
      outline: 0.8,
      drill: 2.2,
      drill_y: 0,
    },
    body: p => {
        if(p.drill_y == 0) {
            p.drill_y = p.drill
        }

        const size_x = p.drill + p.outline * 2
        const size_y = p.drill_y + p.outline * 2

        const courtyard_offset = 0.25
        const courtyard_x = size_x / 2 + courtyard_offset
        const courtyard_y = size_y / 2 + courtyard_offset

        const top = `
            (module mounting_hole (layer F.Cu) (tedit 64B5A986)
                ${p.at /* parametric position */}
                (fp_text reference "${p.ref}" (at 0 3) (layer F.SilkS) ${p.ref_hide}
                    (effects (font (size 1 1) (thickness 0.15)))
                )
        `

        const pad_circle = `
                (pad "" thru_hole circle (at 0 0 180) (size ${size_x} ${size_y}) (drill ${p.drill}) (layers *.Cu *.Mask))
        `
        const courtyard_circle = `
                (fp_circle (center 0 0) (end -${courtyard_x} 0) (layer F.CrtYd) (width 0.05))
                (fp_circle (center 0 0) (end -${courtyard_x} 0) (layer B.CrtYd) (width 0.05))
        `
        const pad_oval = `
                (pad "" thru_hole oval (at 0 0 180) (size ${size_x} ${size_y}) (drill oval ${p.drill} ${p.drill_y}) (layers *.Cu *.Mask))
        `
        const courtyard_oval = `
                (fp_line (start ${courtyard_x} -${courtyard_y}) (end ${courtyard_x} ${courtyard_y}) (layer F.CrtYd) (width 0.05))
                (fp_line (start -${courtyard_x} -${courtyard_y}) (end -${courtyard_x} ${courtyard_y}) (layer F.CrtYd) (width 0.05))
                (fp_line (start -${courtyard_x} ${courtyard_y}) (end ${courtyard_x} ${courtyard_y}) (layer F.CrtYd) (width 0.05))
                (fp_line (start -${courtyard_x} -${courtyard_y}) (end ${courtyard_x} -${courtyard_y}) (layer F.CrtYd) (width 0.05))
                (fp_line (start -${courtyard_x} ${courtyard_y}) (end ${courtyard_x} ${courtyard_y}) (layer B.CrtYd) (width 0.05))
                (fp_line (start -${courtyard_x} ${courtyard_y}) (end -${courtyard_x} -${courtyard_y}) (layer B.CrtYd) (width 0.05))
                (fp_line (start -${courtyard_x} -${courtyard_y}) (end ${courtyard_x} -${courtyard_y}) (layer B.CrtYd) (width 0.05))
                (fp_line (start ${courtyard_x} ${courtyard_y}) (end ${courtyard_x} -${courtyard_y}) (layer B.CrtYd) (width 0.05))
        `

        const bottom = `
            )
        `

        let final = top
        if(size_x == size_y) {
            final += pad_circle
            final += courtyard_circle
        } else {
            final += pad_oval
            final += courtyard_oval
        }

        final += bottom

        return final
    }
}

return module.exports; })(),
  "infused-kim/nice_view": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Author: @infused-kim
//
// Description:
// Reversible footprint for nice!view display. Includes an outline of the
// display to make positioning easier.

module.exports = {
  params: {
    designator: 'DISP',
    side: 'F',
    reverse: false,
    pcb_thickness: 1.6,
    MOSI: {type: 'net', value: 'MOSI'},
    SCK: {type: 'net', value: 'SCK'},
    VCC: {type: 'net', value: 'VCC'},
    GND: {type: 'net', value: 'GND'},
    CS: {type: 'net', value: 'CS'},
    show_labels: {type: 'boolean', value: true},
    jumpers_at_bottom: false,

    // This side parameter applies to all 3d models
    display_3dmodel_side: '',

    display_3dmodel_filename: '${EG_INFUSED_KIM_3D_MODELS}/Nice_View.step',
    display_3dmodel_xyz_scale: '',
    display_3dmodel_xyz_rotation: '',
    display_3dmodel_xyz_offset: '',

    header_3dmodel_filename: '${EG_INFUSED_KIM_3D_MODELS}/PinHeader_2.54mm_1x-5.step',
    header_3dmodel_xyz_scale: '',
    header_3dmodel_xyz_rotation: '',
    header_3dmodel_xyz_offset: '',

    socket_3dmodel_filename: '${EG_INFUSED_KIM_3D_MODELS}/PinSocket_2.54mm_5mm_1x-5.step',
    socket_3dmodel_xyz_scale: '',
    socket_3dmodel_xyz_rotation: '',
    socket_3dmodel_xyz_offset: '',

  },
  body: p => {

    const gen_3d_model = (filename, scale, rotation, offset, side, {
      default_side =  'F',
      scale_f =       [1, 1, 1],
      rotation_f =    [0, 0, 0],
      offset_f =      [0, 0, 0],
      scale_b =       [1, 1, 1],
      rotation_b =    [0, 0, 0],
      offset_b =      [0, 0, 0]
    } = {}) => {

      if(filename == '') {
        return '';
      }

      const get_3d_model_side = (side, default_side) => {

          if(side == '') {
              if(p.reverse == true) {
                  side = default_side;
              } else {
                  side = p.side;
              }
          }

          if(side == 'F' || side == 'B') {
              return side;
          } else {
              return default_side;
          }
      }

      const final_side = get_3d_model_side(side, default_side, p);
      const is_front = final_side === 'F';

      // Determine the actual values to use
      const final_scale = scale || (is_front ? scale_f : scale_b);
      const final_rotation = rotation || (is_front ? rotation_f : rotation_b);
      let final_offset = offset || (is_front ? offset_f : offset_b);

      // Fix bug that seems to happen during the upgrade from KiCad 5 to
      // 8. All offset values seem to be multiplied by 25.4. So here we
      // divide them so that the upgrade KiCad file ends up with the
      // correct value.
      const offset_divisor = 25.4;
      final_offset = final_offset.map(value => value / offset_divisor);

      return  `
        (model ${JSON.stringify(filename)}
          (at (xyz ${final_offset[0]} ${final_offset[1]} ${final_offset[2]}))
          (scale (xyz ${final_scale[0]} ${final_scale[1]} ${final_scale[2]}))
          (rotate (xyz ${final_rotation[0]} ${final_rotation[1]} ${final_rotation[2]}))
        )
      `;
    };

    let dst_nets = [
      p.MOSI.str,
      p.SCK.str,
      p.VCC.str,
      p.GND.str,
      p.CS.str,
    ];
    local_nets = [
      p.local_net("1").str,
      p.local_net("2").str,
      p.VCC.str,
      p.local_net("4").str,
      p.local_net("5").str,
    ];

    let socket_nets = dst_nets;
    if(p.reverse) {
      socket_nets = local_nets;
    } else if(p.side == 'B') {
      socket_nets = dst_nets.slice().reverse();
    }

    let jumpers_offset = 0;
    let jumpers_rot = 0;
    let labels_offset = 0;
    let label_vcc_offset = 0;

    let jumpers_front_top = dst_nets;
    let jumpers_front_bottom = local_nets;
    let jumpers_back_top = dst_nets;
    let jumpers_back_bottom = local_nets.slice().reverse();
    if(p.jumpers_at_bottom) {
      jumpers_offset = 5.7;
      jumpers_rot = 180;
      labels_offset = jumpers_offset + 2 + 1 + 0.1;
      label_vcc_offset = 4.85;

      jumpers_front_top = local_nets;
      jumpers_front_bottom = dst_nets;
      jumpers_back_top = local_nets.slice().reverse();
      jumpers_back_bottom = dst_nets;
    }

    const top = `
      (module nice!view (layer F.Cu) (tedit 6448AF5B)
        ${p.at /* parametric position */}
        (attr virtual)
        (fp_text reference "${p.ref}" (at 0 20 ${p.rot}) (layer ${p.side}.SilkS) ${p.ref_hide}
          (effects (font (size 1 1) (thickness 0.15)))
        )
        `
    const front = `
        (fp_line (start -6.5 -18) (end 6.5 -18) (layer F.Fab) (width 0.15))
        (fp_line (start 6.5 18) (end -6.5 18) (layer F.Fab) (width 0.15))
        (fp_line (start -7 17.5) (end -7 -17.5) (layer F.Fab) (width 0.15))
        (fp_line (start 7 17.5) (end 7 -17.5) (layer F.Fab) (width 0.15))
        (fp_line (start -6.41 15.37) (end -6.41 18.03) (layer F.SilkS) (width 0.12))
        (fp_line (start 6.41 18.03) (end -6.41 18.03) (layer F.SilkS) (width 0.12))
        (fp_line (start 6.88 14.9) (end 6.88 18.45) (layer F.CrtYd) (width 0.15))
        (fp_line (start 6.88 18.45) (end -6.82 18.45) (layer F.CrtYd) (width 0.15))
        (fp_line (start -6.82 18.45) (end -6.82 14.9) (layer F.CrtYd) (width 0.15))
        (fp_line (start -6.82 14.9) (end 6.88 14.9) (layer F.CrtYd) (width 0.15))
        (fp_line (start 6.41 15.37) (end 6.41 18.03) (layer F.SilkS) (width 0.12))
        (fp_line (start 6.41 15.37) (end -6.41 15.37) (layer F.SilkS) (width 0.12))
        (fp_arc (start -6.5 17.5) (end -7 17.5) (angle -90) (layer F.Fab) (width 0.15))
        (fp_arc (start 6.5 17.5) (end 6.5 18) (angle -90) (layer F.Fab) (width 0.15))
        (fp_arc (start 6.5 -17.5) (end 6.5 -18) (angle 90) (layer F.Fab) (width 0.15))
        (fp_arc (start -6.5 -17.5) (end -6.5 -18) (angle -90) (layer F.Fab) (width 0.15))
        (fp_text user %R (at 0 20 ${p.rot}) (layer F.Fab)
          (effects (font (size 1 1) (thickness 0.15)))
        )

    `

    const front_jumpers = `
        (fp_line (start 5.93 ${12.9 + jumpers_offset}) (end 5.93 ${14.9 + jumpers_offset}) (layer F.Fab) (width 0.15))
        (fp_line (start -5.93 ${14.9 + jumpers_offset}) (end -5.93 ${12.9 + jumpers_offset}) (layer F.Fab) (width 0.15))
        (fp_line (start -5.93 ${12.9 + jumpers_offset}) (end -4.23 ${12.9 + jumpers_offset}) (layer F.Fab) (width 0.15))
        (fp_line (start -4.23 ${14.9 + jumpers_offset}) (end -5.93 ${14.9 + jumpers_offset}) (layer F.Fab) (width 0.15))
        (fp_line (start -4.23 ${12.9 + jumpers_offset}) (end -4.23 ${14.9 + jumpers_offset}) (layer F.Fab) (width 0.15))
        (fp_line (start -3.39 ${14.9 + jumpers_offset}) (end -3.39 ${12.9 + jumpers_offset}) (layer F.Fab) (width 0.15))
        (fp_line (start -3.39 ${12.9 + jumpers_offset}) (end -1.69 ${12.9 + jumpers_offset}) (layer F.Fab) (width 0.15))
        (fp_line (start -1.69 ${14.9 + jumpers_offset}) (end -3.39 ${14.9 + jumpers_offset}) (layer F.Fab) (width 0.15))
        (fp_line (start -1.69 ${12.9 + jumpers_offset}) (end -1.69 ${14.9 + jumpers_offset}) (layer F.Fab) (width 0.15))
        (fp_line (start 3.39 ${12.9 + jumpers_offset}) (end 3.39 ${14.9 + jumpers_offset}) (layer F.Fab) (width 0.15))
        (fp_line (start 3.39 ${14.9 + jumpers_offset}) (end 1.69 ${14.9 + jumpers_offset}) (layer F.Fab) (width 0.15))
        (fp_line (start 1.69 ${14.9 + jumpers_offset}) (end 1.69 ${12.9 + jumpers_offset}) (layer F.Fab) (width 0.15))
        (fp_line (start 1.69 ${12.9 + jumpers_offset}) (end 3.39 ${12.9 + jumpers_offset}) (layer F.Fab) (width 0.15))
        (fp_line (start 5.93 ${14.9 + jumpers_offset}) (end 4.23 ${14.9 + jumpers_offset}) (layer F.Fab) (width 0.15))
        (fp_line (start 4.23 ${14.9 + jumpers_offset}) (end 4.23 ${12.9 + jumpers_offset}) (layer F.Fab) (width 0.15))
        (fp_line (start 4.23 ${12.9 + jumpers_offset}) (end 5.93 ${12.9 + jumpers_offset}) (layer F.Fab) (width 0.15))

        (pad 14 smd rect (at -5.08 ${13.45 + jumpers_offset} ${90 + p.rot}) (size 0.6 1.2) (layers F.Cu F.Mask) ${ jumpers_front_top[0] })
        (pad 15 smd rect (at -2.54 ${13.45 + jumpers_offset} ${90 + p.rot}) (size 0.6 1.2) (layers F.Cu F.Mask) ${ jumpers_front_top[1] })
        (pad 16 smd rect (at 2.54 ${13.45 + jumpers_offset} ${90 + p.rot}) (size 0.6 1.2) (layers F.Cu F.Mask) ${ jumpers_front_top[3] })
        (pad 17 smd rect (at 5.08 ${13.45 + jumpers_offset} ${90 + p.rot}) (size 0.6 1.2) (layers F.Cu F.Mask) ${ jumpers_front_top[4] })

        (pad 10 smd rect (at -5.08 ${14.35 + jumpers_offset} ${90 + p.rot}) (size 0.6 1.2) (layers F.Cu F.Mask) ${ jumpers_front_bottom[0] })
        (pad 11 smd rect (at -2.54 ${14.35 + jumpers_offset} ${90 + p.rot}) (size 0.6 1.2) (layers F.Cu F.Mask) ${ jumpers_front_bottom[1] })
        (pad 12 smd rect (at 2.54 ${14.35 + jumpers_offset} ${90 + p.rot}) (size 0.6 1.2) (layers F.Cu F.Mask) ${ jumpers_front_bottom[3] })
        (pad 13 smd rect (at 5.08 ${14.35 + jumpers_offset} ${90 + p.rot}) (size 0.6 1.2) (layers F.Cu F.Mask) ${ jumpers_front_bottom[4] })
    `

    const back = `
        (fp_line (start 6.41 15.37) (end 6.41 18.03) (layer B.SilkS) (width 0.12))
        (fp_line (start 6.41 15.37) (end -6.41 15.37) (layer B.SilkS) (width 0.12))
        (fp_line (start 6.41 18.03) (end -6.41 18.03) (layer B.SilkS) (width 0.12))
        (fp_line (start 6.88 14.9) (end 6.88 18.45) (layer B.CrtYd) (width 0.15))
        (fp_line (start 6.88 18.45) (end -6.82 18.45) (layer B.CrtYd) (width 0.15))
        (fp_line (start -6.82 18.45) (end -6.82 14.9) (layer B.CrtYd) (width 0.15))
        (fp_line (start -6.82 14.9) (end 6.88 14.9) (layer B.CrtYd) (width 0.15))
        (fp_line (start -6.41 15.37) (end -6.41 18.03) (layer B.SilkS) (width 0.12))
        (fp_line (start -6.5 18) (end 6.5 18) (layer B.Fab) (width 0.15))
        (fp_line (start 7 -17.5) (end 7 17.5) (layer B.Fab) (width 0.15))
        (fp_line (start 6.5 -18) (end -6.5 -18) (layer B.Fab) (width 0.15))
        (fp_line (start -7 -17.5) (end -7 17.5) (layer B.Fab) (width 0.15))
        (fp_arc (start -6.5 -17.5) (end -7 -17.5) (angle 90) (layer B.Fab) (width 0.15))
        (fp_arc (start 6.5 -17.5) (end 6.5 -18) (angle 90) (layer B.Fab) (width 0.15))
        (fp_arc (start 6.5 17.5) (end 6.5 18) (angle -90) (layer B.Fab) (width 0.15))
        (fp_arc (start -6.5 17.5) (end -6.5 18) (angle 90) (layer B.Fab) (width 0.15))
    `

    const back_jumpers = `
        (fp_line (start -5.93 ${12.9 + jumpers_offset}) (end -5.93 ${14.9 + jumpers_offset}) (layer B.Fab) (width 0.15))
        (fp_line (start -5.93 ${14.9 + jumpers_offset}) (end -4.23 ${14.9 + jumpers_offset}) (layer B.Fab) (width 0.15))
        (fp_line (start -4.23 ${12.9 + jumpers_offset}) (end -5.93 ${12.9 + jumpers_offset}) (layer B.Fab) (width 0.15))
        (fp_line (start -4.23 ${14.9 + jumpers_offset}) (end -4.23 ${12.9 + jumpers_offset}) (layer B.Fab) (width 0.15))
        (fp_line (start -3.39 ${14.9 + jumpers_offset}) (end -1.69 ${14.9 + jumpers_offset}) (layer B.Fab) (width 0.15))
        (fp_line (start -1.69 ${12.9 + jumpers_offset}) (end -3.39 ${12.9 + jumpers_offset}) (layer B.Fab) (width 0.15))
        (fp_line (start 4.23 ${14.9 + jumpers_offset}) (end 5.93 ${14.9 + jumpers_offset}) (layer B.Fab) (width 0.15))
        (fp_line (start 5.93 ${14.9 + jumpers_offset}) (end 5.93 ${12.9 + jumpers_offset}) (layer B.Fab) (width 0.15))
        (fp_line (start 3.39 ${12.9 + jumpers_offset}) (end 1.69 ${12.9 + jumpers_offset}) (layer B.Fab) (width 0.15))
        (fp_line (start -1.69 ${14.9 + jumpers_offset}) (end -1.69 ${12.9 + jumpers_offset}) (layer B.Fab) (width 0.15))
        (fp_line (start -3.39 ${12.9 + jumpers_offset}) (end -3.39 ${14.9 + jumpers_offset}) (layer B.Fab) (width 0.15))
        (fp_line (start 1.69 ${12.9 + jumpers_offset}) (end 1.69 ${14.9 + jumpers_offset}) (layer B.Fab) (width 0.15))
        (fp_line (start 1.69 ${14.9 + jumpers_offset}) (end 3.39 ${14.9 + jumpers_offset}) (layer B.Fab) (width 0.15))
        (fp_line (start 3.39 ${14.9 + jumpers_offset}) (end 3.39 ${12.9 + jumpers_offset}) (layer B.Fab) (width 0.15))
        (fp_line (start 5.93 ${12.9 + jumpers_offset}) (end 4.23 ${12.9 + jumpers_offset}) (layer B.Fab) (width 0.15))
        (fp_line (start 4.23 ${12.9 + jumpers_offset}) (end 4.23 ${14.9 + jumpers_offset}) (layer B.Fab) (width 0.15))

        (pad 24 smd rect (at 5.08 ${13.45 + jumpers_offset} ${270 + p.rot}) (size 0.6 1.2) (layers B.Cu B.Mask) ${ jumpers_back_top[0] })
        (pad 25 smd rect (at 2.54 ${13.45 + jumpers_offset} ${270 + p.rot}) (size 0.6 1.2) (layers B.Cu B.Mask) ${ jumpers_back_top[1] })
        (pad 26 smd rect (at -2.54 ${13.45 + jumpers_offset} ${270 + p.rot}) (size 0.6 1.2) (layers B.Cu B.Mask) ${ jumpers_back_top[3] })
        (pad 27 smd rect (at -5.08 ${13.45 + jumpers_offset} ${270 + p.rot}) (size 0.6 1.2) (layers B.Cu B.Mask) ${ jumpers_back_top[4] })

        (pad 20 smd rect (at 5.08 ${14.35 + jumpers_offset} ${270 + p.rot}) (size 0.6 1.2) (layers B.Cu B.Mask) ${ jumpers_back_bottom[0] })
        (pad 21 smd rect (at 2.54 ${14.35 + jumpers_offset} ${270 + p.rot}) (size 0.6 1.2) (layers B.Cu B.Mask) ${ jumpers_back_bottom[1] })
        (pad 22 smd rect (at -2.54 ${14.35 + jumpers_offset} ${270 + p.rot}) (size 0.6 1.2) (layers B.Cu B.Mask) ${ jumpers_back_bottom[3] })
        (pad 23 smd rect (at -5.08 ${14.35 + jumpers_offset} ${270 + p.rot}) (size 0.6 1.2) (layers B.Cu B.Mask) ${ jumpers_back_bottom[4] })
    `

    const labels = `
        (fp_text user DA (at -5.08 ${12.5 + labels_offset} ${p.rot}) (layer F.SilkS)
          (effects (font (size 1 0.7) (thickness 0.1)))
        )
        (fp_text user CS (at 5.12 ${12.5 + labels_offset} ${p.rot}) (layer F.SilkS)
          (effects (font (size 1 0.7) (thickness 0.1)))
        )
        (fp_text user GND (at 2.62 ${12.5 + labels_offset} ${p.rot}) (layer F.SilkS)
          (effects (font (size 1 0.7) (thickness 0.1)))
        )
        (fp_text user VCC (at 0.15 ${14.4 + label_vcc_offset} ${p.rot}) (layer F.SilkS)
          (effects (font (size 1 0.7) (thickness 0.1)))
        )
        (fp_text user CL (at -2.48 ${12.5 + labels_offset} ${p.rot}) (layer F.SilkS)
          (effects (font (size 1 0.7) (thickness 0.1)))
        )
        (fp_text user CS (at -4.98 ${12.5 + labels_offset} ${p.rot}) (layer B.SilkS)
          (effects (font (size 1 0.7) (thickness 0.1)) (justify mirror))
        )
        (fp_text user VCC (at 0.15 ${14.4 + label_vcc_offset} ${p.rot}) (layer B.SilkS)
          (effects (font (size 1 0.7) (thickness 0.1)) (justify mirror))
        )
        (fp_text user DA (at 5.22 ${12.5 + labels_offset} ${p.rot}) (layer B.SilkS)
          (effects (font (size 1 0.7) (thickness 0.1)) (justify mirror))
        )
        (fp_text user CL (at 2.72 ${12.5 + labels_offset} ${p.rot}) (layer B.SilkS)
          (effects (font (size 1 0.7) (thickness 0.1)) (justify mirror))
        )
        (fp_text user GND (at -2.38 ${12.5 + labels_offset} ${p.rot}) (layer B.SilkS)
          (effects (font (size 1 0.7) (thickness 0.1)) (justify mirror))
        )
    `

    const bottom = `
      (pad 1 thru_hole oval (at -5.08 16.7 ${270 + p.rot}) (size 1.7 1.7) (drill 1) (layers *.Cu *.Mask) ${ socket_nets[0] })
      (pad 2 thru_hole oval (at -2.54 16.7 ${270 + p.rot}) (size 1.7 1.7) (drill 1) (layers *.Cu *.Mask) ${ socket_nets[1] })
      (pad 3 thru_hole oval (at 0 16.7 ${270 + p.rot}) (size 1.7 1.7) (drill 1) (layers *.Cu *.Mask) ${ socket_nets[2] })
      (pad 4 thru_hole oval (at 2.54 16.7 ${270 + p.rot}) (size 1.7 1.7) (drill 1) (layers *.Cu *.Mask) ${ socket_nets[3] })
      (pad 5 thru_hole circle (at 5.08 16.7 ${270 + p.rot}) (size 1.7 1.7) (drill 1) (layers *.Cu *.Mask) ${ socket_nets[4] })

      (fp_line (start 5.4 13.4) (end 5.4 -11.9) (layer Dwgs.User) (width 0.15))
      (fp_line (start -5.4 13.4) (end -5.4 -11.9) (layer Dwgs.User) (width 0.15))
      (fp_line (start 5.4 -11.9) (end -5.4 -11.9) (layer Dwgs.User) (width 0.15))
      (fp_line (start -5.4 13.4) (end 5.4 13.4) (layer Dwgs.User) (width 0.15))
    )
    `

    let final = top;

    if(p.side == "F" || p.reverse) {
      final += front;
    }
    if(p.side == "B" || p.reverse) {
      final += back;
    }

    if(p.reverse) {
      final += front_jumpers;
      final += back_jumpers;

      if(p.show_labels) {
        final += labels;
      }
    }

    // Match the standard 8.5 mm socket while retaining the model-side override.
    const socket_height = 8.5;
    const display_height = socket_height + 1.8;
    const header_height = socket_height - 2;
    final += `
      ${ gen_3d_model(
            p.display_3dmodel_filename,
            p.display_3dmodel_xyz_scale,
            p.display_3dmodel_xyz_rotation,
            p.display_3dmodel_xyz_offset,
            p.display_3dmodel_side,
            {
              rotation_f: [0, 0, 0],
              offset_f: [-7, -18, display_height],

              rotation_b: [0, 180, 0],
              offset_b: [7, -18, -p.pcb_thickness - display_height],
            },
        )
      }
      ${ gen_3d_model(
            p.header_3dmodel_filename,
            p.header_3dmodel_xyz_scale,
            p.header_3dmodel_xyz_rotation,
            p.header_3dmodel_xyz_offset,
            p.display_3dmodel_side,
            {
              rotation_f: [0, 0, -90],
              offset_f: [0.0124, -16.732, header_height],

              rotation_b: [180, 0, -90],
              offset_b: [-0.0124, -16.732, -p.pcb_thickness - header_height],
            },
        )
      }
      ${ gen_3d_model(
            p.socket_3dmodel_filename,
            p.socket_3dmodel_xyz_scale,
            p.socket_3dmodel_xyz_rotation,
            p.socket_3dmodel_xyz_offset,
            p.display_3dmodel_side,
            {
              rotation_f: [0, 0, -90],
              offset_f: [-5.08, -16.7, 0],

              rotation_b: [180, 0, -90],
              offset_b: [5.08, -16.7, -p.pcb_thickness],
            },
        )
      }
    `

    final += bottom;

    return final;
  }
}
;module.exports = ((original) => {
    const defaults = {"display_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/Nice_View.step","header_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/PinHeader_2.54mm_1x-5.step","socket_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/kicad/PinSocket_1x05_P2.54mm_Vertical.step"};
    const bossVariant = null;
    const chocV2 = "${KIPRJMOD}/models/boardstudio/koktoh/Choc_V2_Red.step";
    return {
      ...original,
      params: {...original.params, ...defaults},
      body: p => {
        // The bundled extension crosses the PCB with a 5 mm outer diameter.
        const extensionDiameter = 5;
        if (false && p.drill < extensionDiameter
          && p.tp_extension_3dmodel_filename === defaults.tp_extension_3dmodel_filename
          && !p.tp_extension_3dmodel_xyz_scale
          && !p.tp_extension_3dmodel_xyz_rotation
          && !p.tp_extension_3dmodel_xyz_offset) {
          throw new Error('The bundled trackpoint extension requires a center drill of at least 5 mm. Select a compatible extension model or drill size.');
        }
        if (false && !p.choc_v1_support) {
          p = {...p};
          if (!p.choc_v2_support) {
            for (const key of Object.keys(defaults)) {
              if (p[key] === defaults[key]) { p[key] = ''; }
            }
          } else {
            if (p.switch_3dmodel_filename === defaults.switch_3dmodel_filename) {
              const automatic = !p.switch_3dmodel_xyz_rotation && !p.switch_3dmodel_xyz_offset
                && p.switch_3dmodel_xyz_scale.every(value => value === 1);
              const postDiameter = 4.8;
              if (automatic && (!p.include_stabilizer_pad || p.oval_stabilizer_pad
                || (p.center_hole_diameter > 0 && p.center_hole_diameter < postDiameter))) {
                throw new Error('The bundled Choc V2 model requires the round stabilizer hole and a center drill of at least 4.8 mm. Use a matching model or transform for modified hardware.');
              }
              p.switch_3dmodel_filename = chocV2;
              p.switch_3dmodel_xyz_rotation ||= p.side === 'F' ? [180, 0, 0] : [0, 180, 0];
              p.switch_3dmodel_xyz_offset ||= [0, 0, -p.pcb_thickness];
            }
            if (p.keycap_3dmodel_filename === defaults.keycap_3dmodel_filename) {
              p.keycap_3dmodel_filename = '';
            }
          }
        }
        if (bossVariant && p.include_bosses && p.reset_switch_3dmodel_filename === defaults.reset_switch_3dmodel_filename) {
          p = {...p, reset_switch_3dmodel_filename: bossVariant};
        }
        return original.body(p);
      }
    };
  })(module.exports);

return module.exports; })(),
  "infused-kim/pads": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Author: @infused-kim
//
// Description:
// Let's you place solder pads on the PCB that can be used instead of
// connectors. Useful for batteries and other peripherals in case the end-user
// does not have the right cable connector.
//
// Fully reversible and pads are mirrored on the back side.

module.exports = {
    params: {
        designator: 'PAD',
        side: 'F',
        reverse: true,
        width: 1.25,
        height: 2.5,
        space: 2,
        mirror: true,
        pads: 2,
        net_1: {type: 'net', value: 'PAD_1'},
        net_2: {type: 'net', value: 'PAD_2'},
        net_3: {type: 'net', value: 'PAD_3'},
        net_4: {type: 'net', value: 'PAD_4'},
        net_5: {type: 'net', value: 'PAD_5'},
        net_6: {type: 'net', value: 'PAD_6'},
        label_1: '',
        label_2: '',
        label_3: '',
        label_4: '',
        label_5: '',
        label_6: '',
        label_at_bottom: false,
      },
    body: p => {

        const gen_nets = (p) => {
          const all_nets = [
            p.net_1.str, p.net_2.str, p.net_3.str,
            p.net_4.str, p.net_5.str, p.net_6.str,
          ];
          const all_labels = [
            p.label_1, p.label_2, p.label_3,
            p.label_4, p.label_5, p.label_6,
          ];

          pad_cnt = p.pads;
          if(pad_cnt > all_nets.length || pad_cnt > all_labels.length) {
            pad_cnt = Math.min(all_nets.length, all_labels.length);
          }

          let nets = [];
          for(let i = 0; i < pad_cnt; i++) {
            let net = [
              all_nets[i],
              all_labels[i],
            ];
            nets.push(net);
          }

          return nets;
        }

        const gen_pad = (pad_idx, pad_cnt, net_str, net_label, width, height, space, rot, layer, label_at_bottom) =>
        {
            // Calculate the pad position from center
            const pos_x_raw = (width + space) * pad_idx;

            // Adjust it so that the pads are centered in the middle
            const pos_x = (
              pos_x_raw - (width + space) * (pad_cnt - 1) / 2
            );

            let label_pos_y = -1 * (height / 2 + 0.2);
            let label_justify_direction = "left";
            if(label_at_bottom) {
              label_pos_y = label_pos_y * -1;
              label_justify_direction = "right";
            }

            if(label_at_bottom == false || layer == 'B') {
              if((rot > 0 && rot <= 180) || (rot <= -180)) {
                label_justify_direction = "right";
              } else {
                label_justify_direction = "left";
              }
            } else {
              if((rot > 0 && rot <= 180) || (rot <= -180)) {
                label_justify_direction = "left";
              } else {
                label_justify_direction = "right";
              }
            }

            let justify_mirror = '';
            if(layer == 'B') {
              justify_mirror = 'mirror'
            }

            let label_justify = '';
            if(justify_mirror != '' || label_justify_direction != '') {
              label_justify = `(justify ${label_justify_direction} ${justify_mirror})`;
            }

            let pad = `
                (pad ${pad_idx + 1} smd rect (at ${pos_x} 0 ${rot}) (size ${width} ${height}) (layers ${layer}.Cu ${layer}.Paste ${layer}.Mask) ${net_str})
            `

            if(net_label) {
              pad += `
                (fp_text user "${net_label}" (at ${pos_x} ${label_pos_y} ${90 + rot}) (layer ${layer}.SilkS)
                  (effects (font (size 1 1) (thickness 0.1)) ${label_justify})
                )
              `
            }

            return pad;
        }

        const gen_pads = (nets, width, height, space, rot, layer, label_at_bottom, mirror) => {

            if(mirror) {
                nets = nets.slice().reverse();
            }

            let pads = '';
            for (let [net_idx, net] of nets.entries()) {

                const net_str = net[0];
                const net_label = net[1];

                const pad = gen_pad(
                  net_idx,
                  nets.length,
                  net_str,
                  net_label,
                  width,
                  height,
                  space,
                  rot,
                  layer,
                  label_at_bottom,
                );

                pads += pad;
            }

            return pads;
        }

        const nets = gen_nets(p);

        let pads_front = '';
        if(p.side == 'F' || p.reverse) {
          pads_front = gen_pads(
            nets,
            p.width, p.height, p.space, p.rot, "F",
            p.label_at_bottom, false,
          );
        }

        let pads_back = '';
        if(p.side == 'B' || p.reverse) {
          pads_back = gen_pads(
            nets,
            p.width, p.height, p.space, p.rot, "B",
            p.label_at_bottom, p.mirror,
          );
        }
        const fp = `
          (module pads (layer F.Cu) (tedit 6446BF3D)
            ${p.at /* parametric position */}
            (attr smd)

            (fp_text reference "${p.ref}" (at 0 2.2) (layer F.SilkS) ${p.ref_hide}
              (effects (font (size 1 1) (thickness 0.15)))
            )
            ${pads_front}
            ${pads_back}
          )
        `

        return fp;
    }
}

return module.exports; })(),
  "infused-kim/point_debugger": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Author: @infused-kim
//
// Description:
// Adds tiny x and y arrows at the origin of points to help with the debugging
// of complex layouts.
//
// This will only show arrows for actual points you define. So if you draw an
// outline without defining points, then they won't show up.
//
// Usage:
// You can make enabling and disabling easy with ergogen's preprocessor:
//
// ```js
// settings:
//   point_debugger:
//     enabled: false
// [...]
// pcbs:
//   your_keyboard:
//     footprints:
//       point_debugger:
//         what: infused-kim/point_debugger
//         where: true
//         params:
//           $extends: settings.point_debugger
// ```
//
// Showing the point name:
// The footprint supports showing the name of the point when you zoom in, but
// the latest version of ergogen (4.0.4) does not make the name available to
// the footprint.
//
// Until this gets merged, you can use the ergogen fork from this PR:
// https://github.com/ergogen/ergogen/pull/103

module.exports = {
    params: {
      designator: 'P',
      enabled: true,
    },
    body: p => {
        if(p.enabled == false) {
            return '';
        }

        const top = `
            (module point_debugger (layer F.Cu) (tedit 64B42FA5)
                ${p.at /* parametric position */}
                (fp_text reference ${p.ref} (at 0 2) (layer F.SilkS) ${p.ref_hide}
                    (effects (font (size 1 1) (thickness 0.15)))
                )
                (fp_line (start -0.6 0) (end 0.6 0) (layer Dwgs.User) (width 0.05))
                (fp_line (start 0 -0.6) (end 0 0.6) (layer Dwgs.User) (width 0.05))
                (fp_line (start 0.6 0) (end 0.5 -0.1) (layer Dwgs.User) (width 0.05))
                (fp_line (start 0.6 0) (end 0.5 0.1) (layer Dwgs.User) (width 0.05))
                (fp_line (start 0 -0.6) (end 0.1 -0.5) (layer Dwgs.User) (width 0.05))
                (fp_line (start 0 -0.6) (end -0.1 -0.5) (layer Dwgs.User) (width 0.05))
        `

        const bottom = `
            )
        `

        let final = top;

        // point is a property that is not available to footprints as of
        // ergogen 4.0.4. I have submitted a PR to add it and until then
        // my custom fork needs to be used if you want to see it.
        if('point' in p) {
            final += `
                    (fp_text user ${p.point.meta.name} (at -0.3 -0.05 ${p.rot}) (layer Dwgs.User)
                        (effects (font (size 0.0254 0.0254) (thickness 0.001)))
                    )
            `;
        }

        final += bottom;

        return final;
    }
}

return module.exports; })(),
  "infused-kim/smd_0805": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Author: @infused-kim
//
// Description:
// Let's you place multiple SMD 0805 components, such as resistors and
// capacitors.
//
// Fully reversible and component order is mirrored on the back.

module.exports = {
    params: {
        designator: 'SMD',
        side: 'F',
        reverse: true,
        space: 2,
        mirror: true,
        swap_pad_direction: false,
        components: 2,
        net_1_from: {type: 'net', value: 'SMD_1_F'},
        net_1_to: {type: 'net', value: 'SMD_1_T'},
        net_2_from: {type: 'net', value: 'SMD_2_F'},
        net_2_to: {type: 'net', value: 'SMD_2_T'},
        net_3_from: {type: 'net', value: 'SMD_3_F'},
        net_3_to: {type: 'net', value: 'SMD_3_T'},
        net_4_from: {type: 'net', value: 'SMD_4_F'},
        net_4_to: {type: 'net', value: 'SMD_4_T'},
        net_5_from: {type: 'net', value: 'SMD_5_F'},
        net_5_to: {type: 'net', value: 'SMD_5_T'},
        net_6_from: {type: 'net', value: 'SMD_6_F'},
        net_6_to: {type: 'net', value: 'SMD_6_T'},
        label_1: '',
        label_2: '',
        label_3: '',
        label_4: '',
        label_5: '',
        label_6: '',
        label_at_bottom: false,

        component_3dmodel_side: '',

        component_1_3dmodel_filename: '',
        component_1_3dmodel_xyz_scale: '',
        component_1_3dmodel_xyz_rotation: '',
        component_1_3dmodel_xyz_offset: '',
        component_2_3dmodel_filename: '',
        component_2_3dmodel_xyz_scale: '',
        component_2_3dmodel_xyz_rotation: '',
        component_2_3dmodel_xyz_offset: '',
        component_3_3dmodel_filename: '',
        component_3_3dmodel_xyz_scale: '',
        component_3_3dmodel_xyz_rotation: '',
        component_3_3dmodel_xyz_offset: '',
        component_4_3dmodel_filename: '',
        component_4_3dmodel_xyz_scale: '',
        component_4_3dmodel_xyz_rotation: '',
        component_4_3dmodel_xyz_offset: '',
        component_5_3dmodel_filename: '',
        component_5_3dmodel_xyz_scale: '',
        component_5_3dmodel_xyz_rotation: '',
        component_5_3dmodel_xyz_offset: '',
        component_6_3dmodel_filename: '',
        component_6_3dmodel_xyz_scale: '',
        component_6_3dmodel_xyz_rotation: '',
        component_6_3dmodel_xyz_offset: '',
      },
    body: p => {

        const get_3d_model_side = (model_side, default_side) => {

            if(model_side == '') {
                if(p.reverse == true) {
                    model_side = default_side;
                } else {
                    model_side = p.side;
                }
            }

            if(model_side == 'F' || model_side == 'B') {
                return model_side;
            } else {
                return default_side;
            }
        }

        const gen_3d_model = (filename, scale, rotation, offset, side, {
            default_side =  'F',
            scale_f =       [1, 1, 1],
            rotation_f =    [0, 0, 0],
            offset_f =      [0, 0, 0],
            scale_b =       [1, 1, 1],
            rotation_b =    [0, 0, 0],
            offset_b =      [0, 0, 0]
        } = {}) => {

            if(filename == '') {
              return '';
            }

            const final_side = get_3d_model_side(side, default_side, p);
            const is_front = final_side === 'F';

            // Determine the actual values to use
            const final_scale = scale || (is_front ? scale_f : scale_b);
            const final_rotation = rotation || (is_front ? rotation_f : rotation_b);
            let final_offset = offset || (is_front ? offset_f : offset_b);

            // Fix bug that seems to happen during the upgrade from KiCad 5 to
            // 8. All offset values seem to be multiplied by 25.4. So here we
            // divide them so that the upgrade KiCad file ends up with the
            // correct value.
            const offset_divisor = 25.4;
            final_offset = final_offset.map(value => value / offset_divisor);

            return  `
                (model ${JSON.stringify(filename)}
                (at (xyz ${final_offset[0]} ${final_offset[1]} ${final_offset[2]}))
                (scale (xyz ${final_scale[0]} ${final_scale[1]} ${final_scale[2]}))
                (rotate (xyz ${final_rotation[0]} ${final_rotation[1]} ${final_rotation[2]}))
                )
            `;
        };

        const gen_3d_model_for_net = (net_idx, pos_x) => {
            const prop_base =         `component_${net_idx + 1}`;
            const prop_filename =     `${prop_base}_3dmodel_filename`;
            const prop_scale =        `${prop_base}_3dmodel_xyz_scale`;
            const prop_rotation =     `${prop_base}_3dmodel_xyz_rotation`;
            const prop_offset =       `${prop_base}_3dmodel_xyz_offset`;

            if(!p[prop_filename]) {
                return '';
            }

            const model = gen_3d_model(
                p[prop_filename],
                p[prop_scale],
                p[prop_rotation],
                p[prop_offset],
                p.component_3dmodel_side,
                {
                  rotation_f: [0, 0, 0],
                  offset_f:   [pos_x, 0, 0],

                  rotation_b: [0, 180, 0],
                  offset_b:   [p.mirror ? -pos_x : pos_x, 0, -1.6],
                },
            )

            return model;
        }

        const gen_nets = (p) => {
          const all_nets_from = [
            p.net_1_from.str, p.net_2_from.str, p.net_3_from.str,
            p.net_4_from.str, p.net_5_from.str, p.net_6_from.str,
          ];
          const all_nets_to = [
            p.net_1_to.str, p.net_2_to.str, p.net_3_to.str,
            p.net_4_to.str, p.net_5_to.str, p.net_6_to.str,
          ];
          const all_labels = [
            p.label_1, p.label_2, p.label_3,
            p.label_4, p.label_5, p.label_6,
          ];

          pad_cnt = p.components;
          if(pad_cnt > all_nets_from.length || pad_cnt > all_nets_to.length ||
             pad_cnt > all_labels.length) {
            pad_cnt = Math.min(
              all_nets_from.length, all_nets_to.length, all_labels.length
            );
          }

          let nets = [];
          for(let i = 0; i < pad_cnt; i++) {
            let net = [
              all_nets_from[i],
              all_nets_to[i],
              all_labels[i],
            ];
            nets.push(net);
          }

          return nets;
        }

        const gen_pad = (pad_idx, pad_cnt, net_from, net_to, net_label, space, rot, layer, label_at_bottom) =>
        {
            const width = 1.025;
            const height= 3.36;

            // Calculate the pad position from center
            const pos_x_raw = (width + space) * pad_idx;

            // Adjust it so that the pads are centered in the middle
            const pos_x = (
              pos_x_raw - (width + space) * (pad_cnt - 1) / 2
            );

            let label_pos_y = -1 * (height / 2 + 0.2);
            let label_justify_direction = "left";
            if(label_at_bottom) {
              label_pos_y = label_pos_y * -1;
              label_justify_direction = "right";
            }

            if(label_at_bottom == false || layer == 'B') {
              if((rot > 0 && rot <= 180) || (rot <= -180)) {
                label_justify_direction = "right";
              } else {
                label_justify_direction = "left";
              }
            } else {
              if((rot > 0 && rot <= 180) || (rot <= -180)) {
                label_justify_direction = "left";
              } else {
                label_justify_direction = "right";
              }
            }

            let justify_mirror = '';
            if(layer == 'B') {
              justify_mirror = 'mirror'
            }

            let label_justify = '';
            if(justify_mirror != '' || label_justify_direction != '') {
              label_justify = `(justify ${label_justify_direction} ${justify_mirror})`;
            }

            let label_fab_justify = '';
            if(justify_mirror) {
              label_fab_justify = `(justify ${justify_mirror})`;
            }

            const pad_num = pad_idx*2+1;
            let pad = `
                (fp_line (start ${0.625 + pos_x} -1) (end ${0.625 + pos_x} 1) (layer ${layer}.Fab) (width 0.1))
                (fp_line (start ${-0.625 + pos_x} -1) (end ${0.625 + pos_x} -1) (layer ${layer}.Fab) (width 0.1))
                (fp_line (start ${-0.625 + pos_x} 1) (end ${-0.625 + pos_x} -1) (layer ${layer}.Fab) (width 0.1))
                (fp_line (start ${0.625 + pos_x} 1) (end ${-0.625 + pos_x} 1) (layer ${layer}.Fab) (width 0.1))

                (fp_line (start ${0.95 + pos_x} -1.68) (end ${0.95 + pos_x} 1.68) (layer ${layer}.CrtYd) (width 0.05))
                (fp_line (start ${-0.95 + pos_x} -1.68) (end ${0.95 + pos_x} -1.68) (layer ${layer}.CrtYd) (width 0.05))
                (fp_line (start ${-0.95 + pos_x} 1.68) (end ${-0.95 + pos_x} -1.68) (layer ${layer}.CrtYd) (width 0.05))
                (fp_line (start ${0.95 + pos_x} 1.68) (end ${-0.95 + pos_x} 1.68) (layer ${layer}.CrtYd) (width 0.05))

                (fp_line (start ${0.735 + pos_x} 0.227064) (end ${0.735 + pos_x} -0.227064) (layer ${layer}.SilkS) (width 0.12))
                (fp_line (start ${-0.735 + pos_x} 0.227064) (end ${-0.735 + pos_x} -0.227064) (layer ${layer}.SilkS) (width 0.12))

                (pad ${pad_num} smd roundrect (at ${0 + pos_x} 0.9125 ${90 + rot}) (size 1.025 1.4) (layers ${layer}.Cu ${layer}.Paste ${layer}.Mask) (roundrect_rratio 0.243902) ${net_from})
                (pad ${pad_num + 1} smd roundrect (at ${0 + pos_x} -0.9125 ${90 + rot}) (size 1.025 1.4) (layers ${layer}.Cu ${layer}.Paste ${layer}.Mask) (roundrect_rratio 0.243902) ${net_to})
            `

            if(net_label) {
              pad += `
              (fp_text user "${net_label}" (at ${0 + pos_x} 0 ${90 + rot}) (layer ${layer}.Fab)
                (effects (font (size 0.5 0.5) (thickness 0.08)) ${label_fab_justify})
              )
              (fp_text user "${net_label}" (at ${pos_x} ${label_pos_y} ${90 + rot}) (layer ${layer}.SilkS)
                  (effects (font (size 1 1) (thickness 0.1)) ${label_justify})
                )
              `
            }

            const side_3dmodel = get_3d_model_side(p.component_3dmodel_side, 'F');
            if(layer == side_3dmodel) {
                pad += gen_3d_model_for_net(pad_idx, pos_x);
            }

            return pad;
        }

        const gen_pads = (nets, space, rot, layer, label_at_bottom, mirror, swap_pad_direction) => {

            if(mirror) {
                nets = nets.slice().reverse();
            }

            let pads = '';
            for (let [net_idx, net] of nets.entries()) {

                let net_from = net[0];
                let net_to = net[1];
                const net_label = net[2];

                if(swap_pad_direction) {
                  net_from = net[1];
                  net_to = net[0];
                }

                const pad = gen_pad(
                  net_idx,
                  nets.length,
                  net_from,
                  net_to,
                  net_label,
                  space,
                  rot,
                  layer,
                  label_at_bottom,
                  swap_pad_direction,
                );

                pads += pad;
            }

            return pads;
        }

        const nets = gen_nets(p);

        let pads_front = '';
        if(p.side == 'F' || p.reverse) {
          pads_front = gen_pads(
            nets,
            p.space, p.rot, "F",
            p.label_at_bottom, false, p.swap_pad_direction,
          );
        }

        let pads_back = '';
        if(p.side == 'B' || p.reverse) {
          pads_back = gen_pads(
            nets,
            p.space, p.rot, "B",
            p.label_at_bottom, p.mirror, p.swap_pad_direction,
          );
        }

        const fp = `
          (module smd_805 (layer F.Cu) (tedit 6446BF3D)
            ${p.at /* parametric position */}
            (attr smd)

            (fp_text reference "${p.ref}" (at 0 3) (layer F.SilkS) ${p.ref_hide}
              (effects (font (size 1 1) (thickness 0.15)))
            )
            ${pads_front}
            ${pads_back}
          )
        `

        return fp;
    }
}

;module.exports = ((original) => {
    const defaults = {"component_1_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/SMD_0805_Resistor.step","component_2_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/SMD_0805_Resistor.step","component_3_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/SMD_0805_Resistor.step","component_4_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/SMD_0805_Resistor.step","component_5_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/SMD_0805_Resistor.step","component_6_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/SMD_0805_Resistor.step"};
    const bossVariant = null;
    const chocV2 = "${KIPRJMOD}/models/boardstudio/koktoh/Choc_V2_Red.step";
    return {
      ...original,
      params: {...original.params, ...defaults},
      body: p => {
        // The bundled extension crosses the PCB with a 5 mm outer diameter.
        const extensionDiameter = 5;
        if (false && p.drill < extensionDiameter
          && p.tp_extension_3dmodel_filename === defaults.tp_extension_3dmodel_filename
          && !p.tp_extension_3dmodel_xyz_scale
          && !p.tp_extension_3dmodel_xyz_rotation
          && !p.tp_extension_3dmodel_xyz_offset) {
          throw new Error('The bundled trackpoint extension requires a center drill of at least 5 mm. Select a compatible extension model or drill size.');
        }
        if (false && !p.choc_v1_support) {
          p = {...p};
          if (!p.choc_v2_support) {
            for (const key of Object.keys(defaults)) {
              if (p[key] === defaults[key]) { p[key] = ''; }
            }
          } else {
            if (p.switch_3dmodel_filename === defaults.switch_3dmodel_filename) {
              const automatic = !p.switch_3dmodel_xyz_rotation && !p.switch_3dmodel_xyz_offset
                && p.switch_3dmodel_xyz_scale.every(value => value === 1);
              const postDiameter = 4.8;
              if (automatic && (!p.include_stabilizer_pad || p.oval_stabilizer_pad
                || (p.center_hole_diameter > 0 && p.center_hole_diameter < postDiameter))) {
                throw new Error('The bundled Choc V2 model requires the round stabilizer hole and a center drill of at least 4.8 mm. Use a matching model or transform for modified hardware.');
              }
              p.switch_3dmodel_filename = chocV2;
              p.switch_3dmodel_xyz_rotation ||= p.side === 'F' ? [180, 0, 0] : [0, 180, 0];
              p.switch_3dmodel_xyz_offset ||= [0, 0, -p.pcb_thickness];
            }
            if (p.keycap_3dmodel_filename === defaults.keycap_3dmodel_filename) {
              p.keycap_3dmodel_filename = '';
            }
          }
        }
        if (bossVariant && p.include_bosses && p.reset_switch_3dmodel_filename === defaults.reset_switch_3dmodel_filename) {
          p = {...p, reset_switch_3dmodel_filename: bossVariant};
        }
        return original.body(p);
      }
    };
  })(module.exports);

return module.exports; })(),
  "infused-kim/switch_power": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Author: @infused-kim
//
// Description:
// Power switch for wireless boards.
//
// Should be compatible with:
//  - G-Switch MK-12C02-G015
//  - Alps SSSS811101
//  - PCM12SMTR

module.exports = {
    params: {
      designator: 'SW',
      side: 'F',
      reverse: false,
      from: {type: 'net', value: 'BAT_P'},
      to: {type: 'net', value: 'RAW'},

      switch_3dmodel_filename: '${EG_INFUSED_KIM_3D_MODELS}/Switch_Power.step',
      switch_3dmodel_side: '',
      switch_3dmodel_xyz_scale: '',
      switch_3dmodel_xyz_rotation: '',
      switch_3dmodel_xyz_offset: '',
    },
    body: p => {

      const gen_3d_model = (filename, scale, rotation, offset, side, {
        default_side =  'F',
        scale_f =       [1, 1, 1],
        rotation_f =    [0, 0, 0],
        offset_f =      [0, 0, 0],
        scale_b =       [1, 1, 1],
        rotation_b =    [0, 0, 0],
        offset_b =      [0, 0, 0]
      } = {}) => {

        if(filename == '') {
          return '';
        }

        const get_3d_model_side = (side, default_side) => {

            if(side == '') {
                if(p.reverse == true) {
                    side = default_side;
                } else {
                    side = p.side;
                }
            }

            if(side == 'F' || side == 'B') {
                return side;
            } else {
                return default_side;
            }
        }

        const final_side = get_3d_model_side(side, default_side, p);
        const is_front = final_side === 'F';

        // Determine the actual values to use
        const final_scale = scale || (is_front ? scale_f : scale_b);
        const final_rotation = rotation || (is_front ? rotation_f : rotation_b);
        let final_offset = offset || (is_front ? offset_f : offset_b);

        // Fix bug that seems to happen during the upgrade from KiCad 5 to
        // 8. All offset values seem to be multiplied by 25.4. So here we
        // divide them so that the upgrade KiCad file ends up with the
        // correct value.
        const offset_divisor = 25.4;
        final_offset = final_offset.map(value => value / offset_divisor);

        return  `
          (model ${JSON.stringify(filename)}
            (at (xyz ${final_offset[0]} ${final_offset[1]} ${final_offset[2]}))
            (scale (xyz ${final_scale[0]} ${final_scale[1]} ${final_scale[2]}))
            (rotate (xyz ${final_rotation[0]} ${final_rotation[1]} ${final_rotation[2]}))
          )
        `;
      };

      const shared_1 = `
        (module power_switch (layer F.Cu) (tedit 644556E6)
          ${p.at /* parametric position */}
          (attr smd)

      `;

      const front_switch = `
          (fp_text reference "${p.ref}" (at -3.6 0 ${-90 + p.rot}) (layer F.SilkS) ${p.ref_hide}
            (effects (font (size 1 1) (thickness 0.15)))
          )

          (fp_line (start 0.415 -3.45) (end -0.375 -3.45) (layer F.SilkS) (width 0.12))
          (fp_line (start -0.375 3.45) (end 0.415 3.45) (layer F.SilkS) (width 0.12))
          (fp_line (start -1.425 1.6) (end -1.425 -0.1) (layer F.SilkS) (width 0.12))
          (fp_line (start 1.425 2.85) (end 1.425 -2.85) (layer F.SilkS) (width 0.12))
          (fp_line (start 1.795 4.4) (end -2.755 4.4) (layer F.CrtYd) (width 0.05))
          (fp_line (start 1.795 1.65) (end 1.795 4.4) (layer F.CrtYd) (width 0.05))
          (fp_line (start 3.095 1.65) (end 1.795 1.65) (layer F.CrtYd) (width 0.05))
          (fp_line (start 3.095 -1.65) (end 3.095 1.65) (layer F.CrtYd) (width 0.05))
          (fp_line (start 1.795 -1.65) (end 3.095 -1.65) (layer F.CrtYd) (width 0.05))
          (fp_line (start 1.795 -4.4) (end 1.795 -1.65) (layer F.CrtYd) (width 0.05))
          (fp_line (start -2.755 -4.4) (end 1.795 -4.4) (layer F.CrtYd) (width 0.05))
          (fp_line (start -2.755 4.4) (end -2.755 -4.4) (layer F.CrtYd) (width 0.05))
          (fp_line (start -1.425 -1.4) (end -1.425 -1.6) (layer F.SilkS) (width 0.12))
          (fp_line (start -1.305 -3.35) (end -1.305 3.35) (layer F.Fab) (width 0.1))
          (fp_line (start 1.295 -3.35) (end -1.305 -3.35) (layer F.Fab) (width 0.1))
          (fp_line (start 1.295 3.35) (end 1.295 -3.35) (layer F.Fab) (width 0.1))
          (fp_line (start -1.305 3.35) (end 1.295 3.35) (layer F.Fab) (width 0.1))
          (fp_line (start 2.595 0.1) (end 1.295 0.1) (layer F.Fab) (width 0.1))
          (fp_line (start 2.645 0.15) (end 2.595 0.1) (layer F.Fab) (width 0.1))
          (fp_line (start 2.845 0.35) (end 2.645 0.15) (layer F.Fab) (width 0.1))
          (fp_line (start 2.845 1.2) (end 2.845 0.35) (layer F.Fab) (width 0.1))
          (fp_line (start 2.645 1.4) (end 2.845 1.2) (layer F.Fab) (width 0.1))
          (fp_line (start 1.345 1.4) (end 2.645 1.4) (layer F.Fab) (width 0.1))

          (pad "" smd rect (at 1.125 -3.65 ${90 + p.rot}) (size 1 0.8) (layers F.Cu F.Paste F.Mask))
          (pad "" smd rect (at -1.085 -3.65 ${90 + p.rot}) (size 1 0.8) (layers F.Cu F.Paste F.Mask))
          (pad "" smd rect (at -1.085 3.65 ${90 + p.rot}) (size 1 0.8) (layers F.Cu F.Paste F.Mask))
          (pad 1 smd rect (at -1.735 2.25 ${90 + p.rot}) (size 0.7 1.5) (layers F.Cu F.Paste F.Mask))
          (pad 2 smd rect (at -1.735 -0.75 ${90 + p.rot}) (size 0.7 1.5) (layers F.Cu F.Paste F.Mask) ${p.from.str})
          (pad 3 smd rect (at -1.735 -2.25 ${90 + p.rot}) (size 0.7 1.5) (layers F.Cu F.Paste F.Mask) ${p.to.str})
          (pad "" smd rect (at 1.125 3.65 ${90 + p.rot}) (size 1 0.8) (layers F.Cu F.Paste F.Mask))

      `
      const back_switch = `
        ${'' /* Add the optional parts here */}
        (fp_text user "${p.ref}" (at -3.5 0 ${90 + p.rot}) (layer B.SilkS) ${p.ref_hide}
        (effects (font (size 1 1) (thickness 0.15)) (justify mirror))
        )
        (fp_line (start 2.595 -0.1) (end 1.295 -0.1) (layer B.Fab) (width 0.1))
        (fp_line (start -1.305 3.35) (end -1.305 -3.35) (layer B.Fab) (width 0.1))
        (fp_line (start 2.645 -0.15) (end 2.595 -0.1) (layer B.Fab) (width 0.1))
        (fp_line (start -1.425 1.4) (end -1.425 1.6) (layer B.SilkS) (width 0.12))
        (fp_line (start 0.415 3.45) (end -0.375 3.45) (layer B.SilkS) (width 0.12))
        (fp_line (start -0.375 -3.45) (end 0.415 -3.45) (layer B.SilkS) (width 0.12))
        (fp_line (start -1.425 -1.6) (end -1.425 0.1) (layer B.SilkS) (width 0.12))
        (fp_line (start 1.425 -2.85) (end 1.425 2.85) (layer B.SilkS) (width 0.12))
        (fp_line (start 1.795 4.4) (end 1.795 1.65) (layer B.CrtYd) (width 0.05))
        (fp_line (start -2.755 4.4) (end 1.795 4.4) (layer B.CrtYd) (width 0.05))
        (fp_line (start 2.845 -1.2) (end 2.845 -0.35) (layer B.Fab) (width 0.1))
        (fp_line (start 1.345 -1.4) (end 2.645 -1.4) (layer B.Fab) (width 0.1))
        (fp_line (start 1.795 -4.4) (end -2.755 -4.4) (layer B.CrtYd) (width 0.05))
        (fp_line (start 1.795 -1.65) (end 1.795 -4.4) (layer B.CrtYd) (width 0.05))
        (fp_line (start 3.095 -1.65) (end 1.795 -1.65) (layer B.CrtYd) (width 0.05))
        (fp_line (start 2.845 -0.35) (end 2.645 -0.15) (layer B.Fab) (width 0.1))
        (fp_line (start 2.645 -1.4) (end 2.845 -1.2) (layer B.Fab) (width 0.1))
        (fp_line (start 1.295 -3.35) (end 1.295 3.35) (layer B.Fab) (width 0.1))
        (fp_line (start 1.295 3.35) (end -1.305 3.35) (layer B.Fab) (width 0.1))
        (fp_line (start -1.305 -3.35) (end 1.295 -3.35) (layer B.Fab) (width 0.1))
        (fp_line (start -2.755 -4.4) (end -2.755 4.4) (layer B.CrtYd) (width 0.05))
        (fp_line (start 3.095 1.65) (end 3.095 -1.65) (layer B.CrtYd) (width 0.05))
        (fp_line (start 1.795 1.65) (end 3.095 1.65) (layer B.CrtYd) (width 0.05))
        (pad "" smd rect (at -1.085 -3.65 ${270 + p.rot}) (size 1 0.8) (layers B.Cu B.Paste B.Mask))
        (pad "" smd rect (at 1.125 -3.65 ${270 + p.rot}) (size 1 0.8) (layers B.Cu B.Paste B.Mask))
        (pad 4 smd rect (at -1.735 2.25 ${270 + p.rot}) (size 0.7 1.5) (layers B.Cu B.Paste B.Mask))
        (pad "" smd rect (at -1.085 3.65 ${270 + p.rot}) (size 1 0.8) (layers B.Cu B.Paste B.Mask))
        (pad 5 smd rect (at -1.735 0.75 ${270 + p.rot}) (size 0.7 1.5) (layers B.Cu B.Paste B.Mask) ${p.from.str})
        (pad 6 smd rect (at -1.735 -2.25 ${270 + p.rot}) (size 0.7 1.5) (layers B.Cu B.Paste B.Mask) ${p.to.str})
        (pad "" smd rect (at 1.125 3.65 ${270 + p.rot}) (size 1 0.8) (layers B.Cu B.Paste B.Mask))
        `

        const shared_2 = `
          (pad "" np_thru_hole circle (at 0.025 -1.5 ${90 + p.rot}) (size 0.9 0.9) (drill 0.9) (layers *.Cu *.Mask))
          (pad "" np_thru_hole circle (at 0.025 1.5 ${90 + p.rot}) (size 0.9 0.9) (drill 0.9) (layers *.Cu *.Mask))
        `

        let final = shared_1;

        if(p.side == "F" || p.reverse) {
          final += front_switch;
        }
        if(p.side == "B" || p.reverse) {
          final += back_switch;
        }


        final += shared_2;

        final += gen_3d_model(
          p.switch_3dmodel_filename,
          p.switch_3dmodel_xyz_scale,
          p.switch_3dmodel_xyz_rotation,
          p.switch_3dmodel_xyz_offset,
          p.switch_3dmodel_side,
          {
            default_side: 'B',

            rotation_f: [-90, 0, -90],
            offset_f: [0, 0, 0],

            rotation_b: [90, 0, 90],
            offset_b: [0, 0, -1.6],
          },
        );

        final += `
          )
        `

        return final;
    }
  }
;module.exports = ((original) => {
    const defaults = {"switch_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/Switch_Power.step"};
    const bossVariant = null;
    const chocV2 = "${KIPRJMOD}/models/boardstudio/koktoh/Choc_V2_Red.step";
    return {
      ...original,
      params: {...original.params, ...defaults},
      body: p => {
        // The bundled extension crosses the PCB with a 5 mm outer diameter.
        const extensionDiameter = 5;
        if (false && p.drill < extensionDiameter
          && p.tp_extension_3dmodel_filename === defaults.tp_extension_3dmodel_filename
          && !p.tp_extension_3dmodel_xyz_scale
          && !p.tp_extension_3dmodel_xyz_rotation
          && !p.tp_extension_3dmodel_xyz_offset) {
          throw new Error('The bundled trackpoint extension requires a center drill of at least 5 mm. Select a compatible extension model or drill size.');
        }
        if (false && !p.choc_v1_support) {
          p = {...p};
          if (!p.choc_v2_support) {
            for (const key of Object.keys(defaults)) {
              if (p[key] === defaults[key]) { p[key] = ''; }
            }
          } else {
            if (p.switch_3dmodel_filename === defaults.switch_3dmodel_filename) {
              const automatic = !p.switch_3dmodel_xyz_rotation && !p.switch_3dmodel_xyz_offset
                && p.switch_3dmodel_xyz_scale.every(value => value === 1);
              const postDiameter = 4.8;
              if (automatic && (!p.include_stabilizer_pad || p.oval_stabilizer_pad
                || (p.center_hole_diameter > 0 && p.center_hole_diameter < postDiameter))) {
                throw new Error('The bundled Choc V2 model requires the round stabilizer hole and a center drill of at least 4.8 mm. Use a matching model or transform for modified hardware.');
              }
              p.switch_3dmodel_filename = chocV2;
              p.switch_3dmodel_xyz_rotation ||= p.side === 'F' ? [180, 0, 0] : [0, 180, 0];
              p.switch_3dmodel_xyz_offset ||= [0, 0, -p.pcb_thickness];
            }
            if (p.keycap_3dmodel_filename === defaults.keycap_3dmodel_filename) {
              p.keycap_3dmodel_filename = '';
            }
          }
        }
        if (bossVariant && p.include_bosses && p.reset_switch_3dmodel_filename === defaults.reset_switch_3dmodel_filename) {
          p = {...p, reset_switch_3dmodel_filename: bossVariant};
        }
        return original.body(p);
      }
    };
  })(module.exports);

return module.exports; })(),
  "infused-kim/switch_reset": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Author: @infused-kim
//
// Description:
// Reversible footprint for nice nano

// Should be compatible with:
// EVQ-P7A01P
//
// WARNING: This is not the same reset switch commonly used in the keyboard
// community. This switch faces sideways and is lower profile.

module.exports = {
    params: {
      designator: 'SW',
      side: 'F',
      reverse: false,
      from: {type: 'net', value: 'GND'},
      to: {type: 'net', value: 'RST'},

      switch_3dmodel_filename: '${EG_INFUSED_KIM_3D_MODELS}/Switch_Reset.step',
      switch_3dmodel_side: '',
      switch_3dmodel_xyz_scale: '',
      switch_3dmodel_xyz_rotation: '',
      switch_3dmodel_xyz_offset: '',
    },
    body: p => {

      const gen_3d_model = (filename, scale, rotation, offset, side, {
        default_side =  'F',
        scale_f =       [1, 1, 1],
        rotation_f =    [0, 0, 0],
        offset_f =      [0, 0, 0],
        scale_b =       [1, 1, 1],
        rotation_b =    [0, 0, 0],
        offset_b =      [0, 0, 0]
      } = {}) => {

        if(filename == '') {
          return '';
        }

        const get_3d_model_side = (side, default_side) => {

            if(side == '') {
                if(p.reverse == true) {
                    side = default_side;
                } else {
                    side = p.side;
                }
            }

            if(side == 'F' || side == 'B') {
                return side;
            } else {
                return default_side;
            }
        }

        const final_side = get_3d_model_side(side, default_side, p);
        const is_front = final_side === 'F';

        // Determine the actual values to use
        const final_scale = scale || (is_front ? scale_f : scale_b);
        const final_rotation = rotation || (is_front ? rotation_f : rotation_b);
        let final_offset = offset || (is_front ? offset_f : offset_b);

        // Fix bug that seems to happen during the upgrade from KiCad 5 to
        // 8. All offset values seem to be multiplied by 25.4. So here we
        // divide them so that the upgrade KiCad file ends up with the
        // correct value.
        const offset_divisor = 25.4;
        final_offset = final_offset.map(value => value / offset_divisor);

        return  `
          (model ${JSON.stringify(filename)}
            (at (xyz ${final_offset[0]} ${final_offset[1]} ${final_offset[2]}))
            (scale (xyz ${final_scale[0]} ${final_scale[1]} ${final_scale[2]}))
            (rotate (xyz ${final_rotation[0]} ${final_rotation[1]} ${final_rotation[2]}))
          )
        `;
      };

      const top = `
        (module sw_reset_side (layer F.Cu) (tedit 64473C6F)
          ${p.at /* parametric position */}
          (attr smd)

          (fp_text reference "${p.ref}" (at 0 3.5 ${p.rot}) (layer ${p.side}.SilkS) ${p.ref_hide}
            (effects (font (size 1 1) (thickness 0.15)))
          )
      `
      const front = `
          (fp_line (start 1.7 2.75) (end -1.7 2.75) (layer F.CrtYd) (width 0.05))
          (fp_line (start -1.7 2.75) (end -1.7 -2.75) (layer F.CrtYd) (width 0.05))
          (fp_line (start 2.1 0.85) (end 2.1 -0.85) (layer F.Fab) (width 0.1))
          (fp_line (start 1.7 -1.1) (end 2.35 -1.1) (layer F.CrtYd) (width 0.05))
          (fp_line (start -1.7 -2.75) (end 1.7 -2.75) (layer F.CrtYd) (width 0.05))
          (fp_line (start 1.45 -1.75) (end 1.45 1.75) (layer F.Fab) (width 0.1))
          (fp_line (start 1.7 1.1) (end 1.7 2.75) (layer F.CrtYd) (width 0.05))
          (fp_line (start 2.35 1.1) (end 1.7 1.1) (layer F.CrtYd) (width 0.05))
          (fp_line (start 1.7 -2.75) (end 1.7 -1.1) (layer F.CrtYd) (width 0.05))
          (fp_line (start 1.55 -1.75) (end 1.55 1.75) (layer F.SilkS) (width 0.12))
          (fp_line (start 2.1 -0.85) (end 1.45 -0.85) (layer F.Fab) (width 0.1))
          (fp_line (start 2.35 -1.1) (end 2.35 1.1) (layer F.CrtYd) (width 0.05))
          (fp_line (start 2.1 0.85) (end 1.45 0.85) (layer F.Fab) (width 0.1))
          (fp_line (start -1.55 1.75) (end -1.55 -1.75) (layer F.SilkS) (width 0.12))
          (fp_line (start 1.45 1.75) (end -1.4 1.75) (layer F.Fab) (width 0.1))
          (fp_line (start -1.45 1.75) (end -1.45 -1.75) (layer F.Fab) (width 0.1))
          (fp_line (start -1.45 -1.75) (end 1.45 -1.75) (layer F.Fab) (width 0.1))

          (pad 1 smd rect (at -0.72 -1.8 ${90 + p.rot}) (size 1.4 1.05) (layers F.Cu F.Paste F.Mask) ${p.from.str})

          (pad 1 smd rect (at -0.72 1.8 ${90 + p.rot}) (size 1.4 1.05) (layers F.Cu F.Paste F.Mask) ${p.from.str})
          (pad 2 smd rect (at 0.72 -1.8 ${90 + p.rot}) (size 1.4 1.05) (layers F.Cu F.Paste F.Mask) ${p.to.str})
          (pad 2 smd rect (at 0.72 1.8 ${90 + p.rot}) (size 1.4 1.05) (layers F.Cu F.Paste F.Mask) ${p.to.str})
      `
      const back = `
      (fp_line (start -1.45 1.75) (end 1.45 1.75) (layer B.Fab) (width 0.1))
      (fp_line (start 1.45 1.75) (end 1.45 -1.75) (layer B.Fab) (width 0.1))
      (fp_line (start 1.7 -1.1) (end 1.7 -2.75) (layer B.CrtYd) (width 0.05))
      (fp_line (start 2.35 -1.1) (end 1.7 -1.1) (layer B.CrtYd) (width 0.05))
      (fp_line (start 1.7 2.75) (end 1.7 1.1) (layer B.CrtYd) (width 0.05))
      (fp_line (start 1.55 1.75) (end 1.55 -1.75) (layer B.SilkS) (width 0.12))
      (fp_line (start 2.1 0.85) (end 1.45 0.85) (layer B.Fab) (width 0.1))
      (fp_line (start 2.35 1.1) (end 2.35 -1.1) (layer B.CrtYd) (width 0.05))
      (fp_line (start 2.1 -0.85) (end 1.45 -0.85) (layer B.Fab) (width 0.1))
      (fp_line (start -1.55 -1.75) (end -1.55 1.75) (layer B.SilkS) (width 0.12))
      (fp_line (start 1.45 -1.75) (end -1.4 -1.75) (layer B.Fab) (width 0.1))
      (fp_line (start -1.45 -1.75) (end -1.45 1.75) (layer B.Fab) (width 0.1))
      (fp_line (start 1.7 -2.75) (end -1.7 -2.75) (layer B.CrtYd) (width 0.05))
      (fp_line (start -1.7 -2.75) (end -1.7 2.75) (layer B.CrtYd) (width 0.05))
      (fp_line (start 2.1 -0.85) (end 2.1 0.85) (layer B.Fab) (width 0.1))
      (fp_line (start 1.7 1.1) (end 2.35 1.1) (layer B.CrtYd) (width 0.05))
      (fp_line (start -1.7 2.75) (end 1.7 2.75) (layer B.CrtYd) (width 0.05))
      (pad 1 smd rect (at -0.72 -1.8 ${270 + p.rot}) (size 1.4 1.05) (layers B.Cu B.Paste B.Mask) ${p.from.str})
      (pad 2 smd rect (at 0.72 1.8 ${270 + p.rot}) (size 1.4 1.05) (layers B.Cu B.Paste B.Mask) ${p.to.str})
      (pad 2 smd rect (at 0.72 -1.8 ${270 + p.rot}) (size 1.4 1.05) (layers B.Cu B.Paste B.Mask) ${p.to.str})
      (pad 1 smd rect (at -0.72 1.8 ${270 + p.rot}) (size 1.4 1.05) (layers B.Cu B.Paste B.Mask) ${p.from.str})
      (fp_text user ${p.ref} (at 0 3.5 ${p.rot}) (layer B.SilkS) ${p.ref_hide}
        (effects (font (size 1 1) (thickness 0.15)) (justify mirror))
      )
      `

      const bottom = `
        )
      `

      let final = top;

      if(p.side == "F" || p.reverse) {
        final += front;
      }
      if(p.side == "B" || p.reverse) {
        final += back;
      }

      final += gen_3d_model(
        p.switch_3dmodel_filename,
        p.switch_3dmodel_xyz_scale,
        p.switch_3dmodel_xyz_rotation,
        p.switch_3dmodel_xyz_offset,
        p.switch_3dmodel_side,
        {
          default_side: 'B',

          rotation_f: [-90, 0, -90],
          offset_f: [0, 0, 0],

          rotation_b: [90, 0, 90],
          offset_b: [0, 0, -1.6],
        },
      );

      final += bottom;

      return final;
    }
 }

;module.exports = ((original) => {
    const defaults = {"switch_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/Switch_Reset.step"};
    const bossVariant = null;
    const chocV2 = "${KIPRJMOD}/models/boardstudio/koktoh/Choc_V2_Red.step";
    return {
      ...original,
      params: {...original.params, ...defaults},
      body: p => {
        // The bundled extension crosses the PCB with a 5 mm outer diameter.
        const extensionDiameter = 5;
        if (false && p.drill < extensionDiameter
          && p.tp_extension_3dmodel_filename === defaults.tp_extension_3dmodel_filename
          && !p.tp_extension_3dmodel_xyz_scale
          && !p.tp_extension_3dmodel_xyz_rotation
          && !p.tp_extension_3dmodel_xyz_offset) {
          throw new Error('The bundled trackpoint extension requires a center drill of at least 5 mm. Select a compatible extension model or drill size.');
        }
        if (false && !p.choc_v1_support) {
          p = {...p};
          if (!p.choc_v2_support) {
            for (const key of Object.keys(defaults)) {
              if (p[key] === defaults[key]) { p[key] = ''; }
            }
          } else {
            if (p.switch_3dmodel_filename === defaults.switch_3dmodel_filename) {
              const automatic = !p.switch_3dmodel_xyz_rotation && !p.switch_3dmodel_xyz_offset
                && p.switch_3dmodel_xyz_scale.every(value => value === 1);
              const postDiameter = 4.8;
              if (automatic && (!p.include_stabilizer_pad || p.oval_stabilizer_pad
                || (p.center_hole_diameter > 0 && p.center_hole_diameter < postDiameter))) {
                throw new Error('The bundled Choc V2 model requires the round stabilizer hole and a center drill of at least 4.8 mm. Use a matching model or transform for modified hardware.');
              }
              p.switch_3dmodel_filename = chocV2;
              p.switch_3dmodel_xyz_rotation ||= p.side === 'F' ? [180, 0, 0] : [0, 180, 0];
              p.switch_3dmodel_xyz_offset ||= [0, 0, -p.pcb_thickness];
            }
            if (p.keycap_3dmodel_filename === defaults.keycap_3dmodel_filename) {
              p.keycap_3dmodel_filename = '';
            }
          }
        }
        if (bossVariant && p.include_bosses && p.reset_switch_3dmodel_filename === defaults.reset_switch_3dmodel_filename) {
          p = {...p, reset_switch_3dmodel_filename: bossVariant};
        }
        return original.body(p);
      }
    };
  })(module.exports);

return module.exports; })(),
  "infused-kim/text": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Author: @infused-kim
//
// Description:
// Allows oyu to place text on the PCB.

module.exports = {
  params: {
    designator: 'TXT',
    side: 'F',
    reverse: false,
    text: 'Awesomeness',
  },
  body: p => {
    const front = `
      (gr_text "${p.text}" ${p.at} (layer F.SilkS)
        (effects (font (size 1 1) (thickness 0.15)))
      )
    `
    const back = `
      (gr_text "${p.text}" ${p.at} (layer B.SilkS)
        (effects (font (size 1 1) (thickness 0.15)) (justify mirror))
      )
    `

    let final = '';

    if(p.side == "F" || p.reverse) {
      final += front;
    }
    if(p.side == "B" || p.reverse) {
      final += back;
    }

    return final;
  }
}

return module.exports; })(),
  "infused-kim/trackpoint_mount": (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;
// Author: @infused-kim
//
// Description:
// Adds mounting holes for a trackpoint to the PCB.
//
// Should be compatible with:
//  - Thinkpad T430
//  - Thinkpad T440 / X240
//
// Check this page for other models:
// https://deskthority.net/wiki/TrackPoint_Hardware

module.exports = {
  params: {
    designator: 'TP',
    side: 'B',
    reverse: false,

    // T430: 3.5
    // T460S (red one): 3.5
    // X240: 5.5
    drill: 5.5,
    outline: 0.25,

    show_outline_t430: false,
    show_outline_x240: false,
    show_outline_t460s: false,
    show_board: false,

      // This side parameter applies to all 3d models
      tp_3dmodel_side: '',

      tp_cap_3dmodel_filename: '${EG_INFUSED_KIM_3D_MODELS}/trackpoint/TP_Cap_Red_T460S.step',
      tp_cap_3dmodel_xyz_scale: '',
      tp_cap_3dmodel_xyz_rotation: '',
      tp_cap_3dmodel_xyz_offset: '',

      tp_extension_3dmodel_filename: '${EG_INFUSED_KIM_3D_MODELS}/trackpoint/TP_Extension_Red_T460S_h10.5_md0.0_pcb1.6.step',
      tp_extension_3dmodel_xyz_scale: '',
      tp_extension_3dmodel_xyz_rotation: '',
      tp_extension_3dmodel_xyz_offset: '',

      tp_3dmodel_filename: '${EG_INFUSED_KIM_3D_MODELS}/trackpoint/TP_Red_T460S_platform_z_offset_+0.0_pcb_offset_-2.0.step',
      tp_3dmodel_xyz_scale: '',
      tp_3dmodel_xyz_rotation: '',
      tp_3dmodel_xyz_offset: '',
  },
  body: p => {

    const gen_3d_model = (filename, scale, rotation, offset, side, {
      default_side =  'F',
      scale_f =       [1, 1, 1],
      rotation_f =    [0, 0, 0],
      offset_f =      [0, 0, 0],
      scale_b =       [1, 1, 1],
      rotation_b =    [0, 0, 0],
      offset_b =      [0, 0, 0]
    } = {}) => {

      if(filename == '') {
        return '';
      }

      const get_3d_model_side = (side, default_side) => {

          if(side == '') {
              if(p.reverse == true) {
                  side = default_side;
              } else {
                  side = p.side;
              }
          }

          if(side == 'F' || side == 'B') {
              return side;
          } else {
              return default_side;
          }
      }

      const final_side = get_3d_model_side(side, default_side, p);
      const is_front = final_side === 'F';

      // Determine the actual values to use
      const final_scale = scale || (is_front ? scale_f : scale_b);
      const final_rotation = rotation || (is_front ? rotation_f : rotation_b);
      let final_offset = offset || (is_front ? offset_f : offset_b);

      // Fix bug that seems to happen during the upgrade from KiCad 5 to
      // 8. All offset values seem to be multiplied by 25.4. So here we
      // divide them so that the upgrade KiCad file ends up with the
      // correct value.
      const offset_divisor = 25.4;
      final_offset = final_offset.map(value => value / offset_divisor);

      return  `
        (model ${JSON.stringify(filename)}
          (at (xyz ${final_offset[0]} ${final_offset[1]} ${final_offset[2]}))
          (scale (xyz ${final_scale[0]} ${final_scale[1]} ${final_scale[2]}))
          (rotate (xyz ${final_rotation[0]} ${final_rotation[1]} ${final_rotation[2]}))
        )
      `;
    };

    const top = `
      (module trackpoint_mount_t430 (layer F.Cu) (tedit 6449FFC5)
        ${p.at /* parametric position */}
        (attr virtual)

        (fp_text reference "${p.ref}" (at 0 0) (layer ${p.side}.SilkS) ${p.ref_hide}
          (effects (font (size 1 1) (thickness 0.15)))
        )
    `;

    const front = `
        (fp_circle (center 0 -9.75) (end -2.15 -9.75) (layer F.CrtYd) (width 0.05))
        (fp_circle (center 0 -9.75) (end -1.9 -9.75) (layer Cmts.User) (width 0.15))
        (fp_circle (center 0 9.75) (end -2.15 9.75) (layer F.CrtYd) (width 0.05))
        (fp_circle (center 0 9.75) (end -1.9 9.75) (layer Cmts.User) (width 0.15))
        (fp_circle (center 0 0) (end -3.95 0) (layer F.CrtYd) (width 0.05))
        (fp_circle (center 0 0) (end -3.7 0) (layer Cmts.User) (width 0.15))

        (fp_text user %R (at 0 0 180) (layer F.Fab)
          (effects (font (size 1 1) (thickness 0.15)))
        )
    `
    const back = `
        (fp_circle (center 0 0) (end -3.95 0) (layer B.CrtYd) (width 0.05))
        (fp_circle (center 0 0) (end -3.7 0) (layer Cmts.User) (width 0.15))
        (fp_circle (center 0 9.75) (end -2.15 9.75) (layer B.CrtYd) (width 0.05))
        (fp_circle (center 0 -9.75) (end -2.15 -9.75) (layer B.CrtYd) (width 0.05))
    `

    const outline_t430_front = `
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
    `

    const outline_t430_back = `
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
    `

    const outline_x240_front = `
        (fp_line (start 12.25 -6.5) (end 6.75 -6.5) (layer F.Fab) (width 0.2))
        (fp_line (start 12.25 6.5) (end 6.75 6.5) (layer F.Fab) (width 0.2))
        (fp_line (start 12.25 6.5) (end 12.25 -6.5) (layer F.Fab) (width 0.2))
        (fp_line (start 6.75 11.5) (end -6.75 11.5) (layer F.Fab) (width 0.2))
        (fp_line (start 6.75 -11.5) (end -6.75 -11.5) (layer F.Fab) (width 0.2))
        (fp_line (start -6.75 11.5) (end -6.75 -11.5) (layer F.Fab) (width 0.2))
        (fp_line (start 6.75 11.5) (end 6.75 -11.5) (layer F.Fab) (width 0.2))
    `

    const outline_x240_back = `
        (fp_line (start 12.25 -6.5) (end 6.75 -6.5) (layer B.Fab) (width 0.2))
        (fp_line (start 12.25 -6.5) (end 12.25 6.5) (layer B.Fab) (width 0.2))
        (fp_line (start 6.75 -11.5) (end -6.75 -11.5) (layer B.Fab) (width 0.2))
        (fp_line (start 6.75 11.5) (end -6.75 11.5) (layer B.Fab) (width 0.2))
        (fp_line (start -6.75 -11.5) (end -6.75 11.5) (layer B.Fab) (width 0.2))
        (fp_line (start 6.75 -11.5) (end 6.75 11.5) (layer B.Fab) (width 0.2))
        (fp_line (start 12.25 6.5) (end 6.75 6.5) (layer B.Fab) (width 0.2))
    `

    const outline_x240_board = `
        (fp_line (start 39.25 12) (end 23.25 12) (layer Dwgs.User) (width 0.2))
        (fp_line (start 23.25 5.5) (end 23.25 12) (layer Dwgs.User) (width 0.2))
        (fp_line (start 23.25 -5.5) (end 23.25 5.5) (layer Dwgs.User) (width 0.2))
        (fp_line (start 23.25 5.5) (end 12.25 5.5) (layer Dwgs.User) (width 0.2))
        (fp_line (start 23.25 -5.5) (end 12.25 -5.5) (layer Dwgs.User) (width 0.2))
        (fp_line (start 39.25 -22) (end 39.25 12) (layer Dwgs.User) (width 0.2))
        (fp_line (start 39.25 -22) (end 23.25 -22) (layer Dwgs.User) (width 0.2))
        (fp_line (start 23.25 -22) (end 23.25 -5.5) (layer Dwgs.User) (width 0.2))
        (fp_line (start 12.25 -5.5) (end 12.25 5.5) (layer Dwgs.User) (width 0.2))
    `

    const outline_t460s_front = `
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
    `

    const outline_t460s_back = `
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
    `

    const outline_t460s_board = `
        (fp_line (start 38.25 12.25) (end 22.25 12.25) (layer Dwgs.User) (width 0.2))
        (fp_line (start 22.25 2.75) (end 22.25 12.25) (layer Dwgs.User) (width 0.2))
        (fp_line (start 22.25 -2.75) (end 22.25 2.75) (layer Dwgs.User) (width 0.2))
        (fp_line (start 22.25 2.75) (end 6.25 2.75) (layer Dwgs.User) (width 0.2))
        (fp_line (start 22.25 -2.75) (end 6.25 -2.75) (layer Dwgs.User) (width 0.2))
        (fp_line (start 38.25 -22.25) (end 38.25 12.25) (layer Dwgs.User) (width 0.2))
        (fp_line (start 38.25 -22.25) (end 22.25 -22.25) (layer Dwgs.User) (width 0.2))
        (fp_line (start 22.25 -22.25) (end 22.25 -2.75) (layer Dwgs.User) (width 0.2))
        (fp_line (start 6.25 -2.75) (end 6.25 2.75) (layer Dwgs.User) (width 0.2))
    `

    const size = p.drill + (p.outline * 2)
    const bottom = `
        (pad "" thru_hole circle (at 0 -9.75 180) (size 3.8 3.8) (drill 2.2) (layers *.Cu *.Mask))
        (pad 1 np_thru_hole circle (at 0 0 180) (size ${size} ${size}) (drill ${p.drill}) (layers *.Cu *.Mask))
        (pad "" thru_hole circle (at 0 9.75 180) (size 3.8 3.8) (drill 2.2) (layers *.Cu *.Mask))
      )
    `

    let final = top;

    if(p.side == "F" || p.reverse) {
      final += front;

      if(p.show_outline_t430) {
        final += outline_t430_front;
      }
      if(p.show_outline_x240) {
        final += outline_x240_front;
      }
      if(p.show_outline_t460s) {
        final += outline_t460s_front;
      }
    }

    if(p.side == "B" || p.reverse) {
      final += back;
      if(p.show_outline_t430) {
        final += outline_t430_back;
      }
      if(p.show_outline_x240) {
        final += outline_x240_back;
      }
      if(p.show_outline_t460s) {
        final += outline_t460s_back;
      }
    }

    if(p.show_board) {
      if(p.show_outline_x240) {
        final += outline_x240_board
      }
      if(p.show_outline_t460s) {
        final += outline_t460s_board
      }
    }

    final += `
      ${ gen_3d_model(
              p.tp_cap_3dmodel_filename,
              p.tp_cap_3dmodel_xyz_scale,
              p.tp_cap_3dmodel_xyz_rotation,
              p.tp_cap_3dmodel_xyz_offset,
              p.tp_3dmodel_side,
              {
                rotation_f: [0, 0, 0],
                offset_f: [0, 0, 10.5],

                rotation_b: [0, 180, 0],
                offset_b: [0, 0, -(10.5+1.6)],
              },
          )
        }

      ${ gen_3d_model(
              p.tp_extension_3dmodel_filename,
              p.tp_extension_3dmodel_xyz_scale,
              p.tp_extension_3dmodel_xyz_rotation,
              p.tp_extension_3dmodel_xyz_offset,
              p.tp_3dmodel_side,
              {
                rotation_f: [0, 0, 0],
                offset_f: [0, 0, 0],

                rotation_b: [0, 180, 0],
                offset_b: [0, 0, -1.6],
              },
          )
        }

      ${ gen_3d_model(
              p.tp_3dmodel_filename,
              p.tp_3dmodel_xyz_scale,
              p.tp_3dmodel_xyz_rotation,
              p.tp_3dmodel_xyz_offset,
              p.tp_3dmodel_side,
              {
                rotation_f: [0, 0, 180],
                offset_f: [0, 0, 0],

                rotation_b: [0, 0, 0],
                offset_b: [0, 0, 0],
              },
          )
        }
    `;

    final += bottom;

    return final;
  }
}

;module.exports = ((original) => {
    const defaults = {"tp_cap_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/trackpoint/TP_Cap_Red_T460S.step","tp_extension_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/trackpoint/TP_Extension_Red_T460S_h10.5_md0.0_pcb1.6.step","tp_3dmodel_filename":"${KIPRJMOD}/models/boardstudio/infused-kim/trackpoint/TP_Red_T460S_platform_z_offset_+0.0_pcb_offset_-2.0.step"};
    const bossVariant = null;
    const chocV2 = "${KIPRJMOD}/models/boardstudio/koktoh/Choc_V2_Red.step";
    return {
      ...original,
      params: {...original.params, ...defaults},
      body: p => {
        // The bundled extension crosses the PCB with a 5 mm outer diameter.
        const extensionDiameter = 5;
        if (true && p.drill < extensionDiameter
          && p.tp_extension_3dmodel_filename === defaults.tp_extension_3dmodel_filename
          && !p.tp_extension_3dmodel_xyz_scale
          && !p.tp_extension_3dmodel_xyz_rotation
          && !p.tp_extension_3dmodel_xyz_offset) {
          throw new Error('The bundled trackpoint extension requires a center drill of at least 5 mm. Select a compatible extension model or drill size.');
        }
        if (false && !p.choc_v1_support) {
          p = {...p};
          if (!p.choc_v2_support) {
            for (const key of Object.keys(defaults)) {
              if (p[key] === defaults[key]) { p[key] = ''; }
            }
          } else {
            if (p.switch_3dmodel_filename === defaults.switch_3dmodel_filename) {
              const automatic = !p.switch_3dmodel_xyz_rotation && !p.switch_3dmodel_xyz_offset
                && p.switch_3dmodel_xyz_scale.every(value => value === 1);
              const postDiameter = 4.8;
              if (automatic && (!p.include_stabilizer_pad || p.oval_stabilizer_pad
                || (p.center_hole_diameter > 0 && p.center_hole_diameter < postDiameter))) {
                throw new Error('The bundled Choc V2 model requires the round stabilizer hole and a center drill of at least 4.8 mm. Use a matching model or transform for modified hardware.');
              }
              p.switch_3dmodel_filename = chocV2;
              p.switch_3dmodel_xyz_rotation ||= p.side === 'F' ? [180, 0, 0] : [0, 180, 0];
              p.switch_3dmodel_xyz_offset ||= [0, 0, -p.pcb_thickness];
            }
            if (p.keycap_3dmodel_filename === defaults.keycap_3dmodel_filename) {
              p.keycap_3dmodel_filename = '';
            }
          }
        }
        if (bossVariant && p.include_bosses && p.reset_switch_3dmodel_filename === defaults.reset_switch_3dmodel_filename) {
          p = {...p, reset_switch_3dmodel_filename: bossVariant};
        }
        return original.body(p);
      }
    };
  })(module.exports);

return module.exports; })(),
};

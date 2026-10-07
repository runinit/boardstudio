# Licences and attribution

The crate is a mix of licences. The framework (everything outside
`src/generators/`) is MIT OR Apache-2.0, like the other BoardStudio crates. Each
file under `src/generators/` is a Rust port of a footprint generator and keeps the
licence and attribution of its source:

- **MIT** (13 generators): `ceoloide/ergogen-footprints` files marked MIT. The
  licence text is in [licenses/MIT-ceoloide-ergogen-footprints.txt](licenses/MIT-ceoloide-ergogen-footprints.txt).
- **CC-BY-NC-SA-4.0** (23 generators): `ceoloide/ergogen-footprints` files marked
  CC-BY-NC-SA-4.0 and every `infused-kim/kb_ergogen_fp` file. These are
  NonCommercial and ShareAlike: a build that includes them is not licensed for
  commercial use, and adaptations (these ports) must be shared under the same
  licence. The licence text is in
  [licenses/CC-BY-NC-SA-4.0-infused-kim.txt](licenses/CC-BY-NC-SA-4.0-infused-kim.txt).

The crate's `Cargo.toml` expression describes the whole crate; individual files are
marked with their own `SPDX-License-Identifier`.

Upstream sources:

- <https://github.com/ceoloide/ergogen-footprints>, commit
  `48935f54b456ff1503d78d6b17d9d146b54e8ade`
- <https://github.com/infused-kim/kb_ergogen_fp>, commit
  `bb80a207d8a6fa7b9245caad2c2d97e2adc2f612`

A test (`tests/licenses.rs`) checks that every generator's declared licence matches
the SPDX identifier in its file header, is one of the two above, and is listed here.

| Generator | Licence | Authors |
| --- | --- | --- |
| `ceoloide/battery_connector_jst_ph_2` | MIT | @ceoloide |
| `ceoloide/battery_connector_molex_pico_ezmate_1x02` | CC-BY-NC-SA-4.0 | @infused-kim + @ceoloide improvements |
| `ceoloide/diode_tht_sod123` | CC-BY-NC-SA-4.0 | @ergogen + (@infused-kim, @ceoloide, @achamian, @im-AMS improvements) |
| `ceoloide/display_nice_view` | CC-BY-NC-SA-4.0 | @infused-kim + @ceoloide improvements |
| `ceoloide/display_ssd1306` | CC-BY-NC-SA-4.0 | @ceoloide |
| `ceoloide/led_sk6812mini-e` | MIT | @ceoloide |
| `ceoloide/mcu_nice_nano` | CC-BY-NC-SA-4.0 | @infused-kim + @ceoloide improvements |
| `ceoloide/mcu_supermini_nrf52840` | CC-BY-NC-SA-4.0 | @infused-kim + @ceoloide improvements |
| `ceoloide/mounting_hole_npth` | MIT | @ceoloide |
| `ceoloide/mounting_hole_plated` | CC-BY-NC-SA-4.0 | @infused-kim + @ceoloide improvements |
| `ceoloide/power_switch_smd_side` | CC-BY-NC-SA-4.0 | @infused-kim + @ceoloide improvements |
| `ceoloide/reset_switch_smd_side` | MIT | @ceoloide |
| `ceoloide/reset_switch_tht_top` | MIT | @ceoloide |
| `ceoloide/rotary_encoder_ec11_ec12` | MIT | @ceoloide |
| `ceoloide/switch_choc_v1_v2` | CC-BY-NC-SA-4.0 | @ergogen + @infused-kim, @ceoloide, @grazfather, @nxtk improvements |
| `ceoloide/switch_gateron_ks27_ks33` | MIT | @nxtk |
| `ceoloide/switch_mx` | MIT | @ceoloide |
| `ceoloide/trrs_pj320a` | MIT | @ergogen + @ceoloide improvements |
| `ceoloide/utility_filled_zone` | MIT | @ceoloide |
| `ceoloide/utility_keepout_zone` | MIT | @ceoloide |
| `ceoloide/utility_logo` | MIT | @dieseltravis + @ceoloide improvements |
| `ceoloide/utility_point_debugger` | CC-BY-NC-SA-4.0 | @infused-kim + @ceoloide improvements |
| `ceoloide/utility_router` | MIT | @yanshay + @ceoloide improvements |
| `ceoloide/utility_text` | CC-BY-NC-SA-4.0 | @infused-kim + @ceoloide & @dieseltravis improvements |
| `infused-kim/conn_molex_pico_ezmate_1x02` | CC-BY-NC-SA-4.0 | @infused-kim |
| `infused-kim/conn_molex_pico_ezmate_1x05` | CC-BY-NC-SA-4.0 | @infused-kim |
| `infused-kim/icon_bat` | CC-BY-NC-SA-4.0 | @infused-kim |
| `infused-kim/mounting_hole` | CC-BY-NC-SA-4.0 | @infused-kim |
| `infused-kim/nice_view` | CC-BY-NC-SA-4.0 | @infused-kim |
| `infused-kim/pads` | CC-BY-NC-SA-4.0 | @infused-kim |
| `infused-kim/point_debugger` | CC-BY-NC-SA-4.0 | @infused-kim |
| `infused-kim/smd_0805` | CC-BY-NC-SA-4.0 | @infused-kim |
| `infused-kim/switch_power` | CC-BY-NC-SA-4.0 | @infused-kim |
| `infused-kim/switch_reset` | CC-BY-NC-SA-4.0 | @infused-kim |
| `infused-kim/text` | CC-BY-NC-SA-4.0 | @infused-kim |
| `infused-kim/trackpoint_mount` | CC-BY-NC-SA-4.0 | @infused-kim |

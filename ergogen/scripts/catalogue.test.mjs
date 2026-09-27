import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
import catalogue from '../generated/catalogue.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const library = resolve(root, 'library');
const expected = [
  'ceoloide/battery_connector_jst_ph_2',
  'ceoloide/battery_connector_molex_pico_ezmate_1x02',
  'ceoloide/diode_tht_sod123',
  'ceoloide/display_nice_view',
  'ceoloide/display_ssd1306',
  'ceoloide/led_sk6812mini-e',
  'ceoloide/mcu_nice_nano',
  'ceoloide/mcu_supermini_nrf52840',
  'ceoloide/mounting_hole_npth',
  'ceoloide/mounting_hole_plated',
  'ceoloide/power_switch_smd_side',
  'ceoloide/reset_switch_smd_side',
  'ceoloide/reset_switch_tht_top',
  'ceoloide/rotary_encoder_ec11_ec12',
  'ceoloide/switch_choc_v1_v2',
  'ceoloide/switch_gateron_ks27_ks33',
  'ceoloide/switch_mx',
  'ceoloide/trrs_pj320a',
  'ceoloide/utility_ergogen_logo',
  'ceoloide/utility_filled_zone',
  'ceoloide/utility_keepout_zone',
  'ceoloide/utility_point_debugger',
  'ceoloide/utility_router',
  'ceoloide/utility_text',
  'infused-kim/conn_molex_pico_ezmate_1x02',
  'infused-kim/conn_molex_pico_ezmate_1x05',
  'infused-kim/icon_bat',
  'infused-kim/mounting_hole',
  'infused-kim/nice_view',
  'infused-kim/pads',
  'infused-kim/point_debugger',
  'infused-kim/smd_0805',
  'infused-kim/switch_power',
  'infused-kim/switch_reset',
  'infused-kim/text',
  'infused-kim/trackpoint_mount',
];

assert.deepEqual(Object.keys(catalogue).sort(), expected.slice().sort(), 'catalogue must contain every bundled generator');
for (const id of expected) {
  const [namespace, name] = id.split('/');
  const source = await readFile(resolve(library, namespace === 'ceoloide' ? `${name}.js` : `vendor/infused-kim/${name}.js`), 'utf8');
  const generator = catalogue[id];
  assert.equal(typeof generator.body, 'function', `${id} must expose a body`);
  assert.equal(typeof generator.params, 'object', `${id} must expose params`);
  assert.match(source, /module\.exports\s*=\s*\{/u, `${id} must remain a CommonJS generator`);
}

// Side and mirrored variants must remain represented by the migrated sources.
for (const id of ['ceoloide/switch_mx', 'ceoloide/switch_choc_v1_v2']) {
  const generator = catalogue[id];
  assert.ok('side' in generator.params || 'reverse' in generator.params, `${id} lost its side variant parameter`);
}

console.log(`Verified ${expected.length} generated Ergogen generators and side variants`);

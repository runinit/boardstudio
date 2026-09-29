import { expect, it } from 'vitest';
import { demoProject } from './demo';
import { createMechanicalConfiguration } from './mechanicalPresets';
import { mechanicalDefaults } from './mechanicalDefaults';
it('derives wireless battery clearance and only wide-key stabilizers without mutating the layout', () => {
  const doc = demoProject();
  doc.definitions[0].generator = { source: 'ceoloide/switch_mx', version: 'bundled-1', parameters: {} };
  doc.parts[0].keycap = { x: 18, y: 37.1 };
  doc.hardware = { topology: 'split', transport: 'wireless', boards: [], instances: [], sharedConstruction: null };
  const config = createMechanicalConfiguration(doc);
  const before = structuredClone(doc);
  const result = mechanicalDefaults(doc, config);
  expect(result.battery?.size.z).toBe(6);
  expect(result.batteryHeight).toBe(6);
  expect(result.stabilizers).toEqual([{ partId: doc.parts[0].id, kind: 'pcb-mount', units: 2, rotation: 90 }]);
  expect(doc).toEqual(before);
  expect(config.battery).toBeUndefined();
});

it.each(['gh60', 'discipline', 'reviung41'] as const)('does not duplicate imported stabilizers in %s', async id => {
  const { keyboardProject } = await import('./demos/keyboards');
  const doc = keyboardProject(id);
  const config = createMechanicalConfiguration(doc);
  expect(mechanicalDefaults(doc, config).stabilizers).toEqual([]);
});

import assert from 'node:assert/strict';
import { catalogue, geometry, normalizeDefinition, parameters, parseForms, render } from '../src/index.ts';

const definitions = catalogue();
assert.equal(definitions.length, 36);
assert.ok(!definitions.some(({ generator }) => generator?.source === 'infused-kim/nice_nano_pretty'), 'retired controller stays out of the catalogue');

for (const definition of definitions.filter(({ kind }) => kind !== 'utility')) {
  assert.ok(definition.courtyard.length >= 3, `${definition.id} must have a physical outline envelope`);
}

for (const source of ['ceoloide/switch_mx', 'ceoloide/switch_choc_v1_v2', 'ceoloide/switch_gateron_ks27_ks33']) {
  const definition = definitions.find(({ generator }) => generator?.source === source);
  assert.ok(definition?.keycap, `${source} must expose its keycap dimensions`);
  const keycapToggle = 'include_keycap';
  assert.equal(parameters(source)[keycapToggle].value, true, `${source} must show its keycap by default`);
  assert.ok(definition.terminals?.from?.length && definition.terminals?.to?.length, `${source} must expose its electrical terminal pad groups`);
  assert.ok(definition.matrixTerminals?.row === 'from' && definition.matrixTerminals?.column === 'to', `${source} must be matrix-ready`);
}

const part = (definition, side = 'front', properties = {}) => ({
  id: `test:${side}`,
  definitionId: definition.id,
  reference: 'U1',
  pose: { at: { x: 12, y: -7 }, rotation: 23 },
  side,
  properties,
});

for (const definition of definitions) {
  const front = render(definition, { part: part(definition), netIndex: (name) => name === 'GND' ? 7 : 3 });
  // Routing/text/zone utilities intentionally emit nothing until configured.
  const utility = definition.id.includes('/utility_') || definition.id.includes('/text') || definition.id.includes('/pads') || definition.id.includes('/point_debugger');
  assert.ok(front.length > 0 || utility, `${definition.id} produced no forms`);
  if (front.length > 0) assert.ok(geometry(front).pads.length >= 0, `${definition.id} geometry failed`);

  const back = render(definition, { part: part(definition, 'back'), netIndex: () => 4 });
  assert.ok(back.length > 0 || utility, `${definition.id} back variant produced no forms`);
}

const switchDefinition = definitions.find(({ id }) => id === 'ergogen:ceoloide/switch_mx');
assert.ok(switchDefinition);
const resizedSwitch = normalizeDefinition({
  ...switchDefinition,
  generator: { ...switchDefinition.generator, parameters: { ...switchDefinition.generator.parameters, keycap_width: 23 } },
});
assert.equal(resizedSwitch.keycap?.x, 23, 'generated keycap dimensions follow generator values');
const authoredCourtyard = [{ x: -20, y: -19 }, { x: 20, y: -19 }, { x: 20, y: 19 }, { x: -20, y: 19 }];
const authored = normalizeDefinition({
  ...resizedSwitch,
  courtyard: authoredCourtyard,
  keycap: { x: 22, y: 21 },
  envelopeSource: { courtyard: 'authored', keycap: 'authored' },
  generator: { ...resizedSwitch.generator, parameters: { ...resizedSwitch.generator.parameters, keycap_width: 25 } },
});
assert.deepEqual(authored.courtyard, authoredCourtyard, 'authored courtyard is preserved');
assert.deepEqual(authored.keycap, { x: 22, y: 21 }, 'authored keycap envelope is preserved');
const translated = render(switchDefinition, { part: part(switchDefinition, 'front', { include_keycap: true }) });
assert.ok(geometry(translated).pads.every(({ at }) => Number.isFinite(at.x) && Number.isFinite(at.y)));

assert.throws(() => render({ ...switchDefinition, generator: { ...switchDefinition.generator, source: 'ceoloide/missing' } }), /Unknown Ergogen generator/);
assert.throws(() => render({ ...switchDefinition, generator: { ...switchDefinition.generator, version: 'future' } }), /Unsupported Ergogen generator version/);
assert.throws(() => parseForms('(footprint'), /Unbalanced Ergogen output/);
assert.throws(() => parseForms('footprint'), /KiCad forms/);

console.log(`Rendered ${definitions.length} Ergogen generators in front and back poses`);

// Jumper local nets follow stable part identity, never the editable reference.
const reversibleMcu = definitions.find(({ generator }) => generator?.source === 'ceoloide/mcu_nice_nano');
const localNames = (id, reference) => {
  const seen = new Set();
  render(reversibleMcu, { part: { ...part(reversibleMcu), id, reference, generatorParameters: { reversible: true } }, netIndex: name => { seen.add(name); return seen.size; } });
  return [...seen].filter(name => name.startsWith('__boardstudio_local_')).sort();
};
const stableLocals = localNames('mcu/left', 'U1');
assert.ok(stableLocals.length >= 24, 'reversible MCU has distinct local socket nets');
assert.deepEqual(stableLocals, localNames('mcu/left', 'U99'), 'renaming a part preserves local nets');
assert.ok(localNames('mcu/right', 'U1').every(name => !stableLocals.includes(name)), 'same reference on different parts cannot join local nets');

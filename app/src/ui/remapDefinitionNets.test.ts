import { expect, test } from 'vitest';
import { catalogue, normalizeDefinition } from '@boardstudio/v2-ergogen';
import { demoProject } from '../demo';
import { remapDefinitionNets } from './remapDefinitionNets';

function powerSwitchFaceChange() {
  const original = catalogue().find((definition) => definition.generator?.source === 'ceoloide/power_switch_smd_side')!;
  const next = normalizeDefinition({ ...original, generator: { ...original.generator!, parameters: { ...original.generator!.parameters, side: 'B' } } });
  const document = demoProject();
  document.definitions = [original];
  document.parts = document.parts.slice(0, 2).map((part) => ({ ...part, definitionId: original.id }));
  return { document, original, next };
}

test('rejects a real face change that collides with a retained manual net on a later instance', () => {
  const { document, original, next } = powerSwitchFaceChange();
  const destination = next.terminals!.from[0];
  expect(Object.values(original.terminals!).flat()).not.toContain(destination);
  document.nets = [
    { id: 'signal', name: 'Signal', pins: document.parts.flatMap((part) => original.terminals!.from.map((padId) => ({ partId: part.id, padId }))) },
    { id: 'manual', name: 'Manual', pins: [{ partId: document.parts[1].id, padId: destination }] },
  ];
  const before = structuredClone(document);
  expect(remapDefinitionNets(document, original, next)).toEqual({ ok: false, error: expect.stringContaining('multiple nets') });
  expect(document).toEqual(before);
});

test('allows an existing destination on the same net without duplicate pins', () => {
  const { document, original, next } = powerSwitchFaceChange();
  const partId = document.parts[0].id;
  document.nets = [{ id: 'signal', name: 'Signal', pins: [
    ...original.terminals!.from.map((padId) => ({ partId, padId })),
    { partId, padId: next.terminals!.from[0] },
  ] }];
  const result = remapDefinitionNets(document, original, next);
  expect(result).toEqual({ ok: true, nets: [{ id: 'signal', name: 'Signal', pins: next.terminals!.from.map((padId) => ({ partId, padId })) }] });
  if (result.ok) expect(remapDefinitionNets({ ...document, nets: result.nets }, next, next)).toEqual(result);
});

test('rejects two remapped terminals targeting the same pad on different nets', () => {
  const document = demoProject();
  const partId = document.parts[0].id;
  const original = { ...document.definitions[0], terminals: { from: ['a'], to: ['b'] } };
  const next = { ...original, terminals: { from: ['shared'], to: ['shared'] } };
  document.nets = [
    { id: 'row', name: 'Row', pins: [{ partId, padId: 'a' }] },
    { id: 'column', name: 'Column', pins: [{ partId, padId: 'b' }] },
  ];
  expect(remapDefinitionNets(document, original, next)).toEqual({ ok: false, error: expect.stringContaining('multiple nets') });
});

test('allows terminals to exchange pads when both old assignments are remapped', () => {
  const document = demoProject();
  const partId = document.parts[0].id;
  const original = { ...document.definitions[0], terminals: { from: ['a'], to: ['b'] } };
  const next = { ...original, terminals: { from: ['b'], to: ['a'] } };
  document.nets = [
    { id: 'row', name: 'Row', pins: [{ partId, padId: 'a' }] },
    { id: 'column', name: 'Column', pins: [{ partId, padId: 'b' }] },
  ];
  expect(remapDefinitionNets(document, original, next)).toEqual({ ok: true, nets: [
    { id: 'row', name: 'Row', pins: [{ partId, padId: 'b' }] },
    { id: 'column', name: 'Column', pins: [{ partId, padId: 'a' }] },
  ] });
});

test('rejects a split terminal on a later instance without changing any nets', () => {
  const document = demoProject();
  const [first, second] = document.parts;
  const original = { ...document.definitions[0], terminals: { from: ['old-from'], to: ['old-to'] } };
  const next = { ...original, terminals: { from: ['new-from'], to: ['new-to'] } };
  document.nets = [
    { id: 'row', name: 'ROW', pins: [{ partId: first.id, padId: 'old-from' }, { partId: second.id, padId: 'old-from' }] },
    { id: 'other-row', name: 'OTHER', pins: [{ partId: second.id, padId: 'old-from' }] },
  ];
  const before = structuredClone(document.nets);

  const result = remapDefinitionNets(document, original, next);

  expect(result).toEqual({ ok: false, error: expect.stringContaining('split across nets') });
  expect(document.nets).toEqual(before);
});

test('remaps assigned terminals while preserving unrelated pins and repeated saves', () => {
  const document = demoProject();
  const [first, second] = document.parts;
  const original = { ...document.definitions[0], terminals: { from: ['old-from'], to: ['old-to'] } };
  const next = { ...original, terminals: { from: ['new-from'], to: ['new-to'] } };
  document.nets = [
    { id: 'row', name: 'ROW', pins: [{ partId: first.id, padId: 'old-from' }, { partId: second.id, padId: 'old-from' }, { partId: 'unrelated', padId: 'x' }] },
    { id: 'manual', name: 'MANUAL', pins: [{ partId: first.id, padId: 'custom' }] },
  ];

  const result = remapDefinitionNets(document, original, next);
  expect(result.ok).toBe(true);
  if (!result.ok) return;
  expect(result.nets[0].pins).toEqual([
    { partId: 'unrelated', padId: 'x' },
    { partId: first.id, padId: 'new-from' },
    { partId: second.id, padId: 'new-from' },
  ]);
  expect(result.nets[1]).toEqual(document.nets[1]);
  const repeated = remapDefinitionNets({ ...document, nets: result.nets }, next, next);
  expect(repeated).toEqual(result);
});

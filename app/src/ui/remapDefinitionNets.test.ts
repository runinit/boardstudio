import { expect, test } from 'vitest';
import { demoProject } from '../demo';
import { remapDefinitionNets } from './remapDefinitionNets';

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

import { renderToStaticMarkup } from 'react-dom/server';
import { expect, it } from 'vitest';
import { catalogue } from '@boardstudio/v2-ergogen';
import { PartsLibrary } from './PartsLibrary';

it('offers canonical key parts without duplicate legacy choices', () => {
  const definitions = catalogue();
  const before = structuredClone(definitions);
  const markup = renderToStaticMarkup(<PartsLibrary definitions={definitions} assemblies={[]} selected="" query="" onSearch={() => {}} onSelect={() => {}} onAssembly={() => {}} />);
  expect(markup).not.toContain('title="builtin:mx-switch"');
  expect(markup).not.toContain('title="infused-kim/choc"');
  expect(markup).not.toContain('title="infused-kim/diode"');
  expect(markup).not.toContain('title="infused-kim/nice_nano_pretty"');
  expect(markup).toContain('title="ceoloide/switch_mx"');
  expect(markup).toContain('SK6812 MINI-E');
  expect(definitions).toEqual(before);
});

import { partChoices } from './partsCatalog';

it('contains only supported generators instead of hidden retired definitions', () => {
  const definitions = catalogue();
  expect(partChoices(definitions)).toHaveLength(definitions.length);
  expect(definitions.filter(definition => definition.kind === 'switch')).toHaveLength(3);
  expect(definitions.some(definition => ['infused-kim/choc', 'infused-kim/diode'].includes(definition.generator!.source))).toBe(false);
});

it('does not deduplicate custom parts by name or by their underlying generator', () => {
  const builtin = catalogue()[0];
  const custom = { ...builtin, id: 'custom-switch', name: 'MX switch' };
  const imported = { ...builtin, kicadSource: { formatVersion: 1 as const, source: '(footprint Custom)' } };
  expect(partChoices([builtin, custom, imported])).toEqual([builtin, custom, imported]);
});

it('keeps placement snapshots out of the catalog without losing existing assignments', () => {
  const builtin = catalogue()[0];
  const snapshots = ['assembly-preset-mx-rgb-south-matrix-0/definition/switch', '25aa53b5-59f9-4808-82a5-0b3027aecc9f/definition/switch'].map(id => ({ ...builtin, id }));
  expect(partChoices(snapshots)).toEqual([]);
  expect(partChoices(snapshots, snapshots[1].id)).toEqual([snapshots[1]]);
});

it('does not bundle the retired Infused Kim controller', () => {
  expect(catalogue().some(definition => definition.generator?.source === 'infused-kim/nice_nano_pretty')).toBe(false);
});

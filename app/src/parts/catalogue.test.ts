import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { expect, test } from 'vitest';
import { initSync, artifact_request } from '../../../core/pkg/boardstudio_core';
import { importedPartDefinitions } from './catalogue';
import manifest from './import-manifest.json';

initSync({ module: readFileSync(new URL('../../../core/pkg/boardstudio_core_bg.wasm', import.meta.url)) });

const sourcePadCounts: Record<string, number> = {
  'thqwgd001:rotation-reversible': 10,
  'thqwgd001:c-2pin-reversible': 18,
  'thqwgd001:c-4pin-reversible': 20,
  'thqwgd001:slit': 0,
  'thqwgd001:slit-drill': 8,
  'thqwgd001:slitc': 0,
  'thqwgd001:slitc-drill': 8,
};
expect(manifest.entries.filter(entry => entry.id.startsWith('kicad:marbastlib/STAB_MX_'))).toHaveLength(2);
for (const entry of manifest.entries) {
  test(`${entry.name} preserves original source, physical pads and independent snapshots`, () => {
    const source = readFileSync(new URL(entry.file, import.meta.url), 'utf8');
    expect(createHash('sha256').update(source).digest('hex')).toBe(entry.sha256);
    const definition = importedPartDefinitions().find(part => part.id === entry.id)!;
    expect(definition.kicadSource?.source).toBe(source);
    const stabilizer = entry.id.startsWith('kicad:marbastlib/STAB_MX_');
    const expectedPads = stabilizer ? 4 : sourcePadCounts[entry.id];
    expect(expectedPads).toBeDefined();
    expect(definition.pads).toHaveLength(expectedPads);
    if (stabilizer) {
    expect(definition.pads.every(pad => pad.plated === false && (pad.drill ?? 0) > 0)).toBe(true);
    expect(definition.terminals ?? {}).toEqual({});
    expect(definition.mechanicalProfile?.cutouts).toHaveLength(2);
    expect(definition.mechanicalProfile?.pcbHoles).toHaveLength(4);
    expect(definition.mechanicalProfile?.switchFamily).toBe('mx');
    expect(definition.mechanicalProfile?.plateToPcb).toBe(3.5);
    }
    const imported = JSON.parse(artifact_request(JSON.stringify({ id: 'test', kind: 'import-footprint', definitionId: entry.id, source })));
    expect(imported.kind).toBe('import-footprint');
    expect(definition.pads).toEqual(imported.result.definition.pads);
    definition.pads.length = 0;
    expect(importedPartDefinitions().find(part => part.id === entry.id)!.pads).toHaveLength(expectedPads);
  });
}

test('THQ assembly models preserve pinned geometry and the mounting datum', () => {
  const models = [
    ['thqwgd001:rotation-reversible','THQWGD001-rotation.stp','6ae146dcff8704aed80e95e8aa4693df567c55ea88027825a98bfcae4b0b92eb'],
    ['thqwgd001:c-2pin-reversible','THQWGD001C-2pin.stp','6cc61fd27e32e4f6fc202b96f26ac1f0e6edb6afce58e36d09a91eac5eeeb71d'],
    ['thqwgd001:c-4pin-reversible','THQWGD001C-4pin.stp','9ad6b52629349e8ef9a28b6bdc07f5f8f787266d24b025770e9d46c71be1c7c0'],
  ];
  for (const [id, filename, sha256] of models) {
    const definition = importedPartDefinitions().find(part => part.id === id)!;
    const bytes = readFileSync(new URL(`../../../ergogen/library/vendor/thqwgd001/3d_models/${filename}`,import.meta.url));
    expect(createHash('sha256').update(bytes).digest('hex')).toBe(sha256);
    expect(definition.models).toHaveLength(1);
    expect(definition.models![0].offset).toEqual({x:0,y:0,z:0});
    expect(definition.models![0].scale).toEqual({x:1,y:1,z:1});
    expect(definition.models![0].rotation).toEqual({x:0,y:0,z:0});
    expect(definition.hardwareProfile?.gates.some(gate => gate.code === 'reversible-contact-mapping-unverified')).toBe(true);
  }
});

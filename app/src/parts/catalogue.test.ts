import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { expect, test } from 'vitest';
import { initSync, artifact_request } from '../../../core/pkg/boardstudio_core';
import { importedPartDefinitions } from './catalogue';
import manifest from './import-manifest.json';

initSync({ module: readFileSync(new URL('../../../core/pkg/boardstudio_core_bg.wasm', import.meta.url)) });

for (const entry of manifest.entries) {
  test(`${entry.name} preserves source, four drills, and two plate openings`, () => {
    const source = readFileSync(new URL(entry.file, import.meta.url), 'utf8');
    expect(createHash('sha256').update(source).digest('hex')).toBe(entry.sha256);
    const definition = importedPartDefinitions().find(part => part.id === entry.id)!;
    expect(definition.kicadSource?.source).toBe(source);
    expect(definition.pads).toHaveLength(4);
    expect(definition.pads.every(pad => pad.plated === false && (pad.drill ?? 0) > 0)).toBe(true);
    expect(definition.terminals ?? {}).toEqual({});
    expect(definition.mechanicalProfile?.cutouts).toHaveLength(2);
    expect(definition.mechanicalProfile?.pcbHoles).toHaveLength(4);
    expect(definition.mechanicalProfile?.switchFamily).toBe('mx');
    expect(definition.mechanicalProfile?.plateToPcb).toBe(3.5);
    const imported = JSON.parse(artifact_request(JSON.stringify({ id: 'test', kind: 'import-footprint', definitionId: entry.id, source })));
    expect(imported.kind).toBe('import-footprint');
    expect(definition.pads).toEqual(imported.result.definition.pads);
    definition.pads.length = 0;
    expect(importedPartDefinitions().find(part => part.id === entry.id)!.pads).toHaveLength(4);
  });
}

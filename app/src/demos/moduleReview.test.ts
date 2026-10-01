import { readFileSync } from 'node:fs';
import { describe, expect, test } from 'vitest';
import type { CoreReply, CoreRequest } from '@boardstudio/v2-contracts';
import { CoreEngine, initSync } from '../../../core/pkg/boardstudio_core';
import { moduleReviewProject, openModuleReviewDemo } from './moduleReview';

initSync({ module: readFileSync(new URL('../../../core/pkg/boardstudio_core_bg.wasm', import.meta.url)) });

describe('VIK app review project', () => {
  test('keeps above/below placements and the embedded circuit within the public edit flow', async () => {
    const fixture = moduleReviewProject();
    expect(fixture.parameters.moduleHumanReview).toMatchObject({ purpose: 'Human in-app review fixture' });
    expect(fixture.moduleDefinitions?.every(definition => definition.source.revision === 'cd5d16e4cd9137a229fc673412a89d75f4e64553')).toBe(true);
    expect(fixture.modules?.map(instance => instance.hostFace)).toEqual(['front', 'back', 'front']);
    expect(fixture.modules?.every(instance => (instance.mountSupports ?? []).length === 0)).toBe(true);
    expect(fixture.moduleDefinitions?.some(definition => definition.id.includes('ec11-evqwgd001'))).toBe(true);
    const rotaryDefinition = fixture.moduleDefinitions?.find(definition => definition.catalogueRow === 'ec11-evqwgd001');
    expect(rotaryDefinition?.electrical.rotaryProfile).toMatchObject({a:'gpio1',b:'gpio2',common:'gnd',driver:'ec11'});
    expect(rotaryDefinition?.electrical.rotaryProfile?.steps).toBeUndefined();
    expect(rotaryDefinition?.electrical.rotaryProfile?.triggersPerRotation).toBeUndefined();
    expect(fixture.parts.some(part => part.id === 'review/wheel-rotation-only')).toBe(true);
    expect(fixture.moduleDefinitions?.[0].volumes).toHaveLength(0);
    expect(fixture.moduleDefinitions?.[0].gates.some(gate => gate.code === 'assembled-envelope')).toBe(true);

    const engine = new CoreEngine();
    const request = async (input: CoreRequest) => JSON.parse(engine.request(JSON.stringify(input))) as CoreReply;
    try {
      const opened = await openModuleReviewDemo(request);
      expect(opened.document.modules).toHaveLength(3);
      expect(opened.document.modules?.map(instance => instance.hostFace)).toEqual(['front', 'back', 'front']);
      expect(opened.document.keymap?.layers.map(layer => layer.name)).toEqual(['Base', 'Navigation']);
      expect(opened.document.keymap?.layers[0].bindings['matrix/matrix/r0c0']).toMatchObject({kind:'mod-tap'});
      expect(opened.document.keymap?.layers[0].bindings['matrix/matrix/r0c1']).toMatchObject({kind:'layer-tap',layerId:'navigation'});
      expect(opened.document.keymap?.layers[1].bindings['matrix/matrix/r0c1']).toMatchObject({kind:'macro',macroId:'review_navigation_pulse'});
      expect(opened.document.keymap?.macros).toContainEqual(expect.objectContaining({id:'review_navigation_pulse',steps:expect.arrayContaining([expect.objectContaining({kind:'wait',ms:40})])}));
      expect(opened.document.keymap?.layers.every(layer => layer.sensors?.['review/ec11-rotary'])).toBe(true);
      expect(opened.document.parts.some(part => part.definitionId === 'thqwgd001:c-2pin-reversible')).toBe(true);
      expect(opened.document.parts.some(part => part.definitionId === 'thqwgd001:c-4pin-reversible')).toBe(true);
      expect(opened.document.embeddedCircuits).toHaveLength(1);
      const copy = opened.document.embeddedCircuits![0];
      expect(copy.partIds.length).toBeGreaterThan(10);
      expect(new Set(copy.partIds).size).toBe(copy.partIds.length);
      expect(copy.netIds.every(id => id.startsWith(`embedded/${copy.id}/`))).toBe(true);
      expect(copy.netIds.some(id => fixture.boards[0].netIds.includes(id))).toBe(false);
      const boardEnvelope = opened.document.outline.find(feature => feature.id === 'board-envelope');
      expect(boardEnvelope?.kind).toBe('part-envelope');
      expect(boardEnvelope?.kind === 'part-envelope' && copy.partIds.every(id => boardEnvelope.partIds.includes(id))).toBe(true);
      expect(opened.document.parameters.moduleHumanReview).toEqual(fixture.parameters.moduleHumanReview);
    } finally {
      engine.free();
    }
  });
});

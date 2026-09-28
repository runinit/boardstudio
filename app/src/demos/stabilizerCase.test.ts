import { readFileSync } from 'node:fs';
import { expect, test } from 'vitest';
import { CoreEngine, initSync } from '../../../core/pkg/boardstudio_core';
import { openKeyboardDemo } from './keyboards';
import { createMechanicalConfiguration } from '../mechanicalPresets';
import { defaultGasketLayout, defaultInternalGasket } from '../gasketEditing';
import type { CoreReply, CoreRequest } from '@boardstudio/v2-contracts';

initSync({ module: readFileSync(new URL('../../../core/pkg/boardstudio_core_bg.wasm', import.meta.url)) });
test.each(['reviung41', 'mysterium'] as const)('%s generates a gasket case with screws scaled to its size', async (id) => {
  const engine = new CoreEngine();
  try {
    const request = async (input: CoreRequest) => JSON.parse(engine.request(JSON.stringify(input))) as CoreReply;
    const opened = await openKeyboardDemo(id, request);
    const document = opened.document;
    document.mechanical = { ...createMechanicalConfiguration(document), mount: 'gasket', gasketTravel: 0.1,
      gasketLayout: defaultGasketLayout(), internalGasket: defaultInternalGasket() };
    const reply = await request({ id: 'case', kind: 'resolve-mechanical', document, contours: opened.scene.boardContours[0].contours });
    expect(reply.kind).toBe('mechanical-resolved');
    if (reply.kind !== 'mechanical-resolved') throw new Error(JSON.stringify(reply));
    expect(reply.assembly.diagnostics.filter(d => d.severity === 'error')).toEqual([]);
    expect(reply.assembly.generationBlocked).toBe(false);
    if (id === 'mysterium') {
      expect(reply.assembly.suggestedMounts.length).toBeGreaterThanOrEqual(10);
      expect(reply.assembly.suggestedMounts.length).toBeLessThanOrEqual(12);
    }
  } finally { engine.free(); }
});

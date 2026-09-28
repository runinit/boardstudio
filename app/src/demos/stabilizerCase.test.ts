import { readFileSync } from 'node:fs';
import { expect, test } from 'vitest';
import { CoreEngine, initSync } from '../../../core/pkg/boardstudio_core';
import { openKeyboardDemo } from './keyboards';
import { createMechanicalConfiguration } from '../mechanicalPresets';
import { defaultGasketLayout, defaultInternalGasket } from '../gasketEditing';
import type { CoreReply, CoreRequest } from '@boardstudio/v2-contracts';

initSync({ module: readFileSync(new URL('../../../core/pkg/boardstudio_core_bg.wasm', import.meta.url)) });
test('REVIUNG41 generates with stabilizers and the default gasket stack', async () => {
  const engine = new CoreEngine();
  try {
    const request = async (input: CoreRequest) => JSON.parse(engine.request(JSON.stringify(input))) as CoreReply;
    const opened = await openKeyboardDemo('reviung41', request);
    const document = opened.document;
    document.mechanical = { ...createMechanicalConfiguration(document), mount: 'gasket', gasketTravel: 0.1,
      gasketLayout: defaultGasketLayout(), internalGasket: defaultInternalGasket() };
    const reply = await request({ id: 'case', kind: 'resolve-mechanical', document, contours: opened.scene.boardContours[0].contours });
    expect(reply.kind).toBe('mechanical-resolved');
    if (reply.kind !== 'mechanical-resolved') throw new Error(JSON.stringify(reply));
    expect(reply.assembly.diagnostics.filter(d => d.severity === 'error')).toEqual([]);
    expect(reply.assembly.generationBlocked).toBe(false);
  } finally { engine.free(); }
});

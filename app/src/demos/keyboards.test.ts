import { readFileSync } from 'node:fs';
import { describe, expect, test } from 'vitest';
import type { ArtifactReply, ArtifactRequest, CoreReply, CoreRequest, ElectricalPlan } from '@boardstudio/v2-contracts';
import { CoreEngine, initSync, artifact_request } from '../../../core/pkg/boardstudio_core';
import { keyboardDemos, keyboardProject, openKeyboardDemo } from './keyboards';
import measurements from './keyboard-layouts.json';
import { exportErgogenForms } from '@boardstudio/v2-kicad';
import { modelBindings } from '@boardstudio/v2-ergogen';
import { firmwareRequest } from '../firmwareHandoff';
import { unsupportedPads } from './test/outlineHelpers';

initSync({ module: readFileSync(new URL('../../../core/pkg/boardstudio_core_bg.wasm', import.meta.url)) });

describe.each(keyboardDemos)('$name', ({ id }) => {
  test('preserves key count and uses existing or approved definitions', () => {
    const doc = keyboardProject(id);
    const count = measurements[id].keys.length * (measurements[id].split ? 2 : 1);
    expect(doc.parts.filter(part => doc.definitions.find(definition => definition.id === part.definitionId)?.kind === 'switch')).toHaveLength(count);
    expect(doc.definitions.every(definition => definition.generator || /^kicad:marbastlib\/STAB_MX_(2u|6.25u)$/.test(definition.id))).toBe(true);
    expect(new Set(doc.parts.map(part => part.id)).size).toBe(doc.parts.length);
    expect(doc.matrices.every(matrix => matrix.rows + matrix.columns <= 21)).toBe(true);
  });

  test('wires without errors and keeps key assemblies editable', async () => {
    const engine = new CoreEngine();
    const request = async (input: CoreRequest) => JSON.parse(engine.request(JSON.stringify(input))) as CoreReply;
    try {
      const opened = await openKeyboardDemo(id, request);
      expect(opened.scene.boardContours).toHaveLength(opened.document.boards.length);
      for (const board of opened.document.boards) {
        expect(board.netIds.length).toBeGreaterThan(10);
        const ids = new Set(board.partIds);
        expect(opened.document.nets.filter(net => board.netIds.includes(net.id)).every(net => net.pins.every(pin => ids.has(pin.partId)))).toBe(true);
      }
      const plans: ElectricalPlan[] = [];
      const artifact = (input: ArtifactRequest): ArtifactReply => {
        const reply = JSON.parse(artifact_request(JSON.stringify(input))) as ArtifactReply;
        if (reply.kind === 'error') throw new Error(reply.error.message);
        return reply;
      };
      for (const board of opened.document.boards) {
        const resolved = await request({ id: 'resolve', kind: 'resolve-electrical', request: { document: opened.document, instanceId: board.id, boardId: board.id, controllerPartId: `${board.id}/U1`, controllerProfile: null, mode: 'matrix', locks: {} } });
        if (resolved.kind !== 'electrical-resolved') throw new Error('Expected electrical plan');
        expect(resolved.plan.diagnostics.filter(finding => finding.severity === 'error')).toEqual([]);
        plans.push(resolved.plan);
        const paths = new Map(opened.document.definitions.filter(definition => definition.generator).flatMap(definition => modelBindings(definition).map(model => [model.assetId, `models/${model.assetId.replaceAll(/[^a-zA-Z0-9]/g, '_')}.step`] as const)));
        const prepared = artifact({ id: 'prepare', kind: 'prepare-export', request: { document: opened.document, expectedRevision: opened.document.revision, snapshotToken: 'demo', target: { kind: 'board', boardId: board.id }, contours: opened.scene.boardContours.find(contour => contour.boardId === board.id)!.contours, modelPaths: Object.fromEntries(paths) } });
        if (prepared.kind !== 'prepare-export') throw new Error('Expected export plan');
        const plan = prepared.result;
        const nets = [...plan.reservedNets];
        let next = plan.nextNetIndex;
        const results = plan.jobs.map(job => {
          const forms = exportErgogenForms(job.definition, job.part, paths, name => {
            let net = nets.find(net => net.name === name);
            if (!net) { net = { name, index: next++ }; nets.push(net); }
            return net.index;
          });
          return { snapshotToken: plan.snapshotToken, revision: plan.revision, jobId: job.jobId, source: [...forms.footprints, ...forms.objects].join('\n'), nets: [...nets] };
        });
        const exported = artifact({ id: 'finish', kind: 'finish-export', request: { plan, results } });
        expect(exported.kind).toBe('finish-export');
        if (exported.kind !== 'finish-export') throw new Error('Expected board');
        const pcb = exported.result.files.find(file => file.filename.endsWith('.kicad_pcb'))!;
        expect(pcb.content).toContain('Edge.Cuts');
        const preview = artifact({ id: 'preview', kind: 'preview-board', source: pcb.content, revision: opened.document.revision });
        expect(preview.kind).toBe('preview-board');
      }
      const firmware = firmwareRequest(opened.document, plans[0], plans[1]);
      expect((await request({ id: 'firmware', kind: 'generate-firmware', request: firmware.request })).kind).toBe('firmware-generated');
      const matrix = opened.document.matrices[0];
      const edited = await request({ id:'move',kind:'edit',command:{ transactionId:'move',targetIds:[matrix.id],baseRevision:opened.document.revision,phase:'commit',operation:{kind:'set-matrix',matrix:{...matrix,origin:{x:matrix.origin.x+2,y:matrix.origin.y}}}} });
      expect(edited.kind).toBe('scene');
      if (edited.kind !== 'scene') throw new Error('Could not edit');
      expect(edited.document.parts).toHaveLength(opened.document.parts.length);
      for (const partId of matrix.partIds) {
        expect(edited.document.parts.find(part => part.id === partId)!.pose.at.x).toBeCloseTo(opened.document.parts.find(part => part.id === partId)!.pose.at.x + 2);
      }
      const undone = await request({id:'undo',kind:'undo'});
      expect(undone.kind).toBe('scene');
    } finally { engine.free(); }
  });
});


test.each(keyboardDemos)('$name keeps pads and holes inside its live outline', ({ id }) => {
  expect(unsupportedPads(keyboardProject(id))).toEqual([]);
});

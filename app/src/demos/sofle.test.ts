import { readFileSync } from 'node:fs';
import { describe, expect, test } from 'vitest';
import type { ArtifactReply, ArtifactRequest, CoreReply, CoreRequest, ElectricalPlan } from '@boardstudio/v2-contracts';
import { CoreEngine, initSync, artifact_request } from '../../../core/pkg/boardstudio_core';
import { exportErgogenForms } from '@boardstudio/v2-kicad';
import { modelBindings } from '@boardstudio/v2-ergogen';
import { firmwareRequest } from '../firmwareHandoff';
import { unsupportedPads } from './test/outlineHelpers';
import { openSofleDemo, sofleDemos, sofleProject } from './sofle';

initSync({ module: readFileSync(new URL('../../../core/pkg/boardstudio_core_bg.wasm', import.meta.url)) });

describe.each(sofleDemos)('$name', ({ id }) => {
  test('uses only existing generators and reconstructs two editable 29-key halves', () => {
    const doc = sofleProject(id);
    expect(doc.boards).toHaveLength(2);
    expect(doc.parts.filter(part => doc.definitions.find(definition => definition.id === part.definitionId)?.kind === 'switch')).toHaveLength(58);
    expect(doc.definitions.every(definition => definition.generator && !definition.kicadSource)).toBe(true);
    expect(doc.matrices.map(matrix => matrix.cells!.filter(cell => cell.enabled).length)).toEqual([24, 5, 24, 5]);
    expect(doc.parts.filter(part => part.id.endsWith('/led'))).toHaveLength(id === 'v2' ? 0 : 58);
    expect(new Set(doc.parts.map(part => part.id)).size).toBe(doc.parts.length);
    expect(sofleProject(id).id).not.toBe(doc.id);
  });

  test('opens and wires both halves without electrical errors, and supports edit/undo', async () => {
    const engine = new CoreEngine();
    const request = async (input: CoreRequest) => JSON.parse(engine.request(JSON.stringify(input))) as CoreReply;
    try {
      const opened = await openSofleDemo(id, request);
      expect(opened.document.nets.length).toBeGreaterThan(60);
      expect(opened.scene.boardContours).toHaveLength(2);
      for (const board of opened.document.boards) {
        expect(board.netIds.length).toBeGreaterThan(30);
        const partIds = new Set(board.partIds);
        expect(opened.document.nets.filter(net => board.netIds.includes(net.id)).every(net => net.pins.every(pin => partIds.has(pin.partId)))).toBe(true);
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
        const paths = new Map(opened.document.definitions.flatMap(definition => modelBindings(definition).map(model => [model.assetId, `models/${model.assetId.replaceAll(/[^a-zA-Z0-9]/g, '_')}.step`] as const)));
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
      const before = opened.document.parts.find(part => part.id === `matrix/${matrix.id}/r0c0`)!;
      const changed = await request({ id: 'edit', kind: 'edit', command: { baseRevision: opened.document.revision, transactionId: 'move', phase: 'commit', targetIds: [matrix.id], operation: { kind: 'set-matrix', matrix: { ...matrix, origin: { x: matrix.origin.x + 2, y: matrix.origin.y } } } } });
      expect(changed.kind).toBe('scene');
      if (changed.kind !== 'scene') return;
      expect(changed.document.parts.find(part => part.id === before.id)!.pose.at.x).toBeCloseTo(before.pose.at.x + 2);
      const undone = await request({ id: 'undo', kind: 'undo' });
      expect(undone.kind).toBe('scene');
      if (undone.kind === 'scene') expect(undone.document.parts.find(part => part.id === before.id)!.pose).toEqual(before.pose);
    } finally { engine.free(); }
  });
});

test.each(sofleDemos)('$name keeps every drilled hole and pad inside its PCB', ({ id }) => {
  expect(unsupportedPads(sofleProject(id))).toEqual([]);
});

import { describe, expect, it, vi } from 'vitest';
import type { ArtifactRequest, CoreRequest } from '@boardstudio/v2-contracts';
import { emptyProject } from '@boardstudio/v2-contracts';
import { exportMechanicalAssembly, mechanicalStl } from './exportMechanicalAssembly';
import type { CoreClient } from './CoreClient';
import type { CaseClient } from './CaseClient';
import type { ExportClient } from './ExportClient';

describe('mechanical package', () => {
  it('rejects an invalid active outline before generating plate or case deliverables', async () => {
    const { input, cad } = fixture();
    await expect(exportMechanicalAssembly({ ...input, outlineReady: false })).rejects.toThrow('outline');
    expect(input.core.request).not.toHaveBeenCalled();
    expect(cad.request).not.toHaveBeenCalled();
    expect(input.exporter.artifact).not.toHaveBeenCalled();
  });
  it('writes binary STL triangles in millimetres', () => {
    const data = mechanicalStl({ revision: 7, step: new Uint8Array(), mesh: {
      positions: new Float32Array([0, 0, 0, 10, 0, 0, 0, 10, 0]),
      normals: new Float32Array([0, 0, 1, 0, 0, 1, 0, 0, 1]),
    } });
    const view = new DataView(data.buffer);
    expect(data.length).toBe(134);
    expect(view.getUint32(80, true)).toBe(1);
    expect(view.getFloat32(108, true)).toBe(10);
  });

  it('rejects results after the committed document changes', async () => {
    let current = true;
    const core = { request: vi.fn(async () => {
      current = false;
      return { kind: 'mechanical-resolved', assembly: { revision: 0 } };
    }) };
    const exporter = { artifact: vi.fn() };
    await expect(exportMechanicalAssembly({ document: emptyProject('id', 'Board'), contours: [], outlineReady: true,
      core: core as unknown as CoreClient, cad: {} as CaseClient,
      exporter: exporter as unknown as ExportClient, isCurrent: () => current,
    })).rejects.toThrow('stale');
    expect(exporter.artifact).not.toHaveBeenCalled();
  });

  it('blocks a diagnostic error before invoking manufacturing exporters', async () => {
    const core = { request: vi.fn(async () => ({ kind: 'mechanical-resolved', assembly: {
      revision: 0, diagnostics: [{ severity: 'error', message: 'Profile is unqualified' }],
    } })) };
    const exporter = { artifact: vi.fn() };
    await expect(exportMechanicalAssembly({ document: emptyProject('id', 'Board'), contours: [], outlineReady: true,
      core: core as unknown as CoreClient, cad: {} as CaseClient,
      exporter: exporter as unknown as ExportClient, isCurrent: () => true,
    })).rejects.toThrow('Profile is unqualified');
    expect(exporter.artifact).not.toHaveBeenCalled();
  });

  it('packages individual bodies, nominal drawings and assembled STEP', async () => {
    const { input, cad } = fixture();
    const files = await exportMechanicalAssembly(input);
    expect(Object.keys(files)).toEqual(expect.arrayContaining([
      'assembly.step', 'parts/1-plate.step', 'parts/1-plate.stl',
      'outlines/1-plate.svg', 'outlines/1-plate.dxf', 'FABRICATION.md', 'critical-fit.svg',
    ]));
    expect(cad.request).toHaveBeenCalledTimes(2);
    expect((cad.request.mock.calls[0][0] as { bodies: { body: { id: string } }[] }).bodies.map(entry => entry.body.id)).toEqual(['plate', 'pcb-reference']);
    expect(Object.keys(files).some(path => path.includes('pcb-reference'))).toBe(false);
    expect(Object.keys(files).some(path => path.startsWith('plate-kicad/'))).toBe(false);
    expect(new TextDecoder().decode(files['FABRICATION.md'])).toContain('No automatic shrinkage, kerf');
  });

  it('includes the separate KiCad project only for the PCB plate process', async () => {
    const { input } = fixture();
    input.document.mechanical!.method = 'pcb-fr4';
    const files = await exportMechanicalAssembly(input);
    expect(files['plate-kicad/mechanical-plate.kicad_pcb']).toBeDefined();
  });

  it('rejects a stale CAD result before packaging individual bodies', async () => {
    const { input, cad } = fixture();
    let current = true;
    input.isCurrent = () => current;
    cad.request.mockImplementationOnce(async () => { current = false; return { revision: 0 }; });
    await expect(exportMechanicalAssembly(input)).rejects.toThrow('stale');
    expect(cad.request).toHaveBeenCalledTimes(1);
  });

});


function fixture() {
  const document = emptyProject('id', 'Board');
  document.mechanical = {
    boardId: 'board', method: 'printed', mount: 'rigid', integratedPlateFrame: false, plateThickness: 1.5,
    plateFoamThickness: 0, pcbThickness: 1.6, bottomFoamThickness: 0, batteryHeight: 0,
    bottomThickness: 2, plateToPcb: 3.5, wallThickness: 2, clearance: 0.2, profiles: [], mounts: [],
  };
  const contours = [{ hole: false, points: [{ x: 0, y: 0 }, { x: 40, y: 0 }, { x: 40, y: 30 }, { x: 0, y: 30 }] }];
  const body = { id: 'plate', name: 'plate', boardId: 'board', kind: 'plate', thickness: 1.5, clearance: 0 };
  const assembly = { revision: 0, plateContours: contours, case: { revision: 0, bodies: [{ revision: 0, body, contours }] }, stack: [], diagnostics: [] };
  const core = { request: vi.fn(async (request: CoreRequest) => request.kind === 'resolve-mechanical'
    ? { kind: 'mechanical-resolved', assembly }
    : { kind: 'case-prepared', ir: { revision: 0, bodies: request.kind === 'prepare-case' ? request.ir.bodies.map(entry => ({ revision: entry.revision, body: entry.body, regions: [] })) : [] } }) };
  const cad = { request: vi.fn(async (_ir: unknown): Promise<any> => ({ revision: 0, step: new Uint8Array([1]), mesh: { positions: new Float32Array(), normals: new Float32Array() } })) };
  const exporter = { artifact: vi.fn(async (request: Extract<ArtifactRequest, { kind: 'export-mechanical-plate' | 'export-outline' }>) => request.kind === 'export-mechanical-plate'
    ? { kind: 'export-mechanical-plate', result: { revision: 0, files: [{ filename: 'mechanical-plate.kicad_pcb', content: '(kicad_pcb)' }] } }
    : { kind: 'export-outline', result: { filename: request.request.filename, content: 'outline' } }) };
  return { cad, input: { document, contours, outlineReady: true, core: core as unknown as CoreClient, cad: cad as unknown as CaseClient, exporter: exporter as unknown as ExportClient, isCurrent: () => true } };
}

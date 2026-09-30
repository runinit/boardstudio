import { beforeEach, describe, expect, it, vi } from 'vitest';
import { emptyProject, type CoreReply, type CoreRequest, type ElectricalPlan, type KeycapSpec, type SceneDelta } from '@boardstudio/v2-contracts';
import type { CoreClient } from './CoreClient';
import type { CaseClient } from './CaseClient';
import type { ExportClient } from './ExportClient';
import { createProjectExporter } from './createProjectExporter';
import type { ExportSnapshot } from './exports/context';
import { packProject } from './storage';

vi.mock('./storage', () => ({ packProject: vi.fn(), loadAsset: vi.fn() }));

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(done => { resolve = done; });
  return { promise, resolve };
}

function harness() {
  const document = emptyProject('first', 'First');
  document.boards = [{ id: 'board', name: 'Board', partIds: [], netIds: [], outlineIds: [], thickness: 1.6 }];
  document.hardware = { topology: 'unibody', transport: 'none', instances: [], sharedConstruction: null, boards: [{ boardId: 'board', mode: 'direct', controllerPartId: 'mcu', locks: {}, assignments: {}, keyBindings: {}, jumperStates: {}, protectedHandoff: null }] };
  const scene: SceneDelta = { revision: 0, transactionId: 'initial', changedIds: [], transforms: [], matrixScenes: [], contours: [], boardContours: [],
    boardReadiness: [{ boardId: 'board', outline: true, pcb: true, case: true }], findings: [], readiness: { layout: true, outline: true, pcb: true, case: true } };
  let snapshot: ExportSnapshot = { document, scene, boardId: 'board', embedUsedModels: true, generation: { status: 'required' },
    previewContext: { documentId: document.id, boardId: 'board', session: 1, revision: 0, scene, committedScene: scene } };
  const events: string[] = [];
  const plan: ElectricalPlan = { revision: 0, boardId: 'board', instanceId: null, mode: 'direct', controllerPartId: 'mcu', controllerProfile: 'ceoloide/mcu_nice_nano',
    assignments: [], rowPins: [], columnPins: [], diagnostics: [], fingerprint: 'plan', freePins: [], nets: [], diodeDirection: 'col2row', peripherals: [], peripheralPins: {}, peripheralTerminals: {}, moduleAliases: {}, jumpers: [] };
  const core = { request: vi.fn(async (input: CoreRequest): Promise<CoreReply> => {
    events.push(input.kind);
    if (input.kind === 'protect-electrical-handoff' || input.kind === 'apply-electrical') {
      const next = { ...snapshot.document, revision: snapshot.document.revision + 1 };
      return { id: input.id, kind: 'scene', document: next, scene: { ...snapshot.scene, revision: next.revision } };
    }
    throw new Error(`Unexpected core request ${input.kind}`);
  }) };
  const cad = { keycaps: vi.fn(), request: vi.fn() };
  const exporter = {
    request: vi.fn(async () => { events.push('artifact'); return { id: 'file', kind: 'file' as const, filename: 'board.kicad_pcb', bytes: new Uint8Array([1]), mediaType: 'application/octet-stream' }; }),
    archive: vi.fn(async () => { events.push('packed'); return { id: 'archive', kind: 'archive' as const, reply: { kind: 'packed' as const, bytes: new Uint8Array([1, 2]) } }; }),
  };
  const accept = vi.fn(async (reply: CoreReply) => {
    events.push('saved');
    if (reply.kind !== 'scene') throw new Error('Expected scene');
    snapshot = { ...snapshot, document: reply.document, scene: reply.scene, previewContext: { ...snapshot.previewContext, revision: reply.document.revision, scene: reply.scene, committedScene: reply.scene } };
  });
  const resolveWiring = vi.fn(async () => ({ ...plan, revision: snapshot.document.revision }));
  const deliver = vi.fn(() => { events.push('download'); });
  const cadFactory = vi.fn(() => cad as unknown as CaseClient);
  let work!: () => Promise<void>;
  const actions = createProjectExporter({ readSnapshot: () => snapshot, schedule: next => { work = next; }, deliver,
    services: { core: () => core as unknown as CoreClient, cad: cadFactory, exporter: () => exporter as unknown as ExportClient, resolveWiring, accept } });
  return { actions, core, cad, cadFactory, exporter, accept, resolveWiring, deliver, events, plan, run: () => work(),
    snapshot: () => snapshot, change: (next: Partial<ExportSnapshot>) => { snapshot = { ...snapshot, ...next }; } };
}

beforeEach(() => { vi.mocked(packProject).mockReset().mockResolvedValue(new Uint8Array([1])); });

describe('export snapshot ownership', () => {
  it('does not download a project after opening a different project at the same revision', async () => {
    const test = harness(), archive = deferred<Uint8Array>();
    vi.mocked(packProject).mockReturnValueOnce(archive.promise);
    test.actions.exportFile('project');
    const pending = test.run();
    const rejected = expect(pending).rejects.toThrow(/changed|stale/i);
    test.change({ document: emptyProject('second', 'Second') });
    archive.resolve(new Uint8Array([1]));
    await rejected;
    expect(test.deliver).not.toHaveBeenCalled();
  });

  it('rejects an export queued before the same project is reopened in a new session', async () => {
    const test = harness();
    test.actions.exportFile('project');
    test.change({ previewContext: { ...test.snapshot().previewContext, session: 2 } });
    await expect(test.run()).rejects.toThrow(/changed/i);
    expect(packProject).not.toHaveBeenCalled();
    expect(test.deliver).not.toHaveBeenCalled();
  });

  it('rejects a scope change while packaging and retains the captured embed preference', async () => {
    const test = harness(), archive = deferred<Uint8Array>();
    vi.mocked(packProject).mockReturnValueOnce(archive.promise);
    test.actions.exportFile('project');
    const pending = test.run();
    const rejected = expect(pending).rejects.toThrow(/changed/i);
    expect(packProject).toHaveBeenCalledWith(test.snapshot().document, { embedUsedModels: true }, expect.anything());
    test.change({ previewContext: { ...test.snapshot().previewContext, boardId: 'other' } });
    archive.resolve(new Uint8Array([1]));
    await rejected;
    expect(test.deliver).not.toHaveBeenCalled();
  });

  it('downloads a completed project through the injected delivery port without starting CAD', async () => {
    const test = harness();
    test.actions.exportFile('project');
    await test.run();
    expect(test.deliver).toHaveBeenCalledWith({ filename: 'First.boardstudio', bytes: new Uint8Array([1]), mediaType: 'application/zip' });
    expect(test.cadFactory).not.toHaveBeenCalled();
  });
});

describe('PCB artifact and handoff ownership', () => {
  it('protects and saves the handoff after packaging and before delivery', async () => {
    const test = harness();
    test.actions.exportFile('kicad');
    await test.run();
    expect(test.events).toEqual(['artifact', 'packed', 'protect-electrical-handoff', 'saved', 'download']);
    expect(test.snapshot().document.revision).toBe(1);
    expect(test.deliver).toHaveBeenCalledWith(expect.objectContaining({ filename: 'First-pcb-handoff.zip' }));
  });

  it('does not record a protected handoff or download when packaging fails', async () => {
    const test = harness();
    test.exporter.archive.mockRejectedValueOnce(new Error('Packing failed'));
    test.actions.exportFile('kicad');
    await expect(test.run()).rejects.toThrow('Packing failed');
    expect(test.core.request).not.toHaveBeenCalled();
    expect(test.accept).not.toHaveBeenCalled();
    expect(test.deliver).not.toHaveBeenCalled();
  });

  it('adopts applied wiring and rejects stale resolution before mutating the core', async () => {
    const test = harness();
    test.change({ document: { ...test.snapshot().document, hardware: undefined } });
    test.actions.exportFile('kicad-draft');
    await test.run();
    expect(test.events).toEqual(['apply-electrical', 'saved', 'artifact', 'packed', 'protect-electrical-handoff', 'saved', 'download']);
    expect(test.deliver).toHaveBeenCalledWith(expect.objectContaining({ filename: 'First-draft-pcb-handoff.zip' }));
    const stale = harness(), plan = deferred<ElectricalPlan>();
    stale.resolveWiring.mockReturnValueOnce(plan.promise);
    stale.actions.exportFile('kicad');
    const pending = stale.run();
    const rejected = expect(pending).rejects.toThrow(/changed/i);
    stale.change({ previewContext: { ...stale.snapshot().previewContext, session: 2 } });
    plan.resolve(stale.plan);
    await rejected;
    expect(stale.core.request).not.toHaveBeenCalled();
    expect(stale.deliver).not.toHaveBeenCalled();
  });
});

describe('keycap and firmware export ownership', () => {
  it('sends Rust-resolved specs to the export CAD client and checks the CAD revision', async () => {
    const test = harness();
    const specs = [{ id: 'key' } as KeycapSpec];
    test.core.request.mockResolvedValueOnce({ id: 'caps', kind: 'keycaps-resolved', result: { revision: 0, specs, findings: [] } });
    test.cad.keycaps.mockResolvedValueOnce({ revision: 0, step: new Uint8Array([3]) });
    test.actions.exportFile('keycaps-step');
    await test.run();
    expect(test.cad.keycaps).toHaveBeenCalledWith(0, specs, true);
    expect(test.deliver).toHaveBeenCalledWith({ filename: 'First-keycaps.step', bytes: new Uint8Array([3]), mediaType: 'model/step' });
    test.core.request.mockResolvedValueOnce({ id: 'caps', kind: 'keycaps-resolved', result: { revision: 0, specs, findings: [] } });
    test.cad.keycaps.mockResolvedValueOnce({ revision: 1, step: new Uint8Array([4]) });
    test.actions.exportFile('keycaps-step');
    await expect(test.run()).rejects.toThrow(/changed/i);
    expect(test.deliver).toHaveBeenCalledOnce();
  });

  it('does not initialize CAD for invalid keycap clearance', async () => {
    const test = harness();
    test.core.request.mockResolvedValueOnce({ id: 'caps', kind: 'keycaps-resolved', result: { revision: 0, specs: [], findings: [{ id: 'caps', severity: 'error', message: 'Collision', scope: 'case', targetIds: [] }] } });
    test.actions.exportFile('keycaps-step');
    await expect(test.run()).rejects.toThrow('Collision');
    expect(test.cadFactory).not.toHaveBeenCalled();
    expect(test.deliver).not.toHaveBeenCalled();
  });

  it('packages Rust firmware with its resolved plan without recording a PCB handoff', async () => {
    const test = harness();
    test.core.request.mockResolvedValueOnce({ id: 'firmware', kind: 'firmware-generated', package: { files: { 'board.keymap': '&none', 'build-local.sh': 'west build' }, warnings: [] } });
    test.actions.exportFile('firmware');
    await test.run();
    expect(test.exporter.archive).toHaveBeenCalledWith(expect.objectContaining({ request: { kind: 'pack-files', entries: [
      { path: 'board.keymap', bufferIndex: 0 }, { path: 'build-local.sh', bufferIndex: 1 }, { path: 'electrical-plan.json', bufferIndex: 2 },
    ] } }));
    expect(test.accept).not.toHaveBeenCalled();
    expect(test.deliver).toHaveBeenCalledWith(expect.objectContaining({ filename: 'First-zmk.zip' }));
  });
});

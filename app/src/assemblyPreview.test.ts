import { describe, expect, it, vi } from 'vitest';
import { emptyProject, type CoreReply, type KeycapSpec, type PcbPreview, type ProjectDoc, type CaseResult } from '@boardstudio/v2-contracts';
import { AssemblyPreview, type AssemblyPreviewSnapshot } from './assemblyPreview';

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(done => { resolve = done; });
  return { promise, resolve };
}
const flush = async () => { for (let i = 0; i < 8; i++) await Promise.resolve(); };
const pcb = (revision = 0): PcbPreview => ({ revision, models: [], diagnostics: [], surfaces: [] } as unknown as PcbPreview);
const input = (id = 'project', revision = 0) => ({ document: { ...emptyProject(id, id), revision }, boardId: 'board', contours: [] });
const spec = (id: string): KeycapSpec => ({ id, color: '#ff0000', legendColor: '#ffffff' } as KeycapSpec);
const caps = (revision: number, id: string): CaseResult => ({ revision, step: new Uint8Array(), mesh: { positions: new Float32Array(), normals: new Float32Array() }, bodies: [
  { id: `keycap:${id}`, name: id, positions: new Float32Array([1, 2, 3]), normals: new Float32Array([0, 0, 1]) },
] });

describe('assembly preview ownership', () => {
  it('rejects replies from another project with the same board and revision', async () => {
    const first = deferred<PcbPreview>(), second = deferred<PcbPreview>();
    const exporter = { preview: vi.fn().mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise), artifact: vi.fn(), close: vi.fn() };
    const owner = new AssemblyPreview({ exporter: () => exporter });
    let state!: AssemblyPreviewSnapshot;
    owner.subscribe(value => { state = value; });
    owner.update(input('first'));
    owner.update(input('second'));
    second.resolve({ ...pcb(), diagnostics: ['second'] });
    await flush();
    first.resolve({ ...pcb(), diagnostics: ['first'] });
    await flush();
    expect(state.messages).toContain('second');
    expect(state.messages).not.toContain('first');
    owner.close();
    expect(exporter.close).toHaveBeenCalledOnce();
  });

  it.each(['session', 'boardId', 'instanceId'] as const)('clears the previous scene when %s changes with the same revision', async field => {
    const next = deferred<PcbPreview>();
    const exporter = { preview: vi.fn().mockResolvedValueOnce(pcb()).mockReturnValueOnce(next.promise), artifact: vi.fn(), close: vi.fn() };
    const owner = new AssemblyPreview({ exporter: () => exporter });
    let state!: AssemblyPreviewSnapshot;
    owner.subscribe(value => { state = value; });
    const original = { ...input(), session: 1, instanceId: 'left' };
    owner.update(original);
    await flush();
    expect(state.board).toBeDefined();
    owner.update({ ...original, [field]: field === 'session' ? 2 : 'other' });
    expect(state.board).toBeUndefined();
    expect(state.pending).toBe(true);
    next.resolve({ ...pcb(), diagnostics: ['new scope'] });
    await flush();
    expect(state.messages).toContain('new scope');
    owner.close();
  });

  it('discards stale keycap resolution before starting CAD', async () => {
    const first = deferred<CoreReply>(), second = deferred<CoreReply>();
    const core = { request: vi.fn().mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise), close: vi.fn() };
    const cad = { keycaps: vi.fn().mockResolvedValue(caps(1, 'new')), requestModel: vi.fn(), close: vi.fn() };
    const owner = new AssemblyPreview({ core: () => core, cad: () => cad, exporter: () => ({ preview: vi.fn().mockImplementation(({ document }: { document: ProjectDoc }) => Promise.resolve(pcb(document.revision))), artifact: vi.fn(), close: vi.fn() }) });
    let state!: AssemblyPreviewSnapshot;
    owner.subscribe(value => { state = value; });
    owner.update({ ...input(), document: { ...input().document, keycaps: { boards: {}, matrices: {}, keys: {} } } });
    owner.update({ ...input(), document: { ...input().document, revision: 1, keycaps: { boards: {}, matrices: {}, keys: {} } } });
    first.resolve({ id: 'old', kind: 'keycaps-resolved', result: { revision: 0, findings: [], specs: [spec('old')] } });
    await flush();
    expect(cad.keycaps).not.toHaveBeenCalled();
    second.resolve({ id: 'new', kind: 'keycaps-resolved', result: { revision: 1, findings: [], specs: [spec('new')] } });
    await flush();
    expect(state.keycaps[0].id).toBe('keycap:new');
    expect(state.keycaps[0].color).toBe('#ff0000');
    expect(state.keycapsPending).toBe(false);
    owner.close();
  });

  it('aborts obsolete CAD and prevents its late result replacing the current preview', async () => {
    const first = deferred<CaseResult>(), second = deferred<CaseResult>();
    const core = { request: vi.fn().mockImplementation(({ document }: { document: ProjectDoc }) => Promise.resolve({ id: 'caps', kind: 'keycaps-resolved', result: { revision: document.revision, findings: [], specs: [spec('key')] } })), close: vi.fn() };
    const cad = { keycaps: vi.fn().mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise), requestModel: vi.fn(), close: vi.fn() };
    const owner = new AssemblyPreview({ core: () => core, cad: () => cad, exporter: () => ({ preview: vi.fn().mockResolvedValue(pcb()), artifact: vi.fn(), close: vi.fn() }) });
    let state!: AssemblyPreviewSnapshot;
    owner.subscribe(value => { state = value; });
    const document = { ...input().document, keycaps: { boards: {}, matrices: {}, keys: {} } };
    owner.update({ ...input(), document });
    await flush();
    const signal: AbortSignal = cad.keycaps.mock.calls[0][3];
    owner.update({ ...input(), document: { ...document, revision: 1 } });
    await flush();
    expect(signal.aborted).toBe(true);
    second.resolve(caps(1, 'current'));
    await flush();
    first.resolve(caps(0, 'old'));
    await flush();
    expect(state.keycaps[0].id).toBe('keycap:current');
    owner.close();
    expect(cad.close).toHaveBeenCalledOnce();
    expect(core.close).toHaveBeenCalledOnce();
  });

  it('reuses an in-flight model conversion across geometry updates', async () => {
    const bytes = deferred<Uint8Array | undefined>();
    const mesh = { positions: new Float32Array([1, 2, 3]), normals: new Float32Array([0, 0, 1]) };
    const cad = { keycaps: vi.fn(), requestModel: vi.fn().mockResolvedValue({ mesh }), close: vi.fn() };
    const loadAsset = vi.fn().mockReturnValue(bytes.promise);
    const models = [{ id: 'model', reference: 'SW1', path: '${KIPRJMOD}/models/preview/0.step' }] as PcbPreview['models'];
    const exporter = { preview: vi.fn().mockImplementation(({ document }: { document: ProjectDoc }) => Promise.resolve({ ...pcb(document.revision), models })), artifact: vi.fn(), close: vi.fn() };
    const owner = new AssemblyPreview({ cad: () => cad, exporter: () => exporter, loadAsset });
    let state!: AssemblyPreviewSnapshot;
    owner.subscribe(value => { state = value; });
    const document = { ...input().document, assets: [{ id: 'asset', name: 'switch.step', sha256: 'hash', mediaType: 'model/step' }] };
    owner.update({ ...input(), document });
    await flush();
    owner.update({ ...input(), document: { ...document, revision: 1 }, contours: [{ hole: false, points: [{ x: 0, y: 0 }, { x: 5, y: 0 }, { x: 5, y: 5 }] }] });
    await flush();
    bytes.resolve(new Uint8Array([1]));
    await vi.waitFor(() => expect(state.models).toEqual([{ id: 'model', mesh }]));
    expect(loadAsset).toHaveBeenCalledOnce();
    expect(cad.requestModel).toHaveBeenCalledOnce();
    owner.close();
  });

  it('keeps failed conversion out of the cache so retry can recover', async () => {
    const mesh = { positions: new Float32Array([1, 2, 3]), normals: new Float32Array([0, 0, 1]) };
    const cad = { keycaps: vi.fn(), requestModel: vi.fn().mockRejectedValueOnce(new Error('bad STEP')).mockResolvedValueOnce({ mesh }), close: vi.fn() };
    const models = [{ id: 'model', reference: 'SW1', path: '${KIPRJMOD}/models/preview/0.step' }] as PcbPreview['models'];
    const owner = new AssemblyPreview({ cad: () => cad, loadAsset: vi.fn().mockResolvedValue(new Uint8Array([1])), exporter: () => ({ preview: vi.fn().mockResolvedValue({ ...pcb(), models }), artifact: vi.fn(), close: vi.fn() }) });
    let state!: AssemblyPreviewSnapshot;
    owner.subscribe(value => { state = value; });
    owner.update({ ...input(), document: { ...input().document, assets: [{ id: 'asset', name: 'switch.step', sha256: 'hash', mediaType: 'model/step' }] } });
    await vi.waitFor(() => expect(state.messages.some(message => message.includes('bad STEP'))).toBe(true));
    owner.retry();
    await vi.waitFor(() => expect(state.models).toHaveLength(1));
    expect(cad.requestModel).toHaveBeenCalledTimes(2);
    owner.close();
  });

  it('closes lazily created workers and prevents late replies notifying subscribers', async () => {
    const reply = deferred<PcbPreview>();
    const exporter = { preview: vi.fn().mockReturnValue(reply.promise), artifact: vi.fn(), close: vi.fn() };
    const cad = vi.fn(), core = vi.fn();
    const owner = new AssemblyPreview({ exporter: () => exporter, cad, core });
    const listener = vi.fn();
    owner.subscribe(listener);
    owner.update(input());
    owner.close();
    const count = listener.mock.calls.length;
    reply.resolve(pcb());
    await flush();
    owner.retry();
    expect(listener).toHaveBeenCalledTimes(count);
    expect(exporter.close).toHaveBeenCalledOnce();
    expect(core).not.toHaveBeenCalled();
    expect(cad).not.toHaveBeenCalled();
  });
});

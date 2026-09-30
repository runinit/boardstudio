import type { BoardReference, Contour, KeycapResolution, PcbPreview, PreparedCaseAssemblyIR, ProjectDoc } from '@boardstudio/v2-contracts';
import { modelAssetId } from '@boardstudio/v2-ergogen';
import { CoreClient } from './CoreClient';
import { CaseClient } from './CaseClient';
import { ExportClient } from './ExportClient';
import { bundledModel, bundledModelBytes, bundledModels } from './bundledModels';
import { readMeshModel, type ModelMesh } from './modelMesh';
import { loadAsset } from './storage';

export type LoadedModel = { id: string; mesh: ModelMesh };
export type AssemblyBody = { id: string; name: string; color?: string; mesh: ModelMesh };
export type AssemblyPreviewInput = {
  document: ProjectDoc; boardId: string; contours: Contour[]; preparedCase?: PreparedCaseAssemblyIR; session?: number; instanceId?: string;
};
export type AssemblyPreviewSnapshot = {
  board?: PcbPreview; reference?: BoardReference; models: LoadedModel[]; messages: string[];
  pending: boolean; error: string; keycaps: AssemblyBody[]; keycapResolution?: KeycapResolution;
  keycapsPending: boolean; keycapError: string;
};
type Dependencies = {
  core: () => Pick<CoreClient, 'request' | 'close'>;
  cad: () => Pick<CaseClient, 'keycaps' | 'requestModel' | 'close'>;
  exporter: () => Pick<ExportClient, 'preview' | 'artifact' | 'close'>;
  loadAsset: typeof loadAsset;
  modelBytes: typeof bundledModelBytes;
};
const initial = (): AssemblyPreviewSnapshot => ({
  board: undefined, reference: undefined, keycapResolution: undefined,
  models: [], messages: [], pending: true, error: '', keycaps: [], keycapsPending: false, keycapError: '',
});

/** Owns preview worker lifetimes, asset reuse, and acceptance of asynchronous results. */
export class AssemblyPreview {
  private readonly dependencies: Dependencies;
  private core?: ReturnType<Dependencies['core']>;
  private cad?: ReturnType<Dependencies['cad']>;
  private exporter?: ReturnType<Dependencies['exporter']>;
  private readonly cache = new Map<string, Promise<ModelMesh>>();
  private readonly listeners = new Set<(snapshot: AssemblyPreviewSnapshot) => void>();
  private snapshot = initial();
  private input?: AssemblyPreviewInput;
  private scope = '';
  private pcbKey = '';
  private keycapKey = '';
  private pcbSequence = 0;
  private keycapSequence = 0;
  private keycapAbort?: AbortController;
  private closed = false;

  constructor(dependencies: Partial<Dependencies> = {}) {
    this.dependencies = {
      core: () => new CoreClient(), cad: () => new CaseClient(), exporter: () => new ExportClient(),
      loadAsset, modelBytes: bundledModelBytes, ...dependencies,
    };
  }

  subscribe(listener: (snapshot: AssemblyPreviewSnapshot) => void): () => void {
    this.listeners.add(listener);
    listener(this.snapshot);
    return () => { this.listeners.delete(listener); };
  }

  update(input: AssemblyPreviewInput): void {
    if (this.closed) return;
    this.input = input;
    const { document, boardId, contours, preparedCase } = input;
    const scope = JSON.stringify([document.id, boardId, input.session, input.instanceId]);
    if (scope !== this.scope) {
      this.scope = scope;
      this.pcbSequence++; this.keycapSequence++;
      this.keycapAbort?.abort();
      this.cache.clear();
      this.pcbKey = ''; this.keycapKey = '';
      this.publish(initial());
    }
    const reference = document.boardReferences?.find(item => item.boardId === boardId && item.enabled);
    const ids = new Set([
      ...document.assets.map(asset => asset.id), ...bundledModels().map(model => model.id),
      ...document.definitions.flatMap(definition => (definition.models ?? []).map(model => model.assetId)),
    ]);
    const paths: [string, string][] = [...ids].map((id, index) => [id,
      `models/preview/${index}.${(document.assets.find(asset => asset.id === id)?.name ?? bundledModel(id)?.filename ?? 'model.step').split('.').pop()}`,
    ]);
    const generatedReferences = new Set(document.parts.filter(part => document.keycaps?.keys[part.id]?.profile
      || document.matrices.some(matrix => matrix.partIds.includes(part.id) && document.keycaps?.matrices[matrix.id]?.profile)).map(part => part.reference));
    const pcbKey = JSON.stringify({ scope, board: document.boards.find(board => board.id === boardId), parts: document.parts,
      generatedReferences: [...generatedReferences], definitions: document.definitions, assets: document.assets, contours, reference, paths });
    if (pcbKey !== this.pcbKey) {
      this.pcbKey = pcbKey;
      void this.prepareBoard(input, reference, paths, generatedReferences, ++this.pcbSequence);
    }
    const keycapKey = JSON.stringify({ scope, revision: document.revision, keycaps: document.keycaps, parts: document.parts,
      matrices: document.matrices, hardware: document.hardware, definitions: document.definitions, cases: preparedCase });
    if (keycapKey !== this.keycapKey) {
      this.keycapKey = keycapKey;
      this.keycapAbort?.abort();
      const abort = new AbortController();
      this.keycapAbort = abort;
      void this.prepareKeycaps(input, abort, ++this.keycapSequence);
    }
  }

  retry(): void {
    this.pcbKey = ''; this.keycapKey = '';
    if (this.input) this.update(this.input);
  }

  close(): void {
    if (this.closed) return;
    this.closed = true;
    this.keycapAbort?.abort();
    this.listeners.clear();
    this.core?.close(); this.cad?.close(); this.exporter?.close();
    this.cache.clear();
  }

  private publish(change: Partial<AssemblyPreviewSnapshot>): void {
    if (this.closed) return;
    this.snapshot = { ...this.snapshot, ...change };
    for (const listener of this.listeners) listener(this.snapshot);
  }

  private async prepareKeycaps(input: AssemblyPreviewInput, abort: AbortController, sequence: number): Promise<void> {
    const current = () => !this.closed && sequence === this.keycapSequence && !abort.signal.aborted;
    const { document, boardId, preparedCase } = input;
    this.publish({ keycaps: [], keycapError: '', keycapResolution: undefined, keycapsPending: Boolean(document.keycaps) });
    if (!document.keycaps) return;
    try {
      this.core ??= this.dependencies.core();
      const reply = await this.core.request({ id: crypto.randomUUID(), kind: 'resolve-keycaps', document, boardId, cases: preparedCase ?? null });
      if (!current()) return;
      if (reply.kind !== 'keycaps-resolved') throw new Error(reply.kind === 'error' ? reply.message : 'Expected resolved keycaps');
      if (reply.result.revision !== document.revision) return;
      this.publish({ keycapResolution: reply.result });
      if (reply.result.findings.some(finding => finding.severity === 'error') || !reply.result.specs.length) return;
      this.cad ??= this.dependencies.cad();
      const result = await this.cad.keycaps(document.revision, reply.result.specs, false, abort.signal);
      if (!current() || result.revision !== document.revision) return;
      const specs = new Map(reply.result.specs.map(spec => [spec.id, spec]));
      this.publish({ keycaps: (result.bodies ?? []).map(body => {
        const legend = body.id.startsWith('keycap-legend:');
        const spec = specs.get(body.id.slice(legend ? 14 : 7));
        return { id: body.id, name: body.name, color: legend ? spec?.legendColor : spec?.color,
          mesh: { positions: body.positions, normals: body.normals } };
      }) });
    } catch (cause) {
      if (current()) this.publish({ keycapError: String(cause) });
    } finally {
      if (current()) this.publish({ keycapsPending: false });
    }
  }

  private async prepareBoard(input: AssemblyPreviewInput, reference: BoardReference | undefined, paths: [string, string][], generatedReferences: Set<string>, sequence: number): Promise<void> {
    const current = () => !this.closed && sequence === this.pcbSequence;
    const { document, boardId, contours } = input;
    this.publish({ pending: true, error: '', messages: [] });
    try {
      this.exporter ??= this.dependencies.exporter();
      let result: PcbPreview;
      if (reference) {
        const asset = document.assets.find(asset => asset.id === reference.assetId);
        const bytes = asset && await this.dependencies.loadAsset(asset.sha256);
        if (!current()) return;
        if (!bytes) throw new Error('The saved KiCad board is missing. Import it again.');
        const reply = await this.exporter.artifact({ kind: 'preview-board', source: new TextDecoder().decode(bytes), revision: document.revision });
        if (reply.kind !== 'preview-board') throw new Error('Expected routed board preview');
        result = reply.result;
      } else result = await this.exporter.preview({ document, boardId, contours, paths });
      if (!current() || result.revision !== document.revision) return;
      result = { ...result, models: result.models.filter(model => !(generatedReferences.has(model.reference) && /keycap/i.test(model.path))) };
      this.publish({ board: result, reference, models: [], pending: false,
        messages: result.models.length ? result.diagnostics : [...result.diagnostics, 'No component models are attached. Add models or choose a configured assembly in Parts.'] });
      const lookup = new Map(paths.map(([id, path]) => [`\${KIPRJMOD}/${path}`, id]));
      const loaded: LoadedModel[] = [];
      await Promise.all(result.models.map(async model => {
        const id = reference?.modelAssets[model.path] ?? lookup.get(model.path) ?? modelAssetId(model.path);
        const asset = document.assets.find(asset => asset.id === id);
        const bundled = id ? bundledModel(id) : undefined;
        if (!asset && !bundled) {
          if (current()) this.publish({ messages: [...this.snapshot.messages, `${model.reference}: missing model ${model.path}. Attach the model in the inspector.`] });
          return;
        }
        const name = asset?.name ?? bundled!.filename, key = asset?.sha256 ?? id!;
        let task = this.cache.get(key);
        const scope = this.scope;
        if (!task) {
          task = (asset ? this.dependencies.loadAsset(asset.sha256) : this.dependencies.modelBytes(id!)).then(async bytes => {
            if (this.closed || this.scope !== scope) throw new Error('Model preview superseded');
            if (!bytes) throw new Error('Saved model bytes are missing');
            if (/\.(step|stp)$/i.test(name)) {
              this.cad ??= this.dependencies.cad();
              return (await this.cad.requestModel(bytes)).mesh;
            }
            return readMeshModel(bytes, name);
          }).catch(cause => { if (this.cache.get(key) === task) this.cache.delete(key); throw cause; });
          this.cache.set(key, task);
          if (this.cache.size > 80) this.cache.delete(this.cache.keys().next().value!);
        }
        try {
          const mesh = await task;
          if (current()) loaded.push({ id: model.id, mesh });
        } catch (cause) {
          if (current()) this.publish({ messages: [...this.snapshot.messages, `${model.reference}: ${String(cause)}`] });
        }
      }));
      if (current()) this.publish({ models: loaded });
    } catch (cause) {
      if (current()) this.publish({ error: String(cause instanceof Error ? cause.message : cause), pending: false });
    }
  }
}

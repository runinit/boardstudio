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
  pcbTopZ?: number;
};
export type AssemblyPreviewSnapshot = {
  board?: PcbPreview; reference?: BoardReference; models: LoadedModel[]; messages: string[];
  pending: boolean; error: string; keycaps: AssemblyBody[]; keycapResolution?: KeycapResolution;
  keycapsPending: boolean; keycapError: string;
  moduleBodies: AssemblyBody[]; modulesPending: boolean; moduleError: string;
};
type Dependencies = {
  core: () => Pick<CoreClient, 'request' | 'close'>;
  cad: () => Pick<CaseClient, 'keycaps' | 'requestModel' | 'close'> & Partial<Pick<CaseClient, 'preview'>>;
  exporter: () => Pick<ExportClient, 'preview' | 'artifact' | 'close'>;
  loadAsset: typeof loadAsset;
  modelBytes: typeof bundledModelBytes;
};
const initial = (): AssemblyPreviewSnapshot => ({
  board: undefined, reference: undefined, keycapResolution: undefined,
  models: [], messages: [], pending: true, error: '', keycaps: [], keycapsPending: false, keycapError: '',
  moduleBodies: [], modulesPending: false, moduleError: '',
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
  private moduleKey = '';
  private moduleSequence = 0;
  private moduleAbort?: AbortController;
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
      this.pcbSequence++; this.keycapSequence++; this.moduleSequence++;
      this.keycapAbort?.abort();
      this.moduleAbort?.abort();
      this.cache.clear();
      this.pcbKey = ''; this.keycapKey = ''; this.moduleKey = '';
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
    const moduleKey=JSON.stringify({scope,modules:document.modules,definitions:document.moduleDefinitions,hardware:document.hardware,physicalInstanceId:document.physicalInstanceId,thickness:document.boards.find(board=>board.id===boardId)?.thickness,assets:document.assets,pcbTopZ:input.pcbTopZ});
    if(moduleKey!==this.moduleKey) {
      this.moduleKey=moduleKey;this.moduleAbort?.abort();
      const abort=new AbortController();this.moduleAbort=abort;
      void this.prepareModules(input,abort,++this.moduleSequence);
    }
  }

  retry(): void {
    this.pcbKey = ''; this.keycapKey = ''; this.moduleKey = '';
    if (this.input) this.update(this.input);
  }

  close(): void {
    if (this.closed) return;
    this.closed = true;
    this.keycapAbort?.abort();
    this.moduleAbort?.abort();
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

  private async prepareModules(input:AssemblyPreviewInput,abort:AbortController,sequence:number):Promise<void> {
    const current=()=>!this.closed&&sequence===this.moduleSequence&&!abort.signal.aborted;
    const {document,boardId}=input;
    const active=document.modules?.some(instance=>instance.hostBoardId===boardId&&!instance.detached);
    this.publish({moduleBodies:[],moduleError:'',modulesPending:Boolean(active)});
    if(!active)return;
    try {
      this.core??=this.dependencies.core();
      const reply=await this.core.request({id:crypto.randomUUID(),kind:'resolve-modules',document,boardId,previewTopZ:input.pcbTopZ??0});
      if(!current())return;
      if(reply.kind!=='modules-resolved')throw new Error(reply.kind==='error'?reply.message:'Expected resolved module assembly');
      if(reply.result.revision!==document.revision)return;
      const bodies:AssemblyBody[]=[];
      if(reply.result.preview) {
        this.cad??=this.dependencies.cad();
        if(!this.cad.preview)throw new Error('Module CAD preview is unavailable');
        const result=await this.cad.preview(reply.result.preview,()=>{},abort.signal);
        if(!current()||result.revision!==document.revision)return;
        bodies.push(...(result.bodies??[]).map(body=>({id:body.id,name:body.name,color:'#487d67',mesh:{positions:body.positions,normals:body.normals}})));
      }
      for(const placement of reply.result.modelPlacements??[]) {
        if(!current())return;
        const asset=document.assets.find(asset=>asset.id===placement.assetId);
        const bundled=bundledModel(placement.assetId);
        if(!asset&&!bundled)throw new Error(`Module model is missing: ${placement.assetId}`);
        const name=asset?.name??bundled!.filename;
        const task = this.convertModel(asset, placement.assetId, name);
        const mesh=await task;
        if(current())bodies.push({id:placement.id,name,color:'#8f9b9e',mesh:transformedMesh(mesh,placement.matrix)});
      }
      if(current())this.publish({moduleBodies:bodies});
    }catch(cause){if(current())this.publish({moduleError:String(cause instanceof Error?cause.message:cause)});}
    finally{if(current())this.publish({modulesPending:false});}
  }

  private convertModel(asset: ProjectDoc['assets'][number] | undefined, id: string, name: string): Promise<ModelMesh> {
    const key = asset?.sha256 ?? id;
    const existing = this.cache.get(key);
    if (existing) return existing;
    const scope = this.scope;
    const task = (asset ? this.dependencies.loadAsset(asset.sha256) : this.dependencies.modelBytes(id)).then(async bytes => {
      if (this.closed || scope !== this.scope) throw new Error('Model preview superseded');
      if (!bytes) throw new Error('Saved model bytes are missing');
      if (/\.(step|stp)$/iu.test(name)) {
        this.cad ??= this.dependencies.cad();
        return (await this.cad.requestModel(bytes)).mesh;
      }
      return readMeshModel(bytes, name);
    }).catch(cause => {
      if (this.cache.get(key) === task) this.cache.delete(key);
      throw cause;
    });
    this.cache.set(key, task);
    if (this.cache.size > 80) this.cache.delete(this.cache.keys().next().value!);
    return task;
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
        const name = asset?.name ?? bundled!.filename;
        const task = this.convertModel(asset, id!, name);
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

/** Apply the Rust-supplied affine matrix; pose and board-side rules remain in the core. */
function transformedMesh(mesh:ModelMesh,m:number[]):ModelMesh {
  const positions=new Float32Array(mesh.positions.length),normals=new Float32Array(mesh.normals.length);
  const lengths=[0,4,8].map(index=>m[index]**2+m[index+1]**2+m[index+2]**2);
  for(let i=0;i<positions.length;i+=3) {
    const x=mesh.positions[i],y=mesh.positions[i+1],z=mesh.positions[i+2];
    positions[i]=m[0]*x+m[4]*y+m[8]*z+m[12];positions[i+1]=m[1]*x+m[5]*y+m[9]*z+m[13];positions[i+2]=m[2]*x+m[6]*y+m[10]*z+m[14];
    const nx=mesh.normals[i]/lengths[0],ny=mesh.normals[i+1]/lengths[1],nz=mesh.normals[i+2]/lengths[2];
    const a=m[0]*nx+m[4]*ny+m[8]*nz,b=m[1]*nx+m[5]*ny+m[9]*nz,c=m[2]*nx+m[6]*ny+m[10]*nz,length=Math.hypot(a,b,c)||1;
    normals[i]=a/length;normals[i+1]=b/length;normals[i+2]=c/length;
  }
  return {...mesh,positions,normals};
}

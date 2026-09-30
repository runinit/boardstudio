import type { CaseDisplay } from './caseDisplay';
import type { GenerationState } from '../generationState';
import { useEffect, useMemo, useRef, useState } from 'react';
import type {
  BoardReference,
  Contour,
  MechanicalAssembly,
  MechanicalConfiguration,
  PcbPreview,
  PreparedCaseAssemblyIR,
  ProjectDoc,
  Mount,
} from '@boardstudio/v2-contracts';
import { modelAssetId } from '@boardstudio/v2-ergogen';
import { ExportClient } from '../ExportClient';
import { CaseClient } from '../CaseClient';
import { CoreClient } from '../CoreClient';
import type { KeycapResolution } from '@boardstudio/v2-contracts';
import {
  bundledModel,
  bundledModelBytes,
  bundledModels,
} from '../bundledModels';
import { loadAsset } from '../storage';
import { readMeshModel, type ModelMesh } from '../modelMesh';
import {
  AssemblyScene,
  type AssemblyBody,
  type LoadedModel,
} from './AssemblyScene';

const noBodies: AssemblyBody[] = [];
export function AssemblyViewer({
  document,
  boardId,
  contours,
  bodies = noBodies,
  mechanical,
  generation,
  preparedCase,
  onGasketChange,
  onGasketDraft,
  onCaseMountChange,
  onCaseMountDraft,
  onCasePreviewDraft,
  selectedLayer,
  onSelectLayer,
  onSelect,
  colorScheme,
  sample = false,
  display, onDisplayChange, displayKey,
}: {
  document: ProjectDoc;
  boardId: string;
  contours: Contour[];
  bodies?: AssemblyBody[];
  mechanical?: MechanicalAssembly;
  generation?: GenerationState;
  preparedCase?: PreparedCaseAssemblyIR;
  onGasketChange?: (config: MechanicalConfiguration) => void | Promise<boolean>;
  onGasketDraft?: (config: MechanicalConfiguration | null, disposition?: 'commit') => void;
  onCaseMountChange?: (bodyId: string, mounts: Mount[]) => void | Promise<boolean>;
  onCaseMountDraft?: (bodyId: string, mounts: Mount[] | null, disposition?: 'commit') => void;
  onCasePreviewDraft?: (document: ProjectDoc | null, disposition?: 'commit') => void;
  selectedLayer?: string;
  onSelectLayer?: (id: string) => void;
  onSelect?: (reference: string) => void;
  colorScheme: 'light' | 'dark';
  sample?: boolean;
  display?: CaseDisplay;
  onDisplayChange?: (next: CaseDisplay) => void;
  displayKey?: string;
}) {
  const client = useRef<ExportClient | undefined>(undefined),
    cad = useRef<CaseClient | undefined>(undefined);
  const core = useRef<CoreClient | undefined>(undefined);
  const [keycaps, setKeycaps] = useState<AssemblyBody[]>([]);
  const [keycapResolution, setKeycapResolution] = useState<KeycapResolution>();
  const [keycapError, setKeycapError] = useState('');
  const [keycapsPending, setKeycapsPending] = useState(false);
  const allBodies = useMemo(() => [...bodies, ...keycaps], [bodies, keycaps]);
  const cache = useRef(new Map<string, Promise<ModelMesh>>());
  const [board, setBoard] = useState<PcbPreview>(),
    [models, setModels] = useState<LoadedModel[]>([]),
    [messages, setMessages] = useState<string[]>([]),
    [error, setError] = useState(''),
    [pending, setPending] = useState(true),
    [attempt, setAttempt] = useState(0);
  const [shownReference, setShownReference] = useState<BoardReference>();
  const authoredBodies = useMemo(() => document.caseBodies.filter(body => body.boardId === boardId), [document.caseBodies, boardId]);
  const reference = document.boardReferences?.find(
    (r) => r.boardId === boardId && r.enabled,
  );
  const paths = useMemo<[string, string][]>(() => {
    const ids = new Set([
      ...document.assets.map((a) => a.id),
      ...bundledModels().map((m) => m.id),
      ...document.definitions.flatMap((d) => [
        ...(d.models ?? []).map((m) => m.assetId),
      ]),
    ]);
    return [...ids].map((id, index) => [
      id,
      `models/preview/${index}.${(document.assets.find((a) => a.id === id)?.name ?? bundledModel(id)?.filename ?? 'model.step').split('.').pop()}`,
    ]);
  }, [document.assets, document.definitions]);
  const generatedReferences = new Set(document.parts.filter(part => document.keycaps?.keys[part.id]?.profile || document.matrices.some(matrix => matrix.partIds.includes(part.id) && document.keycaps?.matrices[matrix.id]?.profile)).map(part => part.reference));
  const pcbKey = JSON.stringify({ board: document.boards.find(b => b.id === boardId), parts: document.parts,
    generatedReferences: [...generatedReferences], definitions: document.definitions, assets: document.assets, contours, reference, paths });
  useEffect(
    () => () => {
      client.current?.close();
      cad.current?.close();
      core.current?.close();
      cache.current.clear();
    },
    [],
  );
  const keycapKey = JSON.stringify({ id: document.id, revision: document.revision, boardId, keycaps: document.keycaps, parts: document.parts, matrices: document.matrices, hardware: document.hardware, definitions: document.definitions, cases: preparedCase });
  useEffect(() => {
    let current = true;
    const abort = new AbortController();
    setKeycaps([]); setKeycapError(''); setKeycapResolution(undefined);
    if (!document.keycaps) { setKeycapsPending(false); return; }
    setKeycapsPending(true);
    core.current ??= new CoreClient();
    void (async () => {
      const reply = await core.current!.request({ id: crypto.randomUUID(), kind: 'resolve-keycaps', document, boardId, cases: preparedCase ?? null });
      if (!current) return;
      if (reply.kind !== 'keycaps-resolved') throw new Error(reply.kind === 'error' ? reply.message : 'Expected resolved keycaps');
      if (reply.result.revision !== document.revision) return;
      setKeycapResolution(reply.result);
      if (reply.result.findings.some(finding => finding.severity === 'error')) return;
      if (!reply.result.specs.length) return;
      cad.current ??= new CaseClient();
      const result = await cad.current.keycaps(document.revision, reply.result.specs, false, abort.signal);
      if (!current || result.revision !== document.revision) return;
      setKeycaps((result.bodies ?? []).map(body => {
        const isLegend = body.id.startsWith('keycap-legend:');
        const spec = reply.result.specs.find(spec => spec.id === body.id.slice(isLegend ? 14 : 7));
        return { id: body.id, name: body.name, color: isLegend ? spec?.legendColor : spec?.color, mesh: { positions: body.positions, normals: body.normals } };
      }));
    })().catch(cause => { if (current) setKeycapError(String(cause)); }).finally(() => { if (current) setKeycapsPending(false); });
    return () => { current = false; abort.abort(); };
  }, [keycapKey, attempt]);

  useEffect(() => {
    let current = true;
    setPending(true);
    setError('');
    setMessages([]);
    client.current ??= new ExportClient();
    const run = async () => {
      let result: PcbPreview;
      if (reference) {
        const asset = document.assets.find((a) => a.id === reference.assetId);
        const bytes = asset && (await loadAsset(asset.sha256));
        if (!bytes)
          throw new Error('The saved KiCad board is missing. Import it again.');
        const reply = await client.current!.artifact({
          kind: 'preview-board',
          source: new TextDecoder().decode(bytes),
          revision: document.revision,
        });
        if (reply.kind !== 'preview-board')
          throw new Error('Expected routed board preview');
        result = reply.result;
      } else
        result = await client.current!.preview({
          document,
          boardId,
          contours,
          paths,
        });
      if (!current || result.revision !== document.revision) return;
      result = { ...result, models: result.models.filter(model => !(generatedReferences.has(model.reference) && /keycap/i.test(model.path))) };
      setBoard(result);
      setShownReference(reference);
      setModels([]);
      setPending(false);
      setMessages(
        result.models.length
          ? result.diagnostics
          : [
              ...result.diagnostics,
              'No component models are attached. Add models or choose a configured assembly in Parts.',
            ],
      );
      const lookup = new Map(
        paths.map(([id, path]) => [`\${KIPRJMOD}/${path}`, id]),
      );
      const loaded: LoadedModel[] = [];
      await Promise.all(
        result.models.map(async (model) => {
          const id =
            reference?.modelAssets[model.path] ??
            lookup.get(model.path) ??
            modelAssetId(model.path);
          const asset = document.assets.find((a) => a.id === id);
          const bundled = id ? bundledModel(id) : undefined;
          if (!asset && !bundled) {
            if (current)
              setMessages((old) => [
                ...old,
                `${model.reference}: missing model ${model.path}. Attach the model in the inspector.`,
              ]);
            return;
          }
          const name = asset?.name ?? bundled!.filename,
            key = asset?.sha256 ?? id!;
          let task = cache.current.get(key);
          if (!task) {
            task = (asset ? loadAsset(asset.sha256) : bundledModelBytes(id!))
              .then(async (bytes) => {
                if (!bytes) throw new Error('Saved model bytes are missing');
                if (/\.(step|stp)$/i.test(name)) {
                  cad.current ??= new CaseClient();
                  return (await cad.current.requestModel(bytes)).mesh;
                }
                return readMeshModel(bytes, name);
              })
              .catch((cause) => {
                cache.current.delete(key);
                throw cause;
              });
            cache.current.set(key, task);
            if (cache.current.size > 80)
              cache.current.delete(cache.current.keys().next().value!);
          }
          try {
            const mesh = await task;
            if (current) loaded.push({ id: model.id, mesh });
          } catch (cause) {
            if (current)
              setMessages((old) => [
                ...old,
                `${model.reference}: ${String(cause)}`,
              ]);
          }
        }),
      );
      if (current) setModels(loaded);
    };
    void run().catch((cause) => {
      if (current) {
        setError(String(cause instanceof Error ? cause.message : cause));
        setPending(false);
      }
    });
    return () => {
      current = false;
    };
  }, [pcbKey, attempt]);
  return (
    <div className="wb-assembly-view">
      {board && (
        <AssemblyScene
          board={board}
          models={models}
          bodies={allBodies}
          mechanical={mechanical}
          generation={generation}
          preparedCase={preparedCase}
          onGasketChange={onGasketChange}
          onGasketDraft={(configuration, disposition) => { onGasketDraft?.(configuration, disposition); onCasePreviewDraft?.(configuration ? { ...document, mechanical: configuration } : null, disposition); }}
          onCaseMountChange={onCaseMountChange}
          onCaseMountDraft={(bodyId, mounts, disposition) => { onCaseMountDraft?.(bodyId, mounts, disposition); const body = document.caseBodies.find((entry) => entry.id === bodyId); onCasePreviewDraft?.(body && mounts ? { ...document, caseBodies: document.caseBodies.map((entry) => entry.id === bodyId ? { ...body, mounts } : entry) } : null, disposition); }}
          authoredCaseBodies={authoredBodies}
          mechanicalConfiguration={document.mechanical}
          display={display} onDisplayChange={onDisplayChange}
          selectedLayer={selectedLayer}
          onSelectLayer={onSelectLayer}
          reference={shownReference}
        colorScheme={colorScheme}
        key={`${document.id}:${displayKey ?? boardId}`}
        persistenceKey={`${document.id}:${displayKey ?? boardId}`}
          onSelect={id => onSelect?.(document.parts.find(part => part.id === id.replace(/^keycap(?:-legend)?:/, ''))?.reference ?? id)}
        />
      )}
      {keycapsPending && <p className="wb-assembly-loading" role="status">Generating keycap CAD…</p>}
      {keycapError && <div role="alert" className="wb-assembly-error">{keycapError}<button onClick={() => setAttempt(a => a + 1)}>Retry keycaps</button></div>}
      {keycapResolution && keycapResolution.findings.length > 0 && <details className="wb-assembly-notices" open><summary>Keycap clearance · {keycapResolution.findings.length} findings</summary>{keycapResolution.findings.map(finding => <p key={finding.id}>{finding.message}</p>)}</details>}
      {pending && (
        <p className="wb-assembly-loading" role="status">
          Preparing PCB assembly…
        </p>
      )}
      {(sample || messages.length > 0) && (
        <details className="wb-assembly-notices">
          <summary>
            {sample ? 'Sample PCB' : `${messages.length} model ${messages.length === 1 ? 'notice' : 'notices'}`}
          </summary>
          {sample && <p>Sample board around this footprint assembly.</p>}
          {messages.map((m, i) => (
            <p key={i}>{m}</p>
          ))}
          {messages.length > 0 && (
            <button onClick={() => setAttempt((a) => a + 1)}>
              Retry models
            </button>
          )}
        </details>
      )}
      {error && (
        <div className="wb-assembly-error" role="alert">
          <p>{error}</p>
          <button onClick={() => setAttempt((a) => a + 1)}>
            Retry preview
          </button>
        </div>
      )}
    </div>
  );
}

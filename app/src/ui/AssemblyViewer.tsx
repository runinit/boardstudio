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
  const pcbKey = JSON.stringify({ board: document.boards.find(b => b.id === boardId), parts: document.parts,
    definitions: document.definitions, assets: document.assets, contours, reference, paths });
  useEffect(
    () => () => {
      client.current?.close();
      cad.current?.close();
      cache.current.clear();
    },
    [],
  );
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
          bodies={bodies}
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
          onSelect={onSelect}
        />
      )}
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

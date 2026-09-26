import type { CasePreviewResult, CadProgress } from '@boardstudio/v2-cad';
import type { CaseAssemblyIR, MechanicalAssembly, ProjectDoc, SceneDelta } from '@boardstudio/v2-contracts';
import type { MutableRefObject } from 'react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { CaseClient } from './CaseClient';
import { CoreClient } from './CoreClient';
import { caseAssembly } from './caseAssembly';
import type { CasePreviewContext, ContextualCaseResult } from './casePreviewContext';
import { casePreviewContextMatches, reusableCaseResult } from './casePreviewContext';
import type { GenerationState } from './generationState';
import { effectiveCaseDocument, effectiveCaseScene, mechanicalFingerprint } from './hardwareInstances';
import { prepareCase } from './prepareCase';
import { resolveMechanical } from './resolveMechanical';

export async function generateCasePreview(
  core: CoreClient,
  getCad: () => CaseClient,
  ir: CaseAssemblyIR,
  isCurrent: () => boolean,
  onProgress: (progress: CadProgress) => void,
): Promise<CasePreviewResult | undefined> {
  const prepared = await prepareCase(core, ir);
  if (!isCurrent()) return undefined;
  const result = await getCad().preview(prepared, onProgress);
  if (!isCurrent()) return undefined;
  if (result.revision !== ir.revision) throw new Error('CAD returned a different case revision');
  return result;
}

type Inputs = {
  project: ProjectDoc;
  scene: SceneDelta;
  selectedBoardId: string;
  selectedInstance: NonNullable<ProjectDoc['hardware']>['instances'][number] | undefined;
  projectSession: number;
  projectRef: MutableRefObject<ProjectDoc>;
  committedScene: MutableRefObject<SceneDelta>;
  client: MutableRefObject<CoreClient | null>;
  caseClient: MutableRefObject<CaseClient | null>;
  previewCache: MutableRefObject<Map<string, ContextualCaseResult<CasePreviewResult>>>;
  activeMode: string;
  ready: boolean;
  setError: (message: string) => void;
};

export function useCaseGeneration({ project, scene, selectedBoardId, selectedInstance, projectSession, projectRef, committedScene, client, caseClient, previewCache, activeMode, ready, setError }: Inputs) {
  const [casePreview, setCasePreview] = useState<ContextualCaseResult<CasePreviewResult> | undefined>();

  const [mechanicalAssembly, setMechanicalAssembly] = useState<ContextualCaseResult<MechanicalAssembly> | undefined>();

  const [generation, setGeneration] = useState<GenerationState>({ status: 'required' });

  const generationSeq = useRef(0);

  const generationRunning = useRef(false);

  const caseSeq = useRef(0);

  const physicalDocument = useMemo(() => effectiveCaseDocument(project, selectedInstance), [project, selectedInstance]);

  const physicalScene = useMemo(() => effectiveCaseScene(project, scene, selectedInstance), [project, scene, selectedInstance]);

  const previewCacheKey = `${projectSession}/${project.id}/${selectedInstance?.id ?? selectedBoardId}`;

  const previewContext: CasePreviewContext = {
    documentId: project.id, boardId: selectedBoardId, revision: project.revision,
    scene, committedScene: committedScene.current,
    instanceId: selectedInstance?.id, session: projectSession,
    mechanicalFingerprint: mechanicalFingerprint(project, scene, selectedBoardId, selectedInstance),
  };

  const currentPreviewContext = useRef(previewContext);
  currentPreviewContext.current = previewContext;

  const visibleCasePreview = casePreview?.context.documentId === project.id && casePreview.context.boardId === selectedBoardId && casePreview.context.instanceId === selectedInstance?.id ? casePreview.result : undefined;

  const visibleMechanicalAssembly = mechanicalAssembly?.context.documentId === project.id && mechanicalAssembly.context.boardId === selectedBoardId && mechanicalAssembly.context.instanceId === selectedInstance?.id ? mechanicalAssembly.result : undefined;

  useEffect(() => {
    generationSeq.current += 1;
    if (generationRunning.current) { caseClient.current?.cancel(); generationRunning.current = false; }
    const cached = previewCache.current.get(previewCacheKey);
    const reusable = reusableCaseResult(cached, currentPreviewContext.current);
    if (reusable) setCasePreview({ context: currentPreviewContext.current, result: reusable });
    else if (cached) setCasePreview(cached);
    setGeneration(reusable ? { status: 'ready', revision: project.revision } : { status: 'required' });
  }, [project, scene, selectedBoardId, selectedInstance?.id, previewCacheKey]);

  useEffect(() => {
    const sequence = ++caseSeq.current;
    const context = currentPreviewContext.current;
    const isCurrent = () => sequence === caseSeq.current && casePreviewContextMatches(context, {
      ...currentPreviewContext.current, committedScene: committedScene.current,
    });
    if (activeMode !== 'Case') {
      return;
    }

    const boardReady = scene.boardReadiness.find((entry) => entry.boardId === selectedBoardId);

    if (scene !== committedScene.current) {
      return;
    }

    const generated = physicalDocument.mechanical?.boardId === selectedBoardId;
    if (!ready || !(generated ? boardReady?.outline : boardReady?.case)) {
      setMechanicalAssembly(undefined);
      setCasePreview(undefined);
      return;
    }

    const timer = setTimeout(async () => {
      try {
        if (generated) {
          const contours = physicalScene.boardContours.find((entry) => entry.boardId === selectedBoardId)?.contours ?? [];
          const assembly = await resolveMechanical(client.current!, physicalDocument, contours, isCurrent);
          if (!assembly) return;
          setMechanicalAssembly({ context, result: assembly });
          if (assembly.generationBlocked) {
            setGeneration({ status: 'blocked' });
            return;
          }
        } else {
          setMechanicalAssembly(undefined);
        }
      } catch (cause) {
        if (isCurrent()) {
          setError(String(cause));
        }
      }
    }, 100);

    return () => {
      clearTimeout(timer);
      if (sequence === caseSeq.current) caseSeq.current += 1;
    };
  }, [activeMode, ready, physicalDocument, physicalScene, selectedBoardId]);

  function cancelGeneration(): void {
    generationSeq.current += 1;
    generationRunning.current = false;
    caseClient.current?.cancel();
    setGeneration({ status: 'cancelled' });
  }

  async function generateCase(): Promise<void> {
    if (generationRunning.current) return;
    const context = currentPreviewContext.current;
    const document = effectiveCaseDocument(projectRef.current, selectedInstance);
    const sequence = ++generationSeq.current;
    const isCurrent = () => sequence === generationSeq.current && casePreviewContextMatches(context, currentPreviewContext.current);
    generationRunning.current = true;
    setGeneration({ status: 'preparing', revision: document.revision });
    try {
      // Allow the progress state to paint before scheduling preparation.
      await new Promise(resolve => requestAnimationFrame(() => setTimeout(resolve, 0)));
      const generatedScene = effectiveCaseScene(projectRef.current, context.scene, selectedInstance);
      let ir = caseAssembly(document, generatedScene, context.boardId);
      if (document.mechanical?.boardId === context.boardId) {
        const contours = generatedScene.boardContours.find(entry => entry.boardId === context.boardId)?.contours ?? [];
        const assembly = await resolveMechanical(client.current!, document, contours, isCurrent);
        if (!assembly || !isCurrent()) return;
        setMechanicalAssembly({ context, result: assembly });
        if (assembly.generationBlocked) { setGeneration({ status: 'blocked' }); return; }
        ir = assembly.case;
      }
      let started = 0;
      const result = await generateCasePreview(client.current!, () => {
        caseClient.current ??= new CaseClient();
        started = performance.now();
        return caseClient.current;
      }, ir, isCurrent, progress => {
        if (isCurrent()) setGeneration({ status: 'running', revision: document.revision, progress });
      });
      if (!result) return;
      if (!isCurrent()) return;
      performance.measure('boardstudio.cad.preview', { start: started, end: performance.now() });
      setCasePreview({ context, result });
      previewCache.current.set(previewCacheKey, { context, result });
      setGeneration({ status: 'ready', revision: result.revision });
    } catch (cause) {
      if (isCurrent()) setGeneration({ status: 'failed', message: String(cause) });
    } finally {
      if (sequence === generationSeq.current) generationRunning.current = false;
    }
  }

  return { physicalDocument, physicalScene, generation, currentPreviewContext, visibleCasePreview, visibleMechanicalAssembly, cancelGeneration, generateCase };
}

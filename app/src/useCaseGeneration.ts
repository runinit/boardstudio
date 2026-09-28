import type { CadProgress } from '@boardstudio/v2-cad';
import { bodyKey, type BodyPreview } from '../../cad/src/preview';
import type { CaseAssemblyIR, MechanicalAssembly, PreparedCaseAssemblyIR, ProjectDoc, SceneDelta } from '@boardstudio/v2-contracts';
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
import { LatestPreviewScheduler } from './livePreviewScheduler';
import { recordCadMeasure, trackCadPreview } from './cadPerformance';

export type PreparedCasePreview = BodyPreview & { prepared: PreparedCaseAssemblyIR };

export async function generateCasePreview(
  core: CoreClient,
  getCad: () => CaseClient,
  ir: CaseAssemblyIR,
  isCurrent: () => boolean,
  onProgress: (progress: CadProgress) => void,
  signal?: AbortSignal,
  candidate?: PreparedCasePreview,
): Promise<PreparedCasePreview | undefined> {
  const prepared = await prepareCase(core, ir);
  if (!isCurrent()) return undefined;
  if (candidate && candidate.bodies.length === prepared.bodies.length
    && candidate.prepared.bodies.length === prepared.bodies.length
    && prepared.bodies.every((body, index) => {
      const previous = candidate.prepared.bodies[index];
      return body.body.id === previous.body.id && body.body.name === previous.body.name && bodyKey(body) === bodyKey(previous);
    })) {
    recordCadMeasure('boardstudio.cad.draft-reuse', { start: performance.now(), duration: 0, detail: { revision: ir.revision } });
    return { revision: ir.revision, bodies: candidate.bodies, prepared };
  }
  const result = await getCad().preview(prepared, onProgress, ...(signal ? [signal] : []));
  if (!isCurrent()) return undefined;
  if (result.revision !== ir.revision) throw new Error('CAD returned a different case revision');
  return { revision: result.revision, bodies: result.bodies, prepared };
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
  previewCache: MutableRefObject<Map<string, ContextualCaseResult<PreparedCasePreview>>>;
  activeMode: string;
  ready: boolean;
  setError: (message: string) => void;
};
type Draft = { context: CasePreviewContext; document: ProjectDoc; sequence: number; committing?: boolean };
type PreviewJob = {
  context: CasePreviewContext; document: ProjectDoc; scene: SceneDelta;
  cacheKey: string; sequence: number; draftSequence?: number; started: number;
};
const sameIdentity = (a: CasePreviewContext, b: CasePreviewContext) => a.documentId === b.documentId
  && a.boardId === b.boardId && a.instanceId === b.instanceId && a.session === b.session;

export function useCaseGeneration({ project, scene, selectedBoardId, selectedInstance, projectSession, committedScene, client, caseClient, previewCache, activeMode, ready, setError }: Inputs) {
  const [casePreview, setCasePreview] = useState<ContextualCaseResult<PreparedCasePreview>>();
  const [draftPreview, setDraftPreview] = useState<ContextualCaseResult<PreparedCasePreview>>();
  const [mechanicalAssembly, setMechanicalAssembly] = useState<ContextualCaseResult<MechanicalAssembly>>();
  const [state, setGeneration] = useState<GenerationState>({ status: 'required' });
  const [livePreview, setLiveEnabled] = useState(true);
  const [draftActive, setDraftActive] = useState(false);
  const liveRef = useRef(true);
  const draft = useRef<Draft | null>(null);
  const completedDraft = useRef<ContextualCaseResult<PreparedCasePreview> | undefined>(undefined);
  const draftSequence = useRef(0);
  const requestSequence = useRef(0);
  const activePreview = useRef<AbortController | null>(null);
  const mounted = useRef(true);
  const runRef = useRef<(job: PreviewJob) => Promise<void>>(async () => {});
  const scheduler = useRef<LatestPreviewScheduler<PreviewJob> | null>(null);
  scheduler.current ??= new LatestPreviewScheduler(job => runRef.current(job));

  const physicalDocument = useMemo(() => effectiveCaseDocument(project, selectedInstance), [project, selectedInstance]);
  const physicalScene = useMemo(() => effectiveCaseScene(project, scene, selectedInstance), [project, scene, selectedInstance]);
  const previewCacheKey = `${projectSession}/${project.id}/${selectedInstance?.id ?? selectedBoardId}`;
  const fingerprint = useMemo(() => mechanicalFingerprint(project, scene, selectedBoardId, selectedInstance), [project, scene, selectedBoardId, selectedInstance]);
  const previewContext: CasePreviewContext = {
    documentId: project.id, boardId: selectedBoardId, revision: project.revision,
    scene, committedScene: committedScene.current, instanceId: selectedInstance?.id,
    session: projectSession, mechanicalFingerprint: fingerprint,
  };
  const currentPreviewContext = useRef(previewContext);
  currentPreviewContext.current = previewContext;
  const current = useRef({ activeMode, ready, physicalDocument, physicalScene, previewCacheKey });
  current.current = { activeMode, ready, physicalDocument, physicalScene, previewCacheKey };

  function eligible(): boolean {
    const context = currentPreviewContext.current;
    const input = current.current;
    const board = context.scene.boardReadiness.find(entry => entry.boardId === context.boardId);
    const generated = input.physicalDocument.mechanical?.boardId === context.boardId;
    return mounted.current && input.ready && input.activeMode === 'Case' && Boolean(client.current)
      && context.scene === committedScene.current && context.scene.revision === context.revision
      && Boolean(generated ? board : board?.case);
  }

  function isCurrent(job: PreviewJob): boolean {
    return eligible() && job.sequence === requestSequence.current
      && casePreviewContextMatches(job.context, currentPreviewContext.current)
      && (job.draftSequence === undefined ? !draft.current : draft.current?.sequence === job.draftSequence);
  }

  runRef.current = async job => {
    if (!isCurrent(job)) return;
    const controller = new AbortController();
    activePreview.current = controller;
    const started = performance.now();
    setGeneration({ status: 'preparing', revision: job.context.revision });
    try {
      let ir = caseAssembly(job.document, job.scene, job.context.boardId);
      if (job.document.mechanical?.boardId === job.context.boardId) {
        const contours = job.scene.boardContours.find(entry => entry.boardId === job.context.boardId)?.contours ?? [];
        const assembly = await resolveMechanical(client.current!, job.document, contours, () => isCurrent(job));
        if (!assembly || !isCurrent(job)) return;
        // Gesture geometry is presentation-only; inspector diagnostics remain committed.
        if (job.draftSequence === undefined) setMechanicalAssembly({ context: job.context, result: assembly });
        if (assembly.generationBlocked) { setGeneration({ status: 'blocked' }); return; }
        ir = assembly.case;
      }
      const result = await generateCasePreview(client.current!, () => {
        caseClient.current ??= new CaseClient();
        return caseClient.current;
      }, ir, () => isCurrent(job), progress => {
        if (isCurrent(job)) setGeneration({ status: 'running', revision: job.context.revision, progress });
      }, controller.signal, job.draftSequence === undefined && completedDraft.current
        && sameIdentity(completedDraft.current.context, job.context) ? completedDraft.current.result : undefined);
      if (!result || !isCurrent(job)) return;
      recordCadMeasure('boardstudio.cad.preview', { start: job.started, end: performance.now(), detail: { revision: result.revision, draft: job.draftSequence !== undefined } });
      trackCadPreview(result, job.started, () => isCurrent(job));
      if (job.draftSequence !== undefined) {
        completedDraft.current = { context: job.context, result };
        setDraftPreview({ context: job.context, result });
        setGeneration({ status: 'required', message: 'Placement preview · release to apply' });
      } else {
        const completed = { context: job.context, result };
        previewCache.current.set(job.cacheKey, completed);
        setCasePreview(completed);
        setDraftPreview(undefined);
        completedDraft.current = undefined;
        setGeneration({ status: 'ready', revision: result.revision });
      }
    } catch (cause) {
      if (isCurrent(job)) setGeneration({ status: 'failed', message: String(cause) });
    } finally {
      if (activePreview.current === controller) activePreview.current = null;
      recordCadMeasure('boardstudio.cad.job', { start: job.started, end: performance.now(), detail: {
        revision: job.context.revision, draft: job.draftSequence !== undefined,
        queueMs: started - job.started, obsolete: !isCurrent(job),
      } });
    }
  };

  function enqueue(): Promise<void> {
    if (!eligible()) return Promise.resolve();
    activePreview.current?.abort();
    const context = currentPreviewContext.current;
    const input = current.current;
    const candidate = draft.current;
    const job: PreviewJob = {
      context, document: candidate?.document ?? input.physicalDocument, scene: input.physicalScene,
      cacheKey: input.previewCacheKey, sequence: ++requestSequence.current,
      draftSequence: candidate?.sequence, started: performance.now(),
    };
    setGeneration({ status: 'preparing', revision: context.revision });
    return scheduler.current!.enqueue(job).then(() => {});
  }

  // Changes supersede work without terminating the warm kernel. The scheduler
  // drains its current call before starting the newest captured request.
  useEffect(() => {
    requestSequence.current += 1;
    activePreview.current?.abort();
    scheduler.current!.cancel();
    if (draft.current && !casePreviewContextMatches(draft.current.context, currentPreviewContext.current)) {
      if (draft.current.committing && draftPreview && sameIdentity(draft.current.context, currentPreviewContext.current)) {
        setCasePreview(draftPreview);
      }
      draft.current = null; setDraftActive(false); setDraftPreview(undefined);
    }
    const context = currentPreviewContext.current;
    const cached = previewCache.current.get(previewCacheKey);
    const reusable = reusableCaseResult(cached, context);
    if (reusable) setCasePreview({ context, result: reusable });
    else if (cached && !completedDraft.current) setCasePreview(cached);
    setGeneration(reusable ? { status: 'ready', revision: context.revision } : { status: 'required' });
    if (!eligible()) { draft.current = null; setDraftActive(false); setDraftPreview(undefined); return; }
    if (liveRef.current && !reusable) {
      const timer = setTimeout(() => { void enqueue(); }, 0);
      return () => clearTimeout(timer);
    }
  }, [project, scene, selectedBoardId, selectedInstance?.id, projectSession, activeMode, ready, livePreview]);

  // Resolving the inspector must also work when automatic solids are paused or
  // the exact mesh was reused across a nonmechanical edit.
  useEffect(() => {
    if (!eligible() || physicalDocument.mechanical?.boardId !== selectedBoardId) return;
    const context = currentPreviewContext.current;
    let stopped = false;
    const valid = () => !stopped && mounted.current && current.current.activeMode === 'Case'
      && casePreviewContextMatches(context, currentPreviewContext.current);
    const timer = setTimeout(async () => {
      if (liveRef.current && !reusableCaseResult(previewCache.current.get(previewCacheKey), context)) return;
      try {
        const contours = physicalScene.boardContours.find(entry => entry.boardId === selectedBoardId)?.contours ?? [];
        const result = await resolveMechanical(client.current!, physicalDocument, contours, valid);
        if (result && valid()) {
          setMechanicalAssembly({ context, result });
          if (result.generationBlocked) setGeneration({ status: 'blocked' });
        }
      } catch (cause) { if (valid()) setError(String(cause)); }
    }, 80);
    return () => { stopped = true; clearTimeout(timer); };
  }, [physicalDocument, physicalScene, selectedBoardId, activeMode, ready, livePreview]);

  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; requestSequence.current += 1; activePreview.current?.abort(); scheduler.current!.cancel(); };
  }, []);

  function setLivePreview(enabled: boolean): void {
    liveRef.current = enabled;
    setLiveEnabled(enabled);
    if (!enabled) {
      requestSequence.current += 1;
      activePreview.current?.abort();
      scheduler.current!.cancel();
      setGeneration(previous => previous.status === 'ready' ? previous : { status: 'required' });
    }
  }

  function cancelGeneration(): void {
    setLivePreview(false);
    draft.current = null; setDraftActive(false); setDraftPreview(undefined);
    caseClient.current?.cancel();
    setGeneration({ status: 'cancelled' });
  }

  function setPreviewDraft(document: ProjectDoc | null, disposition?: 'commit'): void {
    const context = currentPreviewContext.current;
    if (document && (document.id !== context.documentId || document.revision !== context.revision || !eligible())) return;
    requestSequence.current += 1;
    activePreview.current?.abort();
    scheduler.current!.cancel();
    draft.current = document ? { context, document, sequence: ++draftSequence.current, committing: disposition === 'commit' } : null;
    setDraftActive(Boolean(document));
    if (document) {
      if (disposition === 'commit') setGeneration({ status: 'required', message: 'Saving placement…' });
      else if (liveRef.current) void enqueue();
    } else {
      completedDraft.current = undefined;
      setDraftPreview(undefined);
      const reusable = reusableCaseResult(previewCache.current.get(current.current.previewCacheKey), context);
      setGeneration(reusable ? { status: 'ready', revision: context.revision } : { status: 'required' });
      if (liveRef.current && !reusable) void enqueue();
    }
  }

  const committedVisible = casePreview && sameIdentity(casePreview.context, previewContext) ? casePreview.result : undefined;
  const visibleCasePreview = draftActive && draftPreview && sameIdentity(draftPreview.context, previewContext) ? draftPreview.result : committedVisible;
  const visibleMechanicalAssembly = mechanicalAssembly && sameIdentity(mechanicalAssembly.context, previewContext) ? mechanicalAssembly.result : undefined;
  const preparedCase = reusableCaseResult(casePreview, previewContext)?.prepared;
  const generation: GenerationState = { ...state, live: livePreview, draft: draftActive };
  return { physicalDocument, physicalScene, preparedCase, generation, currentPreviewContext, visibleCasePreview, visibleMechanicalAssembly,
    cancelGeneration, generateCase: enqueue, livePreview, setLivePreview, setPreviewDraft };
}

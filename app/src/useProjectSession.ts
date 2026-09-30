import type { PreparedCasePreview } from './useCaseGeneration';
import { emptyProject, type CoreReply, type ProjectDoc, type SceneDelta } from '@boardstudio/v2-contracts';
import type { MutableRefObject } from 'react';
import { useEffect, useRef, useState } from 'react';
import { CaseClient } from './CaseClient';
import { CoreClient } from './CoreClient';
import { ExportClient } from './ExportClient';
import type { ContextualCaseResult } from './casePreviewContext';
import { activeProjectId, listProjects, loadProject, saveProject } from './storage';
import { recordCadMeasure } from './cadPerformance';

const EMPTY_SCENE: SceneDelta = {
  revision: 0,
  transactionId: 'initial',
  changedIds: [],
  transforms: [],
  matrixScenes: [],
  contours: [],
  boardContours: [],
  boardReadiness: [],
  findings: [],
  readiness: { layout: false, outline: false, pcb: false, case: false },
};

type Inputs = {
  caseClient: MutableRefObject<CaseClient | null>;
  exportClient: MutableRefObject<ExportClient | null>;
  previewCache: MutableRefObject<Map<string, ContextualCaseResult<PreparedCasePreview>>>;
  setSelectedInstanceId: (id: string) => void;
  setError: (message: string) => void;
};

export function useProjectSession({ caseClient, exportClient, previewCache, setSelectedInstanceId, setError }: Inputs) {
  const [project, setProject] = useState<ProjectDoc>(() => emptyProject('', ''));
  const [hasProject, setHasProject] = useState(false);

  const [selectedBoardId, setSelectedBoardId] = useState('main-board');

  const [scene, setScene] = useState<SceneDelta>(EMPTY_SCENE);

  const [ready, setReady] = useState(false);

  const client = useRef<CoreClient | null>(null);

  const [projectSession, setProjectSession] = useState(0);

  const [saveStatus, setSaveStatus] = useState<'saving' | 'saved' | 'failed'>('saving');

  const projectRef = useRef(project);

  const committedScene = useRef(scene);

  const queue = useRef<Promise<void>>(Promise.resolve());

  async function accept(reply: CoreReply, mode: 'open' | 'commit' | 'preview'): Promise<void> {
    if (reply.kind === 'error') {
      throw new Error(reply.message);
    }
    if (reply.kind !== 'scene' && reply.kind !== 'preview') return;

    if (mode !== 'open' && reply.scene.revision < projectRef.current.revision) {
      return;
    }

    if (reply.kind === 'preview') {
      setScene(reply.scene);
      return;
    }

    if (mode === 'open') {
      previewCache.current.clear();
      setSelectedInstanceId('');
    }

    setSaveStatus('saving');
    const saveStarted = performance.now();
    let saved = false;
    try {
      await saveProject(reply.document);
      saved = true;
      setSaveStatus('saved');
    } catch (cause) {
      setSaveStatus('failed');
      throw cause;
    } finally {
      recordCadMeasure('boardstudio.cad.persistence', { start: saveStarted, end: performance.now(),
        detail: { revision: reply.document.revision, mode, saved } });
    }
    if (mode === 'open') {
      setProjectSession((value) => value + 1);
      setHasProject(true);
      setReady(true);
      setError('');
    }
    projectRef.current = reply.document;
    setProject(reply.document);
    setScene(reply.scene);
    setSelectedBoardId((current) => reply.document.boards.some((board) => board.id === current)
      ? current
      : reply.document.boards[0]?.id ?? '');

    committedScene.current = reply.scene;
  }

  function schedule(work: () => Promise<void>): Promise<boolean> {
    const pending = queue.current.then(work);
    queue.current = pending.catch((cause) => setError(String(cause)));
    return pending.then(() => true, () => false);
  }

  useEffect(() => {
    const core = new CoreClient();

    client.current = core;
    schedule(async () => {
      const saved = await loadProject(activeProjectId(''))
        ?? (await listProjects())[0];
      if (!saved) {
        setReady(true);
        return;
      }
      const reply = await core.request({ id: crypto.randomUUID(), kind: 'open', document: saved });

      await accept(reply, 'open');
    });

    if (import.meta.env.PROD && 'serviceWorker' in navigator) {
      void navigator.serviceWorker.register('./sw.js').catch((cause) => setError(String(cause)));
    }

    return () => {
      core.close();
      caseClient.current?.close();
      exportClient.current?.close();
    };
  }, []);

  return { project, hasProject, scene, selectedBoardId, setSelectedBoardId, ready, client, projectSession, saveStatus, projectRef, committedScene, accept, schedule };
}

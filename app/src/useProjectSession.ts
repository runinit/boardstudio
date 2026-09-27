import type { CasePreviewResult } from '@boardstudio/v2-cad';
import type { CoreReply, ProjectDoc, SceneDelta } from '@boardstudio/v2-contracts';
import type { MutableRefObject } from 'react';
import { useEffect, useRef, useState } from 'react';
import { CaseClient } from './CaseClient';
import { CoreClient } from './CoreClient';
import { ExportClient } from './ExportClient';
import type { ContextualCaseResult } from './casePreviewContext';
import { demoProject } from './demo';
import { activeProjectId, loadProject, saveProject } from './storage';

const STARTER_ID = 'starter';
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
  previewCache: MutableRefObject<Map<string, ContextualCaseResult<CasePreviewResult>>>;
  setSelectedInstanceId: (id: string) => void;
  setError: (message: string) => void;
};

export function useProjectSession({ caseClient, exportClient, previewCache, setSelectedInstanceId, setError }: Inputs) {
  const [project, setProject] = useState<ProjectDoc>(demoProject);

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
    try {
      await saveProject(reply.document);
      setSaveStatus('saved');
    } catch (cause) {
      setSaveStatus('failed');
      throw cause;
    }
    if (mode === 'open') {
      setProjectSession((value) => value + 1);
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

  function schedule(work: () => Promise<void>): void {
    queue.current = queue.current.then(work).catch((cause) => setError(String(cause)));
  }

  useEffect(() => {
    const core = new CoreClient();

    client.current = core;
    schedule(async () => {
      const saved = await loadProject(activeProjectId(STARTER_ID));
      const document = saved ?? demoProject();
      const reply = await core.request({ id: crypto.randomUUID(), kind: 'open', document });

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

  return { project, scene, selectedBoardId, setSelectedBoardId, ready, client, projectSession, saveStatus, projectRef, committedScene, accept, schedule };
}

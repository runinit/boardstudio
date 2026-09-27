import { useEffect, useRef, useState } from 'react';
import { setupGuideStages, type SetupGuideStage } from './setupGuide';

type StoredState = { open?: unknown; currentStep?: unknown };
type SetupGuidePrefs = { open: boolean; currentStep: SetupGuideStage };
type NewProjectRequest = { projectId: string; requestId: string };
const keyFor = (projectId: string) => `boardstudio:v2:setup-guide:${projectId}`;
const validStep = (value: unknown): value is SetupGuideStage => setupGuideStages.includes(value as SetupGuideStage);
export function readSetupGuidePrefs(raw: string | null): SetupGuidePrefs {
  try {
    const value = JSON.parse(raw ?? '{}') as StoredState | null;
    return { open: value?.open === true, currentStep: validStep(value?.currentStep) ? value.currentStep : 'project' };
  } catch { return { open: false, currentStep: 'project' }; }
}

function loadPreferences(projectId: string): SetupGuidePrefs {
  try { return readSetupGuidePrefs(localStorage.getItem(keyFor(projectId))); }
  catch { return { open: false, currentStep: 'project' }; }
}

export function useSetupGuide({ projectId, newProjectRequest }: { projectId: string; newProjectRequest?: NewProjectRequest }) {
  const consumed = useRef(new Set<string>());
  const [state, setState] = useState<{ projectId: string; open: boolean; currentStep: SetupGuideStage }>(() => ({ projectId, ...loadPreferences(projectId) }));
  useEffect(() => {
    setState(current => current.projectId === projectId ? current : { projectId, ...loadPreferences(projectId) });
  }, [projectId]);
  useEffect(() => {
    if (!newProjectRequest || newProjectRequest.projectId !== projectId || state.projectId !== projectId || consumed.current.has(newProjectRequest.requestId)) return;
    consumed.current.add(newProjectRequest.requestId);
    setState((current) => ({ ...current, open: true, currentStep: 'project' }));
  }, [newProjectRequest, projectId, state.projectId]);
  useEffect(() => {
    if (state.projectId !== projectId) return;
    try { localStorage.setItem(keyFor(projectId), JSON.stringify({ open: state.open, currentStep: state.currentStep })); } catch { /* Browser preferences are optional. */ }
  }, [projectId, state]);
  const active = state.projectId === projectId ? state : { projectId, ...loadPreferences(projectId) };
  return {
    open: active.open,
    currentStep: active.currentStep,
    setOpen: (open: boolean) => setState((current) => ({ ...(current.projectId === projectId ? current : active), open })),
    setCurrentStep: (currentStep: SetupGuideStage) => setState((current) => ({ ...(current.projectId === projectId ? current : active), currentStep })),
  };
}

import type { CadProgress } from '@boardstudio/v2-cad';

export type GenerationState = {
  status: 'required' | 'preparing' | 'running' | 'ready' | 'blocked' | 'failed' | 'cancelled';
  revision?: number;
  live?: boolean;
  draft?: boolean;
  progress?: CadProgress;
  message?: string;
};

export function generationMessage(state: GenerationState): string {
  if (state.status === 'preparing') return 'Updating preview…';
  if (state.status === 'running') {
    const progress = state.progress;
    if (!progress || progress.stage === 'loading') return 'Updating preview · loading CAD…';
    return `Updating preview · ${progress.stage === 'building' ? 'Building' : 'Tessellating'} ${progress.body ?? 'assembly'} · ${progress.completed}/${progress.total} parts`;
  }
  if (state.status === 'ready') return state.live === false ? 'Preview paused · geometry current' : 'Preview current';
  if (state.status === 'blocked') return 'Generation blocked · review mechanical findings';
  if (state.status === 'failed') return state.message ?? 'Generation failed';
  if (state.status === 'cancelled') return 'Preview paused · previous geometry retained';
  return state.message ?? (state.live ? 'Waiting to update preview' : 'Preview paused · changes are not yet in the solids');
}

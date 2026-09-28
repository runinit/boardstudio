import { useId, type ReactNode } from 'react';
import type { CaseReadiness } from './caseReadiness';
import { createPortal } from 'react-dom';
import { generationMessage, type GenerationState } from '../generationState';
import './mechanical-assembly.css';

export function CaseGenerationControls({ generation, onGenerate, onCancel, onExport, exportReady = false, children, target, readiness, onConfigure, livePreview, onLivePreviewChange }: {
  onConfigure?: () => void;
  livePreview?: boolean;
  onLivePreviewChange?: (enabled: boolean) => void;
  readiness?: CaseReadiness;
  target?: HTMLElement | null;
  generation?: GenerationState;
  onGenerate?: () => void;
  onCancel?: () => void;
  onExport?: () => void;
  exportReady?: boolean;
  children?: ReactNode;
}) {
  exportReady = readiness?.canExport ?? exportReady;
  const statusId = useId();
  const state = generation ?? { status: 'required' };
  const busy = state.status === 'preparing' || state.status === 'running';
  const progress = busy && state.progress && state.progress.total > 0 ? state.progress : undefined;

  const controls = <section className="wb-case-generation" data-state={state.status} aria-label="Case generation">
    <div className="wb-case-generation-heading"><h3>Case preview</h3>
      {onGenerate && onLivePreviewChange && <label className="wb-live-preview-toggle">
        <input type="checkbox" role="switch" checked={livePreview === true} onChange={event => onLivePreviewChange(event.target.checked)} />
        <span>Live preview</span>
      </label>}
    </div>
    <p id={statusId} className="wb-case-generation-status" role="status" aria-live="polite">{onConfigure && !onGenerate ? 'Configure a mechanical stack or add a case body to begin.' : readiness?.message ?? generationMessage(state)}</p>
    {busy && <progress aria-label="Generation progress" max={progress?.total} value={progress?.completed} />}
    {!readiness && state.status === 'failed' && <p className="wb-mech-hint">Update preview to retry.</p>}
    {!readiness && state.status === 'cancelled' && <p className="wb-mech-hint">Update preview when you are ready to continue.</p>}
    <div className="wb-case-generation-buttons">
      {onConfigure && !onGenerate ? <button type="button" className="wb-primary" onClick={onConfigure}>Configure case</button> : <button type="button" className={!exportReady ? 'wb-primary' : 'wb-secondary'} aria-describedby={statusId} disabled={busy || !onGenerate} onClick={onGenerate}>Update preview</button>}
      {busy && onCancel && <button type="button" className="wb-secondary" onClick={onCancel}>Cancel</button>}
      {onExport && <button type="button" className={exportReady ? 'wb-primary' : 'wb-secondary'} aria-describedby={statusId} disabled={!exportReady || busy} onClick={onExport}>Export geometry</button>}
    </div>
    {!target && children && <div className="wb-case-generation-meta">{children}</div>}
  </section>;
  return target ? <>{createPortal(controls, target)}{children && <div className="wb-case-generation-meta">{children}</div>}</> : controls;
}

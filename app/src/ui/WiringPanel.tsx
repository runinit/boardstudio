import React from 'react';
import './wiring-panel.css';

export type WiringAssignment = {
  id: string;
  label: string;
  detail?: string;
  value?: string;
  locked?: boolean;
  status?: 'assigned' | 'available' | 'conflict' | 'unresolved';
};

type WiringPanelProps = {
  controller?: { name: string; detail?: string };
  controllerOptions?: { id: string; name: string; detail?: string }[];
  selectedControllerId?: string;
  onControllerChange?: (id: string) => void;
  topology?: string;
  onTopologyChange?: (topology: 'matrix' | 'direct') => void;
  assignments?: WiringAssignment[];
  usedPins?: string[];
  freePins?: string[];
  findings?: string[];
  onResolve?: () => void;
  onApply?: () => void;
  onReview?: (assignment: WiringAssignment) => void;
  onToggleLock?: (assignment: WiringAssignment) => void;
  onAssignPin?: (assignmentId: string, pin: string) => void;
  firmwareControls?: React.ReactNode;
  protectedSummary?: string;
  onReviewRemap?: () => void;
  existingConnections?: { names: string[]; pinCount: number; onReplace: () => void };
};

/** Presentation-only wiring inspector. Core remains authoritative for resolution. */
export function WiringPanel({ firmwareControls, controller, controllerOptions = [], selectedControllerId = '', onControllerChange, topology = 'Matrix', onTopologyChange, assignments = [], usedPins = [], freePins = [], findings = [], onResolve, onApply, onReview, onToggleLock, onAssignPin, protectedSummary, onReviewRemap, existingConnections }: WiringPanelProps) {
  return <section className="wb-wiring-panel" aria-label="Electrical wiring">
    <header className="wb-wiring-header"><div><h2>Wiring</h2><p>{controller ? `${controller.name}${controller.detail ? ` · ${controller.detail}` : ''}` : 'No controller selected'}</p></div><span className="wb-mini-tag">{topology}</span></header>
    {(controllerOptions.length > 0 || onTopologyChange) && <div className="wb-wiring-selectors"><label>Controller<select value={selectedControllerId} onChange={(event) => onControllerChange?.(event.target.value)}>{controllerOptions.map((option) => <option value={option.id} key={option.id}>{option.name}{option.detail ? ` · ${option.detail}` : ''}</option>)}</select></label><label>Wiring mode<select value={topology.toLowerCase()} onChange={(event) => onTopologyChange?.(event.target.value as 'matrix' | 'direct')}><option value="matrix">Matrix</option><option value="direct">Direct GPIO</option></select></label></div>}
    {controller && <div className="wb-wiring-pins"><div><strong>Used pins</strong><span>{usedPins.length ? usedPins.join(' · ') : 'None assigned'}</span></div><div><strong>Free pins</strong><span>{freePins.length ? freePins.join(' · ') : 'None available'}</span></div></div>}
    <div className="wb-wiring-actions"><button type="button" className="wb-secondary" onClick={onResolve} disabled={!onResolve}>Resolve automatically</button><button type="button" className="wb-primary" onClick={onApply} disabled={!onApply}>Apply wiring</button></div>
    {protectedSummary && <section className="wb-wiring-protected"><strong>Protected handoff</strong><p>{protectedSummary}</p><details><summary>Review PCB remap</summary><p>Changing a protected assignment creates a new hardware revision. The old PCB and firmware must be regenerated before export.</p><button type="button" className="wb-secondary" onClick={onReviewRemap} disabled={!onReviewRemap}>Start a new PCB revision</button></details></section>}
    {findings.length > 0 && <div className="wb-wiring-findings" role="alert"><strong>Review before handoff</strong>{findings.map((finding) => <p key={finding}>{finding}</p>)}</div>}
    {existingConnections && <details className="wb-wiring-protected"><summary>Review existing connections</summary><p>{existingConnections.pinCount} pin connections already belong to {existingConnections.names.join(', ')}. Switching them to automatic wiring removes these assignments so the board plan can replace them. Other connections stay in place. You can undo this change.</p><button type="button" className="wb-secondary" onClick={existingConnections.onReplace}>Use automatic wiring for these connections</button></details>}
    {firmwareControls}
    <div className="wb-wiring-assignments"><div className="wb-wiring-section-heading"><h3>Assignments</h3><span>{assignments.length}</span></div>{assignments.length ? assignments.map((assignment) => <div className={`wb-wiring-assignment is-${assignment.status ?? 'assigned'}`} key={assignment.id}><div><strong>{assignment.label}</strong>{assignment.detail && <small>{assignment.detail}</small>}</div><label className="wb-wiring-pin"><span className="wb-sr-only">Pin for {assignment.label}</span><select aria-label={`Pin for ${assignment.label}`} value={assignment.value ?? ''} onChange={(event) => onAssignPin?.(assignment.id, event.target.value)} disabled={!onAssignPin || assignment.locked}><option value="">Unresolved</option>{[...new Set([...(assignment.value ? [assignment.value] : []), ...freePins])].map((pin) => <option key={pin} value={pin}>{pin}</option>)}</select></label>{(onToggleLock || assignment.locked) && <button type="button" className="wb-wiring-lock-button" onClick={() => onToggleLock?.(assignment)} disabled={!onToggleLock}>{assignment.locked ? 'Unlock' : 'Lock'}</button>}{assignment.status === 'conflict' && onReview && <button type="button" className="wb-wiring-review" onClick={() => onReview(assignment)}>Review</button>}</div>) : <p className="wb-empty-note">Resolve the board to see controller, matrix, and peripheral assignments.</p>}</div>
  </section>;
}

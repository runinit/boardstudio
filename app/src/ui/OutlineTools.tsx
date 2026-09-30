import type { ReactNode } from 'react';
import type { OutlineDrawMode } from './outlineEditing';

export function OutlineToolIcon({ kind }: { kind: OutlineDrawMode | 'remove' | 'back' | 'add-point' | 'attached' }) {
  return <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    {kind === 'add' && <><path d="M3 4h9v4h5v9H3z" /><path d="M6 10h5M8.5 7.5v5" /></>}
    {kind === 'subtract' && <><rect x="3" y="3" width="14" height="14" rx="1" /><path d="M7 7h6v6H7z" strokeDasharray="2 2" /></>}
    {kind === 'connect' && <><path d="M4 14V6h12v8" /><circle cx="4" cy="15" r="2" /><circle cx="16" cy="15" r="2" /></>}
    {kind === 'remove' && <><path d="M4 5h12M7 5V3h6v2M6 5l1 12h6l1-12M9 8v6M11 8v6" /></>}
    {kind === 'back' && <path d="m8 4-6 6 6 6M3 10h14" />}
    {kind === 'add-point' && <><path d="M3 10h5M12 10h5" /><circle cx="10" cy="10" r="2" /><path d="M10 2v4M8 4h4" /></>}
    {kind === 'attached' && <><path d="m8 6 2-2a3 3 0 0 1 4 4l-2 2M8 10l-2 2a3 3 0 0 0 4 4l2-2M7 13l6-6" /></>}
  </svg>;
}

export const snapGridLabel = (value:number) => value < 0 ? `${-value} mm` : value===0 ? 'Off' : value===.125 ? '⅛u' : value===.25 ? '¼u' : value===.5 ? '½u' : '1u';
export function OutlineGridControl({ value, selection, onChange, label = 'Outline point grid' }: { value:number; selection?:number; onChange:(value:number)=>void; label?:string }) {
  return <label className="wb-outline-grid-control"><span>Grid step</span><select aria-label={label} value={selection??-value} onChange={event=>onChange(selection===undefined?-Number(event.target.value):Number(event.target.value))}>
    <option value={0}>Off</option><option value={.125}>⅛u</option><option value={.25}>¼u</option><option value={.5}>½u</option><option value={1}>1u</option>
    <option value={-1}>1 mm</option><option value={-.5}>0.5 mm</option><option value={-.1}>0.1 mm</option>
  </select></label>;
}

export function OutlineDrawingBar({ mode, count, grid, selection, onGrid, snapControls, onUndo, onCancel, onFinish }: {
  mode: OutlineDrawMode; count: number; grid: number; selection?:number; snapControls?:ReactNode; onGrid: (value: number) => void;
  onUndo: () => void; onCancel: () => void; onFinish: () => void;
}) {
  const minimum = mode === 'connect' ? 2 : 3;
  const remaining = Math.max(0, minimum - count);
  const title = mode === 'add' ? 'Draw addition' : mode === 'subtract' ? 'Draw cutout' : 'Connect points';
  return <div className="wb-outline-drawing-bar" onKeyDown={event => {
    // Let keyboard activation use the focused control instead of the canvas shortcut.
    if (event.key === 'Enter' || event.key === ' ') event.stopPropagation();
  }}>
    <div className="wb-outline-drawing-heading"><OutlineToolIcon kind={mode} /><strong>{title}</strong><span role="status" aria-label="Outline drawing">{count} {count === 1 ? 'point' : 'points'}</span></div>
    <p>{remaining ? `Place ${remaining} ${count ? 'more ' : ''}${remaining === 1 ? 'point' : 'points'} to ${mode === 'connect' ? 'connect' : 'close'}.` : mode === 'connect' ? 'Continue the path or finish this connection.' : 'Continue the edge or finish to close the shape.'}</p>
    <div className="wb-outline-drawing-actions">{snapControls ?? <OutlineGridControl value={grid} selection={selection} onChange={onGrid} />}<button className="wb-secondary" disabled={!count} onClick={onUndo}>Undo point</button><button className="wb-primary" disabled={count < minimum} onClick={onFinish}>{mode === 'connect' ? 'Finish connection' : 'Close outline'}</button><button className="wb-outline-cancel" aria-label="Cancel drawing" onClick={onCancel}>Cancel</button></div>
    <small><kbd>Enter</kbd> finish <kbd>Esc</kbd> cancel <span>Guides snap automatically · Hold Alt to move freely</span></small>
  </div>;
}

import type { Mode } from './workbenchTypes';
import { ArrowIcon, ModeIcon } from './WorkbenchIcons';

const views = [
  { mode: 'Design', label: 'Layout' },
  { mode: 'PCB', label: 'PCB' },
  { mode: 'Case', label: 'Case' },
  { mode: 'Library', label: 'Parts' },
] as const;

export function WorkflowNavigation({ mode, onNavigate }: {
  mode: Mode;
  onNavigate: (mode: Mode) => void;
}) {
  return <div className="wb-workflow-navigation">
    <nav className="wb-workflow-tabs" role="tablist" aria-label="Board workflow">
      {views.map(view => <button type="button" role="tab" key={view.mode} aria-selected={mode === view.mode} onClick={() => onNavigate(view.mode)}><ModeIcon mode={view.mode} />{view.label}</button>)}
    </nav>
    <select className="wb-workflow-select" aria-label="Workspace" value={mode} onChange={event => onNavigate(event.target.value as Mode)}>
      {views.map(view => <option key={view.mode} value={view.mode}>{view.label}</option>)}
      {mode === 'Export' && <option value="Export">Export</option>}
    </select>
  </div>;
}

export function WorkflowReturn({ returnMode, onNavigate }: {
  returnMode: Mode;
  onNavigate: (mode: Mode) => void;
}) {
  return <button type="button" className="wb-back-link" onClick={() => onNavigate(returnMode)}><ArrowIcon />Back to {returnMode === 'Design' ? 'Layout' : returnMode}</button>;
}

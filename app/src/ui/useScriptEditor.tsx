import { useEffect, useState } from 'react';
import type { EditCommand, ProjectDoc, SceneDelta } from '@boardstudio/v2-contracts';
import { FindingList } from './FindingList';
import { ArrowIcon } from './WorkbenchIcons';
import { makeId } from './workbenchGeometry';

type Inputs = {
  document: ProjectDoc; scene: SceneDelta;
  emit: (operation: EditCommand['operation'], ids: string[]) => unknown;
  showFinding: (finding: SceneDelta['findings'][number]) => void;
};

export function useScriptEditor({ document, scene, emit, showFinding }: Inputs) {
  const [activeScriptId, setActiveScriptId] = useState('');
  const [scriptName, setScriptName] = useState('');
  const [scriptSource, setScriptSource] = useState('');
  const [scriptEnabled, setScriptEnabled] = useState(true);
  const activeScript = document.scripts.find(script => script.id === activeScriptId);
  useEffect(() => {
    setScriptName(activeScript?.name ?? '');
    setScriptSource(activeScript?.source ?? '');
    setScriptEnabled(activeScript?.enabled ?? true);
  }, [activeScript?.id, activeScript?.source, activeScript?.enabled]);
  const addScript = () => {
    const script: ProjectDoc['scripts'][number] = { id: makeId(), name: `Script ${document.scripts.length + 1}`, source: '', enabled: false };
    emit({ kind: 'replace-document', document: { ...document, scripts: [...document.scripts, script] } }, [script.id]);
    setActiveScriptId(script.id);
  };

  const applyScript = () => {
    if (!activeScript || !scriptSource.trim()) return;
    const scripts = document.scripts.map((script) => script.id === activeScript.id
      ? { ...script, name: scriptName, source: scriptSource, enabled: scriptEnabled }
      : script);
    emit({ kind: 'replace-document', document: { ...document, scripts } }, [activeScript.id]);
  };

  const panel = <>
      <div className="wb-inspect-head"><h2>Geometry scripts</h2></div>
        <div className="wb-script-heading"><h3 className="wb-subtitle">Scripts</h3><button onClick={addScript}>+ New script</button></div>
        {document.scripts.length > 0 && <label className="wb-script-select-label">Active script<select aria-label="Active script" value={activeScript?.id ?? ''} onChange={(event) => setActiveScriptId(event.target.value)}>
          {document.scripts.map((script) => <option key={script.id} value={script.id}>{script.name}</option>)}
        </select></label>}
        {activeScript ? <div className="wb-script-editor">
          <label className="wb-script-name">Name<input value={scriptName} onChange={(event) => setScriptName(event.target.value)} /></label>
          <label className="wb-script-source-label">Rhai source<textarea spellCheck={false} value={scriptSource} onChange={(event) => setScriptSource(event.target.value)} placeholder="// Describe generated geometry and component groups" /></label>
          <label className="wb-script-enabled"><input type="checkbox" checked={scriptEnabled} onChange={(event) => setScriptEnabled(event.target.checked)} /> Enable on Apply</label>
          <button className="wb-primary wb-script-apply" disabled={!scriptSource.trim()} onClick={applyScript}>Apply script <ArrowIcon /></button>
          <div className="wb-script-diagnostics"><span>Core findings</span><FindingList document={document} onShow={showFinding} findings={scene.findings} /></div>
        </div> : <p className="wb-empty-state">Scripts generate named geometry groups. Apply a script to run it in the core.</p>}
    </>;

  return { panel };
}

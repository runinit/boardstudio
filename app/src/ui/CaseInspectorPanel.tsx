import type { ReactNode } from 'react';
import type { Board, CaseBody, Finding, ProjectDoc } from '@boardstudio/v2-contracts';
import type { GenerationState } from '../generationState';
import { CaseGenerationControls } from './CaseGenerationControls';
import { CaseNumber } from './InspectorControls';
import { InspectorSection } from './InspectorSection';
import { FindingList } from './FindingList';

type Props = {
  bodies: CaseBody[];
  activeCaseBody?: CaseBody;
  selectedBoard?: Board;
  findings: Finding[];
  findingDocument: ProjectDoc;
  onShowFinding: (finding: Finding) => void;
  mechanicalPanel: ReactNode;
  instanceSetup: ReactNode;
  caseActionsTarget: HTMLElement | null;
  generation?: GenerationState;
  canConfigure: boolean;
  previewCurrent: boolean;
  onConfigure: () => void;
  onResolveMechanical?: () => void;
  onCancelGeneration?: () => void;
  setCaseBodyId: (id: string) => void;
  addCaseBody: () => void;
  updateCaseBody: (changes: Partial<CaseBody>) => void;
  addMount: () => void;
  updateMount: (id: string, changes: Partial<NonNullable<CaseBody['mounts']>[number]>) => void;
  removeMount: (id: string) => void;
};

export function CaseInspectorPanel({ bodies, activeCaseBody, selectedBoard, findings, findingDocument, onShowFinding, mechanicalPanel, instanceSetup, caseActionsTarget, generation, canConfigure, previewCurrent, onConfigure, onResolveMechanical, onCancelGeneration, setCaseBodyId, addCaseBody, updateCaseBody, addMount, updateMount, removeMount }: Props) {
  const caseFindings = activeCaseBody ? findings.filter((finding) => finding.scope === 'case' && (finding.targetIds.length === 0 || finding.targetIds.includes(activeCaseBody.id))) : [];
  return <>
    {instanceSetup}
    {mechanicalPanel}
    <div className="wb-panel-rule" />
    <div className="wb-inspect-head"><h2>Case stack</h2><button className="wb-case-new-body" onClick={addCaseBody} disabled={!selectedBoard}>+ New case body</button></div>
    <CaseGenerationControls target={caseActionsTarget} onConfigure={canConfigure ? onConfigure : undefined} generation={generation} onGenerate={activeCaseBody ? onResolveMechanical : undefined} onCancel={onCancelGeneration}>
      <span className="wb-mech-revision">{!activeCaseBody ? 'Add a case body to generate geometry.' : previewCurrent ? 'Preview current' : 'Generate builds the current case solids'}</span>
    </CaseGenerationControls>
    {bodies.some((body) => !selectedBoard || body.boardId === selectedBoard.id) && <div className="wb-case-body-list">{bodies.filter((body) => !selectedBoard || body.boardId === selectedBoard.id).map((body, index) => <button className={`wb-case-body-tab ${body.id === activeCaseBody?.id ? 'is-active' : ''}`} key={body.id} onClick={() => setCaseBodyId(body.id)}>
      <span>{String(index + 1).padStart(2, '0')}</span><strong>{body.name}</strong><small>{body.kind}</small>
    </button>)}</div>}
    {!activeCaseBody ? <div className="wb-case-empty"><p>{selectedBoard ? 'Add a plate, tray, or lid to begin the case stack.' : 'Add a board before creating a case body.'}</p></div> : <>
      <div className="wb-case-controls">
        <label className="wb-case-select"><span>Body type</span><select value={activeCaseBody.kind} onChange={(event) => {
          const kind = event.target.value as CaseBody['kind'];
          updateCaseBody({ kind, wallHeight: activeCaseBody.wallHeight ?? 14, wallThickness: activeCaseBody.wallThickness ?? 2 });
        }}>
          <option value="plate">Plate</option><option value="tray">Tray</option><option value="lid">Lid</option>
        </select></label>
        <div className="wb-case-measures">
          <CaseNumber label="Thickness" value={activeCaseBody.thickness} unit="mm" validation="positive" onCommit={(value) => updateCaseBody({ thickness: value })} />
          <CaseNumber label="Clearance" value={activeCaseBody.clearance} unit="mm" validation="nonnegative" onCommit={(value) => updateCaseBody({ clearance: value })} />
          <CaseNumber label="Z offset" value={activeCaseBody.z ?? 0} unit="mm" validation="finite" onCommit={(value) => updateCaseBody({ z: value })} />
          {activeCaseBody.kind !== 'plate' && <><CaseNumber label="Wall height" value={activeCaseBody.wallHeight ?? 14} unit="mm" validation="positive" onCommit={(value) => updateCaseBody({ wallHeight: value })} />
          <CaseNumber label="Wall thickness" value={activeCaseBody.wallThickness ?? 2} unit="mm" validation="positive" onCommit={(value) => updateCaseBody({ wallThickness: value })} /></>}
        </div>
      </div>
      <InspectorSection title="Mounting" detail={`${activeCaseBody.mounts?.length ?? 0} mounts`} defaultOpen={Boolean(activeCaseBody.mounts?.length)}>
      <div className="wb-case-section-head"><h3 className="wb-subtitle">Mounts <small>{activeCaseBody.mounts?.length ?? 0}</small></h3><button onClick={addMount}>+ Add mount</button></div>
      {(activeCaseBody.mounts ?? []).map((mount, index) => <div className="wb-mount-editor" key={mount.id}>
        <div className="wb-mount-head"><strong>Mount {index + 1}</strong><button aria-label={`Remove mount ${index + 1}`} onClick={() => removeMount(mount.id)}>Remove</button></div>
        <label className="wb-case-select"><span>Type</span><select value={mount.kind} onChange={(event) => updateMount(mount.id, { kind: event.target.value as 'hole' | 'boss' })}><option value="hole">Hole</option><option value="boss">Boss</option></select></label>
        <div className="wb-case-measures">
          <CaseNumber label="X position" value={mount.at.x} unit="mm" validation="finite" onCommit={(value) => updateMount(mount.id, { at: { ...mount.at, x: value } })} />
          <CaseNumber label="Y position" value={mount.at.y} unit="mm" validation="finite" onCommit={(value) => updateMount(mount.id, { at: { ...mount.at, y: value } })} />
          <CaseNumber label="Hole diameter" value={mount.holeDiameter} unit="mm" validation="positive" onCommit={(value) => updateMount(mount.id, { holeDiameter: value })} />
          {mount.kind === 'boss' && <>
            <CaseNumber label="Boss diameter" value={mount.bossDiameter ?? 5} unit="mm" validation="positive" onCommit={(value) => updateMount(mount.id, { bossDiameter: value })} />
            <CaseNumber label="Height" value={mount.height ?? 5} unit="mm" validation="positive" onCommit={(value) => updateMount(mount.id, { height: value })} />
          </>}
        </div>
      </div>)}
      </InspectorSection>
      <InspectorSection title="Gasket channel" detail={activeCaseBody.gasket ? "Configured" : "Optional"} defaultOpen={Boolean(activeCaseBody.gasket)}>
      <div className="wb-case-section-head"><h3 className="wb-subtitle">Gasket channel</h3>{activeCaseBody.gasket ? <button onClick={() => updateCaseBody({ gasket: undefined })}>Remove</button> : <button onClick={() => updateCaseBody({ gasket: { inset: 2, width: 2, depth: 1.5 } })}>+ Add gasket</button>}</div>
      {activeCaseBody.gasket && <div className="wb-case-measures wb-gasket-measures">
        <CaseNumber label="Inset" value={activeCaseBody.gasket.inset} unit="mm" validation="nonnegative" onCommit={(value) => updateCaseBody({ gasket: { ...activeCaseBody.gasket!, inset: value } })} />
        <CaseNumber label="Width" value={activeCaseBody.gasket.width} unit="mm" validation="positive" onCommit={(value) => updateCaseBody({ gasket: { ...activeCaseBody.gasket!, width: value } })} />
        <CaseNumber label="Depth" value={activeCaseBody.gasket.depth} unit="mm" validation="positive" onCommit={(value) => updateCaseBody({ gasket: { ...activeCaseBody.gasket!, depth: value } })} />
      </div>}
      </InspectorSection>
    </>}
    {caseFindings.length > 0 && <InspectorSection title="Clearance review" defaultOpen><FindingList document={findingDocument} onShow={onShowFinding} findings={caseFindings} /></InspectorSection>}
  </>;
}

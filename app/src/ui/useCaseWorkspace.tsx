import { useState } from 'react';
import type { ReactNode } from 'react';
import type { CaseBody, EditCommand, PartDefinition, ProjectDoc, SceneDelta } from '@boardstudio/v2-contracts';
import { useCaseDisplay, displayIds } from './caseDisplay';
import { caseReadiness, mechanicalFindings } from './caseReadiness';
import { CaseInspectorPanel } from './CaseInspectorPanel';
import { MechanicalAssemblyPanel } from './MechanicalAssemblyPanel';
import { makeId } from './workbenchGeometry';
import type { Props } from './workbenchTypes';

type Workflow = Pick<Props, 'caseInstanceId' | 'selectedCaseInstanceId' | 'caseBodies' | 'casePreview' | 'generation' | 'mechanicalAssembly' | 'livePreview' | 'onLivePreviewChange' | 'onResolveMechanical' | 'onCancelGeneration' | 'onExportMechanical' | 'onMechanicalProfile' | 'onExtractMechanicalProfile'>;
type Inputs = {
  document: ProjectDoc; scene: SceneDelta; caseDocument: ProjectDoc; caseScene: SceneDelta; boardId: string; projectSession?: number;
  workflow: Workflow; definitions: () => PartDefinition[]; instanceControls?: ReactNode; showInstances: boolean;
  emit: (operation: EditCommand['operation'], ids: string[]) => unknown;
  selectBoard: (id: string) => void; revealMechanicalInspector: () => void; editParts: (id: string) => void;
  showFinding: (finding: SceneDelta['findings'][number]) => void;
};

export function useCaseWorkspace({ document, scene, caseDocument, caseScene, boardId: selectedBoardId, projectSession, workflow, definitions, instanceControls, showInstances, emit, selectBoard, revealMechanicalInspector, editParts, showFinding }: Inputs) {
  const { caseInstanceId, selectedCaseInstanceId, caseBodies, casePreview, generation, mechanicalAssembly, livePreview, onLivePreviewChange, onResolveMechanical, onCancelGeneration, onExportMechanical, onMechanicalProfile, onExtractMechanicalProfile } = workflow;
  const [caseActionsTarget, setCaseActionsTarget] = useState<HTMLDivElement | null>(null);
  const [diagnosticsRequest, setDiagnosticsRequest] = useState(0);
  const [selectedMechanicalLayer, setSelectedMechanicalLayer] = useState('');
  const [caseBodyId, setCaseBodyId] = useState('');
  const selectedBoard = document.boards.find(board => board.id === selectedBoardId);
  const selectedBoardReadiness = scene.boardReadiness?.find(board => board.boardId === selectedBoardId);
  const activeCaseBody = document.caseBodies.find(body => body.id === caseBodyId && (!selectedBoard || body.boardId === selectedBoard.id))
    ?? document.caseBodies.find(body => !selectedBoard || body.boardId === selectedBoard.id);
  const caseDisplay = useCaseDisplay(document.id, selectedCaseInstanceId ?? selectedBoardId);
  const authoredCaseReady = Boolean(activeCaseBody) && (selectedBoardReadiness?.case ?? (document.boards.length <= 1 ? scene.readiness.case : false));
  const generatedCase = caseDocument.mechanical?.boardId === selectedBoardId;
  const mechanicalReadiness = caseReadiness({ revision: caseDocument.revision, sceneRevision: caseScene.revision,
    previewRevision: casePreview?.revision, boardId: selectedBoardId, configuredBoardId: caseDocument.mechanical?.boardId,
    generation, assembly: mechanicalAssembly, hasGeometry: Boolean(caseBodies?.length) });
  const generatedCaseReady = mechanicalReadiness.canExport;
  const visibleMechanicalFindings = generatedCase ? mechanicalFindings(mechanicalAssembly, document) : [];
  const caseReady = generatedCase ? generatedCaseReady : authoredCaseReady;
  const addCaseBody = () => {
    if (!selectedBoard) return;
    const body: CaseBody = {
      id: makeId(),
      name: `${selectedBoard.name} plate`,
      boardId: selectedBoard.id,
      kind: 'plate',
      thickness: 3,
      clearance: 0.5,
      materialId: document.materials.find((material) => material.id === 'pla' || material.name.toLowerCase() === 'pla')?.id ?? 'pla',
      z: 0,
      wallHeight: 14,
      wallThickness: 2,
      mounts: [],
    };
    emit({ kind: 'set-case', body }, [body.id]);
    setCaseBodyId(body.id);
  };

  const updateCaseBody = (changes: Partial<CaseBody>) => {
    if (!activeCaseBody) return;
    emit({ kind: 'set-case', body: { ...activeCaseBody, ...changes } }, [activeCaseBody.id]);
  };

  const updateMount = (mountId: string, changes: Partial<NonNullable<CaseBody['mounts']>[number]>) => {
    if (!activeCaseBody) return;
    const mounts = (activeCaseBody.mounts ?? []).map((mount) => mount.id === mountId ? { ...mount, ...changes } : mount);
    updateCaseBody({ mounts });
  };

  const addMount = () => {
    if (!activeCaseBody) return;
    const mount = { id: makeId(), at: { x: 0, y: 0 }, kind: 'hole' as const, holeDiameter: 2.5, bossDiameter: 5, height: 5 };
    updateCaseBody({ mounts: [...(activeCaseBody.mounts ?? []), mount] });
  };

  const removeMount = (mountId: string) => {
    if (!activeCaseBody) return;
    updateCaseBody({ mounts: (activeCaseBody.mounts ?? []).filter((mount) => mount.id !== mountId) });
  };

  const panel = () => {
      const libraryDefinitions = definitions();
      const configuredBoard = document.boards.find((board) => board.id === caseDocument.mechanical?.boardId);
      const instanceSetup = !selectedMechanicalLayer && showInstances && instanceControls && <div className="wb-case-instance-controls">{instanceControls}</div>;
      const mechanicalPanel = caseDocument.mechanical && !generatedCase
        ? <section className="wb-mechanical-panel"><h3>Mechanical stack belongs to {configuredBoard?.name ?? caseDocument.mechanical.boardId}</h3><p>This board shows its authored case bodies. Select a physical assembly to configure a separate case.</p><button className="wb-secondary" disabled={!configuredBoard} onClick={() => configuredBoard && selectBoard(configuredBoard.id)}>Show configured board</button></section>
        : <MechanicalAssemblyPanel instanceId={caseInstanceId} key={selectedCaseInstanceId ?? selectedBoardId} livePreview={livePreview} onLivePreviewChange={onLivePreviewChange} readiness={mechanicalReadiness} diagnosticsRequest={diagnosticsRequest} onDiagnosticsShown={() => setDiagnosticsRequest(0)} generationTarget={caseActionsTarget} onRevealDiagnostics={revealMechanicalInspector} document={caseDocument} boardId={selectedBoardId} projectSession={projectSession} definitions={libraryDefinitions} configuration={caseDocument.mechanical} assembly={mechanicalAssembly} onChange={(configuration) => emit({ kind: 'set-mechanical', configuration }, [document.id])} onResolve={onResolveMechanical} onCancel={onCancelGeneration} generation={generation} onExport={onExportMechanical} onMechanicalProfile={onMechanicalProfile} onExtractMechanicalProfile={onExtractMechanicalProfile} onEditParts={editParts} onShowFinding={showFinding} selectedLayer={selectedMechanicalLayer} onSelectLayer={setSelectedMechanicalLayer} />;
      const displayPanel = selectedMechanicalLayer && <section className="wb-case-display" aria-label="Part appearance"><h3>Display</h3><label>Colour <input type="color" aria-label="Part colour" value={caseDisplay.current.colors[displayIds(selectedMechanicalLayer)[0]] ?? '#b4bac2'} onChange={event => caseDisplay.color(selectedMechanicalLayer, event.target.value)} /></label><button onClick={() => caseDisplay.color(selectedMechanicalLayer, '')}>Reset colour</button><label><input type="checkbox" checked={!displayIds(selectedMechanicalLayer).some(id => caseDisplay.current.hidden.includes(id))} onChange={() => caseDisplay.toggle(selectedMechanicalLayer)} />Visible</label></section>;
      if (generatedCase) return <>{instanceSetup}{mechanicalPanel}{displayPanel}{!selectedMechanicalLayer && <p className="wb-empty-note">Generated assembly preview · {document.caseBodies.filter((body) => body.boardId === selectedBoardId).length} authored case bodies remain saved. Disable the mechanical stack to preview and edit them.</p>}</>;
      return <CaseInspectorPanel bodies={document.caseBodies} activeCaseBody={activeCaseBody} selectedBoard={selectedBoard}
        findings={scene.findings} findingDocument={document} onShowFinding={showFinding}
        mechanicalPanel={mechanicalPanel} instanceSetup={instanceSetup} caseActionsTarget={caseActionsTarget}
        generation={generation} livePreview={livePreview} onLivePreviewChange={onLivePreviewChange} canConfigure={!document.mechanical} previewCurrent={casePreview?.revision === scene.revision && caseReady}
        onConfigure={() => { revealMechanicalInspector(); requestAnimationFrame(() => globalThis.document.querySelector<HTMLButtonElement>('.wb-mech-start button')?.focus()); }}
        onResolveMechanical={onResolveMechanical} onCancelGeneration={onCancelGeneration}
        setCaseBodyId={setCaseBodyId} addCaseBody={addCaseBody} updateCaseBody={updateCaseBody}
        addMount={addMount} updateMount={updateMount} removeMount={removeMount}
      />;
    }
  return {
    panel, activeCaseBody, selectedMechanicalLayer, selectLayer: setSelectedMechanicalLayer, selectBody: setCaseBodyId,
    display: caseDisplay, setGenerationTarget: setCaseActionsTarget,
    reviewDiagnostics: () => setDiagnosticsRequest(value => value + 1),
    readiness: { authoredCaseReady, generatedCase, mechanicalReadiness, generatedCaseReady, visibleMechanicalFindings },
  };
}

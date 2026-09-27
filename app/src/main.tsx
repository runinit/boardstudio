import type { CasePreviewResult } from '@boardstudio/v2-cad';
import type { FootprintCompileJob, Matrix, MechanicalBuiltinProfile, MechanicalExtraction, MechanicalPartProfile, MechanicalPurposeMapping } from '@boardstudio/v2-contracts';
import { useCallback, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { CaseClient } from './CaseClient';
import { ExportClient } from './ExportClient';
import type { ContextualCaseResult } from './casePreviewContext';
import { createProjectActions } from './createProjectActions';
import { createProjectExporter } from './createProjectExporter';
import { releaseReviewedConnections } from './electricalHandoff';
import { isWiringApplied } from './electricalPlanContext';
import { FirmwareKeymapPanel } from './ui/FirmwareKeymapPanel';
import { HardwareInstancesPanel } from './ui/HardwareInstancesPanel';
import { Workbench } from './ui/Workbench';
import type { MatrixScene } from './ui/matrixGeometry';
import { useCaseGeneration } from './useCaseGeneration';
import { useElectricalPlanning } from './useElectricalPlanning';
import { useProjectSession } from './useProjectSession';

function App() {
  const [selectedInstanceId, setSelectedInstanceId] = useState('');
  const [error, setError] = useState('');
  const [embedUsedModels, setEmbedUsedModels] = useState(true);
  const [setupRequest, setSetupRequest] = useState<{ projectId: string; requestId: string }>();
  const [activeMode, setActiveMode] = useState<'Design' | 'PCB' | 'Case' | 'Library' | 'Export'>('Design');
  const caseClient = useRef<CaseClient | null>(null);
  const exportClient = useRef<ExportClient | null>(null);
  const previewCache = useRef(new Map<string, ContextualCaseResult<CasePreviewResult>>());
  const { project, scene, selectedBoardId, setSelectedBoardId, ready, client, projectSession, saveStatus, projectRef, committedScene, accept, schedule } = useProjectSession({ caseClient, exportClient, previewCache, setSelectedInstanceId, setError });

  const selectedInstance = project.hardware?.instances.find(instance => instance.id === selectedInstanceId && instance.boardId === selectedBoardId)
    ?? project.hardware?.instances.find(instance => instance.boardId === selectedBoardId);

  const { physicalDocument, physicalScene, generation, currentPreviewContext, visibleCasePreview, visibleMechanicalAssembly, cancelGeneration, generateCase } = useCaseGeneration({ project, scene, selectedBoardId, selectedInstance, projectSession, projectRef, committedScene, client, caseClient, previewCache, activeMode, ready, setError });

  const { edit, history, importProject, newProject, duplicateDesign, importPart, importModel } = createProjectActions({ projectRef, client, exportClient, selectedInstance, schedule, accept, ensureExportClient, onProjectCreated: projectId => setSetupRequest({ projectId, requestId: crypto.randomUUID() }) });

  const { refreshWiring, resolveWiring, changeWiring, applyWiring, applyExportWiring, wiringConfiguration, controllerOptions, activePlan, connectionReview, assignments } = useElectricalPlanning({ project, projectRef, selectedBoardId, client, ready, setError, schedule, accept, edit });

  const { exportFile, exportMechanical } = createProjectExporter({ projectRef, committedScene, client, caseClient, exportClient, selectedBoardId, selectedInstance, embedUsedModels, generation, currentPreviewContext, schedule, accept, ensureExportClient, resolveWiring, applyExportWiring });

  function ensureExportClient(): ExportClient {
    exportClient.current ??= new ExportClient();
    return exportClient.current;
  }

  const requestMechanicalProfile = useCallback(async (definitionId: string, source: MechanicalBuiltinProfile, plateToPcb: number): Promise<MechanicalPartProfile> => {
    if (!client.current) throw new Error('The core worker is not ready');
    const reply = await client.current.request({ id: crypto.randomUUID(), kind: 'mechanical-profile', definitionId, source, plateToPcb });
    if (reply.kind === 'error') throw new Error(reply.message);
    if (reply.kind !== 'mechanical-profile') throw new Error('Expected a library mechanical profile');
    return reply.profile;
  }, []);

  const extractMechanicalProfile = useCallback(async (source: string, mappings: MechanicalPurposeMapping[]): Promise<MechanicalExtraction> => {
    const reply = await ensureExportClient().artifact({ kind: 'extract-mechanical', source, mappings, maxDeviationMm: 0.005 });
    if (reply.kind === 'error') throw new Error(reply.error.message);
    if (reply.kind !== 'extract-mechanical') throw new Error('Expected extracted mechanical geometry');
    return reply.result;
  }, []);

  const compileFootprints = useCallback(async (jobs: FootprintCompileJob[]) => {
    exportClient.current ??= new ExportClient();
    return exportClient.current.compile(jobs);
  }, []);

  const projectMatrices = useCallback(async (matrices: Matrix[]): Promise<MatrixScene[] | undefined> => {
    const core = client.current;
    if (!core) return undefined;
    const requestedRevision = projectRef.current.revision;
    const reply = await core.projectMatrices(crypto.randomUUID(), requestedRevision, matrices);
    if (reply.kind !== 'matrix-projections' || reply.revision !== requestedRevision || projectRef.current.revision !== requestedRevision) return undefined;
    return reply.matrixScenes;
  }, []);

  if (!ready) {
    return <div className="boot-status">Opening Board Studio…</div>;
  }

  return <>
    {error && <div className="app-error" role="alert" onClick={() => setError('')}>{error}</div>}
    <Workbench
      saveStatus={saveStatus}
      projectSession={projectSession}
      setupRequest={setupRequest}
      document={project}
      scene={scene}
      physicalCaseDocument={physicalDocument}
      physicalCaseScene={physicalScene}
      wiringStatus={{ current: Boolean(activePlan), ready: Boolean(activePlan && !activePlan.diagnostics.some(finding => finding.severity === 'error')), applied: Boolean(activePlan && isWiringApplied(project, activePlan)) }}
      wiring={{
        existingConnections: connectionReview.length ? {
          names: connectionReview.map(net => net.name),
          pinCount: connectionReview.reduce((count, net) => count + net.pins.length, 0),
          onReplace: () => {
            if (!activePlan) return;
            edit({ baseRevision: project.revision, phase: 'commit', transactionId: crypto.randomUUID(), targetIds: [selectedBoardId], operation: { kind: 'replace-document', document: releaseReviewedConnections(project, activePlan) } });
          },
        } : undefined,
        ready: Boolean(activePlan && !activePlan.diagnostics.some(finding => finding.severity === 'error')),
        firmwareControls: <FirmwareKeymapPanel keys={[...(activePlan?.assignments.map(assignment => ({ id: assignment.keyId, label: project.parts.find(part => part.id === assignment.keyId)?.reference ?? assignment.keyId })) ?? []), ...(activePlan?.peripherals.filter(peripheral => peripheral.kind === 'encoder' && peripheral.gpioTerminals.some(([terminal]) => terminal === 'S1')).map(peripheral => ({ id: `${peripheral.partId}/push`, label: `${project.parts.find(part => part.id === peripheral.partId)?.reference ?? peripheral.partId} push` })) ?? [])]} bindings={wiringConfiguration?.keyBindings ?? {}} onChange={(keyId, binding) => changeWiring({ keyBindings: { ...wiringConfiguration?.keyBindings, [keyId]: binding } })} />,
        controllerOptions, selectedControllerId: wiringConfiguration?.controllerPartId ?? activePlan?.controllerPartId ?? '',
        controller: controllerOptions.find(option => option.id === (wiringConfiguration?.controllerPartId ?? activePlan?.controllerPartId)),
        onControllerChange: controllerPartId => changeWiring({ controllerPartId }),
        topology: wiringConfiguration?.mode ?? 'matrix', onTopologyChange: mode => changeWiring({ mode }),
        assignments, usedPins: assignments.map(assignment => assignment.value ?? '').filter(Boolean), freePins: activePlan?.freePins ?? [],
        findings: activePlan?.diagnostics.map(finding => `${finding.severity}: ${finding.message}`) ?? ['Resolving wiring…'],
        onAssignPin: (assignmentId, pin) => changeWiring({ locks: { ...wiringConfiguration?.locks, [assignmentId]: pin } }),
        protectedSummary: wiringConfiguration?.protectedHandoff ? `Pins protected by PCB handoff at revision ${wiringConfiguration.protectedHandoff.revision}` : undefined,
        onReviewRemap: wiringConfiguration?.protectedHandoff ? () => schedule(async () => {
          await accept(await client.current!.request({ id: crypto.randomUUID(), kind: 'review-electrical-remap', baseRevision: projectRef.current.revision, boardId: selectedBoardId, expectedFingerprint: wiringConfiguration.protectedHandoff!.fingerprint }), 'commit');
        }) : undefined,
      }}
      onResolveWiring={() => schedule(refreshWiring)}
      onApplyWiring={activePlan && !activePlan.diagnostics.some(finding => finding.severity === 'error') ? applyWiring : undefined}
      onReviewWiring={assignment => {
        const locks = { ...wiringConfiguration?.locks };
        if (locks[assignment.id]) delete locks[assignment.id]; else if (assignment.value) locks[assignment.id] = assignment.value;
        changeWiring({ locks });
      }}
      instanceControls={<HardwareInstancesPanel document={project} boardId={selectedBoardId} selectedId={selectedInstance?.id}
        onSelect={(id, boardId) => { setSelectedInstanceId(id); setSelectedBoardId(boardId); }}
        onChange={hardware => edit({ baseRevision: project.revision, phase: 'commit', transactionId: crypto.randomUUID(), targetIds: [], operation: { kind: 'replace-document', document: { ...project, hardware } } })} />}
      casePreview={visibleCasePreview && { revision: visibleCasePreview.revision, ...visibleCasePreview.mesh }}
      caseBodies={visibleCasePreview?.bodies}
      mechanicalAssembly={physicalDocument.mechanical?.boardId === selectedBoardId ? visibleMechanicalAssembly : undefined}
      onResolveMechanical={() => { void generateCase(); }}
      onCancelGeneration={cancelGeneration}
      generation={generation}
      onExportMechanical={exportMechanical}
      onMechanicalProfile={requestMechanicalProfile}
      onExtractMechanicalProfile={extractMechanicalProfile}
      onModeChange={setActiveMode}
      onEdit={edit}
      onUndo={() => history('undo')}
      onRedo={() => history('redo')}
      onExport={exportFile}
      compileFootprints={compileFootprints}
      embedUsedModels={embedUsedModels}
      onEmbedUsedModelsChange={setEmbedUsedModels}
      selectedBoardId={selectedBoardId}
      onSelectBoard={setSelectedBoardId}
      onImport={importProject}
      onNewProject={newProject}
      onDuplicateDesign={duplicateDesign}
      onProjectMatrices={projectMatrices}
      onImportFootprint={importPart}
      onImportModel={importModel}
    />
  </>;
}

createRoot(document.getElementById('root')!).render(<App />);

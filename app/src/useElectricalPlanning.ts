import type { CoreReply, EditCommand, ElectricalBoardConfiguration, ElectricalPlan, ProjectDoc } from '@boardstudio/v2-contracts';
import type { MutableRefObject } from 'react';
import { useEffect, useRef, useState } from 'react';
import { CoreClient } from './CoreClient';
import { existingConnectionReview } from './electricalHandoff';
import { canPublishElectricalPlan, isCurrentElectricalPlan, isWiringApplied, type ElectricalPlanContext } from './electricalPlanContext';
import type { WiringAssignment } from './ui/WiringPanel';

type Inputs = {
  project: ProjectDoc;
  projectRef: MutableRefObject<ProjectDoc>;
  selectedBoardId: string;
  client: MutableRefObject<CoreClient | null>;
  ready: boolean;
  setError: (message: string) => void;
  schedule: (work: () => Promise<void>) => void;
  accept: (reply: CoreReply, mode: 'open' | 'commit' | 'preview') => Promise<void>;
  edit: (command: EditCommand) => void;
};

export function useElectricalPlanning({ project, projectRef, selectedBoardId, client, ready, setError, schedule, accept, edit }: Inputs) {
  const [electricalPlan, setElectricalPlan] = useState<ElectricalPlanContext>();
  const selectedBoardRef = useRef(selectedBoardId);
  selectedBoardRef.current = selectedBoardId;

  async function resolveWiring(document = projectRef.current, boardId = selectedBoardId, instanceId: string | null = null): Promise<ElectricalPlan> {
    if (!client.current) throw new Error('The project is still opening');
    const configuration = document.hardware?.boards.find(board => board.boardId === boardId);
    const reply = await client.current.request({
      id: crypto.randomUUID(), kind: 'resolve-electrical', request: {
        document, boardId, instanceId, mode: configuration?.mode ?? 'matrix', locks: configuration?.locks ?? {},
        controllerPartId: document.hardware?.instances.find(instance => instance.id === instanceId)?.controllerPartId ?? configuration?.controllerPartId ?? null, controllerProfile: null,
      }
    });
    if (reply.kind === 'error') throw new Error(reply.message);
    if (reply.kind !== 'electrical-resolved') throw new Error('Expected resolved wiring');
    return reply.plan;
  }

  useEffect(() => {
    if (!ready) return;
    let current = true;
    void resolveWiring(project, selectedBoardId).then(plan => { if (current && projectRef.current === project) setElectricalPlan({ document: project, plan }); }).catch(cause => { if (current) setError(String(cause)); });
    return () => { current = false; };
  }, [ready, project, selectedBoardId]);

  async function refreshWiring(): Promise<void> {
    const document = projectRef.current;
    const boardId = selectedBoardId;
    const plan = await resolveWiring(document, boardId);
    if (canPublishElectricalPlan(document, boardId, projectRef.current, selectedBoardRef.current)) setElectricalPlan({ document, plan });
  }

  function changeWiring(change: Partial<ElectricalBoardConfiguration>): void {
    const hardware = project.hardware ?? { topology: 'unibody' as const, transport: 'none' as const, boards: [], instances: [], sharedConstruction: null };
    const previous = hardware.boards.find(board => board.boardId === selectedBoardId);
    const configuration: ElectricalBoardConfiguration = { boardId: selectedBoardId, controllerPartId: null, mode: 'matrix', locks: {}, assignments: {}, keyBindings: {}, jumperStates: {}, protectedHandoff: null, ...previous, ...change };
    edit({
      baseRevision: project.revision, transactionId: crypto.randomUUID(), phase: 'commit', targetIds: [selectedBoardId], operation: {
        kind: 'replace-document', document: {
          ...project, hardware: {
            ...hardware, boards: [...hardware.boards.filter(board => board.boardId !== selectedBoardId), configuration],
          }
        }
      }
    });
  }

  function applyWiring(): void {
    schedule(async () => {
      const document = projectRef.current;
      const plan = await resolveWiring(document);
      await accept(await client.current!.request({ id: crypto.randomUUID(), kind: 'apply-electrical', baseRevision: document.revision, plan, draft: false }), 'commit');
    });
  }

  const wiringConfiguration = project.hardware?.boards.find(board => board.boardId === selectedBoardId);

  const boardParts = project.parts.filter(part => project.boards.find(board => board.id === selectedBoardId)?.partIds.includes(part.id));

  const controllerOptions = boardParts.flatMap(part => {
    const definition = project.definitions.find(definition => definition.id === part.definitionId);
    return definition?.kind === 'controller' || definition?.generator?.source.includes('/mcu_') ? [{ id: part.id, name: `${part.reference} · ${definition.name}` }] : [];
  });

  const activePlan = isCurrentElectricalPlan(electricalPlan, project, selectedBoardId) ? electricalPlan.plan : undefined;

  const connectionReview = activePlan ? existingConnectionReview(project, activePlan) : [];

  const scanAssignments: WiringAssignment[] = activePlan?.mode === 'direct'
    ? activePlan.assignments.map(assignment => ({ id: assignment.keyId, label: project.parts.find(part => part.id === assignment.keyId)?.reference ?? assignment.keyId, value: assignment.columnPin, detail: assignment.directGpio ?? 'Unresolved', locked: Boolean(wiringConfiguration?.locks[assignment.keyId]) }))
    : [...(activePlan?.rowPins ?? []).map((pin, row) => ({ id: `row/${row}`, label: `Scan row ${row + 1}`, value: pin, locked: Boolean(wiringConfiguration?.locks[`row/${row}`]) })),
    ...(activePlan?.columnPins ?? []).map((pin, column) => ({ id: `column/${column}`, label: `Scan column ${column + 1}`, value: pin, locked: Boolean(wiringConfiguration?.locks[`column/${column}`]) }))];

  const assignments: WiringAssignment[] = [...scanAssignments, ...Object.entries(activePlan?.peripheralTerminals ?? {}).map(([id, pin]) => ({ id, label: id.startsWith('peripheral/') ? id.split('/').slice(-2).join(' · ') : id, value: pin, detail: activePlan?.peripheralPins[id], locked: Boolean(wiringConfiguration?.locks[id]) }))];

  async function applyExportWiring(document: ProjectDoc, boardId: string, draft: boolean): Promise<{ document: ProjectDoc; plan: ElectricalPlan }> {
    const plan = await resolveWiring(document, boardId);
    const errors = plan.diagnostics.filter(finding => finding.severity === 'error');
    if (!draft && errors.length) throw new Error(errors.map(finding => finding.message).join('\n'));
    if (isWiringApplied(document, plan)) return { document, plan };
    const reply = await client.current!.request({ id: crypto.randomUUID(), kind: 'apply-electrical', baseRevision: document.revision, plan, draft });
    if (reply.kind !== 'scene') throw new Error(reply.kind === 'error' ? reply.message : 'Expected applied wiring');
    await accept(reply, 'commit');
    return { document: reply.document, plan: await resolveWiring(reply.document, boardId) };
  }

  return { refreshWiring, resolveWiring, changeWiring, applyWiring, applyExportWiring, wiringConfiguration, controllerOptions, activePlan, connectionReview, assignments };
}

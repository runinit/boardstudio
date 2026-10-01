import type { CaseDisplay } from './caseDisplay';
import type { GenerationState } from '../generationState';
import { FindingList } from './FindingList';
import { useMemo } from 'react';
import { useAssemblyPreview } from '../useAssemblyPreview';
import type {
  Contour,
  MechanicalAssembly,
  MechanicalConfiguration,
  PreparedCaseAssemblyIR,
  ProjectDoc,
  Mount,
} from '@boardstudio/v2-contracts';
import {
  AssemblyScene,
  type AssemblyBody,
} from './AssemblyScene';

const noBodies: AssemblyBody[] = [];
export function AssemblyViewer({
  document, projectSession,
  boardId,
  contours,
  bodies = noBodies,
  mechanical,
  generation,
  preparedCase,
  onGasketChange,
  onGasketDraft,
  onCaseMountChange,
  onCaseMountDraft,
  onCasePreviewDraft,
  selectedLayer, onShowFinding, focusedFinding,
  onSelectLayer,
  onSelect,
  colorScheme,
  sample = false,
  display, onDisplayChange, displayKey,
}: {
  document: ProjectDoc;
  projectSession?: number;
  boardId: string;
  contours: Contour[];
  bodies?: AssemblyBody[];
  mechanical?: MechanicalAssembly;
  generation?: GenerationState;
  preparedCase?: PreparedCaseAssemblyIR;
  onGasketChange?: (config: MechanicalConfiguration) => void | Promise<boolean>;
  onGasketDraft?: (config: MechanicalConfiguration | null, disposition?: 'commit') => void;
  onCaseMountChange?: (bodyId: string, mounts: Mount[]) => void | Promise<boolean>;
  onCaseMountDraft?: (bodyId: string, mounts: Mount[] | null, disposition?: 'commit') => void;
  onCasePreviewDraft?: (document: ProjectDoc | null, disposition?: 'commit') => void;
  selectedLayer?: string;
  focusedFinding?: import('@boardstudio/v2-contracts').Finding;
  onShowFinding?: (finding: import('@boardstudio/v2-contracts').Finding) => void;
  onSelectLayer?: (id: string) => void;
  onSelect?: (reference: string) => void;
  colorScheme: 'light' | 'dark';
  sample?: boolean;
  display?: CaseDisplay;
  onDisplayChange?: (next: CaseDisplay) => void;
  displayKey?: string;
}) {
  const pcbTopZ=mechanical?.stack.length?0:document.boards.find(board=>board.id===boardId)?.thickness??1.6;
  const { board, models, messages, error, pending, keycaps, keycapResolution, keycapError, keycapsPending, moduleBodies, modulesPending, moduleError, reference: shownReference, retry } = useAssemblyPreview({ document, boardId, contours, preparedCase, session: projectSession, instanceId: displayKey,pcbTopZ });
  const allBodies = useMemo(() => [...bodies, ...keycaps,...moduleBodies], [bodies, keycaps,moduleBodies]);
  const authoredBodies = useMemo(() => document.caseBodies.filter(body => body.boardId === boardId), [document.caseBodies, boardId]);
  return (
    <div className="wb-assembly-view">
      {board && (
        <AssemblyScene
          board={board}
          models={models}
          bodies={allBodies}
          mechanical={mechanical}
          generation={generation}
          preparedCase={preparedCase}
          onGasketChange={onGasketChange}
          onGasketDraft={(configuration, disposition) => { onGasketDraft?.(configuration, disposition); onCasePreviewDraft?.(configuration ? { ...document, mechanical: configuration } : null, disposition); }}
          onCaseMountChange={onCaseMountChange}
          onCaseMountDraft={(bodyId, mounts, disposition) => { onCaseMountDraft?.(bodyId, mounts, disposition); const body = document.caseBodies.find((entry) => entry.id === bodyId); onCasePreviewDraft?.(body && mounts ? { ...document, caseBodies: document.caseBodies.map((entry) => entry.id === bodyId ? { ...body, mounts } : entry) } : null, disposition); }}
          authoredCaseBodies={authoredBodies}
          mechanicalConfiguration={document.mechanical}
          display={display} onDisplayChange={onDisplayChange}
          focusedFinding={focusedFinding}
          selectedLayer={selectedLayer}
          onSelectLayer={onSelectLayer}
          reference={shownReference}
        colorScheme={colorScheme}
        key={`${document.id}:${displayKey ?? boardId}`}
        persistenceKey={`${document.id}:${displayKey ?? boardId}`}
          onSelect={id => onSelect?.(document.parts.find(part => part.id === id.replace(/^keycap(?:-legend)?:/, ''))?.reference ?? id.replace(/^module-(?:body|model)\//u,'').replace(/\/\d+$/u,''))}
        />
      )}
      {keycapsPending && <p className="wb-assembly-loading" role="status">Generating keycap CAD…</p>}
      {modulesPending&&<p className="wb-assembly-loading" role="status">Preparing mounted modules…</p>}
      {moduleError&&<div role="alert" className="wb-assembly-error">{moduleError}<button onClick={retry}>Retry module preview</button></div>}
      {keycapError && <div role="alert" className="wb-assembly-error">{keycapError}<button onClick={retry}>Retry keycaps</button></div>}
      {keycapResolution && keycapResolution.findings.length > 0 && <details className="wb-assembly-notices" open><summary>Keycap clearance · {keycapResolution.findings.length} findings</summary>{onShowFinding ? <FindingList document={document} findings={keycapResolution.findings} onShow={onShowFinding} /> : keycapResolution.findings.map(finding => <p key={finding.id}>{finding.message}</p>)}</details>}
      {pending && (
        <p className="wb-assembly-loading" role="status">
          Preparing PCB assembly…
        </p>
      )}
      {(sample || messages.length > 0) && (
        <details className="wb-assembly-notices">
          <summary>
            {sample ? 'Sample PCB' : `${messages.length} model ${messages.length === 1 ? 'notice' : 'notices'}`}
          </summary>
          {sample && <p>Sample board around this footprint assembly.</p>}
          {messages.map((m, i) => (
            <p key={i}>{m}</p>
          ))}
          {messages.length > 0 && (
            <button onClick={retry}>
              Retry models
            </button>
          )}
        </details>
      )}
      {error && (
        <div className="wb-assembly-error" role="alert">
          <p>{error}</p>
          <button onClick={retry}>
            Retry preview
          </button>
        </div>
      )}
    </div>
  );
}

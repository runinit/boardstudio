import React, { type ReactNode } from 'react';
import type {
  CompiledFootprint,
  EditCommand,
  FootprintCompileJob,
  Matrix,
  MatrixSplayChange,
  MechanicalAssembly,
  MechanicalBuiltinProfile,
  MechanicalExtraction,
  MechanicalPartProfile,
  MechanicalPurposeMapping,
  Part,
  ProjectDoc,
  SceneDelta,
  Vec2
} from '../../../contracts/src/index';
import type { GenerationState } from '../generationState';
import { type MatrixPresetId } from './assemblyCatalog';
import { type SwitchOrientation } from './assemblyPresets';
import { getBounds } from './canvasBounds';
import { type MatrixScene } from './matrixGeometry';
import { type WiringAssignment } from './WiringPanel';

export type Mode = 'Design' | 'PCB' | 'Case' | 'Library' | 'Export';

export type ExportKind = 'project' | 'kicad' | 'kicad-draft' | 'firmware' | 'footprints' | 'case-step' | 'svg' | 'dxf';

export type Props = {
  projectSession?: number;
  setupRequest?: { projectId: string; requestId: string };
  document: ProjectDoc;
  saveStatus?: 'saving' | 'saved' | 'failed';
  scene: SceneDelta;
  onEdit: (command: EditCommand) => void;
  onUndo: () => void;
  onRedo: () => void;
  onExport: (kind: ExportKind, boardId?: string) => void;
  compileFootprints: (jobs: FootprintCompileJob[]) => Promise<CompiledFootprint[]>;
  selectedBoardId?: string;
  onSelectBoard?: (boardId: string) => void;
  physicalCaseDocument?: ProjectDoc;
  physicalCaseScene?: SceneDelta;
  instanceControls?: ReactNode;
  wiringStatus?: { current: boolean; ready: boolean; applied: boolean };
  wiring?: { existingConnections?: { names: string[]; pinCount: number; onReplace: () => void }; ready?: boolean; firmwareControls?: ReactNode; controller?: { name: string; detail?: string }; controllerOptions?: { id: string; name: string; detail?: string }[]; selectedControllerId?: string; onControllerChange?: (id: string) => void; topology?: string; onTopologyChange?: (topology: 'matrix' | 'direct') => void; assignments?: WiringAssignment[]; usedPins?: string[]; freePins?: string[]; findings?: string[]; onToggleLock?: (assignment: WiringAssignment) => void; onAssignPin?: (assignmentId: string, pin: string) => void; protectedSummary?: string; onReviewRemap?: () => void };
  onResolveWiring?: () => void;
  onApplyWiring?: () => void;
  onReviewWiring?: (assignment: WiringAssignment) => void;
  onNewProject?: () => void;
  onOpenDemo?: (variant: import('../demos/keyboards').DemoId) => void;
  onImport?: (file: File) => void;
  onImportFootprint?: (file: File) => void;
  onImportModel?: (file: File, definitionId: string, parameter?: string) => void;
  mechanicalAssembly?: MechanicalAssembly;
  onResolveMechanical?: () => void;
  onCancelGeneration?: () => void;
  generation?: GenerationState;
  onExportMechanical?: () => void;
  onMechanicalProfile?: (definitionId: string, source: MechanicalBuiltinProfile, plateToPcb: number) => Promise<MechanicalPartProfile>;
  onExtractMechanicalProfile?: (source: string, mappings: MechanicalPurposeMapping[]) => Promise<MechanicalExtraction>;
  onDuplicateDesign?: (matrixId: string, presetId: MatrixPresetId, orientation?: SwitchOrientation) => void;
  onProjectMatrices?: (matrices: Matrix[]) => Promise<MatrixScene[] | undefined>;
  onModeChange?: (mode: Mode) => void;
  caseBodies?: import('@boardstudio/v2-contracts').CaseBodyMesh[];
  casePreview?: { positions: Float32Array; normals: Float32Array; revision: number };
  embedUsedModels?: boolean;
  onEmbedUsedModelsChange?: (value: boolean) => void;
};

export type Drag = {
  ids: string[];
  origins: { id: string; at: Vec2 }[];
  pointerStart: Vec2;
  clientStart: Vec2;
  pending: { id: string; at: Vec2 }[];
  transactionId: string;
  pointerId: number;
  target: SVGGElement;
  bounds: ReturnType<typeof getBounds>;
  frame: number | null;
  moved: boolean;
  matrixCell?: { matrixId: string; row: number; column: number; offset: Vec2; assemblyId?: string };
  matrixScope?: { kind: 'matrix'; origin: Vec2 } | { kind: 'row' | 'column'; index: number; offset: Vec2 };
  pendingMatrix?: Matrix;
  matrixStart?: Matrix;
};

export type SelectionScope = {
  kind: 'matrix' | 'row' | 'column' | 'key' | 'component';
  matrixId?: string;
  row?: number;
  column?: number;
  partId?: string;
};

export type StaggerDrag = {
  matrixStart: Matrix;
  transactionId: string;
  axis: 'row' | 'column';
  index: number;
  startClient: Vec2;
  worldPerPixel: Vec2;
  offset: Vec2;
  pointerId: number;
  target: SVGElement;
  pending: Matrix;
};

export type SceneHandlers = {
  hoverPart: (id: string | null) => void;
  startDrag: (event: React.PointerEvent<SVGGElement>, part: Part) => void;
  moveDrag: (event: React.PointerEvent<SVGGElement>) => void;
  endDrag: (event: React.PointerEvent<SVGGElement>) => void;
  choosePart: (id: string, modifiers?: { additive?: boolean; range?: boolean }) => void;
  nudgePart: (event: React.KeyboardEvent<Element>, part: Part) => void;
};

export type SplayDrag = {
  matrix: Matrix;
  pending?: MatrixSplayChange;
  origin: Vec2;
  column: number;
  kind: 'origin' | 'angle';
  pointerId: number;
  startAngle: number;
  target: SVGGElement;
  transactionId: string;
  bounds: ReturnType<typeof getBounds>;
};

export type PanDrag = {
  pointerId: number;
  startX: number;
  startY: number;
  pan: Vec2;
  width: number;
  height: number;
};

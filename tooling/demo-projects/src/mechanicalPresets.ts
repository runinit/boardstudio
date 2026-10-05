import type {
  MechanicalConfiguration,
  MechanicalPartProcess,
  MechanicalSwitchFamily,
  PartDefinition,
  ProjectDoc,
} from '@boardstudio/v2-contracts';

const constraintsVersion = '2026-09-24';
const standardProcessIds = ['plate', 'plate-foam', 'bottom-foam', 'bottom'] as const;
type StandardProcessId = (typeof standardProcessIds)[number];
type ProcessMethod = MechanicalConfiguration['method'];

const materialForProcess = (
  partId: string,
  method: MechanicalConfiguration['method'],
): string => {
  if (partId.endsWith('foam')) return 'EVA';
  switch (method) {
    case 'printed': return 'PLA';
    case 'cnc': return 'Aluminium';
    case 'cut-sheet': return 'Acrylic';
    case 'pcb-fr4': return 'FR-4';
  }
};

const defaultPlateThickness = (family: MechanicalSwitchFamily | undefined): number =>
  family === 'choc-v1' ? 1.3 : family === 'choc-v2' ? 1.5 : 1.5;

const switchMountingDatum = (family: MechanicalSwitchFamily): number =>
  family === 'choc-v1' ? 3.5 : 5.0;

const plateToPcbGap = (
  family: MechanicalSwitchFamily,
  plateThickness: number,
): number => switchMountingDatum(family) - plateThickness;

const defaultPlateFoamThickness = (gap: number): number =>
  Math.max(0, Math.floor((Math.min(3, gap - 0.2) + 1e-6) * 10) / 10);

function makeProcess(
  partId: StandardProcessId,
  method: ProcessMethod,
  thickness: number,
): MechanicalPartProcess {
  const processMethod = partId.endsWith('foam') ? 'cut-sheet' : method;
  return {
    partId,
    method: processMethod,
    material: materialForProcess(partId, processMethod),
    thickness,
    constraintsVersion,
  };
}

export function createMechanicalConfiguration(
  document: ProjectDoc,
  boardId?: string,
): MechanicalConfiguration {
  const selectedBoardId = boardId ?? document.boards[0]?.id ?? '';
  const family = initialSwitchFamily(document, selectedBoardId) ?? 'mx';
  const plateThickness = defaultPlateThickness(family);
  const plateToPcb = plateToPcbGap(family, plateThickness);
  const plateFoamThickness = defaultPlateFoamThickness(plateToPcb);
  const thicknesses = {
    plateThickness,
    plateFoamThickness,
    bottomFoamThickness: 2,
    bottomThickness: 3,
  };
  return {
    boardId: selectedBoardId,
    method: 'printed',
    mount: 'tray',
    integratedPlateFrame: false,
    bottomStyle: 'shell',
    middleFrame: false,
    plateThickness,
    plateFoamThickness,
    pcbThickness: validPositive(document.boards.find((board) => board.id === selectedBoardId)?.thickness)
      ? document.boards.find((board) => board.id === selectedBoardId)!.thickness
      : 1.6,
    bottomFoamThickness: 2,
    batteryHeight: 0,
    bottomThickness: 3,
    plateToPcb,
    wallThickness: 2,
    clearance: 0.3,
    mounts: [],
    partProcesses: [
      makeProcess('plate', 'printed', thicknesses.plateThickness),
      makeProcess('plate-foam', 'printed', thicknesses.plateFoamThickness),
      makeProcess('bottom-foam', 'printed', thicknesses.bottomFoamThickness),
      makeProcess('bottom', 'printed', thicknesses.bottomThickness),
    ],
    profiles: [],
  };
}

function validPositive(value: number | undefined): value is number {
  return typeof value === 'number' && Number.isFinite(value) && value > 0;
}

function inferSwitchFamily(
  definition: PartDefinition | undefined,
  partParameters?: Record<string, unknown>,
): MechanicalSwitchFamily | undefined {
  if (!definition) return undefined;
  const source = definition.generator?.source.toLowerCase() ?? '';
  if (source === 'ceoloide/switch_mx') return 'mx';
  if (!source.endsWith('/switch_choc_v1_v2')) return undefined;
  const getEnabled = (name: string): boolean | undefined => {
    const value = partParameters?.[name] ?? definition.generator?.parameters[name];
    if (typeof value === 'boolean') return value;
    if (value && typeof value === 'object' && 'value' in value) {
      const parameterValue = (value as { value?: unknown }).value;
      return typeof parameterValue === 'boolean' ? parameterValue : undefined;
    }
    return undefined;
  };
  const v1 = getEnabled('choc_v1_support') ?? true;
  const v2 = getEnabled('choc_v2_support') ?? true;
  return v1 === v2 ? undefined : v1 ? 'choc-v1' : 'choc-v2';
}

function initialSwitchFamily(
  document: ProjectDoc,
  boardId: string,
): MechanicalSwitchFamily | undefined {
  const board = document.boards.find((entry) => entry.id === boardId);
  const parts = document.parts.filter((part) => {
    const definition = document.definitions.find((item) => item.id === part.definitionId);
    return board?.partIds.includes(part.id) && (definition?.kind === 'switch'
      || inferSwitchFamily(definition, part.generatorParameters)
      || definition?.generator?.source.endsWith('/switch_choc_v1_v2'));
  });
  if (!parts.length) return undefined;
  const families = parts.map((part) => inferSwitchFamily(
    document.definitions.find((definition) => definition.id === part.definitionId),
    part.generatorParameters,
  ));
  if (families.some((family) => !family)) return undefined;
  const distinct = new Set(families);
  return distinct.size === 1 ? families[0] : undefined;
}

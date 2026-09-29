import { mechanicalDefaults } from './mechanicalDefaults';
import type { MechanicalConfiguration, PhysicalBoardInstance, ProjectDoc, SceneDelta, Vec2 } from '@boardstudio/v2-contracts';

const constructionKeys = [
  'method', 'mount', 'integratedPlateFrame', 'bottomStyle', 'middleFrame', 'plateThickness',
  'plateFoamThickness', 'bottomFoamThickness', 'bottomThickness', 'plateToPcb', 'wallThickness',
  'clearance', 'gasket', 'gasketTravel', 'openingAllowance', 'partProcesses',
  'internalGasket', 'hardware', 'criticalFits', 'profiles',
] as const satisfies readonly (keyof MechanicalConfiguration)[];

const reflect = (point: Vec2): Vec2 => ({ x: -point.x, y: point.y });

export function effectiveCaseDocument(document: ProjectDoc, instance?: PhysicalBoardInstance): ProjectDoc {
  if (!instance) return document.mechanical ? { ...document, mechanical: mechanicalDefaults(document, document.mechanical) } : document;
  const board = document.boards.find(entry => entry.id === instance.boardId);
  const common = document.hardware?.sharedConstruction ?? document.hardware?.instances.find(entry => entry.mechanical)?.mechanical;
  const base = instance.mechanical ?? (common ? { ...common, openings: [], mounts: [], closureMounts: undefined, battery: undefined, batteryHeight: 0 } : undefined);
  let mechanical: MechanicalConfiguration | undefined = base ? { ...base, boardId: instance.boardId, pcbThickness: board?.thickness ?? base.pcbThickness } : undefined;
  if (mechanical && common) {
    mechanical = { ...mechanical, ...Object.fromEntries(constructionKeys.map(key => [key, common[key]])),
      gasketLayout: common.gasketLayout ? { ...common.gasketLayout, supports: instance.mechanical?.gasketLayout?.supports ?? [] } : mechanical.gasketLayout };
  }
  if (mechanical) mechanical = mechanicalDefaults(document, mechanical, instance.flipped);
  if (!instance.flipped) return { ...document, mechanical };
  const selected = new Set(board?.partIds);
  return {
    ...document, mechanical,
    parts: document.parts.map(part => selected.has(part.id) ? {
      ...part,
      pose: { at: reflect(part.pose.at), rotation: -part.pose.rotation },
      side: part.side === 'front' ? 'back' : 'front',
      generatorParameters: { ...part.generatorParameters, side: part.side === 'front' ? 'B' : 'F' },
    } : part),
    // A routed import is a canonical PCB reference. Reusing it after a physical
    // turn would silently show the wrong face, so the preview uses current parts.
    boardReferences: document.boardReferences?.map(reference => reference.boardId === instance.boardId ? { ...reference, enabled: false } : reference),
  };
}

export function effectiveCaseScene(document: ProjectDoc, scene: SceneDelta, instance?: PhysicalBoardInstance): SceneDelta {
  if (!instance?.flipped) return scene;
  const selected = new Set(document.boards.find(board => board.id === instance.boardId)?.partIds);
  return {
    ...scene,
    transforms: scene.transforms.map(transform => selected.has(transform.id) ? {
      ...transform, pose: { at: reflect(transform.pose.at), rotation: -transform.pose.rotation },
    } : transform),
    boardContours: scene.boardContours.map(board => board.boardId === instance.boardId ? {
      ...board, contours: board.contours.map(contour => ({ ...contour, points: contour.points.map(reflect).reverse() })),
    } : board),
  };
}

export function updateInstanceMechanical(document: ProjectDoc, instanceId: string, configuration: MechanicalConfiguration | null): ProjectDoc {
  const hardware = document.hardware;
  if (!hardware) return document;
  const common = configuration ? { ...configuration, gasketLayout: configuration.gasketLayout ? { ...configuration.gasketLayout, supports: [] } : undefined } : null;
  return { ...document, hardware: {
    ...hardware, sharedConstruction: common,
    instances: hardware.instances.map(entry => ({ ...entry, constructionLinked: Boolean(configuration),
      mechanical: !configuration ? null : entry.id === instanceId ? configuration : entry.mechanical,
    })),
  } };
}

function stable(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(stable).join(',')}]`;
  if (value && typeof value === 'object') return `{${Object.entries(value).filter(([, entry]) => entry !== undefined).sort(([a], [b]) => a.localeCompare(b)).map(([key, entry]) => `${JSON.stringify(key)}:${stable(entry)}`).join(',')}}`;
  return JSON.stringify(value) ?? 'null';
}

/** Exact dependency signature, deliberately excluding nets and document revision. */
export function mechanicalFingerprint(document: ProjectDoc, scene: SceneDelta, boardId: string, instance?: PhysicalBoardInstance): string {
  const effective = effectiveCaseDocument(document, instance);
  const board = effective.boards.find(entry => entry.id === boardId);
  const parts = effective.parts.filter(part => board?.partIds.includes(part.id));
  const definitions = effective.definitions.filter(definition => parts.some(part => part.definitionId === definition.id));
  const geometryParameters = (parameters: Record<string, unknown> | undefined, terminalNames: string[]) => Object.fromEntries(Object.entries(parameters ?? {}).filter(([key, value]) => !terminalNames.includes(key) && !(value && typeof value === 'object' && 'type' in value && value.type === 'net')));
  return stable({
    documentId: document.id, boardId, instanceId: instance?.id, flipped: instance?.flipped,
    thickness: board?.thickness, mechanical: effective.mechanical,
    parts: parts.map(part => ({ id: part.id, definitionId: part.definitionId, pose: part.pose, side: part.side, keycap: part.keycap, outline: part.outline,
      parameters: geometryParameters(part.generatorParameters, Object.keys(definitions.find(definition => definition.id === part.definitionId)?.terminals ?? {})) })),
    definitions: definitions.map(definition => ({ id: definition.id, kind: definition.kind, pads: definition.pads, courtyard: definition.courtyard,
      profile: definition.mechanicalProfile, generator: definition.generator && { ...definition.generator, parameters: geometryParameters(definition.generator.parameters, Object.keys(definition.terminals ?? {})) } })),
    contours: scene.boardContours.find(entry => entry.boardId === boardId)?.contours,
    transforms: scene.transforms.filter(transform => board?.partIds.includes(transform.id)),
    bodies: effective.caseBodies.filter(body => body.boardId === boardId),
  });
}

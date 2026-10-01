import type { CoreReply, CoreRequest, ModuleDefinition, PartDefinition, ProjectDoc } from '@boardstudio/v2-contracts';
import { demoProject } from '../demo';
import catalogue from '../modules/imported-modules.json';
import thqParts from '../parts/imported-parts.json';
import { loadVikHostConnectorDefinition } from '../modules/hostConnector';

const reviewId = 'vik-module-review';
const reviewAssumptions = [
  'Illustrative placement anchors and 3 mm face gaps; they are not measured mounting dimensions.',
  'Source PCB outlines and thicknesses are shown where recorded. The 6 mm outer, 2.8 mm hole, 3 mm high standoffs are designer-selected examples; they are not vendor fastener dimensions. Component, cable and actuator envelopes are incomplete.',
  'The source catalogue is not proof of electrical continuity, host voltage compatibility or a working firmware driver.',
  'Use the app to inspect above/below transforms, model bounds, findings and output gates. Do not treat this project as fabrication-ready.',
].join(' ');

function moduleDefinition(row: string, variant?: string): ModuleDefinition {
  const entries = (catalogue.modules as { row: string; definition: ModuleDefinition }[]).filter(entry => entry.row === row);
  const entry = variant ? entries.find(item => item.definition.variant === variant) : entries[0];
  if (!entry) throw new Error(`Missing module review source ${row}${variant ? ` (${variant})` : ''}`);
  return structuredClone(entry.definition);
}

function thqDefinition(id: string): PartDefinition {
  const entry = (thqParts.parts as { definition: PartDefinition }[]).find(item => item.definition.id === id);
  if (!entry) throw new Error(`Missing encoder review source ${id}`);
  return structuredClone(entry.definition);
}

/** A reproducible, editable review project. All module data comes from pinned source snapshots. */
export function moduleReviewProject(): ProjectDoc {
  const document = demoProject();
  document.id = reviewId;
  document.name = 'VIK module review · above and below';
  document.parameters.moduleHumanReview = {
    purpose: 'Human in-app review fixture',
    assumptions: reviewAssumptions,
    sourceRevision: 'sadekbaroudi/vik@cd5d16e4cd9137a229fc673412a89d75f4e64553',
  };
  document.parameters.demo = reviewId;

  const splitter = moduleDefinition('vik-splitter');
  const haptic = moduleDefinition('haptic-drv2605l', 'pcb/haptic-drv2605l/haptic-drv2605l · 3V3 pullups, JP1 bridged');
  const rotary = moduleDefinition('ec11-evqwgd001');
  const rotationOnly = thqDefinition('thqwgd001:rotation-reversible');
  const tactile2Pin = thqDefinition('thqwgd001:c-2pin-reversible');
  const tactile4Pin = thqDefinition('thqwgd001:c-4pin-reversible');
  document.definitions.push(rotationOnly, tactile2Pin, tactile4Pin);
  document.parts.push({ id: 'review/wheel-rotation-only', definitionId: rotationOnly.id, reference: 'ENC1', pose: { at: { x: 101, y: 18 }, rotation: 0 }, side: 'front' });
  document.boards[0].partIds.push('review/wheel-rotation-only');
  const boardEnvelope = document.outline.find(feature => feature.id === 'board-envelope');
  if (boardEnvelope?.kind === 'part-envelope') boardEnvelope.partIds.push('review/wheel-rotation-only');
  document.keymap = {
    layers: [
      {
        id: 'base', name: 'Base',
        bindings: {
          'matrix/matrix/r0c0': { kind: 'mod-tap', hold: 'LSHIFT', tap: 'A' },
          'matrix/matrix/r0c1': { kind: 'layer-tap', layerId: 'navigation', tap: 'SPACE' },
        },
        sensors: {
          'review/wheel-rotation-only': { clockwise: { kind: 'key-press', keycode: 'UP' }, counterclockwise: { kind: 'key-press', keycode: 'DOWN' } },
          'review/ec11-rotary': { clockwise: { kind: 'key-press', keycode: 'RIGHT' }, counterclockwise: { kind: 'key-press', keycode: 'LEFT' } },
          'matrix/matrix/r0c0': { clockwise: { kind: 'key-press', keycode: 'RIGHT' }, counterclockwise: { kind: 'key-press', keycode: 'LEFT' } },
          'matrix/matrix/r0c1': { clockwise: { kind: 'key-press', keycode: 'HOME' }, counterclockwise: { kind: 'key-press', keycode: 'END' } },
        },
      },
      {
        id: 'navigation', name: 'Navigation',
        bindings: {
          'matrix/matrix/r0c0': { kind: 'key-press', keycode: 'ESC' },
          'matrix/matrix/r0c1': { kind: 'macro', macroId: 'review_navigation_pulse' },
        },
        sensors: {
          'review/wheel-rotation-only': { clockwise: { kind: 'key-press', keycode: 'RIGHT' }, counterclockwise: { kind: 'key-press', keycode: 'LEFT' } },
          'review/ec11-rotary': { clockwise: { kind: 'key-press', keycode: 'UP' }, counterclockwise: { kind: 'key-press', keycode: 'DOWN' } },
          'matrix/matrix/r0c0': { clockwise: { kind: 'key-press', keycode: 'UP' }, counterclockwise: { kind: 'key-press', keycode: 'DOWN' } },
          'matrix/matrix/r0c1': { clockwise: { kind: 'key-press', keycode: 'HOME' }, counterclockwise: { kind: 'key-press', keycode: 'END' } },
        },
      },
    ],
    macros: [{id:'review_navigation_pulse',name:'Navigation pulse',tapMs:30,waitMs:0,steps:[{kind:'tap',binding:{kind:'key-press',keycode:'ENTER'}},{kind:'wait',ms:40},{kind:'tap',binding:{kind:'key-press',keycode:'SPACE'}}]}],
  };
  document.moduleDefinitions = [splitter, haptic, rotary];
  document.modules = [
    { id: 'review/splitter-above', definitionId: splitter.id, hostBoardId: 'main-board', hostFace: 'front', facingFace: 'back', at: { x: 29, y: 20 }, rotation: 0, gap: 3, attachment: 'board', detached: false, serviceClearance: 4 },
    { id: 'review/splitter-below', definitionId: splitter.id, hostBoardId: 'main-board', hostFace: 'back', facingFace: 'front', at: { x: 57, y: 20 }, rotation: 180, gap: 3, attachment: 'board', detached: false, serviceClearance: 4 },
    { id: 'review/ec11-rotary', definitionId: rotary.id, hostBoardId: 'main-board', hostFace: 'front', facingFace: 'back', at: { x: 85, y: 20 }, rotation: 0, gap: 3, attachment: 'board', detached: false, serviceClearance: 4 },
  ];
  return document;
}

export async function openModuleReviewDemo(request: (input: CoreRequest) => Promise<CoreReply>): Promise<Extract<CoreReply, { kind: 'scene' }>> {
  const document = moduleReviewProject();
  const opened = await request({ id: crypto.randomUUID(), kind: 'open', document });
  if (opened.kind !== 'scene') throw new Error(opened.kind === 'error' ? opened.message : 'Could not open the module review project');

  let attached = opened;
  const connectorDefinition = await loadVikHostConnectorDefinition();
  for (const original of opened.document.modules ?? []) {
    const instance = attached.document.modules?.find(item => item.id === original.id) ?? original;
    const definition = attached.document.moduleDefinitions?.find(item => item.id === instance.definitionId);
    if (!definition) throw new Error(`The review module definition is missing for ${instance.id}`);
    const sourceMountIds = definition.mounts.slice(0, 2).map(mount => mount.sourceId);
    const supportZ = instance.facingFace === 'front' ? 0.8 : -3.8;
    const updatedInstance = {
      ...instance,
      mountSupports: sourceMountIds.map(mountId => ({ mountId, outerDiameter: 6, holeDiameter: 2.8, z: supportZ, height: 3 })),
      connection: {
        hostConnectorPartId: '',
        modulePortId: definition.interfaces.find(port => port.role === 'module')?.id ?? '',
        busId: `review/${instance.id}/vik`,
        assignments: {},
        cableType: 'type-a-12-0.5',
        railVoltages: {},
      },
    };
    const connected = await request({
      id: crypto.randomUUID(), kind: 'edit',
      command: {
        transactionId: `review/attach/${instance.id}`, baseRevision: attached.document.revision, phase: 'commit', targetIds: [instance.id],
        operation: { kind: 'set-mounted-module', instance: updatedInstance, definition: null, hostConnectorDefinition: connectorDefinition },
      },
    });
    if (connected.kind !== 'scene') throw new Error(connected.kind === 'error' ? connected.message : `Could not connect review module ${instance.id}`);
    attached = connected;
  }

  const matrix = attached.document.matrices.find(item => item.id === 'matrix');
  if (!matrix) throw new Error('The review keyboard matrix is missing');
  const definitions: Record<string, string> = {
    '0/0': 'thqwgd001:c-2pin-reversible',
    '0/1': 'thqwgd001:c-4pin-reversible',
  };
  const replaced = await request({
      id: crypto.randomUUID(), kind: 'edit',
    command: {
      transactionId: 'review/replace-matrix-encoders', baseRevision: attached.document.revision, phase: 'commit', targetIds: [matrix.id],
      operation: {
        kind: 'set-matrix',
        matrix: {
          ...matrix,
          cells: Array.from({ length: matrix.rows * matrix.columns }, (_, index) => {
            const row = Math.floor(index / matrix.columns);
            const column = index % matrix.columns;
            const definitionId = definitions[`${row}/${column}`];
            return { row, column, enabled: true, ...(definitionId ? { definitionId, variant: definitionId } : {}) };
          }),
        },
      },
    },
  });
  if (replaced.kind !== 'scene') throw new Error(replaced.kind === 'error' ? replaced.message : 'Could not prepare the matrix encoder replacements');

  const haptic = document.moduleDefinitions!.find(definition => definition.family === 'feedback-expansion' && definition.id.includes('haptic-drv2605l'))!;
  const embedded = await request({
    id: crypto.randomUUID(), kind: 'edit',
    command: {
      transactionId: 'review/embedded-haptic', baseRevision: replaced.document.revision, phase: 'commit', targetIds: ['review/embedded-haptic'],
      operation: { kind: 'embed-module-circuit', id: 'review/embedded-haptic', definition: haptic, hostBoardId: 'main-board', pose: { at: { x: 101, y: 18 }, rotation: 0 }, side: 'front', joins: {} },
    },
  });
  if (embedded.kind !== 'scene') throw new Error(embedded.kind === 'error' ? embedded.message : 'Could not prepare the editable circuit review copy');
  const outline = embedded.document.outline.find(feature => feature.id === 'board-envelope');
  const embeddedCircuit = embedded.document.embeddedCircuits?.find(copy => copy.id === 'review/embedded-haptic');
  if (!outline || outline.kind !== 'part-envelope' || !embeddedCircuit) throw new Error('The review circuit could not be included in the host PCB envelope');
  const outlined = await request({
    id: crypto.randomUUID(), kind: 'edit',
    command: {
      transactionId: 'review/outline-circuit', baseRevision: embedded.document.revision, phase: 'commit', targetIds: [outline.id],
      operation: { kind: 'set-outline', feature: { ...outline, partIds: [...new Set([...outline.partIds, ...embeddedCircuit.partIds])] } },
    },
  });
  if (outlined.kind !== 'scene') throw new Error(outlined.kind === 'error' ? outlined.message : 'Could not include the review circuit in the host PCB outline');
  return outlined;
}

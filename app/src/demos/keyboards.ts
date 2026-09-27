import { defaultOutlineSettings, emptyProject, type CoreReply, type CoreRequest, type Part, type ProjectDoc } from '@boardstudio/v2-contracts';
import { catalogue, normalizeDefinition } from '@boardstudio/v2-ergogen';
import { appendPhysicalMatrix, physicalKeys } from './physicalLayout';
import measurements from './keyboard-layouts.json';
import { openSofleDemo, sofleDemos, type SofleVariant } from './sofle';

type KeyboardDemoId = keyof typeof measurements;
export type DemoId = SofleVariant | KeyboardDemoId;
export const keyboardDemos = Object.entries(measurements).map(([id, layout]) => ({ id: id as KeyboardDemoId, name: layout.name }));
export const demos = [...sofleDemos, ...keyboardDemos];

/** Editable library-based adaptations: measured keys, physical clusters and generated outlines. */
export function keyboardProject(id: KeyboardDemoId): ProjectDoc {
  const layout = measurements[id];
  const document = emptyProject(crypto.randomUUID(), layout.name);
  document.parameters = { demo: id, source: `${layout.repository}/blob/${layout.revision}/${layout.path}`, sourceSha256: layout.sha256, adaptation: 'Measured key layout; library switches, diode matrix, nice!nano controller bay, and a live generated outline. Source peripherals and routing are not reproduced.' };
  document.hardware = { topology: layout.split ? 'split' : 'unibody', transport: layout.split ? 'wired' : 'none', instances: [], boards: [], sharedConstruction: null };
  const library = catalogue();
  const addDefinition = (source: string, parameters: Record<string, string | number | boolean> = {}) => {
    const original = library.find(definition => definition.generator?.source === `ceoloide/${source}`);
    if (!original) throw new Error(`Missing library footprint: ${source}`);
    const definition = normalizeDefinition({ ...original, id: `demo/${source}`, generator: { ...original.generator!, parameters } });
    document.definitions.push(definition);
    return definition.id;
  };
  const controller = addDefinition('mcu_nice_nano', { include_extra_pins: true });
  const reset = addDefinition('reset_switch_tht_top');
  const connector = layout.split ? addDefinition('trrs_pj320a') : undefined;
  const recipe = physicalKeys(id, layout.keys);
  const minX = Math.min(...layout.keys.map(key => key.x - key.width * 9.525));
  const maxX = Math.max(...layout.keys.map(key => key.x + key.width * 9.525));
  const minY = Math.min(...layout.keys.map(key => key.y));
  const width = maxX - minX + 44;
  const sourceRight = id === 'lily58' || id === 'klor';
  for (const boardId of layout.split ? ['left', 'right'] : ['main']) {
    const mirror = layout.split && ((boardId === 'right') !== sourceRight);
    const shift = boardId === 'right' ? width + 25 : 0;
    for (const cluster of new Set(recipe.map(key => key.cluster))) {
      const members = recipe.filter(key => key.cluster === cluster);
      const lastColumn = Math.max(...members.map(key => key.column));
      appendPhysicalMatrix(document, members.map(key => ({ ...key,
        column: mirror ? lastColumn - key.column : key.column,
        x: (mirror ? maxX - key.x : key.x - minX) + 6 + shift,
        y: minY - key.y,
        rotation: mirror ? -key.rotation : key.rotation,
      })), `${boardId}-${cluster}`, boardId, layout.choc ? 'choc-hotswap' : 'mx-hotswap', { x: layout.choc ? 18 : 19.05, y: layout.choc ? 17 : 19.05 });
    }
    const parts: Part[] = [];
    const bayX = shift + width - 16;
    parts.push({ id: `${boardId}/U1`, definitionId: controller, reference: `${boardId}-U1`, side: 'front', pose: { at: { x: bayX, y: -15 }, rotation: 0 } });
    parts.push({ id: `${boardId}/RST`, definitionId: reset, reference: `${boardId}-RST`, side: 'front', pose: { at: { x: bayX, y: -45 }, rotation: 0 } });
    if (connector) parts.push({ id: `${boardId}/TRRS`, definitionId: connector, reference: `${boardId}-TRRS`, side: 'front', pose: { at: { x: bayX, y: -65 }, rotation: 0 } });
    document.parts.push(...parts);
    const outlineId = `${boardId}-outline`;
    const boardParts = document.parts.filter(part => part.id.startsWith(`${boardId}/`) || part.id.startsWith(`matrix/${boardId}-`));
    document.outline.push({ id: outlineId, kind: 'part-envelope', partIds: boardParts.map(part => part.id), margin: 4, settings: defaultOutlineSettings, operation: 'add' });
    document.boards.push({ id: boardId, name: boardId === 'main' ? 'Keyboard PCB' : `${boardId === 'left' ? 'Left' : 'Right'} PCB`, thickness: 1.6, partIds: boardParts.map(part => part.id), netIds: [], outlineIds: [outlineId] });
    document.layouts!.find(layout => layout.boardId === boardId)!.partIds = parts.map(part => part.id);
    document.hardware.instances.push({ id: boardId, name: boardId, boardId, half: boardId, role: boardId === 'right' ? 'peripheral' : 'central', flipped: false, controllerPartId: `${boardId}/U1`, mechanical: null, constructionLinked: false });
    document.hardware.boards.push({ boardId, controllerPartId: `${boardId}/U1`, mode: 'matrix', locks: {}, assignments: {}, keyBindings: {}, jumperStates: {}, protectedHandoff: null });
  }
  return document;
}

export async function openKeyboardDemo(id: DemoId, request: (input: CoreRequest) => Promise<CoreReply>): Promise<Extract<CoreReply, { kind: 'scene' }>> {
  if (id === 'v2' || id === 'rgb' || id === 'choc') return openSofleDemo(id, request);
  let opened = await request({ id: crypto.randomUUID(), kind: 'open', document: keyboardProject(id) });
  if (opened.kind !== 'scene') throw new Error(opened.kind === 'error' ? opened.message : 'Could not open demo');
  for (const board of opened.document.boards) {
    const resolved = await request({ id: crypto.randomUUID(), kind: 'resolve-electrical', request: { document: opened.document, instanceId: board.id, boardId: board.id, controllerPartId: `${board.id}/U1`, controllerProfile: null, mode: 'matrix', locks: {} } });
    if (resolved.kind !== 'electrical-resolved') throw new Error(resolved.kind === 'error' ? resolved.message : 'Could not wire demo');
    const errors = resolved.plan.diagnostics.filter(diagnostic => diagnostic.severity === 'error');
    if (errors.length) throw new Error(errors.map(error => error.message).join('; '));
    opened = await request({ id: crypto.randomUUID(), kind: 'apply-electrical', baseRevision: opened.document.revision, plan: resolved.plan, draft: false });
    if (opened.kind !== 'scene') throw new Error(opened.kind === 'error' ? opened.message : 'Could not apply demo wiring');
  }
  return opened;
}

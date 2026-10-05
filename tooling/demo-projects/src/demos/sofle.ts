import { defaultOutlineSettings, emptyProject, type CoreReply, type CoreRequest, type Part, type ProjectDoc, type Vec2 } from '@boardstudio/v2-contracts';
import { catalogue, normalizeDefinition } from '@boardstudio/v2-ergogen';
import { appendPhysicalMatrix } from './physicalLayout';
import measurements from './sofle-layouts.json';

export type SofleVariant = keyof typeof measurements.layouts;
export const sofleDemos: { id: SofleVariant; name: string }[] = [
  { id: 'v2', name: 'Sofle v2' },
  { id: 'rgb', name: 'Sofle RGB' },
  { id: 'choc', name: 'Sofle Choc' },
];

/** Measured key geometry with supported library assemblies, not imported PCBs. */
function sofleProject(variant: SofleVariant): ProjectDoc {
  const layout = measurements.layouts[variant];
  const document = emptyProject(crypto.randomUUID(), sofleDemos.find(demo => demo.id === variant)!.name);
  document.parameters = { demo: `sofle-${variant}`, source: `${measurements.repository}/blob/${measurements.revision}/${layout.path}`, sourceSha256: layout.sha256 };
  document.hardware = { topology: 'split', transport: 'wired', instances: [], boards: [], sharedConstruction: null };
  const library = catalogue();
  const addDefinition = (source: string, parameters: Record<string, string | number | boolean> = {}) => {
    const original = library.find(definition => definition.generator?.source === `ceoloide/${source}`);
    if (!original) throw new Error(`Missing bundled footprint: ${source}`);
    const definition = normalizeDefinition({ ...original, id: `sofle/${source}`, generator: { ...original.generator!, parameters } });
    document.definitions.push(definition);
    return definition.id;
  };
  const controller = addDefinition('mcu_nice_nano', { include_extra_pins: true });
  const encoder = addDefinition('rotary_encoder_ec11_ec12');
  const display = addDefinition('display_ssd1306');
  const connector = addDefinition('trrs_pj320a');
  const reset = addDefinition('reset_switch_tht_top');
  const hole = addDefinition('mounting_hole_npth');
  const underglow = variant === 'rgb' ? addDefinition('led_sk6812mini-e', { side: 'B', reverse_mount: false }) : undefined;
  const minX = Math.min(...layout.outline.map(point => point.x));
  const maxX = Math.max(...layout.outline.map(point => point.x));
  const minY = Math.min(...layout.outline.map(point => point.y));
  const width = maxX - minX;
  const component = (reference: string) => {
    const result = layout.components.find(part => part.reference === reference);
    if (!result) throw new Error(`Missing Sofle measurement: ${reference}`);
    return result;
  };

  for (const half of ['left', 'right'] as const) {
    // Upstream draws the right half. Both replacements have front-mounted keys.
    const position = (point: Vec2): Vec2 => ({ x: half === 'left' ? maxX - point.x : point.x - minX + width + 30, y: minY - point.y });
    const angle = (rotation: number) => half === 'left' ? -rotation : rotation;
    const start = document.parts.length;
    const measuredKeys = layout.components.filter(part => /^SW\d+$/.test(part.reference) && part.reference !== 'SW25');
    for (const cluster of ['keys', 'thumbs']) {
      const members = measuredKeys.filter(key => (Number(key.reference.slice(2)) <= 24) === (cluster === 'keys'));
      appendPhysicalMatrix(document, members.map(key => {
        const number = Number(key.reference.slice(2));
        const column = cluster === 'keys' ? (number - 1) % 6 : number - 26;
        const point = position(key);
        return { ...key, ...point, width: 1, cluster,
          row: cluster === 'keys' ? 3 - Math.floor((number - 1) / 6) : 0,
          column: half === 'left' ? (cluster === 'keys' ? 5 : 4) - column : column,
          rotation: angle(key.rotation - (variant === 'v2' ? 0 : 180)),
        };
      }), `${half}-${cluster}`, half, variant === 'v2' ? 'mx-hotswap' : variant === 'rgb' ? 'mx-hotswap-rgb' : 'choc-hotswap-rgb', { x: 19.05, y: 19.05 });
    }
    const parts: Part[] = [];
    const addPart = (reference: string, definitionId: string, rotation = 0, offset = { x: 0, y: 0 }) => {
      const measured = component(reference);
      const part: Part = { id: `${half}/${reference}`, reference: `${half}-${reference}`, definitionId, side: 'front', pose: { at: position({ x: measured.x + offset.x, y: measured.y + offset.y }), rotation: angle(rotation) } };
      parts.push(part);
      return part.id;
    };
    const controllerId = addPart('U1', controller);
    addPart('SW25', encoder);
    // Align the centered OLED header (local Y -16.7) to source pin 1.
    addPart('J3', display, 0, { x: 3.81, y: -16.7 });
    // The PJ-320A replaces the upstream jack; pull its body inside the board edge.
    addPart('J2', connector, 90, { x: 4, y: 0 });
    addPart('RSW1', reset, 90, { x: 1, y: 0 });
    for (const measured of layout.components.filter(part => part.reference.startsWith('TH'))) addPart(measured.reference, hole);
    if (underglow) {
      for (const measured of layout.components.filter(part => /^D3[1-7]$/.test(part.reference))) {
        addPart(measured.reference, underglow, measured.rotation);
        parts[parts.length - 1].side = 'back';
      }
    }
    document.parts.push(...parts);
    const outlineId = `${half}-outline`;
    const boardParts = document.parts.slice(start);
    document.outline.push({ id: outlineId, kind: 'part-envelope', operation: 'add', partIds: boardParts.map(part => part.id), margin: 4, settings: defaultOutlineSettings });
    document.boards.push({ id: half, name: `${half === 'left' ? 'Left' : 'Right'} PCB`, thickness: 1.6, partIds: boardParts.map(part => part.id), netIds: [], outlineIds: [outlineId] });
    document.layouts!.find(layout => layout.boardId === half)!.partIds = parts.map(part => part.id);
    document.hardware.instances.push({ id: half, name: `${half} half`, boardId: half, half, role: half === 'left' ? 'central' : 'peripheral', flipped: false, controllerPartId: controllerId, mechanical: null, constructionLinked: false });
    document.hardware.boards.push({ boardId: half, controllerPartId: controllerId, mode: 'matrix', locks: {}, assignments: {}, keyBindings: {}, jumperStates: {}, protectedHandoff: null });
  }
  return document;
}

/** Use the same reviewed wiring resolver and atomic apply operation as the UI. */
export async function openSofleDemo(variant: SofleVariant, request: (request: CoreRequest) => Promise<CoreReply>): Promise<Extract<CoreReply, { kind: 'scene' }>> {
  let opened = await request({ id: crypto.randomUUID(), kind: 'open', document: sofleProject(variant) });
  if (opened.kind !== 'scene') throw new Error(opened.kind === 'error' ? opened.message : 'Could not open demo');
  for (const boardId of ['left', 'right']) {
    const result = await request({ id: crypto.randomUUID(), kind: 'resolve-electrical', request: { document: opened.document, instanceId: boardId, boardId, controllerPartId: `${boardId}/U1`, controllerProfile: null, mode: 'matrix', locks: {} } });
    if (result.kind !== 'electrical-resolved') throw new Error(result.kind === 'error' ? result.message : 'Could not wire demo');
    const errors = result.plan.diagnostics.filter(diagnostic => diagnostic.severity === 'error');
    if (errors.length) throw new Error(errors.map(error => error.message).join('; '));
    opened = await request({ id: crypto.randomUUID(), kind: 'apply-electrical', baseRevision: opened.document.revision, plan: result.plan, draft: false });
    if (opened.kind !== 'scene') throw new Error(opened.kind === 'error' ? opened.message : 'Could not apply demo wiring');
  }
  return opened;
}

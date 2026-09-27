import type { Pad, Part, PartDefinition, Vec2 } from '../../contracts/src/index.ts';
// The generated module wraps reviewed CommonJS sources without changing their bytes.
// @ts-expect-error Generated during prepare:catalog.
import modules from '../generated/catalogue.mjs';

type ErgogenValue = string | number | boolean | ErgogenValue[] | { [key: string]: ErgogenValue };
type ErgogenParameter = { type: 'string' | 'number' | 'boolean' | 'array' | 'object' | 'net' | 'anchor'; value: ErgogenValue | undefined };
type Generator = { params: Record<string, unknown>; body: (context: Record<string, unknown>) => string };
export type Expression = string | Expression[];
const generators = modules as Record<string, Generator>;

export function isErgogen(source: string | undefined): boolean {
  return Boolean(source && Object.hasOwn(generators, source));
}

function normalize(value: unknown): ErgogenParameter {
  if (value && typeof value === 'object' && !Array.isArray(value) && 'type' in value) {
    const input = value as { type: ErgogenParameter['type']; value?: ErgogenValue };
    return { type: input.type, value: input.value };
  }
  const type = Array.isArray(value) ? 'array' : value == null ? 'net' : typeof value;
  return { type: type === 'object' ? 'object' : type as ErgogenParameter['type'], value: value as ErgogenValue };
}

export function parameters(source: string): Record<string, ErgogenParameter> {
  const generator = generators[source];
  if (!generator) throw new Error(`Unknown Ergogen generator: ${source}`);
  // Show keycap guides by default; explicit saved parameter values still win.
  return Object.fromEntries(Object.entries(generator.params).map(([name, value]) => [
    name, name === 'include_keycap' ? { type: 'boolean', value: true } : normalize(value),
  ]));
}

export function catalogue(): PartDefinition[] {
  return Object.keys(generators).sort().map((source) => {
    const name = source.split('/')[1].replaceAll('_', ' ').replaceAll('-', ' ');
    const utility = /\/utility_|\/(?:text|point_debugger|icon_bat)$/u.test(source);
    const keyboardSwitch = ['ceoloide/switch_mx', 'ceoloide/switch_choc_v1_v2', 'ceoloide/switch_gateron_ks27_ks33'].includes(source);
    const kind = utility ? 'utility'
      : keyboardSwitch ? 'switch'
      : /mcu|nano|nrf/.test(source) ? 'controller'
      : /connector|conn_|trrs|socket/.test(source) ? 'connector'
      : /encoder/.test(source) ? 'encoder'
      : /diode|led|smd/.test(source) ? 'passive' : 'custom';
    const definition: PartDefinition = {
      id: `ergogen:${source}`, name, kind, pads: [], courtyard: [],
      ...(keyboardSwitch ? { keycap: { x: 18, y: 18 }, matrixTerminals: { row: 'from', column: 'to' } } : {}),
      envelopeSource: {}, terminals: {},
      generator: { source, version: 'bundled-1', parameters: {} },
    };
    try {
      return normalizeDefinition(definition);
    } catch (error) {
      throw new Error(`Cannot catalog ${source}: ${error instanceof Error ? error.message : String(error)}`);
    }
  });
}

const validPolygon = (points: Vec2[]) => points.length >= 3 && points.every((point) => Number.isFinite(point.x) && Number.isFinite(point.y));

/** The one adapter used for generated catalog geometry and subsequently edited definitions. */
export function normalizeDefinition(input: PartDefinition): PartDefinition {
  if (!isErgogen(input.generator?.source)) return input;
  const source = input.generator!.source;
  const generated = geometry(render(input));
  const pads = generated.pads.map((pad, index) => {
    const previous = input.pads[index];
    return previous ? { ...pad, id: previous.id, ...(previous.netId ? { netId: previous.netId } : {}) } : pad;
  });
  const keyboardSwitch = input.kind === 'switch' && input.matrixTerminals !== undefined;
  const old = input.envelopeSource ?? {};
  const courtyardAuthored = old.courtyard === 'authored' && validPolygon(input.courtyard);
  const keycapAuthored = old.keycap === 'authored' && Boolean(input.keycap && input.keycap.x > 0 && input.keycap.y > 0);
  const dims = keycapParameters(source, input.generator!.parameters);
  const keycap = keycapAuthored ? input.keycap : dims ? { x: dims[0], y: dims[1] } : input.keycap;
  const terminals = Object.fromEntries(Object.entries(terminalPads(input)).map(([name, ids]) => [name, ids.map((id) => {
    const index = Number(id.slice('pad-'.length));
    return pads[index]?.id ?? id;
  })]));
  const ext = generated.courtyard;
  return {
    ...input,
    pads,
    courtyard: courtyardAuthored ? input.courtyard : ext,
    ...(keycap ? { keycap } : {}),
    envelopeSource: { courtyard: courtyardAuthored ? 'authored' : 'generated', ...(keycap ? { keycap: keycapAuthored ? 'authored' : 'generated' } : {}) },
    terminals: Object.keys(terminals).length ? terminals : input.terminals ?? {},
    ...(keyboardSwitch ? { matrixTerminals: input.matrixTerminals } : {}),
    ...(!courtyardAuthored && generated.courtyardFallback && input.kind !== 'utility' ? { envelopeNotice: 'No closed courtyard is available; the outline uses physical graphics and pad extents.' } : { envelopeNotice: undefined }),
  };
}

function keycapParameters(source: string, values: Record<string, unknown>): [number, number] | undefined {
  const keys = ['keycap_width', 'keycap_height'];
  if (!parameters(source)[keys[0]]) return undefined;
  const defaults = parameters(source);
  const width = Number(values[keys[0]] ?? defaults[keys[0]]?.value);
  const height = Number(values[keys[1]] ?? defaults[keys[1]]?.value);
  if (!Number.isFinite(width) || width <= 0 || !Number.isFinite(height) || height <= 0) {
    throw new Error('Keycap envelope dimensions must be greater than zero.');
  }
  return [width, height];
}

function terminalPads(definition: PartDefinition): Record<string, string[]> {
  const source = definition.generator?.source;
  if (!source || !isErgogen(source)) return {};
  const schema = parameters(source);
  const result: Record<string, string[]> = {};
  for (const [name, parameter] of Object.entries(schema)) {
    if (parameter.type !== 'net') continue;
    const marker = `__boardstudio_terminal_${name}__`;
    const draft = { ...definition, generator: { ...definition.generator!, parameters: { ...definition.generator!.parameters, [name]: marker } } };
    for (const form of render(draft, { netIndex: () => 1 })) {
      if (form[0] !== 'footprint' && form[0] !== 'module') continue;
      let index = 0;
      for (const pad of children(form, 'pad')) {
        const net = child(pad, 'net');
        if (net && value(net[2]) === marker) (result[name] ??= []).push(`pad-${index}`);
        index += 1;
      }
    }
  }
  return result;
}

function tokens(source: string): string[] {
  const result: string[] = [];
  const pattern = /\s*(\(|\)|"(?:\\.|[^"\\])*"|[^\s()]+)/guy;
  let offset = 0;
  while (offset < source.length) {
    pattern.lastIndex = offset;
    const match = pattern.exec(source);
    if (!match) {
      if (/^\s*$/u.test(source.slice(offset))) break;
      throw new Error(`Invalid Ergogen output at byte ${offset}`);
    }
    result.push(match[1]);
    offset = pattern.lastIndex;
  }
  return result;
}

export function parseForms(source: string): Expression[][] {
  const input = tokens(source);
  let cursor = 0;
  const read = (): Expression => {
    const token = input[cursor++];
    if (token === undefined || token === ')') throw new Error('Unbalanced Ergogen output');
    if (token !== '(') return token.startsWith('"') ? `\u0000${JSON.parse(token) as string}` : token;
    const result: Expression[] = [];
    while (input[cursor] !== ')') {
      if (cursor >= input.length) throw new Error('Unbalanced Ergogen output');
      result.push(read());
    }
    cursor++;
    return result;
  };
  const forms: Expression[][] = [];
  while (cursor < input.length) {
    const node = read();
    if (!Array.isArray(node)) throw new Error('Ergogen output must contain KiCad forms');
    forms.push(node);
  }
  return forms;
}

export function serialize(node: Expression): string {
  if (Array.isArray(node)) return `(${node.map(serialize).join(' ')})`;
  return node.startsWith('\u0000') ? JSON.stringify(node.slice(1)) : node;
}

export function value(node: Expression | undefined): string {
  if (typeof node !== 'string') throw new Error('Expected Ergogen scalar');
  return node.startsWith('\u0000') ? node.slice(1) : node;
}

export function child(node: Expression[], name: string): Expression[] | undefined {
  return node.find((item): item is Expression[] => Array.isArray(item) && item[0] === name);
}

function children(node: Expression[], name: string): Expression[][] {
  return node.filter((item): item is Expression[] => Array.isArray(item) && item[0] === name);
}

function number(node: Expression | undefined): number {
  const result = Number(node);
  if (!Number.isFinite(result)) throw new Error('Invalid numeric Ergogen geometry');
  return result;
}

export function geometry(forms: Expression[][]): { pads: Pad[]; courtyard: Vec2[]; courtyardFallback: boolean } {
  const footprints = forms.filter((form) => form[0] === 'footprint' || form[0] === 'module');
  const pads: Pad[] = [];
  const courtyardPoints: Vec2[] = [];
  const bodyPoints: Vec2[] = [];
  const courtyardSegments: [Vec2, Vec2][] = [];
  const courtyardPolygons: Vec2[][] = [];
  for (const footprint of footprints) {
    for (const item of children(footprint, 'pad')) {
      const at = child(item, 'at');
      const size = child(item, 'size');
      if (!at || !size) throw new Error('Ergogen pad is missing position or size');
      const drill = child(item, 'drill');
      const layers = child(item, 'layers');
      const shape = String(item[3]);
      const rotation = at[3] === undefined ? 0 : -number(at[3]);
      const angle = rotation * Math.PI / 180;
      const hx = number(size[1]) / 2;
      const hy = number(size[2]) / 2;
      for (const [x, y] of [[-hx,-hy],[hx,-hy],[hx,hy],[-hx,hy]]) {
        bodyPoints.push({ x: number(at[1]) + x * Math.cos(angle) - y * Math.sin(angle), y: -number(at[2]) - x * Math.sin(angle) - y * Math.cos(angle) });
      }
      pads.push({
        id: `pad-${pads.length}`,
        number: value(item[1]),
        at: { x: number(at[1]), y: -number(at[2]) },
        size: { x: number(size[1]), y: number(size[2]) },
        shape: ['circle', 'oval', 'rect', 'roundrect'].includes(shape) ? shape as Pad['shape'] : 'rect',
        ...(drill ? { drill: drill[1] === 'oval' ? Math.min(number(drill[2]), number(drill[3])) : number(drill[1]) } : {}),
        ...(item[2] === 'np_thru_hole' ? { plated: false } : {}),
        ...(layers?.includes('B.Cu') && !layers.includes('F.Cu') ? { side: 'back' as const } : {}),
        ...(at[3] !== undefined ? { rotation } : {}),
      });
    }
    for (const item of footprint.slice(1).filter((entry): entry is Expression[] => Array.isArray(entry))) {
      const graphic = String(item[0]);
      if (!['fp_line', 'fp_rect', 'fp_poly', 'fp_circle', 'fp_arc'].includes(graphic)) continue;
      const layer = value(child(item, 'layer')?.[1]);
      if (!/^(F|B)\.(?:CrtYd|Fab|SilkS)$/u.test(layer)) continue;
      const destination = layer.endsWith('CrtYd') ? courtyardPoints : bodyPoints;
      const courtyardGraphic = layer.endsWith('CrtYd');
      const point = (node: Expression[]): Vec2 => ({ x: number(node[1]), y: -number(node[2]) });
      for (const name of ['start', 'end', 'center', 'mid', 'xy']) {
        for (const point of children(item, name)) {
          if (point.length >= 3 && !Array.isArray(point[1])) destination.push({ x: number(point[1]), y: -number(point[2]) });
        }
      }
      for (const polygon of children(item, 'pts')) {
        const points = children(polygon, 'xy').map(point);
        destination.push(...points);
        if (courtyardGraphic && points.length >= 3) courtyardPolygons.push(points);
      }
      if (graphic === 'fp_rect') {
        const start = child(item, 'start'); const end = child(item, 'end');
        if (start && end) {
          const rectangle = [[start[1],start[2]],[end[1],start[2]],[end[1],end[2]],[start[1],end[2]]].map(([x,y]) => ({x:number(x), y:-number(y)}));
          destination.push(...rectangle);
          if (courtyardGraphic) courtyardPolygons.push(rectangle);
        }
      }
      if (graphic === 'fp_line' && courtyardGraphic) {
        const start = child(item, 'start'); const end = child(item, 'end');
        if (start && end) courtyardSegments.push([point(start), point(end)]);
      }
    }
  }
  // Courtyard graphics win when present. Otherwise bound physical fab/silkscreen
  // and the rotated pad extents; labels, models, and copper are never candidates.
  const closed = [...courtyardPolygons, ...joinSegments(courtyardSegments)];
  const hasClosedCourtyard = closed.length > 0;
  const source = hasClosedCourtyard
    ? closed.length === 1 ? closed[0] : bounds(closed.flat())
    : courtyardPoints.length >= 3 ? courtyardPoints : bodyPoints;
  const physical = (values: Vec2[]) => bounds(values);
  let courtyard = hasClosedCourtyard && closed.length === 1 ? source : physical(source);
  if (courtyard.length < 3) courtyard = physical(pads.flatMap((pad) => {
    const angle = (pad.rotation ?? 0) * Math.PI / 180; const hx = pad.size.x / 2; const hy = pad.size.y / 2;
    return [[-hx,-hy],[hx,-hy],[hx,hy],[-hx,hy]].map(([x,y]) => ({x:pad.at.x + x*Math.cos(angle)-y*Math.sin(angle), y:pad.at.y+x*Math.sin(angle)+y*Math.cos(angle)}));
  }));
  return { pads, courtyard, courtyardFallback: !hasClosedCourtyard };
}

function joinSegments(source: [Vec2, Vec2][]): Vec2[][] {
  const pending = [...source];
  const loops: Vec2[][] = [];
  const same = (a: Vec2, b: Vec2) => Math.hypot(a.x - b.x, a.y - b.y) < 0.03;
  while (pending.length) {
    const [first] = pending.splice(0, 1);
    const path = [first[0], first[1]];
    while (pending.length && !same(path[path.length - 1], path[0])) {
      const index = pending.findIndex(([a, b]) => same(a, path[path.length - 1]) || same(b, path[path.length - 1]));
      if (index < 0) break;
      const [a, b] = pending.splice(index, 1)[0];
      path.push(same(a, path[path.length - 1]) ? b : a);
    }
    if (path.length >= 4 && same(path[path.length - 1], path[0])) loops.push(path.slice(0, -1));
  }
  return loops;
}

function bounds(points: Vec2[]): Vec2[] {
  if (!points.length) return [];
  const xs = points.map(({x}) => x); const ys = points.map(({y}) => y);
  const minX = Math.min(...xs); const maxX = Math.max(...xs); const minY = Math.min(...ys); const maxY = Math.max(...ys);
  if (maxX - minX < 0.01 || maxY - minY < 0.01) return [];
  return [{x:minX,y:minY},{x:maxX,y:minY},{x:maxX,y:maxY},{x:minX,y:maxY}];
}

export function modelAssetId(path: string): string | undefined {
  const attached = path.match(/^boardstudio-asset:([a-zA-Z0-9_-]+)$/u);
  if (attached) return attached[1];
  const bundled = path.match(/^\$\{KIPRJMOD\}\/models\/boardstudio\/(.+)$/u);
  if (bundled) return `ergogen:model:${bundled[1]}`;
  const infused = path.match(/^\$\{EG_INFUSED_KIM_3D_MODELS\}\/(.+)$/u);
  if (infused) return `ergogen:model:infused-kim/${infused[1]}`;
  return undefined;
}

export function modelAssetIds(definition: PartDefinition, part?: Part): string[] {
  const paths = render(definition, { part }).flatMap((form) =>
    form[0] === 'footprint' || form[0] === 'module'
      ? children(form, 'model').map((model) => value(model[1]))
      : []);
  return [...new Set(paths.map((path) => {
    const id = modelAssetId(path);
      if (!id) throw new Error(`Ergogen model path is not attached or bundled: ${path}`);
    return id;
  }))];
}

export function modelBindings(definition: PartDefinition, part?: Part): NonNullable<PartDefinition['models']> {
  const xyz = (form: Expression[], name: string, fallback: Vec2 & { z: number }) => {
    const entry = child(child(form, name) ?? [], 'xyz');
    return entry ? { x: number(entry[1]), y: number(entry[2]), z: number(entry[3]) } : fallback;
  };
  return render(definition, { part }).flatMap((form) => {
    if (form[0] !== 'footprint' && form[0] !== 'module') return [];
    return children(form, 'model').map((model) => {
      const path = value(model[1]);
      const assetId = modelAssetId(path);
      const resolvedAssetId = assetId ?? `unresolved-model:${encodeURIComponent(path)}`;
      const offset = xyz(model, 'offset', { x: 0, y: 0, z: 0 });
      const rotation = xyz(model, 'rotate', { x: 0, y: 0, z: 0 });
      return {
        assetId: resolvedAssetId,
        offset: { ...offset, y: -offset.y },
        rotation: { ...rotation, z: -rotation.z },
        scale: xyz(model, 'scale', { x: 1, y: 1, z: 1 }),
      };
    });
  });
}

function xy(x: number, y: number): string { return `${x} ${y}`; }

type RenderOptions = {
  part?: Part;
  netIndex?: (name: string) => number;
};

export function render(definition: PartDefinition, options: RenderOptions = {}): Expression[][] {
  const source = definition.generator?.source;
  const generator = source && generators[source];
  if (!generator) throw new Error(`Unknown Ergogen generator: ${source}`);
  if (definition.generator?.version !== 'bundled-1') throw new Error(`Unsupported Ergogen generator version: ${definition.generator?.version}`);
  const part = options.part;
  const pose = part?.pose ?? { at: { x: 0, y: 0 }, rotation: 0 };
  const mirrored = part?.side === 'back';
  const netIndex = options.netIndex ?? (() => 0);
  const params = parameters(source);
  const inputs = { ...definition.generator.parameters, ...(part?.generatorParameters ?? {}) } as Record<string, unknown>;
  const p: Record<string, unknown> = {};
  for (const [key, param] of Object.entries(params)) {
    const value = inputs[key] ?? param.value;
    if (param.type === 'net') {
      const name = value === undefined ? '' : String(value);
      const index = name ? netIndex(name) : 0;
      const str = `(net ${index} ${JSON.stringify(name)})`;
      p[key] = { name, index, str, toString: () => str };
    } else if (param.type === 'anchor') {
      const anchor = value as { x?: number; y?: number } | undefined;
      p[key] = { x: anchor?.x ?? pose.at.x, y: -(anchor?.y ?? pose.at.y), r: pose.rotation };
    } else {
      p[key] = value;
    }
  }
  const angle = pose.rotation * Math.PI / 180;
  const transform = (x: number, y: number, resist: boolean): Vec2 => {
    const sx = resist || !mirrored ? x : -x;
    return { x: pose.at.x + sx * Math.cos(angle) + y * Math.sin(angle), y: -pose.at.y - sx * Math.sin(angle) + y * Math.cos(angle) };
  };
  p.side = inputs.side ?? (mirrored ? 'B' : 'F');
  p.ref = part?.reference ?? 'REF**';
  p.ref_hide = '';
  p.point = { x: pose.at.x, y: pose.at.y, r: pose.rotation, meta: { mirrored } };
  p.x = pose.at.x;
  p.y = -pose.at.y;
  p.r = pose.rotation;
  p.rot = pose.rotation;
  p.xy = xy(pose.at.x, -pose.at.y);
  p.at = `(at ${p.xy} ${pose.rotation})`;
  p.isxy = (x: number, y: number) => xy(mirrored ? -x : x, y);
  p.iaxy = (x: number, y: number) => xy(x, y);
  p.esxy = (x: number, y: number) => { const at = transform(x, y, false); return xy(at.x, at.y); };
  p.eaxy = (x: number, y: number) => { const at = transform(x, y, true); return xy(at.x, at.y); };
  p.local_net = (suffix: string) => {
    const name = `__boardstudio_local_${encodeURIComponent(part?.id ?? definition.id)}_${encodeURIComponent(String(suffix))}`;
    const index = netIndex(name);
    const str = `(net ${index} ${JSON.stringify(name)})`;
    return { name, index, str, toString: () => str };
  };
  const output = generator.body(p);
  return parseForms(output);
}

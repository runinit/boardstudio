import type {
  Matrix,
  MatrixCell,
  Part,
  PartDefinition,
  Vec2
} from '../../../contracts/src/index';

export const unit = 10;

export const PITCH_MM = 19.05;

export const nudgeStep = 0.1;

export const nudgeLargeStep = 1;

export const makeId = (): string => `ui-${globalThis.crypto?.randomUUID?.() ?? Date.now().toString(36)}`;

export const isTyping = (target: EventTarget | null): boolean => target instanceof HTMLElement && ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName);

export const courtyardSize = (points: Vec2[]): Vec2 => {
  if (points.length === 0) return { x: 10, y: 6 };
  const xs = points.map((point) => point.x);
  const ys = points.map((point) => point.y);
  return { x: Math.max(...xs) - Math.min(...xs), y: Math.max(...ys) - Math.min(...ys) };
};

export const definitionIssues = (definition: PartDefinition): string[] => {
  const issues: string[] = [];
  if (!definition.name.trim()) issues.push('Name is required.');
  const size = courtyardSize(definition.courtyard);
  if (definition.courtyard.length < 3 || !Number.isFinite(size.x) || !Number.isFinite(size.y) || size.x <= 0 || size.y <= 0) {
    issues.push('Courtyard must have positive width and height.');
  }
  const ids = new Set<string>();
  const numbers = new Set<string>();
  for (const [index, pad] of definition.pads.entries()) {
    if (!pad.id.trim() || ids.has(pad.id)) issues.push(`Pad ${index + 1} needs a unique ID.`);
    ids.add(pad.id);
    if (!definition.kicadSource) {
      if (!pad.number.trim()) issues.push(`Pad ${index + 1} needs a number.`);
      if (numbers.has(pad.number)) issues.push(`Pad ${index + 1} number must be unique.`);
      numbers.add(pad.number);
    }
    if (!Number.isFinite(pad.at.x) || !Number.isFinite(pad.at.y)) issues.push(`Pad ${index + 1} position must be finite.`);
    if (!Number.isFinite(pad.size.x) || !Number.isFinite(pad.size.y) || pad.size.x <= 0 || pad.size.y <= 0) {
      issues.push(`Pad ${index + 1} size must be positive.`);
    }
    if (pad.drill !== undefined && (!Number.isFinite(pad.drill) || pad.drill <= 0)) issues.push(`Pad ${index + 1} drill must be positive.`);
  }
  return issues;
};

export const withCell = (matrix: Matrix, row: number, column: number, changes: Partial<MatrixCell>): Matrix => {
  const cells = matrix.cells ?? [];
  const current = cells.find((cell) => cell.row === row && cell.column === column);
  const next: MatrixCell = { row, column, enabled: current?.enabled ?? true, ...current, ...changes };
  return { ...matrix, cells: cells.filter((cell) => cell.row !== row || cell.column !== column).concat(next) };
};

export const localMatrixDelta = (matrix: Matrix, delta: Vec2, column = 0): Vec2 => {
  const angle = -(matrix.rotation ?? 0) * Math.PI / 180;
  let x = delta.x * Math.cos(angle) - delta.y * Math.sin(angle);
  let y = delta.x * Math.sin(angle) + delta.y * Math.cos(angle);
  if (matrix.mirror === 'x') x *= -1;
  if (matrix.mirror === 'y') y *= -1;
  const splay = -(matrix.columnSplays ?? []).slice(0, column + 1).reduce((sum, value) => sum + value, 0) * Math.PI / 180;
  return { x: x * Math.cos(splay) - y * Math.sin(splay), y: x * Math.sin(splay) + y * Math.cos(splay) };
};

/** Positive values are unit fractions; negative values are shared mm steps. */
export const snapDelta = (delta: Vec2, pitch: Vec2, fraction: number): Vec2 => fraction === 0 ? delta : ({
  x: Math.round(delta.x / Math.max(0.001, (fraction < 0 ? -fraction : pitch.x * fraction))) * (fraction < 0 ? -fraction : pitch.x * fraction),
  y: Math.round(delta.y / Math.max(0.001, (fraction < 0 ? -fraction : pitch.y * fraction))) * (fraction < 0 ? -fraction : pitch.y * fraction),
});

export const createPart = (definition: PartDefinition, parts: Part[]): Part => {
  const prefix = definition.kind === 'switch' ? 'S' : definition.kind === 'controller' ? 'U' : definition.kind === 'encoder' ? 'E' : 'J';
  const references = new Set(parts.map((part) => part.reference));
  let count = 1;
  while (references.has(`${prefix}${count}`)) count += 1;
  return {
    id: makeId(),
    definitionId: definition.id,
    reference: `${prefix}${count}`,
    pose: { at: { x: 0, y: 0 }, rotation: 0 },
    side: 'front',
  };
};

export const pointFromEvent = (event: { clientX: number; clientY: number }, svg: SVGSVGElement | null): Vec2 | null => {
  if (!svg) return null;
  const matrix = svg.getScreenCTM();
  if (!matrix) return null;
  const point = svg.createSVGPoint();
  point.x = event.clientX;
  point.y = event.clientY;
  const local = point.matrixTransform(matrix.inverse());
  return {
    x: local.x,
    y: -local.y,
  };
};

export const formatSize = (points: Vec2[]): string => {
  if (points.length === 0) return '—';
  const xs = points.map((point) => point.x);
  const ys = points.map((point) => point.y);
  return `${(Math.max(...xs) - Math.min(...xs)).toFixed(1)} × ${(Math.max(...ys) - Math.min(...ys)).toFixed(1)} mm`;
};

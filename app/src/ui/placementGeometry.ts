import type { Part, PartDefinition, Vec2 } from '../../../contracts/src/index';
import { selectionOutline } from './selectionOutline';

export function polygonBounds(points: Vec2[]) {
  if (!points.length) return undefined;
  return { minX: Math.min(...points.map((p) => p.x)), maxX: Math.max(...points.map((p) => p.x)), minY: Math.min(...points.map((p) => p.y)), maxY: Math.max(...points.map((p) => p.y)) };
}

export function alignmentDelta(moving: Vec2[], target: Vec2[], axis: 'x' | 'y', anchor: 'min' | 'center' | 'max'): Vec2 | undefined {
  const a = polygonBounds(moving), b = polygonBounds(target);
  if (!a || !b) return;
  const coordinate = (bounds: typeof a) => axis === 'x'
    ? anchor === 'min' ? bounds.minX : anchor === 'max' ? bounds.maxX : (bounds.minX + bounds.maxX) / 2
    : anchor === 'min' ? bounds.minY : anchor === 'max' ? bounds.maxY : (bounds.minY + bounds.maxY) / 2;
  return { x: axis === 'x' ? coordinate(b) - coordinate(a) : 0, y: axis === 'y' ? coordinate(b) - coordinate(a) : 0 };
}

type Snap = { at: Vec2; from: Vec2; to: Vec2; label: string };
const landmarks = (points: Vec2[], center: Vec2) => [center, ...points, ...points.map((p, i) => ({ x: (p.x + points[(i + 1) % points.length].x) / 2, y: (p.y + points[(i + 1) % points.length].y) / 2 }))];
const axisAligned = (points: Vec2[]) => points.length === 4 && points.every((p, i) => {
  const q = points[(i + 1) % points.length];
  return Math.abs(p.x - q.x) < .0001 || Math.abs(p.y - q.y) < .0001;
});

/** Snaps real envelope landmarks; gap snapping only uses axis-aligned rectangular edges. */
export function snapPart(part: Part, targets: Part[], definitions: Map<string, PartDefinition>, tolerance: number, gap: number | null): Snap | undefined {
  const moving = selectionOutline([part], definitions);
  if (!moving.length || tolerance < 0) return;
  const movingLandmarks = landmarks(moving, part.pose.at);
  const movingBounds = gap !== null && axisAligned(moving) ? polygonBounds(moving) : undefined;
  let best: Snap | undefined;
  let distanceSquared = tolerance * tolerance;
  const consider = (from: Vec2, to: Vec2, label: string) => {
    const dx = to.x - from.x, dy = to.y - from.y;
    const d = dx * dx + dy * dy;
    if (d > distanceSquared) return;
    distanceSquared = d;
    best = { at: { x: part.pose.at.x + to.x - from.x, y: part.pose.at.y + to.y - from.y }, from, to, label };
  };
  for (const target of targets) {
    if (part.id === target.id) continue;
    const polygon = selectionOutline([target], definitions);
    if (!polygon.length) continue;
    const targetLandmarks = landmarks(polygon, target.pose.at);
    const landmarkLabel = `${target.reference} · corner / midpoint / center`;
    for (const from of movingLandmarks) for (const to of targetLandmarks) consider(from, to, landmarkLabel);
    if (gap === null || !movingBounds || !axisAligned(polygon)) continue;
    const a = movingBounds, b = polygonBounds(polygon)!;
    const label = `${target.reference} · envelope gap ${gap.toFixed(2)} mm`;
    if (Math.min(a.maxY, b.maxY) > Math.max(a.minY, b.minY)) {
      const y = (Math.max(a.minY, b.minY) + Math.min(a.maxY, b.maxY)) / 2;
      consider({ x: a.minX, y }, { x: b.maxX + gap, y }, label);
      consider({ x: a.maxX, y }, { x: b.minX - gap, y }, label);
    }
    if (Math.min(a.maxX, b.maxX) > Math.max(a.minX, b.minX)) {
      const x = (Math.max(a.minX, b.minX) + Math.min(a.maxX, b.maxX)) / 2;
      consider({ x, y: a.minY }, { x, y: b.maxY + gap }, label);
      consider({ x, y: a.maxY }, { x, y: b.minY - gap }, label);
    }
  }
  if (best && movingBounds && gap !== null && best.label.includes('envelope gap')) {
    const snapped = polygonBounds(selectionOutline([{ ...part, pose: { ...part.pose, at: best.at } }], definitions))!;
    let alignment: { axis: 'x' | 'y'; delta: number } | undefined;
    for (const target of targets) {
      if (target.id === part.id) continue;
      const polygon = selectionOutline([target], definitions);
      if (!axisAligned(polygon)) continue;
      const b = polygonBounds(polygon)!;
      const horizontal = Math.abs(snapped.minX - b.maxX - gap) < 0.0001 || Math.abs(b.minX - snapped.maxX - gap) < 0.0001;
      const vertical = Math.abs(snapped.minY - b.maxY - gap) < 0.0001 || Math.abs(b.minY - snapped.maxY - gap) < 0.0001;
      const candidates = horizontal ? [b.minY - snapped.minY, b.maxY - snapped.maxY] : vertical ? [b.minX - snapped.minX, b.maxX - snapped.maxX] : [];
      for (const delta of candidates) {
        if (Math.abs(delta) <= tolerance && (!alignment || Math.abs(delta) < Math.abs(alignment.delta))) alignment = { axis: horizontal ? 'y' : 'x', delta };
      }
    }
    if (alignment) {
      best.at[alignment.axis] += alignment.delta;
      best.to[alignment.axis] += alignment.delta;
      best.label += ' · edges aligned';
    }
  }
  return best;
}

/** Snap a transform origin to physical geometry, including rotated edge segments. */
export function snapOrigin(point: Vec2, targets: Part[], definitions: Map<string, PartDefinition>, tolerance: number): Snap | undefined {
  let best: Snap | undefined;
  let distance = tolerance;
  const consider = (to: Vec2, label: string) => {
    const next = Math.hypot(point.x - to.x, point.y - to.y);
    if (next > distance) return;
    distance = next;
    best = { at: to, from: point, to, label };
  };
  for (const part of targets) {
    const polygon = selectionOutline([part], definitions);
    // Landmarks take precedence over a nearby arbitrary point on an edge.
    for (const to of landmarks(polygon, part.pose.at)) consider(to, `${part.reference} · origin / corner / midpoint`);
  }
  if (best) return best;
  for (const part of targets) {
    const polygon = selectionOutline([part], definitions);
    polygon.forEach((a, index) => {
      const b = polygon[(index + 1) % polygon.length];
      const dx = b.x - a.x, dy = b.y - a.y;
      const length = dx * dx + dy * dy;
      if (!length) return;
      const t = Math.max(0, Math.min(1, ((point.x - a.x) * dx + (point.y - a.y) * dy) / length));
      consider({ x: a.x + t * dx, y: a.y + t * dy }, `${part.reference} · edge`);
    });
  }
  return best;
}

import type { Mount, PreparedCaseIR, Vec2 } from '@boardstudio/v2-contracts';

type MountMoveConstraints = { outer?: Vec2[] | Vec2[][]; holes?: Vec2[][] };

/** Use the same offset contours and openings that construct the case solid. */
export function caseMountConstraints({ body, regions }: PreparedCaseIR): MountMoveConstraints {
  const base = body.z ?? 0;
  const top = base + body.thickness + (body.kind === 'plate' ? 0 : body.wallHeight ?? 0);
  const openings = (body.openings ?? []).filter(opening => opening.z < top && opening.z + opening.height > base);
  return {
    outer: regions.map(region => region.outer),
    holes: [...regions.flatMap(region => region.holes), ...openings.map(opening => opening.points)],
  };
}

function inside(point: Vec2, polygon: Vec2[]): boolean {
  let hit = false;
  for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i++) {
    const a = polygon[i], b = polygon[j];
    if ((a.y > point.y) !== (b.y > point.y) && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x) hit = !hit;
  }
  return hit;
}

function edgeDistance(point: Vec2, polygon: Vec2[]): number {
  let nearest = Infinity;
  for (let index = 0; index < polygon.length; index += 1) {
    const a = polygon[index], b = polygon[(index + 1) % polygon.length];
    const dx = b.x - a.x, dy = b.y - a.y;
    const length = dx * dx + dy * dy;
    const t = length ? Math.max(0, Math.min(1, ((point.x - a.x) * dx + (point.y - a.y) * dy) / length)) : 0;
    nearest = Math.min(nearest, Math.hypot(point.x - a.x - t * dx, point.y - a.y - t * dy));
  }
  return nearest;
}

const mountRadius = (mount: Mount) => (mount.kind === 'boss' ? mount.bossDiameter ?? mount.holeDiameter : mount.holeDiameter) / 2;

/** Move one authored case mount while keeping PCB geometry independent. */
export function moveCaseMount(
  point: Vec2,
  id: string,
  mounts: Mount[],
  clearance = 0.5,
  constraints: MountMoveConstraints = {},
): Mount[] | undefined {
  const original = mounts.find((mount) => mount.id === id);
  if (!original || !Number.isFinite(point.x) || !Number.isFinite(point.y)) return undefined;
  const radius = mountRadius(original);
  if (!Number.isFinite(radius) || radius <= 0 || !Number.isFinite(clearance) || clearance < 0) return undefined;
  const margin = radius + clearance;
  if (constraints.outer) {
    const outers = Array.isArray(constraints.outer[0]) ? constraints.outer as Vec2[][] : [constraints.outer as Vec2[]];
    if (!outers.some((outer) => inside(point, outer) && edgeDistance(point, outer) >= margin)) return undefined;
  }
  if (constraints.holes?.some(hole => inside(point, hole) || edgeDistance(point, hole) < margin)) return undefined;
  if (mounts.some(other => {
    if (other.id === id) return false;
    const otherRadius = mountRadius(other);
    return !Number.isFinite(otherRadius) || otherRadius <= 0
      || Math.hypot(point.x - other.at.x, point.y - other.at.y) < radius + otherRadius + clearance;
  })) return undefined;
  return mounts.map(mount => mount.id === id ? { ...mount, at: { x: point.x, y: point.y } } : mount);
}

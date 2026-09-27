import type { Vec2 } from '../../../contracts/src/index';

export type KeycapPlacement = {
  id: string;
  reference: string;
  matrixId: string;
  row: number;
  column: number;
  at: Vec2;
  rotation: number;
  size: Vec2;
};

export type KeycapResize = KeycapPlacement & {
  nextSize: Vec2;
  axisX: Vec2;
  axisY: Vec2;
};

type KeycapOverlap = { first: KeycapPlacement; second: KeycapPlacement };

const addDelta = (deltas: Map<string, Vec2>, id: string, delta: Vec2) => {
  const current = deltas.get(id) ?? { x: 0, y: 0 };
  deltas.set(id, { x: current.x + delta.x, y: current.y + delta.y });
};

/** Moves the keys on the same row or column far enough to keep the selected caps apart. */
export function keycapReflowDeltas(resizes: KeycapResize[], placements: readonly KeycapPlacement[]): Map<string, Vec2> {
  const deltas = new Map<string, Vec2>();
  const resizeById = new Map(resizes.map((resize) => [resize.id, resize]));
  const halfExtent = (placement: KeycapPlacement, axis: Vec2, size = placement.size) => {
    const angle = placement.rotation * Math.PI / 180;
    const keyX = { x: Math.cos(angle), y: Math.sin(angle) };
    const keyY = { x: -Math.sin(angle), y: Math.cos(angle) };
    return (Math.abs(keyX.x * axis.x + keyX.y * axis.y) * size.x
      + Math.abs(keyY.x * axis.x + keyY.y * axis.y) * size.y) / 2;
  };
  const scalar = (point: Vec2, axis: Vec2) => point.x * axis.x + point.y * axis.y;
  const reflowGroups = (direction: 'row' | 'column', movementAxis: 'axisX' | 'axisY') => {
    const groupedResizes = new Map<string, KeycapResize[]>();
    for (const resize of resizes) {
      const key = JSON.stringify([resize.matrixId, direction === 'row' ? resize.row : resize.column]);
      const group = groupedResizes.get(key) ?? [];
      group.push(resize);
      groupedResizes.set(key, group);
    }

    for (const [groupKey, groupResizes] of groupedResizes) {
      const [matrixId, index] = JSON.parse(groupKey) as [string, number];
      const members = placements.filter((placement) => placement.matrixId === matrixId
        && (direction === 'row' ? placement.row === index : placement.column === index))
        .sort((first, second) => direction === 'row' ? first.column - second.column : first.row - second.row);
      if (members.length < 2) continue;
      const movement = groupResizes[0][movementAxis];
      const oldScalars = members.map((member) => scalar(member.at, movement));
      const nextScalars = [oldScalars[0]];

      for (let memberIndex = 1; memberIndex < members.length; memberIndex += 1) {
        const previous = members[memberIndex - 1];
        const current = members[memberIndex];
        const previousResize = resizeById.get(previous.id);
        const currentResize = resizeById.get(current.id);
        const previousSize = previousResize?.nextSize ?? previous.size;
        const currentSize = currentResize?.nextSize ?? current.size;
        const oldDistance = Math.max(0, oldScalars[memberIndex] - oldScalars[memberIndex - 1]);
        const oldGap = Math.max(0, oldDistance - halfExtent(previous, movement) - halfExtent(current, movement));
        const newDistance = halfExtent(previous, movement, previousSize)
          + halfExtent(current, movement, currentSize)
          + oldGap;
        nextScalars.push(nextScalars[memberIndex - 1] + newDistance);
      }

      const selectedMembers = members.flatMap((member, memberIndex) => {
        const resize = resizeById.get(member.id);
        return resize ? [{ member, resize, memberIndex }] : [];
      });
      if (selectedMembers.length === 0) continue;
      const oldMin = Math.min(...selectedMembers.map(({ member }) => scalar(member.at, movement) - halfExtent(member, movement)));
      const oldMax = Math.max(...selectedMembers.map(({ member }) => scalar(member.at, movement) + halfExtent(member, movement)));
      const nextMin = Math.min(...selectedMembers.map(({ member, resize, memberIndex }) => nextScalars[memberIndex] - halfExtent(member, movement, resize.nextSize)));
      const nextMax = Math.max(...selectedMembers.map(({ member, resize, memberIndex }) => nextScalars[memberIndex] + halfExtent(member, movement, resize.nextSize)));
      const centerCorrection = (oldMin + oldMax - nextMin - nextMax) / 2;

      members.forEach((member, memberIndex) => {
        const delta = (nextScalars[memberIndex] - oldScalars[memberIndex] + centerCorrection);
        if (delta !== 0) addDelta(deltas, member.id, { x: movement.x * delta, y: movement.y * delta });
      });
    }
  };

  reflowGroups('row', 'axisX');
  reflowGroups('column', 'axisY');

  const selectedBefore = resizes.flatMap((resize) => corners(resize));
  const selectedAfter = resizes.flatMap((resize) => corners({
    ...resize,
    size: resize.nextSize,
    at: {
      x: resize.at.x + (deltas.get(resize.id)?.x ?? 0),
      y: resize.at.y + (deltas.get(resize.id)?.y ?? 0),
    },
  }));
  if (selectedBefore.length && selectedAfter.length) {
    const center = (points: Vec2[]) => ({
      x: (Math.min(...points.map((point) => point.x)) + Math.max(...points.map((point) => point.x))) / 2,
      y: (Math.min(...points.map((point) => point.y)) + Math.max(...points.map((point) => point.y))) / 2,
    });
    const before = center(selectedBefore);
    const after = center(selectedAfter);
    const correction = { x: before.x - after.x, y: before.y - after.y };
    if (correction.x !== 0 || correction.y !== 0) {
      for (const resize of resizes) addDelta(deltas, resize.id, correction);
      for (const [id, delta] of deltas) {
        if (!resizeById.has(id)) deltas.set(id, { x: delta.x + correction.x, y: delta.y + correction.y });
      }
    }
  }

  return deltas;
}

const corners = ({ at, rotation, size }: KeycapPlacement): Vec2[] => {
  const angle = rotation * Math.PI / 180;
  const cos = Math.cos(angle);
  const sin = Math.sin(angle);
  return [
    { x: -size.x / 2, y: -size.y / 2 },
    { x: size.x / 2, y: -size.y / 2 },
    { x: size.x / 2, y: size.y / 2 },
    { x: -size.x / 2, y: size.y / 2 },
  ].map((point) => ({
    x: at.x + point.x * cos - point.y * sin,
    y: at.y + point.x * sin + point.y * cos,
  }));
};

const projection = (points: Vec2[], axis: Vec2): [number, number] => {
  const values = points.map((point) => point.x * axis.x + point.y * axis.y);
  return [Math.min(...values), Math.max(...values)];
};

const overlapsOnAxes = (first: Vec2[], second: Vec2[], source: Vec2[]): boolean => {
  for (let index = 0; index < source.length; index += 1) {
    const start = source[index];
    const end = source[(index + 1) % source.length];
    const length = Math.hypot(end.x - start.x, end.y - start.y);
    if (length === 0) continue;
    const axis = { x: -(end.y - start.y) / length, y: (end.x - start.x) / length };
    const [firstMin, firstMax] = projection(first, axis);
    const [secondMin, secondMax] = projection(second, axis);
    if (firstMax <= secondMin + 1e-7 || secondMax <= firstMin + 1e-7) return false;
  }
  return true;
};

/** Returns overlapping keycap pairs, ignoring boxes that only touch at their edges. */
export function findKeycapOverlaps(placements: KeycapPlacement[]): KeycapOverlap[] {
  const bounds = placements.map((placement) => {
    const polygon = corners(placement);
    return {
      placement,
      polygon,
      minX: Math.min(...polygon.map((point) => point.x)),
      maxX: Math.max(...polygon.map((point) => point.x)),
      minY: Math.min(...polygon.map((point) => point.y)),
      maxY: Math.max(...polygon.map((point) => point.y)),
    };
  }).sort((first, second) => first.minX - second.minX);
  const overlaps: KeycapOverlap[] = [];

  for (let index = 0; index < bounds.length; index += 1) {
    const first = bounds[index];
    for (let next = index + 1; next < bounds.length && bounds[next].minX < first.maxX - 1e-7; next += 1) {
      const second = bounds[next];
      if (first.maxY <= second.minY + 1e-7 || second.maxY <= first.minY + 1e-7) continue;
      if (overlapsOnAxes(first.polygon, second.polygon, first.polygon)
        && overlapsOnAxes(first.polygon, second.polygon, second.polygon)) {
        overlaps.push({ first: first.placement, second: second.placement });
      }
    }
  }

  return overlaps;
}

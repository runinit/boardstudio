import type { MechanicalGasketAnchor, MechanicalGasketLayout, MechanicalGasketSupport, MechanicalGasketTrack, Vec2 } from '@boardstudio/v2-contracts';

export function defaultGasketLayout(): MechanicalGasketLayout {
  return { length: 12, width: 3, thickness: 2, compression: 0.15, supports: [] };
}

function projectGasket(point: Vec2, support: MechanicalGasketSupport, tracks: MechanicalGasketTrack[]): MechanicalGasketSupport | undefined {
  let best: MechanicalGasketSupport | undefined;
  let distance = Infinity;
  for (const track of tracks.filter(track => track.regionId === support.regionId)) {
    const dx = track.end.x - track.start.x, dy = track.end.y - track.start.y;
    const length = Math.hypot(dx, dy);
    if (!length) continue;
    const t = Math.max(0, Math.min(1, ((point.x - track.start.x) * dx + (point.y - track.start.y) * dy) / (length * length)));
    const at = { x: track.start.x + t * dx, y: track.start.y + t * dy };
    const separation = Math.hypot(point.x - at.x, point.y - at.y);
    if (separation >= distance) continue;
    distance = separation;
    best = { ...support, at, anchor: track.startAnchor + t * (track.endAnchor - track.startAnchor), tangent: { x: dx / length, y: dy / length }, normal: { x: dy / length, y: -dx / length } };
  }
  return distance <= 8 ? best : undefined;
}

export function moveGasket(point: Vec2, id: string, supports: MechanicalGasketSupport[], tracks: MechanicalGasketTrack[]): MechanicalGasketSupport[] | undefined {
  const original = supports.find(support => support.id === id);
  if (!original) return undefined;
  const moved = projectGasket(point, original, tracks);
  if (!moved) return undefined;
  const replacements = new Map([[id, moved]]);
  if (original.pairId && original.mirrorAxis != null && !original.unlinked) {
    const pair = supports.find(support => support.id === original.pairId);
    if (pair && !pair.unlinked) {
      const reflected = projectGasket({ x: 2 * original.mirrorAxis - moved.at.x, y: moved.at.y }, pair, tracks);
      if (!reflected || Math.hypot(reflected.at.x - (2 * original.mirrorAxis - moved.at.x), reflected.at.y - moved.at.y) > 0.001) return undefined;
      replacements.set(pair.id, reflected);
    }
  }
  const next = supports.map(support => replacements.get(support.id) ?? support);
  // Closure lobes need more room than the foam strip itself.
  if (next.some((support, index) => next.slice(0, index).some(other =>
    Math.hypot(support.at.x - other.at.x, support.at.y - other.at.y) < (support.length + other.length) / 2 + 3))) return undefined;
  return next;
}

export function gasketAnchors(layout: MechanicalGasketLayout, before: MechanicalGasketSupport[], after: MechanicalGasketSupport[]): MechanicalGasketAnchor[] {
  const updates = after.filter(support => {
    const previous = before.find(item => item.id === support.id);
    return !previous || Math.abs(previous.anchor - support.anchor) > 1e-7 || previous.unlinked !== support.unlinked;
  }).map(({ id, regionId, outlineKey, anchor, unlinked }) => ({ id, regionId, outlineKey, anchor, unlinked }));
  return [...layout.supports.filter(old => !updates.some(update => update.id === old.id)), ...updates];
}

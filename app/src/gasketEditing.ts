import type { InternalGasketConfiguration, GasketFoamPreset, MechanicalGasketAnchor, MechanicalGasketLayout, MechanicalGasketSupport, MechanicalGasketTrack, Vec2 } from '@boardstudio/v2-contracts';

export function defaultGasketLayout(): MechanicalGasketLayout {
  return { autoSize: true, length: 80, width: 3, thickness: 2, compression: 0.15, supports: [] };
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
  return best;
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
  return next;
}

export function gasketAnchors(layout: MechanicalGasketLayout, before: MechanicalGasketSupport[], after: MechanicalGasketSupport[]): MechanicalGasketAnchor[] {
  const updates = after.filter(support => {
    const previous = before.find(item => item.id === support.id);
    return !previous || Math.abs(previous.anchor - support.anchor) > 1e-7 || previous.unlinked !== support.unlinked;
  }).map(({ id, regionId, outlineKey, anchor, unlinked, length, width }) => ({ ...layout.supports.find(saved => saved.id === id), id, regionId, outlineKey, anchor, unlinked, length, width, placement: 'user' as const }));
  return [...layout.supports.filter(old => !updates.some(update => update.id === old.id)), ...updates];
}

/** Starting Custom dimensions, not a supplier-qualified hardware preset. */
export function defaultInternalGasket(): InternalGasketConfiguration {
  return {
    version: 'internal-v1', minimumWall: 2, tolerance: 0.05, supportCount: 4,
    hardware: {
      id: 'custom-m2', thread: 'M2 × 0.4', threadDiameter: 2, pitch: 0.4,
      drive: 'hex', installation: 'heat-set', lengthDatum: 'under-head', headProfile: 'flat',
      screwLengths: [8, 10, 12, 14, 15, 16], headDiameter: 4, headHeight: 1,
      holeDiameter: 2.2, insertDiameter: 3.6, insertLength: 4, seatDiameter: 3.2,
      seatDepth: 4.5, engagement: 3, threadStart: 0.2, tipAllowance: 0.1,
      bottomingClearance: 0.5, roof: 1.5, surround: 1.5, seatLeadDepth: 0.25,
      seatLeadDiameter: 3.4, bearingThickness: 1.5,
    },
  };
}

export const gasketFoamPresets: { id: GasketFoamPreset; length: number; width: number; thickness: number }[] = [
  { id: 'A2', length: 20, width: 3, thickness: 2 },
  { id: 'A3', length: 20, width: 3, thickness: 3 },
  { id: 'A4', length: 20, width: 3, thickness: 4 },
  { id: 'B2', length: 20, width: 4, thickness: 2 },
  { id: 'B3', length: 20, width: 4, thickness: 3 },
  { id: 'B4', length: 20, width: 4, thickness: 4 },
  { id: 'E2', length: 80, width: 3, thickness: 2 },
  { id: 'E3', length: 80, width: 3, thickness: 3 },
  { id: 'E4', length: 80, width: 3, thickness: 4 },
  { id: 'F2', length: 80, width: 4, thickness: 2 },
  { id: 'F3', length: 80, width: 4, thickness: 3 },
  { id: 'F4', length: 80, width: 4, thickness: 4 },
  { id: 'F5', length: 80, width: 4, thickness: 5 },
];

import { expect, test } from 'vitest';
import type { MechanicalGasketSupport, MechanicalGasketTrack } from '@boardstudio/v2-contracts';
import { defaultGasketLayout, gasketAnchors, moveGasket } from './gasketEditing';

const left: MechanicalGasketSupport = { id: 'left:0', regionId: 'left', outlineKey: 'square', anchor: 0.1, at: { x: 20, y: 0 }, tangent: { x: 1, y: 0 }, normal: { x: 0, y: -1 }, length: 12, width: 3, z: 1.8, thickness: 1.7, pairId: 'right:0', mirrorAxis: 100, unlinked: false };
const right: MechanicalGasketSupport = { ...left, id: 'right:0', regionId: 'right', at: { x: 180, y: 0 }, pairId: 'left:0' };
const tracks: MechanicalGasketTrack[] = [
  { regionId: 'left', start: { x: 10, y: 0 }, end: { x: 80, y: 0 }, startAnchor: 0, endAnchor: 0.35 },
  { regionId: 'right', start: { x: 120, y: 0 }, end: { x: 190, y: 0 }, startAnchor: 0, endAnchor: 0.35 },
];

test('projects a move onto the perimeter and mirrors its linked partner', () => {
  const before = [left, right];
  const after = moveGasket({ x: 35.25, y: 1 }, left.id, before, tracks)!;
  expect(after[0].at).toEqual({ x: 35.25, y: 0 });
  expect(after[1].at).toEqual({ x: 164.75, y: 0 });
  const anchors = gasketAnchors(defaultGasketLayout(), before, after);
  expect(anchors.map(anchor => anchor.id)).toEqual(['left:0', 'right:0']);
  expect(before[0].at.x).toBe(20);
});

test('an unlinked support moves independently and blocked moves are rejected', () => {
  const before = [{ ...left, unlinked: true }, right];
  expect(moveGasket({ x: 35, y: 0 }, left.id, before, tracks)![1]).toBe(right);
  expect(moveGasket({ x: 35, y: 20 }, left.id, before, tracks)).toBeUndefined();
  expect(moveGasket({ x: 180, y: 0 }, left.id, before, tracks)).toBeUndefined();
});

test('a linked move requires an exact reflected position on the other track', () => {
  const shifted = tracks.map(track => track.regionId === 'right' ? { ...track, start: { ...track.start, y: 4 }, end: { ...track.end, y: 4 } } : track);
  expect(moveGasket({ x: 35, y: 0 }, left.id, [left, right], shifted)).toBeUndefined();
});

test('a committed linked move pins both supports without pinning untouched supports', () => {
  const before = [{ ...left, placement: 'generated' as const }, { ...right, placement: 'generated' as const }];
  const after = moveGasket({ x: 35.25, y: 1 }, left.id, before, tracks)!;
  const anchors = gasketAnchors(defaultGasketLayout(), before, after);
  expect(anchors.map(anchor => anchor.placement)).toEqual(['user', 'user']);
  expect(gasketAnchors(defaultGasketLayout(), before, before)).toEqual([]);
});

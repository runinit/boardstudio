import { expect, test } from 'vitest';
import type { Part, PartDefinition } from '../../../contracts/src/index';
import { alignmentDelta, snapPart } from './placementGeometry';
const definition: PartDefinition = { id: 'switch', name: 'Switch', kind: 'switch', pads: [], keycap: { x: 18, y: 18 }, courtyard: [] };
const definitions = new Map([[definition.id, definition]]);
const part = (id: string, x: number, rotation = 0): Part => ({ id, reference: id, definitionId: 'switch', pose: { at: { x, y: 0 }, rotation }, side: 'front' });
test('19 mm pitch with an 18 mm key envelope snaps to a 1 mm gap', () => {
  const snap = snapPart(part('moving', 19.6), [part('fixed', 0)], definitions, 1, 1);
  expect(snap?.at.x).toBe(19);
  expect(snap?.label).toContain('gap 1.00 mm');
});
test('does not interpret rotated bounding boxes as physical edges for gap snapping', () => {
  const snap = snapPart(part('moving', 23.3279, 30), [part('fixed', 0)], definitions, .5, 1);
  expect(snap?.label ?? '').not.toContain('gap');
});
test('aligns unequal-sized bounds to a fixed reference', () => {
  expect(alignmentDelta([{ x: 10, y: 3 }, { x: 20, y: 8 }], [{ x: -4, y: -2 }, { x: 4, y: 2 }], 'x', 'center')).toEqual({ x: -15, y: 0 });
});

test('origin snapping finds off-grid origins, corners and rotated physical edges', async () => {
  const { snapOrigin } = await import('./placementGeometry');
  const fixed = part('target', 2.3, 30);
  expect(snapOrigin({ x: 2.4, y: .1 }, [fixed], definitions, .5)?.at).toEqual(fixed.pose.at);
  const corner = { x: 2.3 + 9 * Math.cos(Math.PI / 6) - 9 * Math.sin(Math.PI / 6), y: 9 * Math.sin(Math.PI / 6) + 9 * Math.cos(Math.PI / 6) };
  const snap = snapOrigin({ x: corner.x + .1, y: corner.y }, [fixed], definitions, .5);
  expect(snap?.at.x).toBeCloseTo(corner.x);
  expect(snap?.at.y).toBeCloseTo(corner.y);
  const edge = snapOrigin({ x: 3, y: 9.1 }, [part('target', 0)], definitions, .3);
  expect(edge?.at).toEqual({ x: 3, y: 9 });
  expect(edge?.label).toContain('edge');
});

test('a controller snaps to the key gap and aligns its top edge in the same move', () => {
  const controller: PartDefinition = { id: 'controller', name: 'Controller', kind: 'controller', pads: [], courtyard: [{x:-9,y:-16},{x:9,y:-16},{x:9,y:16},{x:-9,y:16}] };
  const moving = { ...part('moving', 19.6), definitionId: controller.id, pose: { at: { x: 19.6, y: -6.4 }, rotation: 0 } };
  const snap = snapPart(moving, [part('fixed', 0)], new Map([...definitions, [controller.id, controller]]), 1, 1);
  expect(snap?.at).toEqual({ x: 19, y: -7 });
});

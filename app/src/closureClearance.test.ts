import { expect, it } from 'vitest';
import { demoProject } from './demo';
import { createMechanicalConfiguration } from './mechanicalPresets';
import { withClosureClearance } from './closureClearance';
it('adds real NPTH boss clearance and removes only owned holes', () => {
  const doc = demoProject();
  const config = { ...createMechanicalConfiguration(doc), closureMounts: [{ id: 'one', at: { x: 30, y: 20 }, kind: 'boss' as const, holeDiameter: 2.2, bossDiameter: 6 }] };
  const next = withClosureClearance({ ...doc, mechanical: { ...config, closureMounts: config.closureMounts.map(mount => ({ ...mount, at: { x: -mount.at.x, y: mount.at.y } })) } });
  const part = next.parts.at(-1)!;
  expect(part.pose.at).toEqual({ x: -30, y: 20 });
  expect(next.definitions.at(-1)?.pads[0].drill).toBeCloseTo(6.6);
  expect(next.definitions.at(-1)?.pads[0].plated).toBe(false);
  expect(next.boards[0].partIds).toContain(part.id);
  expect(withClosureClearance({ ...next, mechanical: undefined }).parts).toEqual(doc.parts);
});

it('deduplicates shared-PCB holes after reflecting the right physical half', () => {
  const doc = demoProject();
  const config = createMechanicalConfiguration(doc);
  doc.hardware = { topology: 'split', transport: 'wireless', boards: [], sharedConstruction: null,
    instances: [false, true].map(flipped => ({ id: flipped ? 'right' : 'left', name: 'Half', boardId: doc.boards[0].id,
      half: flipped ? 'right' : 'left', role: flipped ? 'peripheral' : 'central', flipped, constructionLinked: true, controllerPartId: null,
      mechanical: { ...config, closureMounts: [{ id: 'screw', at: { x: flipped ? -30 : 30, y: 20 }, kind: 'boss', holeDiameter: 2.2, bossDiameter: flipped ? 7 : 6 }] } })) };
  const next = withClosureClearance(doc);
  const holes = next.parts.filter(part => part.id.startsWith('case-closure/'));
  expect(holes).toHaveLength(1);
  expect(holes[0].pose.at).toEqual({ x: 30, y: 20 });
  expect(next.definitions.find(definition => definition.id === holes[0].definitionId)?.pads[0].drill).toBeCloseTo(7.6);
});

import { expect, test } from 'vitest';
import type { ElectricalPlan } from '@boardstudio/v2-contracts';
import { demoProject } from './demo';
import { canPublishElectricalPlan, isCurrentElectricalPlan, isWiringApplied } from './electricalPlanContext';

const planFor = (document: ReturnType<typeof demoProject>): ElectricalPlan => ({
  instanceId: null, jumpers: [], moduleAliases: {}, mode: 'matrix', assignments: [], rowPins: [], columnPins: [],
  diagnostics: [], fingerprint: 'test', boardId: document.boards[0].id, controllerPartId: null, revision: document.revision,
  controllerProfile: null, freePins: [], nets: [], diodeDirection: 'cathode-to-row', peripherals: [], peripheralPins: {}, peripheralTerminals: {},
});

test('a plan belongs to the captured document object, not only id/revision/board', () => {
  const first = demoProject();
  const reloaded = structuredClone(first);
  const context = { document: first, plan: planFor(first) };
  expect(isCurrentElectricalPlan(context, first, first.boards[0].id)).toBe(true);
  expect(isCurrentElectricalPlan(context, reloaded, reloaded.boards[0].id)).toBe(false);
});

test('late resolutions publish only to the captured document and board', () => {
  const document = demoProject();
  const otherDocument = structuredClone(document);
  expect(canPublishElectricalPlan(document, 'main', document, 'main')).toBe(true);
  expect(canPublishElectricalPlan(document, 'main', document, 'aux')).toBe(false);
  expect(canPublishElectricalPlan(document, 'main', otherDocument, 'main')).toBe(false);
});

test('wiring is applied only for matching revision, configuration, nets, and board membership', () => {
  const document = demoProject();
  const boardId = document.boards[0].id;
  const generated = [{ id: `generated/electrical/${boardId}/ROW0`, name: 'ROW0', pins: [{ partId: 'key-1', padId: '1' }] }];
  document.boards[0].netIds = generated.map(net => net.id);
  document.nets = generated;
  const plan = planFor(document);
  plan.nets = generated;
  plan.mode = 'matrix';
  document.hardware = { topology: 'unibody', transport: 'none', boards: [{ boardId, controllerPartId: null, mode: 'matrix', locks: {}, assignments: {}, keyBindings: {}, jumperStates: {}, protectedHandoff: null }], instances: [], sharedConstruction: null };
  expect(isWiringApplied(document, plan)).toBe(true);
  expect(isWiringApplied({ ...document, nets: [...document.nets, { id: `generated/electrical/${boardId}/ROW1`, name: 'ROW1', pins: [] }] }, plan)).toBe(false);
  expect(isWiringApplied({ ...document, boards: [{ ...document.boards[0], netIds: [] }, ...document.boards.slice(1)] }, plan)).toBe(false);
  expect(isWiringApplied({ ...document, hardware: { ...document.hardware!, boards: [{ ...document.hardware!.boards[0], mode: 'direct' }] } }, plan)).toBe(false);
  expect(isWiringApplied({ ...document, revision: document.revision + 1 }, plan)).toBe(false);
  expect(isWiringApplied({ ...document, hardware: undefined }, plan)).toBe(false);
});

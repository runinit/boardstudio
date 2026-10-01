import type { Page } from '@playwright/test';
import { catalogue, normalizeDefinition } from '@boardstudio/v2-ergogen';
import { demoProject } from '../src/demo';
import { navigateWorkspace } from './workspace-navigation';
import { openWorkspaceDocument } from './workspace-storage';
export async function openKeymapFixture(page: Page, withEncoder = false) {
  const doc = demoProject(); doc.id = 'keycaps-browser'; doc.name = 'Keycap keyboard';
  const source = catalogue().find(definition => definition.generator?.source === 'ceoloide/mcu_nice_nano')!;
  const mcu = normalizeDefinition({ ...source, generator: { ...source.generator!, parameters: { reversible: true } } });
  doc.definitions.push(mcu);
  doc.parts.push({ id: 'controller', reference: 'U1', definitionId: mcu.id, pose: { at: { x: 110, y: 19 }, rotation: 0 }, side: 'front' });
  doc.boards[0].partIds.push('controller'); doc.nets = []; doc.boards[0].netIds = [];
  doc.hardware = { topology: 'unibody', transport: 'none', instances: [], sharedConstruction: null, boards: [{ boardId: 'main-board', controllerPartId: 'controller', mode: 'direct', locks: {}, assignments: {}, keyBindings: {}, jumperStates: {}, protectedHandoff: null }] };
  if (withEncoder) {
    const source = catalogue().find(definition => definition.generator?.source === 'ceoloide/rotary_encoder_ec11_ec12')!;
    const encoder = normalizeDefinition(source); doc.definitions.push(encoder);
    doc.parts.push({ id: 'encoder', reference: 'ENC1', definitionId: encoder.id, pose: { at: { x: 110, y: -20 }, rotation: 0 }, side: 'front' });
    doc.boards[0].partIds.push('encoder');
  }
  await openWorkspaceDocument(page, doc);
  await navigateWorkspace(page, 'Keycaps');
}

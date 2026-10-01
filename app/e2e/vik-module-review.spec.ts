import { expect, test } from '@playwright/test';
import { navigateWorkspace } from './workspace-navigation';
import { readWorkspaceDocument } from './workspace-storage';

test('the app gallery opens a saved above/below review project with visible model bounds', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'Start VIK module review · above and below', exact: true }).click();
  await expect(page.locator('.wb-project-name')).toHaveText('VIK module review · above and below');

  let saved = await readWorkspaceDocument(page);
  expect(saved.modules).toHaveLength(3);
  expect(saved.modules?.map(instance => instance.hostFace)).toEqual(['front', 'back', 'front']);
  expect(saved.keymap?.layers.map(layer => layer.name)).toEqual(['Base', 'Navigation']);
  expect(saved.keymap?.layers[0].bindings['matrix/matrix/r0c0']).toMatchObject({kind:'mod-tap'});
  expect(saved.keymap?.layers[0].bindings['matrix/matrix/r0c1']).toMatchObject({kind:'layer-tap',layerId:'navigation'});
  expect(saved.keymap?.layers[1].bindings['matrix/matrix/r0c1']).toMatchObject({kind:'macro',macroId:'review_navigation_pulse'});
  expect(saved.keymap?.macros.find(macro=>macro.id==='review_navigation_pulse')?.steps.some(step=>step.kind==='wait'&&step.ms===40)).toBe(true);
  expect(saved.keymap?.layers.every(layer => layer.sensors?.['review/ec11-rotary'])).toBe(true);
  expect(saved.parts.some(part => part.definitionId === 'thqwgd001:c-2pin-reversible')).toBe(true);
  expect(saved.parts.some(part => part.definitionId === 'thqwgd001:c-4pin-reversible')).toBe(true);
  expect(saved.embeddedCircuits).toHaveLength(1);
  const circuit = saved.embeddedCircuits![0];
  expect(circuit.partIds.length).toBeGreaterThan(10);
  expect(circuit.netIds.every(id => id.startsWith(`embedded/${circuit.id}/`))).toBe(true);
  const matrixNetIds = saved.nets.filter(net => /^row-|^col-/u.test(net.id)).map(net => net.id);
  expect(circuit.netIds.some(id => matrixNetIds.includes(id))).toBe(false);
  const envelope = saved.outline.find(feature => feature.id === 'board-envelope');
  expect(envelope?.kind).toBe('part-envelope');
  expect(envelope?.kind === 'part-envelope' && circuit.partIds.every(id => envelope.partIds.includes(id))).toBe(true);

  await navigateWorkspace(page, 'Parts');
  await page.getByRole('listbox', { name: 'VIK modules', exact: true }).getByRole('option', { name: /split one VIK|module to split one VIK/iu }).click();
  await expect(page.getByRole('note', { name: 'Review fixture assumptions', exact: true })).toContainText('illustrative');
  await page.locator('details').filter({ hasText: 'Available 3D models' }).locator('summary').click();
  await expect(page.getByText('10.300 × 5.850 × 2.110 mm', { exact: false })).toBeVisible();
  await expect(page.getByText('Alignment unreviewed', { exact: false }).first()).toBeVisible();

  await page.getByRole('listbox', { name: 'VIK modules', exact: true }).getByRole('option', { name: /EC11 and EVQWDG001 with RGB leds/u }).click();
  await page.locator('details.wb-inspector-section').filter({ hasText: 'Assembly geometry' }).first().locator(':scope > summary').click();
  await page.getByText('Rotary encoder profile', { exact: true }).click();
  await expect(page.getByText('Pulses and actions per rotation are not supplied', { exact: false })).toBeVisible();
  await expect(page.getByLabel('Rotary pulses per rotation', { exact: true })).toHaveValue('');
  await page.getByRole('button', { name: 'Save rotary profile', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('positive whole-number pulses and actions');
  // Explicit test inputs exercise editing and serialization; they are not vendor defaults in the demo.
  await page.getByLabel('Rotary pulses per rotation', { exact: true }).fill('2');
  await page.getByLabel('Rotary pulses per rotation', { exact: true }).press('Tab');
  await page.getByLabel('Rotary actions per rotation', { exact: true }).fill('4');
  await page.getByLabel('Rotary actions per rotation', { exact: true }).press('Tab');
  await page.getByRole('button', { name: 'Save rotary profile', exact: true }).click();
  await expect.poll(async () => (await readWorkspaceDocument(page)).moduleDefinitions?.find(definition => definition.catalogueRow === 'ec11-evqwgd001')?.electrical.rotaryProfile).toMatchObject({ steps: 2, triggersPerRotation: 4, driver: 'ec11' });

  await page.getByRole('listbox', { name: 'VIK modules', exact: true }).getByRole('option', { name: /module to split one VIK/iu }).click();
  await page.getByLabel('Module placement', { exact: true }).selectOption('review/splitter-above');
  await page.getByLabel('Module attachment', { exact: true }).selectOption('case');
  await page.getByText('Case support rings', { exact: false }).click();
  await expect(page.getByLabel('Support source mounting hole', { exact: true })).toContainText('source drill 2.2 mm');
  await expect(page.getByLabel('Support outerDiameter', { exact: true })).toHaveValue('');
  await expect(page.getByLabel('Support holeDiameter', { exact: true })).toHaveValue('');
  await page.getByLabel('Support source mounting hole', { exact: true }).selectOption('PART0');
  await page.getByRole('button', { name: 'Add specified ring', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('enter finite dimensions');

  await page.getByLabel('Module placement', { exact: true }).selectOption('review/splitter-below');
  await expect(page.getByLabel('Module host face', { exact: true })).toHaveValue('back');
  await expect(page.getByLabel('Module facing surface', { exact: true })).toHaveValue('front');
  await page.getByLabel('Module yaw', { exact: true }).fill('190');
  await page.getByLabel('Module yaw', { exact: true }).press('Tab');
  await page.getByRole('button', { name: 'Save placement', exact: true }).click();
  await expect.poll(async () => (await readWorkspaceDocument(page)).modules?.find(instance => instance.id === 'review/splitter-below')?.rotation).toBe(190);
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect.poll(async () => (await readWorkspaceDocument(page)).modules?.find(instance => instance.id === 'review/splitter-below')?.rotation).toBe(180);
  await page.getByRole('button', { name: 'Redo', exact: true }).click();
  await expect.poll(async () => (await readWorkspaceDocument(page)).modules?.find(instance => instance.id === 'review/splitter-below')?.rotation).toBe(190);

  await page.reload();
  saved = await readWorkspaceDocument(page);
  expect(saved.modules?.find(instance => instance.id === 'review/splitter-below')?.rotation).toBe(190);
  expect(saved.embeddedCircuits?.[0].id).toBe(circuit.id);

  // The project archive must preserve the source snapshots, editable assembly, keymap and
  // embedded circuit identities across an actual download/import round trip.
  const downloading = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'Save project copy…', exact: true }).click();
  const archivePath = await (await downloading).path();
  expect(archivePath).toBeTruthy();
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'New project', exact: true }).click();
  const selectingArchive = page.waitForEvent('filechooser');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'Open project…', exact: true }).click();
  await (await selectingArchive).setFiles(archivePath!);
  await expect(page.locator('.wb-project-name')).toHaveText('VIK module review · above and below');
  const restored = await readWorkspaceDocument(page);
  expect(restored.modules?.map(instance => instance.id)).toEqual(saved.modules?.map(instance => instance.id));
  expect(restored.moduleDefinitions?.find(definition => definition.catalogueRow === 'ec11-evqwgd001')?.electrical.rotaryProfile).toMatchObject({ steps: 2, triggersPerRotation: 4 });
  expect(restored.keymap?.macros.find(macro => macro.id === 'review_navigation_pulse')?.steps).toHaveLength(3);
  expect(restored.embeddedCircuits?.[0].partIds).toEqual(circuit.partIds);

  await navigateWorkspace(page, 'Keymap');
  await page.getByRole('button', { name: 'Export ZMK source', exact: true }).click();
  // The fixture intentionally has no reviewed host controller/wiring, so it may not produce
  // firmware merely because it exercises the editable keymap and module profiles.
  await expect(page.getByRole('alert')).toContainText(/controller|GPIO|wiring/iu);

  await navigateWorkspace(page, 'Case');
  await expect(page.locator('.wb-assembly-scene canvas')).toBeVisible();
  await expect(page.getByText('Preparing mounted modules…', { exact: true })).toHaveCount(0, { timeout: 60_000 });
  await expect(page.locator('.wb-assembly-error')).toHaveCount(0);
  await page.screenshot({ path: 'test-results/vik-module-human-review-case.png' });
});

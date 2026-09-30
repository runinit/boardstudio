import { expect, test } from '@playwright/test';
import { readFile } from 'node:fs/promises';
import { unzipSync, strFromU8 } from 'fflate';
import { openKeymapFixture } from './keymap-fixture';
import { navigateWorkspace } from './workspace-navigation';

test('layers, hold taps and structured macros persist and export through Rust', async ({ page }) => {
  const errors: string[] = []; page.on('pageerror', error => errors.push(error.message));
  await openKeymapFixture(page); await navigateWorkspace(page, 'Keymap');
  const panel = page.locator('.wb-keymap-panel');
  await expect(panel.getByLabel('Board keycap color')).toHaveCount(0);
  await page.getByRole('button', { name: 'Edit key SW1', exact: true }).click();
  await panel.getByLabel('SW1 behavior', { exact: true }).selectOption('mod-tap');
  await panel.getByLabel('SW1 tap', { exact: true }).fill('SPACE');
  await panel.getByLabel('SW1 tap', { exact: true }).press('Tab');
  await expect(page.locator('.wb-keymap-layout .is-selected text').first()).toHaveText('SPACE / LSHIFT');
  await panel.getByRole('button', { name: 'Add layer', exact: true }).click();
  await panel.getByRole('group', { name: 'Keymap layers' }).getByRole('button', { name: '1 Layer 1' }).click();
  await panel.getByLabel('SW1 behavior', { exact: true }).selectOption('layer-tap');
  await panel.getByLabel('SW1 layer', { exact: true }).selectOption('base');
  await panel.getByRole('button', { name: 'Macros', exact: true }).click();
  await panel.getByRole('button', { name: 'Add macro', exact: true }).click();
  await panel.getByRole('button', { name: 'Add step', exact: true }).click();
  await panel.getByLabel('Macro 1 step 2', { exact: true }).selectOption('wait');
  await panel.getByRole('button', { name: 'Keys', exact: true }).click();
  await panel.getByLabel('SW1 behavior', { exact: true }).selectOption('macro');
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(panel.getByLabel('SW1 behavior', { exact: true })).toHaveValue('layer-tap');
  await page.getByRole('button', { name: 'Redo', exact: true }).click();
  await expect(panel.getByLabel('SW1 behavior', { exact: true })).toHaveValue('macro');
  await page.reload(); await navigateWorkspace(page, 'Keymap');
  await panel.getByRole('group', { name: 'Keymap layers' }).getByRole('button', { name: '1 Layer 1' }).click();
  await page.getByRole('button', { name: 'Edit key SW1', exact: true }).click();
  await expect(panel.getByLabel('SW1 behavior', { exact: true })).toHaveValue('macro');
  const download = page.waitForEvent('download'); await panel.getByRole('button', { name: 'Export ZMK source', exact: true }).click();
  const archive = unzipSync(await readFile((await (await download).path())!));
  const source = strFromU8(archive['config/boards/shields/boardstudio/boardstudio.keymap']);
  expect(source).toContain('&mt LSHIFT SPACE'); expect(source).toContain('&bs_macro_0'); expect(source).toContain('&macro_press &none');
  expect(strFromU8(archive['config/boards/shields/boardstudio/boardstudio.conf'])).toContain('CONFIG_ZMK_BEHAVIORS_QUEUE_SIZE=512');
  expect(errors).toEqual([]);
});

test('keymap controls fit a narrow viewport', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await openKeymapFixture(page); await navigateWorkspace(page, 'Keymap');
  await page.getByLabel('Selected key', { exact: true }).selectOption({ label: 'SW1 · Unassigned' });
  await page.getByLabel('SW1 behavior', { exact: true }).selectOption('mod-tap');
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await expect.poll(() => page.locator('.wb-keymap-panel').locator('input:visible, select:visible, button:visible').evaluateAll(elements => elements.filter(element => element.getBoundingClientRect().right > innerWidth || element.getBoundingClientRect().width < 40).length)).toBe(0);
});

test('encoder rotation and push bindings reach the local ZMK package', async ({ page }) => {
  await openKeymapFixture(page, true); await navigateWorkspace(page, 'Keymap');
  const panel = page.locator('.wb-keymap-panel');
  await panel.getByRole('button', { name: 'Encoders', exact: true }).click();
  for (const [direction, keycode] of [['clockwise', 'C_VOL_UP'], ['counterclockwise', 'C_VOL_DN']] as const) {
    await panel.getByLabel(`ENC1 ${direction} behavior`, { exact: true }).selectOption('key-press');
    await panel.getByLabel(`ENC1 ${direction} keycode`, { exact: true }).fill(keycode);
    await panel.getByLabel(`ENC1 ${direction} keycode`, { exact: true }).press('Tab');
  }
  await panel.getByText('Push button', { exact: true }).click();
  await panel.getByLabel('ENC1 push behavior', { exact: true }).selectOption('key-press');
  const download = page.waitForEvent('download'); await panel.getByRole('button', { name: 'Export ZMK source', exact: true }).click();
  const archive = unzipSync(await readFile((await (await download).path())!));
  const source = strFromU8(archive['config/boards/shields/boardstudio/boardstudio.keymap']);
  expect(source).toContain('<&kp C_VOL_UP>, <&kp C_VOL_DN>'); expect(source).toContain('sensor-bindings');
  expect(strFromU8(archive['config/boards/shields/boardstudio/boardstudio.overlay'])).toContain('zmk,keymap-sensors');
});

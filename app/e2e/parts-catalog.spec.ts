import { navigateWorkspace } from './workspace-navigation';
import { expect, test, type Locator } from '@playwright/test';

const thqNames = [
  'THQWGD001 · rotation only · reversible',
  'THQWGD001C · 2-pin tactile · reversible',
  'THQWGD001C · 4-pin tactile · reversible',
  'THQWGD001 mounting cuts · routed slots',
  'THQWGD001 mounting cuts · drilled slots',
  'THQWGD001C mounting cuts · routed slots',
  'THQWGD001C mounting cuts · drilled slots',
];

async function expectCanonicalLibrary(library: Locator) {
  await expect(library.getByRole('option').filter({ hasNotText: /^THQWGD001/ })).toHaveCount(38);
  await expect(library.getByRole('option').filter({ hasText: /^THQWGD001/ })).toHaveCount(7);
  for (const name of thqNames) await expect(library.getByRole('option', { name, exact: true })).toHaveCount(1);
}

const retired = ['mx-switch', 'mx-hotswap', 'choc-switch', 'choc-hotswap', 'rgb-led', 'matrix-diode', 'ergogen:infused-kim/choc', 'ergogen:infused-kim/diode'];

test('the Parts catalog shows preferred key parts and uses the canonical starter board', async ({ page }) => {
  await page.goto('/');
  const revision = await page.locator('.wb-root').getAttribute('data-revision');
  await navigateWorkspace(page, 'Parts');
  const library = page.getByRole('listbox', { name: 'Footprint library' });
  for (const name of ['MX switch', 'Choc V1 / V2 switch', 'Gateron KS27 / KS33 switch', 'Matrix diode (SOD-123 / THT)', 'SK6812 MINI-E']) {
    await expect(library.getByRole('option', { name, exact: true })).toHaveCount(1);
  }
  await expectCanonicalLibrary(library);
  for (const source of ['builtin:mx-switch', 'builtin:choc-hotswap', 'infused-kim/choc', 'infused-kim/diode', 'infused-kim/nice_nano_pretty']) {
    await expect(library.locator(`[title="${source}"]`)).toHaveCount(0);
  }
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', revision!);
  await navigateWorkspace(page, 'Layout');
  await expect(page.locator('.wb-scene-part')).toHaveCount(15);
  await page.getByRole('treeitem', { name: /^Matrix 1/ }).click();
  await page.locator('summary').filter({ hasText: /^Key assembly/ }).click();
  const select = page.getByRole('combobox', { name: 'Matrix part definition' });
  await expect(select).toHaveValue('ergogen:ceoloide/switch_mx');
  await expect(select.locator('option[value="ergogen:ceoloide/switch_mx"]')).toHaveCount(1);
  for (const id of retired) await expect(select.locator(`option[value="${id}"]`)).toHaveCount(0);
  await select.selectOption('ergogen:ceoloide/switch_choc_v1_v2');
  await expect(page.locator('.wb-scene-part')).toHaveCount(15);
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(select).toHaveValue('ergogen:ceoloide/switch_mx');
});

test('assembly component selectors omit duplicates and label Choc V1 explicitly', async ({ page }) => {
  await page.goto('/');
  await navigateWorkspace(page, 'Parts');
  await page.getByRole('listbox', { name: 'Key assemblies' }).getByRole('option', { name: 'Choc V1 Hotswap RGB', exact: true }).click();
  await page.getByRole('button', { name: 'Customize 3D assembly' }).click();
  const components = page.getByRole('combobox', { name: 'Component', exact: true });
  await expect(components).toHaveCount(3);
  for (const id of retired) await expect(components.locator(`option[value="${id}"]`)).toHaveCount(0);
  await expect(components.first()).toHaveValue('ergogen:ceoloide/switch_choc_v1_v2');
  await expect(components.nth(2)).toHaveValue('ergogen:ceoloide/led_sk6812mini-e');
});

test('placing, undoing, and reloading an assembly never adds duplicate catalog products', async ({ page }) => {
  await page.goto('/');
  const parts = page.getByRole('tab', { name: 'Parts', exact: true });
  const library = page.getByRole('listbox', { name: 'Footprint library' });
  await parts.click();
  await page.getByRole('searchbox', { name: 'Search footprints' }).fill('RGB');
  await expect(library.getByRole('option', { name: 'SK6812 MINI-E', exact: true })).toBeVisible();
  await page.getByRole('searchbox', { name: 'Search footprints' }).fill('');
  await page.getByRole('option', { name: 'MX Hotswap RGB', exact: true }).click();
  await page.getByRole('button', { name: 'Place key assembly' }).click();
  await page.getByRole('button', { name: 'Ghost key, row 1, column 1', exact: true }).click();
  await expect(page.locator('.wb-scene-part')).toHaveCount(18);
  await parts.click();
  await expectCanonicalLibrary(library);
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expectCanonicalLibrary(library);
  await page.getByRole('button', { name: 'Redo', exact: true }).click();
  await navigateWorkspace(page, 'Layout');
  await expect(page.locator('.wb-scene-part')).toHaveCount(18);
  await expect(page.getByLabel('Saved locally', { exact: true })).toBeVisible();
  await page.reload();
  await parts.click();
  await expectCanonicalLibrary(library);
  await navigateWorkspace(page, 'Layout');
  await expect(page.locator('.wb-scene-part')).toHaveCount(18);
});

test('updating a matrix preset is explicit, repeatable, and undoable', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('treeitem', { name: /^Matrix 1/ }).click();
  await page.locator('summary').filter({ hasText: /^Key assembly/ }).click();
  await page.getByRole('combobox', { name: 'Apply matrix preset' }).selectOption('mx-hotswap-rgb');
  const update = page.getByRole('button', { name: 'Update assembly preset', exact: true });
  await update.click();
  await expect(page.locator('.wb-scene-part')).toHaveCount(45);
  const geometry = await page.locator('.wb-scene-part').evaluateAll(nodes => nodes.map(node => node.getAttribute('transform')));
  await update.click();
  await expect(page.locator('.wb-scene-part')).toHaveCount(45);
  expect(await page.locator('.wb-scene-part').evaluateAll(nodes => nodes.map(node => node.getAttribute('transform')))).toEqual(geometry);
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(page.locator('.wb-scene-part')).toHaveCount(15);
  await expect(page.getByRole('combobox', { name: 'Matrix part definition' })).toHaveValue('ergogen:ceoloide/switch_mx');
});

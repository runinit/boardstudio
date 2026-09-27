import { navigateWorkspace, openWorkspaceSettings } from './workspace-navigation';
import { zipSync, strToU8 } from 'fflate';
import { catalogue } from '@boardstudio/v2-ergogen';
import { demoProject } from '../src/demo';
import { expect, test } from '@playwright/test';

for (const source of ['ceoloide/switch_choc_v1_v2', 'ceoloide/switch_gateron_ks27_ks33']) {
  test(`${source} dashed outline follows the centered keycap dimensions`, async ({ page }) => {
    await page.goto('/');

    await expect(page.locator('.wb-outline-shape')).toHaveCount(1);
    await navigateWorkspace(page, 'Parts');
    await page.getByRole('searchbox', { name: 'Search footprints' }).fill(source);
    await page.getByRole('listbox', { name: 'Footprint library', exact: true }).getByRole('option').click();
    await expect(page.getByRole('checkbox', { name: 'include_keycap', exact: true })).toBeChecked();
    if (source.includes('gateron')) {
      await page.locator('summary').filter({ hasText: 'Advanced footprint options' }).click();
      await page.getByRole('checkbox', { name: 'include_socket_silks', exact: true }).check();
      await page.getByRole('checkbox', { name: 'include_socket_fabs', exact: true }).check();
    }
    const sides = source.startsWith('ceoloide/') ? ['B', 'F'] : ['B'];
    for (const side of sides) {
      if (source.startsWith('ceoloide/')) await page.getByRole('combobox', { name: 'side', exact: true }).selectOption(side);
      const bounds = await page.getByRole('img', { name: 'Footprint preview', exact: true }).evaluate((svg) => {
        const box = (selector: string) => {
          const { x, y, width, height } = (svg.querySelector(selector) as SVGGraphicsElement).getBBox();
          return { left: x, top: y, right: x + width, bottom: y + height };
        };
        return { outline: box('.wb-preview-courtyard, .wb-preview-keycap'), body: box('.wb-ergogen-drawing') };
      });
      const height = 18;
      expect(bounds.outline.left).toBeCloseTo(-9);
      expect(bounds.outline.right).toBeCloseTo(9);
      expect(bounds.outline.top).toBeCloseTo(-height / 2);
      expect(bounds.outline.bottom).toBeCloseTo(height / 2);
      await expect(page.locator('.wb-library-workspace-scale')).toContainText('Dashed: keycap');
      // Socket graphics retain their KiCad position relative to the pads.
      expect(bounds.body.right - bounds.body.left).toBeGreaterThan(0);
      expect(bounds.body.top).toBeCloseTo(source.includes('gateron') ? 2.525 : -8.2);
      expect(bounds.body.bottom).toBeCloseTo(source.includes('gateron') ? 7.925 : -1.5);
    }
  });
}

test('keycap outline resizes with settings, survives undo and reload, and leaves component courtyards intact', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.wb-outline-shape')).toHaveCount(1);
  await navigateWorkspace(page, 'Parts');
  const search = page.getByRole('searchbox', { name: 'Search footprints' });
  const library = page.getByRole('listbox', { name: 'Footprint library', exact: true });
  await search.fill('ceoloide/switch_mx');
  await library.getByRole('option', { name: /MX switch/ }).click();
  const keycapBox = () => page.locator('.wb-preview-keycap').evaluate((node) => {
    const { x, y, width, height } = (node as SVGGraphicsElement).getBBox();
    return { x, y, width, height };
  });
  const original = { x: -9, y: -9, width: 18, height: 18 };
  const resized = { x: -11.5, y: -9.5, width: 23, height: 19 };
  await expect.poll(keycapBox).toEqual(original);
  await page.getByRole('spinbutton', { name: 'Keycap Width', exact: true }).fill('23');
  await page.getByRole('spinbutton', { name: 'Keycap Height', exact: true }).fill('19');
  await expect.poll(keycapBox).toEqual(resized);
  await page.getByRole('button', { name: 'Apply generator settings' }).click();
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect.poll(keycapBox).toEqual(original);
  await page.getByRole('button', { name: 'Redo', exact: true }).click();
  await expect.poll(keycapBox).toEqual(resized);
  await page.reload();
  await navigateWorkspace(page, 'Parts');
  await search.fill('ceoloide/switch_mx');
  await library.getByRole('option', { name: /MX switch/ }).click();
  await expect.poll(keycapBox).toEqual(resized);
  await expect(page.locator('.wb-preview-courtyard')).toHaveCount(0);
  await expect(page.locator('.wb-library-workspace-scale')).toContainText('Keycap 23.0 × 19.0 mm');
  await page.getByRole('checkbox', { name: 'include_keycap', exact: true }).uncheck();
  await expect(page.locator('.wb-preview-keycap')).toHaveCount(0);
  const savedRevision = page.locator('.wb-root');
  const previousRevision = await savedRevision.getAttribute('data-revision');
  await page.getByRole('button', { name: 'Apply generator settings' }).click();
  await expect(savedRevision).not.toHaveAttribute('data-revision', previousRevision!);
  await page.reload();
  await navigateWorkspace(page, 'Parts');
  await search.fill('ceoloide/switch_mx');
  await library.getByRole('option', { name: /MX switch/ }).click();
  await expect(page.getByRole('checkbox', { name: 'include_keycap', exact: true })).not.toBeChecked();
  await expect(page.locator('.wb-preview-keycap')).toHaveCount(0);
  await search.fill('SK6812 MINI-E');
  await library.getByRole('option', { name: /^SK6812 MINI-E/ }).click();
  await expect(page.locator('.wb-preview-keycap')).toHaveCount(0);
  await expect(page.locator('.wb-preview-courtyard')).toBeVisible();
  await expect(page.locator('.wb-library-workspace-scale')).toContainText('Dashed: courtyard');
});

test('bundled Ergogen generators are searchable, editable, and previewed', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.wb-outline-shape')).toHaveCount(1);
  await navigateWorkspace(page, 'Parts');
  const search = page.getByRole('searchbox', { name: 'Search footprints' });
  await search.fill('ceoloide/utility_filled_zone');
  await expect(page.getByRole('region', { name: 'Utilities' })).toBeVisible();
  await page.getByRole('option', { name: /utility filled zone/ }).click();
  await expect(page.locator('.wb-ergogen-zone')).toBeVisible();
  await expect(page.getByRole('textbox', { name: 'name', exact: true })).toBeVisible();

  await page.locator('.wb-topbar').getByRole('button', { name: 'Export', exact: true }).click();
  const embed = page.getByRole('checkbox', { name: 'Embed used models' });
  await expect(embed).toBeChecked();
  await embed.uncheck();
  await expect(embed).not.toBeChecked();
});

test('saved Ergogen parameters return when switching library items', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.wb-outline-shape')).toHaveCount(1);
  await navigateWorkspace(page, 'Parts');
  const search = page.getByRole('searchbox', { name: 'Search footprints' });
  await search.fill('ceoloide/switch_mx');
  await page.getByRole('option', { name: /MX switch/ }).first().click();
  await page.locator('summary').filter({ hasText: 'Advanced footprint options' }).click();
  const traceWidth = page.getByRole('spinbutton', { name: 'Trace Width' });
  await traceWidth.fill('0.47');
  await page.getByRole('button', { name: 'Apply generator settings' }).click();
  await search.fill('ceoloide/utility_text');
  await page.getByRole('option', { name: /utility text/ }).click();
  await search.fill('ceoloide/switch_mx');
  await page.getByRole('option', { name: /MX switch/ }).first().click();
  if (!await page.getByRole('spinbutton', { name: 'Trace Width' }).isVisible()) await page.locator('summary').filter({ hasText: 'Advanced footprint options' }).click();
  await expect(page.getByRole('spinbutton', { name: 'Trace Width' })).toHaveValue('0.47');
  await page.getByRole('button', { name: 'Undo' }).click();
  await expect(page.getByRole('spinbutton', { name: 'Trace Width' })).not.toHaveValue('0.47');
  await page.getByRole('button', { name: 'Redo' }).click();
  await expect(page.getByRole('spinbutton', { name: 'Trace Width' })).toHaveValue('0.47');
});

test('attaching a model to an unsaved generator definition stays within the workbench', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.wb-outline-shape')).toHaveCount(1);
  await navigateWorkspace(page, 'Parts');
  await page.getByRole('searchbox', { name: 'Search footprints' }).fill('ceoloide/switch_mx');
  await page.getByRole('option', { name: /MX switch/ }).first().click();
  await page.locator('summary').filter({ hasText: '3D model placement' }).click();
  await page.getByRole('group', { name: '3D model placement' }).getByLabel('Attach STEP / STL / WRL').first().setInputFiles({ name: 'switch.step', mimeType: 'model/step', buffer: Buffer.from('not a valid STEP model') });
  await expect(page.getByRole('textbox', { name: 'switch_3dmodel_filename' })).toHaveValue(/^boardstudio-asset:/);
  await expect(page.getByRole('button', { name: 'Apply generator settings' })).toBeEnabled();
  await page.getByRole('button', { name: 'Undo' }).click();
  await expect(page.getByRole('textbox', { name: 'switch_3dmodel_filename' })).not.toHaveValue(/^boardstudio-asset:/);
  await page.getByRole('button', { name: 'Redo' }).click();
  await expect(page.getByRole('textbox', { name: 'switch_3dmodel_filename' })).toHaveValue(/^boardstudio-asset:/);
  await expect(page.getByRole('tab', { name: 'Layout' })).toBeVisible();
});

test('bundled MX models load and generator settings are grouped', async ({ page }, info) => {
  test.setTimeout(60_000);
  await page.goto('/');
  await expect(page.locator('.wb-outline-shape')).toHaveCount(1);
  await navigateWorkspace(page, 'Parts');
  await page.getByRole('searchbox', { name: 'Search footprints' }).fill('ceoloide/switch_mx');
  await page.getByRole('option', { name: /MX switch/ }).click();
  await page.getByRole('checkbox', { name: 'hotswap', exact: true }).check();
  await page.getByRole('button', { name: '3D model', exact: true }).click();
  await expect(page.getByText('2 / 2 models · 1.6 mm PCB', {exact:true})).toBeVisible({ timeout: 20_000 });
  const canvasBox = (await page.locator('.wb-library-model-workspace canvas').boundingBox())!;
  const switchBox = (await page.getByRole('button', { name: '2D footprint', exact: true }).boundingBox())!;
  expect(canvasBox.y).toBeGreaterThanOrEqual(switchBox.y + switchBox.height);
  await expect(page.getByRole('group', { name: 'Footprint options', exact: true })).toBeVisible();
  await expect(page.getByRole('checkbox', { name: 'reversible', exact: true })).toBeVisible();
  await page.screenshot({ path: info.outputPath('mx-desktop.png') });
  await openWorkspaceSettings(page);
  await page.getByRole('combobox', { name: 'Color theme' }).selectOption('dark');
  await page.keyboard.press('Escape');
  await page.screenshot({ path: info.outputPath('mx-desktop-dark.png') });
  await page.setViewportSize({ width: 390, height: 844 });
  if (!await page.locator('.wb-inspector-content').isVisible()) await page.getByRole('button', { name: 'Inspect', exact: true }).click();
  await expect.poll(() => page.locator('.wb-inspector-content').evaluate((node) => node.scrollWidth <= node.clientWidth + 1)).toBe(true);
  await page.screenshot({ path: info.outputPath('mx-narrow.png') });
  await openWorkspaceSettings(page);
  await page.getByRole('combobox', { name: 'Color theme' }).selectOption('light');
  await page.keyboard.press('Escape');
  await page.screenshot({ path: info.outputPath('mx-narrow-light.png') });
});

test.describe('model network recovery', () => {
  test.use({ serviceWorkers: 'block' });

test('model failures can retry and parts without models do not keep loading', async ({ page }) => {
  test.setTimeout(60_000);
  const modelUrl = /\.(step|stp)(\?|$)/i;
  await page.route(modelUrl, (route) => route.request().resourceType() === 'fetch' ? route.abort() : route.continue());
  await page.goto('/');
  await expect(page.locator('.wb-outline-shape')).toHaveCount(1);
  await navigateWorkspace(page, 'Parts');
  const search = page.getByRole('searchbox', { name: 'Search footprints' });
  await search.fill('ceoloide/switch_mx');
  await page.getByRole('option', { name: /MX switch/ }).click();
  await page.getByRole('checkbox', { name: 'hotswap', exact: true }).check();
  await page.getByRole('button', { name: '3D model', exact: true }).click();
  await page.getByText('Sample PCB', {exact:true}).click();
  await expect(page.getByRole('button', { name: 'Retry models' })).toBeVisible();
  await page.unroute(modelUrl);
  await page.getByRole('button', { name: 'Retry models' }).click();
  await expect(page.getByText('2 / 2 models · 1.6 mm PCB', {exact:true})).toBeVisible({ timeout: 20_000 });
  await search.fill('ceoloide/utility_filled_zone');
  await page.getByRole('option', { name: /utility filled zone/ }).click();
  await page.getByRole('button', { name: '3D model', exact: true }).click();
  await page.getByText('Sample PCB', {exact:true}).click();
  await expect(page.getByText(/No component models are attached/)).toBeVisible();
  await expect(page.locator('.wb-assembly-scene canvas')).toBeVisible();
  await expect(page.getByText('Preparing PCB assembly…')).toHaveCount(0);
});

});

import { navigateWorkspace } from './workspace-navigation';
import { expect, test } from '@playwright/test';
import { readFile } from 'node:fs/promises';
import { unzipSync } from 'fflate';

const importedSource = `(footprint "Rich Imported Ω" (version 20240108) (generator "pcbnew")
  (layer "F.Cu") (uuid 00000000-0000-4000-8000-000000000001)
  (at 0 0)
  (property "Reference" "REF**" (at 0 0 0) (layer "F.SilkS") (effects (font (size 1 1) (thickness 0.15))))
  (property "Value" "Rich Imported Ω" (at 0 1 0) (layer "F.Fab") (effects (font (size 1 1) (thickness 0.15))))
  (property "Vendor note" "café 東京 & Ω" (at 0 2 0) (layer "F.Fab") (effects (font (size 1 1) (thickness 0.15))))
  (attr through_hole)
  (fp_arc (start -3 -2) (mid -2 -3) (end -1 -2) (stroke (width 0.12) (type default)) (layer "F.CrtYd") (uuid 00000000-0000-4000-8000-000000000002))
  (fp_line (start -3 -2) (end 3 -2) (stroke (width 0.12) (type default)) (layer "F.Fab") (uuid 00000000-0000-4000-8000-000000000003))
  (pad "1" thru_hole oval (at -1.5 0 37) (size 2 1) (drill 0.8) (layers "*.Cu" "*.Mask") (uuid 00000000-0000-4000-8000-000000000004))
  (pad "1" thru_hole circle (at 2 3 30) (size 1.8 1.8) (drill 0.8) (layers "*.Cu" "*.Mask") (uuid 00000000-0000-4000-8000-000000000005)))`;

test('imports, keeps source pads read-only, and exports the original KiCad source after reload', async ({ page }) => {
  await page.goto('/');
  await navigateWorkspace(page, 'Parts');
  await page.locator('.wb-footprint-import input[type=file][accept=".kicad_mod"]').setInputFiles({
    name: 'rich.kicad_mod',
    mimeType: 'text/plain',
    buffer: Buffer.from(importedSource),
  });

  const search = page.getByRole('searchbox', { name: 'Search footprints' });
  await search.fill('Rich Imported');
  await page.getByRole('option', { name: 'Rich Imported Ω', exact: true }).click();
  await page.getByText('Edit footprint', { exact: true }).click();
  await expect(page.getByRole('textbox', { name: 'Definition name' })).toHaveValue('Rich Imported Ω');
  await expect(page.getByRole('textbox', { name: 'Pad 1 number' })).toBeDisabled();
  await expect(page.getByRole('button', { name: '+ Add pad' })).toBeDisabled();
  await expect(page.getByText(/Imported pad geometry stays linked/u)).toBeVisible();

  const width = page.getByRole('spinbutton', { name: 'Courtyard width' });
  const revision = await page.locator('.wb-root').getAttribute('data-revision');
  await width.fill('14');
  await width.blur();
  await expect(page.locator('.wb-root')).not.toHaveAttribute('data-revision', revision!);
  await expect(page.locator('.wb-save-state summary')).toHaveAccessibleName('Saved locally');
  await page.reload();
  await navigateWorkspace(page, 'Parts');
  await search.fill('Rich Imported');
  await page.getByRole('option', { name: 'Rich Imported Ω', exact: true }).click();
  await page.getByText('Edit footprint', { exact: true }).click();
  await expect(page.getByRole('spinbutton', { name: 'Courtyard width' })).toHaveValue('14');

  await page.locator('.wb-topbar').getByRole('button', { name: 'Export', exact: true }).click();
  const download = page.waitForEvent('download');
  await page.locator('.wb-export-row').filter({ hasText: 'KiCad footprints' }).getByRole('button', { name: 'Export' }).click();
  const archive = unzipSync(new Uint8Array(await readFile(await (await download).path())));
  const footprintPath = Object.keys(archive).find((path) => path.startsWith('BoardStudio.pretty/')
    && path.endsWith('.kicad_mod')
    && new TextDecoder().decode(archive[path]).includes('Vendor note'));
  expect(footprintPath).toBeDefined();
  const exported = new TextDecoder().decode(archive[footprintPath!]);
  expect(exported).toContain('(property "Vendor note" "café 東京 & Ω"');
  expect(exported.match(/\(pad "1"/gu)).toHaveLength(2);
  expect(exported).toContain('(at -1.5 0 37)');
  expect(exported).toContain('(at 2 3 30)');
});

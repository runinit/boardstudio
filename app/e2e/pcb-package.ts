import { navigateWorkspace } from './workspace-navigation';
import { readFile } from 'node:fs/promises';
import { expect, type Page } from '@playwright/test';
import { strFromU8, unzipSync } from 'fflate';

// Geometry fixtures have no MCU. Exercise the explicit draft handoff while
// retaining the production wiring gate and inspecting the actual board files.
export async function downloadDraftBoard(page: Page, { reviewExistingConnections = false } = {}) {
  if (reviewExistingConnections) {
    await navigateWorkspace(page, 'Layout');
    await navigateWorkspace(page, 'PCB');
    await page.getByText('Review existing connections', { exact: true }).click();
    await page.getByRole('button', { name: 'Use automatic wiring for these connections', exact: true }).click();
    await expect(page.getByText('Review existing connections', { exact: true })).toHaveCount(0);
    await page.locator('.wb-topbar').getByRole('button', { name: 'Export', exact: true }).click();
  }
  await expect(page.getByRole('button', { name: 'Export KiCad board', exact: true })).toBeDisabled();
  const downloading = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Export Draft KiCad board', exact: true }).click();
  const download = await downloading;
  const packageFiles = unzipSync(await readFile((await download.path())!));
  const report = JSON.parse(strFromU8(packageFiles['wiring-report.json']));
  expect(report.draft).toBe(true);
  expect(report.plan.diagnostics.some((finding: { severity: string }) => finding.severity === 'error')).toBe(true);
  const files = { ...packageFiles };
  for (const [name, bytes] of Object.entries(packageFiles)) {
    if (name.endsWith('.zip')) Object.assign(files, unzipSync(bytes));
  }
  const boards = Object.keys(files).filter(name => name.endsWith('.kicad_pcb'));
  expect(boards).toHaveLength(1);
  return { files, boardName: boards[0], board: strFromU8(files[boards[0]]), filename: download.suggestedFilename() };
}

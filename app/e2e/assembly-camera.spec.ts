import { expect, test } from '@playwright/test';
import { navigateWorkspace } from './workspace-navigation';
import { splitFixture } from './splitMechanicalFixture';
import { saveWorkspaceDocuments } from './workspace-storage';

test('retained assembly stays navigable while new geometry prepares', async ({ page }) => {
  // Hold a real worker request after the initial scene has been displayed.
  // This reproduces slow background preparation without replacing the renderer.
  await page.addInitScript(() => {
    const audit = { hold: false, release: undefined as (() => void) | undefined };
    Object.assign(window, { cameraPreparation: audit });
    const OriginalWorker = window.Worker;
    window.Worker = class extends OriginalWorker {
      constructor(url: string | URL, options?: WorkerOptions) {
        super(url, options);
        if (!String(url).includes('scene.worker')) return;
        const post = this.postMessage.bind(this);
        this.postMessage = (message: unknown, transfer?: Transferable[]) => {
          if (audit.hold) audit.release = () => post(message, transfer ?? []);
          else post(message, transfer ?? []);
        };
      }
    };
  });
  await page.goto('/');
  await saveWorkspaceDocuments(page, [splitFixture()]);
  await page.reload();
  await navigateWorkspace(page, 'Case');
  const configure = page.getByRole('button', { name: 'Configure mechanical stack', exact: true });
  if (await configure.count()) await configure.click();
  await expect(page.getByRole('region', { name: 'Case generation' })).toHaveAttribute('data-state', 'ready', { timeout: 45_000 });
  await expect(page.getByText('Preparing 3D geometry…', { exact: true })).toHaveCount(0, { timeout: 30_000 });
  const controls = page.getByRole('group', { name: 'Assembly camera' });
  await expect(controls.getByRole('button', { name: 'Fit', exact: true })).toBeEnabled();
  await page.evaluate(() => { (window as unknown as { cameraPreparation: { hold: boolean } }).cameraPreparation.hold = true; });
  await page.locator('.wb-mechanical-panel summary').filter({ hasText: 'Dimensions & clearances' }).click();
  const thickness = page.getByRole('spinbutton', { name: 'Wall thickness mm', exact: true });
  await thickness.fill('2.5');
  await thickness.press('Tab');
  await page.waitForFunction(() => typeof (window as unknown as { cameraPreparation: { release?: unknown } }).cameraPreparation.release === 'function');
  await expect(page.getByText('Preparing 3D geometry…', { exact: true })).toBeVisible();
  await expect(controls.getByRole('button', { name: 'Fit', exact: true })).toBeEnabled();
  await controls.getByRole('button', { name: 'Top', exact: true }).click();
  await page.evaluate(() => {
    const audit = (window as unknown as { cameraPreparation: { hold: boolean; release: () => void } }).cameraPreparation;
    audit.hold = false;
    audit.release();
  });
  await expect(page.getByText('Preparing 3D geometry…', { exact: true })).toHaveCount(0, { timeout: 30_000 });
  await expect(controls.getByRole('button', { name: 'Fit', exact: true })).toBeEnabled();
});

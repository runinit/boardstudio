import { navigateWorkspace, openSetupGuide } from './workspace-navigation';
import { expect, test } from '@playwright/test';
import { readFile } from 'node:fs/promises';
import { unzipSync, strFromU8 } from 'fflate';
import { catalogue, normalizeDefinition } from '@boardstudio/v2-ergogen';
import { demoProject } from '../src/demo';

test('configured Case keeps physical half selection and setup available', async ({ page }) => {
  await page.addInitScript(() => {
    const audit = (window as any).__physicalPreviews = [];
    const original = Worker.prototype.postMessage;
    Worker.prototype.postMessage = function (message: any, ...rest: any[]) {
      if (message.kind === 'pcb-preview') audit.push({ parts: message.document.parts, contours: message.contours });
      return original.call(this, message, ...rest);
    };
  });
  await page.goto('/');
  await navigateWorkspace(page, 'Case');
  await page.getByRole('button', { name: 'Configure mechanical stack', exact: true }).click();
  await openSetupGuide(page);
  await page.getByRole('button', { name: /Project & hardware/ }).click();
  await page.getByRole('button', { name: 'Split keyboard', exact: true }).click();
  await page.getByRole('checkbox', { name: 'Reversible layout' }).click();
  await page.getByRole('button', { name: /Case \(optional\)/ }).click();
  await page.getByRole('button', { name: 'Back to objects', exact: true }).click();
  const assembly = page.getByRole('treeitem', { name: 'Right case assembly', exact: true });
  await expect(assembly).toBeVisible();
  await assembly.click();
  await expect(assembly).toHaveAttribute('aria-selected', 'true');
  await expect(page.getByRole('checkbox', { name: 'Reversible layout' })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Update preview', exact: true })).toBeVisible();
  await expect.poll(() => page.evaluate(() => (window as any).__physicalPreviews.at(-1)?.parts.find((part: any) => part.reference === 'SW5')?.pose.at.x)).toBeLessThan(0);
  const physical = await page.evaluate(() => (window as any).__physicalPreviews.at(-1));
  expect(Math.max(...physical.contours.flatMap((contour: any) => contour.points.map((point: any) => point.x)))).toBeLessThan(20);
  expect(physical.parts.find((part: any) => part.reference === 'SW5').side).toBe('back');
  await navigateWorkspace(page, 'Layout');
  await page.getByRole('button', { name: '3D assembly', exact: true }).click();
  await expect.poll(() => page.evaluate(() => (window as any).__physicalPreviews.at(-1)?.parts.find((part: any) => part.reference === 'SW5')?.pose.at.x)).toBeGreaterThan(70);
  const canonical = await page.evaluate(() => (window as any).__physicalPreviews.at(-1));
  expect(Math.min(...canonical.contours.flatMap((contour: any) => contour.points.map((point: any) => point.x)))).toBeGreaterThan(-20);
  expect(canonical.parts.find((part: any) => part.reference === 'SW5').side).toBe('front');
});

function fixture() {
  const doc = demoProject(); doc.id = 'electrical-handoff';
  const source = catalogue().find(definition => definition.generator?.source === 'ceoloide/mcu_nice_nano')!;
  const mcu = normalizeDefinition({ ...source, generator: { ...source.generator!, parameters: { reversible:true } } });
  doc.definitions.push(mcu);
  doc.parts.push({id:'controller-left',reference:'U1',definitionId:mcu.id,pose:{at:{x:104,y:19},rotation:0},side:'front'});
  doc.boards[0].partIds.push('controller-left');
  const envelope = doc.outline.find(outline => outline.kind === 'part-envelope');
  if (envelope?.kind === 'part-envelope') envelope.partIds.push('controller-left');
  doc.hardware = { topology:'unibody',transport:'none',instances:[],sharedConstruction:null,boards:[{boardId:'main-board',controllerPartId:'controller-left',mode:'direct',locks:{},assignments:{},keyBindings:{},jumperStates:{},protectedHandoff:null}] };
  return doc;
}

test('real MCU wiring exports a protected PCB package and matching editable firmware', async ({page}) => {
  test.setTimeout(90000);
  await page.goto('/');
  await expect(page.locator('.wb-root')).toBeVisible();
  await page.evaluate(async document => {
    const db = await new Promise<IDBDatabase>((resolve,reject) => {const request=indexedDB.open('boardstudio-v2',1);request.onsuccess=()=>resolve(request.result);request.onerror=()=>reject(request.error);});
    await new Promise<void>((resolve,reject) => {const transaction=db.transaction('projects','readwrite');transaction.objectStore('projects').put(document);transaction.oncomplete=()=>resolve();transaction.onerror=()=>reject(transaction.error);});
    db.close();localStorage.setItem('boardstudio-v2-active-project',document.id);
  },fixture());
  await page.reload();
  await navigateWorkspace(page, 'PCB');
  await expect(page.getByRole('button', { name: 'Apply wiring', exact: true })).toBeDisabled();
  await page.getByText('Review existing connections', { exact: true }).click();
  await page.getByRole('button', { name: 'Use automatic wiring for these connections', exact: true }).click();
  await expect(page.getByRole('button',{name:'Apply wiring',exact:true})).toBeEnabled();
  await page.getByRole('button',{name:'Apply wiring',exact:true}).click();
  await page.getByText('Firmware keymap',{exact:true}).click();
  await page.getByLabel('Binding for SW1',{exact:true}).selectOption('&kp A');
  await page.getByRole('button',{name:'Export',exact:true}).click();
  const pcbDownload = page.waitForEvent('download');
  await page.getByRole('button',{name:'Export KiCad board',exact:true}).click();
  const pcb = await pcbDownload;
  const packed = unzipSync(await readFile((await pcb.path())!));
  expect(packed['wiring-report.json']).toBeTruthy();
  expect(strFromU8(packed['ASSEMBLY.md'])).toContain('Bridge');
  expect(Object.keys(packed).some(name=>name.startsWith('jumpers/'))).toBe(true);
  const firmwareDownload = page.waitForEvent('download');
  await page.getByRole('button',{name:'Export ZMK firmware',exact:true}).click();
  const firmware = unzipSync(await readFile((await (await firmwareDownload).path())!));
  expect(strFromU8(firmware['config/boards/shields/boardstudio/boardstudio.keymap'])).toContain('&kp A');
  expect(strFromU8(firmware['electrical-plan.json'])).toContain('P0.31');
  await page.getByRole('button', { name: 'Back to PCB', exact: true }).click();
  await expect(page.getByText('Protected handoff',{exact:true})).toBeVisible();
  await expect(page.locator('.app-error')).toHaveCount(0);
});

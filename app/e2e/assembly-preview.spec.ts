import { navigateWorkspace } from './workspace-navigation';
import { expect, test, type Locator } from '@playwright/test';

async function openLayers(page: import('@playwright/test').Page) {
  const trigger = page.getByRole('region', { name: 'Canvas layers' }).getByRole('button', { name: 'Layers', exact: true });
  if (await trigger.getAttribute('aria-expanded') === 'false') await trigger.click();
}

async function countPcbPixels(canvas: Locator) {
  const screenshot = await canvas.screenshot();
  return await canvas.page().evaluate(async ({ imageData }) => {
    const image = new Image();
    image.src = `data:image/png;base64,${imageData}`;
    await image.decode();
    const sample = document.createElement('canvas');
    sample.width = image.width;
    sample.height = image.height;
    const context = sample.getContext('2d');
    if (!context) throw new Error('Could not read the rendered assembly canvas');
    context.drawImage(image, 0, 0);
    const pixels = context.getImageData(0, 0, image.width, image.height).data;
    const counts = { green: 0, gold: 0, silkscreen: 0 };
    for (let index = 0; index < pixels.length; index += 4) {
      const red = pixels[index];
      const green = pixels[index + 1];
      const blue = pixels[index + 2];
      if (green > red * 1.25
        && green > blue * 0.9
        && red < 180) counts.green += 1;
      if (red > green * 1.05 && green > blue * 1.2 && red > 100) counts.gold += 1;
      // Rear lighting can round green and blue to the same byte (225,224,224).
      if (red > 180 && red < 250 && red > green && red - green < 4 && green >= blue && green - blue < 12)
        counts.silkscreen += 1;
    }
    return counts;
  }, { imageData: screenshot.toString('base64') });
}

const pcb=`(kicad_pcb (version 20241229) (general (thickness 1.2))
(gr_rect (start -8 -8) (end 8 8) (stroke (width 0.05)) (layer "Edge.Cuts"))
(gr_circle (center 5 5) (end 6 5) (stroke (width 0.05)) (layer "Edge.Cuts"))
(segment (start -4 0) (end 4 0) (width 0.5) (layer "F.Cu"))
(arc (start 4 0) (mid 5 1) (end 4 2) (width 0.3) (layer "B.Cu"))
(gr_text "PCB" (at 0 -5) (layer "F.SilkS") (effects (font (size 1 1))))
(footprint "component" (layer "F.Cu") (at 0 0) (property "Reference" "SW1")
(pad "1" thru_hole oval (at 0 0) (size 2 3) (drill oval 0.8 1.8) (layers "*.Cu" "*.Mask"))
(model "body.stl" (offset (xyz 0 0 0)) (scale (xyz 1 1 1)) (rotate (xyz 0 0 0)))))`;
const pcbWithoutModels=`(kicad_pcb (version 20241229) (general (thickness 1.6))
(gr_rect (start -8 -8) (end 8 8) (stroke (width 0.05)) (layer "Edge.Cuts"))
(gr_circle (center 0 0) (end 0.8 0) (stroke (width 0.05)) (layer "Edge.Cuts"))
(segment (start -6 -2) (end 5 -2) (width 0.35) (layer "F.Cu"))
(segment (start -5 2) (end 6 2) (width 0.3) (layer "B.Cu"))
(gr_text "FRONT" (at -2 -5) (layer "F.SilkS") (effects (font (size 1 1))))
(gr_text "BACK" (at 2 5) (layer "B.SilkS") (effects (font (size 1 1))))
(footprint "front-pad" (layer "F.Cu") (at -3 1) (property "Reference" "U1")
(pad "1" smd rect (at 0 0) (size 3 2) (layers "F.Cu" "F.Mask")))
(footprint "back-pad" (layer "B.Cu") (at 3 -1) (property "Reference" "U2")
(pad "1" smd rect (at 0 0) (size 2 3) (layers "B.Cu" "B.Mask"))))`;
const stl='solid part\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 2 0 0\nvertex 0 3 0\nendloop\nendfacet\nendsolid part';

test('Design shows PCB and case bodies with view-only visibility controls',async({page})=>{
  await page.goto('/');await navigateWorkspace(page, 'Case');
  await page.getByRole('button',{name:'Update preview',exact:true}).click();
  await expect(page.getByRole('region', { name: 'Case canvas' }).getByText('Preview current', { exact: true })).toBeVisible({timeout:45000});
  await navigateWorkspace(page, 'Layout');
  await page.getByRole('button',{name:'3D assembly',exact:true}).click();
  await expect(page.getByLabel('Complete PCB assembly preview')).toBeVisible();
  await expect(page.getByText(/1.6 mm PCB/)).toBeVisible();
  await openLayers(page);
  await expect(page.getByRole('button', { name: /^(Hide|Show) Switch\ plate$/, exact: true })).toBeVisible({timeout:45000});
  const revision=await page.locator('.wb-root').getAttribute('data-revision');
  await page.getByRole('button', { name: /^(Hide|Show) Switch\ plate$/, exact: true }).click();
  await page.getByRole('button', { name: /^(Hide|Show) Copper$/, exact: true }).click();
  await page.getByRole('button',{name:'Bottom',exact:true}).click();
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision',revision!);
  await page.getByRole('button',{name:'2D',exact:true}).click();await expect(page.getByRole('application',{name:/Board layout canvas/})).toBeVisible();
  await page.getByRole('button',{name:'3D assembly',exact:true}).click();
  await expect(page.locator('.wb-assembly-scene canvas')).toBeVisible();
  await expect(page.getByRole('group',{name:'Assembly camera'}).getByRole('button',{name:'Fit',exact:true})).toBeEnabled();
});

test('routed reference and STL mapping survive reopening and undo',async({page})=>{
  await page.goto('/');await page.getByRole('button',{name:'3D assembly',exact:true}).click();
  await page.getByText('Routed PCB reference',{exact:true}).click();
  await page.getByLabel('Import KiCad board',{exact:true}).setInputFiles({name:'routed.kicad_pcb',mimeType:'text/plain',buffer:Buffer.from(pcb)});
  await expect(page.getByText(/1.2 mm PCB/)).toBeVisible();
  await page.getByLabel('Attach model for body.stl',{exact:true}).setInputFiles({name:'body.stl',mimeType:'model/stl',buffer:Buffer.from(stl)});
  await expect(page.getByText('1 / 1 models · 1.2 mm PCB',{exact:true})).toBeVisible();
  await page.getByRole('spinbutton',{name:'Z (mm)',exact:true}).fill('7');await page.getByRole('spinbutton',{name:'Z (mm)',exact:true}).press('Tab');
  await page.reload();await page.getByRole('button',{name:'3D assembly',exact:true}).click();await page.getByText('Routed PCB reference',{exact:true}).click();
  await expect(page.getByRole('spinbutton',{name:'Z (mm)',exact:true})).toHaveValue('7');
  await expect(page.getByText('1 / 1 models · 1.2 mm PCB',{exact:true})).toBeVisible();
  await page.getByRole('button',{name:'Remove PCB reference',exact:true}).click();
  await expect(page.getByText(/1.6 mm PCB/)).toBeVisible();await page.getByRole('button',{name:'Undo',exact:true}).click();
  await expect(page.getByText('1 / 1 models · 1.2 mm PCB',{exact:true})).toBeVisible();
});

test('renders front and rear PCB layers with no component models attached', async ({ page }, testInfo) => {
  // Reserve enough drawing height for the fixed pixel thresholds below the workflow bar.
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/');
  await page.getByRole('button', { name: '3D assembly', exact: true }).click();
  await page.getByText('Routed PCB reference', { exact: true }).click();
  await page.getByLabel('Import KiCad board', { exact: true }).setInputFiles({
    name: 'board-no-models.kicad_pcb',
    mimeType: 'text/plain',
    buffer: Buffer.from(pcbWithoutModels),
  });
  await expect(page.getByText('0 / 0 models · 1.6 mm PCB', { exact: true })).toBeVisible();

  const canvas = page.locator('.wb-assembly-scene canvas');
  await expect(canvas).toBeVisible();
  await page.waitForFunction(() => {
    const canvas = document.querySelector('.wb-assembly-scene canvas');
    if (!canvas) return false;
    const fiberKey = Object.keys(canvas).find((key) => key.startsWith('__reactFiber$'));
    let fiber = fiberKey ? (canvas as unknown as Record<string, unknown>)[fiberKey] as { return?: unknown; memoizedProps?: unknown } : undefined;
    while (fiber) {
      const props = fiber.memoizedProps as { board?: { surfaces?: { text?: string }[] } } | undefined;
      const surfaces = props?.board?.surfaces;
      if (surfaces?.some((surface) => surface.text === 'FRONT')
        && surfaces.some((surface) => surface.text === 'BACK')) return true;
      fiber = fiber.return as typeof fiber;
    }
    return false;
  }, undefined, { timeout: 10_000 });
  const plate = page.getByRole('button', { name: /^(Hide|Show) Switch\ plate$/, exact: true });
  await openLayers(page);
  if (await plate.isVisible()) {
    await plate.click();
    await expect(plate).toHaveAttribute('aria-pressed', 'false');
  }
  await expect(page.getByText('Preparing 3D geometry…', { exact: true })).not.toBeVisible();
  await page.getByRole('button', { name: 'Fit', exact: true }).click();
  for (const name of ['PCB', 'Copper', 'Mask openings', 'Silkscreen']) {
    const layer = page.getByRole('button', { name: new RegExp(`^(Hide|Show) ${name}$`), exact: true });
    await expect(layer).toBeVisible();
    await layer.click();
    await expect(layer).toHaveAttribute('aria-pressed', 'false');
    await layer.click();
    await expect(layer).toHaveAttribute('aria-pressed', 'true');
  }

  for (const view of ['Top', 'Bottom', 'Isometric'] as const) {
    await page.getByRole('button', { name: view, exact: true }).click();
    await expect.poll(async () => (await countPcbPixels(canvas)).green).toBeGreaterThan(2000);
    const colors = await countPcbPixels(canvas);
    expect(colors.green, `${view} should show an opaque green PCB face`).toBeGreaterThan(2000);
    if (view !== 'Isometric') {
      await expect.poll(async () => (await countPcbPixels(canvas)).gold).toBeGreaterThan(500);
      await expect.poll(async () => (await countPcbPixels(canvas)).silkscreen).toBeGreaterThan(50);
    }
    await page.screenshot({ path: testInfo.outputPath(`pcb-${view.toLowerCase()}-no-models.png`) });
  }

  await page.getByRole('button', { name: 'Top', exact: true }).click();
  const copper = page.getByRole('button', { name: /^(Hide|Show) Copper$/, exact: true });
  const mask = page.getByRole('button', { name: /^(Hide|Show) Mask\ openings$/, exact: true });
  const silkscreen = page.getByRole('button', { name: /^(Hide|Show) Silkscreen$/, exact: true });
  await copper.click();
  await mask.click();
  await expect.poll(async () => (await countPcbPixels(canvas)).gold).toBeLessThan(100);
  await copper.click();
  await mask.click();
  await expect.poll(async () => (await countPcbPixels(canvas)).gold).toBeGreaterThan(500);
  await silkscreen.click();
  await expect.poll(async () => (await countPcbPixels(canvas)).silkscreen).toBeLessThan(20);
  await silkscreen.click();
  await expect.poll(async () => (await countPcbPixels(canvas)).silkscreen).toBeGreaterThan(50);
  const pcbVisibility = page.getByRole('button', { name: /^(Hide|Show) PCB$/, exact: true });
  await pcbVisibility.click();
  await expect.poll(async () => (await countPcbPixels(canvas)).green).toBeLessThan(100);
  await pcbVisibility.click();

  const bounds = await canvas.boundingBox();
  if (!bounds) throw new Error('Assembly canvas is not visible');
  await canvas.click({ position: { x: bounds.width * 0.35, y: bounds.height / 2 } });
  const selectedColors = await countPcbPixels(canvas);
  expect(selectedColors.green, 'Selecting the PCB should preserve its green surface').toBeGreaterThan(2000);
  expect(selectedColors.gold, 'Selecting the PCB should preserve copper colors').toBeGreaterThan(500);
  await page.screenshot({ path: testInfo.outputPath('pcb-selected-no-models.png') });
});

test('custom assembly saves member placement and places it on the board',async({page})=>{
  await page.goto('/');await navigateWorkspace(page, 'Parts');
  await page.getByRole('button',{name:'New assembly',exact:true}).click();
  const editor=page.locator('.wb-assembly-editor-layout');await editor.getByRole('textbox',{name:'Name',exact:true}).fill('Test assembly');
  await editor.getByRole('button',{name:'Add component',exact:true}).click();
  await editor.getByRole('combobox',{name:'Component',exact:true}).selectOption('ergogen:ceoloide/switch_mx');
  await editor.getByRole('button',{name:'Save assembly',exact:true}).click();await expect(editor.getByText(/Assembly saved/)).toBeVisible();
  await editor.getByRole('button',{name:'Place on selected board',exact:true}).click();
  await expect(page.getByRole('tab',{name:'Layout',exact:true})).toHaveAttribute('aria-selected','true');await expect(page.getByLabel('Complete PCB assembly preview')).toBeVisible();
});

test('applies a configured assembly to an existing matrix with undo', async ({page}) => {
  test.setTimeout(60_000);
  await page.goto('/');
  await page.getByRole('treeitem', {name:'Matrix 1 15 keys',exact:true}).click();
  const revision = Number(await page.locator('.wb-root').getAttribute('data-revision'));
  await navigateWorkspace(page, 'Parts');
  await page.getByRole('option',{name:'Choc V1 Hotswap',exact:true}).click();
  await page.getByRole('button',{name:'Customize 3D assembly',exact:true}).click();
  await page.getByRole('button',{name:'Apply to Matrix 1',exact:true}).click();
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision',String(revision + 1));
  await navigateWorkspace(page, 'Layout');
  await expect(page.locator('.wb-scene-part')).toHaveCount(30);
  await expect(page.getByRole('treeitem',{name:'Matrix 1 15 keys',exact:true})).toBeVisible();
  await page.getByRole('button',{name:'3D assembly',exact:true}).click();
  await expect(page.getByText('60 / 60 models · 1.6 mm PCB',{exact:true})).toBeVisible({timeout:45_000});
  await page.getByRole('button',{name:'Undo',exact:true}).click();
  await page.getByRole('button',{name:'2D',exact:true}).click();
  await expect(page.locator('.wb-scene-part')).toHaveCount(15);
});

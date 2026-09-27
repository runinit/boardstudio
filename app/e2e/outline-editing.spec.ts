import { expect, test, type Page } from '@playwright/test';

type Point={x:number;y:number};
async function screen(page:Page,point:Point) {
  return page.locator('.wb-canvas').evaluate((node:SVGSVGElement,p)=>{const at=new DOMPoint(p.x,-p.y).matrixTransform(node.getScreenCTM()!);return {x:at.x,y:at.y};},point);
}
async function point(page:Page,at:Point) {const p=await screen(page,at);await page.mouse.click(p.x,p.y);}
async function openOutline(page:Page) {
  await page.getByRole('button',{name:'Add object',exact:true}).click();
  await page.getByRole('button',{name:'Board outline…',exact:true}).click();
}
async function draw(page:Page,kind:string,points:Point[]) {
  await page.getByRole('button',{name:kind,exact:true}).click();
  for(const p of points) await point(page,p);
  await page.keyboard.press('Enter');
  await expect(page.getByLabel('Point 1 X')).toBeVisible();
}

test('outline points use the mm grid, drag cancellation, numeric edits, undo and reload',async({page},info)=>{
  await page.goto('/');await openOutline(page);
  await draw(page,'Draw addition',[{x:1,y:-1},{x:5,y:-1},{x:5,y:-5}]);
  await page.getByLabel('Outline point grid').selectOption('0.5');
  const handle=page.getByRole('button',{name:'Outline point 1',exact:true});
  await handle.focus();await page.keyboard.press('ArrowLeft');
  await expect(page.getByLabel('Point 1 X')).toHaveValue('0.5');
  const start=await screen(page,{x:.5,y:-1}),end=await screen(page,{x:-2.3,y:-2.2});
  await page.mouse.move(start.x,start.y);await page.mouse.down();await page.mouse.move(end.x,end.y,{steps:4});
  await expect(handle).toHaveAttribute('cx','-2.5');
  await page.keyboard.press('Escape');await page.mouse.up();
  await expect(page.getByLabel('Point 1 X')).toHaveValue('0.5');
  const restored=await screen(page,{x:.5,y:-1}),destination=await screen(page,{x:-2.3,y:-2.2});
  await page.mouse.move(restored.x,restored.y);await page.mouse.down();await page.mouse.move(destination.x,destination.y,{steps:3});await page.mouse.up();
  await expect(page.getByLabel('Point 1 X')).toHaveValue('-2.5');
  await page.getByRole('button',{name:'Undo',exact:true}).click();
  await expect(page.getByLabel('Point 1 X')).toHaveValue('0.5');
  await page.getByRole('button',{name:'Redo',exact:true}).click();
  await expect(page.getByLabel('Point 1 X')).toHaveValue('-2.5');
  await page.getByRole('button',{name:'Insert after 1',exact:true}).click();
  await expect(page.locator('.wb-outline-handle')).toHaveCount(4);
  await page.getByRole('button',{name:'Remove point 2',exact:true}).click();
  await expect(page.locator('.wb-outline-handle')).toHaveCount(3);
  await page.getByLabel('Point 1 X').fill('-3.25');await page.getByLabel('Point 1 X').blur();
  await page.getByLabel('Shape attachment').selectOption({label:'SW1'});
  await expect(page.getByLabel('Point 1 X')).toHaveValue('-3.25');
  await page.screenshot({path:info.outputPath('points-desktop.png')});
  await page.setViewportSize({width:390,height:844});
  await expect(page.getByLabel('Point 1 X')).toBeVisible();
  expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
  await page.screenshot({path:info.outputPath('points-mobile.png')});
  await page.setViewportSize({width:1280,height:720});
  await expect(page.getByLabel('Saved locally',{exact:true})).toBeVisible();await page.reload();await openOutline(page);
  await page.getByRole('button',{name:'Edit addition 1',exact:true}).click();
  await expect(page.getByLabel('Point 1 X')).toHaveValue('-3.25');
  await expect(page.getByLabel('Shape attachment')).not.toHaveValue('');
});

test('manual connection endpoints follow components and positive width persists',async({page})=>{
  await page.goto('/');await openOutline(page);
  await draw(page,'Connect points',[{x:0,y:0},{x:0,y:-10},{x:19,y:-10},{x:19,y:0}]);
  await expect(page.getByLabel('Point 1 attachment')).not.toHaveValue('');
  await page.getByRole('button',{name:'Select outline point 4',exact:true}).click();
  await expect(page.getByLabel('Point 4 attachment')).not.toHaveValue('');
  await page.getByRole('button',{name:'Select outline point 2',exact:true}).click();
  await expect(page.getByLabel('Point 2 attachment')).toHaveValue('');
  await page.getByLabel('Connection width').fill('0');await page.getByLabel('Connection width').blur();
  await expect(page.getByLabel('Connection width')).toHaveAttribute('aria-invalid','true');
  await page.getByLabel('Connection width').fill('3');await page.getByLabel('Connection width').blur();
  await page.getByRole('button',{name:'Select outline point 1',exact:true}).click();
  await page.getByLabel('Point 1 attachment').selectOption('');
  await expect(page.getByLabel('Point 1 X')).toHaveValue('0');
  await page.getByRole('button',{name:'Done',exact:true}).click();
  await expect(page.getByRole('button',{name:'Edit connection 1',exact:true})).toBeVisible();
  await expect(page.getByLabel('Saved locally',{exact:true})).toBeVisible();await page.reload();await openOutline(page);
  await page.getByRole('button',{name:'Edit connection 1',exact:true}).click();
  await expect(page.getByLabel('Connection width')).toHaveValue('3');
  await expect(page.locator('.wb-outline-handle')).toHaveCount(4);
  await page.getByRole('button',{name:'Done',exact:true}).click();await page.getByRole('button',{name:'Remove connection 1',exact:true}).click();
  await expect(page.getByRole('button',{name:'Edit connection 1',exact:true})).toHaveCount(0);
});

test('selected point is shared with the inspector and drawing can undo or cancel without changing the board',async({page})=>{
  await page.goto('/');await openOutline(page);
  const before=await page.locator('.wb-outline-shape').first().getAttribute('points');
  await page.getByRole('button',{name:'Draw addition',exact:true}).click();
  await point(page,{x:1,y:-1});await point(page,{x:5,y:-1});await point(page,{x:5,y:-5});
  await page.getByRole('button',{name:'Undo point',exact:true}).focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('status',{name:'Outline drawing'})).toContainText('2 points');
  await page.getByRole('button',{name:'Undo point',exact:true}).click();
  await expect(page.getByRole('status',{name:'Outline drawing'})).toContainText('1 point');
  await page.getByRole('button',{name:'Cancel drawing',exact:true}).click();
  await expect(page.getByRole('heading',{name:'Board outline',exact:true})).toBeVisible();
  expect(await page.locator('.wb-outline-shape').first().getAttribute('points')).toBe(before);
  await draw(page,'Draw addition',[{x:1,y:-1},{x:5,y:-1},{x:5,y:-5}]);
  await page.getByRole('button',{name:'Outline point 3',exact:true}).click();
  await expect(page.getByLabel('Point 3 X')).toBeVisible();
  await expect(page.getByLabel('Point 1 X')).toHaveCount(0);
  await page.getByRole('button',{name:'Select outline point 2',exact:true}).click();
  await expect(page.getByRole('button',{name:'Outline point 2',exact:true})).toHaveAttribute('aria-pressed','true');
  await expect(page.getByLabel('Point 2 X')).toBeVisible();
});

test('mobile outline drawing closes panels and supports undo and closing at the first point',async({page})=>{
  await page.setViewportSize({width:390,height:844});
  await page.goto('/');
  await page.getByRole('button',{name:'Objects',exact:true}).click();
  await openOutline(page);
  await expect(page.locator('#wb-objects-toggle')).toHaveAttribute('aria-expanded','false');
  await page.getByRole('button',{name:'Draw addition',exact:true}).click();
  await expect(page.getByRole('button',{name:'Inspect',exact:true})).toHaveAttribute('aria-expanded','false');
  await expect(page.getByRole('button',{name:'Close panels',exact:true})).toHaveCount(0);
  for(const at of [{x:0,y:0},{x:10,y:0},{x:10,y:10}]) await point(page,at);
  await page.keyboard.press('Control+z');
  await expect(page.getByRole('status',{name:'Outline drawing'})).toContainText('2 points');
  await point(page,{x:10,y:10});
  await point(page,{x:0,y:0});
  await expect(page.getByLabel('Point 1 X')).toBeVisible();
  await expect(page.getByRole('button',{name:'Select outline point 3',exact:true})).toBeVisible();
  expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
});

import { expect, test, type Page } from '@playwright/test';

test('outline editing exposes the same snap settings as layout', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Snap', exact: true }).click();
  await page.getByLabel('Snap increment').selectOption('0.125');
  await page.getByRole('button', { name: 'Snap', exact: true }).click();
  await page.getByRole('treeitem', { name: 'Outline Generated', exact: true }).click();
  await page.getByRole('button', { name: 'Edit perimeter points', exact: true }).click();
  await page.getByRole('button', { name: 'Snap', exact: true }).click();
  await expect(page.getByLabel('Snap increment')).toHaveValue('0.125');
  await page.getByLabel('Geometry snap', { exact: true }).uncheck();
  await page.getByLabel('Snap increment').selectOption('-0.5');
  await page.getByRole('button', { name: 'Finish editing outline points', exact: true }).click();
  await page.getByRole('button', { name: 'Snap', exact: true }).click();
  await expect(page.getByLabel('Snap increment')).toHaveValue('-0.5');
  await expect(page.getByLabel('Geometry snap', { exact: true })).not.toBeChecked();
});

async function screen(page:Page, point:{x:number;y:number}) {
  return page.locator('.wb-canvas').evaluate((node:SVGSVGElement,p)=>{const q=new DOMPoint(p.x,-p.y).matrixTransform(node.getScreenCTM()!);return {x:q.x,y:q.y};},point);
}
test('drawing and perimeter dragging show right-edge guides, snap the preview and click alike, and release with Alt',async({page},info)=>{
  await page.goto('/');
  await page.getByRole('button',{name:'Project',exact:true}).click();
  await page.locator('.wb-project-file-input').setInputFiles('../docs/design/evidence/board-outlines/reviung41-original.boardstudio');
  await expect(page.locator('.wb-scene-part')).toHaveCount(85);
  await page.getByRole('treeitem',{name:'Outline Generated',exact:true}).click();
  await page.getByRole('button',{name:'Draw addition',exact:true}).click();
  await page.getByRole('button',{name:'Snap',exact:true}).click();
  await page.getByLabel('Snap increment').selectOption('-1');
  await page.getByRole('button',{name:'Snap',exact:true}).click();
  let p=await screen(page,{x:275,y:3.03});await page.mouse.click(p.x,p.y);
  p=await screen(page,{x:286.05,y:3.13});await page.mouse.move(p.x,p.y);
  await expect(page.locator('[data-guide="Vertical alignment"]')).toHaveCount(1);
  await expect(page.locator('.wb-outline-rubber-band')).toHaveAttribute('x2','286.15');
  await expect(page.locator('.wb-outline-rubber-band')).toHaveAttribute('y2','3.03');
  await page.screenshot({path:info.outputPath('drawing-alignment.png')});
  await page.keyboard.down('Alt');
  await expect(page.locator('.wb-outline-snap-guides')).toHaveCount(0);
  await page.keyboard.up('Alt');
  await expect(page.locator('[data-guide="Vertical alignment"]')).toHaveCount(1);
  await page.mouse.click(p.x,p.y);
  await expect(page.locator('.wb-outline-draft circle').last()).toHaveAttribute('cx','286.15');
  await expect(page.locator('.wb-outline-draft circle').last()).toHaveAttribute('cy','3.03');
  await page.getByRole('button',{name:'Cancel drawing',exact:true}).click();
  await page.getByRole('button',{name:'Edit perimeter points',exact:true}).click();
  const index=await page.locator('.wb-outline-handle').evaluateAll(nodes=>nodes.findIndex(node=>node.getAttribute('cx')==='286.15'&&node.getAttribute('cy')==='3.03'));
  expect(index).toBeGreaterThanOrEqual(0);
  const handle=page.locator('.wb-outline-handle').nth(index);
  p=await screen(page,{x:286.15,y:3.03});await page.mouse.move(p.x,p.y);await page.mouse.down();
  p=await screen(page,{x:286.05,y:10});await page.mouse.move(p.x,p.y);
  await expect(page.locator('[data-guide="Vertical alignment"]')).toHaveCount(1);
  await expect(handle).toHaveAttribute('cx','286.15');
  await page.screenshot({path:info.outputPath('perimeter-alignment.png')});
  await page.keyboard.down('Alt');await expect(page.locator('.wb-outline-snap-guides')).toHaveCount(0);
  await page.keyboard.up('Alt');await page.keyboard.press('Escape');await page.mouse.up();
  await expect(page.getByRole('treeitem',{name:'Generated Active',exact:true})).toBeVisible();
});

test('a drawn addition closes a shallow recess with material rather than a new cutout',async({page})=>{
  await page.goto('/');
  await page.getByRole('button',{name:'Project',exact:true}).click();
  await page.locator('.wb-project-file-input').setInputFiles('../docs/design/evidence/board-outlines/reviung41-original.boardstudio');
  await expect(page.locator('.wb-scene-part')).toHaveCount(85);
  await page.getByRole('treeitem',{name:'Outline Generated',exact:true}).click();
  await page.getByRole('button',{name:'Draw addition',exact:true}).click();
  await page.keyboard.down('Alt');
  for(const at of [{x:222,y:14},{x:246,y:14},{x:246,y:17},{x:222,y:17}]){const p=await screen(page,at);await page.mouse.click(p.x,p.y);}
  await page.keyboard.up('Alt');await page.keyboard.press('Enter');
  await expect(page.getByLabel('Point 1 X')).toBeVisible();
  await expect(page.locator('.wb-outline-shape.is-hole')).toHaveCount(0);
  const material=await page.locator('.wb-outline-shape').evaluateAll(nodes=>{
    const p={x:235,y:13.5};
    return nodes.reduce((inside,node)=>{
      const points=(node.getAttribute('points')??'').trim().split(/\s+/).map(pair=>{const [x,y]=pair.split(',').map(Number);return{x,y};});let hit=false;
      points.forEach((a,i)=>{const b=points[(i+1)%points.length];if((a.y>p.y)!==(b.y>p.y)&&p.x<(b.x-a.x)*(p.y-a.y)/(b.y-a.y)+a.x)hit=!hit;});
      return inside!==hit;
    },false);
  });
  expect(material).toBe(true);
});

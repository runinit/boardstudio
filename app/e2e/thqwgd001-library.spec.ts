import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import type { PartDefinition } from '@boardstudio/v2-contracts';
const thqDefinitions: PartDefinition[] = JSON.parse(readFileSync(new URL('../src/parts/imported-parts.json', import.meta.url),'utf8')).parts.map((part: {definition: PartDefinition}) => part.definition);
import { demoProject } from '../src/demo';
import { openKeymapFixture } from './keymap-fixture';
import { navigateWorkspace } from './workspace-navigation';
import { openWorkspaceDocument, readWorkspaceDocument } from './workspace-storage';

const variants = ['THQWGD001 · rotation only · reversible','THQWGD001C · 2-pin tactile · reversible','THQWGD001C · 4-pin tactile · reversible'];

test('THQ variants are distinct and disclose hardware qualification before placement',async ({page}) => {
  await page.goto('/');
  await navigateWorkspace(page,'Parts');
  await page.getByRole('searchbox',{name:'Search footprints'}).fill('THQWGD001');
  const library=page.getByRole('listbox',{name:'Footprint library'});
  await expect(library.getByRole('option')).toHaveCount(7);
  for(const name of variants) {
    await library.getByRole('option',{name,exact:true}).click();
    await expect(page.getByText('Hardware readiness',{exact:true})).toBeVisible();
    await page.getByText(/Review \d+ remaining items/).click();
    await expect(page.getByText(/Published alternate pads are retained/)).toBeVisible();
    await expect(page.getByRole('link',{name:'Pinned source ↗'})).toHaveAttribute('href',/78e1c42dbebca1a9e28cf057d7d84f7eb786aa15/);
    // The shared inspector retains its expanded review list across selections.
    await page.getByText(/Review \d+ remaining items/).click();
  }
});

test('THQ matrix replacement preserves key bindings through undo and saved reload',async ({page}) => {
  await openKeymapFixture(page);
  await navigateWorkspace(page,'Keymap');
  await page.getByRole('button',{name:'Edit key SW1',exact:true}).click();
  await page.getByLabel('SW1 behavior',{exact:true}).selectOption('mod-tap');
  await expect.poll(async () => (await readWorkspaceDocument(page)).keymap?.layers[0].bindings['matrix/matrix/r0c0']?.kind).toBe('mod-tap');
  const before=await readWorkspaceDocument(page);
  const key=before.parts.find(part=>part.reference==='SW1')!;
  await navigateWorkspace(page,'Layout');
  await page.getByRole('button',{name:'Select key, row 1, column 1',exact:true}).press('Enter');
  await page.getByLabel('Key Assembly',{exact:true}).selectOption('thqwgd001:c-4pin-reversible');
  await expect.poll(async () => (await readWorkspaceDocument(page)).parts.find(part=>part.id===key.id)?.definitionId).toBe('thqwgd001:c-4pin-reversible');
  let saved=await readWorkspaceDocument(page);
  expect(saved.keymap).toEqual(before.keymap);
  expect(saved.parts.find(part=>part.id===key.id)?.pose).toEqual(key.pose);
  expect(saved.matrices[0].partIds).toContain(key.id);
  await page.getByRole('button',{name:'Undo',exact:true}).click();
  await expect(page.getByLabel('Key Assembly',{exact:true})).toHaveValue(key.definitionId);
  await page.getByRole('button',{name:'Redo',exact:true}).click();
  await expect(page.getByLabel('Key Assembly',{exact:true})).toHaveValue('thqwgd001:c-4pin-reversible');
  await page.reload();
  saved=await readWorkspaceDocument(page);
  expect(saved.keymap).toEqual(before.keymap);
  expect(saved.parts.find(part=>part.id===key.id)?.definitionId).toBe('thqwgd001:c-4pin-reversible');
});

test('THQ nominal fit finding highlights the actual overlapping bounds',async ({page}) => {
  const document=demoProject();document.id='thq-fit';
  document.definitions=structuredClone(thqDefinitions).filter(part=>part.id==='thqwgd001:c-4pin-reversible');
  document.parts=['a','b'].map((id,index)=>({id,reference:`ENC${index+1}`,definitionId:document.definitions[0].id,side:'front',pose:{at:{x:index*19.05,y:0},rotation:0}}));
  document.matrices=[];document.nets=[];document.caseBodies=[];
  document.boards[0].partIds=['a','b'];document.boards[0].netIds=[];
  document.outline=[{id:'edge',kind:'rect',center:{x:9.5,y:0},size:{x:60,y:40},radius:0,rotation:0,operation:'add'}];
  document.boards[0].outlineIds=['edge'];
  await openWorkspaceDocument(page,document);
  await page.getByRole('button',{name:/^Layout findings:/}).click();
  const finding=page.locator('.wb-findings li').filter({hasText:'nominal occupied model bounds overlap'});
  await finding.getByRole('button',{name:'Select affected geometry',exact:true}).click();
  const marker=page.locator('.wb-outline-finding.is-focused');
  await expect(marker).toBeVisible();
  const points=await marker.locator('polygon').first().getAttribute('points');
  const xs=points!.trim().split(/\s+/).map(pair=>Number(pair.split(',')[0]));
  expect(Math.max(...xs)-Math.min(...xs)).toBeCloseTo(0.369158001,3);
});

test('THQ fit findings follow MX and Choc pitches after rotation and back mounting',async ({page}) => {
  const scenarios=[
    {label:'Choc 0°',side:'front',rotation:0,at:{x:18,y:0},finding:true},
    {label:'MX 0°',side:'front',rotation:0,at:{x:19.05,y:0},finding:true},
    {label:'MX 90° horizontal clear',side:'front',rotation:90,at:{x:19.05,y:0},finding:false},
    {label:'MX 90° vertical',side:'front',rotation:90,at:{x:0,y:19.05},finding:true},
    {label:'MX 90° back vertical',side:'back',rotation:90,at:{x:0,y:19.05},finding:true},
  ] as const;
  for(const [index,scenario] of scenarios.entries()) {
    const document=demoProject();document.id=`thq-fit-${index}`;
    document.definitions=structuredClone(thqDefinitions).filter(part=>part.id==='thqwgd001:c-4pin-reversible');
    document.parts=['a','b'].map((id,partIndex)=>({id,reference:`ENC${partIndex+1}`,definitionId:document.definitions[0].id,side:scenario.side,pose:{at:partIndex===0?{x:0,y:0}:scenario.at,rotation:scenario.rotation}}));
    document.matrices=[];document.nets=[];document.caseBodies=[];
    document.boards[0].partIds=['a','b'];document.boards[0].netIds=[];
    document.outline=[{id:'edge',kind:'rect',center:{x:scenario.at.x/2,y:scenario.at.y/2},size:{x:80,y:80},radius:0,rotation:0,operation:'add'}];
    document.boards[0].outlineIds=['edge'];
    await openWorkspaceDocument(page,document);
    await page.getByRole('button',{name:/^Layout findings:/}).click();
    const fits=page.locator('.wb-findings li').filter({hasText:'nominal occupied model bounds overlap'});
    if(scenario.finding) await expect(fits,scenario.label).toHaveCount(1);
    else await expect(fits,scenario.label).toHaveCount(0);
  }
});

test('THQ C standalone placement keeps its press outside the matrix through reload',async ({page}) => {
  await openKeymapFixture(page);
  const before=await readWorkspaceDocument(page);
  await navigateWorkspace(page,'Parts');
  await page.getByRole('listbox',{name:'Footprint library'}).getByRole('option',{name:variants[1],exact:true}).click();
  await page.getByRole('button',{name:'Place component',exact:true}).click();
  await page.keyboard.press('Enter');
  await expect.poll(async () => (await readWorkspaceDocument(page)).parts.filter(part=>part.definitionId==='thqwgd001:c-2pin-reversible').length).toBe(1);
  const saved=await readWorkspaceDocument(page);
  const encoder=saved.parts.find(part=>part.definitionId==='thqwgd001:c-2pin-reversible')!;
  expect(saved.parts).toHaveLength(before.parts.length+1);
  expect(saved.matrices).toEqual(before.matrices);
  expect(saved.matrices.every(matrix=>!matrix.partIds.includes(encoder.id))).toBe(true);
  await page.reload();
  await navigateWorkspace(page,'PCB');
  await page.getByRole('treeitem',{name:new RegExp(encoder.reference)}).click();
  await expect(page.getByLabel('Press scan mode',{exact:true})).toHaveValue('direct');
  expect((await readWorkspaceDocument(page)).parts.find(part=>part.id===encoder.id)?.definitionId).toBe('thqwgd001:c-2pin-reversible');
});

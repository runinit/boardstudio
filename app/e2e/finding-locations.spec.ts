import { expect, test } from '@playwright/test';
import { readFile } from 'node:fs/promises';
import type { ProjectDoc } from '@boardstudio/v2-contracts';
import { openWorkspaceDocument } from './keymap-fixture';

test('Show outline highlights reduced corners instead of the whole perimeter', async ({ page }) => {
  const document: ProjectDoc = JSON.parse(await readFile(new URL('../../core/tests/fixtures/reviung41-outline-original.json', import.meta.url), 'utf8'));
  document.id = 'located-corners-browser';
  for (const feature of document.outline) if (feature.kind === 'part-envelope') { feature.settings.corners = 'fillet'; feature.settings.size = 2; }
  await openWorkspaceDocument(page, document);
  await page.getByRole('button', { name: /^Layout findings:/ }).click();
  const warning = page.locator('.wb-findings li').filter({ hasText: 'Corner size reduced' }).first();
  await warning.getByRole('button', { name: 'Show outline', exact: true }).click();
  const marker = page.locator('.wb-outline-finding.is-focused');
  await expect(marker).toBeVisible();
  const polygons = await marker.locator('polygon').evaluateAll(nodes => nodes.map(node => (node.getAttribute('points') ?? '').split(/\s+/).filter(Boolean).map(pair => pair.split(',').map(Number))));
  expect(polygons.length).toBeGreaterThan(0);
  for (const points of polygons) expect(Math.max(...points.map(point => point[0])) - Math.min(...points.map(point => point[0]))).toBeLessThanOrEqual(6);
});

test('Keycaps finding action retains its highlight after changing workspace', async ({ page }) => {
  const { openKeymapFixture } = await import('./keymap-fixture');
  await openKeymapFixture(page);
  await page.getByLabel('Keycap profile for matrix', { exact: true }).selectOption('dsa');
  await page.getByLabel('Keycap clearance', { exact: true }).fill('2');
  const warning = page.locator('.wb-keymap-panel .wb-findings li').filter({ hasText: 'keycap clearance' }).first();
  await warning.getByRole('button', { name: 'Select affected geometry', exact: true }).click();
  await expect(page.locator('.wb-outline-finding.is-focused')).toBeVisible();
  await expect(page.locator('.wb-outline-finding.is-focused polygon')).toHaveCount(2);
});

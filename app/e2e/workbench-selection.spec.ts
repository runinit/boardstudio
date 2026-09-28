import { chooseScope } from './selection';
import { configureMatrix } from './matrix-setup';
import { expect, test, type Page } from '@playwright/test';

async function createMatrix(page: Page): Promise<void> {
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'New project' }).click();
  await page.getByRole('button', { name: 'Back to objects', exact: true }).click();
  await configureMatrix(page);
  await page.getByRole('button', { name: 'Ghost key, row 1, column 1' }).click();
  await expect(page.locator('.wb-scene-part')).toHaveCount(60);
  await page.keyboard.press('Escape');
}

// Select logical keys independently of companion numbering.
const key = (page: Page, index: number) => page.getByRole('button', { name: /^SW\d+, switch mx,/ }).nth(index - 1);

test('selection type can be chosen before selecting a matrix key', async ({ page }) => {
  await createMatrix(page);

  await page.getByRole('button', { name: /^Select:/ }).click();
  const rowScope = page.getByRole('group', { name: 'Selection scope' }).getByRole('button', { name: 'Row', exact: true });
  await expect(rowScope).toBeEnabled();
  await rowScope.click();
  await expect(page.getByRole('button', { name: 'Select: Row', exact: true })).toBeVisible();

  await key(page, 1).click();
  await expect(page.locator('.wb-scene-part.is-selected')).toHaveCount(10);
  await chooseScope(page, 'key');
  await expect(page.locator('.wb-scene-part.is-selected')).toHaveCount(1);
  await chooseScope(page, 'row');
  await expect(page.locator('.wb-scene-part.is-selected')).toHaveCount(10);
});

test('Ctrl-click toggles keys into the selection so Delete removes the group', async ({ page }) => {
  await createMatrix(page);

  await chooseScope(page, 'key');
  await key(page, 1).click();
  await key(page, 2).click({ modifiers: ['Control'] });
  await expect(page.locator('.wb-scene-part.is-selected')).toHaveCount(2);

  await page.keyboard.press('Delete');
  await expect(page.locator('.wb-scene-part')).toHaveCount(56);
});

test('Shift-click selects the inclusive rectangle between matrix keys', async ({ page }) => {
  await createMatrix(page);

  await chooseScope(page, 'key');
  await key(page, 1).click();
  await key(page, 13).click({ modifiers: ['Shift'] });

  await expect(page.locator('.wb-scene-part.is-selected')).toHaveCount(9);
});

test('group selection stays intact through keyboard nudge and canvas drag', async ({ page }) => {
  await createMatrix(page);

  const first = key(page, 1);
  const second = key(page, 2);
  await first.click();
  await second.click({ modifiers: ['Control'] });
  await expect(page.locator('.wb-scene-part.is-selected')).toHaveCount(2);

  const xFrom = (label: string | null) => Number(label?.match(/\bX (-?[\d.]+)/u)?.[1]);
  const firstBeforeNudge = xFrom(await first.getAttribute('aria-label'));
  const secondBeforeNudge = xFrom(await second.getAttribute('aria-label'));
  await page.keyboard.press('ArrowRight');
  await expect.poll(async () => xFrom(await first.getAttribute('aria-label'))).toBeCloseTo(firstBeforeNudge + 0.1, 5);
  await expect.poll(async () => xFrom(await second.getAttribute('aria-label'))).toBeCloseTo(secondBeforeNudge + 0.1, 5);

  const firstBeforeDrag = xFrom(await first.getAttribute('aria-label'));
  const secondBeforeDrag = xFrom(await second.getAttribute('aria-label'));
  const box = await first.boundingBox();
  expect(box).not.toBeNull();
  await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);
  await page.mouse.down();
  await page.mouse.move(box!.x + box!.width / 2 + 40, box!.y + box!.height / 2, { steps: 5 });
  await page.mouse.up();

  await expect.poll(async () => xFrom(await first.getAttribute('aria-label'))).not.toBe(firstBeforeDrag);
  const firstAfterDrag = xFrom(await first.getAttribute('aria-label'));
  const secondAfterDrag = xFrom(await second.getAttribute('aria-label'));
  expect(secondAfterDrag - secondBeforeDrag).toBeCloseTo(firstAfterDrag - firstBeforeDrag, 5);
  await expect(page.locator('.wb-scene-part.is-selected')).toHaveCount(2);
  await key(page, 3).click();
  await expect(page.locator('.wb-scene-part.is-selected')).toHaveCount(1);
});

test('resizing a selected group keeps its center fixed and undoes as one edit', async ({ page }) => {
  await createMatrix(page);

  const first = key(page, 1);
  const second = key(page, 2);
  await first.click();
  await second.click({ modifiers: ['Control'] });
  const xFrom = (label: string | null) => Number(label?.match(/\bX (-?[\d.]+)/u)?.[1]);
  const firstBefore = xFrom(await first.getAttribute('aria-label'));
  const secondBefore = xFrom(await second.getAttribute('aria-label'));
  const centerBefore = (firstBefore + secondBefore) / 2;

  const width = page.getByRole('slider', { name: 'Key width' });
  await width.focus();
  await width.press('Home');
  await width.press('ArrowRight');
  await expect.poll(async () => Number(await first.locator('.wb-keycap-overlay > rect').first().getAttribute('width'))).toBeCloseTo(22.8125, 4);
  const firstAfter = xFrom(await first.getAttribute('aria-label'));
  const secondAfter = xFrom(await second.getAttribute('aria-label'));
  expect((firstAfter + secondAfter) / 2).toBeCloseTo(centerBefore, 4);

  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect.poll(async () => Number(await first.locator('.wb-keycap-overlay > rect').first().getAttribute('width'))).toBeCloseTo(18, 4);
  await expect.poll(async () => xFrom(await first.getAttribute('aria-label'))).toBeCloseTo(firstBefore, 4);
  await expect.poll(async () => xFrom(await second.getAttribute('aria-label'))).toBeCloseTo(secondBefore, 4);
});

test('resizing a key pushes its row outward and shrinking restores the neighbors', async ({ page }) => {
  await createMatrix(page);

  const selectedKey = key(page, 1);
  const rightNeighbor = key(page, 2);
  await selectedKey.click();
  const selectedBefore = await selectedKey.getAttribute('aria-label');
  const neighborBefore = await rightNeighbor.getAttribute('aria-label');
  const xFrom = (label: string | null) => Number(label?.match(/\bX (-?[\d.]+)/u)?.[1]);

  const width = page.getByRole('slider', { name: 'Key width' });
  await width.focus();
  await width.press('Home');
  await width.press('ArrowRight');
  await expect.poll(async () => xFrom(await rightNeighbor.getAttribute('aria-label'))).not.toBe(xFrom(neighborBefore));
  await expect(selectedKey).toHaveAttribute('aria-label', selectedBefore!);

  await width.press('Home');
  await expect.poll(async () => xFrom(await rightNeighbor.getAttribute('aria-label'))).toBeCloseTo(xFrom(neighborBefore), 1);
});

test('cross-axis keycap collisions show a visible warning', async ({ page }) => {
  await createMatrix(page);
  await key(page, 1).click();

  const rotation = page.getByRole('spinbutton', { name: 'Key rotation °' });
  await rotation.fill('45');
  await rotation.blur();

  const width = page.getByRole('slider', { name: 'Key width' });
  await width.focus();
  await width.press('End');
  await expect(page.locator('.wb-key-size-warning')).toBeVisible();
  // The adjacent row's switches have odd references; each key also places a diode.
  await expect(page.locator('.wb-key-size-warning')).toContainText(/SW(?:11|13|15|17|19)\b/);
});

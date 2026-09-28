import { expect, test } from '@playwright/test';

test('saved keyboards remain discoverable and reopen after creating another project', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await expect(page.getByRole('region', { name: 'Your keyboards', exact: true })).toBeVisible();
  await page.getByRole('textbox', { name: 'Project name', exact: true }).fill('My saved keyboard');
  await page.getByRole('textbox', { name: 'Project name', exact: true }).press('Enter');
  await page.getByRole('button', { name: 'New project', exact: true }).click();
  await expect(page.locator('.wb-project-name')).toHaveText('Untitled keyboard');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'Open My saved keyboard', exact: true }).click();
  await expect(page.locator('.wb-project-name')).toHaveText('My saved keyboard');
  await page.reload();
  await expect(page.locator('.wb-project-name')).toHaveText('My saved keyboard');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Open Untitled keyboard', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Open My saved keyboard', exact: true }).locator('svg rect')).not.toHaveCount(0);
});

test('demo gallery opens editable copies and keeps earlier work', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'Start Sofle v2', exact: true }).click();
  await expect(page.locator('.wb-project-name')).toHaveText('Sofle v2');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('textbox', { name: 'Project name', exact: true }).fill('My Sofle');
  await page.getByRole('textbox', { name: 'Project name', exact: true }).press('Enter');
  await page.getByRole('button', { name: 'Start Sofle v2', exact: true }).click();
  await expect(page.locator('.wb-project-name')).toHaveText('Sofle v2');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'Open My Sofle', exact: true }).click();
  await expect(page.locator('.wb-project-name')).toHaveText('My Sofle');
});

test('keyboard browser fits small screens and always offers a way back', async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 640 });
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  const library = page.locator('#wb-project-dropdown');
  const bounds = (await library.boundingBox())!;
  expect(bounds.x).toBeGreaterThanOrEqual(0);
  expect(bounds.x + bounds.width).toBeLessThanOrEqual(320);
  expect(bounds.y + bounds.height).toBeLessThanOrEqual(640);
  await page.getByRole('searchbox', { name: 'Search saved keyboards' }).fill('no matching keyboard');
  await expect(page.getByText('No keyboards match your search.')).toBeVisible();
  await page.getByRole('button', { name: 'Clear search', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Workspace settings', exact: true })).toBeInViewport();
  await page.getByRole('button', { name: 'Workspace settings', exact: true }).click();
  await page.getByRole('button', { name: 'Back to project menu', exact: true }).click();
  await page.getByRole('button', { name: 'Close project menu', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Project', exact: true })).toBeFocused();
});

test('saved keyboard deletion requires confirmation and preserves other keyboards', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', {name:'Project',exact:true}).click();
  await page.getByRole('button', {name:'Start Sofle v2',exact:true}).click();
  await expect(page.locator('.wb-project-name')).toHaveText('Sofle v2');
  await page.getByRole('button', {name:'Project',exact:true}).click();
  const remove = page.getByRole('button', {name:'Delete Starter keyboard',exact:true});
  await remove.click();
  const confirmation = page.getByRole('dialog', {name:'Delete “Starter keyboard”?',exact:true});
  await expect(confirmation).toBeVisible();
  await page.screenshot({path:'test-results/delete-keyboard-confirmation.png'});
  await expect(confirmation.getByRole('button', {name:'Cancel',exact:true})).toBeFocused();
  await confirmation.getByRole('button', {name:'Cancel',exact:true}).click();
  await expect(page.getByRole('button', {name:'Open Starter keyboard',exact:true})).toBeVisible();
  await expect(remove).toBeFocused();
  await remove.click();
  await page.keyboard.press('Escape');
  await expect(confirmation).not.toBeVisible();
  await expect(remove).toBeVisible();
  await remove.click();
  await confirmation.getByRole('button', {name:'Delete keyboard',exact:true}).click();
  await expect(confirmation).not.toBeVisible();
  await expect(page.getByRole('button', {name:'Open Starter keyboard',exact:true})).toHaveCount(0);
  await expect(page.locator('.wb-project-name')).toHaveText('Sofle v2');
  await page.reload();
  await expect(page.locator('.wb-project-name')).toHaveText('Sofle v2');
  await page.getByRole('button', {name:'Project',exact:true}).click();
  await expect(page.getByRole('button', {name:'Open Starter keyboard',exact:true})).toHaveCount(0);
  await expect(page.getByRole('button', {name:'Open Sofle v2',exact:true})).toBeVisible();
});

test('deleting the current keyboard opens a remaining saved keyboard', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', {name:'Project',exact:true}).click();
  await page.getByRole('button', {name:'Start Sofle v2',exact:true}).click();
  await expect(page.locator('.wb-project-name')).toHaveText('Sofle v2');
  await page.getByRole('button', {name:'Project',exact:true}).click();
  await page.getByRole('button', {name:'Delete Sofle v2',exact:true}).click();
  await page.getByRole('dialog').getByRole('button', {name:'Delete keyboard',exact:true}).click();
  await expect(page.locator('.wb-project-name')).toHaveText('Starter keyboard');
  await expect(page.getByRole('dialog')).not.toBeVisible();
  await expect(page.getByRole('button', {name:'Open Sofle v2',exact:true})).toHaveCount(0);
  await page.reload();
  await expect(page.locator('.wb-project-name')).toHaveText('Starter keyboard');
  await page.getByRole('button', {name:'Project',exact:true}).click();
  await expect(page.getByRole('button', {name:'Open Sofle v2',exact:true})).toHaveCount(0);
});

test('deleting the last keyboard starts a blank project without restoring the deleted one', async ({ page }) => {
  await page.setViewportSize({width:320,height:640});
  await page.goto('/');
  await page.getByRole('button', {name:'Project',exact:true}).click();
  await page.getByRole('button', {name:'Delete Starter keyboard',exact:true}).click();
  await page.getByRole('dialog').getByRole('button', {name:'Delete keyboard',exact:true}).click();
  await expect(page.getByRole('dialog')).not.toBeVisible();
  await expect(page.locator('.wb-project-name')).toHaveText('Untitled keyboard');
  await expect(page.getByRole('button', {name:'Open Starter keyboard',exact:true})).toHaveCount(0);
  await expect(page.getByRole('button', {name:'Open Untitled keyboard',exact:true})).toContainText('0 keys');
  await page.reload();
  await expect(page.locator('.wb-project-name')).toHaveText('Untitled keyboard');
  await page.getByRole('button', {name:'Project',exact:true}).click();
  await expect(page.getByRole('button', {name:'Open Starter keyboard',exact:true})).toHaveCount(0);
});

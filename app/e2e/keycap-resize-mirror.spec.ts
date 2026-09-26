import { expect, test } from '@playwright/test';

for (const [halfIndex, halfName] of ['Left', 'Right'].entries()) {
  test(`resizing the ${halfName.toLowerCase()} linked half preserves both halves and outline through undo and reload`, async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: 'Add object', exact: true }).click();
    await page.getByRole('button', { name: 'Mirrored pair…', exact: true }).click();
    await page.getByRole('spinbutton', { name: 'Rows per half', exact: true }).fill('2');
    await page.getByRole('spinbutton', { name: 'Columns per half', exact: true }).fill('3');
    await page.getByRole('button', { name: 'Preview placement', exact: true }).click();
    await page.locator('.wb-canvas').press('Enter');
    await expect(page.getByRole('treeitem', { name: 'Right half Linked', exact: true })).toBeVisible();
    await page.getByRole('treeitem', { name: `${halfName} half Linked`, exact: true }).click();
    await page.getByRole('treeitem', { name: 'Column 1 2 keys', exact: true }).nth(halfIndex).click();
    const outline = page.locator('.wb-outline-shape').first();
    const outlineBefore = await outline.getAttribute('points');
    const positions = () => page.locator('.wb-scene-part').evaluateAll(nodes => nodes.map(node => node.getAttribute('transform')));
    const positionsBefore = await positions();
    await page.getByRole('slider', { name: 'Key width' }).fill('2');
    await page.getByRole('slider', { name: 'Key width' }).blur();
    await expect(page.locator('.wb-scene-part.is-selected .wb-keycap-overlay > rect:first-child')).toHaveCount(2);
    await expect(page.locator('.wb-scene-part.is-selected .wb-keycap-overlay > rect:first-child').first()).toHaveAttribute('width', '37.1');
    await expect.poll(() => outline.getAttribute('points')).not.toBe(outlineBefore);

    const positionsAfter = await positions();
    expect(positionsAfter).not.toEqual(positionsBefore);
    const outlineAfter = await outline.getAttribute('points');
    await page.getByRole('treeitem', { name: 'Column 1 2 keys', exact: true }).nth(1 - halfIndex).click();
    await expect(page.locator('.wb-scene-part.is-selected .wb-keycap-overlay > rect:first-child').first()).toHaveAttribute('width', '37.1');

    await page.getByRole('button', { name: 'Undo', exact: true }).click();
    await expect(page.locator('.wb-scene-part.is-selected .wb-keycap-overlay > rect:first-child').first()).toHaveAttribute('width', '18');
    await expect(outline).toHaveAttribute('points', outlineBefore!);
    await expect.poll(positions).toEqual(positionsBefore);
    await page.getByRole('button', { name: 'Redo', exact: true }).click();
    await expect(page.locator('.wb-scene-part.is-selected .wb-keycap-overlay > rect:first-child').first()).toHaveAttribute('width', '37.1');
    await expect.poll(() => outline.getAttribute('points')).not.toBe(outlineBefore);

    await expect.poll(positions).toEqual(positionsAfter);
    await expect(page.locator('.wb-save-state summary')).toHaveAccessibleName('Saved locally');
    await page.reload();
    await page.getByRole('treeitem', { name: 'Right half', exact: true }).click();
    await page.getByRole('treeitem', { name: 'Right half Linked', exact: true }).click();
    await page.getByRole('button', { name: 'Expand Right half', exact: true }).last().click();
    await page.getByRole('treeitem', { name: 'Column 1 2 keys', exact: true }).last().click();
    await expect(page.locator('.wb-scene-part.is-selected .wb-keycap-overlay > rect:first-child').first()).toHaveAttribute('width', '37.1');
    await expect(outline).toHaveAttribute('points', outlineAfter!);
    await expect.poll(positions).toEqual(positionsAfter);
    await expect(page.locator('.wb-scene-part')).toHaveCount(39);
  });
}

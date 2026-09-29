// @vitest-environment jsdom
import 'fake-indexeddb/auto';
import { expect, it, vi } from 'vitest';
import { emptyProject } from '@boardstudio/v2-contracts';
import { activeProjectId, loadAsset, loadProject, resetLocalProjects, saveAsset, saveProject } from './storage';

it('keeps projects and assets when the replacement write fails', async () => {
  const existing = emptyProject('reset-existing', 'Existing');
  await saveProject(existing);
  await saveAsset('reset-asset', new Uint8Array([4, 5]));
  const put = vi.spyOn(IDBObjectStore.prototype, 'put').mockImplementationOnce(() => {
    throw new DOMException('Storage full', 'QuotaExceededError');
  });
  try {
    await expect(resetLocalProjects(emptyProject('reset-new', 'New'))).rejects.toThrow('Storage full');
  } finally {
    put.mockRestore();
  }
  expect(await loadProject(existing.id)).toEqual(existing);
  expect(Array.from((await loadAsset('reset-asset'))!)).toEqual([4, 5]);
  expect(activeProjectId('fallback')).toBe(existing.id);
});

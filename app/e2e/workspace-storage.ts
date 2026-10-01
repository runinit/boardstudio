import type { ProjectDoc } from '@boardstudio/v2-contracts';
import { expect, type Page } from '@playwright/test';

/** Persist fixtures in an initialized app, preserving other projects and assets. */
export async function saveWorkspaceDocuments(page: Page, documents: ProjectDoc[]): Promise<void> {
  if (!documents.length) throw new Error('At least one workspace document is required');
  await page.evaluate(async documents => {
    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      const request = indexedDB.open('boardstudio-v2', 1);
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
    try {
      await new Promise<void>((resolve, reject) => {
        const transaction = db.transaction('projects', 'readwrite');
        transaction.oncomplete = () => resolve();
        transaction.onabort = transaction.onerror = () => reject(transaction.error ?? new Error('Fixture transaction aborted'));
        try {
          for (const document of documents) transaction.objectStore('projects').put(document);
        } catch (cause) {
          transaction.abort();
          reject(cause);
        }
      });
      localStorage.setItem('boardstudio-v2-active-project', documents[0].id);
    } finally {
      db.close();
    }
  }, documents);
}

export async function openWorkspaceDocument(page: Page, document: ProjectDoc): Promise<void> {
  await page.goto('/');
  await expect(page.locator('.wb-root')).toBeVisible();
  await saveWorkspaceDocuments(page, [document]);
  await page.reload();
  await expect(page.locator('.wb-root')).toBeVisible();
}

/** Read committed state; workflow tests still decide when persistence is ready. */
export async function readWorkspaceDocument(page: Page, projectId?: string): Promise<ProjectDoc> {
  return page.evaluate(async projectId => {
    const id = projectId ?? localStorage.getItem('boardstudio-v2-active-project');
    if (!id) throw new Error('No active workspace document');
    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      const request = indexedDB.open('boardstudio-v2', 1);
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
    try {
      return await new Promise<ProjectDoc>((resolve, reject) => {
        const request = db.transaction('projects').objectStore('projects').get(id);
        request.onsuccess = () => request.result ? resolve(request.result) : reject(new Error(`Workspace document not found: ${id}`));
        request.onerror = () => reject(request.error);
      });
    } finally {
      db.close();
    }
  }, projectId);
}

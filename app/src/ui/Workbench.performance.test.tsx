/** @vitest-environment jsdom */
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { expect, test, vi } from 'vitest';
import { emptyProject, type SceneDelta } from '@boardstudio/v2-contracts';
import { catalogue } from '@boardstudio/v2-ergogen';
import { Workbench } from './Workbench';

vi.mock('@boardstudio/v2-ergogen', async importOriginal => {
  const original = await importOriginal<typeof import('@boardstudio/v2-ergogen')>();
  return { ...original, catalogue: vi.fn(original.catalogue) };
});

test('keeps the bundled parts catalogue across committed document updates', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  vi.stubGlobal('matchMedia', () => ({ matches: false, addEventListener() {}, removeEventListener() {} }));
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
  vi.stubGlobal('requestAnimationFrame', vi.fn(() => 1));
  vi.stubGlobal('cancelAnimationFrame', vi.fn());
  const container = document.createElement('div');
  const root = createRoot(container);
  const project = emptyProject('test', 'Test');
  const scene: SceneDelta = {
    revision: 0, transactionId: 'initial', changedIds: [], transforms: [], matrixScenes: [],
    contours: [], boardContours: [], boardReadiness: [], findings: [],
    readiness: { layout: false, outline: false, pcb: false, case: false },
  };
  const props = { document: project, scene, onEdit: vi.fn(), onUndo: vi.fn(), onRedo: vi.fn(),
    onExport: vi.fn(), compileFootprints: vi.fn(async () => []) };
  vi.mocked(catalogue).mockClear();
  try {
    await act(async () => root.render(<Workbench {...props} />));
    expect(catalogue).toHaveBeenCalledOnce();
    // Core replies deserialize the whole document, including unchanged definitions.
    const updated = { ...structuredClone(project), revision: 1 };
    await act(async () => root.render(<Workbench {...props} document={updated} scene={{ ...scene, revision: 1 }} />));
    expect(catalogue).toHaveBeenCalledOnce();
    const library = vi.mocked(catalogue).mock.results[0].value as ReturnType<typeof catalogue>;
    const bundled = library.find(item => item.id === 'ergogen:ceoloide/switch_mx')!;
    updated.definitions = [{ ...structuredClone(bundled), name: 'My custom switch' }];
    await act(async () => root.render(<Workbench {...props} document={{ ...updated, revision: 2 }} scene={{ ...scene, revision: 2 }} />));
    const parts = [...container.querySelectorAll<HTMLButtonElement>('[role="tab"]')].find(tab => tab.textContent === 'Parts')!;
    await act(async () => parts.click());
    expect(container.querySelector('[aria-label="My custom switch graphics and text"]')).not.toBeNull();
    expect(catalogue).toHaveBeenCalledOnce();
    expect(bundled.name).not.toBe('My custom switch');
  } finally {
    await act(async () => root.unmount());
    vi.unstubAllGlobals();
  }
});

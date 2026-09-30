/** @vitest-environment jsdom */
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { expect, it, vi } from 'vitest';
import { useWorkbenchNavigation } from './useWorkbenchNavigation';

it('keeps workflow cameras and the return mode within the current project session', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  const root = createRoot(document.createElement('div'));
  let navigation!: ReturnType<typeof useWorkbenchNavigation>;
  const changed = vi.fn(), beforeNavigate = vi.fn();
  function Host({ session }: { session: number }) {
    navigation = useWorkbenchNavigation({ documentId: 'same-project', session, boardId: () => 'board', beforeNavigate, onProjectChange: changed });
    return null;
  }
  try {
    await act(async () => root.render(<Host session={1} />));
    await act(async () => { navigation.setZoom(2); navigation.setPan({ x: 10, y: 20 }); });
    await act(async () => navigation.navigate('Keymap'));
    expect(navigation.zoom).toBe(1);
    await act(async () => navigation.navigate('Export'));
    expect(navigation.lastDesignMode).toBe('Keymap');
    await act(async () => navigation.navigate('Design'));
    expect(navigation.zoom).toBe(2);
    expect(navigation.pan).toEqual({ x: 10, y: 20 });
    await act(async () => root.render(<Host session={2} />));
    expect(navigation.zoom).toBe(1);
    expect(navigation.lastDesignMode).toBe('Design');
    expect(changed).toHaveBeenCalledOnce();
    await act(async () => navigation.navigate('Keymap'));
    await act(async () => navigation.navigate('Design'));
    expect(navigation.zoom).toBe(1);
    expect(beforeNavigate).toHaveBeenCalledWith('Keymap', true);
  } finally {
    await act(async () => root.unmount());
    vi.unstubAllGlobals();
  }
});

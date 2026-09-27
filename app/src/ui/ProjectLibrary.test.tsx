// @vitest-environment jsdom
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, expect, it, vi } from 'vitest';
import { emptyProject, type ProjectDoc } from '@boardstudio/v2-contracts';
import { listProjects } from '../storage';
import { ProjectLibrary } from './ProjectLibrary';

vi.mock('../storage', () => ({ listProjects: vi.fn() }));
Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
const container = document.createElement('div');
document.body.append(container);
let root = createRoot(container);
afterEach(async () => { await act(async () => root.unmount()); root = createRoot(container); vi.resetAllMocks(); });

it('keeps the project browser usable when a stored keyboard cannot be previewed', async () => {
  vi.mocked(listProjects).mockResolvedValue([
    { id: 'damaged' } as ProjectDoc,
    emptyProject('healthy', 'Healthy keyboard'),
  ]);
  await act(async () => root.render(<ProjectLibrary document={emptyProject('current', 'Current keyboard')} onOpen={() => {}} />));
  expect(container.textContent).toContain('Healthy keyboard');
  expect(container.textContent).toContain('Preview unavailable');
  expect(container.querySelector('[aria-current="true"]')?.textContent).toContain('Current keyboard');
});

it('offers retry after storage failure and keeps the current keyboard visible', async () => {
  vi.mocked(listProjects).mockRejectedValueOnce(new Error('Storage unavailable'));
  await act(async () => root.render(<ProjectLibrary document={emptyProject('current', 'Current keyboard')} onOpen={() => {}} />));
  expect(container.querySelector('[role="alert"]')?.textContent).toContain('could not be loaded');
  expect(container.textContent).toContain('Current keyboard');
  vi.mocked(listProjects).mockResolvedValueOnce([emptyProject('saved', 'Recovered keyboard')]);
  await act(async () => Array.from(container.querySelectorAll('button')).find(button => button.textContent === 'Try again')!.click());
  expect(container.querySelector('[role="alert"]')).toBeNull();
  expect(container.textContent).toContain('Recovered keyboard');
});

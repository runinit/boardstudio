/** @vitest-environment jsdom */
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, expect, test, vi } from 'vitest';
import type { CaseBody, PcbPreview, PreparedCaseAssemblyIR } from '@boardstudio/v2-contracts';
import { AssemblyScene } from './AssemblyScene';

const renderer = vi.hoisted(() => ({
  setScene: vi.fn(async () => true), setSceneBodies: vi.fn(async () => true),
  setState: vi.fn(), setHandles: vi.fn(), setDrag: vi.fn(), view: vi.fn(), dispose: vi.fn(),
}));
vi.mock('../renderClient', () => ({ createRendererCanvas: async () => renderer }));
const roots: ReturnType<typeof createRoot>[] = [];
afterEach(async () => {
  for (const root of roots.splice(0)) await act(async () => root.unmount());
  vi.clearAllMocks(); vi.unstubAllGlobals();
});
const rectangle = (low: number, high: number) => [{ x: low, y: low }, { x: high, y: low }, { x: high, y: high }, { x: low, y: high }];

async function editing() {
  vi.stubGlobal('requestAnimationFrame', vi.fn(() => 1));
  vi.stubGlobal('cancelAnimationFrame', vi.fn());
  const body: CaseBody = { id: 'plate', name: 'Plate', boardId: 'b', kind: 'plate', thickness: 3, clearance: 5,
    mounts: [{ id: 'm', kind: 'hole', holeDiameter: 2, at: { x: 5, y: 50 } }],
    openings: [{ points: rectangle(40, 60), z: 0, height: 3 }] };
  const preparedCase: PreparedCaseAssemblyIR = { revision: 1, bodies: [{ revision: 1, body,
    regions: [{ outer: rectangle(-5, 105), holes: [rectangle(70, 80)], cavities: [], gaskets: [], mounts: body.mounts! }] }] };
  const board = { models: [], thickness: 1.6, contours: [{ hole: false, points: rectangle(0, 100) }], holes: [] } as unknown as PcbPreview;
  const commit = vi.fn();
  const draft = vi.fn();
  const container = document.createElement('div'), root = createRoot(container); roots.push(root);
  const props = { board, models: [], authoredCaseBodies: [body], preparedCase, colorScheme: 'light' as const, onCaseMountChange: commit, onCaseMountDraft: draft };
  await act(async () => root.render(<AssemblyScene {...props} />));
  const edit = [...container.querySelectorAll('button')].find(button => button.textContent === 'Edit mounts')!;
  expect(edit.disabled).toBe(false);
  await act(async () => edit.click());
  const drag = renderer.setDrag.mock.lastCall![0]!;
  return { commit, draft, container, drag };
}

test('authored mounts can move into prepared case material outside the PCB', async () => {
  const { drag, commit } = await editing();
  await act(async () => { drag.start('case-mount:plate/m'); drag.move({ x: -2, y: 50 }); drag.end(false); });
  expect(commit).toHaveBeenCalledWith('plate', [expect.objectContaining({ at: { x: -2, y: 50 } })]);
});

test('retains the released handle while saving and restores it when the commit fails', async () => {
  const { drag, commit, draft, container } = await editing();
  let finish!: (saved: boolean) => void;
  commit.mockReturnValue(new Promise<boolean>(resolve => { finish = resolve; }));
  await act(async () => { drag.start('case-mount:plate/m'); drag.move({ x: -2, y: 50 }); drag.end(false); });
  expect(renderer.setHandles.mock.lastCall![0][0].at).toEqual({ x: -2, y: 50 });
  expect(draft).toHaveBeenLastCalledWith('plate', [expect.objectContaining({ at: { x: -2, y: 50 } })], 'commit');
  await act(async () => finish(false));
  expect(draft).toHaveBeenLastCalledWith('plate', null);
  expect(renderer.setHandles.mock.lastCall![0][0].at).toEqual({ x: 5, y: 50 });
  expect(container.textContent).toContain('Move could not be saved');
});

test.each([{ x: -4.5, y: 50 }, { x: 50, y: 50 }, { x: 75, y: 75 }])('case edges, openings and prepared holes block mount placement at %j', async point => {
  const { drag, commit, container } = await editing();
  await act(async () => { drag.start('case-mount:plate/m'); drag.move(point); drag.end(false); });
  expect(commit).not.toHaveBeenCalled();
  expect(container.textContent).toContain('Blocked move was not saved');
});

test('generation status changes do not redraw unchanged scene state', async () => {
  const container = document.createElement('div'), root = createRoot(container); roots.push(root);
  const props = { board: { models: [], thickness: 1.6, contours: [] } as unknown as PcbPreview, models: [], colorScheme: 'light' as const };
  await act(async () => root.render(<AssemblyScene {...props} generation={{ status: 'ready', revision: 1 }} />));
  renderer.setState.mockClear();
  await act(async () => root.render(<AssemblyScene {...props} generation={{ status: 'preparing', revision: 2 }} />));
  await act(async () => root.render(<AssemblyScene {...props} generation={{ status: 'running', revision: 2 }} />));
  expect(renderer.setState).not.toHaveBeenCalled();
});

test('acknowledges a new exact revision that reuses the already displayed draft buffers', async () => {
  const container = document.createElement('div'), root = createRoot(container); roots.push(root);
  const bodies = [{ id: 'plate', name: 'Plate', mesh: { positions: new Float32Array(9), normals: new Float32Array(9) } }];
  const props = { board: { models: [], thickness: 1.6, contours: [] } as unknown as PcbPreview, models: [], bodies, colorScheme: 'light' as const };
  await act(async () => root.render(<AssemblyScene {...props} generation={{ status: 'ready', revision: 1 }} />));
  renderer.setSceneBodies.mockClear(); renderer.setState.mockClear();
  await act(async () => root.render(<AssemblyScene {...props} generation={{ status: 'preparing', revision: 2 }} />));
  expect(renderer.setSceneBodies).not.toHaveBeenCalled();
  await act(async () => root.render(<AssemblyScene {...props} generation={{ status: 'ready', revision: 2 }} />));
  expect(renderer.setSceneBodies).toHaveBeenCalledOnce();
  expect(renderer.setState).not.toHaveBeenCalled();
});

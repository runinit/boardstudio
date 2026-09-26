import type React from 'react';
import type { Matrix, Part } from '@boardstudio/v2-contracts';
import { afterEach, expect, test, vi } from 'vitest';
import { matrixSceneAdapter } from './matrixGeometry';
import { createCanvasInteractions } from './createCanvasInteractions';

afterEach(() => vi.unstubAllGlobals());

function harness() {
  let nextFrame = 0;
  const frames = new Map<number, FrameRequestCallback>();
  vi.stubGlobal('window', {
    requestAnimationFrame: (callback: FrameRequestCallback) => { frames.set(++nextFrame, callback); return nextFrame; },
    cancelAnimationFrame: (id: number) => frames.delete(id),
    setTimeout: vi.fn(),
  });
  const captured = new Set<number>();
  const cameraShift = { x: 0, y: 0 };
  const target = {
    setPointerCapture: (id: number) => captured.add(id),
    hasPointerCapture: (id: number) => captured.has(id),
    releasePointerCapture: (id: number) => captured.delete(id),
    getBoundingClientRect: () => ({ width: 100, height: 100 }),
    getScreenCTM: () => ({ inverse: () => ({}) }),
    createSVGPoint: () => ({ x: 0, y: 0, matrixTransform() { return { x: this.x + cameraShift.x, y: this.y + cameraShift.y }; } }),
  };
  const part: Part = { id: 'key', definitionId: 'switch', reference: 'SW1', side: 'front', pose: { at: { x: 0, y: 0 }, rotation: 0 } };
  const matrix: Matrix = { id: 'matrix', rows: 1, columns: 1, pitch: { x: 19, y: 19 }, origin: { x: 0, y: 0 }, definitionId: 'switch', partIds: [part.id] };
  const bounds = { minX: 0, maxX: 100, minY: 0, maxY: 100, width: 100, height: 100 };
  const inputs: Parameters<typeof createCanvasInteractions>[0] = {
    refs: { dragRef: { current: null }, staggerDragRef: { current: null }, splayDrag: { current: null }, panDrag: { current: null }, svgRef: { current: target as unknown as SVGSVGElement }, suppressClick: { current: false }, spaceDown: { current: false }, transaction: { current: 'initial' } },
    selection: { selected: [part.id], scope: null, selectionMode: 'component', setScope: vi.fn(), setSelectionMode: vi.fn(), setSelectionAnchor: vi.fn(), setSelected: vi.fn(), setOutlineSettingsOpen: vi.fn(), setRightOpen: vi.fn(), selectScope: vi.fn() },
    layout: { matrixPartLookup: new Map(), matrixMap: new Map([[matrix.id, matrix]]), matrixCellOverrides: new Map(), matrixScenes: new Map(), parts: new Map([[part.id, part]]), definitions: new Map(), visibleParts: [part], bounds },
    snapping: { snapFraction: 0, geometrySnap: false, gapSnap: false, effectiveGap: 1, setSnapGuide: vi.fn() },
    placement: { outlineActive: false, pendingPart: null, matrixGhost: null, mode: 'Design', originPicking: false, selectedMatrix: undefined, splayAffect: 'following', snapPlacement: point => point, setPlacement: vi.fn() },
    camera: { viewBounds: bounds, setPan: vi.fn() },
    edits: { emit: vi.fn(), commitMatrix: vi.fn(), commitSplay: vi.fn() },
  };
  const event = (type: string, x = 0, y = 0, pointerId = 7) => ({ type, clientX: x, clientY: y, pointerId, button: 0, altKey: true, currentTarget: target, preventDefault: vi.fn(), stopPropagation: vi.fn() }) as unknown as React.PointerEvent<SVGGElement>;
  const canvasEvent = (type: string, x = 0, y = 0, pointerId = 7) => event(type, x, y, pointerId) as unknown as React.PointerEvent<SVGSVGElement>;
  return { inputs, part, matrix, captured, frames, cameraShift, event, canvasEvent, controller: createCanvasInteractions(inputs) };
}

test('cancelling a part drag restores its starting position without committing or leaving scheduled work', () => {
  const h = harness();
  h.controller.startDrag(h.event('pointerdown'), h.part);
  h.controller.moveDrag(h.event('pointermove', 5));
  h.controller.endDrag(h.event('pointercancel', 5));
  expect(h.inputs.edits.emit).toHaveBeenCalledWith({ kind: 'move-parts', positions: [{ id: h.part.id, at: { x: 0, y: 0 } }] }, [h.part.id], 'preview', expect.any(String));
  expect(vi.mocked(h.inputs.edits.emit).mock.calls.some(call => call[2] === 'commit')).toBe(false);
  expect(h.frames.size).toBe(0);
  expect(h.captured.size).toBe(0);
  expect(h.inputs.refs.dragRef.current).toBeNull();
});

test('cancelling stagger restores the captured matrix even after a preview rerender', () => {
  const h = harness();
  h.controller.startStagger(h.event('pointerdown'), h.matrix, 'column', 0);
  h.controller.moveCanvasPointer(h.canvasEvent('pointermove', 8));
  const preview = vi.mocked(h.inputs.edits.commitMatrix).mock.calls.at(-1)![0];
  h.inputs.layout.matrixMap.set(h.matrix.id, preview);
  const rerender = createCanvasInteractions(h.inputs);
  rerender.endCanvasPointer(h.canvasEvent('pointercancel', 8));
  expect(h.inputs.edits.commitMatrix).toHaveBeenLastCalledWith(h.matrix, 'preview', expect.any(String));
  expect(vi.mocked(h.inputs.edits.commitMatrix).mock.calls.some(call => call[1] === 'commit')).toBe(false);
  expect(h.captured.size).toBe(0);
});

test('pointerup consumes its final position and commits exactly once under the starting transaction', () => {
  const h = harness();
  h.controller.startDrag(h.event('pointerdown'), h.part);
  const id = h.inputs.refs.dragRef.current!.transactionId;
  h.controller.moveDrag(h.event('pointermove', 3));
  h.controller.endDrag(h.event('pointerup', 9));
  const commits = vi.mocked(h.inputs.edits.emit).mock.calls.filter(call => call[2] === 'commit');
  expect(commits).toEqual([[{ kind: 'move-parts', positions: [{ id: h.part.id, at: { x: 9, y: 0 } }] }, [h.part.id], 'commit', id]]);
  h.controller.endDrag(h.event('pointerup', 9));
  expect(vi.mocked(h.inputs.edits.emit).mock.calls.filter(call => call[2] === 'commit')).toHaveLength(1);
  expect(h.frames.size).toBe(0);
});

test('Escape or board changes cancel live edits and make late pointer events inert', () => {
  const h = harness();
  h.controller.startDrag(h.event('pointerdown'), h.part);
  h.controller.moveDrag(h.event('pointermove', 6));
  expect(h.controller.cancelInteractions()).toBe(true);
  const calls = vi.mocked(h.inputs.edits.emit).mock.calls.length;
  h.controller.moveDrag(h.event('pointermove', 12));
  h.controller.endDrag(h.event('pointerup', 12));
  expect(h.inputs.edits.emit).toHaveBeenCalledTimes(calls);
  expect(h.controller.cancelInteractions()).toBe(false);
  expect(h.captured.size).toBe(0);
});

test('opening a different project discards a captured edit without replaying it into that project', () => {
  const h = harness();
  h.controller.startDrag(h.event('pointerdown'), h.part);
  h.controller.moveDrag(h.event('pointermove', 6));
  h.controller.cancelInteractions('discard');
  expect(h.inputs.edits.emit).not.toHaveBeenCalled();
  expect(h.frames.size).toBe(0);
  expect(h.captured.size).toBe(0);
});

test('unrelated pointers cannot update, cancel, or replace an active drag', () => {
  const h = harness();
  h.controller.startDrag(h.event('pointerdown'), h.part);
  const active = h.inputs.refs.dragRef.current;
  h.controller.startDrag(h.event('pointerdown', 0, 0, 8), h.part);
  h.controller.moveDrag(h.event('pointermove', 12, 0, 8));
  h.controller.endDrag(h.event('pointercancel', 12, 0, 8));
  expect(h.inputs.refs.dragRef.current).toBe(active);
  expect(h.inputs.edits.emit).not.toHaveBeenCalled();
});

test('matrix key cancellation restores the full captured matrix', () => {
  const h = harness();
  h.inputs.selection.selectionMode = 'key';
  h.inputs.layout.matrixPartLookup.set(h.part.id, { matrixId: h.matrix.id, row: 0, column: 0 });
  const controller = createCanvasInteractions(h.inputs);
  controller.startDrag(h.event('pointerdown'), h.part);
  controller.moveDrag(h.event('pointermove', 5));
  controller.endDrag(h.event('pointercancel', 5));
  expect(h.inputs.edits.commitMatrix).toHaveBeenLastCalledWith(h.matrix, 'preview', expect.any(String));
  expect(h.inputs.edits.emit).not.toHaveBeenCalled();
});

test('a stagger commit keeps its own transaction across rerenders and consumes the release position', () => {
  const h = harness();
  h.controller.startStagger(h.event('pointerdown'), h.matrix, 'column', 0);
  const transaction = h.inputs.refs.staggerDragRef.current!.transactionId;
  h.controller.moveCanvasPointer(h.canvasEvent('pointermove', 3));
  h.inputs.refs.transaction.current = 'unrelated';
  createCanvasInteractions(h.inputs).endCanvasPointer(h.canvasEvent('pointerup', 9));
  expect(h.inputs.edits.commitMatrix).toHaveBeenLastCalledWith({ ...h.matrix, columnOffsets: [{ x: 9, y: 0 }] }, 'commit', transaction);
  expect(vi.mocked(h.inputs.edits.commitMatrix).mock.calls.filter(call => call[1] === 'commit')).toHaveLength(1);
});


test('splay cancellation restores its original matrix and releases capture', () => {
  const h = harness();
  h.inputs.selection.scope = { kind: 'column', matrixId: h.matrix.id, column: 0 };
  h.inputs.layout.matrixScenes.set(h.matrix.id, matrixSceneAdapter({ matrixId: h.matrix.id, cells: [], columns: [
    { column: 0, splayOrigin: { x: 0, y: 0 }, splayAngle: 0, customOrigin: false, axisX: { x: 1, y: 0 }, axisY: { x: 0, y: 1 } },
  ] }));
  const controller = createCanvasInteractions(h.inputs);
  controller.startSplay(h.event('pointerdown', 10, 0), 'angle');
  controller.moveCanvasPointer(h.canvasEvent('pointermove', 10, -10));
  expect(h.inputs.edits.commitSplay).toHaveBeenCalledWith(h.matrix, 0, { kind: 'angle', angle: 45, affect: 'following' }, 'preview', expect.any(String));
  controller.endCanvasPointer(h.canvasEvent('pointercancel', 10, -10));
  expect(h.inputs.edits.commitMatrix).toHaveBeenLastCalledWith(h.matrix, 'preview', expect.any(String));
  expect(h.inputs.refs.splayDrag.current).toBeNull();
  expect(h.captured.size).toBe(0);
});

test('cancelling a pan restores the camera without editing the document', () => {
  const h = harness();
  h.inputs.refs.panDrag.current = { pointerId: 7, startX: 0, startY: 0, pan: { x: 2, y: 3 }, width: 100, height: 100 };
  h.captured.add(7);
  h.controller.moveCanvasPointer(h.canvasEvent('pointermove', 10, 20));
  expect(h.inputs.camera.setPan).toHaveBeenLastCalledWith({ x: -8, y: 23 });
  h.controller.endCanvasPointer(h.canvasEvent('pointercancel', 10, 20));
  expect(h.inputs.camera.setPan).toHaveBeenLastCalledWith({ x: 2, y: 3 });
  expect(h.inputs.edits.emit).not.toHaveBeenCalled();
  expect(h.captured.size).toBe(0);
});


test('clicking a part does not move it when opening the inspector shifts the canvas projection', () => {
  const h = harness();
  h.controller.startDrag(h.event('pointerdown'), h.part);
  h.cameraShift.x = 10;
  h.controller.endDrag(h.event('pointerup'));
  expect(h.inputs.edits.emit).not.toHaveBeenCalled();
  expect(h.inputs.edits.commitMatrix).not.toHaveBeenCalled();
});

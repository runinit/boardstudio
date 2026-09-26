import type React from 'react';
import type { Dispatch, MutableRefObject, RefObject, SetStateAction } from 'react';
import type { EditCommand, Matrix, MatrixCell, MatrixSplayChange, Part, PartDefinition, Vec2 } from '@boardstudio/v2-contracts';
import type { Drag, StaggerDrag, SplayDrag, PanDrag, SelectionScope, Mode } from './workbenchTypes';
import type { MatrixProjection } from './matrixGeometry';
import type { PlacementState } from './placementState';
import { movePlacement } from './placementState';
import { snapOrigin, snapPart } from './placementGeometry';
import { getBounds } from './canvasBounds';
import { PITCH_MM, localMatrixDelta, makeId, pointFromEvent, snapDelta, withCell } from './workbenchGeometry';

type CellLocation = { matrixId: string; row: number; column: number; assemblyId?: string };
type Inputs = {
  refs: {
    dragRef: MutableRefObject<Drag | null>;
    staggerDragRef: MutableRefObject<StaggerDrag | null>;
    splayDrag: MutableRefObject<SplayDrag | null>;
    panDrag: MutableRefObject<PanDrag | null>;
    svgRef: RefObject<SVGSVGElement>;
    suppressClick: MutableRefObject<boolean>;
    spaceDown: MutableRefObject<boolean>;
    transaction: MutableRefObject<string>;
  };
  selection: {
    selected: string[];
    scope: SelectionScope | null;
    selectionMode: SelectionScope['kind'];
    setScope: (scope: SelectionScope) => void;
    setSelectionMode: (mode: SelectionScope['kind']) => void;
    setSelectionAnchor: (anchor: { matrixId: string; row: number; column: number }) => void;
    setSelected: (ids: string[]) => void;
    setOutlineSettingsOpen: (open: boolean) => void;
    setRightOpen: (open: boolean) => void;
    selectScope: (scope: SelectionScope) => void;
  };
  layout: {
    matrixPartLookup: Map<string, CellLocation>;
    matrixMap: Map<string, Matrix>;
    matrixCellOverrides: Map<string, Map<string, MatrixCell>>;
    matrixScenes: Map<string, MatrixProjection>;
    parts: Map<string, Part>;
    definitions: Map<string, PartDefinition>;
    visibleParts: Part[];
    bounds: ReturnType<typeof getBounds>;
  };
  snapping: {
    snapFraction: number;
    geometrySnap: boolean;
    gapSnap: boolean;
    effectiveGap: number;
    setSnapGuide: (snap: ReturnType<typeof snapPart>) => void;
  };
  placement: {
    outlineActive: boolean;
    pendingPart: PartDefinition | null;
    matrixGhost: Matrix | null;
    mode: Mode;
    originPicking: boolean;
    selectedMatrix: Matrix | undefined;
    splayAffect: 'column' | 'following';
    snapPlacement: (point: Vec2, free: boolean) => Vec2;
    setPlacement: Dispatch<SetStateAction<PlacementState>>;
  };
  camera: {
    viewBounds: ReturnType<typeof getBounds>;
    setPan: (pan: Vec2) => void;
  };
  edits: {
    emit: (operation: EditCommand['operation'], ids: string[], phase?: EditCommand['phase'], transactionId?: string) => void;
    commitMatrix: (matrix: Matrix, phase?: EditCommand['phase'], transactionId?: string) => void;
    commitSplay: (matrix: Matrix, column: number, change: MatrixSplayChange, phase?: EditCommand['phase'], transactionId?: string) => void;
  };
};

export function createCanvasInteractions({ refs, selection, layout, snapping, placement, camera, edits }: Inputs) {
  const { dragRef, staggerDragRef, splayDrag, panDrag, svgRef, suppressClick, spaceDown, transaction } = refs;
  const { selected, scope, selectionMode, setScope, setSelectionMode, setSelectionAnchor, setSelected, setOutlineSettingsOpen, setRightOpen, selectScope } = selection;
  const { matrixPartLookup, matrixMap, matrixCellOverrides, matrixScenes, parts, definitions, visibleParts, bounds } = layout;
  const { snapFraction, geometrySnap, gapSnap, effectiveGap, setSnapGuide } = snapping;
  const { outlineActive, pendingPart, matrixGhost, mode, originPicking, selectedMatrix, splayAffect, snapPlacement, setPlacement } = placement;
  const { viewBounds, setPan } = camera;
  const { emit, commitMatrix, commitSplay } = edits;

  const snapSplayOrigin = (point: Vec2, matrix: Matrix, free: boolean) => {
    const rect = svgRef.current?.getBoundingClientRect();
    const snapped = !free && geometrySnap ? snapOrigin(point, visibleParts, definitions, rect ? viewBounds.width / rect.width * 8 : 1) : undefined;
    setSnapGuide(snapped);
    return free ? point : snapped?.at ?? snapDelta(point, matrix.pitch, snapFraction);
  };
  const hasActiveInteraction = () => Boolean(dragRef.current || staggerDragRef.current || splayDrag.current || panDrag.current);

  const releasePointer = (target: Element, pointerId: number) => {
    if (target.hasPointerCapture(pointerId)) target.releasePointerCapture(pointerId);
  };

  const suppressPointerClick = () => {
    suppressClick.current = true;
    window.setTimeout(() => { suppressClick.current = false; }, 0);
  };

  const cancelInteractions = (mode: 'restore' | 'discard' = 'restore') => {
    const active = hasActiveInteraction();
    const drag = dragRef.current;
    const stagger = staggerDragRef.current;
    const splay = splayDrag.current;
    const pan = panDrag.current;
    // Clear ownership before releasing capture so a lost-capture event is inert.
    dragRef.current = null;
    staggerDragRef.current = null;
    splayDrag.current = null;
    panDrag.current = null;
    if (drag) {
      if (drag.frame !== null) window.cancelAnimationFrame(drag.frame);
      if (drag.moved && mode === 'restore') {
        if (drag.matrixStart) commitMatrix(drag.matrixStart, 'preview', drag.transactionId);
        else emit({ kind: 'move-parts', positions: drag.origins }, drag.ids, 'preview', drag.transactionId);
      }
      releasePointer(drag.target, drag.pointerId);
      if (drag.moved) suppressPointerClick();
    }
    if (stagger) {
      if (stagger.pending !== stagger.matrixStart && mode === 'restore') commitMatrix(stagger.matrixStart, 'preview', stagger.transactionId);
      releasePointer(stagger.target, stagger.pointerId);
      suppressPointerClick();
    }
    if (splay) {
      if (splay.pending && mode === 'restore') commitMatrix(splay.matrix, 'preview', splay.transactionId);
      releasePointer(splay.target, splay.pointerId);
      suppressPointerClick();
    }
    if (pan) {
      if (mode === 'restore') setPan(pan.pan);
      if (svgRef.current) releasePointer(svgRef.current, pan.pointerId);
    }
    if (active) setSnapGuide(undefined);
    return active;
  };

  const startDrag = (event: React.PointerEvent<SVGGElement>, part: Part) => {
    if (hasActiveInteraction() || event.button !== 0 || part.locked || outlineActive || pendingPart || matrixGhost) return;
    if (spaceDown.current) return;
    const pointerStart = pointFromEvent(event, svgRef.current);
    if (!pointerStart) return;
    event.stopPropagation();
    if (event.ctrlKey || event.metaKey || event.shiftKey) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    const matrixCell = matrixPartLookup.get(part.id);
    const matrix = matrixCell ? matrixMap.get(matrixCell.matrixId) : undefined;
    const activeScope: SelectionScope = matrixCell && !matrixCell.assemblyId && selectionMode !== 'component'
      ? { kind: selectionMode, matrixId: matrixCell.matrixId, row: matrixCell.row, column: matrixCell.column }
      : { kind: 'component', ...(matrixCell ?? {}), partId: part.id };
    const usesMatrixScope = Boolean(matrix && matrixCell && activeScope.matrixId === matrix.id
      && (activeScope.kind === 'matrix' || activeScope.kind === 'row' || activeScope.kind === 'column'));
    const matrixScope = usesMatrixScope && matrix && matrixCell
      ? activeScope.kind === 'matrix'
        ? { kind: 'matrix' as const, origin: matrix!.origin }
        : activeScope.kind === 'row'
          ? { kind: 'row' as const, index: matrixCell!.row, offset: matrix?.rowOffsets?.[matrixCell!.row] ?? { x: 0, y: 0 } }
          : activeScope.kind === 'column'
            ? { kind: 'column' as const, index: matrixCell!.column, offset: matrix?.columnOffsets?.[matrixCell!.column] ?? { x: 0, y: 0 } }
            : undefined
      : undefined;
    const ids = usesMatrixScope && matrix && matrixCell
      ? matrix.partIds.filter((id) => {
        const cell = matrixPartLookup.get(id);
        if (!cell) return false;
        if (matrixScope?.kind === 'row') return cell.row === matrixScope.index;
        if (matrixScope?.kind === 'column') return cell.column === matrixScope.index;
        return true;
      })
      : selected.includes(part.id) ? selected : [part.id];
    setScope(activeScope);
    setSelectionMode(activeScope.kind);
    if (!selected.includes(part.id) && matrixCell && !matrixCell.assemblyId) {
      setSelectionAnchor({ matrixId: matrixCell.matrixId, row: matrixCell.row, column: matrixCell.column });
    }
    setOutlineSettingsOpen(false);
    const origins = ids.map((id) => parts.get(id)).filter((item): item is Part => Boolean(item)).map((item) => ({ id: item.id, at: item.pose.at }));
    setSelected(ids);
    setRightOpen(true);
    const transactionId = makeId();
    transaction.current = transactionId;
    dragRef.current = {
      ids,
      origins,
      pointerStart,
      clientStart: { x: event.clientX, y: event.clientY },
      pending: origins,
      transactionId,
      pointerId: event.pointerId,
      target: event.currentTarget,
      bounds,
      frame: null,
      moved: false,
      ...(matrixScope && matrix ? { matrixScope, pendingMatrix: matrix, matrixStart: matrix } : {}),
      ...(matrixCell && matrix && ids.length === 1 ? {
        matrixCell: {
          ...matrixCell,
          offset: matrixCell.assemblyId
            ? matrixCellOverrides.get(matrix.id)?.get(`${matrixCell.row}:${matrixCell.column}`)?.assemblies?.find((assembly) => assembly.id === matrixCell.assemblyId)?.offset ?? { x: 0, y: 0 }
            : matrixCellOverrides.get(matrix.id)?.get(`${matrixCell.row}:${matrixCell.column}`)?.offset ?? { x: 0, y: 0 },
        },
        pendingMatrix: matrix,
        matrixStart: matrix,
      } : {}),
    };
  };

  const moveDrag = (event: React.PointerEvent<SVGGElement>) => {
    const drag = dragRef.current;
    if (!drag || event.pointerId !== drag.pointerId) return;
    const point = pointFromEvent(event, svgRef.current);
    if (!point) return;
    event.preventDefault();
    event.stopPropagation();
    const dx = point.x - drag.pointerStart.x;
    const dy = point.y - drag.pointerStart.y;
    if (!drag.moved && dx === 0 && dy === 0) return;
    if (drag.matrixScope && drag.pendingMatrix) {
      const matrix = drag.pendingMatrix;
      const pointerDelta = { x: dx, y: dy };
      const local = drag.matrixScope.kind === 'matrix' ? pointerDelta : localMatrixDelta(matrix, pointerDelta, drag.matrixScope.kind === 'column' ? drag.matrixScope.index : 0);
      const delta = event.altKey ? local : snapDelta(local, matrix.pitch, snapFraction);
      if (drag.matrixScope.kind === 'matrix') {
        drag.pendingMatrix = { ...matrix, origin: { x: drag.matrixScope.origin.x + delta.x, y: drag.matrixScope.origin.y + delta.y } };
      } else {
        const offsets = drag.matrixScope.kind === 'row' ? [...(matrix.rowOffsets ?? [])] : [...(matrix.columnOffsets ?? [])];
        while (offsets.length <= drag.matrixScope.index) offsets.push({ x: 0, y: 0 });
        offsets[drag.matrixScope.index] = { x: drag.matrixScope.offset.x + delta.x, y: drag.matrixScope.offset.y + delta.y };
        drag.pendingMatrix = drag.matrixScope.kind === 'row'
          ? { ...matrix, rowOffsets: offsets }
          : { ...matrix, columnOffsets: offsets };
      }
      drag.moved = true;
      if (drag.frame !== null) return;
      drag.frame = window.requestAnimationFrame(() => {
        drag.frame = null;
        if (drag.pendingMatrix) commitMatrix(drag.pendingMatrix, 'preview', drag.transactionId);
      });
      return;
    }
    if (drag.matrixCell && drag.pendingMatrix) {
      const matrix = drag.pendingMatrix;
      const local = localMatrixDelta(matrix, { x: dx, y: dy }, drag.matrixCell.column);
      const moved = event.altKey ? local : snapDelta(local, matrix.pitch, snapFraction);
      const offset = { x: drag.matrixCell.offset.x + moved.x, y: drag.matrixCell.offset.y + moved.y };
      const currentCell = matrixCellOverrides.get(matrix.id)?.get(`${drag.matrixCell.row}:${drag.matrixCell.column}`);
      drag.pendingMatrix = drag.matrixCell.assemblyId && drag.matrixCell.assemblyId !== 'diode'
        ? withCell(matrix, drag.matrixCell.row, drag.matrixCell.column, {
          assemblies: (currentCell?.assemblies ?? []).map((assembly) => assembly.id === drag.matrixCell!.assemblyId ? { ...assembly, offset } : assembly),
        })
        : withCell(matrix, drag.matrixCell.row, drag.matrixCell.column, { offset });
      drag.moved = true;
      if (drag.frame !== null) return;
      drag.frame = window.requestAnimationFrame(() => {
        drag.frame = null;
        if (drag.pendingMatrix) commitMatrix(drag.pendingMatrix, 'preview', drag.transactionId);
      });
      return;
    }
    drag.pending = drag.origins.map(({ id, at }) => ({ id, at: { x: at.x + dx, y: at.y + dy } }));
    const keyUnit = (scope?.matrixId ? matrixMap.get(scope.matrixId)?.pitch : undefined) ?? { x: PITCH_MM, y: PITCH_MM };
    if (!event.altKey) {
      const snapped = snapDelta({ x: dx, y: dy }, keyUnit, snapFraction);
      drag.pending = drag.origins.map(({ id, at }) => ({ id, at: { x: at.x + snapped.x, y: at.y + snapped.y } }));
    }
    if (!event.altKey && geometrySnap && drag.origins.length === 1) {
      const original = parts.get(drag.origins[0].id);
      if (original) {
        const moving = { ...original, pose: { ...original.pose, at: { x: drag.origins[0].at.x + dx, y: drag.origins[0].at.y + dy } } };
        const snap = snapPart(moving, visibleParts.filter((part) => !drag.ids.includes(part.id)), definitions, 2, gapSnap ? effectiveGap : null);
        setSnapGuide(snap);
        if (snap) drag.pending = [{ id: original.id, at: snap.at }];
      }
    } else setSnapGuide(undefined);
    drag.moved = true;
    if (drag.frame !== null) return;
    drag.frame = window.requestAnimationFrame(() => {
      drag.frame = null;
      emit({ kind: 'move-parts', positions: drag.pending }, drag.ids, 'preview', drag.transactionId);
    });
  };

  const endDrag = (event: React.PointerEvent<SVGGElement>) => {
    const drag = dragRef.current;
    if (!drag || event.pointerId !== drag.pointerId) return;
    event.stopPropagation();
    if (event.type !== 'pointerup') {
      cancelInteractions();
      return;
    }
    // A panel may resize the canvas on selection; a stationary click is not a drag.
    if (drag.moved || event.clientX !== drag.clientStart.x || event.clientY !== drag.clientStart.y) moveDrag(event);
    setSnapGuide(undefined);
    if (drag.frame !== null) window.cancelAnimationFrame(drag.frame);
    dragRef.current = null;
    if (drag.moved) {
      if (drag.pendingMatrix) {
        commitMatrix(drag.pendingMatrix, 'preview', drag.transactionId);
        commitMatrix(drag.pendingMatrix, 'commit', drag.transactionId);
      } else {
        emit({ kind: 'move-parts', positions: drag.pending }, drag.ids, 'preview', drag.transactionId);
        emit({ kind: 'move-parts', positions: drag.pending }, drag.ids, 'commit', drag.transactionId);
      }
      suppressPointerClick();
    }
    releasePointer(drag.target, drag.pointerId);
  };

  const startStagger = (event: React.PointerEvent<SVGElement>, matrix: Matrix, axis: 'row' | 'column', index: number) => {
    if (event.button !== 0 || hasActiveInteraction()) return;
    const rect = svgRef.current?.getBoundingClientRect();
    if (!rect) return;
    event.preventDefault();
    event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    const values = axis === 'row' ? matrix.rowOffsets : matrix.columnOffsets;
    const offset = values?.[index] ?? { x: 0, y: 0 };
    staggerDragRef.current = {
      matrixStart: matrix,
      transactionId: makeId(),
      axis,
      index,
      startClient: { x: event.clientX, y: event.clientY },
      worldPerPixel: { x: viewBounds.width / rect.width, y: viewBounds.height / rect.height },
      offset,
      pointerId: event.pointerId,
      target: event.currentTarget,
      pending: matrix,
    };
    selectScope(axis === 'row'
      ? { kind: 'row', matrixId: matrix.id, row: index }
      : { kind: 'column', matrixId: matrix.id, column: index });
  };

  const startSplay = (event: React.PointerEvent<SVGGElement>, kind: 'origin' | 'angle') => {
    const matrix = scope?.matrixId ? matrixMap.get(scope.matrixId) : undefined;
    if (!matrix || scope?.column === undefined || event.button !== 0 || hasActiveInteraction()) return;
    const point = pointFromEvent(event, svgRef.current);
    if (!point) return;
    const origin = matrixScenes.get(matrix.id)?.basis(scope.column)?.splayOrigin;
    if (!origin) return;
    event.preventDefault(); event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    splayDrag.current = { matrix, origin, column: scope.column, kind, pointerId: event.pointerId,
      startAngle: Math.atan2(point.y - origin.y, point.x - origin.x), target: event.currentTarget, transactionId: makeId(), bounds };
  };

  const moveCanvasPointer = (event: React.PointerEvent<SVGSVGElement>) => {
    const splay = splayDrag.current;
    if (splay && splay.pointerId === event.pointerId) {
      const point = pointFromEvent(event, svgRef.current);
      if (!point) return;
      if (splay.kind === 'origin') splay.pending = { kind: 'origin', world: snapSplayOrigin(point, splay.matrix, event.altKey) };
      else {
        const origin = splay.origin;
        const angle = Math.atan2(point.y - origin.y, point.x - origin.x) - splay.startAngle;
        const delta = Math.atan2(Math.sin(angle), Math.cos(angle)) * 180 / Math.PI * (splay.matrix.mirror === 'x' || splay.matrix.mirror === 'y' ? -1 : 1);
        const value = (splay.matrix.columnSplays?.[splay.column] ?? 0) + delta;
        splay.pending = { kind: 'angle', angle: event.altKey ? value : Math.round(value), affect: splayAffect };
      }
      commitSplay(splay.matrix, splay.column, splay.pending, 'preview', splay.transactionId);
      event.preventDefault(); return;
    }
    if (originPicking && selectedMatrix) {
      const point = pointFromEvent(event, svgRef.current);
      if (point) snapSplayOrigin(point, selectedMatrix, event.altKey);
    }
    if (pendingPart) {
      const point = pointFromEvent(event, svgRef.current);
      if (point) setPlacement((current) => movePlacement(current, snapPlacement(point, event.altKey)));
    }
    if (matrixGhost && mode === 'Design' && !panDrag.current) {
      const point = pointFromEvent(event, svgRef.current);
      if (point) setPlacement((current) => movePlacement(current, point));
    }
    const panState = panDrag.current;
    if (panState && event.pointerId === panState.pointerId) {
      const rect = svgRef.current?.getBoundingClientRect();
      if (!rect) return;
      const dx = (event.clientX - panState.startX) / rect.width * panState.width;
      const dy = -(event.clientY - panState.startY) / rect.height * panState.height;
      setPan({ x: panState.pan.x - dx, y: panState.pan.y - dy });
      event.preventDefault();
      return;
    }
    const drag = staggerDragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    const matrix = drag.matrixStart;
    const local = localMatrixDelta(matrix, {
      x: (event.clientX - drag.startClient.x) * drag.worldPerPixel.x,
      y: -(event.clientY - drag.startClient.y) * drag.worldPerPixel.y,
    }, drag.axis === 'column' ? drag.index : 0);
    const delta = event.altKey ? local : snapDelta(local, matrix.pitch, snapFraction);
    if (drag.pending === drag.matrixStart && delta.x === 0 && delta.y === 0) return;
    const offset = { x: drag.offset.x + delta.x, y: drag.offset.y + delta.y };
    const values = drag.axis === 'row' ? [...(matrix.rowOffsets ?? [])] : [...(matrix.columnOffsets ?? [])];
    while (values.length <= drag.index) values.push({ x: 0, y: 0 });
    values[drag.index] = offset;
    drag.pending = drag.axis === 'row' ? { ...matrix, rowOffsets: values } : { ...matrix, columnOffsets: values };
    commitMatrix(drag.pending, 'preview', drag.transactionId);
    event.preventDefault();
  };

  const endCanvasPointer = (event: React.PointerEvent<SVGSVGElement>) => {
    const splay = splayDrag.current;
    const pan = panDrag.current;
    const stagger = staggerDragRef.current;
    if (![splay, pan, stagger].some(drag => drag?.pointerId === event.pointerId)) return;
    if (event.type !== 'pointerup') {
      cancelInteractions();
      return;
    }
    moveCanvasPointer(event);
    setSnapGuide(undefined);
    if (splay) {
      splayDrag.current = null;
      if (splay.pending) commitSplay(splay.matrix, splay.column, splay.pending, 'commit', splay.transactionId);
      releasePointer(splay.target, splay.pointerId);
      suppressPointerClick();
    } else if (pan) {
      panDrag.current = null;
      if (svgRef.current) releasePointer(svgRef.current, pan.pointerId);
    } else if (stagger) {
      staggerDragRef.current = null;
      if (stagger.pending !== stagger.matrixStart) commitMatrix(stagger.pending, 'commit', stagger.transactionId);
      releasePointer(stagger.target, stagger.pointerId);
      suppressPointerClick();
    }
  };

  return { startDrag, moveDrag, endDrag, startStagger, startSplay, moveCanvasPointer, endCanvasPointer, snapSplayOrigin, cancelInteractions };
}

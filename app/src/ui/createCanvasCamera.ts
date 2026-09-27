import type { Pose2 } from '@boardstudio/v2-contracts';
import type { Dispatch, MutableRefObject, RefObject, SetStateAction } from 'react';
import React from 'react';
import type {
  Matrix,
  Part,
  PartDefinition,
  SceneDelta,
  Vec2
} from '../../../contracts/src/index';
import { fitCamera, getBounds } from './canvasBounds';
import type { MatrixProjection } from './matrixGeometry';
import { Drag, StaggerDrag, SplayDrag, PanDrag } from './workbenchTypes';

type Inputs = {
  dragRef: MutableRefObject<Drag | null>;
  splayDrag: MutableRefObject<SplayDrag | null>;
  staggerDragRef: MutableRefObject<StaggerDrag | null>;
  poses: Map<string, Pose2>;
  matrixScenes: Map<string, MatrixProjection>;
  definitions: Map<string, PartDefinition>;
  keyEnvelopes: Map<string, Vec2>;
  svgRef: RefObject<SVGSVGElement | null>;
  bounds: ReturnType<typeof getBounds>;
  canvasSize: { width: number; height: number; };
  setZoom: Dispatch<SetStateAction<number>>;
  setPan: Dispatch<SetStateAction<Vec2>>;
  spaceDown: MutableRefObject<boolean>;
  panDrag: MutableRefObject<PanDrag | null>;
  pan: Vec2;
  viewBounds: { minX: number; maxX: number; minY: number; maxY: number; width: number; height: number; }
};

export function createCanvasCamera({ dragRef, splayDrag, staggerDragRef, poses, matrixScenes, definitions, keyEnvelopes, svgRef, bounds, canvasSize, setZoom, setPan, spaceDown, panDrag, pan, viewBounds }: Inputs) {
  const fitParts = (targetParts: Part[], contours: SceneDelta['contours'] = [], matrices: Matrix[] = []) => {
    if (dragRef.current || splayDrag.current || staggerDragRef.current) return;
    const target = getBounds(poses, targetParts, contours, matrices, matrixScenes, definitions, keyEnvelopes);
    const toolbarHeight = svgRef.current?.closest('.wb-canvas-column')?.querySelector('.wb-canvas-toolbar')?.getBoundingClientRect().height ?? 48;
    const layersHeight = 0;
    const camera = fitCamera(bounds, target, canvasSize.width, canvasSize.height, toolbarHeight + 24, layersHeight + 24);
    setZoom(camera.zoom); setPan(camera.pan);
  };

  const startCanvasPan = (event: React.PointerEvent<SVGSVGElement>) => {
    if (event.button !== 1 && !spaceDown.current) return;
    const rect = svgRef.current?.getBoundingClientRect();
    if (!rect) return;
    event.preventDefault();
    svgRef.current?.setPointerCapture(event.pointerId);
    panDrag.current = { pointerId: event.pointerId, startX: event.clientX, startY: event.clientY, pan, width: viewBounds.width, height: viewBounds.height };
  };

  const zoomAt = (clientX: number, clientY: number, nextZoom: number) => {
    const svg = svgRef.current;
    const rect = svg?.getBoundingClientRect();
    if (!svg || !rect) {
      setZoom(nextZoom);
      return;
    }
    const fx = (clientX - rect.left) / rect.width;
    const fy = (clientY - rect.top) / rect.height;
    const worldX = viewBounds.minX + fx * viewBounds.width;
    const worldY = viewBounds.maxY - fy * viewBounds.height;
    const nextWidth = bounds.width / nextZoom;
    const nextHeight = bounds.height / nextZoom;
    const centerX = (bounds.minX + bounds.maxX) / 2;
    const centerY = (bounds.minY + bounds.maxY) / 2;
    setZoom(nextZoom);
    setPan({
      x: worldX - centerX - (fx - 0.5) * nextWidth,
      y: worldY - centerY - (0.5 - fy) * nextHeight,
    });
  };
  return { fitParts, startCanvasPan, zoomAt };
}

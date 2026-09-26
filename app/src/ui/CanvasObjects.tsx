import React, { memo } from 'react';
import type {
  Matrix,
  Part,
  PartDefinition,
  Vec2
} from '../../../contracts/src/index';
import { componentPoseSvgTransform } from './componentPreview';
import { type MatrixProjection } from './matrixGeometry';
import { KeycapOverlay, SceneFootprint } from './WorkbenchLayers';
import { SceneHandlers, SelectionScope } from './workbenchTypes';

export const MatrixGhost = memo(({ matrix, scope, onSelect, onStagger, ghost = false, parts = new Map(), definitions = new Map(), projection }: {
  matrix: Matrix;
  scope: SelectionScope | null;
  ghost?: boolean;
  parts?: Map<string, Part>;
  definitions?: Map<string, PartDefinition>;
  projection?: MatrixProjection;
  onSelect: (row: number, column: number) => void;
  onStagger: (event: React.PointerEvent<SVGElement>, axis: 'row' | 'column', index: number) => void;
}) => {
  if (!projection?.scene) return null;
  const keyWidth = Math.max(1, matrix.pitch.x - (matrix.edgeGap?.x ?? 1));
  const keyHeight = Math.max(1, matrix.pitch.y - (matrix.edgeGap?.y ?? 1));
  return <g className={`wb-matrix-ghost ${ghost ? 'is-placement-preview' : ''}`} aria-label={`Matrix ${matrix.rows} by ${matrix.columns}`}>
    {projection?.scene?.cells.map(({ row, column, enabled, pose, memberId }) => {
      if (!enabled) return null;
      const center = pose.at;
      const part = parts.get(memberId ?? '');
      const keycap = part?.keycap ?? definitions.get(part?.definitionId ?? matrix.definitionId)?.keycap;
      const selected = scope?.matrixId === matrix.id && scope.row === row && scope.column === column && ['key', 'component'].includes(scope.kind);
      const transform = `translate(${center.x} ${center.y}) rotate(${pose.rotation})`;
      const label = `${ghost ? 'Ghost' : 'Select'} key, row ${row + 1}, column ${column + 1}`;
      return <rect
        key={`${row}:${column}`}
        className={`wb-matrix-cell is-enabled ${selected ? 'is-selected' : ''}`}
        x={-(keycap?.x ?? keyWidth) / 2}
        y={-(keycap?.y ?? keyHeight) / 2}
        width={keycap?.x ?? keyWidth}
        height={keycap?.y ?? keyHeight}
        rx="0.9"
        transform={transform}
        role="button"
        tabIndex={0}
        aria-label={label}
        aria-pressed={selected}
        onClick={(event) => { if (ghost) return; event.stopPropagation(); onSelect(row, column); }}
        onKeyDown={(event) => {
          if (event.key !== 'Enter' && event.key !== ' ') return;
          event.preventDefault();
          if (ghost) return;
          onSelect(row, column);
        }}
      />;
    })}
    {projection.scene.cells.filter((cell) => cell.column === 0).map(({ row }) => {
      const point = projection?.pose(row, 0)?.at;
      if (!point) return null;
      const basis = projection?.basis(0);
      if (!basis) return null;
      const offset = { x: -matrix.pitch.x * 0.9 * basis.axisX.x, y: -matrix.pitch.x * 0.9 * basis.axisX.y };
      return <g key={`row-handle-${row}`} className="wb-stagger-handle is-row" transform={`translate(${point.x + offset.x} ${point.y + offset.y})`}>
        <rect width="1.2" height="2.8" x="-0.6" y="-1.4" rx="0.4" aria-label={`Stagger row ${row + 1}`} onPointerDown={(event) => onStagger(event, 'row', row)} />
      </g>;
    })}
    {projection.scene.columns.map(({ column }) => {
      const point = projection?.pose(0, column)?.at;
      if (!point) return null;
      const basis = projection?.basis(column);
      if (!basis) return null;
      const offset = { x: -matrix.pitch.y * 0.9 * basis.axisY.x, y: -matrix.pitch.y * 0.9 * basis.axisY.y };
      return <g key={`column-handle-${column}`} className="wb-stagger-handle is-column" transform={`translate(${point.x + offset.x} ${point.y + offset.y})`}>
        <rect width="2.8" height="1.2" x="-1.4" y="-0.6" rx="0.4" aria-label={`Stagger column ${column + 1}`} onPointerDown={(event) => onStagger(event, 'column', column)} />
      </g>;
    })}
  </g>;
});

export const ScenePart = memo(({ part, definition, active, constrained, handlers, hiddenLayers, pcb, footprints, keycap }: {
  part: Part;
  definition?: PartDefinition;
  active: boolean;
  constrained: boolean;
  handlers: React.MutableRefObject<SceneHandlers | null>;
  hiddenLayers: ReadonlySet<string>;
  pcb: boolean;
  footprints: boolean;
  keycap?: Vec2;
}) => <g className={`wb-scene-part ${pcb ? 'is-pcb' : ''} ${active ? 'is-selected' : ''} ${constrained ? 'is-constrained' : ''}`} role="button" tabIndex={0}
  aria-label={`${part.reference}, ${definition?.name ?? 'part'}${constrained ? ', constrained target' : ''}, X ${part.pose.at.x} Y ${part.pose.at.y}`}
  aria-pressed={active}
  aria-description="Arrow keys move 0.1 mm; Shift+Arrow moves 1 mm. Delete removes the selection."
  onPointerEnter={() => handlers.current?.hoverPart(part.id)}
  onPointerLeave={() => handlers.current?.hoverPart(null)}
  onPointerDown={(event) => handlers.current?.startDrag(event, part)}
  onPointerMove={(event) => handlers.current?.moveDrag(event)}
  onPointerUp={(event) => handlers.current?.endDrag(event)}
  onPointerCancel={(event) => handlers.current?.endDrag(event)}
  onLostPointerCapture={(event) => handlers.current?.endDrag(event)}
  onClick={(event) => { event.stopPropagation(); handlers.current?.choosePart(part.id, { additive: event.ctrlKey || event.metaKey, range: event.shiftKey }); }}
  onKeyDown={(event) => {
    if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); handlers.current?.choosePart(part.id, { additive: event.ctrlKey || event.metaKey, range: event.shiftKey }); }
    handlers.current?.nudgePart(event, part);
  }}
  transform={componentPoseSvgTransform(part.pose.at, part.pose.rotation, part.side)}>
    <polygon points={(definition?.courtyard ?? [{ x: -4, y: -4 }, { x: 4, y: -4 }, { x: 4, y: 4 }, { x: -4, y: 4 }]).map((point) => `${point.x},${point.y}`).join(' ')} className={`wb-part-courtyard ${pcb && hiddenLayers.has('Courtyards') ? 'is-hidden' : ''}`} />
    {definition && footprints && <SceneFootprint definition={definition} part={part} hidden={hiddenLayers} />}
    {!pcb && !hiddenLayers.has('Keycaps') && keycap && <KeycapOverlay size={keycap} />}

    {constrained && <circle r="8.1" className="wb-part-constrained" aria-hidden="true" />}
    {!hiddenLayers.has('References') && (pcb || !keycap || hiddenLayers.has('Keycaps')) && <text x="0" y="-5.2" transform="scale(1,-1)" className="wb-part-reference">{part.reference}</text>}

  </g>, (previous, next) => previous.part.id === next.part.id
    && previous.part.reference === next.part.reference
    && previous.part.definitionId === next.part.definitionId
    && previous.part.side === next.part.side
    && previous.part.locked === next.part.locked
    && previous.part.pose.at.x === next.part.pose.at.x
    && previous.part.pose.at.y === next.part.pose.at.y
    && previous.part.pose.rotation === next.part.pose.rotation
    && previous.part.keycap === next.part.keycap
    && previous.part.properties === next.part.properties
    && previous.definition === next.definition
    && previous.active === next.active
    && previous.constrained === next.constrained
    && previous.hiddenLayers === next.hiddenLayers
    && previous.pcb === next.pcb
    && previous.footprints === next.footprints
    && previous.keycap?.x === next.keycap?.x && previous.keycap?.y === next.keycap?.y);

export const SplayHandles = ({ handleScale = 1, originOnly, matrix, column, parts, projection, onStart, onOrigin, onAngle }: {
  handleScale?: number; originOnly?: boolean; matrix: Matrix; column: number; parts: Part[]; projection?: MatrixProjection;
  onStart: (event: React.PointerEvent<SVGGElement>, kind: 'origin' | 'angle') => void;
  onOrigin: (point: Vec2) => void; onAngle: (angle: number) => void;
}) => {
  const basis = projection?.basis(column);
  if (!basis) return null;
  const origin = basis.splayOrigin;
  const x = parts.length ? parts.reduce((sum, part) => sum + part.pose.at.x, 0) / parts.length : origin.x;
  const y = parts.length ? Math.max(...parts.map((part) => part.pose.at.y)) + matrix.pitch.y * .9 : origin.y + 20;
  return <g className="wb-splay-handles" onClick={(event) => event.stopPropagation()}>
    {!originOnly && <path className="wb-splay-guide" d={`M${origin.x} ${origin.y} L${x} ${y}`} />}
    <g transform={`translate(${origin.x} ${origin.y}) scale(${handleScale})`} role="button" tabIndex={0} aria-label="Move splay origin" onPointerDown={(event) => onStart(event, 'origin')} onKeyDown={(event) => {
      if (!event.key.startsWith('Arrow')) return;
      event.preventDefault(); event.stopPropagation(); const step = event.shiftKey ? 1 : .1;
      onOrigin({ x: origin.x + (event.key === 'ArrowRight' ? step : event.key === 'ArrowLeft' ? -step : 0), y: origin.y + (event.key === 'ArrowUp' ? step : event.key === 'ArrowDown' ? -step : 0) });
    }}>
      <circle className="wb-handle-target" r="4" /><circle r="2" /><path d="M-3.5 0h7M0-3.5v7" /><title>Move splay origin</title>
    </g>
    {!originOnly && <g transform={`translate(${x} ${y}) scale(${handleScale})`} role="button" tabIndex={0} aria-label="Drag to splay" onPointerDown={(event) => onStart(event, 'angle')} onKeyDown={(event) => {
      if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
      event.preventDefault(); event.stopPropagation(); onAngle(basis.splayAngle + (event.key === 'ArrowRight' ? 1 : -1) * (event.shiftKey ? 5 : 1));
    }}>
      <rect className="wb-handle-target" x="-11" y="-5" width="22" height="10" /><path d="M-9 0Q0 6 9 0M-9 0l1 3M-9 0l3-.5M9 0l-1 3M9 0l-3-.5" /><title>Drag to splay</title>
    </g>}
  </g>;
};

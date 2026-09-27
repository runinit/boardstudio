import { useEffect, useRef, useState } from 'react';
import type { Vec2 } from '@boardstudio/v2-contracts';
import { snapOutline, type OutlineDrawMode } from './outlineEditing';
import { pointFromEvent } from './workbenchGeometry';

export function OutlineDraftPreview({ points, mode, grid, scale }: { points: Vec2[]; mode: OutlineDrawMode; grid: number; scale: number }) {
  const group = useRef<SVGGElement>(null);
  const [cursor, setCursor] = useState<Vec2|null>(null);
  useEffect(() => {
    const svg = group.current?.ownerSVGElement;
    if (!svg) return;
    const move = (event: PointerEvent) => {
      const at = pointFromEvent(event, svg);
      setCursor(at ? event.altKey ? at : snapOutline(at, grid) : null);
    };
    const leave = () => setCursor(null);
    svg.addEventListener('pointermove', move);
    svg.addEventListener('pointerleave', leave);
    return () => { svg.removeEventListener('pointermove', move); svg.removeEventListener('pointerleave', leave); };
  }, [grid]);
  const preview = [...points, ...(cursor ? [cursor] : [])];
  return <g ref={group} className={`wb-outline-draft is-${mode}`}>
    {mode !== 'connect' && preview.length >= 3 && <polygon className="wb-outline-draft-fill" points={preview.map(point => `${point.x},${point.y}`).join(' ')} />}
    <polyline points={points.map(point => `${point.x},${point.y}`).join(' ')} />
    {cursor && points.length > 0 && <line className="wb-outline-rubber-band" x1={points.at(-1)!.x} y1={points.at(-1)!.y} x2={cursor.x} y2={cursor.y} />}
    {points.map((point, index) => <g key={index}><circle cx={point.x} cy={point.y} r={scale * (index === 0 ? 5 : 4)} /><text className="wb-outline-point-number" transform={`translate(${point.x + scale * 10} ${point.y + scale * 10}) scale(1,-1)`} fontSize={scale * 11} strokeWidth={scale * 3}>{index + 1}</text></g>)}
    {cursor && <g transform={`translate(${cursor.x} ${cursor.y}) scale(${scale})`}><path className="wb-outline-crosshair" d="M-6 0h12M0-6v12" /><text className="wb-outline-point-number" transform="translate(10 18) scale(1,-1)" fontSize="11" strokeWidth="3">{Number(cursor.x.toFixed(3))}, {Number(cursor.y.toFixed(3))} mm</text></g>}
  </g>;
}

import type { OutlineSnap } from './outlineSnapping';
export function OutlineSnapGuides({snap,scale}:{snap:OutlineSnap|null;scale:number}) {
  if(!snap?.guides.length)return null;
  return <g className="wb-outline-snap-guides" aria-label="Outline alignment guides">{snap.guides.map(guide=><g key={guide.id}>
    <line data-guide={guide.label} x1={guide.from.x-guide.direction.x*scale*2000} y1={guide.from.y-guide.direction.y*scale*2000} x2={guide.from.x+guide.direction.x*scale*2000} y2={guide.from.y+guide.direction.y*scale*2000}/>
    <text className="wb-outline-point-number" transform={`translate(${snap.at.x-scale*12} ${snap.at.y-scale*(30+snap.guides.indexOf(guide)*14)}) scale(1,-1)`} textAnchor="end" fontSize={scale*11} strokeWidth={scale*2}>{guide.label}</text>
  </g>)}</g>;
}

import { useEffect, useRef, useState } from 'react';
import type { EditPhase, OutlineFeature, Part, Vec2 } from '@boardstudio/v2-contracts';
import { insertOutlinePoint, moveOutlinePoint, outlinePoints, removeOutlinePoint, snapOutline } from './outlineEditing';
import { OutlineSnapGuides } from './OutlineSnapGuides';
import type { OutlineSnap, OutlineSnapContext } from './outlineSnapping';
import { pointFromEvent } from './workbenchGeometry';

type Props = { feature: OutlineFeature; connectionId?: string; parts: Part[]; grid: number; snap?: (point: Vec2, context?:OutlineSnapContext, free?:boolean) => OutlineSnap; scale: number; selectedPoint: number; onSelectPoint: (index: number) => void; onDragChange: (active: boolean) => void; onCancel?: () => void; onChange: (feature: OutlineFeature, phase: EditPhase, transaction: string) => void };
type Drag = { feature: OutlineFeature; index: number; transaction: string; target: SVGCircleElement; pointerId: number; pending: OutlineFeature; moved: boolean; world?:Vec2 };

export function OutlineControlOverlay({ feature, connectionId, parts, grid, snap, scale, selectedPoint, onSelectPoint, onDragChange, onChange, onCancel }: Props) {
  const drag = useRef<Drag|null>(null);
  const frame = useRef<number|undefined>(undefined);
  const current = useRef({onChange,onDragChange,onCancel,snap});
  current.current = {onChange,onDragChange,onCancel,snap};
  const [guidance,setGuidance] = useState<OutlineSnap|null>(null);
  const [preview,setPreview] = useState<OutlineFeature|null>(null);
  const finish = (commit: boolean) => {
    const state = drag.current;
    if (!state) return;
    drag.current = null;
    setPreview(null);setGuidance(null);
    current.current.onDragChange(false);
    cancelAnimationFrame(frame.current??0);
    frame.current = undefined;
    if (state.target.hasPointerCapture(state.pointerId)) state.target.releasePointerCapture(state.pointerId);
    if (state.moved) {
      if (!commit && current.current.onCancel) current.current.onCancel();
      else current.current.onChange(commit?state.pending:state.feature,commit?'commit':'preview',state.transaction);
    }
  };
  const updateDrag=(world:Vec2,free:boolean)=>{
    const state=drag.current;if(!state)return;
    const original=outlinePoints(state.feature,parts,connectionId), n=original.length;
    const context={exclude:original[state.index],anchor:original[(state.index+n-1)%n],previous:original[state.index],neighbors:[original[(state.index+1)%n]]};
    const resolved=current.current.snap?.(world,context,free)??{at:free?world:snapOutline(world,grid),guides:[]};
    state.world=world;state.pending=moveOutlinePoint(state.feature,state.index,resolved.at,parts,connectionId);state.moved=true;
    setPreview(state.pending);setGuidance(resolved);
    if(frame.current!==undefined)return;
    frame.current=requestAnimationFrame(()=>{frame.current=undefined;if(drag.current)current.current.onChange(drag.current.pending,'preview',drag.current.transaction);});
  };
  useEffect(()=>{
    const key = (event: KeyboardEvent) => {
      if(event.key==='Alt'&&drag.current?.world){updateDrag(drag.current.world,event.altKey);return;}
      if (!drag.current || !(event.key==='Escape' || (event.ctrlKey||event.metaKey)&&['z','y'].includes(event.key.toLowerCase()))) return;
      event.preventDefault(); event.stopImmediatePropagation(); finish(false);
    };
    const blur = ()=>finish(false);
    window.addEventListener('keydown',key,true);window.addEventListener('keyup',key,true);
    window.addEventListener('blur',blur);
    return ()=>{window.removeEventListener('keydown',key,true);window.removeEventListener('keyup',key,true);window.removeEventListener('blur',blur);finish(false);};
  },[]);
  const points = outlinePoints(preview??feature,parts,connectionId);
  const change = (next: OutlineFeature) => { if(next!==feature) onChange(next,'commit',crypto.randomUUID()); };
  const r = scale*3;
  return <g className="wb-outline-controls" transform="scale(1,-1)" onClick={event=>event.stopPropagation()} onDoubleClick={event=>event.stopPropagation()}>
    <OutlineSnapGuides snap={guidance} scale={scale} />
    {feature.kind!=='rect' && <polyline className="wb-outline-control-path" points={(feature.kind==='polygon'?[...points,points[0]]:points).filter(Boolean).map(point=>`${point.x},${point.y}`).join(' ')} />}
    {points.map((point,index)=>{
      const next = points[(index+1)%points.length];
      const active = index === Math.min(selectedPoint, points.length - 1);
      const insert = () => { change(insertOutlinePoint(feature,index,parts,connectionId)); onSelectPoint(index+1); };
      return <g key={index} data-selected={active}>
        {feature.kind!=='rect' && (feature.kind==='polygon'||index<points.length-1) && <circle className="wb-outline-insert" role="button" tabIndex={0} aria-label={`Insert outline point after ${index+1}`} cx={(point.x+next.x)/2} cy={(point.y+next.y)/2} r={scale*10}
          onPointerDown={event=>event.stopPropagation()} onClick={insert}
          onKeyDown={event=>{if(event.key==='Enter'||event.key===' '){event.preventDefault();event.stopPropagation();insert();}}} />}
        {feature.kind!=='rect' && (feature.kind==='polygon'||index<points.length-1) && <g className="wb-outline-midpoint-mark" transform={`translate(${(point.x+next.x)/2} ${(point.y+next.y)/2}) scale(${scale})`}><circle r="4"/><path d="M-2 0h4M0-2v4"/></g>}
        <circle className="wb-outline-handle" role="button" tabIndex={0} aria-pressed={active} aria-label={`Outline point ${index+1}`} cx={point.x} cy={point.y} r={scale*12}
          onFocus={()=>onSelectPoint(index)} onKeyDown={event=>{
            if (event.key==='Delete'||event.key==='Backspace') {event.preventDefault();event.stopPropagation();const next = removeOutlinePoint(feature,index,connectionId); if (next !== feature) { change(next); onSelectPoint(Math.max(0,index-1)); } return;}
            if (!event.key.startsWith('Arrow')) return;
            event.preventDefault();event.stopPropagation();
            const step=(grid||.1)*(event.shiftKey?10:1);
            change(moveOutlinePoint(feature,index,{x:point.x+(event.key==='ArrowRight'?step:event.key==='ArrowLeft'?-step:0),y:point.y+(event.key==='ArrowUp'?step:event.key==='ArrowDown'?-step:0)},parts,connectionId));
          }}
          onPointerDown={event=>{
            if(event.button!==0)return;
            event.preventDefault();event.stopPropagation();event.currentTarget.focus();onSelectPoint(index);
            event.currentTarget.setPointerCapture(event.pointerId);
            onDragChange(true);
            drag.current={feature,index,transaction:crypto.randomUUID(),target:event.currentTarget,pointerId:event.pointerId,pending:feature,moved:false};
          }} onPointerMove={event=>{
            const state=drag.current;
            if(!state)return;
            event.stopPropagation();
            const world=pointFromEvent(event,event.currentTarget.ownerSVGElement);
            if(!world)return;
            updateDrag(world,event.altKey);
          }} onPointerUp={event=>{if(drag.current){event.stopPropagation();finish(true);}}} onPointerCancel={()=>finish(false)} onLostPointerCapture={()=>finish(false)} />
        <circle className="wb-outline-knob" cx={point.x} cy={point.y} r={active?r+scale:r} />
        <text className="wb-outline-point-number" transform={`translate(${point.x+scale*10} ${point.y+scale*10}) scale(1,-1)`} fontSize={scale*11} strokeWidth={scale*3}>{index+1}</text>
        {active && preview && <text className="wb-outline-point-number" transform={`translate(${point.x+scale*14} ${point.y-scale*17}) scale(1,-1)`} fontSize={scale*11} strokeWidth={scale*3}>{Number(point.x.toFixed(3))}, {Number(point.y.toFixed(3))} mm</text>}
      </g>;
    })}
  </g>;
}

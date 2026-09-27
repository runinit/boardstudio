// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import type { OutlineFeature } from '@boardstudio/v2-contracts';
import { OutlineControlOverlay } from './OutlineControlOverlay';

let host:HTMLDivElement, root:Root;
const feature:OutlineFeature={kind:'polygon',id:'tab',operation:'add',points:[{x:0,y:0},{x:10,y:0},{x:10,y:10}]};
beforeEach(()=>{
  (globalThis as typeof globalThis & {IS_REACT_ACT_ENVIRONMENT:boolean}).IS_REACT_ACT_ENVIRONMENT=true;
  host=document.createElement('div');document.body.append(host);root=createRoot(host);
  let captured=false;
  Object.assign(SVGElement.prototype,{setPointerCapture:()=>{captured=true;},hasPointerCapture:()=>captured,releasePointerCapture:()=>{captured=false;},focus:()=>{}});
  Object.assign(SVGSVGElement.prototype,{getScreenCTM:()=>({inverse:()=>({})}),createSVGPoint:()=>({x:0,y:0,matrixTransform(){return {x:this.x,y:this.y};}})});
  vi.stubGlobal('requestAnimationFrame',(fn:FrameRequestCallback)=>setTimeout(()=>fn(0),0));
  vi.stubGlobal('cancelAnimationFrame',clearTimeout);
});
afterEach(async()=>{await act(async()=>root.unmount());host.remove();vi.unstubAllGlobals();});
async function event(target:Element,type:string,x=0,y=0,extra:Record<string,unknown>={}) {
  await act(async()=>{target.dispatchEvent(new MouseEvent(type,{bubbles:true,clientX:x,clientY:y,button:0,...extra}));await new Promise(resolve=>setTimeout(resolve,1));});
}

test('drag previews snap, Escape restores without a commit, and a later drag commits once',async()=>{
  const changes=vi.fn(),drag=vi.fn();
  await act(async()=>root.render(<svg><OutlineControlOverlay feature={feature} selectedPoint={0} onSelectPoint={()=>{}} parts={[]} grid={.5} scale={1} onChange={changes} onDragChange={drag}/></svg>));
  const handle=host.querySelector('.wb-outline-handle')!;
  await event(handle,'pointerdown');await event(handle,'pointermove',2.3,3.2);
  expect(handle.getAttribute('cx')).toBe('2.5');expect(handle.getAttribute('cy')).toBe('-3');
  expect(changes.mock.calls[0][1]).toBe('preview');
  await act(async()=>window.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true})));
  expect(handle.getAttribute('cx')).toBe('0');expect(changes.mock.calls.at(-1)![0]).toEqual(feature);
  expect(changes.mock.calls.some(call=>call[1]==='commit')).toBe(false);
  await event(handle,'pointerdown');await event(handle,'pointermove',1.23,2.34,{altKey:true});await event(handle,'pointerup',1.23,2.34);
  const commits=changes.mock.calls.filter(call=>call[1]==='commit');
  expect(commits).toHaveLength(1);expect(commits[0][0].points[0]).toEqual({x:1.23,y:-2.34});
  expect(drag.mock.calls.map(call=>call[0])).toEqual([true,false,true,false]);
});

test('focused points nudge on the grid and minimum vertex count cannot be deleted',async()=>{
  const changes=vi.fn();
  await act(async()=>root.render(<svg><OutlineControlOverlay feature={feature} selectedPoint={0} onSelectPoint={()=>{}} parts={[]} grid={.1} scale={1} onChange={changes} onDragChange={()=>{}}/></svg>));
  const handle=host.querySelector('.wb-outline-handle')!;
  await act(async()=>handle.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowRight',shiftKey:true,bubbles:true})));
  expect(changes.mock.calls[0][0].points[0]).toEqual({x:1,y:0});
  await act(async()=>handle.dispatchEvent(new KeyboardEvent('keydown',{key:'Delete',bubbles:true})));
  expect(changes).toHaveBeenCalledTimes(1);
});

import { expect, test } from 'vitest';
import { snapDelta } from './workbenchGeometry';
import { snapOutlinePoint } from './outlineSnapping';
const paths = [[{x:0,y:0},{x:269,y:0},{x:269,y:13},{x:0,y:13}]];
const settings = { paths, grid:{x:1,y:1}, tolerance:1, enabled:true, anchor:{x:230,y:20} };
test('shows and snaps the right edge alignment together with the previous point horizontal',()=>{
  const result=snapOutlinePoint({x:268.4,y:20.4},settings);
  expect(result.at).toEqual({x:269,y:20});
  expect(result.guides.map(g=>g.label)).toContain('Vertical alignment');
  expect(result.guides.map(g=>g.label)).toContain('Horizontal alignment');
});
test('sticks until release distance and Alt bypass releases every inference and grid',()=>{
  const locked=snapOutlinePoint({x:268.4,y:20.4},settings);
  expect(snapOutlinePoint({x:270.3,y:20.3},settings,locked).at.x).toBe(269);
  expect(snapOutlinePoint({x:272,y:23},settings,locked).guides).toHaveLength(0);
  expect(snapOutlinePoint({x:268.4,y:20.4},{...settings,free:true},locked)).toEqual({at:{x:268.4,y:20.4},guides:[]});
});
test('infers collinear and perpendicular continuations of an angled edge',()=>{
  const config={...settings,paths:[],anchor:{x:10,y:10},previous:{x:0,y:0}};
  expect(snapOutlinePoint({x:20,y:20.4},config).at.x).toBeCloseTo(20.2);
  expect(snapOutlinePoint({x:0,y:20.4},config).at.x).toBeCloseTo(-.2);
});

test('shared grid supports mm steps for layout and outlines without changing unit steps or Off',()=>{
  const point={x:4.23,y:-2.31},pitch={x:19,y:20};
  expect(snapDelta(point,pitch,-.5)).toEqual({x:4,y:-2.5});
  expect(snapDelta(point,pitch,0)).toEqual(point);
  expect(snapDelta(point,pitch,.25)).toEqual({x:4.75,y:-0});
  expect(snapOutlinePoint(point,{...settings,grid:{x:.5,y:.5},enabled:false})).toEqual({at:{x:4,y:-2.5},guides:[]});
});

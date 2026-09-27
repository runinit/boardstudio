import { expect, test } from 'vitest';
import type { OutlineFeature, Part } from '@boardstudio/v2-contracts';
import { attachOutline, insertOutlinePoint, moveOutlinePoint, outlineLocal, outlinePoints, outlineWorld, removeOutlinePoint, snapOutline } from './outlineEditing';
const part: Part = {id:'anchor',definitionId:'switch',reference:'SW1',pose:{at:{x:50,y:70},rotation:32},side:'back'};
const polygon: OutlineFeature = {id:'tab',kind:'polygon',operation:'add',points:[{x:5,y:10},{x:15,y:10},{x:15,y:20}]};

test('attachments roundtrip rotated back-side coordinates without moving a polygon',()=>{
  const point={x:3.25,y:-9.5};
  const restored=outlineLocal(outlineWorld(point,part),part);
  expect(restored.x).toBeCloseTo(point.x,8);expect(restored.y).toBeCloseTo(point.y,8);
  const attached=attachOutline(polygon,part.id,[part]);
  outlinePoints(attached,[part]).forEach((point,index)=>{expect(point.x).toBeCloseTo(polygon.points[index].x,8);expect(point.y).toBeCloseTo(polygon.points[index].y,8);});
  const detached=attachOutline(attached,undefined,[part]);
  expect(detached.kind).toBe('polygon');
  outlinePoints(detached,[]).forEach((point,index)=>expect(point.x).toBeCloseTo(polygon.points[index].x,8));
});

test('rectangles retain world rotation and radius through attaching and detaching',()=>{
  const rect:OutlineFeature={id:'rect',kind:'rect',operation:'subtract',center:{x:30,y:25},size:{x:14,y:7},radius:2,rotation:17};
  const attached=attachOutline(rect,part.id,[part]);
  const detached=attachOutline(attached,undefined,[part]);
  expect(detached).toMatchObject({kind:'rect',radius:2,rotation:17,size:rect.size});
  expect(outlinePoints(detached,[])[0].x).toBeCloseTo(30,8);
});

test('bridge point attachment, insertion and moving preserve other endpoint references',()=>{
  let feature:OutlineFeature={kind:'part-envelope',id:'edge',partIds:[part.id],margin:4,operation:'add',connections:[{id:'bridge',width:3,points:[{at:{x:0,y:0},partId:part.id},{at:{x:90,y:20}}]}]};
  feature=insertOutlinePoint(feature,0,[part],'bridge');
  feature=moveOutlinePoint(feature,0,{x:52,y:72},[part],'bridge');
  feature=attachOutline(feature,part.id,[part],'bridge',2);
  expect(outlinePoints(feature,[part],'bridge')[0]).toEqual({x:52,y:72});
  if(feature.kind!=='part-envelope') throw new Error('Expected envelope');
  expect(feature.connections![0].points[0].partId).toBe(part.id);
  expect(feature.connections![0].points[1].partId).toBeUndefined();
  expect(feature.connections![0].points[2].partId).toBe(part.id);
  const removed=removeOutlinePoint(feature,1,'bridge');
  expect(outlinePoints(removed,[part],'bridge')).toHaveLength(2);
  expect(removeOutlinePoint(removed,0,'bridge')).toEqual(removed);
  expect(removeOutlinePoint(polygon,0)).toBe(polygon);
});

test('millimetre grid rounds positive and negative positions independently of keyboard pitch',()=>{
  expect(snapOutline({x:4.23,y:-2.31},.5)).toEqual({x:4,y:-2.5});
  expect(snapOutline({x:4.23,y:-2.31},.1)).toEqual({x:4.2,y:-2.3});
});

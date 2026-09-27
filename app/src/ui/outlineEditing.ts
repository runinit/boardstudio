import type { OutlineControlPoint, OutlineFeature, Part, Vec2 } from '@boardstudio/v2-contracts';

export type OutlineSelection = { featureId: string; connectionId?: string };
export type OutlineDrawMode = 'add' | 'subtract' | 'connect';

export function outlineWorld(at: Vec2, part?: Part): Vec2 {
  if (!part) return at;
  const angle = part.pose.rotation * Math.PI / 180;
  const x = at.x * (part.side === 'back' ? -1 : 1);
  return { x: part.pose.at.x + x*Math.cos(angle)-at.y*Math.sin(angle), y: part.pose.at.y+x*Math.sin(angle)+at.y*Math.cos(angle) };
}

export function outlineLocal(at: Vec2, part?: Part): Vec2 {
  if (!part) return at;
  const angle = -part.pose.rotation * Math.PI / 180;
  const x = at.x-part.pose.at.x, y = at.y-part.pose.at.y;
  return { x: (x*Math.cos(angle)-y*Math.sin(angle))*(part.side==='back'?-1:1), y: x*Math.sin(angle)+y*Math.cos(angle) };
}

export function snapOutline(point: Vec2, grid: number): Vec2 {
  return { x: Number((Math.round(point.x/grid)*grid).toFixed(6)), y: Number((Math.round(point.y/grid)*grid).toFixed(6)) };
}

export function outlinePoints(feature: OutlineFeature, parts: Part[], connectionId?: string): Vec2[] {
  if (feature.kind === 'part-envelope') return feature.connections?.find(item=>item.id===connectionId)?.points.map(point=>outlineWorld(point.at, parts.find(part=>part.id===point.partId))) ?? [];
  const anchor = parts.find(part=>part.id===feature.anchorPartId);
  return (feature.kind==='polygon'?feature.points:[feature.center]).map(point=>outlineWorld(point,anchor));
}

export function moveOutlinePoint(feature: OutlineFeature, index: number, world: Vec2, parts: Part[], connectionId?: string): OutlineFeature {
  if (feature.kind === 'part-envelope') return { ...feature, connections: feature.connections?.map(connection=>connection.id!==connectionId?connection:{...connection,points:connection.points.map((point,i)=>i!==index?point:{...point,at:outlineLocal(world,parts.find(part=>part.id===point.partId))})}) };
  const at = outlineLocal(world, parts.find(part=>part.id===feature.anchorPartId));
  return feature.kind==='polygon'?{...feature,points:feature.points.map((point,i)=>i===index?at:point)}:{...feature,center:at};
}

export function insertOutlinePoint(feature: OutlineFeature, index: number, parts: Part[], connectionId?: string): OutlineFeature {
  const points = outlinePoints(feature,parts,connectionId);
  const a = points[index], b = points[(index+1)%points.length];
  if (!a || !b || feature.kind === 'rect') return feature;
  const at = {x:(a.x+b.x)/2,y:(a.y+b.y)/2};
  if (feature.kind==='polygon') return {...feature,points:[...feature.points.slice(0,index+1),outlineLocal(at,parts.find(part=>part.id===feature.anchorPartId)),...feature.points.slice(index+1)]};
  return {...feature,connections:feature.connections?.map(connection=>connection.id!==connectionId?connection:{...connection,points:[...connection.points.slice(0,index+1),{at},...connection.points.slice(index+1)]})};
}

export function removeOutlinePoint(feature: OutlineFeature, index: number, connectionId?: string): OutlineFeature {
  if (feature.kind==='polygon') return feature.points.length<=3?feature:{...feature,points:feature.points.filter((_,i)=>i!==index)};
  if (feature.kind==='rect' || (feature.connections?.find(connection=>connection.id===connectionId)?.points.length??0)<=2) return feature;
  return {...feature,connections:feature.connections?.map(connection=>connection.id!==connectionId||connection.points.length<=2?connection:{...connection,points:connection.points.filter((_,i)=>i!==index)})};
}

/** Changing attachment never changes the visible shape. Coordinates remain local in storage. */
export function attachOutline(feature: OutlineFeature, partId: string | undefined, parts: Part[], connectionId?: string, index = 0): OutlineFeature {
  const anchor = parts.find(part=>part.id===partId);
  if (feature.kind==='part-envelope') return {...feature,connections:feature.connections?.map(connection=>connection.id!==connectionId?connection:{...connection,points:connection.points.map((point,i)=>i!==index?point:{at:outlineLocal(outlineWorld(point.at,parts.find(part=>part.id===point.partId)),anchor),partId})})};
  const old = parts.find(part=>part.id===feature.anchorPartId);
  if (feature.kind==='polygon') return {...feature,anchorPartId:partId,points:feature.points.map(point=>outlineLocal(outlineWorld(point,old),anchor))};
  const worldRotation = (old?.pose.rotation??0)+(feature.rotation??0)*(old?.side==='back'?-1:1);
  return {...feature,anchorPartId:partId,center:outlineLocal(outlineWorld(feature.center,old),anchor),rotation:(worldRotation-(anchor?.pose.rotation??0))*(anchor?.side==='back'?-1:1)};
}

export function connectionEndpoint(at: Vec2, parts: Part[]): OutlineControlPoint {
  const nearest = parts.reduce<Part|undefined>((best,part)=>!best||Math.hypot(part.pose.at.x-at.x,part.pose.at.y-at.y)<Math.hypot(best.pose.at.x-at.x,best.pose.at.y-at.y)?part:best,undefined);
  return nearest && Math.hypot(nearest.pose.at.x-at.x,nearest.pose.at.y-at.y)<=10 ? {at:outlineLocal(at,nearest),partId:nearest.id} : {at};
}

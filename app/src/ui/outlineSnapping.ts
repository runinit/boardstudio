import type { Vec2 } from '@boardstudio/v2-contracts';
type OutlineGuide = { id:string; from:Vec2; direction:Vec2; label:string };
export type OutlineSnap = { at:Vec2; guides:OutlineGuide[] };
export type OutlineSnapContext = { anchor?:Vec2; previous?:Vec2; exclude?:Vec2; neighbors?:Vec2[] };
type Settings = OutlineSnapContext & { paths:Vec2[][]; grid:Vec2; tolerance:number; enabled:boolean; free?:boolean };
const distance = (a:Vec2,b:Vec2)=>Math.hypot(a.x-b.x,a.y-b.y);
/** Screen-distance acquisition/release avoids zoom-dependent magnetic strength. */
export function snapOutlinePoint(point:Vec2, settings:Settings, previous?:OutlineSnap):OutlineSnap {
  if(settings.free) return {at:point,guides:[]};
  const at={x:settings.grid.x>0?Number((Math.round(point.x/settings.grid.x)*settings.grid.x).toFixed(6)):point.x,y:settings.grid.y>0?Number((Math.round(point.y/settings.grid.y)*settings.grid.y).toFixed(6)):point.y};
  if(!settings.enabled) return {at,guides:[]};
  const candidates:OutlineGuide[]=[];
  const add=(from:Vec2,direction:Vec2,label:string)=>{
    const length=Math.hypot(direction.x,direction.y); if(length<1e-8)return;
    const d={x:direction.x/length,y:direction.y/length};
    candidates.push({id:`${label}/${from.x}/${from.y}/${d.x}/${d.y}`,from,direction:d,label});
  };
  const landmarks=(p:Vec2)=>{if(settings.exclude&&distance(p,settings.exclude)<1e-6)return;add(p,{x:0,y:1},'Vertical alignment');add(p,{x:1,y:0},'Horizontal alignment');};
  for(const path of settings.paths){
    for(let i=0;i<path.length;i++){
      const a=path[i],b=path[(i+1)%path.length]; landmarks(a);
      const dx=b.x-a.x,dy=b.y-a.y,length=dx*dx+dy*dy;if(!length)continue;
      if(settings.exclude&&(distance(a,settings.exclude)<1e-6||distance(b,settings.exclude)<1e-6))continue;
      const t=((point.x-a.x)*dx+(point.y-a.y)*dy)/length;
      if(t>=0&&t<=1&&distance(a,b)>settings.tolerance*2)add(a,{x:dx,y:dy},'On edge');
      if(settings.anchor){
        const k=((settings.anchor.x-a.x)*dx+(settings.anchor.y-a.y)*dy)/length;
        if(k>=-1e-6&&k<=1+1e-6&&distance(settings.anchor,{x:a.x+k*dx,y:a.y+k*dy})<.002){add(settings.anchor,{x:dx,y:dy},'Collinear');add(settings.anchor,{x:-dy,y:dx},'Perpendicular');}
      }
    }
  }
  if(settings.anchor)landmarks(settings.anchor);
  settings.neighbors?.forEach(neighbor=>{
    landmarks(neighbor);
    if(settings.exclude){const d={x:settings.exclude.x-neighbor.x,y:settings.exclude.y-neighbor.y};add(neighbor,d,'Collinear');add(neighbor,{x:-d.y,y:d.x},'Perpendicular');}
  });
  if(settings.anchor&&settings.previous){const d={x:settings.anchor.x-settings.previous.x,y:settings.anchor.y-settings.previous.y};add(settings.anchor,d,'Collinear');add(settings.anchor,{x:-d.y,y:d.x},'Perpendicular');}
  const projection=(guide:OutlineGuide)=>{const t=(point.x-guide.from.x)*guide.direction.x+(point.y-guide.from.y)*guide.direction.y;return {x:guide.from.x+t*guide.direction.x,y:guide.from.y+t*guide.direction.y};};
  const ranked=candidates.map(guide=>{const projected=projection(guide),d=distance(point,projected),held=previous?.guides.some(g=>g.id===guide.id);return {guide,projected,d,held};}).filter(item=>item.d<=settings.tolerance*(item.held?1.8:1)).sort((a,b)=>Number(b.held)-Number(a.held)||a.d-b.d);
  const first=ranked[0];if(!first)return {at,guides:[]};
  const second=ranked.find(item=>Math.abs(first.guide.direction.x*item.guide.direction.y-first.guide.direction.y*item.guide.direction.x)>.1);
  const guides=[first.guide]; let snapped=first.projected;
  if(Math.abs(first.guide.direction.x)<1e-6)snapped.y=at.y;
  else if(Math.abs(first.guide.direction.y)<1e-6)snapped.x=at.x;
  if(second){
    const a=first.guide,b=second.guide,den=a.direction.x*b.direction.y-a.direction.y*b.direction.x;
    const t=((b.from.x-a.from.x)*b.direction.y-(b.from.y-a.from.y)*b.direction.x)/den;
    const intersection={x:a.from.x+t*a.direction.x,y:a.from.y+t*a.direction.y};
    if(distance(intersection,point)<=settings.tolerance*2.6){snapped=intersection;guides.push(b);}
  }
  return {at:{x:Number(snapped.x.toFixed(6)),y:Number(snapped.y.toFixed(6))},guides};
}

import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
const stage = resolve(process.argv[2]);
const directory = resolve(process.argv[3]);
const polygonContains = (points, x, y) => {
  let inside = false;
  for (let i=0,j=points.length-1;i<points.length;j=i++) {
    const a=points[i],b=points[j];
    if ((a.y>y)!==(b.y>y) && x<(b.x-a.x)*(y-a.y)/(b.y-a.y)+a.x) inside=!inside;
  }
  return inside;
};
function meshChecks(mesh,input,maxChordError) {
  const edges = new Map(); let volume=0,degenerate=0,badNormals=0;
  const vertical=[];const p=mesh.positions;
  assert.equal(p.length%9,0);
  assert.equal(p.length,mesh.normals.length);
  assert.ok(p.every(Number.isFinite)&&mesh.normals.every(Number.isFinite));
  for(let i=0;i<p.length;i+=9){
    const a=p.slice(i,i+3),b=p.slice(i+3,i+6),c=p.slice(i+6,i+9);
    const cross=[(b[1]-a[1])*(c[2]-a[2])-(b[2]-a[2])*(c[1]-a[1]),
      (b[2]-a[2])*(c[0]-a[0])-(b[0]-a[0])*(c[2]-a[2]),
      (b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0])];
    if(Math.hypot(...cross)<1e-10)degenerate++;
    volume+=(a[0]*(b[1]*c[2]-b[2]*c[1])+a[1]*(b[2]*c[0]-b[0]*c[2])+a[2]*(b[0]*c[1]-b[1]*c[0]))/6;
    for(const [v,w] of [[a,b],[b,c],[c,a]]){
      const vs=v.join(','),ws=w.join(','),key=vs<ws?vs+'|'+ws:ws+'|'+vs;
      const e=edges.get(key)??{count:0,direction:0};e.count++;e.direction+=vs<ws?1:-1;edges.set(key,e);
    }
    for(let j=i;j<i+9;j+=3){const normal=mesh.normals.slice(j,j+3);if(Math.abs(Math.hypot(...normal)-1)>1e-4||normal.reduce((s,n,k)=>s+n*cross[k],0)<=0)badNormals++;}
    if(Math.abs(cross[2])>1e-10)vertical.push({a,b,c,den:cross[2]});
  }
  const meshContains=(x,y,z)=>{
    const hits=[];
    for(const {a,b,c,den} of vertical){
      const u=((x-a[0])*(c[1]-a[1])-(y-a[1])*(c[0]-a[0]))/den;
      const v=((b[0]-a[0])*(y-a[1])-(b[1]-a[1])*(x-a[0]))/den;
      if(u>=0&&v>=0&&u+v<=1){const height=a[2]+u*(b[2]-a[2])+v*(c[2]-a[2]);if(height>z&&!hits.some(h=>Math.abs(h-height)<1e-6))hits.push(height);}
    }
    return hits.length%2===1;
  };
  const {body,regions}=input,z0=body.z,z1=z0+body.thickness;
  const levels=[...new Set([z0,z1,...body.openings.flatMap(o=>[Math.max(z0,o.z),Math.min(z1,o.z+o.height)])])].sort((a,b)=>a-b);
  const depths=levels.slice(1).map((z,i)=>(z+levels[i])/2).concat([z0-0.1,z1+0.1]);
  let samples=0,mismatches=0,excludedCircularBand=0;
  const check=(x,y,z)=>{
    if(regions.some(r=>r.mounts.some(m=>Math.abs(Math.hypot(x-m.at.x,y-m.at.y)-m.holeDiameter/2)<maxChordError+1e-4))){excludedCircularBand++;return;}
    const expected=z>z0&&z<z1&&regions.some(r=>polygonContains(r.outer,x,y)&&!r.holes.some(h=>polygonContains(h,x,y))
      &&!r.mounts.some(m=>Math.hypot(x-m.at.x,y-m.at.y)<m.holeDiameter/2))
      &&!body.openings.some(o=>z>o.z&&z<o.z+o.height&&polygonContains(o.points,x,y));
    samples++;if(meshContains(x,y,z)!==expected)mismatches++;
  };
  const outer=regions.flatMap(r=>r.outer);
  const minX=Math.min(...outer.map(p=>p.x))-2.113,maxX=Math.max(...outer.map(p=>p.x))+2.113;
  const minY=Math.min(...outer.map(p=>p.y))-1.727,maxY=Math.max(...outer.map(p=>p.y))+2.4;
  for(let x=minX;x<maxX;x+=3.07)for(let y=minY;y<maxY;y+=3.13)for(const z of depths)check(x,y,z);
  // Target the through holes and pocket depths, not just broad uniform space.
  for(const r of regions)for(const m of r.mounts)for(const offset of [0,-0.02,0.02])for(let angle=0;angle<16;angle++)for(const z of depths){
    const radius=offset===0?0:m.holeDiameter/2+offset;check(m.at.x+radius*Math.cos(angle*Math.PI/8),m.at.y+radius*Math.sin(angle*Math.PI/8),z);
  }
  return {meshVolume:volume,degenerate,badNormals,unpairedEdges:[...edges.values()].filter(e=>e.count!==2||e.direction!==0).length,samples,mismatches,excludedCircularBand};
}
const report=[];
for(const fixture of (process.env.CAD_EXPERIMENT_FIXTURES??'regression,edit-0,edit-2,edit-5').split(',')){
  const input=JSON.parse(await readFile(resolve(stage,'inputs',fixture+'.json')));
  const reference=JSON.parse(await readFile(resolve(directory,fixture+'-control-geometry.json'))).result;
  for(const variant of (process.env.CAD_EXPERIMENT_VARIANTS??'control,profiles,manifold').split(',')){
    const data=JSON.parse(await readFile(resolve(directory,fixture+'-'+variant+'-geometry.json'))).result;
    const checks=meshChecks(data.mesh,input,data.maxChordError??0.1);
    const row={fixture,variant,solids:data.solids,volume:data.volume,volumeDifference:data.volume-reference.volume,
      volumeErrorBound:data.volumeErrorBound??0.01,maxChordError:data.maxChordError??null,...checks};report.push(row);
    assert.equal(data.solids,2);assert.equal(checks.degenerate,0);assert.equal(checks.badNormals,0);
    // BRep tessellation can have unwelded topological seams; record, do not conflate
    // the render triangle soup with the independent STEP-solid validity check.
    if(variant==='manifold')assert.equal(checks.unpairedEdges,0);
    assert.equal(checks.mismatches,0,`${fixture}/${variant} occupancy`);
    assert.ok(Math.abs(row.volumeDifference)<=row.volumeErrorBound+0.01,`${fixture}/${variant} analytic volume`);
  }
}
await writeFile(resolve(directory,'preview-validation.json'),JSON.stringify(report,null,2));
console.log(report.map(({fixture,variant,samples,mismatches,unpairedEdges})=>({fixture,variant,samples,mismatches,unpairedEdges})));

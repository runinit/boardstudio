import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
const root=resolve(import.meta.dirname,'../../..');
const captured=JSON.parse(await readFile(resolve(root,'app/performance-results/phase2-attribution-before/prepared.json')));
const axisAligned=points=>points.every((a,i)=>{const b=points[(i+1)%points.length];return (a.x===b.x)!==(a.y===b.y);});
const key=JSON.stringify;
const plates=[];
for(const prepared of captured.gaskets[0].bodies){
  if(prepared.body.kind!=='plate')continue;
  for(const [index,region] of prepared.regions.entries()){
    const vertices=[...region.outer,...region.holes.flat()];
    const cells=(new Set(vertices.map(p=>p.x)).size-1)*(new Set(vertices.map(p=>p.y)).size-1);
    plates.push({body:prepared.body.id,region:index,outerVertices:region.outer.length,
      outerOrthogonal:axisAligned(region.outer),holesOrthogonal:region.holes.every(axisAligned),
      holeVertices:region.holes.flat().length,cells,mounts:region.mounts.length,
      openings:prepared.body.openings?.length??0,cavities:region.cavities.length,gaskets:region.gaskets.length});
  }
}
const bottoms=captured.gaskets.map(x=>x.bodies.find(b=>b.body.id==='bottom'));
const features=bottom=>[...bottom.regions.flatMap((r,i)=>[
  {type:'outer',id:`region-${i}`,value:r.outer},
  ...r.mounts.map((m,j)=>({type:'mount',id:`region-${i}-${j}`,value:{...m,z:bottom.body.z,height:bottom.body.thickness}}))]),
  ...bottom.body.openings.map((o,i)=>({type:'opening',id:`opening-${i}`,value:o}))];
const baseline=features(bottoms[0]);
const stability=baseline.map(feature=>({type:feature.type,id:feature.id,
  distinctInputs:new Set(bottoms.map(b=>key(features(b).find(f=>f.id===feature.id&&f.type===feature.type).value))).size}));
const report={note:'Candidate screening only; exact orthogonal validity, contact/fallback and speed remain untested. Repeated primitive inputs do not imply topology can be shared safely.',plates,stability};
await writeFile(resolve(process.argv[2]),JSON.stringify(report,null,2));
console.log(JSON.stringify(report,null,2));

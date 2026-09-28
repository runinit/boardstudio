import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
const directory=resolve(process.argv[2]);
const raw=JSON.parse(await readFile(resolve(directory,'raw.json')));
const quantile=(values,q=0.5)=>values.sort((a,b)=>a-b)[Math.max(0,Math.ceil(values.length*q)-1)];
const summary=JSON.parse(await readFile(resolve(directory,'summary.json')));
const comparisons=summary.map(row=>{
  const control=summary.find(r=>r.fixture===row.fixture&&r.variant==='control');
  return {...row,medianChangePercent:100*(row.medianMs/control.medianMs-1),
    p95ChangePercent:100*(row.p95Ms/control.p95Ms-1),
    pairedMedianChanges:row.sessionMedians.map((v,i)=>100*(v/control.sessionMedians[i]-1))};
});
const attribution=[];
for(const variant of new Set(raw.filter(r=>r.phase==='attribution').map(r=>r.variant))){
  const rows=raw.filter(r=>r.phase==='attribution'&&r.variant===variant);
  attribution.push({variant,count:rows.length,medianMs:quantile(rows.map(r=>r.result.generationMs)),
    stages:Object.fromEntries(Object.keys(rows[0].result.stages).map(name=>[name,{
      medianMs:quantile(rows.map(r=>r.result.stages[name].durationMs)),
      p95Ms:quantile(rows.map(r=>r.result.stages[name].durationMs),0.95),
      calls:rows[0].result.stages[name].calls}]))});
}
const memory=[];
for(let session=0;session<5;session++)for(const kernel of ['cadrum','manifold']){
  const rows=raw.filter(r=>r.session===session&&r.phase==='warm'&&(kernel==='manifold'?r.variant==='manifold':r.variant!=='manifold'));
  if(!rows.length)continue;
  memory.push({session,kernel,count:rows.length,firstCapacity:rows[0].heapBytes,lastCapacity:rows.at(-1).heapBytes,
    maximumCapacity:rows.every(r=>r.heapBytes!==null)?Math.max(...rows.map(r=>r.heapBytes)):null});
}
const report={comparisons,attribution,memory};
await writeFile(resolve(directory,'analysis.json'),JSON.stringify(report,null,2));
console.log(JSON.stringify({comparisons:comparisons.filter(r=>r.fixture==='regression'),attribution,memory},null,2));

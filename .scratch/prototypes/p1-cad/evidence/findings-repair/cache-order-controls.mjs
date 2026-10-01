import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {writeFileSync} from 'node:fs';
import {gzipSync} from 'node:zlib';
const probe=fileURLToPath(new URL('cache-order-probe.mjs',import.meta.url));
const sequences=['preview','export','preview,export','export,preview'];
const controls=Object.fromEntries(sequences.map(sequence=>[sequence,JSON.parse(execFileSync(process.execPath,[probe,sequence],{encoding:'utf8'}))]));
function geometry(result){
    const {positions:p,normals:n}=result.mesh;assert.equal(p.length,n.length);assert.ok(p.length>0 && p.length%9===0);assert.ok([...p,...n].every(Number.isFinite));
    let volume=0;const min=[Infinity,Infinity,Infinity],max=[-Infinity,-Infinity,-Infinity];
    for(let i=0;i<p.length;i+=3){for(let a=0;a<3;a++){min[a]=Math.min(min[a],p[i+a]);max[a]=Math.max(max[a],p[i+a]);}assert.ok(Math.abs(Math.hypot(...n.slice(i,i+3))-1)<.001);}
    for(let i=0;i<p.length;i+=9){const t=p.slice(i,i+9);volume+=(t[0]*(t[4]*t[8]-t[5]*t[7])-t[1]*(t[3]*t[8]-t[5]*t[6])+t[2]*(t[3]*t[7]-t[4]*t[6]))/6;}
    assert.ok(Math.abs(Math.abs(volume)-720)<.1);assert.deepEqual(min,[-.5,-.5,0]);assert.deepEqual(max,[20.5,20.5,2]);assert.equal(result.revision,7);assert.deepEqual(result.bodyIds,['case']);
    return {vertices:p.length/3,volume:Math.abs(volume),min,max,positionHash:result.mesh.positionHash,normalHash:result.mesh.normalHash};
}
const summary=Object.fromEntries(Object.entries(controls).map(([sequence,outputs])=>[sequence,outputs.map(result=>({operation:result.operation,...geometry(result)}))]));
assert.equal(summary.preview[0].vertices,192);assert.equal(summary.export[0].vertices,96);
assert.equal(summary['preview,export'][1].vertices,192);assert.equal(summary['export,preview'][1].vertices,96);
assert.equal(summary['preview,export'][1].positionHash,summary.preview[0].positionHash);
assert.equal(summary['export,preview'][1].positionHash,summary.export[0].positionHash);
assert.equal(summary['preview,export'][1].normalHash,summary.preview[0].normalHash);
assert.equal(summary['export,preview'][1].normalHash,summary.export[0].normalHash);
writeFileSync(new URL('cache-order-controls.json.gz',import.meta.url),gzipSync(JSON.stringify(controls)));
writeFileSync(new URL('cache-order-summary.json',import.meta.url),JSON.stringify({status:'passed',summary},null,2)+'\n');
console.log(JSON.stringify(summary));

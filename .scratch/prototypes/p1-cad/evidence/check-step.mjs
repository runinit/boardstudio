// Copied existing test oracle, applied to STEP bytes returned by the Rust worker.
import assert from 'node:assert/strict';
import {readFileSync, writeFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {gunzipSync} from 'node:zlib';
import {bodyKey} from '../../../../cad/src/preview.ts';
import {prepareAssembly} from '../../../../cad/test/native-prepare.mjs';
const require=createRequire(new URL('../../../../cad/package.json',import.meta.url));
const {createInstance}=await import(require.resolve('libcascade/single/init'));
const p=new URL('./',import.meta.url);
const read=name=>JSON.parse(readFileSync(new URL(name,p),'utf8'));
const prepared=read('../fixtures/prepared.json');
assert.deepEqual(prepareAssembly(read('../fixtures/raw.json')),prepared,'copied public preparation remains exact');
assert.equal(bodyKey(prepared.bodies[0]),readFileSync(new URL('../fixtures/key.txt',p),'utf8'));
// Match the browser's separate preview/export module lifetimes. Sharing this
// reference instance lets export warm preview with a different valid mesh.
const freshReference=operation=>JSON.parse(execFileSync(process.execPath,[fileURLToPath(new URL('findings-repair/cache-order-probe.mjs',p)),operation],{encoding:'utf8'}))[0];
const reference=freshReference('export');
reference.step=new Uint8Array(Buffer.from(reference.step,'base64'));
const preview=freshReference('preview');
assert.deepEqual(reference.bodyIds,['case']);assert.deepEqual(preview.bodyIds,['case']);
assert.equal(reference.revision,7);
const oc=await createInstance();
function inspect(bytes){
    oc.FS.writeFile('/p1-cad.step',bytes);
    using reader=new oc.STEPControl_Reader();
    assert.equal(reader.ReadFile('/p1-cad.step'),oc.IFSelect_ReturnStatus.IFSelect_RetDone);
    reader.TransferRoots();
    using shape=reader.OneShape();using mass=new oc.GProp_GProps();using box=new oc.Bnd_Box();
    using solids=new oc.TopExp_Explorer(shape,oc.TopAbs_ShapeEnum.TopAbs_SOLID);
    oc.BRepGProp.VolumeProperties(shape,mass,true,true,false);oc.BRepBndLib.AddOptimal(shape,box,false,false);
    let count=0;while(solids.More()){count++;solids.Next();}oc.FS.unlink('/p1-cad.step');
    return {volume:mass.Mass(),solids:count,min:[box.GetXMin(),box.GetYMin(),box.GetZMin()],max:[box.GetXMax(),box.GetYMax(),box.GetZMax()]};
}
const baseline=inspect(reference.step);
const results=[];
const controls=JSON.parse(gunzipSync(readFileSync(new URL('findings-repair/cache-order-controls.json.gz',p))));
const cachedExports=[];
for(const [history,outputs] of Object.entries(controls)){
    for(const output of outputs.filter(o=>o.operation==='export')){
        const actual=inspect(new Uint8Array(Buffer.from(output.step,'base64')));
        assert.equal(actual.solids,1);assert.equal(actual.solids,baseline.solids);
        assert.ok(Math.abs(actual.volume-720)<.1);assert.ok(Math.abs(actual.volume-baseline.volume)<.1);
        for(const key of ['min','max'])actual[key].forEach((n,i)=>assert.ok(Math.abs(n-baseline[key][i])<.001));
        cachedExports.push({history,actual});
    }
}
for(const label of ['root','subpath']){
    const name=`boardstudio-p1-cad-20261001-${label}`;
    const report=read(name+'-browser.json');assert.equal(report.status,'passed');
    const bytes=new Uint8Array(readFileSync(new URL(name+'.step',p)));
    const actual=inspect(bytes);
    assert.equal(actual.solids,1);assert.equal(actual.solids,baseline.solids);
    assert.ok(Math.abs(actual.volume-720)<.1);assert.ok(Math.abs(actual.volume-baseline.volume)<.1);
    for(const key of ['min','max'])actual[key].forEach((n,i)=>assert.ok(Math.abs(n-baseline[key][i])<.001));
    assert.equal(report.report.export.vertices,reference.mesh.positions.length/3);
    assert.equal(report.report.preview.vertices,preview.mesh.positions.length/3);
    for(const [kind,expected] of [['preview',preview],['export',reference]]){
        const measured=report.report[kind+'_mesh'];
        assert.ok(measured,`${label} missing ${kind} mesh-byte evidence`);
        for(const field of ['positions','normals']){
            const actual=Buffer.from(measured[field+'_hex'],'hex');
            // JSON number arrays erase signed zero. Compare original provider
            // bytes, including its Float32 signs, rather than reconstructing them.
            const wanted=Buffer.from(expected.mesh[field+'Hex'],'hex');
            assert.deepEqual(actual,wanted,`${label} ${kind} ${field} changed at matched cache state`);
        }
    }
    results.push({label,actual,baseline,referenceExportVertices:reference.mesh.positions.length/3,referencePreviewVertices:preview.mesh.positions.length/3});
}
writeFileSync(new URL('step-oracle.json',p),JSON.stringify({status:'passed',oracle:'existing cad/test/case.test.mjs inspectStep, unchanged libcascade 3.0.2, volume720 and one solid; independent preview/export reference lifetimes and cached/uncached STEP histories',cachedExports,results},null,2)+'\n');
console.log(JSON.stringify(results));

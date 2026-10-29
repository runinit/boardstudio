// Actual sealed-handle tests. Expected reports are oracles, never proof inputs.
import { assert, readBytes, readText, sha256Hex, runCases } from './provider-support.mjs';

export async function run(cad, memory) {
return await runCases('qualified-analysis', [['main', async () => {
const fixture = JSON.parse(await readText('cad/wasm/tests/fixtures/generated-analysis-contract.json'));
const v2 = fixture.qualificationV2, activation = v2.activation;
const clone = x => JSON.parse(JSON.stringify(x));
const fields = x => Object.keys(x).sort();
const started = performance.now();
const memoryUsage = () => ({wasmMemoryBytes: memory.memory.buffer.byteLength, usedJSHeapSize: performance.memory?.usedJSHeapSize ?? null});
const memoryBefore = memoryUsage();
const context = clone(activation.providerContextExample);
const preparedAt = performance.now();
const plate = cad.analyze_generated_body(fixture.preparedBody,fixture.options);
const preparedMs = performance.now()-preparedAt;
assert.deepEqual(fields(plate), [...activation.directGeneratedRecipe.expectedProviderKeys].sort());
assert(plate.analysisHandle instanceof cad.AnalysisHandle);
assert.throws(()=>new cad.AnalysisHandle().retainedStorageBytes(), /null pointer/);
assert.equal(plate.sourceQualification.state,'bounded');
assert.equal(plate.sourceQualification.directBoundMm,0);
assert.equal(plate.analysisErrorBoundMm,0);
assert.equal(plate.sourceQualification.meshIdentity,plate.analysisMesh.meshIdentity);
assert.deepEqual({...plate.analysisMesh,meshIdentity:activation.descriptors.plane.meshIdentity},activation.descriptors.plane);
const retained = plate.analysisHandle.retainedStorageBytes();
assert(Number.isSafeInteger(retained) && retained > plate.surfacePositionsMmF64.byteLength+plate.triangleFaceOccurrenceIndices.byteLength);
const plateCopy = cad.analyze_generated_body(fixture.preparedBody,{});
assert.notEqual(plate.analysisMesh.meshIdentity,plateCopy.analysisMesh.meshIdentity);

function queryFrom(example,left,right) {
  const binding=clone(example.binding);
  const operands=['left','right'].map((side,i)=>{
    const asset=[left,right][i];
    const reference=binding[side+'Analysis'];
    reference.meshIdentity=asset.analysisMesh.meshIdentity;
    const source=reference.source;
    return {
      modelId:binding[side+'ModelId'],
      ...(source.kind==='vendor-step'?{sourceSha256:source.sourceSha256}:{generatedGeometryIdentity:source.geometryIdentity}),
      importOptions:source.kind==='vendor-step'?clone(source.importOptions):{},
      rowTransformF64:binding[side+'RowTransformF64'],
      analysisRef:reference,analysisMesh:clone(asset.analysisMesh),
    };
  });
  return {left:operands[0],right:operands[1],clearanceThresholdMm:binding.clearanceThresholdMm,requestedAccuracyMm:binding.requestedAccuracyMm,qualificationLimits:clone(example.limits)};
}
function buffers(left,right) {
  return {leftPositions:left.surfacePositionsMmF64.slice(),leftTags:left.triangleFaceOccurrenceIndices.slice(),rightPositions:right.surfacePositionsMmF64.slice(),rightTags:right.triangleFaceOccurrenceIndices.slice()};
}
function invoke(left,right,metadata,buf=buffers(left,right),ctx=context,leftHandle=left.analysisHandle,rightHandle=right.analysisHandle) {
  return cad.query_qualified_collision_mesh_pair(buf,metadata,ctx,leftHandle,rightHandle);
}
function checked(left,right,metadata) {
  const response=invoke(left,right,metadata);
  assert.deepEqual(fields(response),[...activation.qualifiedProviderKeys].sort());
  const q=response.queryQualification;
  assert.deepEqual(fields(q),fields(activation.generatedPlaneQuery));
  assert.equal(q.algorithm,'aabb-interval-exact-v1'); assert.equal(q.version,1);
  assert.deepEqual(q.limits,metadata.qualificationLimits);
  assert.deepEqual(q.binding.snapshot,context.snapshot);
  assert.deepEqual(q.binding.leftAnalysis,metadata.left.analysisRef);
  assert.deepEqual(q.binding.rightAnalysis,metadata.right.analysisRef);
  assert.deepEqual(q.binding.leftMesh,left.analysisMesh); assert.deepEqual(q.binding.rightMesh,right.analysisMesh);
  assert.deepEqual(q.binding.leftRowTransformF64,metadata.left.rowTransformF64);
  assert.deepEqual(q.binding.rightRowTransformF64,metadata.right.rowTransformF64);
  return response;
}
const planeQuery=queryFrom(activation.generatedPlaneQuery,plate,plate);
const queryAt=performance.now();
const plane=checked(plate,plate,planeQuery);
const queryMs=performance.now()-queryAt;
assert.equal(plane.observation.kind,'surface-separated');
assert.equal(plane.observation.surfaceDistanceMm,1);
const planeEvidence=plane.queryQualification;
for(const [override,reason] of [[{maxTriangles:1},'triangle cap'],[{maximumExponent:0},'exponent cap']]) {
  const capped=clone(planeQuery);Object.assign(capped.qualificationLimits,override);
  const response=checked(plate,plate,capped),q=response.queryQualification;
  assert.equal(response.observation.kind,'surface-separated');assert.equal(response.observation.surfaceDistanceMm,1);
  const states=[q.meshDistance,...q.sourceBounds,q.contactRegion,q.containment];
  for(const state of states) assert.equal(state.state,'unknown',`${reason}: no certificate survives preflight refusal`);
  for(const state of states) assert.deepEqual(state.reasons,[reason]);
}

const changedRow=clone(planeQuery);changedRow.right.rowTransformF64[14]=6;
const changed=checked(plate,plate,changedRow).queryQualification;
assert(changed.meshDistance.lowerMm<=4 && changed.meshDistance.upperMm>=4,'a different valid physical row is evaluated once');

assert.equal(planeEvidence.meshDistance.state,'bounded');
assert(planeEvidence.meshDistance.lowerMm<=1 && planeEvidence.meshDistance.upperMm>=1);
assert(planeEvidence.meshDistance.upperMm-planeEvidence.meshDistance.lowerMm<=1e-10);
assert.equal(planeEvidence.containment.method,'slab'); assert.equal(planeEvidence.containment.axis,2);
assert.equal(planeEvidence.contactRegion.state,'boxes');
assert(planeEvidence.sourceBounds.every(b=>b.totalBoundMm===0));
// Resolve actual emitted indices; canonical [2,0] is independently checked below.
function triangle(asset,index,row) {
  return Array.from({length:3},(_,v)=>{
    const p=Array.from(asset.surfacePositionsMmF64.slice(index*9+v*3,index*9+v*3+3));
    return [0,1,2].map(i=>row[i]*p[0]+row[i+4]*p[1]+row[i+8]*p[2]+row[i+12]);
  });
}
const witness=planeEvidence.meshDistance.witnessTriangles;
const leftTri=triangle(plate,witness[0],planeQuery.left.rowTransformF64);
const rightTri=triangle(plate,witness[1],planeQuery.right.rowTransformF64);
assert(leftTri.every(p=>p[2]<=2)); assert(rightTri.every(p=>p[2]>=3));
assert(leftTri.some(a=>rightTri.some(b=>a[0]===b[0] && a[1]===b[1] && b[2]-a[2]===1)),'actual feasible common-XY upper witness');
const topIndex=Array.from(plate.triangleFaceOccurrenceIndices,(_,i)=>i).find(i=>triangle(plate,i,planeQuery.left.rowTransformF64).every(p=>p[2]===2));
const bottomIndex=Array.from(plate.triangleFaceOccurrenceIndices,(_,i)=>i).find(i=>triangle(plate,i,planeQuery.right.rowTransformF64).every(p=>p[2]===3));
assert(Number.isInteger(topIndex) && Number.isInteger(bottomIndex),'resolve the canonical plane seed from actual arrays');
const canonical={surfacePositionsMmF64:new Float64Array(fixture.directPayload.surfacePositionsMmF64)};
const canonicalLeft=triangle(canonical,2,planeQuery.left.rowTransformF64);
const canonicalRight=triangle(canonical,0,planeQuery.right.rowTransformF64);
assert(canonicalLeft.every(p=>p[2]===2) && canonicalRight.every(p=>p[2]===3));
assert(canonicalLeft.some(a=>canonicalRight.some(b=>a[0]===b[0] && a[1]===b[1])));
function pointOnTriangle(p,t) {
  const u=t[1].map((x,i)=>x-t[0][i]),v=t[2].map((x,i)=>x-t[0][i]);
  const n=[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]];
  assert.equal(n.reduce((sum,x,i)=>sum+x*(p[i]-t[0][i]),0),0);
  const drop=n.map(Math.abs).indexOf(Math.max(...n.map(Math.abs))),axes=[0,1,2].filter(i=>i!==drop);
  const cross=(a,b,c)=>(b[axes[0]]-a[axes[0]])*(c[axes[1]]-a[axes[1]])-(b[axes[1]]-a[axes[1]])*(c[axes[0]]-a[axes[0]]);
  const signs=[cross(t[0],t[1],p),cross(t[1],t[2],p),cross(t[2],t[0],p)];
  assert(signs.every(x=>x>=0)||signs.every(x=>x<=0));
}
const rawWitness=plane.observation.closestPoints;
pointOnTriangle(rawWitness.point1Mm,triangle(plate,rawWitness.triangles.left.triangleIndex,planeQuery.left.rowTransformF64));
pointOnTriangle(rawWitness.point2Mm,triangle(plate,rawWitness.triangles.right.triangleIndex,planeQuery.right.rowTransformF64));

assert.equal(activation.directVendorRecipe.originalAsset,'ergogen/library/vendor/infused-kim/3d_models/SMD_0805_Capacitor.step');
const sourceBytes=await readBytes('ergogen/library/vendor/infused-kim/3d_models/SMD_0805_Capacitor.step');
const sourceSha256=await sha256Hex(sourceBytes);
const vendorAt=performance.now();
const vendorRoot=cad.read_step_model(new Uint8Array(sourceBytes),{sourceSha256});
const vendorPrepareMs=performance.now()-vendorAt;
assert.deepEqual(fields(vendorRoot),[...activation.directVendorRecipe.expectedRootKeys].sort());
const vendor=vendorRoot.vendorStep;
assert.deepEqual(fields(vendor),[...activation.directVendorRecipe.expectedVendorStepKeys].sort());
assert.equal(vendor.sourceFaces.length,v2.expectedCensus.faces);
assert.equal(vendor.faceOccurrences.length,v2.expectedCensus.faceOccurrences);
assert.equal(new Set(vendor.faceOccurrences.map(f=>f.solidOccurrenceIndex)).size,4);
assert.equal(vendor.sourceQualification.state,'bounded');
assert.equal(vendor.sourceQualification.sourcePrecisionMm,0.01);
assert(vendor.sourceQualification.directBoundMm<=v2.maximumDirectBoundMm);
assert(vendor.sourceQualification.totalBoundMm>=0.01 && vendor.sourceQualification.totalBoundMm<0.01000001);
assert.equal(vendor.accuracy.analysisErrorBoundMm,vendor.sourceQualification.totalBoundMm);
assert.equal(vendor.accuracy.maxRepairDisplacementMm,null);
assert(vendor.faceOccurrences.every(f=>f.repairDisplacementMm===null));
const vendorQuery=queryFrom(v2.queryExamples,vendor,vendor);
const vendorQueryAt=performance.now();
const vendorPair=checked(vendor,vendor,vendorQuery).queryQualification;
const vendorQueryMs=performance.now()-vendorQueryAt;
assert.equal(vendorPair.meshDistance.state,'bounded');
assert(vendorPair.meshDistance.lowerMm<1+1e-10 && vendorPair.meshDistance.upperMm>1-1e-10);
assert.equal(vendorPair.containment.method,'slab'); assert.equal(vendorPair.containment.axis,1);
assert.equal(vendorPair.contactRegion.state,'boxes');
for(const b of vendorPair.sourceBounds) {
  assert.equal(b.state,'bounded'); assert.equal(b.sourcePrecisionMm,0.01);
  assert(b.totalBoundMm>=b.directBoundMm+b.sourcePrecisionMm+b.transformRoundingMm);
  assert(b.totalBoundMm<0.01000001,'source precision counted once');
}
function accuracy(q) {
  if(q.meshDistance.state!=='bounded'||q.sourceBounds.some(b=>b.state!=='bounded')||q.contactRegion.state!=='boxes')return Infinity;
  return (q.meshDistance.upperMm-q.meshDistance.lowerMm)/2+q.sourceBounds[0].totalBoundMm+q.sourceBounds[1].totalBoundMm;
}
assert(accuracy(vendorPair)<=0.1 && accuracy(vendorPair)>0.01);
const wrongDerivation=clone(vendorQuery);wrongDerivation.left.analysisRef.derivationIdentity+='x';assert.throws(()=>invoke(vendor,vendor,wrongDerivation));
const tight=clone(vendorQuery); tight.requestedAccuracyMm=0.01;
assert(accuracy(checked(vendor,vendor,tight).queryQualification)>0.01,'tight budget remains Unknown in assessment');
const unsupportedBytes=await readBytes('cad/monstertruck/tests/fixtures/touching-unit-boxes.step');
const unsupportedSha=await sha256Hex(unsupportedBytes);
const unsupported=cad.read_step_model(new Uint8Array(unsupportedBytes),{sourceSha256:unsupportedSha}).vendorStep;
assert.equal(unsupported.sourceQualification.state,'unknown');
assert(unsupported.triangleFaceOccurrenceIndices.length>1);
const unsupportedQuery=queryFrom(v2.queryExamples,unsupported,unsupported);
for(const side of ['left','right']) {
  unsupportedQuery[side].sourceSha256=unsupportedSha;
  unsupportedQuery[side].analysisRef.source.sourceSha256=unsupportedSha;
  unsupportedQuery[side].analysisRef.derivationIdentity='vendor-step-analysis-v2/'+unsupportedSha;
}
unsupportedQuery.qualificationLimits.maxTriangles=1;
const unsupportedResponse=checked(unsupported,unsupported,unsupportedQuery);
const rawMetadata=clone(unsupportedQuery);delete rawMetadata.qualificationLimits;
for(const side of ['left','right']) {delete rawMetadata[side].analysisRef;delete rawMetadata[side].analysisMesh;}
const rawUnsupported=cad.query_collision_mesh_pair(unsupported.surfacePositionsMmF64,unsupported.triangleFaceOccurrenceIndices,unsupported.surfacePositionsMmF64,unsupported.triangleFaceOccurrenceIndices,rawMetadata);
assert.equal(rawUnsupported.kind,'surface-separated');
assert.deepEqual(unsupportedResponse.observation,rawUnsupported);
for(const state of [unsupportedResponse.queryQualification.meshDistance,...unsupportedResponse.queryQualification.sourceBounds,unsupportedResponse.queryQualification.contactRegion,unsupportedResponse.queryQualification.containment]) {
  assert.deepEqual(state,{state:'unknown',reasons:['triangle cap']});
}
unsupported.analysisHandle.free();
const reverse=clone(vendorQuery); [reverse.left.rowTransformF64,reverse.right.rowTransformF64]=[reverse.right.rowTransformF64,reverse.left.rowTransformF64];
assert.equal(checked(vendor,vendor,reverse).queryQualification.containment.state,'unknown');
const reduced=clone(vendorQuery); reduced.qualificationLimits.maxExactPairs=1;
assert.equal(checked(vendor,vendor,reduced).queryQualification.meshDistance.state,'unknown');
const exponent=clone(planeQuery); exponent.qualificationLimits.minimumExponent=2;
assert.equal(checked(plate,plate,exponent).queryQualification.meshDistance.state,'unknown');
function rotateRow(row,c,s) {
  const out=row.slice();
  for(const i of [0,4,8,12]) {out[i]=c*row[i]-s*row[i+1];out[i+1]=s*row[i]+c*row[i+1];}
  return out;
}
for(const [c,s,slab] of [[v2.rotatedControl.cos30F64,v2.rotatedControl.sin30F64,true],[Math.SQRT1_2,Math.SQRT1_2,false]]) {
  const rotated=clone(vendorQuery);
  for(const side of ['left','right'])rotated[side].rowTransformF64=rotateRow(rotated[side].rowTransformF64,c,s);
  const result=checked(vendor,vendor,rotated).queryQualification;
  assert.equal(result.meshDistance.state,'bounded');
  assert.equal(result.containment.state,slab?'ruledOut':'unknown');
  assert(result.sourceBounds.every(b=>b.state==='bounded' && b.normUpper>=1));
}
const mixed=checked(vendor,plate,queryFrom(activation.mixedVendorGeneratedQuery,vendor,plate)).queryQualification;
assert.equal(mixed.meshDistance.state,'bounded'); assert.equal(mixed.containment.method,'slab'); assert.equal(mixed.containment.axis,2);
assert(accuracy(mixed)<=0.1);

for(const mutate of [
 q=>q.left.analysisMesh.meshIdentity+='x',q=>q.left.analysisRef.meshIdentity+='x',q=>q.left.analysisRef.workerGeneration+='x',
 q=>q.left.analysisMesh.settings.angularRad=1,q=>q.left.analysisMesh.sourceLimits.maxFinalTriangles=9999,
 q=>q.left.analysisRef.source.revision++,q=>q.left.analysisRef.source.bodyId+='x',q=>q.left.analysisRef.derivationIdentity+='x',
 q=>q.left.rowTransformF64[0]=NaN,q=>q.left.rowTransformF64[15]=0,q=>q.qualificationLimits.maxExactPairs=0,
 q=>q.qualificationLimits.maxTriangles=4097,q=>q.qualificationLimits.minimumExponent=-129,q=>q.qualificationLimits.maxContactBoxes=1.5,
 q=>q.qualificationLimits.requestedNumericalWidthMm=NaN,q=>q.qualificationLimits.requestedNumericalWidthMm=1e-8,q=>q.sourceQualification={state:'bounded'},
 q=>q.left.analysisHandle=plate.analysisHandle,q=>q.left.analysisRef.source.extra=true,q=>q.left.analysisMesh.settings.extra=true,
 q=>q.requestedAccuracyMm=0,q=>delete q.requestedAccuracyMm,q=>q.clearanceThresholdMm=-1,
 q=>q.left.sourceSha256=sourceSha256,
]) {const query=clone(planeQuery);mutate(query);assert.throws(()=>invoke(plate,plate,query));}
for(const key of activation.rejectedQueryKeys) {
  // These are forbidden page-owned proof assertions, wherever supplied.
  if(typeof key==='string'){const query=clone(planeQuery);query[key]={};assert.throws(()=>invoke(plate,plate,query));}
}
assert(globalThis.crossOriginIsolated, 'SharedArrayBuffer rejection requires a cross-origin isolated page');
assert.equal(typeof SharedArrayBuffer, 'function', 'SharedArrayBuffer must be available');
for(const mutate of [
 b=>b.leftPositions[0]+=1,b=>b.leftPositions[0]=-0,b=>b.leftPositions[0]=NaN,b=>b.leftTags[0]++,
 b=>b.leftPositions=new Float32Array(b.leftPositions),b=>b.leftTags=new Int32Array(b.leftTags),
 b=>b.leftPositions=new Float64Array(0),b=>b.extra=true,
 b=>{const shared=new Float64Array(new SharedArrayBuffer(b.leftPositions.byteLength));shared.set(b.leftPositions);b.leftPositions=shared;},
 b=>structuredClone(b.leftPositions.buffer,{transfer:[b.leftPositions.buffer]}),
]) {const buf=buffers(plate,plate);mutate(buf);assert.throws(()=>invoke(plate,plate,clone(planeQuery),buf));}
assert.throws(()=>invoke(plate,plate,clone(planeQuery),buffers(plate,plate),context,plateCopy.analysisHandle));
assert.throws(()=>invoke(plate,plate,clone(planeQuery),buffers(plate,plate),context,vendor.analysisHandle));
for(const mutate of [c=>c.snapshot.token='18446744073709551616',c=>c.snapshot.sessionEpoch='02',c=>c.snapshot.revision++,c=>c.snapshot.extra=0,c=>c.extra=0]) {
 const ctx=clone(context);mutate(ctx);assert.throws(()=>invoke(plate,plate,clone(planeQuery),buffers(plate,plate),ctx));
}
const largeContext=clone(context);largeContext.snapshot.token='18446744073709551615';largeContext.snapshot.sessionEpoch='9007199254740993';
assert.deepEqual(invoke(plate,plate,clone(planeQuery),buffers(plate,plate),largeContext).queryQualification.binding.snapshot,largeContext.snapshot);
// Detachment of the producer's transfer copies cannot detach the sealed owner.
const copied=buffers(plate,plate), costBefore=plate.analysisHandle.retainedStorageBytes();
structuredClone(plate.surfacePositionsMmF64.buffer,{transfer:[plate.surfacePositionsMmF64.buffer]});
structuredClone(plate.triangleFaceOccurrenceIndices.buffer,{transfer:[plate.triangleFaceOccurrenceIndices.buffer]});
assert.equal(plate.surfacePositionsMmF64.length,0);
assert.equal(invoke(plate,plate,clone(planeQuery),copied).queryQualification.meshDistance.state,'bounded');
assert.equal(plate.analysisHandle.retainedStorageBytes(),costBefore);

const cylinderBody=clone(fixture.preparedBody);
for(const [pointer,value] of Object.entries(activation.cylinderBudgetCases.preparedBodyRecipe.replace)) {
  const parts=pointer.split('/').slice(1);let parent=cylinderBody;for(const part of parts.slice(0,-1))parent=parent[part];parent[parts.at(-1)]=clone(value);
}
const cylinderTimes=[];
for(const name of ['cylinderDefault','cylinderRefined']) {
 const requested=name==='cylinderDefault'?null:0.01;
 const at=performance.now(),cylinder=cad.analyze_generated_body(cylinderBody,{targetDeflectionMm:requested});
 assert.equal(cylinder.sourceQualification.state,'bounded');
 const expected=activation.sourceReports[name];
 assert(cylinder.sourceQualification.totalBoundMm<=expected.totalBoundMm);
 assert(cylinder.sourceQualification.totalBoundMm>0.01,'selected deflection is not an accuracy certificate');
 const example=clone(activation.generatedPlaneQuery);
 example.binding.leftAnalysis=clone(activation.workerReferences[name]);
 example.binding.rightAnalysis=clone(activation.workerReferences.plane);
 const metadata=queryFrom(example,cylinder,plateCopy);metadata.requestedAccuracyMm=requested;
 const pair=checked(cylinder,plateCopy,metadata).queryQualification;
 if(name==='cylinderDefault') {assert.equal(pair.meshDistance.state,'bounded');assert(accuracy(pair)<=0.1);}
 else assert(accuracy(pair)>0.01);
 cylinderTimes.push({name,prepareAndQueryMs:performance.now()-at,triangles:cylinder.triangleFaceOccurrenceIndices.length,retainedBytes:cylinder.analysisHandle.retainedStorageBytes(),meshDistance:pair.meshDistance.state,accuracyBudgetMm:accuracy(pair)});
 cylinder.analysisHandle.free();
}
console.log(JSON.stringify({profile:'web binding in Chrome',preparedMs,queryMs,vendorPrepareMs,vendorQueryMs,plateRetainedBytes:retained,vendorRetainedBytes:vendor.analysisHandle.retainedStorageBytes(),plateTriangles:plateCopy.triangleFaceOccurrenceIndices.length,vendorTriangles:vendor.triangleFaceOccurrenceIndices.length,limits:activation.queryLimits,cylinderTimes,totalMs:performance.now()-started,memoryBefore,memoryAfter:memoryUsage()}));
plate.analysisHandle.free();plateCopy.analysisHandle.free();vendor.analysisHandle.free();
console.log('qualified analysis: actual handles, SAME fixture activation, pair bands, caps, binding, copies and detachment passed');

}]]);
}

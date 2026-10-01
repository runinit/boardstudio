import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { readStepModel } from '../src/index.ts';

// Independent FreeCAD optimalBoundingBox measurements of the pinned source
// assemblies, in millimeters. Keep the mounting datum and protruding leads.
const assemblies = [
  ['THQWGD001-rotation.stp',[-9.216318757,-8.812124802,-3.5],[9.166318857,9.183731752,20.372216785]],
  ['THQWGD001C-2pin.stp',[-9.064170501,-8.952765450,-3.461776741],[9.554079635,9.042790450,20.066001209]],
  ['THQWGD001C-4pin.stp',[-9.064170501,-8.952765450,-3.461776741],[10.3549875,9.042790450,20.066001209]],
];
for (const [filename,min,max] of assemblies) {
  test(`THQ nominal assembly ${filename} imports its original occupied bounds`, async () => {
    const bytes=await readFile(new URL(`../../ergogen/library/vendor/thqwgd001/3d_models/${filename}`,import.meta.url));
    const imported=await readStepModel(new Uint8Array(bytes));
    const meshMin=[Infinity,Infinity,Infinity],meshMax=[-Infinity,-Infinity,-Infinity];
    imported.mesh.positions.forEach((value,index)=> {
      const axis=index%3;
      meshMin[axis]=Math.min(meshMin[axis],value);
      meshMax[axis]=Math.max(meshMax[axis],value);
    });
    // The kernel reports conservative bounds for these curved solids. Compare
    // the actual imported triangles to independent tight geometry measurements.
    for (let axis=0;axis<3;axis++) {
      assert.ok(Math.abs(meshMin[axis]-min[axis])<0.03,`${filename} mesh minimum axis ${axis}: ${meshMin[axis]}`);
      assert.ok(Math.abs(meshMax[axis]-max[axis])<0.03,`${filename} mesh maximum axis ${axis}: ${meshMax[axis]}`);
      assert.ok(imported.bounds.min[axis]<=meshMin[axis]+1e-6,'Kernel bounds contain imported geometry');
      assert.ok(imported.bounds.max[axis]>=meshMax[axis]-1e-6,'Kernel bounds contain imported geometry');
    }
    assert.ok(imported.mesh.positions.length>10_000);
    assert.equal(imported.mesh.positions.length,imported.mesh.normals.length);
    assert.ok([...imported.mesh.positions].every(Number.isFinite));
    assert.ok(imported.bounds.min[2]<-3,'Leads are not raised to the PCB top plane');
  });
}

const sourceCatalogue=JSON.parse(await readFile(new URL('../../app/src/parts/imported-parts.json',import.meta.url),'utf8'));
const pinFixtures=[
  ['thqwgd001:rotation-reversible','THQWGD001-rotation.stp',['A','B','C']],
  ['thqwgd001:c-2pin-reversible','THQWGD001C-2pin.stp',['A','B','C','1','2']],
  ['thqwgd001:c-4pin-reversible','THQWGD001C-4pin.stp',['A','B','C','1','2']],
];

for(const [definitionId,filename,contacts] of pinFixtures) {
  test(`THQ ${definitionId} STEP lead centers fall at its pinned footprint hole centers`,async()=>{
    const definition=sourceCatalogue.parts.find(({definition})=>definition.id===definitionId).definition;
    const bytes=await readFile(new URL(`../../ergogen/library/vendor/thqwgd001/3d_models/${filename}`,import.meta.url));
    const imported=await readStepModel(new Uint8Array(bytes));
    const protruding=[];
    for(let i=0;i<imported.mesh.positions.length;i+=3) {
      if(imported.mesh.positions[i+2]<-3) protruding.push([imported.mesh.positions[i],imported.mesh.positions[i+1]]);
    }
    const holes=definition.pads.filter(pad=>pad.drill&&contacts.includes(pad.number)&&
      (['A','B','C'].includes(pad.number)?pad.at.x>-6:true));
    assert.ok(holes.length>0);
    for(const hole of holes) {
      const nearest=Math.min(...protruding.map(([x,y])=>Math.hypot(x-hole.at.x,y-hole.at.y)));
      assert.ok(nearest<0.19,`${filename} lead is ${nearest.toFixed(3)} mm from source hole ${hole.id} at ${hole.at.x},${hole.at.y}`);
    }
  });
}

const transformOracle=JSON.parse(await readFile(new URL('./fixtures/thqwgd001-transform-oracle.json',import.meta.url),'utf8'));
for(let modelIndex=0;modelIndex<assemblies.length;modelIndex++) {
  const [filename]=assemblies[modelIndex];
  const sourceModel=transformOracle.models[modelIndex];
  test(`THQ ${filename} front/back model datums match the pinned source oracle`,async()=>{
    const bytes=await readFile(new URL(`../../ergogen/library/vendor/thqwgd001/3d_models/${filename}`,import.meta.url));
    assert.equal(createHash('sha256').update(bytes).digest('hex'), sourceModel.modelSource.sha256);
    const imported=await readStepModel(new Uint8Array(bytes));
    const yaw=transformOracle.pose.yawDegrees*Math.PI/180;
    const {at:[tx,ty],boardThickness}=transformOracle.pose;
    for(const side of ['front','back']) {
      const min=[Infinity,Infinity,Infinity],max=[-Infinity,-Infinity,-Infinity];
      for(let i=0;i<imported.mesh.positions.length;i+=3) {
        const x=imported.mesh.positions[i],y=imported.mesh.positions[i+1],z=imported.mesh.positions[i+2];
        const localX=side==='back'?-x:x;
        const point=[tx+localX*Math.cos(yaw)-y*Math.sin(yaw),ty+localX*Math.sin(yaw)+y*Math.cos(yaw),side==='front'?boardThickness+z:-z];
        point.forEach((value,axis)=>{min[axis]=Math.min(min[axis],value);max[axis]=Math.max(max[axis],value)});
      }
      const expected=sourceModel.placements[side].exactTransformedSolidBounds;
      for(let axis=0;axis<3;axis++) {
        assert.ok(Math.abs(min[axis]-expected.min[axis])<0.03,`${side} mesh minimum axis ${axis}: ${min[axis]}`);
        assert.ok(Math.abs(max[axis]-expected.max[axis])<0.03,`${side} mesh maximum axis ${axis}: ${max[axis]}`);
      }
    }
  });
}

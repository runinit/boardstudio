import assert from 'node:assert/strict';
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

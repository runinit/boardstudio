import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { createInstance } from 'libcascade/single/init';
const directory=resolve(process.argv[2]);
const input=JSON.parse(await readFile(resolve(process.argv[3])));
const oc=await createInstance();
const shapes=[],report=[];
const volume=shape=>{
  using mass=new oc.GProp_GProps();
  oc.BRepGProp.VolumeProperties(shape,mass,true,true,false);
  return mass.Mass();
};
try {
  for(const name of ['base','cutter']){
    oc.FS.writeFile(`/${name}.step`,await readFile(resolve(directory,name+'.step')));
    using reader=new oc.STEPControl_Reader();
    assert.equal(reader.ReadFile(`/${name}.step`),oc.IFSelect_ReturnStatus.IFSelect_RetDone);
    reader.TransferRoots();
    const shape=reader.OneShape();shapes.push(shape);
    using check=new oc.BRepCheck_Analyzer(shape,true,false,true);
    const row={name,valid:check.IsValid(),volume:volume(shape)};
    assert.ok(row.valid&&row.volume>0);report.push(row);
  }
  const points=input.regions[0].outer;
  const area=Math.abs(points.reduce((sum,p,i)=>{const q=points[(i+1)%points.length];return sum+p.x*q.y-q.x*p.y;},0))/2;
  assert.ok(Math.abs(report[0].volume-area*input.body.thickness)<0.01);
  using cut=new oc.BRepAlgoAPI_Cut(...shapes);
  assert.ok(cut.IsDone());
  using result=cut.Shape();
  using check=new oc.BRepCheck_Analyzer(result,true,false,true);
  assert.ok(check.IsValid());
  const expectedVolume=report[0].volume-Math.PI*(input.regions[0].mounts[0].holeDiameter/2)**2*input.body.thickness;
  const row={name:'occtCutOfMonstertruckInputs',done:cut.IsDone(),valid:check.IsValid(),volume:volume(result),expectedVolume};
  assert.ok(Math.abs(row.volume-expectedVolume)<0.01);report.push(row);
  await writeFile(resolve(directory,'independent.json'),JSON.stringify(report,null,2));
  console.log(report);
} finally { for(const shape of shapes)shape.delete(); }

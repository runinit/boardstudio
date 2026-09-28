import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { createInstance } from 'libcascade/single/init';
const directory = resolve(process.argv[2]);
const oc = await createInstance();
const load = async name => {
  oc.FS.writeFile(`/${name}.step`, await readFile(resolve(directory, `${name}.step`)));
  using reader = new oc.STEPControl_Reader();
  assert.equal(reader.ReadFile(`/${name}.step`), oc.IFSelect_ReturnStatus.IFSelect_RetDone);
  reader.TransferRoots();
  return reader.OneShape();
};
const volume = shape => {
  using mass = new oc.GProp_GProps();
  oc.BRepGProp.VolumeProperties(shape, mass, true, true, false);
  return mass.Mass();
};
const report = [];
using reference = await load('control');
for (const variant of (process.env.CAD_EXPERIMENT_VARIANTS ?? 'control,no-edges,no-ids,lean,profiles,split-timing').split(',')) {
  using shape = await load(variant);
  using analyzer = new oc.BRepCheck_Analyzer(shape, true, false, true);
  using explorer = new oc.TopExp_Explorer(shape, oc.TopAbs_ShapeEnum.TopAbs_SOLID);
  let count = 0;
  while (explorer.More()) { count++; explorer.Next(); }
  const differences = [];
  for (const [a, b] of [[reference, shape], [shape, reference]]) {
    using cut = new oc.BRepAlgoAPI_Cut(a, b);
    assert.ok(cut.IsDone());
    using difference = cut.Shape();
    differences.push(volume(difference));
  }
  const row = { variant, valid: analyzer.IsValid(), solids: count, volume: volume(shape), differences };
  report.push(row);
  assert.ok(row.valid, `${variant} BRep`);
  assert.equal(count, 2, `${variant} components`);
  assert.ok(differences.every(v => Math.abs(v) < 0.01), `${variant} material difference`);
}
await writeFile(resolve(directory, 'independent-step.json'), JSON.stringify(report, null, 2));
console.log(`${directory}: ${report.length} independently validated STEP files`);

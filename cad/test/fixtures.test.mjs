import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import test from 'node:test';
import { createInstance } from 'libcascade/single/init';
import { buildAssembly, readStepModel } from '../src/index.ts';
import { prepareAssembly } from './native-prepare.mjs';

const manifests = await Promise.all(['manifest.json', 'followup-manifest.json'].map(async name =>
  JSON.parse(await readFile(new URL(`../bench/fixtures/${name}`, import.meta.url)))));
for (const fixture of manifests.flatMap(manifest => manifest.fixtures)) {
  test(`fixed benchmark fixture ${fixture.id} retains its hash and STEP bounds`, async () => {
    const path = fixture.kind === 'step' ? new URL(`../../${fixture.file}`, import.meta.url) : new URL(`../bench/fixtures/${fixture.file}`, import.meta.url);
    const bytes = await readFile(path);
    assert.equal(createHash('sha256').update(bytes).digest('hex'), fixture.sha256);
    const prepared = fixture.kind === 'assembly' ? prepareAssembly(JSON.parse(bytes)) : undefined;
    if (fixture.expectedCavities) assert.equal(prepared.bodies[0].regions[0].cavities.length, fixture.expectedCavities);
    const step = fixture.kind === 'step' ? bytes : (await buildAssembly(prepared)).step;
    const imported = await readStepModel(new Uint8Array(step));
    const oc = await createInstance();
    const filename = `/fixture-${fixture.id}.step`;
    oc.FS.writeFile(filename, step);
    using reader = new oc.STEPControl_Reader();
    assert.equal(reader.ReadFile(filename), oc.IFSelect_ReturnStatus.IFSelect_RetDone);
    reader.TransferRoots();
    using shape = reader.OneShape();
    if (fixture.kind === 'assembly') {
      using validity = new oc.BRepCheck_Analyzer(shape, true, false, true);
      assert.equal(validity.IsValid(), true, 'exported assembly is a valid BRep');
    }
    using solids = new oc.TopExp_Explorer(shape, oc.TopAbs_ShapeEnum.TopAbs_SOLID);
    let count = 0;
    while (solids.More()) { count++; solids.Next(); }
    assert.equal(count, fixture.expectedSolids);
    using mass = new oc.GProp_GProps();
    oc.BRepGProp.VolumeProperties(shape, mass, true, true, false);
    assert.ok(mass.Mass() > 0);
    if (fixture.expectedVolume) assert.ok(Math.abs(mass.Mass() - fixture.expectedVolume) < 0.1);
    oc.FS.unlink(filename);
    for (const [index, key] of ['min', 'max'].entries()) {
      imported.bounds[key].forEach((value, axis) => assert.ok(Math.abs(value - fixture.expectedBounds[index][axis]) < 0.01,
        `${key} axis ${axis}: ${value} expected ${fixture.expectedBounds[index][axis]}`));
    }
    assert.ok(imported.mesh.positions.length > 0);
    assert.equal(imported.mesh.positions.length, imported.mesh.normals.length);
  });
}

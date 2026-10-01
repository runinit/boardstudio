// Run only after the paired performance window and explicit host handoff.
// Independent QA of the exact STEP downloaded from the final public release.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '../../../..');
const build = path.resolve(process.argv[2] ?? path.join(repo, 'web/target/builds/m1-release-20261001-8f509433'));
const stepPath = path.join(build, 'qa-reviung41.step');
const outputPath = path.join(here, 'readback-final-reviung41.json');
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const bytes = new Uint8Array(await readFile(stepPath));
assert.equal(sha256(bytes), 'f946d69c2ad39abd3f799ca996067d7f89dba7cc361a9516251c9e32307722f4');
assert.equal(new TextDecoder().decode(bytes.slice(0, 20)), 'ISO-10303-21;\nHEADER');

// This invokes the exact public TypeScript readStepModel -> packaged Cadrum WASM API.
const { readStepModel } = await import(pathToFileURL(path.join(repo, 'cad/src/index.ts')).href);
const publicModel = await readStepModel(bytes);
const positions = publicModel.mesh.positions;
assert.ok(positions instanceof Float32Array && positions.length > 0 && positions.length % 9 === 0);
let signedVolume = 0;
const meshBounds = { min: [Infinity, Infinity, Infinity], max: [-Infinity, -Infinity, -Infinity] };
for (let i = 0; i < positions.length; i += 9) {
  const ax = positions[i], ay = positions[i + 1], az = positions[i + 2];
  const bx = positions[i + 3], by = positions[i + 4], bz = positions[i + 5];
  const cx = positions[i + 6], cy = positions[i + 7], cz = positions[i + 8];
  signedVolume += (ax * (by * cz - bz * cy) + ay * (bz * cx - bx * cz) + az * (bx * cy - by * cx)) / 6;
  for (const vertex of [[ax, ay, az], [bx, by, bz], [cx, cy, cz]]) {
    for (let axis = 0; axis < 3; axis++) {
      meshBounds.min[axis] = Math.min(meshBounds.min[axis], vertex[axis]);
      meshBounds.max[axis] = Math.max(meshBounds.max[axis], vertex[axis]);
    }
  }
}
const publicMeshVolumeMm3 = Math.abs(signedVolume);

// Independent STEP parser/mass calculation used by the repository CAD tests.
const requireFromCad = createRequire(path.join(repo, 'cad/package.json'));
const { createInstance } = await import(pathToFileURL(requireFromCad.resolve('libcascade/single/init')).href);
const oc = await createInstance();
const filename = '/m1-final-reviung41.step';
oc.FS.writeFile(filename, bytes);
using reader = new oc.STEPControl_Reader();
const readStatus = reader.ReadFile(filename);
assert.equal(readStatus, oc.IFSelect_ReturnStatus.IFSelect_RetDone);
reader.TransferRoots();
using shape = reader.OneShape();
using mass = new oc.GProp_GProps();
using bounds = new oc.Bnd_Box();
using solids = new oc.TopExp_Explorer(shape, oc.TopAbs_ShapeEnum.TopAbs_SOLID);
oc.BRepGProp.VolumeProperties(shape, mass, true, true, false);
oc.BRepBndLib.AddOptimal(shape, bounds, false, false);
let solidCount = 0;
while (solids.More()) { solidCount++; solids.Next(); }
oc.FS.unlink(filename);

const independent = {
  volumeMm3: mass.Mass(),
  solidCount,
  bounds: {
    min: [bounds.GetXMin(), bounds.GetYMin(), bounds.GetZMin()],
    max: [bounds.GetXMax(), bounds.GetYMax(), bounds.GetZMax()]
  }
};
const acceptedOracle = {
  fixtureJsonSha256: 'b577dd2009fffbf00489cc8d0f2ccc088861f62a7f30af42470d35c11534bdae',
  cadJsSha256: '60f0f79a3647787f4d570c8f68cc491714d4f65d26961c6de410f34c75d2d03e',
  cadWasmSha256: '408f80e7dfafcb0fedac20ad7bcc41b1f7cac399cac3f2f1413b8e5788338719',
  exactBodyMeshVolumeMm3: 284846.3201776872,
  stepReadbackVolumeMm3: 284846.3201793419,
  stepReadbackBoundsMm: {
    min: [-296.3500001, -98.7790001, -6.7500001],
    max: [9.4410001, 25.0050001, 11.4500001]
  }
};
const record = {
  buildId: path.basename(build),
  sourceCommit: '8f509433bacb84935dc1e1bbbb3840cd56b33029',
  step: { path: stepPath, bytes: bytes.byteLength, sha256: sha256(bytes) },
  runtimePackageHashes: {
    cadJs: sha256(await readFile(path.join(build, 'site-root/assets/cad/boardstudio_cadrum_wasm.js'))),
    cadWasm: sha256(await readFile(path.join(build, 'site-root/assets/cad/boardstudio_cadrum_wasm_bg.wasm'))),
    fixtureJson: sha256(await readFile(path.join(build, 'fixtures/reviung41.json')))
  },
  publicReadStepModel: {
    bounds: publicModel.bounds,
    meshBounds,
    vertexCount: positions.length / 3,
    meshVolumeMm3: publicMeshVolumeMm3
  },
  independentStepParser: independent,
  acceptedOracle,
  deltas: {
    publicMeshVsIndependentBrepVolumeMm3: publicMeshVolumeMm3 - independent.volumeMm3,
    independentBrepVsAcceptedReadbackVolumeMm3: independent.volumeMm3 - acceptedOracle.stepReadbackVolumeMm3,
    independentBrepVsAcceptedExactMeshVolumeMm3: independent.volumeMm3 - acceptedOracle.exactBodyMeshVolumeMm3,
    publicBoundsVsAcceptedReadbackBoundsMm: {
      min: publicModel.bounds.min.map((v, i) => v - acceptedOracle.stepReadbackBoundsMm.min[i]),
      max: publicModel.bounds.max.map((v, i) => v - acceptedOracle.stepReadbackBoundsMm.max[i])
    }
  },
  note: 'No expected-value assertions are imposed for parity; compare measured geometry against the accepted oracle and report any mismatch as a limitation.'
};
await writeFile(outputPath, JSON.stringify(record, null, 2) + '\n');
console.log(JSON.stringify(record, null, 2));

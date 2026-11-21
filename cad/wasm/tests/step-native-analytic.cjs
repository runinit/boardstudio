// Actual packaged provider: node this-file /absolute/path/to/package.js --web
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');

(async () => {
  const cad = await require('./load-provider.cjs')(process.argv[2]);
  const repo = path.resolve(__dirname, '../../..');
  const read = (relative) => fs.readFileSync(path.join(repo, relative));
  function importModel(bytes, targetDeflectionMm) {
    return cad.read_step_model(new Uint8Array(bytes), {
      sourceSha256: crypto.createHash('sha256').update(bytes).digest('hex'),
      targetDeflectionMm,
    });
  }

  const bytes = read('ergogen/library/vendor/infused-kim/3d_models/Nice_View.step');
  const coarse = importModel(bytes, 0.1);
  const fine = importModel(bytes, 0.003);
  try {
    for (const result of [coarse, fine]) {
      const asset = result.vendorStep;
      assert.equal(asset.sourceFaces.length, 67);
      assert.equal(asset.faceOccurrences.length, 67);
      assert.equal(asset.sourceFaces.filter(face => face.supportKind === 'CYLINDRICAL_SURFACE').length, 23);
      assert.equal(asset.accuracy.meshClosedOriented, true);
      assert.equal(asset.accuracy.boundsFromSourceGeometry, true);
      assert.equal(asset.accuracy.fallbackMeshedFaces, 0);
      assert.equal(asset.accuracy.maxRepairDisplacementMm, null);
      assert.equal(asset.accuracy.analysisErrorBoundMm, null);
      assert.equal(asset.sourceQualification.state, 'unknown');

      const { surfacePositionsMmF64: points, triangleFaceOccurrenceIndices: tags } = asset;
      assert(points instanceof Float64Array);
      assert(tags instanceof Uint32Array);
      assert(result.mesh.positions instanceof Float32Array);
      assert.equal(points.length, tags.length * 9);
      assert.equal(result.mesh.positions.length, points.length);
      assert(points.every((point, i) => Number.isFinite(point) && Object.is(Math.fround(point), result.mesh.positions[i])));
      assert.deepEqual([...new Set(tags)].sort((a, b) => a - b), Array.from({ length: 67 }, (_, i) => i));

      const annulus = asset.sourceFaces.findIndex(face => face.faceId === '1041');
      const tag = asset.faceOccurrences.findIndex(face => face.sourceFaceIndex === annulus);
      assert(annulus >= 0 && tag >= 0);
      for (let i = 0; i < tags.length; i++) {
        if (tags[i] !== tag) continue;
        const side = (a, b) => {
          const x = points[9 * i + 3 * a];
          const y = points[9 * i + 3 * a + 1];
          return (points[9 * i + 3 * b] - x) * (1.3 - y)
            - (points[9 * i + 3 * b + 1] - y) * (7 - x);
        };
        const sides = [side(0, 1), side(1, 2), side(2, 0)];
        assert(sides.some(v => v > 0) && sides.some(v => v < 0), 'plated hole center stays uncovered');
      }
    }
    assert.deepEqual(fine.bounds, coarse.bounds);
    assert(fine.mesh.positions.length > coarse.mesh.positions.length);
  } finally {
    fine.vendorStep.analysisHandle.free();
    coarse.vendorStep.analysisHandle.free();
  }

  const nrf = read('ergogen/library/vendor/tsuki/3d_models/nrf52840.step');
  const nrfCoarse = importModel(nrf, 0.1);
  const nrfFine = importModel(nrf, 0.001);
  try {
    function checkNrf(result) {
      const asset = result.vendorStep;
      assert.equal(asset.sourceFaces.length, 110);
      assert.equal(asset.faceOccurrences.length, 110);
      assert.equal(asset.accuracy.meshClosedOriented, true);
      assert.equal(asset.accuracy.boundsFromSourceGeometry, true);
      assert.equal(asset.accuracy.fallbackMeshedFaces, 0);
      assert.equal(asset.accuracy.maxRepairDisplacementMm, null);
      assert.equal(asset.accuracy.analysisErrorBoundMm, null);
      assert.equal(asset.sourceQualification.state, 'unknown');
      assert.equal(asset.sourceSha256, crypto.createHash('sha256').update(nrf).digest('hex'));
      const split = asset.sourceFaces.findIndex(face => face.faceId === '2101' && face.supportId === '94');
      const seam = asset.sourceFaces.findIndex(face => face.faceId === '2149' && face.supportId === '96');
      assert(split >= 0 && seam >= 0);
      const splitTag = asset.faceOccurrences.findIndex(face => face.sourceFaceIndex === split);
      const seamTag = asset.faceOccurrences.findIndex(face => face.sourceFaceIndex === seam);
      assert(splitTag >= 0 && seamTag >= 0);
      const { surfacePositionsMmF64: points, triangleFaceOccurrenceIndices: tags } = asset;
      assert.equal(new Set(tags).size, 110);
      let seamUp = 0;
      let seamDown = 0;
      for (let i = 0; i < tags.length; i++) {
        if (tags[i] !== seamTag) continue;
        const triangle = [0, 1, 2].map(j => points.subarray(9 * i + 3 * j, 9 * i + 3 * j + 3));
        for (const [x, y] of triangle) {
          assert(Math.abs(Math.hypot(x - 1.26, y - 16.51) - 0.5) < 1e-9);
        }
        for (let j = 0; j < 3; j++) {
          const [a, b] = [triangle[j], triangle[(j + 1) % 3]];
          // Source edge #1330 is used in both directions on the same cylindrical face.
          if ([a, b].every(([x, y]) => Math.abs(x - 0.76) < 1e-9 && Math.abs(y - 16.51) < 1e-9)) {
            if (a[2] < b[2]) seamUp++;
            if (a[2] > b[2]) seamDown++;
          }
        }
      }
      assert.equal(seamUp, 1);
      assert.equal(seamDown, 1);
      return tags.filter(tag => tag === splitTag).length;
    }
    assert(checkNrf(nrfFine) > checkNrf(nrfCoarse));
    assert.deepEqual(nrfFine.bounds, nrfCoarse.bounds);
    assert.deepEqual(nrfFine.vendorStep.sourceFaces, nrfCoarse.vendorStep.sourceFaces);
    assert.deepEqual(nrfFine.vendorStep.faceOccurrences, nrfCoarse.vendorStep.faceOccurrences);
  } finally {
    nrfFine.vendorStep.analysisHandle.free();
    nrfCoarse.vendorStep.analysisHandle.free();
  }

  const sector = read('cad/monstertruck/tests/fixtures/trimmed-cylinder-sector.step').toString();
  const invalid = sector.replace("CYLINDRICAL_SURFACE('',#20,10.)", "CYLINDRICAL_SURFACE('',#20,11.)");
  assert.notEqual(invalid, sector);
  assert.throws(() => importModel(Buffer.from(invalid), 0.1));
  console.log('PASS packaged STEP analytic preview, split cylinder trims, seam winding, hole and off-support refusal');
})().catch(error => { console.error(error); process.exitCode = 1; });

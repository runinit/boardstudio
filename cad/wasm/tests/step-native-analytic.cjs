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

  const sector = read('cad/monstertruck/tests/fixtures/trimmed-cylinder-sector.step').toString();
  const invalid = sector.replace("CYLINDRICAL_SURFACE('',#20,10.)", "CYLINDRICAL_SURFACE('',#20,11.)");
  assert.notEqual(invalid, sector);
  assert.throws(() => importModel(Buffer.from(invalid), 0.1));
  console.log('PASS packaged STEP analytic preview, refinement, hole and off-support refusal');
})().catch(error => { console.error(error); process.exitCode = 1; });

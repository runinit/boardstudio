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

  const cirque = read('ergogen/library/vendor/vik/3d_models/sadekbaroudi-vik/pcb/per56-cirque-leds/pers60-cirque-leds-pcb.step');
  const cirqueCoarse = importModel(cirque, 0.1);
  const cirqueFine = importModel(cirque, 0.001);
  try {
    for (const result of [cirqueCoarse, cirqueFine]) {
      const asset = result.vendorStep;
      assert.equal(asset.sourceSha256, crypto.createHash('sha256').update(cirque).digest('hex'));
      assert.equal(asset.sourceFaces.length, 36);
      assert.equal(asset.faceOccurrences.length, 36);
      assert(asset.faceOccurrences.every(f => f.solidOccurrenceIndex === 0 && f.representationId === '25'));
      assert.equal(asset.accuracy.meshClosedOriented, true);
      assert.equal(asset.accuracy.boundsFromSourceGeometry, true);
      assert.equal(asset.accuracy.fallbackMeshedFaces, 0);
      assert.equal(asset.accuracy.maxRepairDisplacementMm, null);
      assert.equal(asset.accuracy.analysisErrorBoundMm, null);
      assert.equal(asset.sourceQualification.state, 'unknown');
      const { surfacePositionsMmF64: points, triangleFaceOccurrenceIndices: tags } = asset;
      assert.equal(new Set(tags).size, 36);
      const faceTag = (faceId, supportId) => {
        const source = asset.sourceFaces.findIndex(f => f.faceId === faceId && f.supportId === supportId);
        assert(source >= 0);
        const tag = asset.faceOccurrences.findIndex(f => f.sourceFaceIndex === source);
        assert(tag >= 0);
        return tag;
      };
      const top = faceTag('1219', '84');
      const hole = faceTag('2106', '459');
      // Hole axis from support #459 and shared oriented seam #2111.
      const center = [146.05, -148.59];
      const near = (a, b) => Math.abs(a - b) < 1e-9;
      const seam = [];
      for (let i = 0; i < tags.length; i++) {
        if (tags[i] !== top && tags[i] !== hole) continue;
        const triangle = [0, 1, 2].map(j => points.subarray(9 * i + 3 * j, 9 * i + 3 * j + 3));
        const edges = triangle.map((a, j) => [a, triangle[(j + 1) % 3]]);
        if (tags[i] === top) {
          const sides = edges.map(([a, b]) =>
            (b[0] - a[0]) * (center[1] - a[1]) - (b[1] - a[1]) * (center[0] - a[0]));
          assert(sides.some(v => v > 0) && sides.some(v => v < 0), 'PCB through-hole stays open');
        } else {
          for (const [x, y] of triangle) {
            assert(near(Math.hypot(x - center[0], y - center[1]), 1.1));
          }
          for (const [a, b] of edges) {
            if ([a, b].every(([x, y]) => near(x, 147.15) && near(y, -148.59))) {
              seam.push(b[2] - a[2]);
            }
          }
        }
      }
      assert.deepEqual(seam.sort((a, b) => a - b), [-1.6, 1.6]);
    }
    assert.deepEqual(cirqueFine.bounds, cirqueCoarse.bounds);
    assert.deepEqual(cirqueFine.vendorStep.sourceFaces, cirqueCoarse.vendorStep.sourceFaces);
    assert.deepEqual(cirqueFine.vendorStep.faceOccurrences, cirqueCoarse.vendorStep.faceOccurrences);
    assert(cirqueFine.mesh.positions.length > cirqueCoarse.mesh.positions.length);
  } finally {
    cirqueFine.vendorStep.analysisHandle.free();
    cirqueCoarse.vendorStep.analysisHandle.free();
  }

  const vendorDirectory = 'ergogen/library/vendor/infused-kim/3d_models';
  const reset = read(vendorDirectory + '/Switch_Reset.step');
  const rim = (asset, faceId) => {
    const index = asset.sourceFaces.findIndex(face => face.faceId === faceId && face.solidId === '25');
    const tag = asset.faceOccurrences.findIndex(face => face.sourceFaceIndex === index && face.solidOccurrenceIndex === 5);
    assert(index >= 0 && tag >= 0);
    const points = asset.surfacePositionsMmF64;
    const onRim = ([x, y, z]) => Math.abs(Math.hypot(y - 0.3, z - 1.9) - 0.2) < 1e-9 && Math.abs(x + y + 0.45) < 1e-9;
    const edges = new Set();
    asset.triangleFaceOccurrenceIndices.forEach((face, i) => {
      if (face !== tag) return;
      const tri = [0, 1, 2].map(v => points.subarray(9 * i + 3 * v, 9 * i + 3 * v + 3));
      tri.forEach((a, j) => {
        const b = tri[(j + 1) % 3];
        if (onRim(a) && onRim(b) && !edges.delete(b + '>' + a)) {
          const edge = a + '>' + b;
          assert(!edges.has(edge));
          edges.add(edge);
        }
      });
    });
    return edges;
  };
  const centimetres = Buffer.from(reset.toString('latin1').replaceAll('SI_UNIT(.MILLI.,.METRE.)', 'SI_UNIT(.CENTI.,.METRE.)'), 'latin1');
  assert(!centimetres.equals(reset));
  const scaled = importModel(centimetres, 0.01);
  try {
    for (const [deflection, expectedSegments] of [[0.1, 4], [0.001, 10]]) {
      const result = importModel(reset, deflection);
      const asset = result.vendorStep;
      try {
        assert.equal(asset.sourceSha256, crypto.createHash('sha256').update(reset).digest('hex'));
        assert.equal(asset.sourceQualification.state, 'unknown');
        assert.equal(asset.sourceFaces.length, 326);
        assert.equal(asset.faceOccurrences.length, 326);
        assert.equal(new Set(asset.sourceFaces.map(face => face.solidId)).size, 6);
        assert.equal(new Set(asset.faceOccurrences.map(face => face.solidOccurrenceIndex)).size, 6);
        assert.equal(asset.accuracy.meshClosedOriented, true);
        assert.equal(asset.accuracy.boundsFromSourceGeometry, true);
        const expected = [-2.35, 0, -1.45, 2.35, 1.35, 2.1];
        assert([...result.bounds.min, ...result.bounds.max].every((x, i) => Math.abs(x - expected[i]) < 1e-9));
        const cylinder = rim(asset, '6077');
        const plane = rim(asset, '6180');
        assert.equal(cylinder.size, expectedSegments);
        assert.deepEqual(cylinder, new Set([...plane].map(edge => edge.split('>').reverse().join('>'))));
        if (deflection === 0.001) {
          assert.deepEqual(scaled.vendorStep.triangleFaceOccurrenceIndices, asset.triangleFaceOccurrenceIndices);
          assert.equal(scaled.vendorStep.surfacePositionsMmF64.length, asset.surfacePositionsMmF64.length);
          assert(asset.surfacePositionsMmF64.every((x, i) => Math.abs(10 * x - scaled.vendorStep.surfacePositionsMmF64[i]) < 1e-10));
        }
      } finally {
        asset.analysisHandle.free();
      }
    }
  } finally {
    scaled.vendorStep.analysisHandle.free();
  }
  for (const [pins, count] of [[2, 181], [5, 351]]) {
    const cable = importModel(read(vendorDirectory + '/Molex_Ezmate_Pico_Cable_' + pins + 'pin.step'));
    try {
      assert.equal(cable.vendorStep.faceOccurrences.length, count);
      assert.equal(cable.vendorStep.accuracy.boundsFromSourceGeometry, false);
    } finally {
      cable.vendorStep.analysisHandle.free();
    }
  }
  console.log('PASS packaged reset switch shared elliptical rims, centimetre scaling, and both cable fallbacks');

  // At 0.01 mm the native and vendor triangulations have distinct facet counts.
  // The vendor controls emit 6906, 8148, 4032 and 3746 triangles respectively.
  for (const [file, faces, occurrences, triangles, bounds] of [
    [vendorDirectory + '/Switch_Power.step', 143, 5, 750, [-3.8608, -0.508, -2.1844, 3.8608, 1.397, 2.794]],
    ...[['occt-cone-apex', 2, 88], ['occt-frustum-pcurves', 3, 150], ['freecad-notched-frustum', 7, 280]].map(
      ([name, count, triangles]) => ['cad/monstertruck/tests/fixtures/' + name + '.step', count, 1, triangles, [-1, -6, 7, 7, 2, 12]]),
  ]) {
    const source = read(file);
    const result = importModel(source, 0.01);
    const asset = result.vendorStep;
    try {
      assert.equal(asset.sourceSha256, crypto.createHash('sha256').update(source).digest('hex'));
      assert.equal(asset.faceOccurrences.length, faces);
      assert.equal(asset.triangleFaceOccurrenceIndices.length, triangles, file);
      assert.equal(new Set(asset.faceOccurrences.map(face => face.solidOccurrenceIndex)).size, occurrences);
      assert(asset.sourceFaces.some(face => face.supportKind === 'CONICAL_SURFACE'));
      assert.equal(asset.accuracy.boundsFromSourceGeometry, true);
      assert.equal(asset.accuracy.meshClosedOriented, true);
      assert.equal(asset.sourceQualification.state, 'unknown');
      assert([...result.bounds.min, ...result.bounds.max].every((value, i) => Math.abs(value - bounds[i]) < 1e-9));
    } finally {
      asset.analysisHandle.free();
    }
  }
  const panasonic = importModel(read('ergogen/library/vendor/kicad/3d_models/Panasonic_EVQPUL_EVQPUC.step'));
  try {
    assert.equal(panasonic.vendorStep.accuracy.meshClosedOriented, false);
    assert.equal(panasonic.vendorStep.faceOccurrences.length, 155);
  } finally {
    panasonic.vendorStep.analysisHandle.free();
  }
  console.log('PASS packaged conical switch, apex, seamed and notched frustums, and Panasonic fallback');

  const bands = read('cad/monstertruck/tests/fixtures/vik-connector-bands.step');
  // Both routes retain the 15 faces. At these deflections the vendor meshes
  // contain 184 and 12618 triangles, respectively, so this pins native delivery.
  for (const [deflection, triangles] of [[0.1, 128], [0.001, 372]]) {
    const { vendorStep: asset, bounds } = importModel(bands, deflection);
    try {
      assert.equal(asset.faceOccurrences.length, 15);
      assert.equal(new Set(asset.faceOccurrences.map(f => f.solidOccurrenceIndex)).size, 2);
      assert.equal(asset.triangleFaceOccurrenceIndices.length, triangles);
      assert.equal(asset.accuracy.meshClosedOriented, true);
      assert.equal(asset.sourceQualification.state, 'unknown');
      const expected = [7.2, -5.25, 0.2, 7.7, -0.75, 2.1];
      assert([...bounds.min, ...bounds.max].every((v, i) => Math.abs(v - expected[i]) < 1e-10));
    } finally {
      asset.analysisHandle.free();
    }
  }

  const sector = read('cad/monstertruck/tests/fixtures/trimmed-cylinder-sector.step').toString();
  const invalid = sector.replace("CYLINDRICAL_SURFACE('',#20,10.)", "CYLINDRICAL_SURFACE('',#20,11.)");
  assert.notEqual(invalid, sector);
  assert.throws(() => importModel(Buffer.from(invalid), 0.1));
  console.log('PASS packaged STEP analytic preview, surface curves, split cylinder trims, seam winding, holes and off-support refusal');
})().catch(error => { console.error(error); process.exitCode = 1; });

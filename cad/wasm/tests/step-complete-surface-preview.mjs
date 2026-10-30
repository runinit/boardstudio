// Actual packaged provider, exercised in an isolated browser page.
import { assert, decodeText, readBytes, sha256Hex, runCases } from './provider-support.mjs';

export async function run(cad, memory) {
return await runCases('step-complete-surface-preview', [['main', async () => {
const sha = bytes => sha256Hex(bytes);

async function check(bytes, expectedBounds, expectedFaces, solidId, occurrences = 1, unitAreas = false) {
  const original = new Uint8Array(bytes), input = new Uint8Array(bytes), digest = await sha(bytes);
  const text = decodeText(bytes);
  const source = [...text.matchAll(/#(\d+)\s*=\s*ADVANCED_FACE\('[^']*',\([^)]*\),#(\d+),\.[TF]\.\)/g)];
  assert.equal(source.length, expectedFaces);
  const faceSupports = new Map(source.map(match => [match[1], match[2]]));
  assert.equal(faceSupports.size, expectedFaces, 'distinct original source face IDs');
  const result = cad.read_step_model(input, {sourceSha256: digest});
  assert.deepEqual(new Uint8Array(input), original, 'provider preserves transferred original bytes');
  const v = result.vendorStep;
  assert.equal(v.sourceSha256, digest);
  assert.equal(v.accuracy.meshClosedOriented, false);
  assert.equal(v.accuracy.analysisErrorBoundMm, null);
  assert.equal(v.accuracy.maxRepairDisplacementMm, null);
  assert.equal(v.accuracy.previewFloorMm, 0.02);
  assert.equal(v.accuracy.previewDeclaredMultiplier, 2);
  assert.equal(v.sourceFaces.length, expectedFaces);
  assert.equal(v.faceOccurrences.length, expectedFaces * occurrences);
  assert.equal(new Set(v.faceOccurrences.map(face => face.solidOccurrenceIndex)).size, occurrences);
  assert.deepEqual(new Set(v.sourceFaces.map(face => face.faceId)), new Set(faceSupports.keys()));
  for (const face of v.sourceFaces) {
    assert.equal(face.solidId, String(solidId));
    assert.equal(face.shellIndex, 0);
    assert.equal(face.supportId, faceSupports.get(face.faceId));
  }
  const identities = new Set();
  for (const face of v.faceOccurrences) {
    assert(Number.isInteger(face.sourceFaceIndex) && face.sourceFaceIndex >= 0 && face.sourceFaceIndex < expectedFaces);
    assert(Number.isInteger(face.solidOccurrenceIndex) && face.solidOccurrenceIndex >= 0 && face.solidOccurrenceIndex < occurrences);
    assert.equal(face.repairDisplacementMm, null);
    identities.add(`${face.sourceFaceIndex}/${face.solidOccurrenceIndex}`);
  }
  assert.equal(identities.size, expectedFaces * occurrences, 'every original face occurrence exactly once');
  assert(v.surfacePositionsMmF64 instanceof Float64Array);
  assert(v.triangleFaceOccurrenceIndices instanceof Uint32Array);
  assert(result.mesh.positions instanceof Float32Array);
  assert.equal(v.surfacePositionsMmF64.length, 9 * v.triangleFaceOccurrenceIndices.length);
  assert.equal(result.mesh.positions.length, v.surfacePositionsMmF64.length);
  assert.equal(result.mesh.normals.length, result.mesh.positions.length);
  assert(result.mesh.normals.every(Number.isFinite));
  const areas = new Array(v.faceOccurrences.length).fill(0);
  for (let i = 0; i < v.surfacePositionsMmF64.length; i++) {
    assert(Number.isFinite(v.surfacePositionsMmF64[i]));
    assert.equal(result.mesh.positions[i], Math.fround(v.surfacePositionsMmF64[i]));
  }
  for (let triangle = 0; triangle < v.triangleFaceOccurrenceIndices.length; triangle++) {
    const tag = v.triangleFaceOccurrenceIndices[triangle];
    assert(tag < areas.length);
    const p = v.surfacePositionsMmF64.subarray(triangle * 9, triangle * 9 + 9);
    const ab = [0, 1, 2].map(i => p[3 + i] - p[i]), ac = [0, 1, 2].map(i => p[6 + i] - p[i]);
    const area = Math.hypot(ab[1]*ac[2]-ab[2]*ac[1], ab[2]*ac[0]-ab[0]*ac[2], ab[0]*ac[1]-ab[1]*ac[0]) / 2;
    assert(Number.isFinite(area) && area > 0, 'every retained final f64 triangle has positive area');
    areas[tag] += area;
  }
  assert(areas.every(area => area > 0), 'every source face occurrence retains positive coverage');
  if (unitAreas) assert(areas.every(area => Math.abs(area - 1) < 1e-12), 'independent unit-box face areas');
  // Boundary LINE supports are not standalone curve-set display lines.
  assert.equal(v.lineOccurrences.length, 0);
  assert.equal(v.lineSegmentsMmF64.length, 0);
  for (let end = 0; end < 2; end++) {
    result.bounds[end ? 'max' : 'min'].forEach((value, axis) => {
      assert(Number.isFinite(value) && Math.abs(value - expectedBounds[end][axis]) < 1e-6);
    });
  }
  console.log(`PASS complete surface: ${expectedFaces} source faces, ${occurrences} occurrences; positive f64 coverage, original SHA, f32 identity, no solid/accuracy/movement eligibility`);
  v.analysisHandle.free();
}

const panasonic = await readBytes('ergogen/library/vendor/kicad/3d_models/Panasonic_EVQPUL_EVQPUC.step');
assert.equal(panasonic.length, 123401);
assert.equal(await sha(panasonic), 'c30e15d8da5c3856f0a5b19ab62f22eef9b06d69161963e77cdf6c0b09a7ac8c');
assert.equal([...decodeText(panasonic).matchAll(/=LINE\(/g)].length, 353);
await check(panasonic, [[-3.2,-1.75,-0.5],[3.2,2.75,1.65]], 155, 15);
const boxes = await readBytes('cad/monstertruck/tests/fixtures/touching-unit-boxes.step');
await check(boxes, [[0,0,0],[2,2,1]], 12, 2001, 1, true);
console.log('PASS Panasonic V1 untimed functional preview admission; no triangle-count or performance oracle');
}]]);
}

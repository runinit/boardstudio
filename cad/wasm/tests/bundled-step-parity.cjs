// Compare the content build's native emission with the actual web CAD provider.
const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const { readFileSync } = require('node:fs');
const path = require('node:path');
const { gunzipSync } = require('node:zlib');
const loadProvider = require('./load-provider.cjs');

const root = path.resolve(__dirname, '../../..');
const previews = path.join(root, 'web/target/providers/bundled-step');
const models = path.join(root, 'ergogen/library/vendor');
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');

function withoutLease(result) {
  const { analysisHandle, analysisMesh, ...vendorStep } = result.vendorStep;
  const { meshIdentity, ...settings } = analysisMesh;
  analysisHandle.free();
  return { mesh: result.mesh, bounds: result.bounds, vendorStep, settings };
}

async function main() {
  const cad = await loadProvider(path.join(root, 'cad/wasm/pkg/boardstudio_cad_wasm.js'), true);
  const { assets } = JSON.parse(readFileSync(path.join(previews, 'manifest.json')));
  for (const file of [
    'infused-kim/3d_models/Choc_V1_Switch.step',
    'kicad/3d_models/Panasonic_EVQPUJ_EVQPUA.step',
  ]) {
    const source = readFileSync(path.join(models, file));
    const sourceSha256 = sha256(source);
    const digest = assets[sourceSha256];
    assert.match(digest, /^[a-f0-9]{64}$/, file);
    const compressed = readFileSync(path.join(previews, `${digest}.bstep`));
    assert.equal(sha256(compressed), digest);
    const payload = gunzipSync(compressed);
    const baked = withoutLease(cad.read_baked_step_model(payload, sourceSha256));
    const live = withoutLease(cad.read_step_model(source, { sourceSha256 }));
    assert.equal(baked.vendorStep.sourceQualification.state, 'unknown');
    assert.deepEqual(baked, live, file);
    assert.throws(() => cad.read_baked_step_model(payload, '0'.repeat(64)), /Invalid baked STEP/);
    assert.throws(() => cad.read_baked_step_model(payload.subarray(0, -1), sourceSha256), /Invalid baked STEP/);
    console.log(`matched ${file} (${baked.vendorStep.triangleFaceOccurrenceIndices.length} triangles)`);
  }
}

main().catch(error => {
  console.error(error);
  process.exitCode = 1;
});

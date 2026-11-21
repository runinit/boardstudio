// Bake and verify the packaged provider's default preview of selected bundled STEP sources.
const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const {
  mkdirSync, mkdtempSync, readFileSync, readdirSync, renameSync, rmSync, writeFileSync,
} = require('node:fs');
const path = require('node:path');
const { gunzipSync, gzipSync } = require('node:zlib');
const loadProvider = require('../wasm/tests/load-provider.cjs');

const root = path.resolve(__dirname, '../..');
const binding = path.join(root, 'cad/wasm/pkg/boardstudio_cad_wasm.js');
const providers = path.join(root, 'web/target/providers');
const output = path.join(providers, 'bundled-step');
// Keep the first wave to common switches, headers and small board components.
// The two Choc V1 sources also cover the byte-identical KiSwitch catalogue entries.
const compressedBudget = 384 * 1024;
const sources = [
  'infused-kim/3d_models/Choc_V1_Hotswap.step',
  'infused-kim/3d_models/Choc_V1_Switch.step',
  'infused-kim/3d_models/Diode_1N4148W.step',
  'infused-kim/3d_models/Molex_Ezmate_Pico_Cable_2pin.step',
  'infused-kim/3d_models/Molex_Ezmate_Pico_Socket_2pin.step',
  'infused-kim/3d_models/Molex_Ezmate_Pico_Socket_5pin.step',
  'infused-kim/3d_models/Nice_View.step',
  'infused-kim/3d_models/PinHeader_2.54mm_1x-4.step',
  'infused-kim/3d_models/PinHeader_2.54mm_1x-5.step',
  'infused-kim/3d_models/PinSocket_2.54mm_5mm_1x-4.step',
  'infused-kim/3d_models/PinSocket_2.54mm_5mm_1x-5.step',
  'infused-kim/3d_models/SMD_0805_Resistor.step',
  'infused-kim/3d_models/Switch_Power.step',
  'infused-kim/3d_models/Switch_Reset.step',
  'keebio/3d_models/PJ-320A.step',
  'kicad/3d_models/JST_PH_S2B-PH-K_1x02_P2.00mm_Horizontal.step',
  'kicad/3d_models/Panasonic_EVQPUJ_EVQPUA.step',
  'kicad/3d_models/Panasonic_EVQPUL_EVQPUC.step',
  'kicad/3d_models/PinSocket_1x05_P2.54mm_Vertical.step',
  'kicad/3d_models/PinSocket_1x12_P2.54mm_Vertical.step',
  'tsuki/3d_models/nrf52840.step',
];
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const shaPattern = /^[a-f0-9]{64}$/;

function stagedSources() {
  const { destination, entries } = JSON.parse(readFileSync(path.join(providers, 'ergogen-models.json')));
  const models = new Map(entries.filter(entry => entry.mediaType === 'model/step')
    .map(entry => [entry.sourceRelativePath, path.basename(entry.emittedPath)]));
  const selected = new Map();
  for (const file of sources) {
    assert(models.has(file), `${file} is missing from the shipped STEP catalogue`);
    const bytes = readFileSync(path.join(destination, models.get(file)));
    selected.set(sha256(bytes), { file, bytes });
  }
  return selected;
}

function currentAssets(manifest, inputs, selected) {
  try {
    const previous = JSON.parse(readFileSync(manifest));
    if (previous?.inputs !== inputs || !previous.assets ||
        Object.keys(previous.assets).length !== selected.size) return;
    for (const sha of selected.keys()) {
      const digest = previous.assets[sha];
      if (typeof digest !== 'string' || !shaPattern.test(digest) ||
          sha256(readFileSync(path.join(output, `${digest}.bstep`))) !== digest) return;
    }
    return previous.assets;
  } catch (error) {
    if (error.code !== 'ENOENT' && !(error instanceof SyntaxError)) throw error;
  }
}

function withoutLease({ vendorStep, ...result }) {
  const { analysisHandle, analysisMesh, ...metadata } = vendorStep;
  // Mesh identity names the particular live handle and is allocated afresh by both entry points.
  const { meshIdentity, ...settings } = analysisMesh;
  analysisHandle.free();
  return { ...result, vendorStep: { ...metadata, analysisMesh: settings } };
}

function verify(cad, compressed, sourceSha256, { file, bytes }) {
  const payload = gunzipSync(compressed);
  const baked = withoutLease(cad.read_baked_step_model(payload, sourceSha256));
  const live = withoutLease(cad.read_step_model(bytes, { sourceSha256 }));
  assert.equal(baked.vendorStep.sourceQualification.state, 'unknown');
  assert.deepEqual(baked, live, file);
  assert.throws(() => cad.read_baked_step_model(payload, '0'.repeat(64)), /Invalid baked STEP/);
  assert.throws(() => cad.read_baked_step_model(payload.subarray(0, -1), sourceSha256), /Invalid baked STEP/);
  console.log(`cad: matched ${file} (${baked.vendorStep.triangleFaceOccurrenceIndices.length} triangles, ${compressed.length} bytes)`);
}

async function main() {
  const selected = stagedSources();
  const files = [binding, binding.replace(/\.js$/, '_bg.wasm'), __filename,
    require.resolve('../wasm/tests/load-provider.cjs')];
  // The exact packaged binding and module are the authority, including the wasm-opt pass.
  const inputs = sha256(files.map(file => sha256(readFileSync(file))).join('') +
    [...selected.keys()].join(''));
  const manifest = path.join(output, 'manifest.json');
  const current = currentAssets(manifest, inputs, selected);
  const verifyOnly = process.argv.includes('--verify');
  if (current && !verifyOnly) {
    console.log(`cad: ${selected.size} bundled STEP previews are current`);
    return;
  }

  const cad = await loadProvider(binding, true);
  if (verifyOnly) {
    assert(current, 'bundled STEP previews must be current before verification');
    for (const [sha, source] of selected) {
      verify(cad, readFileSync(path.join(output, `${current[sha]}.bstep`)), sha, source);
    }
    return;
  }

  mkdirSync(output, { recursive: true });
  const temporary = mkdtempSync(path.join(output, 'bake-'));
  try {
    const assets = {};
    let bytes = 0;
    for (const [sha, source] of selected) {
      try {
        const payload = cad.bake_bundled_step_model(source.bytes, sha);
        const compressed = gzipSync(payload, { level: 9 });
        verify(cad, compressed, sha, source);
        const digest = sha256(compressed);
        writeFileSync(path.join(temporary, `${digest}.bstep`), compressed);
        assets[sha] = digest;
        bytes += compressed.length;
      } catch (error) {
        throw new Error(`bundled STEP bake failed for ${source.file}`, { cause: error });
      }
    }
    assert(bytes <= compressedBudget, `bundled STEP payloads total ${bytes} bytes, budget is ${compressedBudget}`);
    const wanted = new Set(Object.values(assets).map(digest => `${digest}.bstep`));
    for (const name of wanted) renameSync(path.join(temporary, name), path.join(output, name));
    const stagedManifest = path.join(temporary, 'manifest.json');
    writeFileSync(stagedManifest, JSON.stringify({ inputs, assets }) + '\n');
    renameSync(stagedManifest, manifest);
    for (const name of readdirSync(output)) {
      if (name.endsWith('.bstep') && !wanted.has(name)) rmSync(path.join(output, name));
    }
    console.log(`cad: baked ${selected.size} bundled STEP previews (${bytes} compressed bytes)`);
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
}

main().catch(error => {
  console.error(error);
  process.exitCode = 1;
});

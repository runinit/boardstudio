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
// Wave 1 covers common switches, headers and small board components.
// Wave 2 adds the deferred catalogue paths with value-dense previews:
// whole vendor-fallback component assets plus small connectors and mounts.
// The two Choc V1 sources also cover the byte-identical KiSwitch catalogue entries.
const compressedBudget = 2883584;
const largeSidecarBudget = 256 * 1024;
const sources = [
  // Wave 1.
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
  // Wave 2: keycaps with fallback faces, larger headers and sockets, trackpoint caps
  // and extensions, board connectors, knobs, trackball and trackpad mounts,
  // whole-case vendor assets, display housings and display-adapter glass.
  'infused-kim/3d_models/Choc_V1_Keycap_MBK_Black_1u.step',
  'infused-kim/3d_models/Choc_V1_Keycap_MBK_Red_1u.step',
  'infused-kim/3d_models/Choc_V1_Keycap_MBK_White_1u.step',
  'infused-kim/3d_models/Molex_Ezmate_Pico_Cable_5pin.step',
  'infused-kim/3d_models/PinHeader_2.54mm_2x-12.step',
  'infused-kim/3d_models/PinSocket_2.54mm_5mm_2x-12.step',
  'infused-kim/3d_models/trackpoint/TP_Cap_Green_T430.step',
  'infused-kim/3d_models/trackpoint/TP_Cap_Red_T460S.step',
  'infused-kim/3d_models/trackpoint/TP_Extension_Red_T460S_h10.5_md0.0_pcb1.6.step',
  'infused-kim/3d_models/trackpoint/TP_Extension_Red_T460S_h10.5_md0.5_pcb1.6.step',
  'infused-kim/3d_models/trackpoint/TP_Extension_Red_T460S_h10.5_md1.0_pcb1.6.step',
  'infused-kim/3d_models/trackpoint/TP_Extension_Red_T460S_h10.5_md2.0_pcb1.6.step',
  'kicad/3d_models/PinSocket_2x12_W15.24mm_Vertical.step',
  'vik/3d_models/Ariamelon-Crenshaw/PCB/Crenshaw.pretty/3d/FH12-12S-0.5SH.step',
  'vik/3d_models/Ariamelon-Crenshaw/PCB/Crenshaw.pretty/3d/FH12-30S-0.5SH.step',
  'vik/3d_models/Ariamelon-Kiwano/3DP/Knob Base.step',
  'vik/3d_models/Ariamelon-Kiwano/3DP/Trackball Mount Full.step',
  'vik/3d_models/Ariamelon-Kiwano/3DP/Trackball Mount Low Profile.step',
  'vik/3d_models/Ariamelon-Kiwano/3DP/Trackball Mount Semi.step',
  'vik/3d_models/Ariamelon-Kiwano/3DP/Trackpad Mount.step',
  'vik/3d_models/george-norton-peacock/case/peacock-low-case-v5-5deg-tilt.step',
  'vik/3d_models/george-norton-peacock/case/peacock-low-case-v5.step',
  'vik/3d_models/sadekbaroudi-fingerpunch/3d-models/display-housings/ili9341/ili9341-4inch-cover.step',
  'vik/3d_models/sadekbaroudi-fingerpunch/3d-models/display-housings/ili9341/ili9341-4inch-housing.step',
  'vik/3d_models/sadekbaroudi-vik/kicad/3dmodels/vik-connector-horizontal.stp',
  'vik/3d_models/sadekbaroudi-vik/pcb/per56-cirque-leds/pers60-cirque-leds-pcb.step',
  'vik/3d_models/zzeneg-vik-display-adapter/pcb/1.28inch-round/GC9A01.step',
  'vik/3d_models/zzeneg-vik-display-adapter/pcb/1.47inch/LCD_lcd_ips.stp',
];
// Byte-identical catalogue entries share their canonical source's sidecar.
const aliasSources = {
  'kiswitch/3d_models/SW_Hotswap_Kailh_Choc_V1.stp': 'infused-kim/3d_models/Choc_V1_Hotswap.step',
  'kiswitch/3d_models/SW_Kailh_Choc_V1.stp': 'infused-kim/3d_models/Choc_V1_Switch.step',
};
// Catalogue STEP paths with no sidecar, and why. Import refusals and probe-limit
// timeouts are never attempted; oversize previews carry no fallback value.
const excludedSources = {
  'foostan/3d_models/OLED-Module-with-Pins.step': 'probe limit: source cold import timed out',
  'gdek/3d_models/KS33.stp': 'import refusal: presented VALUE_REPRESENTATION_ITEM #16 is not retained by scene discovery',
  'infused-kim/3d_models/Nice_Nano_V2.step': 'import refusal: Monstertruck produced a degenerate mesh triangle',
  'infused-kim/3d_models/SMD_0805_Capacitor.step': 'eligibility refusal: preview bake requires an unqualified STEP source',
  'infused-kim/3d_models/trackpoint/TP_Keycap_MBK_Black_stagger_2.375_large.step': 'oversize without fallback value: 659829 compressed bytes, 0 fallback faces',
  'infused-kim/3d_models/trackpoint/TP_Keycap_MBK_Black_stagger_2.375_small.step': 'oversize without fallback value: 643411 compressed bytes, 0 fallback faces',
  'infused-kim/3d_models/trackpoint/TP_Red_T460S_platform_z_offset_+0.0_pcb_offset_-2.0.step': 'probe limit: source cold import timed out',
  'infused-kim/3d_models/trackpoint/TP_Red_T460S_screw_mount_z_offset_+0.0_pcb_-2.0.step': 'probe limit: source cold import timed out',
  'infused-kim/3d_models/trackpoint/TP_Red_T460S_screw_mount_z_offset_-2.0_pcb_+0.0.step': 'probe limit: source cold import timed out',
  'keebio/3d_models/SK6812MINI-E v1.step': 'import refusal: source curve #16 is CARTESIAN_POINT, expected TRIMMED_CURVE',
  'kiswitch/3d_models/SW_Cherry_MX_PCB.stp': 'oversize without fallback value: 324924 compressed bytes, 0 fallback faces',
  'kiswitch/3d_models/SW_Hotswap_Kailh_MX.stp': 'probe limit: source cold import timed out',
  'koktoh/3d_models/Choc_V2_Red.step': 'probe limit: source cold import timed out',
  'thqwgd001/3d_models/THQWGD001-rotation.stp': 'eligibility refusal: preview bake does not support source lines',
  'thqwgd001/3d_models/THQWGD001C-2pin.stp': 'oversize without fallback value: 332143 compressed bytes, 0 fallback faces',
  'thqwgd001/3d_models/THQWGD001C-4pin.stp': 'oversize without fallback value: 333768 compressed bytes, 0 fallback faces',
  'vik/3d_models/Ariamelon-Crenshaw/Showcase/Crenshaw.step': 'probe limit: source cold import timed out',
  'vik/3d_models/Ariamelon-Kiwano/3DP/Knob Full.step': 'probe limit: source cold import timed out',
  'vik/3d_models/Ariamelon-Kiwano/3DP/Knob Low Profile.step': 'probe limit: source cold import timed out',
  'vik/3d_models/Ariamelon-Kiwano/3DP/Knob Ribbed.step': 'oversize without fallback value: 310326 compressed bytes, 0 fallback faces',
  'vik/3d_models/Ariamelon-Kiwano/Showcase/Kiwano.step': 'probe limit: source cold import timed out',
  'vik/3d_models/christrotter-shift-register-spi-breakout-pcb/matrix-inator-rev2.step': 'probe limit: source cold import timed out',
  'vik/3d_models/sadekbaroudi-fingerpunch/keyboards/vikropad/cases/choc-glp/vikropad-choc-glp-xivik.step': 'probe limit: source cold import timed out',
  'vik/3d_models/sadekbaroudi-vik/kicad/3dmodels/vik-connector-horizontal-mini.step': 'eligibility refusal: preview bake requires an unqualified STEP source',
  'vik/3d_models/sadekbaroudi-vik/pcb/vik-playground/vik-playground.step': 'probe limit: source cold import timed out',
  'vik/3d_models/zzeneg-vik-display-adapter/pcb/1.28inch-round/1.28inch-round.step': 'probe limit: source cold import timed out',
  'vik/3d_models/zzeneg-vik-display-adapter/pcb/1.47inch/1.47inch.step': 'probe limit: source cold import timed out',
  'vik/3d_models/zzeneg-vik-display-adapter/pcb/2inch-0.8mm/2inch-0.8mm.step': 'probe limit: source cold import timed out',
};
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
  assert.equal(selected.size, sources.length, 'bundled STEP sources must not repeat source bytes');
  for (const [alias, canonical] of Object.entries(aliasSources)) {
    assert(models.has(alias), `${alias} is missing from the shipped STEP catalogue`);
    assert(sources.includes(canonical), `${alias} names an unbaked canonical source`);
    const aliasBytes = readFileSync(path.join(destination, models.get(alias)));
    const canonicalSha = [...selected.entries()]
      .find(([, source]) => source.file === canonical)?.[0];
    assert.equal(sha256(aliasBytes), canonicalSha, `${alias} is no longer byte-identical to ${canonical}`);
  }
  const covered = new Set([...sources, ...Object.keys(aliasSources), ...Object.keys(excludedSources)]);
  assert.deepEqual([...models.keys()].sort(), [...covered].sort(),
    'bundled STEP selection must account for every shipped catalogue path');
  for (const file of Object.keys(excludedSources)) {
    assert(models.has(file), `${file} left the shipped STEP catalogue; drop its exclusion`);
    assert(!sources.includes(file), `${file} is both baked and excluded`);
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
  const fallbackFaces = baked.vendorStep.accuracy.fallbackMeshedFaces;
  console.log(`cad: matched ${file} (${baked.vendorStep.triangleFaceOccurrenceIndices.length} triangles, ${compressed.length} bytes, ${fallbackFaces} fallback faces)`);
  return { fallbackFaces, triangles: baked.vendorStep.triangleFaceOccurrenceIndices.length };
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
        const { fallbackFaces } = verify(cad, compressed, sha, source);
        assert(compressed.length <= largeSidecarBudget || fallbackFaces > 0,
          `bundled STEP preview for ${source.file} is ${compressed.length} bytes without fallback value`);
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

import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { cpus } from 'node:os';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { emptyProject } from '../../../../contracts/src/index.ts';

const PITCH_MM = 19.05;
const counts = [30, 100, 200];

// Kept byte-for-byte equivalent in shape to app/src/bench-workbench.tsx's
// reference fixture(): same three part definitions, positions, nets, board,
// switch-envelope outline, and 5 / 10 / 20 columns. This creates benchmark
// inputs only; all open, edit, history, rendering, and persistence behavior is
// executed by the application under test.
function referenceFixture(keys) {
  const definitions = [
    {
      id: 'bench-switch', name: 'MX switch', kind: 'switch',
      courtyard: [{ x: -7, y: -7 }, { x: 7, y: -7 }, { x: 7, y: 7 }, { x: -7, y: 7 }],
      pads: [
        { id: '1', number: '1', at: { x: -3, y: 0 }, size: { x: 2, y: 2 }, shape: 'circle' },
        { id: '2', number: '2', at: { x: 3, y: 0 }, size: { x: 2, y: 2 }, shape: 'circle' },
      ],
    },
    {
      id: 'bench-diode', name: 'SMD diode', kind: 'passive',
      courtyard: [{ x: -2, y: -1 }, { x: 2, y: -1 }, { x: 2, y: 1 }, { x: -2, y: 1 }],
      pads: [
        { id: '1', number: '1', at: { x: -1.5, y: 0 }, size: { x: 1, y: 1 }, shape: 'rect' },
        { id: '2', number: '2', at: { x: 1.5, y: 0 }, size: { x: 1, y: 1 }, shape: 'rect' },
      ],
    },
    {
      id: 'bench-led', name: 'RGB LED', kind: 'passive',
      courtyard: [{ x: -2, y: -2 }, { x: 2, y: -2 }, { x: 2, y: 2 }, { x: -2, y: 2 }],
      pads: [
        { id: '1', number: '1', at: { x: -1, y: -1 }, size: { x: 0.8, y: 0.8 }, shape: 'rect' },
        { id: '2', number: '2', at: { x: 1, y: -1 }, size: { x: 0.8, y: 0.8 }, shape: 'rect' },
        { id: '3', number: '3', at: { x: -1, y: 1 }, size: { x: 0.8, y: 0.8 }, shape: 'rect' },
        { id: '4', number: '4', at: { x: 1, y: 1 }, size: { x: 0.8, y: 0.8 }, shape: 'rect' },
      ],
    },
  ];
  const document = emptyProject(`workbench-bench-${keys}`, `${keys} key assembly benchmark`);
  const columns = keys === 30 ? 5 : keys === 100 ? 10 : 20;
  const rows = keys / columns;
  const parts = [];
  const pins = new Map();
  const switches = [];
  const connect = (name, partId, padId) => {
    const list = pins.get(name) ?? [];
    list.push({ partId, padId });
    pins.set(name, list);
  };

  for (let row = 0; row < rows; row += 1) {
    for (let col = 0; col < columns; col += 1) {
      const index = row * columns + col;
      const x = col * PITCH_MM;
      const y = -row * PITCH_MM;
      const keyId = `key-${row}-${col}`;
      const diodeId = `diode-${row}-${col}`;
      const ledId = `led-${row}-${col}`;

      switches.push(keyId);
      parts.push({ id: keyId, definitionId: 'bench-switch', reference: `SW${index + 1}`, pose: { at: { x, y }, rotation: 0 }, side: 'front' });
      parts.push({ id: diodeId, definitionId: 'bench-diode', reference: `D${index + 1}`, pose: { at: { x: x + 7, y: y + 5 }, rotation: 0 }, side: 'front' });
      parts.push({ id: ledId, definitionId: 'bench-led', reference: `LED${index + 1}`, pose: { at: { x: x - 5, y: y + 5 }, rotation: 0 }, side: 'front' });

      connect(`ROW${row}`, keyId, '1');
      connect(`KEY${index}`, keyId, '2');
      connect(`KEY${index}`, diodeId, '1');
      connect(`COL${col}`, diodeId, '2');
      connect('VDD', ledId, '1');
      connect('GND', ledId, '2');
      connect(`LED${index}`, ledId, '3');
      connect(`LED${index + 1}`, ledId, '4');
    }
  }

  document.definitions = definitions;
  document.parts = parts;
  document.nets = [...pins].map(([name, netPins]) => ({ id: name, name, pins: netPins }));
  document.outline = [{ id: 'bench-outline', kind: 'part-envelope', partIds: switches, margin: 4, operation: 'add' }];
  document.boards = [{ id: 'bench-board', name: 'Benchmark board', outlineIds: ['bench-outline'], partIds: parts.map((part) => part.id), netIds: document.nets.map((net) => net.id), thickness: 1.6 }];
  return document;
}

const modulePath = process.env.BOARDSTUDIO_CORE_MODULE;
const wasmPath = process.env.BOARDSTUDIO_CORE_WASM;
if (!modulePath || !wasmPath) {
  throw new Error('Set BOARDSTUDIO_CORE_MODULE and BOARDSTUDIO_CORE_WASM to the built public boardstudio_core JS/WASM artifacts. This script does not build them.');
}

const core = await import(pathToFileURL(resolve(modulePath)).href);
const coreWasm = await readFile(wasmPath);
core.initSync({ module: coreWasm });
const fixtureId = `${new Date().toISOString().replaceAll(':', '-').replaceAll('.', '-')}-${process.pid}-${createHash('sha256').update(coreWasm).digest('hex').slice(0, 12)}`;
const outputDir = resolve(process.env.BOARDSTUDIO_FIXTURE_OUTPUT ?? `.scratch/m1-production/evidence/performance/fixtures/${fixtureId}`);
await mkdir(outputDir, { recursive: true });
const generatorSource = await readFile(new URL(import.meta.url), 'utf8');
const referenceSource = await readFile(new URL('../../../../app/src/bench-workbench.tsx', import.meta.url));
const contractsSource = await readFile(new URL('../../../../contracts/src/index.ts', import.meta.url));
const manifest = {
  source: 'app/src/bench-workbench.tsx referenceFixture shape, copied for test-input creation',
  generatorSha256: createHash('sha256').update(generatorSource).digest('hex'),
  referenceFixtureSourceSha256: createHash('sha256').update(referenceSource).digest('hex'),
  contractsSourceSha256: createHash('sha256').update(contractsSource).digest('hex'),
  coreModuleSha256: createHash('sha256').update(await readFile(modulePath)).digest('hex'),
  coreWasmSha256: createHash('sha256').update(coreWasm).digest('hex'),
  sourceCommit: process.env.BOARDSTUDIO_SOURCE_COMMIT ?? 'unreported',
  argv: process.argv,
  cwd: process.cwd(),
  node: process.version,
  cpu: cpus()[0]?.model ?? 'unknown',
  startedAt: new Date().toISOString(),
  generatedAt: new Date().toISOString(),
  fixtures: [],
};

try {
  for (const keys of counts) {
    const document = referenceFixture(keys);
    assert.equal(document.parts.length, keys * 3);
    assert.equal(document.boards[0].partIds.length, keys * 3);
    const engine = new core.CoreEngine();
    let coreOpenReply;
    try {
      const opened = JSON.parse(engine.request(JSON.stringify({ id: `open-${keys}`, kind: 'open', document })));
      assert.equal(opened.kind, 'scene', `public CoreEngine must accept ${keys}-key fixture`);
      assert.equal(opened.document.parts.length, keys * 3);
      coreOpenReply = opened.kind;
    } finally {
      engine.free();
    }

    const projectJson = JSON.stringify(document);
    const packRequest = {
      kind: 'pack-project',
      projectJson,
      archiveJson: JSON.stringify({ embedUsedModels: true }),
      assets: [],
    };
    const [packReplyJson, packedBuffers] = core.archive_request(JSON.stringify(packRequest), []);
    const packReply = JSON.parse(packReplyJson);
    assert.equal(packReply.kind, 'packed', `public Rust archive API must pack ${keys}-key fixture`);
    assert.equal(packedBuffers.length, 1);
    const archiveBytes = new Uint8Array(packedBuffers[0]);
    const [unpackReplyJson, unpackedBuffers] = core.archive_request(JSON.stringify({ kind: 'unpack-project' }), [archiveBytes]);
    const unpackReply = JSON.parse(unpackReplyJson);
    assert.equal(unpackReply.kind, 'unpacked', `public Rust archive API must unpack ${keys}-key fixture`);
    assert.equal(JSON.stringify(JSON.parse(unpackReply.projectJson)), JSON.stringify(document));
    assert.equal(unpackedBuffers.length, 0);

    const stem = `pointer-${keys}-keys`;
    const archivePath = resolve(outputDir, `${stem}.boardstudio`);
    await writeFile(archivePath, archiveBytes);
    manifest.fixtures.push({
      keys,
      parts: document.parts.length,
      projectId: document.id,
      archive: `${stem}.boardstudio`,
      archiveBytes: archiveBytes.byteLength,
      archiveSha256: createHash('sha256').update(archiveBytes).digest('hex'),
      projectJsonSha256: createHash('sha256').update(projectJson).digest('hex'),
      coreOpenReply,
      archivePackReply: packReply.kind,
      archiveUnpackReply: unpackReply.kind,
    });
  }
} catch (error) {
  manifest.failure = { at: new Date().toISOString(), message: error instanceof Error ? error.stack ?? error.message : String(error) };
  throw error;
} finally {
  manifest.finishedAt = new Date().toISOString();
  manifest.exitStatus = manifest.failure ? 1 : 0;
  manifest.stdoutSummary = manifest.failure ? '' : `Generated and publicly validated ${manifest.fixtures.length} pointer archives in ${outputDir}`;
  manifest.stderrSummary = manifest.failure?.message ?? '';
  await writeFile(resolve(outputDir, 'manifest.json'), `${JSON.stringify(manifest, null, 2)}\n`);
}

console.log(`Generated and publicly validated ${manifest.fixtures.length} pointer archives in ${outputDir}`);

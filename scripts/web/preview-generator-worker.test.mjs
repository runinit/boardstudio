import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, readdir, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { test } from 'node:test';

const root = resolve(new URL('../..', import.meta.url).pathname);
const builder = join(root, 'scripts/web/build-preview-generator.mjs');

async function builtWorker(t) {
  const output = await mkdtemp(join(tmpdir(), 'boardstudio-preview-generator-'));
  t.after(() => rm(output, { recursive: true, force: true }));
  execFileSync(process.execPath, [builder, output], { cwd: root, stdio: 'pipe' });
  return join(output, 'preview-generator/worker.mjs');
}

function envelope(overrides = {}) {
  const owner = { scope: { documentId: 'doc-1', boardId: 'main', sessionEpoch: 4 }, token: 'snap-7', viewer_instance: 2, projection_generation: 3 };
  const batch = { accepted_revision: 7, batch_generation: 5 };
  const plan_key = { snapshot_token: 'plan-token', revision: 7, job_ids: ['main:sw1', 'main:sw2'] };
  const plan = {
    snapshotToken: 'plan-token', revision: 7, jobs: [
      { jobId: 'main:sw1', definition: {}, part: {} },
      { jobId: 'main:sw2', definition: {}, part: {} },
    ], reservedNets: [{ name: 'GND', index: 1 }], nextNetIndex: 2,
  };
  return {
    kind: 'generate-preview-jobs', worker_generation: 8, request_id: 11,
    owner, batch, plan_key, jobs: plan.jobs, reserved_nets: plan.reservedNets,
    next_net_index: plan.nextNetIndex, paths: [], ...overrides,
  };
}

test('packages a standalone relative-module worker without React or another Core WASM runtime', async (t) => {
  const workerPath = await builtWorker(t);
  const directory = workerPath.slice(0, workerPath.lastIndexOf('/'));
  const files = await readdir(directory);
  assert.deepEqual(files.sort(), ['catalogue.mjs', 'ergogen.mjs', 'kicad.mjs', 'provenance.json', 'worker.mjs']);
  const sources = await Promise.all(files.filter((name) => name.endsWith('.js') || name.endsWith('.mjs')).map((name) => readFile(join(directory, name), 'utf8')));
  const joined = sources.join('\n');
  const imports = [...joined.matchAll(/^\s*(?:import|export)\s+(?:[^;\n]*?\sfrom\s*)?(['"])([^'"]+)\1/gmu)].map((match) => match[2]);
  assert.ok(imports.length > 0, 'worker bundle must contain its local module graph');
  assert.ok(imports.every((specifier) => specifier.startsWith('./') || specifier.startsWith('../')), 'bundle imports must resolve within its static module graph');
  assert.doesNotMatch(joined, /core\/pkg|artifact_request/u);
  assert.match(await readFile(join(directory, 'worker.mjs'), 'utf8'), /from\s+['"]\.\/kicad\.mjs['"]/u);
  assert.match(await readFile(join(directory, 'kicad.mjs'), 'utf8'), /from\s+['"]\.\/ergogen\.mjs['"]/u);
  assert.match(await readFile(join(directory, 'ergogen.mjs'), 'utf8'), /from\s+['"]\.\/catalogue\.mjs['"]/u);
  const provenance = JSON.parse(await readFile(join(directory, 'provenance.json'), 'utf8'));
  assert.equal(provenance.purpose, 'Private Ergogen preview job worker');
  assert.equal(provenance.outputs['worker.mjs'], createHash('sha256').update(await readFile(join(directory, 'worker.mjs'))).digest('hex'));
});

test('generates ordered preview jobs with per-job net snapshots and matching envelope identity', async (t) => {
  const workerPath = await builtWorker(t);
  const worker = await import(pathToFileURL(workerPath));
  const { catalogue } = await import(pathToFileURL(join(workerPath.slice(0, workerPath.lastIndexOf('/')), 'ergogen.mjs')));
  const definition = structuredClone(catalogue().find((entry) => entry.generator?.source === 'ceoloide/switch_mx'));
  assert.ok(definition);
  const modelRoot = '${KIPRJMOD}/models/boardstudio/';
  const jobs = ['sw1', 'sw2'].map((id, index) => ({
    jobId: `main:${id}`,
    definition: structuredClone(definition),
    part: {
      id, definitionId: definition.id, reference: id.toUpperCase(),
      pose: { at: { x: index * 19, y: 0 }, rotation: 0 }, side: 'front',
      generatorParameters: {
        from: `ROW${index}`, to: `COL${index}`, side: 'F', hotswap: true,
        switch_3dmodel_filename: `${modelRoot}switch.step`,
        hotswap_3dmodel_filename: `${modelRoot}socket.step`,
        keycap_3dmodel_filename: `${modelRoot}keycap.step`,
      },
    },
  }));
  const request = envelope({
    jobs,
    plan_key: { snapshot_token: 'plan-token', revision: 7, job_ids: jobs.map((job) => job.jobId) },
    paths: [
      ['ergogen:model:switch.step', 'models/switch.step'],
      ['ergogen:model:socket.step', 'models/socket.step'],
      ['ergogen:model:keycap.step', 'models/keycap.step'],
    ],
  });
  const reply = worker.generatePreviewJobs(request);
  assert.equal(reply.kind, 'generated-preview-jobs');
  assert.equal(reply.worker_generation, request.worker_generation);
  assert.equal(reply.request_id, request.request_id);
  assert.deepEqual(reply.owner, request.owner);
  assert.deepEqual(reply.batch, request.batch);
  assert.deepEqual(reply.plan_key, request.plan_key);
  assert.deepEqual(reply.results.map((result) => result.jobId), ['main:sw1', 'main:sw2']);
  assert.equal(reply.results[0].snapshotToken, 'plan-token');
  assert.equal(reply.results[0].revision, 7);
  assert.deepEqual(reply.results[0].nets, [
    { name: 'GND', index: 1 }, { name: 'ROW0', index: 2 }, { name: 'COL0', index: 3 },
    { name: 'D1', index: 4 }, { name: 'D2', index: 5 },
  ]);
  assert.deepEqual(reply.results[1].nets, [
    { name: 'GND', index: 1 }, { name: 'ROW0', index: 2 }, { name: 'COL0', index: 3 },
    { name: 'D1', index: 4 }, { name: 'D2', index: 5 },
    { name: 'ROW1', index: 6 }, { name: 'COL1', index: 7 },
  ]);
  for (const result of reply.results) {
    assert.match(result.source, /\(footprint/u);
    assert.equal((result.source.match(/\(model "\$\{KIPRJMOD\}\/models\//gu) ?? []).length, 3);
  }
});

test('preserves repeated reserved names in plan order and allocates the first matching index', async (t) => {
  const workerPath = await builtWorker(t);
  const worker = await import(pathToFileURL(workerPath));
  const { catalogue, modelBindings } = await import(pathToFileURL(join(workerPath.slice(0, workerPath.lastIndexOf('/')), 'ergogen.mjs')));
  const definition = structuredClone(catalogue().find((entry) => entry.generator?.source === 'ceoloide/battery_connector_jst_ph_2'));
  assert.ok(definition);
  const job = {
    jobId: 'main:sw-repeat', definition,
    part: {
      id: 'sw-repeat', definitionId: definition.id, reference: 'SWR',
      pose: { at: { x: 0, y: 0 }, rotation: 0 }, side: 'front',
      generatorParameters: { BAT_P: 'SHARED', BAT_N: 'SHARED' },
    },
  };
  const request = envelope({
    jobs: [job],
    plan_key: { snapshot_token: 'plan-token', revision: 7, job_ids: [job.jobId] },
    reserved_nets: [{ name: 'SHARED', index: 1 }, { name: 'SHARED', index: 2 }],
    next_net_index: 3,
    paths: modelBindings(definition, job.part).map((model, index) => [model.assetId, `models/generated-${index}.step`]),
  });
  const reply = worker.generatePreviewJobs(request);
  assert.equal(reply.kind, 'generated-preview-jobs');
  assert.deepEqual(reply.results[0].nets.slice(0, 2), [
    { name: 'SHARED', index: 1 }, { name: 'SHARED', index: 2 },
  ]);
  assert.match(reply.results[0].source, /\(net 1 "SHARED"\)/u);
});

test('preserves the established safe unresolved-model path mapping', async (t) => {
  const workerPath = await builtWorker(t);
  const worker = await import(pathToFileURL(workerPath));
  const { catalogue } = await import(pathToFileURL(join(workerPath.slice(0, workerPath.lastIndexOf('/')), 'ergogen.mjs')));
  const definition = structuredClone(catalogue().find((entry) => entry.generator?.source === 'ceoloide/switch_mx'));
  assert.ok(definition);
  const job = {
    jobId: 'main:sw-unresolved', definition,
    part: {
      id: 'sw-unresolved', definitionId: definition.id, reference: 'SWX',
      pose: { at: { x: 0, y: 0 }, rotation: 0 }, side: 'front',
      generatorParameters: {
        from: 'ROW', to: 'COL', side: 'F', hotswap: false,
        switch_3dmodel_filename: 'custom://boardstudio-special.step',
      },
    },
  };
  const request = envelope({
    jobs: [job],
    plan_key: { snapshot_token: 'plan-token', revision: 7, job_ids: [job.jobId] },
  });
  const reply = worker.generatePreviewJobs(request);
  assert.equal(reply.kind, 'generated-preview-jobs');
  assert.match(reply.results[0].source, /models\/unresolved\/custom%3A%2F%2Fboardstudio-special\.step/u);
});

test('rejects unsafe revision and net-index numbers before generation', async (t) => {
  const workerPath = await builtWorker(t);
  const worker = await import(pathToFileURL(workerPath));
  const unsafeRevision = envelope({ batch: { accepted_revision: Number.MAX_SAFE_INTEGER + 1, batch_generation: 5 } });
  assert.throws(() => worker.generatePreviewJobs(unsafeRevision), /safe integer/u);
  const unsafeIndex = envelope({ next_net_index: 0x1_0000_0000 });
  assert.throws(() => worker.generatePreviewJobs(unsafeIndex), /safe integer in range/u);
  const unsafeReserved = envelope({ reserved_nets: [{ name: 'GND', index: Number.MAX_SAFE_INTEGER + 1 }] });
  assert.throws(() => worker.generatePreviewJobs(unsafeReserved), /safe integer in range/u);
});

test('returns correlated error replies and rejects mismatched plan identity', async (t) => {
  const workerPath = await builtWorker(t);
  const worker = await import(pathToFileURL(workerPath));
  const badPlan = envelope({ plan_key: { snapshot_token: 'plan-token', revision: 7, job_ids: ['main:sw2', 'main:sw1'] } });
  const reply = worker.handlePreviewGeneratorMessage(badPlan);
  assert.equal(reply.kind, 'preview-generator-error');
  assert.equal(reply.worker_generation, badPlan.worker_generation);
  assert.equal(reply.request_id, badPlan.request_id);
  assert.deepEqual(reply.owner, badPlan.owner);
  assert.match(reply.message, /job order/u);
  const wrongKind = worker.handlePreviewGeneratorMessage(envelope({ kind: 'other' }));
  assert.equal(wrongKind.kind, 'preview-generator-error');
  assert.match(wrongKind.message, /request kind/u);
  const oversizedError = envelope({
    jobs: [{ jobId: 'main:sw1', definition: { generator: { source: 'x'.repeat(3000) } }, part: {} }, { jobId: 'main:sw2', definition: {}, part: {} }],
  });
  assert.equal(worker.handlePreviewGeneratorMessage(oversizedError).message.length, 2048);
});

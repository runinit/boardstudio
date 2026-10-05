// TEMPORARY golden-baseline recorder for the Rust footprint generator port
// (docs/investigations/footprint-generators-rust.md, step 1). It drives the
// production JavaScript provider (ergogen/src, kicad/src/ergogen.ts and the
// preview worker) and writes the committed fixtures beside this directory.
// It is deleted with the JavaScript in step 6, after which the fixtures can no
// longer be regenerated.
//
// Run from the repository root with Node 24+ (type stripping):
//   cargo build --manifest-path core/Cargo.toml --example core_request
//   node --no-warnings --import ./footprints/tests/golden/harness/register.mjs \
//        footprints/tests/golden/harness/record.mjs [path/to/core_request]
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { buildDemoDocuments } from './demos.mjs';

const root = fileURLToPath(new URL('../../../../', import.meta.url));
const out = `${root}footprints/tests/golden/`;
const driver = process.argv[2] ?? `${root}core/target/debug/examples/core_request`;

const provider = await import(`${root}ergogen/src/index.ts`);
const { exportErgogenForms } = await import(`${root}kicad/src/ergogen.ts`);
const worker = await import(`${root}scripts/web/preview-generator-worker.ts`);
const { catalogue, parameters, render, geometry, normalizeDefinition, modelAssetIds, modelBindings,
  modelAssetIdsForPaths, parseForms, serialize, isErgogen } = provider;

const hash = (path) => createHash('sha256').update(readFileSync(root + path)).digest('hex');
const capture = (fn) => {
  try {
    return { ok: fn() };
  } catch (error) {
    return { error: error instanceof Error ? error.message : String(error) };
  }
};

// Same lookup semantics as the preview worker's allocator.
function allocator(reserved, next) {
  const nets = reserved.map((net) => ({ ...net }));
  let nextIndex = next;
  return {
    lookup(name) {
      const known = nets.find((net) => net.name === name);
      if (known) return known.index;
      const net = { name, index: nextIndex++ };
      nets.push(net);
      return net.index;
    },
    snapshot: () => nets.map((net) => ({ ...net })),
  };
}

const DEFAULT_NETS = { reserved: [{ name: 'GND', index: 1 }], next: 2 };

/** Model paths a real export would supply: bundled assets map to files, unresolved paths follow the worker rule. */
function exportPaths(definition, part, emptyPaths) {
  const paths = new Map();
  if (emptyPaths) return paths;
  for (const model of modelBindings(definition, part)) {
    if (model.assetId.startsWith('unresolved-model:')) {
      const original = decodeURIComponent(model.assetId.slice('unresolved-model:'.length));
      paths.set(original, `models/unresolved/${encodeURIComponent(original)}`);
    } else {
      paths.set(model.assetId, `models/${model.assetId.replace(/[:/]/gu, '_')}`);
    }
  }
  return paths;
}

/**
 * Record every output the plan lists for one input. `part` is absent for a
 * standalone render (no allocator, no placement); `nets` is absent likewise.
 */
function evaluate(definition, part, nets, { normalized = false, emptyPaths = false } = {}) {
  const fresh = () => (nets ? allocator(nets.reserved, nets.next) : null);
  const output = {};
  const renderOptions = (a) => ({ part, netIndex: a?.lookup });
  output.forms = capture(() => {
    const a = fresh();
    const forms = render(definition, renderOptions(a)).map(serialize);
    return { forms, nets: a?.snapshot() };
  });
  output.geometry = capture(() => geometry(render(definition, renderOptions(fresh()))));
  output.modelAssetIds = capture(() => modelAssetIds(definition, part));
  output.modelBindings = capture(() => modelBindings(definition, part));
  output.export = capture(() => {
    const a = fresh();
    const paths = exportPaths(definition, part, emptyPaths);
    const result = exportErgogenForms(definition, part, paths, a?.lookup);
    return { ...result, modelPaths: Object.fromEntries(paths), nets: a?.snapshot() };
  });
  if (normalized) output.normalized = capture(() => normalizeDefinition(definition));
  return output;
}

const definitions = catalogue();
const sources = definitions.map((definition) => definition.generator.source);
const byId = new Map(definitions.map((definition) => [definition.generator.source, definition]));

const withParameters = (definition, overrides, { savedPads = false } = {}) => ({
  ...structuredClone(definition),
  ...(savedPads ? { pads: definition.pads.map((pad, index) => ({ ...pad, id: `saved-${index}`, netId: `net-${index}` })) } : {}),
  generator: { ...definition.generator, parameters: { ...definition.generator.parameters, ...overrides } },
});
const placement = (definition, { x = 0, y = 0, rotation = 0, side = 'front', params } = {}) => ({
  id: 'part-1', definitionId: definition.id, reference: 'U1',
  pose: { at: { x, y }, rotation }, side,
  ...(params ? { generatorParameters: params } : {}),
});

function checkNames(source, values, label) {
  const schema = parameters(source);
  for (const name of Object.keys(values)) {
    // `side` is read from the render inputs whether or not a generator declares it.
    if (name !== 'side' && !schema[name]) throw new Error(`${label}: ${source} has no parameter ${name}`);
  }
}

// Inputs the generic matrix cannot reach: utility routes and zones, validation
// errors, model-path forms. [id, parameters, { definition-level | emptyPaths }]
const extras = {
  'ceoloide/utility_router': [
    ['route-basic', { route: 'f(0,0)(10,0)v(10,5)b(10,5)(0,5)', net: 'ROW1' }],
    ['routes-multi', { routes: ['f(0,0)(10,0)', 'b(0,2) (10,2)'], net: 'COL1', locked: true, width: 0.3 }],
    ['error-no-layer', { route: '(0,0)(1,1)' }],
    ['error-character', { route: 'f(0,0)z' }],
    ['error-global-net', { route: 'f(0,0)<GND>(1,1)' }],
    ['error-via-position', { route: 'fv' }],
    ['error-bad-position', { route: 'f(a,b)(1,2)' }],
  ],
  'ceoloide/utility_keepout_zone': [
    ['hatch-error', { hatch_pitch: 3 }],
    ['hatch-valid', { hatch_pitch: 1.5, name: 'KO', locked: true }],
    ['custom-points', { points: [[0, 0], [10, 0], [10, 10]] }],
  ],
  'ceoloide/utility_filled_zone': [
    ['hatch-fill', { fill_type: 'hatch', name: 'Z', net: 'GND', priority: 2, points: [[0, 0], [10, 0], [10, 10], [0, 10]] }],
    ['net-assigned', { net: 'VCC' }],
  ],
  'ceoloide/utility_text': [
    ['text', { text: 'Hello', height: 1.5, width: 1.2 }],
    ['reversible-styled', { text: 'Hi', reversible: true, mirrored: true, knockout: true, bold: true, italic: true }],
  ],
  'infused-kim/text': [['text', { text: 'Hello' }], ['reverse', { text: 'Hi', reverse: true, side: 'B' }]],
  'ceoloide/utility_ergogen_logo': [['scaled', { scale: 2 }]],
  'infused-kim/pads': [['six-pads', { pads: 6, label_1: 'A', label_6: 'F', label_at_bottom: true }]],
  'infused-kim/icon_bat': [['spaced', { spacing: 2 }]],
  'infused-kim/trackpoint_mount': [['drill-too-small', { drill: 3 }], ['drill-five', { drill: 5 }]],
  'ceoloide/switch_choc_v1_v2': [
    ['v2-only', { choc_v1_support: false }],
    ['v2-oval-error', { choc_v1_support: false, oval_stabilizer_pad: true }],
    ['v2-no-stabilizer-error', { choc_v1_support: false, include_stabilizer_pad: false }],
    ['v2-small-center-hole-error', { choc_v1_support: false, center_hole_diameter: 3 }],
    ['v2-custom-model', { choc_v1_support: false, switch_3dmodel_filename: '${KIPRJMOD}/models/boardstudio/custom.step' }],
  ],
  'ceoloide/rotary_encoder_ec11_ec12': [
    ['signal-hole-zero-error', { signal_hole_width: 0 }],
    ['signal-hole-round', { signal_hole_width: 1, signal_hole_height: 1 }],
  ],
  'ceoloide/mounting_hole_npth': [
    ['numeric-values', { hole_size: 3, hole_drill: 3 }],
    ['string-values', { hole_size: '3.5', hole_drill: '3.5' }],
  ],
  'ceoloide/diode_tht_sod123': [['smd-pads-reversible', { include_thru_hole_smd_pads: true, reversible: true }]],
  'ceoloide/switch_gateron_ks27_ks33': [['solder-only', { hotswap: false, solder: true }]],
  'ceoloide/switch_mx': [
    ['model-unresolved', { switch_3dmodel_filename: 'custom://boardstudio-special.step' }],
    ['model-attached', { switch_3dmodel_filename: 'boardstudio-asset:abc_123' }],
    ['model-infused', { switch_3dmodel_filename: '${EG_INFUSED_KIM_3D_MODELS}/example.step' }],
    ['model-attached-no-export-path', { switch_3dmodel_filename: 'boardstudio-asset:abc_123' }, { emptyPaths: true }],
  ],
};
// Definition-level inputs reached through normalizeDefinition.
const definitionExtras = {
  'ceoloide/switch_mx': [['keycap-zero-error', { keycap_width: 0 }], ['keycap-custom', { keycap_width: 23, keycap_height: 17 }]],
  'ceoloide/switch_choc_v1_v2': [['keycap-zero-error', { keycap_height: 0 }], ['keycap-custom', { keycap_width: 18.5, keycap_height: 17.5 }]],
  'ceoloide/switch_gateron_ks27_ks33': [['keycap-zero-error', { keycap_width: -1 }], ['keycap-custom', { keycap_width: 23, keycap_height: 17 }]],
  'ceoloide/diode_tht_sod123': [['hotswap-conflict-error', { include_thru_hole_smd_pads: true }]],
};

const rotations = [0, 90, 37];
const OFFSET = { x: 12.5, y: -7.25 };

function generatorCases(source) {
  const definition = byId.get(source);
  const schema = parameters(source);
  const booleans = Object.entries(schema).filter(([, parameter]) => parameter.type === 'boolean').map(([name]) => name);
  const numbers = Object.entries(schema).filter(([, parameter]) => parameter.type === 'number').map(([name]) => name);
  const nets = Object.entries(schema).filter(([, parameter]) => parameter.type === 'net').map(([name]) => name);
  const cases = [];
  const add = (id, family, input, output) => cases.push({ id, family, input, output });
  const definitionCase = (id, family, overrides, options = {}) => {
    checkNames(source, overrides, id);
    const edited = withParameters(definition, overrides, { savedPads: options.savedPads });
    add(id, family, { definitionParameters: overrides, savedPads: Boolean(options.savedPads), standalone: true },
      evaluate(edited, undefined, null, { normalized: true }));
  };
  const placedCase = (id, family, spec, { definitionParameters = {}, ...options } = {}) => {
    checkNames(source, spec.params ?? {}, id);
    checkNames(source, definitionParameters, id);
    const edited = withParameters(definition, definitionParameters);
    const part = placement(edited, spec);
    add(id, family, { definitionParameters, part, reservedNets: DEFAULT_NETS.reserved, nextNetIndex: DEFAULT_NETS.next },
      evaluate(edited, part, DEFAULT_NETS, options));
  };

  definitionCase('default', 'default', {});
  placedCase('placed-default', 'default', {});
  for (const name of booleans) {
    const flipped = !schema[name].value;
    definitionCase(`flip:${name}`, 'boolean-flip', { [name]: flipped }, { savedPads: true });
  }
  for (const rotation of rotations) {
    placedCase(`pose:front-F-${rotation}`, 'side-rotation', { ...OFFSET, rotation, side: 'front', params: { side: 'F' } });
    placedCase(`pose:back-B-${rotation}`, 'side-rotation', { ...OFFSET, rotation, side: 'back', params: { side: 'B' } });
  }
  placedCase('pose:front-B-0', 'side-rotation', { ...OFFSET, side: 'front', params: { side: 'B' } });
  placedCase('pose:back-F-0', 'side-rotation', { ...OFFSET, side: 'back', params: { side: 'F' } });
  placedCase('pose:front-unset-37', 'side-rotation', { ...OFFSET, rotation: 37, side: 'front' });
  placedCase('pose:back-unset-37', 'side-rotation', { ...OFFSET, rotation: 37, side: 'back' });
  if (nets.length) {
    placedCase('nets:assigned', 'marker-nets', { params: Object.fromEntries(nets.map((name) => [name, `NET_${name}`])) });
    placedCase('nets:reserved', 'marker-nets', { params: Object.fromEntries(nets.map((name) => [name, 'GND'])) });
    placedCase('nets:empty', 'marker-nets', { params: Object.fromEntries(nets.map((name) => [name, ''])) });
  }
  if (booleans.length >= 2) {
    const [first, second] = booleans;
    placedCase('merge:part-overrides-definition', 'parameter-merge',
      { params: { [first]: Boolean(schema[first].value), [second]: !schema[second].value } },
      { definitionParameters: { [first]: !schema[first].value } });
  }
  if (numbers.length) {
    placedCase('coerce:numeric-strings', 'coercion',
      { params: Object.fromEntries(numbers.map((name) => [name, String(schema[name].value)])) });
  }
  for (const [id, overrides, options] of extras[source] ?? []) placedCase(`extra:${id}`, 'extra', { params: overrides }, options);
  for (const [id, overrides] of definitionExtras[source] ?? []) definitionCase(`extra:${id}`, 'extra', overrides);
  return cases;
}

// ----- Demo project configurations ---------------------------------------

function demoCases(documents) {
  const perGenerator = new Map(sources.map((source) => [source, []]));
  const seenDefinitions = new Set();
  const seenParts = new Set();
  const summary = [];
  for (const { project, document } of documents) {
    const defs = new Map(document.definitions.map((definition) => [definition.id, definition]));
    const stats = { project, definitions: 0, parts: 0, uniqueDefinitions: 0, uniqueParts: 0 };
    for (const definition of document.definitions) {
      const source = definition.generator?.source;
      if (!source || !isErgogen(source)) continue;
      stats.definitions += 1;
      const key = JSON.stringify([source, definition.generator.version, definition.generator.parameters]);
      if (seenDefinitions.has(key)) continue;
      seenDefinitions.add(key);
      stats.uniqueDefinitions += 1;
      perGenerator.get(source).push({
        id: `demo-definition:${project}:${definition.id}`, family: 'demo-definition',
        input: { project, definition: { id: definition.id, generator: definition.generator, savedPadCount: definition.pads.length }, standalone: true },
        output: evaluate(definition, undefined, null, { normalized: true }),
      });
    }
    const rotated = new Set();
    for (const part of document.parts) {
      const definition = defs.get(part.definitionId);
      const source = definition?.generator?.source;
      if (!source || !isErgogen(source)) continue;
      stats.parts += 1;
      const schema = parameters(source);
      const shape = Object.fromEntries(Object.entries(part.generatorParameters ?? {}).sort(([a], [b]) => a.localeCompare(b))
        .map(([name, value]) => [name, schema[name]?.type === 'net' ? '<net>' : value]));
      const key = JSON.stringify([source, definition.generator.parameters, part.side, shape]);
      const rotation = part.pose.rotation % 360 !== 0;
      const first = !seenParts.has(key);
      const firstRotated = rotation && !rotated.has(key);
      if (!first && !firstRotated) continue;
      seenParts.add(key);
      if (rotation) rotated.add(key);
      stats.uniqueParts += 1;
      perGenerator.get(source).push({
        id: `demo-part:${project}:${part.id}`, family: 'demo-part',
        input: { project, definitionId: definition.id, definitionParameters: definition.generator.parameters, part,
          reservedNets: DEFAULT_NETS.reserved, nextNetIndex: DEFAULT_NETS.next },
        output: evaluate(definition, part, DEFAULT_NETS),
      });
    }
    summary.push(stats);
  }
  return { perGenerator, summary };
}

// ----- Provider and worker scenarios ---------------------------------------

function providerScenarios() {
  const mx = byId.get('ceoloide/switch_mx');
  const unknown = { ...structuredClone(mx), generator: { ...mx.generator, source: 'ceoloide/missing' } };
  const future = { ...structuredClone(mx), generator: { ...mx.generator, version: 'future' } };
  const parseInputs = [
    '(footprint', 'footprint', '', '   ', ')', '(a (b "c d" e) "f\\"g")', '(a "")', '(a b)(c d)', '(module "x" (at 1 2.50 -0))',
    '(a "line\\nbreak")', '(a "é ☃")', '(a #x)', '"standalone"',
  ].map((source) => ({ source, result: capture(() => parseForms(source).map(serialize)), tree: capture(() => parseForms(source)) }));
  const paths = ['boardstudio-asset:abc_123', 'boardstudio-asset:bad id', '${KIPRJMOD}/models/boardstudio/kicad/a.step',
    '${EG_INFUSED_KIM_3D_MODELS}/b.wrl', '${KIPRJMOD}/other/c.step', 'custom://x.step', ''];
  return {
    isErgogen: Object.fromEntries(['ceoloide/switch_mx', 'infused-kim/text', 'infused-kim/choc', 'ceoloide/missing', 'toString', ''].map((source) => [source, isErgogen(source)])),
    isErgogenUndefined: isErgogen(undefined),
    unknownGenerator: { parameters: capture(() => parameters('ceoloide/missing')), render: capture(() => render(unknown)),
      normalize: capture(() => normalizeDefinition(unknown)) },
    unsupportedVersion: capture(() => render(future)),
    noGenerator: capture(() => render({ id: 'x' })),
    parseForms: parseInputs,
    modelAssetIdsForPaths: { paths, ids: modelAssetIdsForPaths(paths) },
  };
}

function workerScenarios() {
  const envelope = (overrides = {}) => {
    const owner = { scope: { documentId: 'doc-1', boardId: 'main', sessionEpoch: 4 }, token: 'snap-7', viewer_instance: 2, projection_generation: 3 };
    const jobs = [{ jobId: 'main:sw1', definition: {}, part: {} }, { jobId: 'main:sw2', definition: {}, part: {} }];
    return {
      kind: 'generate-preview-jobs', worker_generation: 8, request_id: 11, owner,
      batch: { accepted_revision: 7, batch_generation: 5 },
      plan_key: { snapshot_token: 'plan-token', revision: 7, job_ids: jobs.map((job) => job.jobId) },
      jobs, reserved_nets: [{ name: 'GND', index: 1 }], next_net_index: 2, paths: [], ...overrides,
    };
  };
  const jobsFor = (source, specs) => specs.map(([id, spec]) => {
    const definition = structuredClone(byId.get(source));
    return { jobId: `main:${id}`, definition, part: { id, definitionId: definition.id, reference: id.toUpperCase(), side: 'front', pose: { at: { x: spec.x ?? 0, y: 0 }, rotation: 0 }, generatorParameters: spec.params } };
  });
  const run = (request) => {
    const reply = worker.handlePreviewGeneratorMessage(request);
    return { request, reply };
  };
  // Export paths for each resolved bundled model, as the page supplies them.
  const modelPaths = (jobs) => [...new Set(jobs.flatMap((job) => capture(() => modelBindings(job.definition, job.part)).ok?.map((model) => model.assetId) ?? []))]
    .filter((id) => !id.startsWith('unresolved-model:')).map((id, index) => [id, `models/generated-${index}.step`]);
  const planned = (jobs, overrides = {}) => envelope({ jobs, paths: modelPaths(jobs), plan_key: { snapshot_token: 'plan-token', revision: 7, job_ids: jobs.map((job) => job.jobId) }, ...overrides });
  const model = '${KIPRJMOD}/models/boardstudio/';
  const mxJobs = jobsFor('ceoloide/switch_mx', [0, 1].map((index) => [`sw${index + 1}`, { x: index * 19, params: {
    from: `ROW${index}`, to: `COL${index}`, side: 'F', hotswap: true,
    switch_3dmodel_filename: `${model}switch.step`, hotswap_3dmodel_filename: `${model}socket.step`, keycap_3dmodel_filename: `${model}keycap.step` } }]));
  const batteryJobs = jobsFor('ceoloide/battery_connector_jst_ph_2', [['sw-repeat', { params: { BAT_P: 'SHARED', BAT_N: 'SHARED' } }]]);
  const unresolvedJobs = jobsFor('ceoloide/switch_mx', [['sw-unresolved', { params: { from: 'ROW', to: 'COL', side: 'F', hotswap: false, switch_3dmodel_filename: 'custom://boardstudio-special.step' } }]]);
  const scenarios = {
    'two-switches-net-snapshots': planned(mxJobs, { paths: [['ergogen:model:switch.step', 'models/switch.step'], ['ergogen:model:socket.step', 'models/socket.step'], ['ergogen:model:keycap.step', 'models/keycap.step']] }),
    'repeated-reserved-names': planned(batteryJobs, { reserved_nets: [{ name: 'SHARED', index: 1 }, { name: 'SHARED', index: 2 }], next_net_index: 3 }),
    'unresolved-model-path': planned(unresolvedJobs),
    'job-failure-propagates': planned(jobsFor('ceoloide/utility_router', [['r1', { params: { route: '(0,0)(1,1)' } }]])),
    'net-index-exhausted': planned(batteryJobs, { reserved_nets: [], next_net_index: 0xffff_ffff }),
    'net-index-last-valid': planned(jobsFor('ceoloide/diode_tht_sod123', [['d1', { params: { from: 'ONE' } }]]), { reserved_nets: [], next_net_index: 0xffff_ffff }),
    'invalid-kind': envelope({ kind: 'other' }),
    'invalid-worker-generation': envelope({ worker_generation: 0 }),
    'invalid-request-id': envelope({ request_id: 1.5 }),
    'invalid-owner': envelope({ owner: { scope: {}, token: '', viewer_instance: 0, projection_generation: 0 } }),
    'invalid-viewer-instance': envelope({ owner: { scope: {}, token: 't', viewer_instance: -1, projection_generation: 0 } }),
    'invalid-batch': envelope({ batch: 'x' }),
    'unsafe-revision': envelope({ batch: { accepted_revision: Number.MAX_SAFE_INTEGER + 1, batch_generation: 5 } }),
    'invalid-plan-key': envelope({ plan_key: { snapshot_token: '', revision: 7, job_ids: [] } }),
    'revision-mismatch': envelope({ plan_key: { snapshot_token: 'plan-token', revision: 6, job_ids: ['main:sw1', 'main:sw2'] } }),
    'job-id-types': envelope({ plan_key: { snapshot_token: 'plan-token', revision: 7, job_ids: ['main:sw1', ''] } }),
    'inputs-not-arrays': envelope({ jobs: null }),
    'job-order-mismatch': envelope({ plan_key: { snapshot_token: 'plan-token', revision: 7, job_ids: ['main:sw2', 'main:sw1'] } }),
    'duplicate-job-ids': envelope({ jobs: [{ jobId: 'a' }, { jobId: 'a' }], plan_key: { snapshot_token: 'plan-token', revision: 7, job_ids: ['a', 'a'] } }),
    'reserved-name-invalid': envelope({ reserved_nets: [{ name: 1, index: 1 }] }),
    'reserved-index-zero': envelope({ reserved_nets: [{ name: 'GND', index: 0 }] }),
    'reserved-unsafe-index': envelope({ reserved_nets: [{ name: 'GND', index: Number.MAX_SAFE_INTEGER + 1 }] }),
    'reserved-duplicate-index': envelope({ reserved_nets: [{ name: 'A', index: 1 }, { name: 'B', index: 1 }] }),
    'next-index-overlap': envelope({ reserved_nets: [{ name: 'A', index: 5 }], next_net_index: 5 }),
    'next-index-out-of-range': envelope({ next_net_index: 0x1_0000_0000 }),
    'path-entry-invalid': envelope({ paths: [['a']] }),
    'path-duplicate': envelope({ paths: [['a', 'b'], ['a', 'c']] }),
    'error-message-truncated': envelope({ jobs: [{ jobId: 'main:sw1', definition: { generator: { source: 'x'.repeat(3000) } }, part: {} }, { jobId: 'main:sw2', definition: {}, part: {} }] }),
  };
  return Object.entries(scenarios).map(([id, request]) => {
    const { reply } = run(request);
    const long = reply.kind === 'preview-generator-error' && reply.message.length >= 2048;
    return { id, kind: reply.kind, request, reply: long ? { ...reply, message: reply.message.slice(0, 80), messageLength: reply.message.length } : reply };
  });
}

// ----- Write -----------------------------------------------------------------

function writeCases(path, header, cases) {
  const body = cases.map((entry) => `  ${JSON.stringify(entry)}`).join(',\n');
  const head = JSON.stringify(header).slice(0, -1);
  writeFileSync(path, `${head},"cases":[\n${body}\n]}\n`);
}

const documents = await buildDemoDocuments(root, driver);
const { perGenerator, summary } = demoCases(documents);

rmSync(`${out}generators`, { recursive: true, force: true });
mkdirSync(`${out}generators`, { recursive: true });
const manifest = { generators: {}, totals: { generators: sources.length, cases: 0, byFamily: {}, errors: 0 } };
for (const source of sources) {
  const cases = [...generatorCases(source), ...perGenerator.get(source)];
  const file = `generators/${source.replace('/', '__')}.json`;
  writeCases(`${out}${file}`, {
    source, parameters: parameters(source), catalogue: byId.get(source),
  }, cases);
  const byFamily = {};
  let errors = 0;
  for (const entry of cases) {
    byFamily[entry.family] = (byFamily[entry.family] ?? 0) + 1;
    errors += Object.values(entry.output).filter((value) => value && 'error' in value).length;
    manifest.totals.byFamily[entry.family] = (manifest.totals.byFamily[entry.family] ?? 0) + 1;
  }
  manifest.generators[source] = { file, cases: cases.length, byFamily, outputErrors: errors, parameterCount: Object.keys(parameters(source)).length };
  manifest.totals.cases += cases.length;
  manifest.totals.errors += errors;
}
writeFileSync(`${out}provider.json`, `${JSON.stringify(providerScenarios(), null, 1)}\n`);
const workerReplies = workerScenarios();
writeCases(`${out}worker.json`, { description: 'Preview worker replies; replies for definitions inline their full source text.' }, workerReplies);
manifest.totals.workerScenarios = workerReplies.length;
manifest.demoProjects = summary;
manifest.provenance = {
  node: process.version,
  revision: 'recorded from the working tree at commit 851875a0e (provider unchanged since 3cdeb2ac2)',
  sourceHashes: Object.fromEntries(['ergogen/src/index.ts', 'ergogen/generated/catalogue.mjs', 'kicad/src/ergogen.ts',
    'scripts/web/preview-generator-worker.ts', 'ergogen/library/manifest/default-models.json']
    .map((path) => [path, hash(path)])),
  fixtureProjects: ['content/archives/reviung41-original.boardstudio', 'tooling/demo-projects/src/demos/*', 'tooling/demo-projects/src/demo.ts'],
};
writeFileSync(`${out}manifest.json`, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(JSON.stringify(manifest.totals), readdirSync(`${out}generators`).length, 'generator files');

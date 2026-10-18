// Real Host worker + real web-target CAD provider. Expected fixture reports never mint evidence.
export async function checkWorkerQualification(check, contract) {
  const a = contract.qualificationV2.activation;
  const clone = value => structuredClone(value);
  const equal = (x, y) => JSON.stringify(x) === JSON.stringify(y);
  const require = (name, condition, detail = '') => {
    check(name, condition, detail);
    if (!condition) throw new Error(`${name}: ${detail}`);
  };
  const moduleUrl = new URL('../../../web/target/providers/cad-worker/m1_cad_worker.js', location.href).href;
  const providerUrl = new URL('../../wasm/pkg/boardstudio_cad_wasm.js', location.href).href;
  const entry = URL.createObjectURL(new Blob([
    `import init, {start_cad_worker} from ${JSON.stringify(moduleUrl)};\nawait init();\nstart_cad_worker(${JSON.stringify(providerUrl)});`,
  ], { type: 'text/javascript' }));
  let worker, sequence = 0;
  const waiting = new Map();
  const identity = { ...a.snapshot, token: Number(a.snapshot.token), sessionEpoch: Number(a.snapshot.sessionEpoch) };
  async function start() {
    worker = new Worker(entry, { type: 'module' });
    await new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error('actual CAD worker ready timeout')), 60000);
      worker.onerror = event => { clearTimeout(timer); reject(new Error(event.message)); };
      worker.onmessage = event => {
        const message = event.data;
        if (message.kind === 'ready') { clearTimeout(timer); resolve(); }
        if (message.kind === 'reply') {
          const frame = JSON.parse(message.frame);
          const pending = waiting.get(frame.requestId);
          if (pending) { waiting.delete(frame.requestId); clearTimeout(pending.timer); pending.resolve({ frame, result: message.result }); }
        }
      };
    });
  }
  function request(operation, fields = {}, payload = {}, cancel = false) {
    const requestId = `actual-v2-${++sequence}`;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => { waiting.delete(requestId); reject(new Error(`${operation} timeout`)); }, 60000);
      waiting.set(requestId, { resolve, timer });
      const frame = { requestId, jobId: requestId, identity, operation, ...fields };
      const transfers = Object.values(payload).filter(value => value instanceof ArrayBuffer);
      worker.postMessage({ kind: 'request', frame: JSON.stringify(frame), ...payload }, transfers);
      if (cancel) worker.postMessage({ kind: 'cancel', jobId: requestId });
    });
  }
  async function completed(operation, fields, payload) {
    const reply = await request(operation, fields, payload);
    require(`actual worker ${operation} completes`, reply.frame.outcome === 'completed', reply.frame.error);
    require('actual worker preserves captured identity', equal(reply.frame.identity, identity));
    return reply.result;
  }
  async function generated(body = contract.preparedBody, target = null) {
    // Same immutable prepared-body identity algorithm as the active producer/consumer contract.
    // The canonical fixture identity is checked by the worker, never accepted as a certificate.
    const geometryIdentity = body === contract.preparedBody ? a.workerReferences.plane.source.geometryIdentity
      : a.workerReferences.cylinderDefault.source.geometryIdentity;
    const result = await completed('analyze-generated-body', {
      prepared: { revision: body.revision, bodies: [body] },
      generatedAnalysisRequest: { bodyId: body.body.id, geometryIdentity, targetDeflectionMm: target },
    });
    const asset = result.generatedAnalysis;
    require('worker transfers copies without exposing the opaque handle', !('analysisHandle' in asset)
      && asset.surfacePositionsMmF64 instanceof Float64Array && asset.triangleFaceOccurrenceIndices instanceof Uint32Array
      && asset.analysisRef.meshIdentity === asset.analysisMesh.meshIdentity);
    return asset;
  }
  async function vendor(path, options = {}) {
    const response = await fetch(new URL(`../../../${path}`, location.href));
    if (!response.ok) throw new Error(`original STEP unavailable: ${path}`);
    const bytes = new Uint8Array(await response.arrayBuffer());
    const sha = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)), byte => byte.toString(16).padStart(2, '0')).join('');
    const result = await completed('read-step', { sourceSha256: sha, stepImportOptions: options }, { inputBytes: bytes.buffer });
    require('worker authenticates original STEP bytes', result.vendorStep.sourceSha256 === sha);
    return result.vendorStep;
  }
  function metadata(example, left, right, accuracy = null) {
    const b = example.binding;
    const operand = (asset, side) => {
      const source = asset.analysisRef.source;
      return { modelId: b[`${side}ModelId`],
        ...(source.kind === 'vendor-step' ? { sourceSha256: source.sourceSha256 } : { generatedGeometryIdentity: source.geometryIdentity }),
        importOptions: source.kind === 'vendor-step' ? source.importOptions : {},
        rowTransformF64: clone(b[`${side}RowTransformF64`]),
        analysisRef: clone(asset.analysisRef), analysisMesh: clone(asset.analysisMesh) };
    };
    return { left: operand(left, 'left'), right: operand(right, 'right'), clearanceThresholdMm: b.clearanceThresholdMm,
      requestedAccuracyMm: accuracy, qualificationLimits: clone(example.limits) };
  }
  function buffers(left, right) {
    return { leftPositions: left.surfacePositionsMmF64.slice().buffer, leftTags: left.triangleFaceOccurrenceIndices.slice().buffer,
      rightPositions: right.surfacePositionsMmF64.slice().buffer, rightTags: right.triangleFaceOccurrenceIndices.slice().buffer };
  }
  async function query(left, right, q) {
    const result = await completed('collision-query', { collisionQuery: q }, buffers(left, right));
    const report = result.queryQualification;
    require('qualified worker reply binds actual owned references, rows and snapshot',
      equal(report.binding.leftAnalysis, q.left.analysisRef) && equal(report.binding.rightAnalysis, q.right.analysisRef)
      && equal(report.binding.leftRowTransformF64, q.left.rowTransformF64) && equal(report.binding.rightRowTransformF64, q.right.rowTransformF64)
      && equal(report.binding.snapshot, a.snapshot) && equal(report.limits, q.qualificationLimits));
    return result;
  }
  function bounded(name, result, expected) {
    const q = result.queryQualification;
    const inflation = q.sourceBounds.reduce((sum, bound) => sum + bound.totalBoundMm, 0);
    require(name, result.collision.kind === 'surface-separated' && Math.abs(result.collision.surfaceDistanceMm - 1) < 1e-10
      && q.meshDistance.state === 'bounded' && q.meshDistance.lowerMm - inflation <= 1 && q.meshDistance.upperMm + inflation >= 1
      && q.contactRegion.state === 'boxes' && q.containment.method === 'slab'
      && q.sourceBounds.every(bound => bound.state === 'bounded'), JSON.stringify(q));
    require(`${name}: source interval stays inside independent fixture enclosure`,
      q.meshDistance.upperMm - q.meshDistance.lowerMm <= q.limits.requestedNumericalWidthMm
      && q.meshDistance.lowerMm - inflation >= expected[0] - q.limits.requestedNumericalWidthMm
      && q.meshDistance.upperMm + inflation <= expected[1] + q.limits.requestedNumericalWidthMm, JSON.stringify(q));
  }
  try {
    await start();
    const plate = await generated();
    require('actual worker generated plate has authenticated exact source bound', plate.sourceQualification.state === 'bounded' && plate.analysisErrorBoundMm === 0);
    const planeQ = metadata(a.generatedPlaneQuery, plate, plate);
    bounded('actual worker generated default supported pair', await query(plate, plate, planeQ), [1, 1]);
    const shifted = clone(planeQ); shifted.right.rowTransformF64[14] = 6;
    const moved = await query(plate, plate, shifted);
    require('physical row is applied exactly once', moved.collision.surfaceDistanceMm === 4 && moved.queryQualification.meshDistance.lowerMm <= 4 && moved.queryQualification.meshDistance.upperMm >= 4);
    const cap = clone(planeQ); cap.qualificationLimits.maxTriangles = 1;
    const capped = await query(plate, plate, cap);
    require('actual worker cap preserves raw gap and refuses every certificate', capped.collision.surfaceDistanceMm === 1
      && [capped.queryQualification.meshDistance, ...capped.queryQualification.sourceBounds, capped.queryQualification.contactRegion, capped.queryQualification.containment].every(state => state.state === 'unknown'));
    const originalVendor = await vendor(a.directVendorRecipe.originalAsset);
    require('actual original 0805 complete source census', originalVendor.sourceFaces.length === 30 && originalVendor.faceOccurrences.length === 30
      && new Set(originalVendor.triangleFaceOccurrenceIndices).size === 30 && originalVendor.sourceQualification.state === 'bounded');
    const vendorQ = metadata(contract.qualificationV2.queryExamples, originalVendor, originalVendor);
    bounded('actual worker translated-first 0805 default supported pair', await query(originalVendor, originalVendor, vendorQ), a.pairCases[0].expectedSourceIntervalMm);
    const tight = clone(vendorQ); tight.requestedAccuracyMm = 0.01;
    const tightResult = await query(originalVendor, originalVendor, tight);
    const accuracy = q => (q.meshDistance.upperMm - q.meshDistance.lowerMm) / 2 + q.sourceBounds.reduce((sum, bound) => sum + bound.totalBoundMm, 0);
    require('actual 0805 precision prevents explicit .01 accuracy', accuracy(tightResult.queryQualification) > 0.01);
    bounded('actual worker mixed vendor/generated supported pair', await query(originalVendor, plate, metadata(a.mixedVendorGeneratedQuery, originalVendor, plate)), a.pairCases[2].expectedSourceIntervalMm);
    const cylinderBody = clone(contract.preparedBody);
    for (const [pointer, value] of Object.entries(a.cylinderBudgetCases.preparedBodyRecipe.replace)) {
      const parts = pointer.split('/').slice(1); let parent = cylinderBody;
      for (const part of parts.slice(0, -1)) parent = parent[part];
      parent[parts.at(-1)] = clone(value);
    }
    let coarse;
    for (const target of [null, .01]) {
      const cylinder = await generated(cylinderBody, target);
      const result = await query(cylinder, plate, metadata(a.generatedPlaneQuery, cylinder, plate, .01));
      require(`actual worker cylinder ${target ?? 'default'} cannot certify .01`, cylinder.sourceQualification.state === 'bounded'
        && cylinder.analysisMesh.settings.targetDeflectionMm === target && accuracy(result.queryQualification) > .01);
      if (target === null) coarse = cylinder;
      else require('refinement owns a new mesh and cannot reuse coarse settings', cylinder.analysisMesh.meshIdentity !== coarse.analysisMesh.meshIdentity);
    }
    const nested = clone(planeQ);
    nested.left.rowTransformF64 = [.1,0,0,0, 0,.1,0,0, 0,0,.1,0, 1,1,.5,1];
    nested.right.rowTransformF64 = clone(planeQ.left.rowTransformF64);
    const contained = await query(plate, plate, nested);
    require('actual nested surface miss cannot certify solid clear', contained.collision.kind === 'surface-separated'
      && contained.queryQualification.containment.state === 'unknown');
    const unsupported = await vendor('cad/monstertruck/tests/fixtures/touching-unit-boxes.step');
    require('actual complete nonmanifold vendor source remains Unknown', unsupported.sourceQualification.state === 'unknown' && !unsupported.accuracy.meshClosedOriented);
    for (const [angle, cosine, sine, eligible] of [[30, contract.qualificationV2.rotatedControl.cos30F64, contract.qualificationV2.rotatedControl.sin30F64, true], [45, Math.SQRT1_2, Math.SQRT1_2, false]]) {
      const rotated = clone(vendorQ);
      for (const side of ['left', 'right']) {
        const row = rotated[side].rowTransformF64, out = row.slice();
        for (const i of [0, 4, 8, 12]) { out[i] = cosine * row[i] - sine * row[i + 1]; out[i + 1] = sine * row[i] + cosine * row[i + 1]; }
        rotated[side].rowTransformF64 = out;
      }
      const result = await query(originalVendor, originalVendor, rotated);
      require(`actual worker ${angle} degree containment eligibility`, result.queryQualification.meshDistance.state === 'bounded'
        && result.queryQualification.containment.state === (eligible ? 'ruledOut' : 'unknown'));
    }
    const badBytes = await request('read-step', { sourceSha256: originalVendor.sourceSha256, stepImportOptions: {} }, { inputBytes: new Uint8Array([1, 2, 3]).buffer });
    require('actual worker refuses mismatched source bytes', badBytes.frame.outcome === 'failed' && /SHA-256/.test(badBytes.frame.error));
    const wrong = clone(planeQ); wrong.left.analysisRef.meshIdentity += '-wrong';
    const mismatch = await request('collision-query', { collisionQuery: wrong }, buffers(plate, plate));
    require('actual worker refuses forged reference/descriptor identity', mismatch.frame.outcome === 'failed');
    const corrupted = buffers(plate, plate); new Float64Array(corrupted.leftPositions)[0] += 1;
    const wrongBits = await request('collision-query', { collisionQuery: planeQ }, corrupted);
    require('actual worker refuses changed returned f64 bits', wrongBits.frame.outcome === 'failed');
    const cancelled = await request('analyze-generated-body', { prepared: { revision: 7, bodies: [contract.preparedBody] },
      generatedAnalysisRequest: { bodyId: contract.preparedBody.body.id, geometryIdentity: a.workerReferences.plane.source.geometryIdentity } }, {}, true);
    require('actual worker cancellation prevents evidence publication', cancelled.frame.outcome === 'cancelled' && cancelled.result == null);
    const panasonic = await vendor('ergogen/library/vendor/kicad/3d_models/Panasonic_EVQPUL_EVQPUC.step');
    require('actual worker Panasonic retains all 155 faces without solid/source promotion', panasonic.sourceFaces.length === 155 && panasonic.faceOccurrences.length === 155
      && panasonic.accuracy.meshClosedOriented === false && panasonic.accuracy.maxRepairDisplacementMm === null
      && panasonic.sourceQualification.state === 'unknown' && panasonic.accuracy.analysisErrorBoundMm === null);
    for (let i = 0; i <= a.opaqueHandleRules.sidecarMaxEntries; i++) await generated();
    const evicted = await query(plate, plate, planeQ);
    require('actual worker eviction cannot retain an old certificate', evicted.collision.kind === 'surface-separated'
      && evicted.queryQualification.meshDistance.state === 'unknown');
    const recovered = await generated();
    bounded('actual worker new owned preparation recovers after eviction', await query(recovered, recovered, metadata(a.generatedPlaneQuery, recovered, recovered)), [1, 1]);
    worker.terminate();
    await start();
    const restarted = await request('collision-query', { collisionQuery: planeQ }, buffers(plate, plate));
    require('actual worker restart cannot reuse previous owned evidence', restarted.frame.outcome === 'failed' || restarted.result?.queryQualification.meshDistance.state === 'unknown');
  } finally {
    worker?.terminate(); URL.revokeObjectURL(entry);
    for (const pending of waiting.values()) clearTimeout(pending.timer);
  }
}

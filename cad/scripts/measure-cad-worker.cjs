module.exports = async ({ backend, fixture, operation, sequence, isStartup }) => {
      const fail = message => { throw new Error(message); };
      const check = (condition, message) => { if (!condition) fail(message); };
      const json = async url => {
        const response = await fetch(url); check(response.ok, `missing ${url}`); return response.json();
      };
      const prepared = ['C1plate', 'C2trayboss', 'C3gasket', 'P1analyticPCB'].includes(fixture)
        ? await json(`/__perf/cad/test/fixtures/performance/${fixture}.json`) : null;
      const keycaps = fixture === 'K1keycap'
        ? await json('/__perf/cad/test/fixtures/performance/K1keycap.json') : null;
      const contract = fixture === 'Qvendor'
        ? await json('/__perf/cad/wasm/tests/fixtures/generated-analysis-contract.json') : null;
      let vendor;
      if (fixture === 'V1switch' || fixture === 'V2vendor') {
        const manifest = await json(`/__perf/cad/test/fixtures/${fixture === 'V1switch' ? 'followup-manifest' : 'manifest'}.json`);
        const entries = Array.isArray(manifest) ? manifest : manifest.fixtures;
        vendor = entries.find(entry => entry.id === (fixture === 'V1switch' ? 'imported-switch' : 'imported-multi-solid'));
        check(vendor, 'missing pinned vendor entry');
      } else if (contract) {
        vendor = { file: contract.qualificationV2.activation.directVendorRecipe.originalAsset,
          sha256: contract.qualificationV2.queryExamples.binding.leftAnalysis.source.sourceSha256 };
      }
      let bytes;
      if (vendor) {
        const response = await fetch(`/__source/${vendor.file}`); check(response.ok, 'missing original STEP');
        bytes = new Uint8Array(await response.arrayBuffer());
        const sha = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)),
          v => v.toString(16).padStart(2, '0')).join('');
        check(sha === vendor.sha256, 'original STEP authentication failed');
      }
      const revision = contract ? contract.qualificationV2.activation.snapshot.revision : (prepared?.revision ?? keycaps?.revision ?? 1);
      const identity = contract ? { ...contract.qualificationV2.activation.snapshot,
        token: Number(contract.qualificationV2.activation.snapshot.token),
        sessionEpoch: Number(contract.qualificationV2.activation.snapshot.sessionEpoch) }
        : { token: 11, sessionEpoch: 2, documentId: 'performance', boardId: prepared?.bodies[0].body.boardId ?? 'performance',
          instanceId: null, revision };
      let worker, sequenceId = 0;
      const waiting = new Map(), startup = [];
      const startWorker = async () => {
        worker?.terminate();
        const start = performance.now();
        worker = new Worker('/boardstudio/assets/cad-worker/entry.js', { type: 'module' });
        await new Promise((resolve, reject) => {
          const timer = setTimeout(() => reject(Error('worker ready watchdog: 180 seconds')), 180000);
          worker.onerror = e => { clearTimeout(timer); reject(Error(e.message)); };
          worker.onmessage = e => {
            const message = e.data;
            if (message.kind === 'ready') { clearTimeout(timer); resolve(); }
            if (message.kind === 'init-failed') { clearTimeout(timer); reject(Error(JSON.stringify(message))); }
            if (message.kind === 'reply') {
              let frame;
              try {
                frame = JSON.parse(message.frame);
                if (!frame || typeof frame !== 'object' || Array.isArray(frame) || typeof frame.requestId !== 'string') throw Error();
              } catch (_) {
                for (const pending of waiting.values()) { clearTimeout(pending.timer); pending.reject(Error('malformed worker reply')); }
                waiting.clear();
                return;
              }
              const pending = waiting.get(frame.requestId);
              if (pending) { waiting.delete(frame.requestId); clearTimeout(pending.timer); pending.resolve({ frame, result: message.result }); }
            }
          };
        });
        startup.push(performance.now() - start);
      };
      const request = async (op, fields, payload = {}) => {
        const requestId = `performance-${++sequenceId}`;
        return new Promise((resolve, reject) => {
          const timer = setTimeout(() => { waiting.delete(requestId); reject(Error(`${op} watchdog: 180 seconds`)); }, 180000);
          waiting.set(requestId, { resolve, reject, timer });
          worker.postMessage({ kind: 'request', frame: JSON.stringify({ requestId, jobId: requestId, identity, operation: op, ...fields }),
            ...payload }, Object.values(payload).filter(v => v instanceof ArrayBuffer));
        });
      };
      const completed = async (op, fields, payload) => {
        const reply = await request(op, fields, payload);
        check(reply.frame.outcome === 'completed', reply.frame.error ?? 'worker failed');
        check(JSON.stringify(reply.frame.identity) === JSON.stringify(identity), 'captured identity drift');
        return reply.result;
      };
      await startWorker();
      let queryFields, queryPayload;
      if (contract) {
        // Mint real owned references. Only the authored rows/threshold/limits come from the fixture.
        const imported = await completed('read-step', { sourceSha256: vendor.sha256,
          stepImportOptions: { previewRepairBudgetMm: null, targetDeflectionMm: null } }, { inputBytes: bytes.slice().buffer });
        const asset = imported.vendorStep, example = contract.qualificationV2.queryExamples, b = example.binding;
        check(asset.sourceFaces.length === 30 && asset.faceOccurrences.length === 30, '0805 source census');
        const operand = side => ({ modelId: b[`${side}ModelId`], sourceSha256: vendor.sha256,
          importOptions: asset.importOptions, rowTransformF64: b[`${side}RowTransformF64`],
          analysisRef: asset.analysisRef, analysisMesh: asset.analysisMesh });
        queryFields = { collisionQuery: { left: operand('left'), right: operand('right'),
          clearanceThresholdMm: b.clearanceThresholdMm, requestedAccuracyMm: operation === 'tight' ? .01 : null,
          qualificationLimits: example.limits } };
        queryPayload = () => ({ leftPositions: asset.surfacePositionsMmF64.slice().buffer,
          rightPositions: asset.surfacePositionsMmF64.slice().buffer,
          leftTags: asset.triangleFaceOccurrenceIndices.slice().buffer,
          rightTags: asset.triangleFaceOccurrenceIndices.slice().buffer });
      }
      const input = isStartup ? await json('/__perf/cad/test/fixtures/performance/C1plate.json') : prepared;
      const run = async () => {
        let op, fields, payload;
        if (input) { op = operation === 'step' ? 'export-step' : 'preview'; fields = { prepared: input }; }
        else if (keycaps) { op = operation === 'print' ? 'print-keycaps' : 'preview'; fields = { keycaps }; }
        else if (contract) { op = 'collision-query'; fields = queryFields; payload = queryPayload(); }
        else { op = 'read-step'; fields = { sourceSha256: vendor.sha256,
          stepImportOptions: { previewRepairBudgetMm: null, targetDeflectionMm: operation === 'refine' ? .01 : null } };
          payload = { inputBytes: bytes.slice().buffer }; }
        const start = performance.now();
        const reply = await completed(op, fields, payload);
        const duration = performance.now() - start;
        // All traversal/quality controls are outside the operation timer. Full independent QA is a separate admission.
        const meshes = reply.bodies ?? (reply.mesh ? [reply.mesh] : []);
        if (input || keycaps) check(reply.revision === revision, 'stale result revision');
        let bufferBytes = 0, triangles = 0;
        for (const mesh of meshes) {
          check(mesh.positions instanceof Float32Array && mesh.normals instanceof Float32Array
            && mesh.positions.length > 0 && mesh.positions.length % 9 === 0
            && mesh.positions.length === mesh.normals.length && mesh.positions.every(Number.isFinite)
            && mesh.normals.every(Number.isFinite), 'invalid mesh');
          bufferBytes += mesh.positions.byteLength + mesh.normals.byteLength;
          triangles += mesh.positions.length / 9;
        }
        if (input && op === 'preview') check(JSON.stringify(reply.bodies.map(b => b.id)) ===
          JSON.stringify(input.bodies.map(b => b.body.id)), 'missing/changed generated bodies');
        if (input && op === 'export-step') check(reply.step instanceof Uint8Array && reply.step.length > 0
          && !reply.mesh && !reply.bodies, 'invalid STEP-only output');
        if (keycaps) {
          const expected = keycaps.specs.flatMap(s => operation === 'print' ? [`keycap:${s.id}`]
            : [`keycap:${s.id}`, `keycap-legend:${s.id}`]);
          check(JSON.stringify(reply.bodies.map(b => b.id)) === JSON.stringify(expected), 'incomplete cap/legend bodies');
        }
        let metadata = {};
        if (vendor && !contract) {
          metadata = { bounds: reply.bounds };
          if (backend !== 'cadrum') {
            const asset = reply.vendorStep, count = fixture === 'V1switch' ? 155 : 529;
            check(asset.sourceSha256 === vendor.sha256 && asset.sourceFaces.length === count
              && asset.faceOccurrences.length === count && new Set(asset.triangleFaceOccurrenceIndices).size === count,
            'incomplete vendor source/occurrence coverage');
            metadata = { bounds: reply.bounds, precision: asset.precision, accuracy: asset.accuracy,
              importOptions: asset.importOptions, analysisMesh: asset.analysisMesh,
              sourceQualification: asset.sourceQualification, faces: asset.sourceFaces.length,
              occurrences: asset.faceOccurrences.length, lines: asset.lineOccurrences.length };
            bufferBytes += asset.surfacePositionsMmF64.byteLength + asset.triangleFaceOccurrenceIndices.byteLength
              + asset.lineSegmentsMmF64.byteLength + asset.segmentLineOccurrenceIndices.byteLength
              + (asset.lineRenderPositions?.byteLength ?? 0);
          }
        }
        if (contract) {
          check(reply.queryQualification?.binding?.leftAnalysis?.meshIdentity === queryFields.collisionQuery.left.analysisRef.meshIdentity,
            'query lost owned reference');
          const q = reply.queryQualification, expected = contract.qualificationV2.activation.pairCases[0].expectedSourceIntervalMm;
          check(reply.collision.kind === 'surface-separated' && Math.abs(reply.collision.surfaceDistanceMm-1) < 1e-10,
            'qualified query must preserve the independently authored 1mm surface gap');
          check(q.meshDistance.state === 'bounded' && q.sourceBounds.every(bound => bound.state === 'bounded'),
            'supported 0805 query must retain actual owned numerical/source bands');
          const inflation = q.sourceBounds.reduce((sum, bound) => sum+bound.totalBoundMm, 0);
          const interval = [q.meshDistance.lowerMm-inflation, q.meshDistance.upperMm+inflation];
          check(interval[0] <= 1 && interval[1] >= 1
            && interval[0] >= expected[0]-q.limits.requestedNumericalWidthMm
            && interval[1] <= expected[1]+q.limits.requestedNumericalWidthMm, 'source band must stay in the existing fixture enclosure');
          const budget = inflation+(q.meshDistance.upperMm-q.meshDistance.lowerMm)/2;
          if (operation === 'tight') check(budget > .01, 'tight request cannot certify the 0805 precision budget');
          metadata = { collision: reply.collision, queryQualification: q, sourceIntervalMm: interval,
            assessment: operation === 'tight' ? 'Unknown: precision budget exceeds requested 0.01mm' : 'supported default source band' };
        }
        window.__lastResult = reply;
        return { duration, triangles, bufferBytes, stepBytes: reply.step?.byteLength ?? 0, metadata };
      };
      window.__measure = { run, startWorker, startup, drop: () => { window.__lastResult = null; } };
      return { startup, input: input ?? keycaps ?? { source: vendor, targetDeflectionMm: operation === 'refine' ? .01 : null },
        identity, settings: { linearMm: .1, angularRad: .5 }, mode: sequence };
    };

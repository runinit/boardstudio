// Test-only observer supplied with agent-browser --init-script.
// Keeps protocol summaries (not document payloads or transferable byte buffers).
(() => {
  const OriginalWorker = window.Worker;
  const workers = [];
  const summary = (direction, url, value) => {
    const row = { at: performance.now(), direction, url };
    if (value === 'boardstudio-core-ready') {
      row.kind = 'core-ready';
    } else if (value && typeof value === 'object') {
      row.keys = Object.keys(value).sort();
      row.kind = value.kind ?? null;
      row.requestId = value.request_id ?? null;
      row.executorEpoch = value.executor_epoch ?? null;
      if (typeof value.frame === 'string') {
        try {
          const frame = JSON.parse(value.frame);
          row.frameKind = frame.kind ?? null;
          row.frameId = frame.id ?? frame.requestId ?? null;
          row.documentId = frame.document?.id ?? frame.documentId ?? null;
          row.revision = frame.revision ?? frame.document?.revision ?? null;
        } catch {
          row.frameParse = 'not-json';
        }
      }
    } else {
      row.valueType = typeof value;
    }
    return row;
  };
  window.Worker = new Proxy(OriginalWorker, {
    construct(target, args, newTarget) {
      const worker = Reflect.construct(target, args, newTarget);
      const url = new URL(String(args[0]), location.href).pathname;
      const entry = { url, createdAt: performance.now(), terminated: false };
      workers.push(entry);
      const postMessage = worker.postMessage.bind(worker);
      worker.postMessage = (message, transfer) => {
        window.__m1WorkerTrace.events.push(summary('out', url, message));
        return transfer === undefined ? postMessage(message) : postMessage(message, transfer);
      };
      worker.addEventListener('message', (event) => {
        window.__m1WorkerTrace.events.push(summary('in', url, event.data));
      });
      const terminate = worker.terminate.bind(worker);
      worker.terminate = () => {
        if (!entry.terminated) {
          entry.terminated = true;
          entry.terminatedAt = performance.now();
        }
        return terminate();
      };
      return worker;
    },
  });
  window.__m1WorkerTrace = {
    events: [],
    snapshot: () => ({
      workers: workers.map(({ url, createdAt, terminated, terminatedAt }) => ({ url, createdAt, terminated, terminatedAt })),
      events: [...window.__m1WorkerTrace.events],
    }),
  };
})();

(() => {
  const NativeWorker = window.Worker;
  window.__p2WorkerTraffic = { workers: [] };
  window.Worker = new Proxy(NativeWorker, {
    construct(target, argumentsList, newTarget) {
      const worker = Reflect.construct(target, argumentsList, newTarget);
      const url = String(argumentsList[0]);
      if (url.includes("worker-entry")) {
        const record = { url, requests: [], replies: [] };
        window.__p2WorkerTraffic.workers.push(record);
        const postMessage = worker.postMessage.bind(worker);
        Object.defineProperty(worker, "postMessage", {
          configurable: true,
          value: (message, ...transfer) => {
            const frame = typeof message === "string" ? message : JSON.stringify(message);
            record.requests.push({ bytes: new TextEncoder().encode(frame).byteLength, frame });
            return postMessage(message, ...transfer);
          },
        });
        worker.addEventListener("message", (event) => {
          const frame = typeof event.data === "string" ? event.data : JSON.stringify(event.data);
          record.replies.push({ bytes: new TextEncoder().encode(frame).byteLength, frame });
        });
      }
      return worker;
    },
  });
})()

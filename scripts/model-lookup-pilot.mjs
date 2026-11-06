// The worker is an experiment, not a security sandbox or an isolate equivalent.
// Errors the worker reports after its results are not observed (see the worker's settle comment).
export async function run(suite) {
  const responses = await Promise.all([fetch('/model-lookup.js'), fetch('/model-lookup.md')]);
  if (responses.some(response => !response.ok)) throw Error('model source or recipe unavailable');
  const [source, recipe] = await Promise.all(responses.map(response => response.text()));
  const worker = new Worker('/model-lookup-pilot-worker.js');
  try {
    return await new Promise((resolve, reject) => {
      const timeout = setTimeout(() => reject(Error('model lookup worker timeout')), 10000);
      let result;
      worker.addEventListener('message', event => {
        if (result || !Array.isArray(event.data)) {
          clearTimeout(timeout);
          reject(Error(`invalid worker message: ${JSON.stringify(event.data)}`));
          return;
        }
        // The worker publishes results only after its settle turns, so any error it reports comes first.
        result = event.data;
        clearTimeout(timeout);
        resolve(result);
      });
      worker.addEventListener('error', event => { clearTimeout(timeout); reject(Error(`worker error: ${event.message}`)); }, { once: true });
      worker.addEventListener('messageerror', () => { clearTimeout(timeout); reject(Error('worker message error')); }, { once: true });
      worker.postMessage({ suite, source, recipe });
    });
  } finally {
    worker.terminate();
  }
}
